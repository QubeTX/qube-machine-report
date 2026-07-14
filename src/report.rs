//! Report generation for TR-300
//!
//! Generates the complete system report using a compact fixed-width layout.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::collectors::{CollectMode, SystemInfo};
use crate::config::{Config, OutputFormat, MAX_DATA_WIDTH};
use crate::render::bar::render_bar;
use crate::render::table::TableRenderer;

/// JSON output schema version. Bump on breaking renames or removals
/// (additive changes — new keys — do not require a bump).
pub const SCHEMA_VERSION: u32 = 1;

/// Generate the complete system report
pub fn generate(info: &SystemInfo, config: &Config) -> String {
    match config.format {
        OutputFormat::Table => generate_table(info, config),
        OutputFormat::Json => generate_json(info),
    }
}

/// Generate table format output
fn generate_table(info: &SystemInfo, config: &Config) -> String {
    let chars = config.box_chars();
    let (bar_filled, bar_empty) = config.bar_chars();

    // Use fixed column widths for stable terminal alignment.
    let label_width = 12; // "LAST LOGIN" is 10 chars, use 12 for padding
    let data_width = MAX_DATA_WIDTH;

    let renderer = TableRenderer::new(label_width, data_width, chars);

    let mut output = String::new();

    // Header section
    output.push_str(&renderer.render_top_header());
    output.push_str(&renderer.render_header_bottom());
    output.push_str(&renderer.render_centered(config.title()));
    output.push_str(&renderer.render_centered(config.subtitle()));
    output.push_str(&renderer.render_top_divider());

    // OS Section
    let os_display = format!("{} {}", info.os_name, info.os_version);
    output.push_str(&renderer.render_row("OS", &os_display));
    if let Some(ref edition) = info.os_edition {
        output.push_str(&renderer.render_row("EDITION", edition));
    }
    if let Some(ref codename) = info.os_codename {
        output.push_str(&renderer.render_row("CODENAME", codename));
    }
    if let Some(ref build) = info.os_build {
        output.push_str(&renderer.render_row("BUILD", build));
    }
    output.push_str(&renderer.render_row("KERNEL", &info.kernel));
    output.push_str(&renderer.render_row("ARCH", &info.architecture));
    if let Some(ref model) = info.machine_model {
        output.push_str(&renderer.render_row("MODEL", model));
    }
    if let Some(ref board) = info.motherboard {
        output.push_str(&renderer.render_row("BOARD", board));
    }
    if let Some(ref bios) = info.bios {
        output.push_str(&renderer.render_row("BIOS", bios));
    }
    if let Some(ref boot_mode) = info.boot_mode {
        output.push_str(&renderer.render_row("BOOT MODE", boot_mode));
    }
    if let Some(ref desktop) = info.desktop_environment {
        output.push_str(&renderer.render_row("DESKTOP", desktop));
    }
    if let Some(ref server) = info.display_server {
        output.push_str(&renderer.render_row("SESSION", server));
    }
    if let Some(ref resolution) = info.display_resolution {
        output.push_str(&renderer.render_row("DISPLAY", resolution));
    }
    output.push_str(&renderer.render_middle_divider());

    // Network Section
    output.push_str(&renderer.render_row("HOSTNAME", &info.hostname));
    if let Some(ref ip) = info.machine_ip {
        output.push_str(&renderer.render_row("DEFAULT IP", ip));
    }
    output.push_str(&renderer.render_row(
        "SSH CLIENT",
        info.client_ip.as_deref().unwrap_or("Not an SSH session"),
    ));

    // DNS servers (up to 5)
    for (i, dns) in info.dns_servers.iter().take(5).enumerate() {
        let label = format!("DNS  IP {}", i + 1);
        output.push_str(&renderer.render_row(&label, dns));
    }

    output.push_str(&renderer.render_row("USER", &info.username));
    output.push_str(&renderer.render_middle_divider());

    // CPU Section
    output.push_str(&renderer.render_row("PROCESSOR", &info.processor));
    output.push_str(&renderer.render_row("CORES", &info.cores_str()));
    if let Some(ref topology) = info.cpu_core_topology {
        output.push_str(&renderer.render_row("CORE TYPE", topology));
    }

    // GPU display: if ≤3 GPUs, show each on own row; if >3, show as compact list
    if !info.gpus.is_empty() {
        if info.gpus.len() <= 3 {
            for (i, gpu) in info.gpus.iter().enumerate() {
                let label = if info.gpus.len() == 1 {
                    "GPU".to_string()
                } else {
                    format!("GPU {}", i + 1)
                };
                output.push_str(&renderer.render_row(&label, gpu));
            }
        } else {
            // Compact comma-separated list for >3 GPUs
            let gpu_list = info.gpus.join(", ");
            output.push_str(&renderer.render_row("GPUs", &gpu_list));
        }
    }

    if let Some(ref hypervisor) = info.hypervisor {
        output.push_str(&renderer.render_row("HYPERVISOR", hypervisor));
    }
    if info.cpu_freq_ghz > 0.0 && info.cpu_freq_ghz.is_finite() {
        let label = if info.cpu_frequency_kind.as_deref() == Some("maximum") {
            "MAX FREQ"
        } else {
            "REPORTED FREQ"
        };
        output.push_str(&renderer.render_row(label, &info.freq_str()));
    }
    if let Some(usage) = info.cpu_usage_percent {
        output.push_str(&renderer.render_row(
            "CPU USAGE",
            &render_percent_bar(usage, data_width, bar_filled, bar_empty),
        ));
    }

    // Load averages as bar graphs (only shown when available)
    if let (Some(l1), Some(l5), Some(l15)) = (info.load_1m, info.load_5m, info.load_15m) {
        output.push_str(&renderer.render_row(
            "LOAD/CPU 1m",
            &render_percent_bar(l1, data_width, bar_filled, bar_empty),
        ));
        output.push_str(&renderer.render_row(
            "LOAD/CPU 5m",
            &render_percent_bar(l5, data_width, bar_filled, bar_empty),
        ));
        output.push_str(&renderer.render_row(
            "LOAD/CPU 15m",
            &render_percent_bar(l15, data_width, bar_filled, bar_empty),
        ));
    }
    output.push_str(&renderer.render_middle_divider());

    // Disk Section
    output.push_str(&renderer.render_row("VOLUME", &info.disk_usage_str()));
    let disk_bar = render_percent_bar(info.disk_percent, data_width, bar_filled, bar_empty);
    output.push_str(&renderer.render_row("DISK USAGE", &disk_bar));

    // ZFS health if available
    if let Some(ref zfs_health) = info.zfs_health {
        output.push_str(&renderer.render_row("ZFS HEALTH", zfs_health));
    }

    output.push_str(&renderer.render_middle_divider());

    // Memory Section
    output.push_str(&renderer.render_row("MEMORY", &info.memory_usage_str()));
    output.push_str(&renderer.render_row(
        "AVAILABLE",
        &format!("{} GiB", SystemInfo::format_gib(info.mem_available_bytes)),
    ));
    if info.swap_total_bytes > 0 {
        output.push_str(&renderer.render_row("SWAP", &info.swap_usage_str()));
    }
    if let Some(ref ram_slots) = info.ram_slots {
        output.push_str(&renderer.render_row("RAM SLOTS", ram_slots));
    }
    let mem_bar = render_percent_bar(info.mem_percent, data_width, bar_filled, bar_empty);
    output.push_str(&renderer.render_row("USAGE", &mem_bar));
    output.push_str(&renderer.render_middle_divider());

    // Session Section
    if let Some(ref last_login) = info.last_login {
        output.push_str(&renderer.render_row("LAST LOGIN", last_login));
        if let Some(ref ip) = info.last_login_ip {
            output.push_str(&renderer.render_row("LOGIN ORIGIN", ip));
        }
    }
    output.push_str(&renderer.render_row("UPTIME", &info.uptime_formatted()));

    // Shell and Terminal (only show if available)
    if let Some(ref shell) = info.shell {
        output.push_str(&renderer.render_row("LOGIN SHELL", shell));
    }
    if let Some(ref terminal) = info.terminal {
        output.push_str(&renderer.render_row("TERMINAL", terminal));
    }
    if let Some(ref locale) = info.locale {
        output.push_str(&renderer.render_row("LOCALE", locale));
    }
    // Battery only shown if present (laptops)
    if let Some(ref battery) = info.battery {
        if let Some((status, health)) = battery.split_once("; ") {
            output.push_str(&renderer.render_row("BATTERY", status));
            output.push_str(&renderer.render_row("BAT HEALTH", health));
        } else {
            output.push_str(&renderer.render_row("BATTERY", battery));
        }
    }
    // Encryption status (BitLocker / FileVault / LUKS) is shown only when a
    // collector can establish it. Absence is unknown, not "unencrypted".
    if let Some(ref enc) = info.encryption {
        output.push_str(&renderer.render_row("ENCRYPTION", enc));
    }

    // Simplified footer (single line, no bottom_divider)
    output.push_str(&renderer.render_footer());

    // Elevation-tier footer hint: only shown when running unelevated on a platform
    // where sudo/admin would unlock additional data, in full mode, and not opted-out.
    // Never shown in --fast mode (auto-run) — would clutter the prompt-ready output.
    if should_render_elevation_footer(
        info.is_elevated,
        info.elevation_unlocks_more,
        info.mode,
        config.no_elevation_hint,
    ) {
        output.push_str(&render_elevation_footer(config.use_colors));
    }

    output
}

