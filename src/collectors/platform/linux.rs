//! Linux-specific information collectors

use super::{CollectMode, PlatformInfo};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Collect Linux-specific information
/// Linux is already fast (reads /proc, env vars) — minimal skips in fast mode.
pub fn collect(mode: CollectMode) -> PlatformInfo {
    PlatformInfo {
        desktop_environment: detect_desktop_environment(),
        display_server: detect_display_server(),
        boot_mode: if mode == CollectMode::Fast {
            None
        } else {
            detect_boot_mode()
        },
        virtualization: detect_virtualization(), // Fast: reads /proc
        windows_edition: None,
        macos_codename: None,
        gpus: if mode == CollectMode::Fast {
            Vec::new()
        } else {
            get_gpus()
        }, // lspci is a subprocess
        architecture: get_architecture(),
        terminal: get_terminal(),
        shell: get_shell(),
        display_resolution: if mode == CollectMode::Fast {
            None
        } else {
            get_display_resolution()
        }, // xrandr subprocess
        battery: get_battery(), // Fast: reads /sys
        locale: get_locale(),   // Fast: reads env var
    }
}

/// Detect the desktop environment
fn detect_desktop_environment() -> Option<String> {
    // Check XDG_CURRENT_DESKTOP first
    if let Ok(de) = env::var("XDG_CURRENT_DESKTOP") {
        return Some(de);
    }

    // Check DESKTOP_SESSION
    if let Ok(session) = env::var("DESKTOP_SESSION") {
        return Some(session);
    }

    // Check for specific environment variables
    if env::var("GNOME_DESKTOP_SESSION_ID").is_ok() {
        return Some("GNOME".to_string());
    }

    if env::var("KDE_FULL_SESSION").is_ok() {
        return Some("KDE".to_string());
    }

    None
}

/// Detect the display server (X11 or Wayland)
fn detect_display_server() -> Option<String> {
    if let Ok(session_type) = env::var("XDG_SESSION_TYPE") {
        return Some(session_type);
    }

    if env::var("WAYLAND_DISPLAY").is_ok() {
        return Some("wayland".to_string());
    }

    if env::var("DISPLAY").is_ok() {
        return Some("x11".to_string());
    }

    None
}

/// Detect boot mode (UEFI or Legacy BIOS)
fn detect_boot_mode() -> Option<String> {
    if Path::new("/sys/firmware/efi").exists() {
        Some("UEFI".to_string())
    } else {
        Some("Legacy BIOS".to_string())
    }
}

/// Detect if running in a virtual machine
fn detect_virtualization() -> Option<String> {
    // Check /sys/class/dmi/id/product_name
    if let Ok(product) = fs::read_to_string("/sys/class/dmi/id/product_name") {
        let product = product.trim().to_lowercase();
        if product.contains("virtualbox") {
            return Some("VirtualBox".to_string());
        }
        if product.contains("vmware") {
            return Some("VMware".to_string());
        }
        if product.contains("qemu") || product.contains("kvm") {
            return Some("QEMU/KVM".to_string());
        }
        if product.contains("hyper-v") {
            return Some("Hyper-V".to_string());
        }
    }

    // Check /proc/cpuinfo for hypervisor flag
    if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
        if cpuinfo.contains("hypervisor") {
            return Some("Virtual Machine".to_string());
        }
    }

    None
}

/// Get GPU names
fn get_gpus() -> Vec<String> {
    let mut gpus = Vec::new();

    // Try lspci for VGA/3D controllers
    if let Ok(output) = Command::new("lspci").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let lower = line.to_lowercase();
            if lower.contains("vga") || lower.contains("3d controller") || lower.contains("display")
            {
                // Format: "00:02.0 VGA compatible controller: Intel Corporation ..."
                if let Some(pos) = line.find(": ") {
                    let gpu_name = line[pos + 2..].trim();
                    if !gpu_name.is_empty() {
                        gpus.push(gpu_name.to_string());
                    }
                }
            }
        }
    }

    // Fallback: check /sys/class/drm
    if gpus.is_empty() {
        if let Ok(entries) = fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("card") && !name.contains('-') {
                    let device_path = entry.path().join("device/vendor");
                    if device_path.exists() {
                        gpus.push(format!("GPU {}", name));
                    }
                }
            }
        }
    }

    gpus
}

