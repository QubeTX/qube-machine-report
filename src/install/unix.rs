//! Unix/macOS installation utilities
//!
//! Adds TR-300 alias and auto-run to shell profiles.
//! Removes legacy TR-100 and TR-200 configurations before installing.

use crate::error::{AppError, Result};
use std::fs;
use std::path::PathBuf;

/// Marker comments for shell profile modifications
const MARKER_START: &str = "# TR-300 Machine Report";
const MARKER_END: &str = "# End TR-300";

/// TR-100 legacy markers (bash scripts)
const TR100_MARKERS: &[&str] = &[
    "# Run Machine Report only when in interactive mode",
];

/// TR-200 legacy markers (various installation styles)
const TR200_MARKERS: &[&str] = &[
    "# TR-200 Machine Report configuration",
    "# TR-200 Machine Report - run on login",
    "# TR-200 Machine Report - run on bash login",
    "# TR-200 Machine Report (npm) - auto-run",
];

/// Shell profile content to add
const SHELL_ADDITIONS: &str = r#"# TR-300 Machine Report
alias report='tr300'

# Auto-run on interactive shell
if [[ $- == *i* ]]; then
    tr300
fi
# End TR-300"#;

/// Get the installation path for Unix systems
pub fn install_path() -> PathBuf {
    // Prefer ~/.local/bin if it exists
    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(".local").join("bin");
        if local_bin.exists() {
            return local_bin.join("tr300");
        }
    }

    PathBuf::from("/usr/local/bin/tr300")
}

/// Install tr300 to shell profiles
pub fn install() -> Result<()> {
    let home = dirs::home_dir()
        .ok_or_else(|| AppError::platform("Could not determine home directory"))?;

    let mut modified_files = Vec::new();

    // Try to update .bashrc
    let bashrc = home.join(".bashrc");
    if bashrc.exists() {
        if update_shell_profile(&bashrc)? {
            modified_files.push(bashrc.display().to_string());
        }
    }

    // Try to update .zshrc
    let zshrc = home.join(".zshrc");
    if zshrc.exists() {
        if update_shell_profile(&zshrc)? {
            modified_files.push(zshrc.display().to_string());
        }
    }

    // If neither exists, try to create .bashrc (common default)
    if modified_files.is_empty() {
        if !bashrc.exists() {
            fs::write(&bashrc, SHELL_ADDITIONS)
                .map_err(|e| AppError::platform(format!("Failed to create {}: {}", bashrc.display(), e)))?;
            modified_files.push(bashrc.display().to_string());
        }
    }

    if modified_files.is_empty() {
        return Err(AppError::platform("No shell profile found to update"));
    }

    println!("Modified shell profiles:");
    for file in &modified_files {
        println!("  - {}", file);
    }

    Ok(())
}

/// Uninstall tr300 from shell profiles
pub fn uninstall() -> Result<()> {
    let home = dirs::home_dir()
        .ok_or_else(|| AppError::platform("Could not determine home directory"))?;

    let mut modified_files = Vec::new();

    // Try to clean .bashrc
    let bashrc = home.join(".bashrc");
    if bashrc.exists() {
        if remove_from_profile(&bashrc)? {
            modified_files.push(bashrc.display().to_string());
        }
    }

    // Try to clean .zshrc
    let zshrc = home.join(".zshrc");
    if zshrc.exists() {
        if remove_from_profile(&zshrc)? {
            modified_files.push(zshrc.display().to_string());
        }
    }

    if modified_files.is_empty() {
        println!("No TR-300 configuration found in shell profiles.");
    } else {
        println!("Cleaned shell profiles:");
        for file in &modified_files {
            println!("  - {}", file);
        }
    }

    Ok(())
}

/// Update a shell profile with TR-300 additions
/// First removes any legacy TR-100, TR-200, or existing TR-300 config
fn update_shell_profile(path: &PathBuf) -> Result<bool> {
    let content = fs::read_to_string(path)
        .map_err(|e| AppError::platform(format!("Failed to read {}: {}", path.display(), e)))?;

    // Remove legacy configurations and existing TR-300
    let cleaned_content = remove_legacy_blocks(&content);

    // Append TR-300 config to cleaned content
    let new_content = if cleaned_content.trim().is_empty() {
        format!("{}\n", SHELL_ADDITIONS)
    } else {
        format!("{}\n\n{}\n", cleaned_content.trim_end(), SHELL_ADDITIONS)
    };

    fs::write(path, &new_content)
        .map_err(|e| AppError::platform(format!("Failed to write {}: {}", path.display(), e)))?;

    Ok(true)
}

/// Remove legacy TR-100, TR-200, and existing TR-300 blocks from content
fn remove_legacy_blocks(content: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();

    // Remove TR-300 blocks (between MARKER_START and MARKER_END)
    lines = remove_delimited_block(&lines, MARKER_START, MARKER_END);

    // Remove TR-200 blocks - each marker starts a block ending at next blank line or EOF
    for marker in TR200_MARKERS {
        lines = remove_marker_block(&lines, marker);
    }

    // Remove TR-100 blocks - these use if...fi blocks
    for marker in TR100_MARKERS {
        lines = remove_if_fi_block(&lines, marker);
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
        result.join("\n") + "\n"
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

/// Remove a block starting with a marker and ending at next blank line or significant content change
fn remove_marker_block<'a>(lines: &[&'a str], marker: &str) -> Vec<&'a str> {
    let mut result = Vec::new();
    let mut skip_until_blank = false;

    for line in lines {
        if line.contains(marker) {
            skip_until_blank = true;
            continue;
        }
        if skip_until_blank {
            // Skip lines until we hit a blank line, then include the blank and continue
            if line.trim().is_empty() {
                skip_until_blank = false;
                // Don't include this blank line (it was part of the block)
            }
            continue;
        }
        result.push(*line);
    }

    result
}

/// Remove an if...fi block that starts with a marker comment
fn remove_if_fi_block<'a>(lines: &[&'a str], marker: &str) -> Vec<&'a str> {
    let mut result = Vec::new();
    let mut skip_if_block = false;
    let mut if_depth = 0;

    for line in lines {
        if line.contains(marker) {
            skip_if_block = true;
            continue;
        }
        if skip_if_block {
            let trimmed = line.trim();
            // Track nested if statements
            if trimmed.starts_with("if ") || trimmed.starts_with("if[") {
                if_depth += 1;
            } else if trimmed == "fi" || trimmed.starts_with("fi ") || trimmed.starts_with("fi;") {
                if if_depth > 0 {
                    if_depth -= 1;
                    if if_depth == 0 {
                        skip_if_block = false;
                    }
                }
            }
            continue;
        }
        result.push(*line);
    }

    result
}

/// Remove TR-300 additions from a shell profile
fn remove_from_profile(path: &PathBuf) -> Result<bool> {
    let content = fs::read_to_string(path)
        .map_err(|e| AppError::platform(format!("Failed to read {}: {}", path.display(), e)))?;

    // Check if TR-300 is configured
    if !content.contains(MARKER_START) {
        return Ok(false);
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

    let new_content = new_lines.join("\n") + "\n";

    fs::write(path, new_content)
        .map_err(|e| AppError::platform(format!("Failed to write {}: {}", path.display(), e)))?;

    Ok(true)
}
