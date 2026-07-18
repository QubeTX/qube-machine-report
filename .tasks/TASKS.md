# Tasks

## Backlog

## To-Do

- [ ] **Validate TR-300 on the AMD64 Linux laptop** - full/fast reports, shell/Cargo update preservation, permissions, networking, and graceful optional probes (ms #hw4) #amd
- [ ] **Validate TR-300 on Raspberry Pi 4 ARM64** - native ARM asset, full/fast reports, shell/Cargo updates, permissions, networking, and graceful optional probes (ms #hw4) #pi4
- [ ] **Hand off the bounded ND-300 v3.7.3 native-installer acceptance lanes** - only after TR-300 is complete and no installer/UAC process remains; run the Alienware lane first, then reuse the proven legacy-DMG-to-direct-PKG Mac procedure only when the user is on the testing Mac (needs #v42) (ms #hw4) #nd372

## Active

- [ ] **Codify and ship MIC-1 managed installation behavior** - make CLI installers the documented default, preserve update origin, make fresh managed intent authoritative, and fail closed across unsupported native scope transitions (ms #v42) (owner codex) #mic1
  - [x] Define the reusable managed-install/update state machine and raw-Cargo boundary
  - [x] Add stable public wrappers over exact-tag cargo-dist installer transactions
  - [x] Add Windows native-to-PowerShell and Mac shell/PKG convergence paths
  - [x] Make cross-edition native Windows packages stop before unsafe mutation
  - [x] Pass local Rust/script/package-plan/installer-source and Alienware candidate gates
  - [ ] Pass hosted Windows/macOS/Linux transition jobs
  - [x] Reconcile README, changelogs, ADR, testing, plans, handoff, and agent guides
- [ ] **Publish direct universal PKG with the v4.1.x DMG bridge** - make the signed PKG the current Mac package/update artifact without stranding immutable DMG clients (ms #v42) (owner codex) #pkg42
  - [x] Change current Mac updater selection from exact DMG to exact direct PKG
  - [x] Build one signed/notarized/stapled PKG plus byte-identical compatibility DMG
  - [x] Add native Intel/ARM direct install, bidirectional CLI takeover, and legacy bridge jobs
  - [ ] Pass Apple credential, package, updater, bridge, and publication gates
  - [ ] Audit the final 34-asset immutable release

## Done

- [x] **Validate the public v4.1.3 Global MSI on the Alienware** - update the natural 4.0.1 installation, then re-run functionality, origin, cleanup, PATH, hardware, code-page, save, and performance evidence (needs #rsh) (ms #v41) (owner codex) (done 2026-07-18) #w413
  - [x] Complete the one-UAC Global MSI update and capture installed-success evidence
  - [x] Prove one Program Files binary/registration/marker/PATH and no backup/duplicate
  - [x] Re-run public report modes, save/no-write, code-page, performance, and hardware checks
- [x] **Audit the public v4.1.3 distribution and close the release ledger** - verify all public bytes, installers, signatures, notarization, update channels, and recovery behavior (needs #rsh) (ms #v41) (owner codex) (done 2026-07-18) #rau
  - [x] Verify crates.io and all 30 GitHub Release assets
  - [x] Verify every Windows installer family and both Apple architectures
  - [x] Record exact run IDs, hashes, and final evidence in tracked docs
- [x] **Ship v4.1.3 through exact-SHA hosted gates** - preserve v4.1.0-v4.1.2, qualify supported Mac ownership proof and every Windows transition including Global live-image repair, then tag and publish without bypassing a gate (needs #c8r) (ms #v41) (owner codex) (done 2026-07-18) #rsh
  - [x] Publish v4.1.0 CI, crates, and signed archives and record the failed supplemental DMG gate
  - [x] Publish v4.1.1 CI/crates/archives/Windows assets and retain its failed DMG/partial Windows evidence
  - [x] Commit, push, tag, and publish the immutable v4.1.2 hosted distribution
  - [x] Commit and push the v4.1.3 Global updater fix-forward to main
  - [x] Wait for exact-SHA v4.1.3 CI and crates.io publication
  - [x] Push only tag v4.1.3 and watch every release workflow

- [x] **Issue and validate the Developer ID Installer credential** - complete Apple G2 issuance, encrypted PKCS#12/GitHub upload, and hosted identity proof (ms #v41) (owner codex) (done 2026-07-18) #c8r
  - [x] Generate and locally verify an encrypted RSA-2048 CSR
  - [x] Issue the Developer ID Installer certificate from Apple G2
  - [x] Convert it to an encrypted PKCS#12 and upload GitHub secrets/variable
  - [x] Prove the imported Installer identity on a native GitHub macOS runner

- [x] **Validate TR-300 on the Alienware Windows machine** - updater, functionality, hardware fields, hybrid topology, modes, save behavior, code page, and performance (ms #hw4) (done 2026-07-17) #win
