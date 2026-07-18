# TR-300 Master Plan

> Pickup-ready status and sequencing. Historical release detail belongs in
> `CHANGELOG.md` / `TESTING.md`; architectural rationale belongs in
> `docs/architecture-decisions.md`.

**Last updated:** 2026-07-18
**Published / working manifest:** 4.1.1 / 4.1.2
**Release scope:** origin-preserving updates, native PKG-in-DMG, Apple Installer
credentials, hosted ARM/Intel Mac gates, Windows installer matrix, and real
Alienware validation. AMD laptop and Pi evidence remain open.
**Default branch:** `main` (atomically renamed from `master` on 2026-07-17)
**Repository:** `QubeTX/qube-machine-report`

The v4.0.1 branch migration/release baseline remains complete: commit
`41c30b1e43f8abc5208f0d94702ed12cd91fb7a7` passed all 13 hosted CI jobs on
`main` (run 29557626125), and downstream crates run 29557758673 succeeded by
skipping the already-published 4.0.1 package. Fresh-clone/default/redirect,
workflow activity, immutable tags, 28 release assets, public Mac signatures,
crates.io, and the production homepage/install wrappers were re-verified.
The branch CI and crates workflows also use `actions/checkout@v6` on its
supported Node 24 runtime, matching release and supplemental Windows packaging.
Commit `1714d1fc0b90475d5f0aa590b1ec7d93b24d2eee` passed all 13 jobs in CI
run 29559148638 with zero annotations and no checkout deprecation warning;
exact-SHA crates run 29559305341 safely skipped already-published 4.0.1.
The canonical ADR is also reconciled through this state: its coverage index and
expanded records preserve the one-Rust architecture, collection/output
contracts, v4 safety/trust/evidence boundaries, Windows consolidation, and the
full `main`/checkout-v6 decision for the next machine.
Substantive ledger commit `e38fe2abcffdf6f85d4dac1c12dd294f36604a59`
passed all 13 jobs in CI run 29560970377 with zero annotations; exact-SHA crates
run 29561137746 performed the expected existing-4.0.1 pre-token safe skip.

## 1. Read order for the next machine

1. `docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md`
2. `AGENTS.md` and `CLAUDE.md`
3. this file
4. `TESTING.md`
5. `CHANGELOG.md` and `HUMAN_CHANGELOG.md`

The `.tasks/` board, milestones, task details, and dashboard assets are tracked;
only its runtime/secure state is gitignored. The board and tracked handoff are
both pickup-ready on a fresh clone.

## 2. v4.1.2 fix-forward release plan

The implementation is tracked in git and is intended to release from the
Alienware. A physical Mac is not a normal requirement: native GitHub
`macos-15` Apple Silicon and `macos-15-intel` runners are the required build,
sign, notarize, mount, install, update-strategy, report, and uninstall gates.
Use physical hardware only for an optional visual smoke or a CI-discovered
GUI-only defect.

### Implemented locally

- The updater resolves latest once but pins every payload, checksum, installer
  script, and Cargo command to that exact tag/version. Public commands and
  filenames remain versionless; the release workflow also normalizes generated
  release-note links and copy-paste commands to `latest` before publication.
- MSI Global/Corporate, EXE Global/Corporate, cargo-dist PowerShell/shell,
  Cargo, and macOS PKG/DMG each retain their own channel. Unknown/conflicting
  origins do not mutate the host and receive fresh-installer recovery links.
- Update JSON remains one stdout object and adds `install_channel`,
  `recovery_url`, and `requires_user_action`; installer progress is stderr.
- Windows uses scoped Global/Corporate markers plus legacy compatibility,
  conservative ARP recovery, and same-edition MSI/Inno replacement for a fresh
  explicit installer choice. v4.1.0-and-newer MSI authoring also accepts a
  fresh older/same-version MSI as the user's newest instruction; automatic
  updates remain latest-only.
- The universal macOS binary is packaged in a signed
  `com.qubetx.tr300.pkg` inside
  `tr300-universal-apple-darwin.dmg`. Both containers are notarized/stapled;
  future updates trust the PKG only when its identifier, version, payload,
  per-file owner, install path, and installed Developer ID product identity all
  match the running binary. This uses current `pkgutil` receipt/file-owner
  commands plus strict `codesign`, not the removed `pkgutil --verify` switch.
- CI includes native Apple Silicon, native Intel, Linux AMD64/ARM64, Windows,
  actionlint, and shellcheck. The DMG publishes only after native install gates.
- The Alienware's natural Global MSI v3.17.0 → v4.0.1 update preserved the
  Program Files path, Global marker, product registration, and PATH with no
  Corporate/Cargo duplicate. The Windows collector now reports the Core Ultra
  7 155H as `6P + 10E` with 16 physical / 22 logical cores.

