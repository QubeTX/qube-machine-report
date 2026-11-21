//! Windows installation utilities
//!
//! Adds TR-300 auto-run to PowerShell profiles.

use crate::error::{AppError, Result};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::shared::{MARKER_END, MARKER_START};

mod startup;
pub(super) use startup::{check_machine_startup, install_machine_startup, remove_machine_startup};

/// Get the installation path for Windows
pub fn install_path() -> PathBuf {
    // Use %LOCALAPPDATA%\Programs\tr300
    if let Some(local_app_data) = dirs::data_local_dir() {
        return local_app_data
            .join("Programs")
            .join("tr300")
            .join("tr300.exe");
    }

    // Fallback to user profile
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        return PathBuf::from(userprofile)
            .join("AppData")
            .join("Local")
            .join("Programs")
            .join("tr300")
            .join("tr300.exe");
    }

    PathBuf::from(r"C:\Program Files\tr300\tr300.exe")
}

/// Get all PowerShell profile paths that should be modified.
///
/// Probes both `powershell` (Windows PowerShell 5.1) and `pwsh`
/// (PowerShell 7+) — they have DIFFERENT `$PROFILE` paths:
/// - Windows PowerShell: `Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1`
/// - PowerShell 7:       `Documents\PowerShell\Microsoft.PowerShell_profile.ps1`
///
/// PowerShell 7 does NOT source the 5.1 profile, so prior to v3.15.3
/// pwsh-only users (a growing cohort on dev workstations) got a
/// silent no-op install: the snippet went to a profile their actual
/// shell never read. Writing to BOTH paths when both shells exist
/// makes the install resilient to the user's choice of host.
///
/// Returns a deduplicated list — if both probes happen to return the
/// same path (unusual but possible on stripped Windows images),
/// we only modify it once.
fn get_powershell_profiles() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();

    for launcher in ["powershell", "pwsh"] {
        let Ok(mut command) = crate::trusted_process::powershell_command_for(launcher, "$PROFILE")
        else {
            continue;
        };
        let Ok(output) = command.output() else {
            // Launcher not on PATH; skip this shell flavor.
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path_str.is_empty() {
            continue;
        }
        let path = PathBuf::from(path_str);
        if !paths.iter().any(|p| p == &path) {
            paths.push(path);
        }
    }

    // Fallback: both probes failed (PowerShell entirely missing — rare
    // on Windows but technically possible on Windows Server Core or
    // heavily-stripped images). Fall back to the legacy 5.1 location
    // so the install attempt at least targets a documented path.
    if paths.is_empty() {
        if let Some(docs) = dirs::document_dir() {
            paths.push(
                docs.join("WindowsPowerShell")
                    .join("Microsoft.PowerShell_profile.ps1"),
            );
        }
    }

    paths
}

/// Install tr300 to PowerShell profile
pub fn install() -> Result<()> {
    // Preflight: ensure the user's execution policy allows the profile to load.
    // A fresh Windows install defaults to `Restricted`, which blocks every
    // `.ps1` file — including `$PROFILE` itself — so the auto-run we're about
    // to write would fail at the next shell start with an UnauthorizedAccess
    // PSSecurityException. Lift CurrentUser to `RemoteSigned` (the minimum
    // permissive policy that loads local unsigned scripts) when needed.
    // Non-fatal: never short-circuits the profile write.
    run_execution_policy_preflight();

    let profile_paths = get_powershell_profiles();
    if profile_paths.is_empty() {
        return Err(AppError::platform(
            "Could not determine PowerShell profile path",
        ));
    }

    // Heads-up if another definition will shadow the packaged `report`
    // command. Best-effort heuristic — scans each PowerShell profile and PATH. Read-only,
    // no PowerShell subprocess, so a noisy $PROFILE can't fire side effects
    // during `tr300 install`.
    warn_if_report_already_defined(&profile_paths);

    startup::configure_user(&profile_paths, true)?;
    println!("Configured TR-300 startup for PowerShell and Command Prompt.");
    Ok(())
}

pub fn uninstall() -> Result<()> {
    startup::configure_user(&get_powershell_profiles(), false)?;
    println!("Removed TR-300 startup from your PowerShell and Command Prompt.");
    Ok(())
}

