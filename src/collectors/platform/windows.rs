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
            gpus: get_gpus_fast(),
            terminal: get_terminal_fast(),
            shell: None,
            display_resolution: None,
            battery: None,
            locale: None,
            encryption: None,
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
        encryption: get_bitlocker_status(),
    }
}

/// Query BitLocker status for the system drive (`C:`) via the
/// `root\CIMV2\Security\MicrosoftVolumeEncryption` namespace.
///
/// This namespace is readable by non-admin users on most Win11 Device
/// Encryption laptops; older Win10 / domain-joined configurations may require
/// admin and will return `None` (the elevation footer hint covers that case).
fn get_bitlocker_status() -> Option<String> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename = "Win32_EncryptableVolume")]
    #[serde(rename_all = "PascalCase")]
    struct EncryptableVolume {
        drive_letter: Option<String>,
        protection_status: Option<u32>,
        // Note: queried as a separate property because not all driver versions
        // expose ConversionStatus, and some return it as i32 vs u32.
    }

    #[derive(Deserialize)]
    #[serde(rename = "Win32_EncryptableVolume")]
    #[serde(rename_all = "PascalCase")]
    struct EncryptableVolumeMethod {
        drive_letter: Option<String>,
        encryption_method: Option<u32>,
    }

    let com = COMLibrary::new().ok()?;
    let wmi =
        WMIConnection::with_namespace_path(r"ROOT\CIMV2\Security\MicrosoftVolumeEncryption", com)
            .ok()?;

    let volumes: Vec<EncryptableVolume> = wmi.query().ok()?;
    let methods: Vec<EncryptableVolumeMethod> = wmi.query().ok().unwrap_or_default();

    // Find the system drive — typically C:.
    let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
    let target = volumes
        .iter()
        .find(|v| v.drive_letter.as_deref() == Some(system_drive.as_str()))?;

    let protection = target.protection_status?;
    let method_for_drive = methods
        .iter()
        .find(|m| m.drive_letter.as_deref() == Some(system_drive.as_str()))
        .and_then(|m| m.encryption_method);

    Some(format_bitlocker_status(protection, method_for_drive))
}

/// Format BitLocker status. ProtectionStatus: 0=Off, 1=On, 2=Unknown.
/// EncryptionMethod values per Microsoft Learn:
/// https://learn.microsoft.com/en-us/windows/win32/secprov/getencryptionmethod-win32-encryptablevolume
fn format_bitlocker_status(protection_status: u32, method: Option<u32>) -> String {
    match protection_status {
        0 => "BitLocker Off".to_string(),
        1 => {
            let method_name = method
                .map(bitlocker_method_name)
                .unwrap_or_else(|| "Unknown".to_string());
            format!("BitLocker On ({})", method_name)
        }
        _ => "BitLocker (status unknown)".to_string(),
    }
}

