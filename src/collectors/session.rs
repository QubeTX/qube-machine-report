//! User session information collector

#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::collectors::command::{run_output_with_env, CommandTimeout};
use crate::collectors::CollectMode;
use crate::error::Result;
use std::env;

/// Session/user information
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Current username
    pub username: String,
    /// User's home directory
    pub home_dir: String,
    /// Current shell (Unix) or COMSPEC (Windows)
    pub shell: String,
    /// Current working directory
    pub current_dir: String,
    /// Terminal type
    pub terminal: String,
    /// Last login time (None if skipped in fast mode)
    pub last_login: Option<String>,
    /// Last login IP (if available)
    pub last_login_ip: Option<String>,
}

/// Collect session information
pub fn collect(mode: CollectMode) -> Result<SessionInfo> {
    let username = get_username();
    let home_dir = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let shell = get_shell();
    let current_dir = env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "Unknown".to_string());

    let terminal = get_terminal();

    let should_skip_last_login = mode == CollectMode::Fast && should_skip_last_login_on_platform();
    let (last_login, last_login_ip) = if should_skip_last_login {
        (None, None)
    } else {
        get_last_login(&username)
    };

    Ok(SessionInfo {
        username,
        home_dir,
        shell,
        current_dir,
        terminal,
        last_login,
        last_login_ip,
    })
}

/// Whether to skip last_login in fast mode on this platform.
/// Windows uses PowerShell (slow). Linux/macOS use fast commands.
fn should_skip_last_login_on_platform() -> bool {
    #[cfg(target_os = "windows")]
    {
        true
    }

    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Get current username
fn get_username() -> String {
    #[cfg(unix)]
    {
        uzers::get_current_username()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| env::var("USER").unwrap_or_else(|_| "Unknown".to_string()))
    }

    #[cfg(windows)]
    {
        #[link(name = "advapi32")]
        extern "system" {
            fn GetUserNameW(buffer: *mut u16, size: *mut u32) -> i32;
        }
        let mut buffer = [0u16; 257];
        let mut size = buffer.len() as u32;
        // SAFETY: the buffer holds `size` UTF-16 code units and the API updates
        // `size` to include the terminating NUL on success.
        if unsafe { GetUserNameW(buffer.as_mut_ptr(), &mut size) } != 0 && size > 1 {
            return String::from_utf16_lossy(&buffer[..size as usize - 1]);
        }
        env::var("USERNAME").unwrap_or_else(|_| "Unknown".to_string())
    }

    #[cfg(not(any(unix, windows)))]
    {
        "Unknown".to_string()
    }
}

/// Get current shell
fn get_shell() -> String {
    #[cfg(unix)]
    {
        env::var("SHELL").unwrap_or_else(|_| "Unknown".to_string())
    }

    #[cfg(windows)]
    {
        "Unknown".to_string()
    }

    #[cfg(not(any(unix, windows)))]
    {
        "Unknown".to_string()
    }
}

/// Get terminal type
fn get_terminal() -> String {
    // Check common terminal environment variables
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        return term_program;
    }

    if let Ok(wt_session) = env::var("WT_SESSION") {
        if !wt_session.is_empty() {
            return "Windows Terminal".to_string();
        }
    }

    #[cfg(not(windows))]
    if let Ok(term) = env::var("TERM") {
        if !term.is_empty() && term != "dumb" {
            return term;
        }
    }

    #[cfg(windows)]
    {
        if env::var("ConEmuANSI").is_ok() {
            return "ConEmu".to_string();
        }
        "Unknown".to_string()
    }

    #[cfg(not(windows))]
    {
        "Unknown".to_string()
    }
}

/// Get last login information.
///
/// Returns `(None, _)` when login tracking is genuinely unavailable (no
/// `lastlog`/`last`/WTS data, parse failure, unsupported platform) so the
/// caller can omit the row / emit JSON `null` — distinct from a real
/// `Some("Never logged in")` value, which IS a meaningful answer.
fn get_last_login(username: &str) -> (Option<String>, Option<String>) {
    #[cfg(target_os = "linux")]
    {
        get_last_login_linux(username)
    }

    #[cfg(target_os = "macos")]
    {
        get_last_login_macos(username)
    }

    #[cfg(target_os = "windows")]
    {
        get_last_login_windows(username)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = username;
        (None, None)
    }
}

