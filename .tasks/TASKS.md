# Tasks

## Backlog

## To-Do

- [ ] **Validate TR-300 on the AMD64 Linux laptop** - full/fast reports, shell/Cargo update preservation, permissions, networking, and graceful optional probes (ms #hw4) #amd
- [ ] **Validate TR-300 on Raspberry Pi 4 ARM64** - native ARM asset, full/fast reports, shell/Cargo updates, permissions, networking, and graceful optional probes (ms #hw4) #pi4
- [ ] **Complete the Alienware Global MSI v4.1.3 → v4.2.2 UAC update** - approve the same-channel transaction, then reprove one Program Files owner, registry/PATH, cleanup, JSON, functionality, and hardware (needs #v42) (ms #hw4) #w422

## Active

- [ ] **Integrate, qualify, and release TR-300 v4.3.0** - finish the no-click credential/automation cutover, then run exact-SHA crates/tag/native/public/deployment gates (needs #v430) (ms #v43) (owner codex) #r43
  - [x] Remediate and independently validate every confirmed release/privilege security finding
  - [x] Retire Mac postinstall takeover and reject standard-path managed evidence in `/Users` plus eligible local Directory Service homes and non-root target volumes before payload
  - [x] Prove receipt-aware Unix Complete uninstall safely removes exact managed ownership and fails closed on ambiguity
  - [x] Pass PR #15 exact-head local/hosted/security/native qualification, merge it, and pass exact-main CI
  - [ ] Finish Apple environment-only reproof and merge removal of the temporary migration workflow
  - [ ] Merge automatic post-CI OIDC publication, prove v4.3.0 public, then revoke/delete the unused legacy crates token and secret
  - [x] Merge hardened main into the local PR #14 branch and pass the complete local gate plus controlled benchmark
  - [x] Push the integrated candidate and pass final exact-head hosted, security, and review gates
  - [x] Merge only the accepted candidate and verify exact-main CI
  - [ ] Push only v4.3.0 and pass the private 24-to-30-to-validated-to-34 publication chain
  - [ ] Audit public bytes and post-public updater/Linux/macOS smokes before homepage deployment

## Done

- [x] **Repair and revalidate v4.3.0 PR #14** - correct platform accuracy, preserve the measurable Windows full-mode gain, prove exact-head CI, and leave the PR open (ms #v43) (owner codex) (done 2026-08-23) #v430
  - [x] Repair Windows launch-relative deadlines and trustworthy NVIDIA-only thermals
  - [x] Repair Linux battery compatibility and deterministic fault-aware thermals
  - [x] Tighten macOS battery parsing and cross-format thermal rendering tests
  - [x] Add `-f/--full` explicit flag
  - [x] Reconcile unreleased docs, task state, and PR claims with measured evidence
  - [x] Pass the full local gate and controlled v4.2.2 comparison
  - [x] Push the corrected exact head and pass hosted CI/review without merging
- [x] **Record the completed ND-300 v3.7.3 native-installer acceptance** - external operator-approved Alienware update completed without overlap; physical Mac GUI acceptance is optional/deferred behind native Intel/ARM direct-PKG and legacy-DMG gates (ms #hw4) (done 2026-07-18) #nd372
- [x] **Codify and ship MIC-1 managed installation behavior** - make CLI installers the documented default, preserve update origin, make fresh managed intent authoritative, and fail closed across unsupported native scope transitions (ms #v42) (owner codex) (done 2026-07-18) #mic1
  - [x] Define the reusable managed-install/update state machine and raw-Cargo boundary
  - [x] Add stable public wrappers over exact-tag cargo-dist installer transactions
  - [x] Add Windows native-to-PowerShell and Mac shell/PKG convergence paths
  - [x] Make cross-edition native Windows packages stop before unsafe mutation
  - [x] Pass local Rust/script/package-plan/installer-source and Alienware candidate gates
  - [x] Pass hosted Windows/macOS/Linux transition jobs
  - [x] Reconcile README, changelogs, ADR, testing, plans, handoff, and agent guides
- [x] **Publish direct universal PKG with the v4.1.x DMG bridge** - make the signed PKG the current Mac package/update artifact without stranding immutable DMG clients (ms #v42) (owner codex) (done 2026-07-18) #pkg42
  - [x] Change current Mac updater selection from exact DMG to exact direct PKG
  - [x] Build one signed/notarized/stapled PKG plus byte-identical compatibility DMG
  - [x] Add native Intel/ARM direct install, bidirectional CLI takeover, and legacy bridge jobs
  - [x] Pass Apple credential, package, updater, bridge, and publication gates
  - [x] Audit the final 34-asset immutable release

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
