//! User session information collector

#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::collectors::command::CommandTimeout;
// run_output is only used by the macOS `last` fallback; the Linux
// `lastlog` / `lastlog2` / `last` calls were migrated to
// `run_output_with_env` (v3.15.2 audit finding F19) to force
// `LC_ALL=C` so localized labels don't break the parsers.
#[cfg(target_os = "macos")]
use crate::collectors::command::run_output;
use crate::collectors::CollectMode;
use crate::error::Result;
use std::env;

/// Session/user information
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
        let (login, ip) = get_last_login(&username);
        (Some(login), ip)
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
        env::var("COMSPEC").unwrap_or_else(|_| {
            env::var("PSModulePath")
                .map(|_| "PowerShell".to_string())
                .unwrap_or_else(|_| "cmd.exe".to_string())
        })
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

    if let Ok(term) = env::var("TERM") {
        return term;
    }

    #[cfg(windows)]
    {
        if env::var("ConEmuANSI").is_ok() {
            return "ConEmu".to_string();
        }
        "Console".to_string()
    }

    #[cfg(not(windows))]
    {
        "Unknown".to_string()
    }
}

/// Get last login information
fn get_last_login(username: &str) -> (String, Option<String>) {
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
        ("Login tracking unavailable".to_string(), None)
    }
}

#[cfg(target_os = "linux")]
fn get_last_login_linux(username: &str) -> (String, Option<String>) {
    use crate::collectors::command::run_output_with_env;

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
            if let Some(line) = stdout.lines().nth(1) {
                // Parse lastlog2 output
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let date_time = parts[1..4].join(" ");
                    let ip = parts.get(4).map(|s| s.to_string());
                    return (date_time, ip);
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
            if let Some(line) = stdout.lines().nth(1) {
                if line.contains("Never logged in") {
                    return ("Never logged in".to_string(), None);
                }
                // Parse lastlog output
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    // Format: Username Port From Latest
                    let from = parts.get(2).map(|s| s.to_string());
                    let date_time = parts[3..].join(" ");
                    return (date_time, from);
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
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        // Format: user tty from day mon date time
                        let from = parts.get(2).and_then(|s| {
                            if s.starts_with(':') || s.starts_with("pts") || s.starts_with("tty") {
                                None
                            } else {
                                Some(s.to_string())
                            }
                        });
                        let date_time = parts[3..7.min(parts.len())].join(" ");
                        return (date_time, from);
                    }
                }
            }
        }
    }

    ("Login tracking unavailable".to_string(), None)
}

#[cfg(target_os = "macos")]
fn get_last_login_macos(username: &str) -> (String, Option<String>) {
    // Use last command on macOS
    if let Some(output) = run_output("last", ["-1", username], CommandTimeout::Normal) {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().next() {
                if !line.contains("wtmp begins") && !line.is_empty() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        let from = parts.get(2).and_then(|s| {
                            if s.starts_with(':')
                                || s.starts_with("console")
                                || s.starts_with("tty")
                            {
                                None
                            } else {
                                Some(s.to_string())
                            }
                        });
                        let date_time = parts[3..7.min(parts.len())].join(" ");
                        return (date_time, from);
                    }
                }
            }
        }
    }

    ("Login tracking unavailable".to_string(), None)
}

#[cfg(target_os = "windows")]
fn get_last_login_windows(_username: &str) -> (String, Option<String>) {
    // Preferred: WTSQuerySessionInformation(WTSLogonTime / WTSConnectTime) for
    // the current session — works without admin and is accurate for RDP /
    // network logons. Reference:
    // https://learn.microsoft.com/en-us/windows/win32/api/wtsapi32/nf-wtsapi32-wtsquerysessioninformationw
    if let Some(when) = wts_query_session_connect_time() {
        return (when, None);
    }

    // Fallback: derive from boot time. For local console sessions Windows
    // leaves WTSLogonTime/WTSConnectTime at 0 — but the user has effectively
    // been "logged in" since boot, so the boot time is the meaningful answer.
    // GetTickCount64 returns ms since boot and continues across hibernation
    // resume on Windows 10/11, so it survives Fast Startup correctly.
    if let Some(boot_time_str) = boot_time_local_string() {
        return (boot_time_str, None);
    }

    ("Login tracking unavailable".to_string(), None)
}

#[cfg(target_os = "windows")]
fn boot_time_local_string() -> Option<String> {
    // SAFETY: GetTickCount64 takes no args, returns u64 (ms since boot).
    let uptime_ms: u64 = unsafe { winapi::um::sysinfoapi::GetTickCount64() };
    let now = chrono::Local::now();
    let boot = now - chrono::Duration::milliseconds(uptime_ms as i64);
    Some(boot.format("%a %b %-d %H:%M").to_string())
}

// `WTSQuerySessionInformationW` and friends are declared manually because
// `winapi-rs`'s `wtsapi32` feature does not expose them on stable. See:
// https://learn.microsoft.com/en-us/windows/win32/api/wtsapi32/nf-wtsapi32-wtsquerysessioninformationw
#[cfg(target_os = "windows")]
const WTS_CURRENT_SESSION: u32 = 0xFFFF_FFFF;
#[cfg(target_os = "windows")]
const WTS_CONNECT_TIME: u32 = 14; // WTS_INFO_CLASS::WTSConnectTime
#[cfg(target_os = "windows")]
const WTS_LOGON_TIME: u32 = 17; // WTS_INFO_CLASS::WTSLogonTime

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
fn wts_query_session_connect_time() -> Option<String> {
    // Try WTSLogonTime first (authentication time — populated for console + RDP).
    // Fall back to WTSConnectTime (RDP/network only — 0 for console sessions).
    let filetime = wts_query_filetime(WTS_LOGON_TIME)
        .filter(|&ft| ft > 0)
        .or_else(|| wts_query_filetime(WTS_CONNECT_TIME).filter(|&ft| ft > 0))?;
    filetime_to_local_string(filetime)
}

#[cfg(target_os = "windows")]
fn wts_query_filetime(info_class: u32) -> Option<i64> {
    let mut buffer_ptr: *mut u16 = std::ptr::null_mut();
    let mut bytes_returned: u32 = 0;

    // SAFETY: WTSQuerySessionInformationW returns a pointer to a LARGE_INTEGER
    // (FILETIME, 8 bytes) for the time-typed info classes. We must call
    // WTSFreeMemory after reading. `hServer = NULL` is `WTS_CURRENT_SERVER_HANDLE`.
    let ok = unsafe {
        WTSQuerySessionInformationW(
            std::ptr::null_mut(),
            WTS_CURRENT_SESSION,
            info_class,
            &mut buffer_ptr,
            &mut bytes_returned,
        )
    };

    if ok == 0 || buffer_ptr.is_null() || (bytes_returned as usize) < std::mem::size_of::<i64>() {
        if !buffer_ptr.is_null() {
            unsafe { WTSFreeMemory(buffer_ptr as *mut _) };
        }
        return None;
    }

    // Read the LARGE_INTEGER (FILETIME) — 100-ns intervals since 1601-01-01 UTC.
    // Copy bytes instead of dereferencing an i64 pointer; WTS gives us a byte
    // buffer and Rust cannot assume i64 alignment for that pointer.
    let mut raw = [0u8; 8];
    unsafe {
        std::ptr::copy_nonoverlapping(buffer_ptr as *const u8, raw.as_mut_ptr(), raw.len());
    }
    let filetime = i64::from_le_bytes(raw);
    unsafe { WTSFreeMemory(buffer_ptr as *mut _) };
    Some(filetime)
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
