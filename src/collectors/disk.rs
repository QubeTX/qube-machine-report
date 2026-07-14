//! Disk information collector

use crate::error::Result;
use sysinfo::Disks;

/// Disk/volume information
#[non_exhaustive]
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
    /// Unallocated/free space in bytes. On Unix this includes blocks reserved
    /// from unprivileged callers; `available_bytes` does not.
    pub free_bytes: u64,
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
        let fallback = (disk.total_space(), disk.available_space());
        let (total, free, available) = disk_space_for_mount(disk.mount_point(), fallback);
        // "Used" means allocated blocks, not "total minus bytes currently
        // available to this caller". Those differ on ext filesystems with
        // root-reserved blocks, APFS reclaimable capacity, and Windows quota
        // views. Keep caller-available bytes separately.
        let used = total.saturating_sub(free.min(total));

        // Skip disks with no space (virtual filesystems, etc.)
        if total == 0 {
            continue;
        }

        result.push(DiskInfo {
            mount_point: disk.mount_point().to_string_lossy().to_string(),
            filesystem: disk.file_system().to_string_lossy().to_string(),
            total_bytes: total,
            available_bytes: available,
            free_bytes: free,
            used_bytes: used,
            is_removable: disk.is_removable(),
            name: disk.name().to_string_lossy().to_string(),
        });
    }

    Ok(result)
}

#[cfg(unix)]
fn disk_space_for_mount(path: &std::path::Path, fallback: (u64, u64)) -> (u64, u64, u64) {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let Ok(path) = CString::new(path.as_os_str().as_bytes()) else {
        return (fallback.0, fallback.1, fallback.1);
    };
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `path` is NUL-terminated and `stats` points to writable storage
    // for exactly one `statvfs` structure. We read it only after success.
    if unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) } != 0 {
        return (fallback.0, fallback.1, fallback.1);
    }
    let stats = unsafe { stats.assume_init() };
    let fragment_size = if stats.f_frsize > 0 {
        stats.f_frsize
    } else {
        stats.f_bsize
    };
    let total = (stats.f_blocks as u64).saturating_mul(fragment_size);
    let free = (stats.f_bfree as u64).saturating_mul(fragment_size);
    let available = (stats.f_bavail as u64).saturating_mul(fragment_size);
    if total == 0 {
        (fallback.0, fallback.1, fallback.1)
    } else {
        (total, free, available)
    }
}

#[cfg(windows)]
fn disk_space_for_mount(path: &std::path::Path, fallback: (u64, u64)) -> (u64, u64, u64) {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(
            directory_name: *const u16,
            free_bytes_available_to_caller: *mut u64,
            total_number_of_bytes: *mut u64,
            total_number_of_free_bytes: *mut u64,
        ) -> i32;
    }

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut available = 0u64;
    let mut total = 0u64;
    let mut free = 0u64;
    // SAFETY: `wide` is NUL-terminated and the three output pointers refer to
    // valid `u64` storage for the duration of the call.
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, &mut total, &mut free) };
    if ok == 0 || total == 0 {
        (fallback.0, fallback.1, fallback.1)
    } else {
        (total, free, available)
    }
}

#[cfg(not(any(unix, windows)))]
fn disk_space_for_mount(_path: &std::path::Path, fallback: (u64, u64)) -> (u64, u64, u64) {
    (fallback.0, fallback.1, fallback.1)
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
        crate::format_bytes(bytes)
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
