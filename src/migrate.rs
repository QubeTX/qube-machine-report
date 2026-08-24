// Cross-method install cleanup for TR-300 (`tr300 migrate-cleanup`).
//
// Mirrors ND-300's `nd300 migrate-cleanup` (same flags, same JSON contract, same
// safety guarantees) so the two sibling tools behave identically. TR-300 ships a
// SINGLE binary (`tr300`), is synchronous (ureq, no tokio), and keeps its
// install-origin detection in `update.rs` — so this module is the TR-300-shaped
// counterpart, not a byte-for-byte copy.
//
// PURPOSE
// -------
// On Windows a user can end up with more than one copy of tr300 on PATH:
//   * A prior `cargo install tr300` / cargo-dist PowerShell-installer copy in
//     `~\.cargo\bin` that SHADOWS a freshly-installed MSI/EXE copy (both on PATH;
//     the cargo copy usually wins because `.cargo\bin` is earlier).
//   * Two Windows editions coexisting: Global perMachine
//     (`C:\Program Files\tr300\bin`) and Corporate perUser
//     (`%LocalAppData%\Programs\tr300\bin`).
//
// Operator policy: exactly ONE version/edition installed at a time. The current
// native integration is Windows, where installers invoke this to consolidate a
// prior Cargo/cargo-dist copy and may remove the other Global/Corporate edition.
// Unix keeps the hidden action for legacy callers, while Complete uninstall has
// a separate running-image path below. Ordinary calls remain harmless no-ops
// when no Cargo-path copy is present.
//
// HARD SAFETY GUARANTEES (see unit tests):
//   1. Only ever deletes a file whose stem is in `OUR_BINARIES` (`tr300`). Never
//      cargo.exe / rustup.exe / any non-allowlisted file.
//   2. Never removes the `.cargo\bin` PATH entry — it never touches PATH at all;
//      it only deletes a single binary file.
//   3. Never touches `~/Downloads` (no path this module computes is under it).
//   4. Never deletes the RUNNING install — every candidate is `same_path`-checked
//      against the running exe's directory and skipped if it matches.
//   5. Never escalates privileges. If a target needs admin we don't have, it
//      reports "needs admin: <path>" and preserves the prior installation.
//   6. Deletes a cargo-dist receipt only when its provider/app/prefix exactly
//      identify the same Cargo-home copy selected above.
//
// EXIT CODE: legacy calls remain advisory (0 on partial/empty/needs-admin).
// Current native packages pass `--strict`; incomplete or ambiguous requested
// cleanup then exits 2 so the package cannot counterfeit successful convergence.

use crate::config::Config;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::fs::{File, OpenOptions};
#[cfg(unix)]
use std::io::Read;
#[cfg(any(windows, unix))]
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

// Reused install-origin detection lives in update.rs and is Windows-only there,
// so import it Windows-gated to avoid an unused-import warning on macOS/Linux.
#[cfg(windows)]
use crate::update::{detect_install_origin, InstallOrigin};

/// Options for `migrate-cleanup`, mirrored from the CLI flags. Plain value so the
/// resolution logic is unit-testable and the contract matches ND-300's exactly.
#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub struct MigrateOptions {
    pub cargo_copy: bool,
    pub other_edition: bool,
    pub quiet: bool,
    pub dry_run: bool,
    pub strict: bool,
    pub json: bool,
    pub user_profile: Option<String>,
    pub cargo_home: Option<String>,
}

/// The single binary TR-300 ships. (ND-300 ships two; TR-300 ships one — this is
/// the allowlist that bounds every deletion.)
const OUR_BINARIES: &[&str] = &["tr300"];

/// Outcome of a single cleanup target after deletion was attempted (or skipped).
/// Full variant set is the platform-agnostic contract; on macOS/Linux only
/// `Skipped` is constructed in non-test code, so allow dead_code there.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TargetOutcome {
    Removed,
    WouldRemove,
    Skipped(String),
    NeedsAdmin(String),
    Failed(String),
}

#[derive(Debug, Clone)]
pub(crate) struct TargetReport {
    pub(crate) id: &'static str,
    pub(crate) label: String,
    pub(crate) path: Option<PathBuf>,
    pub(crate) outcome: TargetOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CleanupTargets {
    pub(crate) cargo_copy: bool,
    pub(crate) other_edition: bool,
}

/// With NO target flag, default to `--cargo-copy` only (the safest,
/// never-needs-admin consolidation). Pure + unit-tested.
pub(crate) fn resolve_targets(cargo_copy: bool, other_edition: bool) -> CleanupTargets {
    if !cargo_copy && !other_edition {
        CleanupTargets {
            cargo_copy: true,
            other_edition: false,
        }
    } else {
        CleanupTargets {
            cargo_copy,
            other_edition,
        }
    }
}

/// Whether an io error kind is a permission problem (-> NeedsAdmin). Pure +
/// testable; the real caller is Windows-only so allow dead_code off-Windows.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn is_permission_error(kind: std::io::ErrorKind) -> bool {
    matches!(kind, std::io::ErrorKind::PermissionDenied)
}

/// True if `exe`'s file name is one of OUR_BINARIES (with or without `.exe`).
/// Case-insensitive; cross-platform (pure) so it's unit-testable everywhere.
/// Real caller is Windows-only; allow dead_code off-Windows.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn is_allowlisted(exe: &Path) -> bool {
    let name = exe
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    OUR_BINARIES
        .iter()
        .any(|b| name == format!("{}.exe", b) || name == *b)
}

// ── Public entry point ─────────────────────────────────────────────

/// Run the consolidation. Returns 0 for success/advisory and 2 for a true
/// internal error or strict incomplete convergence. Synchronous to match
/// TR-300's `update::run`.
pub fn run(config: &Config, opts: &MigrateOptions) -> i32 {
    let targets = resolve_targets(opts.cargo_copy, opts.other_edition);
    let json = opts.json || matches!(config.format, crate::config::OutputFormat::Json);

    let reports = collect_and_execute(opts, targets);

    let internal_error = reports
        .iter()
        .any(|r| matches!(&r.outcome, TargetOutcome::Failed(m) if m == INTERNAL_ERROR_MARKER));
    let strict_failure = opts.strict && reports.iter().any(strict_report_failed);
    let success = !internal_error && !strict_failure;

    if json {
        print_json(&reports, &targets, opts.dry_run, success);
    } else if !opts.quiet {
        print_human(&reports, config, opts.dry_run);
    }

    if success {
        0
    } else {
        2
    }
}

fn strict_report_failed(report: &TargetReport) -> bool {
    match &report.outcome {
        TargetOutcome::Removed | TargetOutcome::WouldRemove => false,
        TargetOutcome::NeedsAdmin(_) | TargetOutcome::Failed(_) => true,
        TargetOutcome::Skipped(reason) => {
            !(reason.starts_with("no ") || reason.starts_with("not applicable"))
        }
    }
}

const INTERNAL_ERROR_MARKER: &str = "__internal_error__";

// ── Shared (cross-platform) path helpers ───────────────────────────

/// Canonicalized path of the running executable (best-effort).
#[cfg(any(windows, unix))]
fn current_exe_real_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.canonicalize().unwrap_or(exe))
}

/// The user's cargo-bin dir, preferring installer-supplied overrides
/// (`--cargo-home`, then `--user-profile`) over the process env so a perMachine
/// installer running as a different user can still resolve the invoking user's
/// `.cargo`. Falls back to `CARGO_HOME`/`%USERPROFILE%`/`$HOME`.
#[cfg(any(windows, unix))]
fn resolve_cargo_home(opts: &MigrateOptions) -> Option<PathBuf> {
    if let Some(home) = &opts.cargo_home {
        return Some(PathBuf::from(home));
    }
    if let Some(profile) = &opts.user_profile {
        return Some(PathBuf::from(profile).join(".cargo"));
    }
    if let Some(cargo_home) = std::env::var_os("CARGO_HOME") {
        return Some(PathBuf::from(cargo_home));
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".cargo"))
}

#[cfg(any(windows, unix))]
fn resolve_cargo_bin_dir(opts: &MigrateOptions) -> Option<PathBuf> {
    resolve_cargo_home(opts).map(|home| home.join("bin"))
}

/// Windows path equality after best-effort canonicalization. Windows paths are
/// compared case-insensitively for the drive-letter and ordinary
/// case-insensitive-volume contract used by the supported installers.
#[cfg(windows)]
fn same_path(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    let left = left.to_string_lossy();
    let right = right.to_string_lossy();
    let left = left.trim_end_matches(['\\', '/']);
    let right = right.trim_end_matches(['\\', '/']);
    left.eq_ignore_ascii_case(right)
}

/// Unix paths are arbitrary byte strings. Never authorize deletion through a
/// lossy UTF-8 rendering: distinct invalid-byte and literal U+FFFD paths must
/// remain distinct. Successful canonicalization also removes redundant
/// separators, so direct `Path` equality is the exact platform contract.
#[cfg(unix)]
fn same_path(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    left == right
}

// ── Detection + execution ──────────────────────────────────────────

#[cfg(any(windows, unix))]
fn collect_and_execute(opts: &MigrateOptions, targets: CleanupTargets) -> Vec<TargetReport> {
    let mut reports = Vec::new();

    let Some(running) = current_exe_real_path() else {
        reports.push(TargetReport {
            id: "internal",
            label: "determine running install location".to_string(),
            path: None,
            outcome: TargetOutcome::Failed(INTERNAL_ERROR_MARKER.to_string()),
        });
        return reports;
    };
    let running_dir = running.parent().map(|p| p.to_path_buf());

    if targets.cargo_copy {
        if opts.strict {
            reports.extend(execute_strict_cargo_pair(opts, running_dir.as_deref()));
        } else {
            let binary = execute_cargo_copy(opts, running_dir.as_deref());
            let may_remove_receipt = matches!(&binary.outcome, TargetOutcome::Removed)
                || (matches!(&binary.outcome, TargetOutcome::WouldRemove) && opts.dry_run)
                || matches!(&binary.outcome, TargetOutcome::Skipped(reason) if reason == "no cargo copy present");
            reports.push(binary);
            reports.push(execute_cargo_dist_receipt(opts, may_remove_receipt));
        }
    }
    #[cfg(windows)]
    if targets.other_edition {
        reports.push(execute_other_edition(opts, running_dir.as_deref()));
    }
    #[cfg(not(windows))]
    if targets.other_edition {
        reports.push(TargetReport {
            id: "other_edition",
            label: "other edition".to_string(),
            path: None,
            outcome: TargetOutcome::Skipped(
                "not applicable on this platform (no Global/Corporate editions)".to_string(),
            ),
        });
    }
    reports
}

#[cfg(not(any(windows, unix)))]
fn collect_and_execute(_opts: &MigrateOptions, targets: CleanupTargets) -> Vec<TargetReport> {
    // Mac/Linux are already safe — the shell installer overwrites the same
    // ~/.cargo/bin, so there's no second copy to consolidate. Clean no-op.
    let mut reports = Vec::new();
    if targets.cargo_copy {
        reports.push(TargetReport {
            id: "cargo_copy",
            label: "older cargo copy".to_string(),
            path: None,
            outcome: TargetOutcome::Skipped(
                "not applicable on this platform (single install location)".to_string(),
            ),
        });
    }
    if targets.other_edition {
        reports.push(TargetReport {
            id: "other_edition",
            label: "other edition".to_string(),
            path: None,
            outcome: TargetOutcome::Skipped(
                "not applicable on this platform (no Global/Corporate editions)".to_string(),
            ),
        });
    }
    reports
}

