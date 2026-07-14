//! macOS-specific information collectors

use super::{CollectMode, PlatformInfo};
use crate::collectors::command::{run_stdout, CommandTimeout};
use std::env;
use std::path::Path;

/// Collect macOS-specific information
/// In fast mode, skip system_profiler calls (slow ~1-2s each)
pub fn collect(mode: CollectMode) -> PlatformInfo {
    let translated = is_rosetta_translated();
    if mode == CollectMode::Fast {
        return PlatformInfo {
            os_build: get_macos_build(),
            macos_codename: get_macos_codename(), // Fast: sw_vers is quick
            boot_mode: None,                      // Skip uname subprocess
            virtualization: None,                 // Skip system_profiler SPHardwareDataType
            desktop_environment: Some("Aqua".to_string()),
            display_server: Some("Quartz".to_string()),
            windows_edition: None,
            gpus: get_gpus_fast(), // ioreg is fast (~20-40ms) vs system_profiler (~1-2s)
            architecture: get_architecture(translated),
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
            encryption: None,
            elevation_unlocks_more: false,
        };
    }

    // One structured snapshot supplies hardware, displays, power, and software
    // data. This avoids multiple localized text parses and expensive repeated
    // system_profiler launches.
    let snapshot = get_system_profiler_snapshot(translated);
    let (gpus, display_resolution) = snapshot
        .as_ref()
        .map(parse_display_info)
        .unwrap_or_else(|| (get_gpus_fast(), None));
    let machine_model = snapshot
        .as_ref()
        .and_then(parse_machine_model)
        .or_else(get_machine_model);
    let battery = snapshot
        .as_ref()
        .and_then(parse_snapshot_battery)
        .or_else(get_battery);
    let boot_mode = snapshot.as_ref().and_then(parse_boot_mode);
    let virtualization = detect_virtualization(snapshot.as_ref());

    PlatformInfo {
        os_build: get_macos_build(),
        macos_codename: get_macos_codename(),
        boot_mode,
        virtualization,
        desktop_environment: Some("Aqua".to_string()),
        display_server: Some("Quartz".to_string()),
        windows_edition: None,
        gpus,
        architecture: get_architecture(translated),
        machine_model,
        cpu_core_topology: get_core_topology(),
        terminal: get_terminal(),
        shell: get_shell(),
        display_resolution,
        battery,
        zfs_health: None,
        motherboard: None,
        bios: None,
        ram_slots: None,
        locale: get_locale(),
        encryption: get_filevault_status(),
        elevation_unlocks_more: false,
    }
}

fn get_system_profiler_snapshot(translated: bool) -> Option<serde_json::Value> {
    let direct_args = [
        "-json",
        "SPHardwareDataType",
        "SPDisplaysDataType",
        "SPPowerDataType",
        "SPSoftwareDataType",
    ];
    // Under Rosetta, system_profiler's translated slice can return a generic
    // machine name ("Mac"), `chip_type: Unknown`, and omit battery maximum
    // capacity. Ask the universal `arch` launcher for the native arm64 slice;
    // fall back to the translated profiler if that unexpectedly fails.
    let json = if translated {
        run_stdout(
            "/usr/bin/arch",
            [
                "-arm64",
                "/usr/sbin/system_profiler",
                "-json",
                "SPHardwareDataType",
                "SPDisplaysDataType",
                "SPPowerDataType",
                "SPSoftwareDataType",
            ],
            CommandTimeout::Slow,
        )
        .or_else(|| run_stdout("system_profiler", direct_args, CommandTimeout::Slow))?
    } else {
        run_stdout("system_profiler", direct_args, CommandTimeout::Slow)?
    };
    serde_json::from_str(&json).ok()
}

