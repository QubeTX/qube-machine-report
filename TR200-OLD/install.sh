#!/bin/bash
# TR-200 Machine Report - Automated Installation Script
# Copyright 2026, ES Development LLC (https://emmetts.dev)
# Based on original work by U.S. Graphics, LLC (BSD-3-Clause)

set -e  # Exit on error

echo "=========================================="
echo "TR-200 Machine Report Installation"
echo "=========================================="
echo ""

# Detect OS type
if [[ "$OSTYPE" == "darwin"* ]]; then
    OS_TYPE="macos"
    OS_NAME="macos"
    if command -v sw_vers &> /dev/null; then
        MACOS_VERSION=$(sw_vers -productVersion 2>/dev/null)
        echo "✓ Detected OS: macOS ${MACOS_VERSION}"
    else
        echo "✓ Detected OS: macOS (version unknown)"
    fi
    IS_RPI=0
elif [ -f /etc/os-release ]; then
    OS_TYPE="linux"
    . /etc/os-release
    OS_NAME="${ID}"
    OS_VERSION="${VERSION_ID}"
    echo "✓ Detected OS: ${PRETTY_NAME}"

    # Special handling for Raspberry Pi OS detection
    if [ -f /proc/device-tree/model ]; then
        RPI_MODEL=$(cat /proc/device-tree/model 2>/dev/null | tr -d '\0')
        if [[ "$RPI_MODEL" == *"Raspberry Pi"* ]]; then
            echo "✓ Raspberry Pi detected: $RPI_MODEL"
            IS_RPI=1
        else
            IS_RPI=0
        fi
    else
        IS_RPI=0
    fi
else
    OS_TYPE="unknown"
    echo "⚠ Warning: Could not detect OS. Proceeding anyway..."
    OS_NAME="unknown"
    IS_RPI=0
fi

# Check architecture
ARCH=$(uname -m)
echo "✓ Architecture: ${ARCH}"

# Check Bash version
BASH_MAJOR="${BASH_VERSINFO[0]}"
BASH_MINOR="${BASH_VERSINFO[1]}"
echo "✓ Bash version: ${BASH_VERSION}"

if [ "$BASH_MAJOR" -lt 4 ]; then
    echo ""
    echo "⚠ WARNING: Bash 4.0+ is recommended for best compatibility"
    echo "  Current version: ${BASH_VERSION}"
    if [ "$OS_TYPE" = "macos" ]; then
        echo ""
        echo "  macOS ships with Bash 3.2 by default."
        echo "  For full compatibility, install Bash 4+ via Homebrew:"
        echo "    brew install bash"
        echo ""
        echo "  The script will still work but may show some fields differently."
    fi
    echo ""
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Installation cancelled."
        exit 1
    fi
fi
echo ""

# ============================================================================
# CLEANUP FUNCTIONS - Remove all previous TR-100/TR-200 installations
# ============================================================================

# Clean a single profile file of ALL TR-100/TR-200 entries
# Uses awk for robust multi-line block removal
clean_profile_file() {
    local profile="$1"

    if [ ! -f "$profile" ]; then
        return 1
    fi

    # Check if any Machine Report markers exist
    if ! grep -qE "(TR-100|TR-200|Run Machine Report only when in interactive mode|machine_report\.sh|machine_report)" "$profile" 2>/dev/null; then
        return 2  # No cleanup needed
    fi

    echo "  Cleaning: $profile"
    local backup="${profile}.tr-backup"
    cp "$profile" "$backup"

    # Use awk for robust multi-line block removal
    awk '
    BEGIN { skip = 0 }

    # Skip TR-100 block (original upstream pattern)
    /# Run Machine Report only when in interactive mode/ { skip = 1; next }

    # Skip TR-200 configuration blocks
    /# TR-200 Machine Report configuration/ { skip = 1; next }
    /# TR-200 Machine Report - run on login/ { skip = 1; next }
    /# TR-200 Machine Report - run on bash login/ { skip = 1; next }

    # Skip npm-installed TR-200 block
    /# TR-200 Machine Report \(npm\) - auto-run/ { skip = 1; next }

    # End of if block (handles indented fi too)
    skip && /^[[:space:]]*fi[[:space:]]*$/ { skip = 0; next }

    # Skip standalone alias lines (only when not inside a block)
    !skip && /^[[:space:]]*alias report=.*machine_report/ { next }
    !skip && /^[[:space:]]*alias uninstall=.*machine_report/ { next }

    # Skip TR-100 style alias comment
    !skip && /^# Machine Report alias/ { next }

    # Print lines we are not skipping
    !skip { print }
    ' "$backup" > "${profile}.tmp"

    # Clean up consecutive blank lines (more than 2 in a row)
    cat -s "${profile}.tmp" > "$profile"
    rm -f "${profile}.tmp" "$backup"

    return 0
}

