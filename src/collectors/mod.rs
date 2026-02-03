//! System information collectors
//!
//! Each module is responsible for collecting a specific category
//! of system information in a platform-agnostic way.

pub mod cpu;
pub mod disk;
pub mod memory;
pub mod network;
pub mod os;
pub mod platform;
pub mod session;

use crate::error::Result;

/// Collected system information matching TR-200 fields
#[derive(Debug, Clone)]
pub struct SystemInfo {
    // OS Section
    pub os_name: String,
    pub os_version: String,
    pub kernel: String,

    // Network Section
    pub hostname: String,
    pub machine_ip: String,
    pub client_ip: Option<String>,
    pub dns_servers: Vec<String>,
    pub username: String,

    // CPU Section
    pub processor: String,
    pub cores: usize,
    pub sockets: usize,
    pub hypervisor: String,
    pub cpu_freq_ghz: f64,
    pub load_1m: f64,
    pub load_5m: f64,
    pub load_15m: f64,

    // Disk Section
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_percent: f64,
    pub zfs_health: Option<String>,

    // Memory Section
    pub mem_used_bytes: u64,
    pub mem_total_bytes: u64,
    pub mem_percent: f64,

    // Session Section
    pub last_login: String,
    pub last_login_ip: Option<String>,
    pub uptime_seconds: u64,
}

impl SystemInfo {
    /// Collect all system information
    pub fn collect() -> Result<Self> {
        let os_info = os::collect()?;
        let cpu_info = cpu::collect()?;
        let mem_info = memory::collect()?;
        let disks = disk::collect()?;
        let net_info = network::collect_network_info()?;
        let session_info = session::collect()?;
        let platform_info = platform::collect();

        // Aggregate disk info (use root/C: or sum all)
        let (disk_used, disk_total) = aggregate_disk_usage(&disks);
        let disk_percent = if disk_total > 0 {
            (disk_used as f64 / disk_total as f64) * 100.0
        } else {
            0.0
        };

        // Memory percentage
        let mem_percent = if mem_info.total_bytes > 0 {
            (mem_info.used_bytes as f64 / mem_info.total_bytes as f64) * 100.0
        } else {
            0.0
        };

        // Hypervisor detection
        let hypervisor = platform_info
            .virtualization
            .unwrap_or_else(|| "Bare Metal".to_string());

        Ok(Self {
            os_name: os_info.name,
            os_version: os_info.version,
            kernel: os_info.kernel_version,
            hostname: os_info.hostname,
            machine_ip: net_info.machine_ip,
            client_ip: net_info.client_ip,
            dns_servers: net_info.dns_servers,
            username: session_info.username,
            processor: cpu_info.brand,
            cores: cpu_info.logical_cores,
            sockets: cpu_info.sockets,
            hypervisor,
            cpu_freq_ghz: cpu_info.frequency_mhz as f64 / 1000.0,
            load_1m: cpu_info.load_1m,
            load_5m: cpu_info.load_5m,
            load_15m: cpu_info.load_15m,
            disk_used_bytes: disk_used,
            disk_total_bytes: disk_total,
            disk_percent,
            zfs_health: None, // TODO: implement ZFS check
            mem_used_bytes: mem_info.used_bytes,
            mem_total_bytes: mem_info.total_bytes,
            mem_percent,
            last_login: session_info.last_login,
            last_login_ip: session_info.last_login_ip,
            uptime_seconds: os_info.uptime_seconds,
        })
    }

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

    /// Format bytes as GB
    pub fn format_gb(bytes: u64) -> String {
        let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        format!("{:.2}", gb)
    }

    /// Format bytes as GiB (for memory)
    pub fn format_gib(bytes: u64) -> String {
        let gib = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        format!("{:.2}", gib)
    }

    /// Get disk usage string in TR-200 format
    pub fn disk_usage_str(&self) -> String {
        format!(
            "{}/{} GB [{:.2}%]",
            Self::format_gb(self.disk_used_bytes),
            Self::format_gb(self.disk_total_bytes),
            self.disk_percent
        )
    }

    /// Get memory usage string in TR-200 format
    pub fn memory_usage_str(&self) -> String {
        format!(
            "{}/{} GiB [{:.1}%]",
            Self::format_gib(self.mem_used_bytes),
            Self::format_gib(self.mem_total_bytes),
            self.mem_percent
        )
    }

    /// Get cores string in TR-200 format
    pub fn cores_str(&self) -> String {
        format!("{} vCPU(s) / {} Socket(s)", self.cores, self.sockets)
    }

    /// Get CPU frequency string
    pub fn freq_str(&self) -> String {
        format!("{:.1} GHz", self.cpu_freq_ghz)
    }
}

/// Aggregate disk usage - prioritize root/C: drive, or sum all non-removable
fn aggregate_disk_usage(disks: &[disk::DiskInfo]) -> (u64, u64) {
    // Try to find the root or C: drive
    for d in disks {
        if d.mount_point == "/" || d.mount_point == "C:\\" || d.mount_point.starts_with("C:") {
            return (d.used_bytes, d.total_bytes);
        }
    }

    // Otherwise sum all non-removable disks
    let mut total_used = 0u64;
    let mut total_size = 0u64;
    for d in disks {
        if !d.is_removable {
            total_used += d.used_bytes;
            total_size += d.total_bytes;
        }
    }

    if total_size == 0 && !disks.is_empty() {
        // Fallback to first disk
        (disks[0].used_bytes, disks[0].total_bytes)
    } else {
        (total_used, total_size)
    }
}