fn get_macos_build() -> Option<String> {
    run_stdout("sw_vers", ["-buildVersion"], CommandTimeout::Normal)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Get macOS version codename
fn get_macos_codename() -> Option<String> {
    // Get the OS version from sw_vers (e.g. "26.0", "15.6", "10.15.7").
    let version = run_stdout("sw_vers", ["-productVersion"], CommandTimeout::Normal)?
        .trim()
        .to_string();
    let mut parts = version.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    macos_codename(major, minor)
}

/// Map a macOS `(major, minor)` version to its marketing codename.
///
/// Pure + testable. Apple jumped from 15 (Sequoia, 2024) straight to 26
/// (Tahoe, 2025) when it switched to year-based version numbers, so there is
/// no 16–25. Unknown future majors remain `None`; a codename field must not
/// contain a fabricated version label.
fn macos_codename(major: u32, minor: u32) -> Option<String> {
    let name = match major {
        27 => "Golden Gate",
        26 => "Tahoe",
        15 => "Sequoia",
        14 => "Sonoma",
        13 => "Ventura",
        12 => "Monterey",
        11 => "Big Sur",
        // The 10.x era spanned many releases; key off the minor. Older than
        // High Sierra is long EOL, so fall back to a generic "macOS 10.x".
        10 => match minor {
            15 => "Catalina",
            14 => "Mojave",
            13 => "High Sierra",
            _ => return None,
        },
        _ => return None,
    };
    Some(name.to_string())
}

/// Parse the actual operating-system boot state. Architecture and firmware
/// type are not boot modes, so this intentionally does not label Apple
/// Silicon as a mode or assume every Intel boot was a normal UEFI boot.
fn parse_boot_mode(snapshot: &serde_json::Value) -> Option<String> {
    let software = snapshot["SPSoftwareDataType"].as_array()?.first()?;
    let raw = software["boot_mode"].as_str()?.trim();
    match raw {
        "normal_boot" => Some("Normal".to_string()),
        "safe_boot" => Some("Safe Mode".to_string()),
        "recovery_boot" => Some("Recovery".to_string()),
        value if !value.is_empty() => Some(value.to_string()),
        _ => None,
    }
}

/// Detect if running in a virtual machine
fn detect_virtualization(snapshot: Option<&serde_json::Value>) -> Option<String> {
    if let Some(kind) = snapshot.and_then(virtualization_from_snapshot) {
        return Some(kind);
    }

    // A positive kernel signal is authoritative. A zero or failed probe is
    // simply "not detected"; it is not proof of bare metal.
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

fn virtualization_from_snapshot(snapshot: &serde_json::Value) -> Option<String> {
    let hardware = snapshot["SPHardwareDataType"].as_array()?.first()?;
    let evidence = [
        hardware["machine_name"].as_str(),
        hardware["machine_model"].as_str(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("|")
    .to_ascii_lowercase();
    if evidence.contains("parallels") {
        Some("Parallels".to_string())
    } else if evidence.contains("vmware") {
        Some("VMware".to_string())
    } else if evidence.contains("virtualbox") || evidence.contains("vbox") {
        Some("VirtualBox".to_string())
    } else if evidence.contains("qemu") {
        Some("QEMU".to_string())
    } else if evidence.contains("virtualmac") || evidence.contains("virtual mac") {
        Some("Apple Virtualization".to_string())
    } else {
        None
    }
}

fn parse_display_info(snapshot: &serde_json::Value) -> (Vec<String>, Option<String>) {
    let mut gpus = Vec::new();
    let mut displays: Vec<(String, String)> = Vec::new();
    for adapter in snapshot["SPDisplaysDataType"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if let Some(gpu) = adapter
            .get("sppci_model")
            .or_else(|| adapter.get("_name"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            push_unique_case_insensitive(&mut gpus, gpu);
        }
        for display in adapter["spdisplays_ndrvs"].as_array().into_iter().flatten() {
            if display["spdisplays_online"].as_str() == Some("spdisplays_no") {
                continue;
            }
            let name = display["_name"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("Display");
            let physical = display["_spdisplays_pixels"]
                .as_str()
                .map(normalize_resolution)
                .filter(|value| !value.is_empty());
            let logical = display["_spdisplays_resolution"]
                .as_str()
                .map(normalize_resolution)
                .filter(|value| !value.is_empty());
            let resolution = match (logical, physical) {
                (Some(logical), Some(physical))
                    if logical.split('@').next() != Some(physical.as_str()) =>
                {
                    Some(format!("{} / {}px", logical, physical))
                }
                (Some(logical), _) => Some(logical),
                (None, physical) => physical,
            };
            if let Some(resolution) = resolution {
                if !displays.iter().any(|(existing_name, existing_resolution)| {
                    existing_name.eq_ignore_ascii_case(name)
                        && existing_resolution.eq_ignore_ascii_case(&resolution)
                }) {
                    displays.push((name.to_string(), resolution));
                }
            }
        }
    }
    let display = match displays.as_slice() {
        [] => None,
        [(_, resolution)] => Some(resolution.clone()),
        _ => Some(
            displays
                .into_iter()
                .map(|(name, resolution)| format!("{}: {}", name, resolution))
                .collect::<Vec<_>>()
                .join("; "),
        ),
    };
    (gpus, display)
}

fn normalize_resolution(value: &str) -> String {
    value
        .replace(" x ", "x")
        .replace(" × ", "x")
        .replace(" @ ", "@")
        .replace(".00Hz", "Hz")
        .trim()
        .to_string()
}

fn push_unique_case_insensitive(values: &mut Vec<String>, value: &str) {
    let value = value.trim();
    if !value.is_empty()
        && !values
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(value))
    {
        values.push(value.to_string());
    }
}

fn parse_machine_model(snapshot: &serde_json::Value) -> Option<String> {
    let hardware = snapshot["SPHardwareDataType"].as_array()?.first()?;
    let name = hardware["machine_name"].as_str().map(str::trim);
    let identifier = hardware["machine_model"].as_str().map(str::trim);
    match (
        name.filter(|v| !v.is_empty()),
        identifier.filter(|v| !v.is_empty()),
    ) {
        (Some(name), Some(identifier)) => Some(format!("{} ({})", name, identifier)),
        (Some(name), None) => Some(name.to_string()),
        (None, Some(identifier)) => Some(identifier.to_string()),
        (None, None) => None,
    }
}

/// Get GPU names quickly using ioreg (fast, ~20-40ms vs system_profiler ~1-2s)
fn get_gpus_fast() -> Vec<String> {
    let mut gpus = Vec::new();

    // Try ioreg for discrete/integrated GPUs
    if let Some(stdout) = run_stdout("ioreg", ["-rc", "IOGPUDevice"], CommandTimeout::Normal) {
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.contains("\"model\"") {
                // Format: "model" = <"AMD Radeon Pro 5500M">
                if let Some(start) = trimmed.find("<\"") {
                    if let Some(end) = trimmed.rfind("\">") {
                        let gpu = &trimmed[start + 2..end];
                        push_unique_case_insensitive(&mut gpus, gpu);
                    }
                }
            }
        }
    }

    // Fallback for Apple Silicon: use sysctl to detect the chip
    if gpus.is_empty() {
        if let Some(output) = run_stdout(
            "sysctl",
            ["-n", "machdep.cpu.brand_string"],
            CommandTimeout::Normal,
        ) {
            let brand = output.trim().to_string();
            if brand.contains("Apple") {
                // Apple Silicon has integrated GPU in the SoC
                push_unique_case_insensitive(&mut gpus, &brand);
            }
        }
    }

    gpus
}

/// Get system architecture
fn get_architecture(translated: bool) -> Option<String> {
    let host = run_stdout("uname", ["-m"], CommandTimeout::Normal)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| std::env::consts::ARCH.to_string());
    Some(format_architecture(
        &host,
        std::env::consts::ARCH,
        translated,
    ))
}

pub(crate) fn is_rosetta_translated() -> bool {
    run_stdout(
        "sysctl",
        ["-n", "sysctl.proc_translated"],
        CommandTimeout::Normal,
    )
    .map(|s| s.trim() == "1")
    .unwrap_or(false)
}

fn format_architecture(host: &str, runtime: &str, translated: bool) -> String {
    if translated {
        // Rosetta 2 exists only on Apple Silicon. `uname -m` can report the
        // translated process architecture, so state both scopes explicitly.
        return format!("arm64 host / {} (Rosetta 2)", runtime);
    }
    host.to_string()
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
    let mut levels = Vec::new();
    let count = sysctl_usize("hw.nperflevels")?;
    for index in 0..count.min(16) {
        let name = run_stdout(
            "sysctl",
            ["-n", &format!("hw.perflevel{}.name", index)],
            CommandTimeout::Normal,
        )
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
        let count = sysctl_usize(&format!("hw.perflevel{}.physicalcpu", index));
        let (Some(name), Some(count)) = (name, count) else {
            continue;
        };
        let label = match name.to_ascii_lowercase().as_str() {
            value if value.contains("performance") => "P".to_string(),
            value if value.contains("efficiency") => "E".to_string(),
            _ => name,
        };
        if count > 0 {
            levels.push(format!("{}{}", count, label));
        }
    }
    (levels.len() > 1).then(|| levels.join(" + "))
}

fn sysctl_usize(name: &str) -> Option<usize> {
    run_stdout("sysctl", ["-n", name], CommandTimeout::Normal)?
        .trim()
        .parse()
        .ok()
}

/// Get terminal emulator name
fn get_terminal() -> Option<String> {
    // TERM_PROGRAM is the most direct signal. LC_TERMINAL and the inherited
    // bundle identifier cover iTerm2 and app-hosted shells that omit it.
    for variable in ["TERM_PROGRAM", "LC_TERMINAL", "__CFBundleIdentifier"] {
        if let Ok(value) = env::var(variable) {
            if let Some(label) = terminal_label(value.trim()) {
                return Some(label);
            }
        }
    }

    // TERM describes terminal capabilities, not the emulator. Avoid reporting
    // `dumb`/`xterm-256color` as if it were an application name.
    None
}

fn terminal_label(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    Some(
        match value {
            "Apple_Terminal" | "com.apple.Terminal" => "Terminal.app",
            "iTerm.app" | "iTerm2" | "com.googlecode.iterm2" => "iTerm2",
            "vscode" | "com.microsoft.VSCode" => "VS Code",
            "WarpTerminal" | "dev.warp.Warp-Stable" => "Warp",
            "WezTerm" | "com.github.wez.wezterm" => "WezTerm",
            "ghostty" | "com.mitchellh.ghostty" => "Ghostty",
            "com.openai.codex" => "Codex",
            other => other,
        }
        .to_string(),
    )
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
        "bash" | "zsh" | "fish" => run_stdout(&shell_path, ["--version"], CommandTimeout::Normal),
        _ => None,
    };

    if let Some(version_str) = version_output {
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

/// Get battery status
fn get_battery() -> Option<String> {
    let stdout = run_stdout("pmset", ["-g", "batt"], CommandTimeout::Normal)?;
    parse_pmset_battery(&stdout)
}

fn parse_pmset_battery(output: &str) -> Option<String> {
    let line = output.lines().find(|line| line.contains('%'))?;
    let percentage = line.split_whitespace().find_map(|word| {
        let word = word.trim_matches(|c: char| c == ';' || c == ',');
        let number = word.strip_suffix('%')?.parse::<u8>().ok()?;
        (number <= 100).then(|| format!("{}%", number))
    })?;
    let lower = line.to_ascii_lowercase();
    let status = if lower.contains("discharging") {
        Some("Discharging")
    } else if lower.contains("not charging") {
        Some("Not charging")
    } else if lower.contains("charging") {
        Some("Charging")
    } else if lower.contains("charged") {
        Some("Charged")
    } else {
        None
    };
    Some(match status {
        Some(status) => format!("{} ({})", percentage, status),
        None => percentage,
    })
}

/// Get system locale
fn get_locale() -> Option<String> {
    for var in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Ok(value) = env::var(var) {
            if !value.trim().is_empty() {
                return Some(value.trim().to_string());
            }
        }
    }

    // Region preference is a fallback when no process locale exists. Preserve
    // Apple's @rg override rather than silently discarding it.
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

    None
}

fn clean_apple_locale(locale: &str) -> String {
    locale.trim().to_string()
}

fn parse_snapshot_battery(snapshot: &serde_json::Value) -> Option<String> {
    let battery = snapshot["SPPowerDataType"]
        .as_array()?
        .iter()
        .find(|entry| entry["_name"].as_str() == Some("spbattery_information"))?;
    let charge = &battery["sppower_battery_charge_info"];
    let percent = charge["sppower_battery_state_of_charge"].as_u64()?;
    if percent > 100 {
        return None;
    }
    let charging = charge["sppower_battery_is_charging"].as_str() == Some("TRUE");
    let full = charge["sppower_battery_fully_charged"].as_str() == Some("TRUE");
    let connected =
        find_string_value(snapshot, "sppower_battery_charger_connected") == Some("TRUE");
    let status = if charging {
        "Charging"
    } else if full {
        "Charged"
    } else if connected {
        "Plugged in"
    } else {
        "Discharging"
    };
    let mut result = format!("{}% ({})", percent, status);
    let health = &battery["sppower_battery_health_info"];
    let condition = health["sppower_battery_health"].as_str();
    let maximum = health["sppower_battery_health_maximum_capacity"].as_str();
    let cycles = health["sppower_battery_cycle_count"].as_u64().or_else(|| {
        health["sppower_battery_cycle_count"]
            .as_str()
            .and_then(|value| value.trim().parse().ok())
    });
    let mut details = Vec::new();
    if let Some(condition) = condition.map(str::trim).filter(|value| !value.is_empty()) {
        details.push(condition.to_string());
    }
    if let Some(maximum) = maximum {
        details.push(format!("max {}", maximum));
    }
    if let Some(cycles) = cycles {
        details.push(format!("{} cycles", cycles));
    }
    if !details.is_empty() {
        result.push_str("; ");
        result.push_str(&details.join(", "));
    }
    Some(result)
}

fn find_string_value<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(found) = map.get(key).and_then(serde_json::Value::as_str) {
                return Some(found);
            }
            map.values().find_map(|value| find_string_value(value, key))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|value| find_string_value(value, key)),
        _ => None,
    }
}

