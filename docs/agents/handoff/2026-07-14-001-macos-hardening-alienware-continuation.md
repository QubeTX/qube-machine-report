# Handoff: macOS hardening to Alienware continuation

**Date:** 2026-07-14 CDT  
**Session:** 001  
**Agent:** Codex  
**Repository:** `QubeTX/qube-machine-report`  
**Default branch:** `master`  
**Published version:** `3.17.0`  
**Planned version:** `4.0.0`, only after deferred hardware validation  
**Baseline:** `2d0c0b2470db603aa2e8058fee382b0dcaf0930c` (`v3.17.0`)  
**Checkpoint implementation commit:** `8e0df9e900c067ed92127f6750f50747cdbbda99`  
**Task IDs:** `#v400`, `#core`, `#plat`, `#test`, `#docs`, `#winhw`,
`#ship`, `#site`

This document is the portable continuation memory. The richer `.tasks/` board
and project memory are intentionally local and gitignored, so an Alienware
checkout must use this file first.

## Session Narrative

The request began as a broad reliability, stability, and information-accuracy
pass for every supported OS while a Mac was available for live testing. The
operator described the real fleet: Alienware Windows machines, MacBook Air and
MacBook Pro systems, an AMD Linux laptop, and a Raspberry Pi 4. The architecture
was required to remain one consistent cross-platform Rust application rather
than split per-OS products.

The initial audit started from the published `v3.17.0` commit and compared
source, documentation, release metadata, recent GitHub workflows, and live Mac
output. It found both Mac-specific precision problems and shared reliability
problems: repeated profiler invocations, misleading translated CPU frequency,
incorrect macOS available-memory presentation, weak optional-command timeout
handling, collision-prone report saves, hand-built JSON, predictable updater
staging, disk aggregation ambiguity, and evidence-free fallback claims.

After the compatibility-preserving CLI/JSON design was approved, the shared
hardening and platform collector corrections were implemented without changing
the JSON schema version. The Mac work was then tested on an M2 MacBook Pro both
as native arm64 and as the x86_64 release binary under Rosetta. The user
clarified that “complete macOS now” includes enforcing the work, not merely
proving it locally, so macOS test/build/speed jobs and the RustSec audit were
restored as blocking CI gates.

The release authorization was deliberately narrowed at handoff time. A new
crate version on `master` would immediately enter the crates.io workflow, while
live Alienware, AMD Linux, and Raspberry Pi evidence is still intentionally
pending. Therefore this checkpoint keeps `Cargo.toml` at the already-published
`3.17.0`, pushes the finished Mac/shared implementation and its enforcement,
and leaves the actual `4.0.0` version bump, tag, deployment, and homepage update
for the continuation after the remaining hardware passes.

The final API review corrected the planned release from 3.18.0 to 4.0.0. Public
Rust records gained fields and selected collector helpers changed signature;
direct external struct literals and exhaustive matches can stop compiling.
That requires a major version under the repository's SemVer promise even though
the CLI and existing schema-v1 JSON keys remain compatible. Changed records are
now `#[non_exhaustive]` so future additive fields do not repeat this break.

The final handoff requirement was to protect the Mac release/notarization path
from well-intentioned Windows-side cleanup. A precise freeze is now repeated in
this handoff, `AGENTS.md`, `CLAUDE.md`, `MASTER_PLAN.md`, `TESTING.md`, the
architecture decisions, and local task memory.

## Plan

1. **Complete and audit the macOS implementation — complete.**
   Consolidate native data gathering, correct every live discrepancy, preserve
   fast-mode behavior, protect privacy, and add parser/unit coverage.
2. **Harden shared cross-platform primitives — complete locally.**
   Bound subprocesses, make report writes non-destructive, structure JSON,
   clarify value semantics, and secure updater staging.
3. **Prove Mac behavior comprehensively — complete locally.**
   Run native arm64 and Rosetta x86_64 tests, release binaries, live parity,
   table/JSON/ASCII/privacy/updater/profile checks, and timing measurements.