fn render_percent_bar(percent: f64, width: usize, filled: char, empty: char) -> String {
    let value = if percent.is_finite() { percent } else { 0.0 };
    let suffix = format!(" {:.1}%", value);
    let bar_width = width.saturating_sub(suffix.len()).max(1);
    format!("{}{}", render_bar(value, bar_width, filled, empty), suffix)
}

/// Decide whether the elevation-tier footer hint should appear under the table.
/// Extracted so the gate is unit-testable independently from rendering.
pub(crate) fn should_render_elevation_footer(
    is_elevated: bool,
    elevation_unlocks_more: bool,
    mode: CollectMode,
    no_elevation_hint: bool,
) -> bool {
    !is_elevated && elevation_unlocks_more && mode != CollectMode::Fast && !no_elevation_hint
}

/// Render the dim elevation-tier hint as a single line.
/// ANSI-dim when colors are enabled; plain text otherwise.
/// Returns an empty string on platforms without elevated-only data (macOS).
pub(crate) fn render_elevation_footer(use_colors: bool) -> String {
    let hint: &str = if cfg!(target_os = "linux") {
        "Run with sudo for SMBIOS RAM module details"
    } else if cfg!(target_os = "windows") {
        "Run as Administrator for BitLocker status"
    } else {
        return String::new();
    };
    if use_colors {
        format!("\x1b[2m{}\x1b[0m\n", hint)
    } else {
        format!("{}\n", hint)
    }
}

