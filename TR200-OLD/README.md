# [TR-200 Machine Report](https://tr200.emmetts.dev)
SKU: [TR-200](https://tr200.emmetts.dev), filed under Technical Reports (TR).

## What is it?
A machine information report by **SHAUGHNESSY V DEVELOPMENT INC.** (originally from [United States Graphics Company](https://x.com/usgraphics))

"Machine Report" is similar to Neofetch, but very basic. It's a bash script (or PowerShell on Windows) that's linked in the user's login startup script (`.bashrc`, `.zshrc`, or PowerShell profile); it displays useful machine information right in the terminal session. The report automatically displays when a user logs in, opens a new terminal, or SSHs into the machine. See installation instructions below.

<img src="https://github.com/usgraphics/TR-200/assets/8161031/2a8412dd-09de-45ff-8dfb-e5c6b6f19212" width="500" />

## 🎉 Key Features

This version includes major enhancements:

- ✅ **Cross-Platform**: Linux, macOS, Windows (PowerShell), BSD (partial)
- ✅ **Multi-Shell Support**: bash, zsh (macOS default), PowerShell
- ✅ **Auto-Run Everywhere**: Boot, login, SSH, new terminal windows
- ✅ **Clean Uninstall**: `uninstall` command removes all configurations
- ✅ **lastlog2 Support**: Works with modern Debian/Raspberry Pi OS (Trixie+)
- ✅ **Non-ZFS Support**: Works on standard ext4/APFS/NTFS/other filesystems
- ✅ **Raspberry Pi Tested**: Fully working on ARM64 systems
- ✅ **Windows Tested**: Native PowerShell implementation, fully working

### Status Update

~~‼️*** WARNING ***‼️~~

~~Alpha release, only compatible with Debian systems with ZFS root partition running as `root` user. This is not ready for public use *at all*.~~

**✅ This fork is stable and tested on:**
- Raspberry Pi OS (Debian Trixie)
- Standard Debian systems (with or without ZFS)
- Windows 10/11 (PowerShell 5.1+ and PowerShell 7+)
- Non-root user installations
- ARM64 and x86_64 architectures

## Software Philosophy
Since it is a bash script, you've got the source code. Just modify that for your needs. No need for any abstractions, directly edit the code. No modules, no DSL, no config files, none of it. Single file for easy deployment. Only abstraction that's acceptable is variables at the top of the script to customize the system, but it should stay minimal.

Problem with providing tools with a silver spoon is that you kill the creativity of the users. Remember MySpace? Let people customize the hell out of it and share it. Central theme as you'll see is this:

```
ENCOURAGE USERS TO DIRECTLY EDIT THE SOURCE
```

When you build a templating engine, a config file, a bunch of switches, etc; it adds 1) bloat 2) complexity 3) limits customization because by definition, customization template engine is going to be less featureful than the source code itself. So let the users just edit the source. Keep it well organized.

Another consideration is to avoid abstracting the source code at the expense of direct 1:1 readability. For e.g., the section "Machine Report" at the end of the bash script prints the output using `printf`—a whole bunch load of `printf` statements. There is no need to add loops or functions returning functions. What you see is roughly what will print. 1:1 mapping is important here for visual ID.

## Design Philosophy
Tabular, short, clear and concise. The tool's job is to inform the user of the current state of the system they are logging in or are operating. No emojis (except for the one used as a warning sign). No colors (as default, might add an option to add colors).

## System Compatibility

### Originally Designed For
- AMD EPYC CPU
- Debian OS
- ZFS installed on root partition
- VMWare Hypervisor

### Now Also Works On
- **Windows 10/11** (native PowerShell implementation)
- **Raspberry Pi** (ARM64 Cortex-A72)
- **Standard Linux** filesystems (ext4, btrfs, xfs, etc.)
- **Bare metal** and virtualized systems
- **Non-root** user installations

## Dependencies
- `lscpu` (usually pre-installed)
- `lastlog2` (modern Debian/Raspberry Pi OS) **OR** `lastlog` (legacy systems)

