---
name: windows-install
description: TR-300 install/uninstall edit-time rules. Load BEFORE editing the installer subsystem — `src/install/` (`unix.rs`, `windows.rs`, `mod.rs`, `prompt.rs`), the `report` alias / auto-run logic, the shell-profile / rc-file write path, the `atomic_write` / `backup_once` / `check_marker_balance` helpers, the alias-collision warning, the Windows execution-policy preflight, or the Windows install error advisor. Triggers on "edit the installer", "tr300 install", "tr300 uninstall", "shell profile", "rc-file write", "$PROFILE", "execution policy", "uninstall prompt", "OneDrive install error", "atomic_write". These are load-bearing invariants whose absence has truncated users' rc files or surfaced confusing errors at next shell start — do not undo them.
---

# TR-300 install / uninstall edit-time rules

The operational "what to do / what not to undo" rules for the install subsystem. The **why**
(prior failure modes, rejected alternatives) lives in
[`docs/architecture-decisions.md` § "Install / update safety primitives (v3.15.2+)"](../../../docs/architecture-decisions.md)
and the per-feature sections noted below; project overview + the tripwire summary live in
[`CLAUDE.md`](../../../CLAUDE.md). This skill is the full reference CLAUDE.md points at.

## Overview

`tr300 install` / `tr300 uninstall` modify shell profiles to add/remove the `report` alias and auto-run. The legacy `--install` / `--uninstall` flags are backward compatible aliases. Installation blocks are wrapped in marker comments for idempotent cleanup.

- **Unix/macOS** — `src/install/unix.rs` modifies `~/.bashrc` and/or `~/.zshrc`
- **Windows** — `src/install/windows.rs` modifies PowerShell `$PROFILE`

## Shell-profile write safety (v3.15.2+)

All four call sites that mutate an rc file route through helpers in `src/install/mod.rs`:

