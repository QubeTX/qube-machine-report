//! Platform-specific collectors
//!
//! These modules provide platform-specific system information
//! that cannot be obtained through cross-platform libraries.

use super::CollectMode;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

/// Platform-specific extended information
#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub struct PlatformInfo {
    /// OS build identifier when the platform exposes one separately.
    pub os_build: Option<String>,
    /// Desktop environment (Linux)
    pub desktop_environment: Option<String>,
    /// Display server (Linux: X11/Wayland)
    pub display_server: Option<String>,
    /// Windows edition (Windows)
    pub windows_edition: Option<String>,
    /// macOS codename
    pub macos_codename: Option<String>,
    /// Boot mode (UEFI/Legacy)
    pub boot_mode: Option<String>,
    /// Virtualization platform if running in VM
    pub virtualization: Option<String>,
    /// GPU names
    pub gpus: Vec<String>,
    /// System architecture (x86_64, aarch64, etc.)
    pub architecture: Option<String>,
    /// Hardware model / marketing name when available.
    pub machine_model: Option<String>,
    /// CPU topology details such as Apple Silicon P/E core split.
    pub cpu_core_topology: Option<String>,
    /// Terminal emulator name
    pub terminal: Option<String>,
    /// Shell name and version
    pub shell: Option<String>,
    /// Display resolution
    pub display_resolution: Option<String>,
    /// Battery status (percentage and charging state)
    pub battery: Option<String>,
    /// ZFS pool health summary when zpool is available.
    pub zfs_health: Option<String>,
    /// Motherboard/baseboard summary, populated only when safely readable.
    pub motherboard: Option<String>,
    /// BIOS/firmware summary, populated only when safely readable.
    pub bios: Option<String>,
    /// RAM slot summary, populated only when safely readable.
    pub ram_slots: Option<String>,
    /// System locale
    pub locale: Option<String>,
    /// Disk encryption status — BitLocker on Windows, FileVault on macOS, LUKS on Linux.
    /// Only populated when the data is readable in the current security context;
    /// otherwise `None` (unelevated users may see this gap on certain configurations).
    pub encryption: Option<String>,
    /// Whether a probe specifically failed because the current process lacks
    /// privileges and elevation is expected to unlock useful data.
    pub elevation_unlocks_more: bool,
}

/// Collect platform-specific information
pub fn collect(mode: CollectMode) -> PlatformInfo {
    #[cfg(target_os = "linux")]
    {
        linux::collect(mode)
    }

    #[cfg(target_os = "macos")]
    {
        macos::collect(mode)
    }

    #[cfg(target_os = "windows")]
    {
        windows::collect(mode)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = mode;
        PlatformInfo::default()
    }
}