/// Generate JSON format output
fn generate_json(info: &SystemInfo) -> String {
    fn finite(value: f64) -> Option<f64> {
        value.is_finite().then_some(value)
    }
    fn finite_positive(value: f64) -> Option<f64> {
        (value.is_finite() && value > 0.0).then_some(value)
    }

    // Build a typed JSON value tree and let serde_json own all escaping,
    // punctuation, and non-finite-number handling. This preserves schema v1
    // while making additive fields much harder to corrupt accidentally.
    let value = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "collection_mode": match info.mode {
            CollectMode::Full => "full",
            CollectMode::Fast => "fast",
        },
        "elevated": info.is_elevated,
        "elevation_unlocks_more": info.elevation_unlocks_more && !info.is_elevated,
        "system": {
            "motherboard": info.motherboard,
            "bios": info.bios,
            "boot_mode": info.boot_mode,
            "desktop_environment": info.desktop_environment,
            "display_server": info.display_server,
            "display_resolution": info.display_resolution,
        },
        "os": {
            "name": info.os_name,
            "version": info.os_version,
            "edition": info.os_edition,
            "codename": info.os_codename,
            "build": info.os_build,
            "kernel": info.kernel,
            "architecture": info.architecture,
            "machine_model": info.machine_model,
            "session_uptime_seconds": info.session_uptime_seconds,
        },
        "network": {
            "hostname": info.hostname,
            "machine_ip": info.machine_ip,
            "machine_ip_scope": info.machine_ip.as_ref().map(|_| "default_route"),
            "client_ip": info.client_ip,
            "client_ip_scope": info.client_ip.as_ref().map(|_| "ssh"),
            "dns_servers": info.dns_servers,
        },
        "cpu": {
            // Original `cores` key remains the logical processor count.
            "processor": info.processor,
            "cores": info.cores,
            "logical_processors": info.cores,
            "physical_cores": info.physical_cores,
            "core_topology": info.cpu_core_topology,
            "sockets": info.sockets,
            "hypervisor": info.hypervisor,
            "frequency_ghz": finite_positive(info.cpu_freq_ghz),
            "frequency_kind": info.cpu_frequency_kind,
            "usage_percent": info.cpu_usage_percent.and_then(finite),
            // Existing load keys remain normalized percent-of-logical-CPU
            // capacity. Raw OS load averages are additive and Unix-only.
            "load_1m": info.load_1m.and_then(finite),
            "load_5m": info.load_5m.and_then(finite),
            "load_15m": info.load_15m.and_then(finite),
            "load_raw_1m": info.raw_load_1m.and_then(finite),
            "load_raw_5m": info.raw_load_5m.and_then(finite),
            "load_raw_15m": info.raw_load_15m.and_then(finite),
            "load_unit": "percent_of_logical_cpu_capacity",
            "load_raw_unit": info.raw_load_1m.map(|_| "runnable_queue_average"),
            "gpus": info.gpus,
        },
        "disk": {
            "used_bytes": info.disk_used_bytes,
            "total_bytes": info.disk_total_bytes,
            "available_bytes": info.disk_available_bytes,
            "percent": finite(info.disk_percent),
            "mount_point": info.disk_mount_point,
            "filesystem": info.disk_filesystem,
            "used_definition": "allocated_bytes",
            "available_definition": "available_to_current_caller",
            "zfs_health": info.zfs_health,
        },
        "memory": {
            "used_bytes": info.mem_used_bytes,
            "total_bytes": info.mem_total_bytes,
            "available_bytes": info.mem_available_bytes,
            "percent": finite(info.mem_percent),
            "usage_definition": info.memory_usage_kind,
            "availability_definition": info.memory_availability_kind,
            "swap_used_bytes": info.swap_used_bytes,
            "swap_total_bytes": info.swap_total_bytes,
            "swap_percent": finite(info.swap_percent),
            "ram_slots": info.ram_slots,
        },
        "session": {
            "username": info.username,
            "last_login": info.last_login,
            "last_login_ip": info.last_login_ip,
            "uptime_seconds": info.uptime_seconds,
            "shell": info.shell,
            "terminal": info.terminal,
            "locale": info.locale,
            "battery": info.battery,
            "encryption": info.encryption,
        }
    });
    serde_json::to_string_pretty(&value).expect("serializing a serde_json::Value cannot fail")
}

