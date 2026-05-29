use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::events::{
    IncomingRequest, TransferCompleted, TransferProgress, TransferStarted,
    EVT_INCOMING_REQUEST, EVT_TRANSFER_COMPLETED, EVT_TRANSFER_PROGRESS, EVT_TRANSFER_STARTED,
};
use crate::hash::StreamHasher;
use crate::history::{self, HistoryRecord};
use crate::protocol::{Hello, HelloAck, CHUNK_SIZE, HASH_LEN};
use crate::transfer::stream::write_resume_offset;
use crate::state::{AppState, PauseToken, PendingDecision, UserDecision};
use crate::transfer::stream::{read_file_header, read_json, write_json};
use crate::transfer::{TransferError, TransferResult};
use crate::{disk, settings};

const REQUEST_TIMEOUT_SECS: u64 = 120;

const PROGRESS_INTERVAL: Duration = Duration::from_millis(50);

pub struct ReceiverHandle {
    pub port: u16,
    pub shutdown: oneshot::Sender<()>,
}

pub async fn start_receiver(
    app: AppHandle,
    save_dir: PathBuf,
    port: u16,
) -> std::io::Result<ReceiverHandle> {
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    let bound = listener.local_addr()?;
    let actual_port = bound.port();
    let (tx, mut rx) = oneshot::channel::<()>();

    let app_clone = app.clone();
    tokio::spawn(async move {
        info!("receiver listening on {}", bound);
        loop {
            tokio::select! {
                _ = &mut rx => {
                    info!("receiver shutdown requested");
                    break;
                }
                accept = listener.accept() => {
                    match accept {
                        Ok((sock, peer)) => {
                            let app2 = app_clone.clone();
                            let dir = save_dir.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(app2, sock, peer.to_string(), dir).await {
                                    error!("receive failed: {e}");
                                }
                            });
                        }
                        Err(e) => {
                            warn!("accept error: {e}");
                        }
                    }
                }
            }
        }
    });

    Ok(ReceiverHandle {
        port: actual_port,
        shutdown: tx,
    })
}

/// Ask the user (via the frontend) whether to accept an incoming transfer.
/// Returns the user's decision, including an optional override save dir.
async fn await_user_decision(
    app: &AppHandle,
    id: &str,
    request: IncomingRequest,
) -> UserDecision {
    let (tx, rx) = oneshot::channel::<UserDecision>();
    {
        let state: tauri::State<'_, AppState> = app.state();
        state
            .pending
            .lock()
            .unwrap()
            .insert(id.to_string(), PendingDecision { tx });
    }
    if let Err(e) = app.emit(EVT_INCOMING_REQUEST, &request) {
        warn!("emit incoming failed: {e}");
    }
    let timeout = tokio::time::Duration::from_secs(REQUEST_TIMEOUT_SECS);
    let outcome = tokio::time::timeout(timeout, rx).await;
    {
        let state: tauri::State<'_, AppState> = app.state();
        state.pending.lock().unwrap().remove(id);
    }
    match outcome {
        Ok(Ok(d)) => d,
        _ => UserDecision {
            accept: false,
            override_save_dir: None,
        },
    }
}

// Touch Arc so dropping cleanup compiles cleanly.
#[allow(dead_code)]
fn _force_arc(_a: Arc<()>) {}

