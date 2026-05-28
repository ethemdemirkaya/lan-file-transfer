//! Persistent user settings (device name, default receive directory).
//! Stored as `settings.json` under the OS app-data directory. The 6-digit
//! pairing code is in-memory only — it lives for the current app session
//! and rotates on demand.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSettings {
    pub device_name: String,
    pub save_dir: String,
    pub configured: bool,
}

pub fn load(dir: &Path) -> UserSettings {
    let path = dir.join("settings.json");
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => UserSettings::default(),
    }
}

pub fn save(dir: &Path, s: &UserSettings) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join("settings.json");
    let bytes = serde_json::to_vec_pretty(s).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
    })?;
    std::fs::write(path, bytes)
}

/// 6-digit numeric pairing code, padded with leading zeros.
pub fn random_code() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
        .unwrap_or(0);
    // xorshift for a little extra mixing — this is not security-sensitive,
    // just a per-session token to avoid accidental cross-talk.
    let mut x = nanos.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(1);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    let n = (x % 1_000_000) as u32;
    format!("{n:06}")
}

#[allow(dead_code)]
pub fn default_save_dir(home: Option<&Path>) -> PathBuf {
    home.map(|h| h.join("Downloads").join("LanBlaze"))
        .unwrap_or_else(|| PathBuf::from("LanBlaze"))
}
