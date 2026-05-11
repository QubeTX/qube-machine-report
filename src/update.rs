// Self-update for TR-300
//
// Checks the GitHub releases API for a newer version and runs an ordered
// cargo/installer strategy chain to update in place.

use crate::config::{Config, OutputFormat};
use crate::VERSION;

use std::process::{Command, Stdio};

/// GitHub API endpoint for the latest release.
const RELEASES_URL: &str =
    "https://api.github.com/repos/QubeTX/qube-machine-report/releases/latest";

/// Shell installer URL (macOS/Linux).
#[cfg(not(windows))]
const SHELL_INSTALLER: &str =
    "https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr300-installer.sh";

/// PowerShell installer URL (Windows).
#[cfg(windows)]
const PS_INSTALLER: &str =
    "https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr300-installer.ps1";

/// Crate name for cargo install.
const CRATE_NAME: &str = "tr300";

const MANUAL_INSTALL_URL: &str = "https://github.com/QubeTX/qube-machine-report#installation";

// ── Strategy types ─────────────────────────────────────────────────

/// Ordered candidate strategies for updating the binary. The runner tries each
/// in order and falls through on preflight or runtime failure until one works.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateStrategy {
    Cargo,
    InstallerCurl,
    InstallerWget,
    InstallerPowerShell,
    InstallerPwsh,
}