4. **Enforce the evidence — implemented; hosted exact-SHA result follows push.**
   Remove macOS `continue-on-error`, make RustSec blocking, and verify every job
   on the exact pushed handoff commit.
5. **Preserve context — complete in tracked docs and local board memory.**
   Synchronize all required guides and leave exact continuation commands.
6. **Continue on real non-Mac hardware — deferred by operator direction.**
   Alienware Windows first, then AMD64 Linux and 64-bit Raspberry Pi 4.
7. **Release and homepage — deferred until all hardware and hosted gates pass.**

## Accomplished

### macOS collection and precision

- Full mode now uses one structured `system_profiler -json` snapshot instead of
  several independent profiler calls. The profiler is invoked natively when an
  arm64 host is running the x86_64 binary through Rosetta.
- Architecture distinguishes a native Apple Silicon process from
  `arm64 host / x86_64 (Rosetta 2)`. A translated compatibility frequency is
  intentionally absent rather than presented as the Apple chip clock.
- Boot state reports Normal, Safe, or Recovery based on actual evidence. It no
  longer labels “Apple Silicon” as a boot mode.
- macOS product version, build, maintained product/codename mapping, model,
  chip, performance/efficiency topology, GPU list, current display resolution,
  battery condition/health/cycle count, FileVault, locale, terminal host, shell,
  and login data are normalized with graceful absence.
- Disconnected displays and duplicate GPU names are filtered. Display parsing
  prefers the current logical resolution and retains native resolution context.
- The terminal mapping recognizes the Codex terminal host.
- Login parsing no longer mistakes a weekday token for a remote address.
- Serial number, hardware UUID, UDID, and equivalent unique identifiers are
  intentionally neither stored nor rendered.
- Fast mode remains subprocess-light and omits slow facts by contract.

### Memory, disk, CPU, uptime, and network semantics

- macOS used memory is `active + wired + compressed`; available memory is the
  exact `total - used` complement. This removes the previous “0 available”
  presentation and keeps used plus available equal to total.
- Disk reporting selects the root/system volume rather than summing unrelated
  or overlapping mounts. JSON distinguishes filesystem-used space from space
  currently available to the user.
- Unix load averages retain their raw runnable-queue values and expose a
  separately defined normalization by logical CPU capacity. Windows no longer
  fabricates 1m/5m/15m values from one instantaneous sample.
- Physical cores, Bare Metal, virtualization, and other facts are emitted only
  when there is positive evidence. Unknown remains `null`/absent.
- Sub-minute uptime is preserved instead of rounding an active machine to zero.
- Default-route-aware local address selection and parser fallbacks were improved
  while keeping optional network failures non-fatal.

### Shared runtime hardening

- Optional command probes drain stdout and stderr concurrently, cap captured
  output at 8 MiB, enforce timeouts, kill Unix process groups, and use a
  best-effort Windows Job Object so descendants do not outlive the probe.
- `--no-save` disables automatic Markdown persistence. Normal report saves use
  `create_new`, collision suffixes, OS-native Downloads discovery, and cleanup
  on incomplete writes; existing files and symlinks are never followed or
  overwritten.
- JSON is assembled as typed `serde_json::Value` data while remaining additive
  schema version 1. Existing keys remain available; new context keys explain
  collection mode and numeric definitions.
- Windows updater payloads use private randomly named temporary directories,
  size caps, RAII cleanup, strict release-version parsing, checksum-pair
  verification wording, and post-install version confirmation.
- Linux platform parsing gained additional AMD/Pi-friendly model and topology
  fallbacks. These compile and test, but real AMD laptop and Pi 4 claims remain
  explicitly unverified until those machines are available.
- Windows collectors gained Win32/WMI/PowerShell fallbacks and accuracy fixes.
  Windows compilation and linting pass; OEM/runtime claims await the Alienware.
- `Cargo.lock` resolves `crossbeam-epoch` 0.9.20, clearing the previously known
  RustSec advisory.

### Testing and enforcement

