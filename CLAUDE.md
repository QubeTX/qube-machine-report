# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**TR-200 Machine Report** is a system information tool that displays machine stats in a tabular format using Unicode box-drawing characters. Originally a bash script for Unix systems, it now includes a native Windows PowerShell implementation.

**Primary use case:** Runs automatically on terminal/SSH login to show system status; can also be invoked on-demand with the `report` command.

**Fork maintainer:** @RealEmmettS (this fork of usgraphics/usgc-machine-report)

### Key Enhancements in This Fork
- Cross-platform support: Linux, macOS, Windows (PowerShell), BSD (partial)
- `lastlog2` support for modern Debian/Raspberry Pi OS
- Non-ZFS filesystem support (ext4, btrfs, xfs, APFS, NTFS)
- ARM64 architecture (Raspberry Pi, Apple Silicon)
- Graceful fallback mechanisms for missing commands
- Native Windows PowerShell implementation

## Project Philosophy

**CRITICAL:** This project follows a "direct source editing" philosophy:
- Users directly modify the script files for customization
- NO config files, templates, DSLs, or abstraction layers
- Single-file design for easy deployment
- When users request customizations, guide them to edit specific lines in the source

## Supported Platforms

| Platform | Script | Status |
|----------|--------|--------|
| Linux (Debian/Ubuntu/Arch/Fedora) | `machine_report.sh` | Full support |
| Raspberry Pi OS (ARM64) | `machine_report.sh` | Primary target, tested |
| macOS (10.13+, Bash 4+ recommended) | `machine_report.sh` | Full support |
| Windows (PowerShell 5.1+/7+) | `WINDOWS/TR-200-MachineReport.ps1` | Full support |
| BSD (FreeBSD/OpenBSD) | `machine_report.sh` | Partial support |

## Repository Structure

```
usgc-machine-report/
├── machine_report.sh          # Main bash script (Linux/macOS/BSD)
├── install.sh                 # Automated installer (Unix)
├── install_linux.sh           # Linux GUI-friendly launcher
├── install_mac.command        # macOS double-clickable launcher
├── package.json               # npm package configuration
├── bin/
│   └── tr200.js               # Node.js CLI wrapper for npm
├── WINDOWS/
│   ├── TR-200-MachineReport.ps1   # Native PowerShell implementation
│   ├── install_windows.ps1        # Windows installer script
│   ├── build_tr200_exe.ps1        # Builds .exe from ps2exe
│   └── install_windows.exe        # Pre-built executable launcher
├── tools/
│   └── package_release.sh     # Creates distributable zip bundle
├── .github/
│   └── workflows/
│       └── npm-publish.yml    # Auto-publish to npm on release
├── .npmignore                 # Excludes dev files from npm package
├── CLAUDE.md                  # This file
├── README.md                  # User documentation
└── LICENSE                    # BSD 3-Clause
```

## Installation Commands

### npm (All Platforms - Simplest)
```bash
npm install -g tr200
```
Then run `tr200` or `report` from anywhere.

### Unix (Linux/macOS) - With Auto-Run
```bash
cd ~/git-projects && \
gh repo clone RealEmmettS/usgc-machine-report && \
cd RealEmmettS-usgc-machine-report && \
./install.sh
```

### Windows (PowerShell)
```powershell
# From repository root
pwsh -File WINDOWS/install_windows.ps1
# or
powershell -ExecutionPolicy Bypass -File WINDOWS/install_windows.ps1
```

### Test Installation
```bash
# Unix
~/.machine_report.sh
# or
report

# Windows (PowerShell)
report
# or directly
& "$HOME\TR200\TR-200-MachineReport.ps1"
```

## Script Architecture

### Unix Script (`machine_report.sh`)

**Lines 1-24:** Header, license, global configuration variables
- `report_title` (line 20): Header text displayed at top
- `zfs_filesystem` (line 23): ZFS pool name for ZFS systems

**Lines 25-68:** Cross-platform compatibility framework
- `detect_os()`: Returns "macos", "linux", "bsd", or "unknown"
- `command_exists()`: Safe command availability check
- `file_readable()`: File existence and readability check
- `is_ipv4()`: IPv4 address validation
- `OS_TYPE` global variable set at line 58
- Bash version warning (lines 61-68)

