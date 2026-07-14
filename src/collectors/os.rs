//! Operating system information collector

use crate::collectors::CollectMode;
use crate::error::Result;
use sysinfo::System;

/// Operating system information
#[non_exhaustive]
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
    /// Operating-system reported uptime in seconds (`System::uptime()`).
    pub uptime_seconds: u64,
    /// Optional independently established session uptime. Currently `None` on
    /// all supported platforms; retained for public API compatibility.
    pub session_uptime_seconds: Option<u64>,
}

/// Collect OS information.
///
/// `mode` is retained for API compatibility and future mode-aware OS fields.
pub fn collect(mode: CollectMode) -> Result<OsInfo> {
    #[cfg(not(target_os = "macos"))]
    let name = System::name().unwrap_or_else(|| "Unknown".to_string());
    let version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
    #[cfg(target_os = "macos")]
    let name = "macOS".to_string();
    #[cfg(target_os = "macos")]
    let version = crate::collectors::command::run_stdout(
        "sw_vers",
        ["-productVersion"],
        crate::collectors::command::CommandTimeout::Normal,
    )
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
    .unwrap_or(version);
    let kernel_version = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
    let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());
    let architecture = System::cpu_arch().unwrap_or_else(|| std::env::consts::ARCH.to_string());
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

    // Uptime is elapsed kernel/system time on every platform. Do not relabel a
    // second boot-time estimate as user-session duration; Windows Fast Startup
    // does not make either source a login timestamp.
    let session_uptime_seconds: Option<u64> = None;
    let _ = mode;

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
        } else if minutes > 0 {
            format!("{}m", minutes)
        } else {
            format!("{}s", self.uptime_seconds)
        }
    }

    /// Get full OS string
    pub fn full_name(&self) -> String {
        format!("{} {}", self.name, self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info_with_uptime(uptime_seconds: u64) -> OsInfo {
        OsInfo {
            name: "Test OS".to_string(),
            version: "1".to_string(),
            kernel_version: "1".to_string(),
            hostname: "test".to_string(),
            architecture: "test".to_string(),
            uptime_seconds,
            session_uptime_seconds: None,
        }
    }

    #[test]
    fn uptime_under_one_minute_is_not_displayed_as_zero_minutes() {
        assert_eq!(info_with_uptime(0).uptime_formatted(), "0s");
        assert_eq!(info_with_uptime(59).uptime_formatted(), "59s");
        assert_eq!(info_with_uptime(60).uptime_formatted(), "1m");
    }
}
