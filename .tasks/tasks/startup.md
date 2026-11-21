# Adaptive terminal startup

## Status

TR-300 4.5.0 is published on crates.io and GitHub. All 34 public assets, the installer lifecycles and the production website were verified. [Release evidence and resume point](../../docs/agents/handoff/2026-11-21-v4.5.0-terminal-startup.md)

## Activity

Verified the merged source, Linux and macOS PTYs, Windows consoles, real OpenSSH sessions, native package startup, account opt-out, policy-aware PowerShell enrollment, and real Windows MSI repair and rollback. Homepage PR #24 shipped after the public installer checks passed.

## Verification

- [x] Verify viewport, wrapping, prompt space and interactive SSH on supported platforms.
- [x] Verify installation, persistent opt-out, upgrade, repair, rollback and cleanup in the candidate.
- [x] Merge the release source and verify the checksum and source of the public crate.
- [x] Obtain Apple notary access, publish the signed native distribution, audit the public bytes and deploy the matching website.