async fn handle_connection(
    app: AppHandle,
    sock: TcpStream,
    peer: String,
    save_dir: PathBuf,
) -> TransferResult<()> {
    sock.set_nodelay(true)?;
    // 4 MB socket buffers — keeps several flights of file bodies in the
    // kernel so the sender's pipelined read doesn't stall on TCP window.
    {
        let sref = socket2::SockRef::from(&sock);
        let _ = sref.set_recv_buffer_size(4 * 1024 * 1024);
        let _ = sref.set_send_buffer_size(4 * 1024 * 1024);
    }
    let (read_half, write_half) = sock.into_split();
    let mut reader = BufReader::with_capacity(CHUNK_SIZE, read_half);
    let mut writer = BufWriter::with_capacity(CHUNK_SIZE, write_half);

    let hello: Hello = read_json(&mut reader).await?;
    info!(
        "incoming transfer from {} ({}): {} files, {} bytes",
        hello.device_name, peer, hello.file_count, hello.total_bytes
    );

    // Verify pairing code before bothering the user.
    let expected_code = {
        let state: tauri::State<'_, AppState> = app.state();
        let code = state.session.lock().unwrap().auth_code.clone();
        code
    };
    if hello.auth_code != expected_code {
        let ack = HelloAck {
            accept: false,
            reason: Some("Yanlış eşleştirme kodu.".into()),
        };
        write_json(&mut writer, &ack).await?;
        writer.flush().await?;
        warn!(
            "rejected transfer from {} ({}): bad pairing code",
            hello.device_name, peer
        );
        return Ok(());
    }

    // Disk-space pre-check before bothering the user. Reject if we can't fit
    // 1.05x the announced size on the destination volume.
    {
        let need = hello.total_bytes.saturating_add(hello.total_bytes / 20);
        if let Some(avail) = disk::available_for(&save_dir) {
            if need > avail {
                let ack = HelloAck {
                    accept: false,
                    reason: Some(format!(
                        "Yetersiz disk alanı: gereken ~{} bayt, mevcut {} bayt.",
                        need, avail
                    )),
                };
                write_json(&mut writer, &ack).await?;
                writer.flush().await?;
                warn!(
                    "rejected transfer from {}: need {} bytes, only {} available",
                    hello.device_name, need, avail
                );
                return Ok(());
            }
        }
    }

    // Auto-accept if this sender is in the trust list.
    let trusted = {
        let state: tauri::State<'_, AppState> = app.state();
        let s = state.session.lock().unwrap();
        s.settings.trusted_devices.contains_key(&hello.device_name)
    };

    let id = Uuid::new_v4().to_string();
    let started_total = hello.total_bytes;
    let started_files = hello.file_count;
    let sender_name = hello.device_name.clone();
    let request = IncomingRequest {
        id: id.clone(),
        peer: peer.clone(),
        device_name: hello.device_name.clone(),
        os: hello.os.clone(),
        file_count: hello.file_count,
        total_bytes: hello.total_bytes,
    };
    let decision = if trusted {
        info!("auto-accepting trusted sender: {}", hello.device_name);
        // Do NOT emit the incoming request — the UI would pop the accept
        // dialog. The transfer://started event right after will surface
        // the in-flight progress; that's enough.
        UserDecision { accept: true, override_save_dir: None }
    } else {
        await_user_decision(&app, &id, request).await
    };
    let ack = HelloAck {
        accept: decision.accept,
        reason: if decision.accept {
            None
        } else {
            Some("Kullanıcı reddetti veya zaman aşımı.".into())
        },
    };
    write_json(&mut writer, &ack).await?;
    writer.flush().await?;
    if !decision.accept {
        return Ok(());
    }
    let effective_save_dir = decision.override_save_dir.unwrap_or(save_dir.clone());
    if !effective_save_dir.is_dir() {
        tokio::fs::create_dir_all(&effective_save_dir).await?;
    }
    let save_dir = effective_save_dir;
    let started_at = Instant::now();
    let total_bytes_expected = hello.total_bytes;
    let files_total = hello.file_count;

    let _ = app.emit(
        EVT_TRANSFER_STARTED,
        TransferStarted {
            id: id.clone(),
            direction: "recv",
            peer: format!("{} ({peer})", hello.device_name),
            file_count: files_total,
            total_bytes: total_bytes_expected,
        },
    );

    // Register a cancel handle so the UI's stop button on the receiver
    // side can abort the body loop. Sender side has its own registration
    // in run_send.
    let (cancel_tx, mut cancel_rx) = oneshot::channel::<()>();
    let pause = Arc::new(PauseToken::default());
    {
        let state: tauri::State<'_, AppState> = app.state();
        state.cancel.lock().unwrap().insert(id.clone(), cancel_tx);
        state.pause.lock().unwrap().insert(id.clone(), pause.clone());
    }

    let disk_counter = Arc::new(AtomicU64::new(0));

    let body_fut = async {
        let mut total_done: u64 = 0;
        let mut files_done: u64 = 0;
        let mut last_emit = Instant::now() - PROGRESS_INTERVAL;
        let mut last_emit_net: u64 = 0;
        let mut last_emit_disk: u64 = 0;
        let mut buf = vec![0u8; CHUNK_SIZE];
        let mut partial_files: Vec<PathBuf> = Vec::new();

        while let Some(header) = read_file_header(&mut reader).await? {
            let safe_rel = sanitize_relative_path(&header.rel_path)?;
            let final_path = save_dir.join(&safe_rel);
            if let Some(parent) = final_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let part_path = make_part_path(&final_path);

            // Resume handshake: for big files, tell the sender how many bytes
            // we already have on disk so it can skip them. Reject mismatched
            // .part files (those bigger than expected) by deleting them.
            let mut resume_offset: u64 = 0;
            if header.resumable {
                if let Ok(meta) = tokio::fs::metadata(&part_path).await {
                    if meta.is_file() && meta.len() <= header.file_size {
                        resume_offset = meta.len();
                    } else {
                        let _ = tokio::fs::remove_file(&part_path).await;
                    }
                }
                write_resume_offset(&mut writer, resume_offset).await?;
                writer.flush().await?;
            } else {
                // Non-resumable files always start fresh; nuke any stale .part.
                let _ = tokio::fs::remove_file(&part_path).await;
            }
            partial_files.push(part_path.clone());

            // Hash already-on-disk bytes (so the final blake3 covers the full
            // file even when we resumed mid-way).
            let mut hasher = StreamHasher::new();
            if resume_offset > 0 {
                let mut pf = BufReader::with_capacity(
                    CHUNK_SIZE,
                    File::open(&part_path).await?,
                );
                let mut tmp = vec![0u8; CHUNK_SIZE];
                let mut read = 0u64;
                while read < resume_offset {
                    let want = (resume_offset - read).min(tmp.len() as u64) as usize;
                    let n = pf.read(&mut tmp[..want]).await?;
                    if n == 0 {
                        return Err(TransferError::Protocol(
                            ".part shrunk during resume".into(),
                        ));
                    }
                    hasher.update(&tmp[..n]);
                    read += n as u64;
                }
                // Bytes already on disk count toward both the running disk
                // total and the disk-rate baseline (otherwise the next emit
                // would falsely spike by resume_offset MB).
                disk_counter.fetch_add(resume_offset, Ordering::Relaxed);
                last_emit_disk = last_emit_disk.saturating_add(resume_offset);
            }

            let out_file = if resume_offset > 0 {
                OpenOptions::new().append(true).open(&part_path).await?
            } else {
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&part_path)
                    .await?
            };

            // Read side and write side run as separate tasks connected by
            // a bounded channel. That decouples the two halves so we can
            // measure them independently and so a slower disk doesn't
            // stall the receive socket beyond ~CAP*CHUNK bytes of buffer.
            const CHANNEL_CAP: usize = 8;
            let (tx, mut rx) = mpsc::channel::<Vec<u8>>(CHANNEL_CAP);
            let disk_counter_w = disk_counter.clone();
            let writer_task = tokio::spawn(async move {
                let mut out = BufWriter::with_capacity(CHUNK_SIZE, out_file);
                while let Some(chunk) = rx.recv().await {
                    out.write_all(&chunk).await?;
                    disk_counter_w.fetch_add(chunk.len() as u64, Ordering::Relaxed);
                }
                out.flush().await?;
                Ok::<(), std::io::Error>(())
            });

            // Count resumed bytes toward both per-file and total progress so
            // the bar starts where it left off instead of at zero.
            let mut file_done: u64 = resume_offset;
            total_done = total_done.saturating_add(resume_offset);
            let mut remaining = header.file_size - resume_offset;

            let read_result = async {
                while remaining > 0 {
                    pause.wait_if_paused().await;
                    let want = remaining.min(buf.len() as u64) as usize;
                    let n = reader.read(&mut buf[..want]).await?;
                    if n == 0 {
                        return Err(TransferError::Protocol(
                            "unexpected EOF in file body".into(),
                        ));
                    }
                    hasher.update(&buf[..n]);
                    if tx.send(buf[..n].to_vec()).await.is_err() {
                        return Err(TransferError::Protocol(
                            "writer task ended unexpectedly".into(),
                        ));
                    }
                    remaining -= n as u64;
                    file_done += n as u64;
                    total_done += n as u64;
                    if last_emit.elapsed() >= PROGRESS_INTERVAL {
                        let elapsed = last_emit.elapsed().as_secs_f64().max(0.001);
                        let now_disk = disk_counter.load(Ordering::Relaxed);
                        let inst_net = (total_done.saturating_sub(last_emit_net)) as f64
                            / elapsed
                            / (1024.0 * 1024.0);
                        let inst_disk = (now_disk.saturating_sub(last_emit_disk)) as f64
                            / elapsed
                            / (1024.0 * 1024.0);
                        last_emit = Instant::now();
                        last_emit_net = total_done;
                        last_emit_disk = now_disk;
                        let _ = app.emit(
                            EVT_TRANSFER_PROGRESS,
                            TransferProgress {
                                id: id.clone(),
                                direction: "recv",
                                current_file: safe_rel.to_string_lossy().into_owned(),
                                current_bytes_done: file_done,
                                current_bytes_total: header.file_size,
                                total_bytes_done: total_done,
                                total_bytes: total_bytes_expected,
                                files_done,
                                files_total,
                                instant_mbps_network: inst_net,
                                instant_mbps_disk: inst_disk,
                            },
                        );
                    }
                }
                Ok::<(), TransferError>(())
            }
            .await;

            // Always drop the sender so the writer task drains and exits,
            // even on error — otherwise the join would hang.
            drop(tx);
            let writer_join = writer_task.await;
            // Propagate any read-side error before surfacing writer errors.
            read_result?;
            match writer_join {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(TransferError::Io(e)),
                Err(e) => return Err(TransferError::Protocol(format!("writer panic: {e}"))),
            }

            let mut received_hash = [0u8; HASH_LEN];
            reader.read_exact(&mut received_hash).await?;
            let computed = hasher.finalize();
            if received_hash != computed {
                return Err(TransferError::HashMismatch(
                    safe_rel.to_string_lossy().into_owned(),
                ));
            }

            tokio::fs::rename(&part_path, &final_path).await?;
            if let Some(pos) = partial_files.iter().position(|p| p == &part_path) {
                partial_files.remove(pos);
            }
            files_done += 1;

            let _ = app.emit(
                EVT_TRANSFER_PROGRESS,
                TransferProgress {
                    id: id.clone(),
                    direction: "recv",
                    current_file: safe_rel.to_string_lossy().into_owned(),
                    current_bytes_done: file_done,
                    current_bytes_total: header.file_size,
                    total_bytes_done: total_done,
                    total_bytes: total_bytes_expected,
                    files_done,
                    files_total,
                    instant_mbps_network: 0.0,
                    instant_mbps_disk: 0.0,
                },
            );
        }
        // partial_files is intentionally kept so the user can clean up
        // .part files manually if they want; with resume support they're
        // valuable across runs.
        let _ = partial_files;
        Ok::<u64, TransferError>(total_done)
    };
    tokio::pin!(body_fut);
    let (result, total_done): (TransferResult<()>, u64) = tokio::select! {
        r = &mut body_fut => match r {
            Ok(td) => (Ok(()), td),
            Err(e) => (Err(e), 0),
        },
        _ = &mut cancel_rx => (Err(TransferError::Cancelled), 0),
    };

    // Drop the cancel + pause handles.
    {
        let state: tauri::State<'_, AppState> = app.state();
        state.cancel.lock().unwrap().remove(&id);
        state.pause.lock().unwrap().remove(&id);
    }

    let elapsed_ms = started_at.elapsed().as_millis() as u64;
    let (success, error) = match &result {
        Ok(_) => (true, None),
        Err(TransferError::Cancelled) => (false, Some("canceled_by_user".into())),
        Err(TransferError::Io(io_err))
            if matches!(
                io_err.kind(),
                std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::BrokenPipe
            ) =>
        {
            (false, Some("canceled_by_peer".into()))
        }
        Err(TransferError::Protocol(msg)) if msg.contains("unexpected EOF") => {
            // The sender closed the socket mid-body; treat that as a peer
            // cancel rather than a "protocol violation".
            (false, Some("canceled_by_peer".into()))
        }
        Err(e) => (false, Some(format!("{e}"))),
    };

    // Persist to history.json so the UI can show it across restarts.
    {
        let state: tauri::State<'_, AppState> = app.state();
        let cfg_dir = state.session.lock().unwrap().settings_dir.clone();
        let record = HistoryRecord {
            id: id.clone(),
            direction: "recv".into(),
            peer: format!("{sender_name} ({peer})"),
            bytes: if success { started_total } else { total_done },
            file_count: started_files,
            elapsed_ms,
            success,
            error: error.clone(),
            finished_at: history::now_ms(),
        };
        if let Err(e) = history::push(&cfg_dir, record) {
            warn!("history append failed: {e}");
        }
    }

    let id_for_event = id.clone();
    let _ = app.emit(
        EVT_TRANSFER_COMPLETED,
        TransferCompleted {
            id: id_for_event,
            direction: "recv",
            success,
            error,
            elapsed_ms,
            total_bytes: total_done,
        },
    );

    let _ = settings::random_code; // silence import-use when no settings rotation here
    result
}

