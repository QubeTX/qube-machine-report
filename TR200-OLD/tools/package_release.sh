#!/usr/bin/env bash
# Build the cross-platform installer zip for TR-200 Machine Report
# Copyright 2026, ES Development LLC (https://emmetts.dev)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="$REPO_ROOT/dist"
RELEASE_NAME="tr-200-machine-report"
RELEASE_ROOT="$DIST_DIR/$RELEASE_NAME"

function log() {
  echo "[package] $*"
}

function clean_dist() {
  rm -rf "$DIST_DIR"
  mkdir -p "$RELEASE_ROOT"
}

function copy_core_files() {
  log "Copying core scripts"
  cp "$REPO_ROOT/machine_report.sh" "$RELEASE_ROOT/"
  cp "$REPO_ROOT/install.sh" "$RELEASE_ROOT/"
  cp "$REPO_ROOT/README.md" "$RELEASE_ROOT/README.md"
  cp "$REPO_ROOT/CLAUDE.md" "$RELEASE_ROOT/CLAUDE.md"
}

function copy_windows_assets() {
  log "Copying Windows assets"
  mkdir -p "$RELEASE_ROOT/WINDOWS"
  cp "$REPO_ROOT/WINDOWS/TR-200-MachineReport.ps1" "$RELEASE_ROOT/WINDOWS/"
  cp "$REPO_ROOT/WINDOWS/install_windows.ps1" "$RELEASE_ROOT/WINDOWS/"
  cp "$REPO_ROOT/WINDOWS/README_WINDOWS.md" "$RELEASE_ROOT/WINDOWS/"
  cp "$REPO_ROOT/WINDOWS/build_tr100_exe.ps1" "$RELEASE_ROOT/WINDOWS/"
}

function copy_launchers() {
  log "Copying launcher scripts"
  cp "$REPO_ROOT/install_mac.command" "$RELEASE_ROOT/"
  cp "$REPO_ROOT/install_linux.sh" "$RELEASE_ROOT/"
  chmod +x "$RELEASE_ROOT/install_mac.command" "$RELEASE_ROOT/install_linux.sh" || true
}

function build_windows_exe() {
  local output_path="$RELEASE_ROOT/install_windows.exe"
  if command -v pwsh >/dev/null 2>&1; then
    log "Building Windows installer executable via ps2exe"
    (cd "$REPO_ROOT/WINDOWS" && pwsh -NoLogo -NoProfile -File build_tr100_exe.ps1 -OutputFile "$output_path") || {
      log "⚠️  Failed to build Windows executable. See output above."
      return 1
    }
  else
    log "⚠️  pwsh not found; skipping automatic Windows .exe build."
    log "    You can build it manually by running WINDOWS/build_tr100_exe.ps1 on Windows."
  fi
}

function create_zip() {
  log "Creating zip archive"
  (cd "$DIST_DIR" && zip -rq "${RELEASE_NAME}.zip" "$RELEASE_NAME")
  log "Created $DIST_DIR/${RELEASE_NAME}.zip"
}

clean_dist
copy_core_files
copy_windows_assets
copy_launchers
build_windows_exe || true
create_zip