# Clean all known profile files
clean_all_profiles() {
    local profiles=(
        "$HOME/.bashrc"
        "$HOME/.zshrc"
        "$HOME/.profile"
        "$HOME/.bash_profile"
        "$HOME/.zprofile"
    )

    local cleaned=0
    for profile in "${profiles[@]}"; do
        if clean_profile_file "$profile"; then
            cleaned=$((cleaned + 1))
        fi
    done

    if [ "$cleaned" -gt 0 ]; then
        echo "  Cleaned $cleaned profile file(s)"
    fi
}

# Remove systemd service (Linux)
clean_systemd_service() {
    if [ "$OS_TYPE" != "linux" ]; then
        return
    fi

    if ! command -v systemctl &> /dev/null; then
        return
    fi

    # Check for TR-200 service
    local service_file="$HOME/.config/systemd/user/tr200-report.service"
    if [ -f "$service_file" ]; then
        echo "  Removing systemd service: tr200-report.service"
        systemctl --user disable tr200-report.service 2>/dev/null || true
        systemctl --user stop tr200-report.service 2>/dev/null || true
        rm -f "$service_file"
        systemctl --user daemon-reload 2>/dev/null || true
    fi

    # Check for potential TR-100 service names
    for svc_name in "machine-report.service" "machine_report.service"; do
        local svc_file="$HOME/.config/systemd/user/$svc_name"
        if [ -f "$svc_file" ]; then
            echo "  Removing systemd service: $svc_name"
            systemctl --user disable "$svc_name" 2>/dev/null || true
            systemctl --user stop "$svc_name" 2>/dev/null || true
            rm -f "$svc_file"
            systemctl --user daemon-reload 2>/dev/null || true
        fi
    done
}

# Remove LaunchAgent (macOS)
clean_launchd_agent() {
    if [ "$OS_TYPE" != "macos" ]; then
        return
    fi

    local agents_dir="$HOME/Library/LaunchAgents"

    # TR-200 plist
    local tr200_plist="$agents_dir/com.tr200.report.plist"
    if [ -f "$tr200_plist" ]; then
        echo "  Removing LaunchAgent: com.tr200.report.plist"
        launchctl unload "$tr200_plist" 2>/dev/null || true
        rm -f "$tr200_plist"
    fi

    # Potential TR-100 plist names
    for plist_name in "com.usgraphics.machine-report.plist" "com.machine-report.plist"; do
        local plist="$agents_dir/$plist_name"
        if [ -f "$plist" ]; then
            echo "  Removing LaunchAgent: $plist_name"
            launchctl unload "$plist" 2>/dev/null || true
            rm -f "$plist"
        fi
    done
}

# Master cleanup function
perform_full_cleanup() {
    echo "=========================================="
    echo "Cleaning Previous Installations"
    echo "=========================================="
    echo ""
    echo "Checking for TR-100/TR-200 configurations..."

    clean_all_profiles
    clean_systemd_service
    clean_launchd_agent

    # Remove old backup files
    rm -f "$HOME/.machine_report.sh.backup"
    rm -f "$HOME/.machine_report_uninstall.sh"

    echo ""
    echo "Cleanup complete"
    echo ""
}

# Check if script exists in current directory
if [ ! -f "machine_report.sh" ]; then
    echo "❌ Error: machine_report.sh not found in current directory"
    echo "   Please run this script from the repository root:"
    echo "   cd ~/git-projects/RealEmmettS-usgc-machine-report && ./install.sh"
    exit 1
fi

# ============================================================================
# PERFORM FULL CLEANUP BEFORE INSTALLATION
# ============================================================================
perform_full_cleanup

# Install dependencies based on OS
echo "=========================================="
echo "Installing Dependencies"
echo "=========================================="
echo ""