#[cfg(any(windows, unix))]
fn execute_cargo_copy(opts: &MigrateOptions, running_dir: Option<&Path>) -> TargetReport {
    let id = "cargo_copy";
    let label = "older cargo copy".to_string();

    let Some(cargo_bin) = resolve_cargo_bin_dir(opts) else {
        return TargetReport {
            id,
            label,
            path: None,
            outcome: TargetOutcome::Skipped("could not locate a .cargo\\bin directory".to_string()),
        };
    };

    // Guard 4: if the running install IS the cargo copy, never remove it.
    if let Some(rd) = running_dir {
        if same_path(rd, &cargo_bin) {
            return TargetReport {
                id,
                label,
                path: None,
                outcome: TargetOutcome::Skipped(
                    "the running install is the cargo copy — preserving it".to_string(),
                ),
            };
        }
    }

    let cargo_exe = cargo_bin.join(if cfg!(windows) { "tr300.exe" } else { "tr300" });
    if !cargo_exe.exists() {
        return TargetReport {
            id,
            label,
            path: None,
            outcome: TargetOutcome::Skipped("no cargo copy present".to_string()),
        };
    }

    delete_target(id, label, &cargo_exe, opts.dry_run)
}

#[cfg(any(windows, unix))]
fn cargo_dist_receipt_path(opts: &MigrateOptions) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let root = if let Some(profile) = &opts.user_profile {
            PathBuf::from(profile).join("AppData").join("Local")
        } else if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            PathBuf::from(xdg)
        } else {
            PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
        };
        Some(root.join("tr300").join("tr300-receipt.json"))
    }

    #[cfg(unix)]
    {
        let root = if let Some(profile) = &opts.user_profile {
            PathBuf::from(profile).join(".config")
        } else if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            PathBuf::from(xdg)
        } else {
            let home = std::env::var_os("HOME")?;
            PathBuf::from(home).join(".config")
        };
        Some(root.join("tr300").join("tr300-receipt.json"))
    }
}

fn receipt_matches_cargo_home(contents: &str, cargo_home: &Path) -> bool {
    let Ok(receipt) = serde_json::from_str::<serde_json::Value>(contents) else {
        return false;
    };
    let source_matches = receipt
        .pointer("/provider/source")
        .and_then(serde_json::Value::as_str)
        == Some("cargo-dist");
    let app_matches = receipt
        .pointer("/source/app_name")
        .and_then(serde_json::Value::as_str)
        == Some("tr300");
    let Some(prefix) = receipt
        .get("install_prefix")
        .and_then(serde_json::Value::as_str)
    else {
        return false;
    };
    source_matches && app_matches && same_path(Path::new(prefix), cargo_home)
}

// A Complete uninstall launched by the Cargo-path binary is the one legitimate
// exception to migrate-cleanup's "never delete the running image" rule. Keep
// that exception out of the installer-facing MigrateOptions surface: the Unix
// uninstall path must first build this opaque plan while the user's profiles
// are still untouched, then consume it through the dedicated commit function.
#[cfg(unix)]
const MAX_SELF_UNINSTALL_RECEIPT_BYTES: u64 = 64 * 1024;

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UnixFileIdentity {
    device: u64,
    inode: u64,
    len: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
impl UnixFileIdentity {
    fn from_metadata(metadata: &std::fs::Metadata) -> Self {
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

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct UnixBinarySnapshot {
    path: PathBuf,
    identity: UnixFileIdentity,
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct UnixReceiptSnapshot {
    path: PathBuf,
    identity: UnixFileIdentity,
    contents: Vec<u8>,
    install_prefix: PathBuf,
    mode: u32,
}

/// Opaque proof that the current Unix process is the exact Cargo-path TR-300
/// candidate and that any cargo-dist receipt is safe to treat as its ownership
/// record. Callers cannot construct or weaken this plan.
#[cfg(unix)]
#[derive(Debug)]
pub(crate) struct CurrentCargoUninstallPlan {
    cargo_home: PathBuf,
    binary_path: PathBuf,
    receipt_path: PathBuf,
    binary: Option<UnixBinarySnapshot>,
    receipt: Option<UnixReceiptSnapshot>,
}

#[cfg(unix)]
impl CurrentCargoUninstallPlan {
    pub(crate) fn binary_path(&self) -> Option<&Path> {
        self.binary.as_ref().map(|binary| binary.path.as_path())
    }

    pub(crate) fn receipt_path(&self) -> Option<&Path> {
        self.receipt.as_ref().map(|receipt| receipt.path.as_path())
    }
}

/// Opaque proof for a running Unix binary that is not governed by the Cargo
/// receipt contract. The snapshot is carried across interactive confirmation
/// and prevents a replacement at the same pathname from being removed.
#[cfg(unix)]
#[derive(Debug)]
pub(crate) struct CurrentBinaryUninstallPlan {
    binary_path: PathBuf,
    binary: Option<UnixBinarySnapshot>,
}

#[cfg(unix)]
impl CurrentBinaryUninstallPlan {
    pub(crate) fn binary_path(&self) -> Option<&Path> {
        self.binary.as_ref().map(|binary| binary.path.as_path())
    }
}

#[cfg(unix)]
#[derive(Debug)]
pub(crate) struct CurrentCargoUninstallOutcome {
    pub(crate) binary_path: Option<PathBuf>,
    pub(crate) receipt_path: Option<PathBuf>,
    pub(crate) cleanup_warnings: Vec<String>,
}

#[cfg(unix)]
fn self_uninstall_error(message: impl Into<String>) -> crate::error::AppError {
    crate::error::AppError::platform(format!("Complete uninstall stopped: {}", message.into()))
}

#[cfg(unix)]
fn open_unix_validation_file(path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .read(true)
        // `O_NOFOLLOW` closes the final-component symlink race. `O_NONBLOCK`
        // is equally load-bearing: a regular file can be swapped for a FIFO
        // after lstat, and opening that FIFO without a writer must not hang the
        // Complete-uninstall transaction before fstat rejects it.
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
}

#[cfg(unix)]
fn validate_owned_regular_file(
    path: &Path,
    label: &str,
) -> crate::error::Result<(File, UnixFileIdentity, u32)> {
    let path_metadata = std::fs::symlink_metadata(path).map_err(|error| {
        self_uninstall_error(format!(
            "could not inspect {label} {}: {error}",
            path.display()
        ))
    })?;
    if !path_metadata.file_type().is_file() {
        return Err(self_uninstall_error(format!(
            "{label} {} is not a regular file; preserving the binary, receipt, and shell profiles",
            path.display()
        )));
    }
    if path_metadata.nlink() != 1 {
        return Err(self_uninstall_error(format!(
            "{label} {} has {} hard links; preserving the binary, receipt, and shell profiles",
            path.display(),
            path_metadata.nlink()
        )));
    }
    let expected_uid = unsafe { libc::geteuid() };
    if path_metadata.uid() != expected_uid {
        return Err(self_uninstall_error(format!(
            "{label} {} is owned by uid {}, not the current uid {}; preserving the binary, receipt, and shell profiles",
            path.display(),
            path_metadata.uid(),
            expected_uid
        )));
    }

    let file = open_unix_validation_file(path).map_err(|error| {
        self_uninstall_error(format!(
            "could not safely open {label} {}: {error}",
            path.display()
        ))
    })?;
    let handle_metadata = file.metadata().map_err(|error| {
        self_uninstall_error(format!(
            "could not inspect opened {label} {}: {error}",
            path.display()
        ))
    })?;
    if !handle_metadata.file_type().is_file()
        || handle_metadata.nlink() != 1
        || handle_metadata.uid() != expected_uid
        || UnixFileIdentity::from_metadata(&handle_metadata)
            != UnixFileIdentity::from_metadata(&path_metadata)
    {
        return Err(self_uninstall_error(format!(
            "{label} {} changed while it was being validated; preserving the binary, receipt, and shell profiles",
            path.display()
        )));
    }

    let identity = UnixFileIdentity::from_metadata(&handle_metadata);
    Ok((file, identity, handle_metadata.mode()))
}

#[cfg(unix)]
fn snapshot_binary(path: &Path) -> crate::error::Result<Option<UnixBinarySnapshot>> {
    snapshot_binary_with_label(path, "Cargo-path binary")
}

#[cfg(unix)]
fn snapshot_binary_with_label(
    path: &Path,
    label: &str,
) -> crate::error::Result<Option<UnixBinarySnapshot>> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(self_uninstall_error(format!(
            "could not inspect {label} {}: {error}",
            path.display()
        ))),
        Ok(_) => {
            let (_file, identity, _mode) = validate_owned_regular_file(path, label)?;
            Ok(Some(UnixBinarySnapshot {
                path: path.to_path_buf(),
                identity,
            }))
        }
    }
}

#[cfg(unix)]
fn snapshot_receipt(path: &Path) -> crate::error::Result<Option<UnixReceiptSnapshot>> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(self_uninstall_error(format!(
                "could not inspect cargo-dist receipt {}: {error}",
                path.display()
            )))
        }
        Ok(_) => {}
    }

    let (file, identity, mode) = validate_owned_regular_file(path, "cargo-dist receipt")?;
    if identity.len > MAX_SELF_UNINSTALL_RECEIPT_BYTES {
        return Err(self_uninstall_error(format!(
            "cargo-dist receipt {} exceeds the {}-byte safety limit; preserving the binary, receipt, and shell profiles",
            path.display(),
            MAX_SELF_UNINSTALL_RECEIPT_BYTES
        )));
    }
    let mut contents = Vec::with_capacity(identity.len as usize);
    file.take(MAX_SELF_UNINSTALL_RECEIPT_BYTES + 1)
        .read_to_end(&mut contents)
        .map_err(|error| {
            self_uninstall_error(format!(
                "could not read cargo-dist receipt {}: {error}",
                path.display()
            ))
        })?;
    if contents.len() as u64 > MAX_SELF_UNINSTALL_RECEIPT_BYTES {
        return Err(self_uninstall_error(format!(
            "cargo-dist receipt {} grew beyond the {}-byte safety limit; preserving the binary, receipt, and shell profiles",
            path.display(),
            MAX_SELF_UNINSTALL_RECEIPT_BYTES
        )));
    }
    let text = std::str::from_utf8(&contents).map_err(|error| {
        self_uninstall_error(format!(
            "cargo-dist receipt {} is not UTF-8 ({error}); preserving the binary, receipt, and shell profiles",
            path.display()
        ))
    })?;
    let receipt = serde_json::from_str::<serde_json::Value>(text).map_err(|error| {
        self_uninstall_error(format!(
            "cargo-dist receipt {} is malformed ({error}); preserving the binary, receipt, and shell profiles",
            path.display()
        ))
    })?;
    let source_matches = receipt
        .pointer("/provider/source")
        .and_then(serde_json::Value::as_str)
        == Some("cargo-dist");
    let app_matches = receipt
        .pointer("/source/app_name")
        .and_then(serde_json::Value::as_str)
        == Some("tr300");
    let install_prefix = receipt
        .get("install_prefix")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .filter(|prefix| prefix.is_absolute());
    let Some(install_prefix) = install_prefix else {
        return Err(self_uninstall_error(format!(
            "cargo-dist receipt {} does not exactly identify TR-300 and an absolute install prefix; preserving the binary, receipt, and shell profiles",
            path.display()
        )));
    };
    if !source_matches || !app_matches {
        return Err(self_uninstall_error(format!(
            "cargo-dist receipt {} does not exactly identify TR-300 and an absolute install prefix; preserving the binary, receipt, and shell profiles",
            path.display()
        )));
    }

    Ok(Some(UnixReceiptSnapshot {
        path: path.to_path_buf(),
        identity,
        contents,
        install_prefix,
        mode,
    }))
}

