//! Unix/macOS installation utilities
//!
//! Adds TR-300 alias and auto-run to shell profiles.

use crate::error::{AppError, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::shared::{MARKER_END, MARKER_START};

/// Shell profile content to add.
///
/// The auto-run block has three load-bearing guards inside the
/// interactive-shell check:
/// - `command -v tr300` — silently skip when the binary is no longer
///   on PATH (post-uninstall, cargo-uninstall, manual rm). Without
///   this, every new shell would print `bash: tr300: command not
///   found` until the user found and removed this block.
/// - `[ -z "${TR300_AUTORUN_RAN-}" ]` + `export TR300_AUTORUN_RAN=1`
///   — recursion sentinel. Nested interactive shells (`bash -i -c`,
///   vim `:term`, a Makefile's nested shell) inherit the env var and
///   the guard short-circuits so the table doesn't render multiple
///   times per top-level session.
/// - The whole `case "$-" in *i*)` wrapper restricts firing to
///   interactive shells (POSIX way to detect this).
///
/// The literal marker lines `# TR-300 Machine Report` and `# End
/// TR-300` MUST appear at the boundaries — they're matched by
/// `super::shared::remove_delimited_block` and the
/// `super::check_marker_balance` pre-write sanity check. The test
/// `shell_additions_contains_shared_markers` below pins this contract.
const SHELL_ADDITIONS: &str = r#"# TR-300 Machine Report
alias report='tr300'

# Auto-run on interactive shell; guards prevent spam-on-every-prompt
# when the binary is missing, and recursion in nested shells.
case "$-" in *i*)
    if command -v tr300 >/dev/null 2>&1 && [ -z "${TR300_AUTORUN_RAN-}" ]; then
        export TR300_AUTORUN_RAN=1
        tr300 --fast
    fi
    ;;