if [ "$OS_TYPE" = "macos" ]; then
    echo "macOS detected - no package installation needed"
    echo "✓ macOS has all required built-in commands (sysctl, vm_stat, etc.)"
    echo ""
    echo "Note: The script will use native macOS commands for system info."
    if [ "$BASH_MAJOR" -lt 4 ]; then
        echo ""
        echo "  Optional: For best experience, install Bash 4+:"
        echo "    brew install bash"
    fi
    echo "✓ Dependencies check complete"

elif [[ "$OS_NAME" == "debian" ]] || [[ "$OS_NAME" == "ubuntu" ]] || [[ "$OS_NAME" == "raspbian" ]]; then
    echo "Installing lastlog2 for Debian/Ubuntu/Raspberry Pi OS..."

    # Check if lastlog2 is already installed
    if command -v lastlog2 &> /dev/null; then
        echo "✓ lastlog2 already installed"
    else
        # Check if running with sudo privileges
        if [ "$EUID" -ne 0 ]; then
            echo "Note: This requires sudo privileges for package installation."

            # Raspberry Pi optimization: Use quieter apt flags to reduce output
            if [[ "$ARCH" == "aarch64" ]] || [[ "$ARCH" == "armv7l" ]]; then
                echo "  Detected ARM architecture - using optimized installation..."
                sudo apt-get update -qq > /dev/null 2>&1
                sudo apt-get install -y -qq lastlog2 > /dev/null 2>&1 && \
                    echo "✓ lastlog2 installed successfully" || \
                    echo "⚠ Warning: lastlog2 installation had issues, but script will continue"
            else
                sudo apt update
                sudo apt install -y lastlog2
            fi
        else
            # Running as root
            if [[ "$ARCH" == "aarch64" ]] || [[ "$ARCH" == "armv7l" ]]; then
                apt-get update -qq > /dev/null 2>&1
                apt-get install -y -qq lastlog2 > /dev/null 2>&1 && \
                    echo "✓ lastlog2 installed successfully" || \
                    echo "⚠ Warning: lastlog2 installation had issues, but script will continue"
            else
                apt update
                apt install -y lastlog2
            fi
        fi
    fi

    echo "✓ Dependencies check complete"

elif [[ "$OS_NAME" == "arch" ]] || [[ "$OS_NAME" == "manjaro" ]]; then
    echo "Arch-based system detected - no special packages needed"
    echo "✓ Script will use standard Linux commands"
    echo "✓ Dependencies check complete"

else
    echo "⚠ Unknown/Other Linux system detected"
    echo "  The script should still work with standard commands."
    echo "  If you encounter issues, check that these are available:"
    echo "    - lscpu (or fallback to /proc/cpuinfo)"
    echo "    - df"
    echo "    - uptime"
    echo "✓ Dependencies check complete"
fi

echo ""

# Install script to home directory
echo "=========================================="
echo "Installing Script"
echo "=========================================="
echo ""

TARGET_FILE="$HOME/.machine_report.sh"
UNINSTALL_FILE="$HOME/.machine_report_uninstall.sh"

if [ -f "$TARGET_FILE" ]; then
    echo "⚠ Warning: $TARGET_FILE already exists"
    read -p "  Overwrite? (y/N): " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "❌ Installation cancelled by user"
        exit 1
    fi
    echo "  Backing up existing file to ${TARGET_FILE}.backup"
    cp "$TARGET_FILE" "${TARGET_FILE}.backup"
fi

echo "Copying machine_report.sh to $TARGET_FILE..."
cp machine_report.sh "$TARGET_FILE"
chmod +x "$TARGET_FILE"
echo "✓ Script installed and made executable"
echo ""

# ============================================================================
# CONFIGURE .bashrc (interactive bash shells)
# ============================================================================
echo "=========================================="
echo "Configuring Shell Profiles"
echo "=========================================="
echo ""

BASHRC="$HOME/.bashrc"

# Check if already configured with TR-200
if grep -q "# TR-200 Machine Report" "$BASHRC" 2>/dev/null; then
    echo "✓ .bashrc already configured for TR-200 Machine Report"
else
    # Note: TR-100/TR-200 cleanup already done by perform_full_cleanup()
    echo "Adding TR-200 Machine Report configuration to $BASHRC..."
    cat >> "$BASHRC" << 'EOF'

