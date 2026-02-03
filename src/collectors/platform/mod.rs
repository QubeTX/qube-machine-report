//! Platform-specific collectors
//!
//! These modules provide platform-specific system information
//! that cannot be obtained through cross-platform libraries.

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

/// Platform-specific extended information
#[derive(Debug, Clone, Default)]
pub struct PlatformInfo {
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
    /// Terminal emulator name
    pub terminal: Option<String>,
    /// Shell name and version
    pub shell: Option<String>,
    /// Display resolution
    pub display_resolution: Option<String>,
    /// Battery status (percentage and charging state)
    pub battery: Option<String>,
    /// System locale
    pub locale: Option<String>,
}

/// Collect platform-specific information
pub fn collect() -> PlatformInfo {
    #[cfg(target_os = "linux")]
    {
        linux::collect()
    }

    #[cfg(target_os = "macos")]
    {
        macos::collect()
    }

    #[cfg(target_os = "windows")]
    {
        windows::collect()
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        PlatformInfo::default()
    }
}