#[cfg(unix)]
fn path_looks_like_managed_binary(path: &Path) -> bool {
    matches!(
        path.file_name(),
        Some(name)
            if name == std::ffi::OsStr::new("tr300")
                || name == std::ffi::OsStr::new("tr300 (deleted)")
    ) && path.parent().and_then(Path::file_name) == Some(std::ffi::OsStr::new("bin"))
}

#[cfg(unix)]
fn path_looks_like_default_cargo_binary(path: &Path) -> bool {
    path_looks_like_managed_binary(path)
        && path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            == Some(std::ffi::OsStr::new(".cargo"))
}

#[cfg(unix)]
fn current_exe_matches_cargo_binary(current_exe: &Path, binary_path: &Path) -> bool {
    if same_path(current_exe, binary_path) {
        return true;
    }

    #[cfg(target_os = "linux")]
    {
        // Linux exposes an already-unlinked running image through /proc/self/exe
        // with this exact suffix. Accept it only when the unsuffixed expected
        // Cargo path and the suffixed presentation are both absent; a real file
        // literally named `tr300 (deleted)` must never authorize deletion of the
        // receipt for another path.
        if std::fs::symlink_metadata(binary_path).is_ok()
            || !matches!(
                std::fs::symlink_metadata(current_exe),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound
            )
        {
            return false;
        }
        let mut deleted_path = binary_path.as_os_str().to_os_string();
        deleted_path.push(" (deleted)");
        current_exe.as_os_str() == deleted_path
    }

    #[cfg(not(target_os = "linux"))]
    false
}

#[cfg(unix)]
fn preflight_current_cargo_uninstall_at(
    current_exe: &Path,
    opts: &MigrateOptions,
    receipt_path: &Path,
) -> crate::error::Result<Option<CurrentCargoUninstallPlan>> {
    let receipt = snapshot_receipt(receipt_path)?;
    if let Some(receipt) = receipt {
        let cargo_home = receipt.install_prefix.clone();
        let binary_path = cargo_home.join("bin").join("tr300");
        if !current_exe_matches_cargo_binary(current_exe, &binary_path) {
            if path_looks_like_managed_binary(current_exe) {
                return Err(self_uninstall_error(format!(
                    "cargo-dist receipt {} belongs to {}, not the running binary {}; preserving the binary, receipt, and shell profiles",
                    receipt_path.display(),
                    binary_path.display(),
                    current_exe.display()
                )));
            }
            return Ok(None);
        }
        let binary = snapshot_binary(&binary_path)?;
        return Ok(Some(CurrentCargoUninstallPlan {
            cargo_home,
            binary_path,
            receipt_path: receipt_path.to_path_buf(),
            binary,
            receipt: Some(receipt),
        }));
    }

    let Some(cargo_home) = resolve_cargo_home(opts) else {
        if path_looks_like_default_cargo_binary(current_exe) {
            return Err(self_uninstall_error(
                "the running binary appears to be in Cargo home, but Cargo home could not be resolved; preserving the binary, receipt, and shell profiles",
            ));
        }
        return Ok(None);
    };
    let binary_path = cargo_home.join("bin").join("tr300");
    if !current_exe_matches_cargo_binary(current_exe, &binary_path) {
        return Ok(None);
    }
    let binary = snapshot_binary(&binary_path)?;

    Ok(Some(CurrentCargoUninstallPlan {
        cargo_home,
        binary_path,
        receipt_path: receipt_path.to_path_buf(),
        binary,
        receipt: None,
    }))
}

/// Validate a running Unix Cargo/cargo-dist install before Complete uninstall
/// mutates a shell profile. `Ok(None)` means the current executable is not the
/// Cargo-path copy and the ordinary single-binary uninstall path should run.
#[cfg(unix)]
pub(crate) fn preflight_current_cargo_uninstall(
    current_exe: &Path,
) -> crate::error::Result<Option<CurrentCargoUninstallPlan>> {
    let opts = MigrateOptions::default();
    let receipt_path = cargo_dist_receipt_path(&opts);
    preflight_current_cargo_uninstall_with_receipt_location(
        current_exe,
        &opts,
        receipt_path.as_deref(),
    )
}

#[cfg(unix)]
fn preflight_current_cargo_uninstall_with_receipt_location(
    current_exe: &Path,
    opts: &MigrateOptions,
    receipt_path: Option<&Path>,
) -> crate::error::Result<Option<CurrentCargoUninstallPlan>> {
    let Some(receipt_path) = receipt_path else {
        if path_looks_like_default_cargo_binary(current_exe) {
            return Err(self_uninstall_error(
                "the running binary may be managed, but the cargo-dist receipt location could not be resolved; preserving the binary and shell profiles",
            ));
        }
        return Ok(None);
    };
    preflight_current_cargo_uninstall_at(current_exe, opts, receipt_path)
}

/// Bind a portable or otherwise non-Cargo Unix Complete uninstall to the exact
/// current-executable pathname and inode observed before confirmation. A
/// missing running image remains a no-binary plan; it must never fall back to
/// a conventional install path that could belong to another installation.
#[cfg(unix)]
pub(crate) fn preflight_current_binary_uninstall(
    current_exe: &Path,
) -> crate::error::Result<CurrentBinaryUninstallPlan> {
    Ok(CurrentBinaryUninstallPlan {
        binary_path: current_exe.to_path_buf(),
        binary: snapshot_binary_with_label(current_exe, "running binary")?,
    })
}

#[cfg(unix)]
#[derive(Debug)]
struct StagedUnixUninstallFile {
    directory: PathBuf,
    path: PathBuf,
    original_path: PathBuf,
}

#[cfg(unix)]
impl StagedUnixUninstallFile {
    fn try_remove(self) -> std::result::Result<Option<String>, (std::io::Error, Self)> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => match std::fs::remove_dir(&self.directory) {
                Ok(()) => Ok(None),
                Err(error) => Ok(Some(format!(
                    "removed {}, but could not remove private staging directory {}: {error}",
                    self.original_path.display(),
                    self.directory.display()
                ))),
            },
            Err(error) => Err((error, self)),
        }
    }

    fn restore(self) -> std::result::Result<(), String> {
        if let Err(error) = std::fs::hard_link(&self.path, &self.original_path) {
            return Err(format!(
                "could not restore {} ({error}); the prior inode and contents remain preserved at {}",
                self.original_path.display(),
                self.path.display()
            ));
        }
        if let Err(error) = std::fs::remove_file(&self.path) {
            return Err(format!(
                "restored {}, but could not remove its staged link ({error}); the extra link remains at {} in preserved directory {}",
                self.original_path.display(),
                self.path.display(),
                self.directory.display()
            ));
        }
        std::fs::remove_dir(&self.directory).map_err(|error| {
            format!(
                "restored {}, but could not remove private staging directory {}: {error}",
                self.original_path.display(),
                self.directory.display()
            )
        })
    }
}

#[cfg(unix)]
fn stage_unix_path(path: &Path, label: &str) -> crate::error::Result<StagedUnixUninstallFile> {
    let parent = path.parent().ok_or_else(|| {
        self_uninstall_error(format!("{label} path has no parent: {}", path.display()))
    })?;
    let directory = tempfile::Builder::new()
        .prefix(".tr300-uninstall-")
        .tempdir_in(parent)
        .map_err(|error| {
            self_uninstall_error(format!(
                "could not create private {label} staging beside {}: {error}",
                path.display()
            ))
        })?
        .keep();
    let staged_path = directory.join(
        path.file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("tr300")),
    );
    if let Err(error) = std::fs::rename(path, &staged_path) {
        let cleanup = std::fs::remove_dir(&directory)
            .err()
            .map(|cleanup_error| {
                format!(
                    "; private staging directory {} was preserved because cleanup failed: {cleanup_error}",
                    directory.display()
                )
            })
            .unwrap_or_default();
        return Err(self_uninstall_error(format!(
            "could not stage {label} {}: {error}{cleanup}",
            path.display()
        )));
    }
    Ok(StagedUnixUninstallFile {
        directory,
        path: staged_path,
        original_path: path.to_path_buf(),
    })
}

#[cfg(unix)]
fn stage_binary_snapshot(
    expected: &UnixBinarySnapshot,
    label: &str,
) -> crate::error::Result<StagedUnixUninstallFile> {
    let refreshed = snapshot_binary_with_label(&expected.path, label)?.ok_or_else(|| {
        self_uninstall_error(format!(
            "{label} {} disappeared after preflight",
            expected.path.display()
        ))
    })?;
    if refreshed != *expected {
        return Err(self_uninstall_error(format!(
            "{label} {} changed after preflight; preserving ownership state",
            expected.path.display()
        )));
    }

    let staged = stage_unix_path(&expected.path, label)?;
    let staged_snapshot = snapshot_binary_with_label(&staged.path, label);
    let valid = matches!(staged_snapshot, Ok(Some(ref snapshot)) if snapshot.identity.same_staged_object(expected.identity));
    if valid {
        return Ok(staged);
    }
    let validation = match staged_snapshot {
        Ok(_) => "staged binary identity does not match preflight".to_string(),
        Err(error) => error.to_string(),
    };
    let restore = staged.restore();
    Err(self_uninstall_error(format!(
        "{validation}; {}",
        restore
            .err()
            .unwrap_or_else(|| "restored the original binary".to_string())
    )))
}

#[cfg(unix)]
fn stage_receipt_snapshot(
    expected: &UnixReceiptSnapshot,
) -> crate::error::Result<StagedUnixUninstallFile> {
    let refreshed = snapshot_receipt(&expected.path)?.ok_or_else(|| {
        self_uninstall_error(format!(
            "cargo-dist receipt {} disappeared after preflight",
            expected.path.display()
        ))
    })?;
    if refreshed != *expected {
        return Err(self_uninstall_error(format!(
            "cargo-dist receipt {} changed after preflight; preserving ownership state",
            expected.path.display()
        )));
    }

    let staged = stage_unix_path(&expected.path, "cargo-dist receipt")?;
    let staged_snapshot = snapshot_receipt(&staged.path);
    let valid = matches!(
        staged_snapshot,
        Ok(Some(ref snapshot))
            if snapshot.identity.same_staged_object(expected.identity)
                && snapshot.contents == expected.contents
                && snapshot.install_prefix == expected.install_prefix
                && snapshot.mode == expected.mode
    );
    if valid {
        return Ok(staged);
    }
    let validation = match staged_snapshot {
        Ok(_) => "staged receipt identity or contents do not match preflight".to_string(),
        Err(error) => error.to_string(),
    };
    let restore = staged.restore();
    Err(self_uninstall_error(format!(
        "{validation}; {}",
        restore
            .err()
            .unwrap_or_else(|| "restored the original receipt".to_string())
    )))
}

