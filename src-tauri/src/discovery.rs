//! mDNS/DNS-SD discovery. Advertises our receiver as `_lanblaze._tcp.local.`
//! and browses for peers running the same service.
//!
//! Browser runs for the lifetime of the app; advertisement is toggled when the
//! receiver starts/stops. Self-discovery is filtered out by instance name.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tracing::{info, warn};

use crate::protocol::PROTOCOL_VERSION;

pub const SERVICE_TYPE: &str = "_lanblaze._tcp.local.";
pub const EVT_PEER_ADDED: &str = "peer://added";
pub const EVT_PEER_REMOVED: &str = "peer://removed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    pub instance: String,
    pub device_name: String,
    pub os: String,
    pub version: String,
    pub addresses: Vec<String>,
    pub port: u16,
}

/// Wrap the mDNS daemon; if init fails (e.g. no network adapters) the struct
/// stays alive in a disabled state so commands keep working.
#[derive(Clone)]
pub struct Discovery {
    inner: Arc<DiscoveryInner>,
}

struct DiscoveryInner {
    daemon: Option<ServiceDaemon>,
    self_instance: Mutex<Option<String>>,
}

impl Discovery {
    pub fn new() -> Self {
        let daemon = match ServiceDaemon::new() {
            Ok(d) => Some(d),
            Err(e) => {
                warn!("mdns init failed, discovery disabled: {e}");
                None
            }
        };
        Self {
            inner: Arc::new(DiscoveryInner {
                daemon,
                self_instance: Mutex::new(None),
            }),
        }
    }

    pub fn start_browser(&self, app: AppHandle) {
        let Some(daemon) = self.inner.daemon.as_ref() else {
            return;
        };
        let recv = match daemon.browse(SERVICE_TYPE) {
            Ok(r) => r,
            Err(e) => {
                warn!("mdns browse failed: {e}");
                return;
            }
        };
        let inner = self.inner.clone();
        std::thread::spawn(move || {
            while let Ok(event) = recv.recv() {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        let instance = info.get_fullname().to_string();
                        if let Some(me) = inner.self_instance.lock().unwrap().as_ref() {
                            if &instance == me {
                                continue;
                            }
                        }
                        let device_name = txt_str(&info, "name")
                            .unwrap_or_else(|| info.get_hostname().to_string());
                        let os = txt_str(&info, "os").unwrap_or_else(|| "unknown".to_string());
                        let version = txt_str(&info, "version").unwrap_or_else(|| "0".to_string());
                        let addresses: Vec<String> = info
                            .get_addresses()
                            .iter()
                            .map(|a| a.to_string())
                            .collect();
                        let peer = Peer {
                            instance: instance.clone(),
                            device_name,
                            os,
                            version,
                            addresses,
                            port: info.get_port(),
                        };
                        let _ = app.emit(EVT_PEER_ADDED, &peer);
                    }
                    ServiceEvent::ServiceRemoved(_type, fullname) => {
                        if let Some(me) = inner.self_instance.lock().unwrap().as_ref() {
                            if &fullname == me {
                                continue;
                            }
                        }
                        let _ = app.emit(EVT_PEER_REMOVED, &fullname);
                    }
                    _ => {}
                }
            }
        });
    }

    pub fn advertise(
        &self,
        device_name: &str,
        os: &str,
        port: u16,
    ) -> Result<(), mdns_sd::Error> {
        let Some(daemon) = self.inner.daemon.as_ref() else {
            return Ok(());
        };
        let local_ip = local_ip_address::local_ip()
            .ok()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| "127.0.0.1".to_string());

        let safe_instance = sanitize_instance(device_name);
        let hostname = format!("{safe_instance}.local.");
        let mut props: HashMap<String, String> = HashMap::new();
        props.insert("name".into(), device_name.to_string());
        props.insert("os".into(), os.to_string());
        props.insert("version".into(), PROTOCOL_VERSION.to_string());
        props.insert("port".into(), port.to_string());

        let info = ServiceInfo::new(
            SERVICE_TYPE,
            &safe_instance,
            &hostname,
            local_ip.as_str(),
            port,
            Some(props),
        )?;

        let fullname = info.get_fullname().to_string();
        info!("mdns advertising as {fullname} on {local_ip}:{port}");
        daemon.register(info)?;
        *self.inner.self_instance.lock().unwrap() = Some(fullname);
        Ok(())
    }

    pub fn unadvertise(&self) {
        let Some(daemon) = self.inner.daemon.as_ref() else {
            return;
        };
        let name = self.inner.self_instance.lock().unwrap().take();
        if let Some(name) = name {
            let _ = daemon.unregister(&name);
        }
    }
}

fn txt_str(info: &ServiceInfo, key: &str) -> Option<String> {
    info.get_property(key).map(|p| p.val_str().to_string())
}

fn sanitize_instance(name: &str) -> String {
    // mDNS instance names should not contain dots; replace with '-'.
    let cleaned: String = name
        .chars()
        .map(|c| if c == '.' || c == '/' || c == '\\' { '-' } else { c })
        .collect();
    if cleaned.is_empty() {
        "lanblaze".to_string()
    } else {
        cleaned
    }
}
