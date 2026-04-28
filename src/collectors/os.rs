//! Operating system information collector

use crate::collectors::CollectMode;
use crate::error::Result;
use sysinfo::System;

/// Operating system information
#[derive(Debug, Clone)]
pub struct OsInfo {
    /// OS name (e.g., "Windows 11", "macOS", "Ubuntu")
    pub name: String,
    /// OS version string
    pub version: String,
    /// Kernel version
    pub kernel_version: String,
    /// System hostname
    pub hostname: String,
    /// System architecture (e.g., "x86_64", "aarch64")
    pub architecture: String,
    /// System uptime in seconds. On Windows with Fast Startup enabled, this is
    /// the *cold boot* time when divergent from the kernel session start;
    /// otherwise it matches `System::uptime()`.
    pub uptime_seconds: u64,
    /// Current kernel-session uptime in seconds. Only `Some(_)` on Windows when
    /// Fast Startup hibernated the kernel and the resumed session differs
    /// meaningfully from the cold-boot time. Drives the
    /// `UPTIME … (session: …)` annotation in the table renderer.
    pub session_uptime_seconds: Option<u64>,
}

/// Collect OS information.
///
/// `mode` gates the slower per-platform enrichments (e.g. Windows Fast Startup
/// cold-boot WMI query in full mode only).
pub fn collect(mode: CollectMode) -> Result<OsInfo> {
    let name = System::name().unwrap_or_else(|| "Unknown".to_string());
    let version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
    let kernel_version = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
    let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());
    let architecture = std::env::consts::ARCH.to_string();
    let uptime_seconds = System::uptime();

    // On Windows, the registry's ProductName is frozen at "Windows 10" even on
    // Windows 11 — we detect Win11 by CurrentBuild >= 22000 and enrich the
    // version with DisplayVersion ("24H2") and UBR for a more accurate display.
    // Reference: Microsoft Q&A: "Windows 11 ProductName in registry"
    // https://learn.microsoft.com/en-us/answers/questions/555857/windows-11-product-name-in-registry
    #[cfg(target_os = "windows")]
    let (name, version, kernel_version) =
        match crate::collectors::platform::windows::get_os_info_from_registry() {
            Some((win_name, win_version, win_kernel)) => (win_name, win_version, win_kernel),
            None => (name, version, kernel_version),
        };

    // Windows Fast Startup: when HiberbootEnabled=1 and the WMI cold-boot time
    // is meaningfully older (>1h) than sysinfo's session uptime, surface BOTH
    // numbers. The "primary" uptime becomes the cold-boot time (matches what
    // users intuitively mean by "since I last restarted") and the session
    // uptime gets surfaced as a parenthetical annotation in the table.
    // Skipped in fast mode (~80 ms WMI cost).
    #[cfg(target_os = "windows")]
    let (uptime_seconds, session_uptime_seconds) = if mode == CollectMode::Full {
        if crate::collectors::platform::windows::detect_fast_startup() {
            match crate::collectors::platform::windows::last_cold_boot_seconds() {
                Some(cold) if cold > uptime_seconds + 3600 => (cold, Some(uptime_seconds)),
                _ => (uptime_seconds, None),
            }
        } else {
            (uptime_seconds, None)
        }
    } else {
        (uptime_seconds, None)
    };

    #[cfg(not(target_os = "windows"))]
    let session_uptime_seconds: Option<u64> = {
        let _ = mode;
        None
    };

    Ok(OsInfo {
        name,
        version,
        kernel_version,
        hostname,
        architecture,
        uptime_seconds,
        session_uptime_seconds,
    })
}

impl OsInfo {
    /// Format uptime as human-readable string
    pub fn uptime_formatted(&self) -> String {
        let days = self.uptime_seconds / 86400;
        let hours = (self.uptime_seconds % 86400) / 3600;
        let minutes = (self.uptime_seconds % 3600) / 60;

        if days > 0 {
            format!("{}d {}h {}m", days, hours, minutes)
        } else if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    }

    /// Get full OS string
    pub fn full_name(&self) -> String {
        format!("{} {}", self.name, self.version)
    }
}
