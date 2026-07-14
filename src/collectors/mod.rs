//! System information collectors
//!
//! Each module is responsible for collecting a specific category
//! of system information in a platform-agnostic way.

pub mod command;
pub mod cpu;
pub mod disk;
pub mod memory;
pub mod network;
pub mod os;
pub mod platform;
pub mod session;

use crate::error::Result;

/// Controls how much data to collect
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectMode {
    /// Full collection — all fields, all subprocess calls
    Full,
    /// Fast collection — skip slow platform-specific collectors for quick auto-run
    Fast,
}

/// Collected system information used by TR-300 reports
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SystemInfo {
    // OS Section
    pub os_name: String,
    pub os_version: String,
    pub kernel: String,
    pub architecture: String,
    pub machine_model: Option<String>,
    pub os_edition: Option<String>,
    pub os_codename: Option<String>,
    pub os_build: Option<String>,

    // Network Section
    pub hostname: String,
    pub machine_ip: Option<String>,
    pub client_ip: Option<String>,
    pub dns_servers: Vec<String>,
    pub username: String,

    // CPU Section
    pub processor: String,
    /// Logical processor count retained under the original public field name.
    pub cores: usize,
    pub physical_cores: usize,
    pub sockets: Option<usize>,
    pub hypervisor: Option<String>,
    pub cpu_freq_ghz: f64,
    pub cpu_frequency_kind: Option<String>,
    pub cpu_usage_percent: Option<f64>,
    pub load_1m: Option<f64>,
    pub load_5m: Option<f64>,
    pub load_15m: Option<f64>,
    pub raw_load_1m: Option<f64>,
    pub raw_load_5m: Option<f64>,
    pub raw_load_15m: Option<f64>,
    pub gpus: Vec<String>,
    pub cpu_core_topology: Option<String>,

    // Disk Section
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_available_bytes: u64,
    pub disk_percent: f64,
    pub disk_mount_point: Option<String>,
    pub disk_filesystem: Option<String>,
    pub zfs_health: Option<String>,

    // Memory Section
    pub mem_used_bytes: u64,
    pub mem_total_bytes: u64,
    pub mem_available_bytes: u64,
    pub mem_percent: f64,
    pub memory_usage_kind: String,
    pub memory_availability_kind: String,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_percent: f64,
    pub motherboard: Option<String>,
    pub bios: Option<String>,
    pub ram_slots: Option<String>,

    // Session Section
    pub last_login: Option<String>,
    pub last_login_ip: Option<String>,
    pub uptime_seconds: u64,
    /// Optional alternate session uptime when a platform can establish one
    /// independently. `None` otherwise — drives the
    /// `UPTIME … (session: …)` annotation when `Some(_)`.
    pub session_uptime_seconds: Option<u64>,
    pub shell: Option<String>,
    pub terminal: Option<String>,
    pub locale: Option<String>,
    pub battery: Option<String>,
    pub encryption: Option<String>,
    pub desktop_environment: Option<String>,
    pub display_server: Option<String>,
    pub display_resolution: Option<String>,
    pub boot_mode: Option<String>,

    /// The collection mode used
    pub mode: CollectMode,

    /// Whether the current process is running with elevated privileges
    /// (Unix euid == 0 / Windows admin token under UAC). Drives the
    /// elevation-tier footer hint and gates admin-only collectors.
    pub is_elevated: bool,
    pub elevation_unlocks_more: bool,
}

