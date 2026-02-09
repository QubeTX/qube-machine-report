//! macOS-specific information collectors

use super::{CollectMode, PlatformInfo};
use std::env;
use std::path::Path;
use std::process::Command;

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
            gpus: Vec::new(), // Skip system_profiler SPDisplaysDataType
            architecture: get_architecture(),
            terminal: get_terminal(), // Fast: reads env vars
            shell: get_shell(),       // Fast: reads env var + quick subprocess
            display_resolution: None, // Skip system_profiler SPDisplaysDataType
            battery: get_battery(),   // Fast: pmset is quick
            locale: get_locale(),     // Fast: reads env var
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
        terminal: get_terminal(),
        shell: get_shell(),
        display_resolution,
        battery: get_battery(),
        locale: get_locale(),
    }
}

/// Get macOS version codename
fn get_macos_codename() -> Option<String> {
    // Get the OS version from sw_vers
    let output = Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()?;

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
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
    let output = Command::new("uname").arg("-m").output().ok()?;

    let arch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if arch == "arm64" {
        Some("Apple Silicon".to_string())
    } else {
        Some("UEFI".to_string())
    }
}

/// Detect if running in a virtual machine
fn detect_virtualization() -> Option<String> {
    // Check system_profiler for VM indicators
    let output = Command::new("system_profiler")
        .arg("SPHardwareDataType")
        .output()
        .ok()?;

    let info = String::from_utf8_lossy(&output.stdout).to_lowercase();

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
    let output = Command::new("sysctl")
        .arg("-n")
        .arg("kern.hv_vmm_present")
        .output()
        .ok()?;

    let vmm = String::from_utf8_lossy(&output.stdout).trim().to_string();
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

    if let Ok(output) = Command::new("system_profiler")
        .args(["SPDisplaysDataType"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
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

fn get_gpus() -> Vec<String> {
    get_display_info().0
}

/// Get system architecture
fn get_architecture() -> Option<String> {
    Some(std::env::consts::ARCH.to_string())
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
        "bash" => Command::new(&shell_path).args(["--version"]).output().ok(),
        "zsh" => Command::new(&shell_path).args(["--version"]).output().ok(),
        "fish" => Command::new(&shell_path).args(["--version"]).output().ok(),
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
fn get_display_resolution() -> Option<String> {
    get_display_info().1
}

/// Get battery status
fn get_battery() -> Option<String> {
    if let Ok(output) = Command::new("pmset").args(["-g", "batt"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
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
                        // Capitalize first letter
                        let status = status
                            .chars()
                            .next()
                            .map(|c| c.to_uppercase().to_string())
                            .unwrap_or_default()
                            + &status[1..];
                        return Some(format!("{} ({})", percentage, status));
                    }
                    return Some(percentage);
                }
            }
        }
    }

    None
}

/// Get system locale
fn get_locale() -> Option<String> {
    // Check LANG environment variable
    if let Ok(lang) = env::var("LANG") {
        let clean = lang.split('.').next().unwrap_or(&lang);
        return Some(clean.to_string());
    }

    // Try defaults command for AppleLocale
    if let Ok(output) = Command::new("defaults")
        .args(["read", "-g", "AppleLocale"])
        .output()
    {
        let locale = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !locale.is_empty() {
            return Some(locale);
        }
    }

    None
}