/// Get system architecture
fn get_architecture() -> Option<String> {
    Some(std::env::consts::ARCH.to_string())
}

/// Get terminal emulator name
fn get_terminal() -> Option<String> {
    // Check TERM_PROGRAM first
    if let Ok(term) = env::var("TERM_PROGRAM") {
        return Some(term);
    }

    // Check common terminal env vars
    if let Ok(term) = env::var("TERMINAL") {
        return Some(term);
    }

    // Try to detect from parent process
    if let Ok(output) = Command::new("ps")
        .args(["-o", "comm=", "-p", &format!("{}", std::process::id())])
        .output()
    {
        // Get grandparent (terminal emulator)
        if let Ok(ppid_output) = Command::new("ps")
            .args(["-o", "ppid=", "-p", &format!("{}", std::process::id())])
            .output()
        {
            let ppid = String::from_utf8_lossy(&ppid_output.stdout)
                .trim()
                .to_string();
            if let Ok(parent_output) = Command::new("ps")
                .args(["-o", "comm=", "-p", &ppid])
                .output()
            {
                let parent = String::from_utf8_lossy(&parent_output.stdout)
                    .trim()
                    .to_string();
                match parent.as_str() {
                    "gnome-terminal" | "gnome-terminal-" => {
                        return Some("GNOME Terminal".to_string())
                    }
                    "konsole" => return Some("Konsole".to_string()),
                    "xterm" => return Some("xterm".to_string()),
                    "alacritty" => return Some("Alacritty".to_string()),
                    "kitty" => return Some("kitty".to_string()),
                    "tilix" => return Some("Tilix".to_string()),
                    "terminator" => return Some("Terminator".to_string()),
                    _ => {
                        if !parent.is_empty() {
                            return Some(parent);
                        }
                    }
                }
            }
        }
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
                    // Take just the version number part
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
    // Try xrandr for X11
    if let Ok(output) = Command::new("xrandr").args(["--current"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains(" connected") && line.contains('x') {
                // Find resolution pattern like "1920x1080+0+0"
                for word in line.split_whitespace() {
                    if word.contains('x')
                        && word
                            .chars()
                            .next()
                            .map(|c| c.is_ascii_digit())
                            .unwrap_or(false)
                    {
                        // Extract just the resolution part
                        let res: String = word
                            .chars()
                            .take_while(|c| c.is_ascii_digit() || *c == 'x')
                            .collect();
                        if res.contains('x') {
                            return Some(res);
                        }
                    }
                }
            }
        }
    }

    // Try wlr-randr for Wayland
    if let Ok(output) = Command::new("wlr-randr").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("current") {
                // Find resolution pattern
                for word in line.split_whitespace() {
                    if word.contains('x')
                        && word
                            .chars()
                            .next()
                            .map(|c| c.is_ascii_digit())
                            .unwrap_or(false)
                    {
                        return Some(word.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Get battery status
fn get_battery() -> Option<String> {
    let battery_path = Path::new("/sys/class/power_supply/BAT0");
    if !battery_path.exists() {
        // Try BAT1
        let alt_path = Path::new("/sys/class/power_supply/BAT1");
        if !alt_path.exists() {
            return None;
        }
    }

    let base = if battery_path.exists() {
        battery_path
    } else {
        Path::new("/sys/class/power_supply/BAT1")
    };

    let capacity = fs::read_to_string(base.join("capacity"))
        .ok()?
        .trim()
        .to_string();

    let status = fs::read_to_string(base.join("status"))
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    Some(format!("{}% ({})", capacity, status))
}

/// Get system locale
fn get_locale() -> Option<String> {
    // Check LANG environment variable
    if let Ok(lang) = env::var("LANG") {
        // Strip .UTF-8 or similar suffix for cleaner display
        let clean = lang.split('.').next().unwrap_or(&lang);
        return Some(clean.to_string());
    }

    // Try locale command
    if let Ok(output) = Command::new("locale").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with("LANG=") {
                let lang = line.strip_prefix("LANG=").unwrap_or("");
                let clean = lang.split('.').next().unwrap_or(lang);
                if !clean.is_empty() {
                    return Some(clean.to_string());
                }
            }
        }
    }

    None
}
