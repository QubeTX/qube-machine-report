// Self-update for TR-300
//
// Checks the GitHub releases API for a newer version and dispatches to an
// installer strategy that matches how this binary was installed. On Windows
// (v3.15.0+), the four first-class installers (MSI Global, MSI Corporate,
// EXE Global, EXE Corporate) write a HKCU\Software\TR300\InstallSource
// registry marker, which `detect_install_origin()` reads to pick the
// matching MSI/EXE for in-place upgrade. Path-based fallback handles the
// cargo install / PowerShell installer path that doesn't write a marker.

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

/// Global (perMachine) MSI installer URL.
#[cfg(windows)]
const MSI_GLOBAL_URL: &str = "https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr300-x86_64-pc-windows-msvc.msi";

/// Corporate (perUser) MSI installer URL.
#[cfg(windows)]
const MSI_CORPORATE_URL: &str = "https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr300-x86_64-pc-windows-msvc-corporate.msi";

/// Global (perMachine) EXE installer URL (Inno Setup).
#[cfg(windows)]
const EXE_GLOBAL_URL: &str = "https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr300-x86_64-pc-windows-msvc-setup.exe";

/// Corporate (perUser) EXE installer URL (Inno Setup).
#[cfg(windows)]
const EXE_CORPORATE_URL: &str = "https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr300-x86_64-pc-windows-msvc-corporate-setup.exe";

/// Crate name for cargo install.
const CRATE_NAME: &str = "tr300";

const MANUAL_INSTALL_URL: &str = "https://github.com/QubeTX/qube-machine-report#installation";

// ── Strategy types ─────────────────────────────────────────────────

/// Ordered candidate strategies for updating the binary.
///
/// For Windows MSI / EXE installer strategies (v3.15.0+), the runner picks
/// exactly one strategy based on `detect_install_origin()` and does NOT
/// fall back to a different installer type on failure — re-running a
/// different product would create coexistence problems (two ARP entries,
/// PATH ordering decides which wins). For cargo install / shell installer
/// users, the legacy probe-and-retry chain runs as before.
// The four MSI/EXE variants are only ever constructed by
// build_strategy_list() inside its #[cfg(windows)] block, so on non-Windows
// targets the dead_code lint flags them as never-constructed. The variants
// still need to exist on every platform so the label()/json_id()/json_method()
// match arms stay exhaustive and the try_strategy() dispatch arms compile.
// cfg_attr keeps Windows clippy strict (so missing wiring on Windows still
// trips the lint) while silencing it on Linux/macOS.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateStrategy {
    Cargo,
    InstallerCurl,
    InstallerWget,
    InstallerPowerShell,
    InstallerPwsh,
    /// Re-runs the Global perMachine MSI (UAC required, replaces v3.15.0+).
    MsiGlobal,
    /// Re-runs the Corporate perUser MSI (no UAC required).
    MsiCorporate,
    /// Re-runs the Global perMachine Inno Setup EXE (UAC required).
    ExeGlobal,
    /// Re-runs the Corporate perUser Inno Setup EXE (no UAC required).
    ExeCorporate,
}

impl UpdateStrategy {
    fn label(self) -> &'static str {
        match self {
            UpdateStrategy::Cargo => "cargo install",
            UpdateStrategy::InstallerCurl => "curl shell installer",
            UpdateStrategy::InstallerWget => "wget shell installer",
            UpdateStrategy::InstallerPowerShell => "PowerShell installer",
            UpdateStrategy::InstallerPwsh => "pwsh installer",
            UpdateStrategy::MsiGlobal => "Global MSI installer",
            UpdateStrategy::MsiCorporate => "Corporate MSI installer",
            UpdateStrategy::ExeGlobal => "Global EXE installer",
            UpdateStrategy::ExeCorporate => "Corporate EXE installer",
        }
    }

    fn json_id(self) -> &'static str {
        match self {
            UpdateStrategy::Cargo => "cargo",
            UpdateStrategy::InstallerCurl => "installer_curl",
            UpdateStrategy::InstallerWget => "installer_wget",
            UpdateStrategy::InstallerPowerShell => "installer_powershell",
            UpdateStrategy::InstallerPwsh => "installer_pwsh",
            UpdateStrategy::MsiGlobal => "msi_global",
            UpdateStrategy::MsiCorporate => "msi_corporate",
            UpdateStrategy::ExeGlobal => "exe_global",
            UpdateStrategy::ExeCorporate => "exe_corporate",
        }
    }

    /// Backward-compatible label for the existing JSON `"method"` field.
    ///
    /// All MSI/EXE strategies map to `"installer"` to match the legacy
    /// taxonomy (anything that isn't `cargo install` was historically called
    /// an "installer"). External consumers looking for the specific installer
    /// type should read the precise `"strategy"` field instead.
    fn json_method(self) -> &'static str {
        if matches!(self, UpdateStrategy::Cargo) {
            "cargo"
        } else {
            "installer"
        }
    }
}

