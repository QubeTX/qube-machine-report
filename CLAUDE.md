# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TR-300 is a cross-platform system information report tool written in Rust. It displays system information in the exact TR-200 format using Unicode box-drawing tables with bar graphs for resource usage.

**Crate name:** `tr-300` (hyphenated — used by `cargo install tr-300` and as the library import path `tr_300`)
**Binary name:** `tr300` (no hyphen — set via `[[bin]] name = "tr300"`)
**Convenience alias:** `report` (created by `--install`)

The crate exposes both a binary (`src/main.rs`) and a library (`src/lib.rs` with public `generate_report()`, `format_bytes()`, etc.) — keep both surfaces working when refactoring.

## Development Commands

```bash
cargo build                      # Debug build
cargo build --release            # Release build
cargo test                       # Run all tests (unit + integration + doc)
cargo test --lib                 # Library tests only
cargo test --test integration    # Integration tests (assert_cmd-based, in tests/integration.rs)
cargo test <test_name>           # Single test by name
cargo clippy -- -D warnings      # Lint (CI mode, warnings = errors)
cargo fmt -- --check             # Check formatting
cargo run -- --fast              # Quick run (skips slow collectors)
cargo run -- --json              # JSON output
cargo run -- --ascii             # ASCII fallback mode
cargo run -- --update            # Self-update from GitHub releases
```

## Architecture

### Data Flow

1. **CLI parsing** (`src/cli.rs`) — `Cli` struct via clap derive macros
2. **Collection** (`src/collectors/mod.rs`) — `SystemInfo::collect_with_mode()` spawns 7 threads via `std::thread::scope` to gather data in parallel
3. **Rendering** (`src/report.rs`) — Converts `SystemInfo` → table or JSON string
4. **Output** (`src/render/table.rs`) — Draws Unicode/ASCII tables with exact TR-200 layout

### Key Architectural Constraints

- **`src/cli.rs` must use `//` comments, not `//!`** — `build.rs` uses `include!("src/cli.rs")` to generate man pages via `clap_mangen`, and inner doc comments fail in that context.
- **Table rendering uses `unicode-width`** for display column calculation. Use `UnicodeWidthStr::width()` instead of `.chars().count()` in `render/table.rs`.
- **Fixed-width columns** match TR-200: 12-char labels, 32-char data, 51 total width including borders.
- **Thread panics are caught** — collector threads use `.unwrap_or_else()` instead of `.unwrap()` on join handles, returning errors gracefully.
- **Shared utility** — `format_bytes()` lives in `src/lib.rs`; the per-module methods in `disk.rs`, `memory.rs`, `network.rs` delegate to it.
- **JSON escaping** handles control characters (0x00-0x1F) via `\u00xx` encoding in `escape_json()` in `report.rs`.
- **UTF-8 / ASCII auto-fallback** — `main.rs::is_utf8_locale()` checks `LC_ALL`/`LC_CTYPE`/`LANG` on Unix and force-applies `--ascii` if none indicate UTF-8. Windows is treated as UTF-8 because `enable_utf8_console()` calls `SetConsoleOutputCP(65001)` when stdout is a terminal. Don't add code that prints box-drawing chars before this auto-detection runs.
- **Markdown auto-save** — Manual full-mode runs (no `--fast`, no `--json`) call `report::save_markdown_report()` which writes to the user's Downloads folder and prints the path to stderr. `--fast` (auto-run) deliberately skips this to keep startup quiet and fast.

### Platform-Specific Code

Uses `#[cfg(target_os = "...")]` conditional compilation. Platform collectors live in `src/collectors/platform/`:
- **linux.rs** — `/proc`, `lscpu`, ZFS commands
- **macos.rs** — `sysctl`, `scutil`, `pmset`, `ioreg`
- **windows.rs** — WMI queries via the `wmi` crate, Win32 API, registry

### Fast Mode (`CollectMode::Fast`)

`--fast` skips slow subprocess calls for sub-second startup. Auto-run uses `tr300 --fast`; the `report` alias runs full mode. What gets skipped varies by platform — see the table in each platform collector.

