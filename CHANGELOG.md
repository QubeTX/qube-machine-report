# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

> **Stability & cross-platform hardening pass.** A fresh three-agent audit of
> the whole tree (core data flow, platform collectors, install/update/CI)
> surfaced a set of correctness, robustness, and packaging issues across all
> six deployment targets (Windows / macOS / Linux × ARM + x86_64). This is the
> first batch (PR1 — output & build robustness). The macOS host-vs-process
> architecture fix (A3) is deferred to a macOS-hosted fast-follow; see
> `MASTER_PLAN.md`.

### Added
- **Windows now shows the machine `MODEL` row** (e.g. "Alienware m16 R2"),
  matching the macOS and Linux reports. The manufacturer + model come from the
  existing WMI batch (previously fetched only for virtualization detection and
  then discarded), composed with de-duplication and placeholder-OEM filtering.
  (D2)

### Changed
- **`session.last_login` is now JSON `null` (and the LAST LOGIN row is omitted)
  when login tracking is genuinely unavailable**, instead of the literal string
  `"Login tracking unavailable"`. Consumers can now distinguish a real failure
  from an absent value; `"Never logged in"` remains a genuine reported value.
  (C2)
- **Markdown auto-save now reports the concrete failure cause.** A failed save
  surfaces the underlying OS error (permissions, full disk, missing directory)
  on stderr instead of a generic "Could not save" warning, and notes when no
  Downloads folder was found and the report was written to the current
  directory instead. (C3)

### Fixed
- **JSON output can never emit invalid `NaN`/`Infinity` tokens.** `frequency_ghz`,
  `disk.percent`, `memory.percent`, and the load averages now serialize as
  `null` if a non-finite `f64` ever reaches them, so the document always parses.
  (C1)
- **`build.rs` no longer fails the build on a read-only source tree.** The
  generated man page is written authoritatively to `OUT_DIR`; the mirror copy
  into the project-root `man/` directory is now best-effort, so `cargo install
  tr300` from a locked-down registry cache (or any read-only/sandboxed source
  checkout) builds instead of panicking, and a normal build no longer dirties
  the working tree. (B1)
- **Linux load average no longer fabricates `0%` on a parse failure.** A
  malformed `/proc/loadavg` now falls through to the libc `getloadavg` fallback
  rather than reporting `0.0` (indistinguishable from a genuinely idle machine);
  only a total failure of both sources reports the rows as unavailable. (D7)
- **macOS shows the correct version codename on current releases.** The codename
  map now includes macOS 26 "Tahoe" (and any future major renders a generic
  `macOS <n>` label instead of nothing), and the 10.x era is keyed off the minor
  version. Previously the map stopped at macOS 15 (Sequoia), so current Macs
  showed no codename at all. (A1)
- **ARM Linux now reports CPU frequency instead of `0.00 GHz`.** On ARM SoCs that
  expose no CPUID leaf 16h and report 0 MHz through sysinfo, the frequency now
  falls back to the kernel's rated maximum from sysfs cpufreq
  (`/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq`). (A2)
- **Windows GPU list no longer leaks software adapters on the fallback path.**
  "Microsoft Basic Render Driver" / "Hyper-V Video" are now filtered in the rare
  PowerShell-fallback branch too, matching the primary path. (D1)
- **Windows boot mode no longer guesses "Legacy BIOS".** When `bcdedit` names no
  recognizable boot loader, the `BOOT` row is omitted rather than positively (and
  possibly wrongly) reporting Legacy on a UEFI machine. (D6)
- **Windows socket count is correct on single-processor machines with older
  PowerShell.** The CPU-count fallback query is wrapped in `@(...)` so a scalar
  result still yields 1 instead of an empty, unparseable string. (D9)
- **Linux battery health is computed from matching units.** The health
  percentage now requires the full-charge and design-capacity readings to come
  from the same unit family (both µWh `energy_*` or both µAh `charge_*`), so a
  system exposing only one of each no longer produces a nonsensical figure. (D3)
- **Linux GPU detection no longer misreads an audio device as a GPU.** The PCI
  class field is matched (rather than the whole `lspci` line), so a controller
  whose name merely contains "Display" (e.g. HDMI "Display Audio") is no longer
  listed as a graphics adapter. (D5)
- **ZFS health severity ranking is more accurate.** A `SUSPENDED` pool (I/O
  halted) is now ranked as the most severe state, and an unrecognized state no
  longer masquerades as `DEGRADED` when choosing the worst-of pool health. (D8)

### Internal
- **CI build/test/clippy/speed jobs run with `--locked`**, matching the
  `crates-publish` gate so CI can no longer resolve a newer transitive
  dependency than `Cargo.lock` pins and let an MSRV-incompatible bump slip
  through to a tagged release. (F1)
- Extracted the post-install version comparison into a pure, platform-
  independent `post_install_version_ok` helper with a unit test, so the
  self-update success check is verifiable on every target rather than only on
  the Windows runner. (E2)
- Corrected a stale comment claiming PowerShell 7 version selection used string
  comparison; the code correctly compares numeric `(major, minor, patch)`
  tuples (a string compare would rank `7.9.0` above `7.10.0`). (D10)

## [3.15.3] - 2026-05-23

> **Deferred-audit-findings follow-up release.** Resolves the three
> findings that the v3.15.2 cross-platform audit explicitly deferred
> (F17 alias-collision warning, F20 windows-installers.yml pre-flight,
> F22 COM init mode conflict for library consumers). All audit work
> from the May 2026 cycle is now landed.

### Fixed
- **Library consumers no longer silently degrade to the PowerShell
  fallback when their thread has initialized COM as
  `COINIT_APARTMENTTHREADED`.** Prior to this release the main `collect()`
  body called `COMLibrary::new()` (which internally calls
  `CoInitializeEx(NULL, COINIT_MULTITHREADED)`) on whatever thread
  invoked `SystemInfo::collect()`. A Tauri / Slint / winit GUI host
  that had already initialized COM as `COINIT_APARTMENTTHREADED` for
  its own UI thread would cause our call to fail with
  `RPC_E_CHANGED_MODE`, dropping all WMI-sourced fields (Windows
  edition, virtualization, GPU list, battery) to the PowerShell
  fallback path — which works but is ~10× slower. The WMI batch now
  runs inside a `with_timeout(WMI_BATCH_TIMEOUT, ...)` closure on a
  fresh worker thread (same pattern as the BitLocker, socket-count,
  network-info, and cold-boot lookups already use). The new thread
  has no prior COM init, so `MULTITHREADED` always succeeds regardless
  of the caller's COM state. The cheap main-thread probes
  (`get_gpus_fast` from the registry, `get_battery_native` via
  `GetSystemPowerStatus`) still run first so we can skip the
  corresponding WMI queries when the faster paths already worked —
  preserving the pre-F22 happy-path latency. (audit finding F22)
- **`windows-installers.yml` now refuses to build add-on installers
  for a torn upstream release.** The `workflow_run` trigger fires when
  cargo-dist's `release.yml` reports `conclusion == success`, but
  cargo-dist's host-job gate tolerates skipped matrix entries — a
  partially-failed release can still report success. A new pre-flight
  step probes the published GitHub Release for two sentinel assets
  (`dist-manifest.json`, cargo-dist's own "all phases done" signal;
  and `tr300-x86_64-pc-windows-msvc.msi`, the Global MSI our add-ons
  ship adjacent to). If either is missing, the workflow fails fast
  with an actionable message rather than spending ~5 minutes building
  installers against an incomplete release. (audit finding F20)
- **`tr300 install` warns when `report` is already defined in the
  user's shell environment.** A best-effort heuristic (read-only file
  scan, no subprocess) checks the standard rc files (`~/.bashrc`,
  `~/.bash_profile`, `~/.bash_aliases`, `~/.zshrc`, `~/.zprofile`,
  `~/.profile` on Unix; each detected `$PROFILE` on Windows) for
  `alias report=`, `function report`, `Set-Alias`, `New-Alias`
  declarations, plus probes standard `bin` directories for an
  executable named `report`. If anything is found, install prints a
  one-time stderr note showing the file:line of each conflict so the
  user knows the install is about to shadow their existing `report`.
  The install still proceeds — the user opted in, and a warning
  preserves their agency over how to resolve the conflict. (audit
  finding F17)

### Internal
- **`.claude/skills/` vendoring.** Four Anthropic-distributed agent
  skills (`brainstorming`, `critical-thinking`, `architecture`,
  `system-design`) are now bundled into the repo so every Claude Code
  agent working on TR-300 gets the same thinking toolkit regardless
  of the contributor's local plugin configuration. See
  `.claude/skills/ATTRIBUTION.md` for provenance and upstream-sync
  rules. No effect on the binary.

## [3.15.2] - 2026-05-18

> **Cross-platform audit + remediation release.** A three-agent
> read-only audit (`Explore` agents, Opus 4.7 1M context) of the
> install / update / runtime paths across macOS, Linux, and Windows
> surfaced 22 real findings — 3 critical (data-loss / security), 4
> high (reliable UX break), 9 medium (reliability / polish), 6
> low / informational. This release ships fixes for 19 of them
> across the install, update, collector, and Inno-Setup paths;
> the remaining 3 (CI hardening, COMLibrary caching, alias-
> collision warning) are filed for future PRs.

### Security
- **`tr300 update` verifies SHA256 of downloaded MSI / EXE
  installers before launching them.** Previously the update flow
  trusted only TLS to `github.com` and immediately handed the
  downloaded bytes to `msiexec /i` or ran the Inno Setup EXE.
  Corporate TLS-interception proxies with a trusted root CA,
  hostile public WiFi, captive portals, and CDN tampering could
  substitute a trojaned installer — worst case the Global EXE,
  which requests `PrivilegesRequired=admin` at launch, so one
  UAC click gave the attacker SYSTEM-equivalent code execution.
  The update flow now fetches `{url}.sha256` in a separate
  request, parses the cargo-dist `<lowercase-hex>  *<filename>`
  format (`.github/workflows/windows-installers.yml:213-220`
  writes the same form), computes SHA-256 via the `sha2` crate,
  and refuses to launch on mismatch with a clear error. (audit
  finding F3)

### Fixed
- **Atomic rc-file writes prevent data loss on partial-write
  failures.** `update_shell_profile` / `remove_from_profile` and
  the Windows `install` / `uninstall` flows previously used
  `std::fs::write`, which opens the target with `O_TRUNC` /
  `CREATE_ALWAYS` and then writes — a Ctrl-C, power loss, or AV
  quarantine between those two steps would leave the user's
  `~/.bashrc` / `~/.zshrc` / `$PROFILE` truncated or empty. All
  four sites now route through `install::atomic_write` (writes
  to a sibling `.<filename>.tr300-tmp`, fsyncs, atomic rename via
  `std::fs::rename` — atomic on POSIX, atomic on NTFS within a
  volume which the helper enforces). End result: the rc file is
  either fully replaced or completely untouched, never partial.
  First install also writes a one-time `<path>.tr300-backup`
  copy of the original profile via `install::backup_once`
  (idempotent — subsequent installs preserve the pre-TR-300
  backup). (audit finding F1)
