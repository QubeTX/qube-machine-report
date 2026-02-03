//! Windows installation utilities
//!
//! Adds TR-300 alias and auto-run to PowerShell profile.
//! Removes legacy TR-100 and TR-200 configurations before installing.

use crate::error::{AppError, Result};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Marker comments for PowerShell profile modifications
const MARKER_START: &str = "# TR-300 Machine Report";
const MARKER_END: &str = "# End TR-300";

/// TR-100 legacy markers (PowerShell scripts)
const TR100_MARKERS: &[&str] = &[
    "# Run Machine Report only when in interactive mode",
    "# TR-100 Machine Report",
];

/// TR-200 delimited block markers
const TR200_START: &str = "# >>> TR-200 Machine Report configuration >>>";
const TR200_END: &str = "# <<< TR-200 Machine Report configuration <<<";

/// TR-200 npm-style marker
const TR200_NPM: &str = "# TR-200 Machine Report (npm) - auto-run";

/// PowerShell profile content to add
const POWERSHELL_ADDITIONS: &str = r#"# TR-300 Machine Report
Set-Alias -Name report -Value tr300

# Auto-run on interactive shell
if ($Host.Name -eq 'ConsoleHost') {
    tr300
}
# End TR-300"#;

/// Get the installation path for Windows
pub fn install_path() -> PathBuf {
    // Use %LOCALAPPDATA%\Programs\tr300
    if let Some(local_app_data) = dirs::data_local_dir() {
        return local_app_data.join("Programs").join("tr300").join("tr300.exe");
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
        return Some(docs
            .join("WindowsPowerShell")
            .join("Microsoft.PowerShell_profile.ps1"));
    }

    None
}

/// Install tr300 to PowerShell profile
pub fn install() -> Result<()> {
    let profile_path = get_powershell_profile()
        .ok_or_else(|| AppError::platform("Could not determine PowerShell profile path"))?;

    // Create profile directory if needed
    if let Some(parent) = profile_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| AppError::platform(format!("Failed to create profile directory: {}", e)))?;
        }
    }

    // Read existing profile or create empty
    let existing_content = if profile_path.exists() {
        fs::read_to_string(&profile_path)
            .map_err(|e| AppError::platform(format!("Failed to read profile: {}", e)))?
    } else {
        String::new()
    };

    // Remove legacy configurations and existing TR-300
    let cleaned_content = remove_legacy_blocks(&existing_content);

    // Append TR-300 config to cleaned content
    let new_content = if cleaned_content.trim().is_empty() {
        POWERSHELL_ADDITIONS.to_string()
    } else {
        format!("{}\r\n\r\n{}", cleaned_content.trim_end(), POWERSHELL_ADDITIONS)
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

/// Remove legacy TR-100, TR-200, and existing TR-300 blocks from content
fn remove_legacy_blocks(content: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();

    // Remove TR-300 blocks (between MARKER_START and MARKER_END)
    lines = remove_delimited_block(&lines, MARKER_START, MARKER_END);

    // Remove TR-200 delimited blocks (between >>> and <<<)
    lines = remove_delimited_block(&lines, TR200_START, TR200_END);

    // Remove TR-200 npm-style blocks
    lines = remove_marker_block(&lines, TR200_NPM);

    // Remove TR-100 blocks - these use if...} blocks in PowerShell
    for marker in TR100_MARKERS {
        lines = remove_if_block(&lines, marker);
    }

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

/// Remove a block starting with a marker and ending at next blank line
fn remove_marker_block<'a>(lines: &[&'a str], marker: &str) -> Vec<&'a str> {
    let mut result = Vec::new();
    let mut skip_until_blank = false;

    for line in lines {
        if line.contains(marker) {
            skip_until_blank = true;
            continue;
        }
        if skip_until_blank {
            if line.trim().is_empty() {
                skip_until_blank = false;
            }
            continue;
        }
        result.push(*line);
    }

    result
}

/// Remove an if...} block that starts with a marker comment (PowerShell style)
fn remove_if_block<'a>(lines: &[&'a str], marker: &str) -> Vec<&'a str> {
    let mut result = Vec::new();
    let mut skip_if_block = false;
    let mut brace_depth = 0;

    for line in lines {
        if line.contains(marker) {
            skip_if_block = true;
            continue;
        }
        if skip_if_block {
            // Count braces to handle nested blocks
            for ch in line.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => {
                        if brace_depth > 0 {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                skip_if_block = false;
                            }
                        }
                    }
                    _ => {}
                }
            }
            continue;
        }
        result.push(*line);
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

    fs::remove_file(binary_path)
        .map_err(|e| AppError::platform(format!("Failed to remove binary {}: {}", binary_path.display(), e)))?;

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
        fs::remove_dir(dir)
            .map_err(|e| AppError::platform(format!("Failed to remove directory {}: {}", dir.display(), e)))?;
        println!("Removed empty directory: {}", dir.display());
    }

    Ok(())
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
