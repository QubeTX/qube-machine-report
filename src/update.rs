// Self-update for TR-300
//
// Checks the GitHub releases API for a newer version and dispatches to an
// installer strategy that matches how this binary was installed. On Windows
// (v3.15.0+), the four first-class installers (MSI Global, MSI Corporate,
// EXE Global, EXE Corporate) write a HKCU\Software\TR300\InstallSource
// registry marker, which `detect_install_origin()` reads to pick the matching
// MSI/EXE for in-place upgrade. Cargo, cargo-dist, and macOS PKG installs use
// their own durable metadata/receipt. Ambiguous or portable origins do not
// mutate the machine.

use crate::config::{Config, OutputFormat};
use crate::VERSION;

#[cfg(target_os = "macos")]
use crate::collectors::command::{run_output, CommandTimeout};

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

static JSON_UPDATE_MODE: AtomicBool = AtomicBool::new(false);

/// GitHub API endpoint for the latest release.
const RELEASES_URL: &str =
    "https://api.github.com/repos/QubeTX/qube-machine-report/releases/latest";

const RELEASE_DOWNLOAD_BASE: &str =
    "https://github.com/QubeTX/qube-machine-report/releases/download";
const SHELL_INSTALLER_ASSET: &str = "tr300-installer.sh";
const PS_INSTALLER_ASSET: &str = "tr300-installer.ps1";
const MSI_GLOBAL_ASSET: &str = "tr300-x86_64-pc-windows-msvc.msi";
const MSI_CORPORATE_ASSET: &str = "tr300-x86_64-pc-windows-msvc-corporate.msi";
const EXE_GLOBAL_ASSET: &str = "tr300-x86_64-pc-windows-msvc-setup.exe";
const EXE_CORPORATE_ASSET: &str = "tr300-x86_64-pc-windows-msvc-corporate-setup.exe";
const MAC_DMG_ASSET: &str = "tr300-universal-apple-darwin.dmg";
#[cfg(any(test, target_os = "macos"))]
const MAC_PKG_ID: &str = "com.qubetx.tr300.pkg";

/// Crate name for cargo install.
const CRATE_NAME: &str = "tr300";

const MANUAL_INSTALL_URL: &str = "https://github.com/QubeTX/qube-machine-report#installation";
const RELEASES_PAGE: &str = "https://github.com/QubeTX/qube-machine-report/releases/latest";

/// Public docs and filenames stay versionless, but an update transaction pins
/// every downloaded byte to the immutable tag returned by the releases API.
fn release_asset_url(version: &str, asset: &str) -> String {
    format!("{RELEASE_DOWNLOAD_BASE}/v{version}/{asset}")
}

// ── Strategy types ─────────────────────────────────────────────────

/// Ordered candidate strategies for updating the binary.
///
/// For Windows MSI / EXE installer strategies (v3.15.0+), the runner picks
/// exactly one strategy based on `detect_install_origin()` and does NOT
/// fall back to a different installer type on failure — re-running a
/// different product would create coexistence problems (two ARP entries,
/// PATH ordering decides which wins). Cargo and cargo-dist users also get only
/// their detected channel; multiple launchers such as curl/wget may implement
/// that one channel, but no strategy crosses into another install method.
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
    /// Reopens the signed universal DMG and waits for Apple Installer.
    MacDmg,
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
            UpdateStrategy::MacDmg => "macOS DMG / PKG installer",
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
            UpdateStrategy::MacDmg => "mac_dmg_pkg",
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

/// Precise, cross-platform installation channel. An update never crosses from
/// one channel to another; ambiguous/portable installs fail safely with a
/// recovery link instead of creating a second copy.
// Each target constructs only its native subset, while shared matching/JSON
// code deliberately retains the complete cross-platform taxonomy.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallChannel {
    MsiGlobal,
    MsiCorporate,
    ExeGlobal,
    ExeCorporate,
    PowerShellInstaller,
    ShellInstaller,
    Cargo,
    MacPkg,
    Unknown,
}

impl InstallChannel {
    fn json_id(self) -> &'static str {
        match self {
            Self::MsiGlobal => "msi-global",
            Self::MsiCorporate => "msi-corporate",
            Self::ExeGlobal => "exe-global",
            Self::ExeCorporate => "exe-corporate",
            Self::PowerShellInstaller => "powershell-installer",
            Self::ShellInstaller => "shell-installer",
            Self::Cargo => "cargo",
            Self::MacPkg => "macos-dmg-pkg",
            Self::Unknown => "unknown",
        }
    }
}

/// Where this binary was installed from, on Windows.
///
/// Determines which installer `tr300 update` downloads and re-runs for an
/// in-place upgrade. First-class installers (the four MSI/EXE variants)
/// write a `HKCU\Software\TR300\InstallSource` registry marker on install;
/// `detect_install_origin()` reads that marker. Receipt/metadata detection
/// later distinguishes Cargo from cargo-dist without trusting the shared path.
#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum InstallOrigin {
    /// `C:\Program Files\tr300\bin\tr300.exe`, installed from wix/main.wxs.
    MsiGlobal,
    /// `%LocalAppData%\Programs\tr300\bin\tr300.exe`, from wix/corporate.wxs.
    MsiCorporate,
    /// `C:\Program Files\tr300\bin\tr300.exe`, from inno/global.iss.
    ExeGlobal,
    /// `%LocalAppData%\Programs\tr300\bin\tr300.exe`, from inno/corporate.iss.
    ExeCorporate,
    /// `~\.cargo\bin\tr300.exe` — installed via `cargo install` or the
    /// cargo-dist PowerShell installer. Receipt/Cargo metadata distinguishes
    /// the precise channel before strategy selection.
    CargoOrInstaller,
    /// Couldn't determine origin (custom install location, portable use,
    /// etc.). No update mutation is attempted.
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

#[derive(Debug)]
enum StrategyError {
    /// Required launcher/tool is unavailable or could not be spawned.
    Preflight(String),
    /// Strategy launched and exited non-zero.
    Runtime(String),
    /// A write, staged file, or launcher was blocked by endpoint/filesystem
    /// policy. Stop instead of trying more write-heavy strategies.
    PolicyBlocked(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttemptKind {
    Skipped,
    Failed,
    Blocked,
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

    let channel = detect_install_channel();
    let strategies = build_strategy_list(channel);
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
            if failure.attempts.is_empty() {
                println!(
                    "  {} {}",
                    red(fail_icon(config), config),
                    red(
                        "No update was attempted because the installation channel is unknown or conflicting.",
                        config,
                    ),
                );
            } else {
                println!(
                    "  {} {}",
                    red(fail_icon(config), config),
                    red("Update failed. Strategies attempted:", config),
                );
                for record in &failure.attempts {
                    let kind = match record.kind {
                        AttemptKind::Skipped => "skipped",
                        AttemptKind::Failed => "failed",
                        AttemptKind::Blocked => "blocked",
                    };
                    println!(
                        "      · {} — {}: {}",
                        record.strategy.label(),
                        kind,
                        record.message
                    );
                }
            }
            println!();
            println!("  Your existing installation was left in place.");
            if let Some(asset) = recovery_asset_for_channel(channel) {
                println!(
                    "  Matching v{} installer: {}",
                    latest,
                    release_asset_url(&latest, asset)
                );
            }
            println!("  Fresh installer: {}", MANUAL_INSTALL_URL);
            println!("  Official latest release: {}", RELEASES_PAGE);
            2
        }
    }
}

// ── JSON output mode ───────────────────────────────────────────────

fn run_json() -> i32 {
    JSON_UPDATE_MODE.store(true, Ordering::Relaxed);
    let channel = detect_install_channel();
    let latest = match fetch_latest_version() {
        Ok(v) => v,
        Err(e) => {
            let mut payload = serde_json::json!({
                "action": "update",
                "success": false,
                "message": format!("Failed to check for updates: {}", e),
                "current_version": VERSION,
            });
            inject_update_context(&mut payload, channel, true);
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
        inject_update_context(&mut payload, channel, false);
        println!("{}", payload);
        return 0;
    }

    let strategies = build_strategy_list(channel);
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
            inject_update_context(&mut payload, channel, false);
            println!("{}", payload);
            0
        }
        Err(failure) => {
            let mut payload = update_failure_payload(&current, &latest, &failure, channel);
            inject_update_context(&mut payload, channel, true);
            println!("{}", payload);
            2
        }
    }
}

