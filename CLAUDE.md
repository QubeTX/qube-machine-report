# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **Companion file:** The canonical architecture decision ledger through
> 2026-07-17 (single-Rust/product/output semantics, v4 manual-save and
> fail-safe-update behavior, enforced Mac trust/freeze, `main` and Actions
> runtime, toolchain/release policy, Windows accuracy/distribution/
> consolidation, and install safety) lives in
> [`docs/architecture-decisions.md`](./docs/architecture-decisions.md) — the
> **why**: context, rejected alternatives, consequences, evidence, revalidation
> triggers, prior failure modes, and historical context.
>
> Forward-looking work tracking (what's shipped, what's pending, who picks up
> next session) is in [`MASTER_PLAN.md`](./MASTER_PLAN.md). Per-version
> verification logs are in [`TESTING.md`](./TESTING.md). Agent-facing
> repository tour and release checklist are in [`AGENTS.md`](./AGENTS.md).
> The current cross-machine continuation is
> [`docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md`](./docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md).
> The earlier `001` file remains the historical Mac/shared implementation
> checkpoint.
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

Published and manifest version: **4.0.1**. The maintainer explicitly authorized
release before live checks on the personal Alienware, AMD64 Linux laptop, and
Raspberry Pi 4. Those checks remain post-release patch work and must not be
reported as completed evidence. Managed-work antivirus behavior is a separate
endpoint-policy case, not personal Windows field-accuracy proof.

Observed distribution state: release source
`b67ad083503d0fff840af8467015d05c659268ea` passed exact-SHA CI/crates, both
hosted Apple notarization jobs, the 28-asset public audit, and supplemental
Windows packaging. Homepage commit
`d77397479ad2b1189cce86b5402eaf1cc966abdf` is deployed at
`https://reports.qubetx.com/`. Exact run IDs, submissions, and hashes live in
`TESTING.md` and the current tracked handoff.

v4 is intentional: new public Rust record fields and changed public collector
helper signatures are source-breaking under SemVer even though the CLI and
existing schema-v1 JSON keys remain compatible. Changed public record types are
`#[non_exhaustive]`; preserve that future-proofing and include a Rust migration
note in the eventual release docs.

## Task management system

This repo uses the SHAUGHV `tasks-*` system locally. `.tasks/TASKS.md` is the board source
of truth, `.tasks/MILESTONES.md` holds dated epics, and `.tasks/tasks/<id>.md` holds each
task's self-contained handoff, verification checklist, current status, and activity log.
Use proper indented checkbox subtasks for small board-visible requirements; use separate
top-level tasks with `(needs #id)` for work that needs independent status or sequencing.

Do not complete tasks over unchecked subtasks or open verification items, and do not close
milestones over open child tasks. Never store secrets in board or memory files; use the OS
keychain, environment variables, or `.tasks/secure/`. Resolve the live board from its own
`.tasks/.board-server.json` rather than assuming a port. Keep Active task status and activity
current as work proceeds. Relevant skills: `tasks-start`, `tasks-create`, `tasks-management`,
`tasks-update`, `tasks-memory`, `tasks-boards`, and `tasks-remove`.

`.tasks/` is intentionally gitignored, so board state does not follow a fresh
clone. Before a cross-machine push, update the local board **and** write a
tracked exhaustive handoff under `docs/agents/handoff/`; never leave the next
machine dependent on local-only task memory.

**Crate name:** `tr300` (lowercase, no hyphen — used by `cargo install tr300` and as the library import path `tr300`)
**Binary name:** `tr300` (no hyphen — set via `[[bin]] name = "tr300"`)
**Convenience alias:** `report` (created by `--install`)

The crate exposes both a binary (`src/main.rs`) and a library (`src/lib.rs` with public `generate_report()`, `format_bytes()`, etc.) — keep both surfaces working when refactoring.

## Development Commands