# TR-200 Machine Report configuration
alias report='~/.machine_report.sh'
alias uninstall='~/.machine_report_uninstall.sh'

# Auto-run on interactive bash shell (clear screen first)
if [[ $- == *i* ]]; then
    clear
    ~/.machine_report.sh
fi
EOF
    echo "✓ .bashrc configured"
fi

# ============================================================================
# CONFIGURE .zshrc (interactive zsh shells - macOS default since Catalina)
# ============================================================================
ZSHRC="$HOME/.zshrc"

if [ -f "$ZSHRC" ] || [ "$OS_TYPE" = "macos" ]; then
    if grep -q "# TR-200 Machine Report" "$ZSHRC" 2>/dev/null; then
        echo "✓ .zshrc already configured for TR-200 Machine Report"
    else
        # Note: TR-100/TR-200 cleanup already done by perform_full_cleanup()
        echo "Adding TR-200 Machine Report configuration to $ZSHRC..."
        cat >> "$ZSHRC" << 'EOF'

# TR-200 Machine Report configuration
alias report='~/.machine_report.sh'
alias uninstall='~/.machine_report_uninstall.sh'

# Auto-run on interactive zsh shell (clear screen first)
if [[ -o interactive ]]; then
    clear
    ~/.machine_report.sh
fi
EOF
        echo "✓ .zshrc configured"
    fi
fi

# ============================================================================
# NOTE ON LOGIN SHELL PROFILES
# ============================================================================
echo ""
echo "Login shell profile note:"

# We intentionally do NOT add auto-run to login profiles (.profile, .zprofile, .bash_profile)
# because this causes duplicate reports:
#
# Linux: .profile sources .bashrc, so .bashrc alone handles SSH login
# macOS: .zprofile + .zshrc both run on SSH, so .zshrc alone is sufficient
#
# The auto-run in .bashrc/.zshrc handles all interactive sessions including SSH.

if [ "$OS_TYPE" = "macos" ]; then
    echo "✓ macOS: .zshrc handles SSH login (interactive zsh session)"
else
    echo "✓ Linux: .bashrc handles SSH login (via .profile sourcing .bashrc)"
fi

# ============================================================================
# CONFIGURE SYSTEMD USER SERVICE (Linux boot-time execution)
# ============================================================================
echo ""
if [ "$OS_TYPE" = "linux" ] && command -v systemctl &> /dev/null; then
    echo "Configuring systemd user service for boot-time execution..."
    SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
    mkdir -p "$SYSTEMD_USER_DIR"

    cat > "$SYSTEMD_USER_DIR/tr200-report.service" << EOF
[Unit]
Description=TR-200 Machine Report at Boot
After=network.target

[Service]
Type=oneshot
ExecStart=$HOME/.machine_report.sh
StandardOutput=journal

[Install]
WantedBy=default.target
EOF

    systemctl --user daemon-reload 2>/dev/null || true
    systemctl --user enable tr200-report.service 2>/dev/null || true
    echo "✓ Systemd user service enabled (runs at boot)"
    echo "  View with: systemctl --user status tr200-report"
fi

# ============================================================================
# CONFIGURE MACOS LAUNCHD AGENT (login-time execution)
# ============================================================================
if [ "$OS_TYPE" = "macos" ]; then
    echo "Configuring macOS LaunchAgent for login-time execution..."
    LAUNCHD_DIR="$HOME/Library/LaunchAgents"
    mkdir -p "$LAUNCHD_DIR"

    cat > "$LAUNCHD_DIR/com.tr200.report.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.tr200.report</string>
    <key>ProgramArguments</key>
    <array>
        <string>$HOME/.machine_report.sh</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/tr200-report.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/tr200-report.err</string>
</dict>
</plist>
EOF

    # Load the agent (may fail if already loaded, that's OK)
    launchctl unload "$LAUNCHD_DIR/com.tr200.report.plist" 2>/dev/null || true
    launchctl load "$LAUNCHD_DIR/com.tr200.report.plist" 2>/dev/null || true
    echo "✓ macOS LaunchAgent configured (runs at login)"
fi

# ============================================================================
# CREATE UNINSTALL SCRIPT
# ============================================================================
echo ""
echo "Creating uninstall script..."