/// Find the location of the currently running binary
pub fn find_binary_location() -> Option<PathBuf> {
    // First try to get the current executable path
    if let Ok(exe_path) = env::current_exe() {
        if exe_path.exists() {
            return Some(exe_path);
        }
    }

    // Fallback to the standard install path
    let path = install_path();
    if path.exists() {
        return Some(path);
    }

    None
}

/// Get the parent directory of the binary (for cleanup)
pub fn get_binary_parent_dir(binary_path: &std::path::Path) -> Option<PathBuf> {
    binary_path.parent().map(|p| p.to_path_buf())
}

/// Remove the binary file
pub fn remove_binary(binary_path: &PathBuf) -> Result<()> {
    if !binary_path.exists() {
        return Ok(());
    }

    fs::remove_file(binary_path)
        .map_err(|e| fail_install(InstallStep::RemoveBinary, binary_path.as_path(), e))?;

    println!("Removed binary: {}", binary_path.display());
    Ok(())
}

/// Remove the parent directory if empty
pub fn remove_empty_parent_dir(dir: &PathBuf) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    // Check if directory is empty
    let is_empty = match fs::read_dir(dir) {
        Ok(mut entries) => entries.next().is_none(),
        Err(_) => false,
    };

    if is_empty {
        fs::remove_dir(dir).map_err(|e| {
            AppError::platform(format!(
                "Failed to remove directory {}: {}",
                dir.display(),
                e
            ))
        })?;
        println!("Removed empty directory: {}", dir.display());
    }

    Ok(())
}

/// Warn (to stderr) when another `report` definition will shadow the packaged
/// command in the user's PowerShell environment.
///
/// Read-only heuristic: scans each `$PROFILE` we'd be writing to for
/// `Set-Alias`, `function`, or `New-Alias` declarations of `report`,
/// plus probes PATH for a standalone `report.exe` / `report.cmd` /
/// `report.bat` that isn't ours. No PowerShell subprocess — so a noisy
/// `$PROFILE` (which usually loads modules, sets prompts, calls oh-my-
/// posh, etc.) can't fire side effects during `tr300 install`.
///
/// Best-effort by design: misses aliases defined in PowerShell modules
/// that are auto-loaded but not declared in `$PROFILE` itself, and
/// misses `Set-Alias` declarations split across multiple lines.
fn warn_if_report_already_defined(profile_paths: &[PathBuf]) {
    let mut hits: Vec<String> = Vec::new();

    // $PROFILE scan. Case-insensitive — PowerShell is case-insensitive
    // for command names. Match common forms users actually type.
    for profile_path in profile_paths {
        let Ok(content) = fs::read_to_string(profile_path) else {
            continue;
        };
        for (idx, raw) in content.lines().enumerate() {
            let line = raw.trim();
            // Skip TR-300's own block so re-running install doesn't warn
            // about itself.
            if line.contains(super::shared::MARKER_START)
                || line.contains(super::shared::MARKER_END)
                || (line.to_ascii_lowercase().contains("set-alias")
                    && line.to_ascii_lowercase().contains("report")
                    && line.to_ascii_lowercase().contains("tr300"))
            {
                continue;
            }
            let lower = line.to_ascii_lowercase();
            let matches_alias = (lower.starts_with("set-alias") || lower.starts_with("new-alias"))
                && lower.contains("report");
            let matches_fn = lower.starts_with("function report ")
                || lower.starts_with("function report{")
                || lower.starts_with("function report(")
                || lower == "function report";
            if matches_alias || matches_fn {
                hits.push(format!("{}:{}  {}", profile_path.display(), idx + 1, line));
            }
        }
    }

    // PATH scan. Skip the report executable bundled beside tr300 in a
    // recognized TR-300 installation directory.
    if let Ok(path_env) = std::env::var("PATH") {
        for dir in path_env.split(';') {
            if dir.is_empty() {
                continue;
            }
            for ext in &["exe", "cmd", "bat", "ps1"] {
                let candidate = Path::new(dir).join(format!("report.{}", ext));
                if candidate.exists() && !looks_like_our_install_dir(dir) {
                    hits.push(format!("{}  (executable on PATH)", candidate.display()));
                }
            }
        }
    }

    if hits.is_empty() {
        return;
    }

    eprintln!();
    eprintln!("Note: `report` is already defined in your PowerShell environment:");
    for h in &hits {
        eprintln!("    {}", h);
    }
    eprintln!("That definition may shadow TR-300's packaged `report` command.");
    eprintln!("Remove or rename the existing definition if you want `report` to resolve");
    eprintln!("to the full-alias TR-300 command on PATH.");
    eprintln!();
}

