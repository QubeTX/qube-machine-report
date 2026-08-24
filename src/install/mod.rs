//! Self-installation utilities
//!
//! Provides commands to install/uninstall tr300 to system paths.

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

pub mod prompt;
mod shared;

use crate::error::Result;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub use prompt::{confirm_complete_uninstall, prompt_uninstall_option, UninstallOption};

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn ReplaceFileW(
        replaced_file_name: *const u16,
        replacement_file_name: *const u16,
        backup_file_name: *const u16,
        replace_flags: u32,
        exclude: *mut std::ffi::c_void,
        reserved: *mut std::ffi::c_void,
    ) -> i32;
}

/// Replace an existing Windows file with a prepared sibling while preserving
/// the original file's DACL, encryption state, named streams, and other
/// identity metadata. `std::fs::rename`/`MoveFileExW` gives the replacement
/// inode the sibling's directory-inherited security descriptor instead.
#[cfg(windows)]
fn replace_existing_windows_file(path: &Path, replacement: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;

    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let replacement: Vec<u16> = replacement
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();

    // SAFETY: both path buffers are NUL-terminated and remain alive for the
    // call. The backup, exclusion, and reserved pointers are intentionally
    // null, and a zero flag set fails closed on any ACL merge error.
    let replaced = unsafe {
        ReplaceFileW(
            path.as_ptr(),
            replacement.as_ptr(),
            null_mut(),
            0,
            null_mut(),
            null_mut(),
        )
    };
    if replaced == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

// ── Shared file-write safety primitives (v3.15.2+) ──────────────────
//
// The install/uninstall flow mutates user-edited config files (~/.bashrc,
// ~/.zshrc, $PROFILE) that have no canonical copy elsewhere. A partial
// write or a stray "drop everything below TR-300 marker" parser bug
// deletes the user's hand-tuned shell config with no warning. These
// helpers are the safety net that makes both classes of failure
// recoverable.

/// Atomically write `content` to `path` via write-temp-then-rename.
///
/// `std::fs::write` opens the target with `O_TRUNC` / `CREATE_ALWAYS` and
/// then writes — if the process dies between the truncate and the write
/// completion (Ctrl-C, power loss, antivirus quarantine mid-write), the
/// rc file is left empty or partial. For files the user has invested
/// real time in (shell profiles), that loss is catastrophic and
/// irrecoverable.
///
/// This helper securely creates a randomized sibling temp file in the same
/// parent directory (guaranteed same volume so `rename` is atomic), fsyncs,
/// and atomically renames over the target. The target file ends up either
/// fully replaced or completely untouched — never partial.
pub(crate) fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    // If the target is a symlink (e.g. `~/.bashrc -> ~/dotfiles/bashrc`),
    // resolve it to the real file so the temp-then-rename happens in the real
    // file's directory and the symlink is preserved. Renaming over the symlink
    // path itself would replace the link with a regular file, orphaning the
    // user's dotfiles-managed target. A broken/unresolvable symlink falls back
    // to the literal path. Unix-only: symlinked rc files are a Unix dotfiles
    // convention, and Windows `canonicalize` returns verbatim `\\?\` paths that
    // would complicate the sibling-temp construction below.
    #[cfg(unix)]
    let resolved: PathBuf = match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
        }
        _ => path.to_path_buf(),
    };
    #[cfg(not(unix))]
    let resolved: PathBuf = path.to_path_buf();
    let path = resolved.as_path();

    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path has no parent: {}", path.display()),
        )
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path has no filename: {}", path.display()),
        )
    })?;
    let mut tmp_prefix = std::ffi::OsString::from(".");
    tmp_prefix.push(file_name);
    tmp_prefix.push(".tr300-");

    #[cfg(windows)]
    let target_existed = path.try_exists()?;

    // A shell profile may intentionally be private (for example mode 0600).
    // File::create applies the process umask to a fresh sibling, which can be
    // more permissive than the file being replaced. Capture the resolved
    // target's Unix permissions and apply them before any profile bytes enter
    // the temporary file, so atomic replacement never widens disclosure.
    #[cfg(unix)]
    let existing_permissions = fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions());

    let mut tmp = tempfile::Builder::new()
        .prefix(&tmp_prefix)
        .suffix(".tmp")
        .tempfile_in(parent)?;
    let write_result = (|| -> io::Result<()> {
        #[cfg(unix)]
        if let Some(permissions) = existing_permissions.as_ref() {
            tmp.as_file().set_permissions(permissions.clone())?;
        }
        tmp.write_all(content.as_bytes())?;
        tmp.as_file().sync_all()?;
        Ok(())
    })();
    write_result?;

    // Close the handle before ReplaceFileW (which can reject an open
    // replacement on Windows), while keeping custody of only this securely
    // created random sibling for the final atomic operation.
    let tmp_path = tmp
        .into_temp_path()
        .keep()
        .map_err(|persist_error| persist_error.error)?;

    #[cfg(windows)]
    let replace_result = if target_existed {
        replace_existing_windows_file(path, &tmp_path)
    } else {
        fs::rename(&tmp_path, path)
    };
    #[cfg(not(windows))]
    let replace_result = fs::rename(&tmp_path, path);

    if let Err(e) = replace_result {
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }

    Ok(())
}

