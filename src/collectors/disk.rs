//! Disk information collector

use crate::error::Result;
use sysinfo::Disks;

/// Disk/volume information
#[derive(Debug, Clone)]
pub struct DiskInfo {
    /// Mount point or drive letter
    pub mount_point: String,
    /// Filesystem type (e.g., "NTFS", "ext4", "APFS")
    pub filesystem: String,
    /// Total space in bytes
    pub total_bytes: u64,
    /// Available space in bytes
    pub available_bytes: u64,
    /// Used space in bytes
    pub used_bytes: u64,
    /// Whether this is a removable disk
    pub is_removable: bool,
    /// Disk name/label
    pub name: String,
}

/// Collect disk information
pub fn collect() -> Result<Vec<DiskInfo>> {
    let disks = Disks::new_with_refreshed_list();
    let mut result = Vec::new();

    for disk in disks.list() {
        let total = disk.total_space();
        let available = disk.available_space();
        let used = total.saturating_sub(available);

        // Skip disks with no space (virtual filesystems, etc.)
        if total == 0 {
            continue;
        }

        result.push(DiskInfo {
            mount_point: disk.mount_point().to_string_lossy().to_string(),
            filesystem: disk.file_system().to_string_lossy().to_string(),
            total_bytes: total,
            available_bytes: available,
            used_bytes: used,
            is_removable: disk.is_removable(),
            name: disk.name().to_string_lossy().to_string(),
        });
    }

    Ok(result)
}

impl DiskInfo {
    /// Get usage percentage
    pub fn usage_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.used_bytes as f64 / self.total_bytes as f64 * 100.0) as f32
        }
    }

    /// Format bytes as human-readable
    fn format_bytes(bytes: u64) -> String {
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

    /// Get formatted total space
    pub fn total_formatted(&self) -> String {
        Self::format_bytes(self.total_bytes)
    }

    /// Get formatted available space
    pub fn available_formatted(&self) -> String {
        Self::format_bytes(self.available_bytes)
    }

    /// Get formatted used space
    pub fn used_formatted(&self) -> String {
        Self::format_bytes(self.used_bytes)
    }

    /// Get usage string
    pub fn usage_string(&self) -> String {
        format!(
            "{} / {} ({:.1}%)",
            self.used_formatted(),
            self.total_formatted(),
            self.usage_percent()
        )
    }

    /// Get display name (mount point or name)
    pub fn display_name(&self) -> String {
        if self.name.is_empty() {
            self.mount_point.clone()
        } else {
            format!("{} ({})", self.name, self.mount_point)
        }
    }
}
