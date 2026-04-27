# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
  (`Run as Administrator for BitLocker status and full login history`). macOS shows no
  hint — there is no equivalent unlock on Apple platforms. (E.1, E.7)
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
- Legacy version cleanup during installation: `tr300 --install` now automatically removes TR-100 and TR-200 configurations before installing TR-300
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
- Renamed from TR-200 to TR-300 to reflect major version bump
- Binary name changed from `tr200` to `tr300`
- Output format completely redesigned with modern Unicode tables

### Deprecated
- Legacy TR-200 bash/PowerShell implementation moved to `TR200-OLD/`

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
