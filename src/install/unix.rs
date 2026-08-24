//! Unix/macOS installation utilities
//!
//! Adds TR-300 alias and auto-run to shell profiles.

use crate::error::{AppError, Result};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProfileFileIdentity {
    device: u64,
    inode: u64,
    len: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl ProfileFileIdentity {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            len: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        }
    }

    fn same_staged_object(self, other: Self) -> bool {
        self.device == other.device
            && self.inode == other.inode
            && self.len == other.len
            && self.modified_seconds == other.modified_seconds
            && self.modified_nanoseconds == other.modified_nanoseconds
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProfileSnapshot {
    display_path: PathBuf,
    target_path: PathBuf,
    contents: String,
    identity: ProfileFileIdentity,
    mode: u32,
}

#[derive(Debug)]
struct ProfileCleanupEntry {
    original: ProfileSnapshot,
    cleaned_contents: Option<String>,
    display_link: Option<ProfileSymlinkProof>,
    additional_displays: Vec<ProfileDisplayBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProfileSymlinkProof {
    identity: ProfileFileIdentity,
    link_target: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProfileDisplayBinding {
    path: PathBuf,
    link: Option<ProfileSymlinkProof>,
}

#[derive(Debug, Default)]
struct ProfileCleanupPlan {
    entries: Vec<ProfileCleanupEntry>,
    missing_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstalledProfileProof {
    identity: ProfileFileIdentity,
    contents: String,
    mode: u32,
}

impl InstalledProfileProof {
    fn matches(&self, snapshot: &ProfileSnapshot) -> bool {
        snapshot.identity.same_staged_object(self.identity)
            && snapshot.contents == self.contents
            && snapshot.mode == self.mode
    }
}

impl ProfileCleanupPlan {
    fn displayed_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        for entry in self
            .entries
            .iter()
            .filter(|entry| entry.cleaned_contents.is_some())
        {
            paths.push(entry.original.display_path.display().to_string());
            paths.extend(
                entry
                    .additional_displays
                    .iter()
                    .map(|binding| binding.path.display().to_string()),
            );
        }
        paths
    }
}

#[derive(Debug)]
struct StagedProfileCleanup {
    entry: ProfileCleanupEntry,
    directory: PathBuf,
    staged_original: PathBuf,
    installed: Option<InstalledProfileProof>,
}

#[derive(Debug, Default)]
struct AppliedProfileCleanup {
    entries: Vec<StagedProfileCleanup>,
    unchanged_entries: Vec<ProfileCleanupEntry>,
    missing_paths: Vec<PathBuf>,
}

impl AppliedProfileCleanup {
    fn revalidate(&self) -> Result<()> {
        for entry in &self.entries {
            validate_installed_profile(entry)?;
        }
        for entry in &self.unchanged_entries {
            revalidate_profile_entry(entry)?;
        }
        revalidate_missing_profiles(&self.missing_paths)?;
        Ok(())
    }

    fn rollback(self) -> Vec<String> {
        self.entries
            .into_iter()
            .rev()
            .filter_map(|entry| entry.rollback().err())
            .collect()
    }

    fn finalize(self) -> Vec<String> {
        self.entries
            .into_iter()
            .filter_map(StagedProfileCleanup::finalize)
            .collect()
    }
}

fn profile_error(message: impl Into<String>) -> AppError {
    AppError::platform(format!("Shell profile cleanup stopped: {}", message.into()))
}

fn read_profile_symlink_proof(path: &Path) -> Result<Option<ProfileSymlinkProof>> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        profile_error(format!(
            "could not inspect profile path {}: {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() {
        let link_target = fs::read_link(path).map_err(|error| {
            profile_error(format!(
                "could not read symlinked profile {}: {error}",
                path.display()
            ))
        })?;
        Ok(Some(ProfileSymlinkProof {
            identity: ProfileFileIdentity::from_metadata(&metadata),
            link_target,
        }))
    } else {
        Ok(None)
    }
}

fn resolve_profile_target(path: &Path) -> Result<(PathBuf, Option<ProfileSymlinkProof>)> {
    let before = read_profile_symlink_proof(path)?;
    // Canonicalize every existing display path, not only a final-component
    // symlink. A user's home itself may be a symlink while `.bashrc` is regular;
    // another profile alias can then name the same inode through its canonical
    // parent. Keeping one canonical target makes that shared object one
    // transaction entry instead of staging it twice through lexical aliases.
    let target = fs::canonicalize(path).map_err(|error| {
        profile_error(format!(
            "could not resolve profile {}: {error}",
            path.display()
        ))
    })?;
    let after = read_profile_symlink_proof(path)?;
    if after != before {
        return Err(profile_error(format!(
            "profile path {} changed while its symlink target was being resolved; no shell profile was changed",
            path.display()
        )));
    }
    Ok((target, before))
}

fn revalidate_profile_display(entry: &ProfileCleanupEntry, target_must_exist: bool) -> Result<()> {
    revalidate_profile_display_binding(
        &entry.original.display_path,
        entry.display_link.as_ref(),
        &entry.original.target_path,
        target_must_exist,
    )?;
    for binding in &entry.additional_displays {
        revalidate_profile_display_binding(
            &binding.path,
            binding.link.as_ref(),
            &entry.original.target_path,
            target_must_exist,
        )?;
    }
    Ok(())
}

fn revalidate_profile_display_binding(
    display_path: &Path,
    expected_link: Option<&ProfileSymlinkProof>,
    target_path: &Path,
    target_must_exist: bool,
) -> Result<()> {
    if target_must_exist {
        let (resolved, proof) = resolve_profile_target(display_path)?;
        if resolved != target_path || proof.as_ref() != expected_link {
            return Err(profile_error(format!(
                "{} no longer resolves through the confirmed symlink to {}; no shell profile was changed",
                display_path.display(),
                target_path.display()
            )));
        }
    } else if expected_link.is_some()
        && read_profile_symlink_proof(display_path)?.as_ref() != expected_link
    {
        return Err(profile_error(format!(
            "symlinked profile {} changed while its confirmed target was staged",
            display_path.display()
        )));
    }
    Ok(())
}

fn read_profile_snapshot(
    display_path: &Path,
    target_path: &Path,
    label: &str,
) -> Result<ProfileSnapshot> {
    let path_metadata = fs::symlink_metadata(target_path).map_err(|error| {
        profile_error(format!(
            "could not inspect {label} {}: {error}",
            target_path.display()
        ))
    })?;
    if !path_metadata.file_type().is_file() {
        return Err(profile_error(format!(
            "{label} {} is not a regular file; no shell profile was changed",
            target_path.display()
        )));
    }
    if path_metadata.nlink() != 1 {
        return Err(profile_error(format!(
            "{label} {} has {} hard links; no shell profile was changed",
            target_path.display(),
            path_metadata.nlink()
        )));
    }
    let expected_uid = unsafe { libc::geteuid() };
    if path_metadata.uid() != expected_uid {
        return Err(profile_error(format!(
            "{label} {} is owned by uid {}, not the current uid {}; no shell profile was changed",
            target_path.display(),
            path_metadata.uid(),
            expected_uid
        )));
    }

    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(target_path)
        .map_err(|error| {
            profile_error(format!(
                "could not safely open {label} {}: {error}",
                target_path.display()
            ))
        })?;
    let opened_metadata = file.metadata().map_err(|error| {
        profile_error(format!(
            "could not inspect opened {label} {}: {error}",
            target_path.display()
        ))
    })?;
    if !opened_metadata.file_type().is_file()
        || ProfileFileIdentity::from_metadata(&opened_metadata)
            != ProfileFileIdentity::from_metadata(&path_metadata)
        || opened_metadata.uid() != expected_uid
        || opened_metadata.nlink() != 1
    {
        return Err(profile_error(format!(
            "{label} {} changed while it was being validated; no shell profile was changed",
            target_path.display()
        )));
    }

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|error| {
        profile_error(format!(
            "could not read {label} {}: {error}",
            target_path.display()
        ))
    })?;
    let final_metadata = file.metadata().map_err(|error| {
        profile_error(format!(
            "could not recheck opened {label} {}: {error}",
            target_path.display()
        ))
    })?;
    if ProfileFileIdentity::from_metadata(&final_metadata)
        != ProfileFileIdentity::from_metadata(&opened_metadata)
    {
        return Err(profile_error(format!(
            "{label} {} changed while it was being read; no shell profile was changed",
            target_path.display()
        )));
    }
    let contents = String::from_utf8(bytes).map_err(|error| {
        profile_error(format!(
            "{label} {} is not UTF-8 ({error}); no shell profile was changed",
            target_path.display()
        ))
    })?;

    Ok(ProfileSnapshot {
        display_path: display_path.to_path_buf(),
        target_path: target_path.to_path_buf(),
        contents,
        identity: ProfileFileIdentity::from_metadata(&opened_metadata),
        mode: opened_metadata.mode(),
    })
}

fn cleaned_profile_contents(content: &str) -> Result<Option<String>> {
    if !content.contains(MARKER_START) {
        return Ok(None);
    }
    super::check_marker_balance(content, MARKER_START, MARKER_END).map_err(AppError::platform)?;

    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines = super::shared::remove_delimited_block(&lines, MARKER_START, MARKER_END);
    while new_lines
        .last()
        .map(|line| line.is_empty())
        .unwrap_or(false)
    {
        new_lines.pop();
    }
    Ok(Some(new_lines.join("\n") + "\n"))
}

fn prepare_profile_cleanup_for_home(home: &Path) -> Result<ProfileCleanupPlan> {
    let mut entries: Vec<ProfileCleanupEntry> = Vec::new();
    let mut missing_paths = Vec::new();
    for profile_name in [".bashrc", ".zshrc"] {
        let display_path = home.join(profile_name);
        match fs::symlink_metadata(&display_path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing_paths.push(display_path);
                continue;
            }
            Err(error) => {
                return Err(profile_error(format!(
                    "could not inspect {}: {error}",
                    display_path.display()
                )))
            }
            Ok(_) => {}
        }
        let (target_path, display_link) = resolve_profile_target(&display_path)?;
        if let Some(existing) = entries
            .iter_mut()
            .find(|entry| entry.original.target_path == target_path)
        {
            existing.additional_displays.push(ProfileDisplayBinding {
                path: display_path,
                link: display_link,
            });
            continue;
        }
        let original = read_profile_snapshot(&display_path, &target_path, "shell profile")?;
        let cleaned_contents = cleaned_profile_contents(&original.contents)?;
        entries.push(ProfileCleanupEntry {
            original,
            cleaned_contents,
            display_link,
            additional_displays: Vec::new(),
        });
    }
    Ok(ProfileCleanupPlan {
        entries,
        missing_paths,
    })
}

fn prepare_profile_cleanup() -> Result<ProfileCleanupPlan> {
    let home =
        dirs::home_dir().ok_or_else(|| AppError::platform("Could not determine home directory"))?;
    prepare_profile_cleanup_for_home(&home)
}

fn revalidate_profile_cleanup(plan: &ProfileCleanupPlan) -> Result<()> {
    for entry in &plan.entries {
        revalidate_profile_entry(entry)?;
    }
    revalidate_missing_profiles(&plan.missing_paths)?;
    Ok(())
}

fn revalidate_profile_entry(entry: &ProfileCleanupEntry) -> Result<()> {
    revalidate_profile_display(entry, true)?;
    let refreshed = read_profile_snapshot(
        &entry.original.display_path,
        &entry.original.target_path,
        "shell profile",
    )?;
    if refreshed != entry.original {
        return Err(profile_error(format!(
            "{} changed after preflight; no shell profile was changed",
            entry.original.display_path.display()
        )));
    }
    Ok(())
}

fn revalidate_missing_profiles(paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        match fs::symlink_metadata(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(_) => {
                return Err(profile_error(format!(
                    "a shell profile appeared at {} after preflight; no shell profile was changed",
                    path.display()
                )))
            }
            Err(error) => {
                return Err(profile_error(format!(
                    "could not recheck absent shell profile {}: {error}",
                    path.display()
                )))
            }
        }
    }
    Ok(())
}

