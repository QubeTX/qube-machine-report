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

### Self-Update (`--update`)

`src/update.rs` checks `https://api.github.com/repos/QubeTX/qube-machine-report/releases/latest` (15s timeout via `ureq`), compares against `VERSION` from `Cargo.toml`, and re-runs the install method that placed the binary:
- `~/.cargo/bin/...` → `cargo install tr-300 --force`
- Otherwise → re-pipes the shell or PowerShell installer URL

`--update --json` emits a single JSON object with `current_version`, `latest_version`, `update_available`, and `success`. Exit codes: `0` success, `2` failure.

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
