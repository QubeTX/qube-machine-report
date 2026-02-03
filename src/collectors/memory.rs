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

    Ok(MemoryInfo {
        total_bytes: sys.total_memory(),
        used_bytes: sys.used_memory(),
        available_bytes: sys.available_memory(),
        swap_total_bytes: sys.total_swap(),
        swap_used_bytes: sys.used_swap(),
    })
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
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;

        if bytes >= TB {
            format!("{:.2} TB", bytes as f64 / TB as f64)
        } else if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
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