// Touch File to avoid unused import warnings (kept for parity with sender).
#[allow(dead_code)]
fn _force_use_file(_f: &File) {}

fn make_part_path(final_path: &Path) -> PathBuf {
    let mut s = final_path.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

/// Reject absolute paths, drive letters, `..` components, root prefixes,
/// and anything not strictly inside the receive directory. Forward-slash
/// separator is normalized to the platform separator.
fn sanitize_relative_path(rel: &str) -> TransferResult<PathBuf> {
    if rel.is_empty() {
        return Err(TransferError::UnsafePath("empty".into()));
    }
    if rel.starts_with('/') || rel.starts_with('\\') {
        return Err(TransferError::UnsafePath(rel.into()));
    }
    // Reject Windows drive letters like "C:/..." or "C:..."
    if rel.len() >= 2 && rel.as_bytes()[1] == b':' {
        return Err(TransferError::UnsafePath(rel.into()));
    }
    let normalized = rel.replace('\\', "/");
    let candidate = PathBuf::from(&normalized);
    let mut out = PathBuf::new();
    for comp in candidate.components() {
        match comp {
            Component::Normal(part) => out.push(part),
            Component::CurDir => continue,
            _ => return Err(TransferError::UnsafePath(rel.into())),
        }
    }
    if out.as_os_str().is_empty() {
        return Err(TransferError::UnsafePath(rel.into()));
    }
    Ok(out)
}