/// Escape special characters for JSON.
///
/// Delegates to `serde_json::to_string` and strips the wrapping
/// quotes that `to_string` always adds, matching the inline-template
/// shape callers expect (`"name": "{}"` with the value pre-escaped).
///
/// Pre-v3.15.7 this was hand-rolled with chained `.replace()` calls
/// plus a `chars()` pass that formatted `is_control()` chars as
/// `\u00xx` (5 chars). The chained replace was correct for the
/// listed cases but easy to break with a future addition; using
/// `serde_json` for the actual escape eliminates a class of
/// hand-roll maintenance risk and aligns with the spec for the
/// full Unicode range. (audit finding F15)
#[cfg_attr(not(test), allow(dead_code))]
fn escape_json(s: &str) -> String {
    match serde_json::to_string(s) {
        Ok(quoted) if quoted.len() >= 2 => quoted[1..quoted.len() - 1].to_string(),
        _ => String::new(),
    }
}

/// Escape markdown table cell delimiters and line breaks.
fn escape_markdown_cell(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\r', "")
        .replace('\n', "<br>")
}

/// Get the user's Downloads directory (cross-platform).
///
/// Returns `(dir, used_cwd_fallback)`. `used_cwd_fallback` is `true` only when
/// no Downloads folder was found and the current working directory was used
/// instead — the caller surfaces this so the report doesn't silently land in
/// an unexpected place.
fn downloads_dir() -> (PathBuf, bool) {
    // `dirs` honors Windows Known Folders and Linux XDG user directories;
    // hard-coding `$HOME/Downloads` misses redirected/localized folders.
    if let Some(dir) = dirs::download_dir() {
        return (dir, false);
    }

    // Fallback: current working directory
    (
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        true,
    )
}