impl SystemInfo {
    /// Collect all system information with the given mode.
    /// Uses `std::thread::scope` to run collectors in parallel —
    /// the 200ms CPU sleep (full mode) overlaps with disk/network/session/platform.
    pub fn collect_with_mode(mode: CollectMode) -> Result<Self> {
        use crate::error::AppError;

        let (os_info, cpu_info, mem_info, disks, net_info, session_info, platform_info) =
            std::thread::scope(|s| {
                let os_h = s.spawn(|| os::collect(mode));
                let cpu_h = s.spawn(|| cpu::collect(mode));
                let mem_h = s.spawn(|| memory::collect_with_mode(mode));
                let disk_h = s.spawn(disk::collect);
                let net_h = s.spawn(|| network::collect_network_info(mode));
                let session_h = s.spawn(|| session::collect(mode));
                let platform_h = s.spawn(|| platform::collect(mode));

                (
                    os_h.join().unwrap_or_else(|_| {
                        Err(AppError::system_info("OS collector thread panicked"))
                    }),
                    cpu_h.join().unwrap_or_else(|_| {
                        Err(AppError::system_info("CPU collector thread panicked"))
                    }),
                    mem_h.join().unwrap_or_else(|_| {
                        Err(AppError::system_info("memory collector thread panicked"))
                    }),
                    disk_h.join().unwrap_or_else(|_| {
                        Err(AppError::system_info("disk collector thread panicked"))
                    }),
                    net_h.join().unwrap_or_else(|_| {
                        Err(AppError::system_info("network collector thread panicked"))
                    }),
                    session_h.join().unwrap_or_else(|_| {
                        Err(AppError::system_info("session collector thread panicked"))
                    }),
                    platform_h
                        .join()
                        .unwrap_or_else(|_| platform::PlatformInfo::default()),
                )
            });

        let os_info = os_info?;
        let cpu_info = cpu_info?;
        let mem_info = mem_info?;
        let disks = disks?;
        let net_info = net_info?;
        let session_info = session_info?;

        // Select the system/root volume. A machine-wide sum is not an honest
        // fallback because bind mounts, APFS volumes, and Windows fixed drives
        // can overlap or represent different resources.
        let disk = aggregate_disk_usage(&disks);
        let disk_used = disk.as_ref().map_or(0, |d| d.used_bytes);
        let disk_total = disk.as_ref().map_or(0, |d| d.total_bytes);
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
        let swap_percent = if mem_info.swap_total_bytes > 0 {
            (mem_info.swap_used_bytes as f64 / mem_info.swap_total_bytes as f64) * 100.0
        } else {
            0.0
        };

        // Absence of a virtualization signal is unknown, not proof of bare
        // metal. Platform collectors return a positive, evidence-backed label
        // only when they can establish one.
        let hypervisor = platform_info.virtualization;

        let cpu_usage_percent = (mode == CollectMode::Full)
            .then_some(cpu_info.usage_percent as f64)
            .filter(|value| value.is_finite());
        let fallback_shell = non_unknown(session_info.shell);
        let fallback_terminal = non_unknown(session_info.terminal);

        Ok(Self {
            os_name: os_info.name,
            os_version: os_info.version,
            kernel: os_info.kernel_version,
            architecture: platform_info.architecture.unwrap_or(os_info.architecture),
            machine_model: platform_info.machine_model,
            os_edition: platform_info.windows_edition,
            os_codename: platform_info.macos_codename,
            os_build: platform_info.os_build,
            hostname: os_info.hostname,
            machine_ip: net_info.machine_ip,
            client_ip: net_info.client_ip,
            dns_servers: net_info.dns_servers,
            username: session_info.username,
            processor: cpu_info.brand,
            cores: cpu_info.logical_cores,
            physical_cores: cpu_info.physical_cores,
            sockets: cpu_info.sockets,
            hypervisor,
            cpu_freq_ghz: cpu_info.frequency_mhz as f64 / 1000.0,
            cpu_frequency_kind: cpu_info.frequency_kind,
            cpu_usage_percent,
            load_1m: cpu_info.load_1m,
            load_5m: cpu_info.load_5m,
            load_15m: cpu_info.load_15m,
            raw_load_1m: cpu_info.raw_load_1m,
            raw_load_5m: cpu_info.raw_load_5m,
            raw_load_15m: cpu_info.raw_load_15m,
            gpus: platform_info.gpus,
            cpu_core_topology: platform_info.cpu_core_topology,
            disk_used_bytes: disk_used,
            disk_total_bytes: disk_total,
            disk_available_bytes: disk.as_ref().map_or(0, |d| d.available_bytes),
            disk_percent,
            disk_mount_point: disk.as_ref().map(|d| d.mount_point.clone()),
            disk_filesystem: disk.as_ref().map(|d| d.filesystem.clone()),
            zfs_health: platform_info.zfs_health,
            mem_used_bytes: mem_info.used_bytes,
            mem_total_bytes: mem_info.total_bytes,
            mem_available_bytes: mem_info.available_bytes,
            mem_percent,
            memory_usage_kind: mem_info.usage_kind,
            memory_availability_kind: mem_info.availability_kind,
            swap_used_bytes: mem_info.swap_used_bytes,
            swap_total_bytes: mem_info.swap_total_bytes,
            swap_percent,
            motherboard: platform_info.motherboard,
            bios: platform_info.bios,
            ram_slots: platform_info.ram_slots,
            last_login: session_info.last_login,
            last_login_ip: session_info.last_login_ip,
            uptime_seconds: os_info.uptime_seconds,
            session_uptime_seconds: os_info.session_uptime_seconds,
            shell: platform_info.shell.or(fallback_shell),
            terminal: platform_info.terminal.or(fallback_terminal),
            locale: platform_info.locale,
            battery: platform_info.battery,
            encryption: platform_info.encryption,
            desktop_environment: platform_info.desktop_environment,
            display_server: platform_info.display_server,
            display_resolution: platform_info.display_resolution,
            boot_mode: platform_info.boot_mode,
            mode,
            is_elevated: crate::is_elevated(),
            elevation_unlocks_more: platform_info.elevation_unlocks_more,
        })
    }

