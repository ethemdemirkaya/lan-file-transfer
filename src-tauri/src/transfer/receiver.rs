use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::events::{
    IncomingRequest, TransferCompleted, TransferProgress, TransferStarted,
    EVT_INCOMING_REQUEST, EVT_TRANSFER_COMPLETED, EVT_TRANSFER_PROGRESS, EVT_TRANSFER_STARTED,
};
use crate::hash::StreamHasher;
use crate::protocol::{Hello, HelloAck, CHUNK_SIZE, HASH_LEN};
use crate::state::{AppState, PendingDecision, UserDecision};
use crate::transfer::stream::{read_file_header, read_json, write_json};
use crate::transfer::{TransferError, TransferResult};

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

    let id = Uuid::new_v4().to_string();
    let request = IncomingRequest {
        id: id.clone(),
        peer: peer.clone(),
        device_name: hello.device_name.clone(),
        os: hello.os.clone(),
        file_count: hello.file_count,
        total_bytes: hello.total_bytes,
    };
    let decision = await_user_decision(&app, &id, request).await;
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

    let mut total_done: u64 = 0;
    let mut files_done: u64 = 0;
    let mut last_emit = Instant::now() - PROGRESS_INTERVAL;
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut partial_files: Vec<PathBuf> = Vec::new();

    let result = async {
        while let Some(header) = read_file_header(&mut reader).await? {
            let safe_rel = sanitize_relative_path(&header.rel_path)?;
            let final_path = save_dir.join(&safe_rel);
            if let Some(parent) = final_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let part_path = make_part_path(&final_path);
            partial_files.push(part_path.clone());

            let mut out = BufWriter::with_capacity(
                CHUNK_SIZE,
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&part_path)
                    .await?,
            );

            let mut hasher = StreamHasher::new();
            let mut remaining = header.file_size;
            let mut file_done: u64 = 0;

            while remaining > 0 {
                let want = remaining.min(buf.len() as u64) as usize;
                let n = reader.read(&mut buf[..want]).await?;
                if n == 0 {
                    return Err(TransferError::Protocol("unexpected EOF in file body".into()));
                }
                out.write_all(&buf[..n]).await?;
                hasher.update(&buf[..n]);
                remaining -= n as u64;
                file_done += n as u64;
                total_done += n as u64;
                if last_emit.elapsed() >= PROGRESS_INTERVAL {
                    last_emit = Instant::now();
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
                        },
                    );
                }
            }
            out.flush().await?;
            drop(out);

            let mut received_hash = [0u8; HASH_LEN];
            reader.read_exact(&mut received_hash).await?;
            let computed = hasher.finalize();
            if received_hash != computed {
                return Err(TransferError::HashMismatch(
                    safe_rel.to_string_lossy().into_owned(),
                ));
            }

            tokio::fs::rename(&part_path, &final_path).await?;
            // Only mark as 'kept' after successful rename. If we crash mid-loop
            // any later .part still in partial_files will be cleaned up below.
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
                },
            );
        }
        Ok::<(), TransferError>(())
    }
    .await;

    let elapsed_ms = started_at.elapsed().as_millis() as u64;
    match result {
        Ok(()) => {
            let _ = app.emit(
                EVT_TRANSFER_COMPLETED,
                TransferCompleted {
                    id,
                    direction: "recv",
                    success: true,
                    error: None,
                    elapsed_ms,
                    total_bytes: total_done,
                },
            );
            Ok(())
        }
        Err(e) => {
            // Cleanup partial files.
            for p in &partial_files {
                let _ = tokio::fs::remove_file(p).await;
            }
            let msg = format!("{e}");
            let _ = app.emit(
                EVT_TRANSFER_COMPLETED,
                TransferCompleted {
                    id,
                    direction: "recv",
                    success: false,
                    error: Some(msg),
                    elapsed_ms,
                    total_bytes: total_done,
                },
            );
            Err(e)
        }
    }
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