fn bitlocker_method_name(method: u32) -> String {
    match method {
        0 => "None".to_string(),
        1 => "AES-128 + Diffuser".to_string(),
        2 => "AES-256 + Diffuser".to_string(),
        3 => "AES-128".to_string(),
        4 => "AES-256".to_string(),
        5 => "Hardware".to_string(),
        6 => "XTS-AES-128".to_string(),
        7 => "XTS-AES-256".to_string(),
        _ => format!("Method #{}", method),
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

/// Get GPU names from registry (fast, no WMI/PowerShell needed, ~5-10ms)
fn get_gpus_fast() -> Vec<String> {
    let mut gpus = Vec::new();
    if let Ok(output) = Command::new("reg")
        .args([
            "query",
            r"HKLM\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}",
            "/s",
            "/v",
            "DriverDesc",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("DriverDesc") {
                // Format: "    DriverDesc    REG_SZ    NVIDIA GeForce RTX 4090"
                if let Some(value) = line.split("REG_SZ").nth(1) {
                    let gpu = value.trim();
                    if !gpu.is_empty() && !gpus.contains(&gpu.to_string()) {
                        gpus.push(gpu.to_string());
                    }
                }
            }
        }
    }
    gpus
}

// --- WMI-based collectors (fast, no PowerShell) ---

fn get_windows_edition_wmi(wmi: &WMIConnection) -> Option<String> {
    let results: Vec<Win32OperatingSystem> = wmi.query().ok()?;
    results.into_iter().next()?.caption
}

fn detect_virtualization_wmi(wmi: &WMIConnection) -> Option<String> {
    // Pull CPUID brand AND DMI manufacturer/model. CPUID is precise but
    // ambiguous on Win11 with VBS: a physical Win11 host running on top of the
    // VBS Hyper-V layer reports "Microsoft Hv" via CPUID even though the user
    // is on bare metal. We disambiguate by checking the SMBIOS manufacturer.
    let cpuid_brand = cpuid_hypervisor_brand();

    let results: Vec<Win32ComputerSystem> = wmi.query().ok()?;
    let cs = results.into_iter().next()?;
    let manufacturer = cs.manufacturer.unwrap_or_default().to_lowercase();
    let model = cs.model.unwrap_or_default().to_lowercase();
    let dmi = format!("{}|{}", manufacturer, model);

    // Definite VM signals from DMI (regardless of CPUID).
    if dmi.contains("vmware") {
        return Some("VMware".to_string());
    }
    if dmi.contains("virtualbox") || dmi.contains("vbox") {
        return Some("VirtualBox".to_string());
    }
    if dmi.contains("qemu") {
        return Some("QEMU".to_string());
    }
    if dmi.contains("xen") {
        return Some("Xen".to_string());
    }
    if dmi.contains("parallels") {
        return Some("Parallels".to_string());
    }
    // "Microsoft Corporation" + "Virtual Machine" = a real Hyper-V VM.
    if dmi.contains("microsoft") && dmi.contains("virtual") {
        return Some("Hyper-V".to_string());
    }

    // CPUID brand is set but DMI looks physical → host is running with
    // VBS/HVCI on a real laptop/desktop. Surface that nuance.
    match cpuid_brand.as_deref() {
        Some("Hyper-V") => {
            // Heuristic: real Hyper-V VMs always have Microsoft Corporation as
            // manufacturer. If we got here and manufacturer is something else
            // (Dell, HP, Lenovo, ASUS, MSI, Razer, Apple, Framework, etc.),
            // the hypervisor is the VBS layer — user is on bare metal.
            return Some("Bare Metal (Hyper-V/VBS)".to_string());
        }
        Some(other) => return Some(other.to_string()),
        None => {}
    }

    // Last-resort: WMI hypervisor_present flag.
    if cs.hypervisor_present == Some(true) {
        return Some("Hypervisor Present".to_string());
    }

    None
}

/// Detect hypervisor brand via CPUID leaf 0x40000000.
///
/// Bit 31 of ECX from leaf 1 indicates "hypervisor present". When set, leaf
/// 0x40000000 returns:
///   - EAX: maximum hypervisor leaf supported (>= 0x40000000)
///   - EBX, ECX, EDX: 12 bytes of ASCII forming the hypervisor vendor string
///     (e.g. "Microsoft Hv", "VMwareVMware", "KVMKVMKVM\0\0\0", "VBoxVBoxVBox")
fn cpuid_hypervisor_brand() -> Option<String> {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::__cpuid;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::__cpuid;
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    return None;

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        // SAFETY: CPUID is supported on every x86/x86_64 CPU we target.
        let leaf1 = unsafe { __cpuid(1) };
        // Bit 31 of ECX = hypervisor present
        if (leaf1.ecx & (1u32 << 31)) == 0 {
            return None;
        }
        let leaf = unsafe { __cpuid(0x4000_0000) };
        let mut bytes = [0u8; 12];
        bytes[0..4].copy_from_slice(&leaf.ebx.to_le_bytes());
        bytes[4..8].copy_from_slice(&leaf.ecx.to_le_bytes());
        bytes[8..12].copy_from_slice(&leaf.edx.to_le_bytes());
        let raw = std::str::from_utf8(&bytes).ok()?.trim_end_matches('\0');
        if raw.is_empty() {
            return None;
        }
        Some(map_hypervisor_vendor(raw))
    }
}

/// Map CPUID hypervisor vendor string to a friendly name.
fn map_hypervisor_vendor(raw: &str) -> String {
    match raw {
        "KVMKVMKVM" | "KVMKVMKVM\0\0\0" => "KVM".to_string(),
        "Microsoft Hv" => "Hyper-V".to_string(),
        "VMwareVMware" => "VMware".to_string(),
        "VBoxVBoxVBox" => "VirtualBox".to_string(),
        "XenVMMXenVMM" => "Xen".to_string(),
        "TCGTCGTCGTCG" => "QEMU".to_string(),
        "prl hyperv  " | "prl hyperv" => "Parallels".to_string(),
        "ACRNACRNACRN" => "ACRN".to_string(),
        "bhyve bhyve " | "bhyve bhyve" => "bhyve".to_string(),
        "QNXQVMBSQG" => "QNX".to_string(),
        other => other.trim().to_string(),
    }
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

/// Read OS info from `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion` so we
/// can detect Windows 11 by build number (the registry's `ProductName` is
/// frozen at "Windows 10" even on Win11) and enrich the version with the
/// release ID (DisplayVersion, e.g. "24H2") and UBR (Update Build Revision).
///
/// Returns `(name, version, kernel)` on success.
pub fn get_os_info_from_registry() -> Option<(String, String, String)> {
    let mut display_version: Option<String> = None;
    let mut current_build: Option<String> = None;
    let mut ubr: Option<u32> = None;
    let mut product_name: Option<String> = None;

    let output = Command::new("reg")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        ])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        // Format: "    <Name>    <Type>    <Value>" — split on whitespace and
        // pattern-match on the first three tokens (name, type, value...).
        let mut parts = line.split_whitespace();
        let name = match parts.next() {
            Some(n) => n,
            None => continue,
        };
        let _type_tok = match parts.next() {
            Some(t) => t,
            None => continue,
        };
        let value = parts.collect::<Vec<_>>().join(" ");
        match name {
            "DisplayVersion" => display_version = Some(value),
            "CurrentBuild" => current_build = Some(value),
            "UBR" => {
                ubr = u32::from_str_radix(value.trim_start_matches("0x"), 16).ok();
            }
            "ProductName" => product_name = Some(value),
            _ => {}
        }
    }

    let build_num: u32 = current_build.as_deref()?.parse().ok()?;

    // Detect Windows 11 by build number (>= 22000), per Microsoft's own gate.
    let name = "Windows".to_string();
    let release = if build_num >= 22000 {
        "11"
    } else if build_num >= 10240 {
        "10"
    } else {
        // Older Windows — fall back to ProductName which is accurate for those.
        return Some((
            product_name
                .clone()
                .unwrap_or_else(|| "Windows".to_string()),
            format!("(Build {})", build_num),
            current_build.unwrap_or_default(),
        ));
    };

    let mut version = release.to_string();
    if let Some(dv) = &display_version {
        if !dv.is_empty() {
            version.push(' ');
            version.push_str(dv);
        }
    }

    // Kernel string: full build with UBR (e.g. "26200.0" or "26100.4317").
    let kernel = match ubr {
        Some(u) => format!("{}.{}", build_num, u),
        None => build_num.to_string(),
    };

    Some((name, version, kernel))
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

