//! CPU information collector

#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::collectors::command::CommandTimeout;
// run_stdout is only used by the macOS `get_socket_count` path; the
// Linux path was migrated to `run_stdout_c_locale` (v3.15.2 audit
// finding F19) and Windows uses platform-native APIs.
#[cfg(target_os = "macos")]
use crate::collectors::command::run_stdout;
use crate::collectors::CollectMode;
use crate::error::Result;
use std::thread;
use std::time::Duration;
use sysinfo::System;

type LoadAverages = (
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
);

/// CPU information
#[non_exhaustive]
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
    /// Meaning of `frequency_mhz` (`maximum` or `reported`). `None` when the
    /// platform could not provide a non-zero value.
    pub frequency_kind: Option<String>,
    /// Current CPU usage percentage (0-100)
    pub usage_percent: f32,
    /// 1-minute Unix load normalized by logical processor count. Windows: None.
    pub load_1m: Option<f64>,
    /// 5-minute Unix load normalized by logical processor count. Windows: None.
    pub load_5m: Option<f64>,
    /// 15-minute Unix load normalized by logical processor count. Windows: None.
    pub load_15m: Option<f64>,
    /// Raw operating-system load averages. Unix load is runnable/queued work,
    /// not CPU utilization; Windows leaves these fields unavailable.
    pub raw_load_1m: Option<f64>,
    pub raw_load_5m: Option<f64>,
    pub raw_load_15m: Option<f64>,
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
    // Unknown physical topology must stay unknown. Falling back to the logical
    // count incorrectly labeled vCPUs and SMT threads as physical cores.
    let physical_cores = sys.physical_core_count().unwrap_or(0);
    let logical_cores = cpus.len();

    let brand = cpus
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());
    let brand = platform_cpu_brand(brand);

    // Frequency strategy (one explicit contract per value):
    //   1. Prefer CPUID leaf 16h (Intel "Processor Frequency Information") — EBX
    //      returns the architectural max frequency. Reflects silicon-rated boost
    //      and is unaffected by the OS power plan. 0 on AMD / older CPUs.
    //   2. On Windows, fall back to CallNtPowerInformation(ProcessorInformation)
    //      MaxMhz, which reflects the active power-plan ceiling (lower than
    //      silicon boost when the user is on battery saver / balanced).
    //   3. Finally fall back to sysinfo's static value (base clock from registry
    //      on Windows; current frequency on Linux).
    let sysinfo_mhz = cpus.first().map(|c| c.frequency()).unwrap_or(0);
    let maximum_mhz = cpuid_16h_max_mhz();
    #[cfg(target_os = "windows")]
    let maximum_mhz = maximum_mhz.or_else(|| cpu_max_mhz_windows(logical_cores));
    // ARM Linux has no CPUID leaf 16h and sysinfo often reports 0 MHz, which
    // renders "0.00 GHz". Fall back to the kernel's rated max from sysfs.
    #[cfg(target_os = "linux")]
    let maximum_mhz = maximum_mhz.or_else(linux_cpufreq_max_mhz);
    #[cfg(target_os = "macos")]
    let translated_frequency = crate::collectors::platform::macos::is_rosetta_translated();
    #[cfg(not(target_os = "macos"))]
    let translated_frequency = false;
    let (frequency_mhz, frequency_kind) = if translated_frequency {
        // Rosetta exposes a compatibility 2.4 GHz sysctl value on Apple
        // Silicon that disagrees with the native processor frequency. The
        // architecture row already identifies translation; absence is more
        // accurate than displaying the compatibility shim as hardware data.
        (0, None)
    } else if let Some(maximum) = maximum_mhz.filter(|v| *v > 0) {
        (maximum, Some("maximum".to_string()))
    } else if sysinfo_mhz > 0 {
        (sysinfo_mhz, Some("reported".to_string()))
    } else {
        (0, None)
    };

    let usage_percent: f32 = if cpus.is_empty() {
        0.0
    } else {
        cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
    };

    // Get load averages and socket count (platform-specific)
    let (load_1m, load_5m, load_15m, raw_load_1m, raw_load_5m, raw_load_15m) =
        get_load_averages(mode, logical_cores);
    let sockets = if mode == CollectMode::Fast {
        None // Skip subprocess call in fast mode
    } else {
        get_socket_count()
    };

    Ok(CpuInfo {
        brand,
        physical_cores,
        logical_cores,
        sockets,
        frequency_mhz,
        frequency_kind,
        usage_percent,
        load_1m,
        load_5m,
        load_15m,
        raw_load_1m,
        raw_load_5m,
        raw_load_15m,
    })
}

