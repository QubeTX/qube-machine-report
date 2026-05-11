# TR-300

[![CI](https://github.com/QubeTX/qube-machine-report/actions/workflows/ci.yml/badge.svg)](https://github.com/QubeTX/qube-machine-report/actions/workflows/ci.yml)
[![Release](https://github.com/QubeTX/qube-machine-report/actions/workflows/release.yml/badge.svg)](https://github.com/QubeTX/qube-machine-report/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-PolyForm%20Noncommercial-blue.svg)](LICENSE)

Cross-platform system information report with Unicode box-drawing tables.

TR-300 is a standalone Rust CLI for fast, reliable, and readable terminal machine reports.

Latest published release: [v3.14.2](https://github.com/QubeTX/qube-machine-report/releases/tag/v3.14.2) (2026-05-11), with cargo-dist artifacts for macOS, Linux, and Windows plus shell, PowerShell, MSI, and crates.io installation paths.

## Features

- Cross-platform support (Windows, macOS, Linux)
- Beautiful Unicode box-drawing tables with ASCII fallback
- ASCII fallback mode for legacy terminals
- Bar graphs for CPU load, memory, and disk usage
- VPN-aware network information on Windows — `MACHINE IP` and `DNS IP` rows reflect the active default route (`GetBestInterfaceEx`-driven) so Tailscale / WireGuard / OpenVPN / corporate VPN tunnels are reported correctly instead of a coin-flip pick
- Hypervisor / virtualization detection (CPUID-based; disambiguates Win11 VBS from real VMs)
- Session info with last-login/current-session tracking; Windows uses WTS APIs with a boot-time fallback
- Disk encryption status (BitLocker on Windows when readable)
- Fast Startup-aware uptime on Windows — when the kernel session age and the WMI cold-boot time diverge by >1h (typical on Win10/Win11 laptops with `HiberbootEnabled`), the `UPTIME` row renders both as `9d 4h 12m (session: 7h 14m)`
- 5-state battery awareness on Windows — distinguishes `AC Power` (plugged in, fully topped up), `X% (Charging)`, `X% (Plugged in)` (gaming-laptop case where peak GPU draw exceeds the brick wattage, OR firmware-limited charging modes like ThinkPad battery longevity), `X% (Discharging)`, plus `Critical` / `Low` overrides
- Smart terminal detection on Windows — env-var pre-checks (`WT_SESSION`, `TERM_PROGRAM`, `CURSOR_TRACE_ID`) then a parent-process walk via Toolhelp32 that recognizes Windows Terminal, WezTerm, Alacritty, VS Code, Cursor, Windsurf, Hyper, Tabby, Ghostty, Kitty, MinTTY, Claude Code, and Antigravity even when env vars are absent
- PowerShell 7+ ("PowerShell Core") detection on Windows — reads `HKLM\SOFTWARE\Microsoft\PowerShellCore\InstalledVersions\<GUID>\SemanticVersion` so `pwsh` users see the actual installed version instead of falling back to Windows PowerShell 5.x
- JSON output for scripting
- Auto-save markdown report to Downloads folder on manual runs
- Fast mode (`--fast`) for sub-second auto-run startup
- Positional action syntax (`tr300 update`, `tr300 install`, `tr300 uninstall`) with legacy flag compatibility
- Resilient self-update with cargo-first probing and shell/PowerShell installer fallbacks
- Conditional platform detail rows for machine model, CPU core topology, ZFS health, motherboard, BIOS, and RAM slots when the host exposes them
- Self-installation with shell alias and auto-run

## Installation

### Cargo

Requires Rust **1.95.0 or later**:

```bash
cargo install tr-300
```

### Shell (macOS/Linux)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr-300-installer.sh | sh
```

### PowerShell (Windows)

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr-300-installer.ps1 | iex"
```

### Windows Installer (.msi)

Download the latest MSI installer from the [Releases page](https://github.com/QubeTX/qube-machine-report/releases).

### Cargo from Git

Requires Rust **1.95.0 or later** (run `rustup update stable` if needed —
older toolchains will fail with `rustc … is not supported by … tr-300`):

Use the crates.io or release installers above for normal installs. For development
workflows that need an exact Git tag:

```bash
rustup update stable
cargo install --git https://github.com/QubeTX/qube-machine-report.git --tag v3.14.2
```

### From Source

```bash
git clone https://github.com/QubeTX/qube-machine-report.git
cd qube-machine-report
cargo build --release
```

## Usage

```bash
# Display system report (default)
tr300

# Use ASCII characters instead of Unicode
tr300 --ascii

# Output as JSON
tr300 --json

# Custom title
tr300 --title "MY SERVER"

# Disable colors
tr300 --no-color

# Self-update to the latest release
tr300 update
# Legacy form still works:
tr300 --update

# Install to shell profile (adds 'report' alias + auto-run)
tr300 install
# Legacy form still works:
tr300 --install

# Remove from shell profile
tr300 uninstall
# Legacy form still works:
tr300 --uninstall

# Show help
tr300 --help
```

## Example Output

```
┌┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┐
├┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┤
│            QUBETX DEVELOPER TOOLS                 │
│             TR-300 MACHINE REPORT                 │
├──────────────┬────────────────────────────────────┤
│ OS           │ Windows 11 24H2                    │
│ KERNEL       │ Windows 10.0.26200                 │
├──────────────┼────────────────────────────────────┤
│ HOSTNAME     │ DESKTOP-ABC123                     │
│ MACHINE IP   │ 192.168.1.100                      │
│ CLIENT  IP   │ Not connected                      │
│ DNS  IP 1    │ 192.168.1.1                        │
│ USER         │ emmett                             │
├──────────────┼────────────────────────────────────┤
│ PROCESSOR    │ Intel Core Ultra 7 155H            │
│ CORES        │ 22 vCPU(s) / 1 Socket(s)           │
│ HYPERVISOR   │ Bare Metal                         │
│ CPU FREQ     │ 1.4 GHz                            │
│ LOAD  1m     │ ███████░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
│ LOAD  5m     │ █████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
│ LOAD 15m     │ ████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
├──────────────┼────────────────────────────────────┤
│ VOLUME       │ 848.18/935.54 GB [90.66%]          │
│ DISK USAGE   │ █████████████████████████████░░░░░ │
├──────────────┼────────────────────────────────────┤
│ MEMORY       │ 18.00/31.52 GiB [57.1%]            │
│ USAGE        │ ██████████████████░░░░░░░░░░░░░░░░ │
├──────────────┼────────────────────────────────────┤
│ LAST LOGIN   │ Login tracking unavailable         │
│ UPTIME       │ 5h 38m                             │
├──────────────┴────────────────────────────────────┤
└──────────────┴────────────────────────────────────┘
```

## Command Line Options

Positional actions can be written without a double dash. They are mutually
exclusive with each other and with the legacy action flags.

| Action | Description |
|--------|-------------|
| `update` | Check for updates and install the latest version |
| `install` | Add to shell profile with alias and auto-run |
| `uninstall` | Remove from shell profile |

| Option | Description |
|--------|-------------|
| `--ascii` | Use ASCII characters instead of Unicode |
| `--json` | Output in JSON format |
| `-t, --title <TITLE>` | Custom title for the report header |
| `--no-color` | Disable colored output |
| `--fast` | Fast mode: skip slow collectors for quick auto-run |
| `--no-elevation-hint` | Suppress the "Run with sudo / Administrator" footer hint |
| `--update` | Legacy flag form of `tr300 update` |
| `--install` | Add to shell profile with alias and auto-run |
| `--uninstall` | Remove from shell profile |
| `-h, --help` | Print help information |
| `-V, --version` | Print version information |

## Self-Update

Both action forms update to the latest release:

```bash
tr300 update
tr300 --update
```

The updater runs a probe-and-retry chain so one missing or failing tool does not
block the update:

1. Checks the latest release on GitHub. If the installed version is current, it
   exits 0 without changing anything.
2. Tries `cargo install tr-300 --force` first when `cargo --version` succeeds.
   If `rustup` is present, it first runs `rustup update stable` best-effort so
   cargo installs against the current MSRV.
3. Falls through to the cargo-dist installer for the platform if cargo is absent
   or fails: `curl | sh` then `wget | sh` on macOS/Linux, `powershell` then
   `pwsh` on Windows.
4. Reports every skipped or failed strategy in both terminal and `--json` output.

In `--json` mode, successful updates include the legacy `"method"` field plus a
precise `"strategy"` value. Failed updates include an `"attempts"` array with each
strategy result and diagnostic message.

## Release Automation

GitHub Actions handles both release assets and crates.io publishing:

- `CI` runs formatting, clippy, tests, release builds, speed checks, audit, and
  cargo-dist planning on pushes to the default branch.
- `Crates.io Publish` runs only after `CI` succeeds for that default-branch
  commit, checks whether the manifest version is already on crates.io with a
  descriptive data-access `User-Agent`, reruns fmt/clippy/tests/package/dry-run
  with `--locked`, and publishes `tr-300` only when the repository
  `CARGO_REGISTRY_TOKEN` Actions secret is configured.
- `Release` is the cargo-dist workflow triggered by an explicit version tag such
  as `v3.14.2`; it builds the cross-platform archives and installers.

`Cargo.lock` is tracked so the crates.io publish workflow uses the same resolved
dependency set that local release verification used.

## Elevation Tier

TR-300 detects whether it is running with elevated privileges (root on Unix /
Administrator on Windows) and surfaces additional data when it is.

- **Default (unelevated)** — the report renders exactly as it always has, with one
  small addition on Linux and Windows: a single dim line below the table noting that
  running with sudo / Administrator would unlock more details (motherboard / BIOS /
  RAM slots on Linux; BitLocker status on Windows). This hint is **never** shown
  during `--fast` auto-run, so the prompt-ready experience is unchanged.
- **Elevated** — extra rows render inline (motherboard, BIOS, RAM slot summary on
  Linux when `dmidecode` is available; BitLocker status when readable on Windows).
- **macOS** — no elevation tier; everything TR-300 needs is accessible without sudo.

To opt out of the hint, run `tr300 --no-elevation-hint` (or wire the flag into your
shell alias). The hint is rendered with ANSI dim, so respects `--no-color` as well.

The JSON output exposes this state under top-level keys `elevated` and
`elevation_unlocks_more` for scripted consumers.

## Installation to Shell Profile

Running `tr300 install` or `tr300 --install` will:

1. **Remove existing TR-300 configuration** (if present)
2. Add a `report` alias so you can type `report` instead of `tr300`
3. Configure auto-run on new interactive shell sessions

This means you can safely run `tr300 install` multiple times without duplicating profile blocks.

**On Unix/macOS:** Modifies `~/.bashrc` and/or `~/.zshrc`

**On Windows:** Modifies PowerShell profile

To remove these additions, run `tr300 uninstall` or `tr300 --uninstall`.

## Building from Source

Requirements:
- Rust 1.95.0 or later (`rustup update stable`)
- Cargo

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test --workspace --all-targets

# Run clippy
cargo clippy --all-targets --workspace -- -D warnings

# Check formatting
cargo fmt --all -- --check

# Run with arguments
cargo run -- --ascii
```

### Man Page (Linux/macOS)

A man page is auto-generated during build via `clap_mangen`. After building from source:

```bash
sudo cp man/tr300.1 /usr/local/share/man/man1/
man tr300
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the PolyForm Noncommercial License 1.0.0 - see the [LICENSE](LICENSE) file for details. This license permits noncommercial use, personal use, and use by charitable organizations, educational institutions, and government agencies.