/// Copy `path` to `<path>.tr300-backup` if no backup exists yet.
///
/// Idempotent: on second and subsequent install runs, the existing
/// backup (which captures the rc file from BEFORE TR-300 ever touched
/// it) is preserved. We never overwrite a backup with a TR-300-modified
/// version — that would silently destroy the user's original config.
/// Best-effort: a failure here is non-fatal; the atomic write itself is
/// the load-bearing safety net.
pub(crate) fn backup_once(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let mut bak_name = path.file_name().unwrap_or_default().to_os_string();
    bak_name.push(".tr300-backup");
    let bak_path = path.with_file_name(bak_name);
    if bak_path.exists() {
        return Ok(());
    }
    fs::copy(path, &bak_path)?;
    Ok(())
}

/// Verify that `MARKER_START` / `MARKER_END` line counts match before
/// the block parser mutates the file.
///
/// `remove_delimited_block` opens a block on any line containing
/// `MARKER_START` and closes it on any line containing `MARKER_END`. If
/// the user hand-edited `MARKER_END` out of their rc file — a real and
/// plausible failure mode when tidying up shell config — the parser
/// silently drops every line from `MARKER_START` to EOF on the next
/// install run. That's user-data loss with no warning.
///
/// This balance check refuses the write up-front with an actionable
/// error message instead, leaving the file untouched so the user can
/// repair it by hand.
pub(crate) fn check_marker_balance(
    content: &str,
    start: &str,
    end: &str,
) -> std::result::Result<(), String> {
    let starts = content.lines().filter(|l| l.contains(start)).count();
    let ends = content.lines().filter(|l| l.contains(end)).count();
    if starts == ends {
        return Ok(());
    }
    Err(format!(
        "TR-300 marker block in your shell profile looks mutilated:\n  found {starts} `{start}` line(s) and {ends} `{end}` line(s) (counts must match)\n\nThis usually means the `{end}` line was accidentally deleted. The TR-300\nblock parser would otherwise silently drop every line below `{start}` on\ninstall, so we refuse to write until you clean it up by hand:\n  - if you want TR-300 gone, remove the whole block manually and re-run\n  - if you want TR-300 installed, re-add the missing `{end}` line below your block"
    ))
}

/// Install tr300 to the system
pub fn install() -> Result<()> {
    #[cfg(unix)]
    {
        unix::install()
    }

    #[cfg(windows)]
    {
        windows::install()
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(crate::error::AppError::platform(
            "Self-installation not supported on this platform",
        ))
    }
}

/// Uninstall tr300 from the system
pub fn uninstall() -> Result<()> {
    #[cfg(unix)]
    {
        unix::uninstall()
    }

    #[cfg(windows)]
    {
        windows::uninstall()
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(crate::error::AppError::platform(
            "Self-uninstallation not supported on this platform",
        ))
    }
}

/// Get the installation path
pub fn install_path() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        Some(unix::install_path())
    }

    #[cfg(windows)]
    {
        Some(windows::install_path())
    }

    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

/// Find the location of the currently running binary
pub fn find_binary_location() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        unix::find_binary_location()
    }

    #[cfg(windows)]
    {
        windows::find_binary_location()
    }

    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

