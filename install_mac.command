#!/bin/bash
# TR-100 Machine Report macOS launcher
# Double-click friendly wrapper that opens Terminal and runs install.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
INSTALLER="$SCRIPT_DIR/install.sh"

if [[ ! -f "$INSTALLER" ]]; then
  osascript -e 'tell application "Terminal" to activate' >/dev/null 2>&1 || true
  echo "❌ install.sh not found next to install_mac.command."
  echo "Please keep the extracted zip structure intact and try again."
  read -r -p "Press Return to close this window..." _
  exit 1
fi

chmod +x "$INSTALLER" >/dev/null 2>&1 || true

echo "=========================================="
echo "TR-100 Machine Report macOS Installer"
echo "=========================================="
echo ""
echo "Running $(basename "$INSTALLER") from:"
echo "  $SCRIPT_DIR"
echo ""

if "$INSTALLER"; then
  echo ""
  echo "✅ Installation finished."
else
  status=$?
  echo ""
  echo "❌ Installer exited with status $status."
  echo "Please review the log above for details."
  read -r -p "Press Return to close this window..." _
  exit "$status"
fi

echo ""
read -r -p "Press Return to close this window..." _