    /// Collect all system information (full mode, backward compatible)
    pub fn collect() -> Result<Self> {
        Self::collect_with_mode(CollectMode::Full)
    }

    /// Format uptime as human-readable string. When `session_uptime_seconds`
    /// is independently available, append a `(session: …)` annotation.
    pub fn uptime_formatted(&self) -> String {
        let primary = format_duration_seconds(self.uptime_seconds);
        match self.session_uptime_seconds {
            Some(s) => format!("{} (session: {})", primary, format_duration_seconds(s)),
            None => primary,
        }
    }

    /// Format bytes as binary GiB. The method name is retained for public API
    /// compatibility; user-facing output labels the unit accurately.
    pub fn format_gb(bytes: u64) -> String {
        let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        format!("{:.2}", gb)
    }

    /// Format bytes as GiB (for memory)
    pub fn format_gib(bytes: u64) -> String {
        let gib = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        format!("{:.2}", gib)
    }

    /// Get disk usage string for table output
    pub fn disk_usage_str(&self) -> String {
        format!(
            "{}/{} GiB [{:.2}%]",
            Self::format_gb(self.disk_used_bytes),
            Self::format_gb(self.disk_total_bytes),
            finite_or_zero(self.disk_percent)
        )
    }

    /// Get memory usage string for table output
    pub fn memory_usage_str(&self) -> String {
        format!(
            "{}/{} GiB [{:.1}%]",
            Self::format_gib(self.mem_used_bytes),
            Self::format_gib(self.mem_total_bytes),
            finite_or_zero(self.mem_percent)
        )
    }

    /// Get swap usage string for table output.
    pub fn swap_usage_str(&self) -> String {
        format!(
            "{}/{} GiB [{:.1}%]",
            Self::format_gib(self.swap_used_bytes),
            Self::format_gib(self.swap_total_bytes),
            finite_or_zero(self.swap_percent)
        )
    }

    /// Get cores string for table output
    pub fn cores_str(&self) -> String {
        let core_text = if self.physical_cores > 0 {
            format!("{} cores / {} threads", self.physical_cores, self.cores)
        } else {
            format!("{} logical processors", self.cores)
        };
        if let Some(sockets) = self.sockets.filter(|s| *s > 1) {
            format!("{} / {} sockets", core_text, sockets)
        } else {
            core_text
        }
    }

