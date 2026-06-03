# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **Companion file:** Long-form rationale for major architectural decisions
> (Windows accuracy patterns by version, MSRV / `rust-toolchain.toml` policy,
> auto-rustup self-update reasoning, Intel macOS CI coverage policy, install /
> update safety primitives) lives in
> [`docs/architecture-decisions.md`](./docs/architecture-decisions.md) — the
> **why**: rejected alternatives, prior failure modes, historical context.
>
> Forward-looking work tracking (what's shipped, what's pending, who picks up
> next session) is in [`MASTER_PLAN.md`](./MASTER_PLAN.md). Per-version
> verification logs are in [`TESTING.md`](./TESTING.md). Agent-facing
> repository tour and release checklist are in [`AGENTS.md`](./AGENTS.md).
>
> **How the edit-time rules are organized (read this).** The deep,
> subsystem-specific edit-time rules — Windows install, Windows accuracy
> collectors, Windows distribution/installers + self-update, the dev workflow,
> and the changelog contract — now live in **project skills** under
> [`.claude/skills/`](./.claude/skills/) so they load *on demand* instead of
> bloating this always-loaded file. Three layers, each with one job:
> **this file** keeps the tripwire index + short summaries (see
> [§ Edit-Time Rule Skills](#edit-time-rule-skills-load-before-touching-a-subsystem));
> the **skill** holds the full operational rules; the **decisions doc** holds
> the why. Project-original skills: `release`, `windows-install`,
> `windows-accuracy`, `windows-distribution-and-update`, `tr300-dev-workflow`,
> `tr300-changelog`. A `PreToolUse` hook
> ([`.claude/hooks/edit-time-reminder.ps1`](./.claude/hooks/edit-time-reminder.ps1),
> registered machine-local in `.claude/settings.local.json`) also injects the
> matching reminder when you edit a sensitive path — so the rule surfaces even
> if the skill didn't auto-trigger.
>
> **Vendored agent skills.** [`.claude/skills/`](./.claude/skills/) also bundles
> four Anthropic-distributed skills into the repo so every agent gets the
> same thinking toolkit regardless of plugin config: `brainstorming` (use
> before any feature work — explores intent before implementation),
> `critical-thinking` (four frameworks for decisions, design, problem-
> solving, and contemplating), `architecture` (create or evaluate an ADR),
> `system-design` (requirements → design → deep dive → scale → trade-offs).
> See [`.claude/skills/ATTRIBUTION.md`](./.claude/skills/ATTRIBUTION.md) for
> provenance and the upstream-sync rule.

## Project Overview

TR-300 is a cross-platform system information report tool written in Rust. It displays system information in a compact fixed-width table using Unicode box-drawing characters and bar graphs.

**Crate name:** `tr300` (lowercase, no hyphen — used by `cargo install tr300` and as the library import path `tr300`)
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
cargo run -- update              # Self-update from GitHub releases
cargo run -- --update            # Self-update from GitHub releases
cargo run -- install             # Add shell profile alias + auto-run
cargo run -- uninstall           # Interactive profile/binary cleanup
```

## Architecture

### Data Flow

1. **CLI parsing** (`src/cli.rs`) — `Cli` struct via clap derive macros
2. **Collection** (`src/collectors/mod.rs`) — `SystemInfo::collect_with_mode()` spawns 7 threads via `std::thread::scope` to gather data in parallel
3. **Rendering** (`src/report.rs`) — Converts `SystemInfo` → table or JSON string
4. **Output** (`src/render/table.rs`) — Draws Unicode/ASCII fixed-width tables

### Key Architectural Constraints

- **`src/cli.rs` must use `//` comments, not `//!`** — `build.rs` uses `include!("src/cli.rs")` to generate man pages via `clap_mangen`, and inner doc comments fail in that context.
- **Table rendering uses `unicode-width`** for display column calculation. Use `UnicodeWidthStr::width()` instead of `.chars().count()` in `render/table.rs`.
- **Fixed-width columns** are 12-char labels, 32-char data, 51 total width including borders.
- **Thread panics are caught** — collector threads use `.unwrap_or_else()` instead of `.unwrap()` on join handles, returning errors gracefully.
- **Shared utility** — `format_bytes()` lives in `src/lib.rs`; the per-module methods in `disk.rs`, `memory.rs`, `network.rs` delegate to it.
- **JSON escaping** handles control characters (0x00-0x1F) via `\u00xx` encoding in `escape_json()` in `report.rs`.
- **UTF-8 / ASCII auto-fallback** — `main.rs::is_utf8_locale()` checks `LC_ALL`/`LC_CTYPE`/`LANG` on Unix and force-applies `--ascii` if none indicate UTF-8. Windows is treated as UTF-8 because `enable_utf8_console()` calls `SetConsoleOutputCP(65001)` when stdout is a terminal. Don't add code that prints box-drawing chars before this auto-detection runs.
- **Markdown auto-save** — Manual full-mode runs (no `--fast`, no `--json`) call `report::save_markdown_report()` which writes to the user's Downloads folder and prints the path to stderr. `--fast` (auto-run) deliberately skips this to keep startup quiet and fast.
- **Collector subprocesses use bounded helpers** — optional platform probes should go through `src/collectors/command.rs` instead of raw `Command::output()`. Use the established fast/normal/slow budgets so missing tools and blocked commands return `None` rather than hanging the report.

### Platform-Specific Code

Uses `#[cfg(target_os = "...")]` conditional compilation. Platform collectors live in `src/collectors/platform/`:
- **linux.rs** — `/proc`, `lscpu`, ZFS commands
- **macos.rs** — `sysctl`, `scutil`, `pmset`, `ioreg`
- **windows.rs** — WMI queries via the `wmi` crate, Win32 API, registry

Windows-specific accuracy rules are extensive — load the **`windows-accuracy`** skill before editing `windows.rs`.

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

## Edit-Time Rule Skills (load before touching a subsystem)

The load-bearing "what to do / what not to undo" rules for each subsystem live in a
trigger-loaded skill. **Before editing one of these areas, load its skill** — it has the full
rules. The skills auto-load when their description matches your prompt, and a `PreToolUse` hook
([`.claude/hooks/edit-time-reminder.ps1`](./.claude/hooks/edit-time-reminder.ps1)) injects the
matching reminder when you edit one of the paths below, so the invariant surfaces even if the
skill didn't trigger. Deep rationale for every rule: [`docs/architecture-decisions.md`](./docs/architecture-decisions.md).

| If you're editing… | Load skill | Load-bearing tripwires (full rules in the skill) |
|---|---|---|
| `src/install/**` (alias / rc-file, exec-policy, uninstall) | `windows-install` | `atomic_write` never `std::fs::write`; `check_marker_balance` before any mutation; exec-policy preflight before `$PROFILE`; `fail_install` for fs errors |
| `src/collectors/platform/windows.rs` (any Windows field) | `windows-accuracy` | WMI on a fresh worker thread (COM init); PSCore version by `(u64,u64,u64)` tuple, not string sort; Win11 = `CurrentBuild >= 22000`; no `net user` for last-login |
| `wix/**`, `wix-corporate/**`, `inno/**`, `windows-installers.yml`, `release.yml`, `src/update.rs` | `windows-distribution-and-update` | the four product GUIDs are PERMANENT; registry `InstallSource` marker strings in lockstep (installer / `update.rs` / JSON); keep SHA256 + post-install verify; don't hand-edit auto-generated `release.yml` |
| `CHANGELOG.md`, `HUMAN_CHANGELOG.md` | `tr300-changelog` | update both in the **same commit** (strip technical noise from the human mirror) |
| `Cargo.toml` `rust-version`, `rust-toolchain.toml` | (§ MSRV policy below) | bump both in lockstep; keep `components = ["rustfmt", "clippy"]` |
| a non-trivial change end-to-end | `tr300-dev-workflow` | follow the 7 phases; never `--no-verify`; tag only after `ci.yml` green + `crates-publish` resolved |
| cutting / shipping a release | `release` | full ordered release procedure + fix-forward loop |

### Summaries for the high-traffic Windows subsystems

These three are the most frequently touched; their full rules are in the matching skill.

**`windows-install`** — install/uninstall flow (`src/install/`):
- **Shell-profile write safety (v3.15.2+).** All rc-file mutations route through `atomic_write` / `backup_once` / `check_marker_balance` in `src/install/mod.rs` (write-temp-then-rename; check marker balance *before* mutating). Never `std::fs::write` — the non-atomicity is the bug that truncated `~/.bashrc`.
- **Alias-collision warning (v3.15.3+, F17).** Read-only scan of rc files / PATH for an existing `report` before writing; best-effort, non-blocking, no subprocess.
- **Windows exec-policy preflight (v3.14.4+).** Set `RemoteSigned` for `CurrentUser` only when `Restricted`/`Undefined`, *before* writing `$PROFILE`; never widen, never touch `AllSigned`/`LocalMachine`; non-fatal.
- **Windows install error advisor (v3.14.5+).** fs failures route through `fail_install()` — rich stderr guidance (OneDrive vs AD/Intune/AppLocker/AV), then a concise `AppError`; always close with "manual `tr300` still works from the prompt."
- **Uninstall is interactive** (`ProfileOnly` / `Complete` / `Cancel`) — don't bypass the prompt.

**`windows-accuracy`** — Windows collectors (`src/collectors/platform/windows.rs`):
- Many rules encode OS/microcode quirks that look like bugs but aren't (Intel hybrid CPUID leaf 16h → 0; VBS vs Hyper-V; Fast Startup uptime). Don't "fix" them.
- OS via registry (Win11 = `CurrentBuild >= 22000`); arch via `IsWow64Process2`; hypervisor via CPUID leaf `0x40000000`.
- **WMI batch runs on a fresh worker thread** (COM init) — don't move it to the caller's thread (F22). 8-state native battery; native socket count; registry-prefer GPU; PSCore version by semver tuple; terminal via parent-process walk (cap 10 levels).
- VPN-aware default route via `GetBestInterfaceEx`; Fast Startup uptime annotation (full-mode only).
- **Minimum Windows: 10 1511 (build 10586)** — `IsWow64Process2` is statically linked; Win7 fails to load (F21).

**`windows-distribution-and-update`** — installers + self-update (`wix/`, `wix-corporate/`, `inno/`, `windows-installers.yml`, `src/update.rs`):
- Four first-class installers per release (Global / Corporate × MSI / EXE). **The four product GUIDs are PERMANENT** — regenerating breaks in-place upgrades. Corporate MSI source lives at `wix-corporate/` (not `wix/`) and is built by bare `candle`+`light` in `windows-installers.yml`; `release.yml` is cargo-dist-generated (don't hand-edit outside the allow-dirty zone).
- `HKCU\Software\TR300\InstallSource` marker (`msi-global` / `msi-corporate` / `exe-global` / `exe-corporate`) is the authoritative updater discriminator; `classify_install_path()` is legacy fallback only. Marker strings stay in lockstep across installer template / `src/update.rs` / JSON output.
- Self-update: `cargo install` first on macOS/Linux (+ best-effort `rustup update stable`); registry-driven MSI/EXE strategies on Windows; **SHA256 sidecar verify + post-install `--version` check are load-bearing** (msiexec exit 0 ≠ binary replaced); `is_newer` is semver-prerelease-aware.

## Output & Runtime Contracts

### Elevation Tier (v3.10.0+)

TR-300 detects whether the current process is elevated (Unix `geteuid() == 0` / Windows `IsUserAnAdmin()` from shell32 — declared as a manual `extern` since `winapi-rs` doesn't bind it) and surfaces this via `SystemInfo.is_elevated`, plus a dim footer hint below the table on platforms where elevation unlocks more data.

- `tr300::is_elevated()` (in `src/lib.rs`) — runtime detection.
- `tr300::platform_has_elevated_data()` — compile-time per-target constant: `true` on Linux + Windows, `false` on macOS. macOS gets no footer because sudo doesn't aesthetically unlock anything (`powermetrics` for live CPU freq is the main candidate, and the chip-name → frequency lookup table on Apple Silicon already gives a reasonable answer non-elevated).
- `report::should_render_elevation_footer(is_elevated, mode, no_elevation_hint)` — the gate. Returns `true` only when the user is unelevated, on a platform with elevated data, in `Full` mode (never in `Fast` — the auto-run prompt must stay free), and hasn't passed `--no-elevation-hint`.
- `report::render_elevation_footer(use_colors)` — emits the line with ANSI dim (`\x1b[2m...\x1b[0m`) when colors are enabled, plain text otherwise. Returns an empty string on macOS even if the gate is bypassed.
- The hint strings are hardcoded per platform in `render_elevation_footer`. Linux: `Run with sudo for motherboard, BIOS, and RAM slot details`. Windows: `Run as Administrator for BitLocker status`.

When adding a new elevated-only collector (e.g. `dmidecode` on Linux), gate it on `info.is_elevated` and let the footer hint cover the unelevated case rather than rendering a stub or warning row inside the table.

### JSON Schema Versioning (v3.10.0+)

Top-level `schema_version` (currently `1`) on every JSON output. Defined as `report::SCHEMA_VERSION`. Bump only on **breaking** schema changes — renames, type changes, or removals. Additive new keys do **not** require a bump. Current additive nullable keys include `os.machine_model`, `cpu.core_topology`, `memory.ram_slots`, and `system.{motherboard,bios}`; absence means the platform collector could not populate the value cheaply/reliably.

Top-level `elevated: bool` and `elevation_unlocks_more: bool` are also emitted on every JSON output. The latter is `true` only when the platform has elevated-only data AND the user isn't currently elevated — i.e. `true` indicates "re-running with sudo/Administrator would give you more". On macOS this is always `false`.

### Disk volume semantics — do not "fix"

sysinfo's reporting on BTRFS subvolumes (reports the pool, not the subvolume) and APFS containers (reports container free space, not per-volume) is **correct**, even though the numbers can look surprising. Don't change `aggregate_disk_usage()` to subtract overlapping space — you'll regress against what the OS itself reports in Disk Utility / `df`. ZFS pool sizes are similar.

## MSRV policy

MSRV is pinned in **two files that move in lockstep on every bump**:

1. **`Cargo.toml` `rust-version = "1.95"`** — cargo-side; produces the user-facing `error: package tr300@X.Y.Z cannot be built because it requires rustc N.M ...` for users on older toolchains.
2. **`rust-toolchain.toml`** at repo root — rustup-side override. **Both fields are required:**
   ```toml
   [toolchain]
   channel = "1.95"
   components = ["rustfmt", "clippy"]
   ```
   - `channel` — `release.yml` is cargo-dist-generated and has NO rustup setup step; without this, runners use their pre-installed rustc (<1.95) and the build fails with `error: rustc … is not supported`. This was the v3.13.1 fix.
   - `components` — when rustup honors a `rust-toolchain.toml` it installs only the default profile and **ignores any action-level `components:` field** in `ci.yml`, so the list must live here or the `fmt`/`clippy` jobs fail.

Use a **minor pin** (`"1.95"`, not `"1.95.0"` and not `"stable"`). **On every MSRV bump, edit both files in the same commit.** Full reasoning, the three rejected alternatives, and the v3.13.1 two-commit fix-forward narrative: [`docs/architecture-decisions.md` § "MSRV policy"](./docs/architecture-decisions.md#msrv-policy-v3111-addendum-v3131).

## Development Workflow

The canonical cadence for any non-trivial change. **Full detail — each phase's sub-steps, the F.1–F.6 documentation block, the parallel-`Explore` + authoritative-research briefs — is the [`tr300-dev-workflow`](./.claude/skills/tr300-dev-workflow/SKILL.md) skill.** Lightweight one-line fixes (typos, version bumps) can skip phases 1–2 but never skip phase 5.

1. **Plan** (read-only) — plan mode; parallel `Explore` agents; authoritative-source research before designing; phase into 4–6 PRs; end with `ExitPlanMode`.
2. **Task tracking** — `TaskCreate` upfront (PR tasks + per-plan-ID sub-tasks + F-block + tests); `TaskUpdate` as you go, never batch.
3. **Implement** — one PR at a time; read every file before editing; `cargo check` after each meaningful change; run the full local gate per PR.
4. **Document (F.1–F.6)** — `CHANGELOG.md` + `HUMAN_CHANGELOG.md` in lockstep, `README.md`, `CLAUDE.md` / the matching subsystem skill, `Cargo.toml` version, auto-memory, `TESTING.md`.
5. **Verify** — re-run the local gate green; Codex review (`codex:codex-rescue`) for non-trivial PRs; manual matrix for touched platforms.
6. **Commit + push** — `git-master` for the local commit; `ci-tester` `[PASS]` *before* pushing; never `--no-verify`, never bypass signing.
7. **Close out** — mark the PR task complete; deferred PRs only run on explicit ask.

## CI

Three GitHub Actions workflows guard release quality (full job-by-job detail + local-repro commands: the [`tr300-dev-workflow`](./.claude/skills/tr300-dev-workflow/SKILL.md) skill):

- **`ci.yml`** — every push to master + every PR: `fmt`, `clippy --all-targets -D warnings`, `test` (Linux + macOS ARM + Windows), `build` smoke (+`--version`/`--fast --json`), `speed` (5-run median of `tr300 --fast` < 1500 ms), `audit` (advisory-only), `dist-plan`.
- **`release.yml`** — cargo-dist v0.31.0, tag-triggered (`vX.Y.Z`); 6 targets + shell/PowerShell/MSI installers + legacy `tr-300-installer.*` aliases. Auto-generated — don't hand-edit outside the allow-dirty zone.
- **`crates-publish.yml`** — after `CI` succeeds on master; checks out the exact tested SHA, re-runs gates `--locked`, publishes to crates.io with `CARGO_REGISTRY_TOKEN`.

Reproduce locally: `cargo fmt -- --check && cargo clippy --all-targets --workspace -- -D warnings && cargo test --workspace --all-targets && cargo build --release --workspace`.

### Intel macOS coverage policy

**Contract: CI never blocks on Intel; releases still produce the artifact.** `ci.yml` has **no `macos-13` entry** (tested matrix = Linux x64 glibc + macOS ARM + Windows x64); `[workspace.metadata.dist].targets` in `Cargo.toml` **still includes `x86_64-apple-darwin`** so `release.yml` builds it on a `macos-13` runner at tag time. **Don't re-add `macos-13` to `ci.yml`** (runner capacity has structurally retired) and **don't drop the Intel dist target** (users on 2019/2020 Intel hardware deserve a working binary). Full reasoning + the concrete queue-time history: [`docs/architecture-decisions.md` § "Intel macOS coverage policy (v3.11.2+)"](./docs/architecture-decisions.md#intel-macos-coverage-policy-v3112) and the `tr300-dev-workflow` skill.

## Release Process

Uses **cargo-dist** (v0.31.0). The full ordered procedure — version bump → doc-set update → master push → wait for `ci.yml` green → wait for `crates-publish.yml` → tag push → watch `release.yml` → fix-forward loop — is the [`release`](./.claude/skills/release/SKILL.md) skill, with [`AGENTS.md`](./AGENTS.md) § "Release checklist" as the canonical 10-file doc list. Load-bearing invariants:

- Bump `Cargo.toml` `version`; update the doc set in lockstep (incl. `HUMAN_CHANGELOG.md` — see the `tr300-changelog` skill).
- Commit `release: vX.Y.Z - <summary>`; push and wait for `ci.yml` green on that exact commit.
- Confirm `crates-publish.yml` published (or skipped) from that SHA.
- **Tag only after `ci.yml` is green AND `crates-publish.yml` has resolved.** Create `git tag vX.Y.Z` and push the single tag explicitly (`git push origin vX.Y.Z`) — **never** `git push --tags`.
- The tag push triggers `release.yml` (6 targets + installers, incl. canonical `tr300-installer.*` and legacy `tr-300-installer.*` aliases).

`Cargo.lock` is intentionally tracked; both local verification and the publish workflow use `cargo publish --locked`. After changing `[workspace.metadata.dist]` in `Cargo.toml`, regenerate with `dist init` (the binary is `dist`, not `cargo dist`) and preserve the legacy installer-alias step.

## HUMAN_CHANGELOG.md (companion changelog)

`HUMAN_CHANGELOG.md` at the repo root is the plain-English mirror of `CHANGELOG.md` — same `## [X.Y.Z] - YYYY-MM-DD` headers and the same `### Added` / `### Changed` / `### Fixed` / `### Internal` groupings, with the technical noise stripped so a non-technical reader can answer "what shipped and why should I care?" in 30 seconds. `CHANGELOG.md` stays authoritative for agents and contributors. **When you add or amend a `CHANGELOG.md` entry, update `HUMAN_CHANGELOG.md` in the same commit — never let one drift ahead of the other.** The full strip list (CI run IDs, SHAs, error codes, API/function names, registry paths, GUIDs, LOC/memory deltas, task IDs, MSRV strings), the keep list (platform/edition names, installer types, user-typed commands and flags, the user-facing benefit), and the voice rules are the [`tr300-changelog`](./.claude/skills/tr300-changelog/SKILL.md) skill.
