mod disk;
mod discovery;
mod events;
mod hash;
mod history;
mod protocol;
mod settings;
mod state;
mod transfer;

use std::path::PathBuf;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_autostart::{ManagerExt as AutostartManagerExt, MacosLauncher};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use crate::discovery::Discovery;
use crate::events::{ReceiverReady, EVT_RECEIVER_READY};
use crate::history::{History, HistoryRecord};
use crate::protocol::{current_os, DEFAULT_PORT};
use crate::settings::{random_code, ThemePref, TrustedDevice, UserSettings};
use crate::state::{AppState, UserDecision};
use crate::transfer::{
    receiver::start_receiver,
    sender::{build_paths_request, run_send},
};

#[tauri::command]
fn ping(name: &str) -> String {
    format!("pong from Rust, hello {name}")
}

#[tauri::command]
fn get_local_ip() -> Option<String> {
    local_ip_address::local_ip().ok().map(|ip| ip.to_string())
}

#[tauri::command]
fn get_default_port() -> u16 {
    DEFAULT_PORT
}

#[tauri::command]
fn get_session(state: State<'_, AppState>) -> serde_json::Value {
    let session = state.session.lock().unwrap();
    let receiver = state.receiver.lock().unwrap();
    serde_json::json!({
        "settings": session.settings,
        "authCode": session.auth_code,
        "receiverPort": if receiver.running { Some(receiver.port) } else { None },
        "receiverRunning": receiver.running,
        "localIp": local_ip_address::local_ip().ok().map(|i| i.to_string()),
        "defaultPort": DEFAULT_PORT,
    })
}

#[tauri::command]
fn save_settings(
    state: State<'_, AppState>,
    device_name: String,
    save_dir: String,
) -> Result<UserSettings, String> {
    let trimmed_name = device_name.trim().to_string();
    if trimmed_name.is_empty() {
        return Err("Device name cannot be empty.".into());
    }
    let dir_path = PathBuf::from(&save_dir);
    if !dir_path.is_dir() {
        std::fs::create_dir_all(&dir_path)
            .map_err(|e| format!("Couldn't create folder: {e}"))?;
    }
    let mut session = state.session.lock().unwrap();
    session.settings.device_name = trimmed_name;
    session.settings.save_dir = dir_path.to_string_lossy().into_owned();
    session.settings.configured = true;
    settings::save(&session.settings_dir, &session.settings)
        .map_err(|e| format!("Couldn't save settings: {e}"))?;
    Ok(session.settings.clone())
}

#[tauri::command]
fn set_theme(state: State<'_, AppState>, theme: String) -> Result<UserSettings, String> {
    let pref = match theme.as_str() {
        "light" => ThemePref::Light,
        "dark" => ThemePref::Dark,
        _ => ThemePref::System,
    };
    let mut s = state.session.lock().unwrap();
    s.settings.theme = pref;
    settings::save(&s.settings_dir, &s.settings).map_err(|e| e.to_string())?;
    Ok(s.settings.clone())
}

#[tauri::command]
fn set_sound_enabled(state: State<'_, AppState>, enabled: bool) -> Result<UserSettings, String> {
    let mut s = state.session.lock().unwrap();
    s.settings.sound_enabled = enabled;
    settings::save(&s.settings_dir, &s.settings).map_err(|e| e.to_string())?;
    Ok(s.settings.clone())
}

#[tauri::command]
fn set_notifications_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<UserSettings, String> {
    let mut s = state.session.lock().unwrap();
    s.settings.notifications_enabled = enabled;
    settings::save(&s.settings_dir, &s.settings).map_err(|e| e.to_string())?;
    Ok(s.settings.clone())
}

#[tauri::command]
fn set_close_to_tray(state: State<'_, AppState>, enabled: bool) -> Result<UserSettings, String> {
    let mut s = state.session.lock().unwrap();
    s.settings.close_to_tray = enabled;
    settings::save(&s.settings_dir, &s.settings).map_err(|e| e.to_string())?;
    Ok(s.settings.clone())
}