- `install::atomic_write(path, content)` — write-temp-then-rename, never `std::fs::write` directly. The temp file (`.<filename>.tr300-tmp`) lives in the target's parent directory so the `fs::rename` is on the same volume and atomic (POSIX rename is atomic; Windows `MoveFileEx` is atomic on NTFS within a volume). Either the new content is fully in place or the old content remains — never partial. Required because rc files are user-edited and have no canonical copy elsewhere; a Ctrl-C / power loss / AV quarantine mid-write previously truncated `~/.bashrc` with no recovery path. **Symlink preservation (v3.16.0+, E3):** on Unix, if the target is a symlink (a dotfiles setup like `~/.bashrc -> ~/dotfiles/bashrc`), `atomic_write` first resolves it via `fs::canonicalize` and does the temp-then-rename in the **real file's** directory — so the backing file is updated and the symlink is preserved, instead of `fs::rename` replacing the link with a regular file and orphaning the dotfiles target. A broken/unresolvable symlink falls back to the literal path; the same-volume-atomic-rename property is preserved (temp lives in the resolved file's parent). Unix-only — Windows `canonicalize` returns verbatim `\\?\` paths.
- `install::backup_once(path)` — copies the rc file to `<path>.tr300-backup` if no backup exists yet. Idempotent — second install run preserves the original (pre-TR-300) backup, never overwrites it with a TR-300-modified version. Best-effort; failure is non-fatal because `atomic_write` is the load-bearing protection.
- `install::check_marker_balance(content, MARKER_START, MARKER_END)` — refuses the write up-front when the user hand-edited the `# End TR-300` line out of their rc file. Without this, the existing `remove_delimited_block` parser would silently drop every line from `# TR-300 Machine Report` to EOF on the next install. The check counts lines containing each marker; an imbalance fails with an actionable error. Call this **before** any state mutation, immediately after `read_to_string`.

Don't replace these with `std::fs::write` for "simplicity." The non-atomicity is the bug.

## Alias-collision warning (v3.15.3+)

`install()` calls `warn_if_report_already_defined(&home)` on Unix and `warn_if_report_already_defined(&profile_paths)` on Windows *before* the rc-file modification loop. Edit-time rules:

- **Read-only scan, no subprocess.** Both implementations open the relevant rc files / `$PROFILE` paths via `fs::read_to_string` and grep them line-by-line for `alias report=` / `function report` / `Set-Alias` / `New-Alias` declarations, plus probe well-known PATH-bin locations (`~/.local/bin/report`, `~/bin/report`, `/usr/local/bin/report`, `/usr/bin/report` on Unix; PATH-scanned `report.exe` / `.cmd` / `.bat` / `.ps1` on Windows). Spawning the user's interactive shell (`bash -i -c 'type report'`) was rejected because sourcing the rc file can trigger fastfetch, network probes, tmux auto-attach, and other observable side effects during `tr300 install`.
- **Skip TR-300's own block.** Both scanners skip lines containing `MARKER_START` / `MARKER_END` or our own `alias report='...tr300'` / `Set-Alias ... report ... tr300` so re-running install doesn't warn about itself.
- **Best-effort heuristic — false negatives and false positives are both acceptable.** Misses aliases defined in shell-specific fragment files, auto-loaded PowerShell modules, etc. False positives surface as a one-time install-time message about a `report` the user is fine shadowing. The warning preserves the user's agency (they explicitly opted into `tr300 install`); we don't conditionally skip our alias write.
- **Non-blocking.** A failed scan (rc file unreadable, PATH unset) silently returns — the install always proceeds. (audit finding F17)

`--uninstall` is interactive (`src/install/prompt.rs`): the user picks `ProfileOnly`, `Complete` (also deletes the binary), or `Cancel`. The `Complete` path uses `find_binary_location()` + `confirm_complete_uninstall()` to show the path before deleting. Don't bypass the prompt unless the user has explicitly opted into a non-interactive variant.

## Windows execution-policy preflight (v3.14.4+)

`install()` runs `run_execution_policy_preflight()` **before** writing `$PROFILE`. Edit-time rules:

- The preflight is the *minimum-permissions* fix: `Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned -Force` *only* when the current `CurrentUser` policy is `Restricted` or `Undefined`. `RemoteSigned` is the strictest of PowerShell's policies that loads a local unsigned profile — it does **not** weaken protection against downloaded unsigned scripts. Never widen beyond `RemoteSigned`. Never touch `LocalMachine` scope (requires admin and affects all users). Never use persistent `Bypass`.
- **Never silently downgrade `AllSigned`.** That's a deliberate security choice — `policy_state("AllSigned")` returns `PolicyState::BlockedAllSigned`; the preflight prints a notice explaining the auto-run won't fire and leaves the policy alone.
- **Verify after set.** `Set-ExecutionPolicy` can exit 0 while a higher-precedence `MachinePolicy` / `UserPolicy` GPO still wins; `try_set_execution_policy()` re-reads `Get-ExecutionPolicy -Scope CurrentUser` and returns `TrySetResult::StillBlocked` so we can surface a fallback warning with the `LocalMachine`-scope remediation.
- **Failures are non-fatal.** `run_execution_policy_preflight()` returns `()` and never propagates an error to `install()`'s `Result` — the alias write half still succeeds even when the policy can't be fixed, so manual `tr300` invocations from the prompt keep working.
- **Order matters.** Run the preflight first, then write the profile. The reverse surfaces the confusing `UnauthorizedAccess` PSSecurityException at the *next* shell start, far from the install-time context where the user can act on it.
- The policy classification uses an enum (`PolicyState::{BlockedDefault, BlockedAllSigned, Permissive, Unknown}`) so unknown future PowerShell policy strings default to `Permissive` rather than triggering destructive action on values we can't reason about.

## Windows install error advisor (v3.14.5+)

Every fallible `std::fs` call in the install/uninstall flow funnels through `fail_install(InstallStep, &Path, io::Error)` instead of the old `map_err(|e| AppError::platform(format!("Failed to ...: {}", e)))` pattern. Edit-time rules:

- **Print the rich guidance to stderr, then return a concise `AppError`.** `fail_install()` streams a multi-paragraph advisory to stderr *before* returning, so it's never swallowed by anything that only captures the returned error. The returned `AppError::platform` is a short tag (`"write profile: ...err..."`) suitable for `main()`'s trailing `Error: ...` line — keep it short so the rich content above stays the focal point.
- **Dispatch on `(InstallStep, io::ErrorKind, raw_os_error, path_inspection)`.** The combination matters: `PermissionDenied` on a OneDrive-redirected path gets OneDrive-specific text (sync state, "always keep on this device"); the same error on a non-OneDrive path gets AD/Intune/AppLocker/WDAC + antivirus + `takeown` guidance. Don't collapse these into one generic block — each cohort needs different remediation.
- **`looks_like_onedrive_path()` checks for an "onedrive" path segment (case-insensitive)** so it catches both `\OneDrive\` and `\OneDrive - <TenantName>\` (OneDrive-for-Business). It will also match `\onedrive-migration.ps1\` etc.; the false-positive harms only the advisory text and is intentionally accepted to keep the predicate simple.
- **Always close with "Manual `tr300` still works from the prompt".** Install failures don't break the binary's basic functionality; the user needs to know what they CAN still do while they sort out the underlying restriction.
- **Don't move the rich output to stdout.** Stdout is for the normal install messages; rich error guidance belongs on stderr so it interleaves correctly with the trailing `Error: ...` line and so callers that capture stdout (e.g. CI scripts) still see the explanation.

## Source of truth

These rules are mirrored as a summary in [`CLAUDE.md` § "Edit-time rule skills"](../../../CLAUDE.md); the long-form rationale is in [`docs/architecture-decisions.md`](../../../docs/architecture-decisions.md) (atomic rc-file writes, marker-balance pre-check, and the v3.14.4 / v3.14.5 install sections). If those disagree with this skill, the docs win — fix this skill.
