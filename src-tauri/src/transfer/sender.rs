use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;

use crate::events::{
    TransferCompleted, TransferProgress, TransferStarted, EVT_TRANSFER_COMPLETED,
    EVT_TRANSFER_PROGRESS, EVT_TRANSFER_STARTED,
};
use crate::hash::StreamHasher;
use crate::protocol::{current_os, Hello, HelloAck, CHUNK_SIZE, PROTOCOL_VERSION};
use crate::transfer::stream::{
    read_json, write_end_marker, write_file_header, write_json,
};
use crate::transfer::{TransferError, TransferResult};

pub struct SendItem {
    pub local_path: PathBuf,
    pub rel_path: String,
    pub size: u64,
}

pub struct SendRequest {
    pub id: String,
    pub peer_addr: String,
    pub device_name: String,
    pub items: Vec<SendItem>,
}

const PROGRESS_INTERVAL: Duration = Duration::from_millis(50);

pub async fn run_send(app: AppHandle, req: SendRequest) -> TransferResult<()> {
    let started_at = Instant::now();
    let total_bytes: u64 = req.items.iter().map(|i| i.size).sum();
    let file_count = req.items.len() as u64;
    let id = req.id.clone();

    let stream = TcpStream::connect(&req.peer_addr).await?;
    stream.set_nodelay(true)?;
    let (read_half, write_half) = stream.into_split();
    let mut reader = BufReader::with_capacity(CHUNK_SIZE, read_half);
    let mut writer = BufWriter::with_capacity(CHUNK_SIZE, write_half);

    let hello = Hello {
        version: PROTOCOL_VERSION,
        device_name: req.device_name.clone(),
        os: current_os().to_string(),
        file_count,
        total_bytes,
    };
    write_json(&mut writer, &hello).await?;
    writer.flush().await?;

    let ack: HelloAck = read_json(&mut reader).await?;
    if !ack.accept {
        let reason = ack.reason.unwrap_or_else(|| "rejected".into());
        let _ = app.emit(
            EVT_TRANSFER_COMPLETED,
            TransferCompleted {
                id: id.clone(),
                direction: "send",
                success: false,
                error: Some(format!("Karşı taraf reddetti: {reason}")),
                elapsed_ms: started_at.elapsed().as_millis() as u64,
                total_bytes: 0,
            },
        );
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
    let mut buf = vec![0u8; CHUNK_SIZE];

    for item in &req.items {
        write_file_header(&mut writer, &item.rel_path, item.size).await?;
        let mut hasher = StreamHasher::new();
        let mut file_done: u64 = 0;
        let mut file = BufReader::with_capacity(CHUNK_SIZE, File::open(&item.local_path).await?);
        loop {
            let n = file.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            writer.write_all(&buf[..n]).await?;
            hasher.update(&buf[..n]);
            file_done += n as u64;
            total_done += n as u64;
            if last_emit.elapsed() >= PROGRESS_INTERVAL {
                last_emit = Instant::now();
                let _ = app.emit(
                    EVT_TRANSFER_PROGRESS,
                    TransferProgress {
                        id: id.clone(),
                        direction: "send",
                        current_file: item.rel_path.clone(),
                        current_bytes_done: file_done,
                        current_bytes_total: item.size,
                        total_bytes_done: total_done,
                        total_bytes,
                        files_done,
                        files_total: file_count,
                    },
                );
            }
        }
        let hash = hasher.finalize();
        writer.write_all(&hash).await?;
        files_done += 1;
        let _ = app.emit(
            EVT_TRANSFER_PROGRESS,
            TransferProgress {
                id: id.clone(),
                direction: "send",
                current_file: item.rel_path.clone(),
                current_bytes_done: file_done,
                current_bytes_total: item.size,
                total_bytes_done: total_done,
                total_bytes,
                files_done,
                files_total: file_count,
            },
        );
    }

    write_end_marker(&mut writer).await?;
    writer.flush().await?;
    writer.shutdown().await.ok();

    let _ = app.emit(
        EVT_TRANSFER_COMPLETED,
        TransferCompleted {
            id,
            direction: "send",
            success: true,
            error: None,
            elapsed_ms: started_at.elapsed().as_millis() as u64,
            total_bytes,
        },
    );
    Ok(())
}

/// Build a one-file SendRequest from an absolute local file path.
/// The rel_path becomes the file's basename (forward-slash safe).
pub fn build_single_file_request(
    id: String,
    peer_addr: String,
    device_name: String,
    file: &Path,
) -> std::io::Result<SendRequest> {
    let metadata = std::fs::metadata(file)?;
    let size = metadata.len();
    let rel_path = file
        .file_name()
        .map(|s| s.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| "file.bin".to_string());
    Ok(SendRequest {
        id,
        peer_addr,
        device_name,
        items: vec![SendItem {
            local_path: file.to_path_buf(),
            rel_path,
            size,
        }],
    })
}
