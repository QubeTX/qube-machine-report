//! Windows-specific information collectors
//!
//! Uses WMI crate for direct queries (replaces PowerShell subprocess spawns).

use super::{CollectMode, PlatformInfo};
use serde::Deserialize;
use std::env;
use std::process::Command;
use wmi::{COMLibrary, WMIConnection};

// --- WMI query structs ---

#[derive(Deserialize)]
#[serde(rename = "Win32_OperatingSystem")]
#[serde(rename_all = "PascalCase")]
struct Win32OperatingSystem {
    caption: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename = "Win32_ComputerSystem")]
#[serde(rename_all = "PascalCase")]
struct Win32ComputerSystem {
    manufacturer: Option<String>,
    model: Option<String>,
    hypervisor_present: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename = "Win32_VideoController")]
#[serde(rename_all = "PascalCase")]
struct Win32VideoController {
    name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename = "Win32_Battery")]
#[serde(rename_all = "PascalCase")]
struct Win32Battery {
    estimated_charge_remaining: Option<u16>,
    battery_status: Option<u16>,
}

#[derive(Deserialize)]
#[serde(rename = "Win32_NetworkAdapterConfiguration")]
#[serde(rename_all = "PascalCase")]
struct Win32NetworkAdapterConfig {
    #[serde(rename = "IPAddress")]
    ip_address: Option<Vec<String>>,
    #[serde(rename = "DNSServerSearchOrder")]
    dns_server_search_order: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename = "Win32_Processor")]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
struct Win32Processor {
    socket_designation: Option<String>,
}

/// Collect Windows-specific information
pub fn collect(mode: CollectMode) -> PlatformInfo {
    // In fast mode, skip all slow calls — return only env-var-based fields
    if mode == CollectMode::Fast {
        return PlatformInfo {
            architecture: get_architecture(),
            desktop_environment: Some("Windows Shell".to_string()),
            display_server: Some("DWM".to_string()),
            windows_edition: None,
            boot_mode: None,
            virtualization: None,
            macos_codename: None,
            gpus: Vec::new(),
            terminal: get_terminal_fast(),
            shell: None,
            display_resolution: None,
            battery: None,
            locale: None,
        };
    }

    // Full mode: try WMI first, fall back to PowerShell if WMI fails
    let wmi_result = COMLibrary::new()
        .ok()
        .and_then(|com| WMIConnection::new(com).ok());

    let (windows_edition, virtualization, gpus, battery) = if let Some(ref wmi) = wmi_result {
        let edition = get_windows_edition_wmi(wmi).or_else(get_windows_edition_ps);
        let virt = detect_virtualization_wmi(wmi).or_else(detect_virtualization_ps);
        let gpu_list = {
            let wmi_gpus = get_gpus_wmi(wmi);
            if wmi_gpus.is_empty() {
                get_gpus_ps()
            } else {
                wmi_gpus
            }
        };
        let bat = get_battery_wmi(wmi).or_else(get_battery_ps);
        (edition, virt, gpu_list, bat)
    } else {
        // WMI connection failed entirely — fall back to PowerShell for everything
        (
            get_windows_edition_ps(),
            detect_virtualization_ps(),
            get_gpus_ps(),
            get_battery_ps(),
        )
    };

    PlatformInfo {
        windows_edition,
        boot_mode: detect_boot_mode(),
        virtualization,
        desktop_environment: Some("Windows Shell".to_string()),
        display_server: Some("DWM".to_string()),
        macos_codename: None,
        gpus,
        architecture: get_architecture(),
        terminal: get_terminal(),
        shell: get_shell(),
        display_resolution: get_display_resolution(),
        battery,
        locale: get_locale(),
    }
}

/// Get terminal name using only env vars (no subprocess)
fn get_terminal_fast() -> Option<String> {
    if env::var("WT_SESSION").is_ok() {
        return Some("Windows Terminal".to_string());
    }
    if env::var("TERM_PROGRAM").ok().as_deref() == Some("vscode") {
        return Some("VS Code".to_string());
    }
    Some("Console".to_string())
}

// --- WMI-based collectors (fast, no PowerShell) ---

fn get_windows_edition_wmi(wmi: &WMIConnection) -> Option<String> {
    let results: Vec<Win32OperatingSystem> = wmi.query().ok()?;
    results.into_iter().next()?.caption
}

fn detect_virtualization_wmi(wmi: &WMIConnection) -> Option<String> {
    let results: Vec<Win32ComputerSystem> = wmi.query().ok()?;
    let cs = results.into_iter().next()?;

    let manufacturer = cs.manufacturer.unwrap_or_default().to_lowercase();
    let model = cs.model.unwrap_or_default().to_lowercase();
    let info = format!("{}|{}", manufacturer, model);

    if info.contains("vmware") {
        return Some("VMware".to_string());
    }
    if info.contains("virtualbox") || info.contains("vbox") {
        return Some("VirtualBox".to_string());
    }
    if info.contains("microsoft") && info.contains("virtual") {
        return Some("Hyper-V".to_string());
    }
    if info.contains("qemu") {
        return Some("QEMU".to_string());
    }
    if info.contains("xen") {
        return Some("Xen".to_string());
    }
    if info.contains("parallels") {
        return Some("Parallels".to_string());
    }

    if cs.hypervisor_present == Some(true) {
        return Some("Hypervisor Present".to_string());
    }

    None
}

