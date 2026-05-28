mod events;
mod hash;
mod protocol;
mod state;
mod transfer;

use std::path::PathBuf;

use tauri::{AppHandle, Emitter, Manager, State};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use crate::events::{ReceiverReady, EVT_RECEIVER_READY};
use crate::protocol::DEFAULT_PORT;
use crate::state::AppState;
use crate::transfer::{
    receiver::start_receiver,
    sender::{build_single_file_request, run_send},
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
fn get_device_name(state: State<'_, AppState>) -> String {
    state.settings.lock().unwrap().device_name.clone()
}

#[tauri::command]
async fn start_receiving(
    app: AppHandle,
    state: State<'_, AppState>,
    save_dir: String,
    port: Option<u16>,
) -> Result<u16, String> {
    {
        let r = state.receiver.lock().unwrap();
        if r.running {
            return Err("Alıcı zaten çalışıyor.".into());
        }
    }
    let dir = PathBuf::from(&save_dir);
    if !dir.is_dir() {
        return Err(format!("Klasör bulunamadı: {save_dir}"));
    }
    let handle = start_receiver(app.clone(), dir.clone(), port.unwrap_or(DEFAULT_PORT))
        .await
        .map_err(|e| format!("Alıcı başlatılamadı: {e}"))?;
    let actual_port = handle.port;
    {
        let mut r = state.receiver.lock().unwrap();
        r.running = true;
        r.port = actual_port;
        r.save_dir = Some(dir.clone());
        r.shutdown = Some(handle.shutdown);
    }
    let _ = app.emit(
        EVT_RECEIVER_READY,
        ReceiverReady {
            port: actual_port,
            local_ip: local_ip_address::local_ip().ok().map(|i| i.to_string()),
            save_dir: dir.to_string_lossy().into_owned(),
        },
    );
    Ok(actual_port)
}

#[tauri::command]
async fn stop_receiving(state: State<'_, AppState>) -> Result<(), String> {
    let shutdown = {
        let mut r = state.receiver.lock().unwrap();
        r.running = false;
        r.shutdown.take()
    };
    if let Some(tx) = shutdown {
        let _ = tx.send(());
    }
    Ok(())
}

#[tauri::command]
async fn send_file(
    app: AppHandle,
    state: State<'_, AppState>,
    peer_ip: String,
    port: Option<u16>,
    file_path: String,
) -> Result<String, String> {
    let device_name = state.settings.lock().unwrap().device_name.clone();
    let port = port.unwrap_or(DEFAULT_PORT);
    let id = Uuid::new_v4().to_string();
    let peer_addr = format!("{peer_ip}:{port}");
    let req = build_single_file_request(
        id.clone(),
        peer_addr,
        device_name,
        std::path::Path::new(&file_path),
    )
    .map_err(|e| format!("Dosya okunamadı: {e}"))?;

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
            app.manage(AppState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            get_local_ip,
            get_default_port,
            get_device_name,
            start_receiving,
            stop_receiving,
            send_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
