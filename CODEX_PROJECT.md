# CODEX_PROJECT.md

## TL;DR

TR-300 is a Rust CLI machine-report tool and successor to the legacy TR-200 shell/PowerShell implementation. The repo currently exposes a `tr300` binary and `tr_300` library, with cross-platform collectors, table/JSON/markdown rendering, install/uninstall helpers, and self-update support.

Current Codex migration status: project Claude plugin settings from `.claude/settings.json` have been mirrored into `.codex/config.toml` for the `codex@openai-codex` plugin and `openai-codex` marketplace.

Current implementation status: the working tree is on the cross-platform
stability/action-syntax pass, adding no-double-dash actions, bounded collector
subprocess helpers, conditional platform rows, and parser/fixture coverage.

## Project Status

- Cargo package: `tr-300`
- Binary: `tr300`
- Library import path: `tr_300`
- Current version: `3.14.0`
- MSRV: Rust `1.95`, pinned in both `Cargo.toml` and `rust-toolchain.toml`
- Primary guide: `AGENTS.md`
- Companion docs: `CLAUDE.md`, `MASTER_PLAN.md`, `TESTING.md`, `docs/architecture-decisions.md`
- Release tooling: cargo-dist `0.31.0`

## Goals

- Generate a compact machine report across macOS, Linux, and Windows.
- Keep TR-200-compatible fixed-width table output unless a user explicitly requests a format change.
- Preserve both binary and library APIs when refactoring.
- Keep fast-mode startup checks lightweight for shell auto-run use.
- Maintain release reliability across cargo-dist targets and GitHub Actions.

## Current Workspace Notes

- `.claude/settings.json` contains Claude plugin marketplace state.
- `.codex/config.toml` contains the Codex-facing migration of that plugin state.
- Source Claude settings are left unchanged.
- Markdown-only guide/config edits do not require Rust test runs.
- `tr300 update`, `tr300 install`, and `tr300 uninstall` are parser-level
  aliases for the legacy action flags. `report update` works through the same
  installed alias path.
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
├── RUST_REWRITE_FEASIBILITY.md
├── TESTING.md
├── TR200-OLD
│   ├── .firecrawl
│   │   ├── upstream-machine_report.md
│   │   └── upstream-readme.md
│   ├── .npmignore
│   ├── AGENTS.md
│   ├── CLAUDE.md
│   ├── LICENSE
│   ├── README.md
│   ├── WINDOWS
│   │   ├── README_WINDOWS.md
│   │   ├── TR-200-MachineReport.ps1
│   │   ├── Unix Shell to PowerShell Conversion Guide.md
│   │   ├── build_tr200_exe.ps1
│   │   ├── compass_artifact_wf-0b886add-c632-4a26-a15f-1ab2d9cbae20_text_markdown.md
│   │   ├── install_windows.ps1
│   │   └── unix_linux shell scripts_files -> converted to.md
│   ├── bin
│   │   └── tr200.js
│   ├── dist
│   │   └── .gitignore
│   ├── install.sh
│   ├── install_linux.sh
│   ├── install_mac.command
│   ├── machine_report.sh
│   ├── package.json
│   └── tools
│       └── package_release.sh
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
