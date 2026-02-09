//! User session information collector

use crate::collectors::CollectMode;
use crate::error::Result;
use std::env;
use std::process::Command;

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
        users::get_current_username()
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
    // Try lastlog2 first (newer systems)
    if let Ok(output) = Command::new("lastlog2").args(["--user", username]).output() {
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
    if let Ok(output) = Command::new("lastlog").args(["-u", username]).output() {
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

    // Try last command
    if let Ok(output) = Command::new("last").args(["-1", username]).output() {
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
    if let Ok(output) = Command::new("last").args(["-1", username]).output() {
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
    // Try net user command first (faster than PowerShell)
    if let Ok(output) = Command::new("net")
        .args(["user", &env::var("USERNAME").unwrap_or_default()])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("Last logon") {
                if let Some(date) = line.split_whitespace().nth(2) {
                    let time = line
                        .split_whitespace()
                        .skip(3)
                        .collect::<Vec<_>>()
                        .join(" ");
                    return (format!("{} {}", date, time), None);
                }
            }
        }
    }

    ("Login tracking unavailable".to_string(), None)
}