fn restore_staged_profile(
    staged_original: &Path,
    target_path: &Path,
    directory: &Path,
) -> std::result::Result<(), String> {
    if let Err(error) = fs::hard_link(staged_original, target_path) {
        return Err(format!(
            "could not restore {} ({error}); the exact prior profile remains at {}",
            target_path.display(),
            staged_original.display()
        ));
    }
    if let Err(error) = fs::remove_file(staged_original) {
        return Err(format!(
            "restored {}, but its extra staged link remains at {} ({error})",
            target_path.display(),
            staged_original.display()
        ));
    }
    fs::remove_dir(directory).map_err(|error| {
        format!(
            "restored {}, but private staging directory {} remains ({error})",
            target_path.display(),
            directory.display()
        )
    })
}

fn stage_profile(entry: ProfileCleanupEntry) -> Result<StagedProfileCleanup> {
    revalidate_profile_display(&entry, true)?;
    let refreshed = read_profile_snapshot(
        &entry.original.display_path,
        &entry.original.target_path,
        "shell profile",
    )?;
    if refreshed != entry.original {
        return Err(profile_error(format!(
            "{} changed after preflight; no shell profile was changed",
            entry.original.display_path.display()
        )));
    }

    let parent = entry.original.target_path.parent().ok_or_else(|| {
        profile_error(format!(
            "profile path has no parent: {}",
            entry.original.target_path.display()
        ))
    })?;
    let directory = tempfile::Builder::new()
        .prefix(".tr300-profile-uninstall-")
        .tempdir_in(parent)
        .map_err(|error| {
            profile_error(format!(
                "could not create private staging beside {}: {error}",
                entry.original.target_path.display()
            ))
        })?
        .keep();
    let staged_original = directory.join(
        entry
            .original
            .target_path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("profile")),
    );
    if let Err(error) = fs::rename(&entry.original.target_path, &staged_original) {
        let cleanup = fs::remove_dir(&directory)
            .err()
            .map(|cleanup_error| {
                format!(
                    "; private staging directory {} remains because cleanup failed: {cleanup_error}",
                    directory.display()
                )
            })
            .unwrap_or_default();
        return Err(profile_error(format!(
            "could not stage {}: {error}{cleanup}",
            entry.original.target_path.display()
        )));
    }

    let staged_snapshot = read_profile_snapshot(
        &entry.original.display_path,
        &staged_original,
        "staged shell profile",
    );
    let valid = matches!(
        staged_snapshot,
        Ok(ref snapshot)
            if snapshot.identity.same_staged_object(entry.original.identity)
                && snapshot.contents == entry.original.contents
                && snapshot.mode == entry.original.mode
    );
    if !valid {
        let validation = staged_snapshot
            .err()
            .map(|error| error.to_string())
            .unwrap_or_else(|| "staged profile identity or contents changed".to_string());
        let restore =
            restore_staged_profile(&staged_original, &entry.original.target_path, &directory)
                .err()
                .unwrap_or_else(|| "restored the original shell profile".to_string());
        return Err(profile_error(format!("{validation}; {restore}")));
    }

    if let Err(error) = revalidate_profile_display(&entry, false) {
        let restore =
            restore_staged_profile(&staged_original, &entry.original.target_path, &directory)
                .err()
                .unwrap_or_else(|| "restored the original shell profile".to_string());
        return Err(profile_error(format!("{error}; {restore}")));
    }

    Ok(StagedProfileCleanup {
        entry,
        directory,
        staged_original,
        installed: None,
    })
}