- Unit coverage now includes malformed/future profiler data, command timeout and
  large-output behavior, report collisions and symlinks, JSON shape/semantics,
  updater staging/version/checksum behavior, and isolated Unix profile edits.
- Integration tests use `--no-save` or temporary locations and do not write into
  the operator's real Downloads directory.
- macOS test, release-build, and auto-run-speed jobs are blocking in
  `.github/workflows/ci.yml`.
- `cargo audit` is a blocking CI job.
- The existing policy remains: hosted per-commit CI tests Apple Silicon; local
  Rosetta covers the x86_64 executable; cargo-dist still creates the physical
  Intel artifact at tag time without making every commit wait for `macos-13`.

### Local Mac evidence

Host: M2 MacBook Pro `Mac14,7`, macOS 26.3.1 build `25D2128`, 8 GiB RAM,
APFS, FileVault On.

- `cargo fmt --all -- --check` — pass.
- `cargo clippy --locked --all-targets --workspace -- -D warnings` — pass.
- `cargo test --locked --workspace --all-targets` — 115 library and 19
  integration tests pass natively.
- `cargo build --locked --release` — native release build passes.
- `cargo build --locked --release --target x86_64-apple-darwin` — Intel/Rosetta
  release build passes.
- `cargo test --locked --target x86_64-apple-darwin --workspace --all-targets`
  — 115 library and 19 integration tests pass under Rosetta.
- Native and Rosetta full/fast table and JSON output parse and agree with
  `sw_vers`, `sysctl`, `pmset`, and `fdesetup` for the relevant host facts.
- Rosetta architecture is explicit and its CPU frequency keys are `null`.
- `LC_ALL=C` output is ASCII-only and every table line is 51 columns.
- Serial/UUID/UDID privacy scan is clean.
- Native and Rosetta updater JSON correctly report the published 3.17.0 as
  current, with no update attempted.
- Timing: native full 0.90s, native fast 0.21s, Rosetta full 1.52s, Rosetta fast
  0.31s. The enforced fast-mode ceiling is 1.5s.
- `cargo audit` — no known advisory in the resolved dependency graph.
- Windows `cargo xwin check` and `cargo xwin clippy -D warnings` — pass.
- An isolated external path-dependent crate compiles and runs the v4-style
  high-level collection/default/render APIs and wildcard enum matching.
- Final five-run medians: native full 0.497s, native fast 0.234s, Rosetta full
  0.627s, Rosetta fast 0.338s.
- Clean committed-tree `cargo package --locked --list`,
  `cargo publish --dry-run --locked`, and `dist plan` — pass.

## Changed Files

The checkpoint deliberately touches a broad shared surface. Review this list
before deciding whether a later finding is cfg-local or invalidates Mac proof.

### Workflows and package metadata

- `.github/workflows/ci.yml` — macOS and audit failures are blocking; the Unix
  speed median script is shellcheck-clean and avoids unsafe array word-splitting.
- `.gitignore` — ignores the local `.tasks/` board/memory.
- `Cargo.toml` — Windows Job Object feature and temporary-directory support;
  version intentionally remains 3.17.0.
- `Cargo.lock` — dependency graph update including `crossbeam-epoch` 0.9.20.
- `man/tr300.1` — regenerated CLI documentation including `--no-save`.

### Rust source and tests

- `src/cli.rs` — `--no-save` CLI surface.
- `src/config.rs` — v4 non-exhaustive public configuration/output records.
- `src/error.rs` — non-exhaustive public application error enum.
- `src/collectors/command.rs` — bounded concurrent pipe draining and process
  cleanup.
- `src/collectors/cpu.rs` — evidence-backed topology/frequency/load semantics.
- `src/collectors/disk.rs` — native space semantics and root-volume handling.
- `src/collectors/memory.rs` — macOS VM accounting and available complement.
- `src/collectors/mod.rs` — aggregate fields, definitions, selection, and
  collection behavior.
