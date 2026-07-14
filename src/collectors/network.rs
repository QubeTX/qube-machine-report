//! Network information collector

#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::collectors::command::run_stdout;
use crate::collectors::command::CommandTimeout;
use crate::collectors::CollectMode;
use crate::error::Result;
use std::env;

/// Network information for TR-300
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct NetworkInfo {
    /// Machine's primary IP address (None if skipped in fast mode)
    pub machine_ip: Option<String>,
    /// Client IP (SSH_CLIENT if in SSH session)
    pub client_ip: Option<String>,
    /// DNS server addresses
    pub dns_servers: Vec<String>,
}

/// Collect network information
pub fn collect_network_info(mode: CollectMode) -> Result<NetworkInfo> {
    let should_skip_slow = mode == CollectMode::Fast && should_skip_network_on_platform();

    let machine_ip = if should_skip_slow {
        None
    } else {
        get_machine_ip()
    };

    let client_ip = get_client_ip();

    let dns_servers = if should_skip_slow {
        Vec::new()
    } else {
        get_dns_servers()
    };

    Ok(NetworkInfo {
        machine_ip,
        client_ip,
        dns_servers,
    })
}

/// Whether to skip network collection in fast mode on this platform.
/// Windows and macOS use subprocess calls (slow). Linux reads /proc (fast).
fn should_skip_network_on_platform() -> bool {
    #[cfg(target_os = "linux")]
    {
        false
    }

    #[cfg(target_os = "windows")]
    {
        true
    }

    #[cfg(target_os = "macos")]
    {
        true
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        true
    }
}

/// Get the machine's primary IP address
fn get_machine_ip() -> Option<String> {
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
        None
    }
}

#[cfg(target_os = "windows")]
fn get_machine_ip_windows() -> Option<String> {
    // Use WMI for IP (no PowerShell subprocess)
    let (ip, _dns) = crate::collectors::platform::windows::get_network_info_wmi();
    if let Some(ip) = ip {
        return valid_routable_ip(&ip);
    }

    // Fallback: try ipconfig
    if let Some(output) = crate::collectors::command::run_output(
        "ipconfig",
        std::iter::empty::<&str>(),
        CommandTimeout::Normal,
    ) {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("IPv4 Address") {
                if let Some((_, ip)) = line.rsplit_once(':') {
                    let ip = ip.trim();
                    if let Some(ip) = valid_routable_ip(ip) {
                        return Some(ip);
                    }
                }
            }
        }
    }

    None
}

#[cfg(target_os = "linux")]
fn get_machine_ip_linux() -> Option<String> {
    // Ask the kernel which source IP it would use for the default route.
    if let Some(stdout) = run_stdout("ip", ["route", "get", "1.1.1.1"], CommandTimeout::Normal) {
        if let Some(ip) = parse_route_src_ip(&stdout) {
            return Some(ip);
        }
    }

    // Fallback: hostname -I
    if let Some(stdout) = run_stdout("hostname", ["-I"], CommandTimeout::Normal) {
        if let Some(ip) = stdout.split_whitespace().find_map(valid_routable_ip) {
            return Some(ip);
        }
    }

    None
}

#[cfg(target_os = "macos")]
fn get_machine_ip_macos() -> Option<String> {
    if let Some(stdout) = run_stdout("scutil", ["--nwi"], CommandTimeout::Normal) {
        if let Some(ip) = parse_scutil_nwi_primary_address(&stdout) {
            return Some(ip);
        }
        if let Some(iface) = parse_scutil_nwi_primary_interface(&stdout) {
            if let Some(ip) = run_stdout(
                "ipconfig",
                ["getifaddr", iface.as_str()],
                CommandTimeout::Normal,
            ) {
                if let Some(ip) = valid_routable_ip(ip.trim()) {
                    return Some(ip);
                }
            }
        }
    }

    // Try ipconfig getifaddr en0 (WiFi) or en1 (Ethernet)
    for iface in &["en0", "en1", "en2"] {
        if let Some(output) = run_stdout("ipconfig", ["getifaddr", iface], CommandTimeout::Normal) {
            if let Some(ip) = valid_routable_ip(output.trim()) {
                return Some(ip);
            }
        }
    }

    // Fallback: try route
    if let Some(stdout) = run_stdout("route", ["get", "default"], CommandTimeout::Normal) {
        for line in stdout.lines() {
            if line.trim().starts_with("interface:") {
                if let Some(iface) = line.split(':').next_back() {
                    let iface = iface.trim();
                    if let Some(output) =
                        run_stdout("ipconfig", ["getifaddr", iface], CommandTimeout::Normal)
                    {
                        if let Some(ip) = valid_routable_ip(output.trim()) {
                            return Some(ip);
                        }
                    }
                }
            }
        }
    }

    None
}