If your system is different, things might break. Look up the offending line and you can try to fix it for your specific system.

---

# Installation

## 📦 npm Install (Recommended - All Platforms)

[![npm version](https://img.shields.io/npm/v/tr200.svg)](https://www.npmjs.com/package/tr200)

**The easiest way to install TR-200 on any platform:**

```bash
npm install -g tr200
```

**Run on-demand:**
```bash
tr200
# or
report
```

**Set up auto-run on terminal startup:**
```bash
tr200 --install
```

**Remove auto-run:**
```bash
tr200 --uninstall
```

**Completely uninstall:**
```bash
tr200 --uninstall
npm uninstall -g tr200
```

**Requirements:** Node.js 14.0.0 or later

---

## Quick Reference

| Command | Description |
|---------|-------------|
| `tr200` | Run the machine report |
| `tr200 --help` | Show help |
| `tr200 --version` | Show version |
| `tr200 --install` | Set up auto-run on terminal startup |
| `tr200 --uninstall` | Remove auto-run configuration |

---

## ⚡ Alternative: install.sh (No Node.js Required)

**For users without Node.js:**

```bash
cd ~/git-projects && gh repo clone RealEmmettS/usgc-machine-report && \
cd RealEmmettS-usgc-machine-report && ./install.sh
```

The `install.sh` script handles everything: OS detection, dependency installation, backup, configuration, and auto-run setup.

---

## 🧳 Downloadable Installer Bundle (GUI-friendly)

Prefer not to touch the terminal? Download the pre-packaged zip (hosted soon at a friendly URL like `https://…/tr-200-machine-report.zip`), extract it anywhere, and double-click one of these launchers inside the extracted folder:

| Platform | Launcher | What it does |
| --- | --- | --- |
| Windows | `install_windows.exe` | Runs the PowerShell installer via the bundled ps2exe executable so it works from Explorer |
| macOS | `install_mac.command` | Opens Terminal automatically and runs `install.sh` with full macOS detection |
| Linux (Debian/Ubuntu/Arch/Fedora/etc.) | `install_linux.sh` | Finds your terminal emulator, launches it, and runs `install.sh` with all the existing distro detection |

Each launcher simply calls the same `install.sh`/`install_windows.ps1` logic already in this repo, so you get identical results without typing commands manually. Keep the extracted directory structure intact so the launchers can find the scripts.

### Building the zip yourself

Maintainers can regenerate the bundle with:

```bash
./tools/package_release.sh
```

The script creates `dist/tr-200-machine-report.zip` containing the launchers, `machine_report.sh`, documentation, and the latest Windows assets. If `pwsh` + `ps2exe` are available it also builds `install_windows.exe` automatically; otherwise it leaves the PowerShell installer in place with a warning so you can build the executable later on Windows.

---

## 🤖 Claude Code Installation

Ask Claude Code:

```
npm install -g tr200 && tr200 --install
```

---

## 🛠️ Manual Installation (Advanced)

For login sessions over ssh, reference the script `~/.machine_report.sh` in your `.bashrc` file. Make sure the script is executable by running `chmod +x ~/.machine_report.sh`.

Copy `machine_report.sh` from this repository and add it to `~/.machine_report.sh` ('.' for hidden file if you wish). Reference it in your `.bashrc` file as follows:

```bash
# This is your .bashrc file.
# Add the following lines anywhere in the file.

# Machine Report alias - run anytime with 'report' command
alias report='~/.machine_report.sh'

# Run Machine Report only when in interactive mode
if [[ $- == *i* ]]; then
    ~/.machine_report.sh
fi
```

---

## 🚀 Using the Report Command

Once installed, you can run the machine report anytime with:

```bash
report
```

Or directly:

```bash
~/.machine_report.sh
```

**Automatic display:** Machine Report will automatically appear when you open a new terminal or SSH into the machine.

---

## 🔧 Customization

Following the project's philosophy, **directly edit the source** to customize:

```bash
nano ~/.machine_report.sh
```

**Common customizations:**
- Line 15: `report_title` - Change the header text
- Line 18: `zfs_filesystem` - Set your ZFS pool name
- Lines 6-11: Adjust column widths and padding

---

## ✅ Compatibility Matrix

| System | Architecture | Filesystem | Shell | Status |
|--------|-------------|------------|-------|--------|
| **Windows 10/11** | x86_64 | NTFS | PowerShell 5.1+/7+ | ✅ **Tested** |
| **Raspberry Pi OS (Trixie)** | ARM64 | ext4 | Bash 5.x | ✅ **Tested** |
| **macOS Sonoma/Ventura** | ARM64/x86_64 | APFS | Bash 4.0+ | ✅ **Full Support** |
| macOS (default Bash 3.2) | ARM64/x86_64 | APFS | Bash 3.2 | ⚠️ Works with warnings |
| Debian 13 (Trixie) | x86_64 | ext4/ZFS | Bash 5.x | ✅ Working |
| Debian 12 (Bookworm) | x86_64 | ext4/ZFS | Bash 5.x | ✅ Working |
| Ubuntu 24.04+ | x86_64 | ext4/ZFS | Bash 5.x | ✅ Should work |
| Fedora/RHEL 9 | x86_64 | ext4/xfs/btrfs | Bash 5.x | ✅ Should work |
| Arch/Manjaro | x86_64 | ext4/btrfs | Bash 5.x | ✅ Should work |
| Alpine Linux | x86_64 | ext4 | varies | ⚠️ May need tweaks |
| BSD (FreeBSD/OpenBSD) | x86_64 | UFS/ZFS | varies | ⚠️ Partial support |

---

## 🐛 Troubleshooting

### lastlog command not found

**Solution:** Install `lastlog2` package:
```bash
sudo apt install -y lastlog2
```

This fork automatically handles both `lastlog2` (modern) and `lastlog` (legacy).

### CPU frequency shows blank

This is normal on some ARM systems where CPU frequency isn't exposed via `/proc/cpuinfo`. The script continues to work normally.

### Disk usage shows wrong partition

Edit `~/.machine_report.sh` and modify:
- Line 293: `root_partition="/"` to your desired partition

For ZFS systems, edit:
- Line 18: `zfs_filesystem="zroot/ROOT/os"` to your pool name

---

## 📝 Changelog (Fork-specific)

### v2.0.1 (2026-01-30) - **INSTALL FLAGS + PS 5.1 FIXES**
**New CLI Flags + PowerShell Compatibility**

- ✨ **`--install` flag**: Set up auto-run on terminal startup via npm
- ✨ **`--uninstall` flag**: Remove auto-run configuration cleanly
- ✨ **`--help` flag**: Show usage information (all scripts)
- ✨ **`--version` flag**: Show version information (all scripts)
- 🔧 **PowerShell 5.1 compatibility**: Fixed `[System.Net.Dns]::GetHostName()` and null-conditional operators
- 📚 **Simplified README**: npm + `--install` promoted as primary installation method

**Upgrade path:**
```bash
npm update -g tr200
tr200 --install  # Re-run to update shell config if needed
```

---

### v2.0.0 (2026-01-30) - **SHAUGHV REBRAND + NPM RELEASE**
**Complete Rebrand + Auto-Run Enhancements + npm Publishing**

- 📦 **Published to npm**: Install globally with `npm install -g tr200`
  - Package name: `tr200` (https://www.npmjs.com/package/tr200)
  - Commands: `tr200` and `report` work globally after npm install
  - Cross-platform Node.js wrapper auto-detects OS and runs appropriate script
- 🎨 **Rebranded to SHAUGHNESSY V DEVELOPMENT INC.**: New company branding throughout
- 🎨 **TR-200 MACHINE REPORT**: Updated product line designation
- 📜 **Copyright Updated**: Now under ES Development LLC (https://emmetts.dev)
- 🔄 **`uninstall` Command**: Clean removal of all configurations on all platforms
- 🐚 **zsh Support**: Full support for macOS default shell (Catalina+)
- 🔐 **Login Shell Support**: Auto-runs on SSH/console login via `.profile`/`.zprofile`
- ⚡ **Boot-Time Execution (Linux)**: systemd user service runs report at boot
- ⚡ **Boot-Time Execution (macOS)**: LaunchAgent runs report at login
- ⚡ **Boot-Time Execution (Windows)**: Task Scheduler runs report at login
- 🧹 **Clear Screen**: Screen cleared before auto-run for clean display
- 🗂️ **File Renames**: TR-100-MachineReport.ps1 → TR-200-MachineReport.ps1
- 📁 **Install Directory**: Windows now uses `$HOME\TR200` instead of `$HOME\TR100`
- 🛠️ **Enhanced Windows Installer**: Task Scheduler integration, uninstall support
- 🤖 **GitHub Actions**: Automated npm publishing on GitHub release

**npm Packaging Details:**
- Node.js wrapper (`bin/tr200.js`) detects OS and spawns bash/PowerShell
- Package includes: `machine_report.sh`, `WINDOWS/TR-200-MachineReport.ps1`
- Requires Node.js 14+ (uses `child_process.spawn()` with `stdio: 'inherit'`)
- Supports: Windows, macOS, Linux, FreeBSD, OpenBSD

**Breaking Changes:**
- Windows install directory changed from `TR100` to `TR200`
- PowerShell function names changed from `Show-TR100Report` to `Show-TR200Report`
- Users upgrading should run `uninstall` first, then reinstall

**Tested On:**
- Windows 11 (PowerShell 7+)
- Raspberry Pi OS (Debian Trixie, ARM64)
- macOS (zsh and bash shells)
- Linux (bash and zsh shells)

---

### v1.2.0-RealEmmettS (2025-11-10) - **PRODUCTION READY**
**Cross-Platform Compatibility Release**

- ✅ **Full macOS Support**: Native `sysctl`, `vm_stat`, `scutil` integration
- ✅ **Multi-Linux Support**: Works on Debian, Ubuntu, Arch, Fedora, RHEL
- ✅ **Robust Error Handling**: Graceful fallbacks, no crashes on missing commands
- ✅ **Fixed ZFS Bug**: Correct disk percentage calculation
- ✅ **ARM Improvements**: CPU frequency now displays on Raspberry Pi
- ✅ **Bash 4.0+ Support**: With Bash 3.2 compatibility warnings
- ✅ **Enhanced install.sh**: macOS detection, Bash version checking
- 🔧 **OS Detection Framework**: Automatic platform-specific command selection
- 🔧 **Helper Functions**: `command_exists`, `file_readable`, `is_ipv4`
- 📚 **Updated Documentation**: macOS installation guide, compatibility matrix

**Breaking Changes:** None - fully backward compatible

**Tested On:**
- Raspberry Pi OS (Debian Trixie, ARM64)
- macOS Sonoma (ARM64) - via analysis
- Compatible with Debian 12/13, Ubuntu 24.04+, Arch, Fedora, RHEL

### v1.1.0-RealEmmettS (2025-11-10)
- Added `lastlog2` support for modern Debian systems
- Added graceful fallback between `lastlog2` and `lastlog`
- Improved non-ZFS filesystem support
- Tested and verified on Raspberry Pi OS (ARM64)
- Added comprehensive installation documentation
- Added Claude Code optimized installation instructions
- Added `report` alias for convenient on-demand execution

### v1.0.0 (Original - US Graphics)
- Initial release
- Designed for Debian + ZFS + VMWare environments

---

## 🤝 Contributing

This project is maintained by **ES Development LLC** (https://emmetts.dev).

For the original upstream project, see: [usgraphics/usgc-machine-report](https://github.com/usgraphics/usgc-machine-report)

Feel free to:
- Fork this repository
- Submit issues
- Customize for your own needs (that's the philosophy!)

---

## 📄 License

BSD 3 Clause License. Copyright 2026, ES Development LLC (https://emmetts.dev). See [`LICENSE`](LICENSE) file for license information.

Based on original work by U.S. Graphics, LLC.