/// Get normalized and raw load averages.
/// On Unix, these are fast (read from /proc or libc) so always collected.
/// Windows has no equivalent Unix-style load average and returns no values.
#[cfg(unix)]
fn get_load_averages(_mode: CollectMode, core_count: usize) -> LoadAverages {
    use std::fs;

    // Try to read from /proc/loadavg on Linux
    if let Ok(content) = fs::read_to_string("/proc/loadavg") {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 3 {
            // Parse all three fields. A malformed field means /proc/loadavg
            // isn't trustworthy, so fall through to the libc getloadavg
            // fallback rather than reporting a fabricated 0% load (which is
            // indistinguishable from a genuinely idle machine).
            if let (Ok(load1), Ok(load5), Ok(load15)) = (
                parts[0].parse::<f64>(),
                parts[1].parse::<f64>(),
                parts[2].parse::<f64>(),
            ) {
                if [load1, load5, load15].into_iter().all(valid_load) {
                    return (
                        Some(normalize_load(load1, core_count)),
                        Some(normalize_load(load5, core_count)),
                        Some(normalize_load(load15, core_count)),
                        Some(load1),
                        Some(load5),
                        Some(load15),
                    );
                }
            }
        }
    }

    // Fallback: try libc getloadavg
    let mut loadavg: [f64; 3] = [0.0; 3];
    unsafe {
        if libc::getloadavg(loadavg.as_mut_ptr(), 3) == 3 && loadavg.into_iter().all(valid_load) {
            return (
                Some(normalize_load(loadavg[0], core_count)),
                Some(normalize_load(loadavg[1], core_count)),
                Some(normalize_load(loadavg[2], core_count)),
                Some(loadavg[0]),
                Some(loadavg[1]),
                Some(loadavg[2]),
            );
        }
    }

    // Both sources failed — report "unavailable" rather than a fake 0% load.
    (None, None, None, None, None, None)
}

#[cfg(unix)]
fn valid_load(load: f64) -> bool {
    load.is_finite() && load >= 0.0
}

#[cfg(unix)]
fn normalize_load(load: f64, core_count: usize) -> f64 {
    load / core_count.max(1) as f64 * 100.0
}

/// Windows has no Unix-style load average.
#[cfg(windows)]
fn get_load_averages(_mode: CollectMode, _core_count: usize) -> LoadAverages {
    // Windows has no Unix-style load average. CPU utilization is carried in
    // `usage_percent`; fabricating 1/5/15-minute values from one sample made
    // the old labels materially false.
    (None, None, None, None, None, None)
}

#[cfg(not(any(unix, windows)))]
fn get_load_averages(_mode: CollectMode, _core_count: usize) -> LoadAverages {
    (None, None, None, None, None, None)
}

