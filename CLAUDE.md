# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TR-300 is a cross-platform system information report tool written in Rust. It displays system information in the exact TR-200 format using Unicode box-drawing tables with bar graphs for resource usage.

**Binary name:** `tr300`
**Convenience alias:** `report` (created by `--install`)

## Architecture

```
src/
├── main.rs              # CLI entry point with clap argument parsing
├── lib.rs               # Library exports and convenience functions
├── config.rs            # Configuration constants (TR-200 widths, chars)
├── error.rs             # Custom error types using thiserror
├── report.rs            # Report generation matching TR-200 format
├── render/
│   ├── mod.rs           # Render module exports
│   ├── table.rs         # TR-200-style Unicode box-drawing table
│   └── bar.rs           # Bar graph rendering (█░)
├── collectors/
│   ├── mod.rs           # SystemInfo struct with all TR-200 fields
│   ├── os.rs            # OS name, version, kernel, uptime
│   ├── cpu.rs           # CPU model, cores, sockets, freq, load avg
│   ├── memory.rs        # Memory total/used/percent
│   ├── disk.rs          # Disk usage, ZFS support
│   ├── network.rs       # Hostname, IP, DNS servers
│   ├── session.rs       # Username, last login, uptime
│   └── platform/
│       ├── mod.rs       # Platform-specific exports
│       ├── linux.rs     # /proc, lscpu, ZFS
│       ├── macos.rs     # sysctl, scutil
│       └── windows.rs   # WMI queries
└── install/
    ├── mod.rs           # Self-installation exports
    ├── unix.rs          # .bashrc/.zshrc modifications
    └── windows.rs       # PowerShell profile modifications
```

## Development Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build

# Test
cargo test               # Run all tests
cargo test --lib         # Run library tests only
cargo test --doc         # Run documentation tests
cargo test <test_name>   # Run a single test by name

# Lint & Format
cargo clippy             # Run linter
cargo clippy -- -D warnings  # Treat warnings as errors (CI mode)
cargo fmt                # Format code
cargo fmt -- --check     # Check formatting without modifying

# Run
cargo run                # Run with default options
cargo run -- --ascii     # ASCII mode
cargo run -- --json      # JSON output
cargo run -- --help      # Show help
cargo run -- --install   # Test installation (modifies shell profiles)
```

## CLI Flags

| Flag | Description |
|------|-------------|
| `--ascii` | Use ASCII instead of Unicode |
| `--json` | Output in JSON format |
| `-t, --title <TITLE>` | Custom title |
| `--no-color` | Disable colors |
| `--fast` | Fast mode: skip slow collectors for quick auto-run |
| `--install` | Add to shell profile (removes TR-100/TR-200 legacy configs first) |
| `--uninstall` | Interactive uninstall with three options (see Installation System below) |

## Installation System

The `--install` and `--uninstall` flags modify shell profiles to add a `report` alias and auto-run TR-300 on new shell sessions.

### Installation Process (`--install`)
1. **Cleanup** - Removes legacy TR-100/TR-200 configurations
2. **Remove existing** - Cleans up any previous TR-300 installation blocks
3. **Add alias** - Creates `alias report='tr300'`
4. **Add auto-run** - Executes `tr300 --fast` on new interactive shells

**Platform-specific modifications:**
- **Unix/macOS** - Modifies `~/.bashrc` and/or `~/.zshrc` (see `src/install/unix.rs`)
- **Windows** - Modifies PowerShell profile at `$PROFILE` (see `src/install/windows.rs`)

Installation blocks are wrapped in markers:
```bash
# BEGIN TR-300 AUTO-CONFIGURATION
...
# END TR-300 AUTO-CONFIGURATION
```

### Uninstallation Process (`--uninstall`)
Interactive prompt with three options:
1. **Remove auto-run only** - Removes shell profile modifications, keeps binary
2. **Uninstall TR-300 entirely** - Removes shell profile AND deletes the binary
3. **Cancel** - Abort operation

Complete uninstall (option 2) requires confirmation and shows:
- Binary location that will be deleted
- Parent directory that will be removed (Windows only, if empty)

## Fast Mode (`CollectMode::Fast`)

The `--fast` flag activates `CollectMode::Fast`, which skips slow platform-specific collectors for sub-second startup. Auto-run (`--install`) uses `tr300 --fast`; the manual `report` alias runs full mode.

**What fast mode skips vs includes per platform:**

| Collector | Linux | macOS | Windows |
|-----------|-------|-------|---------|
| GPU | Included (lspci ~10-20ms) | Included (ioreg ~20-40ms) | Included (registry ~5-10ms) |
| Boot mode | Skipped | Skipped | Skipped |
| Virtualization | Included (/proc) | Skipped (system_profiler) | Skipped (WMI) |
| Display resolution | Skipped (xrandr) | Skipped (system_profiler) | Skipped |
| Windows edition | N/A | N/A | Skipped (WMI) |
| Shell/Terminal | Included | Included | Terminal only (env vars) |
| Battery/Locale | Included | Included | Skipped |

## TR-200 Output Format

The report matches TR-200 exactly:

```
┌┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┐
├┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┤
│          QUBETX DEVELOPER TOOLS               │
│           TR-300 MACHINE REPORT               │
├──────────────┬────────────────────────────────┤
│ OS           │ <OS name + version>            │
│ KERNEL       │ <kernel>                       │
├──────────────┼────────────────────────────────┤
...
└──────────────┴────────────────────────────────┘
```

Key elements:
- Header with `┌┬┬┬...┬┐` then `├┴┴┴...┴┤`
- Centered title and subtitle
- Two-column layout with 12-char labels, 32-char data
- Section dividers with `├───┼───┤`
- Bar graphs using `█` (filled) and `░` (empty)
- Footer with `├───┴───┤` then `└───┴───┘`

## Release Process

Uses **cargo-dist** for fully automated cross-platform releases. No manual NPM packages or Homebrew taps.

**IMPORTANT:** Every release requires ALL of these steps - do not skip any:

1. **Bump version** in `Cargo.toml`
2. **Update CHANGELOG.md** with new version section and changes
3. **Commit** with message `release: vX.Y.Z`
4. **Create git tag** matching the version (e.g., `v3.0.1`)
5. **Push commits AND tags** - the tag push triggers the release

### Quick Release Commands

```bash
# 1. Update version in Cargo.toml (e.g., 3.0.1)
# 2. Update CHANGELOG.md with changes
# 3. Commit, tag, and push:
git add -A && git commit -m "release: v3.0.1"
git tag v3.0.1
git push && git push --tags
```

GitHub Actions will automatically:
1. Build binaries for all target platforms
2. Generate installers (shell, PowerShell, MSI)
3. Create a GitHub Release with all artifacts
4. Generate release notes from CHANGELOG.md

### Target Platforms
| Platform | Target Triple |
|----------|---------------|
| Windows x64 | `x86_64-pc-windows-msvc` |
| macOS Intel | `x86_64-apple-darwin` |
| macOS Apple Silicon | `aarch64-apple-darwin` |
| Linux x64 (glibc) | `x86_64-unknown-linux-gnu` |
| Linux x64 (musl) | `x86_64-unknown-linux-musl` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` |