fn update_failure_payload(
    current: &str,
    latest: &str,
    failure: &UpdateFailure,
    channel: InstallChannel,
) -> serde_json::Value {
    let attempts: Vec<serde_json::Value> = failure
        .attempts
        .iter()
        .map(|record| {
            serde_json::json!({
                "strategy": record.strategy.json_id(),
                "result": match record.kind {
                    AttemptKind::Skipped => "skipped",
                    AttemptKind::Failed => "failed",
                    AttemptKind::Blocked => "blocked",
                },
                "message": record.message,
            })
        })
        .collect();
    let blocked = failure
        .attempts
        .iter()
        .any(|attempt| attempt.kind == AttemptKind::Blocked);
    let message = if failure.attempts.is_empty() {
        "Update not attempted because the installation channel is unknown or conflicting"
    } else {
        "Update failed; see attempts"
    };
    let mut payload = serde_json::json!({
        "action": "update",
        "success": false,
        "message": message,
        "current_version": current,
        "latest_version": latest,
        "update_available": true,
        "attempts": attempts,
        "manual_install_url": MANUAL_INSTALL_URL,
        "recovery_url": RELEASES_PAGE,
        "requires_user_action": true,
    });
    if let Some(asset) = recovery_asset_for_channel(channel) {
        payload["exact_installer_url"] =
            serde_json::Value::String(release_asset_url(latest, asset));
    }
    if blocked {
        payload["official_releases_url"] = serde_json::Value::String(RELEASES_PAGE.to_string());
    }
    payload
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

fn inject_update_context(
    payload: &mut serde_json::Value,
    channel: InstallChannel,
    requires_user_action: bool,
) {
    inject_install_origin(payload);
    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "install_channel".to_string(),
            serde_json::Value::String(channel.json_id().to_string()),
        );
        obj.insert(
            "recovery_url".to_string(),
            serde_json::Value::String(RELEASES_PAGE.to_string()),
        );
        obj.insert(
            "requires_user_action".to_string(),
            serde_json::Value::Bool(requires_user_action),
        );
    }
}

// ── Version check ──────────────────────────────────────────────────

/// Turn a `ureq` error from the releases-API request into an actionable
/// message. The most common intermittent failure is GitHub's unauthenticated
/// rate limit (60 requests/hour per IP) — worth naming explicitly so a user
/// who "ran update and it didn't work" knows to just wait.
fn classify_fetch_error(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, ref resp) => {
            http_status_message(code, resp.header("x-ratelimit-remaining"))
        }
        ureq::Error::Transport(t) => format!("Network error reaching GitHub: {}", t),
    }
}

/// User-facing message for an HTTP error from the releases API. Pure +
/// testable (the live `ureq::Error` matching lives in `classify_fetch_error`).
fn http_status_message(code: u16, ratelimit_remaining: Option<&str>) -> String {
    if code == 403 && ratelimit_remaining == Some("0") {
        "GitHub API rate limit exceeded (unauthenticated requests are capped at 60/hour per IP). Wait for the limit to reset and try again, or update manually.".to_string()
    } else {
        format!(
            "GitHub API returned HTTP {} when checking for updates",
            code
        )
    }
}

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
        .map_err(classify_fetch_error)?;

    let body: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let tag = body["tag_name"]
        .as_str()
        .ok_or("Missing tag_name in response")?;

    // Strip leading 'v' if present, then reject malformed release tags rather
    // than letting a lossy numeric parser silently reinterpret them.
    let version = tag.strip_prefix('v').unwrap_or(tag);
    if parse_numeric_version(version).is_none() {
        return Err(format!(
            "Latest release has an invalid version tag: {tag:?}"
        ));
    }
    Ok(version.to_string())
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

    let (Some(c), Some(l)) = (
        parse_numeric_version(current_stripped),
        parse_numeric_version(latest_stripped),
    ) else {
        return false;
    };

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

fn parse_numeric_version(version: &str) -> Option<Vec<u64>> {
    let parts: Vec<u64> = version
        .split('.')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    (!parts.is_empty()).then_some(parts)
}

// ── Strategy ordering ───────────────────────────────────────────────

fn tool_exists(tool: &str) -> bool {
    crate::collectors::command::run_output(
        tool,
        ["--version"],
        crate::collectors::command::CommandTimeout::Slow,
    )
    .is_some_and(|output| output.status.success())
}

fn build_strategy_list(channel: InstallChannel) -> Vec<UpdateStrategy> {
    match channel {
        InstallChannel::MsiGlobal => vec![UpdateStrategy::MsiGlobal],
        InstallChannel::MsiCorporate => vec![UpdateStrategy::MsiCorporate],
        InstallChannel::ExeGlobal => vec![UpdateStrategy::ExeGlobal],
        InstallChannel::ExeCorporate => vec![UpdateStrategy::ExeCorporate],
        InstallChannel::Cargo => vec![UpdateStrategy::Cargo],
        InstallChannel::PowerShellInstaller => vec![
            UpdateStrategy::InstallerPowerShell,
            UpdateStrategy::InstallerPwsh,
        ],
        InstallChannel::ShellInstaller => {
            vec![UpdateStrategy::InstallerCurl, UpdateStrategy::InstallerWget]
        }
        InstallChannel::MacPkg => vec![UpdateStrategy::MacDmg],
        InstallChannel::Unknown => Vec::new(),
    }
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
    eprintln!("Updating Rust toolchain (rustup update stable)…");
    let mut command = Command::new("rustup");
    command.args(["update", "stable"]);
    suppress_stdout_for_json(&mut command);
    let _ = command.status();
}

fn execute_update(
    latest: &str,
    strategies: &[UpdateStrategy],
) -> Result<UpdateStrategy, UpdateFailure> {
    execute_update_with(latest, strategies, try_strategy)
}

fn execute_update_with<F>(
    latest: &str,
    strategies: &[UpdateStrategy],
    mut attempt: F,
) -> Result<UpdateStrategy, UpdateFailure>
where
    F: FnMut(UpdateStrategy, &str) -> Result<(), StrategyError>,
{
    let mut attempts = Vec::new();
    for &strategy in strategies {
        match attempt(strategy, latest) {
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
            Err(StrategyError::PolicyBlocked(message)) => {
                eprintln!("  · {} blocked: {}", strategy.label(), message);
                attempts.push(AttemptRecord {
                    strategy,
                    kind: AttemptKind::Blocked,
                    message,
                });
                // A policy/antivirus block is a machine-level signal. Trying
                // another downloader or replacement method can repeat the
                // freeze and does not make an unsafe direct overwrite sound.
                break;
            }
        }
    }
    Err(UpdateFailure { attempts })
}

fn try_strategy(strategy: UpdateStrategy, latest: &str) -> Result<(), StrategyError> {
    match strategy {
        UpdateStrategy::Cargo => {
            rustup_update_stable_best_effort();
            run_command_status(
                "cargo",
                &[
                    "install",
                    CRATE_NAME,
                    "--version",
                    latest,
                    "--force",
                    "--locked",
                ],
            )?;
            // Cargo exit 0 doesn't guarantee the running binary changed (for
            // example another tr300 may be earlier on PATH). Verify and fail
            // this channel on mismatch; never switch to a prebuilt installer.
            verify_cargo_post_install(latest)
        }
        UpdateStrategy::InstallerCurl => {
            try_installer_curl(latest)?;
            verify_installer_post_install(latest, "shell installer")
        }
        UpdateStrategy::InstallerWget => {
            try_installer_wget(latest)?;
            verify_installer_post_install(latest, "shell installer")
        }
        UpdateStrategy::InstallerPowerShell => {
            try_installer_powershell("powershell", latest)?;
            verify_installer_post_install(latest, "PowerShell installer")
        }
        UpdateStrategy::InstallerPwsh => {
            try_installer_powershell("pwsh", latest)?;
            verify_installer_post_install(latest, "PowerShell installer")
        }
        UpdateStrategy::MsiGlobal => {
            try_msi_install(&release_asset_url(latest, MSI_GLOBAL_ASSET), latest)
        }
        UpdateStrategy::MsiCorporate => {
            try_msi_install(&release_asset_url(latest, MSI_CORPORATE_ASSET), latest)
        }
        UpdateStrategy::ExeGlobal => {
            try_exe_install(&release_asset_url(latest, EXE_GLOBAL_ASSET), latest)
        }
        UpdateStrategy::ExeCorporate => {
            try_exe_install(&release_asset_url(latest, EXE_CORPORATE_ASSET), latest)
        }
        UpdateStrategy::MacDmg => try_macos_dmg_install(latest),
    }
}

fn recovery_asset_for_channel(channel: InstallChannel) -> Option<&'static str> {
    match channel {
        InstallChannel::MsiGlobal => Some(MSI_GLOBAL_ASSET),
        InstallChannel::MsiCorporate => Some(MSI_CORPORATE_ASSET),
        InstallChannel::ExeGlobal => Some(EXE_GLOBAL_ASSET),
        InstallChannel::ExeCorporate => Some(EXE_CORPORATE_ASSET),
        InstallChannel::PowerShellInstaller => Some(PS_INSTALLER_ASSET),
        InstallChannel::ShellInstaller => Some(SHELL_INSTALLER_ASSET),
        InstallChannel::MacPkg => Some(MAC_DMG_ASSET),
        InstallChannel::Cargo | InstallChannel::Unknown => None,
    }
}