impl UpdateStrategy {
    fn label(self) -> &'static str {
        match self {
            UpdateStrategy::Cargo => "cargo install",
            UpdateStrategy::InstallerCurl => "curl shell installer",
            UpdateStrategy::InstallerWget => "wget shell installer",
            UpdateStrategy::InstallerPowerShell => "PowerShell installer",
            UpdateStrategy::InstallerPwsh => "pwsh installer",
        }
    }

    fn json_id(self) -> &'static str {
        match self {
            UpdateStrategy::Cargo => "cargo",
            UpdateStrategy::InstallerCurl => "installer_curl",
            UpdateStrategy::InstallerWget => "installer_wget",
            UpdateStrategy::InstallerPowerShell => "installer_powershell",
            UpdateStrategy::InstallerPwsh => "installer_pwsh",
        }
    }

    /// Backward-compatible label for the existing JSON `"method"` field.
    fn json_method(self) -> &'static str {
        if matches!(self, UpdateStrategy::Cargo) {
            "cargo"
        } else {
            "installer"
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetOs {
    Unix,
    Windows,
}

#[derive(Debug)]
enum StrategyError {
    /// Required launcher/tool is unavailable or could not be spawned.
    Preflight(String),
    /// Strategy launched and exited non-zero.
    Runtime(String),
}

#[derive(Debug)]
enum AttemptKind {
    Skipped,
    Failed,
}

#[derive(Debug)]
struct AttemptRecord {
    strategy: UpdateStrategy,
    kind: AttemptKind,
    message: String,
}

#[derive(Debug)]
struct UpdateFailure {
    attempts: Vec<AttemptRecord>,
}

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

    let strategies = build_strategy_list();
    if let Some(strategy) = strategies.first() {
        println!(
            "  {} Updating via {}...",
            cyan("*", config),
            strategy.label()
        );
    }
    println!();

    match execute_update(&strategies) {
        Ok(used) => {
            println!();
            println!(
                "  {} {}",
                green(success_icon(config), config),
                green(
                    &format!("Updated to v{} via {}", latest, used.label()),
                    config
                ),
            );
            0
        }
        Err(failure) => {
            println!();
            println!(
                "  {} {}",
                red(fail_icon(config), config),
                red("Update failed. Strategies attempted:", config),
            );
            for record in &failure.attempts {
                let kind = match record.kind {
                    AttemptKind::Skipped => "skipped",
                    AttemptKind::Failed => "failed",
                };
                println!(
                    "      · {} — {}: {}",
                    record.strategy.label(),
                    kind,
                    record.message
                );
            }
            println!();
            println!("  To update manually, see: {}", MANUAL_INSTALL_URL);
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

    let strategies = build_strategy_list();
    match execute_update(&strategies) {
        Ok(used) => {
            println!(
                "{}",
                serde_json::json!({
                    "action": "update",
                    "success": true,
                    "message": format!("Updated from v{} to v{}", current, latest),
                    "current_version": current,
                    "latest_version": latest,
                    "update_available": true,
                    "method": used.json_method(),
                    "strategy": used.json_id(),
                })
            );
            0
        }
        Err(failure) => {
            let attempts: Vec<serde_json::Value> = failure
                .attempts
                .iter()
                .map(|record| {
                    serde_json::json!({
                        "strategy": record.strategy.json_id(),
                        "result": match record.kind {
                            AttemptKind::Skipped => "skipped",
                            AttemptKind::Failed => "failed",
                        },
                        "message": record.message,
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::json!({
                    "action": "update",
                    "success": false,
                    "message": "Update failed; see attempts",
                    "current_version": current,
                    "latest_version": latest,
                    "update_available": true,
                    "attempts": attempts,
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

// ── Strategy ordering ───────────────────────────────────────────────

fn order_strategies(cargo_invokable: bool, os: TargetOs) -> Vec<UpdateStrategy> {
    let mut strategies = Vec::new();
    if cargo_invokable {
        strategies.push(UpdateStrategy::Cargo);
    }
    match os {
        TargetOs::Unix => {
            strategies.push(UpdateStrategy::InstallerCurl);
            strategies.push(UpdateStrategy::InstallerWget);
        }
        TargetOs::Windows => {
            strategies.push(UpdateStrategy::InstallerPowerShell);
            strategies.push(UpdateStrategy::InstallerPwsh);
        }
    }
    strategies
}

fn current_target_os() -> TargetOs {
    if cfg!(windows) {
        TargetOs::Windows
    } else {
        TargetOs::Unix
    }
}

fn cargo_invokable() -> bool {
    tool_exists("cargo")
}

fn tool_exists(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn build_strategy_list() -> Vec<UpdateStrategy> {
    order_strategies(cargo_invokable(), current_target_os())
}

// ── Platform-specific update execution ─────────────────────────────

/// Run `rustup update stable` if rustup is on PATH. Best-effort: any failure is
/// non-fatal (we let the subsequent `cargo install` surface the real error).
/// This keeps the user's toolchain in lock-step with whatever Rust version
/// tr300's MSRV (Cargo.toml `rust-version`) currently requires.
fn rustup_update_stable_best_effort() {
    if !tool_exists("rustup") {
        return;
    }
    println!("Updating Rust toolchain (rustup update stable)…");
    let _ = Command::new("rustup").args(["update", "stable"]).status();
}

fn execute_update(strategies: &[UpdateStrategy]) -> Result<UpdateStrategy, UpdateFailure> {
    let mut attempts = Vec::new();
    for &strategy in strategies {
        match try_strategy(strategy) {
            Ok(()) => return Ok(strategy),
            Err(StrategyError::Preflight(message)) => {
                eprintln!("  · skipped {}: {}", strategy.label(), message);
                attempts.push(AttemptRecord {
                    strategy,
                    kind: AttemptKind::Skipped,
                    message,
                });
            }
            Err(StrategyError::Runtime(message)) => {
                eprintln!("  · {} failed: {}", strategy.label(), message);
                attempts.push(AttemptRecord {
                    strategy,
                    kind: AttemptKind::Failed,
                    message,
                });
            }
        }
    }
    Err(UpdateFailure { attempts })
}

fn try_strategy(strategy: UpdateStrategy) -> Result<(), StrategyError> {
    match strategy {
        UpdateStrategy::Cargo => {
            rustup_update_stable_best_effort();
            run_command_status("cargo", &["install", CRATE_NAME, "--force"])
        }
        UpdateStrategy::InstallerCurl => try_installer_curl(),
        UpdateStrategy::InstallerWget => try_installer_wget(),
        UpdateStrategy::InstallerPowerShell => try_installer_powershell("powershell"),
        UpdateStrategy::InstallerPwsh => try_installer_powershell("pwsh"),
    }
}

fn run_command_status(launcher: &str, args: &[&str]) -> Result<(), StrategyError> {
    match Command::new(launcher).args(args).status() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(StrategyError::Preflight(
            format!("{} not on PATH", launcher),
        )),
        Err(e) => Err(StrategyError::Preflight(format!(
            "Failed to spawn {}: {}",
            launcher, e
        ))),
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(StrategyError::Runtime(format!(
            "{} exited with code {}",
            launcher,
            status.code().unwrap_or(-1)
        ))),
    }
}

#[cfg(unix)]
fn try_installer_curl() -> Result<(), StrategyError> {
    if !tool_exists("curl") {
        return Err(StrategyError::Preflight("curl not on PATH".into()));
    }
    if !tool_exists("bash") {
        return Err(StrategyError::Preflight("bash not on PATH".into()));
    }
    let script = format!(
        "set -euo pipefail; curl --proto '=https' --tlsv1.2 -fLsS {} | sh",
        SHELL_INSTALLER
    );
    run_command_status("bash", &["-c", &script])
}

#[cfg(not(unix))]
fn try_installer_curl() -> Result<(), StrategyError> {
    Err(StrategyError::Preflight(
        "curl installer is Unix-only".into(),
    ))
}

#[cfg(unix)]
fn try_installer_wget() -> Result<(), StrategyError> {
    if !tool_exists("wget") {
        return Err(StrategyError::Preflight("wget not on PATH".into()));
    }
    if !tool_exists("bash") {
        return Err(StrategyError::Preflight("bash not on PATH".into()));
    }
    let script = format!("set -euo pipefail; wget -qO- {} | sh", SHELL_INSTALLER);
    run_command_status("bash", &["-c", &script])
}

#[cfg(not(unix))]
fn try_installer_wget() -> Result<(), StrategyError> {
    Err(StrategyError::Preflight(
        "wget installer is Unix-only".into(),
    ))
}

#[cfg(windows)]
fn try_installer_powershell(launcher: &str) -> Result<(), StrategyError> {
    let script = format!("$ErrorActionPreference='Stop'; irm {} | iex", PS_INSTALLER);
    run_command_status(
        launcher,
        &[
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ],
    )
}

#[cfg(not(windows))]
fn try_installer_powershell(_launcher: &str) -> Result<(), StrategyError> {
    Err(StrategyError::Preflight(
        "PowerShell installer is Windows-only".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_with_cargo_orders_cargo_first() {
        assert_eq!(
            order_strategies(true, TargetOs::Unix),
            vec![
                UpdateStrategy::Cargo,
                UpdateStrategy::InstallerCurl,
                UpdateStrategy::InstallerWget,
            ]
        );
    }

    #[test]
    fn unix_without_cargo_prunes_cargo() {
        assert_eq!(
            order_strategies(false, TargetOs::Unix),
            vec![UpdateStrategy::InstallerCurl, UpdateStrategy::InstallerWget,]
        );
    }

    #[test]
    fn windows_with_cargo_orders_cargo_first() {
        assert_eq!(
            order_strategies(true, TargetOs::Windows),
            vec![
                UpdateStrategy::Cargo,
                UpdateStrategy::InstallerPowerShell,
                UpdateStrategy::InstallerPwsh,
            ]
        );
    }

    #[test]
    fn windows_without_cargo_prunes_cargo() {
        assert_eq!(
            order_strategies(false, TargetOs::Windows),
            vec![
                UpdateStrategy::InstallerPowerShell,
                UpdateStrategy::InstallerPwsh,
            ]
        );
    }

    #[test]
    fn json_method_maps_to_legacy_taxonomy() {
        assert_eq!(UpdateStrategy::Cargo.json_method(), "cargo");
        assert_eq!(UpdateStrategy::InstallerCurl.json_method(), "installer");
        assert_eq!(UpdateStrategy::InstallerWget.json_method(), "installer");
        assert_eq!(
            UpdateStrategy::InstallerPowerShell.json_method(),
            "installer"
        );
        assert_eq!(UpdateStrategy::InstallerPwsh.json_method(), "installer");
    }

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