- **Marker-block sanity check refuses to mutate a mutilated
  profile.** The `remove_delimited_block` parser opens a block
  on any line containing `# TR-300 Machine Report` and closes it
  on any line containing `# End TR-300`. If a user hand-edited
  the `# End TR-300` line out of their rc file (plausible when
  tidying shell config), the next `tr300 install` would have
  silently dropped every line from `# TR-300 Machine Report` to
  EOF. All four call sites now run a marker-balance check first
  and refuse the write on imbalance with an actionable error.
  (audit finding F2)
- **Auto-run snippet no longer spams every new shell when
  `tr300` is missing.** The injected snippet previously invoked
  `tr300 --fast` unconditionally — once the binary was moved,
  deleted, or `cargo uninstall`'d, every new interactive shell
  printed `bash: tr300: command not found` / PowerShell `The
  term 'tr300' is not recognized…` until the user found the
  snippet. The snippet now guards with `command -v tr300`
  (POSIX) / `Get-Command tr300 -ErrorAction SilentlyContinue`
  (Windows). Reinstalls overwrite the old unguarded block via
  the existing marker-based cleanup. (audit finding F4)
- **Recursion sentinel breaks nested-shell render loops.** A
  `TR300_AUTORUN_RAN` env var is set on first auto-run; nested
  shells (`bash -i -c …`, vim's `:term`, `pwsh -Command …`,
  Windows Terminal's nested PowerShell tab) inherit it and the
  guard short-circuits. Prior to this release nested invocations
  re-rendered the table once per level into CI logs and editor
  terminals. (audit finding F4)
- **Windows snippet now uses `[Environment]::UserInteractive`
  instead of `$Host.Name -eq 'ConsoleHost'`.** The prior check
  passed for `pwsh -Command "..."` invocations from CI / VS Code
  / scheduled tasks (all of which report `ConsoleHost` as their
  host name), causing the table to render into non-interactive
  log streams. `UserInteractive` is the documented check for a
  real user session. (audit finding F4)
- **`tr300 install` writes to BOTH Windows PowerShell 5.1 AND
  PowerShell 7 (`pwsh`) profiles when both shells are present.**
  The two shell flavors have different `$PROFILE` paths
  (`Documents\WindowsPowerShell\…` vs `Documents\PowerShell\…`)
  and don't cross-source each other. PowerShell 7-only users
  previously got a silent no-op install — the snippet went to a
  profile their actual shell never read. `get_powershell_profiles()`
  probes both launchers, deduplicates, and `install()` /
  `uninstall()` iterate over all discovered profiles. (audit
  finding F5)
- **`is_utf8_locale()` honors POSIX precedence symmetrically.**
  Pre-fix the function had an asymmetric skip: `LC_ALL=C` /
  `LC_ALL=POSIX` fell through to the next category, while
  `LC_ALL=de_DE.ISO-8859-1` short-circuited to `false`. The new
  implementation reads POSIX's precedence rules straight: any
  non-empty value in `LC_ALL` / `LC_CTYPE` / `LANG` wins
  outright. Users who set `LC_ALL=C, LANG=en_US.UTF-8` now
  correctly get ASCII output (they explicitly asked for the C
  locale). (audit finding F6)
- **`is_newer()` semver parser handles prereleases correctly.**
  The hand-rolled comparator used `filter_map(|s|
  s.parse::<u64>().ok())` which silently dropped non-numeric
  segments. `"3.15.2-rc.1".split('.')` produced
  `["3","15","2-rc","1"]`, the `2-rc` segment failed to parse
  and was dropped, the trailing `1` survived — leaving
  `[3, 15, 1]`, byte-identical to `"3.15.1"`. Users on
  manually-installed prereleases were stuck. A new
  `strip_prerelease_metadata` helper truncates at the first
  non-`[0-9.]` character, and the comparator now treats a
  stable release as newer than a prerelease of the same triple
  (semver §11). Build metadata (`+nightly.42`) is correctly
  ignored per semver §10. (audit finding F7)
- **`try_msi_install` / `try_exe_install` no longer claim
  success when the installer exited 0 but the on-disk binary
  didn't actually change.** After the installer returns, the
  update flow re-execs `env::current_exe() --version` and
  compares against the expected `latest`. msiexec exit code
  `3010` (REBOOT_REQUIRED) now produces a dedicated "Reboot,
  then verify with `tr300 --version`" message. (audit finding
  F8)
- **Inno Setup `EnvRemovePath` no longer strands the first PATH
  entry on uninstall.** Both `inno/global.iss` (HKLM) and
  `inno/corporate.iss` (HKCU) used `Delete(Paths, P - 1, ...)`
  to consume the leading `;` separator. When the target was the
  FIRST entry in PATH, `P` was 1 and `P - 1 = 0` —
  `Delete(s, 0, n)` is undefined behavior in Inno Setup's
  Pascal Script (treated as no-op), leaving the entry stranded.
  Both `.iss` files now branch on `P == 1` and delete from
  position 1 directly. (audit finding F9)
- **`enable_utf8_console()` on Windows restores the prior
  console code page on normal exit.** Pre-fix the function
  called `SetConsoleOutputCP(65001)` and never put it back —
  the change survived `tr300` exit, leaving every subsequent
  program in the same console seeing UTF-8 output, which
  occasionally broke legacy CP-437 batch scripts. The function
  now captures the prior CP via `GetConsoleOutputCP()` and
  returns a `ConsoleCpGuard` whose `Drop` impl restores it.
  Update / install / uninstall paths that call
  `std::process::exit` deliberately skip Drop (one-shot output,
  no meaningful restoration story). The gate now also runs
  when either stdout OR stderr is a terminal. (audit finding
  F10)
- **`tr300 install` on Unix refuses to run as root.**
  `dirs::home_dir()` consults `$HOME` first, but sudoers configs
  frequently reset `$HOME` to `/root` non-deterministically —
  `sudo tr300 install` could end up modifying `/root/.bashrc`
  or writing root-owned files into the user's home (subsequent
  non-sudo invocations fail with `EACCES`). `install()` now
  checks `libc::geteuid() == 0` and refuses with an actionable
  message pointing at `cargo install tr300` and the MSI/EXE
  installers. (audit finding F11)
- **`tr300 install` on macOS prefers `.zshrc` when neither rc
  file exists.** macOS has defaulted to zsh since 10.15
  (Catalina, 2019); pre-fix the install would create `.bashrc`
  on a fresh-account machine, leaving the auto-run silently
  dormant. `cfg(target_os = "macos")` branches now write
  `.zshrc` instead. Linux defaults remain `.bashrc`. (audit
  finding F12)
- **`tr300 uninstall` -> Complete on Windows handles the
  self-EXE delete case.** Pre-fix `uninstall_complete` called
  `fs::remove_file` synchronously on the currently-running
  binary, which Windows refuses with raw OS error 5 — the user
  was left with profile cleaned but binary still on disk.
  `is_running_binary()` now canonicalizes both paths and
  detects when the target IS the running EXE; in that case
  `schedule_self_cleanup` spawns a detached `cmd.exe` job
  (`DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP`) that waits 2
  seconds, then `del`s the binary and `rd /q`s the empty
  parent dir. (audit finding F13)
- **WMI queries in the Windows collector now have a hard
  5-second Rust-side timeout.** Pre-fix the WMI-touching
  collectors (`get_bitlocker_status`, `get_socket_count_wmi`,
  `get_network_info_wmi`, `last_cold_boot_seconds`) had no
  escape hatch — a deadlocked WMI provider (post-Windows-update
  misconfig, GPO lockdown, antivirus interfering with the
  `Win32_EncryptableVolume` security namespace, broken
  `Winmgmt`) could block `wmi.query()` for tens of seconds. A
  new module-level `with_timeout` (thread +
  `mpsc::channel.recv_timeout`) caps each WMI call at
  `WMI_TIMEOUT = 5s` and returns a conservative fallback on
  timeout. (audit finding F14)
- **`escape_json()` now delegates to `serde_json::to_string`.**
  The hand-rolled implementation was correct for the documented
  cases but easy to break with future maintenance. The
  delegated version is spec-compliant by construction across
  the full Unicode range. (audit finding F15)
