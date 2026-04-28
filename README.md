# TR-300

[![CI](https://github.com/QubeTX/qube-machine-report/actions/workflows/ci.yml/badge.svg)](https://github.com/QubeTX/qube-machine-report/actions/workflows/ci.yml)
[![Release](https://github.com/QubeTX/qube-machine-report/actions/workflows/release.yml/badge.svg)](https://github.com/QubeTX/qube-machine-report/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-PolyForm%20Noncommercial-blue.svg)](LICENSE)

Cross-platform system information report with Unicode box-drawing tables.

TR-300 is the modern successor to TR-200 Machine Report, rebuilt from the ground up in Rust for performance, reliability, and beautiful terminal output.

## Features

- Cross-platform support (Windows, macOS, Linux)
- Beautiful Unicode box-drawing tables (TR-200 format)
- ASCII fallback mode for legacy terminals
- Bar graphs for CPU load, memory, and disk usage
- VPN-aware network information on Windows — `MACHINE IP` and `DNS IP` rows reflect the active default route (`GetBestInterfaceEx`-driven) so Tailscale / WireGuard / OpenVPN / corporate VPN tunnels are reported correctly instead of a coin-flip pick
- Hypervisor / virtualization detection (CPUID-based; disambiguates Win11 VBS from real VMs)
- Session info with last-login tracking (RDP-aware on Windows via WTS APIs)
- Disk encryption status (BitLocker on Win11 Device Encryption laptops)
- Fast Startup-aware uptime on Windows — when the kernel session age and the WMI cold-boot time diverge by >1h (typical on Win10/Win11 laptops with `HiberbootEnabled`), the `UPTIME` row renders both as `9d 4h 12m (session: 7h 14m)`
- JSON output for scripting
- Auto-save markdown report to Downloads folder on manual runs
- Fast mode (`--fast`) for sub-second auto-run startup
- Self-installation with shell alias and auto-run

## Installation

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

### Cargo

Requires Rust **1.95.0 or later** (run `rustup update stable` if needed —
older toolchains will fail with `rustc … is not supported by … tr-300`):

```bash
rustup update stable
cargo install tr-300
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

# Install to shell profile (adds 'report' alias + auto-run)
tr300 --install

# Remove from shell profile
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

| Option | Description |
|--------|-------------|
| `--ascii` | Use ASCII characters instead of Unicode |
| `--json` | Output in JSON format |
| `-t, --title <TITLE>` | Custom title for the report header |
| `--no-color` | Disable colored output |
| `--fast` | Fast mode: skip slow collectors for quick auto-run |
| `--no-elevation-hint` | Suppress the "Run with sudo / Administrator" footer hint |
| `--install` | Add to shell profile with alias and auto-run |
| `--uninstall` | Remove from shell profile |
| `-h, --help` | Print help information |
| `-V, --version` | Print version information |

## Elevation Tier

TR-300 detects whether it is running with elevated privileges (root on Unix /
Administrator on Windows) and surfaces additional data when it is.

- **Default (unelevated)** — the report renders exactly as it always has, with one
  small addition on Linux and Windows: a single dim line below the table noting that
  running with sudo / Administrator would unlock more details (motherboard / BIOS /
  RAM slots on Linux; BitLocker / full RDP login history on Windows). This hint is
  **never** shown during `--fast` auto-run, so the prompt-ready experience is
  unchanged.
- **Elevated** — extra rows render inline (motherboard, BIOS, RAM slot summary on
  Linux when `dmidecode` is available; BitLocker status when readable on Windows).
- **macOS** — no elevation tier; everything TR-300 needs is accessible without sudo.

To opt out of the hint, run `tr300 --no-elevation-hint` (or wire the flag into your
shell alias). The hint is rendered with ANSI dim, so respects `--no-color` as well.

The JSON output exposes this state under top-level keys `elevated` and
`elevation_unlocks_more` for scripted consumers.

## Installation to Shell Profile

Running `tr300 --install` will:

1. **Remove legacy configurations** (TR-100 and TR-200 auto-run blocks)
2. **Remove existing TR-300 configuration** (if present)
3. Add a `report` alias so you can type `report` instead of `tr300`
4. Configure auto-run on new interactive shell sessions

This means you can safely run `--install` multiple times or upgrade from TR-100/TR-200 without manual cleanup.

**On Unix/macOS:** Modifies `~/.bashrc` and/or `~/.zshrc`

**On Windows:** Modifies PowerShell profile

To remove these additions, run `tr300 --uninstall`.

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
cargo test

# Run clippy
cargo clippy

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

## Legacy

TR-300 is the successor to [TR-200 Machine Report](TR200-OLD/). The legacy implementation is preserved in the `TR200-OLD` directory for reference.
