//! CPU information collector

use crate::collectors::CollectMode;
use crate::error::Result;
use std::thread;
use std::time::Duration;
use sysinfo::System;

/// CPU information
#[derive(Debug, Clone)]
pub struct CpuInfo {
    /// CPU brand/model name
    pub brand: String,
    /// Number of physical cores
    pub physical_cores: usize,
    /// Number of logical cores (threads)
    pub logical_cores: usize,
    /// Number of CPU sockets (None if skipped in fast mode)
    pub sockets: Option<usize>,
    /// CPU frequency in MHz
    pub frequency_mhz: u64,
    /// Current CPU usage percentage (0-100)
    pub usage_percent: f32,
    /// 1-minute load average (None if skipped in fast mode on Windows)
    pub load_1m: Option<f64>,
    /// 5-minute load average (None if skipped in fast mode on Windows)
    pub load_5m: Option<f64>,
    /// 15-minute load average (None if skipped in fast mode on Windows)
    pub load_15m: Option<f64>,
}

/// Collect CPU information
pub fn collect(mode: CollectMode) -> Result<CpuInfo> {
    let mut sys = System::new();
    sys.refresh_cpu_all();

    // In fast mode, skip the 200ms sleep for accurate CPU usage measurement
    if mode == CollectMode::Full {
        thread::sleep(Duration::from_millis(200));
        sys.refresh_cpu_all();
    }

    let cpus = sys.cpus();
    let physical_cores = sys.physical_core_count().unwrap_or(cpus.len());
    let logical_cores = cpus.len();

    let brand = cpus
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    // Frequency strategy:
    //   1. Prefer CPUID leaf 16h (Intel "Processor Frequency Information") — EBX
    //      returns the architectural max frequency. Reflects silicon-rated boost
    //      and is unaffected by the OS power plan. 0 on AMD / older CPUs.
    //   2. On Windows, fall back to CallNtPowerInformation(ProcessorInformation)
    //      MaxMhz, which reflects the active power-plan ceiling (lower than
    //      silicon boost when the user is on battery saver / balanced).
    //   3. Finally fall back to sysinfo's static value (base clock from registry
    //      on Windows; current frequency on Linux).
    let sysinfo_mhz = cpus.first().map(|c| c.frequency()).unwrap_or(0);
    let frequency_mhz_raw = cpuid_16h_max_mhz();
    #[cfg(target_os = "windows")]
    let frequency_mhz_raw = frequency_mhz_raw.or_else(|| cpu_max_mhz_windows(logical_cores));
    let frequency_mhz = frequency_mhz_raw
        .map(|v| v.max(sysinfo_mhz))
        .unwrap_or(sysinfo_mhz);

    let usage_percent: f32 = if cpus.is_empty() {
        0.0
    } else {
        cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
    };

    // Get load averages and socket count (platform-specific)
    let (load_1m, load_5m, load_15m) = get_load_averages(mode, logical_cores, usage_percent);
    let sockets = if mode == CollectMode::Fast {
        None // Skip subprocess call in fast mode
    } else {
        Some(get_socket_count())
    };

    Ok(CpuInfo {
        brand,
        physical_cores,
        logical_cores,
        sockets,
        frequency_mhz,
        usage_percent,
        load_1m,
        load_5m,
        load_15m,
    })
}

/// Get load averages as percentages
/// On Unix, these are fast (read from /proc or libc) so always collected.
/// On Windows, these depend on the 200ms sleep, so skip in fast mode.
#[cfg(unix)]
fn get_load_averages(
    _mode: CollectMode,
    core_count: usize,
    _current_usage: f32,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    use std::fs;

    // Try to read from /proc/loadavg on Linux
    if let Ok(content) = fs::read_to_string("/proc/loadavg") {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 3 {
            let load1: f64 = parts[0].parse().unwrap_or(0.0);
            let load5: f64 = parts[1].parse().unwrap_or(0.0);
            let load15: f64 = parts[2].parse().unwrap_or(0.0);

            // Convert to percentage of cores
            let max_load = core_count as f64;
            return (
                Some((load1 / max_load * 100.0).min(100.0)),
                Some((load5 / max_load * 100.0).min(100.0)),
                Some((load15 / max_load * 100.0).min(100.0)),
            );
        }
    }

    // Fallback: try libc getloadavg
    let mut loadavg: [f64; 3] = [0.0; 3];
    unsafe {
        if libc::getloadavg(loadavg.as_mut_ptr(), 3) == 3 {
            let max_load = core_count as f64;
            return (
                Some((loadavg[0] / max_load * 100.0).min(100.0)),
                Some((loadavg[1] / max_load * 100.0).min(100.0)),
                Some((loadavg[2] / max_load * 100.0).min(100.0)),
            );
        }
    }

    (Some(0.0), Some(0.0), Some(0.0))
}

/// Get load averages on Windows (uses current CPU usage for all)
/// In fast mode, skip since the 200ms sleep was skipped (no accurate data)
#[cfg(windows)]
fn get_load_averages(
    mode: CollectMode,
    _core_count: usize,
    current_usage: f32,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    if mode == CollectMode::Fast {
        return (None, None, None);
    }
    // Windows doesn't have load averages, so we use current CPU usage
    let usage = current_usage as f64;
    (Some(usage), Some(usage), Some(usage))
}

