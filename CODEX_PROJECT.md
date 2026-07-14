# CODEX_PROJECT.md

## TL;DR

TR-300 is a standalone Rust CLI and library that produces compact, fixed-width
machine reports on macOS, Linux, and Windows. The published version is v3.17.0.
The default branch now contains an unreleased reliability/accuracy checkpoint:
macOS is implementation-complete and thoroughly verified on Apple Silicon and
under Rosetta; Windows live validation on the Alienware, AMD64 Linux, Raspberry
Pi 4, the v4.0.0 bump/release, and the public homepage update are deliberately
deferred.

Start the next session with
[`docs/agents/handoff/2026-07-14-001-macos-hardening-alienware-continuation.md`](./docs/agents/handoff/2026-07-14-001-macos-hardening-alienware-continuation.md),
then `AGENTS.md`, `CLAUDE.md`, `MASTER_PLAN.md`, and `TESTING.md`.

## Current Status

- Cargo package / binary / library import: `tr300`
- Published release: `3.17.0` (2026-06-08)
- Working manifest version: `3.17.0` — intentionally not bumped in this
  checkpoint, so default-branch crates publishing will skip an already-published
  version
- Planned next release: `4.0.0`, only after deferred hardware validation
- Major-version reason: public Rust structs gained fields and selected public
  collector helpers changed signature. CLI and schema-v1 JSON compatibility are
  retained; changed record types are now `#[non_exhaustive]` for safer future
  additive fields.
- MSRV: Rust `1.95`, pinned in both `Cargo.toml` and `rust-toolchain.toml`
- Default branch: `master`
- Release tooling: cargo-dist `0.31.0`
- Last source/docs verification: 2026-07-14 on a MacBook Pro M2, macOS 26.3.1
  build 25D2128

### Completed in the macOS checkpoint

- A single structured full-mode macOS snapshot supplies model, display, GPU,
  battery, boot-state, and virtualization facts with graceful fallbacks.
- Native arm64 and Rosetta x86_64 report the same hardware semantics; Rosetta
  is labeled explicitly and does not expose the translated 2.4 GHz compatibility
  value as a real CPU frequency.
- APFS root-volume and macOS memory figures use explicit, internally consistent
  definitions. Used plus available RAM equals total RAM.
- FileVault, battery, display, terminal, OS build/codename, core topology, locale,
  and last-login parsing have live and fixture coverage.
- JSON is built through `serde_json` while preserving schema version 1 and adds
  nullable/context fields without renaming existing keys.
- Optional commands drain both pipes, cap output, time out, and terminate their
  process tree/group best-effort.
- Markdown saves are collision-safe and symlink-resistant; `--no-save` disables
  the side effect.
- Updater payloads use private randomized staging and bounded downloads.
- CI's macOS test/build/speed legs and RustSec audit are blocking again.
- Native and Rosetta each pass 115 library + 19 integration tests; native and
  translated release binaries pass full/fast JSON and table smokes, a 51-column
  non-UTF ASCII fallback, privacy assertions, and updater no-op checks.

### Deferred — do not mark complete from this Mac checkpoint

- Live Windows report/install/update verification on the user's Alienware.
- Live Linux AMD64 and Raspberry Pi 4/aarch64 report verification.
- Exact-SHA hosted CI from the final handoff commit (must be watched after push).
- Version bump to `4.0.0`, crates.io publish, tag, cargo-dist release, Windows
  installer assets, and production artifact verification.
- TR-300 page changes in
  `/Users/realemmetts/Downloads/temp_git/qube-machine-report-homepage`; update
  that repository only after v4.0.0 is actually deployed.

## Product and Architecture

The crate exposes both a binary (`src/main.rs`) and a public library
(`src/lib.rs`). `SystemInfo::collect_with_mode()` runs seven scoped collectors
in parallel, merges platform enrichments, then `src/report.rs` renders table,
JSON, or Markdown output. The terminal table remains 51 display columns wide
and uses `unicode-width` for alignment.

`CollectMode::Fast` is the shell-startup path. It keeps quick native/environment
facts and skips slow optional probes. The installed profile block invokes
`tr300 --fast`; the `report` alias and plain `tr300` use full mode. Optional
collector failure is represented as absence, not a fabricated value or a whole
report failure.

JSON schema version 1 is stable. Additive keys are allowed; key removal, rename,
or type change requires a schema bump. Current JSON names value provenance for
CPU load/frequency, disk used/available, and memory used/available so consumers
do not have to infer platform semantics.

## Release Contract

