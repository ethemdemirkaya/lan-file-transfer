use serde::Serialize;

pub const EVT_TRANSFER_STARTED: &str = "transfer://started";
pub const EVT_TRANSFER_PROGRESS: &str = "transfer://progress";
pub const EVT_TRANSFER_COMPLETED: &str = "transfer://completed";
pub const EVT_RECEIVER_READY: &str = "receiver://ready";
pub const EVT_INCOMING_REQUEST: &str = "incoming://request";
pub const EVT_INCOMING_CANCELLED: &str = "incoming://cancelled";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferStarted {
    pub id: String,
    pub direction: &'static str,
    pub peer: String,
    pub file_count: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub id: String,
    pub direction: &'static str,
    pub current_file: String,
    pub current_bytes_done: u64,
    pub current_bytes_total: u64,
    pub total_bytes_done: u64,
    pub total_bytes: u64,
    pub files_done: u64,
    pub files_total: u64,
    /// Instantaneous MB/s on the network side (TCP read for receivers,
    /// disk read + socket write for senders), computed over the last
    /// emit interval. Always present.
    pub instant_mbps_network: f64,
    /// Receiver-only: instantaneous MB/s being written to the local disk.
    /// Sender always sets this to 0; the UI should hide it for sends.
    pub instant_mbps_disk: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferCompleted {
    pub id: String,
    pub direction: &'static str,
    pub success: bool,
    pub error: Option<String>,
    pub elapsed_ms: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiverReady {
    pub port: u16,
    pub local_ip: Option<String>,
    pub save_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingRequest {
    pub id: String,
    pub peer: String,
    pub device_name: String,
    pub os: String,
    pub file_count: u64,
    pub total_bytes: u64,
}
