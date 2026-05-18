# Human Changelog

A plain-English mirror of [`CHANGELOG.md`](./CHANGELOG.md) — same releases,
same groupings, but rewritten so a non-technical reader can answer
"what shipped, and why should I care?" in 30 seconds.

The technical detail (CI run IDs, commit hashes, API names, error codes,
file paths, etc.) lives in `CHANGELOG.md` for engineers and agents. This
file is the user-facing summary.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [3.15.2] - 2026-05-18

> Cross-platform audit + remediation release. An end-to-end read-only
> audit of TR-300's install / update / runtime paths across macOS,
> Linux, and Windows surfaced 22 real issues; this release ships
> fixes for 19 of them. The remaining 3 are filed for future
> releases (small CI hardening, an internal refactor for graphical
> embedding, and a cosmetic alias-collision warning).

### Security
- **`tr300 update` now verifies a checksum on every downloaded
  Windows installer before running it.** Until now, the update flow
  trusted only HTTPS to GitHub. That's vulnerable to corporate
  TLS-inspection proxies with a trusted root certificate, public
  WiFi captive portals, and certain CDN tampering scenarios — an
  attacker could substitute a trojaned installer that TR-300 would
  then run, in the worst case (the Global EXE) with full admin
  rights after one UAC click. The update flow now fetches the
  installer's published `.sha256` companion file in a separate
  request, computes the hash of what it downloaded, and refuses to
  launch the installer on any mismatch.

### Fixed
- **Atomic profile writes prevent data loss on partial-write
  failures.** When `tr300 install` or `tr300 uninstall` modified
  your `~/.bashrc`, `~/.zshrc`, or PowerShell profile, the previous
  code truncated the target file and then wrote — a Ctrl-C, power
  loss, or antivirus quarantine in between would have left your
  profile empty or partially written. All four sites now route
  through a new atomic-write helper that writes to a side-by-side
  temporary file, flushes it to disk, then atomically renames it
  over the target. End result: your profile is either fully
  replaced or completely untouched, never partial. The first
  install also writes a one-time `.tr300-backup` copy of your
  original profile so you have a recovery option.
- **Profile sanity check refuses to mutate a damaged profile.**
  The block-removal parser used by install and uninstall opens at
  any line containing the TR-300 start marker and closes at any
  line containing the end marker. If you ever hand-edited the end
  marker out of your profile (a plausible mistake when tidying
  shell config), the next `tr300 install` would have silently
  dropped every line from the start marker to end-of-file — taking
  the rest of your shell profile with it. Install and uninstall
  now run a marker-balance check first and refuse the write on
  imbalance with an actionable error explaining how to repair the
  block by hand.
- **Auto-run no longer spams "command not found" if `tr300` is
  missing.** The snippet that runs `tr300 --fast` on every new
  shell previously called the binary unconditionally — so if you
  moved, deleted, or uninstalled `tr300`, every new terminal
  session printed an error message until you found and removed
  the snippet. The snippet now checks whether `tr300` exists on
  the path before running it, so a missing binary fails silently
  instead of nagging.
- **Nested shells no longer re-render the report.** A small marker
  is now set on the first auto-run in an interactive shell. Nested
  shells (vim's terminal mode, scripts that launch sub-shells, a
  Makefile's interactive bash, Windows Terminal's nested
  PowerShell tab) inherit it and the snippet short-circuits,
  preventing the table from being printed multiple times in CI
  logs and editor terminals.
- **More accurate interactive-session detection on Windows.** The
  PowerShell snippet now uses the documented "real user session"
  check instead of an older host-name match that was tripping on
  PowerShell calls from CI, VS Code, and scheduled tasks. Stops
  the report from rendering into non-interactive log streams.
- **`tr300 install` now writes to both PowerShell 5.1 and
  PowerShell 7 profiles when both are installed.** The two
  PowerShell flavors keep separate profile files and don't read
  each other's. Previously, users running only PowerShell 7 got a
  silent no-op install — the snippet went to a profile their
  shell never read. The installer now probes both shells, writes
  to whichever profiles exist, and deduplicates if they overlap.
- **More accurate locale detection on Unix.** TR-300 decides
  whether to render its table with Unicode box-drawing characters
  or fall back to ASCII based on the user's locale environment
  variables. The previous logic had an asymmetric quirk that
  meant some unusual configurations (like `LC_ALL` set to a
  non-UTF-8 European locale while `LANG` was UTF-8) returned the
  wrong answer. The check is now strict about POSIX precedence:
  the highest-priority variable wins outright.
- **Version comparison correctly handles pre-release tags.** A
  rare edge case: a user manually installed a pre-release like
  `v3.15.2-rc.1` and then ran `tr300 update`. The old comparator
  silently treated the pre-release tag the same as the stable
  release of the same version, so the user got "already on
  latest" and was stuck. The comparator now follows the standard
  semver rules: a stable release is newer than its own
  pre-release, and a pre-release of a higher version is newer
  than the prior stable.
