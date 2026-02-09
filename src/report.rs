//! Report generation matching TR-200 format
//!
//! Generates the complete system report using the exact
//! layout and styling of TR-200 Machine Report.

use crate::collectors::SystemInfo;
use crate::config::{Config, OutputFormat, MAX_DATA_WIDTH};
use crate::render::bar::render_bar;
use crate::render::table::TableRenderer;

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

    // Simplified footer (single line, no bottom_divider)
    output.push_str(&renderer.render_footer());

    output
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

    // Simple JSON serialization without serde
    format!(
        r#"{{
  "os": {{
    "name": "{}",
    "version": "{}",
    "kernel": "{}",
    "architecture": "{}"
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
    "percent": {:.2}
  }},
  "session": {{
    "username": "{}",
    "last_login": {},
    "uptime_seconds": {},
    "shell": {},
    "terminal": {},
    "locale": {},
    "battery": {}
  }}
}}"#,
        escape_json(&info.os_name),
        escape_json(&info.os_version),
        escape_json(&info.kernel),
        escape_json(&info.architecture),
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
        escape_json(&info.username),
        opt_str(&info.last_login),
        info.uptime_seconds,
        opt_str(&info.shell),
        opt_str(&info.terminal),
        opt_str(&info.locale),
        opt_str(&info.battery),
    )
}

/// Escape special characters for JSON
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