// IsWow64Process2 manually-declared because winapi-rs's bindings are stale.
// Returns the host machine's architecture regardless of the running process's
// own architecture (so an x64 binary running on a Surface Pro X correctly
// identifies the host as ARM64). Reference:
// https://learn.microsoft.com/en-us/windows/win32/api/wow64apiset/nf-wow64apiset-iswow64process2
#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
    fn IsWow64Process2(
        hProcess: *mut std::ffi::c_void,
        pProcessMachine: *mut u16,
        pNativeMachine: *mut u16,
    ) -> i32;
}

// IMAGE_FILE_MACHINE_* constants from <winnt.h>.
const IMAGE_FILE_MACHINE_UNKNOWN: u16 = 0;
const IMAGE_FILE_MACHINE_I386: u16 = 0x014C;
const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
const IMAGE_FILE_MACHINE_ARM: u16 = 0x01C0;
const IMAGE_FILE_MACHINE_ARM64: u16 = 0xAA64;

fn get_architecture() -> Option<String> {
    let mut process_machine: u16 = 0;
    let mut native_machine: u16 = 0;
    // SAFETY: IsWow64Process2 takes a HANDLE and two *mut u16 outputs; we pass
    // valid pointers and the pseudo-handle from GetCurrentProcess.
    let ok = unsafe {
        IsWow64Process2(
            GetCurrentProcess(),
            &mut process_machine,
            &mut native_machine,
        )
    };
    if ok == 0 {
        return Some(std::env::consts::ARCH.to_string());
    }
    let host = match native_machine {
        IMAGE_FILE_MACHINE_AMD64 => "x86_64",
        IMAGE_FILE_MACHINE_ARM64 => "aarch64",
        IMAGE_FILE_MACHINE_I386 => "x86",
        IMAGE_FILE_MACHINE_ARM => "arm",
        IMAGE_FILE_MACHINE_UNKNOWN => return Some(std::env::consts::ARCH.to_string()),
        _ => return Some(format!("unknown (0x{:x})", native_machine)),
    };
    // If the running process arch differs from the host, annotate (e.g. an
    // x64-built TR-300 running on Win11 ARM64 reports "aarch64 (x86_64 emulation)").
    if process_machine != IMAGE_FILE_MACHINE_UNKNOWN && process_machine != native_machine {
        let proc_name = match process_machine {
            IMAGE_FILE_MACHINE_AMD64 => "x86_64",
            IMAGE_FILE_MACHINE_I386 => "x86",
            IMAGE_FILE_MACHINE_ARM => "arm",
            IMAGE_FILE_MACHINE_ARM64 => "aarch64",
            _ => "unknown",
        };
        return Some(format!("{} ({} emulation)", host, proc_name));
    }
    Some(host.to_string())
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