### Build Script (`build.rs`)

Auto-generates `man/tr300.1` man page at build time using `clap_mangen`. Reads CLI definition via `include!("src/cli.rs")`.

## Code Patterns

### Adding a New Collector Field
1. Add field to `SystemInfo` in `src/collectors/mod.rs`
2. Collect the value in the parallel thread scope in `SystemInfo::collect_with_mode()`
3. Add row in `generate_table()` in `src/report.rs`
4. Add field to JSON output in `generate_json()` in `src/report.rs`

### Error Handling
Custom error types in `src/error.rs` using `thiserror`:
- `AppError::SystemInfo` — Collection failures
- `AppError::Platform` — Platform-specific failures
- `AppError::Io` — File/IO errors
- `AppError::Config` — Configuration errors
- `AppError::Wmi` — WMI query failures (Windows only)

### Installation System

`--install` / `--uninstall` modify shell profiles to add a `report` alias and auto-run. Installation blocks are wrapped in marker comments for idempotent cleanup. Legacy TR-100/TR-200 configs are removed automatically.

- **Unix/macOS** — `src/install/unix.rs` modifies `~/.bashrc` and/or `~/.zshrc`
- **Windows** — `src/install/windows.rs` modifies PowerShell `$PROFILE`

`--uninstall` is interactive (`src/install/prompt.rs`): the user picks `ProfileOnly`, `Complete` (also deletes the binary), or `Cancel`. The `Complete` path uses `find_binary_location()` + `confirm_complete_uninstall()` to show the path before deleting. Don't bypass the prompt unless the user has explicitly opted into a non-interactive variant.

### Windows accuracy patterns (v3.11.0+)

- **OS detection** reads `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`
  directly (`get_os_info_from_registry`) and overrides sysinfo. Detects Win11
  by `CurrentBuild >= 22000` because the registry `ProductName` is frozen at
  "Windows 10". Adds `DisplayVersion` (release ID like `25H2`) and `UBR` to the
  kernel string for richer output.
- **Architecture detection** (`get_architecture`) calls `IsWow64Process2` via a
  manual `extern "system"` linked against `kernel32`. Returns the host's
  native machine even when the binary itself runs under emulation. Handles
  `IMAGE_FILE_MACHINE_AMD64`, `_ARM64`, `_I386`, `_ARM`. Annotates emulation
  in the form `aarch64 (x86_64 emulation)` when process arch ≠ host arch.
- **CPU frequency** (`cpu.rs::collect`) combines CPUID leaf 16h + Windows
  `CallNtPowerInformation(ProcessorInformation)` + sysinfo, using
  `Iterator::max`. Leaf 16h returns 0 on Intel hybrid (Meteor/Lunar/Arrow Lake)
  — that's a documented Intel microcode change, not a bug. Falls through to
  the next source.
- **Hypervisor detection** (`detect_virtualization_wmi`) calls
  `cpuid_hypervisor_brand()` first (CPUID leaf 0x40000000, 12-byte vendor
  string) and disambiguates the Win11 VBS edge case: if CPUID returns
  `Microsoft Hv` but the SMBIOS manufacturer is a normal OEM (not Microsoft
  Corp), the result is `Bare Metal (Hyper-V/VBS)` instead of `Hyper-V`. Real
  Hyper-V VMs always have Microsoft Corp as manufacturer.
- **Last-login** (`get_last_login_windows`) calls `WTSQuerySessionInformation`
  via a manual `extern "system"` linked against `wtsapi32` (the constants
  `WTS_CURRENT_SESSION = 0xFFFFFFFF`, `WTSLogonTime = 17`,
  `WTSConnectTime = 14` are declared inline). Falls back to a boot-time
  derivation from `GetTickCount64` because Windows leaves the WTS time fields
  at 0 for local console sessions on most modern installs (auto-login + Fast
  Startup mask the actual logon timestamp). The previous `net user`-based
  parsing returned localized strings and "Never" — gone.
