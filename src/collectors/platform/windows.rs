//! Windows-specific information collectors

use super::PlatformInfo;
use std::env;
use std::process::Command;

/// Collect Windows-specific information
pub fn collect() -> PlatformInfo {
    PlatformInfo {
        windows_edition: get_windows_edition(),
        boot_mode: detect_boot_mode(),
        virtualization: detect_virtualization(),
        desktop_environment: Some("Windows Shell".to_string()),
        display_server: Some("DWM".to_string()),
        macos_codename: None,
        gpus: get_gpus(),
        architecture: get_architecture(),
        terminal: get_terminal(),
        shell: get_shell(),
        display_resolution: get_display_resolution(),
        battery: get_battery(),
        locale: get_locale(),
    }
}

/// Get Windows edition (Home, Pro, Enterprise, etc.)
fn get_windows_edition() -> Option<String> {
    // Use WMI query via PowerShell
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_OperatingSystem).Caption",
        ])
        .output()
        .ok()?;

    let caption = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if caption.is_empty() {
        None
    } else {
        Some(caption)
    }
}

/// Detect boot mode (UEFI or Legacy BIOS)
fn detect_boot_mode() -> Option<String> {
    // Check firmware type via PowerShell
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "$env:firmware_type",
        ])
        .output()
        .ok()?;

    let firmware = String::from_utf8_lossy(&output.stdout).trim().to_uppercase();

    if firmware.contains("UEFI") {
        Some("UEFI".to_string())
    } else if firmware.contains("LEGACY") || firmware.contains("BIOS") {
        Some("Legacy BIOS".to_string())
    } else {
        // Alternative check using bcdedit
        let output = Command::new("cmd")
            .args(["/c", "bcdedit", "/enum", "{current}"])
            .output()
            .ok()?;

        let info = String::from_utf8_lossy(&output.stdout).to_lowercase();
        if info.contains("winload.efi") {
            Some("UEFI".to_string())
        } else {
            Some("Legacy BIOS".to_string())
        }
    }
}

/// Detect if running in a virtual machine
fn detect_virtualization() -> Option<String> {
    // Check system manufacturer via WMI
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_ComputerSystem).Manufacturer + '|' + (Get-CimInstance Win32_ComputerSystem).Model",
        ])
        .output()
        .ok()?;

    let info = String::from_utf8_lossy(&output.stdout).to_lowercase();

    if info.contains("vmware") {
        return Some("VMware".to_string());
    }
    if info.contains("virtualbox") || info.contains("vbox") {
        return Some("VirtualBox".to_string());
    }
    if info.contains("microsoft") && info.contains("virtual") {
        return Some("Hyper-V".to_string());
    }
    if info.contains("qemu") {
        return Some("QEMU".to_string());
    }
    if info.contains("xen") {
        return Some("Xen".to_string());
    }
    if info.contains("parallels") {
        return Some("Parallels".to_string());
    }

    // Check for Hyper-V specifically
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_ComputerSystem).HypervisorPresent",
        ])
        .output()
        .ok()?;

    let hypervisor = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
    if hypervisor == "true" {
        return Some("Hypervisor Present".to_string());
    }

    None
}

/// Get GPU names
fn get_gpus() -> Vec<String> {
    let mut gpus = Vec::new();

    if let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_VideoController).Name -join \"`n\"",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let gpu = line.trim();
            if !gpu.is_empty() {
                gpus.push(gpu.to_string());
            }
        }
    }

    gpus
}

/// Get system architecture
fn get_architecture() -> Option<String> {
    // Use Rust's built-in arch detection
    Some(std::env::consts::ARCH.to_string())
}

/// Get terminal emulator name
fn get_terminal() -> Option<String> {
    // Check for Windows Terminal first
    if env::var("WT_SESSION").is_ok() {
        return Some("Windows Terminal".to_string());
    }

    // Check for VS Code terminal
    if env::var("TERM_PROGRAM").ok().as_deref() == Some("vscode") {
        return Some("VS Code".to_string());
    }

    // Check for common terminals via parent process
    if let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-Process -Id $PID).Parent.ProcessName",
        ])
        .output()
    {
        let parent = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
        match parent.as_str() {
            "windowsterminal" => return Some("Windows Terminal".to_string()),
            "code" => return Some("VS Code".to_string()),
            "conhost" => return Some("Console Host".to_string()),
            "cmd" => return Some("Command Prompt".to_string()),
            "powershell" | "pwsh" => return Some("PowerShell".to_string()),
            _ => {
                if !parent.is_empty() {
                    return Some(parent);
                }
            }
        }
    }

    Some("Console".to_string())
}

/// Get shell name and version
fn get_shell() -> Option<String> {
    // Check SHELL env var first (for Git Bash, etc.)
    if let Ok(shell) = env::var("SHELL") {
        if shell.contains("bash") {
            // Try to get bash version
            if let Ok(output) = Command::new("bash").args(["--version"]).output() {
                let version = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = version.lines().next() {
                    // Parse "GNU bash, version 5.2.15(1)-release ..." to "bash 5.2.15"
                    if let Some(ver_start) = line.find("version ") {
                        let ver_part = &line[ver_start + 8..];
                        if let Some(ver_end) = ver_part.find(|c: char| !c.is_ascii_digit() && c != '.') {
                            return Some(format!("bash {}", &ver_part[..ver_end]));
                        }
                    }
                }
            }
            return Some("bash".to_string());
        }
    }

    // Check for PowerShell version
    if let Ok(output) = Command::new("powershell")
        .args(["-NoProfile", "-Command", "$PSVersionTable.PSVersion.ToString()"])
        .output()
    {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() {
            return Some(format!("PowerShell {}", version));
        }
    }

    Some("PowerShell".to_string())
}

/// Get display resolution
fn get_display_resolution() -> Option<String> {
    if let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Screen]::PrimaryScreen.Bounds | ForEach-Object { \"$($_.Width)x$($_.Height)\" }",
        ])
        .output()
    {
        let resolution = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !resolution.is_empty() && resolution.contains('x') {
            return Some(resolution);
        }
    }

    None
}

/// Get battery status
fn get_battery() -> Option<String> {
    if let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "$b = Get-CimInstance Win32_Battery; if ($b) { \"$($b.EstimatedChargeRemaining)% ($($b.BatteryStatus))\" }",
        ])
        .output()
    {
        let battery = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !battery.is_empty() && battery.contains('%') {
            // Translate BatteryStatus codes to human-readable
            let battery = battery
                .replace("(1)", "(Discharging)")
                .replace("(2)", "(AC Power)")
                .replace("(3)", "(Charging)")
                .replace("(4)", "(Low)")
                .replace("(5)", "(Critical)")
                .replace("(6)", "(Charging)")
                .replace("(7)", "(Charging High)")
                .replace("(8)", "(Charging Low)")
                .replace("(9)", "(Charging Critical)");
            return Some(battery);
        }
    }

    None
}

/// Get system locale
fn get_locale() -> Option<String> {
    if let Ok(output) = Command::new("powershell")
        .args(["-NoProfile", "-Command", "(Get-Culture).Name"])
        .output()
    {
        let locale = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !locale.is_empty() {
            return Some(locale);
        }
    }

    None
}