1. Complete live Windows/Linux hardware checks and resolve findings forward.
2. Bump `Cargo.toml` to `4.0.0`; keep the full docs set synchronized.
3. Run locked fmt, clippy, tests, release builds, package list, publish dry-run,
   security audit, and cargo-dist plan.
4. Commit and push `master`; wait for `.github/workflows/ci.yml` to pass on the
   exact commit and for `crates-publish.yml` to publish that same SHA.
5. Create and push only tag `v4.0.0` after CI/crates settle.
6. Verify cargo-dist's GitHub Release and all Windows installer assets/aliases.
7. Only then update, test, commit, and push the homepage repository.

Never publish locally merely because a token exists. Never tag before the
default-branch CI and crates workflow settle.

The v4 release notes must include a concise Rust migration section: downstream
code should obtain `SystemInfo`/`Config` through collection/default APIs rather
than external struct literals, avoid exhaustive public-record patterns, and
account for the changed collector-helper return/signature contracts. Do not
misdescribe the CLI or additive JSON schema as breaking.

## Project Tree

Generated/ephemeral `.git/`, `target/`, and local ignored `.tasks/` contents are
excluded. The tracked project tree is:

```text
.
├── .agents
│   └── skills
│       └── release
│           └── SKILL.md
├── .claude
│   ├── hooks
│   │   └── edit-time-reminder.ps1
│   ├── settings.json
│   ├── settings.local.json
│   └── skills
│       ├── ATTRIBUTION.md
│       ├── architecture
│       │   ├── CONNECTORS.md
│       │   └── SKILL.md
│       ├── brainstorming
│       │   ├── SKILL.md
│       │   ├── spec-document-reviewer-prompt.md
│       │   └── visual-companion.md
│       ├── critical-thinking
│       │   ├── SKILL.md
│       │   └── references
│       ├── release/SKILL.md
│       ├── system-design/SKILL.md
│       ├── tr300-changelog/SKILL.md
│       ├── tr300-dev-workflow/SKILL.md
│       ├── windows-accuracy/SKILL.md
│       ├── windows-distribution-and-update/SKILL.md
│       └── windows-install/SKILL.md
├── .codex/config.toml
├── .firecrawl/polyform-nc-1.0.0.md
├── .github/workflows
│   ├── ci.yml
│   ├── crates-publish.yml
│   ├── release.yml
│   └── windows-installers.yml
├── .gitignore
├── AGENTS.md
├── CHANGELOG.md
├── CLAUDE.md
├── CODEX_PROJECT.md
├── Cargo.lock
├── Cargo.toml
├── HUMAN_CHANGELOG.md
├── LICENSE
├── MASTER_PLAN.md
├── README.md
├── TESTING.md
├── build.rs
├── docs
│   ├── agents/handoff
│   │   └── 2026-07-14-001-macos-hardening-alienware-continuation.md
│   └── architecture-decisions.md
├── inno
│   ├── corporate.iss
│   └── global.iss
├── man/tr300.1
├── rust-toolchain.toml
├── src
│   ├── cli.rs
│   ├── collectors
│   │   ├── command.rs
│   │   ├── cpu.rs
│   │   ├── disk.rs
│   │   ├── memory.rs
│   │   ├── mod.rs
│   │   ├── network.rs
│   │   ├── os.rs
│   │   ├── platform
│   │   │   ├── linux.rs
│   │   │   ├── macos.rs
│   │   │   ├── mod.rs
│   │   │   └── windows.rs
│   │   └── session.rs
│   ├── config.rs
│   ├── error.rs
│   ├── install
│   │   ├── mod.rs
│   │   ├── prompt.rs
│   │   ├── shared.rs
│   │   ├── unix.rs
│   │   └── windows.rs
│   ├── lib.rs
│   ├── main.rs
│   ├── migrate.rs
│   ├── render
│   │   ├── bar.rs
│   │   ├── mod.rs
│   │   └── table.rs
│   ├── report.rs
│   └── update.rs
├── tests/integration.rs
├── wix/main.wxs
└── wix-corporate/corporate.wxs
```

## Local Task Board

The SHAUGHV board is intentionally local and gitignored at `.tasks/`. Its live
root is recorded in `.tasks/.board-server.json`; do not assume a port. Before
this checkpoint is pushed, the board must show macOS implementation/validation
complete and retain separate pending tasks for Alienware Windows validation,
Linux/Raspberry Pi validation, v4.0.0 release, and post-deployment homepage
synchronization. The tracked handoff duplicates all resume-critical state so a
fresh Alienware clone does not depend on the ignored board directory.
