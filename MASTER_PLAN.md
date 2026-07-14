# TR-300 Master Plan

> Pickup-ready status and sequencing. Historical release detail belongs in
> `CHANGELOG.md` / `TESTING.md`; architectural rationale belongs in
> `docs/architecture-decisions.md`.

**Last updated:** 2026-07-14 01:50 CDT
**Published version:** 3.17.0
**Working manifest:** 3.17.0 (intentional checkpoint; no release bump yet)
**Planned release:** 4.0.0 after deferred hardware validation
**Default branch:** `master`
**Repository:** `QubeTX/qube-machine-report`

## 1. Read order for the next machine

1. `docs/agents/handoff/2026-07-14-001-macos-hardening-alienware-continuation.md`
2. `AGENTS.md` and `CLAUDE.md`
3. this file
4. `TESTING.md`
5. `CHANGELOG.md` and `HUMAN_CHANGELOG.md`

The local `.tasks/` board is gitignored. The tracked handoff therefore contains
all continuation-critical state and is authoritative on a fresh Alienware clone.

## 2. Outcome of the Mac checkpoint

macOS work is complete for the planned v4.0.0 hardening scope.

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
- native Apple Silicon: 115 library + 19 integration tests pass
- Rosetta x86_64: 115 library + 19 integration tests pass
- release builds: arm64 and x86_64 Mach-O binaries pass
- native full JSON/table parity against `sw_vers`, `sysctl`, `pmset`, and
  `fdesetup`: pass
- Rosetta full JSON/table parity: pass
- fast JSON: pass; slow fields absent by contract
- non-UTF `C` locale: ASCII-only output, every line exactly 51 columns
- updater no-op against published v3.17.0: pass natively and under Rosetta
- RustSec audit: pass (221 locked dependencies; `crossbeam-epoch` 0.9.20)
- observed release timing: native full 0.90s, native fast 0.21s, Rosetta full
  1.52s, Rosetta fast 0.31s; fast budget is 1.5s

## 3. Shared hardening already implemented

These changes are cross-platform Rust and compile-tested, but platform-specific
runtime claims remain gated by the hardware matrix below.

- Bounded subprocess helper drains stdout/stderr concurrently, caps output at
  8 MiB, kills Unix process groups on timeout, and assigns Windows probes to a
  best-effort Job Object.
- JSON is produced from a typed `serde_json::Value`, preserving schema version 1.
- `--no-save` suppresses automatic Markdown output; normal saves use unique,
  create-new, symlink-resistant filenames and never overwrite an existing path.
- Root/system disk aggregation avoids summing overlapping or unrelated volumes.
- CPU load exposes raw Unix runnable-queue averages plus normalized percent of
  logical capacity; Windows leaves time-window loads absent instead of cloning an
  instantaneous sample.
- Unknown physical topology and virtualization remain unknown rather than being
  labeled as logical cores or Bare Metal.
- Update payloads use randomized private staging, response caps, guaranteed
  cleanup, strict version parsing, and post-install verification.
- Windows-specific code passes xwin clippy with `-D warnings`; this is compile
  evidence only, not live Alienware validation.

## 4. Deferred hardware tasks

### A. Alienware / Windows — first action after pull

Run the full, fast, JSON, and Markdown report from the intended Windows install
and compare every visible fact with native Windows tools:

- OS edition/build/architecture and model/motherboard/BIOS
- CPU model, physical/logical topology, socket count, frequency provenance, GPU
- system drive mount/filesystem/used/available semantics
- RAM/swap, battery/AC state, BitLocker, boot mode, uptime/session distinction
- default-route IP, DNS, SSH client IP, username, terminal, shell, locale/login
- installer-origin detection and the installed edition's `tr300 update --json`
- Console code-page restoration and full/fast runtime budget

Record Windows version/build, install origin, command evidence, screenshots when
useful, and every discrepancy in `TESTING.md`. Fix forward before release.

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

## 5. v4.0.0 release sequence

Do not start this sequence until sections 4A–4C have evidence or an explicitly
documented, user-approved deferral.

1. Resolve hardware findings and rerun affected tests.
2. Bump only `Cargo.toml` version to `4.0.0` (the Rust 1.95 pins do not move).
3. Convert the current `[Unreleased]` changelog material into synchronized
   v4.0.0 technical/human release entries, include the concise Rust-library
   migration note, and refresh every required guide.
4. Run:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --locked --all-targets --workspace -- -D warnings
   cargo test --locked --workspace --all-targets
   cargo build --locked --release
   cargo package --locked --list
   cargo publish --dry-run --locked
   cargo audit
   dist plan
   ```

5. Commit `release: v4.0.0 - <summary>` and push `master`.
6. Wait for `.github/workflows/ci.yml` on the exact release SHA. Every macOS
   leg and the audit are hard gates.
7. Confirm `crates-publish.yml` published v4.0.0 from that same SHA.
8. Create and push only `v4.0.0`; never use `git push --tags`.
9. Verify cargo-dist's GitHub Release and `windows-installers.yml`, including
   all archives, checksums, four first-class Windows installers, and legacy
   `tr-300-installer.*` updater aliases.
10. Record real workflow IDs, SHA, package state, asset count, and live updater
    discovery in `TESTING.md`.

## 6. Homepage sequence

Only after section 5 is fully deployed:

1. Open `/Users/realemmetts/Downloads/temp_git/qube-machine-report-homepage`.
2. Read that repository's `AGENTS.md` / `CLAUDE.md` / project guide.
3. Update the TR-300 page from verified production facts—version, features,
   platform notes, commands, installer/package/release links.
4. Run its complete lint/test/build/link suite and review desktop/mobile output.
5. Commit and push its default branch.

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
- The Alienware continuation must preserve the validated/notarized macOS release
  path: no macOS collector/cfg branch, Apple target, artifact name, cargo-dist/
  release workflow, toolchain, signing/notarization input, or external secret
  change from Windows. The version-only package/root-lock bump, generated man
  version, and release records are allowed bookkeeping. A dependency-resolution,
  shared runtime, build, dist, or workflow change forces native + Rosetta Mac
  revalidation before any tag.

## 8. Completion definition

The v4.0.0 milestone is complete only when:

- Mac evidence above remains green in hosted blocking CI.
- Alienware, AMD Linux, and Pi 4 matrices are recorded and discrepancies fixed.
- all local release gates pass from a clean tree.
- crates.io and GitHub Release point to the exact tested commit/tag.
- all expected release assets and self-update discovery are verified.
- the homepage accurately reflects the deployed release and is pushed.
