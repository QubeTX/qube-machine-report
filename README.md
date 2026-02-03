# TR-300

[![Build Status](https://github.com/QubeTX/qube-machine-report/actions/workflows/release.yml/badge.svg)](https://github.com/QubeTX/qube-machine-report/actions)
[![License](https://img.shields.io/badge/license-BSD--3--Clause-blue.svg)](LICENSE)

Cross-platform system information report with Unicode box-drawing tables.

TR-300 is the modern successor to TR-200 Machine Report, rebuilt from the ground up in Rust for performance, reliability, and beautiful terminal output.

## Features

- Cross-platform support (Windows, macOS, Linux)
- Beautiful Unicode box-drawing tables (TR-200 format)
- ASCII fallback mode for legacy terminals
- Bar graphs for CPU load, memory, and disk usage
- Network information (hostname, IP, DNS servers)
- Hypervisor/virtualization detection
- Session info with last login tracking
- JSON output for scripting
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

```bash
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
│         SHAUGHNESSY V DEVELOPMENT INC.            │
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
| `--install` | Add to shell profile with alias and auto-run |
| `--uninstall` | Remove from shell profile |
| `-h, --help` | Print help information |
| `-V, --version` | Print version information |

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
- Rust 1.70 or later
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

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the BSD 3-Clause License - see the [LICENSE](LICENSE) file for details.

## Legacy

TR-300 is the successor to [TR-200 Machine Report](TR200-OLD/). The legacy implementation is preserved in the `TR200-OLD` directory for reference.