/// Where this binary was installed from, on Windows.
///
/// Determines which installer `tr300 update` downloads and re-runs for an
/// in-place upgrade. First-class installers (the four MSI/EXE variants)
/// write a `HKCU\Software\TR300\InstallSource` registry marker on install;
/// `detect_install_origin()` reads that marker. A path-based fallback
/// covers the cargo install / PowerShell installer path that doesn't write
/// a marker.
#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallOrigin {
    /// `C:\Program Files\tr300\bin\tr300.exe`, installed from wix/main.wxs.
    MsiGlobal,
    /// `%LocalAppData%\Programs\tr300\bin\tr300.exe`, from wix/corporate.wxs.
    MsiCorporate,
    /// `C:\Program Files\tr300\bin\tr300.exe`, from inno/global.iss.
    ExeGlobal,
    /// `%LocalAppData%\Programs\tr300\bin\tr300.exe`, from inno/corporate.iss.
    ExeCorporate,
    /// `~\.cargo\bin\tr300.exe` — installed via `cargo install` or the
    /// cargo-dist PowerShell installer. Uses the legacy strategy chain.
    CargoOrInstaller,
    /// Couldn't determine origin (custom install location, portable use,
    /// etc.). Treated like `CargoOrInstaller` so the legacy chain runs.
    Unknown,
}

#[cfg(windows)]
impl InstallOrigin {
    /// String form for JSON output. Matches the registry marker values
    /// written by the installers; `cargo-or-installer` / `unknown` are
    /// synthesized by the path-based fallback.
    fn json_id(self) -> &'static str {
        match self {
            InstallOrigin::MsiGlobal => "msi-global",
            InstallOrigin::MsiCorporate => "msi-corporate",
            InstallOrigin::ExeGlobal => "exe-global",
            InstallOrigin::ExeCorporate => "exe-corporate",
            InstallOrigin::CargoOrInstaller => "cargo-or-installer",
            InstallOrigin::Unknown => "unknown",
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

    match execute_update(&latest, &strategies) {
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
            let mut payload = serde_json::json!({
                "action": "update",
                "success": false,
                "message": format!("Failed to check for updates: {}", e),
                "current_version": VERSION,
            });
            inject_install_origin(&mut payload);
            println!("{}", payload);
            return 2;
        }
    };

    let current = VERSION.to_string();
    let update_available = is_newer(&current, &latest);

    if !update_available {
        let mut payload = serde_json::json!({
            "action": "update",
            "success": true,
            "message": "Already on the latest version",
            "current_version": current,
            "latest_version": latest,
            "update_available": false,
        });
        inject_install_origin(&mut payload);
        println!("{}", payload);
        return 0;
    }

    let strategies = build_strategy_list();
    match execute_update(&latest, &strategies) {
        Ok(used) => {
            let mut payload = serde_json::json!({
                "action": "update",
                "success": true,
                "message": format!("Updated from v{} to v{}", current, latest),
                "current_version": current,
                "latest_version": latest,
                "update_available": true,
                "method": used.json_method(),
                "strategy": used.json_id(),
            });
            inject_install_origin(&mut payload);
            println!("{}", payload);
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
            let mut payload = serde_json::json!({
                "action": "update",
                "success": false,
                "message": "Update failed; see attempts",
                "current_version": current,
                "latest_version": latest,
                "update_available": true,
                "attempts": attempts,
            });
            inject_install_origin(&mut payload);
            println!("{}", payload);
            2
        }
    }
}

/// Add a top-level `install_origin` field to the JSON payload on Windows.
/// On other platforms this is a no-op — the field is Windows-only because
/// install-origin only meaningfully varies on Windows (where users have a
/// choice of MSI vs EXE installer, perMachine vs perUser scope).
#[cfg(windows)]
fn inject_install_origin(payload: &mut serde_json::Value) {
    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "install_origin".to_string(),
            serde_json::Value::String(detect_install_origin().json_id().to_string()),
        );
    }
}

