//! Linux-specific information collectors

use super::{CollectMode, PlatformInfo};
use crate::collectors::command::{run_stdout, run_stdout_no_args, CommandTimeout};
use std::env;
use std::fs;
use std::path::Path;

/// Collect Linux-specific information
/// Linux is already fast (reads /proc, env vars) — minimal skips in fast mode.
pub fn collect(mode: CollectMode) -> PlatformInfo {
    let hardware_details = get_hardware_details(mode, crate::is_elevated());

    PlatformInfo {
        os_build: None,
        desktop_environment: detect_desktop_environment(),
        display_server: detect_display_server(),
        boot_mode: if mode == CollectMode::Fast {
            None
        } else {
            detect_boot_mode()
        },
        virtualization: detect_virtualization(mode),
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
        motherboard: hardware_details.motherboard,
        bios: hardware_details.bios,
        ram_slots: hardware_details.ram_slots,
        locale: get_locale(), // Fast: reads env var
        encryption: if mode == CollectMode::Full {
            get_root_encryption()
        } else {
            None
        },
        elevation_unlocks_more: hardware_details.elevation_unlocks_more,
    }
}

/// Detect the desktop environment
fn detect_desktop_environment() -> Option<String> {
    // Check XDG_CURRENT_DESKTOP first
    if let Ok(de) = env::var("XDG_CURRENT_DESKTOP") {
        if !de.trim().is_empty() {
            return Some(de.trim().to_string());
        }
    }

    // Check DESKTOP_SESSION
    if let Ok(session) = env::var("DESKTOP_SESSION") {
        if !session.trim().is_empty() {
            return Some(session.trim().to_string());
        }
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
        let session_type = session_type.to_ascii_lowercase();
        if matches!(session_type.as_str(), "x11" | "wayland") {
            return Some(session_type);
        }
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
    } else if sysinfo::System::cpu_arch()
        .as_deref()
        .is_some_and(|arch| matches!(arch, "x86" | "x86_64"))
    {
        Some("Legacy BIOS".to_string())
    } else {
        None
    }
}

/// Detect if running in a virtual machine
fn detect_virtualization(mode: CollectMode) -> Option<String> {
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

    if mode == CollectMode::Full {
        if let Some(value) = run_stdout(
            "systemd-detect-virt",
            std::iter::empty::<&str>(),
            CommandTimeout::Normal,
        ) {
            let value = value.trim();
            if !value.is_empty() && value != "none" {
                return Some(value.to_string());
            }
        }
    }

    None
}

/// Extract a GPU name from one `lspci` line.
///
/// Matches on the PCI **class** field (left of the first `": "`) — e.g.
/// `"VGA compatible controller"`, `"3D controller"`, `"Display controller"` —
/// rather than the whole line, so a non-GPU device whose *name* merely contains
/// "display" (e.g. an HDMI "Display Audio" controller) isn't misdetected as a
/// GPU. Splitting on the *first* `": "` is correct for default `lspci` output:
/// the bus address (`00:02.0`) contains a colon but never a colon-space, so the
/// first `": "` is always the class/device separator. Pure + testable.
fn parse_lspci_gpu_line(line: &str) -> Option<String> {
    let (class_part, device) = line.split_once(": ")?;
    let class = class_part.to_lowercase();
    if class.contains("vga")
        || class.contains("3d controller")
        || class.contains("display controller")
    {
        let name = device.trim();
        if !name.is_empty() {
            return Some(name.to_string());
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
            if let Some(name) = parse_lspci_gpu_line(line) {
                gpus.push(name);
            }
        }
    }

    // Raspberry Pi 4 GPUs are platform devices, not PCI display controllers.
    // Device-tree compatibility is a stable hardware identity and avoids
    // double-counting VC4 display and V3D render nodes.
    if gpus.is_empty() {
        if let Ok(compatible) = fs::read("/sys/firmware/devicetree/base/compatible") {
            if let Some(gpu) = raspberry_pi_gpu_from_compatible(&compatible) {
                gpus.push(gpu);
            }
        }
    }

    // Fallback: check /sys/class/drm and use the bound driver when available.
    if !gpus.is_empty() {
        return gpus;
    } else if let Ok(entries) = fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("card") && !name.contains('-') {
                let driver = fs::read_link(entry.path().join("device/driver"))
                    .ok()
                    .and_then(|path| {
                        path.file_name()
                            .map(|name| name.to_string_lossy().to_string())
                    });
                let label = match driver.as_deref() {
                    Some("v3d") | Some("vc4") => "Broadcom VideoCore (V3D)".to_string(),
                    Some(driver) => format!("{} ({})", driver, name),
                    None if entry.path().join("device/vendor").exists() => format!("GPU {}", name),
                    None => continue,
                };
                if !gpus.contains(&label) {
                    gpus.push(label);
                }
            }
        }
    }

    gpus
}