cat > "$UNINSTALL_FILE" << 'UNINSTALL_EOF'
#!/bin/bash
# TR-200 Machine Report Uninstaller
# Copyright 2026, ES Development LLC (https://emmetts.dev)

echo "=========================================="
echo "TR-200 Machine Report Uninstaller"
echo "=========================================="
echo ""

read -p "Are you sure you want to uninstall TR-200 Machine Report? (y/N): " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Uninstallation cancelled."
    exit 0
fi

echo "Uninstalling TR-200 Machine Report..."
echo ""

# Detect OS type
if [[ "$OSTYPE" == "darwin"* ]]; then
    OS_TYPE="macos"
else
    OS_TYPE="linux"
fi

# Remove main script
if [ -f "$HOME/.machine_report.sh" ]; then
    rm -f "$HOME/.machine_report.sh"
    echo "✓ Removed ~/.machine_report.sh"
fi

# Remove from .bashrc
if [ -f "$HOME/.bashrc" ]; then
    # Create temp file without TR-200 config
    sed '/# TR-200 Machine Report/,/^fi$/d' "$HOME/.bashrc" > "$HOME/.bashrc.tmp" 2>/dev/null || true
    sed -i.bak '/alias report=/d' "$HOME/.bashrc.tmp" 2>/dev/null || \
        sed '/alias report=/d' "$HOME/.bashrc.tmp" > "$HOME/.bashrc.tmp2" && mv "$HOME/.bashrc.tmp2" "$HOME/.bashrc.tmp"
    sed -i.bak '/alias uninstall=/d' "$HOME/.bashrc.tmp" 2>/dev/null || \
        sed '/alias uninstall=/d' "$HOME/.bashrc.tmp" > "$HOME/.bashrc.tmp2" && mv "$HOME/.bashrc.tmp2" "$HOME/.bashrc.tmp"
    mv "$HOME/.bashrc.tmp" "$HOME/.bashrc"
    rm -f "$HOME/.bashrc.tmp.bak" "$HOME/.bashrc.bak"
    echo "✓ Cleaned .bashrc"
fi

# Remove from .zshrc
if [ -f "$HOME/.zshrc" ]; then
    sed '/# TR-200 Machine Report/,/^fi$/d' "$HOME/.zshrc" > "$HOME/.zshrc.tmp" 2>/dev/null || true
    sed -i.bak '/alias report=/d' "$HOME/.zshrc.tmp" 2>/dev/null || \
        sed '/alias report=/d' "$HOME/.zshrc.tmp" > "$HOME/.zshrc.tmp2" && mv "$HOME/.zshrc.tmp2" "$HOME/.zshrc.tmp"
    sed -i.bak '/alias uninstall=/d' "$HOME/.zshrc.tmp" 2>/dev/null || \
        sed '/alias uninstall=/d' "$HOME/.zshrc.tmp" > "$HOME/.zshrc.tmp2" && mv "$HOME/.zshrc.tmp2" "$HOME/.zshrc.tmp"
    mv "$HOME/.zshrc.tmp" "$HOME/.zshrc"
    rm -f "$HOME/.zshrc.tmp.bak" "$HOME/.zshrc.bak"
    echo "✓ Cleaned .zshrc"
fi

# Remove from .profile
if [ -f "$HOME/.profile" ]; then
    sed '/# TR-200 Machine Report/,/^fi$/d' "$HOME/.profile" > "$HOME/.profile.tmp" 2>/dev/null || true
    mv "$HOME/.profile.tmp" "$HOME/.profile"
    echo "✓ Cleaned .profile"
fi

# Remove from .zprofile (macOS)
if [ -f "$HOME/.zprofile" ]; then
    sed '/# TR-200 Machine Report/,/^fi$/d' "$HOME/.zprofile" > "$HOME/.zprofile.tmp" 2>/dev/null || true
    mv "$HOME/.zprofile.tmp" "$HOME/.zprofile"
    echo "✓ Cleaned .zprofile"
fi

# Remove from .bash_profile (macOS)
if [ -f "$HOME/.bash_profile" ]; then
    sed '/# TR-200 Machine Report/,/^fi$/d' "$HOME/.bash_profile" > "$HOME/.bash_profile.tmp" 2>/dev/null || true
    mv "$HOME/.bash_profile.tmp" "$HOME/.bash_profile"
    echo "✓ Cleaned .bash_profile"
