# Adaptive terminal startup

## Status

Active. Qualifying TR-300 4.5.0 in product PR #46. Homepage PR #24 follows public release.

## Activity

Implemented adaptive local and SSH reporting and scoped automatic installer configuration. Verified classic Windows viewport and prompt visibility, Linux OpenSSH, and first SSH login to the native package on both Mac architectures. Hosted Windows MSI/EXE transition assertions passed. Windows OpenSSH and the Unix PowerShell PTY checks passed. Windows Global PowerShell enrollment is being qualified on default-policy clients and upgraded accounts.

## Verification

- [x] Verify viewport, wrapping, prompt space and interactive SSH on supported platforms.
- [ ] Verify installation, persistent opt-out, upgrade, rollback and cleanup.
- [ ] Qualify and merge the release PR, verify public binaries, and deploy the matching website.