- `src/collectors/network.rs` — default-route-aware local IP and parser fixes.
- `src/collectors/os.rs` — precise uptime and OS context.
- `src/collectors/platform/linux.rs` — Linux/Pi/AMD parsing and safe fallbacks.
- `src/collectors/platform/macos.rs` — consolidated structured Mac collector.
- `src/collectors/platform/mod.rs` — shared platform fields.
- `src/collectors/platform/windows.rs` — Windows native/fallback accuracy work.
- `src/collectors/session.rs` — terminal/login/battery/session normalization.
- `src/install/unix.rs` — isolated zsh/profile round-trip coverage.
- `src/install/prompt.rs` — non-exhaustive uninstall choice contract.
- `src/lib.rs` — exports/helpers aligned with additive report behavior.
- `src/main.rs` — `--no-save`, output flushing, platform setup cleanup.
- `src/migrate.rs` — non-exhaustive migration options with default-based use.
- `src/report.rs` — typed JSON and collision-safe Markdown persistence.
- `src/update.rs` — private bounded updater staging and stricter verification.
- `tests/integration.rs` — schema, flag, and side-effect-isolation coverage.

The v4 public future-proofing boundary marks these types `#[non_exhaustive]`:
`Action`, `Cli`, `CollectMode`, `CommandTimeout`, `CpuInfo`, `DiskInfo`,
`MemoryInfo`, `SystemInfo`, `NetworkInfo`, `NetworkInterface`, `OsInfo`,
`PlatformInfo`, `SessionInfo`, `Config`, `OutputFormat`, `BoxChars`, `AppError`,
`UninstallOption`, `MigrateOptions`, and `MarkdownSaveOutcome`. This is part of
the intentional Rust migration boundary, not a claim that existing external
struct literals or exhaustive matches remain source-compatible.

### Required synchronized documentation

- `CHANGELOG.md` — technical Unreleased Mac/shared checkpoint.
- `HUMAN_CHANGELOG.md` — matching plain-English checkpoint.
- `README.md` — current published version, fields, semantics, and `--no-save`.
- `CODEX_PROJECT.md` — current project summary and complete repository tree.
- `AGENTS.md` — canonical source/release/continuation contract and Mac freeze.
- `CLAUDE.md` — edit workflow, exact gates, code patterns, and Mac freeze.
- `MASTER_PLAN.md` — checkpoint status, deferred work, and release sequence.
- `TESTING.md` — local Mac evidence and remaining matrix.
- `docs/architecture-decisions.md` — rationale for data semantics, bounded
  helpers, blocking CI, Intel coverage, and the Alienware freeze.
- `docs/agents/handoff/2026-07-14-001-macos-hardening-alienware-continuation.md`
  — this portable memory.

### Local-only task state, intentionally not committed

- `.tasks/TASKS.md` — board with `#winhw` backlog and deferred release/site.
- `.tasks/tasks/{core,docs,dsg,plat,ship,site,test,winhw}.md` — task dossiers.
- `.tasks/memory/projects/tr-300.md` and `.tasks/CLAUDE.md` — hot memory/cache.

## Decisions

1. **One Rust architecture remains the product boundary.** Platform-specific
   collectors are cfg-gated implementation details, not divergent products.
2. **Unknown is data.** Optional probe failure yields absence, not fabricated
   “Bare Metal,” physical-core, frequency, encryption, or load claims.
3. **Full mode may be richer; fast mode must stay prompt-safe.** Slow profilers,
   encryption checks, and optional subprocesses remain out of fast mode.
4. **Schema version 1 is preserved.** New JSON fields are additive and nullable;
   existing consumers should continue to parse the old keys.
5. **Unique identifiers are out of scope.** Better hardware precision does not
   justify collecting serials, UUIDs, or UDIDs.
6. **Root/system disk is the headline.** Unrelated volumes are not summed into a
   misleading machine total.
7. **Checksums are integrity pairing, not independent authenticity.** The
   payload and sidecar share a release transport, so docs no longer overclaim.
8. **The current commit is a checkpoint, not a release.** Keeping 3.17.0 avoids
   triggering crates publication before real non-Mac hardware is checked.
