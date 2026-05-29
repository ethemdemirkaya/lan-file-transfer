use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

use crate::events::{
    TransferCompleted, TransferProgress, TransferStarted, EVT_TRANSFER_COMPLETED,
    EVT_TRANSFER_PROGRESS, EVT_TRANSFER_STARTED,
};
use crate::hash::StreamHasher;
use crate::history::{self, HistoryRecord};
use crate::protocol::{
    current_os, Hello, HelloAck, CHUNK_SIZE, PROTOCOL_VERSION, RESUME_THRESHOLD,
};
use crate::state::{AppState, PauseToken};
use crate::transfer::stream::{read_json, read_resume_offset};
use std::sync::Arc;
use crate::transfer::{TransferError, TransferResult};
use tauri::Manager;

pub struct SendItem {
    pub local_path: PathBuf,
    pub rel_path: String,
    pub size: u64,
}

pub struct SendRequest {
    pub id: String,
    pub peer_addr: String,
    pub device_name: String,
    pub auth_code: String,
    pub items: Vec<SendItem>,
}

const PROGRESS_INTERVAL: Duration = Duration::from_millis(50);
const SOCKET_BUFFER: usize = 4 * 1024 * 1024;
const WRITER_QUEUE: usize = 16;

/// Commands the read side sends to the dedicated write task.
enum WriterCmd {
    /// Append the bytes to the socket's BufWriter; flush happens lazily
    /// when the buffer fills up.
    Bytes(Vec<u8>),
    /// Force a flush and notify on the oneshot — used right before we
    /// expect to read a reply from the peer (e.g. the resume offset),
    /// so the peer has actually seen our header.
    FlushAndAck(oneshot::Sender<()>),
}

/// Windows `CreateFile` flag that asks the cache manager to optimise for
/// sequential access — bigger read-ahead, lower cache pressure. Roughly
/// 2-3× improvement when streaming many tiny files off NTFS.
#[cfg(windows)]
const FILE_FLAG_SEQUENTIAL_SCAN: u32 = 0x0800_0000;

async fn open_for_streaming(path: &Path) -> std::io::Result<File> {
    let path = path.to_owned();
    let std_file = tokio::task::spawn_blocking(move || {
        let mut opts = std::fs::OpenOptions::new();
        opts.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            opts.custom_flags(FILE_FLAG_SEQUENTIAL_SCAN);
        }
        opts.open(path)
    })
    .await
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))??;
    Ok(File::from_std(std_file))
}

pub async fn run_send(app: AppHandle, req: SendRequest) -> TransferResult<()> {
    let started_at = Instant::now();
    let total_bytes: u64 = req.items.iter().map(|i| i.size).sum();
    let file_count = req.items.len() as u64;
    let id = req.id.clone();
    let peer_label = req.peer_addr.clone();

    // Register a cancel handle so the UI's stop button can abort us.
    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    let pause = Arc::new(PauseToken::default());
    {
        let state: tauri::State<'_, AppState> = app.state();
        state.cancel.lock().unwrap().insert(id.clone(), cancel_tx);
        state.pause.lock().unwrap().insert(id.clone(), pause.clone());
    }

    let do_send_fut = do_send(&app, req, started_at, total_bytes, file_count, id.clone(), pause);
    tokio::pin!(do_send_fut);
    let result: TransferResult<u64> = tokio::select! {
        r = &mut do_send_fut => r,
        _ = cancel_rx => Err(TransferError::Cancelled),
    };

    // Drop the cancel + pause handles regardless of outcome (idempotent
    // if the command already pulled either out).
    {
        let state: tauri::State<'_, AppState> = app.state();
        state.cancel.lock().unwrap().remove(&id);
        state.pause.lock().unwrap().remove(&id);
    }

    let elapsed_ms = started_at.elapsed().as_millis() as u64;
    let (success, error, bytes_sent) = match &result {
        Ok(bytes) => (true, None, *bytes),
        Err(TransferError::Cancelled) => (false, Some("canceled_by_user".into()), 0u64),
        Err(TransferError::Io(io_err)) if is_peer_closed(io_err) => {
            (false, Some("canceled_by_peer".into()), 0u64)
        }
        Err(e) => (false, Some(format!("{e}")), 0u64),
    };

    {
        let state: tauri::State<'_, AppState> = app.state();
        let cfg_dir = state.session.lock().unwrap().settings_dir.clone();
        let record = HistoryRecord {
            id: id.clone(),
            direction: "send".into(),
            peer: peer_label,
            bytes: if success { total_bytes } else { bytes_sent },
            file_count,
            elapsed_ms,
            success,
            error: error.clone(),
            finished_at: history::now_ms(),
        };
        if let Err(e) = history::push(&cfg_dir, record) {
            tracing::warn!("history append failed: {e}");
        }
    }

    let _ = app.emit(
        EVT_TRANSFER_COMPLETED,
        TransferCompleted {
            id,
            direction: "send",
            success,
            error,
            elapsed_ms,
            total_bytes: if success { total_bytes } else { bytes_sent },
        },
    );

    result.map(|_| ())
}