#[cfg(unix)]
fn ensure_absent(path: &Path, label: &str) -> crate::error::Result<()> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(self_uninstall_error(format!(
            "a new path appeared at {label} {}; refusing to clobber it",
            path.display()
        ))),
        Err(error) => Err(self_uninstall_error(format!(
            "could not recheck {label} {}: {error}",
            path.display()
        ))),
    }
}

#[cfg(unix)]
fn restore_staged_pair(
    binary: Option<StagedUnixUninstallFile>,
    receipt: Option<StagedUnixUninstallFile>,
) -> String {
    let mut details = Vec::new();
    if let Some(receipt) = receipt {
        details.push(
            receipt
                .restore()
                .err()
                .unwrap_or_else(|| "restored the cargo-dist receipt".to_string()),
        );
    }
    if let Some(binary) = binary {
        details.push(
            binary
                .restore()
                .err()
                .unwrap_or_else(|| "restored the Cargo-path binary".to_string()),
        );
    }
    details.join("; ")
}

/// Consume a successful preflight and remove the current Unix Cargo binary and
/// its exact cargo-dist receipt. Each present member moves into an exclusive
/// same-directory staging directory and is identity-checked after rename
/// before either staged inode is unlinked.
#[cfg(unix)]
pub(crate) fn commit_current_cargo_uninstall(
    plan: CurrentCargoUninstallPlan,
) -> crate::error::Result<CurrentCargoUninstallOutcome> {
    revalidate_current_cargo_uninstall(&plan)?;

    let binary_stage = match plan.binary.as_ref() {
        Some(binary) => Some(stage_binary_snapshot(binary, "Cargo-path binary")?),
        None => None,
    };
    let receipt_stage = match plan.receipt.as_ref() {
        Some(receipt) => match stage_receipt_snapshot(receipt) {
            Ok(staged) => Some(staged),
            Err(error) => {
                let rollback = restore_staged_pair(binary_stage, None);
                return Err(crate::error::AppError::platform(format!(
                    "{error}; {rollback}"
                )));
            }
        },
        None => None,
    };

    if let Err(error) = ensure_absent(&plan.binary_path, "Cargo-path binary")
        .and_then(|()| ensure_absent(&plan.receipt_path, "cargo-dist receipt"))
    {
        let rollback = restore_staged_pair(binary_stage, receipt_stage);
        return Err(crate::error::AppError::platform(format!(
            "{error}; {rollback}"
        )));
    }

    let binary_removed = plan.binary.as_ref().map(|binary| binary.path.clone());
    let receipt_removed = plan.receipt.as_ref().map(|receipt| receipt.path.clone());

    match (binary_stage, receipt_stage) {
        (Some(binary), Some(receipt)) => match receipt.try_remove() {
            Err((error, receipt)) => {
                let rollback = restore_staged_pair(Some(binary), Some(receipt));
                Err(crate::error::AppError::platform(format!(
                    "could not remove staged cargo-dist receipt: {error}; {rollback}"
                )))
            }
            Ok(receipt_warning) => match binary.try_remove() {
                Ok(binary_warning) => Ok(CurrentCargoUninstallOutcome {
                    binary_path: binary_removed,
                    receipt_path: receipt_removed,
                    cleanup_warnings: receipt_warning.into_iter().chain(binary_warning).collect(),
                }),
                Err((error, binary)) => {
                    let binary_restore = binary
                        .restore()
                        .err()
                        .unwrap_or_else(|| "restored the Cargo-path binary".to_string());
                    let receipt_snapshot = plan.receipt.as_ref().expect("matched above");
                    let permissions = std::fs::Permissions::from_mode(receipt_snapshot.mode);
                    let receipt_restore = restore_receipt_noclobber(
                        &receipt_snapshot.path,
                        &receipt_snapshot.contents,
                        Some(&permissions),
                    )
                    .map(|()| {
                        "restored receipt bytes and permissions; timestamps and extended metadata are not reconstructed"
                            .to_string()
                    })
                    .unwrap_or_else(|restore_error| {
                        format!("receipt restoration failed: {restore_error}")
                    });
                    let cleanup = receipt_warning
                        .map(|warning| format!("; {warning}"))
                        .unwrap_or_default();
                    Err(crate::error::AppError::platform(format!(
                        "could not remove staged Cargo-path binary: {error}; {binary_restore}; {receipt_restore}{cleanup}"
                    )))
                }
            },
        },
        (Some(binary), None) => match binary.try_remove() {
            Ok(cleanup_warning) => Ok(CurrentCargoUninstallOutcome {
                binary_path: binary_removed,
                receipt_path: None,
                cleanup_warnings: cleanup_warning.into_iter().collect(),
            }),
            Err((error, binary)) => {
                let rollback = restore_staged_pair(Some(binary), None);
                Err(crate::error::AppError::platform(format!(
                    "could not remove staged raw Cargo binary: {error}; {rollback}"
                )))
            }
        },
        (None, Some(receipt)) => match receipt.try_remove() {
            Ok(cleanup_warning) => Ok(CurrentCargoUninstallOutcome {
                binary_path: None,
                receipt_path: receipt_removed,
                cleanup_warnings: cleanup_warning.into_iter().collect(),
            }),
            Err((error, receipt)) => {
                let rollback = restore_staged_pair(None, Some(receipt));
                Err(crate::error::AppError::platform(format!(
                    "could not remove staged cargo-dist receipt: {error}; {rollback}"
                )))
            }
        },
        (None, None) => Ok(CurrentCargoUninstallOutcome {
            binary_path: None,
            receipt_path: None,
            cleanup_warnings: Vec::new(),
        }),
    }
}

#[cfg(unix)]
pub(crate) fn revalidate_current_cargo_uninstall(
    plan: &CurrentCargoUninstallPlan,
) -> crate::error::Result<()> {
    let refreshed_binary = snapshot_binary(&plan.binary_path)?;
    let refreshed_receipt = snapshot_receipt(&plan.receipt_path)?;
    if refreshed_binary != plan.binary || refreshed_receipt != plan.receipt {
        return Err(self_uninstall_error(
            "the Cargo-path binary or receipt changed after preflight; preserving ownership state",
        ));
    }
    if let Some(receipt) = refreshed_receipt.as_ref() {
        if !same_path(&receipt.install_prefix, &plan.cargo_home) {
            return Err(self_uninstall_error(
                "the cargo-dist receipt prefix changed after preflight; preserving ownership state",
            ));
        }
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn revalidate_current_binary_uninstall(
    plan: &CurrentBinaryUninstallPlan,
) -> crate::error::Result<()> {
    let refreshed = snapshot_binary_with_label(&plan.binary_path, "running binary")?;
    if refreshed != plan.binary {
        return Err(self_uninstall_error(
            "the running binary changed after preflight; preserving the binary and shell profiles",
        ));
    }
    Ok(())
}

/// Consume the exact portable/non-Cargo binary proof. This uses the same
/// same-directory staging and no-clobber restoration contract as the Cargo
/// pair, but deliberately has no receipt side effect.
#[cfg(unix)]
pub(crate) fn commit_current_binary_uninstall(
    plan: CurrentBinaryUninstallPlan,
) -> crate::error::Result<CurrentCargoUninstallOutcome> {
    revalidate_current_binary_uninstall(&plan)?;
    let Some(binary) = plan.binary.as_ref() else {
        return Ok(CurrentCargoUninstallOutcome {
            binary_path: None,
            receipt_path: None,
            cleanup_warnings: Vec::new(),
        });
    };

    let staged = stage_binary_snapshot(binary, "running binary")?;
    if let Err(error) = ensure_absent(&plan.binary_path, "running binary") {
        let rollback = staged
            .restore()
            .err()
            .unwrap_or_else(|| "restored the running binary".to_string());
        return Err(crate::error::AppError::platform(format!(
            "{error}; {rollback}"
        )));
    }

    match staged.try_remove() {
        Ok(cleanup_warning) => Ok(CurrentCargoUninstallOutcome {
            binary_path: Some(plan.binary_path),
            receipt_path: None,
            cleanup_warnings: cleanup_warning.into_iter().collect(),
        }),
        Err((error, staged)) => {
            let rollback = staged
                .restore()
                .err()
                .unwrap_or_else(|| "restored the running binary".to_string());
            Err(crate::error::AppError::platform(format!(
                "could not remove staged running binary: {error}; {rollback}"
            )))
        }
    }
}

#[cfg(any(windows, unix))]
fn restore_receipt_noclobber(
    path: &Path,
    contents: &[u8],
    permissions: Option<&std::fs::Permissions>,
) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("receipt path has no parent: {}", path.display()),
        )
    })?;
    let mut restore = tempfile::Builder::new()
        .prefix(".tr300-receipt-restore-")
        .suffix(".tmp")
        .tempfile_in(parent)?;
    restore.write_all(contents)?;
    if let Some(permissions) = permissions {
        restore.as_file().set_permissions(permissions.clone())?;
    }
    restore.as_file().sync_all()?;
    restore
        .persist_noclobber(path)
        .map(|_| ())
        .map_err(|error| error.error)
}