9. **The deferred release is v4.0.0.** Public Rust record/signature changes are
   a SemVer-major boundary; CLI and existing JSON behavior remain compatible.
10. **CI enforcement is part of Mac completion.** A macOS or RustSec failure is a
   release blocker, not a warning.
11. **Do not add physical Intel macOS to every commit.** Rosetta plus tag-time
    Intel cargo-dist builds preserve coverage without the known runner-capacity
    failure mode.
12. **The Mac release path is frozen on Alienware.** Exact boundaries follow.

## How It Works

### Report collection

`SystemInfo::collect_with_mode()` launches core collectors in scoped threads,
converts core panics to an application error, and treats optional platform data
as a defaultable enrichment. Each platform collector returns evidence-backed
optional values. The aggregator computes named derived values only when their
inputs are valid and selects the OS root/system disk for the primary usage row.

On macOS full mode, one native `system_profiler -json` snapshot feeds pure
parsers. Cheap `sysctl`, `sw_vers`, and OS-native command fallbacks cover fields
that profiler omits. Under Rosetta, host/process architecture is explicit and
the profiler runs through the native arm64 path so hardware facts remain stable.
Fast mode deliberately bypasses that expensive snapshot.

### Optional commands

The shared command helper spawns a child with piped streams, consumes both
streams on independent threads, enforces a byte ceiling, waits only until the
deadline, and terminates the process tree on timeout/overflow where supported.
Malformed output, a missing utility, permissions, timeout, or excess output
returns absence to the optional collector instead of failing the report.

### Rendering and persistence

Table mode remains a 51-display-column fixed layout with Unicode-width-aware
truncation and an ASCII fallback. JSON is built structurally and serialized by
`serde_json`; schema-v1 fields include explicit metadata for numeric meanings.
Full table mode saves Markdown unless `--no-save` is set. The save path is
created atomically as a new file, tries deterministic collision suffixes, never
follows an existing symlink, and removes partial files after write failures.

### Self-update

Update discovery remains GitHub-release based. Cargo or the matching installer
strategy is selected from install origin. Downloaded Windows installer assets
are bounded and staged in a private temporary directory that cleans itself up.
The sidecar verifies the payload/sidecar pair, and a successful installer must
leave a binary whose reported version matches the intended release.

### CI and release

Default-branch CI gates format, clippy, tests, release builds, speed, audit, and
dist planning. The crates workflow checks out the exact CI-tested SHA and skips
an already-published manifest version. A new version must settle in CI/crates
before its explicit single tag is pushed; the tag then drives cargo-dist and the
follow-on Windows installer workflow.

## Known Issues

- Live Windows precision is not claimed. Hosted/cross compilation cannot prove
  Alienware hybrid CPU/GPU behavior, OEM firmware, battery/AC, BitLocker,
  VPN/default route, or the installed edition's updater origin.
- AMD64 Linux laptop and 64-bit Raspberry Pi 4 output remain unverified on real
  hardware. Keep any findings cfg-local when possible.
- Physical Intel Mac hardware was not available. The x86_64 release binary and
  full test suite passed through Rosetta; cargo-dist retains the Intel target.
- No explicit Apple signing/notarization configuration exists in tracked source.
  This handoff does not invent or certify that external configuration; it
  protects the existing external/release state from Windows-side mutation.
- The manifest still says 3.17.0 and the Unreleased notes are not a deployed
  4.0.0 release. Do not update the homepage yet.
- Some `Config` toggles remain historically under-wired in table rendering;
  that unrelated compatibility work was not folded into this checkpoint.
- Optional external utilities can still be unavailable or localized. The
  intended behavior is a missing optional field, not a report failure.

### Dead ends encountered, for future debugging context

- The installed `cargo-audit` does not accept `--locked`; the command failed on
  that unsupported flag, then `cargo audit` itself passed. CI also runs the
  supported form.
- An early `jq` smoke expected the old string/nesting assumptions and failed;
  the actual new schema uses numeric values and nested definition metadata. The
  corrected schema assertions pass.