fn run_command_status(launcher: &str, args: &[&str]) -> Result<(), StrategyError> {
    run_command_status_inner(launcher, args, None)
}

fn run_command_status_with_env(
    launcher: &str,
    args: &[&str],
    env_name: &str,
    env_value: &str,
) -> Result<(), StrategyError> {
    run_command_status_inner(launcher, args, Some((env_name, env_value)))
}

fn run_command_status_inner(
    launcher: &str,
    args: &[&str],
    extra_env: Option<(&str, &str)>,
) -> Result<(), StrategyError> {
    let mut command = Command::new(launcher);
    command.args(args);
    if let Some((name, value)) = extra_env {
        command.env(name, value);
    }
    suppress_stdout_for_json(&mut command);
    match command.status() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(StrategyError::Preflight(
            format!("{} not on PATH", launcher),
        )),
        Err(e) if likely_endpoint_policy_error(&e) => Err(StrategyError::PolicyBlocked(format!(
            "endpoint security or execution policy blocked `{}`: {}. TR-300 stopped without trying another updater; the update was not reported as successful",
            launcher, e
        ))),
        Err(e) => Err(StrategyError::Preflight(format!("Failed to spawn {}: {}", launcher, e))),
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(StrategyError::Runtime(format!(
            "{} exited with code {}",
            launcher,
            status.code().unwrap_or(-1)
        ))),
    }
}

fn likely_endpoint_policy_error(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::PermissionDenied
            | std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::NotFound
    ) || matches!(
        error.raw_os_error(),
        // Windows: access denied, sharing/lock violation, virus detected or
        // removed, operation aborted, and operation cancelled by policy/UAC.
        Some(5 | 32 | 33 | 225 | 226 | 995 | 1223)
    )
}

#[cfg(any(windows, target_os = "macos"))]
fn prelaunch_installer_io_error(
    operation: &str,
    path: &std::path::Path,
    official_url: &str,
    error: std::io::Error,
) -> StrategyError {
    let detail = format!(
        "{} at {} failed: {}. The installer was not launched and the existing TR-300 installation was left in place",
        operation,
        path.display(),
        error
    );
    if likely_endpoint_policy_error(&error) {
        StrategyError::PolicyBlocked(format!(
            "{}. Endpoint security or filesystem policy may be responsible. Ask IT to allow the official asset or install it manually: {}",
            detail, official_url
        ))
    } else {
        StrategyError::Runtime(detail)
    }
}

#[cfg(unix)]
fn try_installer_curl(latest: &str) -> Result<(), StrategyError> {
    if !tool_exists("curl") {
        return Err(StrategyError::Preflight("curl not on PATH".into()));
    }
    if !tool_exists("bash") {
        return Err(StrategyError::Preflight("bash not on PATH".into()));
    }
    let script = format!(
        "set -euo pipefail; curl --proto '=https' --tlsv1.2 -fLsS {} | sh",
        release_asset_url(latest, SHELL_INSTALLER_ASSET)
    );
    let prefix = cargo_dist_install_prefix().ok_or_else(|| {
        StrategyError::Preflight("matching cargo-dist receipt is unavailable".into())
    })?;
    run_command_status_with_env(
        "bash",
        &["-c", &script],
        "CARGO_DIST_FORCE_INSTALL_DIR",
        &prefix,
    )
}

#[cfg(not(unix))]
fn try_installer_curl(_latest: &str) -> Result<(), StrategyError> {
    Err(StrategyError::Preflight(
        "curl installer is Unix-only".into(),
    ))
}

#[cfg(unix)]
fn try_installer_wget(latest: &str) -> Result<(), StrategyError> {
    if !tool_exists("wget") {
        return Err(StrategyError::Preflight("wget not on PATH".into()));
    }
    if !tool_exists("bash") {
        return Err(StrategyError::Preflight("bash not on PATH".into()));
    }
    let script = format!(
        "set -euo pipefail; wget -qO- {} | sh",
        release_asset_url(latest, SHELL_INSTALLER_ASSET)
    );
    let prefix = cargo_dist_install_prefix().ok_or_else(|| {
        StrategyError::Preflight("matching cargo-dist receipt is unavailable".into())
    })?;
    run_command_status_with_env(
        "bash",
        &["-c", &script],
        "CARGO_DIST_FORCE_INSTALL_DIR",
        &prefix,
    )
}

#[cfg(not(unix))]
fn try_installer_wget(_latest: &str) -> Result<(), StrategyError> {
    Err(StrategyError::Preflight(
        "wget installer is Unix-only".into(),
    ))
}

#[cfg(windows)]
fn try_installer_powershell(launcher: &str, latest: &str) -> Result<(), StrategyError> {
    let script = format!(
        "$ErrorActionPreference='Stop'; irm {} | iex",
        release_asset_url(latest, PS_INSTALLER_ASSET)
    );
    let prefix = cargo_dist_install_prefix().ok_or_else(|| {
        StrategyError::Preflight("matching cargo-dist receipt is unavailable".into())
    })?;
    run_command_status_with_env(
        launcher,
        &[
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ],
        "CARGO_DIST_FORCE_INSTALL_DIR",
        &prefix,
    )
}

#[cfg(not(windows))]
fn try_installer_powershell(_launcher: &str, _latest: &str) -> Result<(), StrategyError> {
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
#[cfg(windows)]
const MSI_EXIT_REBOOT_INITIATED: i32 = 1641;
/// Windows Installer policy explicitly forbids this installation.
#[cfg(windows)]
const MSI_EXIT_INSTALL_REJECTED_BY_POLICY: i32 = 1625;
#[cfg(any(windows, target_os = "macos"))]
const MAX_INSTALLER_BYTES: u64 = 256 * 1024 * 1024;
#[cfg(any(windows, target_os = "macos"))]
const MAX_SIDECAR_BYTES: u64 = 16 * 1024;

/// Download a file from `url` to `path` over HTTPS. Used by the MSI/EXE
/// strategies to fetch the matching installer into a private randomized
/// staging directory before launching it. TLS validation is enforced by
/// `ureq`; the caller then verifies the release sidecar to detect corruption
/// or an installer/sidecar mismatch. The sidecar is not an independent
/// signature and does not replace HTTPS origin authentication.
#[cfg(any(windows, target_os = "macos"))]
fn download_to_file(url: &str, path: &std::path::Path) -> Result<(), StrategyError> {
    use std::io::Read;

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(120))
        .build();

    let resp = agent
        .get(url)
        .set("User-Agent", &format!("tr300/{}", VERSION))
        .call()
        .map_err(|e| StrategyError::Runtime(format!("Request failed: {}", e)))?;

    if resp
        .header("Content-Length")
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > MAX_INSTALLER_BYTES)
    {
        return Err(StrategyError::Runtime(format!(
            "Installer exceeds the {} MiB safety limit",
            MAX_INSTALLER_BYTES / 1024 / 1024
        )));
    }

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            prelaunch_installer_io_error("creating the staged installer", path, url, error)
        })?;
    let reader = resp.into_reader();
    let mut reader = reader.take(MAX_INSTALLER_BYTES + 1);
    let write_result = std::io::copy(&mut reader, &mut file).and_then(|bytes| {
        if bytes > MAX_INSTALLER_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "installer exceeded the 256 MiB safety limit",
            ));
        }
        file.sync_all()
    });
    if let Err(error) = write_result {
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(prelaunch_installer_io_error(
            "writing or syncing the staged installer",
            path,
            url,
            error,
        ));
    }
    Ok(())
}

/// Fetch the cargo-dist `.sha256` sidecar at `<url>.sha256` and return
/// the file contents.
///
/// Format: `<lowercase-64-char-hex>  *<filename>` per cargo-dist's
/// `dist-manifest.json` generation and the parallel implementation in
/// `.github/workflows/windows-installers.yml`. Tolerant of trailing
/// whitespace / missing asterisk via `parse_sha256_sidecar`.
#[cfg(any(windows, target_os = "macos"))]
fn fetch_sha256_sidecar(url: &str) -> Result<String, String> {
    use std::io::Read;

    let sidecar_url = format!("{}.sha256", url);
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(30))
        .build();
    let resp = agent
        .get(&sidecar_url)
        .set("User-Agent", &format!("tr300/{}", VERSION))
        .call()
        .map_err(|e| format!("Sidecar request failed ({}): {}", sidecar_url, e))?;
    let mut body = String::new();
    resp.into_reader()
        .take(MAX_SIDECAR_BYTES + 1)
        .read_to_string(&mut body)
        .map_err(|e| format!("Failed to read sidecar body: {}", e))?;
    if body.len() as u64 > MAX_SIDECAR_BYTES {
        return Err("SHA256 sidecar exceeded the 16 KiB safety limit".to_string());
    }
    Ok(body)
}

