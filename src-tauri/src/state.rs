use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use tokio::sync::oneshot;

use crate::settings::UserSettings;

pub struct AppState {
    pub receiver: Mutex<ReceiverState>,
    pub session: Mutex<SessionState>,
    pub pending: Mutex<HashMap<String, PendingDecision>>,
}

pub struct PendingDecision {
    pub tx: oneshot::Sender<UserDecision>,
}

pub struct UserDecision {
    pub accept: bool,
    pub override_save_dir: Option<PathBuf>,
}

#[derive(Default)]
pub struct ReceiverState {
    pub running: bool,
    pub port: u16,
    pub save_dir: Option<PathBuf>,
    pub shutdown: Option<oneshot::Sender<()>>,
}

pub struct SessionState {
    pub settings: UserSettings,
    pub auth_code: String,
    pub settings_dir: PathBuf,
}

impl AppState {
    pub fn new(settings: UserSettings, settings_dir: PathBuf, auth_code: String) -> Self {
        Self {
            receiver: Mutex::new(ReceiverState::default()),
            session: Mutex::new(SessionState {
                settings,
                auth_code,
                settings_dir,
            }),
            pending: Mutex::new(HashMap::new()),
        }
    }
}

pub fn hostname_best_effort() -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_else(|| "LanBlaze".to_string())
}