- An early FileVault assertion expected a colon (`FileVault: On`), while the
  stable normalized public value is `FileVault On`. The collector was correct.
- An internal Rosetta frequency check expected an “unavailable” kind; the public
  JSON contract intentionally serializes the untrusted frequency as `null`.

## Important Context

### Absolute macOS release freeze for the Alienware agent

The user explicitly requires that Alienware work not damage the established
Mac release/notarization path.

**Do not change from Windows:**

- `src/collectors/platform/macos.rs`;
- any macOS `#[cfg]` branch in `src/collectors/cpu.rs`, `memory.rs`,
  `network.rs`, `os.rs`, `session.rs`, `src/install/unix.rs`, or `src/main.rs`;
- Apple target triples `aarch64-apple-darwin` and `x86_64-apple-darwin`;
- Apple archive names, shell installer names, install paths, or package inputs;
- cargo-dist target/installer settings or `cargo-dist-version`;
- `rust-toolchain.toml`, MSRV pins, `build.rs`, or the Mac package path;
- `.github/workflows/release.yml`;
- repository/organization secrets, certificates, keychain setup, signing,
  notarization, or any external release setting;
- dependencies/features or dependency-resolution entries in `Cargo.toml` /
  `Cargo.lock`;
- shared runtime, renderer, schema, command-helper, updater, or report code
  merely to “clean up” the implementation during hardware testing.

There is no explicit notarization configuration in the tracked repository. Do
not interpret that as missing work to recreate from the Alienware. Treat the
external state as opaque and established.

**Allowed without invalidating the Mac runtime stamp:**

- cfg-local Windows or Linux collector/installer fixes and their tests;
- Windows-only installer workflow/script corrections that do not rename shared
  release assets or edit `release.yml`;
- evidence, screenshots, testing ledgers, task status, changelogs, and docs;
- the eventual version-only `package.version = "4.0.0"` edit, the matching
  root `tr300` version entry in `Cargo.lock`, and generated man-page version text.

If a real Windows/Linux defect cannot be fixed without a forbidden/shared
change, stop. Commit or document the finding, do not tag, and return the release
candidate to a Mac. Rerun native arm64 plus Rosetta x86_64 tests, full/fast
table+JSON smokes, privacy/parity/timing checks, and hosted macOS CI. A green
Windows/Linux run alone is insufficient after such a change.

### Release baseline and state

- Default remote branch: `origin/master`.
- Published/tagged baseline: `v3.17.0` at
  `2d0c0b2470db603aa2e8058fee382b0dcaf0930c`.
- Verified v3.17 workflow runs:
  - CI `27116322705` — success;
  - Release `27116326346` — success;
  - Crates Publish `27116417286` — success;
  - Windows Installers `27116494691` — success.
- v3.17.0 has 28 GitHub Release assets and is the newest crates.io version at
  checkpoint time.
- Do not create or push a tag for this 3.17.0 checkpoint.
- The homepage repository is
  `/Users/realemmetts/Downloads/temp_git/qube-machine-report-homepage`; touch it
  only after v4.0.0 is actually deployed and all live links/assets are verified.

## What’s Next

### Exact first action on the Alienware

```powershell
git switch master
git pull --ff-only origin master
Get-Content .\docs\agents\handoff\2026-07-14-001-macos-hardening-alienware-continuation.md
git status --short --branch
```

Read `AGENTS.md` and `CLAUDE.md` before editing. Confirm the Mac freeze above is
understood. Do not regenerate cargo-dist or `release.yml`.

### Alienware Windows precision matrix (`#winhw`)

1. Record Windows edition/build, exact Alienware model, CPU, GPU mode, power
   state, current install path, and installer-origin registry marker.
2. Run the full local Rust gates available on Windows:

   ```powershell
   cargo fmt --all -- --check
   cargo clippy --locked --all-targets --workspace -- -D warnings
   cargo test --locked --workspace --all-targets
   cargo build --locked --release
   ```