/// Strict native-package cleanup treats the Cargo-path binary and a matching
/// cargo-dist receipt as one ownership record. Validate the receipt before any
/// mutation, then quarantine the binary in a randomized same-directory staging
/// directory while the receipt is removed. Any failure restores the prior pair
/// (or preserves the quarantine path in the diagnostic if restoration itself
/// fails), so a package rollback cannot strand the user's previously working
/// managed install.
#[cfg(any(windows, unix))]
fn execute_strict_cargo_pair(
    opts: &MigrateOptions,
    running_dir: Option<&Path>,
) -> Vec<TargetReport> {
    let binary_id = "cargo_copy";
    let binary_label = "older cargo copy".to_string();
    let receipt_id = "cargo_dist_receipt";
    let receipt_label = "matching cargo-dist receipt".to_string();

    let Some(cargo_home) = resolve_cargo_home(opts) else {
        return vec![
            TargetReport {
                id: binary_id,
                label: binary_label,
                path: None,
                outcome: TargetOutcome::Skipped(
                    "could not locate a Cargo home; preserving the Cargo-path install".to_string(),
                ),
            },
            TargetReport {
                id: receipt_id,
                label: receipt_label,
                path: None,
                outcome: TargetOutcome::Skipped(
                    "could not locate the receipt directory; preserving ownership state"
                        .to_string(),
                ),
            },
        ];
    };
    let cargo_bin = cargo_home.join("bin");
    let cargo_exe = cargo_bin.join(if cfg!(windows) { "tr300.exe" } else { "tr300" });
    let Some(receipt_path) = cargo_dist_receipt_path(opts) else {
        return vec![
            TargetReport {
                id: binary_id,
                label: binary_label,
                path: Some(cargo_exe),
                outcome: TargetOutcome::Skipped(
                    "could not locate the receipt directory; preserving the Cargo-path install"
                        .to_string(),
                ),
            },
            TargetReport {
                id: receipt_id,
                label: receipt_label,
                path: None,
                outcome: TargetOutcome::Skipped(
                    "could not locate the receipt directory; preserving ownership state"
                        .to_string(),
                ),
            },
        ];
    };

    // A raw Cargo install has no receipt and is an unambiguous single-file
    // target. Preserve the legacy deletion implementation for that case.
    if !receipt_path.exists() {
        let binary = execute_cargo_copy(opts, running_dir);
        let may_remove_receipt = matches!(&binary.outcome, TargetOutcome::Removed)
            || (matches!(&binary.outcome, TargetOutcome::WouldRemove) && opts.dry_run)
            || matches!(&binary.outcome, TargetOutcome::Skipped(reason) if reason == "no cargo copy present");
        return vec![binary, execute_cargo_dist_receipt(opts, may_remove_receipt)];
    }

    // A present receipt must be exact before either member of the ownership
    // pair moves. Malformed, unreadable, foreign-app, or wrong-prefix evidence
    // fails closed and leaves the prior installation byte-for-byte intact.
    let receipt_contents = match std::fs::read(&receipt_path) {
        Ok(contents) => contents,
        Err(error) => {
            return vec![
                TargetReport {
                    id: binary_id,
                    label: binary_label,
                    path: Some(cargo_exe),
                    outcome: TargetOutcome::Skipped(
                        "receipt is unreadable; preserving the Cargo-path install".to_string(),
                    ),
                },
                TargetReport {
                    id: receipt_id,
                    label: receipt_label,
                    path: Some(receipt_path.clone()),
                    outcome: if is_permission_error(error.kind()) {
                        TargetOutcome::NeedsAdmin(receipt_path.display().to_string())
                    } else {
                        TargetOutcome::Failed(format!("{}: {error}", receipt_path.display()))
                    },
                },
            ];
        }
    };
    let receipt_permissions = std::fs::metadata(&receipt_path)
        .ok()
        .map(|metadata| metadata.permissions());
    let receipt_text = match std::str::from_utf8(&receipt_contents) {
        Ok(text) => text,
        Err(error) => {
            return vec![
                TargetReport {
                    id: binary_id,
                    label: binary_label,
                    path: Some(cargo_exe),
                    outcome: TargetOutcome::Skipped(
                        "receipt is not UTF-8; preserving the Cargo-path install".to_string(),
                    ),
                },
                TargetReport {
                    id: receipt_id,
                    label: receipt_label,
                    path: Some(receipt_path),
                    outcome: TargetOutcome::Failed(format!("receipt is not UTF-8: {error}")),
                },
            ];
        }
    };
    if !receipt_matches_cargo_home(receipt_text, &cargo_home) {
        return vec![
            TargetReport {
                id: binary_id,
                label: binary_label,
                path: Some(cargo_exe),
                outcome: TargetOutcome::Skipped(
                    "receipt ownership is ambiguous; preserving the Cargo-path install".to_string(),
                ),
            },
            TargetReport {
                id: receipt_id,
                label: receipt_label,
                path: Some(receipt_path),
                outcome: TargetOutcome::Skipped(
                    "receipt does not exactly identify this app and Cargo home; preserving it"
                        .to_string(),
                ),
            },
        ];
    }

    if let Some(rd) = running_dir {
        if same_path(rd, &cargo_bin) {
            return vec![
                TargetReport {
                    id: binary_id,
                    label: binary_label,
                    path: Some(cargo_exe),
                    outcome: TargetOutcome::Skipped(
                        "the running install is the Cargo-path copy; preserving it".to_string(),
                    ),
                },
                TargetReport {
                    id: receipt_id,
                    label: receipt_label,
                    path: Some(receipt_path),
                    outcome: TargetOutcome::Skipped(
                        "the running install still owns this receipt; preserving it".to_string(),
                    ),
                },
            ];
        }
    }

    if !cargo_exe.exists() {
        return vec![
            TargetReport {
                id: binary_id,
                label: binary_label,
                path: None,
                outcome: TargetOutcome::Skipped("no cargo copy present".to_string()),
            },
            execute_cargo_dist_receipt(opts, true),
        ];
    }
    if !is_allowlisted(&cargo_exe) {
        return vec![
            TargetReport {
                id: binary_id,
                label: binary_label,
                path: Some(cargo_exe),
                outcome: TargetOutcome::Skipped(
                    "refusing: filename is not in the tr300 allowlist".to_string(),
                ),
            },
            TargetReport {
                id: receipt_id,
                label: receipt_label,
                path: Some(receipt_path),
                outcome: TargetOutcome::Skipped(
                    "the Cargo-path binary was not removed; preserving its receipt".to_string(),
                ),
            },
        ];
    }
    if opts.dry_run {
        return vec![
            TargetReport {
                id: binary_id,
                label: binary_label,
                path: Some(cargo_exe),
                outcome: TargetOutcome::WouldRemove,
            },
            TargetReport {
                id: receipt_id,
                label: receipt_label,
                path: Some(receipt_path),
                outcome: TargetOutcome::WouldRemove,
            },
        ];
    }

    let staging = match tempfile::Builder::new()
        .prefix(".tr300-migrate-")
        .tempdir_in(&cargo_bin)
    {
        Ok(staging) => staging,
        Err(error) => {
            return vec![
                TargetReport {
                    id: binary_id,
                    label: binary_label,
                    path: Some(cargo_exe),
                    outcome: if is_permission_error(error.kind()) {
                        TargetOutcome::NeedsAdmin(cargo_bin.display().to_string())
                    } else {
                        TargetOutcome::Failed(format!("{}: {error}", cargo_bin.display()))
                    },
                },
                TargetReport {
                    id: receipt_id,
                    label: receipt_label,
                    path: Some(receipt_path),
                    outcome: TargetOutcome::Skipped(
                        "could not stage the Cargo-path binary; preserving its receipt".to_string(),
                    ),
                },
            ];
        }
    };
    let backup = staging
        .path()
        .join(cargo_exe.file_name().unwrap_or_default());
    if let Err(error) = std::fs::rename(&cargo_exe, &backup) {
        let _ = staging.close();
        return vec![
            TargetReport {
                id: binary_id,
                label: binary_label,
                path: Some(cargo_exe),
                outcome: if is_permission_error(error.kind()) {
                    TargetOutcome::NeedsAdmin(cargo_bin.display().to_string())
                } else {
                    TargetOutcome::Failed(format!("{}: {error}", cargo_bin.display()))
                },
            },
            TargetReport {
                id: receipt_id,
                label: receipt_label,
                path: Some(receipt_path),
                outcome: TargetOutcome::Skipped(
                    "the Cargo-path binary was not staged; preserving its receipt".to_string(),
                ),
            },
        ];
    }

    if let Err(error) = std::fs::remove_file(&receipt_path) {
        let restored = std::fs::rename(&backup, &cargo_exe);
        let preserved = if restored.is_err() {
            Some(staging.keep())
        } else {
            let _ = staging.close();
            None
        };
        let restore_detail = match (restored, preserved) {
            (Ok(()), _) => "receipt removal failed; restored the Cargo-path binary".to_string(),
            (Err(restore_error), Some(path)) => format!(
                "receipt removal failed and binary restoration failed ({restore_error}); prior binary preserved at {}",
                path.display()
            ),
            (Err(restore_error), None) => {
                format!("receipt removal failed and binary restoration failed: {restore_error}")
            }
        };
        return vec![
            TargetReport {
                id: binary_id,
                label: binary_label,
                path: Some(cargo_exe),
                outcome: TargetOutcome::Failed(restore_detail),
            },
            TargetReport {
                id: receipt_id,
                label: receipt_label,
                path: Some(receipt_path.clone()),
                outcome: if is_permission_error(error.kind()) {
                    TargetOutcome::NeedsAdmin(receipt_path.display().to_string())
                } else {
                    TargetOutcome::Failed(format!("{}: {error}", receipt_path.display()))
                },
            },
        ];
    }

    if let Err(error) = std::fs::remove_file(&backup) {
        let binary_restore = std::fs::rename(&backup, &cargo_exe);
        let receipt_restore = restore_receipt_noclobber(
            &receipt_path,
            &receipt_contents,
            receipt_permissions.as_ref(),
        );
        let preserved = if binary_restore.is_err() {
            Some(staging.keep())
        } else {
            let _ = staging.close();
            None
        };
        let mut detail = format!("could not remove the private staged binary: {error}");
        match &binary_restore {
            Ok(()) => detail.push_str("; restored the Cargo-path binary"),
            Err(restore_error) => detail.push_str(&format!(
                "; binary restoration failed: {restore_error}{}",
                preserved
                    .as_ref()
                    .map(|p| format!(" (preserved at {})", p.display()))
                    .unwrap_or_default()
            )),
        }
        match receipt_restore {
            Ok(()) => detail.push_str("; restored the cargo-dist receipt"),
            Err(restore_error) => {
                detail.push_str(&format!("; receipt restoration failed: {restore_error}"))
            }
        }
        return vec![
            TargetReport {
                id: binary_id,
                label: binary_label,
                path: Some(cargo_exe),
                outcome: TargetOutcome::Failed(detail),
            },
            TargetReport {
                id: receipt_id,
                label: receipt_label,
                path: Some(receipt_path),
                outcome: TargetOutcome::Skipped(
                    "strict cleanup did not commit; prior ownership restoration was attempted"
                        .to_string(),
                ),
            },
        ];
    }

    let _ = staging.close();
    vec![
        TargetReport {
            id: binary_id,
            label: binary_label,
            path: Some(cargo_exe),
            outcome: TargetOutcome::Removed,
        },
        TargetReport {
            id: receipt_id,
            label: receipt_label,
            path: Some(receipt_path),
            outcome: TargetOutcome::Removed,
        },
    ]
}

#[cfg(any(windows, unix))]
fn execute_cargo_dist_receipt(opts: &MigrateOptions, may_remove: bool) -> TargetReport {
    let id = "cargo_dist_receipt";
    let label = "matching cargo-dist receipt".to_string();
    let Some(path) = cargo_dist_receipt_path(opts) else {
        return TargetReport {
            id,
            label,
            path: None,
            outcome: TargetOutcome::Skipped("could not locate the receipt directory".to_string()),
        };
    };
    if !path.exists() {
        return TargetReport {
            id,
            label,
            path: None,
            outcome: TargetOutcome::Skipped("no cargo-dist receipt present".to_string()),
        };
    }
    if !may_remove {
        return TargetReport {
            id,
            label,
            path: Some(path),
            outcome: TargetOutcome::Skipped(
                "the Cargo-path binary was not removed; preserving its receipt".to_string(),
            ),
        };
    }
    let Some(cargo_home) = resolve_cargo_home(opts) else {
        return TargetReport {
            id,
            label,
            path: Some(path),
            outcome: TargetOutcome::Skipped(
                "could not resolve the Cargo home recorded by the install".to_string(),
            ),
        };
    };
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return TargetReport {
            id,
            label,
            path: Some(path),
            outcome: TargetOutcome::Skipped("receipt is unreadable; preserving it".to_string()),
        };
    };
    if !receipt_matches_cargo_home(&contents, &cargo_home) {
        return TargetReport {
            id,
            label,
            path: Some(path),
            outcome: TargetOutcome::Skipped(
                "receipt does not exactly identify this app and Cargo home; preserving it"
                    .to_string(),
            ),
        };
    }
    if opts.dry_run {
        return TargetReport {
            id,
            label,
            path: Some(path),
            outcome: TargetOutcome::WouldRemove,
        };
    }
    let outcome = match std::fs::remove_file(&path) {
        Ok(()) => TargetOutcome::Removed,
        Err(error) if is_permission_error(error.kind()) => {
            TargetOutcome::NeedsAdmin(path.display().to_string())
        }
        Err(error) => TargetOutcome::Failed(format!("{}: {error}", path.display())),
    };
    TargetReport {
        id,
        label,
        path: Some(path),
        outcome,
    }
}

