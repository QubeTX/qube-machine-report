# Agent Guide (AGENTS.md)

This file is the working guide for AI coding agents in this repository.
Use this file as the canonical source when `AGENTS.md` and `CLAUDE.md` differ.

Last verified against source: 2026-02-09

## Project Snapshot

- Project: TR-300 (Rust rewrite of TR-200)
- Cargo package name: `tr-300`
- Current version: `3.3.0` (`Cargo.toml`)
- Binary name: `tr300`
- Convenience alias installed by `--install`: `report`
- License: PolyForm-Noncommercial-1.0.0
- Repo: `https://github.com/QubeTX/qube-machine-report`

## Repository Map

```text
src/
  main.rs                    # CLI entrypoint
  lib.rs                     # Public library exports
  config.rs                  # Config flags, widths, box char sets
  error.rs                   # AppError + Result alias
  report.rs                  # Table/JSON generation
  render/
    mod.rs
    table.rs                 # Table renderer + ReportBuilder
    bar.rs                   # Percent bar renderer
  collectors/
    mod.rs                   # SystemInfo aggregate + collection pipeline
    os.rs
    cpu.rs
    memory.rs
    disk.rs
    network.rs
    session.rs
    platform/
      mod.rs
      linux.rs
      macos.rs
      windows.rs
  install/
    mod.rs
    prompt.rs                # Interactive uninstall menu/confirm
    unix.rs                  # bash/zsh profile edits
    windows.rs               # PowerShell profile edits

tests/
  integration.rs

.github/workflows/
  release.yml                # cargo-dist generated release workflow

wix/
  main.wxs                   # MSI packaging template

TR200-OLD/                   # Legacy bash/PowerShell implementation
```

## How The Program Works

### Runtime flow (`src/main.rs`)

1. Parse CLI args with `clap`.
2. If `--install`, run install path and exit.
3. Else if `--uninstall`, run interactive uninstall path and exit.
4. Build `Config` from flags (`--ascii`, `--json`, `--no-color`, `--title`).
5. On Windows terminals, set console output code page to UTF-8.
6. Collect system data via `SystemInfo::collect()`.
7. Render output with `report::generate(...)` as table or JSON.
8. Print to stdout.

### Data pipeline (`src/collectors/mod.rs`)

`SystemInfo::collect()` orchestrates:
- `os::collect()`
- `cpu::collect()`
- `memory::collect()`
- `disk::collect()`
- `network::collect_network_info()`
- `session::collect()`
- `platform::collect()` (OS-specific enrichments)

Then it:
- Aggregates disk usage (prefer root `/` or `C:` volume, else non-removable sum, else first disk fallback).
- Computes percentages for disk and memory.
- Derives hypervisor string (`platform virtualization` or `Bare Metal`).
- Builds final `SystemInfo` struct.

### Rendering flow (`src/report.rs`)

- `Config::format == Table`: render TR-200-style table.
- `Config::format == Json`: render JSON string manually (without `serde`).

Table rendering is fixed width:
- Label column: 12 chars
- Data column: 32 chars
- Total row width with borders/spaces: 51 chars

## CLI Reference

Current supported flags:

- `--ascii` -> ASCII table + `#`/`.` bars
- `--json` -> JSON output
- `--install` -> install alias + auto-run profile block
- `--uninstall` -> interactive uninstall menu
- `-t, --title <TITLE>` -> custom title
- `--no-color` -> currently sets config only (no color styling currently applied)

Notes:
- If both `--install` and `--uninstall` are passed, `--install` wins because it is checked first.
- There are no subcommands; all behavior is flag-based.

## Output Format Details

### Table sections and row behavior

Rendered order:

1. Header block (top border, subtitle lines)
2. OS section
   - `OS`
   - `KERNEL`
   - `ARCH`
3. Network section
   - `HOSTNAME`
   - `MACHINE IP`
   - `CLIENT  IP` (`Not connected` if none)
   - `DNS  IP 1..5` (up to 5 rows)
   - `USER`
4. CPU section
   - `PROCESSOR`
   - `CORES`
   - GPU rows:
     - 1 GPU: `GPU`
     - 2-3 GPUs: `GPU 1`, `GPU 2`, `GPU 3`
     - 4+ GPUs: single `GPUs` row (comma-separated)
   - `HYPERVISOR`
   - `CPU FREQ`
   - `LOAD  1m`, `LOAD  5m`, `LOAD 15m` bars
5. Disk section
   - `VOLUME`
   - `DISK USAGE` bar
   - `ZFS HEALTH` only when available (currently not set)
6. Memory section
   - `MEMORY`
   - `USAGE` bar
7. Session section
   - `LAST LOGIN`
   - optional extra row with blank label for `last_login_ip`
   - `UPTIME`
   - optional rows: `SHELL`, `TERMINAL`, `LOCALE`, `BATTERY`