#[cfg(target_os = "linux")]
fn get_last_login_linux(username: &str) -> (Option<String>, Option<String>) {
    // Force LC_ALL=C on these `lastlog*` / `last` calls so the "Never
    // logged in" string match and the column-position parsers don't
    // misfire on non-English locales (where the label becomes e.g.
    // "Nie eingeloggt" in German). (audit finding F19, v3.15.8+)
    let c_locale = || [("LC_ALL", "C")];

    // Try lastlog2 first (newer systems)
    if let Some(output) = run_output_with_env(
        "lastlog2",
        ["--user", username],
        c_locale(),
        CommandTimeout::Normal,
    ) {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut lines = stdout.lines();
            if let (Some(header), Some(line)) = (lines.next(), lines.next()) {
                if let Some(parsed) = parse_lastlog_record(header, line) {
                    return parsed;
                }
            }
        }
    }

    // Try lastlog (older systems)
    if let Some(output) = run_output_with_env(
        "lastlog",
        ["-u", username],
        c_locale(),
        CommandTimeout::Normal,
    ) {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut lines = stdout.lines();
            if let (Some(header), Some(line)) = (lines.next(), lines.next()) {
                if let Some(parsed) = parse_lastlog_record(header, line) {
                    return parsed;
                }
            }
        }
    }

    // Try last command (also LC_ALL=C — `last`'s output is localized
    // on some distros and our parser keys off positional columns).
    if let Some(output) = run_output_with_env(
        "last",
        ["-F", "-1", "-w", username],
        c_locale(),
        CommandTimeout::Normal,
    ) {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().next() {
                if !line.contains("wtmp begins") && !line.is_empty() {
                    if let Some(parsed) = parse_last_record(line) {
                        return parsed;
                    }
                }
            }
        }
    }

    (None, None)
}

#[cfg(target_os = "macos")]
fn get_last_login_macos(username: &str) -> (Option<String>, Option<String>) {
    // Force stable English weekday/month tokens for the fallback parser.
    if let Some(output) = run_output_with_env(
        "last",
        ["-1", username],
        [("LC_ALL", "C")],
        CommandTimeout::Normal,
    ) {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().next() {
                if !line.contains("wtmp begins") && !line.is_empty() {
                    if let Some(parsed) = parse_last_record(line) {
                        return parsed;
                    }
                }
            }
        }
    }

    (None, None)
}

/// Parse fixed-column `lastlog`/`lastlog2` output by using the header's
/// column starts. Whitespace token positions are not stable when `From` is
/// empty, and `lastlog2` emits a full multi-token date.
#[cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]
fn parse_lastlog_record(header: &str, line: &str) -> Option<(Option<String>, Option<String>)> {
    if line.contains("Never logged in") {
        return Some((Some("Never logged in".to_string()), None));
    }
    let latest_start = header.find("Latest")?;
    let date = line.get(latest_start..)?.trim();
    if date.is_empty() {
        return None;
    }
    let origin = header.find("From").and_then(|from_start| {
        (from_start < latest_start)
            .then(|| line.get(from_start..latest_start).unwrap_or("").trim())
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    });
    Some((Some(date.to_string()), origin))
}

/// Parse C-locale `last` output by locating the weekday token. Local console
/// records omit the origin column, so fixed whitespace indexes are unsafe.
#[cfg(any(unix, test))]
fn parse_last_record(line: &str) -> Option<(Option<String>, Option<String>)> {
    const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let fields: Vec<&str> = line.split_whitespace().collect();
    let day_index = fields.iter().position(|field| WEEKDAYS.contains(field))?;
    if day_index < 2 {
        return None;
    }
    let origin = (day_index > 2)
        .then(|| fields[2..day_index].join(" "))
        .filter(|value| !value.is_empty());
    let date_fields: Vec<&str> = fields[day_index..]
        .iter()
        .copied()
        .take_while(|field| *field != "-" && *field != "still" && *field != "gone")
        .take(5)
        .collect();
    (!date_fields.is_empty()).then(|| (Some(date_fields.join(" ")), origin))
}

