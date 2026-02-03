//! Linux-specific information collectors

use super::PlatformInfo;
use std::env;
use std::fs;
use std::path::Path;

/// Collect Linux-specific information
pub fn collect() -> PlatformInfo {
    PlatformInfo {
        desktop_environment: detect_desktop_environment(),
        display_server: detect_display_server(),
        boot_mode: detect_boot_mode(),
        virtualization: detect_virtualization(),
        windows_edition: None,
        macos_codename: None,
    }
}

/// Detect the desktop environment
fn detect_desktop_environment() -> Option<String> {
    // Check XDG_CURRENT_DESKTOP first
    if let Ok(de) = env::var("XDG_CURRENT_DESKTOP") {
        return Some(de);
    }

    // Check DESKTOP_SESSION
    if let Ok(session) = env::var("DESKTOP_SESSION") {
        return Some(session);
    }

    // Check for specific environment variables
    if env::var("GNOME_DESKTOP_SESSION_ID").is_ok() {
        return Some("GNOME".to_string());
    }

    if env::var("KDE_FULL_SESSION").is_ok() {
        return Some("KDE".to_string());
    }

    None
}

/// Detect the display server (X11 or Wayland)
fn detect_display_server() -> Option<String> {
    if let Ok(session_type) = env::var("XDG_SESSION_TYPE") {
        return Some(session_type);
    }

    if env::var("WAYLAND_DISPLAY").is_ok() {
        return Some("wayland".to_string());
    }

    if env::var("DISPLAY").is_ok() {
        return Some("x11".to_string());
    }

    None
}

/// Detect boot mode (UEFI or Legacy BIOS)
fn detect_boot_mode() -> Option<String> {
    if Path::new("/sys/firmware/efi").exists() {
        Some("UEFI".to_string())
    } else {
        Some("Legacy BIOS".to_string())
    }
}

/// Detect if running in a virtual machine
fn detect_virtualization() -> Option<String> {
    // Check /sys/class/dmi/id/product_name
    if let Ok(product) = fs::read_to_string("/sys/class/dmi/id/product_name") {
        let product = product.trim().to_lowercase();
        if product.contains("virtualbox") {
            return Some("VirtualBox".to_string());
        }
        if product.contains("vmware") {
            return Some("VMware".to_string());
        }
        if product.contains("qemu") || product.contains("kvm") {
            return Some("QEMU/KVM".to_string());
        }
        if product.contains("hyper-v") {
            return Some("Hyper-V".to_string());
        }
    }

    // Check /proc/cpuinfo for hypervisor flag
    if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
        if cpuinfo.contains("hypervisor") {
            return Some("Virtual Machine".to_string());
        }
    }

    None
}