8. Footer

Renderer behavior:
- Strings are padded/truncated to fixed width with `...` ellipsis.
- Header text is centered and truncated when needed.
- Footer is single-line in current output (`render_footer()` directly, no extra bottom divider call).

Bar behavior (`src/render/bar.rs`):
- Percent clamped to `0..100`.
- Filled cells computed with rounded value of `(percent / 100) * width`.
- Table bars use width 32 (data width).

### JSON output shape

Top-level keys:
- `os`
- `network`
- `cpu`
- `disk`
- `memory`
- `session`

Important implementation detail:
- JSON is manually formatted with `format!` + `escape_json(...)`, not `serde`.
- Optional values are emitted as either string literals or `null`.

## Collector Behavior By Module

### `os.rs`

Uses `sysinfo::System` static getters for:
- OS name/version
- Kernel version
- Hostname
- Uptime seconds

Architecture defaults to `std::env::consts::ARCH`.

### `cpu.rs`

- Refreshes CPU info twice with a 200ms sleep for stable usage values.
- Brand/frequency from first CPU entry.
- Logical cores from `sys.cpus().len()`.
- Physical cores from `physical_core_count` (fallback logical).
- Sockets:
  - Linux: parse `lscpu`
  - Windows: PowerShell CIM query
  - macOS: `sysctl -n hw.packages`
  - fallback: `1`
- Load averages:
  - Unix: `/proc/loadavg` parse first, then `libc::getloadavg`, converted to percent of cores
  - Windows: uses current CPU usage for 1m/5m/15m

### `memory.rs`

Uses `sysinfo` memory/swap counters. Primary report uses RAM total/used/percent.

### `disk.rs`

Uses `sysinfo::Disks`:
- collects mount point, filesystem, total/available/used, removable flag, name
- skips disks with `total == 0`

Aggregation into report uses logic in `collectors/mod.rs`.

### `network.rs`

Machine IP:
- Windows: PowerShell `Get-NetIPAddress` fallback to `ipconfig`
- Linux: `hostname -I` fallback to `ip route get 1`
- macOS: `ipconfig getifaddr` on common interfaces + route fallback

Client IP:
- from `SSH_CLIENT` or `SSH_CONNECTION`

DNS servers:
- Windows: PowerShell `Get-DnsClientServerAddress`, fallback `ipconfig /all`
- Linux: `/etc/resolv.conf`, fallback `resolvectl status`
- macOS: `scutil --dns`
- de-duplicated, max 5

Also contains legacy interface collector (`collect()`) using `sysinfo::Networks`.

### `session.rs`

Collects:
- username
- home dir
- shell
- cwd
- terminal
- last login (+ optional login IP)

Last-login strategy:
- Linux: `lastlog2`, fallback `lastlog`, fallback `last`
- macOS: `last`
- Windows: PowerShell `Get-LocalUser`, fallback `net user`

### `collectors/platform/*`

Adds OS-specific enrichments (some not currently rendered in table):
- virtualization/hypervisor signals
- GPU names
- architecture
- shell + terminal details
- locale
- battery
- display resolution
- desktop/display server/edition/codename/boot mode metadata

## Install/Uninstall System

Entry points in `src/install/mod.rs`:
- `install()`
- `uninstall()`
- `uninstall_complete()`
- prompt helpers re-exported from `prompt.rs`

### Interactive uninstall (`--uninstall`)

Prompt options:
- `1`: profile-only cleanup
- `2`: complete uninstall (profile + binary)
- `0`: cancel

Complete uninstall requires explicit y/n confirmation and shows binary path (and Windows parent dir when applicable).

### Unix install (`src/install/unix.rs`)

Profiles touched:
- `~/.bashrc` (if exists)
- `~/.zshrc` (if exists)
- if neither modified and `.bashrc` missing, creates `.bashrc`

Injected block:

```bash
# TR-300 Machine Report
alias report='tr300'

# Auto-run on interactive shell
if [[ $- == *i* ]]; then
    tr300
fi
# End TR-300
```

Behavior:
- removes existing TR-300 block first
- removes known TR-100/TR-200 legacy blocks
- idempotent re-install behavior

### Windows install (`src/install/windows.rs`)

Profile target:
- PowerShell `$PROFILE` path (queried via `powershell -NoProfile -Command $PROFILE`)
- creates profile directory if needed

Injected block:

```powershell
# TR-300 Machine Report
Set-Alias -Name report -Value tr300

# Auto-run on interactive shell
if ($Host.Name -eq 'ConsoleHost') {
    tr300
}
# End TR-300
```

Behavior:
- removes existing TR-300 block first
- removes known TR-100/TR-200 legacy blocks (including delimited TR-200 config markers)

Complete uninstall on Windows additionally:
- deletes binary
- attempts to remove empty parent dir when path contains `tr300`

## Distribution And Release Methods

