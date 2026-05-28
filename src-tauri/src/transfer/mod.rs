pub mod receiver;
pub mod sender;
pub mod stream;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransferError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("rejected by peer: {0}")]
    Rejected(String),
    #[error("hash mismatch for {0}")]
    HashMismatch(String),
    #[error("unsafe path: {0}")]
    UnsafePath(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("cancelled")]
    Cancelled,
}

pub type TransferResult<T> = Result<T, TransferError>;
