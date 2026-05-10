//! Memory information collector

use crate::error::Result;
use sysinfo::System;

/// Memory information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    /// Total physical memory in bytes
    pub total_bytes: u64,
    /// Used physical memory in bytes
    pub used_bytes: u64,
    /// Available physical memory in bytes
    pub available_bytes: u64,
    /// Total swap in bytes
    pub swap_total_bytes: u64,
    /// Used swap in bytes
    pub swap_used_bytes: u64,
}

/// Collect memory information
pub fn collect() -> Result<MemoryInfo> {
    let mut sys = System::new();
    sys.refresh_memory();

    let mut used_bytes = sys.used_memory();
    #[cfg(target_os = "macos")]
    {
        if let Some(activity_monitor_used) = macos_activity_monitor_used_bytes() {
            let sysinfo_used = used_bytes.max(1);
            let delta = activity_monitor_used.abs_diff(sysinfo_used);
            if delta as f64 / sysinfo_used as f64 > 0.05 {
                used_bytes = activity_monitor_used;
            }
        }
    }

    Ok(MemoryInfo {
        total_bytes: sys.total_memory(),
        used_bytes,
        available_bytes: sys.available_memory(),
        swap_total_bytes: sys.total_swap(),
        swap_used_bytes: sys.used_swap(),
    })
}

#[cfg(target_os = "macos")]
fn macos_activity_monitor_used_bytes() -> Option<u64> {
    let stdout = crate::collectors::command::run_stdout(
        "vm_stat",
        std::iter::empty::<&str>(),
        crate::collectors::command::CommandTimeout::Normal,
    )?;
    parse_vm_stat_used_bytes(&stdout)
}

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
    fn parses_vm_stat_activity_monitor_used_memory() {
        let vm_stat = "\
Mach Virtual Memory Statistics: (page size of 16384 bytes)
Pages active:                               10.
Pages wired down:                           20.
Pages occupied by compressor:               30.
";
        assert_eq!(parse_vm_stat_used_bytes(vm_stat), Some(60 * 16384));
    }
}
