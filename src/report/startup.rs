use std::net::IpAddr;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::{visit_table_rows, Config, SystemInfo, TableRenderer};

#[derive(Default)]
struct Session {
    remote: bool,
    server: Option<IpAddr>,
    client: Option<IpAddr>,
    alternate_name: Option<String>,
}

fn port(s: &str) -> bool {
    s.parse::<u16>().is_ok_and(|p| p != 0)
}

impl Session {
    fn detect(connection: Option<&str>, client: Option<&str>, tty: Option<&str>) -> Self {
        let connection = connection
            .unwrap_or_default()
            .split_ascii_whitespace()
            .collect::<Vec<_>>();
        let endpoints = match connection.as_slice() {
            [peer, from, host, to] if port(from) && port(to) => {
                peer.parse().ok().zip(host.parse().ok())
            }
            _ => None,
        };
        if let Some((peer, host)) = endpoints {
            return Self {
                remote: true,
                client: Some(peer),
                server: Some(host),
                alternate_name: None,
            };
        }
        let client = client
            .unwrap_or_default()
            .split_ascii_whitespace()
            .collect::<Vec<_>>();
        let peer = match client.as_slice() {
            [peer, from, to] if port(from) && port(to) => peer.parse().ok(),
            _ => None,
        };
        Self {
            remote: peer.is_some()
                || tty.is_some_and(|t| {
                    t.strip_prefix("/dev/")
                        .is_some_and(|p| !p.is_empty() && !p.chars().any(char::is_whitespace))
                }),
            client: peer,
            server: None,
            alternate_name: None,
        }
    }
}

/// Render the shell profile's fast report within the visible terminal.
pub fn generate_startup(
    info: &SystemInfo,
    config: &Config,
    viewport: Option<(u16, u16)>,
) -> String {
    let env = |name| std::env::var(name).ok();
    let mut session = Session::detect(
        env("SSH_CONNECTION").as_deref(),
        env("SSH_CLIENT").as_deref(),
        env("SSH_TTY").as_deref(),
    );
    let alternate = if cfg!(windows) {
        "COMPUTERNAME"
    } else {
        "HOSTNAME"
    };
    session.alternate_name = env(alternate)
        .filter(|value| !value.trim().is_empty() && !value.eq_ignore_ascii_case(&info.hostname));
    render(info, config, viewport, &session)
}

// A stable sort keeps numbered GPUs, DNS servers, and remaining fields in source order.
fn priority(label: &str, remote: bool) -> usize {
    if remote {
        let lead = [
            "HOSTNAME",
            "SERVER IP",
            "DEFAULT IP",
            "BATTERY",
            "MODEL",
            "OS",
            "PROCESSOR",
            "CORES",
            "MEMORY",
            "VOLUME",
            "UPTIME",
        ];
        if let Some(i) = lead.iter().position(|&l| l == label) {
            return i;
        }
    }
    20 + match label {
        "PROCESSOR" => 0,
        "CORES" => 1,
        "MEMORY" => 2,
        "VOLUME" => 3,
        "MODEL" => 4,
        "OS" => 5,
        l if l.starts_with("GPU") && l != "GPU TEMP" => 6,
        "BATTERY" => 7,
        "UPTIME" => 8,
        "CPU USAGE" => 9,
        "CPU TEMP" | "GPU TEMP" => 10,
        "HOSTNAME" => 11,
        "DEFAULT IP" => 12,
        "SSH CLIENT" => 13,
        "CORE TYPE" | "RAM SLOTS" | "BOARD" | "BIOS" | "ARCH" => 14,
        _ => 15,
    }
}

// Select by importance first; group the selected local facts for reading.
fn section(label: &str) -> u8 {
    match label {
        "MEMORY" | "VOLUME" | "AVAILABLE" | "SWAP" | "CPU USAGE" | "CPU TEMP" | "GPU TEMP"
        | "ZFS HEALTH" | "UPTIME" => 1,
        l if l.starts_with("LOAD/CPU") => 1,
        "MACHINE NAME" | "HOSTNAME" | "SERVER IP" | "DEFAULT IP" | "SSH CLIENT" | "USER" => 2,
        l if l.starts_with("DNS ") => 2,
        "LAST LOGIN" | "LOGIN ORIGIN" | "LOGIN SHELL" | "TERMINAL" | "LOCALE" | "ENCRYPTION" => 3,
        _ => 0,
    }
}

// Cap both the per-character and string width. Some terminal emulators display
// emoji sequences as several glyphs while others combine them.
fn fit(value: &str, width: usize) -> String {
    let mut used = 0;
    let mut result = String::new();
    for ch in value.chars() {
        let ch = if ch.is_control()
            || matches!(ch, '\u{2028}' | '\u{2029}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        {
            ' '
        } else {
            ch
        };
        used += ch.width().unwrap_or(0);
        if used > width {
            return clipped(result, width);
        }
        result.push(ch);
        if result.width() > width {
            result.pop();
            return clipped(result, width);
        }
    }
    result
}