#[cfg(not(windows))]
fn inject_install_origin(_payload: &mut serde_json::Value) {
    // No-op on non-Windows; the field would always be the same value.
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

/// Strip any prerelease (`-rc.1`) or build-metadata (`+nightly.42`)
/// suffix from a semver string, returning just the `MAJOR.MINOR.PATCH`
/// portion.
///
/// The hand-rolled `is_newer` was previously silently dropping
/// non-numeric segments inside `filter_map`, which made
/// `"3.15.2-rc.1".split('.')` parse to `[3, 15, 1]` (the `2-rc` segment
/// failed to parse and the trailing `1` survived) — identical to
/// `"3.15.1"`. Users on a prerelease tag could end up stuck because
/// `is_newer` thought they were already on the latest. Truncating at
/// the first non-`[0-9.]` character is the simplest robust fix.
fn strip_prerelease_metadata(v: &str) -> &str {
    match v.find(|c: char| !c.is_ascii_digit() && c != '.') {
        Some(idx) => &v[..idx],
        None => v,
    }
}

/// Compare semver versions. Returns true if `latest` is newer than `current`.
///
/// Handles prerelease tags per the semver spec's general intent: a
/// prerelease of an upcoming version is considered NEWER than the
/// previous stable patch (`3.15.2-rc.1` > `3.15.1`), and a stable
/// release IS newer than any prerelease of the same triple
/// (`3.15.2` > `3.15.2-rc.1`). Two prereleases of the same triple are
/// treated as equal — we don't promote within a prerelease line because
/// GitHub's `/releases/latest` endpoint already filters prereleases
/// out, so the case is theoretical.
fn is_newer(current: &str, latest: &str) -> bool {
    let current_stripped = strip_prerelease_metadata(current);
    let latest_stripped = strip_prerelease_metadata(latest);

    let parse =
        |v: &str| -> Vec<u64> { v.split('.').filter_map(|s| s.parse::<u64>().ok()).collect() };

    let c = parse(current_stripped);
    let l = parse(latest_stripped);

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

    // Numeric parts equal. If the current binary is on a prerelease
    // of the same triple (e.g. `3.15.2-rc.1`) and the latest is the
    // stable release (`3.15.2`), the stable IS newer per semver
    // ordering. Inverse case (current stable, latest prerelease)
    // shouldn't happen via GitHub's `latest` endpoint.
    let current_has_suffix = current.len() != current_stripped.len();
    let latest_has_suffix = latest.len() != latest_stripped.len();
    current_has_suffix && !latest_has_suffix
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
    #[cfg(windows)]
    {
        // For first-class Windows installs, dispatch to a single
        // matching-installer strategy. Don't cross-fall-back to a different
        // installer type — running a different product would create
        // coexistence problems (two ARP entries, PATH ordering wins).
        match detect_install_origin() {
            InstallOrigin::MsiGlobal => return vec![UpdateStrategy::MsiGlobal],
            InstallOrigin::MsiCorporate => return vec![UpdateStrategy::MsiCorporate],
            InstallOrigin::ExeGlobal => return vec![UpdateStrategy::ExeGlobal],
            InstallOrigin::ExeCorporate => return vec![UpdateStrategy::ExeCorporate],
            InstallOrigin::CargoOrInstaller | InstallOrigin::Unknown => {
                // Fall through to the legacy cargo/PS chain.
            }
        }
    }
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

fn execute_update(
    latest: &str,
    strategies: &[UpdateStrategy],
) -> Result<UpdateStrategy, UpdateFailure> {
    let mut attempts = Vec::new();
    for &strategy in strategies {
        match try_strategy(strategy, latest) {
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

fn try_strategy(strategy: UpdateStrategy, latest: &str) -> Result<(), StrategyError> {
    match strategy {
        UpdateStrategy::Cargo => {
            rustup_update_stable_best_effort();
            run_command_status("cargo", &["install", CRATE_NAME, "--force"])
        }
        UpdateStrategy::InstallerCurl => try_installer_curl(),
        UpdateStrategy::InstallerWget => try_installer_wget(),
        UpdateStrategy::InstallerPowerShell => try_installer_powershell("powershell"),
        UpdateStrategy::InstallerPwsh => try_installer_powershell("pwsh"),
        UpdateStrategy::MsiGlobal => try_msi_install(msi_global_url(), latest),
        UpdateStrategy::MsiCorporate => try_msi_install(msi_corporate_url(), latest),
        UpdateStrategy::ExeGlobal => try_exe_install(exe_global_url(), latest),
        UpdateStrategy::ExeCorporate => try_exe_install(exe_corporate_url(), latest),
    }
}

// URL accessors are #[cfg(windows)] under the hood but we expose them
// uniformly so the dispatch arms above don't need their own #[cfg] gates —
// keeps the match exhaustive on all platforms.
#[cfg(windows)]
fn msi_global_url() -> &'static str {
    MSI_GLOBAL_URL
}
#[cfg(windows)]
fn msi_corporate_url() -> &'static str {
    MSI_CORPORATE_URL
}
#[cfg(windows)]
fn exe_global_url() -> &'static str {
    EXE_GLOBAL_URL
}
#[cfg(windows)]
fn exe_corporate_url() -> &'static str {
    EXE_CORPORATE_URL
}
#[cfg(not(windows))]
fn msi_global_url() -> &'static str {
    ""
}
#[cfg(not(windows))]
fn msi_corporate_url() -> &'static str {
    ""
}
#[cfg(not(windows))]
fn exe_global_url() -> &'static str {
    ""
}
#[cfg(not(windows))]
fn exe_corporate_url() -> &'static str {
    ""
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

// ── Windows MSI / EXE installer strategies (v3.15.0+) ──────────────

/// msiexec exit code returned when the install completed successfully
/// but a reboot is required to finalize a file replacement (Windows
/// Installer's Restart Manager couldn't replace a locked file in-place
/// and scheduled a `MoveFileEx`-style delete-on-reboot instead).
#[cfg(windows)]
const MSI_EXIT_REBOOT_REQUIRED: i32 = 3010;