3. Capture TR-300 without mutating Downloads:

   ```powershell
   .\target\release\tr300.exe --no-save
   .\target\release\tr300.exe --fast --no-save
   .\target\release\tr300.exe --ascii --no-save
   .\target\release\tr300.exe --json --no-save |
     Set-Content -Encoding utf8 .\tr300-alienware.json
   Get-Content -Raw .\tr300-alienware.json | ConvertFrom-Json |
     ConvertTo-Json -Depth 10
   ```

4. Compare every populated fact with native evidence, including:

   ```powershell
   Get-ComputerInfo
   Get-CimInstance Win32_OperatingSystem
   Get-CimInstance Win32_ComputerSystem
   Get-CimInstance Win32_Processor
   Get-CimInstance Win32_VideoController
   Get-CimInstance Win32_LogicalDisk
   Get-CimInstance Win32_BIOS
   Get-CimInstance Win32_BaseBoard
   Get-CimInstance Win32_Battery
   Get-NetIPAddress
   Get-DnsClientServerAddress
   Get-ItemProperty HKCU:\Software\TR300 -ErrorAction SilentlyContinue
   Get-BitLockerVolume
   ```

   Treat missing privileges/tools as evidence for graceful absence. Do not
   convert an unavailable value into a guessed negative.
5. Exercise the actually installed edition's `tr300 update --json` path and
   confirm the detected origin/strategy without switching MSI/EXE scope. Record
   exact command output and screenshots. Do not install a second edition.
6. If a defect is Windows-only, keep the fix in `cfg(windows)` source/tests and
   rerun the matrix plus hosted CI. If it needs shared code, invoke the Mac stop
   condition above.

### Deferred Linux hardware matrix (`#plat`)

- AMD64 laptop: compare OS/build/kernel, model/board/BIOS, physical/logical
  topology, AMD GPU, root filesystem bytes, memory, battery, encryption evidence,
  default-route address/DNS, login/session, and fast-mode time with native tools.
- Raspberry Pi 4: use a 64-bit OS, verify SoC/model/revision/topology, ARM64 arch,
  root filesystem, memory, temperature/firmware facts if surfaced, networking,
  and operation when optional desktop/system utilities are absent.
- Save exact distro/kernel/hardware evidence in `TESTING.md`; do not imply 32-bit
  Pi support from an untested target.

### Release v4.0.0 (`#ship`)

Only after Alienware, AMD Linux, Pi 4, and exact-SHA hosted CI are green:

1. Ensure no forbidden Mac/shared change occurred. If one did, return to a Mac
   and renew the complete native/Rosetta stamp before continuing.
2. Change only the package version to 4.0.0 and update the root `tr300` lockfile
   entry; convert Unreleased notes to a dated release, include the Rust-library
   migration note, and synchronize the full required documentation set.
3. Run:

   ```text
   cargo fmt --all -- --check
   cargo clippy --locked --all-targets --workspace -- -D warnings
   cargo test --locked --workspace --all-targets
   cargo build --locked --release --workspace
   cargo audit
   cargo package --locked --list
   cargo publish --dry-run --locked
   dist plan
   ```

4. Commit and push `master`. Wait for `.github/workflows/ci.yml` on the exact
   release SHA. Confirm `crates-publish.yml` publishes 4.0.0 from that same SHA.
5. Only then create and push the single tag:

   ```text
   git tag v4.0.0
   git push origin v4.0.0
   ```

6. Wait for cargo-dist release and Windows Installers workflows. Verify every
   target archive, canonical/legacy installer alias, MSI/EXE/checksum asset,
   crates.io page, release notes, update discovery, and exact tag SHA.
7. Update the post-release ledger from actual run IDs and asset evidence.

### Homepage (`#site`)

After deployment—not before—open the homepage repository, read its local agent
guide, update the TR-300 page from verified production facts, check every link,
run its full lint/test/build/link suite, review desktop/mobile output, and push
its default branch.

The continuation is finished only when the deployed release and homepage are
verified, not merely when local builds pass.
