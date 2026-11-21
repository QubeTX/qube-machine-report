# Adaptive terminal startup

## Status

Product PR #46 is merged. The 4.5.0 crate is public. Full GitHub publication awaits the Apple Account Holder’s agreement. Keep the release source on main until the tag and native assets are published. [Release evidence and resume point](../../docs/agents/handoff/2026-11-21-v4.5.0-terminal-startup.md)

## Activity

Verified the merged source, Linux and macOS PTYs, Windows consoles, real OpenSSH sessions, native package startup, account opt-out, policy-aware PowerShell enrollment, and real Windows MSI repair and rollback. Homepage PR #24 has a successful deployment preview and is held for the public installers.

## Verification

- [x] Verify viewport, wrapping, prompt space and interactive SSH on supported platforms.
- [x] Verify installation, persistent opt-out, upgrade, repair, rollback and cleanup in the candidate.
- [x] Merge the release source and verify the checksum and source of the public crate.
- [ ] Obtain Apple notary access, publish the signed native distribution, audit the public bytes and deploy the matching website.
