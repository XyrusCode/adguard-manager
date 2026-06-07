use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAdapter {
    pub name: String,
    pub adguard_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceSample {
    pub interface_name: String,
    pub timestamp: DateTime<Utc>,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub rx_rate: f64,
    pub tx_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionEntry {
    pub pid: u32,
    pub process_name: String,
    pub local_addr: String,
    pub remote_addr: String,
    pub protocol: String,
    pub state: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MonitorPayload {
    pub interfaces: Vec<InterfaceSample>,
    pub connections: Vec<ConnectionEntry>,
}
