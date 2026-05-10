//! Linux-specific information collectors

use super::{CollectMode, PlatformInfo};
use crate::collectors::command::{run_stdout, run_stdout_no_args, CommandTimeout};
use std::env;
use std::fs;
use std::path::Path;

/// Collect Linux-specific information
/// Linux is already fast (reads /proc, env vars) — minimal skips in fast mode.
pub fn collect(mode: CollectMode) -> PlatformInfo {
    let elevated_details = if mode == CollectMode::Full && crate::is_elevated() {
        get_dmidecode_details()
    } else {
        LinuxElevatedDetails::default()
    };

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
        gpus: get_gpus(), // lspci is fast (~10-20ms), /sys/class/drm fallback is instant
        architecture: get_architecture(),
        machine_model: get_machine_model(),
        cpu_core_topology: None,
        terminal: get_terminal(),
        shell: get_shell(),
        display_resolution: if mode == CollectMode::Fast {
            None
        } else {
            get_display_resolution()
        }, // xrandr subprocess
        battery: get_battery(), // Fast: reads /sys
        zfs_health: if mode == CollectMode::Fast {
            None
        } else {
            get_zfs_health()
        },
        motherboard: elevated_details.motherboard,
        bios: elevated_details.bios,
        ram_slots: elevated_details.ram_slots,
        locale: get_locale(), // Fast: reads env var
        encryption: None,     // LUKS detection deferred to a future PR
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
    if let Ok(osrelease) = fs::read_to_string("/proc/sys/kernel/osrelease") {
        let lower = osrelease.to_lowercase();
        if lower.contains("microsoft-standard-wsl2") {
            return Some("WSL2".to_string());
        }
        if lower.contains("microsoft") {
            return Some("WSL".to_string());
        }
    }

    if Path::new("/.dockerenv").exists() {
        return Some("Docker".to_string());
    }
    if Path::new("/run/.containerenv").exists() || Path::new("/.containerenv").exists() {
        return Some("Podman".to_string());
    }
    if let Ok(container) = env::var("container") {
        if !container.is_empty() {
            return Some(container);
        }
    }
    if let Ok(cgroup) = fs::read_to_string("/proc/1/cgroup") {
        let lower = cgroup.to_lowercase();
        for (needle, label) in [
            ("docker", "Docker"),
            ("libpod", "Podman"),
            ("kubepods", "Kubernetes"),
            ("lxc", "LXC"),
            ("machine.slice", "systemd-nspawn"),
        ] {
            if lower.contains(needle) {
                return Some(label.to_string());
            }
        }
    }

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
        if product.contains("amazon") {
            return Some("Amazon EC2".to_string());
        }
        if product.contains("google") {
            return Some("Google Compute Engine".to_string());
        }
    }

    if let Ok(vendor) = fs::read_to_string("/sys/class/dmi/id/sys_vendor") {
        let vendor = vendor.trim().to_lowercase();
        if vendor.contains("amazon") {
            return Some("Amazon EC2".to_string());
        }
        if vendor.contains("microsoft") {
            return Some("Hyper-V/Azure".to_string());
        }
        if vendor.contains("google") {
            return Some("Google Compute Engine".to_string());
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
    if let Some(stdout) = run_stdout_no_args("lspci", CommandTimeout::Normal) {
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
    if !gpus.is_empty() {
        return gpus;
    } else if let Ok(entries) = fs::read_dir("/sys/class/drm") {
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

    gpus
}

/// Get system architecture
fn get_architecture() -> Option<String> {
    Some(std::env::consts::ARCH.to_string())
}

fn get_machine_model() -> Option<String> {
    for path in [
        "/sys/firmware/devicetree/base/model",
        "/sys/class/dmi/id/product_name",
    ] {
        if let Ok(model) = fs::read_to_string(path) {
            let model = model.trim_matches(char::from(0)).trim();
            if !model.is_empty() {
                return Some(model.to_string());
            }
        }
    }
    None
}

/// Get terminal emulator name
fn get_terminal() -> Option<String> {
    for (var, label) in [
        ("KITTY_WINDOW_ID", "kitty"),
        ("WEZTERM_PANE", "WezTerm"),
        ("GHOSTTY_RESOURCES_DIR", "Ghostty"),
        ("ALACRITTY_LOG", "Alacritty"),
        ("KONSOLE_VERSION", "Konsole"),
        ("FOOT_PID", "foot"),
        ("TILIX_ID", "Tilix"),
        ("WT_SESSION", "Windows Terminal"),
    ] {
        if env::var(var).is_ok() {
            return Some(label.to_string());
        }
    }

    // Check TERM_PROGRAM first
    if let Ok(term) = env::var("TERM_PROGRAM") {
        return Some(term);
    }

    // Check common terminal env vars
    if let Ok(term) = env::var("TERMINAL") {
        return Some(term);
    }

    if let Some(parent) = detect_terminal_from_ps(std::process::id()) {
        return Some(parent);
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
    if let Some(stdout) = run_stdout("xrandr", ["--current"], CommandTimeout::Slow) {
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
    if let Some(stdout) = run_stdout_no_args("wlr-randr", CommandTimeout::Slow) {
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
    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    for entry in entries.flatten() {
        let base = entry.path();
        let Ok(supply_type) = fs::read_to_string(base.join("type")) else {
            continue;
        };
        if !supply_type.trim().eq_ignore_ascii_case("Battery") {
            continue;
        }
        if let Some(summary) = battery_summary_from_path(&base) {
            return Some(summary);
        }
    }
    None
}

/// Get system locale
fn get_locale() -> Option<String> {
    for var in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Ok(value) = env::var(var) {
            if value.is_empty() {
                continue;
            }
            let clean = value.split('.').next().unwrap_or(&value);
            return Some(clean.to_string());
        }
    }

    // Try locale command
    if let Some(stdout) = run_stdout_no_args("locale", CommandTimeout::Normal) {
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

fn detect_terminal_from_ps(current_pid: u32) -> Option<String> {
    let stdout = run_stdout(
        "ps",
        ["-e", "-o", "pid=,ppid=,comm="],
        CommandTimeout::Normal,
    )?;
    let mut rows = Vec::new();
    for line in stdout.lines() {
        let mut parts = line.split_whitespace();
        let Some(pid) = parts.next().and_then(|p| p.parse::<u32>().ok()) else {
            continue;
        };
        let Some(ppid) = parts.next().and_then(|p| p.parse::<u32>().ok()) else {
            continue;
        };
        let comm = parts.collect::<Vec<_>>().join(" ");
        rows.push((pid, ppid, comm));
    }

    let mut pid = current_pid;
    for _ in 0..8 {
        let Some((_, parent_pid, _)) = rows.iter().find(|(row_pid, _, _)| *row_pid == pid) else {
            break;
        };
        let Some((_, _, parent_name)) = rows.iter().find(|(row_pid, _, _)| row_pid == parent_pid)
        else {
            break;
        };
        if let Some(label) = terminal_label_from_process(parent_name) {
            return Some(label);
        }
        if *parent_pid == 0 || *parent_pid == pid {
            break;
        }
        pid = *parent_pid;
    }
    None
}

fn terminal_label_from_process(process: &str) -> Option<String> {
    let base = Path::new(process)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(process);
    match base {
        "gnome-terminal" | "gnome-terminal-" => Some("GNOME Terminal".to_string()),
        "konsole" => Some("Konsole".to_string()),
        "xterm" => Some("xterm".to_string()),
        "alacritty" => Some("Alacritty".to_string()),
        "kitty" => Some("kitty".to_string()),
        "tilix" => Some("Tilix".to_string()),
        "terminator" => Some("Terminator".to_string()),
        "wezterm-gui" | "wezterm" => Some("WezTerm".to_string()),
        "ghostty" => Some("Ghostty".to_string()),
        "foot" => Some("foot".to_string()),
        "bash" | "zsh" | "fish" | "sh" | "nu" | "tr300" | "cargo" => None,
        other if !other.is_empty() => Some(other.to_string()),
        _ => None,
    }
}

fn battery_summary_from_path(base: &Path) -> Option<String> {
    let capacity = fs::read_to_string(base.join("capacity"))
        .ok()?
        .trim()
        .to_string();
    let status = fs::read_to_string(base.join("status"))
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let mut summary = format!("{}% ({})", capacity, status);

    if let (Some(full), Some(design)) = (
        read_u64_from_file(base.join("energy_full"))
            .or_else(|| read_u64_from_file(base.join("charge_full"))),
        read_u64_from_file(base.join("energy_full_design"))
            .or_else(|| read_u64_from_file(base.join("charge_full_design"))),
    ) {
        if design > 0 && full <= design.saturating_mul(2) {
            let health = (full as f64 / design as f64 * 100.0).clamp(0.0, 100.0);
            summary.push_str(&format!("; health {:.0}%", health));
        }
    }

    Some(summary)
}

fn read_u64_from_file(path: impl AsRef<Path>) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn get_zfs_health() -> Option<String> {
    let stdout = run_stdout(
        "zpool",
        ["list", "-H", "-o", "health"],
        CommandTimeout::Slow,
    )?;
    aggregate_zfs_health(stdout.lines().map(str::trim).filter(|s| !s.is_empty()))
}

fn aggregate_zfs_health<I, S>(states: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut worst: Option<String> = None;
    for state in states {
        let state = state.as_ref().trim();
        if state.is_empty() {
            continue;
        }
        let selected = match (worst.as_deref(), state) {
            (None, s) => s,
            (Some(current), s) if zfs_rank(s) > zfs_rank(current) => s,
            (Some(current), _) => current,
        };
        worst = Some(selected.to_string());
    }
    worst
}

fn zfs_rank(state: &str) -> u8 {
    match state {
        "ONLINE" => 1,
        "DEGRADED" => 2,
        "FAULTED" | "OFFLINE" | "UNAVAIL" | "REMOVED" => 3,
        _ => 2,
    }
}

#[derive(Default)]
struct LinuxElevatedDetails {
    motherboard: Option<String>,
    bios: Option<String>,
    ram_slots: Option<String>,
}

fn get_dmidecode_details() -> LinuxElevatedDetails {
    LinuxElevatedDetails {
        motherboard: get_dmidecode_summary(&[
            ("baseboard-manufacturer", ""),
            ("baseboard-product-name", " "),
        ]),
        bios: get_dmidecode_summary(&[
            ("bios-vendor", ""),
            ("bios-version", " "),
            ("bios-release-date", " "),
        ]),
        ram_slots: get_dmidecode_memory_summary(),
    }
}

fn get_dmidecode_summary(fields: &[(&str, &str)]) -> Option<String> {
    let mut value = String::new();
    for (field, separator) in fields {
        let stdout = run_stdout("dmidecode", ["-s", *field], CommandTimeout::Slow)?;
        let part = stdout
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && *line != "Not Specified")?;
        if !value.is_empty() {
            value.push_str(separator);
        }
        value.push_str(part);
    }
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn get_dmidecode_memory_summary() -> Option<String> {
    let stdout = run_stdout("dmidecode", ["-t", "memory"], CommandTimeout::Slow)?;
    parse_dmidecode_memory_summary(&stdout)
}

fn parse_dmidecode_memory_summary(output: &str) -> Option<String> {
    let mut dimms = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in output.lines() {
        if line.trim() == "Memory Device" {
            if !current.is_empty() {
                let parsed = parse_memory_device(&current);
                if let Some(dimm) = parsed {
                    dimms.push(dimm);
                }
                current.clear();
            }
        }
        current.push(line);
    }
    if !current.is_empty() {
        let parsed = parse_memory_device(&current);
        if let Some(dimm) = parsed {
            dimms.push(dimm);
        }
    }

    if dimms.is_empty() {
        return None;
    }

    let first = &dimms[0];
    let same = dimms.iter().all(|d| d == first);
    if same {
        Some(format!("{}x{}", dimms.len(), first))
    } else {
        Some(dimms.join(", "))
    }
}

fn parse_memory_device(lines: &[&str]) -> Option<String> {
    let mut size = None;
    let mut mem_type = None;
    let mut speed = None;
    let mut manufacturer = None;
    for line in lines {
        let trimmed = line.trim();
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() || value == "No Module Installed" || value == "Not Specified" {
            continue;
        }
        match key {
            "Size" => size = Some(value.replace(' ', "")),
            "Type" => mem_type = Some(value.to_string()),
            "Speed" | "Configured Memory Speed" if speed.is_none() => {
                speed = Some(value.replace(' ', ""))
            }
            "Manufacturer" => manufacturer = Some(value.to_string()),
            _ => {}
        }
    }
    let size = size?;
    let mut parts = vec![size];
    if let Some(mem_type) = mem_type {
        parts.push(mem_type);
    }
    if let Some(speed) = speed {
        parts.push(speed);
    }
    if let Some(manufacturer) = manufacturer {
        parts.push(manufacturer);
    }
    Some(parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zfs_health_reports_worst_pool_state() {
        assert_eq!(
            aggregate_zfs_health(["ONLINE", "DEGRADED", "ONLINE"]),
            Some("DEGRADED".to_string())
        );
    }

    #[test]
    fn parses_dmidecode_memory_summary() {
        let output = r#"
Memory Device
	Size: 16 GB
	Type: DDR5
	Speed: 5600 MT/s
	Manufacturer: SK Hynix
Memory Device
	Size: 16 GB
	Type: DDR5
	Speed: 5600 MT/s
	Manufacturer: SK Hynix
"#;
        assert_eq!(
            parse_dmidecode_memory_summary(output),
            Some("2x16GB DDR5 5600MT/s SK Hynix".to_string())
        );
    }
}
