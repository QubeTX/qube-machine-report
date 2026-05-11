# CODEX_PROJECT.md

## TL;DR

TR-300 is a standalone Rust CLI machine-report tool. The repo currently exposes a `tr300` binary and `tr300` library, with cross-platform collectors, table/JSON/markdown rendering, install/uninstall helpers, and self-update support.

Current Codex migration status: project Claude plugin settings from `.claude/settings.json` have been mirrored into `.codex/config.toml` for the `codex@openai-codex` plugin and `openai-codex` marketplace.

Current implementation status: v3.14.3 canonicalizes the crates.io package as
`tr300`, bumps the Rust library import path to `tr300`, and keeps the ND-style
probe-and-retry updater pointed at `cargo install tr300 --force` before
installer fallbacks. The deleted v3.14.2 `tr-300` crate name is treated only as
a legacy compatibility concern for already-installed binaries.

## Project Status

- Cargo package: `tr300`
- Binary: `tr300`
- Library import path: `tr300`
- Current version: `3.14.3`
- MSRV: Rust `1.95`, pinned in both `Cargo.toml` and `rust-toolchain.toml`
- Primary guide: `AGENTS.md`
- Companion docs: `CLAUDE.md`, `MASTER_PLAN.md`, `TESTING.md`, `docs/architecture-decisions.md`
- Release tooling: cargo-dist `0.31.0`

## Goals

- Generate a compact machine report across macOS, Linux, and Windows.
- Keep compact fixed-width table output unless a user explicitly requests a format change.
- Preserve both binary and library APIs when refactoring.
- Keep fast-mode startup checks lightweight for shell auto-run use.
- Maintain release reliability across cargo-dist targets and GitHub Actions.
- Publish new crates.io versions only after the default-branch CI workflow has
  completed successfully for the exact commit being published.

## Current Workspace Notes

- `.claude/settings.json` contains Claude plugin marketplace state.
- `.codex/config.toml` contains the Codex-facing migration of that plugin state.
- Source Claude settings are left unchanged.
- Markdown-only guide/config edits do not require Rust test runs.
- `tr300 update`, `tr300 install`, and `tr300 uninstall` are parser-level
  aliases for the legacy action flags. `report update` works through the same
  installed alias path.
- `tr300 update` uses a cargo-first probe-and-retry chain, then falls back to
  platform cargo-dist installers (`curl`/`wget` on Unix, `powershell`/`pwsh` on
  Windows) with per-attempt diagnostics.
- Release publishing uses two GitHub Actions stages: `Crates.io Publish` runs
  after successful default-branch `CI` on the exact tested SHA and publishes
  `tr300` with `CARGO_REGISTRY_TOKEN` after checking crates.io with a
  descriptive data-access `User-Agent`; `Release` runs after the explicit
  `vX.Y.Z` tag and publishes cargo-dist binary archives/installers. Release
  assets use canonical `tr300-installer.*` names plus legacy
  `tr-300-installer.*` aliases for v3.14.2 updater compatibility. The
  cargo-dist config uses `allow-dirty = ["ci"]` for that checked-in workflow
  customization.
- `Cargo.lock` is tracked so local `cargo publish --dry-run --locked` and the
  GitHub crates.io publish workflow use the same resolved dependency set.
- `src/collectors/command.rs` is the shared timeout wrapper for optional
  collector subprocesses; new platform probes should use it instead of raw
  `Command::output()`.

## Filetree

```text
.
├── .claude
│   └── settings.json
├── .codex
│   └── config.toml
├── .firecrawl
│   └── polyform-nc-1.0.0.md
├── .github
│   └── workflows
│       ├── ci.yml
│       ├── crates-publish.yml
│       └── release.yml
├── .gitignore
├── AGENTS.md
├── CHANGELOG.md
├── CLAUDE.md
├── CODEX_PROJECT.md
├── Cargo.lock
├── Cargo.toml
├── LICENSE
├── MASTER_PLAN.md
├── README.md
├── TESTING.md
├── build.rs
├── docs
│   └── architecture-decisions.md
├── man
│   └── tr300.1
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
│   │   ├── unix.rs
│   │   └── windows.rs
│   ├── lib.rs
│   ├── main.rs
│   ├── render
│   │   ├── bar.rs
│   │   ├── mod.rs
│   │   └── table.rs
│   ├── report.rs
│   └── update.rs
├── tests
│   └── integration.rs
└── wix
    └── main.wxs
```