- **`aggregate_disk_usage` no longer over-matches
  junction-mounted paths on Windows.** Tightened
  `d.mount_point.starts_with("C:")` to an exact case-insensitive
  comparison against `C:\` and `C:` so a directory-mount at
  `C:\mnt\D` doesn't wrongly match as if it were the C: root.
  (audit finding F18)
- **Linux locale-sensitive subprocesses run with `LC_ALL=C`.**
  `lscpu`'s `Socket(s):` label and `lastlog`'s `Never logged in`
  literal are localized — German users saw `Sockel:` and `Nie
  eingeloggt`, silently breaking the parsers. New
  `run_stdout_c_locale` and `run_output_with_env` helpers force
  the C locale; the `lscpu` socket-count call and
  `lastlog`/`lastlog2`/`last` invocations are switched over.
  (audit finding F19)

### Changed
- **New build-time dependency: `sha2 = "0.10"` (Windows-only).**
  Added under `[target.'cfg(windows)'.dependencies]` so
  non-Windows builds are unchanged. Pulls in `digest`,
  `block-buffer`, `crypto-common`, `cpufeatures` (~150 KB
  compiled). `sha2` is a RustCrypto member and the de-facto
  standard SHA-2 implementation in the Rust ecosystem.
- **Shared marker constants and block-removal parser extracted
  to `src/install/shared.rs`.** The `MARKER_START` /
  `MARKER_END` constants and the `remove_delimited_block`
  function were previously duplicated byte-for-byte across
  `unix.rs` and `windows.rs`. Both modules now reference
  `super::shared::{...}` so a future rename touches one file.
  Snippet contents in each platform module are pinned by unit
  tests (`shell_additions_contains_shared_markers`,
  `powershell_additions_contains_shared_markers`). (audit
  finding F16)

### Documentation
- CLAUDE.md now documents the **Windows 10 1511+ minimum**. The
  statically-linked `IsWow64Process2` call in `get_architecture`
  doesn't exist on older Windows; Rust's own minimum already
  drops Windows 7 (since rustc 1.78) so the bound is moot in
  practice. Documenting prevents an accidental Win7 backport
  attempt later. (audit finding F21)

### Tests
- **26 new unit tests** across `install`, `update`, and `report`:
  atomic-write success/failure, backup-once-only semantics,
  marker-balance check (well-formed / two-block /
  missing-end / missing-start), snippet content pinning
  (Unix + Windows), `strip_prerelease_metadata`,
  `is_newer` (prerelease ordering, build-metadata,
  equal-prereleases), `parse_sha256_sidecar` (cargo-dist
  format + variants), `compute_sha256` (RFC empty-input test
  vector), and `escape_json` round-trips (backslash + quote
  pairs, control chars, supplementary-plane Unicode, empty,
  plain ASCII). Total lib test count 72 → 98.

### Deferred to future releases
- The `windows-installers.yml` pre-flight asset-list check
  (audit finding F20) — small CI hardening, not user-facing.
- `OnceLock<COMLibrary>` caching for graphical-host library
  consumers (audit finding F22) — invasive refactor of
  `src/collectors/platform/windows.rs`.
- Pre-existing alias collision warning on install (audit
  finding F17) — marginal value, cosmetic only.

## [3.15.1] - 2026-05-15

> **Release status: COMPLETE.** All five workflow runs green:
> - master CI run 25902114185 (commit `b37e783`)
> - crates-publish run 25902206318 (`tr300 3.15.1` to crates.io @ 05:35:19 UTC)
> - release.yml run 25902253637 (GitHub Release published @ 05:41:10 UTC)
> - workflow_dispatch run 25902740025 of `windows-installers.yml`
>   (commit `5883627` after CI-infra fix-forwards `7715b93` + `5883627`)
>
> **Final GitHub Release v3.15.1: 28 assets** — verified via
> `gh release view v3.15.1`. Contains all four first-class Windows
> installers (Global MSI, Corporate MSI, Global EXE setup, Corporate
> EXE setup), 6 platform binary archives, cargo-dist installer
> scripts (canonical + legacy `tr-300-*` aliases), source tarball,
> and dist-manifest.
>
> Two CI-infra fix-forwards on master after the release commit
> were needed to attach the 3 additional installer assets:
> - `7715b93` switched `windows-installers.yml` trigger from
>   `release: published` to `workflow_run` (the former event doesn't
>   fire from GITHUB_TOKEN-authenticated `gh release create`).
> - `5883627` quoted PowerShell `-D` define args (the bareword
>   tokenizer split `-dVersion=3.15.1` at the dots).
> Future v3.15.2+ releases will fire `windows-installers.yml`
> automatically — no `workflow_dispatch` retry needed.

### Fixed
- **2026-05-15 — Patch release: v3.15.0 release.yml WiX build failure.**
  v3.15.0 was published to crates.io (master CI run 25901108698 +
  crates-publish run 25901198519) but `release.yml` run 25901237669 failed
  at `build-local-artifacts(x86_64-pc-windows-msvc)` with WiX candle exit
  code 6. No GitHub Release artifacts (no Global MSI, no installer scripts)
  were published for v3.15.0, and `windows-installers.yml` never fired
  because it triggers on `release: published`. Two distinct root causes
  were diagnosed by reproducing the failure locally with portable
  [WiX 3.11 binaries](https://github.com/wixtoolset/wix3/releases/tag/wix3112rtm):

  1. `wix/corporate.wxs` declared `<Property Id='ALLUSERS' Value=''/>`
     which WiX 3.11 candle rejects with `CNDL0006` ("Property/@Value
     attribute's value cannot be an empty string"). Fixed by removing
     both that Property and the redundant `MSIINSTALLPERUSER=1` Property
     — `InstallScope='perUser'` on the Package element is sufficient on
     WiX 3.11+ to declare a per-user MSI per the WiX 3 schema docs.

  2. cargo-wix's default behavior compiles ALL `.wxs` files in `wix/` and
     links them into a SINGLE MSI. Putting `corporate.wxs` alongside
     `main.wxs` hits link-stage errors LGHT0089 (Multiple entry sections),
     LGHT0091/0092 (Duplicate Property/Component/Media/Directory
     symbols). Even with cargo-dist's `allow-dirty = ["msi"]` config
     preserving the customized templates, cargo-wix still bundles both
     into one MSI. Fixed by moving the Corporate template to a NEW
     directory `wix-corporate/corporate.wxs` so cargo-wix's default scan
     of `wix/` produces only the Global MSI cleanly. The Corporate MSI is
     built separately by `.github/workflows/windows-installers.yml` using
     bare WiX `candle.exe` + `light.exe` directly (NOT through cargo-wix)
     with `-sice:ICE38 -sice:ICE64 -sice:ICE91` flags suppressing
     per-user-MSI convention violations (cosmetic for our single-binary
     install; install/uninstall both work; only consequence is empty
     `%LocalAppData%\Programs\tr300\bin\` after uninstall).

  Local verification on Windows 11 with WiX 3.11.2.4516 portable
  binaries: Global MSI builds via `cargo wix --no-build --nocapture` →
  1.9 MB output. Corporate MSI builds via bare `candle.exe -arch x64 ...`
  + `light.exe -sice:ICE38 -sice:ICE64 -sice:ICE91 ...` → 1.9 MB output.
  Both `light.exe` exit codes 0.

### Internal
- **2026-05-15 — v3.15.0 → v3.15.1 fix-forward follows the same pattern
  as v3.13.0 → v3.13.1** documented in MASTER_PLAN.md: a tagged release
  fails downstream of ci.yml/crates-publish; the tag stays in git as a
  historic record of the failure (immutable per /release § 13.2); a
  fresh patch release carries the fix. v3.15.0 crate is still on
  crates.io but has no GitHub Release artifacts; users on `cargo install
  tr300` get v3.15.1 automatically. Users tracking GitHub Releases see
  v3.15.1 as the first published release of the four-installer model.
- **2026-05-15 — `InstallSourceMarker` Components in both `wix/main.wxs`
  and `wix-corporate/corporate.wxs` are unchanged** between v3.15.0 and
  v3.15.1. An interim diagnosis attempt speculated they were the cause
  (ICE57 misattribution) and removed them; that was wrong and was
  reverted before this commit. Net change to those Components from
  v3.15.0 to v3.15.1: zero. Both MSIs still write
  `HKCU\Software\TR300\InstallSource`.
- **2026-05-15 — `Cargo.toml` `include` list** picks up the new
  `wix-corporate/**` directory so the published crate ships with the
  Corporate WiX source. cargo-dist's `allow-dirty = ["ci", "msi"]`
  flag, added in `8e98db4`, stays in place — still required for the
  customized `wix/main.wxs` Component additions.
- **2026-05-15 — Non-Windows `dead_code` lint suppression** on the
  `UpdateStrategy` enum (`cfg_attr(not(windows), allow(dead_code))`,
  added in commit `8e98db4`) stays. It's still required because the
  four MSI/EXE strategy variants are only constructed in a
  `cfg(windows)` block.
- **2026-05-15 — `windows-installers.yml` trigger-mechanism gap.** The
  v3.15.1 GitHub Release published successfully (release.yml run
  25902253637, 22 cargo-dist assets) but `windows-installers.yml` did
  NOT fire, so the three additional installer assets (Corporate MSI +
  Global EXE + Corporate EXE + sidecars = 6 files) didn't get
  attached. Root cause: the workflow's `on: release: types: [published]`
  trigger doesn't fire when the release was created via `gh release
  create` using the default `GITHUB_TOKEN`. GitHub
  [intentionally suppresses this](https://docs.github.com/en/actions/security-for-github-actions/security-guides/automatic-token-authentication#using-the-github_token-in-a-workflow)
  ("events triggered by the GITHUB_TOKEN, with the exception of
  workflow_dispatch and repository_dispatch, will not create a new
  workflow run") as loop-prevention. cargo-dist's release.yml uses
  GITHUB_TOKEN. **Fix:** change the trigger to `workflow_run` on the
  Release workflow's completion (the same pattern `crates-publish.yml`
  already uses to chain off `CI`). Also add a `workflow_dispatch`
  trigger with a `tag` input so the workflow can be manually fired
  for past releases that missed the automatic firing — this is what
  attached the three v3.15.1 installer assets retroactively after
  the trigger fix was committed. Future releases (v3.15.2+) will
  fire `windows-installers.yml` automatically off the workflow_run
  event without needing manual intervention.
- **2026-05-15 — PowerShell `-D` argument quoting in
  `windows-installers.yml`.** The first `workflow_dispatch` run of the
  fixed-trigger workflow (run 25902607841) failed with
  `candle.exe : error CNDL0103 : The system cannot find the file
  '.15.1' with type 'Source'`. Root cause: PowerShell's bareword
  tokenizer treats `-dVersion=3.15.1` as two tokens
  (`-dVersion=3` + `.15.1`) because the dots break argument
  boundaries — candle then sees `.15.1` as a positional source
  filename. Fix (commit `5883627`): quote each `-D` / `/D` preprocessor
  define as a single string literal so PowerShell passes the assignment
  as one arg. Three sites updated: candle's `-dVersion=...` and
  `-dCargoTargetBinDir=...`, iscc's `/DMyAppVersion=...` for both
  global.iss and corporate.iss. Run 25902740025 is the retry on this
  fix — that's the run that produces the final 6 assets.

## [3.15.0] - 2026-05-14

> **Release status:** `tr300` 3.15.0 IS on crates.io (crates-publish run
> 25901198519 from commit `8e98db4`) and is installable via
> `cargo install tr300`. The matching GitHub Release was NEVER created
> because `release.yml` run 25901237669 failed at
> `build-local-artifacts(x86_64-pc-windows-msvc)` with WiX candle exit
> code 6 (root cause analysis under [3.15.1] below). No `.msi`, no
> `.exe` installer, no zipped binaries were published for v3.15.0.
> The `v3.15.0` git tag remains as an immutable record of the failed
> release attempt. Use **v3.15.1** for the full four-installer
> distribution; this entry documents what was DESIGNED for v3.15.0,
> all of which shipped via v3.15.1.
>
> v3.15.0 reached the tag via two commits:
> - `92456a9` — initial squash-merge of the four-installer feature branch
> - `8e98db4` — CI fix-forward addressing two distinct issues caught by
>   the first `master` CI run on `92456a9`: (a) the non-Windows
>   `dead_code` lint on the four new `UpdateStrategy` variants
>   (Linux/macOS see them as never-constructed because the only
>   construction site is inside `cfg(windows)`); fixed by
>   `cfg_attr(not(windows), allow(dead_code))` on the enum. (b)
>   cargo-dist's `dist plan` job rejecting `wix/main.wxs` as "out of
>   date" because the customized `InstallSourceMarker` Component
>   diverged from the canonical cargo-dist template; fixed by
>   extending `[workspace.metadata.dist] allow-dirty` from `["ci"]` to
>   `["ci", "msi"]`. Both fixes were preserved in v3.15.1.

### Added
- **2026-05-14 — Corporate Edition MSI installer (perUser, no admin).** New
  `tr300-x86_64-pc-windows-msvc-corporate.msi` release artifact built from
  `wix/corporate.wxs`. Installs to `%LocalAppData%\Programs\tr300\bin\`,
  modifies user PATH only, no UAC prompt. Targets users on locked-down work
  machines who can't install software to `C:\Program Files`. Different WiX
  `UpgradeCode` from the Global MSI (`93F465CB-7F66-4930-A773-FDA017E8FD64`)
  so both editions can coexist as distinct Add/Remove Programs entries
  (though README documents picking only one).
- **2026-05-14 — Global EXE installer (Inno Setup, perMachine, requires admin).**
  New `tr300-x86_64-pc-windows-msvc-setup.exe` release artifact built from
  `inno/global.iss`. Installs to the same `C:\Program Files\tr300\bin\` path
  as the Global MSI; same outcome, different format for users who prefer a
  familiar `setup.exe`. AppId `AB14223F-2693-4EC2-824F-BF53CC32D061`.
- **2026-05-14 — Corporate EXE installer (Inno Setup, perUser, no admin).**
  New `tr300-x86_64-pc-windows-msvc-corporate-setup.exe` release artifact
  built from `inno/corporate.iss`. Installs to `%LocalAppData%\Programs\tr300\bin\`,
  no UAC prompt, modifies user PATH only. AppId
  `76A253EB-3A17-4730-9C54-5BE755A9BC4C`. With this, every release now ships
  four Windows installer formats: 2 MSIs × 2 EXE installers, each in Global
  (perMachine) and Corporate (perUser) editions.
- **2026-05-14 — Hand-authored `.github/workflows/windows-installers.yml`.**
  Triggers on `release: types: [published]` (fires after `release.yml`
  finishes creating the GitHub Release) and uploads 6 additional assets
  (the Corporate MSI + both EXE installers + their `.sha256` sidecars).
  Total release asset count: 28 (was 22). Does NOT touch the auto-generated
  `release.yml`. Installs Inno Setup via `choco install innosetup` and
  cargo-wix via `cargo install`.
- **2026-05-14 — Install-source registry marker (`HKCU\Software\TR300\
  InstallSource`).** All four first-class installers write a literal string
  value (`msi-global`, `msi-corporate`, `exe-global`, `exe-corporate`) on
  install. `tr300 update` reads this to choose the matching installer to
  download and re-run for in-place upgrades. Six new GUIDs total — all
  permanent (renaming any of them breaks user upgrade paths).

### Changed
- **2026-05-14 — `tr300 update` is now MSI/EXE-aware on Windows.**
  `src/update.rs::detect_install_origin()` reads the registry marker (or
  falls back to path-based detection for legacy installs) and dispatches to
  one of four new `UpdateStrategy` variants (`MsiGlobal`, `MsiCorporate`,
  `ExeGlobal`, `ExeCorporate`). MSI strategies run `msiexec /i /passive
  /norestart`; EXE strategies run `setup.exe /SILENT /SUPPRESSMSGBOXES
  /NORESTART`. No cross-fallback between installer types — re-running a
  different product would create coexistence problems. The legacy probe-
  and-retry chain (`cargo install` → PowerShell installer) still runs for
  the `CargoOrInstaller` / `Unknown` install origins, so users on the
  legacy install paths see unchanged behavior. macOS / Linux flow unchanged.
- **2026-05-14 — JSON output schema additions.** Every `tr300 update --json`
  response now includes a top-level `install_origin` field on Windows
  (`msi-global`, `msi-corporate`, `exe-global`, `exe-corporate`,
  `cargo-or-installer`, or `unknown`). New strategy IDs in successful-
  update responses and the `attempts` array: `msi_global`, `msi_corporate`,
  `exe_global`, `exe_corporate`. Additive only — existing fields unchanged,
  no `schema_version` bump.
- **2026-05-14 — README Installation section rewrite.** Leads with the four
  direct-download installer links (Global MSI / Global EXE / Corporate MSI
  / Corporate EXE) grouped by edition. Demotes `cargo install tr300` to a
  collapsed "Build from source" section labeled "for developers." Adds an
  explicit "Choosing between Global and Corporate Edition" decision table.
  SmartScreen "More info → Run anyway" instructions inline.
- **2026-05-14 — README Self-Update section rewrite.** Documents the new
  MSI/EXE-aware update flow on Windows with a per-install-origin table.
  The legacy chain still works for `cargo install` / PowerShell-installer
  users and is documented as such.
- **2026-05-14 — `wix/main.wxs` adds the `InstallSourceMarker` Component.**
  Writes `HKCU\Software\TR300\InstallSource = "msi-global"` on install.
  Component GUID `537B3C60-6E17-4EFF-8D56-E0CA97F447B5`. Backward-compatible
  for existing v3.14.x perMachine MSI installs — the marker just won't be
  set; the path-based fallback in `detect_install_origin()` returns
  `MsiGlobal` for binaries in `C:\Program Files\tr300\` even without it.

### Internal
- **2026-05-14 — Added `winreg = "0.52"` to `[target.'cfg(windows)'.
  dependencies]`** in `Cargo.toml`. Used by `read_install_source_marker()`
  for the registry read. Raw winapi for one registry read would be ~30 LOC
  of unsafe; winreg keeps it to ~5 LOC of safe Rust. The crate is tiny
  (a few hundred LOC wrapping winapi). Negligible binary-size impact.
- **2026-05-14 — Long-form rationale captured in `docs/architecture-
  decisions.md` § "Windows distribution model (v3.15.0+)".** Documents
  why four installers instead of one dual-purpose, why same install paths,
  why HKCU for the registry marker, why Inno Setup over WiX Burn / NSIS,
  why no SHA256 verification, and the rejected single-package-MSI pattern
  per [WiX issue #7137](https://github.com/wixtoolset/issues/issues/7137).

## [3.14.5] - 2026-05-14

### Changed
- **2026-05-14 — Windows install error messages now explain AD/Intune/OneDrive failures.**
  When `tr300 install` (or `tr300 uninstall`) fails to create the profile
  directory, read the profile, write the profile, or remove the binary, the
  error renderer now streams a multi-line advisory to stderr explaining the
  most likely cause and the remediation, before the trailing summary line.
  The advisory is dispatched from `(InstallStep, io::ErrorKind,
  raw_os_error, path_inspection)`: `PermissionDenied` / Windows error 5 on
  a OneDrive-redirected path surfaces OneDrive-specific guidance ("ensure
  Documents is locally available, not online-only"); the same error on a
  non-OneDrive path surfaces AD / Intune / AppLocker / WDAC / antivirus
  guidance with a `takeown` example; sharing-violation (error 32) calls
  out OneDrive sync + EDR holds; storage-full (error 112) and path-too-
  long (error 206) get their own branches. Manual `tr300` is always
  noted as still working, so the user knows the binary itself is fine
  and only the auto-run on new shells needs to be retried after fixing
  the underlying restriction.
## [3.14.4] - 2026-05-14

### Fixed
- **2026-05-14 — Windows `tr300 install` execution-policy preflight.** Fresh
  Windows machines default `ExecutionPolicy` to `Restricted`, which blocks
  every `.ps1` file including `$PROFILE` itself. The auto-run block written
  by `tr300 install` therefore never fired on first-run machines — opening a
  new PowerShell session produced `File ...Microsoft.PowerShell_profile.ps1
  cannot be loaded because running scripts is disabled on this system.
  ... FullyQualifiedErrorId : UnauthorizedAccess`. `tr300 install` now runs
  an execution-policy preflight on Windows: it inspects `Get-ExecutionPolicy
  -Scope CurrentUser`, and when the policy is `Restricted` or `Undefined` it
  runs `Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
  -Force` (HKCU only, no admin required) so the freshly written profile
  loads on the next shell. `RemoteSigned` is the minimum policy that allows
  the local profile to run; it does not weaken protection against downloaded
  unsigned scripts. If the user has deliberately set `AllSigned`, the
  installer prints a one-time notice explaining the auto-run won't fire and
  refuses to silently downgrade. If a higher-precedence Group Policy
  overrides the user-scope change, the installer surfaces a fallback
  warning with the `LocalMachine`-scope remediation. None of these branches
  abort the alias write; manual `tr300` invocations were never affected.

### Changed
- **2026-05-11 — documentation consistency pass.** Rechecked README,
  CHANGELOG, CODEX_PROJECT, AGENTS, CLAUDE, TESTING, MASTER_PLAN,
  architecture decisions, workflow files, and the global Codex agent guide for
  stale release/install/update wording after v3.14.3. Reconciled the internal
  MASTER_PLAN task inventory with shipped v3.14.0/v3.14.3 behavior and
  expanded release-checklist docs guidance so future agents update the full
  documentation set.

## [3.14.3] - 2026-05-11

### Changed
- **2026-05-11 — canonical crates.io package name.** Renamed the Cargo
  package from the deleted `tr-300` crate name to the lowercase canonical
  `tr300` crate name, bumped the release to v3.14.3, and updated the Rust
  library import path from `tr_300` to `tr300`.
- **2026-05-11 — update and installer paths.** Updated self-update's cargo
  strategy to run `cargo install tr300 --force`, changed canonical cargo-dist
  installer URLs to `tr300-installer.sh` and `tr300-installer.ps1`, and kept
  legacy `tr-300-installer.*` GitHub Release aliases so v3.14.2 binaries can
  still fall through from the deleted old Cargo package to the installer
  fallback.
- **2026-05-11 — cargo-dist release workflow compatibility.** Marked the
  cargo-dist CI file as intentionally customized with `allow-dirty = ["ci"]`
  so the release workflow can keep the small legacy installer alias step
  without failing the `dist plan` CI gate.
- **2026-05-11 — documentation alignment.** Refreshed README, local agent
  guides, project notes, architecture decisions, and the global Codex agent
  guide so install, release, self-update, library-import, and crates.io
  publishing instructions consistently point at `tr300`.

### Internal
- **2026-05-11 — crates.io availability check.** Confirmed the canonical
  `tr300` package name returned 404 from the crates.io API before publishing,
  so the v3.14.3 release can recreate the package under the correct name.
- **2026-05-11 — Release publication status.** Published `tr300` v3.14.3 to
  crates.io from GitHub Actions after `master` CI run 25648618096 passed on
  commit `25305d8`; crates publish run 25648707510 reran fmt, clippy, tests,
  package list, dry-run publish, and publish successfully. Pushed tag
  `v3.14.3`; release.yml run 25648740343 published the GitHub Release with 22
  cargo-dist assets, including canonical `tr300-installer.*` installers plus
  the `tr-300-installer.*` compatibility aliases.

## [3.14.2] - 2026-05-11

### Added
- **2026-05-11 — crates.io publication path.** Prepared `tr-300` for
  crates.io publication, documented `cargo install tr-300`, and tightened the
  Cargo package include list so published crates contain only the release
  manifest/lockfile, source, tests, build script, README, LICENSE, man page,
  and packaging assets.
- **2026-05-11 — CI-gated crates.io publishing.** Added `Crates.io Publish`,
  a GitHub Actions workflow that runs only after the `CI` workflow succeeds
  for a default-branch push, checks out the exact CI-tested SHA, skips versions
  already present on crates.io using a descriptive crates.io data-access
  `User-Agent`, reruns fmt/clippy/tests/package/dry-run, and publishes with
  the repository `CARGO_REGISTRY_TOKEN`.
- **2026-05-11 — project identity cleanup.** Removed embedded unrelated
  historical implementation files and public lineage wording so TR-300 is
  documented as a standalone QubeTX project.

### Changed
- **2026-05-11 — ND-style self-update strategy chain.** Replaced path-based
  updater method detection with a probe-and-retry chain matching ND-300:
  cargo first when `cargo` is invokable, then `curl`/`wget` shell installers on
  Unix or `powershell`/`pwsh` on Windows. Failures now fall through to the next
  strategy and surface per-attempt diagnostics in terminal and JSON output.
- **2026-05-11 — update JSON diagnostics.** `tr300 update --json` now includes
  a precise `"strategy"` on successful updates and an `"attempts"` array on
  failure while preserving the legacy `"method"` field.
- **2026-05-11 — README release docs refresh.** Updated install and release
  documentation for v3.14.2, documented all supported install methods
  (`cargo install tr-300`, shell installer, PowerShell installer, MSI, exact
  Git tag, and source builds), described the automatic crates.io workflow, and
  narrowed README wording for Windows session and BitLocker behavior to match
  the implemented surface.
- **2026-05-11 — release workflow docs.** Updated local repo agent guides,
  project notes, and the global Codex agent guide with the v3.14.2 release
  workflow: bump version, keep changelog/docs current, run local release gates,
  push the default branch, wait for CI, let crates.io publishing run from the
  CI-tested SHA, then push the explicit cargo-dist tag for installers.
- **2026-05-11 — lockfile tracking.** Started tracking `Cargo.lock` so
  `cargo publish --locked` and the GitHub publishing workflow use the same
  resolved dependency set that local verification used.

### Internal
- **2026-05-11 — Release publication status.** Published `tr-300` v3.14.2 to
  crates.io from GitHub Actions after `master` CI run 25647466576 passed on
  commit `a6c3841`; crates publish run 25647553585 reran fmt, clippy, tests,
  package list, dry-run publish, and publish successfully. Pushed tag
  `v3.14.2`; release.yml run 25647597021 published the GitHub Release with 20
  cargo-dist assets. An initial crates workflow run 25647407638 failed before
  publishing because the crates.io version check lacked a descriptive
  data-access `User-Agent`; follow-up commit `a6c3841` fixed that workflow
  before the successful publish.

## [3.14.1] - 2026-05-11

### Fixed
- **2026-05-11 — Release confidence patch after CI fix-forward.** Bumped
  the patch version after confirming the latest `master` CI run was green
  and local `fmt` / `clippy -D warnings` / test gates passed. This release
  republishes the v3.14.x line after the warning-clean cross-platform
  collector fixes from `54dbae1` and the v3.14.0 release-publication
  documentation from `5709f9a`.

### Internal
- **2026-05-11 — Release metadata refresh.** Updated package metadata,
  project status docs, and generated man-page version text for v3.14.1.
- **2026-05-11 — Release publication status.** Recorded tag commit
  `3328a8e`, CI run 25645894617, release run 25645999755, and the 20
  published cargo-dist assets for the v3.14.1 GitHub Release.

## [3.14.0] - 2026-05-10

### Added
- **2026-05-10 18:00 CDT — Positional action syntax.** Added
  `tr300 update`, `tr300 install`, and `tr300 uninstall` as no-double-dash
  equivalents for the legacy `--update`, `--install`, and `--uninstall`
  flags. The installed `report` alias inherits the same parser, so
  `report update` now works too. Clap rejects mixed actions such as
  `tr300 update --install` with an argument-conflict error.
- **2026-05-10 18:00 CDT — Conditional platform rows.** Added nullable
  report fields for machine model, CPU core topology, motherboard, BIOS,
  and RAM slot summary. Rows render only when collectors populate them,
  and JSON receives additive nullable keys without a schema-version bump.

### Changed
- **2026-05-10 18:00 CDT — Cross-platform collector hardening.** Added a
  shared subprocess helper with bounded fast/normal/slow timeouts and moved
  collector command probes onto it so missing tools, blocked subprocesses,
  and bad platform responses degrade to omitted data instead of hanging.
  Rendering now guards non-finite percentages, disk aggregation saturates
  instead of overflowing, markdown cells escape table-breaking characters,
  and Windows WTS time buffers are copied before decoding.
- **2026-05-10 18:00 CDT — Platform accuracy pass.** Improved macOS model,
  hostname, locale, Rosetta, Apple Silicon frequency, P/E core, battery
  health, and `vm_stat` fallback behavior; improved Linux default-route IP,
  systemd-resolved DNS, locale, battery, terminal, container/VM/WSL,
  aarch64 CPU, ZFS, and elevated `dmidecode` paths; and added a batched
  PowerShell fallback for Windows WMI-failure cases while keeping fast mode
  free of COM/WMI work.
- **2026-05-10 18:00 CDT — Windows elevation wording.** The runtime footer
  and docs now advertise only implemented elevated Windows data
  (BitLocker status). Admin-only RDP login history remains deferred.

### Internal
- **2026-05-10 18:40 CDT — Release publication status.** Updated
  `MASTER_PLAN.md` and `TESTING.md` after the v3.14.0 GitHub Release
  completed: recorded tag commit `54dbae1`, CI run 25642712712, release
  run 25642853066, and the 20 published cargo-dist assets.
- **2026-05-10 17:28 CDT — Codex plugin settings migration.** Mirrored the
  project Claude plugin setting for `codex@openai-codex` into
  `.codex/config.toml`, including the `openai-codex` Git marketplace source,
  and added `CODEX_PROJECT.md` with the current project summary and filetree.
- **Documentation restructure (zero behavior change).** Moved the
  long-form rationale for six load-bearing decisions out of `CLAUDE.md`
  into a new file at `docs/architecture-decisions.md`: MSRV policy +
  v3.13.1 `rust-toolchain.toml` addendum, self-update auto-rustup
  reasoning, Intel macOS CI coverage policy, and the three Windows
  accuracy pattern blocks (v3.11.0+, v3.12.0+, v3.13.0+). CLAUDE.md
  retains substantive **edit-time rules** (which APIs to use, which
  constants are required, which alternatives not to undo), each capped
  with a deep-link to the corresponding decisions section. Result:
  CLAUDE.md drops from 45 KB → 31.5 KB (~30 %, ~3.4 k tokens saved on
  every session load) while preserving every fact in at least one place.
  AGENTS.md gains a "Companion docs" header pointing at the new file
  alongside CLAUDE.md / MASTER_PLAN.md / TESTING.md, and the repo map
  now lists `docs/architecture-decisions.md`. No version bump (pure
  internal refactor; the next release will pick this up in its CHANGELOG).

## [3.13.1] - 2026-04-29

### Fixed
- **`release.yml` builds for `x86_64-pc-windows-msvc`,
  `x86_64-unknown-linux-gnu`, and `aarch64-unknown-linux-gnu` (task #54).**
  The auto-generated cargo-dist v0.31.0 release workflow was using each
  GitHub-hosted runner image's pre-installed rustc, which on those three
  targets is currently 1.94.1 — below the v3.11.1 MSRV bump to 1.95.
  `Cargo.toml`'s `rust-version = "1.95"` declaration was correctly
  rejecting the build with `error: rustc 1.94.1 is not supported`,
  causing 3/6 target builds to fail and no GitHub Release artifact to be
  published since v3.10.0. Adding a `rust-toolchain.toml` at repo root
  pinning `channel = "1.95"` plus `components = ["rustfmt", "clippy"]`
  makes rustup auto-install the right toolchain (with the components
  the `Format` and `Clippy` CI jobs require) on every runner before
  cargo runs, without needing to hand-edit the auto-generated release.yml
  or bump cargo-dist. CI on every commit also now uses the same pinned
  toolchain (rust-toolchain.toml overrides `dtolnay/rust-toolchain@stable`),
  giving a single source of truth between local builds, CI, and release
  builds. The `components` field is required because rustup honors
  rust-toolchain.toml in preference to action-level `components:` fields,
  and without it `cargo fmt` / `cargo clippy` fail with `'cargo-fmt' is
  not installed for the toolchain '1.95-x86_64-unknown-linux-gnu'`.

  This supersedes the MASTER_PLAN's earlier hypothesis that task #54 was
  a cargo-dist v0.31.0 installer-step regression on
  `x86_64-apple-darwin` + `x86_64-unknown-linux-musl` — that earlier
  failure mode (observed on the v3.10.0 release run) appears to have
  resolved itself in the runner images, but the MSRV mismatch took
  its place starting at v3.11.1. The two targets the plan said were
  broken now succeed; the three new failures all share the same root
  cause.

  **Outcome:** the v3.13.1 tag (`086ef0a`) triggered release.yml run
  25096833278, all 10 jobs succeeded across 6 build-local-artifacts
  targets + plan + build-global-artifacts + host + announce, and the
  GitHub Release was published with 20 assets — 6 platform binaries
  as `.tar.xz` + matching `.sha256`, Windows `.zip` + `.sha256` + `.msi`
  + `.msi.sha256`, source tarball + `.sha256`, `dist-manifest.json`,
  `sha256.sum`, plus the shell + PowerShell installer one-liners. This
  is the **first successful GitHub Release publication since v3.10.0**;
  the README installer one-liners (`irm .../tr-300-installer.ps1 | iex`
  and `curl ... | sh`) once again resolve to a current binary instead
  of `404` or stale v3.10.0.

  **Internal note (two-commit publication):** v3.13.1 was published as
  two commits — `c2e6a65` shipped only `channel = "1.95"` and tripped
  the `cargo-fmt is not installed` issue described above; `086ef0a`
  added `components = ["rustfmt", "clippy"]` and the tag points at
  `086ef0a`. The two-commit shape is preserved in the master history
  rather than rewritten to keep the lesson discoverable in
  `git log` / `git blame`.

## [3.13.0] - 2026-04-28

### Added
- **5-state battery awareness on Windows (C.10b — user-requested
  enhancement).** When the laptop is plugged in and fully topped up
  (≥ 95%, not actively charging), the BATTERY row now renders as just
  `AC Power` — the percentage is uninformative and adds noise. When
  plugged in and charging, `X% (Charging)`. When plugged in but battery
  is < 95% and not charging — common on Alienware / ROG / Razer with
  discrete GPUs whose peak draw exceeds the brick's wattage, AND on
  ThinkPad / ASUS / Lenovo with battery-longevity firmware modes that
  cap charge at 60-80% — `X% (Plugged in)` so the user can see the
  battery state through whichever path their system uses. Off AC, the
  classical `X% (Charging)` / `X% (Critical)` / `X% (Low)` /
  `X% (Discharging)` labels apply. AC-status-unknown edge case (rare —
  some VMs, hypervisor-passthrough batteries) renders as bare `X%`
  rather than a fabricated AC label. Replaces the v3.11.x output of
  `100% (AC Power)` (always showed % even when meaningless) and
  `100% (Discharging (High))` (legacy `BATTERY_FLAG_HIGH` bit
  surfaced as a confusing label). (C.10, C.10b)
- **PowerShell 7+ detection on Windows.** `get_shell()` now probes
  `HKLM\SOFTWARE\Microsoft\PowerShellCore\InstalledVersions\<GUID>\
  SemanticVersion` recursively before falling through to the legacy
  Windows PowerShell 5.x detection. Users running `pwsh` see the
  installed `PowerShell 7.x.y` version instead of `PowerShell 5.1.x`
  (or the bare `PowerShell` fallback). The string-compare for the
  highest version works for 3-tuple semver since each component fits
  in 2 digits. (C.11)
- **Terminal parent-process walk on Windows (C.12).** When neither
  `WT_SESSION` nor `TERM_PROGRAM` is set in the environment (common
  when launched from a desktop shortcut, a fresh subshell that lost
  the parent's environment, or by an AI agent), `get_terminal()` now
  walks the parent-process chain via `CreateToolhelp32Snapshot` (cap
  10 levels — defensive against PID-table cycles). Recognizes
  Windows Terminal, WezTerm, Alacritty, VS Code, Cursor, Windsurf,
  Hyper, Tabby, Ghostty, Kitty, MinTTY, Claude Code, and Antigravity.
  Intermediate hosts (`conhost.exe`, `powershell.exe`, `pwsh.exe`,
  `cmd.exe`, `bash.exe`, etc.) are skipped so the walk continues to
  the actual terminal emulator. Unrecognized exes break the walk
  silently and the terminal row falls back to `Console` as before.
  (C.12)

### Changed
- **Native socket count on Windows via `GetLogicalProcessorInformationEx`
  (C.9).** Replaces the WMI `Win32_Processor` count path (~30 ms
  COM round-trip) with a single Win32 API call (~3 ms). Walks the
  variable-length `SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX` records
  and counts `RelationProcessorPackage` entries via the standard
  two-call buffer-sizing pattern. WMI fallback retained for systems
  where the native call returns nothing. The historical
  `get_socket_count_wmi` function is kept (`#[allow(dead_code)]`)
  through one release so future debugging can A/B against it. (C.9)
- **Native battery on Windows via `GetSystemPowerStatus` (C.10).**
  Replaces the WMI `Win32_Battery` query (~40 ms) with a single
  Win32 API call (~1 ms). The 5-state output model above sits on
  top of this. WMI / PowerShell fallbacks retained. (C.10)
- **GPU enumeration prefers the registry path in full mode (C.8).**
  Historically, full mode used `Win32_VideoController` WMI which
  occasionally surfaced "Microsoft Basic Render Driver", "Microsoft
  Hyper-V Video", and similar software adapters alongside the real
  hardware. The registry path used by `--fast` (the
  `{4d36e968-e325-11ce-bfc1-08002be10318}` Display class under
  HKLM) only enumerates hardware adapters, so we now use it in full
  mode too. Added a name-based `filter_software_gpus()` belt-and-
  suspenders filter that strips known software-adapter strings
  (Microsoft Basic Render / Basic Display / Hyper-V Video / Remote
  Display Adapter / Indirect Display / RDPDD / RDP Encoder Mirror)
  in case a future config re-introduces them via either path. WMI /
  PowerShell remain as second/third fallbacks. Verified on a host
  with Intel Arc + NVIDIA RTX 4070 Laptop + Trigger 6 External
  Graphics — all three hardware GPUs appear, no software adapters.
  (C.8)

### Performance
- **`--fast` median wall-clock: 338 ms** on Windows 11 25H2 build
  26200.8246 (Intel Core Ultra 7 155H, sorted-7-runs middle). +30 ms
  vs the v3.11 baseline (~308 ms) — within the 100 ms budget per
  MASTER_PLAN §5 and well under the 1500 ms CI gate. The regression
  is from the new winapi features pulling additional bindings into
  the binary; collectors themselves are equal-or-faster (C.9 native
  sockets and C.10 native battery save ~70 ms in full mode but are
  not on the fast path).
- **C.14 (drop COM/WMI from `--fast`) — verified, no source
  change needed.** Audit confirms `--fast` returns from
  `platform/windows.rs::collect()` at the early-return on line 66
  *before* any `COMLibrary::new()` call. All COM-using paths
  (`get_bitlocker_status`, `get_socket_count_wmi` fallback,
  `get_network_info_wmi`, `last_cold_boot_seconds`) are reached only
  via full-mode code paths. The "drop COM from fast" outcome is
  already structurally true. (C.14)

### Deferred to a future session
- **E.6 — admin-only RDP login history** via Security Event 4624
  XML parsing. Adds `last_login_history: Option<Vec<String>>` field
  to `PlatformInfo` + `SystemInfo`, gated on `is_elevated()`, renders
  1–5 extra rows under `LAST LOGIN`. Deferred because validation
  requires running TR-300 from an elevated PowerShell on this host;
  the implementation is straightforward but should land with its
  validation evidence. Tracked as task #58.
- **C.13 — batched-PowerShell fallback** when WMI fails. Bundles the
  4 separate `_ps` fallbacks (`get_windows_edition_ps`,
  `detect_virtualization_ps`, `get_gpus_ps`, `get_battery_ps`) into a
  single `pwsh -NoProfile -Command @{…} | ConvertTo-Json` call,
  saving ~3× pwsh startup overhead in the WMI-failure branch.
  Deferred because the WMI-failure path is rarely exercised in
  practice (locked-down corporate hosts, Winmgmt service stopped),
  and validating without breaking the test host is awkward. Tracked
  as task #58.
- **Cross-platform 3-state battery model** for Linux + macOS to
  mirror Windows C.10b. Linux can land independently using
  `/sys/class/power_supply/AC*/online` + `BAT*/status`; macOS
  validation waits on PR #2 hardware. Tracked as task #56.
- **Full DXGI EnumAdapters1 implementation.** v3.13.0 took the
  shorter registry-prefer + name-filter approach (C.8 above) which
  achieves the same user-visible outcome (no software adapters)
  with 25 LOC vs ~100 LOC of unsafe COM code. DXGI remains an
  option if a future bug demands vendor/device-ID filtering that
  name-based filtering can't do.

### Internal
- `winapi` feature set extended further: `winbase` (for
  `GetSystemPowerStatus` + `SYSTEM_POWER_STATUS`), `errhandlingapi`
  (for `GetLastError`), `winnt` (for
  `RelationProcessorPackage` + the `LOGICAL_PROCESSOR_RELATIONSHIP`
  type), `tlhelp32` (for `CreateToolhelp32Snapshot` /
  `Process32FirstW` / `Process32NextW` / `PROCESSENTRY32W` /
  `TH32CS_SNAPPROCESS`), `processthreadsapi` (for
  `GetCurrentProcessId`), `handleapi` (for `CloseHandle` /
  `INVALID_HANDLE_VALUE`).
- `get_socket_count_wmi` marked `#[allow(dead_code)]` — retained as
  a fallback under the C.9 → C.14 transition; will be removed in a
  later release once native socket detection has soak-time on real
  hardware.