/// Download a file from `url` to `path` over HTTPS. Used by the MSI/EXE
/// strategies to fetch the matching installer to `%TEMP%` before launching it.
/// TLS validation is enforced by `ureq`; the caller then re-fetches the
/// `.sha256` sidecar and runs `verify_checksum` for defense against a
/// corporate-proxy interception or a tampered release asset.
#[cfg(windows)]
fn download_to_file(url: &str, path: &std::path::Path) -> Result<(), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(120))
        .build();

    let resp = agent
        .get(url)
        .set("User-Agent", &format!("tr300/{}", VERSION))
        .call()
        .map_err(|e| format!("Request failed: {}", e))?;

    let mut file = std::fs::File::create(path)
        .map_err(|e| format!("Failed to create temp file {}: {}", path.display(), e))?;
    let mut reader = resp.into_reader();
    std::io::copy(&mut reader, &mut file)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    Ok(())
}

/// Fetch the cargo-dist `.sha256` sidecar at `<url>.sha256` and return
/// the file contents.
///
/// Format: `<lowercase-64-char-hex>  *<filename>` per cargo-dist's
/// `dist-manifest.json` generation and the parallel implementation in
/// `.github/workflows/windows-installers.yml`. Tolerant of trailing
/// whitespace / missing asterisk via `parse_sha256_sidecar`.
#[cfg(windows)]
fn fetch_sha256_sidecar(url: &str) -> Result<String, String> {
    let sidecar_url = format!("{}.sha256", url);
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(30))
        .build();
    let resp = agent
        .get(&sidecar_url)
        .set("User-Agent", &format!("tr300/{}", VERSION))
        .call()
        .map_err(|e| format!("Sidecar request failed ({}): {}", sidecar_url, e))?;
    resp.into_string()
        .map_err(|e| format!("Failed to read sidecar body: {}", e))
}

/// Extract the 64-char hex from a `.sha256` sidecar line. Returns
/// `None` when the first whitespace-separated token is not exactly
/// 64 hex characters.
#[cfg(windows)]
fn parse_sha256_sidecar(content: &str) -> Option<String> {
    content
        .split_whitespace()
        .next()
        .filter(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()))
        .map(|s| s.to_lowercase())
}

/// Compute the SHA-256 of `path`, returning the lowercase hex.
#[cfg(windows)]
fn compute_sha256(path: &std::path::Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|e| format!("Failed to hash: {}", e))?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// Fetch the `.sha256` sidecar, compute the SHA-256 of the downloaded
/// installer, refuse to proceed on mismatch.
///
/// Defends against a network MITM that replaces the installer bytes in
/// flight (corporate TLS-interception proxies with a trusted root CA,
/// hostile WiFi, captive portals). The sidecar is fetched in a separate
/// request — an attacker would have to corrupt both the installer and
/// the sidecar in a way that yields a matching hash, which is
/// preimage-hard.
#[cfg(windows)]
fn verify_checksum(installer_path: &std::path::Path, installer_url: &str) -> Result<(), String> {
    println!("  Verifying SHA256 checksum...");
    let sidecar_content = fetch_sha256_sidecar(installer_url)?;
    let expected = parse_sha256_sidecar(&sidecar_content).ok_or_else(|| {
        format!(
            "Malformed .sha256 sidecar from {}.sha256: {:?}",
            installer_url, sidecar_content
        )
    })?;
    let actual = compute_sha256(installer_path)?;
    checksum_verdict(&actual, &expected)
}

/// Compare a computed SHA-256 against the expected sidecar hash, refusing on
/// mismatch. Separated from the network fetch + file read in `verify_checksum`
/// so the load-bearing refusal-on-mismatch is unit-testable on any target.
#[cfg(any(target_os = "windows", test))]
fn checksum_verdict(actual: &str, expected: &str) -> Result<(), String> {
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!(
            "SHA256 mismatch — refusing to run installer.\n         Expected: {}\n         Got:      {}\n         This usually indicates a corrupted download or a network MITM.",
            expected, actual
        ))
    }
}

/// Whether the freshly-installed `--version` string matches the expected
/// release tag. Both sides are stripped of any prerelease/build-metadata
/// suffix before comparison, and an empty `installed` never matches (covers
/// the `--version` parse failing). Pure + platform-independent so it can be
/// unit-tested on any target — the live re-exec lives in `verify_post_install`,
/// which is Windows-only.
#[cfg(any(target_os = "windows", test))]
fn post_install_version_ok(installed: &str, expected: &str) -> bool {
    let installed_stripped = strip_prerelease_metadata(installed);
    let expected_stripped = strip_prerelease_metadata(expected);
    !installed_stripped.is_empty() && installed_stripped == expected_stripped
}