/// The two Windows edition bin dirs. LOCKSTEP with wix/main.wxs (Program
/// Files\tr300), wix-corporate/corporate.wxs + inno/corporate.iss
/// (LocalAppData\Programs\tr300), and `classify_install_path()` in update.rs.
#[cfg(windows)]
fn edition_bin_dirs(opts: &MigrateOptions) -> (Option<PathBuf>, Option<PathBuf>) {
    // Global perMachine: %ProgramFiles%\tr300\bin (machine-wide; process env ok).
    let global =
        std::env::var_os("ProgramFiles").map(|pf| PathBuf::from(pf).join("tr300").join("bin"));

    // Corporate perUser: %LocalAppData%\Programs\tr300\bin. Prefer the invoking
    // user's profile (installer-supplied) over the process env.
    let corporate = if let Some(profile) = &opts.user_profile {
        Some(
            PathBuf::from(profile)
                .join("AppData")
                .join("Local")
                .join("Programs")
                .join("tr300")
                .join("bin"),
        )
    } else {
        std::env::var_os("LOCALAPPDATA")
            .map(|la| PathBuf::from(la).join("Programs").join("tr300").join("bin"))
    };

    (global, corporate)
}

#[cfg(windows)]
fn execute_other_edition(opts: &MigrateOptions, running_dir: Option<&Path>) -> TargetReport {
    let id = "other_edition";
    let label = "other edition (Global/Corporate)".to_string();

    let (global_bin, corporate_bin) = edition_bin_dirs(opts);

    // Which edition is the running install? Authoritative marker first, then path.
    let other_bin: Option<PathBuf> = match detect_install_origin() {
        InstallOrigin::MsiGlobal | InstallOrigin::ExeGlobal => corporate_bin,
        InstallOrigin::MsiCorporate | InstallOrigin::ExeCorporate => global_bin,
        // Running install isn't in a known edition dir (cargo / portable /
        // unknown) — we can't safely pick "the other" edition, so skip.
        InstallOrigin::CargoOrInstaller | InstallOrigin::Unknown => None,
    };

    let Some(other_bin) = other_bin else {
        return TargetReport {
            id,
            label,
            path: None,
            outcome: TargetOutcome::Skipped(
                "running install is not a known Windows edition — cannot determine the other edition"
                    .to_string(),
            ),
        };
    };

    // Guard 4: never the running install's own directory.
    if let Some(rd) = running_dir {
        if same_path(rd, &other_bin) {
            return TargetReport {
                id,
                label,
                path: None,
                outcome: TargetOutcome::Skipped(
                    "computed 'other edition' equals the running edition — preserving it"
                        .to_string(),
                ),
            };
        }
    }

    let other_exe = other_bin.join("tr300.exe");
    if !other_exe.exists() {
        return TargetReport {
            id,
            label,
            path: None,
            outcome: TargetOutcome::Skipped("no other edition installed".to_string()),
        };
    }

    delete_target(id, label, &other_exe, opts.dry_run)
}

// ── Deletion ───────────────────────────────────────────────────────

/// Delete (or, in `--dry-run`, describe) a target binary. Guard 1 (allowlist) is
/// asserted here; the target is always a non-running copy (guard 4 enforced by
/// callers), so a plain `remove_file` suffices — no scheduled-delete needed.
#[cfg(any(windows, unix))]
fn delete_target(id: &'static str, label: String, exe: &Path, dry_run: bool) -> TargetReport {
    if !is_allowlisted(exe) {
        return TargetReport {
            id,
            label,
            path: Some(exe.to_path_buf()),
            outcome: TargetOutcome::Skipped(
                "refusing: filename is not in the tr300 allowlist".to_string(),
            ),
        };
    }

    if dry_run {
        return TargetReport {
            id,
            label,
            path: Some(exe.to_path_buf()),
            outcome: TargetOutcome::WouldRemove,
        };
    }

    let outcome = match std::fs::remove_file(exe) {
        Ok(()) => TargetOutcome::Removed,
        Err(e) if is_permission_error(e.kind()) => TargetOutcome::NeedsAdmin(format!(
            "{} (a perUser process cannot delete a perMachine copy — re-run elevated to remove it)",
            exe.display()
        )),
        Err(e) => TargetOutcome::Failed(format!("{}: {}", exe.display(), e)),
    };

    TargetReport {
        id,
        label,
        path: Some(exe.to_path_buf()),
        outcome,
    }
}

// ── Reporting ──────────────────────────────────────────────────────

fn outcome_word(outcome: &TargetOutcome) -> String {
    match outcome {
        TargetOutcome::Removed => "removed".to_string(),
        TargetOutcome::WouldRemove => "would remove (dry-run)".to_string(),
        TargetOutcome::Skipped(r) => format!("skipped: {}", r),
        TargetOutcome::NeedsAdmin(p) => format!("needs admin: {}", p),
        TargetOutcome::Failed(m) => format!("failed: {}", m),
    }
}

fn outcome_json_status(outcome: &TargetOutcome) -> &'static str {
    match outcome {
        TargetOutcome::Removed => "removed",
        TargetOutcome::WouldRemove => "would_remove",
        TargetOutcome::Skipped(_) => "skipped",
        TargetOutcome::NeedsAdmin(_) => "needs_admin",
        TargetOutcome::Failed(_) => "failed",
    }
}

fn color(text: &str, code: &str, config: &Config) -> String {
    if config.use_colors {
        format!("\x1b[{}m{}\x1b[0m", code, text)
    } else {
        text.to_string()
    }
}

fn print_human(reports: &[TargetReport], config: &Config, dry_run: bool) {
    println!();
    let header = if dry_run {
        "Install consolidation (dry-run — nothing will be deleted):"
    } else {
        "Install consolidation:"
    };
    println!("  {}", color(header, "36", config));
    for r in reports {
        let line = match &r.path {
            Some(p) => format!(
                "{} — {} [{}]",
                r.label,
                outcome_word(&r.outcome),
                p.display()
            ),
            None => format!("{} — {}", r.label, outcome_word(&r.outcome)),
        };
        let code = match &r.outcome {
            TargetOutcome::Removed | TargetOutcome::WouldRemove => "32",
            TargetOutcome::NeedsAdmin(_) | TargetOutcome::Failed(_) => "33",
            TargetOutcome::Skipped(_) => "90",
        };
        println!("    · {}", color(&line, code, config));
    }
    println!();
}

fn print_json(reports: &[TargetReport], targets: &CleanupTargets, dry_run: bool, success: bool) {
    let targets_json: Vec<serde_json::Value> = reports
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "label": r.label,
                "status": outcome_json_status(&r.outcome),
                "detail": match &r.outcome {
                    TargetOutcome::Skipped(s)
                    | TargetOutcome::NeedsAdmin(s)
                    | TargetOutcome::Failed(s) => Some(s.clone()),
                    _ => None,
                },
                "path": r.path.as_ref().map(|p| p.display().to_string()),
            })
        })
        .collect();

    let output = serde_json::json!({
        "action": "migrate-cleanup",
        "dry_run": dry_run,
        "requested": {
            "cargo_copy": targets.cargo_copy,
            "other_edition": targets.other_edition,
        },
        "targets": targets_json,
        "success": success,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    );
}