async fn do_send(
    app: &AppHandle,
    req: SendRequest,
    _started_at: Instant,
    total_bytes: u64,
    file_count: u64,
    id: String,
    pause: Arc<PauseToken>,
) -> TransferResult<u64> {
    let stream = TcpStream::connect(&req.peer_addr).await?;
    stream.set_nodelay(true)?;
    // Bigger socket buffers so the kernel keeps several megabytes in
    // flight on our behalf — that's what lets the disk-read pipeline
    // actually overlap the network send.
    {
        let sref = socket2::SockRef::from(&stream);
        let _ = sref.set_send_buffer_size(SOCKET_BUFFER);
        let _ = sref.set_recv_buffer_size(SOCKET_BUFFER);
    }

    let (read_half, write_half) = stream.into_split();
    let mut reader = BufReader::with_capacity(CHUNK_SIZE, read_half);

    // The write task runs the actual socket.write_all calls on its own
    // tokio task. The read side feeds it via `wtx` and only blocks when
    // the bounded channel fills (back-pressure). This is what lets us
    // start opening / reading the *next* file while the *previous* file
    // is still draining onto the wire — the big win for many-tiny-files.
    let (wtx, mut wrx) = mpsc::channel::<WriterCmd>(WRITER_QUEUE);
    let writer_task = tokio::spawn(async move {
        let mut writer = BufWriter::with_capacity(CHUNK_SIZE, write_half);
        while let Some(cmd) = wrx.recv().await {
            match cmd {
                WriterCmd::Bytes(b) => writer.write_all(&b).await?,
                WriterCmd::FlushAndAck(ack) => {
                    writer.flush().await?;
                    let _ = ack.send(());
                }
            }
        }
        writer.flush().await?;
        writer.shutdown().await.ok();
        Ok::<(), std::io::Error>(())
    });

    // 1) Hello — length-prefixed JSON.
    let hello = Hello {
        version: PROTOCOL_VERSION,
        device_name: req.device_name.clone(),
        os: current_os().to_string(),
        file_count,
        total_bytes,
        auth_code: req.auth_code.clone(),
    };
    send_json(&wtx, &hello).await?;
    flush_through_writer(&wtx).await?;

    let ack: HelloAck = read_json(&mut reader).await?;
    if !ack.accept {
        // Drop the sender so the writer task exits cleanly before we bail.
        drop(wtx);
        let _ = writer_task.await;
        let reason = ack.reason.unwrap_or_else(|| "rejected".into());
        return Err(TransferError::Rejected(reason));
    }

    let _ = app.emit(
        EVT_TRANSFER_STARTED,
        TransferStarted {
            id: id.clone(),
            direction: "send",
            peer: req.peer_addr.clone(),
            file_count,
            total_bytes,
        },
    );

    let mut total_done: u64 = 0;
    let mut files_done: u64 = 0;
    let mut last_emit = Instant::now() - PROGRESS_INTERVAL;
    let mut last_emit_bytes: u64 = 0;
    let mut buf = vec![0u8; CHUNK_SIZE];

    for item in &req.items {
        let resumable = item.size >= RESUME_THRESHOLD;

        // Per-file header: u16 path_len, path bytes, u64 size, u8 resumable.
        let path_bytes = item.rel_path.as_bytes();
        if path_bytes.is_empty() || path_bytes.len() > u16::MAX as usize {
            return Err(TransferError::Protocol(format!(
                "invalid path: {}",
                item.rel_path
            )));
        }
        let mut header = Vec::with_capacity(2 + path_bytes.len() + 8 + 1);
        header.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());
        header.extend_from_slice(path_bytes);
        header.extend_from_slice(&item.size.to_le_bytes());
        header.push(u8::from(resumable));
        send_bytes(&wtx, header).await?;

        let resume_offset = if resumable {
            // Force a flush so the header actually lands on the wire before
            // we wait for the peer's offset reply.
            flush_through_writer(&wtx).await?;
            read_resume_offset(&mut reader).await?
        } else {
            0
        };
        if resume_offset > 0 {
            total_done = total_done.saturating_add(resume_offset);
        }

        let mut hasher = StreamHasher::new();
        let mut consumed: u64 = 0;
        let mut file = BufReader::with_capacity(CHUNK_SIZE, open_for_streaming(&item.local_path).await?);

        loop {
            // Honour pause requests between chunks. wait_if_paused returns
            // immediately when not paused, otherwise blocks on Notify.
            pause.wait_if_paused().await;
            let n = file.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            let bytes_before = consumed;
            consumed += n as u64;

            // Skip the bytes the receiver already has on disk; whatever is
            // left of this chunk goes onto the wire.
            let skip_in_chunk = if bytes_before < resume_offset {
                ((resume_offset - bytes_before).min(n as u64)) as usize
            } else {
                0
            };
            if skip_in_chunk < n {
                send_bytes(&wtx, buf[skip_in_chunk..n].to_vec()).await?;
                let sent = (n - skip_in_chunk) as u64;
                total_done = total_done.saturating_add(sent);
            }

            if last_emit.elapsed() >= PROGRESS_INTERVAL {
                let elapsed = last_emit.elapsed().as_secs_f64().max(0.001);
                let inst = (total_done.saturating_sub(last_emit_bytes)) as f64
                    / elapsed
                    / (1024.0 * 1024.0);
                last_emit = Instant::now();
                last_emit_bytes = total_done;
                let _ = app.emit(
                    EVT_TRANSFER_PROGRESS,
                    TransferProgress {
                        id: id.clone(),
                        direction: "send",
                        current_file: item.rel_path.clone(),
                        current_bytes_done: consumed,
                        current_bytes_total: item.size,
                        total_bytes_done: total_done,
                        total_bytes,
                        files_done,
                        files_total: file_count,
                        instant_mbps_network: inst,
                        instant_mbps_disk: 0.0,
                    },
                );
            }
        }

        send_bytes(&wtx, hasher.finalize().to_vec()).await?;
        files_done += 1;

        let _ = app.emit(
            EVT_TRANSFER_PROGRESS,
            TransferProgress {
                id: id.clone(),
                direction: "send",
                current_file: item.rel_path.clone(),
                current_bytes_done: consumed,
                current_bytes_total: item.size,
                total_bytes_done: total_done,
                total_bytes,
                files_done,
                files_total: file_count,
                instant_mbps_network: 0.0,
                instant_mbps_disk: 0.0,
            },
        );
    }

    // End-of-stream marker (u16 = 0).
    send_bytes(&wtx, vec![0u8, 0u8]).await?;

    drop(wtx);
    match writer_task.await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(TransferError::Io(e)),
        Err(e) => return Err(TransferError::Protocol(format!("writer panic: {e}"))),
    }

    Ok(total_done)
}