- **BitLocker** (`get_bitlocker_status`) queries `Win32_EncryptableVolume` in
  the `ROOT\CIMV2\Security\MicrosoftVolumeEncryption` namespace via the `wmi`
  crate's `WMIConnection::with_namespace_path`. Try-and-degrade pattern: on
  modern Win11 Device Encryption hosts this is readable non-admin and the
  `ENCRYPTION` row renders; on older Win10 / domain configurations the WMI
  call returns access-denied → `None` and the row is gracefully omitted; the
  elevation footer hint covers the unelevated case.

### Self-Update (`--update`)

`src/update.rs` checks `https://api.github.com/repos/QubeTX/qube-machine-report/releases/latest` (15s timeout via `ureq`), compares against `VERSION` from `Cargo.toml`, and re-runs the install method that placed the binary:
- `~/.cargo/bin/...` → `cargo install tr-300 --force`
- Otherwise → re-pipes the shell or PowerShell installer URL

`--update --json` emits a single JSON object with `current_version`, `latest_version`, `update_available`, and `success`. Exit codes: `0` success, `2` failure.

**Auto-rustup on the cargo path (v3.11.1+).** When `execute_update()` takes the
`InstallMethod::Cargo` branch it first calls `rustup_update_stable_best_effort()`,
which probes for `rustup` on PATH (via `rustup --version`, redirecting both
stdout and stderr to `Stdio::null()` so the probe is silent) and, if found,
runs `rustup update stable` and prints `Updating Rust toolchain (rustup
update stable)…` so the user sees what's happening. Any failure — rustup
absent, network timeout, locked toolchain, permission error — is *non-fatal*:
we discard the result with `let _ =` and proceed straight to the
`cargo install tr-300 --force` call. The Installer (cargo-dist shell/PS)
branch never touches Rust because it downloads a prebuilt binary.

*Why this exists — the failure mode it prevents:* TR-300's MSRV tracks the
GitHub Actions `stable` toolchain and moves whenever Rust ships a stable
release that promotes a lint we trigger or changes safety classifications
on stdlib intrinsics we use (cf. the 1.95 `__cpuid` reclassification that
prompted this change). Without auto-rustup, a user who installed via
`cargo install tr-300` on Rust 1.94 and then later runs `tr300 --update`
against a release built with `rust-version = "1.95"` would see cargo print
`error: rustc 1.94.0 is not supported by the following package: tr-300@…
requires rustc 1.95`, our `execute_update` would propagate that as a
non-zero exit, and the user would be silently stuck on the stale binary
forever — they'd assume `--update` "doesn't work" and either give up or
manually research the toolchain pin themselves. The 5–30 seconds spent on
a redundant `rustup update stable` (effectively a no-op when already
current — rustup just prints `info: cleaning up downloads & tmp directories`
and exits) is dramatically cheaper than that user-experience failure, and
this pattern means MSRV bumps in future releases stop being a coordination
problem with users.

*Why best-effort instead of "fail loudly if rustup isn't there":* not every
user manages Rust through rustup. Distro packages (Debian's `rustc`/`cargo`,
Homebrew's `rust`, NixOS's nixpkgs Rust), CI environments where rustup is
intentionally absent, and corporate-managed toolchains all install Rust
without putting `rustup` on PATH. Hard-failing in those cases would be
worse than the status quo (we'd block working updates on a tool we don't
need). Probing first and silently skipping when it's missing means we help
the rustup majority while not surprising the minority — they just see the
plain `cargo install` path and any MSRV mismatch surfaces normally, with
the standard cargo error pointing at `rust-version` so they can update
their distro/Homebrew Rust on their own terms.

