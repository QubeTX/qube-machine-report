# TR-100 Machine Report
SKU: TR-100, filed under Technical Reports (TR).

## What is it?
A machine information report used at [United States Graphics Company](https://x.com/usgraphics)

"Machine Report" is similar to Neofetch, but very basic. It's a bash script that's linked in the user's login startup script, `.bashrc` or `.bash_profile`; it displays useful machine information right in the terminal session. Typically, at U.S. Graphics Company, we use it for remote servers and Machine Report is displayed when a user logs into the server over ssh. See installation instructions on how to do this.

<img src="https://github.com/usgraphics/TR-100/assets/8161031/2a8412dd-09de-45ff-8dfb-e5c6b6f19212" width="500" />

## 🎉 RealEmmettS Fork Enhancements

This fork includes improvements for broader compatibility:

- ✅ **lastlog2 Support** - Works with modern Debian/Raspberry Pi OS (Trixie+)
- ✅ **Graceful Fallback** - Automatically detects `lastlog2` or legacy `lastlog`
- ✅ **Non-ZFS Support** - Works on standard ext4/other filesystems
- ✅ **Raspberry Pi Tested** - Fully working on ARM64 systems
- ✅ **Claude Code Optimized** - Installation instructions designed for AI automation

### Status Update

~~‼️*** WARNING ***‼️~~

~~Alpha release, only compatible with Debian systems with ZFS root partition running as `root` user. This is not ready for public use *at all*.~~

**✅ This fork is stable and tested on:**
- Raspberry Pi OS (Debian Trixie)
- Standard Debian systems (with or without ZFS)
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

## ⚡ Super Quick Install (install.sh)

**Simplest method - fully automated!**

```bash
cd ~/git-projects && gh repo clone RealEmmettS/usgc-machine-report && \
cd RealEmmettS-usgc-machine-report && ./install.sh
```

The `install.sh` script handles everything:
- Detects your OS and architecture
- Installs dependencies (lastlog2)
- Backs up existing installations
- Copies and configures the script
- Tests everything works
- Sets up the `report` alias

**That's it! Type `report` or open a new terminal to see it in action.**

---

## 🧳 Downloadable Installer Bundle (GUI-friendly)

Prefer not to touch the terminal? Download the pre-packaged zip (hosted soon at a friendly URL like `https://…/tr-100-machine-report.zip`), extract it anywhere, and double-click one of these launchers inside the extracted folder:

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

The script creates `dist/tr-100-machine-report.zip` containing the launchers, `machine_report.sh`, documentation, and the latest Windows assets. If `pwsh` + `ps2exe` are available it also builds `install_windows.exe` automatically; otherwise it leaves the PowerShell installer in place with a warning so you can build the executable later on Windows.

---

## 🤖 Claude Code Automated Installation

**Best for:** Using with [Claude Code](https://claude.ai/code) AI assistant

Ask Claude Code:

```
Install usgc-machine-report from RealEmmettS/usgc-machine-report using install.sh
```

Or provide these specific instructions:

```bash
cd ~/git-projects && \
gh repo clone RealEmmettS/usgc-machine-report && \
cd RealEmmettS-usgc-machine-report && \
./install.sh
```

Claude Code will automatically:
1. Clone this repository
2. Install `lastlog2` if on Debian/Raspberry Pi OS
3. Copy script to `~/.machine_report.sh`
4. Add to `.bashrc` with automatic run on login
5. Create convenient `report` alias

**Or provide these specific instructions to Claude Code:**

```bash
# 1. Clone the repository
cd ~/git-projects
gh repo clone RealEmmettS/usgc-machine-report

# 2. Install dependencies (Debian/Ubuntu/Raspberry Pi OS)
sudo apt install -y lastlog2

# 3. Install the script
cp ~/git-projects/RealEmmettS-usgc-machine-report/machine_report.sh ~/.machine_report.sh
chmod +x ~/.machine_report.sh

# 4. Add to .bashrc
cat >> ~/.bashrc << 'EOF'

# Machine Report alias - run anytime with 'report' command
alias report='~/.machine_report.sh'

# Run Machine Report only when in interactive mode
if [[ $- == *i* ]]; then
    ~/.machine_report.sh
fi
EOF

# 5. Test installation
~/.machine_report.sh
```

---

## 🍎 macOS Installation

**Requirements:**
- macOS 10.13 (High Sierra) or later
- Bash 4.0+ recommended (install via `brew install bash`)

**Quick Install:**

```bash
cd ~/git-projects && gh repo clone RealEmmettS/usgc-machine-report && \
cd RealEmmettS-usgc-machine-report && ./install.sh
```

**What works on macOS:**
- ✅ OS version detection (via `sw_vers`)
- ✅ CPU info (via `sysctl`)
- ✅ Memory usage (via `vm_stat`)
- ✅ Disk usage (via `df`)
- ✅ Network info (via `scutil`)
- ✅ System uptime (calculated from boot time)
- ⚠️ Last login may show "unavailable" (macOS limitation)

**macOS-Specific Notes:**
- No package installation needed - uses built-in commands
- Default Bash 3.2 works but Bash 4+ recommended
- To install newer Bash: `brew install bash`
- Script automatically detects macOS and uses appropriate commands

### 🐚 zsh Installation (macOS Default Shell)

**Modern macOS (Catalina 10.15+) uses zsh by default**, not bash. If you're using zsh, follow these instructions:

**Quick Install for zsh:**

```bash
# Clone and install the script
cd ~/Downloads && git clone https://github.com/RealEmmettS/usgc-machine-report.git && \
cp ~/Downloads/usgc-machine-report/machine_report.sh ~/.machine_report.sh && \
chmod +x ~/.machine_report.sh

# Add to .zshrc (not .bashrc!)
cat >> ~/.zshrc << 'EOF'

# Machine Report alias - run anytime with 'report' command
alias report='~/.machine_report.sh'

# Run Machine Report only when in interactive mode
if [[ $- == *i* ]]; then
    ~/.machine_report.sh
fi
EOF

# Clean up cloned repo (optional)
rm -rf ~/Downloads/usgc-machine-report

echo "✅ Installation complete! Open a new terminal or type: source ~/.zshrc"
```

**Important zsh Notes:**
- ✅ Use `~/.zshrc` instead of `~/.bashrc`
- ✅ The script works identically in zsh - no code changes needed
- ✅ Test with: `zsh -c "source ~/.zshrc && report"`
- ℹ️ To check your shell: `echo $SHELL` (should show `/bin/zsh`)

---

## 📦 Quick Install (Manual - Raspberry Pi OS / Debian)

**One-liner installation:**

```bash
cd ~/git-projects && gh repo clone RealEmmettS/usgc-machine-report && \
sudo apt install -y lastlog2 && \
cp ~/git-projects/RealEmmettS-usgc-machine-report/machine_report.sh ~/.machine_report.sh && \
chmod +x ~/.machine_report.sh && \
cat >> ~/.bashrc << 'EOF'

# Machine Report alias - run anytime with 'report' command
alias report='~/.machine_report.sh'

# Run Machine Report only when in interactive mode
if [[ $- == *i* ]]; then
    ~/.machine_report.sh
fi
EOF
echo "✅ Installation complete! Type 'report' or open a new terminal."
```

**Step-by-step installation:**

1. **Clone the repository**:
   ```bash
   cd ~/git-projects
   gh repo clone RealEmmettS/usgc-machine-report
   ```

2. **Install dependencies** (for modern Debian/Raspberry Pi OS):
   ```bash
   sudo apt install -y lastlog2
   ```

   *Note: On systems with the legacy `lastlog` command, this step is optional. The script automatically detects and uses whichever is available.*

3. **Copy the script to your home directory**:
   ```bash
   cp ~/git-projects/RealEmmettS-usgc-machine-report/machine_report.sh ~/.machine_report.sh
   chmod +x ~/.machine_report.sh
   ```

4. **Add to `.bashrc` for automatic display on login**:
   ```bash
   cat >> ~/.bashrc << 'EOF'

# Machine Report alias - run anytime with 'report' command
alias report='~/.machine_report.sh'

# Run Machine Report only when in interactive mode
if [[ $- == *i* ]]; then
    ~/.machine_report.sh
fi
EOF
   ```

5. **Test the installation**:
   ```bash
   ~/.machine_report.sh
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

| System | Architecture | Filesystem | Bash | Status |
|--------|-------------|------------|------|--------|
| **Raspberry Pi OS (Trixie)** | ARM64 | ext4 | 5.x | ✅ **Tested** |
| **macOS Sonoma/Ventura** | ARM64/x86_64 | APFS | 4.0+ | ✅ **Full Support** |
| macOS (default Bash 3.2) | ARM64/x86_64 | APFS | 3.2 | ⚠️ Works with warnings |
| Debian 13 (Trixie) | x86_64 | ext4/ZFS | 5.x | ✅ Working |
| Debian 12 (Bookworm) | x86_64 | ext4/ZFS | 5.x | ✅ Working |
| Ubuntu 24.04+ | x86_64 | ext4/ZFS | 5.x | ✅ Should work |
| Fedora/RHEL 9 | x86_64 | ext4/xfs/btrfs | 5.x | ✅ Should work |
| Arch/Manjaro | x86_64 | ext4/btrfs | 5.x | ✅ Should work |
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

This is a personal fork maintained by [@RealEmmettS](https://github.com/RealEmmettS).

For the original project, see: [usgraphics/usgc-machine-report](https://github.com/usgraphics/usgc-machine-report)

Feel free to:
- Fork this fork!
- Submit issues
- Customize for your own needs (that's the philosophy!)

---

## 📄 License

BSD 3 Clause License, Copyright © 2024, U.S. Graphics, LLC. See [`LICENSE`](LICENSE) file for license information.

Fork modifications Copyright © 2025, Emmett Shaughnessy (RealEmmettS).
