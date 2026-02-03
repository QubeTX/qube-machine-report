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
cargo clippy             # Run linter
cargo fmt                # Format code

# Run
cargo run                # Run with default options
cargo run -- --ascii     # ASCII mode
cargo run -- --json      # JSON output
cargo run -- --help      # Show help
```

## CLI Flags

| Flag | Description |
|------|-------------|
| `--ascii` | Use ASCII instead of Unicode |
| `--json` | Output in JSON format |
| `-t, --title <TITLE>` | Custom title |
| `--no-color` | Disable colors |
| `--install` | Add to shell profile (removes TR-100/TR-200 legacy configs first) |
| `--uninstall` | Remove from shell profile |

## TR-200 Output Format

The report matches TR-200 exactly:

```
┌┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┐
├┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┤
│       SHAUGHNESSY V DEVELOPMENT INC.          │
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

### Pre-release Checklist

1. Run tests: `cargo test`
2. Run linter: `cargo clippy`
3. Update version in `Cargo.toml`
4. Update `CHANGELOG.md` with all changes
5. Commit with message: `release: vX.Y.Z`
6. Create and push tag: `git tag vX.Y.Z && git push --tags`

## Code Patterns

### Adding a New Collector Field
1. Add field to `SystemInfo` in `src/collectors/mod.rs`
2. Collect the value in `SystemInfo::collect()`
3. Add row in `generate_table()` in `src/report.rs`

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

## Dependencies

Core:
- `sysinfo 0.32` - Cross-platform system information
- `clap 4.5` - Command-line argument parsing
- `crossterm 0.28` - Terminal width detection
- `thiserror 2.0` - Error type derivation
- `dirs 5.0` - Standard directory paths

Platform-specific:
- `wmi 0.14`, `winapi 0.3` (Windows)
- `libc 0.2`, `users 0.11` (Unix)

## Legacy Reference

The `TR200-OLD/` directory contains the original TR-200 implementation in bash/PowerShell. Key files for reference:
- `machine_report.sh` - Output format, bar graph logic
- `WINDOWS/TR-200-MachineReport.ps1` - WMI queries
- `install.sh` - Shell profile modification patterns