#[cfg(target_os = "windows")]
fn get_last_login_windows(_username: &str) -> (Option<String>, Option<String>) {
    let when = wts_query_session_logon_time().and_then(filetime_to_local_string);
    let origin = wts_query_client_address();
    (when, origin)
}

// `WTSQuerySessionInformationW` and friends are declared manually because
// `winapi-rs`'s `wtsapi32` feature does not expose them on stable. See:
// https://learn.microsoft.com/en-us/windows/win32/api/wtsapi32/nf-wtsapi32-wtsquerysessioninformationw
#[cfg(target_os = "windows")]
const WTS_CURRENT_SESSION: u32 = 0xFFFF_FFFF;
#[cfg(target_os = "windows")]
const WTS_CLIENT_ADDRESS_CLASS: u32 = 14;
#[cfg(target_os = "windows")]
const WTS_SESSION_INFO_CLASS: u32 = 24;

#[cfg(target_os = "windows")]
#[link(name = "wtsapi32")]
extern "system" {
    fn WTSQuerySessionInformationW(
        hServer: *mut std::ffi::c_void,
        SessionId: u32,
        WTSInfoClass: u32,
        ppBuffer: *mut *mut u16,
        pBytesReturned: *mut u32,
    ) -> i32;
    fn WTSFreeMemory(pMemory: *mut std::ffi::c_void);
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Clone, Copy)]
struct WtsInfoW {
    state: i32,
    session_id: u32,
    incoming_bytes: u32,
    outgoing_bytes: u32,
    incoming_frames: u32,
    outgoing_frames: u32,
    incoming_compressed_bytes: u32,
    outgoing_compressed_bytes: u32,
    win_station_name: [u16; 32],
    domain: [u16; 17],
    user_name: [u16; 21],
    connect_time: i64,
    disconnect_time: i64,
    last_input_time: i64,
    logon_time: i64,
    current_time: i64,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Clone, Copy)]
struct WtsClientAddress {
    address_family: u32,
    address: [u8; 20],
}

#[cfg(target_os = "windows")]
fn wts_query_bytes(info_class: u32) -> Option<Vec<u8>> {
    let mut buffer_ptr: *mut u16 = std::ptr::null_mut();
    let mut bytes_returned: u32 = 0;

    // SAFETY: WTSQuerySessionInformationW returns an allocated byte buffer for
    // the requested class. We copy exactly `bytes_returned` bytes and always
    // release it with WTSFreeMemory. NULL means the current server handle.
    let ok = unsafe {
        WTSQuerySessionInformationW(
            std::ptr::null_mut(),
            WTS_CURRENT_SESSION,
            info_class,
            &mut buffer_ptr,
            &mut bytes_returned,
        )
    };

    if ok == 0 || buffer_ptr.is_null() || bytes_returned == 0 {
        if !buffer_ptr.is_null() {
            unsafe { WTSFreeMemory(buffer_ptr as *mut _) };
        }
        return None;
    }

    let mut bytes = vec![0u8; bytes_returned as usize];
    unsafe {
        std::ptr::copy_nonoverlapping(buffer_ptr as *const u8, bytes.as_mut_ptr(), bytes.len());
        WTSFreeMemory(buffer_ptr as *mut _);
    }
    Some(bytes)
}

#[cfg(target_os = "windows")]
fn wts_query_session_logon_time() -> Option<i64> {
    let bytes = wts_query_bytes(WTS_SESSION_INFO_CLASS)?;
    if bytes.len() < std::mem::size_of::<WtsInfoW>() {
        return None;
    }
    // WTS owns the original allocation; our copied byte vector has alignment
    // 1, so use an unaligned read into the exact repr(C) structure.
    let info = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const WtsInfoW) };
    (info.logon_time > 0).then_some(info.logon_time)
}

#[cfg(target_os = "windows")]
fn wts_query_client_address() -> Option<String> {
    let bytes = wts_query_bytes(WTS_CLIENT_ADDRESS_CLASS)?;
    if bytes.len() < std::mem::size_of::<WtsClientAddress>() {
        return None;
    }
    let address = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const WtsClientAddress) };
    parse_wts_client_address(address.address_family, &address.address)
}