/// Extract the 64-char hex from a `.sha256` sidecar line. Returns
/// `None` when the first whitespace-separated token is not exactly
/// 64 hex characters.
#[cfg(any(windows, target_os = "macos", test))]
fn parse_sha256_sidecar(content: &str) -> Option<String> {
    content
        .split_whitespace()
        .next()
        .filter(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()))
        .map(|s| s.to_lowercase())
}

/// Compute the SHA-256 of `path`, returning the lowercase hex.
#[cfg(any(windows, target_os = "macos", test))]
fn compute_sha256(path: &std::path::Path) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// Fetch the `.sha256` sidecar, compute the SHA-256 of the downloaded
/// installer, refuse to proceed on mismatch.
///
/// Detects a corrupted download or a mismatch between the installer and its
/// published release sidecar. Because both files come from the same HTTPS
/// release origin, this is an integrity check rather than independent artifact
/// authentication; a future signed-release design would be a separate trust
/// layer.
#[cfg(any(windows, target_os = "macos"))]
fn verify_checksum(
    installer_path: &std::path::Path,
    installer_url: &str,
) -> Result<(), StrategyError> {
    eprintln!("  Verifying SHA256 checksum...");
    let sidecar_content = fetch_sha256_sidecar(installer_url).map_err(StrategyError::Runtime)?;
    let expected = parse_sha256_sidecar(&sidecar_content).ok_or_else(|| {
        StrategyError::Runtime(format!(
            "Malformed .sha256 sidecar from {}.sha256: {:?}",
            installer_url, sidecar_content
        ))
    })?;
    let actual = compute_sha256(installer_path).map_err(|error| {
        prelaunch_installer_io_error(
            "reading the staged installer for checksum verification",
            installer_path,
            installer_url,
            error,
        )
    })?;
    checksum_verdict(&actual, &expected).map_err(StrategyError::Runtime)
}

/// Compare a computed SHA-256 against the expected sidecar hash, refusing on
/// mismatch. Separated from the network fetch + file read in `verify_checksum`
/// so the load-bearing refusal-on-mismatch is unit-testable on any target.
#[cfg(any(target_os = "windows", target_os = "macos", test))]
fn checksum_verdict(actual: &str, expected: &str) -> Result<(), String> {
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!(
            "SHA256 mismatch — refusing to run installer.\n         Expected: {}\n         Got:      {}\n         This indicates a corrupted or mismatched release asset.",
            expected, actual
        ))
    }
}

#[cfg(any(windows, target_os = "macos", test))]
struct StagedInstaller {
    dir: tempfile::TempDir,
    path: std::path::PathBuf,
}

#[cfg(any(windows, target_os = "macos", test))]
impl StagedInstaller {
    fn new(extension: &str) -> std::io::Result<Self> {
        let dir = tempfile::Builder::new().prefix("tr300-update-").tempdir()?;
        let path = dir.path().join(format!("installer.{}", extension));
        Ok(Self { dir, path })
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }

    #[cfg(target_os = "macos")]
    fn dir(&self) -> &std::path::Path {
        self.dir.path()
    }

    fn close(self) -> std::io::Result<()> {
        self.dir.close()
    }
}

#[cfg(any(windows, target_os = "macos", test))]
fn finish_staged_attempt(
    staged: StagedInstaller,
    result: Result<(), StrategyError>,
) -> Result<(), StrategyError> {
    let staging_dir = staged
        .path()
        .parent()
        .unwrap_or_else(|| staged.path())
        .to_path_buf();
    match staged.close() {
        Ok(()) => result,
        Err(cleanup_error) => {
            let cleanup_note = format!(
                "staging cleanup at {} also failed: {}. Security software may still be holding the file; it is safe to remove that tr300-update directory after the scanner releases it",
                staging_dir.display(),
                cleanup_error
            );
            match result {
                // The installed version was already re-executed and verified.
                // A leftover temp directory must not turn a real update into a
                // false failure, but it should never be silent.
                Ok(()) => {
                    eprintln!("  · warning: {}", cleanup_note);
                    Ok(())
                }
                Err(StrategyError::Preflight(message)) => Err(StrategyError::Preflight(format!(
                    "{}; {}",
                    message, cleanup_note
                ))),
                Err(StrategyError::Runtime(message)) => Err(StrategyError::Runtime(format!(
                    "{}; {}",
                    message, cleanup_note
                ))),
                Err(StrategyError::PolicyBlocked(message)) => Err(StrategyError::PolicyBlocked(
                    format!("{}; {}", message, cleanup_note),
                )),
            }
        }
    }
}

/// Whether the freshly-installed `--version` string matches the expected
/// release tag. Both sides are stripped of any prerelease/build-metadata
/// suffix before comparison, and an empty `installed` never matches (covers
/// the `--version` parse failing). Pure + platform-independent.
fn post_install_version_ok(installed: &str, expected: &str) -> bool {
    let installed_stripped = strip_prerelease_metadata(installed);
    let expected_stripped = strip_prerelease_metadata(expected);
    !installed_stripped.is_empty() && installed_stripped == expected_stripped
}

/// Re-exec the running binary with `--version` and return the parsed version
/// (the last whitespace token of `tr300 X.Y.Z`). `None` if the spawn fails, the
/// process errors, or the output doesn't parse. Cross-platform — used by both
/// the Windows installer verify and the cargo-path verify.
fn reexec_installed_version() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let output = Command::new(&exe).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let v = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .last()
        .unwrap_or("")
        .trim()
        .to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

/// Confirm a pinned `cargo install tr300 --version ... --force --locked`
/// update actually landed.
///
/// Cargo can report success without replacing the running path when metadata
/// or PATH is inconsistent. Without this check, the updater would print
/// "Updated to vX" while `tr300 --version` is unchanged. Re-exec `--version`;
/// a mismatch fails this channel without falling into another one.
fn verify_cargo_post_install(expected: &str) -> Result<(), StrategyError> {
    match reexec_installed_version() {
        Some(installed) if post_install_version_ok(&installed, expected) => Ok(()),
        Some(installed) => Err(StrategyError::Runtime(format!(
            "cargo install reported success but `tr300 --version` still reports v{installed} (expected v{expected}). Another tr300 may be earlier on PATH; this Cargo-channel update stopped without switching installers."
        ))),
        None => Err(StrategyError::Runtime(
            "cargo install reported success but `tr300 --version` could not be run to confirm the new version; this Cargo-channel update stopped without switching installers.".to_string(),
        )),
    }
}