fn raspberry_pi_gpu_from_compatible(compatible: &[u8]) -> Option<String> {
    let compatible = String::from_utf8_lossy(compatible);
    if compatible.contains("brcm,bcm2712") {
        Some("Broadcom VideoCore VII (V3D)".to_string())
    } else if compatible.contains("brcm,bcm2711") {
        Some("Broadcom VideoCore VI (V3D)".to_string())
    } else if compatible.contains("brcm,bcm2837") {
        Some("Broadcom VideoCore IV".to_string())
    } else {
        None
    }
}

/// Get system architecture
fn get_architecture() -> Option<String> {
    sysinfo::System::cpu_arch().or_else(|| {
        run_stdout("uname", ["-m"], CommandTimeout::Normal)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn get_machine_model() -> Option<String> {
    for path in [
        "/sys/firmware/devicetree/base/model",
        "/sys/class/dmi/id/product_name",
    ] {
        if let Ok(model) = fs::read_to_string(path) {
            let model = model.trim_matches(char::from(0)).trim();
            if !model.is_empty() && !is_placeholder(model) {
                return Some(model.to_string());
            }
        }
    }
    None
}

fn is_placeholder(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.is_empty()
        || value.contains("to be filled")
        || value == "default string"
        || value == "system product name"
        || value == "system manufacturer"
        || value == "not specified"
        || value == "none"
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
        if !term.trim().is_empty() {
            return Some(term.trim().to_string());
        }
    }

    // Check common terminal env vars
    if let Ok(term) = env::var("TERMINAL") {
        if !term.trim().is_empty() {
            return Some(term.trim().to_string());
        }
    }

    if let Some(parent) = detect_terminal_from_ps(std::process::id()) {
        return Some(parent);
    }

    None
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
            if value.trim().is_empty() {
                continue;
            }
            return Some(value.trim().to_string());
        }
    }

    // Try locale command
    if let Some(stdout) = run_stdout_no_args("locale", CommandTimeout::Normal) {
        for line in stdout.lines() {
            if line.starts_with("LANG=") {
                let lang = line
                    .strip_prefix("LANG=")
                    .unwrap_or("")
                    .trim_matches(['\"', '\'']);
                if !lang.is_empty() {
                    return Some(lang.to_string());
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
        _ => None,
    }
}

fn battery_summary_from_path(base: &Path) -> Option<String> {
    let capacity = read_u64_from_file(base.join("capacity"))?;
    if capacity > 100 {
        return None;
    }
    let status = fs::read_to_string(base.join("status"))
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let mut summary = format!("{}% ({})", capacity, status);

    // Health must be computed from a single unit family: energy_* is in µWh,
    // charge_* is in µAh. Mixing energy_full with charge_full_design (which can
    // happen if only one of each is exposed) yields a meaningless ratio, so
    // require both numerator and denominator from the same family.
    let health_pair = match (
        read_u64_from_file(base.join("energy_full")),
        read_u64_from_file(base.join("energy_full_design")),
    ) {
        (Some(full), Some(design)) => Some((full, design)),
        _ => match (
            read_u64_from_file(base.join("charge_full")),
            read_u64_from_file(base.join("charge_full_design")),
        ) {
            (Some(full), Some(design)) => Some((full, design)),
            _ => None,
        },
    };
    if let Some((full, design)) = health_pair {
        if let Some(health) = battery_health_percent(full, design) {
            summary.push_str(&format!("; health {}%", health));
        }
    }

    Some(summary)
}

/// Battery health percentage from a same-unit-family `(full, design)` pair.
/// Returns `None` for implausible readings (zero design, or a full charge more
/// than twice the design capacity — a sign of mismatched units or bad data).
/// Pure + testable.
fn battery_health_percent(full: u64, design: u64) -> Option<u8> {
    if design == 0 || full > design.saturating_mul(2) {
        return None;
    }
    Some(
        (full as f64 / design as f64 * 100.0)
            .clamp(0.0, 100.0)
            .round() as u8,
    )
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
        // Unknown/unparsed state ranks just above ONLINE so it surfaces as
        // "not clearly healthy" without masquerading as a specific DEGRADED.
        "DEGRADED" => 3,
        "FAULTED" | "OFFLINE" | "UNAVAIL" | "REMOVED" => 4,
        // A SUSPENDED pool has stopped servicing I/O — most severe.
        "SUSPENDED" => 5,
        _ => 2,
    }
}

#[derive(Default)]
struct LinuxHardwareDetails {
    motherboard: Option<String>,
    bios: Option<String>,
    ram_slots: Option<String>,
    elevation_unlocks_more: bool,
}

fn get_hardware_details(mode: CollectMode, elevated: bool) -> LinuxHardwareDetails {
    let mut details = LinuxHardwareDetails {
        motherboard: sysfs_summary(&[
            "/sys/class/dmi/id/board_vendor",
            "/sys/class/dmi/id/board_name",
            "/sys/class/dmi/id/board_version",
        ]),
        bios: sysfs_summary(&[
            "/sys/class/dmi/id/bios_vendor",
            "/sys/class/dmi/id/bios_version",
            "/sys/class/dmi/id/bios_date",
        ]),
        ..LinuxHardwareDetails::default()
    };

    if mode == CollectMode::Full && elevated {
        let elevated_details = get_dmidecode_details();
        details.motherboard = details.motherboard.or(elevated_details.motherboard);
        details.bios = details.bios.or(elevated_details.bios);
        details.ram_slots = elevated_details.ram_slots;
    }

    details.elevation_unlocks_more = mode == CollectMode::Full
        && !elevated
        && Path::new("/sys/class/dmi/id").exists()
        && command_exists("dmidecode");
    details
}

fn get_dmidecode_details() -> LinuxHardwareDetails {
    LinuxHardwareDetails {
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
        elevation_unlocks_more: false,
    }
}

fn sysfs_summary(paths: &[&str]) -> Option<String> {
    let mut values = Vec::new();
    for path in paths {
        if let Ok(value) = fs::read_to_string(path) {
            let value = value.trim();
            if !is_placeholder(value)
                && !values
                    .iter()
                    .any(|existing: &String| existing.eq_ignore_ascii_case(value))
            {
                values.push(value.to_string());
            }
        }
    }
    (!values.is_empty()).then(|| values.join(" "))
}

fn command_exists(command: &str) -> bool {
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| directory.join(command).is_file())
    })
}