    /// Get CPU frequency string
    pub fn freq_str(&self) -> String {
        format!("{:.1} GHz", self.cpu_freq_ghz)
    }
}

/// Format a seconds count as a compact "Nd Nh Nm" / "Nh Nm" / "Nm" string.
fn format_duration_seconds(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        if minutes > 0 {
            format!("{}m", minutes)
        } else {
            format!("{}s", secs)
        }
    }
}

/// Select the system/root volume, with the largest fixed volume as an honest
/// single-volume fallback when the system mount is unavailable.
fn aggregate_disk_usage(disks: &[disk::DiskInfo]) -> Option<&disk::DiskInfo> {
    #[cfg(windows)]
    let system_root = std::env::var("SystemDrive")
        .ok()
        .map(|drive| format!("{}\\", drive.trim_end_matches(['\\', '/'])));
    #[cfg(not(windows))]
    let system_root: Option<String> = None;

    disks
        .iter()
        .find(|disk| is_system_mount(&disk.mount_point, system_root.as_deref()))
        .or_else(|| {
            disks
                .iter()
                .filter(|disk| !disk.is_removable)
                .max_by_key(|disk| disk.total_bytes)
        })
        .or_else(|| disks.iter().max_by_key(|disk| disk.total_bytes))
}

fn is_system_mount(mount: &str, system_root: Option<&str>) -> bool {
    if mount == "/" {
        return true;
    }
    system_root.is_some_and(|root| {
        mount.eq_ignore_ascii_case(root)
            || mount
                .trim_end_matches(['\\', '/'])
                .eq_ignore_ascii_case(root.trim_end_matches(['\\', '/']))
    })
}

fn non_unknown(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && !value.eq_ignore_ascii_case("unknown")).then(|| value.to_string())
}

fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk(mount: &str, total: u64, removable: bool) -> disk::DiskInfo {
        disk::DiskInfo {
            mount_point: mount.to_string(),
            filesystem: "testfs".to_string(),
            total_bytes: total,
            available_bytes: total / 2,
            free_bytes: total / 2,
            used_bytes: total / 2,
            is_removable: removable,
            name: mount.to_string(),
        }
    }

    #[test]
    fn root_volume_wins_over_larger_data_volume() {
        let disks = vec![disk("/data", 4_000, false), disk("/", 1_000, false)];
        assert_eq!(aggregate_disk_usage(&disks).unwrap().mount_point, "/");
    }

    #[test]
    fn windows_system_drive_matching_is_case_and_separator_tolerant() {
        assert!(is_system_mount(r"C:\", Some("c:")));
        assert!(is_system_mount("c:", Some(r"C:\")));
        assert!(!is_system_mount(r"D:\", Some(r"C:\")));
    }

    #[test]
    fn largest_fixed_volume_is_the_single_volume_fallback() {
        let disks = vec![
            disk("/mnt/small", 1_000, false),
            disk("/mnt/large", 4_000, false),
            disk("/media/usb", 8_000, true),
        ];
        assert_eq!(
            aggregate_disk_usage(&disks).unwrap().mount_point,
            "/mnt/large"
        );
    }

    #[test]
    fn removable_volume_is_used_only_when_no_fixed_volume_exists() {
        let disks = vec![
            disk("/media/small", 1_000, true),
            disk("/media/large", 2_000, true),
        ];
        assert_eq!(
            aggregate_disk_usage(&disks).unwrap().mount_point,
            "/media/large"
        );
        assert!(aggregate_disk_usage(&[]).is_none());
    }

    #[test]
    fn short_uptime_keeps_seconds_precision() {
        assert_eq!(format_duration_seconds(0), "0s");
        assert_eq!(format_duration_seconds(59), "59s");
        assert_eq!(format_duration_seconds(60), "1m");
    }
}