```bash
cargo build                      # Debug build
cargo build --locked --release   # Release build
cargo test --locked --workspace --all-targets # Run all unit/integration targets
cargo test --locked --target x86_64-apple-darwin --workspace --all-targets # Rosetta/Intel target on Apple Silicon
cargo test --lib                 # Library tests only
cargo test --test integration    # Integration tests (assert_cmd-based, in tests/integration.rs)
cargo test <test_name>           # Single test by name
cargo clippy --locked --all-targets --workspace -- -D warnings # CI lint
cargo fmt --all -- --check       # Check formatting
cargo audit                      # Blocking RustSec dependency gate
cargo run -- --fast              # Quick run (skips slow collectors)
cargo run -- --json              # JSON output
cargo run -- --ascii             # ASCII fallback mode
cargo run -- --report            # Explicitly save a full Markdown report
cargo run -- -r                  # Same save action
cargo run -- --save              # Same save action
cargo run -- -s                  # Same save action
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
- **JSON generation is structured** — `report.rs` builds a typed
  `serde_json::Value` and serializes it once. The compatibility `escape_json()`
  helper delegates to `serde_json`; do not reintroduce hand-built JSON.
- **UTF-8 / ASCII auto-fallback** — `main.rs::is_utf8_locale()` checks `LC_ALL`/`LC_CTYPE`/`LANG` on Unix and force-applies `--ascii` if none indicate UTF-8. Windows is treated as UTF-8 because `enable_utf8_console()` calls `SetConsoleOutputCP(65001)` when stdout is a terminal. Don't add code that prints box-drawing chars before this auto-detection runs.
- **Markdown saving is explicit-only** — Ordinary full/fast/JSON runs never
  call the writer. `-r`/`--report`/`-s`/`--save` request a full-table save;
  clap rejects save with fast/JSON/action modes. `--no-save` is a hidden
  compatibility no-op. The writer still uses the OS Downloads directory,
  `create_new`, collision suffixes, sync/cleanup, and never follows/overwrites
  an existing symlink/path.
- **Collector subprocesses use bounded helpers** — optional platform probes
  go through `src/collectors/command.rs`, which concurrently drains both pipes,
  caps output at 8 MiB, enforces fast/normal/slow budgets, and terminates the
  Unix process group or a best-effort Windows Job Object on timeout. Never
  replace this with raw `Command::output()` plus polling.

### Platform-Specific Code

Uses `#[cfg(target_os = "...")]` conditional compilation. Platform collectors live in `src/collectors/platform/`:
- **linux.rs** — `/proc`, `lscpu`, ZFS commands
- **macos.rs** — quick `sysctl`/`sw_vers`/`scutil`/`pmset`/`ioreg` fallbacks
  plus one structured full-mode `system_profiler` snapshot. Rosetta requests
  the native arm64 profiler slice so model/display/battery facts describe the
  host, while architecture still names the translated process. Unique hardware
  identifiers are intentionally excluded.
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
| `src/install/**` (alias / rc-file, exec-policy, uninstall) | `windows-install` | `atomic_write` never `std::fs::write` (and resolves a symlinked rc target so the link survives, E3); `check_marker_balance` before any mutation; exec-policy preflight before `$PROFILE`; `fail_install` for fs errors |
| `src/collectors/platform/windows.rs` (any Windows field) | `windows-accuracy` | WMI on a fresh worker thread (COM init); PSCore version by `(u64,u64,u64)` tuple, not string sort; Win11 = `CurrentBuild >= 22000`; no `net user` for last-login |
| `wix/**`, `wix-corporate/**`, `inno/**`, `windows-installers.yml`, `release.yml`, `src/update.rs`, `src/migrate.rs` | `windows-distribution-and-update` | the four product GUIDs are PERMANENT; registry `InstallSource` marker strings in lockstep (installer / `update.rs` / JSON); keep SHA256 + post-install verify; preserve both checked-in `release.yml` customizations (legacy aliases + fail-closed Apple trust) |
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
- Four first-class installers per release (Global / Corporate × MSI / EXE). **The four product GUIDs are PERMANENT** — regenerating breaks in-place upgrades. Corporate MSI source lives at `wix-corporate/` (not `wix/`) and is built by bare `candle`+`light` in `windows-installers.yml`; `release.yml` is cargo-dist-generated with checked-in alias and fail-closed Apple signing/notary customizations (preserve both).
- `HKCU\Software\TR300\InstallSource` marker (`msi-global` / `msi-corporate` / `exe-global` / `exe-corporate`) is the authoritative updater discriminator and is accepted only when its edition scope matches the running path. `classify_install_path()` maps `.cargo\bin` to the legacy chain and keeps marker-free Program Files/LocalAppData `Unknown` because the path cannot distinguish MSI from EXE. Marker strings stay in lockstep across installer template / `src/update.rs` / JSON output.
- Self-update: `cargo install` first on macOS/Linux (+ best-effort `rustup
  update stable`); registry-driven MSI/EXE strategies on Windows; bounded
  downloads land in a private randomized staging directory with explicit cleanup;
  **SHA256 sidecar verify + post-install `--version` check are load-bearing**
  (msiexec exit 0 does not prove binary replacement), while the checksum is
  described as corruption/mismatch detection rather than an independent
  signature; the cargo path falls through to the prebuilt installer on a
  crates.io lag (v3.16.0, U1); `is_newer` is semver-prerelease-aware.
