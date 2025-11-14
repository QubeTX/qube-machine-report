#!/usr/bin/env bash
# TR-100 Machine Report Linux launcher
# Attempts to open a terminal window when double-clicked, then runs install.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
INSTALLER="$SCRIPT_DIR/install.sh"
LAUNCHED_IN_TERMINAL=0

function usage_note() {
  echo "This script expects install.sh to live in the same folder."
  echo "If you extracted the TR-100 zip elsewhere, keep the directory structure intact."
}

if [[ "${1:-}" == "--already-in-terminal" ]]; then
  shift
  LAUNCHED_IN_TERMINAL=1
fi

if [[ -t 1 ]]; then
  LAUNCHED_IN_TERMINAL=1
fi

if [[ $LAUNCHED_IN_TERMINAL -eq 0 ]]; then
  TERMINALS=(gnome-terminal konsole xfce4-terminal mate-terminal kitty alacritty xterm)
  for term in "${TERM_WRAPPER_OVERRIDE:-}" "${TERMINALS[@]}"; do
    [[ -z "$term" ]] && continue
    if command -v "$term" >/dev/null 2>&1; then
      case "$term" in
        gnome-terminal|mate-terminal)
          "$term" -- bash -lc "'$SCRIPT_DIR/install_linux.sh' --already-in-terminal; read -p \"Press Enter to close...\" _"
          exit $?
          ;;
        konsole)
          "$term" -e bash -lc "'$SCRIPT_DIR/install_linux.sh' --already-in-terminal; read -p \"Press Enter to close...\" _"
          exit $?
          ;;
        xfce4-terminal|kitty|alacritty|xterm)
          "$term" -e bash -lc "'$SCRIPT_DIR/install_linux.sh' --already-in-terminal; read -p \"Press Enter to close...\" _"
          exit $?
          ;;
      esac
    fi
  done
  echo "[!] Could not find a graphical terminal emulator."
  echo "Continuing in the current shell..."
  echo ""
  LAUNCHED_IN_TERMINAL=1
fi

if [[ ! -f "$INSTALLER" ]]; then
  echo "❌ install.sh not found at: $INSTALLER"
  usage_note
  exit 1
fi

chmod +x "$INSTALLER" >/dev/null 2>&1 || true

echo "=========================================="
echo "TR-100 Machine Report Linux Installer"
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
  exit "$status"
fi