- New `filter_software_gpus(Vec<String>) -> Vec<String>` helper
  (case-insensitive substring match against a small static needle
  list) reused across all three GPU enumeration paths.
- New `get_powershell_core_version() -> Option<String>` helper —
  `reg query /s` recursive, picks highest `SemanticVersion` value
  by string compare.
- New `detect_terminal_via_parent_walk() -> Option<String>` and
  `match_terminal_name(&str) -> Option<&'static str>` helpers
  (~120 LOC including the Toolhelp snapshot walk and the static
  exe-name match table). Walk depth-capped at 10 levels to defend
  against malformed PID tables.
- Added `Cursor` and `Windsurf` env-var pre-checks
  (`CURSOR_TRACE_ID`/`CURSOR_AGENT`) before falling through to the
  parent walk — saves the snapshot syscall when the env-var path
  succeeds.
- v3.12.0's `chrono::DateTime<chrono::Utc>` field for parsing
  `LastBootUpTime` from WMI was the wrong type — the wmi crate's
  serde wrapper for CIM datetime is `wmi::WMIDateTime` (a newtype
  around `DateTime<FixedOffset>`). Fixed during v3.12.0
  development; this entry just notes that the surrounding code
  works as-is in v3.13.0.

## [3.12.0] - 2026-04-28