- Likely antivirus/Group Policy/filesystem blocks during staged create/write/
  sync/launch become `PolicyBlocked`: stop the fallback chain, retain the current
  install, return exit 2 with official manual-release guidance, and never offer
  a force/direct binary replacement path. Cleanup failure is diagnostic; it
  does not turn a verified successful update into a false failure. Failed JSON
  includes `manual_install_url`; blocked JSON additionally includes
  `official_releases_url`.
- **Cross-method consolidation (`tr300 migrate-cleanup`, v3.17.0+):** hidden subcommand (`src/migrate.rs`; `#[value(hide=true)]` `Action::MigrateCleanup` + hidden flags in `cli.rs`) invoked by all four installers — interactive checkboxes AND silent self-update, **both default ON** — to keep ONE install at a time: removes a shadowing `~\.cargo\bin` copy and/or the *other* edition. Only ever deletes `tr300.exe` (allowlist); never cargo/rustup, the `.cargo\bin` PATH entry, `~/Downloads`, or the running install; needs-admin → skip + exit 0; advisory (never fails an install). Reuses `detect_install_origin`/`InstallOrigin` (now `pub(crate)`). **Edition paths + marker strings are in the SAME lockstep** as the installers / `update.rs`. WiX uses a deferred `Impersonate='yes'` `FileKey` CA (no `WixUtilExtension`); the Inno Global EXE does NOT pass `--user-profile` (no reliable Inno pre-elevation constant) and relies on the process-env fallback. Update preserves edition/scope: Corporate→Corporate (perUser), Global→Global (perMachine).

## Output & Runtime Contracts

### Elevation Tier (v3.10.0+)

