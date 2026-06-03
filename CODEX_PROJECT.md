# CODEX_PROJECT.md

## TL;DR

TR-300 is a standalone Rust CLI machine-report tool. The repo currently exposes a `tr300` binary and `tr300` library, with cross-platform collectors, table/JSON/markdown rendering, install/uninstall helpers, and self-update support.

Current Codex migration status: project Claude plugin settings from `.claude/settings.json` have been mirrored into `.codex/config.toml` for the `codex@openai-codex` plugin and `openai-codex` marketplace.

Current implementation status: v3.15.0 ships the four-installer Windows
distribution model — Global MSI (perMachine, existing), Corporate MSI
(perUser, new from `wix-corporate/corporate.wxs`), Global EXE installer (Inno Setup,
perMachine, new from `inno/global.iss`), and Corporate EXE installer (Inno
Setup, perUser, new from `inno/corporate.iss`). All four write
`HKCU\Software\TR300\InstallSource` registry markers so `tr300 update`
dispatches to the matching installer for in-place upgrades on Windows. The
legacy probe-and-retry updater chain (cargo, then `irm | iex` PS installer)
still handles the `cargo install` / shell-installer path on Windows and the
full macOS/Linux flow. The crate is published as canonical lowercase `tr300`
on crates.io with library import path `tr300`; the deleted v3.14.2 `tr-300`
crate name is treated only as a legacy compatibility concern for
already-installed binaries.

## Project Status

- Cargo package: `tr300`
- Binary: `tr300`
- Library import path: `tr300`
- Current version: `3.16.0`
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
- `tr300 update` on Windows (v3.15.0+) reads `HKCU\Software\TR300\InstallSource`
  and dispatches to the matching installer (Global MSI / Corporate MSI /
  Global EXE / Corporate EXE) for in-place upgrade. MSI strategies run
  `msiexec /i /passive /norestart`; EXE strategies run
  `setup.exe /SILENT /SUPPRESSMSGBOXES /NORESTART`. Path-based fallback
  (`\Program Files\tr300\` → MsiGlobal, `\AppData\Local\Programs\tr300\` →
  MsiCorporate, `\.cargo\bin\` → CargoOrInstaller) handles legacy installs
  without the marker. macOS/Linux + Windows-cargo-installed users still use
  the legacy cargo-first probe-and-retry chain.
- Release publishing uses three GitHub Actions stages: `Crates.io Publish`
  runs after successful default-branch `CI` on the exact tested SHA and
  publishes `tr300` with `CARGO_REGISTRY_TOKEN` after checking crates.io with
  a descriptive data-access `User-Agent`; `Release` runs after the explicit
  `vX.Y.Z` tag and publishes cargo-dist binary archives/installers (Global
  MSI + shell/PS installer scripts + legacy aliases for v3.14.2
  compatibility); `Windows Installers` (`windows-installers.yml`,
  hand-authored, triggers on `release: types: [published]`) builds and
  uploads the Corporate MSI + Global EXE installer + Corporate EXE installer.
  Total release asset count: 28 (v3.15.0+) or 22 (pre-v3.15.0). The
  cargo-dist config uses `allow-dirty = ["ci"]` for the checked-in workflow
  customization in `release.yml`.
- v3.14.5 release status: commit `a21a4d1`; CI run 25850693664 succeeded;
  crates-publish run 25850823118 published `tr300` 3.14.5; release.yml run
  25850864213 published the GitHub Release with 22 assets.
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
│       ├── release.yml
│       └── windows-installers.yml
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
├── wix
│   └── main.wxs
├── wix-corporate
│   └── corporate.wxs
└── inno
    ├── global.iss
    └── corporate.iss
```
