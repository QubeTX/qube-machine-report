//! Report generation matching TR-200 format
//!
//! Generates the complete system report using the exact
//! layout and styling of TR-200 Machine Report.

use std::path::PathBuf;

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

/// Generate table format output (TR-200 style)
fn generate_table(info: &SystemInfo, config: &Config) -> String {
    let chars = config.box_chars();
    let (bar_filled, bar_empty) = config.bar_chars();

    // Use fixed column widths matching TR-200
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
    output.push_str(&renderer.render_middle_divider());

    // Network Section
    output.push_str(&renderer.render_row("HOSTNAME", &info.hostname));
    if let Some(ref ip) = info.machine_ip {
        output.push_str(&renderer.render_row("MACHINE IP", ip));
    }
    output.push_str(&renderer.render_row(
        "CLIENT  IP",
        info.client_ip.as_deref().unwrap_or("Not connected"),
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
    output.push_str(&renderer.render_row("CPU FREQ", &info.freq_str()));

    // Load averages as bar graphs (only shown when available)
    let bar_width = data_width;
    if let (Some(l1), Some(l5), Some(l15)) = (info.load_1m, info.load_5m, info.load_15m) {
        let load_1m_bar = render_bar(l1, bar_width, bar_filled, bar_empty);
        let load_5m_bar = render_bar(l5, bar_width, bar_filled, bar_empty);
        let load_15m_bar = render_bar(l15, bar_width, bar_filled, bar_empty);

        output.push_str(&renderer.render_row("LOAD  1m", &load_1m_bar));
        output.push_str(&renderer.render_row("LOAD  5m", &load_5m_bar));
        output.push_str(&renderer.render_row("LOAD 15m", &load_15m_bar));
    }
    output.push_str(&renderer.render_middle_divider());

    // Disk Section
    output.push_str(&renderer.render_row("VOLUME", &info.disk_usage_str()));
    let disk_bar = render_bar(info.disk_percent, bar_width, bar_filled, bar_empty);
    output.push_str(&renderer.render_row("DISK USAGE", &disk_bar));

    // ZFS health if available
    if let Some(ref zfs_health) = info.zfs_health {
        output.push_str(&renderer.render_row("ZFS HEALTH", zfs_health));
    }

    output.push_str(&renderer.render_middle_divider());

    // Memory Section
    output.push_str(&renderer.render_row("MEMORY", &info.memory_usage_str()));
    if let Some(ref ram_slots) = info.ram_slots {
        output.push_str(&renderer.render_row("RAM SLOTS", ram_slots));
    }
    let mem_bar = render_bar(info.mem_percent, bar_width, bar_filled, bar_empty);
    output.push_str(&renderer.render_row("USAGE", &mem_bar));
    output.push_str(&renderer.render_middle_divider());

    // Session Section
    if let Some(ref last_login) = info.last_login {
        output.push_str(&renderer.render_row("LAST LOGIN", last_login));
        if let Some(ref ip) = info.last_login_ip {
            output.push_str(&renderer.render_row("", ip));
        }
    }
    output.push_str(&renderer.render_row("UPTIME", &info.uptime_formatted()));

    // Shell and Terminal (only show if available)
    if let Some(ref shell) = info.shell {
        output.push_str(&renderer.render_row("SHELL", shell));
    }
    if let Some(ref terminal) = info.terminal {
        output.push_str(&renderer.render_row("TERMINAL", terminal));
    }
    if let Some(ref locale) = info.locale {
        output.push_str(&renderer.render_row("LOCALE", locale));
    }
    // Battery only shown if present (laptops)
    if let Some(ref battery) = info.battery {
        output.push_str(&renderer.render_row("BATTERY", battery));
    }
    // Encryption status (BitLocker / FileVault / LUKS) only shown when we
    // can actually read it; absence of the row is intentional on systems
    // where elevation would be required (footer hint covers that case).
    if let Some(ref enc) = info.encryption {
        output.push_str(&renderer.render_row("ENCRYPTION", enc));
    }

    // Simplified footer (single line, no bottom_divider)
    output.push_str(&renderer.render_footer());

    // Elevation-tier footer hint: only shown when running unelevated on a platform
    // where sudo/admin would unlock additional data, in full mode, and not opted-out.
    // Never shown in --fast mode (auto-run) — would clutter the prompt-ready output.
    if should_render_elevation_footer(info.is_elevated, info.mode, config.no_elevation_hint) {
        output.push_str(&render_elevation_footer(config.use_colors));
    }

    output
}

/// Decide whether the elevation-tier footer hint should appear under the table.
/// Extracted so the gate is unit-testable independently from rendering.
pub(crate) fn should_render_elevation_footer(
    is_elevated: bool,
    mode: CollectMode,
    no_elevation_hint: bool,
) -> bool {
    !is_elevated
        && crate::platform_has_elevated_data()
        && mode != CollectMode::Fast
        && !no_elevation_hint
}

/// Render the dim elevation-tier hint as a single line.
/// ANSI-dim when colors are enabled; plain text otherwise.
/// Returns an empty string on platforms without elevated-only data (macOS).
pub(crate) fn render_elevation_footer(use_colors: bool) -> String {
    let hint: &str = if cfg!(target_os = "linux") {
        "Run with sudo for motherboard, BIOS, and RAM slot details"
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
    let opt_str = |o: &Option<String>| -> String {
        o.as_ref()
            .map(|s| format!("\"{}\"", escape_json(s)))
            .unwrap_or_else(|| "null".to_string())
    };
    let opt_f64 = |o: Option<f64>| -> String {
        o.map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "null".to_string())
    };
    let opt_usize = |o: Option<usize>| -> String {
        o.map(|v| format!("{}", v))
            .unwrap_or_else(|| "null".to_string())
    };
    let opt_u64 = |o: Option<u64>| -> String {
        o.map(|v| format!("{}", v))
            .unwrap_or_else(|| "null".to_string())
    };

    // Simple JSON serialization without serde
    format!(
        r#"{{
  "schema_version": {},
  "elevated": {},
  "elevation_unlocks_more": {},
  "system": {{
    "motherboard": {},
    "bios": {}
  }},
  "os": {{
    "name": "{}",
    "version": "{}",
    "kernel": "{}",
    "architecture": "{}",
    "machine_model": {},
    "session_uptime_seconds": {}
  }},
  "network": {{
    "hostname": "{}",
    "machine_ip": {},
    "client_ip": {},
    "dns_servers": [{}]
  }},
  "cpu": {{
    "processor": "{}",
    "cores": {},
    "core_topology": {},
    "sockets": {},
    "hypervisor": {},
    "frequency_ghz": {:.2},
    "load_1m": {},
    "load_5m": {},
    "load_15m": {},
    "gpus": [{}]
  }},
  "disk": {{
    "used_bytes": {},
    "total_bytes": {},
    "percent": {:.2}
  }},
  "memory": {{
    "used_bytes": {},
    "total_bytes": {},
    "percent": {:.2},
    "ram_slots": {}
  }},
  "session": {{
    "username": "{}",
    "last_login": {},
    "uptime_seconds": {},
    "shell": {},
    "terminal": {},
    "locale": {},
    "battery": {},
    "encryption": {}
  }}
}}"#,
        SCHEMA_VERSION,
        info.is_elevated,
        crate::platform_has_elevated_data() && !info.is_elevated,
        opt_str(&info.motherboard),
        opt_str(&info.bios),
        escape_json(&info.os_name),
        escape_json(&info.os_version),
        escape_json(&info.kernel),
        escape_json(&info.architecture),
        opt_str(&info.machine_model),
        opt_u64(info.session_uptime_seconds),
        escape_json(&info.hostname),
        opt_str(&info.machine_ip),
        opt_str(&info.client_ip),
        info.dns_servers
            .iter()
            .map(|s| format!("\"{}\"", escape_json(s)))
            .collect::<Vec<_>>()
            .join(", "),
        escape_json(&info.processor),
        info.cores,
        opt_str(&info.cpu_core_topology),
        opt_usize(info.sockets),
        opt_str(&info.hypervisor),
        info.cpu_freq_ghz,
        opt_f64(info.load_1m),
        opt_f64(info.load_5m),
        opt_f64(info.load_15m),
        info.gpus
            .iter()
            .map(|s| format!("\"{}\"", escape_json(s)))
            .collect::<Vec<_>>()
            .join(", "),
        info.disk_used_bytes,
        info.disk_total_bytes,
        info.disk_percent,
        info.mem_used_bytes,
        info.mem_total_bytes,
        info.mem_percent,
        opt_str(&info.ram_slots),
        escape_json(&info.username),
        opt_str(&info.last_login),
        info.uptime_seconds,
        opt_str(&info.shell),
        opt_str(&info.terminal),
        opt_str(&info.locale),
        opt_str(&info.battery),
        opt_str(&info.encryption),
    )
}