/// Get client IP from SSH_CLIENT environment variable
fn get_client_ip() -> Option<String> {
    // Check SSH_CLIENT env var
    if let Ok(ssh_client) = env::var("SSH_CLIENT") {
        // Format: "IP PORT LOCAL_PORT"
        if let Some(ip) = ssh_client.split_whitespace().next().and_then(valid_ip) {
            return Some(ip);
        }
    }

    // Check SSH_CONNECTION env var
    if let Ok(ssh_conn) = env::var("SSH_CONNECTION") {
        // Format: "CLIENT_IP CLIENT_PORT SERVER_IP SERVER_PORT"
        if let Some(ip) = ssh_conn.split_whitespace().next().and_then(valid_ip) {
            return Some(ip);
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
    // Use WMI for DNS servers (no PowerShell subprocess)
    let (_ip, servers) = crate::collectors::platform::windows::get_network_info_wmi();
    if !servers.is_empty() {
        return validated_servers(servers);
    }

    // Fallback: parse ipconfig /all
    let mut servers = Vec::new();
    if let Some(output) =
        crate::collectors::command::run_output("ipconfig", ["/all"], CommandTimeout::Normal)
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut in_dns_section = false;
        for line in stdout.lines() {
            if line.contains("DNS Servers") {
                in_dns_section = true;
                if let Some((_, ip)) = line.split_once(':') {
                    let ip = ip.trim();
                    if let Some(ip) = valid_ip(ip) {
                        if !servers.contains(&ip) {
                            servers.push(ip);
                        }
                    }
                }
            } else if in_dns_section {
                let trimmed = line.trim();
                if let Some(ip) = valid_ip(trimmed) {
                    if !servers.contains(&ip) {
                        servers.push(ip);
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

    servers
}

#[cfg(target_os = "linux")]
fn get_dns_servers_linux() -> Vec<String> {
    use std::fs;

    for path in [
        "/run/systemd/resolve/resolv.conf",
        "/run/NetworkManager/resolv.conf",
        "/etc/resolv.conf",
    ] {
        if let Ok(content) = fs::read_to_string(path) {
            let servers = parse_resolv_conf_servers(&content, path != "/etc/resolv.conf");
            if !servers.is_empty() {
                return servers;
            }
        }
    }

    // Try systemd-resolved if no servers found
    let mut servers = Vec::new();
    if let Some(stdout) = run_stdout("resolvectl", ["status"], CommandTimeout::Normal) {
        for line in stdout.lines() {
            if line.contains("DNS Servers:") {
                if let Some((_, ips)) = line.split_once(':') {
                    for ip in ips.split_whitespace() {
                        if let Some(ip) = valid_ip(ip) {
                            if !servers.contains(&ip) {
                                servers.push(ip);
                            }
                            if servers.len() >= 5 {
                                break;
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
    if let Some(stdout) = run_stdout("scutil", ["--dns"], CommandTimeout::Normal) {
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("nameserver[") {
                if let Some((_, ip)) = line.split_once(" : ") {
                    let ip = ip.trim();
                    if let Some(ip) = valid_ip(ip) {
                        if !servers.contains(&ip) {
                            servers.push(ip);
                        }
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

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_route_src_ip(route_output: &str) -> Option<String> {
    let mut parts = route_output.split_whitespace();
    while let Some(part) = parts.next() {
        if part == "src" {
            let ip = parts.next()?;
            return valid_routable_ip(ip);
        }
    }
    None
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_resolv_conf_servers(content: &str, skip_loopback: bool) -> Vec<String> {
    let mut servers = Vec::new();
    for line in content.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let mut fields = line.split_whitespace();
        if fields.next() != Some("nameserver") {
            continue;
        }
        let Some(ip) = fields.next().and_then(valid_ip) else {
            continue;
        };
        if skip_loopback && (ip.starts_with("127.") || ip == "::1") {
            continue;
        }
        if !servers.contains(&ip) {
            servers.push(ip);
            if servers.len() >= 5 {
                break;
            }
        }
    }
    servers
}

fn valid_ip(value: &str) -> Option<String> {
    let value = value.trim().trim_matches(['[', ']']);
    let parse_value = value.split('%').next().unwrap_or(value);
    parse_value
        .parse::<std::net::IpAddr>()
        .ok()
        .map(|_| value.to_string())
}

fn valid_routable_ip(value: &str) -> Option<String> {
    let normalized = valid_ip(value)?;
    let parse_value = normalized.split('%').next().unwrap_or(&normalized);
    let ip: std::net::IpAddr = parse_value.parse().ok()?;
    let unsuitable = ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || matches!(ip, std::net::IpAddr::V4(v4) if v4.is_link_local())
        || matches!(ip, std::net::IpAddr::V4(v4) if v4.is_broadcast())
        || matches!(ip, std::net::IpAddr::V6(v6) if v6.is_unicast_link_local());
    (!unsuitable).then_some(normalized)
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn validated_servers(servers: Vec<String>) -> Vec<String> {
    let mut validated = Vec::new();
    for server in servers {
        if let Some(server) = valid_ip(&server) {
            if !validated.contains(&server) {
                validated.push(server);
            }
        }
        if validated.len() >= 5 {
            break;
        }
    }
    validated
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn parse_scutil_nwi_primary_interface(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some((iface, rest)) = trimmed.split_once(':') {
            let iface = iface.trim();
            if rest.contains("IPv4") && !iface.is_empty() {
                return Some(iface.to_string());
            }
        }
    }
    None
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn parse_scutil_nwi_primary_address(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let trimmed = line.trim();
        let (_, value) = trimmed.split_once(" : ")?;
        (trimmed.starts_with("address") || trimmed.starts_with("Address"))
            .then(|| valid_routable_ip(value))
            .flatten()
    })
}

// Keep the old interface struct for backwards compatibility if needed
/// Network interface information (legacy)
#[non_exhaustive]
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
        crate::format_bytes(bytes)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_linux_route_src_ip() {
        let route = "1.1.1.1 via 10.0.0.1 dev wlan0 src 10.0.0.42 uid 1000";
        assert_eq!(parse_route_src_ip(route), Some("10.0.0.42".to_string()));
    }

    #[test]
    fn resolv_conf_skips_comments_and_deduplicates() {
        let content = "\
# comment
nameserver 127.0.0.53
nameserver 1.1.1.1
nameserver 1.1.1.1
nameserver 2606:4700:4700::1111
";
        assert_eq!(
            parse_resolv_conf_servers(content, true),
            vec!["1.1.1.1".to_string(), "2606:4700:4700::1111".to_string()]
        );
    }

    #[test]
    fn validates_ipv4_ipv6_and_rejects_endpoint_syntax() {
        assert_eq!(valid_ip("192.0.2.10"), Some("192.0.2.10".to_string()));
        assert_eq!(valid_ip("[fe80::1%en0]"), Some("fe80::1%en0".to_string()));
        assert_eq!(valid_ip("192.0.2.10:53"), None);
        assert_eq!(valid_ip("not-an-address"), None);
    }

    #[test]
    fn routable_filter_rejects_loopback_and_link_local() {
        assert_eq!(valid_routable_ip("127.0.0.1"), None);
        assert_eq!(valid_routable_ip("169.254.10.2"), None);
        assert_eq!(valid_routable_ip("::1"), None);
        assert_eq!(valid_routable_ip("fe80::1%en0"), None);
        assert_eq!(valid_routable_ip("224.0.0.1"), None);
        assert_eq!(valid_routable_ip("ff02::1"), None);
        assert_eq!(valid_routable_ip("255.255.255.255"), None);
        assert_eq!(
            valid_routable_ip("192.168.1.50"),
            Some("192.168.1.50".to_string())
        );
    }

    #[test]
    fn validated_dns_servers_deduplicate_and_cap_results() {
        let values = vec![
            "1.1.1.1",
            "bad",
            "1.1.1.1",
            "2606:4700:4700::1111",
            "8.8.8.8",
            "9.9.9.9",
            "208.67.222.222",
            "208.67.220.220",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        assert_eq!(
            validated_servers(values),
            vec![
                "1.1.1.1",
                "2606:4700:4700::1111",
                "8.8.8.8",
                "9.9.9.9",
                "208.67.222.222",
            ]
        );
    }

    #[test]
    fn parses_macos_scutil_nwi_primary_interface() {
        let nwi = "\
Network information
IPv4 network interface information
     en0 : flags      : 0x5 (IPv4,DNS)
           reach      : 0x00000002 (Reachable)
";
        assert_eq!(
            parse_scutil_nwi_primary_interface(nwi),
            Some("en0".to_string())
        );
    }

    #[test]
    fn parses_macos_scutil_nwi_primary_address() {
        let nwi = "address : 192.168.50.20\nAddress : fe80::1%en0\n";
        assert_eq!(
            parse_scutil_nwi_primary_address(nwi),
            Some("192.168.50.20".to_string())
        );
    }
}
