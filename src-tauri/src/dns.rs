use crate::types::NetworkAdapter;
use sysinfo::Networks;
use tracing::error;

pub const PRIMARY: &str = "94.140.14.14";
pub const SECONDARY: &str = "94.140.15.15";

pub fn list_adapters() -> Vec<NetworkAdapter> {
    let nets = Networks::new_with_refreshed_list();
    nets.iter()
        .map(|(name, _)| NetworkAdapter {
            adguard_enabled: is_adguard_set(name),
            name: name.to_string(),
        })
        .collect()
}

pub fn enable(adapter: &str) -> Result<(), String> {
    platform::enable(adapter)
}

pub fn disable(adapter: &str) -> Result<(), String> {
    platform::disable(adapter)
}

fn is_adguard_set(adapter: &str) -> bool {
    platform::current_servers(adapter)
        .map(|s| s.iter().any(|ip| ip == PRIMARY || ip == SECONDARY))
        .unwrap_or(false)
}

// ----- Windows ---------------------------------------------------------------
#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::process::Command;

    pub fn current_servers(adapter: &str) -> Result<Vec<String>, String> {
        let out = Command::new("netsh")
            .args(["interface", "ip", "show", "dnsservers", &format!("name={}", adapter)])
            .output()
            .map_err(|e| e.to_string())?;
        Ok(extract_ips(&String::from_utf8_lossy(&out.stdout)))
    }

    fn extract_ips(text: &str) -> Vec<String> {
        let mut ips = Vec::new();
        let mut active = false;
        for line in text.lines() {
            let t = line.trim();
            if t.contains("DNS Servers") || t.contains("DNS servers") {
                active = true;
                if let Some(ip) = t.split(':').nth(1).map(str::trim).filter(|s| is_ip(s)) {
                    ips.push(ip.to_string());
                }
            } else if active {
                if is_ip(t) {
                    ips.push(t.to_string());
                } else if !t.is_empty() {
                    active = false;
                }
            }
        }
        ips
    }

    fn is_ip(s: &str) -> bool {
        s.split('.').count() == 4 && s.chars().all(|c| c.is_ascii_digit() || c == '.')
    }

    pub fn enable(adapter: &str) -> Result<(), String> {
        netsh(&["interface", "ip", "set", "dns", adapter, "static", PRIMARY])?;
        netsh(&["interface", "ip", "add", "dns", adapter, SECONDARY, "index=2"])
    }

    pub fn disable(adapter: &str) -> Result<(), String> {
        netsh(&["interface", "ip", "set", "dns", adapter, "dhcp"])
    }

    fn netsh(args: &[&str]) -> Result<(), String> {
        let out = Command::new("netsh")
            .args(args)
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            let msg = String::from_utf8_lossy(&out.stderr).to_string();
            let alt = String::from_utf8_lossy(&out.stdout).to_string();
            Err(if msg.trim().is_empty() { alt } else { msg })
        }
    }
}

// ----- Linux -----------------------------------------------------------------
#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::process::Command;

    pub fn current_servers(adapter: &str) -> Result<Vec<String>, String> {
        let out = Command::new("nmcli")
            .args(["-t", "-f", "ipv4.dns", "connection", "show", adapter])
            .output()
            .map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout);
        Ok(text
            .lines()
            .filter_map(|l| l.split(':').nth(1))
            .flat_map(|s| s.split(','))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }

    pub fn enable(adapter: &str) -> Result<(), String> {
        nmcli(&["con", "mod", adapter, "ipv4.dns", &format!("{} {}", PRIMARY, SECONDARY)])?;
        nmcli(&["con", "mod", adapter, "ipv4.ignore-auto-dns", "yes"])?;
        nmcli(&["con", "up", adapter])
    }

    pub fn disable(adapter: &str) -> Result<(), String> {
        nmcli(&["con", "mod", adapter, "ipv4.dns", ""])?;
        nmcli(&["con", "mod", adapter, "ipv4.ignore-auto-dns", "no"])?;
        nmcli(&["con", "up", adapter])
    }

    fn nmcli(args: &[&str]) -> Result<(), String> {
        let out = Command::new("nmcli")
            .args(args)
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).to_string())
        }
    }
}

// ----- macOS -----------------------------------------------------------------
#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::process::Command;

    pub fn current_servers(adapter: &str) -> Result<Vec<String>, String> {
        let out = Command::new("networksetup")
            .args(["-getdnsservers", adapter])
            .output()
            .map_err(|e| e.to_string())?;
        Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with("There"))
            .collect())
    }

    pub fn enable(adapter: &str) -> Result<(), String> {
        ns(&["-setdnsservers", adapter, PRIMARY, SECONDARY])
    }

    pub fn disable(adapter: &str) -> Result<(), String> {
        ns(&["-setdnsservers", adapter, "empty"])
    }

    fn ns(args: &[&str]) -> Result<(), String> {
        let out = Command::new("networksetup")
            .args(args)
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).to_string())
        }
    }
}

// ----- Fallback --------------------------------------------------------------
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod platform {
    use super::*;

    pub fn current_servers(_: &str) -> Result<Vec<String>, String> {
        Ok(vec![])
    }
    pub fn enable(_: &str) -> Result<(), String> {
        Err("Unsupported platform".into())
    }
    pub fn disable(_: &str) -> Result<(), String> {
        Err("Unsupported platform".into())
    }
}
