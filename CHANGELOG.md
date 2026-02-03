# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