/// Generate a comprehensive markdown report from system info
fn generate_markdown(info: &SystemInfo) -> String {
    let timestamp = chrono::Local::now().to_rfc3339();
    let version = env!("CARGO_PKG_VERSION");
    let cell = |s: &str| escape_markdown_cell(s);

    let mut md = String::new();

    md.push_str("# TR-300 Machine Report\n\n");
    md.push_str(&format!("**Date:** {}\n", timestamp));
    md.push_str(&format!(
        "**Hostname:** {}\n",
        escape_markdown_cell(&info.hostname)
    ));
    md.push_str("\n---\n\n");

    // System section
    md.push_str("## System\n\n");
    md.push_str("| Field | Value |\n|-------|-------|\n");
    md.push_str(&format!(
        "| OS | {} {} |\n",
        cell(&info.os_name),
        cell(&info.os_version)
    ));
    if let Some(ref edition) = info.os_edition {
        md.push_str(&format!("| Edition | {} |\n", cell(edition)));
    }
    if let Some(ref codename) = info.os_codename {
        md.push_str(&format!("| Codename | {} |\n", cell(codename)));
    }
    if let Some(ref build) = info.os_build {
        md.push_str(&format!("| Build | {} |\n", cell(build)));
    }
    md.push_str(&format!("| Kernel | {} |\n", cell(&info.kernel)));
    md.push_str(&format!(
        "| Architecture | {} |\n",
        cell(&info.architecture)
    ));
    if let Some(ref model) = info.machine_model {
        md.push_str(&format!("| Machine Model | {} |\n", cell(model)));
    }
    if let Some(ref motherboard) = info.motherboard {
        md.push_str(&format!("| Motherboard | {} |\n", cell(motherboard)));
    }
    if let Some(ref bios) = info.bios {
        md.push_str(&format!("| BIOS | {} |\n", cell(bios)));
    }
    if let Some(ref boot_mode) = info.boot_mode {
        md.push_str(&format!("| Boot Mode | {} |\n", cell(boot_mode)));
    }
    if let Some(ref desktop) = info.desktop_environment {
        md.push_str(&format!("| Desktop | {} |\n", cell(desktop)));
    }
    if let Some(ref server) = info.display_server {
        md.push_str(&format!("| Display/Session Server | {} |\n", cell(server)));
    }
    if let Some(ref resolution) = info.display_resolution {
        md.push_str(&format!("| Display | {} |\n", cell(resolution)));
    }
    md.push('\n');

    // Network section
    md.push_str("## Network\n\n");
    md.push_str("| Field | Value |\n|-------|-------|\n");
    md.push_str(&format!("| Hostname | {} |\n", cell(&info.hostname)));
    if let Some(ref ip) = info.machine_ip {
        md.push_str(&format!("| Default-route IP | {} |\n", cell(ip)));
    }
    md.push_str(&format!(
        "| SSH Client IP | {} |\n",
        cell(info.client_ip.as_deref().unwrap_or("Not an SSH session"))
    ));
    for (i, dns) in info.dns_servers.iter().take(5).enumerate() {
        md.push_str(&format!("| DNS Server {} | {} |\n", i + 1, cell(dns)));
    }
    md.push_str(&format!("| User | {} |\n", cell(&info.username)));
    md.push('\n');

    // CPU section
    md.push_str("## CPU\n\n");
    md.push_str("| Field | Value |\n|-------|-------|\n");
    md.push_str(&format!("| Processor | {} |\n", cell(&info.processor)));
    md.push_str(&format!("| Cores | {} |\n", info.cores_str()));
    if let Some(ref topology) = info.cpu_core_topology {
        md.push_str(&format!("| Core Topology | {} |\n", cell(topology)));
    }
    for (i, gpu) in info.gpus.iter().enumerate() {
        let label = if info.gpus.len() == 1 {
            "GPU".to_string()
        } else {
            format!("GPU {}", i + 1)
        };
        md.push_str(&format!("| {} | {} |\n", label, cell(gpu)));
    }
    if let Some(ref hypervisor) = info.hypervisor {
        md.push_str(&format!("| Hypervisor | {} |\n", cell(hypervisor)));
    }
    if info.cpu_freq_ghz > 0.0 && info.cpu_freq_ghz.is_finite() {
        let label = if info.cpu_frequency_kind.as_deref() == Some("maximum") {
            "Maximum Frequency"
        } else {
            "Reported Frequency"
        };
        md.push_str(&format!("| {} | {} |\n", label, info.freq_str()));
    }
    if let Some(usage) = info.cpu_usage_percent {
        md.push_str(&format!("| CPU Usage | {:.2}% |\n", usage));
    }
    if let (Some(l1), Some(l5), Some(l15)) = (info.load_1m, info.load_5m, info.load_15m) {
        md.push_str(&format!("| Load / CPU 1m | {:.2}% |\n", l1));
        md.push_str(&format!("| Load / CPU 5m | {:.2}% |\n", l5));
        md.push_str(&format!("| Load / CPU 15m | {:.2}% |\n", l15));
    }
    if let (Some(l1), Some(l5), Some(l15)) = (info.raw_load_1m, info.raw_load_5m, info.raw_load_15m)
    {
        md.push_str(&format!("| Raw Load 1m | {:.2} |\n", l1));
        md.push_str(&format!("| Raw Load 5m | {:.2} |\n", l5));
        md.push_str(&format!("| Raw Load 15m | {:.2} |\n", l15));
    }
    md.push('\n');

    // Storage section
    md.push_str("## Storage\n\n");
    md.push_str("| Field | Value |\n|-------|-------|\n");
    md.push_str(&format!("| Volume | {} |\n", info.disk_usage_str()));
    if let Some(ref mount) = info.disk_mount_point {
        md.push_str(&format!("| Mount Point | {} |\n", cell(mount)));
    }
    if let Some(ref filesystem) = info.disk_filesystem {
        md.push_str(&format!("| Filesystem | {} |\n", cell(filesystem)));
    }
    md.push_str(&format!(
        "| Available | {} GiB |\n",
        SystemInfo::format_gib(info.disk_available_bytes)
    ));
    md.push_str(&format!("| Disk Usage | {:.2}% |\n", info.disk_percent));
    if let Some(ref zfs_health) = info.zfs_health {
        md.push_str(&format!("| ZFS Health | {} |\n", cell(zfs_health)));
    }
    md.push('\n');

    // Memory section
    md.push_str("## Memory\n\n");
    md.push_str("| Field | Value |\n|-------|-------|\n");
    md.push_str(&format!("| Memory | {} |\n", info.memory_usage_str()));
    md.push_str(&format!(
        "| Available | {} GiB |\n",
        SystemInfo::format_gib(info.mem_available_bytes)
    ));
    md.push_str(&format!(
        "| Usage Definition | {} |\n",
        cell(&info.memory_usage_kind)
    ));
    md.push_str(&format!(
        "| Availability Definition | {} |\n",
        cell(&info.memory_availability_kind)
    ));
    if info.swap_total_bytes > 0 {
        md.push_str(&format!("| Swap | {} |\n", info.swap_usage_str()));
    }
    if let Some(ref ram_slots) = info.ram_slots {
        md.push_str(&format!("| RAM Slots | {} |\n", cell(ram_slots)));
    }
    md.push_str(&format!("| Usage | {:.1}% |\n", info.mem_percent));
    md.push('\n');

    // Session section
    md.push_str("## Session\n\n");
    md.push_str("| Field | Value |\n|-------|-------|\n");
    if let Some(ref last_login) = info.last_login {
        md.push_str(&format!("| Last Login | {} |\n", cell(last_login)));
        if let Some(ref origin) = info.last_login_ip {
            md.push_str(&format!("| Login Origin | {} |\n", cell(origin)));
        }
    }
    md.push_str(&format!("| Uptime | {} |\n", info.uptime_formatted()));
    if let Some(ref shell) = info.shell {
        md.push_str(&format!("| Login Shell | {} |\n", cell(shell)));
    }
    if let Some(ref terminal) = info.terminal {
        md.push_str(&format!("| Terminal | {} |\n", cell(terminal)));
    }
    if let Some(ref locale) = info.locale {
        md.push_str(&format!("| Locale | {} |\n", cell(locale)));
    }
    if let Some(ref battery) = info.battery {
        md.push_str(&format!("| Battery | {} |\n", cell(battery)));
    }
    if let Some(ref encryption) = info.encryption {
        md.push_str(&format!("| Encryption | {} |\n", cell(encryption)));
    }

    md.push_str("\n---\n\n");
    md.push_str(&format!("*Generated by TR-300 v{}*\n", version));

    md
}