**Lines 70-284:** Utility functions
- `max_length()`: Calculates column widths
- `set_current_len()`: Sets dynamic width based on data
- `PRINT_HEADER/FOOTER/DIVIDER/DATA/CENTERED_DATA()`: Table rendering
- `bar_graph()`: Creates visual usage bars
- `get_ip_addr()`: Cross-platform IP detection

**Lines 286-313:** OS information detection (platform-specific)

**Lines 315-357:** Network information (hostname, IPs, DNS, user)

**Lines 359-423:** CPU information
- macOS: `sysctl`, ARM frequency fallback
- Linux: `lscpu`, `/proc/cpuinfo`, sysfs frequency fallback

**Lines 425-439:** Load averages (cross-platform)

**Lines 441-493:** Memory information
- macOS: `vm_stat` parsing
- Linux: `/proc/meminfo` parsing

**Lines 495-549:** Disk information
- ZFS detection and health check
- Standard filesystem via `df`

**Lines 551-617:** Last login (lastlog2/lastlog fallback) and uptime

**Lines 619-633:** Bar graph generation

**Lines 635-684:** Final output rendering

### Windows Script (`WINDOWS/TR-200-MachineReport.ps1`)

**Architecture:**
- `Get-TR200Report`: Collects all system data via CIM/WMI
- `Show-TR200Report`: Renders the table output
- `New-TR200BarGraph`: Creates visual usage bars
- `Get-TR200UptimeString`: Formats uptime display

**Data sources:**
- OS: `Win32_OperatingSystem`
- CPU: `Win32_Processor`, Performance Counters
- Memory: `Win32_OperatingSystem` (TotalVisibleMemorySize, FreePhysicalMemory)
- Disk: `Win32_LogicalDisk` (system drive)
- Network: `Get-NetIPAddress`, `Get-DnsClientServerAddress`
- Hypervisor: `Win32_ComputerSystem.HypervisorPresent`

**Installation locations:**
- Script: `$HOME\TR200\TR-200-MachineReport.ps1`
- CMD shim: `$HOME\TR200\report.cmd`
- PATH: `$HOME\TR200` added to user PATH

## Common Customizations

When users ask to customize, guide them to edit specific lines:

### Change Header Text
**Unix:** Line 20 in `~/.machine_report.sh`
```bash
report_title="YOUR CUSTOM HEADER"
```

**Windows:** Line 278 in `TR-200-MachineReport.ps1`
```powershell
ReportTitle = 'YOUR CUSTOM HEADER'
```

### Change ZFS Pool (Unix only)
Line 23:
```bash
zfs_filesystem="tank/ROOT/default"
```

### Change Disk Partition (Unix non-ZFS)
Line 524:
```bash
root_partition="/home"  # or other mount point
```

### Adjust Column Widths (Unix)
Lines 11-17:
```bash
MIN_NAME_LEN=5
MAX_NAME_LEN=13
MIN_DATA_LEN=20
MAX_DATA_LEN=32
BORDERS_AND_PADDING=7
```

## Troubleshooting

### "lastlog: command not found" (Linux)
```bash
sudo apt install -y lastlog2
```
The script automatically detects and uses whichever is available.

### CPU frequency blank on ARM
Normal on some ARM systems. To check manually:
```bash
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq
```

### Wrong disk partition shown
Edit line 524 (`root_partition`) for non-ZFS, or line 23 (`zfs_filesystem`) for ZFS.

### Script doesn't run on login
1. Verify `.bashrc` (or `.zshrc` on macOS) has the configuration
2. Check script is executable: `chmod +x ~/.machine_report.sh`
3. For Windows: verify PowerShell profile contains TR-200 configuration

### Windows: Box-drawing characters garbled
Ensure terminal supports UTF-8. The script sets `[Console]::OutputEncoding = UTF8` automatically.

## Development Guidelines

### Making Changes
1. Test on multiple platforms (Linux, macOS, Raspberry Pi, Windows)
2. Maintain single-file design - no external dependencies
3. Use helper functions (`command_exists`, `file_readable`) for safe checks
4. Update README.md changelog
5. Update this CLAUDE.md if line numbers change significantly
6. Tag releases with `vX.Y.Z-RealEmmettS` format

### IMPORTANT: Version Bump Before Release

**Before committing and pushing changes to main that should be published to npm:**

1. **Update version in `package.json`** - Increment the version number following semver:
   - Patch (x.y.Z): Bug fixes, minor tweaks
   - Minor (x.Y.0): New features, backwards compatible
   - Major (X.0.0): Breaking changes