fn persist_cleaned_profile(staged: &mut StagedProfileCleanup) -> Result<()> {
    let entry = &staged.entry;
    let target_path = &entry.original.target_path;
    let cleaned_contents = entry.cleaned_contents.as_ref().ok_or_else(|| {
        profile_error(format!(
            "internal profile plan for {} has no cleaned contents",
            target_path.display()
        ))
    })?;
    match fs::symlink_metadata(target_path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Ok(_) => {
            return Err(profile_error(format!(
                "a new path appeared at {}; refusing to clobber it",
                target_path.display()
            )))
        }
        Err(error) => {
            return Err(profile_error(format!(
                "could not verify that {} remained absent: {error}",
                target_path.display()
            )))
        }
    }
    let parent = target_path.parent().ok_or_else(|| {
        profile_error(format!(
            "profile path has no parent: {}",
            target_path.display()
        ))
    })?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".tr300-profile-cleaned-")
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(|error| {
            profile_error(format!(
                "could not create cleaned profile beside {}: {error}",
                target_path.display()
            ))
        })?;
    temporary
        .as_file()
        .set_permissions(fs::Permissions::from_mode(entry.original.mode))
        .and_then(|()| temporary.write_all(cleaned_contents.as_bytes()))
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| {
            profile_error(format!(
                "could not prepare cleaned profile for {}: {error}",
                target_path.display()
            ))
        })?;
    let temporary_metadata = temporary.as_file().metadata().map_err(|error| {
        profile_error(format!(
            "could not capture cleaned-profile identity for {}: {error}",
            target_path.display()
        ))
    })?;
    let installed_proof = InstalledProfileProof {
        identity: ProfileFileIdentity::from_metadata(&temporary_metadata),
        contents: cleaned_contents.clone(),
        mode: temporary_metadata.mode(),
    };
    temporary.persist_noclobber(target_path).map_err(|error| {
        profile_error(format!(
            "could not install cleaned profile at {} without clobbering another path: {}",
            target_path.display(),
            error.error
        ))
    })?;
    // Record rollback custody immediately after the no-clobber persist. No
    // fallible post-persist validation may run before this assignment.
    staged.installed = Some(installed_proof);
    Ok(())
}