fn get_gpus_wmi(wmi: &WMIConnection) -> Vec<String> {
    let results: Vec<Win32VideoController> = wmi.query().unwrap_or_default();
    results
        .into_iter()
        .filter_map(|v| v.name)
        .filter(|n| !n.is_empty())
        .collect()
}

fn get_battery_wmi(wmi: &WMIConnection) -> Option<String> {
    let results: Vec<Win32Battery> = wmi.query().ok()?;
    let bat = results.into_iter().next()?;
    let charge = bat.estimated_charge_remaining?;
    let status_code = bat.battery_status.unwrap_or(0);
    let status = match status_code {
        1 => "Discharging",
        2 => "AC Power",
        3 | 6 => "Charging",
        4 => "Low",
        5 => "Critical",
        7 => "Charging High",
        8 => "Charging Low",
        9 => "Charging Critical",
        _ => "Unknown",
    };
    Some(format!("{}% ({})", charge, status))
}

/// Get socket count via WMI, with PowerShell fallback (used from cpu.rs)
pub fn get_socket_count_wmi() -> usize {
    // Try WMI first
    let count = COMLibrary::new()
        .ok()
        .and_then(|com| WMIConnection::new(com).ok())
        .and_then(|wmi| {
            let results: Vec<Win32Processor> = wmi.query().ok()?;
            Some(results.len().max(1))
        });
    if let Some(c) = count {
        return c;
    }

    // Fallback: PowerShell
    if let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_Processor).Count",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(c) = stdout.trim().parse::<usize>() {
            return c.max(1);
        }
    }
    1
}

/// Get network info via WMI, with PowerShell fallback (used from network.rs)
pub fn get_network_info_wmi() -> (Option<String>, Vec<String>) {
    // Try WMI first
    let result = COMLibrary::new()
        .ok()
        .and_then(|com| WMIConnection::new(com).ok())
        .and_then(|wmi| {
            let results: Vec<Win32NetworkAdapterConfig> = wmi
                .raw_query("SELECT IPAddress, DNSServerSearchOrder FROM Win32_NetworkAdapterConfiguration WHERE IPEnabled = TRUE")
                .ok()?;
            Some(results)
        });

    if let Some(results) = result {
        let mut machine_ip: Option<String> = None;
        let mut dns_servers: Vec<String> = Vec::new();

        for adapter in &results {
            if machine_ip.is_none() {
                if let Some(ref ips) = adapter.ip_address {
                    for ip in ips {
                        if ip.contains('.') && ip != "127.0.0.1" {
                            machine_ip = Some(ip.clone());
                            break;
                        }
                    }
                }
            }
            if let Some(ref dns_list) = adapter.dns_server_search_order {
                for dns in dns_list {
                    if !dns.is_empty() && !dns_servers.contains(dns) {
                        dns_servers.push(dns.clone());
                        if dns_servers.len() >= 5 {
                            break;
                        }
                    }
                }
            }
        }

        if machine_ip.is_some() || !dns_servers.is_empty() {
            return (machine_ip, dns_servers);
        }
    }

    // Fallback: PowerShell for IP
    let mut machine_ip: Option<String> = None;
    if let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile", "-Command",
            "(Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.InterfaceAlias -notmatch 'Loopback' -and $_.PrefixOrigin -ne 'WellKnown' } | Select-Object -First 1).IPAddress",
        ])
        .output()
    {
        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !ip.is_empty() && ip != "127.0.0.1" {
            machine_ip = Some(ip);
        }
    }

    // Fallback: PowerShell for DNS
    let mut dns_servers = Vec::new();
    if let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile", "-Command",
            "(Get-DnsClientServerAddress -AddressFamily IPv4 | Where-Object { $_.ServerAddresses } | Select-Object -ExpandProperty ServerAddresses) -join \"`n\"",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let ip = line.trim();
            if !ip.is_empty() && !dns_servers.contains(&ip.to_string()) {
                dns_servers.push(ip.to_string());
                if dns_servers.len() >= 5 { break; }
            }
        }
    }

    (machine_ip, dns_servers)
}

// --- Non-WMI collectors ---

fn get_architecture() -> Option<String> {
    Some(std::env::consts::ARCH.to_string())
}

/// Detect boot mode via environment variable (no PowerShell needed)
fn detect_boot_mode() -> Option<String> {
    // Check firmware_type env var (set by Windows on UEFI systems)
    if let Ok(firmware) = env::var("firmware_type") {
        let upper = firmware.to_uppercase();
        if upper.contains("UEFI") {
            return Some("UEFI".to_string());
        }
        if upper.contains("LEGACY") || upper.contains("BIOS") {
            return Some("Legacy BIOS".to_string());
        }
    }
    // Fallback: check bcdedit (fast native command, not PowerShell)
    if let Ok(output) = Command::new("cmd")
        .args(["/c", "bcdedit", "/enum", "{current}"])
        .output()
    {
        let info = String::from_utf8_lossy(&output.stdout).to_lowercase();
        if info.contains("winload.efi") {
            return Some("UEFI".to_string());
        }
        return Some("Legacy BIOS".to_string());
    }
    None
}