#[cfg(any(target_os = "windows", test))]
fn parse_wts_client_address(address_family: u32, bytes: &[u8; 20]) -> Option<String> {
    const AF_INET: u32 = 2;
    const AF_INET6: u32 = 23;
    match address_family {
        // Microsoft stores the IPv4 address in bytes 2..6 of the 20-byte WTS
        // address payload (the leading bytes are the port field).
        AF_INET => {
            let address = std::net::Ipv4Addr::new(bytes[2], bytes[3], bytes[4], bytes[5]);
            (!address.is_unspecified() && !address.is_loopback()).then(|| address.to_string())
        }
        AF_INET6 => {
            let octets: [u8; 16] = bytes[0..16].try_into().ok()?;
            let address = std::net::Ipv6Addr::from(octets);
            (!address.is_unspecified() && !address.is_loopback()).then(|| address.to_string())
        }
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn filetime_to_local_string(filetime: i64) -> Option<String> {
    // Convert FILETIME → Unix timestamp:
    //   100-ns intervals → seconds, then subtract the 1601→1970 offset
    //   (11_644_473_600 seconds).
    const FILETIME_UNIX_EPOCH_DIFF_SECS: i64 = 11_644_473_600;
    let unix_secs = filetime / 10_000_000 - FILETIME_UNIX_EPOCH_DIFF_SECS;
    let unix_nsecs = ((filetime % 10_000_000) * 100) as u32;

    let dt_utc = chrono::DateTime::<chrono::Utc>::from_timestamp(unix_secs, unix_nsecs)?;
    let dt_local = dt_utc.with_timezone(&chrono::Local);
    Some(dt_local.format("%a %b %-d %H:%M").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_column_lastlog_parser_preserves_origin_and_full_date() {
        let header = format!("{:<16}{:<8}{:<20}{}", "Username", "Port", "From", "Latest");
        let remote = format!(
            "{:<16}{:<8}{:<20}{}",
            "alice", "pts/0", "192.0.2.44", "Mon Jul 13 22:59:00 +0000 2026"
        );
        assert_eq!(
            parse_lastlog_record(&header, &remote),
            Some((
                Some("Mon Jul 13 22:59:00 +0000 2026".to_string()),
                Some("192.0.2.44".to_string())
            ))
        );

        let local = format!(
            "{:<16}{:<8}{:<20}{}",
            "alice", "tty1", "", "Mon Jul 13 20:00:00 +0000 2026"
        );
        assert_eq!(
            parse_lastlog_record(&header, &local),
            Some((Some("Mon Jul 13 20:00:00 +0000 2026".to_string()), None))
        );
    }

    #[test]
    fn last_parser_does_not_turn_weekday_into_an_origin() {
        assert_eq!(
            parse_last_record("alice console Mon Jul 13 22:59 still logged in"),
            Some((Some("Mon Jul 13 22:59".to_string()), None))
        );
        assert_eq!(
            parse_last_record("alice ttys001 2001:db8::4 Mon Jul 13 22:59 still logged in"),
            Some((
                Some("Mon Jul 13 22:59".to_string()),
                Some("2001:db8::4".to_string())
            ))
        );
    }

    #[test]
    fn lastlog_parser_preserves_never_logged_in() {
        assert_eq!(
            parse_lastlog_record("Username Port From Latest", "alice **Never logged in**"),
            Some((Some("Never logged in".to_string()), None))
        );
    }

    #[test]
    fn wts_client_address_parser_honors_windows_layouts() {
        let mut ipv4 = [0u8; 20];
        ipv4[2..6].copy_from_slice(&[192, 0, 2, 44]);
        assert_eq!(
            parse_wts_client_address(2, &ipv4).as_deref(),
            Some("192.0.2.44")
        );

        let mut ipv6 = [0u8; 20];
        ipv6[..16].copy_from_slice(&[0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4]);
        assert_eq!(
            parse_wts_client_address(23, &ipv6).as_deref(),
            Some("2001:db8::4")
        );
        assert_eq!(parse_wts_client_address(0, &[0; 20]), None);
        assert_eq!(parse_wts_client_address(2, &[0; 20]), None);
    }
}