// ── Tests (cross-platform: bare filenames + forward slashes only) ───

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn no_flag_defaults_to_cargo_only() {
        let t = resolve_targets(false, false);
        assert!(t.cargo_copy);
        assert!(!t.other_edition);
    }

    #[test]
    fn explicit_flags_are_respected() {
        assert_eq!(
            resolve_targets(true, true),
            CleanupTargets {
                cargo_copy: true,
                other_edition: true
            }
        );
        assert_eq!(
            resolve_targets(false, true),
            CleanupTargets {
                cargo_copy: false,
                other_edition: true
            }
        );
    }

    #[test]
    fn strict_mode_accepts_absence_and_rejects_partial_or_ambiguous_cleanup() {
        let absent = TargetReport {
            id: "cargo_copy",
            label: "older cargo copy".to_string(),
            path: None,
            outcome: TargetOutcome::Skipped("no cargo copy present".to_string()),
        };
        assert!(!strict_report_failed(&absent));

        let removed = TargetReport {
            outcome: TargetOutcome::Removed,
            ..absent.clone()
        };
        assert!(!strict_report_failed(&removed));

        let ambiguous = TargetReport {
            path: Some(PathBuf::from("/tmp/tr300-receipt.json")),
            outcome: TargetOutcome::Skipped("receipt does not exactly match".to_string()),
            ..absent.clone()
        };
        assert!(strict_report_failed(&ambiguous));

        let blocked = TargetReport {
            outcome: TargetOutcome::NeedsAdmin("permission required".to_string()),
            ..absent
        };
        assert!(strict_report_failed(&blocked));
    }

    #[test]
    fn allowlist_accepts_only_tr300() {
        // Cross-platform assertions: bare filenames + forward-slash paths parse
        // identically on Windows and Unix.
        assert!(is_allowlisted(Path::new("tr300.exe")));
        assert!(is_allowlisted(Path::new("tr300")));
        assert!(is_allowlisted(Path::new("/home/me/.cargo/bin/tr300")));
        // Backslash paths only parse as paths on Windows; gate to Windows.
        #[cfg(windows)]
        {
            assert!(is_allowlisted(Path::new(
                r"C:\Program Files\tr300\bin\tr300.exe"
            )));
        }
    }

    #[test]
    fn allowlist_refuses_cargo_rustup_and_everything_else() {
        assert!(!is_allowlisted(Path::new("cargo.exe")));
        assert!(!is_allowlisted(Path::new("rustup.exe")));
        assert!(!is_allowlisted(Path::new("rustc")));
        assert!(!is_allowlisted(Path::new("cmd.exe")));
        assert!(!is_allowlisted(Path::new("/home/me/.cargo/bin/cargo")));
        // Merely containing our name is not allowlisted (exact match only).
        assert!(!is_allowlisted(Path::new("tr300-old.exe")));
        assert!(!is_allowlisted(Path::new("tr300-setup.exe")));
        #[cfg(windows)]
        {
            assert!(!is_allowlisted(Path::new(
                r"C:\Users\me\.cargo\bin\cargo.exe"
            )));
            assert!(!is_allowlisted(Path::new(r"C:\Windows\System32\cmd.exe")));
        }
    }

    #[test]
    fn permission_denied_is_an_admin_signal() {
        assert!(is_permission_error(std::io::ErrorKind::PermissionDenied));
        assert!(!is_permission_error(std::io::ErrorKind::NotFound));
    }

    #[test]
    fn outcome_json_status_is_stable() {
        // JSON contract values installers/scripts may read — renaming is a break.
        assert_eq!(outcome_json_status(&TargetOutcome::Removed), "removed");
        assert_eq!(
            outcome_json_status(&TargetOutcome::WouldRemove),
            "would_remove"
        );
        assert_eq!(
            outcome_json_status(&TargetOutcome::Skipped("x".into())),
            "skipped"
        );
        assert_eq!(
            outcome_json_status(&TargetOutcome::NeedsAdmin("x".into())),
            "needs_admin"
        );
        assert_eq!(
            outcome_json_status(&TargetOutcome::Failed("x".into())),
            "failed"
        );
    }

    #[test]
    fn downloads_is_never_a_computed_target_tail() {
        // migrate-cleanup only ever deletes from .cargo\bin, Program Files\tr300,
        // and LocalAppData\Programs\tr300 — none under Downloads.
        for t in [r"\.cargo\bin", r"\tr300\bin", r"\Programs\tr300\bin"] {
            assert!(!t.to_lowercase().contains("download"));
        }
    }

    #[cfg(any(windows, unix))]
    #[test]
    fn dry_run_deletes_nothing() {
        let dir = std::env::temp_dir().join(format!("tr300-migrate-dry-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let exe = dir.join(if cfg!(windows) { "tr300.exe" } else { "tr300" });
        std::fs::write(&exe, b"fake").unwrap();
        let report = delete_target("cargo_copy", "older cargo copy".to_string(), &exe, true);
        assert_eq!(report.outcome, TargetOutcome::WouldRemove);
        assert!(exe.exists(), "dry-run must NOT delete the file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(any(windows, unix))]
    #[test]
    fn delete_target_refuses_non_allowlisted_file() {
        let dir = std::env::temp_dir().join(format!("tr300-migrate-deny-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let cargo_exe = dir.join("cargo.exe");
        std::fs::write(&cargo_exe, b"not ours").unwrap();
        let report = delete_target("cargo_copy", "x".to_string(), &cargo_exe, false);
        assert!(matches!(report.outcome, TargetOutcome::Skipped(_)));
        assert!(cargo_exe.exists(), "cargo.exe must NOT be deleted");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(any(windows, unix))]
    #[test]
    fn receipt_restore_is_no_clobber() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("tr300-receipt.json");
        std::fs::write(&receipt, b"foreign replacement").unwrap();

        assert!(restore_receipt_noclobber(&receipt, b"prior receipt", None).is_err());
        assert_eq!(std::fs::read(&receipt).unwrap(), b"foreign replacement");
        assert!(root.path().read_dir().unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tr300-receipt-restore-")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn receipt_restore_preserves_private_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("tr300-receipt.json");
        let permissions = std::fs::Permissions::from_mode(0o600);
        restore_receipt_noclobber(&receipt, b"prior receipt", Some(&permissions)).unwrap();

        assert_eq!(std::fs::read(&receipt).unwrap(), b"prior receipt");
        assert_eq!(
            std::fs::metadata(&receipt).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn cargo_dist_receipt_requires_exact_provider_app_and_prefix() {
        let prefix = Path::new("/home/test/.cargo");
        let valid = r#"{
            "provider":{"source":"cargo-dist","version":"0.31.0"},
            "source":{"app_name":"tr300","name":"qube-machine-report"},
            "install_prefix":"/home/test/.cargo"
        }"#;
        assert!(receipt_matches_cargo_home(valid, prefix));
        assert!(!receipt_matches_cargo_home(
            &valid.replace("cargo-dist", "cargo"),
            prefix
        ));
        assert!(!receipt_matches_cargo_home(
            &valid.replace("tr300", "other"),
            prefix
        ));
        assert!(!receipt_matches_cargo_home(
            &valid.replace("/home/test/.cargo", "/tmp/other"),
            prefix
        ));
        assert!(!receipt_matches_cargo_home("not json", prefix));
    }

    #[cfg(unix)]
    #[test]
    fn unix_receipt_prefix_is_case_sensitive() {
        let receipt = r#"{
            "provider":{"source":"cargo-dist"},
            "source":{"app_name":"tr300"},
            "install_prefix":"/Users/Example/.cargo"
        }"#;
        assert!(receipt_matches_cargo_home(
            receipt,
            Path::new("/Users/Example/.cargo")
        ));
        assert!(!receipt_matches_cargo_home(
            receipt,
            Path::new("/users/example/.cargo")
        ));
    }

    #[cfg(any(windows, unix))]
    fn strict_fixture() -> (tempfile::TempDir, MigrateOptions, PathBuf, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let profile = root.path();
        let cargo_home = profile.join(".cargo");
        let cargo_bin = cargo_home.join("bin");
        std::fs::create_dir_all(&cargo_bin).unwrap();
        let binary = cargo_bin.join(if cfg!(windows) { "tr300.exe" } else { "tr300" });
        std::fs::write(&binary, b"prior managed binary").unwrap();

        let opts = MigrateOptions {
            cargo_copy: true,
            strict: true,
            user_profile: Some(profile.display().to_string()),
            ..MigrateOptions::default()
        };
        let receipt = cargo_dist_receipt_path(&opts).unwrap();
        std::fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        (root, opts, binary, receipt)
    }

    #[cfg(any(windows, unix))]
    #[test]
    fn strict_cargo_pair_removes_only_exact_binary_and_receipt_together() {
        let (_root, opts, binary, receipt) = strict_fixture();
        let cargo_home = resolve_cargo_home(&opts).unwrap();
        std::fs::write(
            &receipt,
            serde_json::json!({
                "provider": { "source": "cargo-dist" },
                "source": { "app_name": "tr300" },
                "install_prefix": cargo_home.display().to_string(),
            })
            .to_string(),
        )
        .unwrap();

        let running_elsewhere = tempfile::tempdir().unwrap();
        let reports = execute_strict_cargo_pair(&opts, Some(running_elsewhere.path()));
        assert_eq!(reports.len(), 2);
        assert!(reports
            .iter()
            .all(|report| matches!(report.outcome, TargetOutcome::Removed)));
        assert!(!binary.exists());
        assert!(!receipt.exists());
        assert!(binary.parent().unwrap().read_dir().unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tr300-migrate-")
        }));
    }

    #[cfg(any(windows, unix))]
    #[test]
    fn strict_cargo_pair_rejects_bad_receipt_before_mutating_binary() {
        let (_root, opts, binary, receipt) = strict_fixture();
        std::fs::write(&receipt, br#"{"provider":{"source":"foreign"}}"#).unwrap();

        let running_elsewhere = tempfile::tempdir().unwrap();
        let reports = execute_strict_cargo_pair(&opts, Some(running_elsewhere.path()));
        assert_eq!(reports.len(), 2);
        assert!(reports.iter().any(strict_report_failed));
        assert_eq!(std::fs::read(&binary).unwrap(), b"prior managed binary");
        assert!(receipt.exists());
    }

    #[cfg(any(windows, unix))]
    fn write_exact_receipt(opts: &MigrateOptions, receipt: &Path) -> Vec<u8> {
        let contents = serde_json::json!({
            "provider": { "source": "cargo-dist" },
            "source": { "app_name": "tr300" },
            "install_prefix": resolve_cargo_home(opts).unwrap().display().to_string(),
        })
        .to_string()
        .into_bytes();
        std::fs::write(receipt, &contents).unwrap();
        contents
    }

    #[cfg(any(windows, unix))]
    #[test]
    fn hidden_strict_cleanup_still_preserves_the_running_cargo_pair() {
        let (_root, opts, binary, receipt) = strict_fixture();
        let expected_receipt = write_exact_receipt(&opts, &receipt);

        let reports = execute_strict_cargo_pair(&opts, binary.parent());

        assert!(reports.iter().any(strict_report_failed));
        assert_eq!(std::fs::read(&binary).unwrap(), b"prior managed binary");
        assert_eq!(std::fs::read(&receipt).unwrap(), expected_receipt);
    }

    #[cfg(unix)]
    #[test]
    fn current_cargo_uninstall_removes_exact_managed_pair() {
        let (_root, opts, binary, receipt) = strict_fixture();
        write_exact_receipt(&opts, &receipt);

        let plan = preflight_current_cargo_uninstall_at(&binary, &opts, &receipt)
            .unwrap()
            .expect("Cargo-path binary should produce a self-uninstall plan");
        assert_eq!(plan.binary_path(), Some(binary.as_path()));
        assert_eq!(plan.receipt_path(), Some(receipt.as_path()));
        let outcome = commit_current_cargo_uninstall(plan).unwrap();

        assert_eq!(outcome.binary_path.as_deref(), Some(binary.as_path()));
        assert_eq!(outcome.receipt_path.as_deref(), Some(receipt.as_path()));
        assert!(!binary.exists());
        assert!(!receipt.exists());
        assert!(binary.parent().unwrap().read_dir().unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tr300-uninstall-")
        }));
        assert!(receipt.parent().unwrap().read_dir().unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tr300-uninstall-")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn staged_uninstall_surfaces_and_preserves_unexpected_directory_residue() {
        let root = tempfile::tempdir().unwrap();
        let original = root.path().join("tr300");
        std::fs::write(&original, b"managed binary").unwrap();
        let staged = stage_unix_path(&original, "test binary").unwrap();
        let staging_directory = staged.directory.clone();
        let unexpected = staging_directory.join("unexpected");
        std::fs::write(&unexpected, b"do not recursively delete").unwrap();

        let warning = staged
            .try_remove()
            .unwrap()
            .expect("nonempty staging directory must be surfaced");

        assert!(!original.exists());
        assert_eq!(
            std::fs::read(&unexpected).unwrap(),
            b"do not recursively delete"
        );
        assert!(warning.contains(&staging_directory.display().to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn current_cargo_uninstall_derives_custom_prefix_from_xdg_receipt() {
        let root = tempfile::tempdir().unwrap();
        let custom_prefix = root.path().join("managed-prefix");
        let binary = custom_prefix.join("bin").join("tr300");
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, b"custom managed binary").unwrap();

        let xdg_config = root.path().join("xdg-config");
        let receipt = xdg_config.join("tr300").join("tr300-receipt.json");
        std::fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        std::fs::write(
            &receipt,
            serde_json::json!({
                "provider": { "source": "cargo-dist" },
                "source": { "app_name": "tr300" },
                "install_prefix": custom_prefix.display().to_string(),
            })
            .to_string(),
        )
        .unwrap();

        // No CARGO_HOME/user-profile hint: the exact XDG receipt is the
        // authority for this custom cargo-dist prefix.
        let plan =
            preflight_current_cargo_uninstall_at(&binary, &MigrateOptions::default(), &receipt)
                .unwrap()
                .expect("custom receipt prefix should bind the running binary");
        assert_eq!(plan.cargo_home, custom_prefix);
        let outcome = commit_current_cargo_uninstall(plan).unwrap();

        assert_eq!(outcome.binary_path.as_deref(), Some(binary.as_path()));
        assert_eq!(outcome.receipt_path.as_deref(), Some(receipt.as_path()));
        assert!(!binary.exists());
        assert!(!receipt.exists());
    }

    #[cfg(unix)]
    #[test]
    fn current_custom_prefix_rejects_receipt_for_a_different_prefix() {
        let root = tempfile::tempdir().unwrap();
        let custom_prefix = root.path().join("managed-prefix");
        let binary = custom_prefix.join("bin").join("tr300");
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, b"custom managed binary").unwrap();
        let receipt = root
            .path()
            .join("xdg-config")
            .join("tr300")
            .join("tr300-receipt.json");
        std::fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        let receipt_bytes = serde_json::json!({
            "provider": { "source": "cargo-dist" },
            "source": { "app_name": "tr300" },
            "install_prefix": root.path().join("other-prefix").display().to_string(),
        })
        .to_string()
        .into_bytes();
        std::fs::write(&receipt, &receipt_bytes).unwrap();

        assert!(preflight_current_cargo_uninstall_at(
            &binary,
            &MigrateOptions::default(),
            &receipt,
        )
        .is_err());
        assert_eq!(std::fs::read(&binary).unwrap(), b"custom managed binary");
        assert_eq!(std::fs::read(&receipt).unwrap(), receipt_bytes);
    }

    #[cfg(unix)]
    #[test]
    fn current_raw_cargo_uninstall_remains_binary_only() {
        let (_root, opts, binary, receipt) = strict_fixture();
        assert!(!receipt.exists());

        let plan = preflight_current_cargo_uninstall_at(&binary, &opts, &receipt)
            .unwrap()
            .expect("raw Cargo binary should produce a self-uninstall plan");
        let outcome = commit_current_cargo_uninstall(plan).unwrap();

        assert_eq!(outcome.binary_path.as_deref(), Some(binary.as_path()));
        assert!(outcome.receipt_path.is_none());
        assert!(!binary.exists());
        assert!(!receipt.exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn current_cargo_uninstall_removes_valid_receipt_when_binary_path_is_absent() {
        let (_root, opts, binary, receipt) = strict_fixture();
        write_exact_receipt(&opts, &receipt);
        std::fs::remove_file(&binary).unwrap();
        let mut deleted_current_exe = binary.as_os_str().to_os_string();
        deleted_current_exe.push(" (deleted)");
        let deleted_current_exe = PathBuf::from(deleted_current_exe);

        let plan = preflight_current_cargo_uninstall_at(&deleted_current_exe, &opts, &receipt)
            .unwrap()
            .expect("unlinked running image should still validate its exact receipt");
        let outcome = commit_current_cargo_uninstall(plan).unwrap();

        assert!(outcome.binary_path.is_none());
        assert_eq!(outcome.receipt_path.as_deref(), Some(receipt.as_path()));
        assert!(!receipt.exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn existing_literal_deleted_suffix_never_authorizes_receipt_cleanup() {
        let (_root, opts, binary, receipt) = strict_fixture();
        let expected_receipt = write_exact_receipt(&opts, &receipt);
        std::fs::remove_file(&binary).unwrap();
        let mut literal_deleted = binary.as_os_str().to_os_string();
        literal_deleted.push(" (deleted)");
        let literal_deleted = PathBuf::from(literal_deleted);
        std::fs::write(&literal_deleted, b"literal running file").unwrap();

        assert!(preflight_current_cargo_uninstall_at(&literal_deleted, &opts, &receipt).is_err());
        assert_eq!(
            std::fs::read(&literal_deleted).unwrap(),
            b"literal running file"
        );
        assert_eq!(std::fs::read(&receipt).unwrap(), expected_receipt);
    }

    #[cfg(unix)]
    #[test]
    fn deleted_default_cargo_path_fails_closed_without_a_receipt_location() {
        let deleted_current_exe = Path::new("/home/test/.cargo/bin/tr300 (deleted)");
        assert!(path_looks_like_default_cargo_binary(deleted_current_exe));

        let error = preflight_current_cargo_uninstall_with_receipt_location(
            deleted_current_exe,
            &MigrateOptions::default(),
            None,
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("receipt location could not be resolved"));
        assert!(error.contains("preserving the binary and shell profiles"));
    }

    #[cfg(unix)]
    #[test]
    fn current_cargo_uninstall_rejects_ambiguous_receipts_before_any_mutation() {
        let cases = [
            b"not json".as_slice(),
            br#"{"provider":{"source":"foreign"},"source":{"app_name":"tr300"},"install_prefix":"/tmp/foreign"}"#,
            br#"{"provider":{"source":"cargo-dist"},"source":{"app_name":"other"},"install_prefix":"/tmp/foreign"}"#,
            br#"{"provider":{"source":"cargo-dist"},"source":{"app_name":"tr300"},"install_prefix":"/tmp/wrong-prefix"}"#,
        ];

        for contents in cases {
            let (root, opts, binary, receipt) = strict_fixture();
            let profile = root.path().join(".bashrc");
            std::fs::write(&profile, b"profile must remain byte-for-byte\n").unwrap();
            std::fs::write(&receipt, contents).unwrap();

            assert!(preflight_current_cargo_uninstall_at(&binary, &opts, &receipt).is_err());
            assert_eq!(std::fs::read(&binary).unwrap(), b"prior managed binary");
            assert_eq!(std::fs::read(&receipt).unwrap(), contents);
            assert_eq!(
                std::fs::read(&profile).unwrap(),
                b"profile must remain byte-for-byte\n"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn current_cargo_uninstall_rejects_symlinked_or_hardlinked_receipt() {
        use std::os::unix::fs::symlink;

        let (root, opts, binary, receipt) = strict_fixture();
        let target = root.path().join("receipt-target.json");
        let expected = write_exact_receipt(&opts, &target);
        symlink(&target, &receipt).unwrap();
        assert!(preflight_current_cargo_uninstall_at(&binary, &opts, &receipt).is_err());
        assert_eq!(std::fs::read(&binary).unwrap(), b"prior managed binary");
        assert_eq!(std::fs::read(&target).unwrap(), expected);
        assert!(std::fs::symlink_metadata(&receipt)
            .unwrap()
            .file_type()
            .is_symlink());

        let (root, opts, binary, receipt) = strict_fixture();
        let expected = write_exact_receipt(&opts, &receipt);
        let second_link = root.path().join("receipt-hardlink.json");
        std::fs::hard_link(&receipt, &second_link).unwrap();
        assert!(preflight_current_cargo_uninstall_at(&binary, &opts, &receipt).is_err());
        assert_eq!(std::fs::read(&binary).unwrap(), b"prior managed binary");
        assert_eq!(std::fs::read(&receipt).unwrap(), expected);
        assert_eq!(std::fs::read(&second_link).unwrap(), expected);
    }

    #[cfg(unix)]
    #[test]
    fn current_cargo_uninstall_rejects_linked_binary() {
        use std::os::unix::fs::symlink;

        let (root, opts, binary, receipt) = strict_fixture();
        write_exact_receipt(&opts, &receipt);
        let target = root.path().join("real-tr300");
        std::fs::rename(&binary, &target).unwrap();
        symlink(&target, &binary).unwrap();
        assert!(preflight_current_cargo_uninstall_at(&binary, &opts, &receipt).is_err());
        assert!(receipt.exists());
        assert!(target.exists());

        let (root, opts, binary, receipt) = strict_fixture();
        write_exact_receipt(&opts, &receipt);
        let second_link = root.path().join("tr300-hardlink");
        std::fs::hard_link(&binary, &second_link).unwrap();
        assert!(preflight_current_cargo_uninstall_at(&binary, &opts, &receipt).is_err());
        assert!(binary.exists());
        assert!(second_link.exists());
        assert!(receipt.exists());
    }

    #[cfg(unix)]
    #[test]
    fn current_cargo_uninstall_revalidates_before_commit() {
        let (_root, opts, binary, receipt) = strict_fixture();
        write_exact_receipt(&opts, &receipt);
        let plan = preflight_current_cargo_uninstall_at(&binary, &opts, &receipt)
            .unwrap()
            .unwrap();

        std::fs::write(&receipt, br#"{"provider":{"source":"foreign"}}"#).unwrap();
        assert!(commit_current_cargo_uninstall(plan).is_err());
        assert_eq!(std::fs::read(&binary).unwrap(), b"prior managed binary");
        assert_eq!(
            std::fs::read(&receipt).unwrap(),
            br#"{"provider":{"source":"foreign"}}"#
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_path_equality_never_collapses_invalid_bytes_to_replacement_character() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let root = tempfile::tempdir().unwrap();
        let invalid = root
            .path()
            .join(OsString::from_vec(vec![b'p', b'r', b'e', b'f', 0xff]));
        let replacement = root.path().join("pref\u{fffd}");

        // APFS rejects the invalid-byte component itself, so first exercise
        // the non-existent-path fallback on every Unix host. Linux then also
        // proves that successful canonicalization preserves the distinction.
        assert!(!same_path(&invalid, &replacement));

        #[cfg(not(target_os = "macos"))]
        {
            std::fs::create_dir(&invalid).unwrap();
            std::fs::create_dir(&replacement).unwrap();
            assert!(!same_path(&invalid, &replacement));
        }
    }

    #[cfg(unix)]
    #[test]
    fn validation_open_is_nonblocking_for_a_fifo_swap() {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::io::AsRawFd;
        use std::time::Duration;

        let root = tempfile::tempdir().unwrap();
        let fifo = root.path().join("validation-fifo");
        let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);

        let writer_path = fifo.clone();
        let writer = std::thread::spawn(move || {
            // This delayed writer prevents a regression from hanging the suite
            // forever; the descriptor flag assertion below remains the oracle.
            std::thread::sleep(Duration::from_millis(200));
            OpenOptions::new().write(true).open(writer_path).unwrap()
        });
        let reader = open_unix_validation_file(&fifo).unwrap();
        let flags = unsafe { libc::fcntl(reader.as_raw_fd(), libc::F_GETFL) };
        assert_ne!(flags, -1);
        assert_ne!(flags & libc::O_NONBLOCK, 0);
        let writer = writer.join().unwrap();
        drop((reader, writer));
    }

    #[cfg(unix)]
    #[test]
    fn portable_uninstall_uses_an_exact_revalidated_binary_plan() {
        let root = tempfile::tempdir().unwrap();
        let binary = root.path().join("portable-tr300");
        std::fs::write(&binary, b"portable binary").unwrap();

        let plan = preflight_current_binary_uninstall(&binary).unwrap();
        assert_eq!(plan.binary_path(), Some(binary.as_path()));
        let outcome = commit_current_binary_uninstall(plan).unwrap();

        assert_eq!(outcome.binary_path.as_deref(), Some(binary.as_path()));
        assert!(!binary.exists());
    }

    #[cfg(unix)]
    #[test]
    fn portable_uninstall_rejects_a_path_replacement_after_confirmation() {
        let root = tempfile::tempdir().unwrap();
        let binary = root.path().join("portable-tr300");
        std::fs::write(&binary, b"confirmed binary").unwrap();
        let plan = preflight_current_binary_uninstall(&binary).unwrap();

        std::fs::remove_file(&binary).unwrap();
        std::fs::write(&binary, b"replacement binary").unwrap();
        assert!(commit_current_binary_uninstall(plan).is_err());
        assert_eq!(std::fs::read(&binary).unwrap(), b"replacement binary");
    }

    #[cfg(unix)]
    #[test]
    fn missing_portable_image_never_selects_a_fallback_binary() {
        let root = tempfile::tempdir().unwrap();
        let deleted_running_path = root.path().join("portable-tr300 (deleted)");
        let unrelated_standard_path = root.path().join("tr300");
        std::fs::write(&unrelated_standard_path, b"unrelated install").unwrap();

        let plan = preflight_current_binary_uninstall(&deleted_running_path).unwrap();
        assert!(plan.binary_path().is_none());
        let outcome = commit_current_binary_uninstall(plan).unwrap();

        assert!(outcome.binary_path.is_none());
        assert_eq!(
            std::fs::read(&unrelated_standard_path).unwrap(),
            b"unrelated install"
        );
    }
}
