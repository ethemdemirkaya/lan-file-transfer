//! Persistent transfer history. Stored as `history.json` in the OS
//! app-config dir next to settings.json. Bounded to MAX_RECORDS most
//! recent entries to keep the file small and the read cheap on startup.

use std::path::Path;

use serde::{Deserialize, Serialize};

const MAX_RECORDS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub id: String,
    pub direction: String, // "send" | "recv"
    pub peer: String,
    pub bytes: u64,
    pub file_count: u64,
    pub elapsed_ms: u64,
    pub success: bool,
    pub error: Option<String>,
    /// Unix milliseconds.
    pub finished_at: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct History {
    pub records: Vec<HistoryRecord>,
}

pub fn load(dir: &Path) -> History {
    let path = dir.join("history.json");
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => History::default(),
    }
}

pub fn save(dir: &Path, h: &History) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join("history.json");
    let bytes = serde_json::to_vec_pretty(h).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
    })?;
    std::fs::write(path, bytes)
}

pub fn push(dir: &Path, record: HistoryRecord) -> std::io::Result<History> {
    let mut h = load(dir);
    h.records.insert(0, record);
    if h.records.len() > MAX_RECORDS {
        h.records.truncate(MAX_RECORDS);
    }
    save(dir, &h)?;
    Ok(h)
}

pub fn clear(dir: &Path) -> std::io::Result<()> {
    save(dir, &History::default())
}

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
