# TR-300 Master Plan

> **Pickup-ready handoff document.** Read this first on a fresh session, fresh
> machine, or fresh contributor. Tells you exactly what's shipped, what's
> pending, why each decision was made, and how to keep going without
> re-litigating.

**Last updated:** 2026-06-03 (v3.16.0 — stability & cross-platform hardening pass)
**Current version:** 3.16.0
**Repo:** github.com/QubeTX/qube-machine-report
**Local source of truth:** `C:\Users\hey\git\qube-machine-report` (Windows host where this work was authored)

---

## 0. Quick orientation for a new session

If you just pulled this repo and want to keep building, read these in order:

1. **`MASTER_PLAN.md`** (this file) — what's shipped, what's pending, where to pick up.
2. **`CLAUDE.md`** § "Development Workflow" — the canonical 7-phase cadence (plan → tasks → implement → docs → verify → commit/push → close out).
3. **`CLAUDE.md`** § "CI" — what `.github/workflows/ci.yml` enforces and how to reproduce it locally.
4. **`TESTING.md`** — the manual cross-platform verification matrix and per-version verification log.
5. **`CHANGELOG.md`** — every shipped change with task-ID cross-references in parens.

The auto-memory at `~/.claude/projects/C--Users-hey-git-qube-machine-report/memory/` (only valid on the original Windows host) duplicates the workflow notes. Don't recreate it on other machines — `CLAUDE.md` is authoritative.

---

## 1. Status snapshot

### Shipped (committed + pushed to `origin/master`)