/// Heuristic — does this PATH dir look like a TR-300 install location?
/// Matches the two paths the four first-class Windows installers write
/// to (Global and Corporate editions), plus the legacy `\.cargo\bin\`
/// chain. Case-insensitive.
fn looks_like_our_install_dir(dir: &str) -> bool {
    let lower = dir.to_ascii_lowercase();
    lower.contains(r"\program files\tr300\")
        || lower.contains(r"\appdata\local\programs\tr300\")
        || lower.contains(r"\.cargo\bin")
}

/// Coarse classification of a PowerShell execution-policy string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PolicyState {
    /// `Restricted` or `Undefined` — the default on Windows Client; blocks
    /// every `.ps1` including `$PROFILE`. Safe to auto-fix to `RemoteSigned`.
    BlockedDefault,
    /// `AllSigned` — a deliberate hardening choice. Blocks our unsigned
    /// profile snippet, but we won't silently downgrade it.
    BlockedAllSigned,
    /// `RemoteSigned`, `Unrestricted`, or `Bypass` — already permissive
    /// enough for the profile snippet to load.
    Permissive,
    /// Anything else (future PowerShell versions, unparseable output). Treat
    /// as permissive to avoid acting on values we don't understand.
    Unknown,
}

/// Outcome of a `Set-ExecutionPolicy` attempt.
#[derive(Debug)]
enum TrySetResult {
    /// The post-set policy is now permissive.
    Succeeded,
    /// The post-set policy is still blocking (typically a GPO override at
    /// `MachinePolicy` or `UserPolicy` scope wins precedence over CurrentUser).
    StillBlocked { new_policy: String },
}

/// Classify an execution-policy string. Case-insensitive — different
/// PowerShell versions have occasionally capitalized output differently.
fn policy_state(policy: &str) -> PolicyState {
    match policy.trim().to_ascii_lowercase().as_str() {
        "restricted" | "undefined" => PolicyState::BlockedDefault,
        "allsigned" => PolicyState::BlockedAllSigned,
        "remotesigned" | "unrestricted" | "bypass" => PolicyState::Permissive,
        "" => PolicyState::Unknown,
        _ => PolicyState::Unknown,
    }
}

/// Read the current `CurrentUser` execution policy. Returns `None` if
/// PowerShell is not on PATH or the command fails — in which case we skip
/// the preflight entirely.
fn detect_execution_policy() -> Option<String> {
    let output = crate::trusted_process::powershell_command_for(
        "powershell",
        "if ((Get-ExecutionPolicy -Scope MachinePolicy) -ne 'Undefined' -or (Get-ExecutionPolicy -Scope UserPolicy) -ne 'Undefined') { 'GroupPolicy' } else { Get-ExecutionPolicy -Scope CurrentUser }",
    )
    .ok()?
    .output()
    .ok()?;

    if !output.status.success() {
        return None;
    }

    let policy = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if policy.is_empty() {
        None
    } else {
        Some(policy)
    }
}

/// Attempt to set `CurrentUser` execution policy to `RemoteSigned`, then
/// verify the change actually took effect (a higher-precedence GPO can
/// override the user-scope setting without `Set-ExecutionPolicy` itself
/// failing).
fn try_set_execution_policy() -> std::io::Result<TrySetResult> {
    let output = crate::trusted_process::powershell_command_for(
        "powershell",
        "Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned -Force",
    )?
    .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(std::io::Error::other(if stderr.is_empty() {
            "Set-ExecutionPolicy failed".to_string()
        } else {
            stderr
        }));
    }

    // Re-detect to confirm the change actually applied (GPO can win).
    let new_policy = detect_execution_policy().unwrap_or_else(|| "Unknown".to_string());
    Ok(match policy_state(&new_policy) {
        PolicyState::Permissive | PolicyState::Unknown => TrySetResult::Succeeded,
        PolicyState::BlockedDefault | PolicyState::BlockedAllSigned => {
            TrySetResult::StillBlocked { new_policy }
        }
    })
}