### Credential ceremony — complete

- Apple issued the G2 **Developer ID Installer** certificate for Team
  `M9D5379H93`. Its RSA-2048 key, Installer EKU, official G2 chain, subject, and
  locally generated private-key match were verified on the Alienware.
- The certificate and key were converted into an encrypted `.p12` outside the
  repository and uploaded through authenticated GitHub CLI as
  `APPLE_INSTALLER_CERTIFICATE_P12_BASE64` and
  `APPLE_INSTALLER_CERTIFICATE_PASSWORD`; set
  `APPLE_INSTALLER_SIGNING_IDENTITY` to the certificate's full common name as
  a repository variable. No credential value entered git, docs, browser fields,
  or logs.
- Redundant key-generation material was removed after the GitHub upload. The
  retained off-repository backup is the encrypted PKCS#12, its Windows-user-
  encrypted password record, and public certificate copies.
- GitHub Actions run 29637224793 imported the Apple-keychain-compatible
  encrypted identity and signed/verified a disposable PKG on native Apple
  Silicon (job 88061567206) and Intel (job 88061567218). Credential issuance
  and hosted identity proof are complete; no physical Mac was required.

### Remaining release gates

v4.1.0 source SHA `5b4e18d5928e602452a0030a9f5b130dc611d3c9`
passed exact-SHA CI run 29638735899, crates run 29638873747, and signed archive
Release run 29638940801. The supplemental DMG run 29639135342 failed without
publishing either DMG asset because checkout cleaned already-downloaded inputs.
The independent ND-300 hosted path also exposed Xcode 16.4's required
`lipo <file> -verify_arch ...` order before TR-300 could reach that line.
Published v4.1.0 bytes and tag remain immutable; v4.1.1 fixed both lifecycle
defects. Windows packaging run
29639135337 succeeded, while downstream validation 29639224625 exposed a third
orchestration defect: a second-hop `workflow_run` reports `main` rather than the
original tag. v4.1.1 resolves one release from the upstream exact SHA and fails
closed on ambiguity.

v4.1.1 source SHA `09afdc6ae5cbff1a497e6cec07c4cf1b36d2557b`
passed exact-SHA CI 29639632790, crates 29639731682, signed archive Release
29639767064, and Windows packaging 29639898355. Supplemental Mac run
29639898362 successfully built, signed, notarized, stapled, mounted, and
installed the universal PKG-in-DMG on both native architectures, then failed
closed because current macOS rejects the obsolete `pkgutil --verify` option;
the DMG assets were not published. Windows validation 29639998787 proved the
exact-SHA second hop and five clean channel flows, while exposing a PowerShell
host mismatch, an incorrect already-current portable assertion, and an Inno
MSI-enumeration buffer declaration in the fresh-format transitions. v4.1.2
replaces those unsupported/incorrect checks, adds real prior-version
same-channel updates and portable recovery to the disposable matrix, and is the
complete 30-asset distribution target. v4.1.0 and v4.1.1 remain untouched.

1. Finish the tracked ADR/docs/handoff and isolated Windows installer matrix.
2. Run fmt, locked clippy/tests, release build, package/publish dry runs, audit,
   dist plan, actionlint, shellcheck, updater fixtures, and Alienware functional
   modes/save/code-page/performance checks.
3. Commit/push `main`; wait for exact-SHA CI and crates publication.
4. Tag/push only `v4.1.2`; wait for cargo-dist, Windows installers, and native
   PKG-in-DMG workflows.
5. Verify crates.io, signatures/notarization, checksums, every installer family,
   all 30 release assets, update behavior, recovery links, and clean uninstall.
6. Record exact run IDs/hashes/evidence here, in `TESTING.md`, and in the
   canonical handoff before marking release complete.

## 3. v4.0.0 Mac and shared outcome (historical baseline)

macOS collection work is complete for v4.0.0. Shared report/update changes and
the new release-trust path passed comprehensive native arm64, Rosetta x86_64,
and real Apple signing/notarization validation on this Mac before the release
commit.

The version moved from the originally planned 3.18.0 to 4.0.0 during the final
API audit. Public Rust records gained fields and selected collector helpers
changed signature, which is source-breaking for direct struct literals and
exhaustive matches. CLI behavior and existing schema-v1 JSON keys remain
compatible. The affected records are now `#[non_exhaustive]` so future additive
information does not repeat this major-version event.

- Full mode collects one structured native `system_profiler -json` snapshot for
  model, GPU/display, battery, boot state, and positive virtualization evidence.
- Native Apple Silicon and translated Intel/Rosetta processes report the same
  host hardware facts. Rosetta is explicit in `os.architecture`; its compatibility
  CPU frequency is intentionally `null`.