/// Confirm a cargo-dist shell/PowerShell installer actually replaced the
/// running installation. Installer exit code 0 alone is not sufficient: a
/// PATH conflict or locked destination can leave the old executable in place.
fn verify_installer_post_install(expected: &str, label: &str) -> Result<(), StrategyError> {
    match reexec_installed_version() {
        Some(installed) if post_install_version_ok(&installed, expected) => Ok(()),
        Some(installed) => Err(StrategyError::Runtime(format!(
            "{label} reported success but the running install still reports v{installed} (expected v{expected})"
        ))),
        None => Err(StrategyError::Runtime(format!(
            "{label} reported success but `tr300 --version` could not be run to verify v{expected}"
        ))),
    }
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
    let installed = reexec_installed_version()
        .ok_or_else(|| "Failed to run `tr300 --version` to confirm the install".to_string())?;
    if post_install_version_ok(&installed, expected) {
        Ok(())
    } else {
        Err(format!(
            "Installer exited successfully but `tr300 --version` still reports v{} (expected v{}). The installed binary may be locked by another process — close other tr300 windows / shells and re-run, or reboot to let Windows finish a deferred file replace.",
            installed, expected
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
    let staged = StagedInstaller::new("msi").map_err(|error| {
        prelaunch_installer_io_error(
            "creating a private update staging directory",
            &std::env::temp_dir(),
            url,
            error,
        )
    })?;
    let temp_path = staged.path();

    let result = (|| -> Result<(), StrategyError> {
        eprintln!("  Downloading MSI installer...");
        download_to_file(url, temp_path)?;

        verify_checksum(temp_path, url)?;

        eprintln!("  Launching Windows Installer...");
        // /passive shows a progress dialog with no user interaction; /norestart
        // suppresses any reboot prompt (we don't need a reboot for a simple file
        // replace). For the Global perMachine MSI, msiexec triggers UAC before
        // doing anything; for the Corporate perUser MSI, it installs silently
        // into LocalAppData with no elevation prompt.
        let mut command = Command::new("msiexec");
        command.args(["/i", &temp_path.to_string_lossy(), "/passive", "/norestart"]);
        suppress_stdout_for_json(&mut command);
        let status = command.status().map_err(|error| {
            prelaunch_installer_io_error("launching Windows Installer", temp_path, url, error)
        })?;

        let code = status.code().unwrap_or(-1);
        if code == MSI_EXIT_REBOOT_REQUIRED || code == MSI_EXIT_REBOOT_INITIATED {
            // Install completed but Restart Manager couldn't finalize a
            // file replace in-place. Surface this rather than silently
            // claiming success — `verify_post_install` would fail because
            // the on-disk binary is still old.
            return Err(StrategyError::Runtime(format!(
                "MSI install completed but requires a reboot to finalize (msiexec exit {}). Reboot, then verify with `tr300 --version`.",
                code
            )));
        }
        if code == MSI_EXIT_INSTALL_REJECTED_BY_POLICY {
            return Err(StrategyError::PolicyBlocked(format!(
                "Windows Installer rejected the update by system policy (msiexec exit {}). TR-300 stopped without trying another replacement method and will not claim success. Ask IT to approve the official installer: {}",
                MSI_EXIT_INSTALL_REJECTED_BY_POLICY, url
            )));
        }
        if !status.success() {
            return Err(StrategyError::Runtime(format!(
                "msiexec exited with code {} (likely user cancel, UAC denied, security software, or install error). The update was not verified; check the retained installation with `tr300 --version`",
                code,
            )));
        }

        verify_post_install(latest).map_err(StrategyError::Runtime)?;
        Ok(())
    })();

    finish_staged_attempt(staged, result)
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
    let staged = StagedInstaller::new("exe").map_err(|error| {
        prelaunch_installer_io_error(
            "creating a private update staging directory",
            &std::env::temp_dir(),
            url,
            error,
        )
    })?;
    let temp_path = staged.path();

    let result = (|| -> Result<(), StrategyError> {
        eprintln!("  Downloading EXE installer...");
        download_to_file(url, temp_path)?;

        verify_checksum(temp_path, url)?;

        eprintln!("  Launching Inno Setup installer...");
        // /SILENT shows a progress dialog but no wizard pages; /SUPPRESSMSGBOXES
        // suppresses non-critical message boxes; /NORESTART skips reboot prompts.
        let mut command = Command::new(temp_path);
        command.args([
            "/SILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/RESTARTEXITCODE=3010",
        ]);
        suppress_stdout_for_json(&mut command);
        let status = command.status().map_err(|error| {
            prelaunch_installer_io_error("launching the EXE installer", temp_path, url, error)
        })?;

        let code = status.code().unwrap_or(-1);
        if code == MSI_EXIT_REBOOT_REQUIRED {
            return Err(StrategyError::Runtime(
                "EXE install completed but requires a reboot to finalize. Reboot, then verify with `tr300 --version`.".to_string(),
            ));
        }
        if !status.success() {
            return Err(StrategyError::Runtime(format!(
                "EXE installer exited with code {} (likely user cancel, UAC denied, security software, or install error). The update was not verified; check the retained installation with `tr300 --version`",
                code,
            )));
        }

        verify_post_install(latest).map_err(StrategyError::Runtime)?;
        Ok(())
    })();

    finish_staged_attempt(staged, result)
}

fn suppress_stdout_for_json(command: &mut Command) {
    if JSON_UPDATE_MODE.load(Ordering::Relaxed) {
        command.stdout(std::process::Stdio::null());
    }
}

#[cfg(target_os = "macos")]
fn try_macos_dmg_install(latest: &str) -> Result<(), StrategyError> {
    let url = release_asset_url(latest, MAC_DMG_ASSET);
    let staged = StagedInstaller::new("dmg").map_err(|error| {
        prelaunch_installer_io_error(
            "creating a private update staging directory",
            &std::env::temp_dir(),
            &url,
            error,
        )
    })?;
    let dmg_path = staged.path().to_path_buf();
    let mount_path = staged.dir().join("mounted");

    let result = (|| -> Result<(), StrategyError> {
        std::fs::create_dir(&mount_path).map_err(|error| {
            prelaunch_installer_io_error(
                "creating the private DMG mount point",
                &mount_path,
                &url,
                error,
            )
        })?;

        eprintln!("  Downloading signed macOS DMG...");
        download_to_file(&url, &dmg_path)?;
        verify_checksum(&dmg_path, &url)?;
        run_command_status(
            "codesign",
            &[
                "--verify",
                "--deep",
                "--strict",
                &dmg_path.to_string_lossy(),
            ],
        )?;
        run_command_status(
            "xcrun",
            &["stapler", "validate", &dmg_path.to_string_lossy()],
        )?;
        run_command_status(
            "spctl",
            &[
                "--assess",
                "--type",
                "open",
                "--context",
                "context:primary-signature",
                "--verbose=4",
                &dmg_path.to_string_lossy(),
            ],
        )?;
        run_command_status(
            "hdiutil",
            &[
                "attach",
                "-nobrowse",
                "-readonly",
                "-mountpoint",
                &mount_path.to_string_lossy(),
                &dmg_path.to_string_lossy(),
            ],
        )?;

        let pkg_path = mount_path.join("tr300.pkg");
        let install_result = (|| -> Result<(), StrategyError> {
            if !pkg_path.is_file() {
                return Err(StrategyError::Runtime(format!(
                    "The verified DMG did not contain the expected {} package",
                    pkg_path.display()
                )));
            }
            run_command_status(
                "pkgutil",
                &["--check-signature", &pkg_path.to_string_lossy()],
            )?;
            run_command_status(
                "xcrun",
                &["stapler", "validate", &pkg_path.to_string_lossy()],
            )?;
            run_command_status(
                "spctl",
                &[
                    "--assess",
                    "--type",
                    "install",
                    "--verbose=4",
                    &pkg_path.to_string_lossy(),
                ],
            )?;
            eprintln!("  Opening Apple Installer; complete or cancel the prompts there...");
            run_command_status(
                "open",
                &["-W", "-a", "Installer", &pkg_path.to_string_lossy()],
            )?;
            run_command_status("pkgutil", &["--pkg-info", MAC_PKG_ID])?;
            verify_installer_post_install(latest, "macOS DMG / PKG installer")
        })();

        let detach_result =
            run_command_status("hdiutil", &["detach", &mount_path.to_string_lossy()]);
        match (install_result, detach_result) {
            (Err(error), _) => Err(error),
            (Ok(()), Err(error)) => Err(error),
            (Ok(()), Ok(())) => Ok(()),
        }
    })();

    finish_staged_attempt(staged, result)
}

#[cfg(not(target_os = "macos"))]
fn try_macos_dmg_install(_latest: &str) -> Result<(), StrategyError> {
    Err(StrategyError::Preflight(
        "DMG / PKG installer is macOS-only".into(),
    ))
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

// ── Cross-platform install-channel detection ──────────────────────

fn cargo_dist_receipt_path() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(std::path::PathBuf::from)
            .map(|path| path.join("tr300").join("tr300-receipt.json"))
    }
    #[cfg(not(windows))]
    {
        let config_home = std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| dirs::home_dir().map(|home| home.join(".config")))?;
        Some(config_home.join("tr300").join("tr300-receipt.json"))
    }
}

fn find_json_string<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    match value {
        serde_json::Value::Object(map) => map
            .get(key)
            .and_then(serde_json::Value::as_str)
            .or_else(|| map.values().find_map(|child| find_json_string(child, key))),
        serde_json::Value::Array(values) => {
            values.iter().find_map(|child| find_json_string(child, key))
        }
        _ => None,
    }
}

fn path_is_within(path: &std::path::Path, parent: &std::path::Path) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let parent = parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf());
    #[cfg(windows)]
    {
        let path = path
            .to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .to_ascii_lowercase();
        let parent = parent
            .to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .to_ascii_lowercase();
        path == parent || path.starts_with(&format!("{parent}\\"))
    }
    #[cfg(not(windows))]
    {
        path.starts_with(parent)
    }
}

/// A cargo-dist receipt is trusted only when it names this app/provider and
/// its recorded prefix contains the executable that is actually running. Its
/// modification time is evidence for resolving a shared Cargo-bin path: the
/// newest explicit Cargo or cargo-dist install owns the channel.
fn cargo_dist_receipt_evidence() -> Option<std::time::SystemTime> {
    let path = cargo_dist_receipt_path()?;
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return None;
    };
    let Ok(receipt) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return None;
    };
    let Ok(exe) = std::env::current_exe() else {
        return None;
    };
    cargo_dist_receipt_matches(&receipt, &exe)
        .then(|| std::fs::metadata(path).ok()?.modified().ok())
        .flatten()
}