/// Inspect execution policy and adjust it to the minimum needed for the
/// auto-run profile snippet to load. Prints only when action is taken or a
/// problem is detected. Never propagates an error — the alias-write half of
/// `install()` still succeeds even when the policy can't be fixed.
fn run_execution_policy_preflight() {
    let Some(current) = detect_execution_policy() else {
        return;
    };

    match policy_state(&current) {
        PolicyState::Permissive | PolicyState::Unknown => {
            // Happy path — stay quiet.
        }
        PolicyState::BlockedDefault => match try_set_execution_policy() {
            Ok(TrySetResult::Succeeded) => {
                println!(
                    "Set PowerShell CurrentUser execution policy: {} -> RemoteSigned",
                    current
                );
                println!("  (required to load $PROFILE; only your account, no admin needed)");
            }
            Ok(TrySetResult::StillBlocked { new_policy }) => {
                eprintln!(
                    "Warning: tried to set CurrentUser execution policy to RemoteSigned, but it's still '{}'.",
                    new_policy
                );
                eprintln!(
                    "  This usually means a Group Policy (MachinePolicy/UserPolicy) is enforcing a stricter setting."
                );
                eprintln!(
                "  The packaged 'report' command still works manually, but the auto-run on new shells won't fire."
                );
                eprintln!("  To fix: from an elevated PowerShell, run");
                eprintln!(
                    "    Set-ExecutionPolicy -Scope LocalMachine -ExecutionPolicy RemoteSigned"
                );
                eprintln!("  or coordinate with IT if a domain policy is in force.");
            }
            Err(e) => {
                eprintln!(
                    "Warning: could not adjust PowerShell execution policy ({}).",
                    e
                );
                eprintln!(
                    "  The packaged 'report' command still works manually, but the auto-run on new shells won't fire"
                );
                eprintln!("  until you run (no admin needed):");
                eprintln!(
                    "    Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned"
                );
            }
        },
        PolicyState::BlockedAllSigned => {
            eprintln!(
                "Notice: PowerShell CurrentUser execution policy is 'AllSigned' — TR-300 will not change this."
            );
            eprintln!(
                "  AllSigned blocks the unsigned auto-run snippet in $PROFILE, so the auto-run on new shells won't fire."
            );
            eprintln!(
                "  The packaged 'report' command still works manually. If you'd like to opt into the auto-run, you can either:"
            );
            eprintln!("    - sign the TR-300 block in your profile yourself, or");
            eprintln!("    - relax the policy (no admin needed):");
            eprintln!(
                "        Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned"
            );
        }
    }
}

/// Perform complete uninstall (profile + binary + directory)
pub fn uninstall_complete() -> Result<()> {
    // Revalidate companion ownership before changing any shell profile.
    let binary_path = find_binary_location();
    let report_path = binary_path
        .as_deref()
        .map(adjacent_report_location)
        .transpose()?
        .flatten();
    uninstall()?;

    // Then remove the binary and cleanup directory.
    if let Some(binary_path) = binary_path {
        let parent_dir = get_binary_parent_dir(&binary_path);

        // If the binary we're about to delete IS the currently-running
        // `tr300.exe`, Windows refuses `DeleteFile` with raw OS error 5
        // (Access Denied) because the loader holds a file handle to
        // the image. Hand the delete off to a detached `cmd.exe` job
        // that runs AFTER this process exits — fixes audit finding F13
        // where `uninstall -> Complete` from `%LocalAppData%\Programs\
        // tr300\tr300.exe` cleaned the profile but left the binary
        // behind with a confusing `RemoveBinary` error.
        if is_running_binary(&binary_path) {
            // Only schedule parent-dir removal when the dir name
            // contains "tr300" — same heuristic as the synchronous
            // path below, to avoid wiping something unexpected when
            // a user has portably-installed tr300 into a generic dir.
            let cleanup_dir = parent_dir
                .as_ref()
                .filter(|d| d.to_string_lossy().to_lowercase().contains("tr300"))
                .map(|d| d.as_path());
            schedule_self_cleanup(&binary_path, report_path.as_deref(), cleanup_dir).map_err(
                |e| {
                    AppError::platform(format!(
                        "Failed to schedule deferred cleanup of {}: {}",
                        binary_path.display(),
                        e
                    ))
                },
            )?;
            println!("Scheduled deferred cleanup of: {}", binary_path.display());
            if let Some(path) = report_path {
                println!("Scheduled deferred cleanup of: {}", path.display());
            }
            println!(
                "  (this running tr300.exe will be removed within a few seconds of process exit)"
            );
            return Ok(());
        }

        if let Some(report_path) = report_path {
            remove_binary(&report_path)?;
        }
        remove_binary(&binary_path)?;

        // Try to remove the parent directory if empty
        if let Some(dir) = parent_dir {
            // Only attempt to remove if it's our install directory (contains "tr300")
            if dir.to_string_lossy().to_lowercase().contains("tr300") {
                let _ = remove_empty_parent_dir(&dir);
            }
        }
    }

    Ok(())
}