TR-300 detects whether the current process is elevated (Unix `geteuid() == 0` / Windows `IsUserAnAdmin()` from shell32 — declared as a manual `extern` since `winapi-rs` doesn't bind it) and surfaces this via `SystemInfo.is_elevated`, plus a dim footer hint below the table on platforms where elevation unlocks more data.

- `tr300::is_elevated()` (in `src/lib.rs`) — runtime detection.
- `tr300::platform_has_elevated_data()` — compile-time per-target constant:
  `true` only on Linux. Windows/macOS do not make a blanket promise because a
  missing BitLocker/FileVault result does not prove elevation would fix the
  optional probe.
- `report::should_render_elevation_footer(is_elevated, mode, no_elevation_hint)` — the gate. Returns `true` only when the user is unelevated, on a platform with elevated data, in `Full` mode (never in `Fast` — the auto-run prompt must stay free), and hasn't passed `--no-elevation-hint`.
- `report::render_elevation_footer(use_colors)` — emits the Linux line with
  ANSI dim (`\x1b[2m...\x1b[0m`) when colors are enabled, plain text otherwise.
  The current user-facing hint is `Run with sudo for SMBIOS RAM module details`.

When adding a new elevated-only collector (e.g. `dmidecode` on Linux), gate it on `info.is_elevated` and let the footer hint cover the unelevated case rather than rendering a stub or warning row inside the table.

### JSON Schema Versioning (v3.10.0+)

Top-level `schema_version` (currently `1`) and `collection_mode` are on every
report JSON output. Defined as `report::SCHEMA_VERSION`. Bump only on
**breaking** schema changes—renames, type changes, or removals. Additive keys do
not require a bump. Current additive context includes OS build/codename/session
uptime; boot/display; default-route/SSH scopes; explicit physical/logical CPU
counts; frequency provenance; raw and normalized load units; root mount/
filesystem; disk available/value definitions; memory available/swap/value
definitions; and exact uptime. Optional absence means the collector could not
establish the value reliably within its budget.

`cpu.cores` remains the logical processor count for compatibility.
`cpu.load_*` is normalized percent of logical capacity; Unix-only
`cpu.load_raw_*` is the runnable-queue average. `disk.used_bytes` means allocated
bytes while `disk.available_bytes` is caller-available capacity. Memory JSON
must preserve the collector's `usage_definition` and `availability_definition`.

Top-level `elevated: bool` and `elevation_unlocks_more: bool` are also emitted on every JSON output. The latter is `true` only when the platform has elevated-only data AND the user isn't currently elevated — i.e. `true` indicates "re-running with sudo/Administrator would give you more". On macOS this is always `false`.

### Disk and memory semantics — do not "simplify"

Choose one system/root volume: `/`, the normalized Windows system drive, then
one largest fixed volume, then a removable fallback only when no fixed volume
exists. Never sum mounts—APFS/BTRFS/ZFS/bind/container mounts may overlap, and
Windows drives can be independent resources.

Disk used = total minus all-free blocks; available = capacity available to the
current caller. Those values need not sum to total because of reservations and
quotas. On macOS, memory used = active + wired + compressed from `vm_stat`, and
available is its exact total-minus-used complement. Other OSes use OS available
memory and derive used as total-minus-available. Full rationale and rejected
alternatives are in `docs/architecture-decisions.md`.

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

- **`ci.yml`** — every push to `main` + every PR: `fmt`, locked
  `clippy --all-targets -D warnings`, locked `test` (Linux + macOS ARM +
  Windows), locked `build` smoke (+`--version`/`--fast --json`), `speed`
  (5-run median of `tr300 --fast` < 1500 ms), blocking `audit`, and
  `dist-plan`. macOS test/build/speed are hard gates; do not restore the old
  v3.14.5 `continue-on-error` workaround.
- **`release.yml`** — cargo-dist v0.31.0, tag-triggered (`vX.Y.Z`); 6 targets + shell/PowerShell/MSI installers + legacy `tr-300-installer.*` aliases. It is generated and then intentionally checked in with the alias-copy and fail-closed Apple signing/notarization zones. Do not regenerate or edit across those zones without preserving both and reopening the Mac gate.
- **`crates-publish.yml`** — after `CI` succeeds on `main`; checks out the exact tested SHA, re-runs gates `--locked`, publishes to crates.io with `CARGO_REGISTRY_TOKEN`.

All four workflows use `actions/checkout@v6` on Node 24. Keep the branch CI and
crates workflow aligned with the release and supplemental Windows workflows;
do not reintroduce the deprecated Node 20-based checkout v4 action.

Reproduce locally: `cargo fmt --all -- --check && cargo clippy --locked
--all-targets --workspace -- -D warnings && cargo test --locked --workspace
--all-targets && cargo build --locked --release --workspace && cargo audit`.

### Intel macOS coverage policy

**Contract: per-commit CI never waits for a physical Intel runner; releases
still produce the artifact.** `ci.yml` has no `macos-13` entry (tested matrix =
Linux x64 glibc + macOS ARM + Windows x64); the v4 Mac validation additionally
builds and runs the x86_64 target locally under Rosetta. Rosetta is strong
translated-binary coverage but is not a claim about physical Intel hardware.
`[workspace.metadata.dist].targets` still includes `x86_64-apple-darwin`, so
tagged releases build it on Intel. Do not re-add the structurally capacity-
constrained runner to every commit and do not drop the Intel dist target.

### Enforced macOS trust path and Alienware freeze

`.github/workflows/release.yml` explicitly runs
`scripts/sign-notarize-macos.sh` after each Apple `dist build` and before
cargo-dist Post-build/upload. It imports the Developer ID certificate into an
ephemeral keychain, resolves the one expected identity there, and signs by its
certificate fingerprint so a duplicate display name in the login keychain is
not ambiguous. Because clean runners do not automatically search a newly
created keychain, the script temporarily prepends only that keychain for the
signing call and restores the original list immediately and from cleanup. It
signs `tr300` with identifier `com.qubetx.tr300` plus hardened runtime/timestamp;
verifies the embedded leaf fingerprint, authority, Team ID, identifier,
runtime, and timestamp; requires Apple Notary Service `Accepted`; repacks those
exact bytes; updates the archive sidecar and per-target manifest checksum; then
removes all decoded credentials. Missing credentials or any Apple failure
blocks hosting; never add an unsigned fallback.

The standalone CLI is not an `.app`/`.pkg` and has no staplable ticket container.
Use Apple acceptance plus `codesign --verify --strict` as the gate; a bare-binary
`spctl --type execute` message that the code is valid but not an app is expected.

Secret names (values never enter git/docs/logs/tasks/handoffs):
`APPLE_CERTIFICATE_P12_BASE64`, `APPLE_CERTIFICATE_PASSWORD`,
`APPLE_API_KEY_P8_BASE64`, `APPLE_API_KEY_ID`, and
`APPLE_API_ISSUER_ID`. Repository variables are
`APPLE_SIGNING_IDENTITY` and `APPLE_TEAM_ID`. The API key is least-privilege
Developer role and the selected certificate expires in 2031.

During later Windows/Linux/Pi work, do not edit `platform/macos.rs`, any macOS
`#[cfg]` branch, the two Apple target triples, artifact/installer names,
`scripts/sign-notarize-macos.sh`, the Apple step in `release.yml`, cargo-dist
configuration, `rust-toolchain.toml`, or Apple secrets/variables. Do not
regenerate or rotate this setup from Windows.

Prefer Windows/Linux cfg-local fixes. If a finding genuinely requires a shared
module, dependency/Cargo.lock, schema, renderer, command helper, workflow, or
Apple artifact change, Mac proof is invalidated until native arm64 and Rosetta
tests/full-fast/manual-save smokes are rerun on a Mac, a real archive notary
round-trip is repeated when Apple inputs changed, and hosted macOS CI/tag jobs
are green. Windows/Linux evidence alone is insufficient.

## Release Process

Uses **cargo-dist** (v0.31.0). The full ordered procedure — version bump → doc-set update → `main` push → wait for `ci.yml` green → wait for `crates-publish.yml` → tag push → watch `release.yml` → fix-forward loop — is the [`release`](./.claude/skills/release/SKILL.md) skill, with [`AGENTS.md`](./AGENTS.md) § "Release checklist" as the canonical 10-file doc list. Load-bearing invariants:

**v4.0.1 scope:** personal Alienware/AMD Linux/Pi 4 checks are post-release by
explicit maintainer decision. They never substitute for or waive the final
native/Rosetta, package/security, exact-SHA CI/crates, Apple notarization, and
release-asset gates. Keep the tasks/handoff honest and patch forward from real
hardware findings.

- Bump `Cargo.toml` `version`; update the doc set in lockstep (incl. `HUMAN_CHANGELOG.md` — see the `tr300-changelog` skill).
- Commit `release: vX.Y.Z - <summary>`; push and wait for `ci.yml` green on that exact commit.
- Confirm `crates-publish.yml` published (or skipped) from that SHA.
- **Tag only after `ci.yml` is green AND `crates-publish.yml` has resolved.** Create `git tag vX.Y.Z` and push the single tag explicitly (`git push origin vX.Y.Z`) — **never** `git push --tags`.
- The tag push triggers `release.yml` (6 targets + installers, incl. canonical
  `tr300-installer.*` and legacy `tr-300-installer.*` aliases). Both Apple jobs
  must sign and receive Notary `Accepted` before hosting; then
  `windows-installers.yml` must finish and the expected 28 assets must be
  verified before updating the homepage.

`Cargo.lock` is intentionally tracked; both local verification and the publish
workflow use `cargo publish --locked`. `allow-dirty = ["ci", "msi"]` is
intentional for the checked-in release customization and WiX source. After
changing `[workspace.metadata.dist]`, regenerate with `dist init` (the binary is
`dist`, not `cargo dist`) and preserve both the legacy installer-alias step and
the fail-closed Apple signing/notarization step.

## HUMAN_CHANGELOG.md (companion changelog)

`HUMAN_CHANGELOG.md` at the repo root is the plain-English mirror of `CHANGELOG.md` — same `## [X.Y.Z] - YYYY-MM-DD` headers and the same `### Added` / `### Changed` / `### Fixed` / `### Internal` groupings, with the technical noise stripped so a non-technical reader can answer "what shipped and why should I care?" in 30 seconds. `CHANGELOG.md` stays authoritative for agents and contributors. **When you add or amend a `CHANGELOG.md` entry, update `HUMAN_CHANGELOG.md` in the same commit — never let one drift ahead of the other.** The full strip list (CI run IDs, SHAs, error codes, API/function names, registry paths, GUIDs, LOC/memory deltas, task IDs, MSRV strings), the keep list (platform/edition names, installer types, user-typed commands and flags, the user-facing benefit), and the voice rules are the [`tr300-changelog`](./.claude/skills/tr300-changelog/SKILL.md) skill.
