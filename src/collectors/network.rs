//! Network information collector

use crate::error::Result;
use std::env;
use std::process::Command;

/// Network information for TR-300
#[derive(Debug, Clone)]
pub struct NetworkInfo {
    /// Machine's primary IP address
    pub machine_ip: String,
    /// Client IP (SSH_CLIENT if in SSH session)
    pub client_ip: Option<String>,
    /// DNS server addresses
    pub dns_servers: Vec<String>,
}

/// Collect network information
pub fn collect_network_info() -> Result<NetworkInfo> {
    let machine_ip = get_machine_ip();
    let client_ip = get_client_ip();
    let dns_servers = get_dns_servers();

    Ok(NetworkInfo {
        machine_ip,
        client_ip,
        dns_servers,
    })
}

/// Get the machine's primary IP address
fn get_machine_ip() -> String {
    #[cfg(target_os = "windows")]
    {
        get_machine_ip_windows()
    }

    #[cfg(target_os = "linux")]
    {
        get_machine_ip_linux()
    }

    #[cfg(target_os = "macos")]
    {
        get_machine_ip_macos()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        "Unknown".to_string()
    }
}

#[cfg(target_os = "windows")]
fn get_machine_ip_windows() -> String {
    // Try PowerShell command
    if let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.InterfaceAlias -notmatch 'Loopback' -and $_.PrefixOrigin -ne 'WellKnown' } | Select-Object -First 1).IPAddress",
        ])
        .output()
    {
        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !ip.is_empty() && ip != "127.0.0.1" {
            return ip;
        }
    }

    // Fallback: try ipconfig
    if let Ok(output) = Command::new("ipconfig").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("IPv4 Address") {
                if let Some(ip) = line.split(':').next_back() {
                    let ip = ip.trim();
                    if !ip.is_empty() && ip != "127.0.0.1" {
                        return ip.to_string();
                    }
                }
            }
        }
    }

    "Unknown".to_string()
}

#[cfg(target_os = "linux")]
fn get_machine_ip_linux() -> String {
    // Try hostname -I
    if let Ok(output) = Command::new("hostname").args(["-I"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(ip) = stdout.split_whitespace().next() {
            if !ip.is_empty() && ip != "127.0.0.1" {
                return ip.to_string();
            }
        }
    }

    // Try ip route
    if let Ok(output) = Command::new("ip").args(["route", "get", "1"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for part in stdout.split_whitespace() {
            if part.contains('.') && !part.starts_with("1.") {
                // Skip the destination "1.0.0.0"
                if let Some(prev_part) = stdout.split(part).next() {
                    if prev_part.ends_with("src ") {
                        return part.to_string();
                    }
                }
            }
        }
    }

    "Unknown".to_string()
}

#[cfg(target_os = "macos")]
fn get_machine_ip_macos() -> String {
    // Try ipconfig getifaddr en0 (WiFi) or en1 (Ethernet)
    for iface in &["en0", "en1", "en2"] {
        if let Ok(output) = Command::new("ipconfig")
            .args(["getifaddr", iface])
            .output()
        {
            let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !ip.is_empty() && ip != "127.0.0.1" {
                return ip;
            }
        }
    }

    // Fallback: try route
    if let Ok(output) = Command::new("route")
        .args(["get", "default"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.trim().starts_with("interface:") {
                if let Some(iface) = line.split(':').last() {
                    let iface = iface.trim();
                    if let Ok(output) = Command::new("ipconfig")
                        .args(["getifaddr", iface])
                        .output()
                    {
                        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !ip.is_empty() && ip != "127.0.0.1" {
                            return ip;
                        }
                    }
                }
            }
        }
    }

    "Unknown".to_string()
}

/// Get client IP from SSH_CLIENT environment variable
fn get_client_ip() -> Option<String> {
    // Check SSH_CLIENT env var
    if let Ok(ssh_client) = env::var("SSH_CLIENT") {
        // Format: "IP PORT LOCAL_PORT"
        if let Some(ip) = ssh_client.split_whitespace().next() {
            return Some(ip.to_string());
        }
    }

    // Check SSH_CONNECTION env var
    if let Ok(ssh_conn) = env::var("SSH_CONNECTION") {
        // Format: "CLIENT_IP CLIENT_PORT SERVER_IP SERVER_PORT"
        if let Some(ip) = ssh_conn.split_whitespace().next() {
            return Some(ip.to_string());
        }
    }

    None
}

/// Get DNS server addresses
fn get_dns_servers() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        get_dns_servers_windows()
    }

    #[cfg(target_os = "linux")]
    {
        get_dns_servers_linux()
    }

    #[cfg(target_os = "macos")]
    {
        get_dns_servers_macos()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "windows")]
