//! Windows-specific information collectors

use super::PlatformInfo;
use std::process::Command;

/// Collect Windows-specific information
pub fn collect() -> PlatformInfo {
    PlatformInfo {
        windows_edition: get_windows_edition(),
        boot_mode: detect_boot_mode(),
        virtualization: detect_virtualization(),
        desktop_environment: Some("Windows Shell".to_string()),
        display_server: Some("DWM".to_string()),
        macos_codename: None,
    }
}

/// Get Windows edition (Home, Pro, Enterprise, etc.)
fn get_windows_edition() -> Option<String> {
    // Use WMI query via PowerShell
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

/// Detect boot mode (UEFI or Legacy BIOS)
fn detect_boot_mode() -> Option<String> {
    // Check firmware type via PowerShell
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "$env:firmware_type",
        ])
        .output()
        .ok()?;

    let firmware = String::from_utf8_lossy(&output.stdout).trim().to_uppercase();

    if firmware.contains("UEFI") {
        Some("UEFI".to_string())
    } else if firmware.contains("LEGACY") || firmware.contains("BIOS") {
        Some("Legacy BIOS".to_string())
    } else {
        // Alternative check using bcdedit
        let output = Command::new("cmd")
            .args(["/c", "bcdedit", "/enum", "{current}"])
            .output()
            .ok()?;

        let info = String::from_utf8_lossy(&output.stdout).to_lowercase();
        if info.contains("winload.efi") {
            Some("UEFI".to_string())
        } else {
            Some("Legacy BIOS".to_string())
        }
    }
}

/// Detect if running in a virtual machine
fn detect_virtualization() -> Option<String> {
    // Check system manufacturer via WMI
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_ComputerSystem).Manufacturer + '|' + (Get-CimInstance Win32_ComputerSystem).Model",
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

    // Check for Hyper-V specifically
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_ComputerSystem).HypervisorPresent",
        ])
        .output()
        .ok()?;

    let hypervisor = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
    if hypervisor == "true" {
        return Some("Hypervisor Present".to_string());
    }

    None
}