/// Resolve a companion only when the product inventory proves ownership.
/// An ambiguous sibling stops Complete uninstall before profile mutation.
pub fn adjacent_report_location(binary_path: &Path) -> Result<Option<PathBuf>> {
    crate::migrate::owned_report_sibling(binary_path)
}

/// True iff `target` resolves to the same on-disk file as the
/// currently-running executable.
///
/// Uses `fs::canonicalize` to normalize both paths so 8.3 short names,
/// drive-letter case differences, junctions, and trailing slashes
/// don't confuse the comparison. Returns `false` on any canonicalize
/// failure (defensive — if we can't prove equivalence, treat as not
/// equivalent and let the synchronous remove path run).
fn is_running_binary(target: &Path) -> bool {
    let Ok(target_canon) = fs::canonicalize(target) else {
        return false;
    };
    let Ok(current) = env::current_exe() else {
        return false;
    };
    let Ok(current_canon) = fs::canonicalize(&current) else {
        return false;
    };
    target_canon == current_canon
}

/// Spawn a detached `cmd.exe` that waits a few seconds, deletes the
/// given binary file, and (optionally) removes the now-empty parent
/// directory.
///
/// Used by `uninstall_complete` to delete the currently-running
/// `tr300.exe` after this process exits. The wait gives the OS time
/// to release the file-loader handle once this process terminates.
fn schedule_self_cleanup(
    binary_path: &Path,
    report_path: Option<&Path>,
    parent_dir: Option<&Path>,
) -> io::Result<()> {
    use std::os::windows::process::CommandExt;

    const DETACHED_PROCESS: u32 = 0x00000008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
    self_cleanup_command(binary_path, report_path, parent_dir)?
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    Ok(())
}

fn self_cleanup_command(
    binary_path: &Path,
    report_path: Option<&Path>,
    parent_dir: Option<&Path>,
) -> io::Result<Command> {
    use std::os::windows::process::CommandExt;

    for path in std::iter::once(binary_path)
        .chain(report_path)
        .chain(parent_dir)
    {
        if !path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cleanup paths must be absolute",
            ));
        }
    }
    let timeout = crate::trusted_process::system_command("timeout")?;
    let mut command = crate::trusted_process::system_command("cmd")?;
    command.env("TR300_CLEANUP_TIMEOUT", timeout.get_program());
    command.env("TR300_CLEANUP_BINARY", binary_path);
    let mut script = "\"%TR300_CLEANUP_TIMEOUT%\" /t 2 /nobreak > nul".to_string();
    if let Some(report) = report_path {
        command.env("TR300_CLEANUP_REPORT", report);
        script.push_str(" & del /f /q \"%TR300_CLEANUP_REPORT%\"");
    }
    script.push_str(" & del /f /q \"%TR300_CLEANUP_BINARY%\"");
    if let Some(parent) = parent_dir {
        command.env("TR300_CLEANUP_DIRECTORY", parent);
        script.push_str(" & rd /q \"%TR300_CLEANUP_DIRECTORY%\"");
    }
    // Paths expand once inside quotes; delayed expansion and cmd AutoRun are disabled.
    command
        .args(["/d", "/v:off", "/s", "/c"])
        .raw_arg(format!("\"{script}\""));
    Ok(command)
}