fn cargo_dist_install_prefix() -> Option<String> {
    let contents = std::fs::read_to_string(cargo_dist_receipt_path()?).ok()?;
    let receipt: serde_json::Value = serde_json::from_str(&contents).ok()?;
    let exe = std::env::current_exe().ok()?;
    cargo_dist_receipt_matches(&receipt, &exe)
        .then(|| find_json_string(&receipt, "install_prefix").map(str::to_string))
        .flatten()
}

fn cargo_dist_receipt_matches(receipt: &serde_json::Value, exe: &std::path::Path) -> bool {
    if find_json_string(receipt, "source") != Some("cargo-dist") {
        return false;
    }
    if find_json_string(receipt, "app_name").or_else(|| find_json_string(receipt, "name"))
        != Some(CRATE_NAME)
    {
        return false;
    }
    let Some(prefix) = find_json_string(receipt, "install_prefix") else {
        return false;
    };
    path_is_within(exe, std::path::Path::new(prefix))
}

fn cargo_metadata_evidence() -> Option<std::time::SystemTime> {
    let home = std::env::var_os("CARGO_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::home_dir().map(|path| path.join(".cargo")))?;
    let Ok(exe) = std::env::current_exe() else {
        return None;
    };
    if !path_is_within(&exe, &home.join("bin")) {
        return None;
    }
    [".crates2.json", ".crates.toml"]
        .iter()
        .filter_map(|name| {
            let path = home.join(name);
            let contents = std::fs::read_to_string(&path).ok()?;
            contents.contains("tr300 ").then(|| {
                std::fs::metadata(path)
                    .ok()
                    .and_then(|metadata| metadata.modified().ok())
            })?
        })
        .max()
}

fn newest_metadata_channel(
    dist: Option<std::time::SystemTime>,
    cargo: Option<std::time::SystemTime>,
    dist_channel: InstallChannel,
) -> InstallChannel {
    match (dist, cargo) {
        (Some(_), None) => dist_channel,
        (None, Some(_)) => InstallChannel::Cargo,
        (Some(dist), Some(cargo)) if dist > cargo => dist_channel,
        (Some(dist), Some(cargo)) if cargo > dist => InstallChannel::Cargo,
        // Equal/coarse timestamps are conflicting evidence. Refuse to guess.
        _ => InstallChannel::Unknown,
    }
}

fn metadata_install_channel(dist_channel: InstallChannel) -> InstallChannel {
    newest_metadata_channel(
        cargo_dist_receipt_evidence(),
        cargo_metadata_evidence(),
        dist_channel,
    )
}

#[cfg(any(test, target_os = "macos"))]
fn pkgutil_field<'a>(output: &'a str, key: &str) -> Option<&'a str> {
    output.lines().find_map(|line| {
        let (candidate, value) = line.split_once(':')?;
        candidate
            .trim()
            .eq_ignore_ascii_case(key)
            .then(|| value.trim())
    })
}

#[cfg(any(test, target_os = "macos"))]
fn pkg_receipt_metadata_matches(
    package_info: &str,
    payload_files: &str,
    file_info: &str,
    expected_version: &str,
) -> bool {
    let package_id_matches = pkgutil_field(package_info, "package-id") == Some(MAC_PKG_ID);
    let package_version_matches = pkgutil_field(package_info, "version")
        .is_some_and(|version| post_install_version_ok(version, expected_version));
    let package_scope_matches = pkgutil_field(package_info, "volume") == Some("/")
        && pkgutil_field(package_info, "location") == Some("/");

    let payload_matches = payload_files.lines().any(|line| {
        line.trim().trim_start_matches("./").trim_start_matches('/') == "usr/local/bin/tr300"
    });

    let owner_id =
        pkgutil_field(file_info, "pkgid").or_else(|| pkgutil_field(file_info, "package-id"));
    let owner_version =
        pkgutil_field(file_info, "pkg-version").or_else(|| pkgutil_field(file_info, "version"));
    let owner_matches = owner_id == Some(MAC_PKG_ID)
        && owner_version.is_some_and(|version| post_install_version_ok(version, expected_version))
        && pkgutil_field(file_info, "volume") == Some("/")
        && pkgutil_field(file_info, "path") == Some("/usr/local/bin/tr300");

    package_id_matches
        && package_version_matches
        && package_scope_matches
        && payload_matches
        && owner_matches
}

#[cfg(target_os = "macos")]
fn pkgutil_text(args: &[&str]) -> Option<String> {
    let output = run_output(
        "pkgutil",
        args,
        CommandTimeout::Custom(std::time::Duration::from_secs(5)),
    )?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(target_os = "macos")]
fn mac_pkg_receipt_matches_current_exe() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let expected = std::path::Path::new("/usr/local/bin/tr300");
    if exe.canonicalize().unwrap_or(exe)
        != expected.canonicalize().unwrap_or_else(|_| expected.into())
    {
        return false;
    }

    let Some(package_info) = pkgutil_text(&["--pkg-info", MAC_PKG_ID]) else {
        return false;
    };
    let Some(payload_files) = pkgutil_text(&["--files", MAC_PKG_ID]) else {
        return false;
    };
    let Some(file_info) = pkgutil_text(&["--file-info", "/usr/local/bin/tr300"]) else {
        return false;
    };
    if !pkg_receipt_metadata_matches(&package_info, &payload_files, &file_info, VERSION) {
        return false;
    }

    run_output(
        "pkgutil",
        ["--verify", MAC_PKG_ID],
        CommandTimeout::Custom(std::time::Duration::from_secs(5)),
    )
    .is_some_and(|output| output.status.success())
}

#[cfg(windows)]
fn detect_install_channel() -> InstallChannel {
    match detect_install_origin() {
        InstallOrigin::MsiGlobal => InstallChannel::MsiGlobal,
        InstallOrigin::MsiCorporate => InstallChannel::MsiCorporate,
        InstallOrigin::ExeGlobal => InstallChannel::ExeGlobal,
        InstallOrigin::ExeCorporate => InstallChannel::ExeCorporate,
        InstallOrigin::CargoOrInstaller | InstallOrigin::Unknown => {
            metadata_install_channel(InstallChannel::PowerShellInstaller)
        }
    }
}

#[cfg(not(windows))]
fn detect_install_channel() -> InstallChannel {
    #[cfg(target_os = "macos")]
    if mac_pkg_receipt_matches_current_exe() {
        return InstallChannel::MacPkg;
    }
    metadata_install_channel(InstallChannel::ShellInstaller)
}

// ── Windows install-origin detection (v3.15.0+) ────────────────────

/// Read the `HKCU\Software\TR300\InstallSource` registry value written by
/// the four first-class installers on install. The caller still validates its
/// edition scope against the running executable path because markers can be
/// stale when installations coexist or are manually moved.
/// Returns `None` if the key is missing, the value is missing, the value
/// type isn't a string, or the value content doesn't match a known variant.
#[cfg(windows)]
fn parse_install_source_marker(value: &str) -> Option<InstallOrigin> {
    match value {
        "msi-global" => Some(InstallOrigin::MsiGlobal),
        "msi-corporate" => Some(InstallOrigin::MsiCorporate),
        "exe-global" => Some(InstallOrigin::ExeGlobal),
        "exe-corporate" => Some(InstallOrigin::ExeCorporate),
        _ => None,
    }
}

#[cfg(windows)]
fn read_install_source_value(name: &str) -> Option<InstallOrigin> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey("Software\\TR300").ok()?;
    let value: String = key.get_value(name).ok()?;
    parse_install_source_marker(&value)
}

#[cfg(windows)]
fn read_install_source_marker(exe_path: &str) -> Option<InstallOrigin> {
    let scoped = if is_global_install_path(exe_path) {
        read_install_source_value("InstallSourceGlobal")
    } else if is_corporate_install_path(exe_path) {
        read_install_source_value("InstallSourceCorporate")
    } else {
        None
    };
    scoped.or_else(|| read_install_source_value("InstallSource"))
}

/// Determine how this binary was installed on Windows.
///
/// Strategy:
/// 1. Read the `HKCU\Software\TR300\InstallSource` marker and accept it only
///    when its Global/Corporate scope matches the running executable. All four
///    first-class installers write this marker on install.
/// 2. If no marker, consult Add/Remove Programs and accept exactly one
///    registration whose product family/scope matches the running path.
/// 3. A `.cargo\bin` path is classified only as the broad historical
///    `CargoOrInstaller`; receipt/metadata detection later resolves the
///    precise channel.
///
/// A marker-free Program Files or LocalAppData install is deliberately
/// `Unknown`: those paths identify edition scope but cannot distinguish MSI
/// from Inno EXE. Guessing would risk a cross-format update and duplicate
/// uninstall registrations, so the update performs no mutation.
#[cfg(windows)]
pub(crate) fn detect_install_origin() -> InstallOrigin {
    let Ok(exe) = std::env::current_exe() else {
        return InstallOrigin::Unknown;
    };
    let exe = exe.to_string_lossy();
    if let Some(origin) = read_install_source_marker(&exe) {
        // One HKCU marker can outlive an old/coexisting install. Trust it only
        // when its edition scope matches the binary that is actually running.
        let scope_matches = match origin {
            InstallOrigin::MsiGlobal | InstallOrigin::ExeGlobal => is_global_install_path(&exe),
            InstallOrigin::MsiCorporate | InstallOrigin::ExeCorporate => {
                is_corporate_install_path(&exe)
            }
            InstallOrigin::CargoOrInstaller | InstallOrigin::Unknown => false,
        };
        if scope_matches {
            return origin;
        }
    }
    if let Some(origin) = detect_registered_installer(&exe) {
        return origin;
    }
    classify_install_path(&exe)
}