- **`tr300 update` no longer reports a false success when the
  installer didn't actually replace the binary.** The Windows
  Installer can exit with success but defer the file replacement
  until reboot, particularly if another copy of `tr300` is open
  in another shell. The update flow now re-checks the version of
  the on-disk binary after the installer finishes and reports a
  clear "reboot, then verify" message instead of claiming the
  upgrade succeeded.
- **The Corporate EXE installer no longer leaves a stranded PATH
  entry on uninstall.** When TR-300's `bin` directory happened to
  be the very first entry in your PATH (most common on fresh
  corporate workstations where the user PATH was previously
  empty), uninstalling left the entry behind because of an
  off-by-one bug in the Pascal-Script PATH editor. The Global EXE
  installer had the same bug; both are fixed.
- **Windows console no longer leaks UTF-8 mode after TR-300
  exits.** On Windows, TR-300 switches the console output code
  page to UTF-8 while it runs so the box-drawing characters
  render correctly. The previous code never put the code page
  back — every subsequent program in the same console window saw
  UTF-8 output, which occasionally broke legacy batch scripts.
  The code page is now saved at startup and restored when
  TR-300 exits.
- **`tr300 install` on Unix refuses to run as root.** Some users
  reflexively ran `sudo tr300 install` thinking it was a
  system-level installer. It isn't — TR-300 modifies your personal
  shell profile, and running as root either wrote the change into
  root's profile (so it never reached the user) or left
  root-owned files in your home directory that broke the next
  non-sudo `tr300 install`. The command now refuses with a clear
  message pointing at `cargo install tr300` or the MSI/EXE
  installers for anyone who wants a system-wide install.
- **`tr300 install` on macOS now creates `.zshrc` instead of
  `.bashrc` on fresh user accounts.** macOS has used zsh as the
  default shell since 2019. On a fresh account with neither rc
  file present, the previous install created `.bashrc` — which
  the user's actual zsh shell never read, so the auto-run
  silently never fired.
- **`tr300 uninstall` -> Complete now works when run from the
  Corporate EXE install location.** Choosing "Complete" from
  `%LocalAppData%\Programs\tr300\bin\tr300.exe` previously
  cleaned the PowerShell profile but failed to delete the binary
  with a confusing error — Windows refuses to delete a running
  executable. TR-300 now detects this case and hands the
  deletion off to a small background script that runs after
  `tr300` exits, cleaning up the binary and its parent folder
  within a couple seconds. The shell returns to a prompt
  immediately.
- **TR-300 no longer hangs on machines with a broken WMI
  service.** Several Windows data sources (BitLocker status,
  multi-socket detection, network adapter info, cold-boot time)
  read through Windows' WMI subsystem. On machines where WMI is
  wedged (after certain Windows updates, on tightly-managed
  corporate fleets, or when antivirus interferes with the
  security-namespace queries), those calls used to block for
  tens of seconds with no escape hatch. Each WMI call now has a
  5-second timeout — the affected field is reported as missing
  instead, and the rest of the report renders normally.
- **Stronger JSON output guarantees.** The JSON output's string
  escaping is now produced by the standard `serde_json` library
  instead of a hand-rolled implementation. The hand-rolled
  version was correct for the cases it covered, but the swap
  eliminates a maintenance hazard and locks in spec-compliant
  behavior across the full Unicode range.
- **More accurate disk reporting on Windows configs with mounted
  folders.** A previous string match was too loose and could
  count a junction-mounted directory (a "folder shortcut"
  pointing at another drive) as if it were the C: drive itself.
  The check is now tightened to match exactly the system drive
  root.
- **Language-independent socket count and login-history parsing
  on Linux.** A few subprocesses TR-300 runs to gather data
  (`lscpu`, `lastlog`, `last`) translate their column labels
  into the user's language — German users saw "Sockel:" instead
  of "Socket(s):" and the parser silently missed multi-socket
  detection. These calls now run in the C (English) locale so
  the parsers work the same way regardless of the user's
  display language.

### Changed
- **Internal: shared installer constants and parser extracted to a
  single module.** The marker strings and block-removal logic used
  by `tr300 install` and `tr300 uninstall` were previously
  duplicated between the Unix and Windows code paths. Both now
  reference a shared module so a future rename touches one file.

### Documentation
- The project's minimum Windows version is now documented as
  Windows 10 1511 (2015). Older Windows (Windows 7 / Server 2008
  R2) can't load TR-300 at all because of a system-API change;
  the limit is moot in practice because Rust itself dropped
  Windows 7 a year ago, but it's now written down.

### Tests
- **26 new unit tests** across the install, update, and report
  paths exercise the atomic-write helpers, marker-balance check,
  snippet content pinning, semver prerelease ordering, checksum
  sidecar parsing, and JSON escape round-tripping. The library
  test count went from 72 to 98.

## [3.15.1] - 2026-05-15

> **Status:** This is the first published release of the new four-installer
> Windows distribution model. The version on `cargo install tr300` was
> already correct as of v3.15.0; this release is what makes all four
> direct-download Windows installers visible on the GitHub release page.