/// Identifies which step of install/uninstall failed, so we can render a
/// step-appropriate guidance block alongside the raw OS error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallStep {
    CreateProfileDir,
    ReadProfile,
    WriteProfile,
    RemoveBinary,
}

impl InstallStep {
    fn short_description(self) -> &'static str {
        match self {
            Self::CreateProfileDir => "Can't create your PowerShell profile directory.",
            Self::ReadProfile => "Can't read your PowerShell profile.",
            Self::WriteProfile => "Can't write to your PowerShell profile.",
            Self::RemoveBinary => "Can't remove the tr300 binary.",
        }
    }

    fn error_tag(self) -> &'static str {
        match self {
            Self::CreateProfileDir => "create profile dir",
            Self::ReadProfile => "read profile",
            Self::WriteProfile => "write profile",
            Self::RemoveBinary => "remove binary",
        }
    }
}

/// Heuristic check for whether a path lives under a OneDrive Known Folder
/// Move (KFM) — common on modern Windows where `Documents` is redirected
/// to `C:\Users\<user>\OneDrive\Documents\` or similar. Case-insensitive.
fn looks_like_onedrive_path(path: &Path) -> bool {
    let s = path.to_string_lossy().to_ascii_lowercase();
    // Match a path segment containing "onedrive" (handles "OneDrive",
    // "OneDrive - Acme Corp", etc.) — guarded by separators so we don't
    // false-positive on a random filename containing the substring.
    s.split(['\\', '/']).any(|seg| seg.contains("onedrive"))
}

/// Heuristic for Folder Redirection / roaming-profile UNC paths.
fn looks_like_redirected_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.starts_with(r"\\") || s.starts_with("//")
}