| Tag | Commit | Date | Summary |
|---|---|---|---|
| v3.10.0 | `58812cc` | 2026-04-27 | **Foundation** — elevation tier (`is_elevated`, `--no-elevation-hint`, footer hint, JSON `schema_version` + `elevated` + `elevation_unlocks_more`), comprehensive CI (`.github/workflows/ci.yml` with cross-platform fmt/clippy/test/build/speed/audit/dist-plan), Codex plugin config at project scope (`.claude/settings.json`), 7-phase development workflow codified in CLAUDE.md, new `TESTING.md` with manual matrix |
| v3.11.0 | `3a252df` | 2026-04-27 | **Windows accuracy + BitLocker** — registry-based OS detection (Win11 by build ≥ 22000, DisplayVersion + UBR), `IsWow64Process2` arch, `WTSQuerySessionInformation` last-login with `GetTickCount64` boot-time fallback, CPUID leaf 16h + `CallNtPowerInformation` CPU frequency, CPUID leaf `0x40000000` hypervisor brand with VBS disambiguation, BitLocker via `Win32_EncryptableVolume` in `MicrosoftVolumeEncryption` namespace |
| v3.11.1 | `22f2002` | 2026-04-27 | **Rust 1.95 MSRV + auto-rustup self-update + uzers migration** — pin `rust-version = "1.95"` after `__cpuid` reclassified as safe-to-call, drop `unsafe { … }` wrappers; `tr300 --update` runs `rustup update stable` first when the binary was placed via `cargo install` (best-effort, silent if rustup absent); migrate `users` → `uzers` to clear RUSTSEC-2025-0040 / 2023-0040 / 2023-0059 |
| (untagged) | `24fdc60` | 2026-04-27 | RUSTSEC audit fix folded into v3.11.1 (no separate tag needed) |
| (untagged) | `14e0d97` | 2026-04-28 | CI: dropped `macos-13` (Intel macOS) from test+build matrices; rolled into v3.12.0 changelog |
| v3.12.0 | `28bda98` | 2026-04-28 | **Windows accuracy refinements (PR #4b)** — VPN-aware default-route detection via `GetBestInterfaceEx` (the WMI adapter list is reordered so the kernel's preferred-route adapter wins, correct on multi-homed/VPN configs), Fast Startup uptime annotation (`HiberbootEnabled` + WMI `LastBootUpTime` divergence > 1h → `UPTIME` row renders `9d 4h 12m (session: 7h 14m)`), nullable `os.session_uptime_seconds` JSON key, `os::collect()` takes `CollectMode` to gate the Fast Startup WMI cost, `wmi::WMIDateTime` for CIM datetime parsing |
| v3.13.0 | `f34e981` | 2026-04-28 | **Windows polish (PR #5 partial)** — native battery via `GetSystemPowerStatus` with 5-state output model (AC Power / X% Charging / X% Plugged in / X% Discharging / Critical / Low / Unknown — "Plugged in" covers gaming-laptop PSU-undersized AND firmware battery-longevity modes), native socket count via `GetLogicalProcessorInformationEx` (alignment-safe walk via `from_le_bytes`), GPU registry-prefer + `filter_software_gpus()` name filter, PowerShell 7+ detection via `PowerShellCore` registry hive (semver tuple comparison, not string sort), terminal parent-process walk via Toolhelp32 (recognizes Windows Terminal, WezTerm, Alacritty, VS Code, Cursor, Windsurf, Hyper, Tabby, Ghostty, Kitty, MinTTY, Claude Code, Antigravity); E.6 admin RDP login history and C.13 batched-PowerShell fallback were still open at this tag. C.13 later shipped in v3.14.0; task #58 tracks E.6. |
| v3.13.1 | `086ef0a` | 2026-04-29 | **Release infrastructure fix (task #54)** — adds `rust-toolchain.toml` at repo root pinning `channel = "1.95"` AND `components = ["rustfmt", "clippy"]`. Resolves `release.yml` failures on `x86_64-pc-windows-msvc` + `x86_64-unknown-linux-gnu` + `aarch64-unknown-linux-gnu` runners that shipped with rustc 1.94.1 (below MSRV 1.95 declared in v3.11.1). The auto-generated cargo-dist v0.31.0 release workflow has no rustup setup step; pinning at the workspace level lets rustup auto-install the right toolchain before cargo runs. The components addition was a fix-forward (`086ef0a` superseded `c2e6a65`) — when rustup honors a rust-toolchain.toml it ignores action-level `components:` fields, so listing rustfmt/clippy in the file is required to keep ci.yml's Format + Clippy jobs working. **All 10 release.yml jobs green; v3.13.1 GitHub Release published with 20 assets** (6 platform binaries + MSI + source tarball + shell/PowerShell installers). First successful Release publication since v3.10.0. |
| v3.14.0 | `54dbae1` | 2026-05-10 | **Cross-platform stability + action syntax** — adds positional actions (`tr300 update/install/uninstall`, inherited by the `report` alias), bounded collector subprocess helpers, conditional model/core-topology/motherboard/BIOS/RAM/ZFS rows, additive nullable JSON keys, macOS/Linux accuracy improvements, Windows batched PowerShell WMI-failure fallback, fixed-width/JSON/markdown hardening, and documentation cleanup that removes unimplemented Windows RDP-history promises. |
| v3.14.1 | `3328a8e` | 2026-05-11 | **Release confidence patch** — no new runtime behavior; bumps package metadata for a patch release after the v3.14.0 CI warning-as-error fix-forward and follow-up release-publication docs were verified green on `master`. |
| v3.14.2 | `a6c3841` | 2026-05-11 | **Crates.io + resilient updater release** — publishes the `tr-300` crate, tracks `Cargo.lock`, adds CI-gated crates.io publishing after default-branch CI, ports self-update to a cargo-first probe-and-retry strategy chain, documents all install paths, and removes unrelated historical implementation files/references. |
| v3.14.3 | `25305d8` | 2026-05-11 | **Canonical crates.io package name** — recreates the crate as lowercase `tr300`, changes the Rust library import path to `tr300`, points self-update at `cargo install tr300 --force`, and keeps `tr-300-installer.*` release aliases for v3.14.2 updater compatibility via a cargo-dist `allow-dirty = ["ci"]` workflow customization. |
| v3.14.4 | `ac3fd34` | 2026-05-14 | **Windows install execution-policy preflight** — `tr300 install` now adjusts the Windows PowerShell `CurrentUser` execution policy to `RemoteSigned` when it's `Restricted`/`Undefined`, so the freshly written `$PROFILE` auto-run actually loads on fresh Windows machines. `AllSigned` is intentionally left alone (user warning, no silent downgrade). Verify-after-set catches GPO overrides and surfaces a `LocalMachine`-scope remediation. Non-fatal: the alias write half always succeeds. Drive-by cleanup: moved `mod powershell_fallback_tests` to the end of `src/collectors/platform/windows.rs` to satisfy `clippy::items_after_test_module` under local Windows clippy. |
| v3.14.5 | `a21a4d1` | 2026-05-14 | **Windows install error advisor** — when `tr300 install`/`uninstall` fails to create the profile directory, read/write the profile, or remove the binary, `fail_install(InstallStep, &Path, io::Error)` now streams a multi-paragraph advisory to stderr explaining the likely cause (OneDrive sync state, Intune/AD/AppLocker/WDAC restriction, antivirus block, sharing violation, storage-full, MAX_PATH overflow) with concrete remediation (path to allowlist, antivirus exclusion, `takeown` example). Dispatch keys: `(InstallStep, ErrorKind, raw_os_error, path-onedrive-vs-redirected-vs-local)`. The rich advisory goes to stderr before `AppError` is returned, so it surfaces regardless of how callers render the final summary line. Includes a temporary `continue-on-error: ${{ matrix.os == 'macos-latest' }}` on the ci.yml test/build/speed matrix jobs to work around GitHub hosted-runner intermittent failures on macOS ARM (see the "Revert macOS-ARM continue-on-error" TODO entry above). |
| v3.15.0 | `8e98db4` | 2026-05-14 | **Four-installer Windows distribution model + MSI/EXE-aware self-update.** Adds three new Windows installer artifacts to every release: Corporate MSI (initially `wix/corporate.wxs`, moved to `wix-corporate/corporate.wxs` in v3.15.1; perUser scope, no UAC, installs to `%LocalAppData%\Programs\tr300\bin\`), Global EXE installer (`inno/global.iss`, Inno Setup, perMachine, same path as Global MSI), and Corporate EXE installer (`inno/corporate.iss`, Inno Setup, perUser, same path as Corporate MSI). Six new permanent GUIDs total (two MSI UpgradeCodes, two Inno AppIds, two MSI registry-marker Component GUIDs). All four installers write a `HKCU\Software\TR300\InstallSource` registry value (`msi-global` / `msi-corporate` / `exe-global` / `exe-corporate`) which `src/update.rs::detect_install_origin()` reads to dispatch `tr300 update` to the matching installer for in-place upgrades — MSI strategies use `msiexec /i /passive /norestart`, EXE strategies use `setup.exe /SILENT /SUPPRESSMSGBOXES /NORESTART`. Path-based fallback (Program Files → MsiGlobal, LocalAppData\Programs → MsiCorporate, .cargo\bin → CargoOrInstaller) handles legacy installs without the marker. New `.github/workflows/windows-installers.yml` (triggered on `release: types: [published]`) builds the three new artifacts after cargo-dist's `release.yml` finishes; total release asset count is 28. New `winreg = "0.52"` dependency for the registry read. README leads with the four direct-download installer links by edition, demotes `cargo install` to a "Build from source" subsection. **Released to crates.io but `release.yml` failed at the Windows MSI build (see below); no GitHub Release artifacts.** Superseded by v3.15.1. Long-form rationale in `docs/architecture-decisions.md` § "Windows distribution model (v3.15.0+)" + v3.15.1 addendum. |
| v3.15.2 | (this commit) | 2026-05-18 | **Cross-platform audit + remediation.** A three-agent read-only audit (Explore agents, Opus 4.7 1M context) of TR-300's install / update / runtime paths across macOS, Linux, and Windows surfaced 22 real findings; this release ships fixes for 19. Plan file at `~/.claude/plans/can-you-go-through-whimsical-hammock.md`. **Security:** SHA256 verification of every downloaded MSI / EXE installer before launch (F3) — defends against TLS-interception proxies, captive portals, and CDN tampering; without this a corporate IT MITM could substitute a trojaned installer the update flow would then run with elevated permissions. **Install/uninstall hygiene (the largest cluster):** atomic temp-then-rename writes to rc files / `$PROFILE` so a Ctrl-C or power loss can no longer truncate `~/.bashrc` (F1, plus `.tr300-backup` one-time copy of the original); marker-balance pre-check refuses to mutate a hand-mutilated profile that would otherwise have the parser silently drop everything below `# TR-300 Machine Report` to EOF (F2); auto-run snippet now guards with `command -v tr300` / `Get-Command tr300 -ErrorAction SilentlyContinue` so a missing binary no longer spams every new shell with "command not found" (F4); `TR300_AUTORUN_RAN` recursion sentinel breaks nested-shell render loops (F4 follow-on); Windows snippet uses `[Environment]::UserInteractive` instead of `$Host.Name -eq 'ConsoleHost'` so `pwsh -Command "..."` from CI / VS Code / scheduled tasks doesn't dump the table into log streams (F4 follow-on); `tr300 install` writes to BOTH Windows PowerShell 5.1 AND PowerShell 7 profile paths when both shells are present, fixing the silent no-op for pwsh-only users (F5); shared marker constants + parser extracted to `src/install/shared.rs` so future renames touch one file (F16); `sudo tr300 install` is refused on Unix with an actionable message (F11); macOS prefers `.zshrc` over `.bashrc` on fresh accounts (F12); Windows `uninstall -> Complete` from `%LocalAppData%\Programs\tr300\bin\tr300.exe` schedules a detached cleanup job rather than failing on the locked running EXE (F13); Inno `EnvRemovePath` first-entry off-by-one bug fixed in both `inno/global.iss` and `inno/corporate.iss` (F9). **Update reliability:** `is_newer` semver parser correctly handles prereleases and build-metadata (F7); `try_msi_install` / `try_exe_install` re-exec `--version` after success and surface msiexec 3010 (REBOOT_REQUIRED) as actionable text rather than false-positive success (F8). **Runtime:** WMI calls in `get_bitlocker_status` / `get_socket_count_wmi` / `get_network_info_wmi` / `last_cold_boot_seconds` now have a 5-second Rust-side timeout via a new `with_timeout` thread + `mpsc::channel.recv_timeout` wrapper so a hung WMI provider can't stall the full-mode report for tens of seconds (F14); `escape_json` delegates to `serde_json::to_string` instead of hand-roll (F15); `aggregate_disk_usage` `C:\` match tightened against junction-mounted paths (F18); `lscpu` / `lastlog` / `last` subprocesses forced to `LC_ALL=C` via new `run_stdout_c_locale` / `run_output_with_env` so non-English locales don't silently break label-matching parsers (F19). **Locale handling:** `is_utf8_locale()` reads POSIX precedence strictly (F6); `enable_utf8_console()` on Windows now restores the prior console code page on Drop instead of leaving the console in UTF-8 mode after exit (F10). **New dep:** `sha2 = "0.10"` (Windows-only). **Tests:** 26 new unit tests across `install`, `update`, and `report`; lib test count 72 → 98. **Deferred to future:** F17 (alias-collision warning), F20 (workflow_run pre-flight asset check in windows-installers.yml), F22 (OnceLock<COMLibrary> caching for graphical-host library consumers). See `CHANGELOG.md [3.15.2]` for the per-finding detail. |
| v3.15.3 | (this commit) | 2026-05-23 | **Deferred-audit-findings follow-up.** Resolves the three v3.15.2 audit findings that were explicitly deferred: F17 (alias-collision warning at install time — read-only heuristic scan of `~/.bashrc` / `~/.bash_profile` / `~/.bash_aliases` / `~/.zshrc` / `~/.zprofile` / `~/.profile` on Unix and each `$PROFILE` on Windows for pre-existing `alias report=` / `function report` / `Set-Alias` / `New-Alias` declarations + executable PATH scan, prints a one-time stderr note showing each conflict so the user knows the install is about to shadow); F20 (pre-flight asset check in `.github/workflows/windows-installers.yml` — probes `dist-manifest.json` + Global MSI via `gh release view --json assets` before building installers, fails fast on a torn upstream `release.yml` so we don't attach 6 add-on assets to an incomplete release); F22 (COM init mode conflict for library consumers — the main `collect()` WMI block now runs inside `with_timeout(WMI_BATCH_TIMEOUT, ...)` on a fresh worker thread, same pattern as the BitLocker / socket-count / network-info / cold-boot WMI sites already use, so a Tauri/Slint/winit GUI host whose own thread initialized COM as `COINIT_APARTMENTTHREADED` no longer causes `RPC_E_CHANGED_MODE` and silent fallback to the slow PowerShell path; cheap main-thread probes `get_gpus_fast` / `get_battery_native` still run first to preserve the pre-F22 happy-path latency). Also vendors four Anthropic-distributed agent skills into `.claude/skills/` (`brainstorming`, `critical-thinking`, `architecture`, `system-design`) so every Claude Code agent gets the same thinking toolkit regardless of contributor plugin config; see `.claude/skills/ATTRIBUTION.md`. Patch bump — accuracy/resilience fixes, no new flags or fields. All audit work from the May 2026 cycle is now landed. |
| v3.15.1 | `b37e783` | 2026-05-15 | **Patch — fix-forward of v3.15.0 release.yml WiX build failure.** v3.15.0 release.yml run 25901237669 failed at `build-local-artifacts(x86_64-pc-windows-msvc)` with WiX candle exit code 6. Two root causes diagnosed locally via portable WiX 3.11 binaries: (1) `wix/corporate.wxs` had `<Property Id='ALLUSERS' Value=''/>` which WiX 3.11 candle rejects with `CNDL0006` — the Property and its redundant `MSIINSTALLPERUSER=1` sibling are now removed because `InstallScope='perUser'` on the Package element alone is sufficient; (2) cargo-wix's default behavior compiles ALL `.wxs` in `wix/` and links them into ONE MSI, hitting LGHT0089/LGHT0091 link errors when two complete Product definitions coexist — fixed by moving the Corporate template to a new directory `wix-corporate/corporate.wxs` so cargo-wix's `wix/` scan only sees `main.wxs`. The Corporate MSI is now built by `.github/workflows/windows-installers.yml` via bare `candle.exe` + `light.exe -sice:ICE38 -sice:ICE64 -sice:ICE91` (the ICE suppressions are cosmetic — install/uninstall work; only consequence is empty `%LocalAppData%\Programs\tr300\bin\` after uninstall). Two more CI-infra fix-forwards on master after the release commit: (3) `7715b93` switched `windows-installers.yml` from `release: published` to `workflow_run` because GitHub suppresses downstream release events triggered by GITHUB_TOKEN (cargo-dist uses GITHUB_TOKEN); (4) `5883627` quoted PowerShell `-D` define args because the bareword tokenizer split `-dVersion=3.15.1` at the dots. No runtime/collector/renderer changes from v3.15.0. Long-form rationale in `docs/architecture-decisions.md` § "v3.15.1 addendum". v3.15.0 stays in git as the immutable record of the failed release attempt; this is the same fix-forward-by-patch pattern as v3.13.0 → v3.13.1. |

**Tag status (as of 2026-05-23):**
- `v3.10.0` (`58812cc`): tagged + pushed; release.yml run failed (different failure mode — historic record only).
- `v3.11.0` (`3a252df`): NOT tagged.
- `v3.11.1` (`22f2002`): NOT tagged.
- `v3.12.0` (`28bda98`): tagged + pushed; release.yml run failed (3/6 targets failed with `error: rustc 1.94.1 is not supported by the following packages: tr-300@3.12.0 requires rustc 1.95`); no GitHub Release artifact published.
- `v3.13.0` (`f34e981`): tagged + pushed; release.yml run failed identically (same rustc 1.94.1 < MSRV 1.95 mismatch on 3/6 targets); no GitHub Release artifact published. Run 25039719372.
- `v3.13.1` (`086ef0a`): tagged + pushed; all 10 release.yml jobs succeeded (6 build-local-artifacts + plan + build-global-artifacts + host + announce); GitHub Release published with the standard 6 binaries + Windows MSI + shell/PowerShell installer one-liners + source tarball. README installer one-liner now functional for the first time since v3.10.0.
- `v3.14.0` (`54dbae1`): tagged + pushed; CI run 25642712712 succeeded across fmt/clippy/test/build/audit/dist-plan/speed gates; release.yml run 25642853066 succeeded across plan + six target artifact builds + global artifacts + host + announce; GitHub Release published with 20 assets.
- `v3.14.1` (`3328a8e`): tagged + pushed; CI run 25645894617 succeeded across fmt/clippy/test/build/audit/dist-plan/speed gates; release.yml run 25645999755 succeeded across plan + six target artifact builds + global artifacts + host + announce; GitHub Release published with 20 assets.
- `v3.14.2` (`a6c3841`): tagged + pushed; CI run 25647466576 succeeded across fmt/clippy/test/build/audit/dist-plan/speed gates; crates-publish run 25647553585 published `tr-300` 3.14.2 to crates.io after rerunning fmt/clippy/tests/package/dry-run; release.yml run 25647597021 succeeded across plan + six target artifact builds + global artifacts + host + announce; GitHub Release published with 20 assets.
- `v3.14.3` (`25305d8`): tagged + pushed; CI run 25648618096 succeeded across fmt/clippy/test/build/audit/dist-plan/speed gates; crates-publish run 25648707510 published `tr300` 3.14.3 to crates.io after rerunning fmt/clippy/tests/package/dry-run; release.yml run 25648740343 succeeded across plan + six target artifact builds + global artifacts + host + announce; GitHub Release published with 22 assets, including canonical `tr300-installer.*` installers and `tr-300-installer.*` compatibility aliases. `tr300` returned 404 from the crates.io API before release, so this version recreated the package under the corrected lowercase package name. The previous `tr-300` package name is treated as a legacy updater-compatibility concern only.
- `v3.14.4` (`ac3fd34`): tagged + pushed; CI run 25848439537 succeeded across all 13 jobs (fmt + clippy + audit + dist-plan + tests on Linux/macOS ARM/Windows + release builds on the same three + auto-run speed gates on all three); crates-publish run 25848562250 published `tr300` 3.14.4 to crates.io from the exact CI-tested SHA; release.yml run 25848716551 succeeded across plan + six target artifact builds + global artifacts + host + announce; GitHub Release published with 22 assets (canonical `tr300-installer.*` + legacy `tr-300-installer.*` aliases + six platform archives + MSI + checksums + source tarball + dist-manifest). Self-update probe (`tr300 update --json`) from a local v3.14.4 binary confirmed `latest_version=3.14.4` and `update_available=false`. crates.io API confirmed `newest_version=3.14.4`. Fixes the original user-reported `UnauthorizedAccess` PSSecurityException on fresh Windows machines after `tr300 install`.
- `v3.14.5` (`a21a4d1`): tagged + pushed after six commits of CI churn (release commit `06161b2` + retriggers `ff199d5` / `be8f2a0` + main.rs revert `158dc2e` + ci.yml fixes `667e466` / `a21a4d1`) needed to work around GitHub hosted-runner intermittent failures on macOS ARM. Final CI run 25850693664 succeeded with workflow conclusion `success`; macOS ARM test + auto-run-speed matrix entries reported individual `failure` but were non-blocking via `continue-on-error: ${{ matrix.os == 'macos-latest' }}`. Linux + Windows entries + non-matrix jobs all gated normally. crates-publish run 25850823118 published `tr300` 3.14.5 to crates.io from the exact CI-tested SHA. release.yml run 25850864213 succeeded — cargo-dist built all six target binaries (Linux x64 gnu/musl, Linux ARM64 gnu, macOS Intel, macOS ARM, Windows x64) plus MSI, both installer scripts, both legacy aliases, sha256 sums, source tarball, and dist-manifest. GitHub Release published non-draft non-prerelease with 22 assets. `tr300 update --json` from a local v3.14.5 binary reports `latest_version=3.14.5` / `update_available=false`; crates.io API confirms `newest_version=3.14.5`. Six-commit churn was entirely driven by macOS ARM intermittent failures (cargo aborting at 0 seconds with no compile output across multiple identical-source retries); v3.14.4 on a structurally similar tree passed cleanly hours earlier on the same workflow, so the failures are unambiguously infrastructure not code. The `continue-on-error` knob is tracked in the pending-section as a TODO to revert in the first stable release after macOS ARM recovers.
- `v3.15.0` (`8e98db4`): tagged + pushed AFTER the v3.15.0 release commit (squash-merged into `92456a9`) and the `8e98db4` CI fix-forward (non-Windows `dead_code` + `allow-dirty=["ci","msi"]`). master CI run 25901108698 succeeded across all gates. crates-publish run 25901198519 published `tr300` 3.15.0 to crates.io at 05:00:57 UTC. release.yml run 25901237669 then **FAILED** at `build-local-artifacts(x86_64-pc-windows-msvc)` with WiX candle exit 6. Five of six platform binary builds succeeded; the Windows MSI build aborted, which cascaded to `build-global-artifacts` / `host` / `announce` being skipped. **No GitHub Release artifacts were created for v3.15.0** — the tag remains in git as the immutable record of the failure. `cargo install tr300` from crates.io DOES install 3.15.0 successfully (the crate is correct; only the Windows MSI packaging step blew up). Superseded by v3.15.1.
- `v3.15.1` (`b37e783`): tagged + pushed. master CI run 25902114185 succeeded. crates-publish run 25902206318 published `tr300` 3.15.1 to crates.io at 05:35:19 UTC. release.yml run 25902253637 then succeeded — all 10 jobs green including the Windows MSI build that failed for v3.15.0. GitHub Release v3.15.1 published 05:41:10 UTC with the initial 22 cargo-dist assets. `windows-installers.yml` did NOT fire automatically (GitHub's GITHUB_TOKEN downstream-suppression rule blocks `release: published` events from cargo-dist's `gh release create`); fixed in commit `7715b93` by switching the trigger to `workflow_run` + `workflow_dispatch`. First workflow_dispatch retry (run 25902607841) failed on `CNDL0103` because PowerShell tokenized `-dVersion=3.15.1` at the dots; fixed in `5883627` by quoting the `-D` args. Second workflow_dispatch retry (run 25902740025) succeeded and uploaded the 6 additional Windows installer assets (Corporate MSI + Global EXE setup + Corporate EXE setup + their `.sha256` sidecars). **Final v3.15.1 GitHub Release: 28 assets** verified via `gh release view v3.15.1`. The full four-installer Windows distribution model is now live.
- `v3.15.3` (`e030baa`): tagged + pushed after a clean release commit. Master CI run 26342102096 succeeded across all 13 gates. crates-publish run 26342172080 published `tr300` 3.15.3 to crates.io at 2026-05-23T20:01Z. release.yml run 26342193045 succeeded across all 10 jobs (plan + 6 build-local-artifacts + global + host + announce). windows-installers.yml run 26342293613 (triggered via workflow_run after release.yml completion) **FAILED** at the new F20 pre-flight step — the `gh release view` call had no git context because the step runs BEFORE actions/checkout. Fix-forward commit `a70ed50` adds `--repo ${{ github.repository }}` so `gh` works pre-checkout. CI run 26342326674 on the fix commit passed all 13 gates. workflow_dispatch retry of windows-installers.yml (run 26342329480) then succeeded with the master-resident fix and uploaded the 6 additional installer assets (Corporate MSI + Global EXE setup + Corporate EXE setup + 3 `.sha256` sidecars). **Final v3.15.3 GitHub Release: 28 assets** verified via `gh release view v3.15.3`, published 2026-05-23T20:05:52Z. All three deferred v3.15.2 audit findings (F17 alias-collision warning, F20 windows-installers.yml pre-flight, F22 COM init mode conflict for library consumers) are now resolved. The F20 fix-forward itself was caught BY the F20 check on its first real production firing — exactly the scenario the check was designed to catch (a torn-release-like state caused this time by a self-inflicted pre-checkout `gh` invocation rather than an actual cargo-dist failure).
- `v3.15.2` (`0d975e2`): tagged + pushed AFTER the release commit (`c1f9d52`) failed its CI run 26020553352 with a `unused_imports` clippy error on `run_stdout` / `run_output` in macOS-cfg-gated `src/collectors/cpu.rs:4` and `src/collectors/session.rs:4` import lines — the v3.15.2 audit migration moved Linux callsites to `run_stdout_c_locale` / `run_output_with_env` (finding F19) but left the macOS-only callsites unaccompanied, so under `-D warnings` on Linux these imports went unused. Fix-forward commit `0d975e2` split the imports: `CommandTimeout` stays gated to Linux+macOS, but `run_stdout` / `run_output` are now gated to `target_os = "macos"` only. Master CI run 26020803537 (`0d975e2`) passed all 13 gates green. crates-publish run 26020958768 published `tr300` 3.15.2 to crates.io at 07:59Z. release.yml run 26021005159 succeeded across all 10 jobs (plan + 6 build-local-artifacts + global + host + announce). windows-installers.yml run 26021208515 (triggered via workflow_run after release.yml completed — same chain that worked from v3.15.1 onward) added the 6 additional installer assets (Corporate MSI + Global EXE setup + Corporate EXE setup + 3 `.sha256` sidecars). **Final v3.15.2 GitHub Release: 28 assets** verified via `gh release view v3.15.2`, published 2026-05-18T08:04:44Z. Three-agent audit + remediation cycle shipped 19 of 22 findings; F17 / F20 / F22 deferred for future releases (see `~/.claude/projects/.../memory/project_v3_15_2_deferred.md`).

The historical untagged versions (v3.11.0, v3.11.1) are documentation-only; users should install the latest published release, which subsumes them.

### Live behavior changes already on master (as of v3.14.3)

After a fresh `git pull` and `cargo build --release`, you'll see (verified on Windows 11 25H2 build 26200.8246, unelevated user session, Alienware on AC):

- `OS` row: `Windows 11 25H2` (was `Windows 11 (26200)`)
- `KERNEL` row: `26200.8246` (was `26200`)
- `LAST LOGIN` row: real timestamp (was `Login tracking unavailable`)
- `HYPERVISOR` row: `Bare Metal (Hyper-V/VBS)` on Win11 with VBS active (was `Hypervisor Present`)
- `MACHINE IP` / `DNS IP` rows now reflect the kernel's preferred-route adapter (correct on multi-homed/VPN configs — the old WMI-first-match behavior was a coin flip when a tunnel was active)
- `UPTIME` row renders `9d 4h 12m (session: 7h 14m)` annotation when Fast Startup is on AND cold-boot diverges from kernel-session by > 1h (skipped in `--fast` mode)
- `BATTERY` row: 5-state output — `AC Power` (≥95% on AC, no charging — clean, no percentage); `X% (Charging)` (on AC, charging); `X% (Plugged in)` (on AC, < 95%, not charging — gaming-laptop PSU-undersized OR firmware battery-longevity); `X% (Discharging)` (off AC, normal); `X% (Critical)` / `X% (Low)` overrides; `X% (Unknown)` fallback
- `SHELL` row: detects PowerShell 7+ via `HKLM\SOFTWARE\Microsoft\PowerShellCore\InstalledVersions\<GUID>\SemanticVersion` before falling through to legacy WinPS-5.x; semver-tuple comparison (not string sort) so `7.10` correctly outranks `7.9`
- `TERMINAL` row: env-var pre-checks (`WT_SESSION`, `TERM_PROGRAM=vscode`, `CURSOR_TRACE_ID`/`CURSOR_AGENT`) THEN parent-process walk via Toolhelp32 (cap 10 levels, intermediate hosts skipped) — recognizes Windows Terminal, WezTerm, Alacritty, VS Code, Cursor, Windsurf, Hyper, Tabby, Ghostty, Kitty, MinTTY, Claude Code, Antigravity
- `GPU` rows: registry-prefer in full mode + `filter_software_gpus()` name filter (strips Microsoft Basic Render Driver, Hyper-V Video, etc.) — only hardware adapters appear
- New footer line below the table on Linux + Windows when unelevated:
  - Linux: `Run with sudo for motherboard, BIOS, and RAM slot details`
  - Windows: `Run as Administrator for BitLocker status`
  - macOS: no footer (no meaningful elevated unlocks)
- `--no-elevation-hint` flag suppresses the footer
- `--fast` mode never shows the footer (auto-run safety)
- JSON output gains `schema_version: 1`, `elevated`, `elevation_unlocks_more`, `session.encryption`, `os.session_uptime_seconds`
- Positional actions now work without double-dash syntax: `tr300 update`, `tr300 install`, and `tr300 uninstall`. Existing `--update`, `--install`, and `--uninstall` remain supported.
- `tr300 update` now uses a cargo-first probe-and-retry strategy chain instead
  of executable-path install detection: `cargo install tr300 --force` when
  `cargo --version` succeeds, then `curl`/`wget` shell installers on Unix or
  `powershell`/`pwsh` installers on Windows. JSON update output preserves
  legacy `"method"` and adds precise `"strategy"` or failed `"attempts"`.
- Users can install from crates.io with `cargo install tr300`; v3.14.3 was
  published by GitHub Actions, and future crate versions publish after
  successful default-branch CI using `.github/workflows/crates-publish.yml`.
- Reports now render optional platform rows only when populated: `MODEL`, `CPU TOPOLOGY`, `MOTHERBOARD`, `BIOS`, `RAM SLOTS`, and `ZFS HEALTH`. Matching JSON keys are additive nullable fields under schema version 1.
- Collector subprocesses use bounded timeouts and degrade missing, hung, or malformed platform data to omitted rows / `null` values instead of blocking report generation.

### Pending (not yet shipped)

- **RESOLVED as a non-issue — A3: macOS host-vs-process architecture (v3.16.0 stability pass).**
  The audit flagged that `get_architecture()` (`src/collectors/platform/macos.rs:227-239`) might
  misreport an `x86_64` build running *natively* on Apple Silicon. On code review this is
  **physically impossible**: Apple Silicon CPUs have no x86 hardware, so *any* `x86_64` process on an
  M-series Mac runs under Rosetta, which means `sysctl.proc_translated` is **always `1`** there — and
  the existing code already returns `"x86_64 (Apple Silicon, Rosetta 2)"` for that case. The only way
  to reach `arch == "x86_64"` with `proc_translated == 0` is on a **real Intel Mac**, where reporting
  `"x86_64"` is correct. So all three real cases (native ARM → `aarch64`; Rosetta → annotated; Intel
  → `x86_64`) are already handled correctly, and the proposed `hw.optional.arm64` branch would be
  dead code. **No change made.** (If anyone wants to *enrich* the Intel-Mac row to read
  `"x86_64 (Intel)"` for symmetry, that's a cosmetic enhancement, not a bug fix.)
- **Mac-hands-on item — D4: macOS `pmset` battery parsing (v3.16.0 stability pass).** The audit
  flagged a "charging vs not-charging misclassification" in `get_battery()`
  (`src/collectors/platform/macos.rs`). On review the parser **echoes the raw `pmset` status word**
  rather than classifying into buckets, so `"discharging"`/`"not charging"` already render correctly
  — the stated bug doesn't occur. Two genuine micro-issues remain but need a Mac to change safely
  (the code shipped working in v3.14.0 and the exact `pmset -g batt` field layout must be confirmed
  on-device): (a) the percentage capture (`part.ends_with('%')`) can grab the whole
  `-InternalBattery-0 (id=…)\t85%` field instead of a clean `85%` token — harden it to match a
  standalone `^\d+%$` token; (b) the `"finishing charge"` state isn't recognized. Both are LOW
  severity and were deliberately **not** changed blind from the Windows dev host to avoid regressing
  verified behavior.
- **Self-update deep-dive (v3.16.0 / PR7).** A focused review of `src/update.rs` across all
  platforms/deployments (prompted by a report that `tr300 update` "often doesn't work").
  **Verified working:** every installer asset URL matches a published release asset and every
  `.sha256` sidecar exists (checked against the v3.15.3 release), so the v3.15.4 SHA256-verify fix
  *does* function; `is_newer` is robust (7 tests); the Windows MSI/EXE path has SHA256 + post-install
  verify + 3010-reboot handling; strategy ordering and the JSON contract are correct. **Fixed (U1):**
  the **cargo update path had no post-install verification** — `cargo install tr300 --force` reports
  success even when crates.io still serves the OLD version (publish lag / failed publish), so the
  updater claimed success without changing the binary and looped. The cargo path now re-execs
  `tr300 --version` and, on mismatch, returns a `Runtime` error so `execute_update` **falls through
  to the prebuilt GitHub-release installer** (which always carries `latest`). **Fixed (U2):** a clear
  "GitHub API rate limit exceeded (60/hr)" message replaces the opaque "Request failed" on HTTP 403.
  **Deliberately NOT verified:** the terminal curl/wget/PowerShell installer strategies — they're
  last in the chain and install to a default bindir (`~/.cargo/bin` / `~/.local/bin`) that may differ
  from `current_exe()`, so a post-install check there could false-*fail* with no further fallback;
  the cargo→installer fall-through is the safe place for the check. **Hardware test (user):** confirm
  on the Mac + Raspberry Pi that `tr300 update` from an older build either updates cleanly or reports
  an honest, actionable failure (no more silent "Updated to vX" no-op).
- **Deferred security hardening (3 Codex PRs closed 2026-06-03).** Three Codex-authored security
  PRs were closed in favor of the v3.16.0 stability pass because they touched overlapping files, but
  they raise **distinct hardening angles** the stability pass did not target. Pick up as a dedicated
  security PR if desired: (1) *temp-installer TOCTOU* (`src/update.rs`) — the MSI/EXE download uses a
  predictable `%TEMP%\tr300-update-{version}.{msi,exe}` path; an attacker who can pre-create that
  path (or a symlink at it) could influence what gets launched. Consider an unpredictable temp name
  / `O_EXCL`-style create. (2) *GPU-helper subprocess path resolution* (collectors) — `lspci` etc.
  are invoked by bare name, relying on `PATH`; consider absolute paths or PATH-sanitization for the
  privileged/elevated case. (3) *markdown auto-save clobber* (`src/report.rs`) — the report is
  written to a predictable `Downloads/tr300-report-<ts>.md` name; a pre-planted symlink there could
  redirect the write. Consider `O_EXCL`/`O_NOFOLLOW`-style create. (v3.16.0 PR5/G1 added temp-file
  *cleanup* and PR1/C3 reworked the save *error path*, but neither addressed these TOCTOU/symlink
  vectors.)
- ~~**PR #2** — macOS accuracy~~ — substantially shipped in v3.14.0 for the low-risk, verifiable paths on the current Mac: CPU brand/frequency fallback, Rosetta arch label, scutil hostname/IP, AppleLocale precedence, P/E core split, machine model row, full-mode battery health, and `vm_stat` fallback.
- ~~**PR #3** — Linux accuracy~~ — substantially shipped in v3.14.0 with fixture coverage and CI validation: systemd-resolved DNS priority, aarch64 CPU fallback, locale precedence, power_supply battery iteration + health, `ip route get ... src`, terminal env priority + single `ps` fallback, WSL/container/VM detection, ZFS health, and elevated `dmidecode` rows.
- **PR #5 leftovers (task #58)** — E.6 admin-only RDP login history via Security Event 4624 XML parsing. Deferred because it needs elevated-shell validation on Windows. C.13 batched-PowerShell fallback is shipped in v3.14.0.
- **Cross-platform 3-state battery model** (task #56) — extend the v3.13.0 Windows 5-state model to Linux + macOS. Linux can land independently using `/sys/class/power_supply/AC*/online` + `BAT*/status`; macOS waits on PR #2 hardware.
- ~~**Cargo-dist installer regression** (task #54)~~ — **resolved in v3.13.1.** What looked like a cargo-dist v0.31.0 installer regression on `x86_64-apple-darwin` + `x86_64-unknown-linux-musl` (observed on the v3.10.0 release.yml run) actually resolved itself in those runner images at some point between v3.10.0 and v3.12.0. The MSRV bump to 1.95 in v3.11.1 then surfaced a *different* failure on `x86_64-pc-windows-msvc` + `x86_64-unknown-linux-gnu` + `aarch64-unknown-linux-gnu` runners — those images ship rustc 1.94.1, and `release.yml` (auto-generated by cargo-dist) does not run rustup before invoking `dist build`. Fix: `rust-toolchain.toml` at repo root pinning `channel = "1.95"`. Rustup is pre-installed on every GitHub-hosted runner and respects the pin transparently. v3.10.0 retains its broken release.yml run as historic record; v3.13.1 is the first release with a working artifact since then.
- **PR #6** *(optional, deferred unless explicitly requested)* — `--security` flag adding TPM 2.0 + Secure Boot + FileVault + SELinux/AppArmor rows.
- **Revert macOS-ARM `continue-on-error` in `.github/workflows/ci.yml`** *(temporary, added during v3.14.5)* — runs 25849362170, 25849631205, 25850000969, 25850258250 (2026-05-14 07:46Z–09:30Z) all showed macOS ARM jobs aborting cargo at 0 seconds with no compile output, on multiple identical source trees. v3.14.4 (`ac3fd34`) on a structurally similar source passed cleanly hours earlier. The `continue-on-error: ${{ matrix.os == 'macos-latest' }}` on the `test` + `build` matrix jobs allows the overall CI workflow to report `success` so the v3.14.5 publish/release pipeline can proceed. Remove this knob in the first stable release after macOS ARM runner health recovers — confirm on a no-op commit that all macOS ARM matrix entries pass cleanly first.

### Recommended next steps (in order)

1. ~~**Watch CI on the v3.13.1 commit, then tag.**~~ Done. CI run 25096685639 green on `086ef0a`; tag `v3.13.1` pushed; release.yml run 25096833278 succeeded across all 10 jobs; GitHub Release published with 20 assets.
2. ~~**Investigate cargo-dist regression** (task #54)~~ — **done in v3.13.1.** The fix turned out to be smaller than the original plan suggested: `rust-toolchain.toml` at repo root with both `channel` and `components`, no cargo-dist version bump, no migration to the astral-sh fork.
3. ~~**Ship v3.14.0**~~ — done. `master` pushed, CI green, tag `v3.14.0` pushed, release.yml green, GitHub Release published.
4. ~~**Ship v3.14.1**~~ — done. `master` pushed, CI green, tag `v3.14.1` pushed, release.yml green, GitHub Release published.
5. ~~**Ship v3.14.2 + crates.io**~~ — done. `master` pushed, CI green, crates.io publish workflow green, tag `v3.14.2` pushed, release.yml green, GitHub Release published.
6. ~~**Ship v3.14.3 canonical `tr300` crate**~~ — done. `master` pushed, CI green, crates.io publish workflow green, tag `v3.14.3` pushed, release.yml green, GitHub Release published with `tr300-installer.*` plus legacy `tr-300-installer.*` aliases.
7. Next functional work: task #58 E.6 admin-only RDP history, only from an elevated Windows validation session.

---

## 1.5 What's already best-in-class (do not "fix")

Confirmed correct by research and verified in production. Don't waste effort re-investigating these unless a CHANGED upstream invalidates the source.

- **OS pretty name on Linux** — `/etc/os-release` PRETTY_NAME ([os-release(5)](https://man7.org/linux/man-pages/man5/os-release.5.html))
- **Linux load average** — `/proc/loadavg`
- **Linux uptime** — `/proc/uptime`
- **Linux memory available** — `/proc/meminfo` MemAvailable (kernel 3.14+)
- **Linux disk usage** — sysinfo's statvfs path
- **macOS DNS** — `scutil --dns` (resolv.conf on macOS is decorative — it explicitly says so in its own header)
- **macOS uptime** — `kern.boottime` sysctl (sysinfo uses it)
- **macOS sw_vers** — same data as `kern.osproductversion`
- **Windows hostname** — `GetComputerNameExW` (sysinfo uses it)
- **Windows memory** — `GlobalMemoryStatusEx` (sysinfo uses it)
- **Windows locale** — `GetUserDefaultLocaleName` (already in code)
- **SSH client IP detection** — `SSH_CLIENT` / `SSH_CONNECTION` env vars
- **Cores (physical + logical) on all platforms** — sysinfo handles SMT/HT correctly; Apple Silicon P/E cores need `hw.perflevel*` enrichment (M7) but the basic counts are right.
- **Battery presence/charge** — Linux `/sys/class/power_supply`, macOS `pmset`, Windows WMI `Win32_Battery` (PR #5 C.10 will swap Windows to the faster `GetSystemPowerStatus`).
- **macOS Activity Monitor formula** — `(active + wired + compressed) * page_size` matches sysinfo on most M-series; A.9 only swaps if a divergence is observed.

## 2. Hard constraints (non-negotiable, established by the user)

These shaped every decision and **must** be preserved by future PRs:

1. **Pager rejected.** Don't add a built-in pager / scrollback / `less` wrapper. Modern terminals all have native scrollback; auto-run depends on the prompt being free immediately after the table renders.
2. **`--fast` must stay sub-second.** Current Windows release median: ~308 ms. CI gate: < 1500 ms. New collectors that need subprocesses or > 50 ms must be full-mode-only.
3. **No admin/sudo in the default code path.** Auto-run runs as the user. Admin-only collectors must degrade silently (return `None`, let the elevation footer hint cover the gap). Never panic, prompt, or print errors to stderr.
4. **Report can't grow in line count.** Every PR's integration test must verify line count is unchanged. New rows are only allowed when *conditional* (battery enriched in place; ZFS Health only when zpool exists; ENCRYPTION row only when readable).
5. **Don't "fix" BTRFS subvolume / APFS container disk numbers.** sysinfo's reporting matches what the OS itself shows in Disk Utility / `df`. Changing the aggregation logic would regress against user expectations.

---

## 3. The full plan (consolidated from `~/.claude/plans/take-a-look-around-elegant-stonebraker.md`)

### Section 1 — macOS accuracy fixes (PR #2)

Research grounded in [Apple Developer Forums #652667](https://developer.apple.com/forums/thread/652667), [#664774](https://developer.apple.com/forums/thread/664774), [#671792](https://developer.apple.com/forums/thread/671792), [eclecticlight sysctl reference](https://eclecticlight.co/sysctl-information/), [eclecticlight Oct 2025 frequency table](https://eclecticlight.co/2025/10/28/updating-cpu-frequencies-for-apple-silicon-macs/), [psutil #1892](https://github.com/giampaolo/psutil/issues/1892), [cpufetch #139](https://github.com/Dr-Noob/cpufetch/issues/139).

| ID | Issue | Current source | Better source | File |
|---|---|---|---|---|
| M1 | CPU frequency = 0 GHz on Apple Silicon | sysinfo (returns 0 — `hw.cpufrequency` doesn't exist on M-series) | Chip-name → max-frequency lookup table keyed on `machdep.cpu.brand_string` | `cpu.rs`, `platform/macos.rs` |
| M2 | CPU brand stale on M3/M4 (older sysinfo) | sysinfo | `sysctl -n machdep.cpu.brand_string` direct | `cpu.rs`, `platform/macos.rs` |
| M3 | Architecture wrong under Rosetta 2 | `std::env::consts::ARCH` | Add `sysctl -n sysctl.proc_translated`; show `Apple Silicon (running x86_64 via Rosetta 2)` | `platform/macos.rs` |
| M4 | Hostname doesn't match System Settings | sysinfo `gethostname()` | `scutil --get ComputerName` | `os.rs`, `platform/macos.rs` |
| M5 | Local IP picks wrong interface on multi-homed/VPN | Hardcoded en0/en1/en2 | `scutil --nwi` for primary interface, then `ipconfig getifaddr` | `network.rs`, `platform/macos.rs` |
| M6 | Locale shows shell `LANG` instead of user's region | `LANG` first | `defaults read -g AppleLocale` first; strip `@rg=...` | `platform/macos.rs` |
| M7 | No P-core / E-core breakdown | sysinfo physical/logical only | `hw.perflevel0.physicalcpu` + `hw.perflevel1.physicalcpu`; render `12 cores (8P + 4E)` | `cpu.rs`, `platform/macos.rs` |
| M8 | Memory "used" doesn't match Activity Monitor | sysinfo's used | `host_statistics64` formula `(active + wired + compressed) * page_size`; only swap if sysinfo diverges | `memory.rs`, `platform/macos.rs` |

**Skip:** thermal pressure level (undocumented + Intel-only), serial number (PII risk), time-since-last-power-off (no API distinct from `kern.boottime`).

### Section 2 — Linux accuracy fixes (PR #3)

Research grounded in [systemd-resolved.service(8)](https://man7.org/linux/man-pages/man8/systemd-resolved.service.8.html), [systemd-detect-virt(1)](https://man7.org/linux/man-pages/man1/systemd-detect-virt.1.html), [POSIX locale precedence](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap08.html), [kernel sysfs-class-power](https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-class-power), [raspberrypi/linux #3991](https://github.com/raspberrypi/linux/issues/3991).

| ID | Issue | Better source | File |
|---|---|---|---|
| L1 | DNS reads stub `127.0.0.53` on systemd-resolved | `/run/systemd/resolve/resolv.conf` → `/run/NetworkManager/resolv.conf` → `/etc/resolv.conf` | `network.rs` |
| L2 | CPU brand empty on aarch64 | `/proc/cpuinfo` model name → `/sys/firmware/devicetree/base/model` → `CPU implementer`/`CPU part` decode | `cpu.rs`, `platform/linux.rs` |
| L3 | Hypervisor misses cloud + containers | Port `systemd-detect-virt` logic in-process: cpuid hypervisor flag → DMI sys_vendor → cgroup/`.dockerenv`/`.containerenv`/container env; covers kvm, qemu, vmware, hyperv, parallels, virtualbox, xen, wsl, docker, podman, lxc, systemd-nspawn, aws, azure, gcp | `platform/linux.rs` |
| L4 | Locale ignores `LC_ALL` and `LC_CTYPE` | POSIX precedence: `LC_ALL` > `LC_CTYPE` > `LANG` > `C` | `platform/linux.rs` |
| L5 | Battery hardcodes `BAT0`/`BAT1` | Iterate `/sys/class/power_supply/*` filtering on `type == "Battery"`; add health % via `energy_full / energy_full_design` | `platform/linux.rs` |
| L6 | Local IP wrong on multi-homed hosts | `ip route get 1.1.1.1` parse "src" field | `platform/linux.rs`, `network.rs` |
| L7 | Terminal detection ordering causes misses | Check `KITTY_WINDOW_ID`, `WEZTERM_PANE`, `GHOSTTY_RESOURCES_DIR`, `ALACRITTY_LOG`, `KONSOLE_VERSION`, `VTE_VERSION`, `FOOT_PID`, `TILIX_ID`, `WT_SESSION` first; `TERM_PROGRAM` second; ps-walk fallback | `platform/linux.rs` |
| L8 | WSL2 not detected | Check `/proc/sys/kernel/osrelease` for `microsoft-standard-WSL2` (folded into B.3) | `platform/linux.rs` |
| L9 | `last`/`lastlog` deprecated on glibc 2.40+ | `lastlog2` → `lastlog` → `last -F -1 -w` | `session.rs` |
| E.3 | Motherboard / BIOS only readable as root | When `is_elevated`: parse `dmidecode -s baseboard-{manufacturer,product-name}`, `bios-{vendor,version,release-date}`, `chassis-type`. Render in natural spots; full-mode + elevated only | `platform/linux.rs` |
| E.4 | RAM slot detail only readable as root | When `is_elevated` + full mode: `dmidecode -t memory`, render compact summary like `2x16GB DDR5-5600 SK Hynix` | `platform/linux.rs` |

**Container/musl caveats:** Alpine has no systemd, no `mokutil`, often no `lspci`. Logic must be in-process. Containers inherit host kernel/uptime/loadavg/memory — flag in JSON as `in_container: true` rather than "correcting".

### Section 3 — Windows accuracy fixes

**PR #4 (already shipped — see commit `3a252df`):** C.1 (registry OS), C.2 (IsWow64Process2 arch), C.3 (WTS last-login), C.6 (CPUID 16h + CallNtPowerInformation freq), C.7 (CPUID 0x40000000 hypervisor + VBS disambiguation), E.5 (BitLocker).

**PR #4b (already shipped — see commit `28bda98` / v3.12.0):**

| ID | Fix | File |
|---|---|---|
| C.4 | DNS+IP via `GetBestInterfaceEx` + `GetAdaptersAddresses` (VPN-aware default-route detection); replaces WMI Win32_NetworkAdapterConfiguration | `platform/windows.rs:88-109` |
| C.5 | Fast Startup uptime annotation: registry `HiberbootEnabled` + `GetTickCount64` vs `LastBootUpTime`; append `(session: Xh)` suffix when divergent | `os.rs`, `platform/windows.rs` |

**PR #5 (Windows polish, mostly shipped across v3.13.0 + v3.14.0):**

| ID | Fix | File |
|---|---|---|
| C.8 | GPU via DXGI `EnumAdapters1` (filter `DXGI_ADAPTER_FLAG_SOFTWARE` and vendor 0x1414 software adapter) | `platform/windows.rs` |
| C.9 | Cores via `GetLogicalProcessorInformationEx(RelationProcessorPackage, …)` | `cpu.rs`, `platform/windows.rs` |
| C.10 | Battery via `GetSystemPowerStatus` + `DeviceIoControl(\\.\Battery0)` for cycle count + design capacity | `platform/windows.rs` |
| C.11 | PowerShell 7+ detection via `HKLM\SOFTWARE\Microsoft\PowerShellCore\InstalledVersions\*` | `session.rs`, `platform/windows.rs` |
| C.12 | Terminal parent-process walk via `CreateToolhelp32Snapshot` + `Process32NextW` for conhost-only sessions | `platform/windows.rs` |
| C.13 | Batched PowerShell fallback into single JSON call (when WMI/COM init fails) | `platform/windows.rs:88-109` |
| C.14 | Drop COM/WMI init from `--fast` hot path once C.4/C.6/C.9/C.10 land | `platform/windows.rs` |
| E.6 | Admin-only full RDP login history via Security Event Log 4624 (filter `LogonType in [3, 7, 10, 11]`) | `platform/windows.rs` |

**Combined cold-start savings on full mode:** ~150–400 ms by replacing remaining WMI paths with Win32 APIs.

### Section 4 — Cross-platform reliability (mostly done in PR #1)

| ID | Status | Note |
|---|---|---|
| X1 | Done in v3.14.0 | ZFS Health row: when `zpool` is on `$PATH`, run `zpool list -H -o health` and aggregate worst-of. Skip in `--fast`. Render only when zpool exists. |
| X2 | Done in PR #1 | BTRFS/APFS volume-semantics note in CLAUDE.md so contributors don't "fix" what isn't broken. |
| X3 | Done in PR #1 | JSON `schema_version: 1` in every output. |

### Section 5 — Speed improvements

| ID | Optimization | Status | Saves |
|---|---|---|---|
| S1 | Linux: collapse 3 sequential `ps` calls into one `ps -e -o pid,ppid,comm` | Done in v3.14.0 | ~20–30 ms |
| S2 | Windows: replace WMI paths with Win32 APIs, eliminate COM init from hot path | Done for `--fast`; native paths remain primary in full mode | ~150–400 ms cold |
| S3 | Windows: batched PowerShell fallback when WMI fails | Done in v3.14.0 | ~150–250 ms (uncommon path) |
| S4 | macOS: already optimal, no action | N/A | — |

---

## 3.5 Skipped (researched and explicitly rejected)

These were investigated during the planning research and **deliberately not added**. If a future contributor proposes any of them, point them at this section to short-circuit a re-litigation.

- **Built-in pager / scrollback viewer** — modern terminals (Windows Terminal, iTerm2, Wezterm, Konsole, Alacritty, Ghostty) all have native scrollback. Auto-run depends on the prompt being free immediately. **Hard rejected by the user.**
- **Thermal sensors** — opaque on macOS (`machdep.xcpm.cpu_thermal_level` is undocumented and Intel-only), fragile on Linux hwmon (thermal zone naming inconsistent across boards), gone from Windows in modern drivers.
- **Top processes by CPU/RAM** — blows up the compact 51-column table layout.
- **NIC link speed** — ethtool may not be installed on Linux; signal-to-noise is low for a one-line report.
- **Disk SMART data** — requires elevated privileges on most platforms; smartctl + nvme-cli are platform-fragile.
- **Package count** — unportable; nothing comparable across pacman/apt/dnf/brew/winget/snap/flatpak.
- **Serial number** — privacy-sensitive PII; if a user pastes a TR-300 report into Discord or GitHub they leak their AppleCare-claimable serial. Skip unless gated behind an explicit `--include-serial` flag.
- **Pending reboot indicator (Windows)** — low signal-to-noise for the default report; could land in a future `--security` flag if requested.
- **DPI scaling** — no resolution row currently exists to attach to.
- **LUKS root detection via lsblk parsing** — breaks on ZFS-encryption, fscrypt, stacked setups.
- **CPU temperature** — thermal zone naming inconsistent across boards.
- **Battery time-remaining ETA** — `power_now` lags 30s after AC change; misleading.
- **Last-boot reason** — distro-specific, needs root.

## 4. Implementation task checklist (master)

Status legend: `[x]` done, `[ ]` pending, `[~]` deferred to a later PR than originally planned.

### PR #1 — Foundation scaffolding (✅ shipped as v3.10.0, commit `58812cc`)

- [x] **D.1** Add JSON `schema_version` field
- [x] **E.1** Add `is_elevated` detection (`tr300::is_elevated()`, manual `IsUserAnAdmin` extern on Windows)
- [x] **E.2** Add `--no-elevation-hint` CLI flag
- [x] **E.7** Footer renderer + `platform_has_elevated_data()` helper
- [x] **E.8** JSON `elevated` + `elevation_unlocks_more` keys
- [x] **F.1** CHANGELOG.md v3.10.0 entry
- [x] **F.2** README.md scaffold update
- [x] **F.3** CLAUDE.md architecture notes
- [x] **F.4** Cargo.toml bump to 3.10.0
- [x] **F.5** Auto memory writes (project / feedback / reference)
- [x] **F.6** Create TESTING.md with manual matrix
- [x] **CI.1** `.github/workflows/ci.yml` cross-platform matrix
- [x] **CI.2** Fix assert_cmd deprecation in integration tests
- [x] **CI.3** Cargo-audit job
- [x] **CI.4** Verify `dist plan` as a gate
- [x] **CI.5** Speed regression check (`--fast` budget gate)
- [x] **CI.6** Document CI in README + CLAUDE.md + TESTING.md
- [x] Tests: footer logic + JSON schema baseline

### PR #2 — macOS accuracy (✅ low-risk subset shipped in v3.14.0)

- [x] **A.1** macOS CPU brand via `sysctl machdep.cpu.brand_string`
- [x] **A.2** Apple Silicon CPU frequency lookup table (M1/M2/M3/M4 families)
- [x] **A.3** Rosetta 2 architecture detection (`sysctl.proc_translated`)
- [x] **A.4** `scutil --get ComputerName` for hostname
- [x] **A.5** `scutil --nwi` for primary interface IP
- [x] **A.6** Locale via `defaults read -g AppleLocale` first; strip `@rg=...`
- [x] **A.7** P/E core breakdown via `hw.perflevel0/1.physicalcpu`
- [~] **A.8** Mac model row shipped via `hw.model`; marketing-name lookup remains optional future polish
- [x] **A.9** Memory verification + `vm_stat` fallback if sysinfo diverges
- [x] **A.10** Battery health via `system_profiler SPPowerDataType -json`
- [x] **F.1–F.5** macOS PR documentation block
- [x] Tests: macOS parser/unit coverage + local Apple Silicon verification; Intel Mac remains CI-only for release artifact build coverage

### PR #3 — Linux accuracy + dmidecode tier (✅ low-risk subset shipped in v3.14.0)

- [x] **B.1** DNS priority chain: `/run/systemd/resolve/resolv.conf` → NetworkManager → `/etc/resolv.conf`
- [x] **B.2** aarch64 CPU brand fallback chain
- [x] **B.3** In-process virtualization detection subset (cloud + containers + WSL)
- [x] **B.4** POSIX locale precedence
- [x] **B.5** Iterate `/sys/class/power_supply/*` with type filter + battery health
- [x] **B.6** Local IP via `ip route get`
- [x] **B.7** Terminal env-var priority over ps-walk
- [x] **B.8** WSL1/WSL2 detection (folded into B.3)
- [x] **B.9** Last-login fallback chain (lastlog2 → lastlog → last -F -1 -w)
- [x] **B.10** Single ps call for terminal detection (was 3) — speed win S1
- [x] **B.11** ZFS Health when `zpool` present
- [x] **E.3** Linux dmidecode motherboard/BIOS (sudo only)
- [x] **E.4** Linux dmidecode RAM slot summary (sudo only)
- [x] **F.1–F.5** Linux PR documentation block
- [x] Tests: Linux parser fixture coverage + GitHub Actions Linux validation; distro/Raspberry Pi/manual hardware stamps remain future matrix work

### PR #4 — Windows accuracy + BitLocker (✅ shipped as v3.11.0, commit `3a252df`)

- [x] **C.1** Windows OS from registry (Win11 detect by build ≥ 22000, DisplayVersion + UBR)
- [x] **C.2** Windows arch via `IsWow64Process2`
- [x] **C.3** Last-login via `WTSQuerySessionInformation` + boot-time fallback
- [~] **C.4** Windows DNS+IP via `GetBestInterfaceEx` + `GetAdaptersAddresses` — **shipped in v3.12.0 (PR #4b)**, simpler design
- [~] **C.5** Fast Startup uptime detection — **shipped in v3.12.0 (PR #4b)**
- [x] **C.6** CPU freq via CPUID leaf 16h + `CallNtPowerInformation`
- [x] **C.7** Hypervisor via CPUID `0x40000000` with VBS disambiguation
- [x] **E.5** Windows BitLocker collector (try non-admin, escalate)
- [x] **F.1–F.5** Windows accuracy PR documentation
- [x] Tests: Windows unit + manual matrix (user-validatable)

### PR #4b — Deferred Windows accuracy (✅ shipped as v3.12.0, commit `28bda98`)

- [x] **C.4** VPN-aware default-route detection — `GetBestInterfaceEx` (manual extern, `iphlpapi`) + reorder existing WMI `Win32_NetworkAdapterConfiguration` rows by `interface_index` so the kernel-preferred adapter wins. *Plan deviation:* simpler than the originally-planned `GetBestInterfaceEx` + `GetAdaptersAddresses` linked-list walk; same VPN-aware outcome, ~50 LOC vs ~120 LOC, much smaller unsafe surface.
- [x] **C.5** Fast Startup uptime annotation — `detect_fast_startup()` reads `HiberbootEnabled` registry DWORD; `last_cold_boot_seconds()` queries `Win32_OperatingSystem.LastBootUpTime` via `wmi::WMIDateTime` (the wmi crate's serde-aware CIM datetime wrapper — an early hand-written parser was discarded). When divergence > 1h, render `9d 4h 12m (session: 7h 14m)`; full-mode-only.
- [x] **F.1–F.6** PR #4b documentation block — CHANGELOG, README, CLAUDE.md, Cargo.toml, auto-memory, TESTING.md.
- [x] Endianness fix in `sin_addr` (caught during Codex GPT-5.5 review): original `from_be_bytes` would have routed to wrong IP on non-palindromic destinations; fixed to `from_le_bytes` with explanatory comment.

### PR #5 — Windows polish (✅ shipped partially as v3.13.0, commit `f34e981`)

- [x] **C.8** GPU enumeration via DXGI `EnumAdapters1` — *plan deviation:* used the simpler registry-prefer + `filter_software_gpus()` name-based filter approach instead. ~25 LOC vs ~100 LOC of unsafe COM. Same user-visible outcome (no Microsoft Basic Render Driver). DXGI deferred unless a future bug demands vendor/device-ID filtering that name-based filtering can't do.
- [x] **C.9** Cores via `GetLogicalProcessorInformationEx` — two-call buffer-sizing pattern, alignment-safe walk via `u32::from_le_bytes` (caught in Codex review — the original raw-cast approach happened to work in practice but was technically UB per Rust spec). WMI fallback retained.
- [x] **C.10** Battery via `GetSystemPowerStatus` (deferred `DeviceIoControl` cycle-count enrichment).
- [x] **C.10b** *(plan addition, user-requested)* Extended C.10 from 3-state to 5+ states: `AC Power` (≥95% on AC, no charging — clean, no percentage), `X% (Charging)`, `X% (Plugged in)` (gaming-laptop PSU-undersized OR firmware battery-longevity), `X% (Discharging)` / `Critical` / `Low` overrides off AC, `X% (Unknown)` fallback. AC-status-unknown edge case renders bare `X%`.
- [x] **C.11** PowerShell 7+ detection via `PowerShellCore` registry. *Caught in Codex review:* original string-compare approach put `"7.9.0" > "7.10.0"`; fixed via `parse_semver_tuple` comparing `(u64, u64, u64)`.
- [x] **C.12** Terminal parent-process walk via Toolhelp32 — `CreateToolhelp32Snapshot` + `Process32FirstW`/`Process32NextW`, build `HashMap<pid, (parent_pid, name)>`, climb cap 10 levels. Recognizes Windows Terminal, WezTerm, Alacritty, VS Code, Cursor, Windsurf, Hyper, Tabby, Ghostty, Kitty, MinTTY, Claude Code, Antigravity. Intermediate hosts (`conhost.exe`, `powershell.exe`, `pwsh.exe`, `cmd.exe`, shells) skipped.
- [x] **C.13** Batched-PowerShell fallback into single JSON call — shipped in
  v3.14.0 (`get_batched_powershell_fallback()` plus parser fixtures). It is
  used only when the full-mode WMI connection fails, keeping the common path
  native/WMI-first and the `--fast` path COM-free.
- [x] **C.14** Drop COM/WMI init from `--fast` hot path — verified by audit: fast mode early-returns at `platform/windows.rs:66` BEFORE any `COMLibrary::new()` call; structurally already done.
- [ ] **E.6** Admin-only full RDP login history — **deferred to task #58**; needs elevated-shell validation. Implementation plan: `wevtutil qe Security /q:"*[System[EventID=4624]]" /c:50 /rd:true /f:xml`, hand-rolled XML parse, filter `LogonType ∈ {3,7,10,11}`, render 1-5 conditional rows under LAST LOGIN.
- [x] **F.1–F.6** PR #5 documentation block — CHANGELOG, README, CLAUDE.md, Cargo.toml, auto-memory, TESTING.md.
- [x] Tests: Windows polish unit (covered by existing test suite — additive changes don't need new tests; the deferred tests for `cpu.gpus` array regression and `session.login_history` are tied to the deferred E.6).

### PR #6 — Optional `--security` flag (⏸ deferred unless explicitly requested)

- [ ] Add `--security` CLI flag
- [ ] macOS: `fdesetup status`, `csrutil status`
- [ ] Linux: `/sys/class/tpm/tpm0/tpm_version_major`, `/sys/firmware/efi/efivars/SecureBoot-*`, `/sys/kernel/security/lsm`, `/sys/fs/selinux/enforce`
- [ ] Windows: `Win32_Tpm` via CIM (no admin), registry `HKLM\SYSTEM\CurrentControlSet\Control\SecureBoot\State\UEFISecureBootEnabled`
- [ ] Tests + docs

---

## 4.5 Files modified / to-be-modified (full inventory)

Historical inventory from the original implementation plan, reconciled against
the current v3.14.3 source. Items below reflect current repository state, not
only the first PR that touched them.

**Source files:**

- [done] `Cargo.toml` — version bumps; `windows`/`winapi` features as needed
- [done] `src/cli.rs` — new `--no-elevation-hint` flag; future flags will land here
- [done] `src/lib.rs` — `is_elevated()`, `platform_has_elevated_data()`, manual `IsUserAnAdmin` extern
- [done] `src/config.rs` — `no_elevation_hint` field on Config; builder method
- [done] `src/main.rs` — flag → config wiring
- [done] `src/report.rs` — `SCHEMA_VERSION`, `should_render_elevation_footer`, `render_elevation_footer`, JSON additions, BitLocker row, encryption JSON key
- [done] `src/collectors/mod.rs` — `is_elevated`/`encryption` fields on `SystemInfo`; wiring in `collect_with_mode`
- [done] `src/collectors/cpu.rs` — CPUID leaf 16h, `cpu_max_mhz_windows`, manual `CallNtPowerInformation` extern
- [done] `src/collectors/os.rs` — Windows registry override path
- [done] `src/collectors/session.rs` — `wts_query_session_connect_time`, `boot_time_local_string`, manual `WTSQuerySessionInformationW` extern
- [done] `src/collectors/platform/mod.rs` — `encryption` field on `PlatformInfo`
- [done] `src/collectors/platform/windows.rs` — `get_os_info_from_registry`, `cpuid_hypervisor_brand`, `map_hypervisor_vendor`, `get_bitlocker_status`, `format_bitlocker_status`, `bitlocker_method_name`, manual `IsWow64Process2`/`GetCurrentProcess` externs, `IMAGE_FILE_MACHINE_*` constants
- [done] `src/collectors/platform/linux.rs` — Linux accuracy subset, battery
  iteration/health, virtualization/container detection, ZFS health, elevated
  dmidecode motherboard/BIOS/RAM rows
- [done] `src/collectors/platform/macos.rs` — macOS accuracy subset, Computer
  Name, AppleLocale, Rosetta label, P/E core data, model row, battery health
- [done] `src/collectors/network.rs` — Linux default-route source IP, macOS
  `scutil --nwi` primary interface, systemd-resolved DNS priority, macOS
  `scutil --dns`, Windows WMI/native-first network path with graceful
  subprocess fallbacks
- [done] `src/collectors/disk.rs` — sysinfo disk enumeration, zero-size skip,
  removable/root-volume aggregation support through `SystemInfo`
- [done] `src/collectors/memory.rs` — sysinfo memory counters plus macOS
  `vm_stat` Activity Monitor-style used-memory fallback

**Documentation:**

- [done] `CHANGELOG.md` — current release history through v3.14.3 plus
  unreleased docs-consistency notes
- [done] `CLAUDE.md` — edit-time rules, CI gates, release process, updater
  strategy, crates.io workflow, and cargo-dist alias customization
- [done] `AGENTS.md` — agent-facing project guide, release checklist, updater
  and publishing workflow
- [done] `README.md` — current user-facing install/update/release docs for
  `cargo install tr300`, canonical installers, MSI, and crates.io
- [done] `CODEX_PROJECT.md` — current status, filetree, and v3.14.3 release
  evidence
- [done] `TESTING.md` — manual matrix plus per-version verification log through
  v3.14.3
- [done] `docs/architecture-decisions.md` — rationale for MSRV pinning,
  updater strategy, canonical `tr300` package naming, and release workflow
  compatibility aliases
- [done] `MASTER_PLAN.md` — this file

**CI / build:**

- [done] `.github/workflows/ci.yml` — fmt + clippy + cross-platform test/build/speed/audit/dist-plan
- [pre-existing, not hand-edited] `.github/workflows/release.yml` — auto-generated by cargo-dist v0.31.0; regenerate via `dist init` after `[workspace.metadata.dist]` changes
- [done, auto-generated] `man/tr300.1` — regenerated by `build.rs` via `clap_mangen` at build time

**Configuration:**

- [done] `.claude/settings.json` — `extraKnownMarketplaces.openai-codex` + `enabledPlugins.codex@openai-codex` (Codex plugin config at project scope)
- [done] `.gitignore` — `.claude/settings.local.json` (per-machine state)

**Auto memory (only valid on the original Windows host; not committed to repo):**

- `~/.claude/projects/C--Users-hey-git-qube-machine-report/memory/`
  - [done] `MEMORY.md` — index
  - [done] `project_tr300_overview.md` — what TR-300 is, current version, deployment, constraints
  - [done] `feedback_tr300_constraints.md` — pager rejected, --fast must stay sub-second, no admin, etc.
  - [done] `feedback_tr300_workflow.md` — the 7-phase development cadence
  - [done] `reference_tr300_release_process.md` — release step checklist + CI mechanism
- The plan file at `~/.claude/plans/take-a-look-around-elegant-stonebraker.md` (full original plan, ~1500 lines with citations)

## 4.6 Testing strategy (full reference)

CI runs the gates (`.github/workflows/ci.yml`) automatically. Locally reproduce the same checks before pushing. Manual cross-platform matrix runs before tagging a release.

### Unit tests in the lib (T.U.* — run on every CI target)

- [done] **T.U.23** `elevation_footer_string_logic` (PR #1, in `src/report.rs`)
- [done] **schema_version_is_one** (PR #1)
- [done] **elevation_footer_uses_ansi_dim_when_colors_enabled** (PR #1)
- [done] **elevation_footer_string_is_empty_on_macos** (PR #1)
- [done] **elevation_footer_present_when_unelevated_full_no_optout** (PR #1)
- [done] **elevation_footer_skipped_when_user_opted_out** (PR #1)
- [done] **elevation_footer_skipped_in_fast_mode** (PR #1)
- [done] **elevation_footer_skipped_when_elevated** (PR #1)
- [ ] **T.U.1** `parse_resolvectl_status` — feed known fixture, assert DNS list (PR #3 B.1)
- [ ] **T.U.2** `parse_route_get_default_macos` (PR #2 A.5)
- [ ] **T.U.3** `parse_scutil_nwi` (PR #2 A.5)
- [ ] **T.U.4** `parse_scutil_dns` — regression guard
- [ ] **T.U.5** `parse_devicetree_model` (PR #3 B.2)
- [ ] **T.U.6** `decode_arm_implementer_part` (PR #3 B.2)
- [ ] **T.U.7** `parse_zpool_health` (PR #3 B.11)
- [ ] **T.U.8** `detect_wsl_from_osrelease` (PR #3 B.3 + B.8)
- [ ] **T.U.9** `detect_container_from_proc1` (PR #3 B.3)
- [ ] **T.U.10** `cpuid_hypervisor_vendor_to_name` — Windows currently inlined; could extract for unit testability (PR #4 C.7 — defer cleanup)
- [ ] **T.U.11** `apple_silicon_freq_lookup` (PR #2 A.2)
- [ ] **T.U.12** `posix_locale_precedence` (PR #3 B.4)
- [ ] **T.U.13** `battery_health_ratio` (PR #2 A.10, PR #3 B.5, PR #5 C.10)
- [ ] **T.U.14** `parse_event_log_27_xml` (PR #4b C.5)
- [ ] **T.U.15** `parse_si_machine_attributes_plist` (PR #2 A.8)
- [ ] **T.U.16** `parse_apple_locale_strip_rg` (PR #2 A.6)
- [ ] **T.U.17** `terminal_env_var_priority` (PR #3 B.7)
- [ ] **T.U.18** `parse_hw_perflevel_cores` (PR #2 A.7)
- [ ] **T.U.19** `parse_dmidecode_baseboard_output` (PR #3 E.3)
- [ ] **T.U.20** `parse_dmidecode_memory_table` (PR #3 E.4)
- [ ] **T.U.21** `parse_bitlocker_protection_status` — Windows inlined; could extract (PR #4 E.5 — defer cleanup)
- [ ] **T.U.22** `parse_event_4624_xml_logon_types` (PR #5 E.6)

### Integration tests (`tests/integration.rs`, T.I.*)

- [done] **T.I.1** `--json` schema regression — `schema_version: 1` baseline (PR #1)
- [done] **T.I.2** `--fast --json` produces same key set as `--json` modulo nullables — implicitly via field presence checks; full key-set diff TODO
- [done] **T.I.3** `--ascii` output line count baseline — currently informal (captured in TESTING.md); **TODO**: formalize as a snapshot test
- [done] **T.I.4** `--ascii` structural snapshot (border lines / headers) — currently informal; **TODO**: formalize
- [done] **T.I.5** Speed regression: `tr300 --fast` < 1500 ms median (CI gate); local < 800 ms target — **enforced by `.github/workflows/ci.yml`**
- [done] **T.I.6** `--json | jq .` parses without error on all platforms — implicit via `cargo test`'s integration test for `--json`
- [done] **test_json_includes_schema_version** (PR #1)
- [done] **test_json_includes_elevation_keys** (PR #1)
- [done] **test_no_elevation_hint_flag_accepted** (PR #1)
- [done] **test_fast_mode_no_elevation_footer** (PR #1)
- [done] **test_json_includes_encryption_key** (PR #4)

### Output stability gates (T.S.*)

- [done] **T.S.1** Line-count baseline captured (39 lines for `--ascii` on Windows unelevated full mode); enforce no-growth in future PRs
- [done] **T.S.2** Speed baseline captured (~308 ms median for `--fast` on Windows release build); regression tested in CI

### Manual cross-platform matrix (in `TESTING.md`)

Tracks "Last verified" per row. Each PR appends a `### vX.Y.Z — YYYY-MM-DD` section under "Per-release verification log". Already populated for v3.10.0 and v3.11.0 (Windows 11 25H2 build 26200.8246, unelevated user session). Pending hardware: macOS Intel + Apple Silicon M1/M3/M4, Ubuntu 22.04, Debian 12, Fedora, Arch, Alpine in Docker, Raspberry Pi 4 aarch64, AWS EC2 (Graviton + Intel), WSL2, Win10 (no Fast Startup), Win11 ARM64, Win11 with admin shell.

## 4.7 Phasing & sequencing (the canonical PR order)

Land in this order. PR #1 unblocks every later PR. PR #4 already shipped, but C.4 + C.5 are deferred to PR #4b. Each PR is one commit (using the existing `release: vX.Y.Z - <summary>` convention).

| PR | Status | Commit | Purpose |
|---|---|---|---|
| 1 | ✅ shipped | `58812cc` (v3.10.0) | Foundation: elevation tier scaffolding + CI + Codex config + workflow doc |
| 2 | ✅ shipped (v3.14.0) | `54dbae1` | macOS accuracy subset: CPU brand/frequency fallback, Rosetta arch label, scutil hostname/IP, AppleLocale precedence, P/E core split, model row, full-mode battery health, vm_stat fallback |
| 3 | ✅ shipped (v3.14.0) | `54dbae1` | Linux accuracy subset: systemd-resolved DNS, default-route source IP, locale precedence, power_supply battery iteration, terminal detection, WSL/container/VM detection, aarch64 CPU fallback, ZFS health, elevated dmidecode rows |
| 4 | ✅ shipped | `3a252df` (v3.11.0) | Windows accuracy (C.1, C.2, C.3, C.6, C.7) + BitLocker (E.5) + Windows PR docs |
| (4.5) | ✅ shipped | `22f2002` (v3.11.1) | Rust 1.95 MSRV + auto-rustup self-update + uzers RUSTSEC migration |
| 4b | ✅ shipped | `28bda98` (v3.12.0) | Deferred Windows accuracy: VPN-aware default-route (C.4 simplified design) + Fast Startup uptime annotation (C.5) |
| 5 | ✅ shipped (partial) | `f34e981` (v3.13.0) + `54dbae1` (v3.14.0) | Windows polish: native battery+5-state model (C.10/C.10b), native cores (C.9), GPU registry-prefer + name filter (C.8), PSCore detection (C.11), terminal parent walk (C.12), `--fast` COM-free verified (C.14), batched PowerShell WMI-failure fallback (C.13). Task #58 tracks E.6 admin RDP history. |
| 5b | ⏳ pending (task #58) | — | E.6 admin RDP login history only; requires elevated Windows validation |
| 6 | ⏸ optional | — | `--security` flag with TPM + Secure Boot + FileVault + SELinux/AppArmor |
| (cross-cutting) | ⏳ pending (task #56) | — | Battery 3-state model for Linux + macOS (mirror v3.13.0 Windows C.10b) |
| (infra) | ✅ shipped (task #54) | (v3.13.1) | release.yml MSRV/runner-image mismatch: added `rust-toolchain.toml` pinning `channel = "1.95"` so rustup auto-installs the right toolchain on every GitHub-hosted runner before `dist build` invokes cargo. Resolves `error: rustc 1.94.1 is not supported by ... tr-300 requires rustc 1.95` on Windows + Linux gnu + Linux ARM gnu runners. Single source of truth between `Cargo.toml` `rust-version` and the rust-toolchain pin. |

Within each PR, sub-tasks can be tackled in any order that compiles. Per-PR documentation block (F.1–F.6) runs at the end before commit. Verification runs before the push.

## 4.8 Codex plugin config (committed at project scope)

The repo includes `.claude/settings.json` so any session that clones the repo gets the same Codex review subagent without manual setup:

```json
{
  "extraKnownMarketplaces": {
    "openai-codex": {
      "source": {
        "source": "github",
        "repo": "openai/codex-plugin-cc"
      }
    }
  },
  "enabledPlugins": {
    "codex@openai-codex": true
  }
}
```

`.claude/settings.local.json` (per-machine state like `outputStyle`) is gitignored.

When a teammate or future-you on a fresh machine first opens the folder, Claude Code will prompt to trust the marketplace — accept it. Then the `codex:codex-rescue` subagent is available via the Agent tool (not via `/codex:rescue` slash command nor `Skill(codex:rescue)` — those don't work; only `Agent(subagent_type: "codex:codex-rescue")` does).

**Codex review timing**: dispatch *after* the PR exists on GitHub (Codex's `gh pr diff` path needs a real PR number). On `master`-direct workflows like this repo, dispatch Codex pre-push against the local diff if needed, and accept its review before tagging the release.

## 4.9 JSON schema versioning policy

`report::SCHEMA_VERSION` is currently `1`. Bump rules:

- **Bump on breaking changes**: rename, remove, or change-type any existing key.
- **Do NOT bump on additive changes**: new nullable keys such as `os.machine_model`, `cpu.core_topology`, `memory.ram_slots`, and `system.{motherboard,bios}` do not bump the schema version.

Top-level keys in the JSON output:

- `schema_version: u32` — added in v3.10.0 (currently 1)
- `elevated: bool` — added in v3.10.0
- `elevation_unlocks_more: bool` — added in v3.10.0; true only when platform has elevated-only data AND user is unelevated
- `os: { name, version, kernel, architecture }` — pre-existing
- `network: { hostname, machine_ip?, client_ip?, dns_servers[] }` — pre-existing
- `cpu: { processor, cores, sockets?, hypervisor?, frequency_ghz, load_1m?, load_5m?, load_15m?, gpus[] }` — pre-existing
- `disk: { used_bytes, total_bytes, percent }` — pre-existing
- `memory: { used_bytes, total_bytes, percent }` — pre-existing
- `session: { username, last_login?, uptime_seconds, shell?, terminal?, locale?, battery?, encryption? }` — `encryption` added in v3.11.0

Nullable fields (intentional `null` in JSON) per platform:

- `network.machine_ip` — null when offline
- `network.client_ip` — null when not on SSH (no `SSH_CLIENT` / `SSH_CONNECTION`)
- `cpu.sockets` — null in `--fast` mode (skipped)
- `cpu.hypervisor` — null in `--fast` mode unless platform sets it cheaply
- `cpu.load_1m` / `load_5m` / `load_15m` — null on Windows in `--fast` mode (the 200ms sleep is skipped)
- `session.last_login` — null in `--fast` mode on Windows; otherwise filled
- `session.shell` / `session.terminal` / `session.locale` / `session.battery` — null when unavailable on the platform or skipped in `--fast`
- `session.encryption` — null on macOS + Linux until A/B blocks land; null on Windows when query returns access-denied

## 5. The 7-phase development workflow (canonical — see CLAUDE.md § Development Workflow)

Every non-trivial change follows this cadence. Trivial fixes (typo, single-line patch) can skip phases 1–2 but **never** skip phase 5 (verification) or phase 6 (commit/push gates).

1. **Plan in plan mode.** Read-only, write to `~/.claude/plans/<name>.md`. Spawn parallel `Explore` agents for codebase reconnaissance and parallel `general-purpose` agents (`model: opus`) for authoritative web research (Apple docs, Microsoft Learn, kernel.org, systemd, freedesktop). Cite every source. End with `ExitPlanMode`.
2. **Task tracking upfront.** `TaskCreate` PR-level parents AND every sub-task verbatim from the plan. `TaskUpdate` to `in_progress` *before* starting and `completed` *immediately* on finish — never batched.
3. **Implement one PR at a time.** Read every file before editing. `cargo check` after each meaningful change. After each PR completes, run the full local gate.
4. **Per-PR documentation block (F.1–F.6).** CHANGELOG entry with task IDs in parens · README updates · CLAUDE.md arch notes with citations · Cargo.toml version bump · auto memory updates · TESTING.md verification log entry.
5. **Verification + Codex review.** Re-run the full local gate. For non-trivial PRs, dispatch `Agent({subagent_type: "codex:codex-rescue"})` for an independent review. **Codex's `gh pr diff` path needs the PR to actually exist** — open the PR first, then ask Codex to review.
6. **Commit + push gates.** Local commit → `git-master` agent (no ci-tester). Push to remote → `ci-tester` agent FIRST; if `[FAIL]`, fix root cause (no `--no-verify`); once `[PASS]`, hand off to `git-master` for the push. Tag a release → bump version in F.4, commit + push commits, **wait for CI to go green**, then `git tag vX.Y.Z && git push --tags`.
7. **Close out.** Mark the parent PR task `completed`. Move to next PR. Deferred PRs only run on explicit request.

---

## 6. Reproducing the local gate

```bash
cargo fmt -- --check
cargo clippy --all-targets --workspace -- -D warnings
cargo test --workspace --all-targets
cargo build --release --workspace
./target/release/tr300 --version
./target/release/tr300 --fast --json | head -5
./target/release/tr300 --ascii          # visual smoke test
```

Plus on each platform:

```bash
# 5-run --fast median check (the speed gate)
for i in 1 2 3 4 5; do time ./target/release/tr300 --fast > /dev/null; done
```

CI runs the same checks across Linux, macOS ARM, and Windows. Intel macOS x86_64 was dropped from CI on 2026-04-28 because `macos-13` runner capacity has effectively retired (see `CLAUDE.md` § _Intel macOS coverage policy_); cargo-dist still builds the Intel binary at tag time. Local fmt/clippy/test only validates the host platform; cross-platform issues surface on remote CI runs.

---

## 7. Known limitations / honest notes

- **CPU FREQ on Intel Meteor Lake / Lunar Lake / Arrow Lake** (Core Ultra series): CPUID leaf 16h returns 0 because Intel zeroed it in microcode. `CallNtPowerInformation` reflects the active power-plan ceiling, which on these chips is honest about throttling. The user's machine (Core Ultra 7 155H) shows 1.4 GHz because the active power plan caps it at 1400 MHz — the report is correct, the machine is throttled.
- **Last login on local console sessions**: WTSConnectTime / WTSLogonTime return 0 for the local console session on most modern Windows installs (auto-login + Fast Startup mask the actual logon timestamp). The fallback derives the time from `GetTickCount64` (boot time), which is the meaningful answer for "since when have you been logged in".
- **Hyper-V detection on Win11 with VBS**: CPUID `Microsoft Hv` doesn't necessarily mean a VM — Win11 with Virtualization-Based Security runs the kernel on top of a thin Hyper-V layer. The code disambiguates via SMBIOS manufacturer (real Hyper-V VMs always have "Microsoft Corporation"; physical hosts have OEM strings), reporting `Bare Metal (Hyper-V/VBS)` for the latter.
- **BitLocker without admin**: query may return access-denied on older Win10 / domain-joined configurations. The code returns `None` gracefully and the elevation footer hint covers the gap. On Win11 Device Encryption laptops, the query usually succeeds non-admin.

---

## 7.5 Verification: when is a PR "done"?

A PR is done when ALL of these are true:

1. `cargo fmt -- --check` clean
2. `cargo clippy --all-targets --workspace -- -D warnings` clean
3. `cargo test --workspace` — every test passes (no `--ignore`d, no flaky)
4. `cargo build --release --workspace` produces a working binary on the host platform
5. `tr300 --version` reports the bumped version
6. `tr300 --fast --json | jq .` parses
7. `tr300 --ascii` visually correct
8. CI on GitHub Actions is green across all 3 matrix legs (Linux glibc, macOS ARM, Windows). Intel macOS coverage moved to tag-time-only via cargo-dist (see `CLAUDE.md` § _Intel macOS coverage policy_).
9. The `--fast` median wall-clock on the host platform did not regress > 100 ms vs the prior commit
10. `report --ascii` line count did not grow (verified in `tests/integration.rs`)
11. The user was able to confirm the changed field shows the right value on their actual machine for at least one of: macOS Apple Silicon, Win11, Linux

## 7.6 Authoritative citations (full reference index)

When in doubt about an API or behavior, these are the trusted sources used during the planning research. Don't reach for blog posts or Stack Overflow answers older than ~2 years for these topics.

### Apple

- [Apple Developer Forums #652667](https://developer.apple.com/forums/thread/652667) — detecting Apple Silicon under Rosetta
- [Apple Developer Forums #664774](https://developer.apple.com/forums/thread/664774) — P/E core counts via `hw.perflevel*`
- [Apple Developer Forums #671792](https://developer.apple.com/forums/thread/671792) — Apple Silicon CPU frequency (`hw.cpufrequency` removed)
- [Apple SCDNSConfiguration docs](https://developer.apple.com/library/archive/documentation/Networking/Conceptual/SystemConfigFrameworks/SC_DNSConfiguration/SC_DNSConfiguration.html) — why `scutil --dns` beats resolv.conf on macOS
- [Apple vm_statistics64](https://developer.apple.com/documentation/kernel/vm_statistics64_data_t) — memory formula
- [Apple Activity Monitor docs](https://support.apple.com/guide/activity-monitor/view-memory-usage-actmntr1004/mac) — what "Memory Used" means
- [Apple Support 102888](https://support.apple.com/en-us/102888) — battery cycle count
- [Apple Support 102767](https://support.apple.com/en-us/102767) — IOPlatformSerialNumber privacy
- [Apple Rosetta security guide](https://support.apple.com/guide/security/rosetta-2-on-a-mac-with-apple-silicon-secebb113be1/web)

### Microsoft Learn

- [IsWow64Process2](https://learn.microsoft.com/en-us/windows/win32/api/wow64apiset/nf-wow64apiset-iswow64process2)
- [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex)
- [CallNtPowerInformation](https://learn.microsoft.com/en-us/windows/win32/api/powerbase/nf-powerbase-callntpowerinformation)
- [GlobalMemoryStatusEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-globalmemorystatusex)
- [GetAdaptersAddresses](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses)
- [GetSystemPowerStatus](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getsystempowerstatus)
- [WTSQuerySessionInformationW](https://learn.microsoft.com/en-us/windows/win32/api/wtsapi32/nf-wtsapi32-wtsquerysessioninformationw)
- [Event 4624](https://learn.microsoft.com/en-us/previous-versions/windows/it-pro/windows-10/security/threat-protection/auditing/event-4624)
- [Win32_Tpm](https://learn.microsoft.com/en-us/windows/win32/secprov/win32-tpm)
- [Win32_EncryptableVolume.GetEncryptionMethod](https://learn.microsoft.com/en-us/windows/win32/secprov/getencryptionmethod-win32-encryptablevolume)
- [Confirm-SecureBootUEFI](https://learn.microsoft.com/en-us/powershell/module/secureboot/confirm-securebootuefi)
- [HKLM CurrentVersion key](https://learn.microsoft.com/en-us/windows/win32/com/hkey-local-machine-software-microsoft-windows-nt-currentversion)
- [Microsoft Q&A: Win11 ProductName still says Windows 10](https://learn.microsoft.com/en-us/answers/questions/555857/windows-11-product-name-in-registry)
- [Microsoft Q&A: Fast Startup boot time issue](https://learn.microsoft.com/en-us/answers/questions/1443763/how-to-get-oss-start-time-when-fast-startup-mode-i)
- [DXGI_ADAPTER_DESC1](https://learn.microsoft.com/en-us/windows/win32/api/dxgi/ns-dxgi-dxgi_adapter_desc1)

### Linux / kernel / freedesktop / systemd

- [os-release(5)](https://man7.org/linux/man-pages/man5/os-release.5.html)
- [systemd-resolved.service(8)](https://man7.org/linux/man-pages/man8/systemd-resolved.service.8.html)
- [systemd-detect-virt(1)](https://man7.org/linux/man-pages/man1/systemd-detect-virt.1.html)
- [proc_loadavg(5)](https://man7.org/linux/man-pages/man5/proc_loadavg.5.html)
- [proc_uptime(5)](https://man7.org/linux/man-pages/man5/proc_uptime.5.html)
- [proc_meminfo(5)](https://man7.org/linux/man-pages/man5/proc_meminfo.5.html)
- [Kernel sysfs-class-power](https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-class-power)
- [Kernel cgroup-v2 doc](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html)
- [POSIX locale precedence (Open Group Base Spec §8.2)](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap08.html)

### Other primary sources

- [eclecticlight sysctl reference](https://eclecticlight.co/sysctl-information/) — Apple Silicon sysctl values
- [eclecticlight Oct 2025 frequency table](https://eclecticlight.co/2025/10/28/updating-cpu-frequencies-for-apple-silicon-macs/) — M1/M2/M3/M4 max frequencies
- [eclecticlight: Why Big Sur won't stumble over version numbers](https://eclecticlight.co/2020/07/06/why-big-sur-wont-stumble-over-version-numbers/) — `SYSTEM_VERSION_COMPAT` caveat
- [psutil #1892](https://github.com/giampaolo/psutil/issues/1892) — Apple Silicon CPU freq
- [psutil #1277](https://github.com/giampaolo/psutil/issues/1277) — macOS memory formula
- [cpufetch #139](https://github.com/Dr-Noob/cpufetch/issues/139) — Apple Silicon CPU brand
- [gopsutil PR #999](https://github.com/shirou/gopsutil/pull/999) — Apple Silicon freq fallback
- [scriptingosx — Mac marketing name](https://scriptingosx.com/2017/11/get-the-marketing-name-for-a-mac/)
- [raspberrypi/linux #3991](https://github.com/raspberrypi/linux/issues/3991) — aarch64 /proc/cpuinfo "model name" missing
- [Ubuntu LP#1919478](https://bugs.launchpad.net/ubuntu/+source/linux-raspi/+bug/1919478) — same issue on Ubuntu/RPi
- [LWN: CPUID hypervisor detection](https://lwn.net/Articles/301888/)
- [OSDev wiki: CPUID for hypervisor detection](https://wiki.osdev.org/CPUID#CPUID_for_hypervisor_detection)
- [Intel SDM Vol. 2A, CPUID Leaf 16H](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html) — Processor Frequency Information
- [thewayeye.net — primary interface under VPN](https://thewayeye.net/posts/underlying-physical-network-interface-vpn/)
- [happymacadmin — pmset battery](https://happymacadmin.wordpress.com/2024/10/10/read-a-macs-battery-health-with-the-pmset-utility/)
- [osxdaily Jan 2024 — battery cycle count CLI](https://osxdaily.com/2024/01/03/how-to-check-battery-capacity-cycle-count-from-command-line-on-mac/)
- [alansiu.net — AppleLocale](https://www.alansiu.net/2024/06/01/getting-the-macos-selected-region-via-command-line/)
- [Tabby #702](https://github.com/Eugeny/tabby/issues/702) — terminal env var
- [Warp #6990](https://github.com/warpdotdev/warp/issues/6990)
- [Ghostty discussions #7633](https://github.com/ghostty-org/ghostty/discussions/7633)

### Out-of-band research files (NOT committed; on the original Windows host only)

These are large research dumps from the planning phase. Useful as reference but not authoritative — the inline citations above are what matters.

- `~/.claude/plans/take-a-look-around-elegant-stonebraker.md` — full original plan with sections per platform, sub-task IDs, files to modify, testing strategy, phasing
- `~/.claude/plans/take-a-look-around-elegant-stonebraker-agent-a331823be3c6ba4a3.md` — Linux research dump
- `~/.claude/plans/take-a-look-around-elegant-stonebraker-agent-a93ada108401bdab8.md` — Windows research dump

If you need to reproduce the original research from scratch on a different machine, dispatch parallel `general-purpose` agents (model: `opus`) with WebFetch / WebSearch / Firecrawl / Perplexity access against per-platform briefs. The original briefs are in the `tr300-dev-workflow` skill, Phase 1 step 3 (`.claude/skills/tr300-dev-workflow/SKILL.md`).

## 8. Pickup instructions for a fresh session

1. `git pull origin master` to get the latest commits (`58812cc` and `3a252df` should both be present).
2. Read this file (`MASTER_PLAN.md`) end-to-end.
3. Read `CLAUDE.md` § "Development Workflow" and § "CI".
4. Run the local gate (Section 6 above) — confirm it's green on your machine.
5. Check `gh run list --workflow=ci.yml --limit=3` (or the Actions tab) — confirm CI on `3a252df` has gone green.
6. If not green, fix-forward (don't amend). If green and tags haven't been pushed: `git tag v3.10.0 58812cc && git tag v3.11.0 3a252df && git push --tags`.
7. Pick the next PR (Section 1 "Recommended next steps") and follow the 7-phase workflow (Section 5).
8. Update this file's Section 1 status snapshot when you ship a PR. Append to TESTING.md verification log too.

If you're on a Mac, prioritize PR #2. If you're on Linux, PR #3. If you're on Windows, PR #4b or PR #5.
