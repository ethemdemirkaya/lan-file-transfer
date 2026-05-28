//! Wire format and handshake types.
//!
//! All integers are little-endian. A single TCP connection carries the whole
//! transfer:
//!
//! 1. [u32 hello_len][HELLO JSON]
//! 2. [u32 ack_len][HELLO_ACK JSON]
//! 3. Repeated per file:
//!    [u16 path_len][path UTF-8 (forward-slash separated)]
//!    [u64 file_size]
//!    [file_size bytes: file contents]
//!    [32 bytes: blake3 of contents]
//! 4. [u16 path_len = 0]  -- end-of-stream marker.

use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;
pub const DEFAULT_PORT: u16 = 47813;
pub const CHUNK_SIZE: usize = 256 * 1024;
pub const HASH_LEN: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub version: u32,
    pub device_name: String,
    pub os: String,
    pub file_count: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloAck {
    pub accept: bool,
    pub reason: Option<String>,
}

pub fn current_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "other"
    }
}