/// Recover a missing marker only when Add/Remove Programs contains exactly one
/// registered installer family whose scope and install location match the
/// executable that is running. Coexisting or malformed registrations remain
/// Unknown; guessing here would violate the no-cross-channel update contract.
#[cfg(windows)]
fn detect_registered_installer(exe_path: &str) -> Option<InstallOrigin> {
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::RegKey;

    let global = is_global_install_path(exe_path);
    let corporate = is_corporate_install_path(exe_path);
    if !global && !corporate {
        return None;
    }

    let roots = [
        ("hkcu", RegKey::predef(HKEY_CURRENT_USER)),
        ("hklm", RegKey::predef(HKEY_LOCAL_MACHINE)),
    ];
    let views = [KEY_READ | KEY_WOW64_64KEY, KEY_READ | KEY_WOW64_32KEY];
    let mut candidates = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (root_name, root) in roots {
        for view in views {
            let Ok(uninstall) = root.open_subkey_with_flags(
                "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
                view,
            ) else {
                continue;
            };
            for name in uninstall.enum_keys().filter_map(Result::ok) {
                let Ok(entry) = uninstall.open_subkey_with_flags(&name, KEY_READ) else {
                    continue;
                };
                let display: String = entry.get_value("DisplayName").unwrap_or_default();
                let install_location: String =
                    entry.get_value("InstallLocation").unwrap_or_default();
                let uninstall_string: String =
                    entry.get_value("UninstallString").unwrap_or_default();
                let windows_installer: u32 = entry.get_value("WindowsInstaller").unwrap_or(0);
                let Some(origin) = classify_registered_installer_entry(
                    global,
                    &display,
                    &install_location,
                    &uninstall_string,
                    windows_installer == 1,
                ) else {
                    continue;
                };
                // Registry redirection can expose the same product identity in
                // both enumeration views. Count that product once, but retain
                // genuinely distinct product keys as conflicting evidence.
                if !seen.insert((root_name, name.to_ascii_lowercase(), origin)) {
                    continue;
                }
                // Preserve duplicate registrations as conflicting evidence.
                // Two products of the same family are still ambiguous; the
                // updater must not collapse them into one guessed channel.
                candidates.push(origin);
            }
        }
    }
    (candidates.len() == 1).then(|| candidates[0])
}

#[cfg(windows)]
fn classify_registered_installer_entry(
    global: bool,
    display: &str,
    install_location: &str,
    uninstall_string: &str,
    windows_installer: bool,
) -> Option<InstallOrigin> {
    let expected_display = if global {
        "tr300"
    } else {
        "tr300 (Corporate Edition)"
    };
    if display != expected_display {
        return None;
    }

    if windows_installer {
        // MSI commonly leaves ARP InstallLocation empty. The exact edition
        // display identity plus the running executable's already-established
        // scope and WindowsInstaller flag identify the format without guessing.
        return Some(if global {
            InstallOrigin::MsiGlobal
        } else {
            InstallOrigin::MsiCorporate
        });
    }

    let registration_text = format!("{install_location} {uninstall_string}").to_ascii_lowercase();
    let path_matches = if global {
        registration_text.contains("\\program files\\tr300")
    } else {
        registration_text.contains("\\appdata\\local\\programs\\tr300")
    };
    path_matches.then_some(if global {
        InstallOrigin::ExeGlobal
    } else {
        InstallOrigin::ExeCorporate
    })
}

/// Pure-function half of `detect_install_origin()` for unit testing.
/// Lowercased substring match handles drive-letter casing and Windows path
/// case-insensitivity. Order matters: check more-specific paths first.
#[cfg(windows)]
fn classify_install_path(exe_path: &str) -> InstallOrigin {
    let lower = exe_path.to_lowercase();
    if lower.contains("\\.cargo\\bin\\") {
        InstallOrigin::CargoOrInstaller
    } else {
        // Program Files and LocalAppData paths identify the edition scope but
        // cannot distinguish MSI from Inno EXE. Choosing either would risk a
        // cross-format update, so ambiguous/missing-marker installs do not
        // mutate the machine.
        InstallOrigin::Unknown
    }
}

#[cfg(windows)]
fn is_global_install_path(exe_path: &str) -> bool {
    exe_path
        .to_ascii_lowercase()
        .contains("\\program files\\tr300\\")
}

