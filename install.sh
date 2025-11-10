#!/bin/bash
# TR-100 Machine Report - Automated Installation Script
# Copyright © 2025, Emmett Shaughnessy (RealEmmettS)
# Based on original work by U.S. Graphics, LLC (BSD-3-Clause)

set -e  # Exit on error

echo "=========================================="
echo "TR-100 Machine Report Installation"
echo "=========================================="
echo ""

# Detect OS
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS_NAME="${ID}"
    OS_VERSION="${VERSION_ID}"
    echo "✓ Detected OS: ${PRETTY_NAME}"
else
    echo "⚠ Warning: Could not detect OS. Proceeding anyway..."
    OS_NAME="unknown"
fi

# Check architecture
ARCH=$(uname -m)
echo "✓ Architecture: ${ARCH}"
echo ""

# Check if script exists in current directory
if [ ! -f "machine_report.sh" ]; then
    echo "❌ Error: machine_report.sh not found in current directory"
    echo "   Please run this script from the repository root:"
    echo "   cd ~/git-projects/RealEmmettS-usgc-machine-report && ./install.sh"
    exit 1
fi

# Install dependencies based on OS
echo "=========================================="
echo "Installing Dependencies"
echo "=========================================="
echo ""

if [[ "$OS_NAME" == "debian" ]] || [[ "$OS_NAME" == "ubuntu" ]] || [[ "$OS_NAME" == "raspbian" ]]; then
    echo "Installing lastlog2 for Debian/Ubuntu/Raspberry Pi OS..."

    # Check if running with sudo privileges
    if [ "$EUID" -ne 0 ]; then
        echo "Note: This requires sudo privileges for package installation."
        sudo apt update
        sudo apt install -y lastlog2
    else
        apt update
        apt install -y lastlog2
    fi

    echo "✓ Dependencies installed"
else
    echo "⚠ Non-Debian system detected. Skipping lastlog2 installation."
    echo "  The script will work with legacy 'lastlog' if available."
fi

echo ""

# Install script to home directory
echo "=========================================="
echo "Installing Script"
echo "=========================================="
echo ""

TARGET_FILE="$HOME/.machine_report.sh"

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

# Configure .bashrc
echo "=========================================="
echo "Configuring .bashrc"
echo "=========================================="
echo ""

BASHRC="$HOME/.bashrc"

# Check if already configured
if grep -q "alias report=" "$BASHRC" 2>/dev/null && grep -q ".machine_report.sh" "$BASHRC" 2>/dev/null; then
    echo "✓ .bashrc already configured for Machine Report"
else
    echo "Adding Machine Report configuration to $BASHRC..."
    cat >> "$BASHRC" << 'EOF'

# Machine Report alias - run anytime with 'report' command
alias report='~/.machine_report.sh'

# Run Machine Report only when in interactive mode
if [[ $- == *i* ]]; then
    ~/.machine_report.sh
fi
EOF
    echo "✓ .bashrc configured"
fi

echo ""

# Test installation
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

echo ""
echo "=========================================="
echo "Installation Complete!"
echo "=========================================="
echo ""
echo "Next steps:"
echo "  1. Type 'report' to run the machine report anytime"
echo "  2. Open a new terminal to see it run automatically"
echo "  3. Customize by editing: nano ~/.machine_report.sh"
echo ""
echo "Documentation:"
echo "  • README.md - Full documentation"
echo "  • CLAUDE.md - Claude Code automation guide"
echo ""
echo "Installed to: $TARGET_FILE"
echo "Configured in: $BASHRC"
echo ""
echo "To uninstall:"
echo "  rm ~/.machine_report.sh"
echo "  # Then manually remove lines from ~/.bashrc"
echo ""
