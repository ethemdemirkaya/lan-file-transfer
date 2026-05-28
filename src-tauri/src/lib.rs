mod discovery;
mod events;
mod hash;
mod protocol;
mod settings;
mod state;
mod transfer;

use std::path::PathBuf;

use tauri::{AppHandle, Emitter, Manager, State};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use crate::discovery::Discovery;
use crate::events::{ReceiverReady, EVT_RECEIVER_READY};
use crate::protocol::{current_os, DEFAULT_PORT};
use crate::settings::{random_code, UserSettings};
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
        return Err("Cihaz adı boş olamaz.".into());
    }
    let dir_path = PathBuf::from(&save_dir);
    if !dir_path.is_dir() {
        std::fs::create_dir_all(&dir_path)
            .map_err(|e| format!("Klasör oluşturulamadı: {e}"))?;
    }
    let mut session = state.session.lock().unwrap();
    session.settings.device_name = trimmed_name;
    session.settings.save_dir = dir_path.to_string_lossy().into_owned();
    session.settings.configured = true;
    settings::save(&session.settings_dir, &session.settings)
        .map_err(|e| format!("Ayarlar kaydedilemedi: {e}"))?;
    Ok(session.settings.clone())
}

#[tauri::command]
fn regenerate_code(state: State<'_, AppState>) -> String {
    let new_code = random_code();
    state.session.lock().unwrap().auth_code = new_code.clone();
    new_code
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
            return Err("Ayarlar tamamlanmadı.".into());
        }
        (PathBuf::from(&s.settings.save_dir), s.settings.device_name.clone())
    };
    if !save_dir.is_dir() {
        std::fs::create_dir_all(&save_dir)
            .map_err(|e| format!("Kayıt klasörü oluşturulamadı: {e}"))?;
    }
    let handle = start_receiver(app.clone(), save_dir.clone(), DEFAULT_PORT)
        .await
        .map_err(|e| format!("Alıcı başlatılamadı: {e}"))?;
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
        return Err("İstek bulunamadı veya süresi doldu.".into());
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
        return Err("En az bir dosya veya klasör seçin.".into());
    }
    if auth_code.trim().len() != 6 {
        return Err("Eşleştirme kodu 6 haneli olmalı.".into());
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
    .map_err(|e| format!("Yollar okunamadı: {e}"))?;

    if req.items.is_empty() {
        return Err("Seçimden gönderilecek dosya çıkmadı.".into());
    }

    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_send(app, req).await {
            tracing::error!("send failed: {e}");
        }
    });
    Ok(id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Settings live in OS app-data dir.
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

            // Auto-start receiver if already configured.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state: State<'_, AppState> = handle.state();
                let configured = state.session.lock().unwrap().settings.configured;
                drop(state);
                if configured {
                    let app2 = handle.clone();
                    let s2: State<'_, AppState> = app2.state();
                    let d2: State<'_, Discovery> = app2.state();
                    if let Err(e) = ensure_receiver(app2.clone(), s2, d2).await {
                        tracing::warn!("auto-start receiver failed: {e}");
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            get_local_ip,
            get_default_port,
            get_session,
            save_settings,
            regenerate_code,
            ensure_receiver,
            respond_incoming,
            send_paths,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