esac
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
    refuse_root_install()?;

    let home =
        dirs::home_dir().ok_or_else(|| AppError::platform("Could not determine home directory"))?;

    // F17 (v3.15.3+): heads-up if the user already has a `report` defined.
    // Best-effort heuristic — scans common rc files and PATH for a
    // pre-existing definition that the install is about to shadow. Read-only,
    // no subprocess, so it can't trigger rc-file side effects (fastfetch,
    // tmux auto-attach, etc.).
    warn_if_report_already_defined(&home);

    let mut modified_files = Vec::new();

    // Try to update .bashrc
    let bashrc = home.join(".bashrc");
    if bashrc.exists() && update_shell_profile(&bashrc)? {
        modified_files.push(bashrc.display().to_string());
    }

    // Try to update .zshrc
    let zshrc = home.join(".zshrc");
    if zshrc.exists() && update_shell_profile(&zshrc)? {
        modified_files.push(zshrc.display().to_string());
    }

    // If neither rc file exists, create the default for this platform.
    // macOS has defaulted to zsh since 10.15 (Catalina, 2019) — creating
    // `.bashrc` there would silently never fire because the user's
    // actual zsh shell wouldn't source it. Linux defaults remain
    // `.bashrc`.
    if modified_files.is_empty() && !bashrc.exists() && !zshrc.exists() {
        let default_rc = if cfg!(target_os = "macos") {
            &zshrc
        } else {
            &bashrc
        };
        super::atomic_write(default_rc, SHELL_ADDITIONS).map_err(|e| {
            AppError::platform(format!("Failed to create {}: {}", default_rc.display(), e))
        })?;
        modified_files.push(default_rc.display().to_string());
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

/// Warn (to stderr) when `report` is already defined in the user's shell
/// environment so the install doesn't silently shadow it.
///
/// Read-only heuristic: scans `~/.bashrc`, `~/.bash_profile`, `~/.zshrc`,
/// `~/.profile`, and `~/.bash_aliases` for `alias report=` /
/// `report ()` / `function report` declarations, plus probes
/// `~/.local/bin/report`, `~/bin/report`, `/usr/local/bin/report`, and
/// `/usr/bin/report` for an existing executable. No subprocess — so an
/// rc file's side effects (fastfetch, tmux auto-attach, MOTD echoes,
/// network probes) can't fire during `tr300 install`.
///
/// Best-effort by design: misses aliases defined in shell-specific
/// fragment files, sourced configs, or pre-built shell environment
/// modules. False negatives are acceptable — the warning is a courtesy,
/// not a contract. False positives are also acceptable — worst case the
/// user sees a one-time install-time message about a `report` they were
/// fine shadowing.
fn warn_if_report_already_defined(home: &Path) {
    let mut hits: Vec<String> = Vec::new();

    // rc-file scan. Match definitions of an alias, function, or variable
    // called `report`. The patterns are conservative — we look for the
    // word `report` immediately followed by `(` (function) or `=`
    // (alias / assignment) or whitespace then `()` (POSIX function form).
    let rc_candidates = [
        ".bashrc",
        ".bash_profile",
        ".bash_aliases",
        ".zshrc",
        ".zprofile",
        ".profile",
    ];
    for name in &rc_candidates {
        let path = home.join(name);
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for (idx, raw) in content.lines().enumerate() {
            let line = raw.trim();
            // Skip TR-300's own block so re-running install doesn't warn
            // about itself.
            if line.contains(super::shared::MARKER_START)
                || line.contains(super::shared::MARKER_END)
                || (line.contains("alias report=") && line.contains("tr300"))
            {
                continue;
            }
            let matches_alias =
                line.starts_with("alias report=") || line.starts_with("alias report =");
            let matches_fn = line.starts_with("function report")
                || line.starts_with("report()")
                || line.starts_with("report ()");
            if matches_alias || matches_fn {
                hits.push(format!("{}:{}  {}", path.display(), idx + 1, line));
            }
        }
    }

    // Filesystem scan. A file at one of these well-known paths that's
    // executable would also be shadowed by our alias.
    let bin_candidates = [
        home.join(".local").join("bin").join("report"),
        home.join("bin").join("report"),
        PathBuf::from("/usr/local/bin/report"),
        PathBuf::from("/usr/bin/report"),
    ];
    for path in &bin_candidates {
        if path.exists() && !is_our_install(path) {
            hits.push(format!("{}  (executable on PATH)", path.display()));
        }
    }

    if hits.is_empty() {
        return;
    }

    eprintln!();
    eprintln!("Note: `report` is already defined in your environment:");
    for h in &hits {
        eprintln!("    {}", h);
    }
    eprintln!("TR-300 is about to add `alias report='tr300'` to your shell profile,");
    eprintln!("which will shadow the existing definition for new interactive shells.");
    eprintln!("If you want to keep your existing `report`, edit the TR-300 block out");
    eprintln!("of your shell profile after install (search for `# TR-300 Machine Report`).");
    eprintln!();
}

/// Treat our own installed `report` executable (when the user has previously
/// installed via a build that placed a `report` symlink/binary alongside
/// tr300) as not-a-conflict. TR-300 has never shipped a `report` binary —
/// it's always been an alias — so this is mostly defensive. Returns true
/// only when the file is clearly part of a TR-300 install.
fn is_our_install(_path: &Path) -> bool {
    // TR-300 has only ever installed an alias, never a `report` binary.
    // Any `report` file we find is genuinely the user's, not ours.
    false
}

/// Refuse to run `tr300 install` as root.
///
/// `dirs::home_dir()` consults `$HOME` first, but sudoers configs
/// frequently reset `$HOME` to `/root` non-deterministically. So
/// `sudo tr300 install` ended up either:
/// - writing the alias into `/root/.bashrc` (the actual user never
///   benefits — auto-run never fires for them), or
/// - writing it into the real user's `~/.bashrc` BUT as root-owned,
///   causing `EACCES` the next time the user (non-root) tries to
///   re-run `tr300 install` for an upgrade.
///
/// Refusing up-front with an actionable message avoids both. Users who
/// want to install TR-300 system-wide should use the MSI/EXE installer
/// (on Windows) or `cargo install tr300` (cross-platform) — the
/// shell-profile flow is by design per-user.
fn refuse_root_install() -> Result<()> {
    let euid = unsafe { libc::geteuid() };
    if euid == 0 {
        return Err(AppError::platform(
            "Don't run `tr300 install` with sudo / as root — TR-300 modifies your personal shell profile (~/.bashrc / ~/.zshrc). Running as root would either write the auto-run into root's profile (no benefit to your shell) or leave root-owned files in your home directory (the next non-sudo `tr300 install` would fail with permission denied). Re-run as your normal user without sudo.",
        ));
    }
    Ok(())
}

/// Uninstall tr300 from shell profiles
pub fn uninstall() -> Result<()> {
    let home =
        dirs::home_dir().ok_or_else(|| AppError::platform("Could not determine home directory"))?;

    let mut modified_files = Vec::new();

    // Try to clean .bashrc
    let bashrc = home.join(".bashrc");
    if bashrc.exists() && remove_from_profile(&bashrc)? {
        modified_files.push(bashrc.display().to_string());
    }

    // Try to clean .zshrc
    let zshrc = home.join(".zshrc");
    if zshrc.exists() && remove_from_profile(&zshrc)? {
        modified_files.push(zshrc.display().to_string());
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
fn update_shell_profile(path: &PathBuf) -> Result<bool> {
    let content = fs::read_to_string(path)
        .map_err(|e| AppError::platform(format!("Failed to read {}: {}", path.display(), e)))?;

    // Refuse to mutate a mutilated marker block; otherwise
    // `remove_delimited_block` would silently drop everything from
    // `MARKER_START` to EOF.
    super::check_marker_balance(&content, MARKER_START, MARKER_END).map_err(AppError::platform)?;

    // One-time backup of the original rc file before any modification.
    let _ = super::backup_once(path);

    let cleaned_content = remove_tr300_block(&content);

    // Append TR-300 config to cleaned content
    let new_content = if cleaned_content.trim().is_empty() {
        format!("{}\n", SHELL_ADDITIONS)
    } else {
        format!("{}\n\n{}\n", cleaned_content.trim_end(), SHELL_ADDITIONS)
    };

    super::atomic_write(path, &new_content)
        .map_err(|e| AppError::platform(format!("Failed to write {}: {}", path.display(), e)))?;

    Ok(true)
}

/// Remove existing TR-300 blocks from content
fn remove_tr300_block(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let lines = super::shared::remove_delimited_block(&lines, MARKER_START, MARKER_END);

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

/// Remove TR-300 additions from a shell profile
fn remove_from_profile(path: &PathBuf) -> Result<bool> {
    let content = fs::read_to_string(path)
        .map_err(|e| AppError::platform(format!("Failed to read {}: {}", path.display(), e)))?;

    // Check if TR-300 is configured
    if !content.contains(MARKER_START) {
        return Ok(false);
    }

    // Refuse to mutate a mutilated marker block — same hazard as on
    // install. Without this, an uninstall on a hand-edited rc file
    // would drop everything from `MARKER_START` to EOF.
    super::check_marker_balance(&content, MARKER_START, MARKER_END).map_err(AppError::platform)?;

    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines = super::shared::remove_delimited_block(&lines, MARKER_START, MARKER_END);

    // Clean up extra blank lines at the end
    while new_lines.last().map(|s| s.is_empty()).unwrap_or(false) {
        new_lines.pop();
    }

    let new_content = new_lines.join("\n") + "\n";

    super::atomic_write(path, &new_content)
        .map_err(|e| AppError::platform(format!("Failed to write {}: {}", path.display(), e)))?;

    Ok(true)
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

/// Perform complete uninstall (profile + binary)
pub fn uninstall_complete() -> Result<()> {
    // First, uninstall from shell profiles
    uninstall()?;

    // Then remove the binary
    if let Some(binary_path) = find_binary_location() {
        remove_binary(&binary_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        remove_from_profile, update_shell_profile, MARKER_END, MARKER_START, SHELL_ADDITIONS,
    };
    use crate::install::shared::{ALIAS_NAME, AUTORUN_SENTINEL_VAR, BINARY_NAME};

    #[test]
    fn shell_additions_contains_shared_markers() {
        // Pins the contract that the literal snippet uses the same
        // marker text as `super::shared` exposes. A drift here
        // breaks both the install-time block parser and the
        // uninstall-time cleanup path.
        assert!(SHELL_ADDITIONS.contains(MARKER_START));
        assert!(SHELL_ADDITIONS.contains(MARKER_END));
        assert!(SHELL_ADDITIONS.contains(ALIAS_NAME));
        assert!(SHELL_ADDITIONS.contains(BINARY_NAME));
    }

    #[test]
    fn shell_additions_has_path_guard() {
        // F4 hardening: must not invoke `tr300` unconditionally.
        // `command -v` is the POSIX-standard "is this on PATH?"
        // primitive. Without it, every new shell prints a "command
        // not found" error after the binary is uninstalled.
        assert!(SHELL_ADDITIONS.contains("command -v tr300"));
    }

    #[test]
    fn shell_additions_has_recursion_sentinel() {
        // F4 hardening: must set + check `TR300_AUTORUN_RAN` to
        // break recursion into nested shells (vim :term, bash -i,
        // make).
        assert!(SHELL_ADDITIONS.contains(AUTORUN_SENTINEL_VAR));
        assert!(SHELL_ADDITIONS.contains("export TR300_AUTORUN_RAN=1"));
    }

    #[test]
    fn shell_additions_gates_on_interactive_shell() {
        // `case "$-" in *i*)` is the POSIX-standard check for the
        // shell's interactive flag. Required to keep the table from
        // rendering in non-interactive script invocations.
        assert!(SHELL_ADDITIONS.contains(r#"case "$-" in *i*"#));
    }

    #[test]
    fn zsh_profile_round_trip_is_idempotent_and_preserves_original_backup() {
        let dir = tempfile::tempdir().unwrap();
        let profile = dir.path().join(".zshrc");
        std::fs::write(&profile, "export KEEP_ME=yes\n").unwrap();

        assert!(update_shell_profile(&profile).unwrap());
        assert!(update_shell_profile(&profile).unwrap());
        let installed = std::fs::read_to_string(&profile).unwrap();
        assert_eq!(installed.matches(MARKER_START).count(), 1);
        assert_eq!(installed.matches(MARKER_END).count(), 1);
        assert!(installed.contains("export KEEP_ME=yes"));
        assert_eq!(
            std::fs::read_to_string(dir.path().join(".zshrc.tr300-backup")).unwrap(),
            "export KEEP_ME=yes\n"
        );

        assert!(remove_from_profile(&profile).unwrap());
        assert_eq!(
            std::fs::read_to_string(&profile).unwrap(),
            "export KEEP_ME=yes\n"
        );
    }
}