2. **Update version in `bin/tr200.js`** - The `--version` flag output (around line 97)

3. **Update README.md changelog** - Add entry for the new version

4. **Update CLAUDE.md version info** - The "Current version" line in Version Information section

**Publishing to npm:**
- **Automatic:** Create a GitHub release with tag `vX.Y.Z` - GitHub Actions will publish to npm
- **Manual:** Run `npm login` then `npm publish --access public --provenance`

The GitHub Actions workflow (`.github/workflows/npm-publish.yml`) triggers on release creation.

### npm Trusted Publishers Setup (OIDC)

After the first publish, configure Trusted Publishers for more secure, tokenless publishing:

1. Go to https://www.npmjs.com/package/tr200/access
2. Scroll to "Trusted Publishers" section
3. Click "Add trusted publisher" → Select "GitHub Actions"
4. Enter:
   - **Owner:** `RealEmmettS`
   - **Repository:** `usgc-machine-report`
   - **Workflow:** `npm-publish.yml`
   - **Environment:** (leave blank)

Once configured:
- Publishes use OIDC authentication (no token needed)
- Provenance attestations are automatically generated
- Package shows verified build origin on npmjs.com

**Note on NPM_TOKEN:** The granular token in GitHub secrets expires after 90 days (npm security policy). This is fine—it's only needed for the initial publish. Once Trusted Publishers is configured, OIDC handles all future authentication and the token becomes irrelevant. No need to rotate it.

**References:**
- [npm Trusted Publishers docs](https://docs.npmjs.com/trusted-publishers/)
- [Generating provenance statements](https://docs.npmjs.com/generating-provenance-statements/)

### Testing Checklist
- Linux with `lastlog2` (modern Debian)
- Linux with `lastlog` (legacy systems)
- Raspberry Pi OS (ARM64)
- macOS (both Apple Silicon and Intel)
- Windows PowerShell 5.1 and 7+

### Key Areas to Verify
- OS detection works correctly
- CPU info displays (especially frequency on ARM/Apple Silicon)
- Memory stats calculate properly
- Disk usage shows correctly
- Last login detection works (lastlog2 vs lastlog vs unavailable)
- No unhandled errors or crashes

## Important Implementation Notes

### DO NOT automatically modify the script
- Install as-is; guide users to edit specific lines for customization
- Do not create wrapper scripts or config files
- Follow the "edit the source" philosophy

### Cross-Platform Helper Functions (Unix)
Always use these instead of direct command invocation:
```bash
command_exists "lscpu"     # Check if command available
file_readable "/etc/os-release"  # Check file exists and readable
is_ipv4 "$ip"              # Validate IPv4 format
```

### Platform Detection Pattern
```bash
if [ "$OS_TYPE" = "macos" ]; then
    # macOS-specific code
elif [ "$OS_TYPE" = "linux" ]; then
    # Linux-specific code
fi
```

## Building Release Package

### Zip Bundle (for direct download)
To create a distributable zip with cross-platform launchers:
```bash
./tools/package_release.sh
```
Output: `dist/tr-200-machine-report.zip`

### npm Package
The package is automatically published to npm when you create a GitHub release.

**Test locally before publishing:**
```bash
# Preview what will be included
npm pack --dry-run

# Test the package locally
npm link
tr200
npm unlink -g tr200
```

**npm package includes:**
- `bin/tr200.js` - Node.js wrapper that detects OS
- `machine_report.sh` - Unix bash script
- `WINDOWS/TR-200-MachineReport.ps1` - Windows PowerShell script

**npm package excludes (via .npmignore):**
- Install scripts (install.sh, install_windows.ps1, etc.)
- Build tools and dist folder
- Documentation (CLAUDE.md, AGENTS.md)
- GitHub workflows

## Version Information

- **Upstream:** usgraphics/usgc-machine-report (original)
- **This repo:** RealEmmettS/usgc-machine-report (enhanced)
- **Current version:** v2.0.5 (2026-01-30) - Fix npm CLI duplicate install on Linux
- **npm package:** https://www.npmjs.com/package/tr200

## License

BSD 3-Clause License
- Copyright 2026, ES Development LLC (https://emmetts.dev)
- Based on original work by U.S. Graphics, LLC

## Resources

- Repository: https://github.com/RealEmmettS/usgc-machine-report
- Original (upstream): https://github.com/usgraphics/usgc-machine-report
- Website: https://emmetts.dev