/// True when the io error looks like the other end yanked the connection
/// (typical when the peer's user pressed the cancel button).
fn is_peer_closed(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::UnexpectedEof
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::BrokenPipe
    )
}

async fn send_bytes(tx: &mpsc::Sender<WriterCmd>, bytes: Vec<u8>) -> TransferResult<()> {
    tx.send(WriterCmd::Bytes(bytes))
        .await
        .map_err(|_| TransferError::Protocol("writer task ended".into()))
}

async fn send_json<T: serde::Serialize>(
    tx: &mpsc::Sender<WriterCmd>,
    value: &T,
) -> TransferResult<()> {
    let body = serde_json::to_vec(value)?;
    let mut framed = Vec::with_capacity(4 + body.len());
    framed.extend_from_slice(&(body.len() as u32).to_le_bytes());
    framed.extend_from_slice(&body);
    send_bytes(tx, framed).await
}

async fn flush_through_writer(tx: &mpsc::Sender<WriterCmd>) -> TransferResult<()> {
    let (ack_tx, ack_rx) = oneshot::channel();
    tx.send(WriterCmd::FlushAndAck(ack_tx))
        .await
        .map_err(|_| TransferError::Protocol("writer task ended".into()))?;
    ack_rx
        .await
        .map_err(|_| TransferError::Protocol("writer task dropped flush ack".into()))?;
    Ok(())
}

/// Build a SendRequest from a set of absolute local paths (files or
/// directories). Directories are walked recursively; each file's wire path
/// is `<dir_basename>/<relative path inside dir>`. Standalone files use
/// their basename. Forward-slash separator is enforced.
pub fn build_paths_request(
    id: String,
    peer_addr: String,
    device_name: String,
    auth_code: String,
    paths: &[PathBuf],
) -> std::io::Result<SendRequest> {
    let mut items: Vec<SendItem> = Vec::new();
    for p in paths {
        let metadata = std::fs::metadata(p)?;
        if metadata.is_file() {
            let rel = basename_or_default(p);
            items.push(SendItem {
                local_path: p.clone(),
                rel_path: rel,
                size: metadata.len(),
            });
        } else if metadata.is_dir() {
            let root_name = basename_or_default(p);
            for entry in walkdir::WalkDir::new(p).follow_links(false) {
                let entry = entry.map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
                })?;
                if !entry.file_type().is_file() {
                    continue;
                }
                let abs = entry.path();
                let rel_in_dir = abs.strip_prefix(p).unwrap_or(abs);
                let rel_str = rel_in_dir.to_string_lossy().replace('\\', "/");
                let rel_path = if rel_str.is_empty() {
                    root_name.clone()
                } else {
                    format!("{root_name}/{rel_str}")
                };
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                items.push(SendItem {
                    local_path: abs.to_path_buf(),
                    rel_path,
                    size,
                });
            }
        }
    }
    Ok(SendRequest {
        id,
        peer_addr,
        device_name,
        auth_code,
        items,
    })
}

fn basename_or_default(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| "item".to_string())
}