fn validate_installed_profile(staged: &StagedProfileCleanup) -> Result<()> {
    let entry = &staged.entry;
    let target_path = &entry.original.target_path;
    revalidate_profile_display(entry, true)?;
    let proof = staged.installed.as_ref().ok_or_else(|| {
        profile_error(format!(
            "cleaned profile at {} has no rollback identity",
            target_path.display()
        ))
    })?;
    let installed = read_profile_snapshot(
        &entry.original.display_path,
        target_path,
        "cleaned shell profile",
    )?;
    if !proof.matches(&installed) {
        return Err(profile_error(format!(
            "cleaned profile at {} did not retain the prepared inode, contents, and mode",
            target_path.display()
        )));
    }
    Ok(())
}

impl StagedProfileCleanup {
    fn rollback(self) -> std::result::Result<(), String> {
        let target_path = &self.entry.original.target_path;
        if let Some(installed) = self.installed {
            match fs::symlink_metadata(target_path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return restore_staged_profile(
                        &self.staged_original,
                        target_path,
                        &self.directory,
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    return Err(format!(
                        "could not inspect cleaned profile before rollback ({error}); the exact prior profile remains at {}",
                        self.staged_original.display()
                    ));
                }
            }
            let current = read_profile_snapshot(
                &self.entry.original.display_path,
                target_path,
                "cleaned shell profile",
            )
            .map_err(|error| {
                format!(
                    "could not verify cleaned profile before rollback ({error}); the exact prior profile remains at {}",
                    self.staged_original.display()
                )
            })?;
            if !installed.matches(&current) {
                return Err(format!(
                    "{} changed after cleanup; refusing to clobber it during rollback; the exact prior profile remains at {}",
                    target_path.display(),
                    self.staged_original.display()
                ));
            }
            let mut staged_cleaned = self.directory.join(".tr300-cleaned-profile");
            if staged_cleaned == self.staged_original {
                staged_cleaned = self.directory.join(".tr300-cleaned-profile-rollback");
            }
            fs::rename(target_path, &staged_cleaned).map_err(|error| {
                format!(
                    "could not stage cleaned {} for rollback ({error}); the exact prior profile remains at {}",
                    target_path.display(),
                    self.staged_original.display()
                )
            })?;
            let moved = read_profile_snapshot(
                &self.entry.original.display_path,
                &staged_cleaned,
                "staged cleaned shell profile",
            );
            let moved_is_exact = matches!(
                moved,
                Ok(ref snapshot) if installed.matches(snapshot)
            );
            if !moved_is_exact {
                let cleaned_restore = fs::hard_link(&staged_cleaned, target_path);
                return Err(format!(
                    "cleaned profile identity changed during rollback; {}the exact prior profile remains at {} and the moved file remains at {}",
                    if cleaned_restore.is_ok() {
                        "restored the moved profile; "
                    } else {
                        "could not restore the moved profile; "
                    },
                    self.staged_original.display(),
                    staged_cleaned.display()
                ));
            }
            if let Err(error) = fs::hard_link(&self.staged_original, target_path) {
                let cleaned_restore = fs::hard_link(&staged_cleaned, target_path);
                return Err(format!(
                    "could not restore exact prior profile {} ({error}); {}prior bytes remain at {} and cleaned bytes remain at {}",
                    target_path.display(),
                    if cleaned_restore.is_ok() {
                        "restored the cleaned profile; "
                    } else {
                        "could not restore the cleaned profile; "
                    },
                    self.staged_original.display(),
                    staged_cleaned.display()
                ));
            }
            let mut residue = Vec::new();
            if let Err(error) = fs::remove_file(&self.staged_original) {
                residue.push(format!(
                    "extra prior-profile link {} ({error})",
                    self.staged_original.display()
                ));
            }
            if let Err(error) = fs::remove_file(&staged_cleaned) {
                residue.push(format!(
                    "cleaned-profile residue {} ({error})",
                    staged_cleaned.display()
                ));
            }
            if residue.is_empty() {
                if let Err(error) = fs::remove_dir(&self.directory) {
                    residue.push(format!(
                        "private staging directory {} ({error})",
                        self.directory.display()
                    ));
                }
            }
            if residue.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "restored {}, but cleanup left {}",
                    target_path.display(),
                    residue.join("; ")
                ))
            }
        } else {
            restore_staged_profile(&self.staged_original, target_path, &self.directory)
        }
    }

    fn finalize(self) -> Option<String> {
        if let Err(error) = validate_installed_profile(&self) {
            return Some(format!(
                "could not revalidate cleaned {} after ownership cleanup ({error}); the exact prior profile remains at {}",
                self.entry.original.display_path.display(),
                self.staged_original.display()
            ));
        }
        if let Err(error) = fs::remove_file(&self.staged_original) {
            return Some(format!(
                "cleaned {}, but the exact prior profile remains at {} because staged cleanup failed: {error}",
                self.entry.original.display_path.display(),
                self.staged_original.display()
            ));
        }
        fs::remove_dir(&self.directory).err().map(|error| {
            format!(
                "cleaned {}, but private staging directory {} remains: {error}",
                self.entry.original.display_path.display(),
                self.directory.display()
            )
        })
    }
}

fn rollback_staged_profiles(entries: Vec<StagedProfileCleanup>) -> String {
    let failures: Vec<String> = entries
        .into_iter()
        .rev()
        .filter_map(|entry| entry.rollback().err())
        .collect();
    if failures.is_empty() {
        "restored every shell profile".to_string()
    } else {
        format!(
            "shell-profile rollback needs attention: {}",
            failures.join("; ")
        )
    }
}