### Packaging model

Release automation uses `cargo-dist` + GitHub Actions.

`Cargo.toml` (`[workspace.metadata.dist]`):
- `cargo-dist-version = "0.30.3"`
- `ci = "github"`
- `installers = ["shell", "powershell", "msi"]`
- targets:
  - `aarch64-apple-darwin`
  - `aarch64-unknown-linux-gnu`
  - `x86_64-apple-darwin`
  - `x86_64-unknown-linux-gnu`
  - `x86_64-unknown-linux-musl`
  - `x86_64-pc-windows-msvc`
- `pr-run-mode = "plan"`
- `install-updater = false`
- `install-path = "CARGO_HOME"`
- `publish-prereleases = false`

### CI workflow (`.github/workflows/release.yml`)

High-level job flow:
1. `plan` (dist plan/host manifest)
2. `build-local-artifacts` (matrix builds)
3. `build-global-artifacts`
4. `host` (upload + release)
5. `announce`

Trigger:
- push tags matching semver-like pattern (`**[0-9]+.[0-9]+.[0-9]+*`)

### Installer outputs

- Shell installer (`tr-300-installer.sh`) for macOS/Linux
- PowerShell installer (`tr-300-installer.ps1`) for Windows
- MSI installer for Windows

### MSI specifics (`wix/main.wxs`)

- Product name: `tr-300`
- Install scope: `perMachine`
- Default folder under Program Files (`tr-300` with `bin/tr300.exe`)
- Optional PATH feature included in MSI feature tree
- Upgrade code/path GUID are defined in `Cargo.toml` metadata

### Release checklist (current process)

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run checks (`cargo test`, `cargo clippy -- -D warnings`, `cargo fmt -- --check`)
4. Commit (`release: vX.Y.Z`)
5. Tag (`vX.Y.Z`)
6. Push commits and tags

## Development Commands

```bash
# Build
cargo build
cargo build --release

# Test
cargo test
cargo test --lib
cargo test --doc
cargo test <test_name>

# Lint/format
cargo clippy
cargo clippy -- -D warnings
cargo fmt
cargo fmt -- --check

# Run
cargo run
cargo run -- --ascii
cargo run -- --json
cargo run -- --title "MY TITLE"
cargo run -- --install
cargo run -- --uninstall
```

## Error Model

`src/error.rs` defines `AppError` variants:
- `SystemInfo { message }`
- `Platform { message }`
- `Io(std::io::Error)`
- `Config { message }`
- `Display { message }`
- Windows only: `Wmi(wmi::WMIError)`

Type alias:
- `Result<T> = std::result::Result<T, AppError>`

## Dependencies

Core:
- `sysinfo = "0.32"`
- `clap = "4.5"` (derive, env)
- `crossterm = "0.28"`
- `thiserror = "2.0"`
- `dirs = "5.0"`

Platform-specific:
- Windows: `wmi = "0.14"`, `winapi = "0.3"`
- Unix: `libc = "0.2"`, `users = "0.11"`

Dev:
- `assert_cmd = "2"`
- `predicates = "3"`

## Tests

`tests/integration.rs` currently validates:
- help/version flags
- default output contains main title
- ascii mode output
- json mode output structure keys
- custom title injection
- expected key report fields

## Known Caveats / Drift Risks

- `Config` fields `use_colors`, `show_network`, `show_disks`, `width`, and `compact` exist but are not fully wired into report rendering/section toggling yet.
- `zfs_health` is currently always `None` in `SystemInfo::collect()` (explicit TODO).
- Some historical docs/examples still mention older behaviors/markers; source code is authoritative.
- Core data collection still relies on OS commands for several fields (network/session/platform), not purely `sysinfo`.

## Extension Patterns

### Add a new report field

1. Add field to `SystemInfo` (`src/collectors/mod.rs`).
2. Collect/derive value in `SystemInfo::collect()`.
3. Add table row in `generate_table()` (`src/report.rs`).
4. Add JSON field in `generate_json()` (`src/report.rs`).
5. Add/adjust integration tests in `tests/integration.rs`.

### Add or change CLI behavior

1. Update `Cli` struct in `src/main.rs`.
2. Update config wiring in `main()`.
3. Implement behavior in `run_report` or install/uninstall handlers.
4. Add tests for flag behavior.

### Add platform-specific detail

1. Extend `PlatformInfo` in `src/collectors/platform/mod.rs`.
2. Implement value for each supported OS file (`linux.rs`, `macos.rs`, `windows.rs`) with graceful fallback.
3. Surface it in table/json if user-visible.

## Legacy Reference

`TR200-OLD/` keeps the previous shell/PowerShell implementation for behavior comparison and migration context.

## Maintenance Rule

When editing this guide:
- update `AGENTS.md` and `CLAUDE.md` in the same change
- keep statements tied to current source code, not older README wording