/// Get number of CPU sockets
#[cfg(target_os = "linux")]
fn get_socket_count() -> Option<usize> {
    let mut packages = std::collections::HashSet::new();
    if let Ok(cpus) = std::fs::read_dir("/sys/devices/system/cpu") {
        for cpu in cpus.flatten() {
            let name = cpu.file_name().to_string_lossy().to_string();
            if !name.strip_prefix("cpu").is_some_and(|suffix| {
                !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit())
            }) {
                continue;
            }
            if let Ok(package) =
                std::fs::read_to_string(cpu.path().join("topology/physical_package_id"))
            {
                if let Ok(package) = package.trim().parse::<i32>() {
                    packages.insert(package);
                }
            }
        }
    }
    if !packages.is_empty() {
        return Some(packages.len());
    }

    // Force LC_ALL=C so `lscpu` emits English labels — non-English
    // locales rename `Socket(s):` to e.g. `Sockel:` (German),
    // silently breaking the substring match. (audit finding F19,
    // v3.15.8+)
    if let Some(stdout) = crate::collectors::command::run_stdout_c_locale(
        "lscpu",
        std::iter::empty::<&str>(),
        CommandTimeout::Normal,
    ) {
        for line in stdout.lines() {
            if line.starts_with("Socket(s):") {
                if let Some(num) = line.split(':').nth(1) {
                    if let Ok(sockets) = num.trim().parse::<usize>() {
                        return (sockets > 0).then_some(sockets);
                    }
                }
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn get_socket_count() -> Option<usize> {
    // C.9 (v3.13.0+): native GetLogicalProcessorInformationEx, ~10x faster
    // than the WMI path it replaces. WMI fallback retained for systems where
    // the native call returns nothing unexpected.
    crate::collectors::platform::windows::get_socket_count_native()
        .or_else(crate::collectors::platform::windows::get_socket_count_wmi)
}

#[cfg(target_os = "macos")]
fn get_socket_count() -> Option<usize> {
    // Use sysctl to get package count
    if let Some(stdout) = run_stdout("sysctl", ["-n", "hw.packages"], CommandTimeout::Normal) {
        if let Ok(count) = stdout.trim().parse::<usize>() {
            return (count > 0).then_some(count);
        }
    }

    None
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn get_socket_count() -> Option<usize> {
    None
}

#[cfg(target_os = "linux")]
fn linux_cpu_brand_fallback() -> Option<String> {
    use std::fs;

    if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
        for key in ["model name", "Hardware", "Processor"] {
            for line in cpuinfo.lines() {
                if let Some((name, value)) = line.split_once(':') {
                    if name.trim() == key {
                        let value = value.trim();
                        if !value.is_empty() {
                            return Some(value.to_string());
                        }
                    }
                }
            }
        }
    }

    if let Ok(model) = fs::read_to_string("/sys/firmware/devicetree/base/model") {
        let model = model.trim_matches(char::from(0)).trim();
        if !model.is_empty() {
            return Some(model.to_string());
        }
    }

    None
}

#[cfg(target_os = "macos")]
fn platform_cpu_brand(brand: String) -> String {
    crate::collectors::platform::macos::get_cpu_brand().unwrap_or(brand)
}

#[cfg(target_os = "linux")]
fn platform_cpu_brand(brand: String) -> String {
    if brand.trim().is_empty() || brand == "Unknown CPU" {
        linux_cpu_brand_fallback().unwrap_or(brand)
    } else {
        brand
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn platform_cpu_brand(brand: String) -> String {
    brand
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

/// Linux fallback: read the CPU's rated max frequency from sysfs cpufreq.
///
/// On many ARM SoCs sysinfo reports 0 MHz and there is no CPUID leaf 16h, so
/// without this ARM Linux renders "0.00 GHz". `cpuinfo_max_freq` is in kHz.
#[cfg(target_os = "linux")]
fn linux_cpufreq_max_mhz() -> Option<u64> {
    let entries = std::fs::read_dir("/sys/devices/system/cpu/cpufreq").ok()?;
    entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("policy"))
        .filter_map(|entry| {
            std::fs::read_to_string(entry.path().join("cpuinfo_max_freq"))
                .ok()
                .and_then(|raw| parse_cpufreq_khz_to_mhz(&raw))
        })
        .max()
}

/// Parse a sysfs `cpuinfo_max_freq` value (kHz) into MHz. Returns `None` for an
/// empty/zero/garbage value so the caller falls back to sysinfo.
#[cfg(target_os = "linux")]
fn parse_cpufreq_khz_to_mhz(s: &str) -> Option<u64> {
    let khz: u64 = s.trim().parse().ok()?;
    let mhz = khz / 1000;
    (mhz > 0).then_some(mhz)
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
        if self.physical_cores == 0 {
            format!("{} logical processors", self.logical_cores)
        } else if self.physical_cores == self.logical_cores {
            format!("{} cores", self.physical_cores)
        } else {
            format!(
                "{} cores / {} threads",
                self.physical_cores, self.logical_cores
            )
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod linux_freq_tests {
    use super::*;

    #[test]
    fn cpufreq_khz_to_mhz_converts_and_rejects_bad_input() {
        assert_eq!(parse_cpufreq_khz_to_mhz("2400000"), Some(2400));
        assert_eq!(parse_cpufreq_khz_to_mhz("1800000\n"), Some(1800));
        assert_eq!(parse_cpufreq_khz_to_mhz("999"), None);
        assert_eq!(parse_cpufreq_khz_to_mhz("0"), None);
        assert_eq!(parse_cpufreq_khz_to_mhz("garbage"), None);
        assert_eq!(parse_cpufreq_khz_to_mhz(""), None);
    }
}

#[cfg(all(test, unix))]
mod load_tests {
    use super::*;

    #[test]
    fn normalized_load_preserves_overload_above_one_hundred_percent() {
        assert_eq!(normalize_load(0.5, 4), 12.5);
        assert_eq!(normalize_load(8.0, 4), 200.0);
        assert!(valid_load(0.0));
        assert!(!valid_load(f64::NAN));
        assert!(!valid_load(f64::INFINITY));
        assert!(!valid_load(-0.1));
    }
}

#[cfg(test)]
mod cpu_info_tests {
    use super::*;

    #[test]
    fn unknown_physical_topology_is_labeled_as_logical_only() {
        let info = CpuInfo {
            brand: "CPU".to_string(),
            physical_cores: 0,
            logical_cores: 8,
            sockets: None,
            frequency_mhz: 0,
            frequency_kind: None,
            usage_percent: 0.0,
            load_1m: None,
            load_5m: None,
            load_15m: None,
            raw_load_1m: None,
            raw_load_5m: None,
            raw_load_15m: None,
        };
        assert_eq!(info.cores_string(), "8 logical processors");
    }
}