/// After the installer reports success, re-exec `<current_exe>
/// --version` and confirm the on-disk binary has been updated. Catches
/// the case where the installer exits 0 but the file replacement
/// didn't actually take effect (Restart Manager edge cases, MSI
/// re-running the SAME version, etc.).
///
/// Compares the post-install version against `expected` (the
/// `latest` tag from the GitHub releases API). Both are stripped of
/// any prerelease/build-metadata suffix before comparison so that
/// `tr300 3.15.4` matches `3.15.4` exactly even if the release tag
/// has a metadata suffix.
#[cfg(windows)]
fn verify_post_install(expected: &str) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe failed: {}", e))?;
    let output = Command::new(&exe)
        .arg("--version")
        .output()
        .map_err(|e| format!("Failed to spawn `{} --version`: {}", exe.display(), e))?;
    if !output.status.success() {
        return Err(format!(
            "`{} --version` exited with code {}",
            exe.display(),
            output.status.code().unwrap_or(-1)
        ));
    }
    // Output is "tr300 X.Y.Z\n"; the last whitespace-separated token
    // is the version.
    let installed = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .last()
        .unwrap_or("")
        .trim()
        .to_string();
    if post_install_version_ok(&installed, expected) {
        Ok(())
    } else {
        Err(format!(
            "Installer exited successfully but `{} --version` still reports v{} (expected v{}). The installed binary may be locked by another process — close other tr300 windows / shells and re-run, or reboot to let Windows finish a deferred file replace.",
            exe.display(), installed, expected
        ))
    }
}

/// Download the matching MSI, verify its SHA256, re-run it via
/// `msiexec /i /passive /norestart`, then re-exec the binary with
/// `--version` to confirm the file replacement actually took effect.
///
/// WiX `MajorUpgrade` in `wix/main.wxs` and `wix-corporate/corporate.wxs`
/// handles the uninstall-old-then-install-new step atomically. Windows
/// Installer's Restart Manager handles the "binary in use" case by
/// renaming the locked file (the running tr300 process keeps its open
/// file handle to the OLD inode); if RM falls back to delete-on-reboot,
/// msiexec returns 3010 and we surface that without claiming success.
#[cfg(windows)]
fn try_msi_install(url: &str, latest: &str) -> Result<(), StrategyError> {
    let temp_path = std::env::temp_dir().join(format!("tr300-update-{}.msi", latest));

    let result = (|| -> Result<(), StrategyError> {
        println!("  Downloading MSI installer...");
        download_to_file(url, &temp_path)
            .map_err(|e| StrategyError::Runtime(format!("Download failed: {}", e)))?;

        verify_checksum(&temp_path, url).map_err(StrategyError::Runtime)?;

        println!("  Launching Windows Installer...");
        // /passive shows a progress dialog with no user interaction; /norestart
        // suppresses any reboot prompt (we don't need a reboot for a simple file
        // replace). For the Global perMachine MSI, msiexec triggers UAC before
        // doing anything; for the Corporate perUser MSI, it installs silently
        // into LocalAppData with no elevation prompt.
        let status = Command::new("msiexec")
            .args(["/i", &temp_path.to_string_lossy(), "/passive", "/norestart"])
            .status()
            .map_err(|e| StrategyError::Preflight(format!("Failed to spawn msiexec: {}", e)))?;

        let code = status.code().unwrap_or(-1);
        if code == MSI_EXIT_REBOOT_REQUIRED {
            // Install completed but Restart Manager couldn't finalize a
            // file replace in-place. Surface this rather than silently
            // claiming success — `verify_post_install` would fail because
            // the on-disk binary is still old.
            return Err(StrategyError::Runtime(format!(
                "MSI install completed but requires a reboot to finalize (msiexec exit {}). Reboot, then verify with `tr300 --version`.",
                MSI_EXIT_REBOOT_REQUIRED
            )));
        }
        if !status.success() {
            return Err(StrategyError::Runtime(format!(
                "msiexec exited with code {} (likely user cancel, UAC denied, or install error)",
                code
            )));
        }

        verify_post_install(latest).map_err(StrategyError::Runtime)?;
        Ok(())
    })();

    // Best-effort: don't leave the downloaded installer behind in %TEMP%, on
    // success or failure. The SHA256 + post-install verify above are the
    // load-bearing checks; this cleanup is pure hygiene and never alters the
    // result. (msiexec has already exited, so the file is no longer in use.)
    let _ = std::fs::remove_file(&temp_path);
    result
}

