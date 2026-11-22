#!/usr/bin/env bash
#
# Cloud Agent bootstrap for the TR-300 workspace. This script is invoked by the
# `install` phase in .cursor/environment.json and is written to be idempotent so
# it can safely run repeatedly against a cached or partially prepared checkout.
#
# It prepares two independent projects that may share this workspace: the tr300
# Rust CLI (this repository) and, when present, the sibling React + Vite
# marketing homepage (qube-machine-report-homepage).

set -euo pipefail

# Resolve this repository's root from the script location instead of assuming an
# absolute path, so the bootstrap works no matter where a fresh Cloud Agent
# checks the repository out.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLI_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Build the tr300 CLI in release mode. Running cargo here makes rustup honor the
# pinned rust-toolchain.toml (auto-installing the MSRV 1.95 toolchain if needed)
# and warms the full locked dependency cache into the base snapshot.
echo "[install] Building tr300 Rust CLI (release, locked)..."
cd "$CLI_ROOT"
cargo build --release --locked

# Install homepage npm dependencies only when that sibling repository is checked
# out in this workspace. Guarding on the file keeps a single-repo Cloud Agent
# (which has no homepage) finishing the install phase cleanly instead of failing.
HOMEPAGE_ROOT="$(cd "$CLI_ROOT/.." && pwd)/qube-machine-report-homepage"
if [ -f "$HOMEPAGE_ROOT/package.json" ]; then
  echo "[install] Installing homepage npm dependencies (npm ci)..."
  cd "$HOMEPAGE_ROOT"
  npm ci
else
  echo "[install] Sibling homepage repo not present; skipping npm install."
fi

echo "[install] Bootstrap complete."
