//! macOS-specific information collectors

use super::{CollectMode, PlatformInfo};
use crate::collectors::command::{run_stdout, CommandTimeout};
use std::env;
use std::path::Path;

/// Collect macOS-specific information
/// In fast mode, skip system_profiler calls (slow ~1-2s each)
pub fn collect(mode: CollectMode) -> PlatformInfo {
    if mode == CollectMode::Fast {
        return PlatformInfo {
            macos_codename: get_macos_codename(), // Fast: sw_vers is quick
            boot_mode: None,                      // Skip uname subprocess
            virtualization: None,                 // Skip system_profiler SPHardwareDataType
            desktop_environment: Some("Aqua".to_string()),
            display_server: Some("Quartz".to_string()),
            windows_edition: None,
            gpus: get_gpus_fast(), // ioreg is fast (~20-40ms) vs system_profiler (~1-2s)
            architecture: get_architecture(),
            machine_model: get_machine_model(),
            cpu_core_topology: get_core_topology(),
            terminal: get_terminal(), // Fast: reads env vars
            shell: get_shell(),       // Fast: reads env var + quick subprocess
            display_resolution: None, // Skip system_profiler SPDisplaysDataType
            battery: get_battery(),   // Fast: pmset is quick
            zfs_health: None,
            motherboard: None,
            bios: None,
            ram_slots: None,
            locale: get_locale(), // Fast: reads env var
            encryption: None,     // FileVault detection deferred to PR #2
        };
    }

    // Call system_profiler once for both GPUs and resolution
    let (gpus, display_resolution) = get_display_info();

    PlatformInfo {
        macos_codename: get_macos_codename(),
        boot_mode: detect_boot_mode(),
        virtualization: detect_virtualization(),
        desktop_environment: Some("Aqua".to_string()),
        display_server: Some("Quartz".to_string()),
        windows_edition: None,
        gpus,
        architecture: get_architecture(),
        machine_model: get_machine_model(),
        cpu_core_topology: get_core_topology(),
        terminal: get_terminal(),
        shell: get_shell(),
        display_resolution,
        battery: get_battery_with_health().or_else(get_battery),
        zfs_health: None,
        motherboard: None,
        bios: None,
        ram_slots: None,
        locale: get_locale(),
        encryption: None, // FileVault detection deferred to PR #2
    }
}

/// Get macOS version codename
fn get_macos_codename() -> Option<String> {
    // Get the OS version from sw_vers
    let version = run_stdout("sw_vers", ["-productVersion"], CommandTimeout::Normal)?
        .trim()
        .to_string();
    let major: u32 = version.split('.').next()?.parse().ok()?;

    // Map major version to codename
    let codename = match major {
        15 => "Sequoia",
        14 => "Sonoma",
        13 => "Ventura",
        12 => "Monterey",
        11 => "Big Sur",
        10 => "Catalina", // or earlier
        _ => return None,
    };

    Some(codename.to_string())
}

/// Detect boot mode (always UEFI on Intel Macs, native on Apple Silicon)
fn detect_boot_mode() -> Option<String> {
    // Check if running on Apple Silicon
    let arch = run_stdout("uname", ["-m"], CommandTimeout::Normal)?
        .trim()
        .to_string();

    if arch == "arm64" {
        Some("Apple Silicon".to_string())
    } else {
        Some("UEFI".to_string())
    }
}

/// Detect if running in a virtual machine
fn detect_virtualization() -> Option<String> {
    // Check system_profiler for VM indicators
    let info = run_stdout(
        "system_profiler",
        ["SPHardwareDataType"],
        CommandTimeout::Slow,
    )?
    .to_lowercase();

    if info.contains("vmware") {
        return Some("VMware".to_string());
    }
    if info.contains("virtualbox") {
        return Some("VirtualBox".to_string());
    }
    if info.contains("parallels") {
        return Some("Parallels".to_string());
    }
    if info.contains("qemu") {
        return Some("QEMU".to_string());
    }

    // Check for Apple Virtualization framework
    let vmm = run_stdout(
        "sysctl",
        ["-n", "kern.hv_vmm_present"],
        CommandTimeout::Normal,
    )?
    .trim()
    .to_string();
    if vmm == "1" {
        return Some("Virtual Machine".to_string());
    }

    None
}

/// Get GPU names and display resolution from a single system_profiler call.
/// Saves ~1-2 seconds by avoiding duplicate SPDisplaysDataType invocations.
fn get_display_info() -> (Vec<String>, Option<String>) {
    let mut gpus = Vec::new();
    let mut resolution = None;

    if let Some(stdout) = run_stdout(
        "system_profiler",
        ["SPDisplaysDataType"],
        CommandTimeout::Slow,
    ) {
        for line in stdout.lines() {
            let trimmed = line.trim();
            // GPU: "Chipset Model:" lines
            if trimmed.starts_with("Chipset Model:") {
                if let Some(gpu) = trimmed.strip_prefix("Chipset Model:") {
                    let gpu = gpu.trim();
                    if !gpu.is_empty() {
                        gpus.push(gpu.to_string());
                    }
                }
            }
            // Resolution: "Resolution:" lines
            if resolution.is_none() && trimmed.starts_with("Resolution:") {
                if let Some(res) = trimmed.strip_prefix("Resolution:") {
                    let res = res.trim();
                    let clean: String = res
                        .chars()
                        .filter(|c| c.is_ascii_digit() || *c == 'x')
                        .collect();
                    if clean.contains('x') {
                        resolution = Some(clean);
                    } else if !res.is_empty() {
                        resolution = Some(res.to_string());
                    }
                }
            }
        }
    }

    (gpus, resolution)
}