fi

# Remove systemd service (Linux)
if [ "$OS_TYPE" = "linux" ] && command -v systemctl &> /dev/null; then
    if [ -f "$HOME/.config/systemd/user/tr200-report.service" ]; then
        systemctl --user disable tr200-report.service 2>/dev/null || true
        systemctl --user stop tr200-report.service 2>/dev/null || true
        rm -f "$HOME/.config/systemd/user/tr200-report.service"
        systemctl --user daemon-reload 2>/dev/null || true
        echo "✓ Removed systemd user service"
    fi
fi

# Remove launchd agent (macOS)
if [ "$OS_TYPE" = "macos" ]; then
    PLIST="$HOME/Library/LaunchAgents/com.tr200.report.plist"
    if [ -f "$PLIST" ]; then
        launchctl unload "$PLIST" 2>/dev/null || true
        rm -f "$PLIST"
        echo "✓ Removed macOS LaunchAgent"
    fi
fi

# Remove backup file if exists
rm -f "$HOME/.machine_report.sh.backup"

# Remove self (the uninstall script)
SELF_PATH="$HOME/.machine_report_uninstall.sh"
echo ""
echo "=========================================="
echo "TR-200 Machine Report Uninstalled"
echo "=========================================="
echo ""
echo "Note: You may need to restart your shell or log out/in"
echo "      for all changes to take effect."
echo ""

# Remove self last
rm -f "$SELF_PATH"
UNINSTALL_EOF

chmod +x "$UNINSTALL_FILE"
echo "✓ Created uninstall script: $UNINSTALL_FILE"
echo "  Run 'uninstall' command to remove TR-200 Machine Report"

# ============================================================================
# TEST INSTALLATION
# ============================================================================
echo ""
echo "=========================================="
echo "Testing Installation"
echo "=========================================="
echo ""

if [ -x "$TARGET_FILE" ]; then
    echo "Running machine report test..."
    echo ""
    "$TARGET_FILE"
    echo ""
    echo "✓ Test successful!"
else
    echo "❌ Error: Script is not executable"
    exit 1
fi

# ============================================================================
# VALIDATE INSTALLATION
# ============================================================================
echo ""
echo "Validating cleanup..."
tr100_found=0
for profile in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile" "$HOME/.bash_profile" "$HOME/.zprofile"; do
    if [ -f "$profile" ] && grep -q "# Run Machine Report only when in interactive mode" "$profile" 2>/dev/null; then
        echo "  ⚠ Warning: TR-100 markers still in $profile"
        tr100_found=1
    fi
done

if [ "$tr100_found" -eq 0 ]; then
    echo "✓ Validation passed - no TR-100 markers found"
fi

echo ""
echo "=========================================="
echo "Installation Complete!"
echo "=========================================="
echo ""
echo "The TR-200 Machine Report will now run:"
echo "  • At system boot (via systemd/launchd)"
echo "  • At every login (SSH or local terminal)"
echo "  • On every new terminal window"
echo ""
echo "Available commands:"
echo "  report    - Run the machine report manually"
echo "  uninstall - Remove TR-200 Machine Report completely"
echo ""
echo "Customize by editing: nano ~/.machine_report.sh"
echo ""

# Platform-specific tips
if [ "$IS_RPI" -eq 1 ]; then
    echo "Raspberry Pi Specific Notes:"
    echo "  • CPU frequency now shows correctly (thanks to sysfs fallback)"
    echo "  • Script optimized for ARM64 architecture"
    echo "  • Perfect for monitoring your Pi over SSH!"
    echo ""
elif [ "$OS_TYPE" = "macos" ]; then
    echo "macOS Specific Notes:"
    echo "  • Script uses native macOS commands (sysctl, vm_stat, scutil)"
    echo "  • LaunchAgent runs report on login"
    echo "  • Last login info may not be available (macOS limitation)"
    if [ "$BASH_MAJOR" -lt 4 ]; then
        echo "  • Running with Bash 3.2 - some formatting may differ"
        echo "  • Install Bash 4+ for best experience: brew install bash"
    fi
    echo ""
fi

echo "Documentation:"
echo "  • README.md - Full documentation"
echo "  • CLAUDE.md - Claude Code automation guide"
echo ""
echo "Installed to: $TARGET_FILE"
echo ""