- macOS memory is Activity Monitor-compatible (`active+wired+compressed`), and
  available bytes are the exact complement so used + available = total.
- APFS disk used and available values preserve distinct OS meanings and identify
  the root mount/filesystem in JSON.
- FileVault, Normal/Safe/Recovery boot state, current logical/native display,
  battery health/cycles, chip/P-E topology, terminal host, OS build/codename,
  locale, and login parsing are implemented with graceful absence.
- Mac JSON intentionally excludes serial numbers, UUIDs, UDIDs, and similar
  unique identifiers.
- Fresh-account `.zshrc` behavior and profile install/uninstall idempotency are
  covered with isolated filesystem tests; no real profile was mutated.
- CI's macOS test, build, and speed legs are blocking again. The RustSec audit is
  also blocking.

### Mac evidence captured locally

Host: MacBook Pro M2 (`Mac14,7`), macOS 26.3.1 build 25D2128.

- `cargo fmt --all -- --check`: pass
- locked all-target clippy with `-D warnings`: pass
- native Apple Silicon: 121 library + 19 integration tests pass
- Rosetta x86_64: 121 library + 19 integration tests pass
- release builds: arm64 and x86_64 Mach-O binaries pass
- native full JSON/table parity against `sw_vers`, `sysctl`, `pmset`, and
  `fdesetup`: pass
- Rosetta full JSON/table parity: pass
- fast JSON: pass; slow fields absent by contract
- non-UTF `C` locale: ASCII-only output, every line exactly 51 columns
- updater no-op against the previously published v3.17.0: pass natively and
  under Rosetta; final v4 behavior is covered by unit/JSON smokes
- RustSec audit: pass (221 locked dependencies; `crossbeam-epoch` 0.9.20)
- final five-run medians: native full 0.51s, native fast 0.23s, Rosetta full
  0.72s, Rosetta fast 0.36s; fast budget is 1.5s

## 4. Shared hardening already implemented

These changes are cross-platform Rust and compile-tested, but platform-specific
runtime claims remain gated by the hardware matrix below.

- Bounded subprocess helper drains stdout/stderr concurrently, caps output at
  8 MiB, kills Unix process groups on timeout, and assigns Windows probes to a
  best-effort Job Object.
- JSON is produced from a typed `serde_json::Value`, preserving schema version 1.
- Ordinary full/fast/JSON reports do not create report files. Manual
  `-r`/`--report`/`-s`/`--save` uses unique, create-new,
  symlink-resistant filenames and never overwrites an existing path.
- Root/system disk aggregation avoids summing overlapping or unrelated volumes.
- CPU load exposes raw Unix runnable-queue averages plus normalized percent of
  logical capacity; Windows leaves time-window loads absent instead of cloning an
  instantaneous sample.
- Unknown physical topology and virtualization remain unknown rather than being
  labeled as logical cores or Bare Metal.
- Update payloads use randomized private staging, response caps, explicit
  cleanup, strict version parsing, and post-install verification. Likely
  endpoint-policy create/write/sync/launch blocks stop the fallback chain,
  preserve the current install, and return actionable failure without a
  force/direct-overwrite path.
- Both Mac cargo-dist archives are fail-closed behind Developer ID signing,
  hardened runtime/timestamp, and Apple Notary Service acceptance. The exact
  signed bytes are repacked and the sidecar/manifest checksums regenerated
  before upload.
- Windows-specific code passes xwin clippy with `-D warnings`; v4.1.0 also has
  live Alienware validation described below.

## 5. Personal hardware continuation

### A. Personal Alienware / Windows

**Status: active, baseline/update/topology evidence captured.** The natural
Global MSI v3.17.0 → v4.0.1 update preserved path, marker, registration, and
PATH without a duplicate. The source-built v4.1.0 report confirms 16 physical,
22 logical, `6P + 10E`, and both GPUs. Complete the remaining full/fast/table/
JSON/manual-save, code-page, failure/recovery, cleanup, and performance rows
before tagging.

- OS edition/build/architecture and model/motherboard/BIOS
- CPU model, physical/logical topology, socket count, frequency provenance, GPU
- system drive mount/filesystem/used/available semantics
- RAM/swap, battery/AC state, BitLocker, boot mode, uptime/session distinction
- default-route IP, DNS, SSH client IP, username, terminal, shell, locale/login
- installer-origin detection and the installed edition's `tr300 update --json`
- Console code-page restoration and full/fast runtime budget

Record command evidence and every discrepancy in `TESTING.md`; never include
serial numbers. The managed work machine's antivirus freeze remains a separate
endpoint-policy case.

### B. Linux AMD64 laptop

- Full/fast/table/JSON/Markdown parity
- default-route IP and resolver behavior
- CPU topology/frequency/load, GPU filtering, disk/memory definitions
- battery health/AC state, root encryption, boot mode, terminal/locale/login
- unelevated vs elevated `dmidecode`, ZFS/non-ZFS graceful paths

