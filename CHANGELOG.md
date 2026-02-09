# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