fn get_filevault_status() -> Option<String> {
    let status = crate::collectors::command::run_stdout_c_locale(
        "fdesetup",
        ["status"],
        CommandTimeout::Normal,
    )?;
    parse_filevault_status(&status)
}

fn parse_filevault_status(status: &str) -> Option<String> {
    let lower = status.to_ascii_lowercase();
    if lower.contains("encryption in progress") {
        Some(format_filevault_progress(
            "FileVault encryption in progress",
            &lower,
        ))
    } else if lower.contains("decryption in progress") {
        Some(format_filevault_progress(
            "FileVault decryption in progress",
            &lower,
        ))
    } else if lower.contains("filevault is on") {
        Some("FileVault On".to_string())
    } else if lower.contains("filevault is off") {
        Some("FileVault Off".to_string())
    } else {
        None
    }
}

fn format_filevault_progress(label: &str, status: &str) -> String {
    let percent = status
        .split("percent completed")
        .nth(1)
        .and_then(|tail| {
            tail.chars()
                .position(|character| character.is_ascii_digit())
                .map(|i| &tail[i..])
        })
        .and_then(|tail| {
            let digits: String = tail
                .chars()
                .take_while(|character| character.is_ascii_digit())
                .collect();
            digits.parse::<u8>().ok()
        })
        .filter(|percent| *percent <= 100);
    percent
        .map(|percent| format!("{} ({}%)", label, percent))
        .unwrap_or_else(|| label.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_locale_preserves_region_extension() {
        assert_eq!(clean_apple_locale("en_US@rg=gbzzzz"), "en_US@rg=gbzzzz");
    }

    #[test]
    fn macos_codename_maps_current_and_future_versions() {
        // Current: Tahoe (Apple jumped 15 -> 26 in 2025; no 16-25 exist).
        assert_eq!(macos_codename(26, 0), Some("Tahoe".to_string()));
        assert_eq!(macos_codename(15, 6), Some("Sequoia".to_string()));
        assert_eq!(macos_codename(11, 7), Some("Big Sur".to_string()));
        // 10.x era keys off the minor.
        assert_eq!(macos_codename(10, 15), Some("Catalina".to_string()));
        assert_eq!(macos_codename(10, 14), Some("Mojave".to_string()));
        assert_eq!(macos_codename(10, 11), None);
        assert_eq!(macos_codename(27, 0), Some("Golden Gate".to_string()));
        assert_eq!(macos_codename(28, 0), None);
    }

    #[test]
    fn rosetta_architecture_names_host_and_process_scopes() {
        assert_eq!(
            format_architecture("x86_64", "x86_64", true),
            "arm64 host / x86_64 (Rosetta 2)"
        );
        assert_eq!(format_architecture("arm64", "aarch64", false), "arm64");
        assert_eq!(format_architecture("x86_64", "x86_64", false), "x86_64");
    }

    #[test]
    fn boot_mode_comes_from_the_os_state_not_the_cpu_architecture() {
        let normal = serde_json::json!({
            "SPSoftwareDataType": [{ "boot_mode": "normal_boot" }]
        });
        let safe = serde_json::json!({
            "SPSoftwareDataType": [{ "boot_mode": "safe_boot" }]
        });
        assert_eq!(parse_boot_mode(&normal).as_deref(), Some("Normal"));
        assert_eq!(parse_boot_mode(&safe).as_deref(), Some("Safe Mode"));
        assert_eq!(parse_boot_mode(&serde_json::json!({})), None);
    }

    #[test]
    fn snapshot_virtualization_identifies_known_mac_guests() {
        let apple = serde_json::json!({
            "SPHardwareDataType": [{
                "machine_name": "Mac",
                "machine_model": "VirtualMac2,1"
            }]
        });
        let parallels = serde_json::json!({
            "SPHardwareDataType": [{
                "machine_name": "Parallels ARM Virtual Machine",
                "machine_model": "Parallels20,1"
            }]
        });
        assert_eq!(
            virtualization_from_snapshot(&apple).as_deref(),
            Some("Apple Virtualization")
        );
        assert_eq!(
            virtualization_from_snapshot(&parallels).as_deref(),
            Some("Parallels")
        );
    }

    #[test]
    fn terminal_labels_cover_native_macos_hosts() {
        assert_eq!(
            terminal_label("com.apple.Terminal").as_deref(),
            Some("Terminal.app")
        );
        assert_eq!(terminal_label("com.openai.codex").as_deref(), Some("Codex"));
        assert_eq!(
            terminal_label("dev.warp.Warp-Stable").as_deref(),
            Some("Warp")
        );
        assert_eq!(terminal_label(""), None);
    }

    #[test]
    fn display_parser_prefers_current_logical_mode_and_ignores_offline_entries() {
        let snapshot = serde_json::json!({
            "SPDisplaysDataType": [{
                "sppci_model": "Apple M2",
                "spdisplays_ndrvs": [{
                    "_name": "Color LCD",
                    "_spdisplays_pixels": "2880 x 1800",
                    "_spdisplays_resolution": "1440 x 900 @ 60.00Hz",
                    "spdisplays_online": "spdisplays_yes"
                }, {
                    "_name": "Disconnected Display",
                    "_spdisplays_pixels": "1920 x 1080",
                    "spdisplays_online": "spdisplays_no"
                }]
            }, {
                "sppci_model": "apple m2"
            }]
        });
        assert_eq!(
            parse_display_info(&snapshot),
            (
                vec!["Apple M2".to_string()],
                Some("1440x900@60Hz / 2880x1800px".to_string())
            )
        );
    }

    #[test]
    fn parses_structured_snapshot_without_leaking_device_identifiers() {
        let snapshot: serde_json::Value = serde_json::from_str(
            r#"{
          "SPHardwareDataType": [{
            "machine_name": "MacBook Pro",
            "machine_model": "Mac14,7",
            "serial_number": "must-not-appear"
          }],
          "SPDisplaysDataType": [{
            "sppci_model": "Apple M2",
            "spdisplays_ndrvs": [{
              "_name": "Color LCD",
              "_spdisplays_pixels": "2880 x 1800",
              "_spdisplays_display-serial-number": "must-not-appear"
            }]
          }],
          "SPPowerDataType": [{
            "_name": "spbattery_information",
            "sppower_battery_charge_info": {
              "sppower_battery_state_of_charge": 49,
              "sppower_battery_is_charging": "FALSE",
              "sppower_battery_fully_charged": "FALSE"
            },
            "sppower_battery_health_info": {
              "sppower_battery_health": "Good",
              "sppower_battery_health_maximum_capacity": "88%",
              "sppower_battery_cycle_count": 114
            }
          }, {
            "sppower_battery_charger_connected": "FALSE"
          }]
        }"#,
        )
        .unwrap();
        assert_eq!(
            parse_machine_model(&snapshot),
            Some("MacBook Pro (Mac14,7)".to_string())
        );
        assert_eq!(
            parse_display_info(&snapshot),
            (vec!["Apple M2".to_string()], Some("2880x1800".to_string()))
        );
        assert_eq!(
            parse_snapshot_battery(&snapshot),
            Some("49% (Discharging); Good, max 88%, 114 cycles".to_string())
        );
        let combined = format!(
            "{:?}{:?}{:?}",
            parse_machine_model(&snapshot),
            parse_display_info(&snapshot),
            parse_snapshot_battery(&snapshot)
        );
        assert!(!combined.contains("must-not-appear"));
    }

    #[test]
    fn parses_pmset_battery_without_device_prefix() {
        let output = "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=123)\t49%; discharging; 4:00 remaining\n";
        assert_eq!(
            parse_pmset_battery(output),
            Some("49% (Discharging)".to_string())
        );
        assert_eq!(
            parse_pmset_battery(" -InternalBattery-0\t100%; charged; 0:00 remaining\n"),
            Some("100% (Charged)".to_string())
        );
        assert_eq!(
            parse_pmset_battery(" -InternalBattery-0\t80%; not charging; 0:00 remaining\n"),
            Some("80% (Not charging)".to_string())
        );
        assert_eq!(
            parse_pmset_battery(" -InternalBattery-0\t255%; charging\n"),
            None
        );
    }

    #[test]
    fn parses_filevault_states() {
        assert_eq!(
            parse_filevault_status("FileVault is On."),
            Some("FileVault On".to_string())
        );
        assert_eq!(
            parse_filevault_status("FileVault is Off."),
            Some("FileVault Off".to_string())
        );
        assert_eq!(
            parse_filevault_status(
                "FileVault is On. Encryption in progress: Percent completed = 42"
            ),
            Some("FileVault encryption in progress (42%)".to_string())
        );
        assert_eq!(
            parse_filevault_status(
                "FileVault is Off. Decryption in progress: Percent completed = 73"
            ),
            Some("FileVault decryption in progress (73%)".to_string())
        );
        assert_eq!(
            parse_filevault_status(
                "FileVault is On. Encryption in progress: Percent completed = 255"
            ),
            Some("FileVault encryption in progress".to_string())
        );
    }
}