fn clipped(mut prefix: String, width: usize) -> String {
    let dots = ".".repeat(width.min(3));
    while prefix.width() + dots.len() > width {
        prefix.pop();
    }
    prefix.push_str(&dots);
    prefix
}

fn render(
    info: &SystemInfo,
    config: &Config,
    viewport: Option<(u16, u16)>,
    session: &Session,
) -> String {
    let Some((cols, lines)) = viewport else {
        return format!("{}\n", fit(&info.hostname, 24));
    };
    if cols < 12 || lines <= 4 {
        return String::new();
    }
    let available = usize::from(cols - 1);
    let budget = usize::from(lines - 4);
    let mut facts = Vec::new();
    visit_table_rows(info, config, |entry| {
        let Some((label, value)) = entry else { return };
        // Disk and memory totals already include percentages. Keep unrelated
        // default-route and SSH endpoint facts distinct when they differ.
        if matches!(label, "DISK USAGE" | "USAGE" | "SSH CLIENT" | "HOSTNAME")
            || (label == "DEFAULT IP"
                && session.server.is_some()
                && value.parse::<IpAddr>().ok() == session.server)
        {
            return;
        }
        let value = if label == "CPU USAGE" || label.starts_with("LOAD/CPU") {
            value.split_whitespace().last().unwrap_or(value)
        } else {
            value
        };
        if !value.trim().is_empty() {
            facts.push((label.to_owned(), value.to_owned()));
        }
    });
    if let Some(name) = &session.alternate_name {
        facts.push(("MACHINE NAME".to_owned(), name.clone()));
    }
    if let Some(ip) = session.server {
        facts.push(("SERVER IP".to_owned(), ip.to_string()));
    }
    if let Some(ip) = session.client {
        facts.push(("SSH CLIENT".to_owned(), ip.to_string()));
    }
    facts.sort_by_key(|(l, _)| {
        // The default route may differ from the actual SSH endpoint.
        if l == "DEFAULT IP" && session.server.is_some() {
            32
        } else {
            priority(l, session.remote)
        }
    });

    facts.truncate(budget - 1);
    if !session.remote {
        facts.sort_by_key(|(label, _)| section(label));
    }
    let data_width = facts
        .iter()
        .map(|(_, value)| value.width())
        .max()
        .unwrap_or(32)
        .clamp(32, 64);
    let width = available.min(19 + data_width);
    // Spend spare rows on a closed frame only after selecting useful facts.
    // The hostname itself always costs one row.
    let spare = budget - facts.len() - 1;
    let framed = width >= 38 && spare >= 2;
    let full_header = framed && spare >= 6;
    let renderer = TableRenderer::new(12, width.saturating_sub(19), config.box_chars());
    let section_changes = |a: &str, b: &str| !session.remote && section(a) != section(b);
    let dividers = facts
        .windows(2)
        .filter(|pair| section_changes(&pair[0].0, &pair[1].0))
        .count();
    let divided = full_header && spare >= 6 + dividers;
    let mut output = String::new();
    let heading_width = if framed { width - 2 } else { width };
    let branded = format!("TR-300 | {}", info.hostname);
    let heading = if !full_header && width >= 32 && budget >= 5 && branded.width() <= heading_width
    {
        fit(&branded, heading_width)
    } else {
        fit(&info.hostname, heading_width)
    };
    if full_header {
        output.push_str(&renderer.render_top_header());
        output.push_str(&renderer.render_header_bottom());
        output.push_str(&renderer.render_centered(&fit(config.title(), width - 2)));
        output.push_str(&renderer.render_centered(&fit(config.subtitle(), width - 2)));
        output.push_str(&renderer.render_centered(&heading));
        output.push_str(&renderer.render_top_divider());
    } else if framed {
        let chars = config.box_chars();
        output.push(chars.top_left);
        output.extend(std::iter::repeat_n(chars.horizontal, width - 2));
        output.push(chars.top_right);
        output.push('\n');
        output.push_str(&renderer.render_centered(&heading));
        if spare >= 3 {
            output.push_str(&renderer.render_top_divider());
        }
    } else {
        output.push_str(&heading);
        output.push('\n');
    }
    let label_width = if width >= 26 {
        facts
            .iter()
            .map(|(label, _)| label.len())
            .max()
            .unwrap_or(0)
    } else {
        0
    };
    let mut previous = None;
    for (label, value) in &facts {
        if divided && previous.is_some_and(|p| section_changes(p, label)) {
            output.push_str(&renderer.render_middle_divider());
        }
        previous = Some(label.as_str());
        if framed {
            output.push_str(&renderer.render_row(label, &fit(value, width - 19)));
        } else {
            output.push_str(&fit(&format!("{label:label_width$} {value}"), width));
            output.push('\n');
        }
    }
    if framed {
        output.push_str(&renderer.render_footer());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::tests::fixture_info;

    #[test]
    fn remote_endpoint_and_priority() {
        let mut info = fixture_info();
        info.battery = Some("63% (Discharging)".into());
        let session = Session::detect(Some("2001:db8::1 50100 2001:db8::2 22"), None, None);
        let output = render(&info, &Config::default(), Some((80, 7)), &session);
        assert_eq!(
            output.lines().collect::<Vec<_>>(),
            [
                "host",
                "SERVER IP 2001:db8::2",
                "BATTERY   63% (Discharging)"
            ]
        );
        let output = render(&info, &Config::default(), Some((80, 100)), &session);
        for value in [
            "SSH CLIENT",
            "2001:db8::1",
            "DEFAULT IP",
            "192.0.2.10",
            "ZFS HEALTH",
            "ENCRYPTION",
        ] {
            assert!(output.contains(value), "{value}");
        }
    }

    #[test]
    fn local_heading_and_resource_priorities() {
        let info = fixture_info();
        for geometry in [(22, 9), (80, 25), (100, 100)] {
            let output = render(
                &info,
                &Config::default(),
                Some(geometry),
                &Session::default(),
            );
            assert_eq!(output.matches("host").count(), 1);
            for field in ["PROCESSOR", "CORES", "MEMORY", "VOLUME"] {
                assert!(output.contains(field), "{field}: {output}");
            }
            if geometry.1 == 9 {
                assert!(!output.contains("GPU"));
            }
        }
    }

    #[test]
    fn compact_layout_examples() {
        let mut info = fixture_info();
        info.hostname = "SHAGHV-PC".into();
        let config = Config::default();
        let session = Session::default();
        for (cols, rows) in [(120, 30), (80, 25), (22, 9)] {
            let out = render(&info, &config, Some((cols, rows)), &session);
            let lines: Vec<_> = out.lines().collect();
            let heading = if cols < 32 {
                "SHAGHV-PC"
            } else {
                "TR-300 | SHAGHV-PC"
            };
            assert_eq!(lines[0], heading);
            assert!(lines
                .iter()
                .all(|line| !line.starts_with('│') && line.width() < usize::from(cols)));
            assert!(lines.len() <= usize::from(rows - 4));
        }

        // A typical fast sample fits all fields in 120x30 with spare rows for
        // a compact heading and a complete frame, but not the tall title block.
        info.os_edition = None;
        info.os_codename = None;
        info.os_build = None;
        info.last_login = None;
        info.bios = None;
        info.motherboard = None;
        info.shell = None;
        info.locale = None;
        info.encryption = None;
        info.ram_slots = None;
        info.display_resolution = None;
        info.terminal = None;
        info.zfs_health = None;
        info.load_1m = None;
        info.boot_mode = None;
        info.hypervisor = None;
        let out = render(&info, &config, Some((120, 30)), &session);
        assert!(
            out.starts_with('┌') && out.lines().last().unwrap().starts_with('└'),
            "{out}"
        );
        assert!(out.contains("SHAGHV-PC") && out.contains("TR-300"));
        assert!(out.lines().count() <= 26);
    }

    #[test]
    fn validates_ssh_hints_independently() {
        for bad in [
            "x 42 192.0.2.1 22",
            "192.0.2.2 0 192.0.2.1 22",
            "192.0.2.2 42 192.0.2.1 65536",
            "192.0.2.2 42 192.0.2.1 22 trailing",
        ] {
            assert!(!Session::detect(Some(bad), None, None).remote);
            let fallback = Session::detect(Some(bad), Some("192.0.2.3 1001 22"), None);
            assert!(fallback.remote && fallback.server.is_none());
        }
        assert!(Session::detect(None, None, Some("/dev/pts/2")).remote);
        assert!(!Session::detect(None, None, Some(" ")).remote);
    }

    #[test]
    fn reserves_prompt_and_wrap_column_for_untrusted_values() {
        let mut info = fixture_info();
        info.hostname = "ホスト\r\n\x1b[2J".repeat(8);
        info.machine_model = Some("界\t🧑‍💻\u{2028}".repeat(20));
        let session = Session::default();
        for cols in [1, 11, 12, 20, 38, 39, 51, 52, 120] {
            for rows in [2, 4, 5, 7, 12, 24, 64, 100] {
                let output = render(&info, &Config::default(), Some((cols, rows)), &session);
                assert!(output.lines().count() <= usize::from(rows.saturating_sub(4)));
                assert!(output
                    .lines()
                    .all(|l| l.width() < usize::from(cols) && !l.chars().any(char::is_control)));
            }
        }
        let fallback = render(&info, &Config::default(), None, &session);
        assert_eq!(fallback.lines().count(), 1);
        assert!(fallback.trim_end().width() <= 24);
    }
}