fn apply_profile_cleanup_with(
    plan: ProfileCleanupPlan,
    mut before_install: impl FnMut(usize, &Path) -> std::io::Result<()>,
    mut after_persist: impl FnMut(usize, &Path) -> std::io::Result<()>,
) -> Result<AppliedProfileCleanup> {
    revalidate_profile_cleanup(&plan)?;
    let ProfileCleanupPlan {
        entries,
        missing_paths,
    } = plan;
    let mut staged = Vec::with_capacity(entries.len());
    let mut unchanged_entries = Vec::new();
    for entry in entries {
        if entry.cleaned_contents.is_none() {
            unchanged_entries.push(entry);
            continue;
        }
        match stage_profile(entry) {
            Ok(entry) => staged.push(entry),
            Err(error) => {
                let rollback = rollback_staged_profiles(staged);
                return Err(profile_error(format!("{error}; {rollback}")));
            }
        }
    }

    for index in 0..staged.len() {
        let target_path = staged[index].entry.original.target_path.clone();
        if let Err(error) = before_install(index, &target_path) {
            let rollback = rollback_staged_profiles(staged);
            return Err(profile_error(format!(
                "could not continue cleaned-profile installation at {}: {error}; {rollback}",
                target_path.display()
            )));
        }
        if let Err(error) = persist_cleaned_profile(&mut staged[index]) {
            let rollback = rollback_staged_profiles(staged);
            return Err(profile_error(format!("{error}; {rollback}")));
        }
        if let Err(error) = after_persist(index, &target_path) {
            let rollback = rollback_staged_profiles(staged);
            return Err(profile_error(format!(
                "post-persist validation could not continue at {}: {error}; {rollback}",
                target_path.display()
            )));
        }
        if let Err(error) = validate_installed_profile(&staged[index]) {
            let rollback = rollback_staged_profiles(staged);
            return Err(profile_error(format!("{error}; {rollback}")));
        }
    }

    let applied = AppliedProfileCleanup {
        entries: staged,
        unchanged_entries,
        missing_paths,
    };
    if let Err(error) = applied.revalidate() {
        let rollback = rollback_staged_profiles(applied.entries);
        return Err(profile_error(format!("{error}; {rollback}")));
    }
    Ok(applied)
}

fn apply_profile_cleanup(plan: ProfileCleanupPlan) -> Result<AppliedProfileCleanup> {
    apply_profile_cleanup_with(plan, |_, _| Ok(()), |_, _| Ok(()))
}

fn report_profile_cleanup(modified_files: &[String], warnings: Vec<String>) {
    if modified_files.is_empty() {
        println!("No TR-300 configuration found in shell profiles.");
    } else {
        println!("Cleaned shell profiles:");
        for file in modified_files {
            println!("  - {file}");
        }
    }
    for warning in warnings {
        eprintln!("Warning: {warning}");
    }
}