/// Get terminal emulator name
fn get_terminal() -> Option<String> {
    // Check env vars first (instant)
    if env::var("WT_SESSION").is_ok() {
        return Some("Windows Terminal".to_string());
    }
    if env::var("TERM_PROGRAM").ok().as_deref() == Some("vscode") {
        return Some("VS Code".to_string());
    }
    // Env-var only, no PowerShell parent process check
    Some("Console".to_string())
}

/// Get shell name and version
fn get_shell() -> Option<String> {
    // Check SHELL env var first (for Git Bash, etc.)
    if let Ok(shell) = env::var("SHELL") {
        if shell.contains("bash") {
            if let Ok(output) = Command::new("bash").args(["--version"]).output() {
                let version = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = version.lines().next() {
                    if let Some(ver_start) = line.find("version ") {
                        let ver_part = &line[ver_start + 8..];
                        if let Some(ver_end) =
                            ver_part.find(|c: char| !c.is_ascii_digit() && c != '.')
                        {
                            return Some(format!("bash {}", &ver_part[..ver_end]));
                        }
                    }
                }
            }
            return Some("bash".to_string());
        }
    }

    // Check PowerShell version via registry (no subprocess)
    if let Ok(output) = Command::new("reg")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\PowerShell\3\PowerShellEngine",
            "/v",
            "PowerShellVersion",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("PowerShellVersion") {
                if let Some(version) = line.split_whitespace().last() {
                    return Some(format!("PowerShell {}", version));
                }
            }
        }
    }

    Some("PowerShell".to_string())
}

/// Get display resolution via Win32 API (no PowerShell)
fn get_display_resolution() -> Option<String> {
    unsafe {
        let cx = winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN);
        let cy = winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CYSCREEN);
        if cx > 0 && cy > 0 {
            Some(format!("{}x{}", cx, cy))
        } else {
            None
        }
    }
}

/// Get system locale via Win32 API (no PowerShell)
fn get_locale() -> Option<String> {
    let mut buf = [0u16; 85]; // LOCALE_NAME_MAX_LENGTH
    unsafe {
        let len = winapi::um::winnls::GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32);
        if len > 0 {
            let name = String::from_utf16_lossy(&buf[..len as usize - 1]);
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

// --- PowerShell fallbacks (used when WMI fails) ---

fn get_windows_edition_ps() -> Option<String> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_OperatingSystem).Caption",
        ])
        .output()
        .ok()?;
    let caption = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if caption.is_empty() {
        None
    } else {
        Some(caption)
    }
}

fn detect_virtualization_ps() -> Option<String> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile", "-Command",
            "(Get-CimInstance Win32_ComputerSystem).Manufacturer + '|' + (Get-CimInstance Win32_ComputerSystem).Model + '|' + (Get-CimInstance Win32_ComputerSystem).HypervisorPresent",
        ])
        .output()
        .ok()?;
    let info = String::from_utf8_lossy(&output.stdout).to_lowercase();
    if info.contains("vmware") {
        return Some("VMware".to_string());
    }
    if info.contains("virtualbox") || info.contains("vbox") {
        return Some("VirtualBox".to_string());
    }
    if info.contains("microsoft") && info.contains("virtual") {
        return Some("Hyper-V".to_string());
    }
    if info.contains("qemu") {
        return Some("QEMU".to_string());
    }
    if info.contains("xen") {
        return Some("Xen".to_string());
    }
    if info.contains("parallels") {
        return Some("Parallels".to_string());
    }
    if info.contains("true") {
        return Some("Hypervisor Present".to_string());
    }
    None
}

fn get_gpus_ps() -> Vec<String> {
    let mut gpus = Vec::new();
    if let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_VideoController).Name -join \"`n\"",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let gpu = line.trim();
            if !gpu.is_empty() {
                gpus.push(gpu.to_string());
            }
        }
    }
    gpus
}

fn get_battery_ps() -> Option<String> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile", "-Command",
            "$b = Get-CimInstance Win32_Battery; if ($b) { \"$($b.EstimatedChargeRemaining)% ($($b.BatteryStatus))\" }",
        ])
        .output()
        .ok()?;
    let battery = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if battery.is_empty() || !battery.contains('%') {
        return None;
    }
    let battery = battery
        .replace("(1)", "(Discharging)")
        .replace("(2)", "(AC Power)")
        .replace("(3)", "(Charging)")
        .replace("(4)", "(Low)")
        .replace("(5)", "(Critical)")
        .replace("(6)", "(Charging)")
        .replace("(7)", "(Charging High)")
        .replace("(8)", "(Charging Low)")
        .replace("(9)", "(Charging Critical)");
    Some(battery)
}
