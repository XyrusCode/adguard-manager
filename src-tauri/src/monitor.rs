use crate::storage::Storage;
use crate::types::{ConnectionEntry, InterfaceSample, MonitorPayload};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::{Networks, Pid, ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter};
use tracing::error;

const POLL_SECS: u64 = 2;
const SAVE_EVERY: u64 = 10;

pub fn start(handle: AppHandle, storage: Arc<Mutex<Storage>>) {
    std::thread::spawn(move || {
        let mut nets = Networks::new_with_refreshed_list();
        let mut sys = System::new_all();
        let mut last_save = Instant::now();

        loop {
            std::thread::sleep(Duration::from_secs(POLL_SECS));

            nets.refresh();
            sys.refresh_processes(ProcessesToUpdate::All, true);

            let interfaces: Vec<InterfaceSample> = nets
                .iter()
                .map(|(name, data)| InterfaceSample {
                    interface_name: name.to_string(),
                    timestamp: chrono::Utc::now(),
                    bytes_received: data.total_received(),
                    bytes_sent: data.total_transmitted(),
                    rx_rate: data.received() as f64 / POLL_SECS as f64,
                    tx_rate: data.transmitted() as f64 / POLL_SECS as f64,
                })
                .collect();

            let connections = collect_connections(&sys);

            if last_save.elapsed().as_secs() >= SAVE_EVERY {
                if let Ok(st) = storage.lock() {
                    for s in &interfaces {
                        if let Err(e) = st.insert_sample(s) {
                            error!("insert_sample: {}", e);
                        }
                    }
                }
                last_save = Instant::now();
            }

            let payload = MonitorPayload {
                interfaces,
                connections,
            };

            if let Err(e) = handle.emit("network-update", &payload) {
                error!("emit: {}", e);
            }
        }
    });
}

fn collect_connections(sys: &System) -> Vec<ConnectionEntry> {
    #[cfg(target_os = "windows")]
    return windows_conns(sys);
    #[cfg(target_os = "linux")]
    return linux_conns(sys);
    #[cfg(target_os = "macos")]
    return macos_conns(sys);
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    return vec![];
}

fn proc_name(sys: &System, pid: u32) -> String {
    sys.process(Pid::from(pid as usize))
        .map(|p| p.name().to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("PID {}", pid))
}

#[cfg(target_os = "windows")]
fn windows_conns(sys: &System) -> Vec<ConnectionEntry> {
    use std::process::Command;
    let out = match Command::new("netstat").args(["-noa"]).output() {
        Ok(o) => o,
        Err(e) => {
            error!("netstat: {}", e);
            return vec![];
        }
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut entries = Vec::new();
    for line in text.lines().skip(4) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        match (cols.first().copied(), cols.len()) {
            (Some("TCP"), 5) => {
                let pid: u32 = cols[4].parse().unwrap_or(0);
                entries.push(ConnectionEntry {
                    pid,
                    process_name: proc_name(sys, pid),
                    local_addr: cols[1].to_string(),
                    remote_addr: cols[2].to_string(),
                    protocol: "TCP".into(),
                    state: cols[3].to_string(),
                });
            }
            (Some("UDP"), 4) => {
                let pid: u32 = cols[3].parse().unwrap_or(0);
                entries.push(ConnectionEntry {
                    pid,
                    process_name: proc_name(sys, pid),
                    local_addr: cols[1].to_string(),
                    remote_addr: cols[2].to_string(),
                    protocol: "UDP".into(),
                    state: String::new(),
                });
            }
            _ => {}
        }
    }
    entries
}

#[cfg(target_os = "linux")]
fn linux_conns(sys: &System) -> Vec<ConnectionEntry> {
    use std::process::Command;
    let out = match Command::new("ss").args(["-tunpa"]).output() {
        Ok(o) => o,
        Err(e) => {
            error!("ss: {}", e);
            return vec![];
        }
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut entries = Vec::new();
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 6 {
            continue;
        }
        let mut pid = 0u32;
        if let Some(proc_part) = cols.get(6) {
            if let Some(pos) = proc_part.find("pid=") {
                let rest = &proc_part[pos + 4..];
                let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
                pid = rest[..end].parse().unwrap_or(0);
            }
        }
        entries.push(ConnectionEntry {
            pid,
            process_name: if pid > 0 { proc_name(sys, pid) } else { String::new() },
            local_addr: cols[4].to_string(),
            remote_addr: cols[5].to_string(),
            protocol: cols[0].to_string(),
            state: cols[1].to_string(),
        });
    }
    entries
}

#[cfg(target_os = "macos")]
fn macos_conns(sys: &System) -> Vec<ConnectionEntry> {
    use std::process::Command;
    let out = match Command::new("netstat").args(["-vanp", "tcp"]).output() {
        Ok(o) => o,
        Err(e) => {
            error!("netstat: {}", e);
            return vec![];
        }
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut entries = Vec::new();
    for line in text.lines().skip(2) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 10 {
            continue;
        }
        let pid: u32 = cols.last().and_then(|s| s.parse().ok()).unwrap_or(0);
        entries.push(ConnectionEntry {
            pid,
            process_name: proc_name(sys, pid),
            local_addr: cols[3].to_string(),
            remote_addr: cols[4].to_string(),
            protocol: cols[0].to_string(),
            state: cols[5].to_string(),
        });
    }
    entries
}
