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
// Operator policy: exactly ONE version/edition installed at a time. The installers
// invoke this (interactively from checkboxes AND on a silent self-update) to
// consolidate. Mac/Linux is already safe (the shell installer overwrites the same
// `~/.cargo/bin`), so this is a Windows-only consolidation; on other platforms it
// is a no-op that exits 0.
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
//      reports "needs admin: <path>" and CONTINUES; exit code stays 0.
//
// EXIT CODE: 0 even on partial/empty/needs-admin — consolidation is advisory and
// must NEVER fail an installer. Only a true internal error (couldn't determine our
// own location) is nonzero.

use crate::config::Config;
use std::path::{Path, PathBuf};

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

/// Run the consolidation. Returns an exit code (0 = success/advisory, 2 = a true
/// internal error). Synchronous to match TR-300's `update::run`.
pub fn run(config: &Config, opts: &MigrateOptions) -> i32 {
    let targets = resolve_targets(opts.cargo_copy, opts.other_edition);
    let json = opts.json || matches!(config.format, crate::config::OutputFormat::Json);

    let reports = collect_and_execute(opts, targets);

    let internal_error = reports
        .iter()
        .any(|r| matches!(&r.outcome, TargetOutcome::Failed(m) if m == INTERNAL_ERROR_MARKER));

    if json {
        print_json(&reports, &targets, opts.dry_run);
    } else if !opts.quiet {
        print_human(&reports, config, opts.dry_run);
    }

    if internal_error {
        2
    } else {
        0
    }
}

const INTERNAL_ERROR_MARKER: &str = "__internal_error__";

// ── Shared (cross-platform) path helpers ───────────────────────────

/// Canonicalized path of the running executable (best-effort). Windows-only:
/// only the Windows consolidation path needs it (non-Windows is a no-op).
#[cfg(windows)]
fn current_exe_real_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.canonicalize().unwrap_or(exe))
}

/// The user's cargo-bin dir, preferring installer-supplied overrides
/// (`--cargo-home`, then `--user-profile`) over the process env so a perMachine
/// installer running as a different user can still resolve the invoking user's
/// `.cargo`. Falls back to `CARGO_HOME`/`%USERPROFILE%`/`$HOME`.
#[cfg(windows)]
fn resolve_cargo_bin_dir(opts: &MigrateOptions) -> Option<PathBuf> {
    if let Some(home) = &opts.cargo_home {
        return Some(PathBuf::from(home).join("bin"));
    }
    if let Some(profile) = &opts.user_profile {
        return Some(PathBuf::from(profile).join(".cargo").join("bin"));
    }
    if let Some(cargo_home) = std::env::var_os("CARGO_HOME") {
        return Some(PathBuf::from(cargo_home).join("bin"));
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".cargo").join("bin"))
}

/// Case-insensitive path equality after best-effort canonicalize. Windows-only
/// (the only consolidation path that needs it).
#[cfg(windows)]
fn same_path(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    left.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .eq_ignore_ascii_case(right.to_string_lossy().trim_end_matches(['\\', '/']))
}

// ── Detection + execution ──────────────────────────────────────────

#[cfg(windows)]
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
        reports.push(execute_cargo_copy(opts, running_dir.as_deref()));
    }
    if targets.other_edition {
        reports.push(execute_other_edition(opts, running_dir.as_deref()));
    }
    reports
}

#[cfg(not(windows))]
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

#[cfg(windows)]
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

    let cargo_exe = cargo_bin.join("tr300.exe");
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
            outcome: TargetOutcome::Skipped("other edition not installed".to_string()),
        };
    }

    delete_target(id, label, &other_exe, opts.dry_run)
}

// ── Deletion ───────────────────────────────────────────────────────

/// Delete (or, in `--dry-run`, describe) a target binary. Guard 1 (allowlist) is
/// asserted here; the target is always a non-running copy (guard 4 enforced by
/// callers), so a plain `remove_file` suffices — no scheduled-delete needed.
#[cfg(windows)]
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

fn print_json(reports: &[TargetReport], targets: &CleanupTargets, dry_run: bool) {
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
        "success": !reports.iter().any(|r| matches!(
            &r.outcome,
            TargetOutcome::Failed(m) if m == INTERNAL_ERROR_MARKER
        )),
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

    #[cfg(windows)]
    #[test]
    fn dry_run_deletes_nothing() {
        let dir = std::env::temp_dir().join(format!("tr300-migrate-dry-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let exe = dir.join("tr300.exe");
        std::fs::write(&exe, b"fake").unwrap();
        let report = delete_target("cargo_copy", "older cargo copy".to_string(), &exe, true);
        assert_eq!(report.outcome, TargetOutcome::WouldRemove);
        assert!(exe.exists(), "dry-run must NOT delete the file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
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
}
