# Tasks

## Backlog

## To-Do

- [ ] **Audit the public v4.1.2 distribution and close the release ledger** - verify all public bytes, installers, signatures, notarization, update channels, and recovery behavior (needs #rsh) (ms #v41) (owner codex) #rau
  - [ ] Verify crates.io and all 30 GitHub Release assets
  - [ ] Verify every Windows installer family and both Apple architectures
  - [ ] Record exact run IDs, hashes, and final evidence in tracked docs
- [ ] **Validate TR-300 on the AMD64 Linux laptop** - full/fast reports, shell/Cargo update preservation, permissions, networking, and graceful optional probes (ms #hw4) #amd
- [ ] **Validate TR-300 on Raspberry Pi 4 ARM64** - native ARM asset, full/fast reports, shell/Cargo updates, permissions, networking, and graceful optional probes (ms #hw4) #pi4

## Active

- [ ] **Ship v4.1.2 through exact-SHA hosted gates** - preserve v4.1.0/v4.1.1, qualify supported Mac ownership proof and Windows transitions, then tag and publish without bypassing a gate (needs #c8r) (ms #v41) (owner codex) #rsh
  - [x] Publish v4.1.0 CI, crates, and signed archives and record the failed supplemental DMG gate
  - [x] Publish v4.1.1 CI/crates/archives/Windows assets and retain its failed DMG/partial Windows evidence
  - [x] Commit and push the v4.1.2 fix-forward to main
  - [ ] Wait for exact-SHA CI and crates.io publication
  - [ ] Push only tag v4.1.2 and watch every release workflow

## Done

- [x] **Issue and validate the Developer ID Installer credential** - complete Apple G2 issuance, encrypted PKCS#12/GitHub upload, and hosted identity proof (ms #v41) (owner codex) (done 2026-07-18) #c8r
  - [x] Generate and locally verify an encrypted RSA-2048 CSR
  - [x] Issue the Developer ID Installer certificate from Apple G2
  - [x] Convert it to an encrypted PKCS#12 and upload GitHub secrets/variable
  - [x] Prove the imported Installer identity on a native GitHub macOS runner

- [x] **Validate TR-300 on the Alienware Windows machine** - updater, functionality, hardware fields, hybrid topology, modes, save behavior, code page, and performance (ms #hw4) (done 2026-07-17) #win