### C. Raspberry Pi 4 (64-bit aarch64 baseline)

- devicetree CPU/model names and nonzero rated frequency
- physical/logical cores, load, memory, root filesystem, network/DNS
- battery absence, missing desktop/tool fallbacks, fast runtime budget
- installer/update path on the actual distribution

## 6. v4.0.1 fix-forward release outcome — complete

The maintainer explicitly approved sections 4A–4C after release, accepting
forward patches. Every Mac/local/hosted gate still passed before publication.

The immutable `v4.0.0` tag points to
`c21d5981d4109199fa4bcba15ef8af6285a33d56`. CI run 29389974094 passed and
crates run 29390118811 published the source package, but release run
29390216481 failed closed before hosting: the clean Apple runners imported the
identity but `codesign` could not resolve it because the new keychain was not
on the user search list. Do not move or delete that tag. v4.0.1 is the required
patch fix-forward.

- Release source commit:
  `b67ad083503d0fff840af8467015d05c659268ea`.
- Clean-tree local package gate: 39 packaged files; locked publish dry-run
  compiled successfully.
- Exact-SHA CI: run 29391956665, success across all 13 blocking jobs.
- Exact-SHA crates workflow: run 29392101640, success; unyanked crate checksum
  `55086eb631a3b67c8ab0eaa53b9c3783097044ef77321ec8e6849c30e32275da`.
- Explicit tag `v4.0.1` resolves to the release SHA; `v4.0.0` remains unchanged.
- Cargo-dist run 29392185522 and Windows Installers run 29392382949 succeeded.
- Hosted Apple `Accepted` submissions: arm64
  `97b0c295-89d8-4758-a4c3-1dc345c28f0e`; x86_64
  `09cf1403-e546-4f5e-8de1-9bf92fd602e9`.
- The public GitHub Release is non-draft/non-prerelease with exactly 28 assets.
  Both downloaded Mac archives match sidecar/aggregate/manifest/GitHub digests,
  report 4.0.1, and pass strict Developer ID/team/identifier/runtime/timestamp/
  embedded-certificate checks. Canonical/legacy installer aliases match, and
  every supplemental Windows installer matches its sidecar.

## 7. Homepage deployment — complete

- Homepage commit `d77397479ad2b1189cce86b5402eaf1cc966abdf` is pushed to its
  default branch and production at `https://reports.qubetx.com/` serves its
  exact `index-DghJyecZ.js` bundle.
- Package 1.13.0 describes v4.0.1, all four manual save spellings, read-only
  default/startup output, native accuracy semantics, fail-safe managed-Windows
  updates, and enforced notarized Apple Silicon/Intel downloads.
- ESLint, Vite build, wrapper syntax/equality, canonical release/package/docs/
  installer links, and Chrome desktop/mobile checks pass. There is no horizontal
  overflow or site-origin console error; every sample row is exactly 51 columns.
- SD-300 and Shaughv OS remain intentionally WIP-delisted and must not be
  re-linked until their separate work is ready.

## 8. Non-negotiable contracts

- Keep one shared Rust architecture and preserve both binary and library APIs.
- Optional probe failure yields `None`/fallback, never a fabricated claim or a
  failed whole report.
- `--fast` remains prompt-safe and below the 1.5s CI budget.
- JSON schema version 1 permits additive keys only; breaking changes require a
  version bump and migration notes.
- Table output remains 51 display columns unless explicitly redesigned.
- Do not surface unique device identifiers.
- Keep `Cargo.toml` `rust-version` and `rust-toolchain.toml` channel in lockstep.
- Never tag before exact-SHA CI and crates publication settle.
- Never publish locally just because registry credentials are available.
- Windows installer GUIDs and `InstallSource` marker strings are permanent and
  must remain in lockstep across WiX, Inno, updater, and migration code.
- Shared/macOS/release changes require native Apple Silicon and native Intel
  hosted validation. PKG-in-DMG changes also require sign/notary/staple/mount/
  install/update/uninstall proof on both architectures. A physical Mac is
  optional unless those runners expose a GUI-only defect.

## 9. Completion state

The v4.0.1 **release is complete**. Observed evidence satisfies every condition:

- Mac evidence above remains green in hosted blocking CI.
- All local release gates pass from a clean tree.
- Crates.io and GitHub Release point to the exact tested commit/tag.
- Both Mac archives are Developer ID signed/Apple accepted and all expected
  release assets/self-update discovery are verified.
- The homepage accurately reflects the deployed release and is live.

The v4.1.0 release remains active. The broader personal-hardware milestone stays
open for AMD Linux and Pi 4 evidence after this release.