#[cfg(not(any(unix, windows)))]
fn get_load_averages(
    mode: CollectMode,
    _core_count: usize,
    current_usage: f32,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    if mode == CollectMode::Fast {
        return (None, None, None);
    }
    let usage = current_usage as f64;
    (Some(usage), Some(usage), Some(usage))
}

/// Get number of CPU sockets
#[cfg(target_os = "linux")]
fn get_socket_count() -> usize {
    use std::process::Command;

    // Try lscpu first
    if let Ok(output) = Command::new("lscpu").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with("Socket(s):") {
                if let Some(num) = line.split(':').nth(1) {
                    if let Ok(sockets) = num.trim().parse::<usize>() {
                        return sockets;
                    }
                }
            }
        }
    }

    // Fallback: assume 1 socket
    1
}

#[cfg(target_os = "windows")]
fn get_socket_count() -> usize {
    // Use WMI directly instead of PowerShell subprocess
    crate::collectors::platform::windows::get_socket_count_wmi()
}

#[cfg(target_os = "macos")]
fn get_socket_count() -> usize {
    use std::process::Command;

    // Use sysctl to get package count
    if let Ok(output) = Command::new("sysctl").args(["-n", "hw.packages"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(count) = stdout.trim().parse::<usize>() {
            return count.max(1);
        }
    }

    1
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn get_socket_count() -> usize {
    1
}

// CPUID leaf 16h ("Processor Frequency Information") returns the silicon-rated
// max frequency in EBX (MHz). Intel-only; AMD / older CPUs return 0 in EBX.
// Reference: Intel SDM Vol. 2A, CPUID, Leaf 16H.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn cpuid_16h_max_mhz() -> Option<u64> {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::__cpuid;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::__cpuid;

    // CPUID is callable without an unsafe block on Rust ≥ 1.95 (no safety
    // preconditions on x86/x86_64). Leaf 0 returns the maximum supported leaf
    // in EAX; we only query 0x16 if EAX >= 0x16.
    let max_leaf = __cpuid(0).eax;
    if max_leaf < 0x16 {
        return None;
    }
    let info = __cpuid(0x16);
    // EBX = silicon-rated max frequency in MHz. Returns 0 on AMD and on Intel
    // hybrid chips (Meteor Lake / Lunar Lake / Arrow Lake) where Intel zeroed
    // out leaf 16h in microcode.
    if info.ebx > 0 {
        Some(info.ebx as u64)
    } else {
        None
    }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn cpuid_16h_max_mhz() -> Option<u64> {
    None
}

// Windows: query CallNtPowerInformation for accurate per-core max frequency.
// Returns MaxMhz across all logical processors (power-plan ceiling). Declared
// as a manual `extern` because winapi-rs's `powrprof` bindings are not stable.
#[cfg(target_os = "windows")]
#[link(name = "powrprof")]
extern "system" {
    fn CallNtPowerInformation(
        InformationLevel: u32,
        InputBuffer: *mut std::ffi::c_void,
        InputBufferLength: u32,
        OutputBuffer: *mut std::ffi::c_void,
        OutputBufferLength: u32,
    ) -> i32;
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Default, Copy, Clone)]
struct ProcessorPowerInformation {
    number: u32,
    max_mhz: u32,
    current_mhz: u32,
    mhz_limit: u32,
    max_idle_state: u32,
    current_idle_state: u32,
}

#[cfg(target_os = "windows")]
fn cpu_max_mhz_windows(logical_cores: usize) -> Option<u64> {
    if logical_cores == 0 {
        return None;
    }
    const PROCESSOR_INFORMATION: u32 = 11;

    let mut buf: Vec<ProcessorPowerInformation> =
        vec![ProcessorPowerInformation::default(); logical_cores];
    let buf_size = (logical_cores * std::mem::size_of::<ProcessorPowerInformation>()) as u32;

    // SAFETY: CallNtPowerInformation(ProcessorInformation) takes no input buffer
    // and writes one PROCESSOR_POWER_INFORMATION per logical processor into the
    // output buffer. We size `buf` to `logical_cores` exactly.
    let status = unsafe {
        CallNtPowerInformation(
            PROCESSOR_INFORMATION,
            std::ptr::null_mut(),
            0,
            buf.as_mut_ptr() as *mut _,
            buf_size,
        )
    };

    if status != 0 {
        return None;
    }

    // Use the maximum MaxMhz across all cores (power-plan ceiling).
    buf.iter()
        .map(|p| p.max_mhz as u64)
        .max()
        .filter(|&v| v > 0)
}

impl CpuInfo {
    /// Format frequency as GHz string
    pub fn frequency_ghz(&self) -> String {
        format!("{:.2} GHz", self.frequency_mhz as f64 / 1000.0)
    }

    /// Get core count string
    pub fn cores_string(&self) -> String {
        if self.physical_cores == self.logical_cores {
            format!("{} cores", self.physical_cores)
        } else {
            format!(
                "{} cores / {} threads",
                self.physical_cores, self.logical_cores
            )
        }
    }
}
