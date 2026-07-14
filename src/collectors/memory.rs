//! Memory information collector

use crate::collectors::CollectMode;
use crate::error::Result;
use sysinfo::System;

/// Memory information
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    /// Total physical memory in bytes
    pub total_bytes: u64,
    /// Used physical memory in bytes
    pub used_bytes: u64,
    /// Definition used for `used_bytes` on this platform.
    pub usage_kind: String,
    /// Available physical memory in bytes
    pub available_bytes: u64,
    /// Definition used for `available_bytes` on this platform.
    pub availability_kind: String,
    /// Total swap in bytes
    pub swap_total_bytes: u64,
    /// Used swap in bytes
    pub swap_used_bytes: u64,
}

/// Collect memory information
pub fn collect() -> Result<MemoryInfo> {
    collect_with_mode(CollectMode::Full)
}

/// Collect memory information while honoring the fast-mode subprocess budget.
pub fn collect_with_mode(mode: CollectMode) -> Result<MemoryInfo> {
    let mut sys = System::new();
    sys.refresh_memory();

    let total_bytes = sys.total_memory();
    let (used_bytes, available_bytes, usage_kind, availability_kind) =
        platform_memory_bytes(sys.used_memory(), sys.available_memory(), total_bytes, mode);

    Ok(MemoryInfo {
        total_bytes,
        used_bytes,
        usage_kind,
        available_bytes,
        availability_kind,
        swap_total_bytes: sys.total_swap(),
        swap_used_bytes: sys.used_swap(),
    })
}

#[cfg(target_os = "macos")]
fn platform_memory_bytes(
    sysinfo_used: u64,
    _sysinfo_available: u64,
    total: u64,
    mode: CollectMode,
) -> (u64, u64, String, String) {
    let _ = mode;
    if let Some(vm_stat_used) = macos_vm_stat_used_bytes() {
        let used = vm_stat_used.min(total);
        return (
            used,
            total.saturating_sub(used),
            "active+wired+compressed".to_string(),
            "total-minus-reported-used".to_string(),
        );
    }
    let used = sysinfo_used.min(total);
    (
        used,
        total.saturating_sub(used),
        "sysinfo-used".to_string(),
        "total-minus-reported-used".to_string(),
    )
}

#[cfg(not(target_os = "macos"))]
fn platform_memory_bytes(
    _sysinfo_used: u64,
    sysinfo_available: u64,
    total: u64,
    _mode: CollectMode,
) -> (u64, u64, String, String) {
    let available = sysinfo_available.min(total);
    (
        total.saturating_sub(available),
        available,
        "total-minus-operating-system-available".to_string(),
        "operating-system-available".to_string(),
    )
}

#[cfg(target_os = "macos")]
fn macos_vm_stat_used_bytes() -> Option<u64> {
    let stdout = crate::collectors::command::run_stdout(
        "vm_stat",
        std::iter::empty::<&str>(),
        crate::collectors::command::CommandTimeout::Normal,
    )?;
    parse_vm_stat_used_bytes(&stdout)
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn parse_vm_stat_used_bytes(output: &str) -> Option<u64> {
    let mut page_size = None;
    let mut active = None;
    let mut wired = None;
    let mut compressed = None;

    for line in output.lines() {
        if line.contains("page size of") {
            let digits = line
                .split_whitespace()
                .find_map(|word| word.parse::<u64>().ok());
            page_size = digits;
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let pages = value
            .trim()
            .trim_end_matches('.')
            .replace('.', "")
            .parse::<u64>()
            .ok();
        match key.trim() {
            "Pages active" => active = pages,
            "Pages wired down" => wired = pages,
            "Pages occupied by compressor" => compressed = pages,
            _ => {}
        }
    }

    Some(
        active?
            .saturating_add(wired?)
            .saturating_add(compressed.unwrap_or(0))
            .saturating_mul(page_size?),
    )
}

impl MemoryInfo {
    /// Get memory usage percentage
    pub fn usage_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.used_bytes as f64 / self.total_bytes as f64 * 100.0) as f32
        }
    }

    /// Get swap usage percentage
    pub fn swap_usage_percent(&self) -> f32 {
        if self.swap_total_bytes == 0 {
            0.0
        } else {
            (self.swap_used_bytes as f64 / self.swap_total_bytes as f64 * 100.0) as f32
        }
    }

    /// Format bytes as human-readable string
    pub fn format_bytes(bytes: u64) -> String {
        crate::format_bytes(bytes)
    }

    /// Get formatted total memory
    pub fn total_formatted(&self) -> String {
        Self::format_bytes(self.total_bytes)
    }

    /// Get formatted used memory
    pub fn used_formatted(&self) -> String {
        Self::format_bytes(self.used_bytes)
    }

    /// Get formatted available memory
    pub fn available_formatted(&self) -> String {
        Self::format_bytes(self.available_bytes)
    }

    /// Get memory usage string
    pub fn usage_string(&self) -> String {
        format!(
            "{} / {} ({:.1}%)",
            self.used_formatted(),
            self.total_formatted(),
            self.usage_percent()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vm_stat_used_memory_components() {
        let vm_stat = "\
Mach Virtual Memory Statistics: (page size of 16384 bytes)
Pages active:                               10.
Pages wired down:                           20.
Pages occupied by compressor:               30.
";
        assert_eq!(parse_vm_stat_used_bytes(vm_stat), Some(60 * 16384));
    }
}
