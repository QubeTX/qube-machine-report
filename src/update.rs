// Self-update for TR-300
//
// Checks the GitHub releases API for a newer version and
// runs the platform-specific installer to update in place.

use crate::config::{Config, OutputFormat};
use crate::VERSION;

/// GitHub API endpoint for the latest release.
const RELEASES_URL: &str =
    "https://api.github.com/repos/QubeTX/qube-machine-report/releases/latest";

/// Shell installer URL (macOS/Linux).
#[cfg(not(windows))]
const SHELL_INSTALLER: &str =
    "https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr-300-installer.sh";

/// PowerShell installer URL (Windows).
const PS_INSTALLER: &str =
    "https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr-300-installer.ps1";

/// Crate name for cargo install.
const CRATE_NAME: &str = "tr-300";

// ── Color helpers ──────────────────────────────────────────────────

fn green(text: &str, config: &Config) -> String {
    if config.use_colors {
        format!("\x1b[32m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

fn red(text: &str, config: &Config) -> String {
    if config.use_colors {
        format!("\x1b[31m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

fn cyan(text: &str, config: &Config) -> String {
    if config.use_colors {
        format!("\x1b[36m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

fn success_icon(config: &Config) -> &'static str {
    if config.use_unicode {
        "\u{2714}"
    } else {
        "[OK]"
    }
}

fn fail_icon(config: &Config) -> &'static str {
    if config.use_unicode {
        "\u{2718}"
    } else {
        "[FAIL]"
    }
}

// ── Public entry point ─────────────────────────────────────────────

/// Run the self-update flow. Returns an exit code (0 = success, 2 = error).
pub fn run(config: &Config) -> i32 {
    if config.format == OutputFormat::Json {
        return run_json();
    }

    println!();
    println!("  {} Checking for updates...", cyan("*", config));

    // Fetch latest version from GitHub
    let latest = match fetch_latest_version() {
        Ok(v) => v,
        Err(e) => {
            println!(
                "  {} {}",
                red(fail_icon(config), config),
                red(&format!("Failed to check for updates: {}", e), config),
            );
            return 2;
        }
    };

    let current = VERSION.to_string();

    if !is_newer(&current, &latest) {
        println!(
            "  {} {}",
            green(success_icon(config), config),
            green(
                &format!("Already on the latest version (v{})", current),
                config
            ),
        );
        return 0;
    }

    println!(
        "  {} Update available: v{} {} v{}",
        cyan("*", config),
        current,
        cyan("->", config),
        latest,
    );

    // Detect installation method
    let method = detect_install_method();
    let method_label = match method {
        InstallMethod::Cargo => "cargo install",
        InstallMethod::Installer => {
            if cfg!(windows) {
                "PowerShell installer"
            } else {
                "shell installer"
            }
        }
    };

    println!("  {} Updating via {}...", cyan("*", config), method_label);
    println!();

    // Execute update
    match execute_update(&method) {
        Ok(()) => {
            println!();
            println!(
                "  {} {}",
                green(success_icon(config), config),
                green(&format!("Updated to v{}", latest), config),
            );
            0
        }
        Err(e) => {
            println!();
            println!(
                "  {} {}",
                red(fail_icon(config), config),
                red(&format!("Update failed: {}", e), config),
            );
            2
        }
    }
}

// ── JSON output mode ───────────────────────────────────────────────

fn run_json() -> i32 {
    let latest = match fetch_latest_version() {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                serde_json::json!({
                    "action": "update",
                    "success": false,
                    "message": format!("Failed to check for updates: {}", e),
                    "current_version": VERSION,
                })
            );
            return 2;
        }
    };

    let current = VERSION.to_string();
    let update_available = is_newer(&current, &latest);

    if !update_available {
        println!(
            "{}",
            serde_json::json!({
                "action": "update",
                "success": true,
                "message": "Already on the latest version",
                "current_version": current,
                "latest_version": latest,
                "update_available": false,
            })
        );
        return 0;
    }

    let method = detect_install_method();
    match execute_update(&method) {
        Ok(()) => {
            println!(
                "{}",
                serde_json::json!({
                    "action": "update",
                    "success": true,
                    "message": format!("Updated from v{} to v{}", current, latest),
                    "current_version": current,
                    "latest_version": latest,
                    "update_available": true,
                    "method": match method {
                        InstallMethod::Cargo => "cargo",
                        InstallMethod::Installer => "installer",
                    },
                })
            );
            0
        }
        Err(e) => {
            println!(
                "{}",
                serde_json::json!({
                    "action": "update",
                    "success": false,
                    "message": format!("Update failed: {}", e),
                    "current_version": current,
                    "latest_version": latest,
                    "update_available": true,
                })
            );
            2
        }
    }
}

// ── Version check ──────────────────────────────────────────────────

/// Fetch the latest version tag from GitHub releases API.
fn fetch_latest_version() -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(15))
        .build();

    let resp = agent
        .get(RELEASES_URL)
        .set("User-Agent", &format!("tr300/{}", VERSION))
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("Request failed: {}", e))?;

    let body: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let tag = body["tag_name"]
        .as_str()
        .ok_or("Missing tag_name in response")?;

    // Strip leading 'v' if present
    Ok(tag.strip_prefix('v').unwrap_or(tag).to_string())
}

/// Compare semver versions. Returns true if `latest` is newer than `current`.
fn is_newer(current: &str, latest: &str) -> bool {
    let parse =
        |v: &str| -> Vec<u64> { v.split('.').filter_map(|s| s.parse::<u64>().ok()).collect() };

    let c = parse(current);
    let l = parse(latest);

    // Pad shorter vec with zeros
    let len = c.len().max(l.len());
    for i in 0..len {
        let cv = c.get(i).copied().unwrap_or(0);
        let lv = l.get(i).copied().unwrap_or(0);
        if lv > cv {
            return true;
        }
        if lv < cv {
            return false;
        }
    }
    false
}

// ── Installation method detection ──────────────────────────────────

enum InstallMethod {
    Cargo,
    Installer,
}

/// Detect how tr300 was installed by examining the binary's path.
fn detect_install_method() -> InstallMethod {
    if let Ok(exe) = std::env::current_exe() {
        let path_str = exe.to_string_lossy().to_lowercase();
        if path_str.contains(".cargo") && path_str.contains("bin") {
            return InstallMethod::Cargo;
        }
    }
    InstallMethod::Installer
}

// ── Platform-specific update execution ─────────────────────────────

/// Execute the platform-specific update command.
fn execute_update(method: &InstallMethod) -> Result<(), String> {
    let status = match method {
        InstallMethod::Cargo => std::process::Command::new("cargo")
            .args(["install", CRATE_NAME, "--force"])
            .status()
            .map_err(|e| format!("Failed to run cargo install: {}", e))?,
        InstallMethod::Installer => {
            #[cfg(windows)]
            {
                std::process::Command::new("powershell")
                    .args([
                        "-ExecutionPolicy",
                        "ByPass",
                        "-c",
                        &format!("irm {} | iex", PS_INSTALLER),
                    ])
                    .status()
                    .map_err(|e| format!("Failed to run PowerShell installer: {}", e))?
            }

            #[cfg(not(windows))]
            {
                std::process::Command::new("sh")
                    .args([
                        "-c",
                        &format!(
                            "curl --proto '=https' --tlsv1.2 -LsSf {} | sh",
                            SHELL_INSTALLER
                        ),
                    ])
                    .status()
                    .map_err(|e| format!("Failed to run shell installer: {}", e))?
            }
        }
    };

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Installer exited with code {}",
            status.code().unwrap_or(-1)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_basic() {
        assert!(is_newer("1.0.0", "2.0.0"));
        assert!(is_newer("1.0.0", "1.1.0"));
        assert!(is_newer("1.0.0", "1.0.1"));
        assert!(!is_newer("2.0.0", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.0.0"));
    }

    #[test]
    fn test_is_newer_different_lengths() {
        assert!(is_newer("1.0", "1.0.1"));
        assert!(!is_newer("1.0.1", "1.0"));
    }

    #[test]
    fn test_is_newer_major_versions() {
        assert!(is_newer("3.8.0", "4.0.0"));
        assert!(!is_newer("4.0.0", "3.99.99"));
    }
}