fn get_dns_servers_windows() -> Vec<String> {
    let mut servers = Vec::new();

    // Try PowerShell
    if let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-DnsClientServerAddress -AddressFamily IPv4 | Where-Object { $_.ServerAddresses } | Select-Object -ExpandProperty ServerAddresses) -join \"`n\"",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let ip = line.trim();
            if !ip.is_empty() && !servers.contains(&ip.to_string()) {
                servers.push(ip.to_string());
                if servers.len() >= 5 {
                    break;
                }
            }
        }
    }

    if servers.is_empty() {
        // Fallback: parse ipconfig /all
        if let Ok(output) = Command::new("ipconfig").args(["/all"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut in_dns_section = false;
            for line in stdout.lines() {
                if line.contains("DNS Servers") {
                    in_dns_section = true;
                    if let Some(ip) = line.split(':').next_back() {
                        let ip = ip.trim();
                        if !ip.is_empty() && !servers.contains(&ip.to_string()) {
                            servers.push(ip.to_string());
                        }
                    }
                } else if in_dns_section {
                    let trimmed = line.trim();
                    if trimmed.contains('.') && !trimmed.contains(':') {
                        if !servers.contains(&trimmed.to_string()) {
                            servers.push(trimmed.to_string());
                        }
                    } else if !trimmed.is_empty() {
                        in_dns_section = false;
                    }
                }
                if servers.len() >= 5 {
                    break;
                }
            }
        }
    }

    servers
}

#[cfg(target_os = "linux")]
fn get_dns_servers_linux() -> Vec<String> {
    use std::fs;

    let mut servers = Vec::new();

    // Try /etc/resolv.conf
    if let Ok(content) = fs::read_to_string("/etc/resolv.conf") {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("nameserver") {
                if let Some(ip) = line.split_whitespace().nth(1) {
                    if !servers.contains(&ip.to_string()) {
                        servers.push(ip.to_string());
                        if servers.len() >= 5 {
                            break;
                        }
                    }
                }
            }
        }
    }

    // Try systemd-resolved if no servers found
    if servers.is_empty() {
        if let Ok(output) = Command::new("resolvectl").args(["status"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("DNS Servers:") {
                    if let Some(ips) = line.split(':').last() {
                        for ip in ips.split_whitespace() {
                            if !servers.contains(&ip.to_string()) {
                                servers.push(ip.to_string());
                                if servers.len() >= 5 {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    servers
}

#[cfg(target_os = "macos")]
fn get_dns_servers_macos() -> Vec<String> {
    let mut servers = Vec::new();

    // Try scutil --dns
    if let Ok(output) = Command::new("scutil").args(["--dns"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("nameserver[") {
                if let Some(ip) = line.split(':').last() {
                    let ip = ip.trim();
                    if !ip.is_empty() && !servers.contains(&ip.to_string()) {
                        servers.push(ip.to_string());
                        if servers.len() >= 5 {
                            break;
                        }
                    }
                }
            }
        }
    }

    servers
}

// Keep the old interface struct for backwards compatibility if needed
/// Network interface information (legacy)
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub mac_address: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
}

/// Collect network interface information (legacy)
pub fn collect() -> Result<Vec<NetworkInterface>> {
    use sysinfo::Networks;

    let networks = Networks::new_with_refreshed_list();
    let mut result = Vec::new();

    for (name, data) in networks.list() {
        let mac = data.mac_address();
        let mac_string = format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            mac.0[0], mac.0[1], mac.0[2], mac.0[3], mac.0[4], mac.0[5]
        );

        result.push(NetworkInterface {
            name: name.clone(),
            mac_address: mac_string,
            rx_bytes: data.total_received(),
            tx_bytes: data.total_transmitted(),
            rx_packets: data.total_packets_received(),
            tx_packets: data.total_packets_transmitted(),
        });
    }

    result.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(result)
}

impl NetworkInterface {
    fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    pub fn rx_formatted(&self) -> String {
        Self::format_bytes(self.rx_bytes)
    }

    pub fn tx_formatted(&self) -> String {
        Self::format_bytes(self.tx_bytes)
    }

    pub fn traffic_string(&self) -> String {
        format!("RX: {} / TX: {}", self.rx_formatted(), self.tx_formatted())
    }

    pub fn is_active(&self) -> bool {
        self.rx_bytes > 0 || self.tx_bytes > 0
    }
}