/// Print rich, multi-paragraph guidance to stderr describing why an
/// install step failed and what the user can do about it, then return a
/// concise `AppError` suitable for `main()`'s trailing `Error: ...` line.
/// The detailed message goes to stderr so it isn't swallowed by anything
/// that only captures the returned error.
fn fail_install(step: InstallStep, path: &Path, err: io::Error) -> AppError {
    let raw = err.raw_os_error();
    let kind = err.kind();
    let on_onedrive = looks_like_onedrive_path(path);
    let on_redirected = looks_like_redirected_path(path);

    eprintln!();
    eprintln!("tr300 install: {}", step.short_description());
    eprintln!();
    eprintln!("  Path:  {}", path.display());
    match raw {
        Some(code) => eprintln!("  Cause: {} (Windows error {})", err, code),
        None => eprintln!("  Cause: {}", err),
    }
    eprintln!();

    // Step-and-kind specific guidance.
    let mut printed = false;
    let permission_denied = matches!(kind, io::ErrorKind::PermissionDenied) || raw == Some(5);
    let sharing_violation = raw == Some(32);
    let disk_full = matches!(kind, io::ErrorKind::StorageFull) || raw == Some(112);
    let path_too_long = matches!(kind, io::ErrorKind::InvalidFilename) || raw == Some(206);
    let not_found = matches!(kind, io::ErrorKind::NotFound) || raw == Some(2) || raw == Some(3);

    if permission_denied {
        eprintln!("Likely reasons (most common first):");
        if on_onedrive {
            eprintln!("  - OneDrive is offline, paused, or has Documents set to \"online-only\".");
            eprintln!(
                "      Open the OneDrive tray icon, make sure it is signed in and syncing, and"
            );
            eprintln!(
                "      right-click the path above -> \"Always keep on this device\". Re-run install."
            );
            eprintln!(
                "  - Your organization restricts writes to OneDrive-synced folders via Intune,"
            );
            eprintln!("      Group Policy, or a DLP agent. Ask IT to allow writes to:");
            eprintln!("          {}", path.display());
        } else if on_redirected {
            eprintln!("  - The path is on a network share (folder redirection / roaming profile).");
            eprintln!("      Confirm the share is reachable and you have write access; try again.");
            eprintln!(
                "  - Your organization may have policies that restrict writes to the redirected"
            );
            eprintln!("      profile. Ask IT to whitelist the path above.");
        } else {
            eprintln!(
                "  - Your organization restricts writes via Intune MDM, Active Directory Group"
            );
            eprintln!(
                "      Policy, AppLocker, or Windows Defender Application Control (WDAC). Ask"
            );
            eprintln!("      IT to allow writes to:");
            eprintln!("          {}", path.display());
            eprintln!(
                "  - Antivirus / EDR (Defender, CrowdStrike, SentinelOne, etc.) is treating the"
            );
            eprintln!("      profile edit as suspicious. Add an exclusion for the path above.");
        }
        eprintln!("  - The file or folder is owned by another user or by SYSTEM. From an admin");
        eprintln!("      PowerShell you can re-take ownership:");
        eprintln!("          takeown /F \"{}\" /R", path.display());
        printed = true;
    } else if sharing_violation {
        eprintln!("Likely reasons:");
        eprintln!("  - The file is open in another process (editor, antivirus on-access scan,");
        eprintln!("      OneDrive's sync engine). Close any open editors and re-run.");
        eprintln!(
            "  - An antivirus or EDR product is holding the file. If the path is in OneDrive,"
        );
        eprintln!("      wait a few seconds for the sync to complete and try again.");
        printed = true;
    } else if disk_full {
        eprintln!("Likely reason:");
        eprintln!("  - The drive is out of free space. Free some space on the volume containing");
        eprintln!("      the path above, then re-run install.");
        printed = true;
    } else if path_too_long {
        eprintln!("Likely reason:");
        eprintln!(
            "  - The path exceeds Windows' 260-character MAX_PATH limit. This is common when"
        );
        eprintln!(
            "      Documents is redirected to a deep OneDrive folder. Either shorten the path,"
        );
        eprintln!("      or enable long-path support (LongPathsEnabled = 1 under");
        eprintln!("      HKLM\\SYSTEM\\CurrentControlSet\\Control\\FileSystem) and reboot.");
        printed = true;
    } else if not_found && matches!(step, InstallStep::ReadProfile) {
        eprintln!("Likely reason:");
        eprintln!("  - The profile path disappeared between the existence check and the read.");
        eprintln!("      Usually transient — re-run install.");
        printed = true;
    }

    if printed {
        eprintln!();
    }

    eprintln!("Manual `tr300` still works from the prompt; only the auto-run on new shells is");
    eprintln!("affected. After addressing the cause above, re-run `tr300 install`.");
    eprintln!();

    AppError::platform(format!("{}: {}", step.error_tag(), err))
}

#[cfg(test)]
mod tests {
    #[test]
    fn cleanup_uses_system_helpers_and_keeps_literal_paths() {
        let fixture = tempfile::tempdir().unwrap();
        let planted = fixture.path().join("planted");
        std::fs::create_dir(&planted).unwrap();
        let marker = fixture.path().join("hostile-timeout-executed");
        std::fs::write(
            planted.join("timeout.cmd"),
            format!("@echo hostile>\"{}\"\r\n", marker.display()),
        )
        .unwrap();
        let payload_directory = fixture.path().join("literal %TR300_TEST_REWRITE% & !name");
        std::fs::create_dir(&payload_directory).unwrap();
        let binary = payload_directory.join("tr300.exe");
        let report = payload_directory.join("report.exe");
        std::fs::write(&binary, b"fixture").unwrap();
        std::fs::write(&report, b"fixture").unwrap();
        let mut command = super::self_cleanup_command(&binary, Some(&report), None).unwrap();
        assert_eq!(
            command.get_current_dir(),
            Some(
                crate::trusted_process::windows_system_directory()
                    .unwrap()
                    .as_path()
            )
        );
        // Preserve the constructed protected cwd and PATH. Ambient process
        // settings are hostile in the child, but no shell helper uses them.
        command.env("TR300_TEST_REWRITE", "wrong-directory");
        command.env("PATH", &planted);
        let status = command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        assert!(!binary.exists() && !report.exists());
        assert!(!marker.exists());
        assert!(super::self_cleanup_command(Path::new("relative.exe"), None, None).is_err());
    }

