//! Operating system information collector

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
    /// System uptime in seconds
    pub uptime_seconds: u64,
}

/// Collect OS information
pub fn collect() -> Result<OsInfo> {
    let name = System::name().unwrap_or_else(|| "Unknown".to_string());
    let version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
    let kernel_version = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
    let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());
    let architecture = std::env::consts::ARCH.to_string();
    let uptime_seconds = System::uptime();

    Ok(OsInfo {
        name,
        version,
        kernel_version,
        hostname,
        architecture,
        uptime_seconds,
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
