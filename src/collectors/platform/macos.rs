//! macOS-specific information collectors

use super::PlatformInfo;
use std::process::Command;

/// Collect macOS-specific information
pub fn collect() -> PlatformInfo {
    PlatformInfo {
        macos_codename: get_macos_codename(),
        boot_mode: detect_boot_mode(),
        virtualization: detect_virtualization(),
        desktop_environment: Some("Aqua".to_string()),
        display_server: Some("Quartz".to_string()),
        windows_edition: None,
    }
}

/// Get macOS version codename
fn get_macos_codename() -> Option<String> {
    // Get the OS version from sw_vers
    let output = Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()?;

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let major: u32 = version.split('.').next()?.parse().ok()?;

    // Map major version to codename
    let codename = match major {
        15 => "Sequoia",
        14 => "Sonoma",
        13 => "Ventura",
        12 => "Monterey",
        11 => "Big Sur",
        10 => "Catalina", // or earlier
        _ => return None,
    };

    Some(codename.to_string())
}

/// Detect boot mode (always UEFI on Intel Macs, native on Apple Silicon)
fn detect_boot_mode() -> Option<String> {
    // Check if running on Apple Silicon
    let output = Command::new("uname")
        .arg("-m")
        .output()
        .ok()?;

    let arch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if arch == "arm64" {
        Some("Apple Silicon".to_string())
    } else {
        Some("UEFI".to_string())
    }
}

/// Detect if running in a virtual machine
fn detect_virtualization() -> Option<String> {
    // Check system_profiler for VM indicators
    let output = Command::new("system_profiler")
        .arg("SPHardwareDataType")
        .output()
        .ok()?;

    let info = String::from_utf8_lossy(&output.stdout).to_lowercase();

    if info.contains("vmware") {
        return Some("VMware".to_string());
    }
    if info.contains("virtualbox") {
        return Some("VirtualBox".to_string());
    }
    if info.contains("parallels") {
        return Some("Parallels".to_string());
    }
    if info.contains("qemu") {
        return Some("QEMU".to_string());
    }

    // Check for Apple Virtualization framework
    let output = Command::new("sysctl")
        .arg("-n")
        .arg("kern.hv_vmm_present")
        .output()
        .ok()?;

    let vmm = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if vmm == "1" {
        return Some("Virtual Machine".to_string());
    }

    None
}
