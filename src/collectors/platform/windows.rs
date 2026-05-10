//! Windows-specific information collectors
//!
//! Uses WMI crate for direct queries (replaces PowerShell subprocess spawns).

use super::{CollectMode, PlatformInfo};
use crate::collectors::command::{run_output, run_stdout, CommandTimeout};
use serde::Deserialize;
use std::env;
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
    interface_index: Option<u32>,
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
            machine_model: None,
            cpu_core_topology: None,
            display_resolution: None,
            battery: None,
            zfs_health: None,
            motherboard: None,
            bios: None,
            ram_slots: None,
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
        // C.8 (v3.13.0+): the registry path used by `--fast` already
        // returns only hardware adapters (the {4d36e968-...} Display class
        // doesn't enumerate Microsoft Basic Render Driver or Hyper-V Video).
        // Prefer it in full mode too, with WMI / PowerShell as fallbacks
        // and a name-based filter to strip software adapters that historically
        // leaked through the WMI path on some configs.
        let gpu_list = {
            let mut gpus = get_gpus_fast();
            if gpus.is_empty() {
                gpus = get_gpus_wmi(wmi);
            }
            if gpus.is_empty() {
                gpus = get_gpus_ps();
            }
            filter_software_gpus(gpus)
        };
        // C.10: prefer the native GetSystemPowerStatus call (~1 ms, no COM, no
        // PowerShell). Falls back through WMI → PowerShell only if the kernel
        // call somehow returns no data — basically never on real hardware.
        let bat = get_battery_native()
            .or_else(|| get_battery_wmi(wmi))
            .or_else(get_battery_ps);
        (edition, virt, gpu_list, bat)
    } else {
        // WMI connection failed entirely — fall back to PowerShell for everything
        let fallback = get_batched_powershell_fallback();
        let gpus = if fallback.gpus.is_empty() {
            get_gpus_ps()
        } else {
            fallback.gpus
        };
        (
            fallback.windows_edition.or_else(get_windows_edition_ps),
            fallback.virtualization.or_else(detect_virtualization_ps),
            gpus,
            fallback.battery.or_else(get_battery_ps),
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
        machine_model: None,
        cpu_core_topology: None,
        terminal: get_terminal(),
        shell: get_shell(),
        display_resolution: get_display_resolution(),
        battery,
        zfs_health: None,
        motherboard: None,
        bios: None,
        ram_slots: None,
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
    if let Some(output) = run_output(
        "reg",
        [
            "query",
            r"HKLM\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}",
            "/s",
            "/v",
            "DriverDesc",
        ],
        CommandTimeout::Normal,
    ) {
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
        // CPUID is callable without an unsafe block on Rust ≥ 1.95 (no safety
        // preconditions on x86/x86_64).
        let leaf1 = __cpuid(1);
        // Bit 31 of ECX = hypervisor present
        if (leaf1.ecx & (1u32 << 31)) == 0 {
            return None;
        }
        let leaf = __cpuid(0x4000_0000);
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

    let stdout = run_stdout(
        "reg",
        [
            "query",
            r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        ],
        CommandTimeout::Normal,
    )?;

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

/// Native socket count via `GetLogicalProcessorInformationEx` (kernel32),
/// counting `RelationProcessorPackage` entries. ~3 ms vs ~30 ms for the WMI
/// path. Returns `None` on any failure — caller falls back to the WMI path.
///
/// The two-call pattern: first call with `Buffer = null_mut` returns FALSE +
/// `ERROR_INSUFFICIENT_BUFFER (122)` and writes `returned_length`; we then
/// allocate exactly that size and call again. Each
/// `SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX` is variable-length, so we walk
/// the buffer by reading the `Size` field at each entry's offset.
pub fn get_socket_count_native() -> Option<usize> {
    use winapi::shared::winerror::ERROR_INSUFFICIENT_BUFFER;
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::sysinfoapi::GetLogicalProcessorInformationEx;
    use winapi::um::winnt::RelationProcessorPackage;

    let mut returned_length: u32 = 0;
    // SAFETY: First call with null buffer + 0 length is the documented sizing
    // protocol. Returns FALSE; we read the size from `returned_length` and
    // verify the error code is ERROR_INSUFFICIENT_BUFFER before allocating.
    let ok = unsafe {
        GetLogicalProcessorInformationEx(
            RelationProcessorPackage,
            std::ptr::null_mut(),
            &mut returned_length,
        )
    };
    if ok != 0 || returned_length == 0 {
        return None;
    }
    if unsafe { GetLastError() } != ERROR_INSUFFICIENT_BUFFER {
        return None;
    }

    let mut buffer: Vec<u8> = vec![0u8; returned_length as usize];
    // SAFETY: `buffer` is exactly `returned_length` bytes; the cast to
    // `*mut SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX` is the documented
    // calling convention. The function writes 1+ variable-length records.
    let ok = unsafe {
        GetLogicalProcessorInformationEx(
            RelationProcessorPackage,
            buffer.as_mut_ptr() as *mut _,
            &mut returned_length,
        )
    };
    if ok == 0 {
        return None;
    }

    // Walk variable-length records. Each starts with `Relationship: u32` then
    // `Size: u32`; advance `offset` by `Size`. Count entries that report
    // `RelationProcessorPackage` (defensive — we asked for that filter, so
    // every record should match, but the API contract doesn't strictly
    // promise it).
    //
    // We read the two `u32` header fields via `from_le_bytes` rather than a
    // raw `*(ptr as *const u32)` cast. SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX
    // records are sized in 4-byte multiples in practice, so the offsets land
    // on aligned boundaries — but Rust spec doesn't guarantee that
    // `Vec<u8>::as_ptr().add(offset)` produces a u32-aligned pointer, and
    // dereferencing through a misaligned `*const u32` is UB even on x86
    // (LLVM is free to optimize assuming alignment). `from_le_bytes` works
    // on any byte boundary. (Caught in v3.13.0 Codex review.)
    let mut offset: usize = 0;
    let mut sockets: usize = 0;
    while offset + 8 <= buffer.len() {
        let mut header = [0u8; 8];
        header.copy_from_slice(&buffer[offset..offset + 8]);
        let relationship = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        let size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
        if size == 0 || offset + size > buffer.len() {
            break; // Defensive: malformed record stops the walk.
        }
        if relationship == RelationProcessorPackage {
            sockets += 1;
        }
        offset += size;
    }

    if sockets == 0 {
        None
    } else {
        Some(sockets)
    }
}

/// Get socket count via WMI, with PowerShell fallback (used from cpu.rs)
#[allow(dead_code)] // Kept for one release as a fallback under C.13/C.14 transition.
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
    if let Some(stdout) = run_stdout(
        "powershell",
        [
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_Processor).Count",
        ],
        CommandTimeout::Slow,
    ) {
        if let Ok(c) = stdout.trim().parse::<usize>() {
            return c.max(1);
        }
    }
    1
}

/// Get network info via WMI, with PowerShell fallback (used from network.rs).
///
/// Uses `GetBestInterfaceEx` (VPN-aware, kernel default-route lookup) to pick
/// the IP-enabled adapter Windows would actually use to reach the public
/// internet, then extracts that adapter's IPv4 address + DNS servers from the
/// existing WMI query. Falls back to "first IP-enabled adapter" if either the
/// kernel route lookup or the WMI filter returns nothing — preserves the
/// pre-v3.12.0 behavior on hosts where IP Helper service is disabled or no
/// default route exists.
pub fn get_network_info_wmi() -> (Option<String>, Vec<String>) {
    // Try WMI first
    let result = COMLibrary::new()
        .ok()
        .and_then(|com| WMIConnection::new(com).ok())
        .and_then(|wmi| {
            let results: Vec<Win32NetworkAdapterConfig> = wmi
                .raw_query("SELECT IPAddress, DNSServerSearchOrder, InterfaceIndex FROM Win32_NetworkAdapterConfiguration WHERE IPEnabled = TRUE")
                .ok()?;
            Some(results)
        });

    if let Some(results) = result {
        // Ask the kernel for the best-route interface to a public address; the
        // adapter that wins this lookup is the one carrying actual internet
        // traffic right now (correct on multi-homed/VPN configs).
        let best_index = get_best_route_interface_index();

        // Order adapters: best-route adapter first (if known), then the rest in
        // their WMI-natural order. This makes the existing first-match loop
        // produce VPN-correct output without changing its shape.
        let mut ordered: Vec<&Win32NetworkAdapterConfig> = Vec::with_capacity(results.len());
        if let Some(idx) = best_index {
            ordered.extend(results.iter().filter(|a| a.interface_index == Some(idx)));
            ordered.extend(results.iter().filter(|a| a.interface_index != Some(idx)));
        } else {
            ordered.extend(results.iter());
        }

        let mut machine_ip: Option<String> = None;
        let mut dns_servers: Vec<String> = Vec::new();

        for adapter in &ordered {
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
    if let Some(stdout) = run_stdout(
        "powershell",
        [
            "-NoProfile", "-Command",
            "(Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.InterfaceAlias -notmatch 'Loopback' -and $_.PrefixOrigin -ne 'WellKnown' } | Select-Object -First 1).IPAddress",
        ],
        CommandTimeout::Slow,
    )
    {
        let ip = stdout.trim().to_string();
        if !ip.is_empty() && ip != "127.0.0.1" {
            machine_ip = Some(ip);
        }
    }

    // Fallback: PowerShell for DNS
    let mut dns_servers = Vec::new();
    if let Some(stdout) = run_stdout(
        "powershell",
        [
            "-NoProfile", "-Command",
            "(Get-DnsClientServerAddress -AddressFamily IPv4 | Where-Object { $_.ServerAddresses } | Select-Object -ExpandProperty ServerAddresses) -join \"`n\"",
        ],
        CommandTimeout::Slow,
    )
    {
        for line in stdout.lines() {
            let ip = line.trim();
            if !ip.is_empty() && !dns_servers.contains(&ip.to_string()) {
                dns_servers.push(ip.to_string());
                if dns_servers.len() >= 5 {
                    break;
                }
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
    if let Some(info) = run_stdout(
        "cmd",
        ["/c", "bcdedit", "/enum", "{current}"],
        CommandTimeout::Normal,
    ) {
        let info = info.to_lowercase();
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
    if env::var("CURSOR_TRACE_ID").is_ok() || env::var("CURSOR_AGENT").is_ok() {
        return Some("Cursor".to_string());
    }

    // C.12 (v3.13.0+): walk the parent-process chain via Toolhelp32. Catches
    // Windows Terminal / WezTerm / Alacritty / Tabby / Hyper / Cursor / VS
    // Code launches that don't inherit a recognizable env var (e.g. when
    // the user spawned a fresh subshell that lost the parent's environment,
    // or launched via a desktop shortcut). ~5 ms cost; full-mode-only via
    // the existing collect()-time call site.
    if let Some(name) = detect_terminal_via_parent_walk() {
        return Some(name);
    }

    Some("Console".to_string())
}

/// Walk the parent-process chain (cap 10 levels) looking for a known
/// terminal-host executable name. Returns `None` if no match.
fn detect_terminal_via_parent_walk() -> Option<String> {
    use std::collections::HashMap;
    use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
    use winapi::um::processthreadsapi::GetCurrentProcessId;
    use winapi::um::tlhelp32::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    // SAFETY: CreateToolhelp32Snapshot returns INVALID_HANDLE_VALUE on
    // failure or a valid HANDLE on success; we check before walking.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return None;
    }

    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

    let mut pid_to_parent_name: HashMap<u32, (u32, String)> = HashMap::new();

    // SAFETY: snapshot is a valid HANDLE; entry.dwSize is set; W variants
    // expect UTF-16. Process32FirstW returns 0 on failure (incl. empty list).
    if unsafe { Process32FirstW(snapshot, &mut entry) } != 0 {
        loop {
            // szExeFile is null-terminated UTF-16, max 260 chars (MAX_PATH).
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
            pid_to_parent_name.insert(entry.th32ProcessID, (entry.th32ParentProcessID, name));
            if unsafe { Process32NextW(snapshot, &mut entry) } == 0 {
                break;
            }
        }
    }
    // SAFETY: snapshot was a valid HANDLE; CloseHandle accepts that.
    unsafe { CloseHandle(snapshot) };

    // Walk parent chain from current PID (cap 10 levels — defensive against
    // corrupted PID tables that could form cycles).
    let mut current_pid = unsafe { GetCurrentProcessId() };
    for _ in 0..10 {
        let (parent_pid, name) = match pid_to_parent_name.get(&current_pid) {
            Some(v) => v.clone(),
            None => break,
        };
        if let Some(label) = match_terminal_name(&name) {
            return Some(label.to_string());
        }
        if parent_pid == 0 || parent_pid == current_pid {
            break;
        }
        current_pid = parent_pid;
    }
    None
}

/// Match a process exe name (basename, with .exe) to a terminal label.
/// Case-insensitive. Returns `None` if not a recognized terminal host (the
/// caller keeps walking the parent chain). Includes AI-shell hosts (Claude
/// Code, Cursor, Windsurf) so users running TR-300 inside an AI agent see
/// the right thing instead of "Console".
fn match_terminal_name(exe: &str) -> Option<&'static str> {
    let lower = exe.to_lowercase();
    match lower.as_str() {
        "windowsterminal.exe" => Some("Windows Terminal"),
        "wezterm-gui.exe" | "wezterm.exe" => Some("WezTerm"),
        "alacritty.exe" => Some("Alacritty"),
        "code.exe" => Some("VS Code"),
        "cursor.exe" => Some("Cursor"),
        "windsurf.exe" => Some("Windsurf"),
        "hyper.exe" => Some("Hyper"),
        "tabby.exe" => Some("Tabby"),
        "ghostty.exe" => Some("Ghostty"),
        "kitty.exe" => Some("Kitty"),
        "mintty.exe" => Some("MinTTY"),
        "claude.exe" => Some("Claude Code"),
        "antigravity.exe" => Some("Antigravity"),
        // Intermediate process hosts — keep walking the parent chain.
        "conhost.exe" | "powershell.exe" | "pwsh.exe" | "cmd.exe" | "bash.exe" | "sh.exe"
        | "zsh.exe" | "fish.exe" | "nu.exe" | "tr300.exe" | "node.exe" | "python.exe"
        | "python3.exe" => None,
        _ => None,
    }
}

/// Get shell name and version
fn get_shell() -> Option<String> {
    // Check SHELL env var first (for Git Bash, etc.)
    if let Ok(shell) = env::var("SHELL") {
        if shell.contains("bash") {
            if let Some(version) = run_stdout("bash", ["--version"], CommandTimeout::Normal) {
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

    // C.11 (v3.13.0+): check PowerShell 7+ ("PowerShell Core") first. PSCore
    // installs register under HKLM\SOFTWARE\Microsoft\PowerShellCore\
    // InstalledVersions\<GUID>\SemanticVersion. We pick the highest version
    // string found (string compare works for 3-tuple semver). Falls back to
    // legacy Windows PowerShell 5.x detection below if no PSCore subkey.
    if let Some(v) = get_powershell_core_version() {
        return Some(format!("PowerShell {}", v));
    }

    // Check PowerShell version via registry (no subprocess)
    if let Some(stdout) = run_stdout(
        "reg",
        [
            "query",
            r"HKLM\SOFTWARE\Microsoft\PowerShell\3\PowerShellEngine",
            "/v",
            "PowerShellVersion",
        ],
        CommandTimeout::Normal,
    ) {
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

/// Recursive `reg query` of `HKLM\SOFTWARE\Microsoft\PowerShellCore\
/// InstalledVersions` returns lines like:
///   HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\PowerShellCore\InstalledVersions\<GUID>
///       SemanticVersion    REG_SZ    7.4.6
/// We pick the highest version found. Returns `None` if no PSCore subkey.
///
/// Versions are compared as semver tuples `(u64, u64, u64)` rather than by
/// string compare — naive string compare puts `"7.9.0" > "7.10.0"` because
/// `'9' > '1'`. PowerShell Core is at 7.5 today but will eventually ship a
/// 2-digit segment. (Caught in v3.13.0 Codex review.)
fn get_powershell_core_version() -> Option<String> {
    let stdout = run_stdout(
        "reg",
        [
            "query",
            r"HKLM\SOFTWARE\Microsoft\PowerShellCore\InstalledVersions",
            "/s",
            "/v",
            "SemanticVersion",
        ],
        CommandTimeout::Normal,
    )?;
    let mut best_tuple: Option<(u64, u64, u64)> = None;
    let mut best_string: Option<String> = None;
    for line in stdout.lines() {
        if line.contains("SemanticVersion") {
            // Format: "    SemanticVersion    REG_SZ    7.4.6"
            if let Some(version) = line.split_whitespace().last() {
                let version_clean = version.trim();
                if let Some(tuple) = parse_semver_tuple(version_clean) {
                    if best_tuple.map(|b| tuple > b).unwrap_or(true) {
                        best_tuple = Some(tuple);
                        best_string = Some(version_clean.to_string());
                    }
                }
            }
        }
    }
    best_string
}

/// Parse a 3-tuple semver string like "7.4.6" into `(major, minor, patch)`.
/// Pre-release / build-metadata suffixes (e.g. "7.5.0-preview.1") are
/// stripped from the patch segment before parsing. Returns `None` if the
/// string isn't shaped like 3 dot-separated integer segments.
fn parse_semver_tuple(s: &str) -> Option<(u64, u64, u64)> {
    let mut parts = s.splitn(3, '.');
    let major: u64 = parts.next()?.parse().ok()?;
    let minor: u64 = parts.next()?.parse().ok()?;
    // Strip any trailing pre-release / build-metadata before parsing patch.
    let patch_raw = parts.next()?;
    let patch_clean = patch_raw.split(['-', '+']).next().unwrap_or(patch_raw);
    let patch: u64 = patch_clean.parse().ok()?;
    Some((major, minor, patch))
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

#[derive(Default)]
struct WindowsPowerShellFallback {
    windows_edition: Option<String>,
    virtualization: Option<String>,
    gpus: Vec<String>,
    battery: Option<String>,
}

fn get_batched_powershell_fallback() -> WindowsPowerShellFallback {
    let script = r#"
$os = Get-CimInstance Win32_OperatingSystem
$cs = Get-CimInstance Win32_ComputerSystem
$gpu = @(Get-CimInstance Win32_VideoController | ForEach-Object { $_.Name })
$b = Get-CimInstance Win32_Battery
$battery = $null
if ($b) { $battery = "$($b.EstimatedChargeRemaining)% ($($b.BatteryStatus))" }
[pscustomobject]@{
  edition = $os.Caption
  computer = "$($cs.Manufacturer)|$($cs.Model)|$($cs.HypervisorPresent)"
  gpus = $gpu
  battery = $battery
} | ConvertTo-Json -Compress
"#;

    let Some(stdout) = run_stdout(
        "powershell",
        ["-NoProfile", "-Command", script],
        CommandTimeout::Slow,
    ) else {
        return WindowsPowerShellFallback::default();
    };
    parse_batched_powershell_fallback(&stdout).unwrap_or_default()
}

fn parse_batched_powershell_fallback(json: &str) -> Option<WindowsPowerShellFallback> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let windows_edition = value["edition"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let virtualization = value["computer"]
        .as_str()
        .and_then(parse_virtualization_from_ps_computer_system);
    let gpus = match &value["gpus"] {
        serde_json::Value::Array(values) => values
            .iter()
            .filter_map(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        serde_json::Value::String(s) if !s.trim().is_empty() => vec![s.trim().to_string()],
        _ => Vec::new(),
    };
    let battery = value["battery"]
        .as_str()
        .map(normalize_powershell_battery_status)
        .filter(|s| !s.is_empty() && s.contains('%'));

    Some(WindowsPowerShellFallback {
        windows_edition,
        virtualization,
        gpus,
        battery,
    })
}

fn get_windows_edition_ps() -> Option<String> {
    let caption = run_stdout(
        "powershell",
        [
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_OperatingSystem).Caption",
        ],
        CommandTimeout::Slow,
    )?
    .trim()
    .to_string();
    if caption.is_empty() {
        None
    } else {
        Some(caption)
    }
}

fn detect_virtualization_ps() -> Option<String> {
    let info = run_stdout(
        "powershell",
        [
            "-NoProfile", "-Command",
            "(Get-CimInstance Win32_ComputerSystem).Manufacturer + '|' + (Get-CimInstance Win32_ComputerSystem).Model + '|' + (Get-CimInstance Win32_ComputerSystem).HypervisorPresent",
        ],
        CommandTimeout::Slow,
    )?;
    parse_virtualization_from_ps_computer_system(&info)
}

fn parse_virtualization_from_ps_computer_system(info: &str) -> Option<String> {
    let info = info.to_lowercase();
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
    if let Some(stdout) = run_stdout(
        "powershell",
        [
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_VideoController).Name -join \"`n\"",
        ],
        CommandTimeout::Slow,
    ) {
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
    let battery = run_stdout(
        "powershell",
        [
            "-NoProfile", "-Command",
            "$b = Get-CimInstance Win32_Battery; if ($b) { \"$($b.EstimatedChargeRemaining)% ($($b.BatteryStatus))\" }",
        ],
        CommandTimeout::Slow,
    )?
    .trim()
    .to_string();
    if battery.is_empty() || !battery.contains('%') {
        return None;
    }
    Some(normalize_powershell_battery_status(&battery))
}

fn normalize_powershell_battery_status(battery: &str) -> String {
    battery
        .replace("(1)", "(Discharging)")
        .replace("(2)", "(AC Power)")
        .replace("(3)", "(Charging)")
        .replace("(4)", "(Low)")
        .replace("(5)", "(Critical)")
        .replace("(6)", "(Charging)")
        .replace("(7)", "(Charging High)")
        .replace("(8)", "(Charging Low)")
        .replace("(9)", "(Charging Critical)")
}

#[cfg(test)]
mod powershell_fallback_tests {
    use super::*;

    #[test]
    fn parses_batched_powershell_fallback_json() {
        let json = r#"{
            "edition": "Microsoft Windows 11 Pro",
            "computer": "Microsoft Corporation|Virtual Machine|True",
            "gpus": ["Intel Arc Graphics", "NVIDIA RTX"],
            "battery": "87% (2)"
        }"#;

        let parsed = parse_batched_powershell_fallback(json).expect("valid fallback JSON");
        assert_eq!(
            parsed.windows_edition.as_deref(),
            Some("Microsoft Windows 11 Pro")
        );
        assert_eq!(parsed.virtualization.as_deref(), Some("Hyper-V"));
        assert_eq!(
            parsed.gpus,
            vec!["Intel Arc Graphics".to_string(), "NVIDIA RTX".to_string()]
        );
        assert_eq!(parsed.battery.as_deref(), Some("87% (AC Power)"));
    }

    #[test]
    fn parses_single_gpu_string_from_batched_fallback() {
        let json = r#"{
            "edition": "",
            "computer": "QEMU|Standard PC|True",
            "gpus": "Virtio GPU",
            "battery": null
        }"#;

        let parsed = parse_batched_powershell_fallback(json).expect("valid fallback JSON");
        assert_eq!(parsed.windows_edition, None);
        assert_eq!(parsed.virtualization.as_deref(), Some("QEMU"));
        assert_eq!(parsed.gpus, vec!["Virtio GPU".to_string()]);
        assert_eq!(parsed.battery, None);
    }
}

// --- C.8: GPU software-adapter filter (v3.13.0+) ---
//
// The Win32_VideoController WMI path historically returned
// "Microsoft Basic Render Driver" and "Microsoft Hyper-V Video" on some
// configurations alongside the real hardware adapter. The registry-based
// `get_gpus_fast()` doesn't enumerate those (the {4d36e968-...} Display
// class only contains hardware adapters), so preferring registry-first
// + filtering known software names by string is a layered safeguard
// against future WMI configs that re-introduce the issue. Strict
// substring match on `Description` — case-insensitive.
fn filter_software_gpus(gpus: Vec<String>) -> Vec<String> {
    const SOFTWARE_GPU_NEEDLES: &[&str] = &[
        "Microsoft Basic Render Driver",
        "Microsoft Basic Display",
        "Microsoft Hyper-V Video",
        "Microsoft Remote Display Adapter",
        "Microsoft Indirect Display",
        "RDPDD Chained DD",
        "RDP Encoder Mirror",
    ];
    gpus.into_iter()
        .filter(|name| {
            let lower = name.to_lowercase();
            !SOFTWARE_GPU_NEEDLES
                .iter()
                .any(|n| lower.contains(&n.to_lowercase()))
        })
        .collect()
}

// --- C.10: native battery via GetSystemPowerStatus (v3.13.0+) ---
//
// `GetSystemPowerStatus` is a single Win32 API call (~1 ms) that returns
// charge percentage + AC/charging flags. The historical WMI `Win32_Battery`
// path costs ~40 ms on laptops because of the COM round-trip; replacing it
// is the cheapest single speed win in PR #5.
// Reference: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getsystempowerstatus

// BATTERY_FLAG_HIGH (0x01, >66% charge) is intentionally unused — the
// percentage already conveys charge level; we don't need a "(High)" label.
const BATTERY_FLAG_LOW: u8 = 0x02; // < 33%
const BATTERY_FLAG_CRITICAL: u8 = 0x04; // < 5%
const BATTERY_FLAG_CHARGING: u8 = 0x08;
const BATTERY_FLAG_NO_BATTERY: u8 = 0x80;
const BATTERY_FLAG_UNKNOWN: u8 = 0xFF;

fn get_battery_native() -> Option<String> {
    let mut sps: winapi::um::winbase::SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    // SAFETY: `GetSystemPowerStatus` writes a SYSTEM_POWER_STATUS struct to
    // the supplied pointer. We pass a stack-allocated, zero-initialized
    // struct of exactly that type. Returns nonzero on success.
    let ok = unsafe { winapi::um::winbase::GetSystemPowerStatus(&mut sps) };
    if ok == 0 {
        return None;
    }
    // BatteryFlag = 128 (0x80) → no system battery (desktops).
    if sps.BatteryFlag == BATTERY_FLAG_NO_BATTERY {
        return None;
    }
    // BatteryLifePercent: 0-100, or 255 (0xFF) when unknown. Skip on unknown
    // so we never render "255% (Unknown)".
    let percent = sps.BatteryLifePercent;
    if percent == 0xFF {
        return None;
    }
    if sps.BatteryFlag == BATTERY_FLAG_UNKNOWN {
        return Some(format!("{}% (Unknown)", percent));
    }

    // 3-state model (v3.13.0+, gaming-laptop friendly):
    //
    //   1. On AC, fully topped up (≥ 95%, not actively charging) → "AC Power"
    //      with no percentage. The battery is full and idle; the percentage
    //      is uninformative and adds noise.
    //   2. On AC, battery still in play (charging OR firmware-limited
    //      charging OR PSU undersized for peak load — common on Alienware /
    //      ROG / Razer with discrete GPUs that can momentarily exceed the
    //      brick's wattage, OR on ThinkPad / ASUS with battery-longevity
    //      modes capping charge at 60-80%) → "X% (Plugged in)" or
    //      "X% (Charging)" so the percentage is visible. Distinguishes
    //      "supplementing from battery while plugged in" from "discharging
    //      while unplugged".
    //   3. Off AC (battery only) → "X% (Discharging)" / "X% (Low)" /
    //      "X% (Critical)" depending on flags. Critical/Low take precedence
    //      so the user sees the urgency.
    //
    // The Windows API doesn't directly expose "battery is currently
    // supplementing AC" — we infer it from `ACLineStatus == 1 (online) AND
    // CHARGING bit not set AND percent < 95`. That covers both the "PSU
    // can't keep up" and "firmware-limited charging" scenarios.
    // ACLineStatus: 0 = offline (battery), 1 = online (AC), 255 = unknown.
    // The unknown case is rare but real (some VMs, some hypervisor-passthrough
    // batteries); show the percentage with no AC label rather than guessing.
    let ac_status = sps.ACLineStatus;
    let on_ac = ac_status == 1;
    let ac_unknown = ac_status == 0xFF;
    let charging = sps.BatteryFlag & BATTERY_FLAG_CHARGING != 0;
    let critical = sps.BatteryFlag & BATTERY_FLAG_CRITICAL != 0;
    let low = sps.BatteryFlag & BATTERY_FLAG_LOW != 0;

    if ac_unknown {
        // We have charge level but no idea if it's plugged in. Honest output:
        // just the percentage. Better than fabricating "AC Power" or
        // "Discharging".
        return Some(format!("{}%", percent));
    }

    if on_ac {
        if charging {
            return Some(format!("{}% (Charging)", percent));
        }
        // Fully topped up on AC: percentage is noise. Just say "AC Power".
        if percent >= 95 {
            return Some("AC Power".to_string());
        }
        // On AC but battery is at < 95% and not charging — either firmware
        // is intentionally holding charge low (battery-longevity mode) OR
        // the PSU can't keep up with the current load (gaming-laptop case).
        // Either way, the percentage matters.
        return Some(format!("{}% (Plugged in)", percent));
    }

    // Off AC.
    let label = if critical {
        "Critical"
    } else if low {
        "Low"
    } else {
        "Discharging"
    };
    Some(format!("{}% ({})", percent, label))
}

// --- C.4: VPN-aware default-route detection (v3.12.0+) ---
//
// `GetBestInterfaceEx` asks the kernel which interface index it would route
// a packet to a given destination through. Pointing it at a public IPv4
// (1.1.1.1) gives us the interface carrying real internet traffic — correct
// on multi-homed hosts and on hosts with WireGuard / Tailscale / OpenVPN /
// Cisco AnyConnect tunnels active. Reference:
// https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getbestinterfaceex
//
// Declared as a manual `extern` because we want the broader `winapi` feature
// set under `iphlpapi` to stay opt-in elsewhere; we only need this single
// symbol here. `SOCKADDR_IN` is declared inline (3 fields, fixed layout, has
// been stable since Win95) for the same reason.
#[link(name = "iphlpapi")]
extern "system" {
    fn GetBestInterfaceEx(pDestAddr: *mut SockaddrIn, pdwBestIfIndex: *mut u32) -> u32;
}

#[repr(C)]
struct SockaddrIn {
    sin_family: u16, // AF_INET = 2
    sin_port: u16,
    sin_addr: u32, // network byte order (big-endian)
    sin_zero: [u8; 8],
}

const AF_INET: u16 = 2;

/// Look up the interface index Windows would use to reach 1.1.1.1.
/// Returns `None` on any failure — caller must fall back to non-best-route logic.
fn get_best_route_interface_index() -> Option<u32> {
    // SOCKADDR_IN.sin_addr.s_addr is documented as "network byte order" — the
    // first octet of the IP must occupy the lowest-addressed byte of the
    // 4-byte field. On little-endian Windows (x86_64 + ARM64), constructing
    // the u32 via `from_le_bytes` produces a value that, when stored as
    // `sin_addr: u32` in our `repr(C)` struct, writes bytes `01 01 01 01`
    // (= IP 1.1.1.1) into memory in the right order. Using `from_be_bytes`
    // would happen to work for palindromic addresses like 1.1.1.1 but break
    // on anything else (e.g. 8.4.4.8) — we use the right idiom so the code
    // survives a future destination-address change.
    let sin_addr: u32 = u32::from_le_bytes([1, 1, 1, 1]);

    let mut sa = SockaddrIn {
        sin_family: AF_INET,
        sin_port: 0,
        sin_addr,
        sin_zero: [0u8; 8],
    };
    let mut best_index: u32 = 0;

    // SAFETY: `GetBestInterfaceEx` reads `sa` (a valid SOCKADDR_IN we own on
    // the stack) and writes a single `u32` to `best_index`. Both pointers are
    // non-null and properly aligned. The function returns `NO_ERROR (0)` on
    // success.
    let status = unsafe { GetBestInterfaceEx(&mut sa, &mut best_index) };
    if status == 0 {
        Some(best_index)
    } else {
        None
    }
}

// --- C.5: Fast Startup uptime annotation (v3.12.0+) ---
//
// Win10/Win11 default to "Fast Startup" (HiberbootEnabled=1), which
// hibernates the kernel session at shutdown and resumes it at next boot.
// `GetTickCount64` (and sysinfo's `System::uptime()`) report time-since the
// CURRENT kernel session started — usually a few hours. `LastBootUpTime`
// from WMI's Win32_OperatingSystem reports the LAST COLD BOOT time — often
// weeks ago on laptops that get used daily. We surface both so the table is
// honest about what's happening:
//   "9d 4h 12m (session: 7h 14m)"
// References:
//   https://learn.microsoft.com/en-us/answers/questions/1443763/how-to-get-oss-start-time-when-fast-startup-mode-i
//   https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-operatingsystem

/// Read `HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Power` value
/// `HiberbootEnabled` (DWORD). Returns `true` only when explicitly = 1.
/// Default-safe: any read or parse error returns `false`.
pub fn detect_fast_startup() -> bool {
    let stdout = match run_stdout(
        "reg",
        [
            "query",
            r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Power",
            "/v",
            "HiberbootEnabled",
        ],
        CommandTimeout::Normal,
    ) {
        Some(stdout) => stdout,
        None => return false,
    };
    for line in stdout.lines() {
        if line.contains("HiberbootEnabled") {
            // Format: "    HiberbootEnabled    REG_DWORD    0x1"
            if let Some(value) = line.split_whitespace().last() {
                let trimmed = value.trim_start_matches("0x");
                if let Ok(v) = u32::from_str_radix(trimmed, 16) {
                    return v == 1;
                }
            }
        }
    }
    false
}

/// Query `Win32_OperatingSystem.LastBootUpTime` and convert to seconds elapsed
/// since the last *cold* boot. Returns `None` on any WMI/parse failure.
///
/// Uses `wmi::WMIDateTime` which the `wmi` crate's serde deserializer parses
/// from the CIM datetime format (`yyyymmddHHMMSS.mmmmmmsUUU`) into a
/// `chrono::DateTime<chrono::FixedOffset>`.
pub fn last_cold_boot_seconds() -> Option<u64> {
    #[derive(Deserialize)]
    #[serde(rename = "Win32_OperatingSystem")]
    #[serde(rename_all = "PascalCase")]
    struct OsBoot {
        last_boot_up_time: Option<wmi::WMIDateTime>,
    }

    let com = COMLibrary::new().ok()?;
    let wmi = WMIConnection::new(com).ok()?;
    let results: Vec<OsBoot> = wmi.query().ok()?;
    let boot = results.into_iter().next()?.last_boot_up_time?.0;

    let elapsed = chrono::Utc::now()
        .signed_duration_since(boot.with_timezone(&chrono::Utc))
        .num_seconds();
    if elapsed < 0 {
        None
    } else {
        Some(elapsed as u64)
    }
}