/// Get GPU names quickly using ioreg (fast, ~20-40ms vs system_profiler ~1-2s)
fn get_gpus_fast() -> Vec<String> {
    let mut gpus = Vec::new();

    // Try ioreg for discrete/integrated GPUs
    if let Some(stdout) = run_stdout(
        "/usr/sbin/ioreg",
        ["-rc", "IOGPUDevice"],
        CommandTimeout::Normal,
    ) {
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.contains("\"model\"") {
                // Format: "model" = <"AMD Radeon Pro 5500M">
                if let Some(start) = trimmed.find("<\"") {
                    if let Some(end) = trimmed.rfind("\">") {
                        let gpu = &trimmed[start + 2..end];
                        if !gpu.is_empty() {
                            gpus.push(gpu.to_string());
                        }
                    }
                }
            }
        }
    }

    // Fallback for Apple Silicon: use sysctl to detect the chip
    if gpus.is_empty() {
        if let Some(output) = run_stdout(
            "/usr/sbin/sysctl",
            ["-n", "machdep.cpu.brand_string"],
            CommandTimeout::Normal,
        ) {
            let brand = output.trim().to_string();
            if brand.contains("Apple") {
                // Apple Silicon has integrated GPU in the SoC
                let chip = brand.replace("Apple ", "");
                gpus.push(format!("Apple {} GPU", chip));
            }
        }
    }

    gpus
}

#[allow(dead_code)]
fn get_gpus() -> Vec<String> {
    get_display_info().0
}

/// Get system architecture
fn get_architecture() -> Option<String> {
    let translated = run_stdout(
        "sysctl",
        ["-n", "sysctl.proc_translated"],
        CommandTimeout::Normal,
    )
    .map(|s| s.trim() == "1")
    .unwrap_or(false);
    if translated {
        return Some("x86_64 (Apple Silicon, Rosetta 2)".to_string());
    }
    Some(std::env::consts::ARCH.to_string())
}

pub fn get_cpu_brand() -> Option<String> {
    let brand = run_stdout(
        "sysctl",
        ["-n", "machdep.cpu.brand_string"],
        CommandTimeout::Normal,
    )?
    .trim()
    .to_string();
    if brand.is_empty() {
        None
    } else {
        Some(brand)
    }
}

pub fn apple_silicon_max_frequency_mhz(brand: &str) -> Option<u64> {
    let brand = brand.to_ascii_lowercase();
    let mhz = if brand.contains("m4") {
        4400
    } else if brand.contains("m3") {
        4050
    } else if brand.contains("m2") {
        3500
    } else if brand.contains("m1") {
        3200
    } else {
        return None;
    };
    Some(mhz)
}

