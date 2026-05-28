//! Low-level framing helpers shared by sender and receiver.
//!
//! All multi-byte integers are little-endian. Functions are async and operate
//! on any AsyncRead / AsyncWrite. Buffering is the caller's responsibility
//! (use `BufReader` / `BufWriter` with at least `CHUNK_SIZE`).

use serde::{de::DeserializeOwned, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::transfer::{TransferError, TransferResult};

const MAX_JSON_LEN: u32 = 8 * 1024 * 1024; // 8 MiB sanity cap on JSON frames

/// Write a length-prefixed JSON message: `u32 LE len` + UTF-8 JSON bytes.
pub async fn write_json<W, T>(w: &mut W, value: &T) -> TransferResult<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() as u64 > MAX_JSON_LEN as u64 {
        return Err(TransferError::Protocol("json frame too large".into()));
    }
    w.write_all(&(bytes.len() as u32).to_le_bytes()).await?;
    w.write_all(&bytes).await?;
    Ok(())
}

/// Read a length-prefixed JSON message produced by [`write_json`].
pub async fn read_json<R, T>(r: &mut R) -> TransferResult<T>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf);
    if len > MAX_JSON_LEN {
        return Err(TransferError::Protocol(format!(
            "json frame too large: {len}"
        )));
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf).await?;
    let value = serde_json::from_slice::<T>(&buf)?;
    Ok(value)
}

/// Write a per-file header: `u16 path_len` + path bytes + `u64 file_size`
/// + `u8 resumable`. `resumable == 1` means the receiver will reply with a
/// `u64` resume offset; `0` means it won't and the sender streams the whole
/// file. See protocol.rs for the threshold rationale.
pub async fn write_file_header<W>(
    w: &mut W,
    rel_path: &str,
    file_size: u64,
    resumable: bool,
) -> TransferResult<()>
where
    W: AsyncWrite + Unpin,
{
    let path_bytes = rel_path.as_bytes();
    if path_bytes.len() > u16::MAX as usize {
        return Err(TransferError::Protocol("path too long".into()));
    }
    if path_bytes.is_empty() {
        return Err(TransferError::Protocol("empty path".into()));
    }
    w.write_all(&(path_bytes.len() as u16).to_le_bytes()).await?;
    w.write_all(path_bytes).await?;
    w.write_all(&file_size.to_le_bytes()).await?;
    w.write_all(&[u8::from(resumable)]).await?;
    Ok(())
}

/// Receiver → sender: how many bytes already exist on disk for a resumable
/// file. Sender must skip exactly this many bytes from the start of the
/// file before writing the rest, but still hashes the whole file.
pub async fn write_resume_offset<W>(w: &mut W, offset: u64) -> TransferResult<()>
where
    W: AsyncWrite + Unpin,
{
    w.write_all(&offset.to_le_bytes()).await?;
    Ok(())
}

pub async fn read_resume_offset<R>(r: &mut R) -> TransferResult<u64>
where
    R: AsyncRead + Unpin,
{
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf).await?;
    Ok(u64::from_le_bytes(buf))
}

/// Write end-of-stream marker: `u16 0`.
pub async fn write_end_marker<W>(w: &mut W) -> TransferResult<()>
where
    W: AsyncWrite + Unpin,
{
    w.write_all(&0u16.to_le_bytes()).await?;
    Ok(())
}

/// Decoded per-file header.
pub struct FileHeader {
    pub rel_path: String,
    pub file_size: u64,
    pub resumable: bool,
}

/// Read a per-file header. Returns `None` at end-of-stream (path_len == 0).
pub async fn read_file_header<R>(r: &mut R) -> TransferResult<Option<FileHeader>>
where
    R: AsyncRead + Unpin,
{
    let mut len_buf = [0u8; 2];
    r.read_exact(&mut len_buf).await?;
    let path_len = u16::from_le_bytes(len_buf);
    if path_len == 0 {
        return Ok(None);
    }
    let mut path_bytes = vec![0u8; path_len as usize];
    r.read_exact(&mut path_bytes).await?;
    let rel_path = String::from_utf8(path_bytes)
        .map_err(|_| TransferError::Protocol("non-utf8 path".into()))?;
    let mut size_buf = [0u8; 8];
    r.read_exact(&mut size_buf).await?;
    let file_size = u64::from_le_bytes(size_buf);
    let mut flag = [0u8; 1];
    r.read_exact(&mut flag).await?;
    Ok(Some(FileHeader {
        rel_path,
        file_size,
        resumable: flag[0] != 0,
    }))
}