#[cfg(windows)]
fn is_corporate_install_path(exe_path: &str) -> bool {
    exe_path
        .to_ascii_lowercase()
        .contains("\\appdata\\local\\programs\\tr300\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_asset_urls_pin_the_resolved_tag_without_versioning_the_filename() {
        assert_eq!(
            release_asset_url("4.1.0", MSI_GLOBAL_ASSET),
            "https://github.com/QubeTX/qube-machine-report/releases/download/v4.1.0/tr300-x86_64-pc-windows-msvc.msi"
        );
        assert!(!MSI_GLOBAL_ASSET.contains("4.1.0"));
    }

    #[test]
    fn every_recognized_channel_has_a_non_crossing_strategy() {
        assert_eq!(
            build_strategy_list(InstallChannel::MsiGlobal),
            vec![UpdateStrategy::MsiGlobal]
        );
        assert_eq!(
            build_strategy_list(InstallChannel::Cargo),
            vec![UpdateStrategy::Cargo]
        );
        assert_eq!(
            build_strategy_list(InstallChannel::MacPkg),
            vec![UpdateStrategy::MacDmg]
        );
        assert!(build_strategy_list(InstallChannel::Unknown).is_empty());
    }

    #[test]
    fn cargo_dist_receipt_requires_provider_app_and_matching_prefix() {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        let exe = bin.join(if cfg!(windows) { "tr300.exe" } else { "tr300" });
        std::fs::write(&exe, b"fixture").unwrap();
        let receipt = serde_json::json!({
            "install_prefix": root.path().to_string_lossy(),
            "provider": {"source": "cargo-dist", "version": "0.31.0"},
            "source": {"app_name": "tr300"}
        });
        assert!(cargo_dist_receipt_matches(&receipt, &exe));

        let wrong_app = serde_json::json!({
            "install_prefix": root.path().to_string_lossy(),
            "provider": {"source": "cargo-dist"},
            "source": {"app_name": "different-app"}
        });
        assert!(!cargo_dist_receipt_matches(&wrong_app, &exe));
    }

    #[test]
    fn mac_pkg_receipt_requires_exact_payload_version_and_file_ownership() {
        let package_info = "\
package-id: com.qubetx.tr300.pkg\n\
version: 4.1.0\n\
volume: /\n\
location: /\n";
        let payload = "usr\nusr/local\nusr/local/bin\nusr/local/bin/tr300\n";
        let file_info = "\
volume: /\n\
path: /usr/local/bin/tr300\n\
pkgid: com.qubetx.tr300.pkg\n\
pkg-version: 4.1.0\n";

        assert!(pkg_receipt_metadata_matches(
            package_info,
            payload,
            file_info,
            "4.1.0"
        ));
        assert!(!pkg_receipt_metadata_matches(
            package_info,
            payload,
            file_info,
            "4.0.1"
        ));
        assert!(!pkg_receipt_metadata_matches(
            package_info,
            "usr/local/bin/another-tool\n",
            file_info,
            "4.1.0"
        ));
        assert!(!pkg_receipt_metadata_matches(
            package_info,
            payload,
            &file_info.replace("com.qubetx.tr300.pkg", "com.example.shadow"),
            "4.1.0"
        ));
    }

    #[test]
    fn newest_shared_path_metadata_represents_the_latest_install_intent() {
        use std::time::{Duration, SystemTime};

        let older = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
        let newer = SystemTime::UNIX_EPOCH + Duration::from_secs(20);
        assert_eq!(
            newest_metadata_channel(
                Some(newer),
                Some(older),
                InstallChannel::PowerShellInstaller,
            ),
            InstallChannel::PowerShellInstaller
        );
        assert_eq!(
            newest_metadata_channel(Some(older), Some(newer), InstallChannel::ShellInstaller,),
            InstallChannel::Cargo
        );
        assert_eq!(
            newest_metadata_channel(Some(newer), Some(newer), InstallChannel::ShellInstaller,),
            InstallChannel::Unknown
        );
        assert_eq!(
            newest_metadata_channel(None, Some(newer), InstallChannel::ShellInstaller),
            InstallChannel::Cargo
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
            UpdateStrategy::MacDmg.label(),
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
    fn install_origin_classify_program_files_without_marker_is_unknown() {
        assert_eq!(
            classify_install_path(r"C:\Program Files\tr300\bin\tr300.exe"),
            InstallOrigin::Unknown,
        );
        // Case-insensitive: drive letter or "Program Files" capitalization
        // shouldn't change the verdict.
        assert_eq!(
            classify_install_path(r"c:\PROGRAM FILES\tr300\BIN\tr300.exe"),
            InstallOrigin::Unknown,
        );
    }

    #[cfg(windows)]
    #[test]
    fn install_origin_classify_localappdata_without_marker_is_unknown() {
        assert_eq!(
            classify_install_path(r"C:\Users\alice\AppData\Local\Programs\tr300\bin\tr300.exe"),
            InstallOrigin::Unknown,
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

    #[cfg(windows)]
    #[test]
    fn arp_recovery_accepts_msi_without_install_location_but_not_wrong_edition() {
        assert_eq!(
            classify_registered_installer_entry(true, "tr300", "", "MsiExec.exe /I{GUID}", true),
            Some(InstallOrigin::MsiGlobal)
        );
        assert_eq!(
            classify_registered_installer_entry(
                false,
                "tr300 (Corporate Edition)",
                "",
                "MsiExec.exe /I{GUID}",
                true,
            ),
            Some(InstallOrigin::MsiCorporate)
        );
        assert_eq!(
            classify_registered_installer_entry(
                true,
                "tr300 (Corporate Edition)",
                "",
                "MsiExec.exe /I{GUID}",
                true,
            ),
            None
        );
    }

    #[cfg(windows)]
    #[test]
    fn arp_recovery_requires_matching_inno_path() {
        assert_eq!(
            classify_registered_installer_entry(
                true,
                "tr300",
                r"C:\Program Files\tr300\",
                r#""C:\Program Files\tr300\unins000.exe""#,
                false,
            ),
            Some(InstallOrigin::ExeGlobal)
        );
        assert_eq!(
            classify_registered_installer_entry(
                true,
                "tr300",
                r"D:\portable\tr300\",
                r#""D:\portable\tr300\unins000.exe""#,
                false,
            ),
            None
        );
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
    fn malformed_versions_are_not_lossily_reinterpreted() {
        assert_eq!(parse_numeric_version("3.18.0"), Some(vec![3, 18, 0]));
        assert_eq!(parse_numeric_version("3.foo.1"), None);
        assert_eq!(parse_numeric_version("3..1"), None);
        assert_eq!(parse_numeric_version(""), None);
        assert!(!is_newer("3.17.0", "3.foo.1"));
        assert!(!is_newer("broken", "3.18.0"));
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
        // A mismatch is REFUSED — this is the load-bearing corruption/asset
        // mismatch guard, so it must never silently pass.
        let err = checksum_verdict("deadbeef", hash).unwrap_err();
        assert!(err.contains("SHA256 mismatch"), "err: {err}");
    }

    #[test]
    fn staged_installers_use_unique_private_directories_and_cleanup() {
        let first = StagedInstaller::new("msi").unwrap();
        let second = StagedInstaller::new("msi").unwrap();
        let first_dir = first.path().parent().unwrap().to_path_buf();
        let second_dir = second.path().parent().unwrap().to_path_buf();

        assert_ne!(first_dir, second_dir);
        assert_eq!(
            first.path().extension().and_then(|ext| ext.to_str()),
            Some("msi")
        );
        std::fs::write(first.path(), b"payload").unwrap();
        assert!(first.path().is_file());

        drop(first);
        assert!(!first_dir.exists());
        drop(second);
        assert!(!second_dir.exists());
    }

    #[test]
    fn explicit_staging_cleanup_preserves_a_verified_success() {
        let staged = StagedInstaller::new("exe").unwrap();
        let staging_dir = staged.path().parent().unwrap().to_path_buf();
        std::fs::write(staged.path(), b"payload").unwrap();

        assert!(finish_staged_attempt(staged, Ok(())).is_ok());
        assert!(!staging_dir.exists());
    }

    #[test]
    fn endpoint_policy_io_errors_are_classified_conservatively() {
        assert!(likely_endpoint_policy_error(&std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "blocked"
        )));
        assert!(likely_endpoint_policy_error(
            &std::io::Error::from_raw_os_error(225)
        ));
        assert!(!likely_endpoint_policy_error(&std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "bad payload"
        )));
    }

    #[test]
    fn endpoint_policy_block_stops_fallback_chain() {
        let strategies = [UpdateStrategy::Cargo, UpdateStrategy::InstallerCurl];
        let mut calls = 0;
        let failure = execute_update_with("4.0.0", &strategies, |_strategy, _latest| {
            calls += 1;
            Err(StrategyError::PolicyBlocked(
                "simulated endpoint policy block".to_string(),
            ))
        })
        .expect_err("a policy block must fail the update");

        assert_eq!(calls, 1, "no second write strategy should be attempted");
        assert_eq!(failure.attempts.len(), 1);
        assert_eq!(failure.attempts[0].kind, AttemptKind::Blocked);
        assert!(failure.attempts[0].message.contains("endpoint policy"));

        let payload =
            update_failure_payload("3.17.0", "4.0.0", &failure, InstallChannel::MsiGlobal);
        assert_eq!(payload["success"], false);
        assert_eq!(payload["attempts"][0]["result"], "blocked");
        assert_eq!(payload["manual_install_url"], MANUAL_INSTALL_URL);
        assert_eq!(payload["official_releases_url"], RELEASES_PAGE);
        assert_eq!(
            payload["exact_installer_url"],
            release_asset_url("4.0.0", MSI_GLOBAL_ASSET)
        );
    }

    #[test]
    fn ordinary_update_failure_has_manual_url_without_policy_specific_url() {
        let failure = UpdateFailure {
            attempts: vec![AttemptRecord {
                strategy: UpdateStrategy::Cargo,
                kind: AttemptKind::Failed,
                message: "simulated ordinary failure".to_string(),
            }],
        };
        let payload = update_failure_payload("3.17.0", "4.0.0", &failure, InstallChannel::Cargo);
        assert_eq!(payload["attempts"][0]["result"], "failed");
        assert_eq!(payload["manual_install_url"], MANUAL_INSTALL_URL);
        assert!(payload.get("official_releases_url").is_none());
        assert!(payload.get("exact_installer_url").is_none());
    }

    #[test]
    fn unknown_channel_failure_explains_safe_no_mutation() {
        let failure = UpdateFailure { attempts: vec![] };
        let payload = update_failure_payload("3.17.0", "4.0.0", &failure, InstallChannel::Unknown);
        assert!(payload["message"]
            .as_str()
            .unwrap()
            .contains("not attempted"));
        assert_eq!(payload["attempts"], serde_json::json!([]));
        assert!(payload.get("exact_installer_url").is_none());
    }

    #[test]
    fn http_status_message_explains_rate_limit() {
        // 403 with the rate-limit-exhausted header → the explicit message.
        assert!(http_status_message(403, Some("0")).contains("rate limit"));
        // 403 without the exhausted header → generic HTTP message.
        assert!(http_status_message(403, None).contains("HTTP 403"));
        // Other error codes → generic HTTP message.
        assert!(http_status_message(500, None).contains("HTTP 500"));
    }

    #[test]
    fn post_install_version_ok_drives_cargo_verify_logic() {
        // The cargo-path verify (verify_cargo_post_install) treats a matching
        // re-exec version as success and a stale one as fall-through; the
        // decision is post_install_version_ok, re-pinned here for the cargo
        // path specifically (crates.io lag → installed stays old).
        assert!(post_install_version_ok("3.16.0", "3.16.0"));
        assert!(!post_install_version_ok("3.15.3", "3.16.0"));
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

    #[test]
    fn parse_sha256_sidecar_normalizes_to_lowercase() {
        let line = "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789  *foo.msi";
        assert_eq!(
            parse_sha256_sidecar(line).as_deref(),
            Some("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"),
        );
    }

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

    #[test]
    fn parse_sha256_sidecar_rejects_non_hex_chars() {
        // 64 chars but `g` is not a hex digit.
        let bad = format!("{}  *foo.msi", "g".repeat(64));
        assert_eq!(parse_sha256_sidecar(&bad), None);
    }

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