#[tauri::command]
fn set_auto_start(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<UserSettings, String> {
    let autostart = app.autolaunch();
    let res = if enabled {
        autostart.enable()
    } else {
        autostart.disable()
    };
    res.map_err(|e| format!("Autostart toggle failed: {e}"))?;
    let mut s = state.session.lock().unwrap();
    s.settings.auto_start = enabled;
    settings::save(&s.settings_dir, &s.settings).map_err(|e| e.to_string())?;
    Ok(s.settings.clone())
}

#[tauri::command]
fn trust_device(state: State<'_, AppState>, device_name: String) -> Result<UserSettings, String> {
    let mut s = state.session.lock().unwrap();
    s.settings.trusted_devices.insert(
        device_name.clone(),
        TrustedDevice {
            name: device_name,
            trusted_at: history::now_ms(),
        },
    );
    settings::save(&s.settings_dir, &s.settings).map_err(|e| e.to_string())?;
    Ok(s.settings.clone())
}

#[tauri::command]
fn untrust_device(state: State<'_, AppState>, device_name: String) -> Result<UserSettings, String> {
    let mut s = state.session.lock().unwrap();
    s.settings.trusted_devices.remove(&device_name);
    settings::save(&s.settings_dir, &s.settings).map_err(|e| e.to_string())?;
    Ok(s.settings.clone())
}

#[tauri::command]
fn get_history(state: State<'_, AppState>) -> History {
    let dir = state.session.lock().unwrap().settings_dir.clone();
    history::load(&dir)
}

#[tauri::command]
fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    let dir = state.session.lock().unwrap().settings_dir.clone();
    history::clear(&dir).map_err(|e| e.to_string())
}

#[tauri::command]
fn regenerate_code(state: State<'_, AppState>) -> String {
    let new_code = random_code();
    state.session.lock().unwrap().auth_code = new_code.clone();
    new_code
}

#[tauri::command]
fn refresh_discovery(app: AppHandle, discovery: State<'_, Discovery>) -> Result<(), String> {
    discovery.refresh(app);
    Ok(())
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    // Defence-in-depth: only allow http/https so a misbehaving frontend
    // can't talk us into running arbitrary commands via a `file://` or
    // protocol-scheme handler.
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Only http(s) URLs are allowed".into());
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &url])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn cancel_transfer(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let tx = state.cancel.lock().unwrap().remove(&id);
    if let Some(tx) = tx {
        let _ = tx.send(());
        Ok(())
    } else {
        // Either the transfer already finished or never started — both are
        // benign from the UI's point of view.
        Ok(())
    }
}

#[tauri::command]
fn show_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    Ok(())
}

#[tauri::command]
async fn ensure_receiver(
    app: AppHandle,
    state: State<'_, AppState>,
    discovery: State<'_, Discovery>,
) -> Result<u16, String> {
    {
        let r = state.receiver.lock().unwrap();
        if r.running {
            return Ok(r.port);
        }
    }
    let (save_dir, device_name) = {
        let s = state.session.lock().unwrap();
        if !s.settings.configured {
            return Err("Settings not configured.".into());
        }
        (
            PathBuf::from(&s.settings.save_dir),
            s.settings.device_name.clone(),
        )
    };
    if !save_dir.is_dir() {
        std::fs::create_dir_all(&save_dir)
            .map_err(|e| format!("Couldn't create receive folder: {e}"))?;
    }
    let handle = start_receiver(app.clone(), save_dir.clone(), DEFAULT_PORT)
        .await
        .map_err(|e| format!("Couldn't start receiver: {e}"))?;
    let actual_port = handle.port;
    {
        let mut r = state.receiver.lock().unwrap();
        r.running = true;
        r.port = actual_port;
        r.save_dir = Some(save_dir.clone());
        r.shutdown = Some(handle.shutdown);
    }
    if let Err(e) = discovery.advertise(&device_name, current_os(), actual_port) {
        tracing::warn!("mdns advertise failed: {e}");
    }
    let _ = app.emit(
        EVT_RECEIVER_READY,
        ReceiverReady {
            port: actual_port,
            local_ip: local_ip_address::local_ip().ok().map(|i| i.to_string()),
            save_dir: save_dir.to_string_lossy().into_owned(),
        },
    );
    Ok(actual_port)
}

