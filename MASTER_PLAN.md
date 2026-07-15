# TR-300 Master Plan

> Pickup-ready status and sequencing. Historical release detail belongs in
> `CHANGELOG.md` / `TESTING.md`; architectural rationale belongs in
> `docs/architecture-decisions.md`.

**Last updated:** 2026-07-15 01:20 CDT
**Release / working manifest:** 4.0.1
**Release scope:** v4.0.1 and its homepage are deployed; personal
Alienware/AMD/Pi evidence remains open and drives forward patches
**Default branch:** `master`
**Repository:** `QubeTX/qube-machine-report`

## 1. Read order for the next machine

1. `docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md`
2. `AGENTS.md` and `CLAUDE.md`
3. this file
4. `TESTING.md`
5. `CHANGELOG.md` and `HUMAN_CHANGELOG.md`

The local `.tasks/` board is gitignored. The tracked handoff therefore contains
all continuation-critical state and is authoritative on a fresh Alienware clone.

## 2. v4.0.0 Mac and shared outcome

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

## 3. Shared hardening already implemented

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
- Windows-specific code passes xwin clippy with `-D warnings`; this is compile
  evidence only, not live Alienware validation.

## 4. Post-release personal hardware tasks

### A. Personal Alienware / Windows

After v4.0.1, run the full, fast, JSON, and manually saved Markdown report from
the intended Windows install
and compare every visible fact with native Windows tools:

- OS edition/build/architecture and model/motherboard/BIOS
- CPU model, physical/logical topology, socket count, frequency provenance, GPU
- system drive mount/filesystem/used/available semantics
- RAM/swap, battery/AC state, BitLocker, boot mode, uptime/session distinction
- default-route IP, DNS, SSH client IP, username, terminal, shell, locale/login
- installer-origin detection and the installed edition's `tr300 update --json`
- Console code-page restoration and full/fast runtime budget

Record Windows version/build, install origin, command evidence, screenshots when
useful, and every discrepancy in `TESTING.md`. Patch forward as needed. The
managed work machine's antivirus freeze is endpoint-policy evidence, not a
substitute for this personal Alienware accuracy matrix.

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

## 5. v4.0.1 fix-forward release outcome — complete

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

## 6. Homepage deployment — complete

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

## 7. Non-negotiable contracts

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
- The Alienware continuation must preserve the enforced macOS release path: no
  macOS collector/cfg branch, Apple target/artifact name,
  `scripts/sign-notarize-macos.sh`, Apple `release.yml` step, toolchain,
  signing/notarization input, or secret/variable change from Windows. A
  dependency-resolution, shared runtime, build, dist, workflow, or Apple change
  forces native + Rosetta Mac revalidation; Apple-input changes also require a
  real archive notary test before any tag.

## 8. Completion state

The v4.0.1 **release is complete**. Observed evidence satisfies every condition:

- Mac evidence above remains green in hosted blocking CI.
- All local release gates pass from a clean tree.
- Crates.io and GitHub Release point to the exact tested commit/tag.
- Both Mac archives are Developer ID signed/Apple accepted and all expected
  release assets/self-update discovery are verified.
- The homepage accurately reflects the deployed release and is live.

The broader cross-platform hardening milestone stays open until the personal
Alienware, AMD Linux, and Pi 4 matrices are recorded and any confirmed defects
are patched. This is intentional and does not make the v4 release incomplete.