fn get_root_encryption() -> Option<String> {
    let root = run_stdout(
        "findmnt",
        ["-n", "-o", "SOURCE,FSTYPE", "/"],
        CommandTimeout::Normal,
    )?;
    let mut fields = root.split_whitespace();
    let source = fields.next()?;
    let filesystem = fields.next().unwrap_or_default();

    if filesystem.eq_ignore_ascii_case("zfs") {
        let encryption_root = run_stdout(
            "zfs",
            ["get", "-H", "-o", "value", "encryptionroot", source],
            CommandTimeout::Slow,
        )?;
        let value = encryption_root.trim();
        return (!value.is_empty() && value != "-")
            .then(|| "Root volume: ZFS native encryption".to_string());
    }

    if !source.starts_with("/dev/") {
        return None;
    }
    let stack = run_stdout(
        "lsblk",
        ["-s", "-n", "-o", "TYPE,FSTYPE", source],
        CommandTimeout::Normal,
    )?;
    stack
        .lines()
        .any(|line| {
            let lower = line.to_ascii_lowercase();
            lower.split_whitespace().any(|field| field == "crypt") || lower.contains("crypto_luks")
        })
        .then(|| "Root volume: LUKS/dm-crypt".to_string())
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
        let starts_new_device = line.trim() == "Memory Device";
        if starts_new_device && !current.is_empty() {
            let parsed = parse_memory_device(&current);
            if let Some(dimm) = parsed {
                dimms.push(dimm);
            }
            current.clear();
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
    let mut rated_speed = None;
    let mut configured_speed = None;
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
            "Speed" => rated_speed = Some(value.replace(' ', "")),
            "Configured Memory Speed" => configured_speed = Some(value.replace(' ', "")),
            "Manufacturer" => manufacturer = Some(value.to_string()),
            _ => {}
        }
    }
    let size = size?;
    let mut parts = vec![size];
    if let Some(mem_type) = mem_type {
        parts.push(mem_type);
    }
    if let Some(speed) = configured_speed.or(rated_speed) {
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
    fn zfs_rank_orders_suspended_worst_and_unknown_below_degraded() {
        // SUSPENDED (I/O stopped) outranks FAULTED.
        assert!(zfs_rank("SUSPENDED") > zfs_rank("FAULTED"));
        assert_eq!(
            aggregate_zfs_health(["ONLINE", "FAULTED", "SUSPENDED"]),
            Some("SUSPENDED".to_string())
        );
        // A real DEGRADED takes precedence over an unparsed/unknown state, so
        // unknown never masquerades as DEGRADED severity.
        assert!(zfs_rank("WEIRD_STATE") < zfs_rank("DEGRADED"));
        assert_eq!(
            aggregate_zfs_health(["WEIRD_STATE", "DEGRADED"]),
            Some("DEGRADED".to_string())
        );
    }

    #[test]
    fn lspci_gpu_line_matches_class_not_device_name() {
        assert_eq!(
            parse_lspci_gpu_line(
                "00:02.0 VGA compatible controller: Intel Corporation Iris Xe Graphics (rev 0c)"
            ),
            Some("Intel Corporation Iris Xe Graphics (rev 0c)".to_string())
        );
        assert_eq!(
            parse_lspci_gpu_line(
                "01:00.0 3D controller: NVIDIA Corporation GA107M [GeForce RTX 3050]"
            ),
            Some("NVIDIA Corporation GA107M [GeForce RTX 3050]".to_string())
        );
        // An audio controller whose *name* contains "Display" is NOT a GPU.
        assert_eq!(
            parse_lspci_gpu_line("00:1f.3 Audio device: Intel Corporation Display Audio"),
            None
        );
        assert_eq!(
            parse_lspci_gpu_line("00:00.0 Host bridge: Intel Corporation Device 4601"),
            None
        );
    }

    #[test]
    fn raspberry_pi_device_tree_maps_supported_video_core_generations() {
        assert_eq!(
            raspberry_pi_gpu_from_compatible(b"raspberrypi,4-model-b\0brcm,bcm2711\0"),
            Some("Broadcom VideoCore VI (V3D)".to_string())
        );
        assert_eq!(
            raspberry_pi_gpu_from_compatible(b"raspberrypi,5-model-b\0brcm,bcm2712\0"),
            Some("Broadcom VideoCore VII (V3D)".to_string())
        );
        assert_eq!(raspberry_pi_gpu_from_compatible(b"vendor,board\0"), None);
    }

    #[test]
    fn machine_model_placeholders_are_rejected() {
        assert!(is_placeholder("To Be Filled By O.E.M."));
        assert!(is_placeholder("System Product Name"));
        assert!(is_placeholder(" default string "));
        assert!(!is_placeholder("Alienware m18 R2"));
        assert!(!is_placeholder("Raspberry Pi 4 Model B Rev 1.5"));
    }

    #[test]
    fn battery_health_percent_rejects_implausible_values() {
        assert_eq!(battery_health_percent(4800, 6000), Some(80));
        assert_eq!(battery_health_percent(6000, 6000), Some(100));
        // Zero design capacity → no health.
        assert_eq!(battery_health_percent(5000, 0), None);
        // Full more than 2x design → mismatched units / bad data → no health.
        assert_eq!(battery_health_percent(20000, 6000), None);
    }

    #[test]
    fn battery_summary_uses_one_capacity_unit_family() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("capacity"), "73\n").unwrap();
        fs::write(dir.path().join("status"), "Discharging\n").unwrap();
        fs::write(dir.path().join("energy_full"), "4800\n").unwrap();
        fs::write(dir.path().join("energy_full_design"), "6000\n").unwrap();
        // Deliberately incompatible charge-family data must not be mixed with
        // the complete energy pair above.
        fs::write(dir.path().join("charge_full"), "100\n").unwrap();
        fs::write(dir.path().join("charge_full_design"), "1000\n").unwrap();
        assert_eq!(
            battery_summary_from_path(dir.path()),
            Some("73% (Discharging); health 80%".to_string())
        );
    }

    #[test]
    fn battery_summary_rejects_invalid_capacity() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("capacity"), "255\n").unwrap();
        fs::write(dir.path().join("status"), "Unknown\n").unwrap();
        assert_eq!(battery_summary_from_path(dir.path()), None);
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

    #[test]
    fn dmidecode_prefers_configured_memory_speed() {
        let output = r#"
Memory Device
	Size: 32 GB
	Type: DDR5
	Speed: 5600 MT/s
	Configured Memory Speed: 5200 MT/s
	Manufacturer: Micron
"#;
        assert_eq!(
            parse_dmidecode_memory_summary(output),
            Some("1x32GB DDR5 5200MT/s Micron".to_string())
        );
    }
}
