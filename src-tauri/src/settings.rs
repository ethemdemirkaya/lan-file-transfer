//! Persistent user settings (device name, default receive directory,
//! trusted devices, theme, sound, auto-start). Stored as `settings.json`
//! under the OS app-config directory. The 6-digit pairing code is in-memory
//! only — it lives for the current app session and rotates on demand.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Theme preference. "system" follows the OS, the others force a side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePref {
    System,
    Light,
    Dark,
}

impl Default for ThemePref {
    fn default() -> Self {
        ThemePref::System
    }
}

/// A peer the user marked as trusted — their transfers skip the
/// accept/reject dialog and land directly in the default save dir.
/// Keyed by the sender's device name; an identity that's easy to spoof,
/// but on a trusted LAN that's the same risk surface as the pairing
/// code itself. A future TLS-pinned identity would replace this key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedDevice {
    pub name: String,
    pub trusted_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSettings {
    pub device_name: String,
    pub save_dir: String,
    pub configured: bool,
    #[serde(default)]
    pub theme: ThemePref,
    #[serde(default = "default_true")]
    pub sound_enabled: bool,
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub close_to_tray: bool,
    #[serde(default)]
    pub trusted_devices: HashMap<String, TrustedDevice>,
}

fn default_true() -> bool {
    true
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