*Why we don't probe rustc version and conditionally call rustup:* the
naive alternative — parse `rustc --version`, compare against an MSRV
constant, only run rustup when older — adds two new failure modes (parse
errors, drift between the constant and `Cargo.toml`'s `rust-version`) and
saves at most a few seconds. Always running `rustup update stable` is
simpler, idempotent, and self-correcting; rustup itself decides whether
work is needed.

### MSRV policy (v3.11.1+)

`rust-version` is pinned in `Cargo.toml` and tracks the GitHub Actions
`stable` toolchain. As of 3.11.1 it's `1.95` because `std::arch::x86::__cpuid`
and `std::arch::x86_64::__cpuid` were reclassified as safe-to-call in
Rust 1.95 (no safety preconditions on x86/x86_64 — CPUID is universally
available), which made our `unsafe { __cpuid(_) }` wrappers in
`src/collectors/cpu.rs` and `src/collectors/platform/windows.rs` trip the
`unused_unsafe` lint. Under our `-D warnings` policy that's a hard build
error. Bump `rust-version` whenever a new stable lint or stdlib change
forces source edits — and at the same release pin so that users running
older toolchains hit cargo's MSRV check, not E0133s deep in collector
modules.

*Why pin MSRV instead of supporting older Rust via shims:* there are three
realistic alternatives, and we considered each.

1. **`#[allow(unused_unsafe)]` on every `unsafe { __cpuid(_) }` block.**
   Compiles on both old and new toolchains. *Rejected* because the `allow`
   is permanent — once added, even Rust toolchains where the lint is
   correct (i.e. the unsafe block really is necessary because someone
   added a genuinely-unsafe call inside it later) will silently swallow
   the warning and we'd miss real safety regressions. It also bloats every
   CPUID callsite with attribute noise that has to be re-justified at
   review time, and it propagates: every future stable lint we want to
   straddle adds another permanent `allow`. Tech debt that compounds.

2. **`#[cfg(rustc_version)]` ladders to gate per Rust version.** Requires
   pulling in `rustversion` or `rustc_version` build-script crates, adds a
   build-time fingerprint to every release, and means our source has two
   parallel implementations of the same logic — one with `unsafe`, one
   without — that have to be kept in lockstep. *Rejected* as
   over-engineering for a tool whose CI deliberately uses
   `dtolnay/rust-toolchain@stable` and ships from a single toolchain.

3. **Pin MSRV to the CI toolchain (this approach).** Cargo's existing
   `rust-version` field already enforces this without any source-level
   shims. Older toolchains get `error: package tr-300@3.11.1 cannot be
   built because it requires rustc 1.95.0 or newer, while the currently
   active rustc version is 1.94.0` — clear, actionable, and points at
   exactly the right knob to fix. Combined with auto-rustup in
   `--update`, users on rustup-managed toolchains never see the error at
   all because `tr300 --update` brings their stable forward in lockstep
   with the MSRV pin. Users on distro-managed toolchains see the clear
   error and can update on their own schedule.

The combination — pin in `Cargo.toml`, auto-rustup in `--update`, README
mentions `rustup update stable` ahead of `cargo install tr-300` — gives us
a coherent toolchain story across all three install paths (binary
installer, fresh `cargo install`, self-update) without source-level
compatibility shims.

### Elevation Tier (v3.10.0+)

TR-300 detects whether the current process is elevated (Unix `geteuid() == 0` / Windows `IsUserAnAdmin()` from shell32 — declared as a manual `extern` since `winapi-rs` doesn't bind it) and surfaces this via `SystemInfo.is_elevated`, plus a dim footer hint below the table on platforms where elevation unlocks more data.

- `tr_300::is_elevated()` (in `src/lib.rs`) — runtime detection.
- `tr_300::platform_has_elevated_data()` — compile-time per-target constant: `true` on Linux + Windows, `false` on macOS. macOS gets no footer because sudo doesn't aesthetically unlock anything (`powermetrics` for live CPU freq is the main candidate, and the chip-name → frequency lookup table on Apple Silicon already gives a reasonable answer non-elevated).
- `report::should_render_elevation_footer(is_elevated, mode, no_elevation_hint)` — the gate. Returns `true` only when the user is unelevated, on a platform with elevated data, in `Full` mode (never in `Fast` — the auto-run prompt must stay free), and hasn't passed `--no-elevation-hint`.
- `report::render_elevation_footer(use_colors)` — emits the line with ANSI dim (`\x1b[2m...\x1b[0m`) when colors are enabled, plain text otherwise. Returns an empty string on macOS even if the gate is bypassed.
- The hint strings are hardcoded per platform in `render_elevation_footer`. Linux: `Run with sudo for motherboard, BIOS, and RAM slot details`. Windows: `Run as Administrator for BitLocker status and full login history`.

When adding a new elevated-only collector (e.g. `dmidecode` on Linux), gate it on `info.is_elevated` and let the footer hint cover the unelevated case rather than rendering a stub or warning row inside the table.

### JSON Schema Versioning (v3.10.0+)

Top-level `schema_version` (currently `1`) on every JSON output. Defined as `report::SCHEMA_VERSION`. Bump only on **breaking** schema changes — renames, type changes, or removals. Additive new keys do **not** require a bump (so adding `cpu.p_cores`/`cpu.e_cores` in a later PR is fine without a schema bump). Document every nullable key in CLAUDE.md as it lands so contributors know which absences are intentional.

Top-level `elevated: bool` and `elevation_unlocks_more: bool` are also emitted on every JSON output. The latter is `true` only when the platform has elevated-only data AND the user isn't currently elevated — i.e. `true` indicates "re-running with sudo/Administrator would give you more". On macOS this is always `false`.

### Disk volume semantics — do not "fix"

sysinfo's reporting on BTRFS subvolumes (reports the pool, not the subvolume) and APFS containers (reports container free space, not per-volume) is **correct**, even though the numbers can look surprising. Don't change `aggregate_disk_usage()` to subtract overlapping space — you'll regress against what the OS itself reports in Disk Utility / `df`. ZFS pool sizes are similar.

## Development Workflow (canonical — follow for every change)

This is the workflow that proved itself during the v3.10.0 cross-platform accuracy pass. Follow it for any non-trivial change. Lightweight one-line fixes (typos, version bumps) can skip phases 1–2 but never skip phase 5.

### Phase 1 — Plan (read-only)

1. Enter Claude Code plan mode. Plans live at `C:\Users\hey\.claude\plans\<descriptive-name>.md` (the runtime tells you the path).
2. **Explore in parallel.** Spawn up to 3 `Explore` agents simultaneously (single message, multiple tool calls) for codebase context. Each agent gets a focused brief: where the field is collected, what the existing pattern is, what's already best-in-class.
3. **Research authoritative sources before designing.** For platform-specific work, dispatch parallel `general-purpose` agents (model: `opus`) with WebFetch / WebSearch / Firecrawl / Perplexity access. Require citations from Apple Developer Forums, Microsoft Learn, kernel.org, systemd man pages, freedesktop specs, sysinfo crate issues. Verdicts: ✅ best-in-class / ⚠️ acceptable / ❌ inaccurate.
4. **Build the plan incrementally.** Sections: Context · What's Already Best-in-Class (don't redo good work) · Per-Platform Fixes · Cross-Platform Reliability · Speed · New Data Points (with skip list) · Files to Modify · Implementation Task Checklist · Testing Strategy · Phasing & Sequencing · Verification.
5. **Phase the work** into PR-sized chunks (typical: 4–6 PRs). PR #1 is always the foundation primitives that later PRs depend on. Each PR has a docs/version block (`F.1`–`F.6`) at the end.
6. End with `ExitPlanMode` — do not text-prompt for plan approval.

### Phase 2 — Task tracking (`TaskCreate` upfront, `TaskUpdate` as you go)

After plan approval, create:

- **Top-level PR tasks** (one per PR), with `addBlocks`/`addBlockedBy` for sequencing.
- **Sub-task per plan ID** (`[PR1] D.1 …`, `[PR2] A.3 …`, etc.) with the spec verbatim from the plan plus LOC estimate. The user uses these to track progress, so be granular.
- **Per-PR doc block tasks** (`F.1`–`F.6`) and **test tasks** (unit + integration + manual matrix).

`TaskUpdate` to `in_progress` *before* starting any sub-task, and to `completed` *immediately* when done — never batch.

### Phase 3 — Implement (one PR at a time, sequentially)

1. **Read first.** Read every file you'll edit before the first `Edit` call. Don't guess at file structure.
2. **Edit minimally.** No drive-by refactors, no comments that explain *what* the code does, no error handling for impossible cases.
3. **`cargo check` after each meaningful change.** Catches issues while context is fresh.
4. **Run the full local gate after each PR completes** *before* moving to the next PR:
   ```bash
   cargo fmt -- --check
   cargo clippy --all-targets --workspace -- -D warnings
   cargo test --workspace
   cargo build --release
   ./target/release/tr300 --version
   ./target/release/tr300 --fast --json | head -5
   ./target/release/tr300 --ascii          # visual smoke test
   ```
5. **Time `--fast`** on the local platform; record before/after numbers in the PR description.

### Phase 4 — Per-PR documentation (the F-block — never skip)

Every PR completes this block before commit:

- `F.1` — `CHANGELOG.md` new `## [X.Y.Z] — YYYY-MM-DD` section at the top, Keep-a-Changelog voice, **reference task IDs in parens** for traceability.
- `F.2` — `README.md` updates: flag table, sample output, new subsections.
- `F.3` — `CLAUDE.md` architectural notes for any new pattern (cite man pages / Apple docs / Microsoft Learn URLs inline).
- `F.4` — `Cargo.toml` `version =` bump (minor for new fields/flags; patch for pure accuracy fixes).
- `F.5` — Auto memory writes at `C:\Users\hey\.claude\projects\C--Users-hey-Documents-GitHub-qube-machine-report\memory\`: keep `project_tr300_overview.md` (project) up-to-date, append to `feedback_tr300_constraints.md` (feedback) when the user adds a hard rule, update `MEMORY.md` index.
- `F.6` — `TESTING.md` append a `### vX.Y.Z — YYYY-MM-DD` log noting which manual matrix rows were re-verified and on which hardware.

### Phase 5 — Verification + independent review

1. Re-run the full local gate (Phase 3 step 4). Must be green.
2. **Codex review** (`Agent` tool, `subagent_type: "codex:codex-rescue"`) for non-trivial PRs. Use it to spot-check cross-platform safety / YAML / unsafe blocks where a second pair of eyes catches things stale eyes miss. Note: Codex's `gh pr diff` review path needs the PR to actually exist — open the PR first, then ask Codex to review it. Don't over-rely on its findings; double-check.
3. Manual matrix run for the platforms touched (`TESTING.md`).

### Phase 6 — Commit + push

- **Local commit**: `git-master` agent. No `ci-tester` needed for local-only operations.
- **Push to remote**: `ci-tester` agent FIRST. If `[FAIL]`, fix the failures — never skip hooks (`--no-verify`), never bypass signing. Once `ci-tester` is `[PASS]`, hand off to `git-master` for the push.
- **Tag a release**: bump version (already done in `F.4`), commit + push commits, then `git tag vX.Y.Z && git push --tags`. The tag push triggers cargo-dist's `release.yml`. Push the tag *only after* `ci.yml` has gone green on the commit being tagged.

### Phase 7 — Close out

Mark the parent PR task `completed` in `TaskList`. Move on to the next PR's parent task and start phase 3 again. PR #6 (and other "deferred" tasks) only run if the user explicitly asks after the previous PR lands.

## CI

Two GitHub Actions workflows guard the project:

- **`.github/workflows/ci.yml`** — runs on every push to master and every pull request. Jobs:
  - `fmt` — `cargo fmt --check` (Linux only)
  - `clippy` — `cargo clippy --all-targets --workspace -- -D warnings` (Linux only)
  - `test` — `cargo test --workspace --all-targets` on Linux + macOS ARM + Windows
  - `build` — `cargo build --release` smoke test on every platform, plus `--version` and `--fast --json` invocation to verify the binary actually runs
  - `speed` — measures `tr300 --fast` median wall-clock across 5 runs on Linux/macOS/Windows; fails the build if any platform's median exceeds the 1500 ms budget. Records numbers in the GitHub Actions step summary so PR reviewers see them.
  - `audit` — `cargo audit` against RustSec advisories (advisory-only via `continue-on-error: true`; flagged vulnerabilities should be triaged within one release cycle but don't gate PRs)
  - `dist-plan` — runs `dist plan` to verify cargo-dist config parses; catches dist regressions before they bite at tag time
- **`.github/workflows/release.yml`** — auto-generated by cargo-dist v0.31.0. Triggered by tag push (`vX.Y.Z`). Builds 6 targets and produces shell + PowerShell + MSI installers. Do not hand-edit; regenerate via `dist init` after changing `[workspace.metadata.dist]` in `Cargo.toml`.

To reproduce the CI gates locally:

```bash
cargo fmt -- --check
cargo clippy --all-targets --workspace -- -D warnings
cargo test --workspace --all-targets
cargo build --release --workspace
# Speed check (rough — CI uses 5-run median):
time ./target/release/tr300 --fast > /dev/null
```

If a CI job fails, click into the job logs first — `clippy` and `test` failures are usually obvious from the diff. Speed regressions print the per-run times and median in the step summary; correlate against the recent change set.

### Intel macOS coverage policy (v3.11.2+)

**What changed.** On 2026-04-28 the `macos-13` matrix entries were removed
from both the `test` and `build` jobs in `.github/workflows/ci.yml`. CI no
longer exercises Intel macOS x86_64. The Intel binary continues to ship
from tag-push releases — `[workspace.metadata.dist].targets` in
`Cargo.toml` still lists `x86_64-apple-darwin`, and cargo-dist's
`release.yml` still builds it on a `macos-13` runner at every `vX.Y.Z`
tag. CI on every commit no longer touches that runner.

**The triggering symptom.** The five CI runs immediately preceding this
change all stalled exclusively on the two Intel macOS jobs while every
other matrix cell finished within minutes. Concrete examples (run IDs +
queue times before manual cancellation): `#25023039347` ("fix(audit):
migrate users → uzers") sat queued **3h 20m+** before being cancelled to
unblock the next push, `#25022743655` (v3.11.1 release) cancelled at 7m
59s, `#25021879374` (docs commit) cancelled at 22m 34s, `#25021109459`
(v3.11.0 release) cancelled at 18m 41s, and `#24978816648` (v3.10.0
release) sat for **15h 50m** before cancellation. The repeated workflow
was: push → wait → realise Intel never picked up → `gh run cancel ...` →
push next commit, which the `concurrency: cancel-in-progress: true`
group then auto-cancelled anyway. Hours of latency per push, no Intel
runtime ever exercised.

**Why it's structural, not a glitch.** `macos-13` is GitHub's last public
Intel x86_64 macOS hosted runner label. There is no `macos-14`,
`macos-15`, or `macos-latest` Intel variant — Apple Silicon is the only
forward path on hosted runners. GitHub has been progressively winding
down the Intel hosted-fleet capacity through 2025 as Apple Silicon
became default; on top of that thin baseline, transient incidents like
the 2026-04-27 16:31 UTC "Actions experiencing degraded performance"
event push queue depth from "slow" to "indefinite." This is not a
problem we can wait out — capacity is not coming back.

**Why dropping CI coverage is acceptable for this project.** Apple
stopped selling Intel Macs in June 2023; the newest hardware Apple
shipped with an Intel CPU (the 2019 Mac Pro / 2020 Intel iMac /
MacBook Pro) is roughly three years old by 2026-04-28 and falling out
of macOS support tiers release-over-release. Critically, dropping Intel
*CI* does not mean dropping Intel *correctness coverage*: the
`#[cfg(target_os = "macos")]` gates in `src/collectors/platform/macos.rs`
are arch-agnostic, so Apple Silicon CI exercises every line of the
macOS path. The only thing ARM CI doesn't catch is genuinely
arch-specific behavior, of which TR-300's macOS path has effectively
none — no inline asm, `format_bytes()` and the table renderer are
arch-agnostic, sysinfo's `System` API hides arch internally, and the
`sysctl`/`scutil`/`pmset`/`ioreg` subprocess calls produce identical
output regardless of CPU. The accuracy delta from losing Intel CI
coverage is close to zero in practice, and any drift would show up at
tag-push time when cargo-dist builds the Intel target anyway.

**Why not just `continue-on-error: true` on the Intel matrix entry.**
Considered. Rejected because it leaves the queued-3-hours-and-then-
cancel UX intact for every push: the workflow's overall conclusion
would be `success`, but the dashboard would still show two perpetually
pending cells, and the run wouldn't be considered "complete" until
either the runner finally picked up (rare) or `cancel-in-progress`
killed it on the next push. That's theatrical coverage — the user
experience is identical to the current pain. Hard removal is the only
fix that actually changes the dashboard state.

**Why not also drop Intel from cargo-dist's release targets.**
Considered. Rejected per maintainer direction. Release-time builds run
on a tag push (cadence: minor/patch releases, ~weekly to monthly), and
that cadence is willing to absorb a multi-hour Intel queue wait — we
don't tag releases on a deadline. The cost of keeping
`x86_64-apple-darwin` in `[workspace.metadata.dist].targets` is one
extra `macos-13` job per *tag*, not per *commit*. Any user still on
2019/2020-era Intel hardware deserves a working binary download
without having to build from source. The contract is: **CI never
blocks on Intel; releases still produce the artifact.**

**What this implies for future contributors.**

1. Don't re-add `macos-13` to `ci.yml` without a concrete reason and a
   discussion of capacity risk. The default state is "Intel is not in
   CI, period."
2. If GitHub ever ships a hosted Intel macOS replacement label
   (unlikely), prefer that label over `macos-13` and re-evaluate
   whether re-adding to CI makes sense at that point.
3. If `release.yml` starts taking longer than ~2 hours at tag time
   because of `macos-13` queue depth, *that* is the signal to revisit
   dropping `x86_64-apple-darwin` from cargo-dist's targets. Until
   then, releases tolerate the wait.
4. If a macOS-arch-specific bug is reported by an Intel user, reproduce
   locally on Intel hardware (or via a one-shot self-hosted runner)
   rather than re-introducing CI coverage. The bug-rate doesn't justify
   the queue cost.

**Why "Builds 6 targets" in `## Release Process` still says 6.** Six
binary targets continue to ship from `release.yml`: Windows x64, macOS
Intel (x86_64), macOS ARM (aarch64), Linux x64 glibc, Linux x64 musl,
Linux ARM64. CI tests three of them (Linux x64 glibc, macOS ARM,
Windows x64). The mismatch between *tested* and *shipped* platforms
is intentional and is exactly what this section documents — see above.

## Release Process

Uses **cargo-dist** (v0.31.0) for fully automated cross-platform releases.

**Every release requires ALL of these steps:**

1. Bump `version` in `Cargo.toml`
2. Update `CHANGELOG.md` with new version section
3. Commit with message `release: vX.Y.Z - <summary>`
4. Create git tag: `git tag vX.Y.Z`
5. Push commits AND tags: `git push && git push --tags`

The tag push triggers GitHub Actions to build all 6 targets (Windows x64, macOS Intel/ARM, Linux x64 glibc/musl, Linux ARM64) and generate shell/PowerShell/MSI installers.

### Regenerating CI Workflow

After changing `[workspace.metadata.dist]` in Cargo.toml:
```bash
dist init    # Regenerates .github/workflows/release.yml
```

Note: The binary is `dist`, not `cargo dist` — it installs as a standalone command.

## Legacy Reference

`TR200-OLD/` contains the original TR-200 bash/PowerShell implementation for format reference.