/// Download the matching Inno Setup EXE installer, verify its SHA256,
/// re-run it with `/SILENT /SUPPRESSMSGBOXES /NORESTART`, then verify
/// the post-install version.
///
/// Inno Setup's AppId-based upgrade detection silently uninstalls the
/// old version before installing the new one. For the Global perMachine
/// EXE, `PrivilegesRequired=admin` in `inno/global.iss` triggers UAC
/// before any UI; the Corporate perUser EXE (`PrivilegesRequired=lowest`)
/// installs without elevation.
#[cfg(windows)]
fn try_exe_install(url: &str, latest: &str) -> Result<(), StrategyError> {
    let temp_path = std::env::temp_dir().join(format!("tr300-update-{}-setup.exe", latest));

    let result = (|| -> Result<(), StrategyError> {
        println!("  Downloading EXE installer...");
        download_to_file(url, &temp_path)
            .map_err(|e| StrategyError::Runtime(format!("Download failed: {}", e)))?;

        verify_checksum(&temp_path, url).map_err(StrategyError::Runtime)?;

        println!("  Launching Inno Setup installer...");
        // /SILENT shows a progress dialog but no wizard pages; /SUPPRESSMSGBOXES
        // suppresses non-critical message boxes; /NORESTART skips reboot prompts.
        let status = Command::new(&temp_path)
            .args(["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
            .status()
            .map_err(|e| {
                StrategyError::Preflight(format!("Failed to spawn EXE installer: {}", e))
            })?;

        if !status.success() {
            return Err(StrategyError::Runtime(format!(
                "EXE installer exited with code {} (likely user cancel, UAC denied, or install error)",
                status.code().unwrap_or(-1)
            )));
        }

        verify_post_install(latest).map_err(StrategyError::Runtime)?;
        Ok(())
    })();

    // Best-effort cleanup of the downloaded installer (success or failure); the
    // verifies above are the load-bearing checks. The installer process has
    // already exited by here.
    let _ = std::fs::remove_file(&temp_path);
    result
}

#[cfg(not(windows))]
fn try_msi_install(_url: &str, _latest: &str) -> Result<(), StrategyError> {
    Err(StrategyError::Preflight(
        "MSI installer is Windows-only".into(),
    ))
}

#[cfg(not(windows))]
fn try_exe_install(_url: &str, _latest: &str) -> Result<(), StrategyError> {
    Err(StrategyError::Preflight(
        "EXE installer is Windows-only".into(),
    ))
}

// ── Windows install-origin detection (v3.15.0+) ────────────────────

/// Read the `HKCU\Software\TR300\InstallSource` registry value written by
/// the four first-class installers on install. Authoritative when present.
/// Returns `None` if the key is missing, the value is missing, the value
/// type isn't a string, or the value content doesn't match a known variant.
#[cfg(windows)]
fn read_install_source_marker() -> Option<InstallOrigin> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey("Software\\TR300").ok()?;
    let value: String = key.get_value("InstallSource").ok()?;
    match value.as_str() {
        "msi-global" => Some(InstallOrigin::MsiGlobal),
        "msi-corporate" => Some(InstallOrigin::MsiCorporate),
        "exe-global" => Some(InstallOrigin::ExeGlobal),
        "exe-corporate" => Some(InstallOrigin::ExeCorporate),
        _ => None,
    }
}

/// Determine how this binary was installed on Windows.
///
/// Strategy:
/// 1. Read the `HKCU\Software\TR300\InstallSource` marker. Authoritative
///    when present. All four first-class installers (Global MSI, Corporate
///    MSI, Global EXE, Corporate EXE) write this marker on install.
/// 2. If no marker, fall back to path-based detection on the running
///    binary's location. Handles the cargo install / PowerShell installer
///    path (which doesn't write a marker), and any pre-marker legacy
///    installs.
///
/// The path fallback maps Program Files → `MsiGlobal` and
/// LocalAppData\Programs → `MsiCorporate` (it can't distinguish MSI vs EXE
/// when the marker is missing because both installer formats target the
/// same paths within each edition — that's by design, see README "pick
/// one format per edition"). When the marker IS present, the EXE vs MSI
/// distinction is preserved.
#[cfg(windows)]
fn detect_install_origin() -> InstallOrigin {
    if let Some(origin) = read_install_source_marker() {
        return origin;
    }

    let Ok(exe) = std::env::current_exe() else {
        return InstallOrigin::Unknown;
    };
    classify_install_path(&exe.to_string_lossy())
}

/// Pure-function half of `detect_install_origin()` for unit testing.
/// Lowercased substring match handles drive-letter casing and Windows path
/// case-insensitivity. Order matters: check more-specific paths first.
#[cfg(windows)]
fn classify_install_path(exe_path: &str) -> InstallOrigin {
    let lower = exe_path.to_lowercase();
    if lower.contains("\\program files\\tr300\\") {
        InstallOrigin::MsiGlobal
    } else if lower.contains("\\appdata\\local\\programs\\tr300\\") {
        InstallOrigin::MsiCorporate
    } else if lower.contains("\\.cargo\\bin\\") {
        InstallOrigin::CargoOrInstaller
    } else {
        InstallOrigin::Unknown
    }
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
        // New v3.15.0+ MSI/EXE strategies map to "installer" in the legacy
        // `method` field. External consumers wanting the specific installer
        // type should read the precise `strategy` field instead.
        assert_eq!(UpdateStrategy::MsiGlobal.json_method(), "installer");
        assert_eq!(UpdateStrategy::MsiCorporate.json_method(), "installer");
        assert_eq!(UpdateStrategy::ExeGlobal.json_method(), "installer");
        assert_eq!(UpdateStrategy::ExeCorporate.json_method(), "installer");
    }

    #[test]
    fn new_strategies_have_stable_json_ids() {
        // These IDs are part of the public JSON contract; renaming them is a
        // schema break. Keep them in lockstep with the README Self-Update
        // section and docs/architecture-decisions.md.
        assert_eq!(UpdateStrategy::MsiGlobal.json_id(), "msi_global");
        assert_eq!(UpdateStrategy::MsiCorporate.json_id(), "msi_corporate");
        assert_eq!(UpdateStrategy::ExeGlobal.json_id(), "exe_global");
        assert_eq!(UpdateStrategy::ExeCorporate.json_id(), "exe_corporate");
    }

    #[test]
    fn new_strategies_have_distinct_labels() {
        // Labels feed into the text-mode "Updating via X..." line.
        // Verify each one is distinct (otherwise users can't tell which
        // installer is being downloaded from text output alone).
        let labels = [
            UpdateStrategy::MsiGlobal.label(),
            UpdateStrategy::MsiCorporate.label(),
            UpdateStrategy::ExeGlobal.label(),
            UpdateStrategy::ExeCorporate.label(),
            UpdateStrategy::Cargo.label(),
            UpdateStrategy::InstallerPowerShell.label(),
            UpdateStrategy::InstallerPwsh.label(),
            UpdateStrategy::InstallerCurl.label(),
            UpdateStrategy::InstallerWget.label(),
        ];
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(
            unique.len(),
            labels.len(),
            "all strategy labels must be unique"
        );
    }

    #[cfg(windows)]
    #[test]
    fn install_origin_classify_program_files_is_msi_global() {
        assert_eq!(
            classify_install_path(r"C:\Program Files\tr300\bin\tr300.exe"),
            InstallOrigin::MsiGlobal,
        );
        // Case-insensitive: drive letter or "Program Files" capitalization
        // shouldn't change the verdict.
        assert_eq!(
            classify_install_path(r"c:\PROGRAM FILES\tr300\BIN\tr300.exe"),
            InstallOrigin::MsiGlobal,
        );
    }

    #[cfg(windows)]
    #[test]
    fn install_origin_classify_localappdata_is_msi_corporate() {
        assert_eq!(
            classify_install_path(r"C:\Users\alice\AppData\Local\Programs\tr300\bin\tr300.exe"),
            InstallOrigin::MsiCorporate,
        );
    }

    #[cfg(windows)]
    #[test]
    fn install_origin_classify_cargo_bin_is_cargo_or_installer() {
        assert_eq!(
            classify_install_path(r"C:\Users\alice\.cargo\bin\tr300.exe"),
            InstallOrigin::CargoOrInstaller,
        );
    }

    #[cfg(windows)]
    #[test]
    fn install_origin_classify_random_path_is_unknown() {
        assert_eq!(
            classify_install_path(r"D:\portable\tr300\tr300.exe"),
            InstallOrigin::Unknown,
        );
        assert_eq!(
            classify_install_path(r"C:\Users\alice\Downloads\tr300.exe"),
            InstallOrigin::Unknown,
        );
    }

    #[cfg(windows)]
    #[test]
    fn install_origin_json_ids_are_kebab_case() {
        // The JSON id matches the literal registry marker value written by
        // the installers. Keep these in lockstep with wix/main.wxs,
        // wix/corporate.wxs, inno/global.iss, inno/corporate.iss.
        assert_eq!(InstallOrigin::MsiGlobal.json_id(), "msi-global");
        assert_eq!(InstallOrigin::MsiCorporate.json_id(), "msi-corporate");
        assert_eq!(InstallOrigin::ExeGlobal.json_id(), "exe-global");
        assert_eq!(InstallOrigin::ExeCorporate.json_id(), "exe-corporate");
        assert_eq!(
            InstallOrigin::CargoOrInstaller.json_id(),
            "cargo-or-installer"
        );
        assert_eq!(InstallOrigin::Unknown.json_id(), "unknown");
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

    #[test]
    fn strip_prerelease_metadata_handles_prereleases() {
        // Before v3.15.4 the hand-rolled parser silently dropped the
        // `-rc.1` segment via filter_map, mapping this to [3, 15, 1].
        // Stripping the suffix first gives the correct [3, 15, 2].
        assert_eq!(strip_prerelease_metadata("3.15.2-rc.1"), "3.15.2");
        assert_eq!(strip_prerelease_metadata("3.15.2-alpha.10"), "3.15.2");
    }

    #[test]
    fn strip_prerelease_metadata_handles_build_metadata() {
        assert_eq!(strip_prerelease_metadata("3.15.2+nightly.42"), "3.15.2");
        assert_eq!(strip_prerelease_metadata("3.15.2+sha.abc123"), "3.15.2");
    }

    #[test]
    fn strip_prerelease_metadata_leaves_clean_version_alone() {
        assert_eq!(strip_prerelease_metadata("3.15.2"), "3.15.2");
        assert_eq!(strip_prerelease_metadata("1.0"), "1.0");
        assert_eq!(strip_prerelease_metadata(""), "");
    }

    #[test]
    fn post_install_version_ok_matches_after_stripping() {
        // Exact match — the happy path after a successful in-place upgrade.
        assert!(post_install_version_ok("3.16.0", "3.16.0"));
        // Installed string carries a build-metadata suffix the release tag lacks.
        assert!(post_install_version_ok("3.16.0+sha.abc123", "3.16.0"));
        // Mismatch — installer exited 0 but the file replacement didn't take.
        assert!(!post_install_version_ok("3.15.3", "3.16.0"));
        // Empty installed string (e.g. `--version` output failed to parse).
        assert!(!post_install_version_ok("", "3.16.0"));
    }

    #[test]
    fn checksum_verdict_accepts_match_and_refuses_mismatch() {
        let hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        // Exact match passes.
        assert!(checksum_verdict(hash, hash).is_ok());
        // Case-insensitive match passes (sidecars may be upper/lower case).
        assert!(checksum_verdict(&hash.to_uppercase(), hash).is_ok());
        // A mismatch is REFUSED — this is the load-bearing MITM/corruption
        // guard, so it must never silently pass.
        let err = checksum_verdict("deadbeef", hash).unwrap_err();
        assert!(err.contains("SHA256 mismatch"), "err: {err}");
    }

    #[test]
    fn is_newer_treats_prerelease_of_higher_triple_as_newer() {
        // Stable user on 3.15.1, prerelease of 3.15.2 published. The
        // pre-v3.15.4 parser said both were [3, 15, 1] (the `2-rc`
        // segment failed to parse, the trailing `1` survived).
        // Correct semver intent: 3.15.2-rc.1 is "approaching 3.15.2"
        // and IS newer than 3.15.1.
        assert!(is_newer("3.15.1", "3.15.2-rc.1"));
    }

    #[test]
    fn is_newer_treats_stable_as_newer_than_prerelease_of_same_triple() {
        // User on 3.15.2-rc.1 (manually installed prerelease), stable
        // 3.15.2 lands. Per semver, the stable IS newer than its own
        // prerelease. Pre-v3.15.4 parser said both were equal,
        // stranding the user.
        assert!(is_newer("3.15.2-rc.1", "3.15.2"));
    }

    #[test]
    fn is_newer_treats_two_prereleases_of_same_triple_as_equal() {
        // Theoretical case — GitHub /releases/latest filters prereleases
        // out so we shouldn't see this in practice. The conservative
        // "equal → not newer" verdict means tr300 reports "already on
        // latest" and the user can manually pick.
        assert!(!is_newer("3.15.2-rc.1", "3.15.2-rc.2"));
    }

    #[test]
    fn is_newer_handles_build_metadata_as_equal_when_triples_match() {
        // Build metadata MUST be ignored per semver §10. Stripping
        // it before comparison achieves this.
        assert!(!is_newer("3.15.2", "3.15.2+sha.deadbeef"));
        assert!(!is_newer("3.15.2+nightly.41", "3.15.2+nightly.42"));
    }

    #[cfg(windows)]
    #[test]
    fn parse_sha256_sidecar_accepts_cargo_dist_format() {
        // cargo-dist publishes lines like:
        //   "<hex>  *<filename>"  (two spaces, asterisk-prefixed name)
        // (per the parallel implementation in
        // .github/workflows/windows-installers.yml:215-220).
        let line = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789  *tr300-x86_64-pc-windows-msvc.msi";
        assert_eq!(
            parse_sha256_sidecar(line).as_deref(),
            Some("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"),
        );
    }

    #[cfg(windows)]
    #[test]
    fn parse_sha256_sidecar_accepts_no_asterisk_variant() {
        // Some sha256sum invocations omit the asterisk binary-mode
        // marker — accept either form.
        let line = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef tr300.msi";
        assert_eq!(
            parse_sha256_sidecar(line).as_deref(),
            Some("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"),
        );
    }

    #[cfg(windows)]
    #[test]
    fn parse_sha256_sidecar_normalizes_to_lowercase() {
        let line = "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789  *foo.msi";
        assert_eq!(
            parse_sha256_sidecar(line).as_deref(),
            Some("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"),
        );
    }

    #[cfg(windows)]
    #[test]
    fn parse_sha256_sidecar_rejects_wrong_length() {
        // Too short.
        assert_eq!(parse_sha256_sidecar("abcdef  *foo.msi"), None);
        // Too long.
        let too_long = format!("{}0  *foo.msi", "a".repeat(64));
        assert_eq!(parse_sha256_sidecar(&too_long), None);
        // Empty.
        assert_eq!(parse_sha256_sidecar(""), None);
    }

    #[cfg(windows)]
    #[test]
    fn parse_sha256_sidecar_rejects_non_hex_chars() {
        // 64 chars but `g` is not a hex digit.
        let bad = format!("{}  *foo.msi", "g".repeat(64));
        assert_eq!(parse_sha256_sidecar(&bad), None);
    }

    #[cfg(windows)]
    #[test]
    fn compute_sha256_matches_known_value() {
        // Empty file → known SHA256 of the empty input.
        let dir = std::env::temp_dir().join(format!("tr300-update-tests-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("empty.bin");
        std::fs::write(&path, b"").unwrap();
        let hash = compute_sha256(&path).unwrap();
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