/// Outcome of a successful markdown report save.
#[non_exhaustive]
pub struct MarkdownSaveOutcome {
    /// Where the report was written.
    pub path: PathBuf,
    /// `true` when no Downloads folder was found and the report was written to
    /// the current working directory instead.
    pub used_cwd_fallback: bool,
}

/// Save a markdown report to the Downloads folder (or the current working
/// directory if no Downloads folder exists).
///
/// Returns the concrete `io::Error` on failure so the caller can tell the user
/// *why* the save failed (permissions, full disk, missing directory) rather
/// than swallowing it into a generic warning.
pub fn save_markdown_report(info: &SystemInfo) -> std::io::Result<MarkdownSaveOutcome> {
    let (dir, used_cwd_fallback) = downloads_dir();
    let filename_ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let filename_stem = format!("tr300-report-{}", filename_ts);

    save_markdown_report_to_dir(info, &dir, used_cwd_fallback, &filename_stem)
}

fn save_markdown_report_to_dir(
    info: &SystemInfo,
    dir: &Path,
    used_cwd_fallback: bool,
    filename_stem: &str,
) -> std::io::Result<MarkdownSaveOutcome> {
    std::fs::create_dir_all(dir)?;

    let markdown = generate_markdown(info);

    for suffix in 0..=999u16 {
        let filename = if suffix == 0 {
            format!("{}.md", filename_stem)
        } else {
            format!("{}-{}.md", filename_stem, suffix)
        };
        let path = dir.join(filename);

        let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };

        if let Err(error) = file
            .write_all(markdown.as_bytes())
            .and_then(|_| file.sync_all())
        {
            drop(file);
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }

        return Ok(MarkdownSaveOutcome {
            path,
            used_cwd_fallback,
        });
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not allocate a unique report filename after 1000 attempts",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elevation_footer_skipped_when_elevated() {
        // Already running as root/admin — nothing more to unlock; no hint.
        assert!(!should_render_elevation_footer(
            true,
            true,
            CollectMode::Full,
            false
        ));
    }

    #[test]
    fn elevation_footer_skipped_in_fast_mode() {
        // Auto-run uses --fast; the prompt must be free immediately, no footer noise.
        assert!(!should_render_elevation_footer(
            false,
            true,
            CollectMode::Fast,
            false
        ));
    }

    #[test]
    fn elevation_footer_skipped_when_user_opted_out() {
        // --no-elevation-hint suppresses the line for users who find it noisy.
        assert!(!should_render_elevation_footer(
            false,
            true,
            CollectMode::Full,
            true
        ));
    }

    #[test]
    fn elevation_footer_present_when_unelevated_full_no_optout() {
        assert!(should_render_elevation_footer(
            false,
            true,
            CollectMode::Full,
            false
        ));
        assert!(!should_render_elevation_footer(
            false,
            false,
            CollectMode::Full,
            false
        ));
    }

    #[test]
    fn elevation_footer_string_is_empty_on_macos() {
        // macOS has no elevation footer, so the renderer returns an empty
        // string even if the gate was somehow bypassed.
        if cfg!(target_os = "macos") {
            assert_eq!(render_elevation_footer(false), "");
            assert_eq!(render_elevation_footer(true), "");
        }
    }

    #[test]
    fn elevation_footer_uses_ansi_dim_when_colors_enabled() {
        if cfg!(target_os = "linux") || cfg!(target_os = "windows") {
            let with_color = render_elevation_footer(true);
            let no_color = render_elevation_footer(false);
            assert!(with_color.starts_with("\x1b[2m"));
            assert!(with_color.ends_with("\x1b[0m\n"));
            assert!(!no_color.contains("\x1b["));
            assert!(no_color.ends_with('\n'));
        }
    }

    #[test]
    fn schema_version_is_one() {
        // Bump only on breaking changes (renames/removals); additive new keys do not bump.
        assert_eq!(SCHEMA_VERSION, 1);
    }

    fn fixture_info() -> SystemInfo {
        SystemInfo {
            os_name: "TestOS".to_string(),
            os_version: "1.0".to_string(),
            kernel: "1.0.0".to_string(),
            architecture: "test-arch".to_string(),
            machine_model: Some("Model | One".to_string()),
            os_edition: Some("Enterprise".to_string()),
            os_codename: Some("Exact".to_string()),
            os_build: Some("100.2".to_string()),
            hostname: "host".to_string(),
            machine_ip: Some("192.0.2.10".to_string()),
            client_ip: None,
            dns_servers: vec!["1.1.1.1".to_string()],
            username: "user".to_string(),
            processor: "CPU".to_string(),
            cores: 8,
            physical_cores: 4,
            sockets: Some(1),
            cpu_core_topology: Some("4P + 4E".to_string()),
            hypervisor: Some("Bare Metal".to_string()),
            cpu_freq_ghz: 3.2,
            cpu_frequency_kind: Some("maximum".to_string()),
            cpu_usage_percent: Some(12.5),
            load_1m: Some(10.0),
            load_5m: Some(20.0),
            load_15m: Some(30.0),
            raw_load_1m: Some(0.8),
            raw_load_5m: Some(1.6),
            raw_load_15m: Some(2.4),
            gpus: vec!["GPU".to_string()],
            disk_used_bytes: 1,
            disk_total_bytes: 2,
            disk_available_bytes: 1,
            disk_percent: 50.0,
            disk_mount_point: Some("/".to_string()),
            disk_filesystem: Some("testfs".to_string()),
            zfs_health: Some("ONLINE".to_string()),
            mem_used_bytes: 1,
            mem_total_bytes: 2,
            mem_available_bytes: 1,
            mem_percent: 50.0,
            memory_usage_kind: "total-minus-available".to_string(),
            memory_availability_kind: "operating-system-available".to_string(),
            swap_used_bytes: 1,
            swap_total_bytes: 4,
            swap_percent: 25.0,
            motherboard: Some("Board | Vendor".to_string()),
            bios: Some("BIOS | 1.2".to_string()),
            ram_slots: Some("2x16GB | DDR5".to_string()),
            last_login: Some("now".to_string()),
            last_login_ip: None,
            uptime_seconds: 60,
            session_uptime_seconds: None,
            shell: Some("shell".to_string()),
            terminal: Some("term".to_string()),
            locale: Some("en-US".to_string()),
            battery: None,
            encryption: Some("Encrypted".to_string()),
            desktop_environment: Some("Desktop".to_string()),
            display_server: Some("Session".to_string()),
            display_resolution: Some("1920x1080".to_string()),
            boot_mode: Some("UEFI".to_string()),
            mode: CollectMode::Full,
            is_elevated: true,
            elevation_unlocks_more: false,
        }
    }

    #[test]
    fn json_includes_new_nullable_platform_keys_without_schema_bump() {
        let json = generate_json(&fixture_info());
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["os"]["machine_model"], "Model | One");
        assert_eq!(value["cpu"]["core_topology"], "4P + 4E");
        assert_eq!(value["cpu"]["physical_cores"], 4);
        assert_eq!(value["cpu"]["logical_processors"], 8);
        assert_eq!(value["cpu"]["load_raw_1m"], 0.8);
        assert_eq!(value["memory"]["ram_slots"], "2x16GB | DDR5");
        assert_eq!(value["memory"]["available_bytes"], 1);
        assert_eq!(value["memory"]["usage_definition"], "total-minus-available");
        assert_eq!(value["memory"]["swap_percent"], 25.0);
        assert_eq!(value["system"]["motherboard"], "Board | Vendor");
        assert_eq!(value["system"]["bios"], "BIOS | 1.2");
        assert_eq!(value["collection_mode"], "full");
        assert_eq!(value["session"]["encryption"], "Encrypted");
        assert_eq!(value["network"]["machine_ip_scope"], "default_route");
        assert!(value["network"]["client_ip_scope"].is_null());
        assert_eq!(value["disk"]["used_definition"], "allocated_bytes");
    }

    #[test]
    fn json_converts_non_finite_metrics_to_null() {
        let mut info = fixture_info();
        info.cpu_freq_ghz = f64::NAN;
        info.cpu_usage_percent = Some(f64::INFINITY);
        info.load_1m = Some(f64::NEG_INFINITY);
        info.disk_percent = f64::NAN;
        info.mem_percent = f64::INFINITY;
        info.swap_percent = f64::NEG_INFINITY;

        let value: serde_json::Value = serde_json::from_str(&generate_json(&info))
            .expect("valid JSON despite non-finite input");
        assert!(value["cpu"]["frequency_ghz"].is_null());
        assert!(value["cpu"]["usage_percent"].is_null());
        assert!(value["cpu"]["load_1m"].is_null());
        assert!(value["disk"]["percent"].is_null());
        assert!(value["memory"]["percent"].is_null());
        assert!(value["memory"]["swap_percent"].is_null());
    }

    #[test]
    fn markdown_escapes_table_cell_pipes() {
        let markdown = generate_markdown(&fixture_info());
        assert!(markdown.contains("| Machine Model | Model \\| One |"));
        assert!(markdown.contains("| Motherboard | Board \\| Vendor |"));
        assert!(markdown.contains("| RAM Slots | 2x16GB \\| DDR5 |"));
        assert!(markdown.contains("| Encryption | Encrypted |"));
    }

    fn isolated_test_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("tr300-report-{name}-{}", std::process::id()))
    }

    #[test]
    fn markdown_save_never_overwrites_an_existing_filename() {
        let dir = isolated_test_dir("collision");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let existing = dir.join("tr300-report-fixed.md");
        std::fs::write(&existing, "keep me").unwrap();

        let outcome =
            save_markdown_report_to_dir(&fixture_info(), &dir, false, "tr300-report-fixed")
                .unwrap();

        assert_eq!(std::fs::read_to_string(existing).unwrap(), "keep me");
        assert_eq!(
            outcome.path.file_name().and_then(|name| name.to_str()),
            Some("tr300-report-fixed-1.md")
        );
        assert!(std::fs::read_to_string(&outcome.path)
            .unwrap()
            .contains("# TR-300 Machine Report"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn markdown_save_does_not_follow_a_preexisting_symlink() {
        use std::os::unix::fs::symlink;

        let dir = isolated_test_dir("symlink");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let target = dir.join("target.md");
        std::fs::write(&target, "do not replace").unwrap();
        symlink(&target, dir.join("tr300-report-fixed.md")).unwrap();

        let outcome =
            save_markdown_report_to_dir(&fixture_info(), &dir, false, "tr300-report-fixed")
                .unwrap();

        assert_eq!(std::fs::read_to_string(target).unwrap(), "do not replace");
        assert_eq!(
            outcome.path.file_name().and_then(|name| name.to_str()),
            Some("tr300-report-fixed-1.md")
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn escape_json_handles_backslash_and_quote() {
        // The two characters JSON requires escaped inside a string.
        // Verify the round-trip is spec-compliant by parsing the
        // result wrapped in literal quotes.
        let escaped = super::escape_json(r#"path\with"quote"#);
        let wrapped = format!("\"{}\"", escaped);
        let parsed: String = serde_json::from_str(&wrapped).unwrap();
        assert_eq!(parsed, r#"path\with"quote"#);
    }

    #[test]
    fn escape_json_handles_control_chars() {
        let input = "before\nmid\tafter\u{0001}end";
        let escaped = super::escape_json(input);
        let wrapped = format!("\"{}\"", escaped);
        let parsed: String = serde_json::from_str(&wrapped).unwrap();
        assert_eq!(parsed, input, "round-trip must preserve control chars");
    }

    #[test]
    fn escape_json_handles_supplementary_plane_emoji() {
        // Regional indicators (flag building blocks) live above
        // U+FFFF. Pre-v3.15.7 the hand-rolled escape would emit
        // `἟a` for a control char in this range — non-spec.
        // (Such codepoints aren't actually control chars, but the
        // serde_json delegation makes the spec-correctness explicit.)
        let input = "🇺🇸 Daisy's Mac";
        let escaped = super::escape_json(input);
        let wrapped = format!("\"{}\"", escaped);
        let parsed: String = serde_json::from_str(&wrapped).unwrap();
        assert_eq!(parsed, input);
    }

    #[test]
    fn escape_json_handles_empty_string() {
        assert_eq!(super::escape_json(""), "");
    }

    #[test]
    fn escape_json_handles_plain_ascii() {
        // No special chars — output should match input.
        assert_eq!(super::escape_json("plain ascii"), "plain ascii");
    }
}
