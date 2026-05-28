use std::path::PathBuf;
use std::sync::Mutex;

use tokio::sync::oneshot;

#[derive(Default)]
pub struct AppState {
    pub receiver: Mutex<ReceiverState>,
    pub settings: Mutex<Settings>,
}

#[derive(Default)]
pub struct ReceiverState {
    pub running: bool,
    pub port: u16,
    pub save_dir: Option<PathBuf>,
    pub shutdown: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub device_name: String,
}

impl Default for Settings {
    fn default() -> Self {
        let device_name = hostname_best_effort();
        Self { device_name }
    }
}

fn hostname_best_effort() -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_else(|| "Unknown".to_string())
}