/// Uninstall tr300 from shell profiles. Every profile is parsed and validated
/// before mutation; the group is staged and restored without clobbering a
/// concurrently-created path if any later member fails.
pub fn uninstall() -> Result<()> {
    let plan = prepare_profile_cleanup()?;
    let modified_files = plan.displayed_paths();
    let applied = apply_profile_cleanup(plan)?;
    report_profile_cleanup(&modified_files, applied.finalize());
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
#[cfg(test)]
fn remove_from_profile(path: &PathBuf) -> Result<bool> {
    let content = fs::read_to_string(path)
        .map_err(|e| AppError::platform(format!("Failed to read {}: {}", path.display(), e)))?;
    let Some(new_content) = cleaned_profile_contents(&content)? else {
        return Ok(false);
    };

    super::atomic_write(path, &new_content)
        .map_err(|e| AppError::platform(format!("Failed to write {}: {}", path.display(), e)))?;

    Ok(true)
}

/// Find the location of the currently running binary
pub fn find_binary_location() -> Option<PathBuf> {
    // A conventional pathname is not proof that it belongs to this running
    // process. If the running image is already unlinked, return absence rather
    // than selecting a potentially unrelated ~/.local or /usr/local copy.
    env::current_exe().ok().filter(|path| path.exists())
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

/// Opaque, read-only preview of the exact Unix paths a Complete uninstall has
/// proven safe to remove. The ownership plan remains private and is consumed
/// only by [`uninstall_complete_prepared`].
pub struct CompleteUninstallPreview {
    binary_path: Option<PathBuf>,
    receipt_path: Option<PathBuf>,
    cargo_plan: Option<crate::migrate::CurrentCargoUninstallPlan>,
    binary_plan: Option<crate::migrate::CurrentBinaryUninstallPlan>,
    profile_plan: ProfileCleanupPlan,
}

impl CompleteUninstallPreview {
    pub fn binary_path(&self) -> Option<&Path> {
        self.binary_path.as_deref()
    }

    pub fn receipt_path(&self) -> Option<&Path> {
        self.receipt_path.as_deref()
    }
}

/// Validate Complete-uninstall ownership and capture the exact confirmation
/// paths before any prompt or shell-profile mutation.
pub fn prepare_complete_uninstall() -> Result<CompleteUninstallPreview> {
    prepare_complete_uninstall_with(env::current_exe())
}

fn prepare_complete_uninstall_with(
    current_exe: std::io::Result<PathBuf>,
) -> Result<CompleteUninstallPreview> {
    // A Cargo-path Complete uninstall may also own a cargo-dist receipt. Prove
    // that ownership pair before touching either shell profile: malformed,
    // foreign, linked, or wrong-prefix receipt evidence must leave every user
    // file unchanged.
    let current_exe = current_exe.map_err(|error| {
        AppError::platform(format!(
            "Could not resolve the running executable before Complete uninstall: {error}. Preserving the binary, receipt, and shell profiles."
        ))
    })?;
    let cargo_plan = crate::migrate::preflight_current_cargo_uninstall(&current_exe)?;
    let binary_plan = if cargo_plan.is_none() {
        Some(crate::migrate::preflight_current_binary_uninstall(
            &current_exe,
        )?)
    } else {
        None
    };
    let binary_path = cargo_plan
        .as_ref()
        .and_then(crate::migrate::CurrentCargoUninstallPlan::binary_path)
        .or_else(|| {
            binary_plan
                .as_ref()
                .and_then(crate::migrate::CurrentBinaryUninstallPlan::binary_path)
        })
        .map(Path::to_path_buf);
    let receipt_path = cargo_plan
        .as_ref()
        .and_then(crate::migrate::CurrentCargoUninstallPlan::receipt_path)
        .map(Path::to_path_buf);
    let profile_plan = prepare_profile_cleanup()?;

    Ok(CompleteUninstallPreview {
        binary_path,
        receipt_path,
        cargo_plan,
        binary_plan,
        profile_plan,
    })
}

/// Perform Complete uninstall using the exact plans shown at confirmation.
/// Cargo and portable binary identity are revalidated both before profile
/// mutation and again inside their staged ownership commit.
pub fn uninstall_complete_prepared(preview: CompleteUninstallPreview) -> Result<()> {
    let CompleteUninstallPreview {
        cargo_plan,
        binary_plan,
        profile_plan,
        ..
    } = preview;

    // The interactive confirmation may remain open indefinitely. Recheck its
    // opaque ownership proof once more before touching profiles; commit repeats
    // the same check immediately before staging the ownership object(s).
    if let Some(plan) = cargo_plan.as_ref() {
        crate::migrate::revalidate_current_cargo_uninstall(plan)?;
    }
    if let Some(plan) = binary_plan.as_ref() {
        crate::migrate::revalidate_current_binary_uninstall(plan)?;
    }
    revalidate_profile_cleanup(&profile_plan)?;

    let modified_files = profile_plan.displayed_paths();
    let applied_profiles = apply_profile_cleanup(profile_plan)?;

    // Unix permits a running image to unlink itself. The dedicated commit path
    // consumes the opaque proof above and removes either an exact cargo-dist
    // binary/receipt pair or one exact portable binary, with no-clobber
    // rollback on reported errors.
    if let Err(error) = applied_profiles.revalidate() {
        let rollback_failures = applied_profiles.rollback();
        return if rollback_failures.is_empty() {
            Err(AppError::platform(format!(
                "A shell profile or its symlink changed before ownership cleanup: {error}. Every shell profile was restored."
            )))
        } else {
            Err(AppError::platform(format!(
                "A shell profile or its symlink changed before ownership cleanup: {error}. Shell-profile rollback needs attention: {}",
                rollback_failures.join("; ")
            )))
        };
    }
    let ownership_result = if let Some(plan) = cargo_plan {
        crate::migrate::commit_current_cargo_uninstall(plan)
    } else if let Some(plan) = binary_plan {
        crate::migrate::commit_current_binary_uninstall(plan)
    } else {
        unreachable!("Complete uninstall always prepares one Unix ownership plan")
    };

    match ownership_result {
        Ok(outcome) => {
            let profile_warnings = applied_profiles.finalize();
            report_profile_cleanup(&modified_files, profile_warnings);
            if let Some(path) = outcome.binary_path {
                println!("Removed binary: {}", path.display());
            }
            if let Some(path) = outcome.receipt_path {
                println!("Removed cargo-dist receipt: {}", path.display());
            }
            for warning in outcome.cleanup_warnings {
                eprintln!("Warning: {warning}");
            }
            Ok(())
        }
        Err(error) => {
            let rollback_failures = applied_profiles.rollback();
            if rollback_failures.is_empty() {
                Err(AppError::platform(format!(
                    "Ownership cleanup failed after shell profiles were staged: {error}. Every shell profile was restored."
                )))
            } else {
                Err(AppError::platform(format!(
                    "Ownership cleanup failed after shell profiles were staged: {error}. Shell-profile rollback needs attention: {}",
                    rollback_failures.join("; ")
                )))
            }
        }
    }
}

/// Backwards-compatible noninteractive Complete-uninstall entry point.
pub fn uninstall_complete() -> Result<()> {
    let preview = prepare_complete_uninstall()?;
    uninstall_complete_prepared(preview)
}

#[cfg(test)]
mod tests {
    use super::{
        apply_profile_cleanup_with, prepare_complete_uninstall_with,
        prepare_profile_cleanup_for_home, remove_from_profile, update_shell_profile, MARKER_END,
        MARKER_START, SHELL_ADDITIONS,
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
    fn complete_uninstall_stops_when_current_executable_cannot_be_resolved() {
        let error = match prepare_complete_uninstall_with(Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "test resolution failure",
        ))) {
            Ok(_) => panic!("current-executable failure was incorrectly accepted"),
            Err(error) => error.to_string(),
        };

        assert!(error.contains("Could not resolve the running executable"));
        assert!(error.contains("Preserving the binary, receipt, and shell profiles"));
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

    fn installed_profile(prefix: &str) -> String {
        format!("{prefix}\n{MARKER_START}\nalias report='tr300'\n{MARKER_END}\n")
    }

    #[test]
    fn malformed_later_profile_preflight_leaves_every_profile_unchanged() {
        let home = tempfile::tempdir().unwrap();
        let bashrc = home.path().join(".bashrc");
        let zshrc = home.path().join(".zshrc");
        let bash_before = installed_profile("export KEEP_BASH=yes");
        let zsh_before = format!("export KEEP_ZSH=yes\n{MARKER_START}\nmissing end marker\n");
        std::fs::write(&bashrc, &bash_before).unwrap();
        std::fs::write(&zshrc, &zsh_before).unwrap();

        assert!(prepare_profile_cleanup_for_home(home.path()).is_err());
        assert_eq!(std::fs::read_to_string(&bashrc).unwrap(), bash_before);
        assert_eq!(std::fs::read_to_string(&zshrc).unwrap(), zsh_before);
        assert!(home.path().read_dir().unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tr300-profile-uninstall-")
        }));
    }

    #[test]
    fn profile_appearing_after_preflight_blocks_all_mutation() {
        let home = tempfile::tempdir().unwrap();
        let bashrc = home.path().join(".bashrc");
        let zshrc = home.path().join(".zshrc");
        let bash_before = installed_profile("export KEEP_BASH=yes");
        let zsh_after = installed_profile("export NEW_ZSH=yes");
        std::fs::write(&bashrc, &bash_before).unwrap();
        let plan = prepare_profile_cleanup_for_home(home.path()).unwrap();

        std::fs::write(&zshrc, &zsh_after).unwrap();
        let error = apply_profile_cleanup_with(plan, |_, _| Ok(()), |_, _| Ok(()))
            .unwrap_err()
            .to_string();

        assert!(error.contains("appeared"));
        assert_eq!(std::fs::read_to_string(&bashrc).unwrap(), bash_before);
        assert_eq!(std::fs::read_to_string(&zshrc).unwrap(), zsh_after);
    }

    #[test]
    fn unchanged_profile_is_still_guarded_across_confirmation() {
        let home = tempfile::tempdir().unwrap();
        let bashrc = home.path().join(".bashrc");
        let zshrc = home.path().join(".zshrc");
        let bash_before = installed_profile("export KEEP_BASH=yes");
        std::fs::write(&bashrc, &bash_before).unwrap();
        std::fs::write(&zshrc, "export KEEP_ZSH=yes\n").unwrap();
        let plan = prepare_profile_cleanup_for_home(home.path()).unwrap();

        std::fs::write(&zshrc, "export KEEP_ZSH=changed\n").unwrap();
        let error = apply_profile_cleanup_with(plan, |_, _| Ok(()), |_, _| Ok(()))
            .unwrap_err()
            .to_string();

        assert!(error.contains("changed after preflight"));
        assert_eq!(std::fs::read_to_string(&bashrc).unwrap(), bash_before);
        assert_eq!(
            std::fs::read_to_string(&zshrc).unwrap(),
            "export KEEP_ZSH=changed\n"
        );
    }

    #[test]
    fn symlinked_profile_must_still_resolve_to_the_confirmed_target() {
        use std::os::unix::fs::symlink;

        let home = tempfile::tempdir().unwrap();
        let first_target = home.path().join("first-profile");
        let second_target = home.path().join("second-profile");
        let display = home.path().join(".bashrc");
        let first_before = installed_profile("export KEEP_FIRST=yes");
        let second_before = installed_profile("export KEEP_SECOND=yes");
        std::fs::write(&first_target, &first_before).unwrap();
        std::fs::write(&second_target, &second_before).unwrap();
        symlink(&first_target, &display).unwrap();
        let plan = prepare_profile_cleanup_for_home(home.path()).unwrap();

        std::fs::remove_file(&display).unwrap();
        symlink(&second_target, &display).unwrap();
        let error = apply_profile_cleanup_with(plan, |_, _| Ok(()), |_, _| Ok(()))
            .unwrap_err()
            .to_string();

        assert!(error.contains("confirmed symlink"));
        assert_eq!(
            std::fs::read_to_string(&first_target).unwrap(),
            first_before
        );
        assert_eq!(
            std::fs::read_to_string(&second_target).unwrap(),
            second_before
        );
    }

    #[test]
    fn two_profile_symlinks_to_one_target_are_bound_and_cleaned_once() {
        use std::os::unix::fs::symlink;

        let home = tempfile::tempdir().unwrap();
        let target = home.path().join("shared-profile");
        let before = installed_profile("export KEEP_SHARED=yes");
        std::fs::write(&target, &before).unwrap();
        symlink(&target, home.path().join(".bashrc")).unwrap();
        symlink(&target, home.path().join(".zshrc")).unwrap();
        let plan = prepare_profile_cleanup_for_home(home.path()).unwrap();
        assert_eq!(plan.displayed_paths().len(), 2);

        let applied = apply_profile_cleanup_with(plan, |_, _| Ok(()), |_, _| Ok(())).unwrap();
        assert!(applied.finalize().is_empty());

        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "export KEEP_SHARED=yes\n"
        );
        assert!(std::fs::symlink_metadata(home.path().join(".bashrc"))
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(std::fs::symlink_metadata(home.path().join(".zshrc"))
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn symlinked_home_and_profile_alias_share_one_canonical_target() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let real_home = root.path().join("real-home");
        let display_home = root.path().join("display-home");
        std::fs::create_dir(&real_home).unwrap();
        symlink(&real_home, &display_home).unwrap();

        let target = real_home.join(".bashrc");
        let before = installed_profile("export KEEP_SHARED=yes");
        std::fs::write(&target, &before).unwrap();
        symlink(".bashrc", real_home.join(".zshrc")).unwrap();

        let plan = prepare_profile_cleanup_for_home(&display_home).unwrap();
        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.displayed_paths().len(), 2);

        let applied = apply_profile_cleanup_with(plan, |_, _| Ok(()), |_, _| Ok(())).unwrap();
        assert!(applied.finalize().is_empty());
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "export KEEP_SHARED=yes\n"
        );
        assert!(std::fs::symlink_metadata(real_home.join(".zshrc"))
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn later_profile_install_failure_rolls_back_the_entire_group() {
        let home = tempfile::tempdir().unwrap();
        let bashrc = home.path().join(".bashrc");
        let zshrc = home.path().join(".zshrc");
        let bash_before = installed_profile("export KEEP_BASH=yes");
        let zsh_before = installed_profile("export KEEP_ZSH=yes");
        std::fs::write(&bashrc, &bash_before).unwrap();
        std::fs::write(&zshrc, &zsh_before).unwrap();
        let plan = prepare_profile_cleanup_for_home(home.path()).unwrap();

        let error = apply_profile_cleanup_with(
            plan,
            |index, _| {
                if index == 1 {
                    Err(std::io::Error::other("injected later-profile failure"))
                } else {
                    Ok(())
                }
            },
            |_, _| Ok(()),
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("injected later-profile failure"));
        assert!(error.contains("restored every shell profile"));
        assert_eq!(std::fs::read_to_string(&bashrc).unwrap(), bash_before);
        assert_eq!(std::fs::read_to_string(&zshrc).unwrap(), zsh_before);
        assert!(home.path().read_dir().unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tr300-profile-uninstall-")
        }));
    }

    #[test]
    fn post_persist_validation_failure_rolls_back_the_recorded_inode() {
        let home = tempfile::tempdir().unwrap();
        let bashrc = home.path().join(".bashrc");
        let bash_before = installed_profile("export KEEP_BASH=yes");
        std::fs::write(&bashrc, &bash_before).unwrap();
        let plan = prepare_profile_cleanup_for_home(home.path()).unwrap();

        let error = apply_profile_cleanup_with(
            plan,
            |_, _| Ok(()),
            |_, _| Err(std::io::Error::other("injected post-persist failure")),
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("injected post-persist failure"));
        assert!(error.contains("restored every shell profile"));
        assert_eq!(std::fs::read_to_string(&bashrc).unwrap(), bash_before);
        assert!(home.path().read_dir().unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tr300-profile-uninstall-")
        }));
    }

    #[test]
    fn deleted_cleaned_profile_is_restored_from_exact_staged_original() {
        let home = tempfile::tempdir().unwrap();
        let bashrc = home.path().join(".bashrc");
        let bash_before = installed_profile("export KEEP_BASH=yes");
        std::fs::write(&bashrc, &bash_before).unwrap();
        let plan = prepare_profile_cleanup_for_home(home.path()).unwrap();

        let error = apply_profile_cleanup_with(
            plan,
            |_, _| Ok(()),
            |_, target| {
                std::fs::remove_file(target)?;
                Err(std::io::Error::other(
                    "injected deletion after cleaned-profile persist",
                ))
            },
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("injected deletion after cleaned-profile persist"));
        assert!(error.contains("restored every shell profile"));
        assert_eq!(std::fs::read_to_string(&bashrc).unwrap(), bash_before);
        assert!(home.path().read_dir().unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tr300-profile-uninstall-")
        }));
    }

    #[test]
    fn finalize_retains_prior_profile_when_cleaned_target_disappears() {
        let home = tempfile::tempdir().unwrap();
        let bashrc = home.path().join(".bashrc");
        let bash_before = installed_profile("export KEEP_BASH=yes");
        std::fs::write(&bashrc, &bash_before).unwrap();
        let plan = prepare_profile_cleanup_for_home(home.path()).unwrap();
        let applied = apply_profile_cleanup_with(plan, |_, _| Ok(()), |_, _| Ok(())).unwrap();
        let staged_original = applied.entries[0].staged_original.clone();

        std::fs::remove_file(&bashrc).unwrap();
        let warnings = applied.finalize();

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("could not revalidate cleaned"));
        assert!(warnings[0].contains(&staged_original.display().to_string()));
        assert!(!bashrc.exists());
        assert_eq!(
            std::fs::read_to_string(&staged_original).unwrap(),
            bash_before
        );
    }

    #[test]
    fn finalize_never_discards_prior_profile_over_a_replacement() {
        let home = tempfile::tempdir().unwrap();
        let bashrc = home.path().join(".bashrc");
        let bash_before = installed_profile("export KEEP_BASH=yes");
        std::fs::write(&bashrc, &bash_before).unwrap();
        let plan = prepare_profile_cleanup_for_home(home.path()).unwrap();
        let applied = apply_profile_cleanup_with(plan, |_, _| Ok(()), |_, _| Ok(())).unwrap();
        let staged_original = applied.entries[0].staged_original.clone();

        std::fs::remove_file(&bashrc).unwrap();
        std::fs::write(&bashrc, b"concurrent replacement\n").unwrap();
        let warnings = applied.finalize();

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("could not revalidate cleaned"));
        assert_eq!(
            std::fs::read_to_string(&bashrc).unwrap(),
            "concurrent replacement\n"
        );
        assert_eq!(
            std::fs::read_to_string(&staged_original).unwrap(),
            bash_before
        );
    }

    #[test]
    fn profile_rollback_never_clobbers_a_concurrent_replacement() {
        let home = tempfile::tempdir().unwrap();
        let bashrc = home.path().join(".bashrc");
        let zshrc = home.path().join(".zshrc");
        let bash_before = installed_profile("export KEEP_BASH=yes");
        let zsh_before = installed_profile("export KEEP_ZSH=yes");
        std::fs::write(&bashrc, &bash_before).unwrap();
        std::fs::write(&zshrc, &zsh_before).unwrap();
        let plan = prepare_profile_cleanup_for_home(home.path()).unwrap();

        let error = apply_profile_cleanup_with(
            plan,
            |index, target| {
                if index == 1 {
                    std::fs::write(target, b"concurrent replacement\n")?;
                }
                Ok(())
            },
            |_, _| Ok(()),
        )
        .unwrap_err()
        .to_string();

        assert_eq!(
            std::fs::read_to_string(&zshrc).unwrap(),
            "concurrent replacement\n"
        );
        assert_eq!(std::fs::read_to_string(&bashrc).unwrap(), bash_before);
        assert!(error.contains("refusing to clobber"));
        assert!(error.contains("exact prior profile remains at"));
        assert!(home.path().read_dir().unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tr300-profile-uninstall-")
        }));
    }
}