### Installers Generated

| Platform | Installer | Command |
|----------|-----------|---------|
| macOS/Linux | Shell script | `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr-300-installer.sh \| sh` |
| Windows | PowerShell script | `powershell -c "irm https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr-300-installer.ps1 \| iex"` |
| Windows | MSI installer | Download from GitHub Releases |

### cargo-dist Configuration

Configuration is in `Cargo.toml` under `[workspace.metadata.dist]`:

```toml
[workspace.metadata.dist]
cargo-dist-version = "0.30.3"
ci = "github"
installers = ["shell", "powershell", "msi"]
targets = [...]
pr-run-mode = "plan"
install-path = "CARGO_HOME"
```

### Regenerating CI Workflow

If you need to update the GitHub Actions workflow after changing dist config:

```bash
cargo dist init  # Regenerates .github/workflows/release.yml
```

## Code Patterns

### Adding a New Collector Field
1. Add field to `SystemInfo` in `src/collectors/mod.rs`
2. Collect the value in `SystemInfo::collect()`
3. Add row in `generate_table()` in `src/report.rs`
4. Add field to JSON output in `generate_json()` in `src/report.rs`

### Platform-Specific Code
Use conditional compilation:
```rust
#[cfg(target_os = "windows")]
fn get_something_windows() -> String { ... }

#[cfg(target_os = "linux")]
fn get_something_linux() -> String { ... }

#[cfg(target_os = "macos")]
fn get_something_macos() -> String { ... }
```

Platform-specific collectors are in `src/collectors/platform/`:
- **linux.rs** - Uses `/proc`, `lscpu`, ZFS commands
- **macos.rs** - Uses `sysctl`, `scutil` for system queries
- **windows.rs** - Uses WMI queries via the `wmi` crate

### Error Handling
Custom error types defined in `src/error.rs` using `thiserror`:
```rust
use crate::error::{AppError, Result};

// Return Result<T> from functions that can fail
fn collect_info() -> Result<String> {
    // Use ? operator for error propagation
    let data = some_fallible_operation()?;
    Ok(data)
}
```

Error variants:
- `AppError::Io` - File/IO errors
- `AppError::Platform` - Platform-specific failures
- `AppError::Config` - Configuration errors
- `AppError::Collection` - System info collection failures

## Dependencies

Core:
- `sysinfo 0.32` - Cross-platform system information
- `clap 4.5` - Command-line argument parsing with derive macros
- `crossterm 0.28` - Terminal width detection
- `thiserror 2.0` - Error type derivation
- `dirs 5.0` - Standard directory paths

Platform-specific:
- `wmi 0.14`, `winapi 0.3` (Windows) - Windows Management Instrumentation queries
- `libc 0.2`, `users 0.11` (Unix) - Low-level system calls and user info

Dev dependencies:
- `assert_cmd 2` - CLI testing
- `predicates 3` - Assertion helpers for tests

## Architecture Decisions

### Why Rust?
TR-300 rewrites TR-200 from bash/PowerShell to Rust for:
- **Performance** - No subprocess spawning for basic info
- **Cross-platform** - Single codebase for all platforms
- **Type safety** - Compile-time guarantees vs runtime errors
- **Maintainability** - Easier to extend than shell scripts

### Data Flow
1. **Collection** (`collectors/mod.rs`) - `SystemInfo::collect()` gathers all data
2. **Aggregation** - Individual collectors return platform-agnostic structs
3. **Rendering** (`report.rs`) - Converts `SystemInfo` to table or JSON
4. **Output** (`render/table.rs`) - Draws Unicode/ASCII tables with exact TR-200 layout

### Table Rendering
Fixed-width columns match TR-200:
- Label column: 12 characters (padded)
- Data column: 32 characters (truncated if needed)
- Total width: 52 characters (including borders)

Bar graphs normalize values to percentage (0-100%) and fill proportionally across the 32-char data width.

## Legacy Reference

The `TR200-OLD/` directory contains the original TR-200 implementation in bash/PowerShell. Key files for reference:
- `machine_report.sh` - Output format, bar graph logic
- `WINDOWS/TR-200-MachineReport.ps1` - WMI queries
- `install.sh` - Shell profile modification patterns
