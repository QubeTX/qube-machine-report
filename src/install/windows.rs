//! Windows installation utilities
//!
//! Adds TR-300 alias and auto-run to PowerShell profile.

use crate::error::{AppError, Result};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Marker comments for PowerShell profile modifications
const MARKER_START: &str = "# TR-300 Machine Report";
const MARKER_END: &str = "# End TR-300";

/// PowerShell profile content to add
const POWERSHELL_ADDITIONS: &str = r#"# TR-300 Machine Report
Set-Alias -Name report -Value tr300

# Auto-run on interactive shell
if ($Host.Name -eq 'ConsoleHost') {
    tr300 --fast
}
# End TR-300"#;

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

/// Get PowerShell profile path
fn get_powershell_profile() -> Option<PathBuf> {
    // Try to get profile path from PowerShell
    if let Ok(output) = Command::new("powershell")
        .args(["-NoProfile", "-Command", "$PROFILE"])
        .output()
    {
        let profile_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !profile_path.is_empty() {
            return Some(PathBuf::from(profile_path));
        }
    }

    // Fallback to default location
    if let Some(docs) = dirs::document_dir() {
        return Some(
            docs.join("WindowsPowerShell")
                .join("Microsoft.PowerShell_profile.ps1"),
        );
    }

    None
}

/// Install tr300 to PowerShell profile
pub fn install() -> Result<()> {
    // Preflight: ensure the user's execution policy allows the profile to load.
    // A fresh Windows install defaults to `Restricted`, which blocks every
    // `.ps1` file — including `$PROFILE` itself — so the auto-run we're about
    // to write would fail at the next shell start with an UnauthorizedAccess
    // PSSecurityException. Lift CurrentUser to `RemoteSigned` (the minimum
    // permissive policy that loads local unsigned scripts) when needed.
    // Non-fatal: never short-circuits the alias write.
    run_execution_policy_preflight();

    let profile_path = get_powershell_profile()
        .ok_or_else(|| AppError::platform("Could not determine PowerShell profile path"))?;

    // Create profile directory if needed
    if let Some(parent) = profile_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                AppError::platform(format!("Failed to create profile directory: {}", e))
            })?;
        }
    }

    // Read existing profile or create empty
    let existing_content = if profile_path.exists() {
        fs::read_to_string(&profile_path)
            .map_err(|e| AppError::platform(format!("Failed to read profile: {}", e)))?
    } else {
        String::new()
    };

    let cleaned_content = remove_tr300_block(&existing_content);

    // Append TR-300 config to cleaned content
    let new_content = if cleaned_content.trim().is_empty() {
        POWERSHELL_ADDITIONS.to_string()
    } else {
        format!(
            "{}\r\n\r\n{}",
            cleaned_content.trim_end(),
            POWERSHELL_ADDITIONS
        )
    };

    fs::write(&profile_path, new_content)
        .map_err(|e| AppError::platform(format!("Failed to write profile: {}", e)))?;

    println!("Modified PowerShell profile:");
    println!("  - {}", profile_path.display());

    Ok(())
}

/// Uninstall tr300 from PowerShell profile
pub fn uninstall() -> Result<()> {
    let profile_path = get_powershell_profile()
        .ok_or_else(|| AppError::platform("Could not determine PowerShell profile path"))?;

    if !profile_path.exists() {
        println!("No PowerShell profile found.");
        return Ok(());
    }

    let content = fs::read_to_string(&profile_path)
        .map_err(|e| AppError::platform(format!("Failed to read profile: {}", e)))?;

    if !content.contains(MARKER_START) {
        println!("No TR-300 configuration found in PowerShell profile.");
        return Ok(());
    }

    // Remove the TR-300 block
    let mut new_lines = Vec::new();
    let mut in_tr300_block = false;

    for line in content.lines() {
        if line.contains(MARKER_START) {
            in_tr300_block = true;
            continue;
        }
        if line.contains(MARKER_END) {
            in_tr300_block = false;
            continue;
        }
        if !in_tr300_block {
            new_lines.push(line);
        }
    }

    // Clean up extra blank lines at the end
    while new_lines.last().map(|s| s.is_empty()).unwrap_or(false) {
        new_lines.pop();
    }

    let new_content = if new_lines.is_empty() {
        String::new()
    } else {
        new_lines.join("\r\n") + "\r\n"
    };

    fs::write(&profile_path, new_content)
        .map_err(|e| AppError::platform(format!("Failed to write profile: {}", e)))?;

    println!("Cleaned PowerShell profile:");
    println!("  - {}", profile_path.display());

    Ok(())
}

/// Remove existing TR-300 blocks from content
fn remove_tr300_block(content: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();

    // Remove TR-300 blocks (between MARKER_START and MARKER_END)
    lines = remove_delimited_block(&lines, MARKER_START, MARKER_END);

    // Clean up multiple consecutive blank lines
    let mut result = Vec::new();
    let mut prev_blank = false;
    for line in lines {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        result.push(line);
        prev_blank = is_blank;
    }

    // Remove trailing blank lines
    while result.last().map(|s| s.trim().is_empty()).unwrap_or(false) {
        result.pop();
    }

    if result.is_empty() {
        String::new()
    } else {
        result.join("\r\n") + "\r\n"
    }
}

/// Remove a block delimited by start and end markers
fn remove_delimited_block<'a>(lines: &[&'a str], start: &str, end: &str) -> Vec<&'a str> {
    let mut result = Vec::new();
    let mut in_block = false;

    for line in lines {
        if line.contains(start) {
            in_block = true;
            continue;
        }
        if line.contains(end) {
            in_block = false;
            continue;
        }
        if !in_block {
            result.push(*line);
        }
    }

    result
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

    fs::remove_file(binary_path).map_err(|e| {
        AppError::platform(format!(
            "Failed to remove binary {}: {}",
            binary_path.display(),
            e
        ))
    })?;

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
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-ExecutionPolicy -Scope CurrentUser",
        ])
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
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned -Force",
        ])
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
                    "  The 'report' alias still works manually, but the auto-run on new shells won't fire."
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
                    "  The 'report' alias still works manually, but the auto-run on new shells won't fire"
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
                "  The 'report' alias still works manually. If you'd like to opt into the auto-run, you can either:"
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
    // First, uninstall from shell profiles
    uninstall()?;

    // Then remove the binary and cleanup directory
    if let Some(binary_path) = find_binary_location() {
        let parent_dir = get_binary_parent_dir(&binary_path);
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

#[cfg(test)]
mod tests {
    use super::{policy_state, PolicyState};

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
}