### Changed
- **VPN-aware Windows IP + DNS detection.** The Windows network collector
  now asks the kernel which interface index would carry traffic to the
  public internet — via `GetBestInterfaceEx` for `1.1.1.1` — and reorders
  the WMI `Win32_NetworkAdapterConfiguration` query results so the
  best-route adapter is picked first. On hosts running Tailscale,
  WireGuard, OpenVPN, Cisco AnyConnect, or any other tunnel that
  steals the default route, `MACHINE IP` and `DNS IP` rows now reflect
  the tunnel rather than a coin-flip pick from the available adapters.
  Falls back transparently to the pre-v3.12.0 first-IP-enabled-adapter
  behavior on hosts where the kernel route lookup fails (IP Helper
  service disabled, no default route). Reference:
  [GetBestInterfaceEx](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getbestinterfaceex)
  (C.4)
- **Windows Fast Startup uptime annotation.** When `HiberbootEnabled=1`
  in the registry AND the WMI cold-boot time (`Win32_OperatingSystem.
  LastBootUpTime`) diverges from the kernel session uptime by more than
  one hour, the `UPTIME` row now renders as
  `9d 4h 12m (session: 7h 14m)` — the long form is time since the last
  *cold* boot (what users mean by "since I really restarted") and the
  parenthetical is the current resumed-from-hibernation kernel session.
  Both values are correct and meaningful; surfacing both eliminates the
  "wait, I restarted three days ago, why does this say 47 days?"
  confusion that Fast Startup creates on laptops. Skipped in `--fast`
  mode (~80 ms WMI cost). Adds nullable `os.session_uptime_seconds`
  to JSON output (additive, no schema bump). Reference:
  [Microsoft Q&A: how to get OS start time when Fast Startup is enabled](https://learn.microsoft.com/en-us/answers/questions/1443763/how-to-get-oss-start-time-when-fast-startup-mode-i)
  (C.5)
- **CI: dropped `macos-13` (Intel macOS x86_64) from the `test` and `build`
  matrices.** The hosted runner pool is effectively retired — recent CI runs
  sat queued for 3+ hours (and one for 15h 50m) waiting exclusively on
  Intel macOS while every other matrix cell finished in minutes, forcing
  hand-cancellations on every push. Apple Silicon CI continues to exercise
  every line of `src/collectors/platform/macos.rs` because the platform
  cfg-gates aren't arch-specific. `cargo-dist`'s `release.yml` continues
  to ship `tr300-x86_64-apple-darwin.tar.xz` at every tag push, so any
  user on 2019/2020-era Intel hardware still gets a working binary
  download. CI never blocks on Intel; releases still produce the artifact.
  See `CLAUDE.md` § _Intel macOS coverage policy_ for the full rationale.
  (Originally `[Unreleased]`; rolled into v3.12.0.)

### Fixed
- **Endianness in `sin_addr` for `GetBestInterfaceEx` destination**
  caught during Codex (GPT-5.5) pre-commit review. The original draft
  used `u32::from_be_bytes([1,1,1,1])` for the IPv4 destination — which
  works coincidentally for the palindromic 1.1.1.1 because all bytes
  are equal, but on little-endian Windows the `sin_addr: u32` field
  stores its bytes in native (LE) order, meaning a non-palindromic
  destination (e.g. 8.4.4.8) would have routed to the wrong IP.
  Switched to `u32::from_le_bytes` with an inline comment documenting
  the network-byte-order requirement and why both byte-order choices
  happen to produce the same result for 1.1.1.1. Verified against
  [Microsoft `in_addr`](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/ns-winsock2-in_addr)
  docs (s_b1 = highest IP octet at offset 0).

### Internal
- New nullable `OsInfo.session_uptime_seconds: Option<u64>` field
  propagated through `SystemInfo`. Drives the parenthetical UPTIME
  annotation; reserved for future expansion (e.g. macOS / Linux
  session-vs-uptime nuance, should that prove worth surfacing).
- `SystemInfo::uptime_formatted()` refactored to delegate to a new
  module-private `format_duration_seconds(secs: u64) -> String`
  helper so the same compact "Nd Nh Nm" rendering can be reused
  for the parenthetical session-uptime suffix without duplicating
  the day/hour/minute decomposition.
- `os::collect()` now takes a `CollectMode` argument (was nullary)
  so the Windows Fast Startup WMI cold-boot query can be skipped in
  `--fast` mode without affecting cross-platform call sites. The
  callsite in `SystemInfo::collect_with_mode()` was updated to pass
  `mode` through to the spawned thread.
- `winapi` feature set extended to include `iphlpapi`, `ws2def`,
  `ws2ipdef`, `winerror`, `inaddr`, `in6addr`, `ifdef` for the
  `GetBestInterfaceEx` extern + `SOCKADDR_IN` declaration. Inline
  `#[repr(C)] struct SockaddrIn` matches the 16-byte Win32 layout
  (u16 sin_family, u16 sin_port, u32 sin_addr, [u8;8] sin_zero) so
  `GetBestInterfaceEx`'s `pDestAddr: sockaddr*` parameter receives a
  layout-compatible pointer.
- `Win32NetworkAdapterConfig` (the existing serde struct for
  `Win32_NetworkAdapterConfiguration` rows) gained an
  `interface_index: Option<u32>` field so the WMI query can be
  reordered by best-route ifindex.
- `last_cold_boot_seconds()` deserializes `LastBootUpTime` via
  `wmi::WMIDateTime` (a serde-aware newtype around
  `chrono::DateTime<FixedOffset>`). An earlier draft hand-parsed the
  CIM datetime format (`yyyymmddHHMMSS.mmmmmmsUUU`) — discarded after
  field-testing showed the wmi crate already converts the type at the
  COM boundary, so a raw `Option<String>` field can never see the
  format the parser expected.
- New integration test `test_json_includes_session_uptime_seconds_key`
  pins the JSON contract (key always present in `os` object, nullable
  per design — `null` on Windows when Fast Startup hibernation isn't
  active or hasn't diverged enough, and on every non-Windows platform).

## [3.11.1] - 2026-04-27

### Security
- **Migrated off the unmaintained `users` crate** to its maintained fork
  `uzers`. Clears three RustSec advisories that were flagged by `cargo audit`:
  RUSTSEC-2025-0040 (`root` appended to group listings — vulnerability with
  no upgrade available on the original crate), RUSTSEC-2023-0040
  (unmaintained), and RUSTSEC-2023-0059 (unaligned read of
  `*const *const c_char`). Drop-in API-compatible swap; the only callsite is
  `users::get_current_username()` → `uzers::get_current_username()` in
  `src/collectors/session.rs`. Unix-only dependency, so Windows is
  unaffected.

### Changed
- **MSRV bumped to Rust 1.95.0.** The GitHub Actions stable toolchain rolled to
  1.95.0 on 2026-04-14, which reclassified `std::arch::x86::__cpuid` /
  `std::arch::x86_64::__cpuid` as safe-to-call. We drop the `unsafe { … }`
  wrappers in `src/collectors/cpu.rs` and `src/collectors/platform/windows.rs`
  and pin `rust-version = "1.95"` in `Cargo.toml` so older toolchains get a
  clear error instead of a confusing E0133 build failure. Run
  `rustup update stable` before `cargo install tr-300`.
- **Self-update auto-refreshes Rust** when invoked via the cargo path
  (`tr300 --update` against a `cargo install`-placed binary). If `rustup` is
  on PATH we run `rustup update stable` first so the subsequent
  `cargo install tr-300 --force` always meets the current MSRV. Best-effort:
  no rustup → no-op, error → non-fatal. (`src/update.rs`)

### Fixed
- **CI green again on Rust 1.95.** The 1.95 toolchain promoted several lints
  to warnings that fail under our `RUSTFLAGS="-D warnings"` policy. Cleaned up
  15 sites: `clippy::collapsible_if` (×6 in `src/install/unix.rs`),
  `clippy::collapsible_match` (×1 in `src/install/windows.rs`),
  `unused_unsafe` (×4, `__cpuid` callsites), `unused_mut` (×3 in
  `src/collectors/os.rs` — refactored to `#[cfg(target_os = "windows")]`
  shadow), `clippy::unnecessary_lazy_evaluations` (×1 in
  `src/collectors/cpu.rs` — Windows-only `or_else` is now cfg-gated),
  `clippy::double_ended_iterator_last` (×1 in `src/collectors/network.rs`),
  `unused_variables` (×1 in `src/collectors/platform/linux.rs`), and
  `dead_code` (×3, gated `PS_INSTALLER` and `#[allow(dead_code)]` on two
  unused thin wrappers in `src/collectors/platform/macos.rs`). No behavior
  change.

## [3.11.0] - 2026-04-27

### Added
- **Windows BitLocker status** — new `ENCRYPTION` row in the report when readable
  on Win11 Device Encryption laptops. Renders as `BitLocker On (XTS-AES-256)` /
  `BitLocker Off`. Queries `Win32_EncryptableVolume` in the
  `ROOT\CIMV2\Security\MicrosoftVolumeEncryption` namespace. Try-and-degrade:
  works non-admin on most Win11 hosts, gracefully absent on older Win10 / domain
  configurations where the elevation footer hint covers the gap. JSON exposes
  this under `session.encryption`. (E.5 — user priority)
- **Windows last-login** now shows the actual session start instead of "Login
  tracking unavailable". Uses `WTSQuerySessionInformation(WTSLogonTime /
  WTSConnectTime)` for RDP / network logons, falling back to the boot time
  (derived from `GetTickCount64`) for local console sessions where Windows
  leaves the session timestamps at 0. (C.3)

### Changed
- **Windows OS detection** now reads `HKLM\SOFTWARE\Microsoft\Windows NT\
  CurrentVersion` directly. Detects Windows 11 by `CurrentBuild >= 22000` (the
  registry `ProductName` is frozen at "Windows 10" even on Win11) and enriches
  the version with `DisplayVersion` (e.g. `25H2`) and `UBR` (Update Build
  Revision) for kernel display like `26200.8246`. (C.1)
- **Windows architecture detection** via `IsWow64Process2`. Returns the host
  machine's architecture regardless of the running process's own architecture,
  so an x64-built binary running on a Surface Pro X correctly reports
  `aarch64 (x86_64 emulation)`. (C.2)
- **CPU frequency** on Windows now combines three sources: CPUID leaf 16h
  (silicon-rated boost on supported Intel CPUs), `CallNtPowerInformation`
  (active power-plan ceiling, including any per-core MaxMhz reflecting current
  performance state), and sysinfo's static base — using whichever is highest.
  CPUID leaf 16h is empty on Intel hybrid chips (Meteor Lake / Lunar Lake / Arrow
  Lake) where Intel zeroed it; CallNtPowerInformation reflects the user's active
  power plan, which is honest about throttling (Battery Saver users see the real
  ceiling, not the silicon max). (C.6)
- **Hypervisor detection** now reads CPUID leaf `0x40000000` (12-byte vendor
  string) for fast and reliable identification of KVM, Hyper-V, VMware,
  VirtualBox, Xen, QEMU, Parallels, ACRN, and bhyve. On Win11 with VBS
  (Virtualization-Based Security) enabled, where the kernel runs atop a thin
  Hyper-V layer on real hardware, this is disambiguated against SMBIOS
  manufacturer/model and reported as `Bare Metal (Hyper-V/VBS)` rather than
  the misleading `Hyper-V`. (C.7)

### Deferred to a follow-up PR
- C.4 (DNS+IP via `GetBestInterfaceEx` + `GetAdaptersAddresses` for VPN-aware
  default-route detection) and C.5 (Fast Startup uptime annotation via
  `HiberbootEnabled` registry + `GetTickCount64`) are split into PR #4b. The
  existing WMI-based network path and sysinfo uptime continue to work; these
  are accuracy refinements, not bug fixes.

### Internal
- New `encryption: Option<String>` field on `PlatformInfo` and `SystemInfo` —
  populated by Windows BitLocker query, reserved for future macOS FileVault
  (PR #2 A-block) and Linux LUKS (PR #3 B-block). All existing platforms
  initialize it to `None` so the cross-platform compile stays green.
- Added integration test `test_json_includes_encryption_key` (13 integration
  tests now passing).
- New manual-test matrix entries in `TESTING.md` under the v3.11.0
  verification log: live results from Windows 11 25H2 build 26200.8246
  (unelevated user session) for every changed row, plus the pending-verification
  list (Win11 ARM64, Win11 admin shell, Win11 with Device Encryption ON, real
  Hyper-V VM, KVM / VMware / VirtualBox guests).
- Per-PR documentation block (`F.1`–`F.5` from the development workflow) ran
  in full: CHANGELOG (this entry), README features list, CLAUDE.md "Windows
  accuracy patterns" arch notes with Microsoft Learn citations, Cargo.toml
  bumped 3.10.0 → 3.11.0 (minor — additive `encryption` JSON key), auto memory
  `project_tr300_overview.md` refreshed with the v3.11.0 deltas.

## [3.10.0] - 2026-04-27

### Added
- **Elevation tier scaffolding** — TR-300 now detects whether the current process is
  running with elevated privileges (Unix `euid == 0` / Windows admin token under UAC)
  and surfaces this in the JSON output and as a single-line hint at the bottom of the
  table on platforms where running with sudo/Administrator would unlock additional data
  points. The hint is shown only in full mode (never during `--fast` auto-run), on
  Linux (`Run with sudo for motherboard, BIOS, and RAM slot details`) and Windows
  (administrator BitLocker access). macOS shows no hint — there is no equivalent
  unlock on Apple platforms. (E.1, E.7)
- `--no-elevation-hint` flag to suppress the footer hint for users who find it noisy. (E.2)
- JSON `schema_version` field (initial value `1`) for forward-compatibility of
  programmatic consumers. Bumps only on breaking renames or removals; additive new keys
  do not require a bump. (D.1)
- JSON `elevated` and `elevation_unlocks_more` boolean keys. (E.8)

### Changed
- Library now exposes `tr_300::is_elevated()` and `tr_300::platform_has_elevated_data()`
  helpers for callers that want to drive their own elevation-aware UI on top of the
  collected `SystemInfo`.

### Internal
- Foundation for upcoming per-platform accuracy work (PRs #2–#5): macOS Apple Silicon
  CPU brand/frequency, Linux systemd-resolved DNS priority, Windows registry-based OS
  detection and BitLocker status, etc. No collector changes land in this release.
- **Comprehensive CI** — new `.github/workflows/ci.yml` runs on every push and PR with
  jobs for `fmt`, `clippy --all-targets --workspace -- -D warnings`, cross-platform
  `test` (Linux + macOS ARM + macOS Intel + Windows), release build smoke tests, an
  auto-run speed-budget gate (fails if `tr300 --fast` median > 1500 ms), `cargo audit`
  for dependency advisories, and a `dist plan` verification so cargo-dist regressions
  surface on PRs instead of at tag time. (CI.1, CI.3–CI.6)
- Migrated `tests/integration.rs` off the deprecated `assert_cmd::Command::cargo_bin`
  to the canonical `Command::new(env!("CARGO_BIN_EXE_tr300"))` pattern, plus added
  integration tests for the new `schema_version`, `elevated`, and
  `elevation_unlocks_more` JSON keys and the `--no-elevation-hint` / `--fast` footer
  gating. (CI.2)
- **Codified the development workflow** in `CLAUDE.md` (new "Development Workflow"
  section) and saved it as a project memory at
  `~/.claude/projects/.../memory/feedback_tr300_workflow.md`. Seven phases: plan in
  plan mode (parallel Explore + research) → upfront task tracking → sequential
  implementation → per-PR `F.1–F.6` documentation block → local gate + Codex review
  → `ci-tester` + `git-master` for push → close out. Apply for every non-trivial
  change.
- **Codex plugin enabled at project scope** via `.claude/settings.json`
  (`extraKnownMarketplaces.openai-codex` + `enabledPlugins.codex@openai-codex`) so
  cloners get the same Codex review subagent without manual setup. Added
  `.claude/settings.local.json` to `.gitignore` since it's per-machine state.

## [3.9.0] - 2026-04-12

### Added
- **Self-update command (`--update`)** — Check for and install the latest version
  directly from the command line. Automatically detects whether TR-300 was
  installed via `cargo install` or the shell/PowerShell installer and uses the
  appropriate update method. Supports `--json` output for scripted update checks.

### Fixed
- Shell installation now uses POSIX-compatible `case "$-"` syntax instead of
  bash-specific `[[ $- == *i* ]]`, fixing "command not found" errors on
  Raspberry Pi OS and other systems using dash/sh as the default shell

### Dependencies
- Added `ureq` for blocking HTTPS requests to GitHub releases API
- Added `serde_json` for parsing GitHub API responses

## [3.8.0] - 2026-03-21

### Added
- **Automatic UTF-8 locale detection with ASCII fallback** — TR-300 now checks the
  terminal's locale environment variables (`LC_ALL`, `LC_CTYPE`, `LANG`) at startup
  to determine whether the terminal supports UTF-8 encoding. If none of these variables
  indicate UTF-8 support (e.g., the locale is `C`, `POSIX`, or a non-UTF-8 encoding
  like `ISO-8859-1`), the tool automatically falls back to ASCII box-drawing characters
  (`+`, `-`, `|`, `#`, `.`) instead of Unicode (`┌`, `─`, `│`, `█`, `░`).
  - **Problem solved**: On systems like Raspberry Pi OS (Debian), the default locale is
    often `C` or `POSIX` rather than `en_US.UTF-8`. When TR-300 outputs 3-byte UTF-8
    box-drawing characters to a terminal expecting single-byte Latin-1/ISO-8859-1
    encoding, each character gets split into individual bytes and rendered as mojibake
    (garbled `â` sequences with broken table borders). This made the report completely
    unreadable on RPi, many headless Linux servers, Docker containers, minimal Debian
    installs, and SSH sessions where locale forwarding fails.
  - **How it works**: The detection checks environment variables in priority order:
    `LC_ALL` (highest override) → `LC_CTYPE` (character encoding specific) → `LANG`
    (general fallback). If the first non-empty, non-`C`/`POSIX` value contains "UTF-8"
    or "UTF8" (case-insensitive), Unicode mode is used. Otherwise, ASCII mode activates
    automatically. On Windows, UTF-8 is always assumed since the tool already calls
    `SetConsoleOutputCP(65001)` to enable UTF-8 console output.
  - **No behavior change for existing users**: Users with properly configured UTF-8
    locales (the vast majority of modern desktop Linux, macOS, and Windows systems)
    will continue to see the same Unicode table output as before.
  - **Manual override still works**: The `--ascii` flag continues to force ASCII mode
    regardless of locale detection, and users can also fix their locale with
    `export LANG=en_US.UTF-8` in their shell profile to get Unicode output.

## [3.7.0] - 2026-03-12

### Changed
- Upgraded cargo-dist from v0.30.3 to v0.31.0 (CI installer bug fixes, GitHub Actions updates)
- Table rendering now uses Unicode display width for correct alignment with CJK/emoji characters

### Fixed
- Fixed potential panic in macOS battery status when status string is empty
- Fixed thread join panics — collector failures now handled gracefully instead of crashing
- Fixed JSON output producing invalid JSON when system info contains control characters
- Fixed PowerShell legacy cleanup incorrectly counting braces inside comments

### Removed
- Removed dead code: unused `print_version()` function and `ReportBuilder` struct
- Consolidated duplicate `format_bytes()` into shared utility

### Added
- Man page generation via `clap_mangen` — build produces `man/tr300.1` automatically

### Dependencies
- Added `unicode-width` for correct Unicode display width calculation
- Added `clap_mangen` (build dependency) for man page generation

## [3.6.0] - 2026-02-22

### Added
- Auto-save markdown report to Downloads folder on manual full-mode runs
  - Generates a comprehensive `.md` file with all system info in table format
  - Only triggers on `tr300` / `report` (full mode, table output) — never on `--fast` or `--json`
  - Non-fatal: warnings go to stderr, terminal report always displays

### Dependencies
- Added `chrono` for timestamp formatting in markdown reports

## [3.5.0] - 2026-02-09

### Added
- GPU information now displays in `--fast` mode (previously skipped)
  - **Linux**: Uses existing `lspci` (~10-20ms) with `/sys/class/drm` fallback
  - **Windows**: New registry-based GPU detection via `DriverDesc` (~5-10ms, no WMI/PowerShell)
  - **macOS**: New `ioreg -rc IOGPUDevice` GPU detection (~20-40ms) with `sysctl` fallback for Apple Silicon

### Changed
- Auto-run reports (`tr300 --fast`) now include GPU rows

## [3.4.0] - 2026-02-09

### Added
- `--fast` CLI flag for sub-second startup in auto-run mode
  - Platform-aware skipping: Windows skips all WMI/PowerShell calls, macOS skips system_profiler, Linux skips almost nothing (already fast)
  - Auto-run installer now uses `tr300 --fast` for instant terminal startup
  - Manual `report` alias still runs full report
- WMI-based system information collection on Windows (replaces PowerShell subprocesses)
  - GPU, battery, Windows edition, virtualization, network info via direct WMI queries
  - CPU socket count via WMI Win32_Processor
  - PowerShell fallbacks for all WMI queries in case of WMI service issues
- Win32 API calls for display resolution (`GetSystemMetrics`) and locale (`GetUserDefaultLocaleName`)
- Registry-based PowerShell version detection (replaces spawning PowerShell subprocess)
- Parallel system information collection using `std::thread::scope`
  - All 7 collectors (OS, CPU, memory, disk, network, session, platform) run concurrently
  - CPU 200ms measurement sleep now overlaps with other collectors

### Changed
- Full report on Windows improved from ~5-7s to ~500ms (10x faster)
- Fast mode report on Windows completes in ~250ms (22x faster than original)
- macOS `system_profiler SPDisplaysDataType` called once instead of twice (GPU + resolution parsed from single call)
- Multiple `SystemInfo` fields changed to `Option<T>` for graceful fast-mode omission
- Report renderer conditionally omits rows when data was skipped in fast mode

### Dependencies
- Added `serde` (Windows only) for WMI query deserialization
- Extended `winapi` features: `sysinfoapi`, `winuser`, `winnls`

## [3.3.0] - 2026-02-03

### Added
- Interactive uninstall prompt with three options:
  1. Remove auto-run only - Removes shell profile modifications but keeps binary
  2. Uninstall TR300 entirely - Removes shell profile modifications AND the binary
  0. Cancel - Abort uninstall operation
- Complete uninstall feature removes:
  - Shell profile modifications (alias and auto-run)
  - The tr300 binary itself
  - Empty installation directory on Windows

### Changed
- `--uninstall` flag now shows interactive menu instead of immediately uninstalling
- Confirmation prompt required before complete uninstall to prevent accidental removal

## [3.2.0] - 2026-02-03

### Changed
- License changed from BSD 3-Clause to PolyForm Noncommercial 1.0.0
  - Permits noncommercial use, personal use, research, and hobby projects
  - Permits use by charitable organizations, educational institutions, public research organizations, and government agencies
  - Commercial use requires a separate license agreement

## [3.1.0] - 2026-02-03

### Added
- GPU information display - shows GPU name(s) in CPU section
  - Shows each GPU on separate row if ≤3 GPUs
  - Shows compact comma-separated list if >3 GPUs
- System architecture display (x86_64, aarch64, etc.) in OS section
- Shell name and version in session section
- Terminal emulator detection in session section
- System locale display in session section
- Battery status for laptops (percentage and charging state)

### Fixed
- DNS parsing bug on Windows - DNS servers now display on separate rows instead of being concatenated with literal `\n` string

### Changed
- Simplified table footer to single line (removed double-border)
- JSON output now includes all new fields (architecture, gpus, shell, terminal, locale, battery)

## [3.0.3] - 2026-02-03

### Changed
- Rebranded from "SHAUGHNESSY V DEVELOPMENT INC." to "QUBETX DEVELOPER TOOLS"

## [3.0.2] - 2026-02-03

### Fixed
- Fixed incorrect repository URL in Cargo.toml causing installer 404 errors
- Fixed MSI help link pointing to wrong repository

## [3.0.1] - 2026-02-03

### Added
- Running `--install` multiple times is now idempotent (removes existing TR-300 config and reinstalls fresh)

### Changed
- Installation no longer returns "already configured" - it always cleans and reinstalls to ensure consistency

## [3.0.0] - 2026-02-03

### Added
- Complete rewrite in Rust for performance and reliability
- Cross-platform support (Windows, macOS, Linux)
- Unicode box-drawing table rendering with multiple styles (single, double, rounded)
- Color-coded progress bars for resource usage
- CPU information with per-core usage tracking
- Memory and swap usage with visual indicators
- Disk/volume information with usage bars
- Network interface statistics
- Session and user information
- Platform-specific collectors for extended information
- Self-installation commands (`tr300 install` / `tr300 uninstall`)
- Configurable output width and compact mode
- Command-line flags for hiding specific sections
- cargo-dist integration for automated cross-platform releases
- Shell installer for macOS/Linux
- PowerShell installer for Windows
- NSIS installer for Windows GUI installation

### Changed
- Binary name set to `tr300`
- Output format completely redesigned with modern Unicode tables

### Removed
- Shell script implementation (replaced by native Rust)
- External dependencies on system commands

### Fixed
- Consistent cross-platform behavior
- Proper Unicode rendering on all supported terminals
- Accurate memory and disk calculations

### Security
- No external command execution for core functionality
- Safe handling of filesystem paths