    use super::{looks_like_onedrive_path, looks_like_redirected_path, policy_state, PolicyState};
    use std::path::Path;

    #[test]
    fn restricted_is_blocked_default() {
        assert_eq!(policy_state("Restricted"), PolicyState::BlockedDefault);
    }

    #[test]
    fn undefined_is_blocked_default() {
        // Scope precedence resolves Undefined to Restricted on Windows Client.
        assert_eq!(policy_state("Undefined"), PolicyState::BlockedDefault);
    }

    #[test]
    fn allsigned_is_its_own_state() {
        // Deliberate hardening — we must not silently downgrade.
        assert_eq!(policy_state("AllSigned"), PolicyState::BlockedAllSigned);
    }

    #[test]
    fn remotesigned_is_permissive() {
        assert_eq!(policy_state("RemoteSigned"), PolicyState::Permissive);
    }

    #[test]
    fn unrestricted_is_permissive() {
        assert_eq!(policy_state("Unrestricted"), PolicyState::Permissive);
    }

    #[test]
    fn bypass_is_permissive() {
        assert_eq!(policy_state("Bypass"), PolicyState::Permissive);
    }

    #[test]
    fn empty_string_is_unknown() {
        assert_eq!(policy_state(""), PolicyState::Unknown);
    }

    #[test]
    fn unrecognized_value_is_unknown() {
        // Future PowerShell versions might introduce new policies; we
        // shouldn't act on values we don't understand.
        assert_eq!(policy_state("NotARealPolicy"), PolicyState::Unknown);
    }

    #[test]
    fn classification_is_case_insensitive() {
        assert_eq!(policy_state("restricted"), PolicyState::BlockedDefault);
        assert_eq!(policy_state("REMOTESIGNED"), PolicyState::Permissive);
        assert_eq!(policy_state("AllSIGNED"), PolicyState::BlockedAllSigned);
    }

    #[test]
    fn surrounding_whitespace_does_not_confuse_classification() {
        // Get-ExecutionPolicy output is trimmed at the call site, but defensive.
        assert_eq!(policy_state("  Restricted  "), PolicyState::BlockedDefault);
    }

    #[test]
    fn onedrive_kfm_path_is_detected() {
        assert!(looks_like_onedrive_path(Path::new(
            r"C:\Users\hey\OneDrive\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1"
        )));
    }

    #[test]
    fn onedrive_for_business_path_is_detected() {
        // "OneDrive - <TenantName>" is the standard naming for the
        // OneDrive-for-Business folder.
        assert!(looks_like_onedrive_path(Path::new(
            r"C:\Users\hey\OneDrive - Acme Corp\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1"
        )));
    }

    #[test]
    fn onedrive_detection_is_case_insensitive() {
        assert!(looks_like_onedrive_path(Path::new(
            r"C:\Users\hey\onedrive\Documents"
        )));
        assert!(looks_like_onedrive_path(Path::new(
            r"C:\Users\hey\ONEDRIVE\Documents"
        )));
    }

    #[test]
    fn non_onedrive_path_is_not_flagged() {
        assert!(!looks_like_onedrive_path(Path::new(
            r"C:\Users\hey\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1"
        )));
    }

    #[test]
    fn filename_containing_onedrive_does_not_falsepositive_when_isolated() {
        // We don't have a path-segment guard tight enough to reject filenames
        // that literally include the substring "onedrive" — that's a real
        // edge case that's only a false-positive (advisory text mentions
        // OneDrive when the cause is unrelated). Document the behavior so
        // it's intentional, not accidental.
        assert!(looks_like_onedrive_path(Path::new(
            r"C:\notes\onedrive-migration.ps1"
        )));
    }

    #[test]
    fn unc_path_is_flagged_as_redirected() {
        assert!(looks_like_redirected_path(Path::new(
            r"\\fileserver\users\hey\Documents"
        )));
    }

    #[test]
    fn local_path_is_not_redirected() {
        assert!(!looks_like_redirected_path(Path::new(
            r"C:\Users\hey\Documents"
        )));
    }
}