/// Get the parent directory of the binary (for cleanup on Windows)
pub fn get_binary_parent_dir(binary_path: &std::path::Path) -> Option<PathBuf> {
    #[cfg(unix)]
    {
        let _ = binary_path;
        None // Unix doesn't need directory cleanup
    }

    #[cfg(windows)]
    {
        windows::get_binary_parent_dir(binary_path)
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = binary_path;
        None
    }
}

/// Perform complete uninstall (profile + binary)
pub fn uninstall_complete() -> Result<()> {
    #[cfg(unix)]
    {
        unix::uninstall_complete()
    }

    #[cfg(windows)]
    {
        windows::uninstall_complete()
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(crate::error::AppError::platform(
            "Complete uninstallation not supported on this platform",
        ))
    }
}

#[cfg(test)]
mod shared_tests {
    use super::{atomic_write, backup_once, check_marker_balance};
    use std::fs;

    #[test]
    fn atomic_write_replaces_existing_file() {
        let dir = tempdir_in_target();
        let path = dir.join("test.txt");
        fs::write(&path, b"old content").unwrap();
        atomic_write(&path, "new content").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new content");
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_preserves_private_unix_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir_in_target();
        let path = dir.join("private-profile");
        fs::write(&path, b"private old content").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();

        atomic_write(&path, "private new content").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "private new content");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write_ignores_preplanted_legacy_temp_link() {
        let dir = tempdir_in_target();
        let path = dir.join("private-profile");
        let victim = dir.join("unrelated-user-file");
        let legacy_temp = dir.join(".private-profile.tr300-tmp");
        fs::write(&path, b"old profile").unwrap();
        fs::write(&victim, b"do not touch").unwrap();
        fs::hard_link(&victim, &legacy_temp).unwrap();

        atomic_write(&path, "new profile").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new profile");
        assert_eq!(fs::read_to_string(&victim).unwrap(), "do not touch");
        assert_eq!(fs::read_to_string(&legacy_temp).unwrap(), "do not touch");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn concurrent_atomic_writes_never_leave_partial_content() {
        use std::sync::{Arc, Barrier};

        let dir = tempdir_in_target();
        let path = dir.join("concurrent-profile");
        fs::write(&path, b"initial content").unwrap();
        let payloads: Vec<String> = (0..8)
            .map(|index| format!("writer-{index}:{}", "x".repeat(16 * 1024 + index)))
            .collect();
        let barrier = Arc::new(Barrier::new(payloads.len()));

        let outcomes = std::thread::scope(|scope| {
            let handles: Vec<_> = payloads
                .iter()
                .map(|payload| {
                    let path = path.clone();
                    let barrier = Arc::clone(&barrier);
                    scope.spawn(move || {
                        barrier.wait();
                        atomic_write(&path, payload)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });

        assert!(outcomes.iter().any(|outcome| outcome.is_ok()));
        let final_content = fs::read_to_string(&path).unwrap();
        assert!(payloads.iter().any(|payload| payload == &final_content));
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(".concurrent-profile.tr300-") && name.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "leftover temp files: {leftovers:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn atomic_write_preserves_windows_replacement_metadata() {
        use std::os::windows::fs::MetadataExt;

        let dir = tempdir_in_target();
        let path = dir.join("private-profile.ps1");
        fs::write(&path, b"private old content").unwrap();
        let original_creation_time = fs::metadata(&path).unwrap().creation_time();

        // NTFS creation timestamps have sub-millisecond resolution. Keeping a
        // visible gap makes this test prove that ReplaceFileW carried the old
        // identity metadata forward instead of retaining the new sibling's.
        std::thread::sleep(std::time::Duration::from_millis(20));
        atomic_write(&path, "private new content").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "private new content");
        assert_eq!(
            fs::metadata(&path).unwrap().creation_time(),
            original_creation_time
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write_creates_new_file() {
        let dir = tempdir_in_target();
        let path = dir.join("new.txt");
        atomic_write(&path, "hello").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write_cleans_up_temp_on_failure() {
        let dir = tempdir_in_target();
        let path = dir.join("existing-directory");
        fs::create_dir(&path).unwrap();

        assert!(atomic_write(&path, "data").is_err());

        assert!(path.is_dir());
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(".existing-directory.tr300-") && name.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "leftover temp files: {leftovers:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_preserves_symlinked_target() {
        // A dotfiles-managed rc file is commonly a symlink (~/.bashrc ->
        // ~/dotfiles/bashrc). atomic_write must update the backing file and
        // leave the symlink intact, not replace it with a regular file.
        use std::os::unix::fs::symlink;
        let dir = tempdir_in_target();
        let real = dir.join("real_bashrc");
        let link = dir.join(".bashrc");
        fs::write(&real, b"original\n").unwrap();
        symlink(&real, &link).unwrap();

        atomic_write(&link, "updated\n").unwrap();

        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink(),
            "the symlink must be preserved, not replaced by a regular file"
        );
        assert_eq!(fs::read_to_string(&real).unwrap(), "updated\n");
        assert_eq!(fs::read_to_string(&link).unwrap(), "updated\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn backup_once_creates_sidecar_when_missing() {
        let dir = tempdir_in_target();
        let path = dir.join(".bashrc");
        fs::write(&path, b"export FOO=bar\n").unwrap();
        backup_once(&path).unwrap();
        let bak = dir.join(".bashrc.tr300-backup");
        assert!(bak.exists());
        assert_eq!(fs::read_to_string(&bak).unwrap(), "export FOO=bar\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn backup_once_does_not_overwrite_existing_backup() {
        // The whole point of `_once`: a second install must not destroy
        // the original (pre-TR-300) backup.
        let dir = tempdir_in_target();
        let path = dir.join(".bashrc");
        let bak = dir.join(".bashrc.tr300-backup");
        fs::write(&bak, b"ORIGINAL untouched content\n").unwrap();
        fs::write(&path, b"# TR-300 already modified\n").unwrap();
        backup_once(&path).unwrap();
        assert_eq!(
            fs::read_to_string(&bak).unwrap(),
            "ORIGINAL untouched content\n",
            "second backup_once must not clobber the original backup"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn backup_once_is_noop_when_source_missing() {
        let dir = tempdir_in_target();
        let path = dir.join("never_existed");
        backup_once(&path).unwrap();
        let bak = dir.join("never_existed.tr300-backup");
        assert!(!bak.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn marker_balance_clean_file_passes() {
        let content = "export FOO=bar\nalias ll='ls -l'\n";
        assert!(check_marker_balance(content, "# TR-300 Machine Report", "# End TR-300").is_ok());
    }

    #[test]
    fn marker_balance_well_formed_block_passes() {
        let content =
            "export FOO=bar\n\n# TR-300 Machine Report\nalias report='tr300'\n# End TR-300\n";
        assert!(check_marker_balance(content, "# TR-300 Machine Report", "# End TR-300").is_ok());
    }

    #[test]
    fn marker_balance_two_well_formed_blocks_passes() {
        // Pathological but technically balanced (two installs without
        // cleanup). Parser handles this; balance check should too.
        let content = "# TR-300 Machine Report\nfoo\n# End TR-300\n\n# TR-300 Machine Report\nbar\n# End TR-300\n";
        assert!(check_marker_balance(content, "# TR-300 Machine Report", "# End TR-300").is_ok());
    }

    #[test]
    fn marker_balance_missing_end_is_refused() {
        // The actual data-loss scenario: user removed `# End TR-300` line.
        let content = "export FOO=bar\n\n# TR-300 Machine Report\nalias report='tr300'\nthe rest of my bashrc...\n";
        let err =
            check_marker_balance(content, "# TR-300 Machine Report", "# End TR-300").unwrap_err();
        assert!(err.contains("mutilated"), "error: {}", err);
        assert!(err.contains("# End TR-300"), "error: {}", err);
    }

    #[test]
    fn marker_balance_missing_start_is_refused() {
        // The inverse: an orphan `# End TR-300` line on its own. Less
        // common (user would have to delete the start without the end),
        // but refused for symmetry.
        let content = "alias foo=bar\n# End TR-300\nmore stuff\n";
        let err =
            check_marker_balance(content, "# TR-300 Machine Report", "# End TR-300").unwrap_err();
        assert!(err.contains("mutilated"), "error: {}", err);
    }

    /// Test scratch dir under the system temp dir. Each test gets its
    /// own dir keyed by pid+thread-id so parallel test runs don't
    /// collide.
    fn tempdir_in_target() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tr300-install-tests-{}-{:?}",
            std::process::id(),
            std::thread::current().id(),
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