pub fn get_computer_name() -> Option<String> {
    let name = run_stdout("scutil", ["--get", "ComputerName"], CommandTimeout::Normal)?
        .trim()
        .to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn get_machine_model() -> Option<String> {
    let model = run_stdout("sysctl", ["-n", "hw.model"], CommandTimeout::Normal)?
        .trim()
        .to_string();
    if model.is_empty() {
        None
    } else {
        Some(model)
    }
}

fn get_core_topology() -> Option<String> {
    let p = sysctl_usize("hw.perflevel0.physicalcpu")?;
    let e = sysctl_usize("hw.perflevel1.physicalcpu").unwrap_or(0);
    if e == 0 {
        return None;
    }
    Some(format!("{}P + {}E", p, e))
}

fn sysctl_usize(name: &str) -> Option<usize> {
    run_stdout("sysctl", ["-n", name], CommandTimeout::Normal)?
        .trim()
        .parse()
        .ok()
}

/// Get terminal emulator name
fn get_terminal() -> Option<String> {
    // Check TERM_PROGRAM first (set by most macOS terminals)
    if let Ok(term) = env::var("TERM_PROGRAM") {
        return Some(match term.as_str() {
            "Apple_Terminal" => "Terminal.app".to_string(),
            "iTerm.app" => "iTerm2".to_string(),
            "vscode" => "VS Code".to_string(),
            _ => term,
        });
    }

    // Fall back to TERM
    env::var("TERM").ok()
}

/// Get shell name and version
fn get_shell() -> Option<String> {
    let shell_path = env::var("SHELL").ok()?;
    let shell_name = Path::new(&shell_path)
        .file_name()?
        .to_string_lossy()
        .to_string();

    // Try to get version
    let version_output = match shell_name.as_str() {
        "bash" => crate::collectors::command::run_output(
            &shell_path,
            ["--version"],
            CommandTimeout::Normal,
        ),
        "zsh" => crate::collectors::command::run_output(
            &shell_path,
            ["--version"],
            CommandTimeout::Normal,
        ),
        "fish" => crate::collectors::command::run_output(
            &shell_path,
            ["--version"],
            CommandTimeout::Normal,
        ),
        _ => None,
    };

    if let Some(output) = version_output {
        let version_str = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = version_str.lines().next() {
            // Extract version number
            for word in line.split_whitespace() {
                if word
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                {
                    let version: String = word
                        .chars()
                        .take_while(|c| c.is_ascii_digit() || *c == '.')
                        .collect();
                    if !version.is_empty() {
                        return Some(format!("{} {}", shell_name, version));
                    }
                }
            }
        }
    }

    Some(shell_name)
}

/// Get display resolution
#[allow(dead_code)]
fn get_display_resolution() -> Option<String> {
    get_display_info().1
}

/// Get battery status
fn get_battery() -> Option<String> {
    if let Some(stdout) = run_stdout("pmset", ["-g", "batt"], CommandTimeout::Normal) {
        // Parse output like:
        // Now drawing from 'Battery Power'
        //  -InternalBattery-0 (id=...)	85%; charging; 1:23 remaining
        for line in stdout.lines() {
            if line.contains("InternalBattery") || line.contains('%') {
                // Extract percentage and status
                let mut percentage = String::new();
                let mut status = String::new();

                for part in line.split(';') {
                    let part = part.trim();
                    if part.ends_with('%') {
                        percentage = part.to_string();
                    } else if part.contains("charging")
                        || part.contains("discharging")
                        || part.contains("charged")
                        || part.contains("AC")
                    {
                        status = part.to_string();
                    }
                }

                // Also check for percentage in format "85%"
                if percentage.is_empty() {
                    for word in line.split_whitespace() {
                        if word.ends_with('%') {
                            percentage = word.to_string();
                            break;
                        }
                    }
                }

                if !percentage.is_empty() {
                    if !status.is_empty() {
                        // Capitalize first letter (safe for multi-byte chars)
                        let status = {
                            let mut chars = status.chars();
                            match chars.next() {
                                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                                None => String::new(),
                            }
                        };
                        return Some(format!("{} ({})", percentage, status));
                    }
                    return Some(percentage);
                }
            }
        }
    }

    None
}

fn get_battery_with_health() -> Option<String> {
    let base = get_battery()?;
    let json = run_stdout(
        "system_profiler",
        ["SPPowerDataType", "-json"],
        CommandTimeout::Slow,
    )?;
    let health = parse_system_profiler_battery_health(&json)?;
    Some(format!("{}; {}", base, health))
}

/// Get system locale
fn get_locale() -> Option<String> {
    // Try defaults command for AppleLocale
    if let Some(output) = run_stdout(
        "defaults",
        ["read", "-g", "AppleLocale"],
        CommandTimeout::Normal,
    ) {
        let locale = clean_apple_locale(output.trim());
        if !locale.is_empty() {
            return Some(locale);
        }
    }

    // Check LANG environment variable
    if let Ok(lang) = env::var("LANG") {
        let clean = lang.split('.').next().unwrap_or(&lang);
        return Some(clean.to_string());
    }

    None
}

fn clean_apple_locale(locale: &str) -> String {
    locale.split('@').next().unwrap_or(locale).to_string()
}

fn parse_system_profiler_battery_health(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let batteries = value["SPPowerDataType"].as_array()?;
    let text = batteries
        .iter()
        .find_map(find_battery_health_value)
        .filter(|s| !s.is_empty())?;
    Some(format!("health {}", text))
}

fn find_battery_health_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                let key_lower = key.to_ascii_lowercase();
                if key_lower.contains("condition") || key_lower.contains("health") {
                    if let Some(s) = value.as_str() {
                        return Some(s.to_string());
                    }
                }
                if let Some(found) = find_battery_health_value(value) {
                    return Some(found);
                }
            }
            None
        }
        serde_json::Value::Array(values) => values.iter().find_map(find_battery_health_value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_locale_strips_region_extension() {
        assert_eq!(clean_apple_locale("en_US@rg=gbzzzz"), "en_US");
    }

    #[test]
    fn apple_silicon_frequency_lookup_handles_m_families() {
        assert_eq!(apple_silicon_max_frequency_mhz("Apple M1 Pro"), Some(3200));
        assert_eq!(apple_silicon_max_frequency_mhz("Apple M4 Max"), Some(4400));
        assert_eq!(apple_silicon_max_frequency_mhz("Intel Core i9"), None);
    }

    #[test]
    fn parses_system_profiler_battery_health_json() {
        let json = r#"{
          "SPPowerDataType": [{
            "sppower_battery_health_info": {
              "sppower_battery_health": "Normal"
            }
          }]
        }"#;
        assert_eq!(
            parse_system_profiler_battery_health(json),
            Some("health Normal".to_string())
        );
    }
}