#[tauri::command]
fn respond_incoming(
    state: State<'_, AppState>,
    id: String,
    accept: bool,
    override_save_dir: Option<String>,
) -> Result<(), String> {
    let pending = state.pending.lock().unwrap().remove(&id);
    let Some(pending) = pending else {
        return Err("Request not found or timed out.".into());
    };
    let decision = UserDecision {
        accept,
        override_save_dir: override_save_dir.map(PathBuf::from),
    };
    let _ = pending.tx.send(decision);
    Ok(())
}

#[tauri::command]
async fn send_paths(
    app: AppHandle,
    state: State<'_, AppState>,
    peer_ip: String,
    port: Option<u16>,
    auth_code: String,
    paths: Vec<String>,
) -> Result<String, String> {
    if paths.is_empty() {
        return Err("Pick at least one file or folder.".into());
    }
    if auth_code.trim().len() != 6 {
        return Err("Pairing code must be 6 digits.".into());
    }
    let device_name = state.session.lock().unwrap().settings.device_name.clone();
    let port = port.unwrap_or(DEFAULT_PORT);
    let id = Uuid::new_v4().to_string();
    let peer_addr = format!("{peer_ip}:{port}");
    let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let req = build_paths_request(
        id.clone(),
        peer_addr,
        device_name,
        auth_code.trim().to_string(),
        &path_bufs,
    )
    .map_err(|e| format!("Couldn't read paths: {e}"))?;

    if req.items.is_empty() {
        return Err("No files in the selection.".into());
    }

    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_send(app, req).await {
            tracing::error!("send failed: {e}");
        }
    });
    Ok(id)
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Show LanBlaze", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let _ = TrayIconBuilder::with_id("main-tray")
        .tooltip("LanBlaze")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .setup(|app| {
            let settings_dir = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let mut loaded = settings::load(&settings_dir);
            if loaded.device_name.is_empty() {
                loaded.device_name = state::hostname_best_effort();
            }
            if loaded.save_dir.is_empty() {
                if let Ok(download) = app.path().download_dir() {
                    loaded.save_dir = download
                        .join("LanBlaze")
                        .to_string_lossy()
                        .into_owned();
                }
            }
            let auth_code = random_code();
            app.manage(AppState::new(loaded, settings_dir, auth_code));

            let d = Discovery::new();
            d.start_browser(app.handle().clone());
            app.manage(d);

            #[cfg(target_os = "windows")]
            {
                if let Some(win) = app.get_webview_window("main") {
                    use window_vibrancy::apply_mica;
                    if let Err(e) = apply_mica(&win, None) {
                        tracing::warn!("apply_mica failed: {e}");
                    }
                }
            }

            if let Err(e) = build_tray(app.handle()) {
                tracing::warn!("tray init failed: {e}");
            }

            // Auto-start receiver if already configured.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let configured = {
                    let state: State<'_, AppState> = handle.state();
                    let v = state.session.lock().unwrap().settings.configured;
                    v
                };
                if configured {
                    let app2 = handle.clone();
                    let s2: State<'_, AppState> = app2.state();
                    let d2: State<'_, Discovery> = app2.state();
                    if let Err(e) = ensure_receiver(app2.clone(), s2, d2).await {
                        tracing::warn!("auto-start receiver failed: {e}");
                    }
                }
            });

            // Hide window on launch if started via autostart.
            let args: Vec<String> = std::env::args().collect();
            if args.iter().any(|a| a == "--autostart") {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // If close-to-tray is on, hide instead of quitting.
                let app = window.app_handle();
                let state: State<'_, AppState> = app.state();
                let close_to_tray = state.session.lock().unwrap().settings.close_to_tray;
                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            get_local_ip,
            get_default_port,
            get_session,
            save_settings,
            set_theme,
            set_sound_enabled,
            set_notifications_enabled,
            set_close_to_tray,
            set_auto_start,
            trust_device,
            untrust_device,
            get_history,
            clear_history,
            regenerate_code,
            ensure_receiver,
            respond_incoming,
            send_paths,
            show_main_window,
            refresh_discovery,
            cancel_transfer,
            open_external_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Silence the unused-import warning on platforms where Mica isn't applied.
#[allow(dead_code)]
fn _force_use_history() -> HistoryRecord {
    HistoryRecord {
        id: String::new(),
        direction: String::new(),
        peer: String::new(),
        bytes: 0,
        file_count: 0,
        elapsed_ms: 0,
        success: false,
        error: None,
        finished_at: 0,
    }
}
