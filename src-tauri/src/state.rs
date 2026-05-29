use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::{oneshot, Notify};

use crate::settings::UserSettings;

/// Pause / resume signal for an in-flight transfer. Either side of a
/// transfer registers one of these and the worker task awaits
/// `notify.notified()` whenever `paused` is true. Local-only — the peer
/// notices indirectly via TCP back-pressure.
#[derive(Default)]
pub struct PauseToken {
    pub paused: AtomicBool,
    pub notify: Notify,
}

impl PauseToken {
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }
    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
        self.notify.notify_waiters();
    }
    pub async fn wait_if_paused(&self) {
        while self.paused.load(Ordering::SeqCst) {
            self.notify.notified().await;
        }
    }
}

pub struct AppState {
    pub receiver: Mutex<ReceiverState>,
    pub session: Mutex<SessionState>,
    pub pending: Mutex<HashMap<String, PendingDecision>>,
    /// Per-transfer cancel handles. Either side of a transfer (sender or
    /// receiver) can register an oneshot here keyed by transfer id; the
    /// UI's cancel button sends () through it and the worker task wakes
    /// up its `tokio::select!` to abort cleanly.
    pub cancel: Mutex<HashMap<String, oneshot::Sender<()>>>,
    /// Per-transfer pause tokens. Worker tasks `.wait_if_paused().await`
    /// at every chunk boundary; the UI flips `paused` and the worker
    /// blocks until resume.
    pub pause: Mutex<HashMap<String, Arc<PauseToken>>>,
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
            cancel: Mutex::new(HashMap::new()),
            pause: Mutex::new(HashMap::new()),
        }
    }
}

pub fn hostname_best_effort() -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_else(|| "LanBlaze".to_string())
}