/// Escape special characters for JSON
fn escape_json(s: &str) -> String {
    let result = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    // Escape remaining control characters (0x00-0x1F)
    result
        .chars()
        .map(|c| {
            if c.is_control() {
                format!("\\u{:04x}", c as u32)
            } else {
                c.to_string()
            }
        })
        .collect()
}

/// Escape markdown table cell delimiters and line breaks.
fn escape_markdown_cell(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\r', "")
        .replace('\n', "<br>")
}

/// Get the user's Downloads directory (cross-platform)
fn downloads_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            let dir = PathBuf::from(profile).join("Downloads");
            if dir.is_dir() {
                return dir;
            }
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let dir = PathBuf::from(home).join("Downloads");
            if dir.is_dir() {
                return dir;
            }
        }
    }

    // Fallback: current working directory
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Generate a comprehensive markdown report from system info
fn generate_markdown(info: &SystemInfo) -> String {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
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
    md.push('\n');

    // Network section
    md.push_str("## Network\n\n");
    md.push_str("| Field | Value |\n|-------|-------|\n");
    md.push_str(&format!("| Hostname | {} |\n", cell(&info.hostname)));
    if let Some(ref ip) = info.machine_ip {
        md.push_str(&format!("| Machine IP | {} |\n", cell(ip)));
    }
    md.push_str(&format!(
        "| Client IP | {} |\n",
        cell(info.client_ip.as_deref().unwrap_or("Not connected"))
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
    md.push_str(&format!("| Frequency | {} |\n", info.freq_str()));
    if let (Some(l1), Some(l5), Some(l15)) = (info.load_1m, info.load_5m, info.load_15m) {
        md.push_str(&format!("| Load 1m | {:.2}% |\n", l1));
        md.push_str(&format!("| Load 5m | {:.2}% |\n", l5));
        md.push_str(&format!("| Load 15m | {:.2}% |\n", l15));
    }
    md.push('\n');

    // Storage section
    md.push_str("## Storage\n\n");
    md.push_str("| Field | Value |\n|-------|-------|\n");
    md.push_str(&format!("| Volume | {} |\n", info.disk_usage_str()));
    md.push_str(&format!("| Disk Usage | {:.2}% |\n", info.disk_percent));
    if let Some(ref zfs_health) = info.zfs_health {
        md.push_str(&format!("| ZFS Health | {} |\n", cell(zfs_health)));
    }
    md.push('\n');

    // Memory section
    md.push_str("## Memory\n\n");
    md.push_str("| Field | Value |\n|-------|-------|\n");
    md.push_str(&format!("| Memory | {} |\n", info.memory_usage_str()));
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
    }
    md.push_str(&format!("| Uptime | {} |\n", info.uptime_formatted()));
    if let Some(ref shell) = info.shell {
        md.push_str(&format!("| Shell | {} |\n", cell(shell)));
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

    md.push_str("\n---\n\n");
    md.push_str(&format!("*Generated by TR-300 v{}*\n", version));

    md
}

/// Save a markdown report to the Downloads folder.
/// Returns the file path on success, or None on failure.
pub fn save_markdown_report(info: &SystemInfo) -> Option<PathBuf> {
    let dir = downloads_dir();
    let _ = std::fs::create_dir_all(&dir);

    let filename_ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let filename = format!("tr300-report-{}.md", filename_ts);
    let path = dir.join(filename);

    let markdown = generate_markdown(info);

    match std::fs::write(&path, markdown) {
        Ok(()) => Some(path),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elevation_footer_skipped_when_elevated() {
        // Already running as root/admin — nothing more to unlock; no hint.
        assert!(!should_render_elevation_footer(
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
            CollectMode::Fast,
            false
        ));
    }

    #[test]
    fn elevation_footer_skipped_when_user_opted_out() {
        // --no-elevation-hint suppresses the line for users who find it noisy.
        assert!(!should_render_elevation_footer(
            false,
            CollectMode::Full,
            true
        ));
    }

    #[test]
    fn elevation_footer_present_when_unelevated_full_no_optout() {
        // Per platform: Linux+Windows have elevated-only data; macOS does not.
        let expected = cfg!(target_os = "linux") || cfg!(target_os = "windows");
        assert_eq!(
            should_render_elevation_footer(false, CollectMode::Full, false),
            expected
        );
    }

    #[test]
    fn elevation_footer_string_is_empty_on_macos() {
        // platform_has_elevated_data() == false on macOS, so the renderer
        // returns an empty string even if the gate was somehow bypassed.
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
            hostname: "host".to_string(),
            machine_ip: Some("192.0.2.10".to_string()),
            client_ip: None,
            dns_servers: vec!["1.1.1.1".to_string()],
            username: "user".to_string(),
            processor: "CPU".to_string(),
            cores: 8,
            sockets: Some(1),
            cpu_core_topology: Some("4P + 4E".to_string()),
            hypervisor: Some("Bare Metal".to_string()),
            cpu_freq_ghz: 3.2,
            load_1m: Some(10.0),
            load_5m: Some(20.0),
            load_15m: Some(30.0),
            gpus: vec!["GPU".to_string()],
            disk_used_bytes: 1,
            disk_total_bytes: 2,
            disk_percent: 50.0,
            zfs_health: Some("ONLINE".to_string()),
            mem_used_bytes: 1,
            mem_total_bytes: 2,
            mem_percent: 50.0,
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
            encryption: None,
            mode: CollectMode::Full,
            is_elevated: true,
        }
    }

    #[test]
    fn json_includes_new_nullable_platform_keys_without_schema_bump() {
        let json = generate_json(&fixture_info());
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["os"]["machine_model"], "Model | One");
        assert_eq!(value["cpu"]["core_topology"], "4P + 4E");
        assert_eq!(value["memory"]["ram_slots"], "2x16GB | DDR5");
        assert_eq!(value["system"]["motherboard"], "Board | Vendor");
        assert_eq!(value["system"]["bios"], "BIOS | 1.2");
    }

    #[test]
    fn markdown_escapes_table_cell_pipes() {
        let markdown = generate_markdown(&fixture_info());
        assert!(markdown.contains("| Machine Model | Model \\| One |"));
        assert!(markdown.contains("| Motherboard | Board \\| Vendor |"));
        assert!(markdown.contains("| RAM Slots | 2x16GB \\| DDR5 |"));
    }
}
