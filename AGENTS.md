# Agent Guide (AGENTS.md)

This file is the working guide for AI coding agents in this repository.
Use this file as the canonical source when `AGENTS.md` and `CLAUDE.md` differ.

Companion docs:
- [`CLAUDE.md`](./CLAUDE.md) — edit-time rules, the canonical 7-phase development workflow, CI gates, code patterns.
- [`docs/architecture-decisions.md`](./docs/architecture-decisions.md) — long-form rationale for major decisions (MSRV / `rust-toolchain.toml`, auto-rustup self-update, Intel macOS CI coverage policy, Windows accuracy patterns by version). Open this when you're about to undo or revise an existing decision and need the original reasoning + rejected alternatives.
- [`MASTER_PLAN.md`](./MASTER_PLAN.md) — what's shipped, what's pending, where to pick up next session.
- [`TESTING.md`](./TESTING.md) — manual cross-platform verification matrix + per-release verification log.

Last verified against source: 2026-06-03

## Project Snapshot

- Project: TR-300, a standalone Rust machine-report CLI
- Cargo package name: `tr300`
- Library import path: `tr300`
- Current version: `3.16.0` (`Cargo.toml`)
- MSRV: `1.95` (declared in both `Cargo.toml` `rust-version` AND `rust-toolchain.toml` `channel` — the two-place pin is required; see "Toolchain pinning" below)
- Binary name: `tr300`
- Convenience alias installed by `--install`: `report`
- License: PolyForm-Noncommercial-1.0.0
- Repo: `https://github.com/QubeTX/qube-machine-report`
- Release tooling: cargo-dist `0.31.0`

The crate exposes both:
- a binary in `src/main.rs`
- a library in `src/lib.rs`, including `generate_report()`, `generate_report_with_config()`, `format_bytes()`, `CollectMode`, `SystemInfo`, `Config`, `AppError`, and `Result`

Keep both surfaces working when refactoring.

## Repository Map

```text
.claude/
  settings.local.json         # Claude Code local output style setting

.github/workflows/
  ci.yml                      # cross-platform fmt/clippy/test/build/speed/audit/dist-plan
  crates-publish.yml          # publishes new crates.io versions after successful default-branch CI
  release.yml                 # cargo-dist generated release workflow
  windows-installers.yml      # hand-authored; builds Corporate MSI + both Inno Setup EXEs after release.yml (v3.15.0+)

docs/
  architecture-decisions.md   # long-form rationale for MSRV policy, Windows accuracy patterns, etc.

rust-toolchain.toml           # rustup pin (channel = "1.95", rustfmt + clippy components) — see "Toolchain pinning"

man/
  tr300.1                     # generated man page

src/
  cli.rs                      # clap CLI definition shared by main.rs and build.rs
  main.rs                     # binary entrypoint and action dispatch
  lib.rs                      # public library exports and helpers
  config.rs                   # config flags, widths, box char sets
  error.rs                    # AppError + Result alias
  report.rs                   # table/JSON/markdown report generation
  update.rs                   # self-update flow
  render/
    mod.rs
    table.rs                  # fixed-width table renderer
    bar.rs                    # percent bar renderer
  collectors/
    command.rs                # subprocess timeout helper for optional probes
    mod.rs                    # SystemInfo aggregate + parallel collection pipeline
    os.rs
    cpu.rs
    memory.rs
    disk.rs
    network.rs
    session.rs
    platform/
      mod.rs
      linux.rs
      macos.rs
      windows.rs
  install/
    mod.rs
    prompt.rs                 # interactive uninstall menu/confirm
    unix.rs                   # bash/zsh profile edits
    windows.rs                # PowerShell profile edits

tests/
  integration.rs

wix/
  main.wxs                    # Global MSI packaging template (perMachine, system PATH)

wix-corporate/
  corporate.wxs               # Corporate MSI packaging template (perUser, no admin) — v3.15.0+, in separate dir so cargo-wix's wix/ scan doesn't bundle both into one MSI

inno/
  global.iss                  # Inno Setup script for Global EXE installer (perMachine) — v3.15.0+
  corporate.iss               # Inno Setup script for Corporate EXE installer (perUser) — v3.15.0+

build.rs                      # generates man/tr300.1 via clap_mangen
Cargo.lock                    # tracked for locked local checks and crates.io publishing
Cargo.toml                    # package metadata, dependencies, cargo-dist config
CHANGELOG.md                  # release history (technical)
HUMAN_CHANGELOG.md            # release history (plain-English mirror — see CLAUDE.md companion-changelog rules)
README.md                     # user-facing docs
```

## How The Program Works

### Runtime flow (`src/main.rs`)

1. Parse CLI args from `src/cli.rs` with `clap`.
2. Build `Config` up front so action commands can honor output/color settings.
3. Apply `--ascii` or automatic ASCII fallback when Unix locale is not UTF-8.
4. Apply `--json`, `--no-color`, and custom `--title`.
5. On Windows terminals, set console output code page to UTF-8.
6. Handle action flags or positional actions with early exit:
   - `--update`
   - `--install`
   - `--uninstall`
   - `update`
   - `install`
   - `uninstall`
7. Choose collection mode:
   - `CollectMode::Fast` when `--fast` is set
   - `CollectMode::Full` otherwise
8. Collect system data with `SystemInfo::collect_with_mode(mode)`.
9. Render output with `report::generate(...)`.
10. Print to stdout.
11. In full table mode only, auto-save a markdown report to Downloads and print the path or warning to stderr.

Important ordering rule:
- Do not print Unicode box-drawing characters before `is_utf8_locale()` and Windows UTF-8 setup have run.

### CLI definition (`src/cli.rs`)

`src/cli.rs` is included by both:
- `src/main.rs`
- `build.rs`

Because `build.rs` uses `include!("src/cli.rs")`, `src/cli.rs` must use normal `//` comments rather than inner doc comments like `//!`.

Current supported flags:
- `--ascii` -> ASCII table + `#`/`.` bars
- `--json` -> JSON output
- `--install` -> install alias + shell auto-run block
- `--uninstall` -> interactive uninstall path
- `--update` -> self-update from GitHub releases
- `-t, --title <TITLE>` -> custom title
- `--no-color` -> disables update-flow ANSI styling
- `--fast` -> skip slow collectors for quick auto-run startup

Current supported positional actions:
- `update` -> self-update from GitHub releases
- `install` -> install alias + shell auto-run block
- `uninstall` -> interactive uninstall path

These are optional positional values, not clap subcommands. They intentionally
share the same dispatch paths as the legacy flags, and clap rejects mixed
actions such as `tr300 update --install`.

### Data pipeline (`src/collectors/mod.rs`)

`SystemInfo::collect_with_mode(mode)` runs collectors in parallel with `std::thread::scope`.

The scoped collector threads are:
- `os::collect()`
- `cpu::collect(mode)`
- `memory::collect()`
- `disk::collect()`
- `network::collect_network_info(mode)`
- `session::collect(mode)`
- `platform::collect(mode)`

Thread panic behavior:
- Core collector thread panics are converted to `AppError::SystemInfo`.
- Platform collector panics fall back to `PlatformInfo::default()`.

After collection it:
- Aggregates disk usage by preferring root `/` or `C:` volume, then non-removable disk sum, then first-disk fallback.
- Computes disk and memory percentages.
- Derives hypervisor string from platform virtualization.
- Uses `Bare Metal` as the full-mode fallback when no virtualization signal exists.
- Leaves hypervisor as `None` in fast mode when not cheaply known.
- Builds final `SystemInfo` with the collection mode stored in `mode`.

### Fast mode (`CollectMode::Fast`)

`--fast` is intended for shell startup auto-run and avoids slow subprocess-heavy checks where possible.

Install profile auto-run uses `tr300 --fast`; the `report` alias still runs full mode. Exact skipped work varies by platform collector, so check `src/collectors/platform/{linux,macos,windows}.rs` before changing fast-mode behavior.

### Rendering flow (`src/report.rs`)

- `Config::format == OutputFormat::Table`: render fixed-width terminal table.
- `Config::format == OutputFormat::Json`: render manually formatted JSON.
- Full table mode auto-save calls `save_markdown_report(info)`.

Table rendering is fixed width:
- Label column: 12 chars
- Data column: 32 chars
- Total row width with borders/spaces: 51 chars

`src/render/table.rs` uses `unicode-width` for display width. Use `UnicodeWidthStr::width()` or `UnicodeWidthChar::width()` instead of `.chars().count()` for visible terminal alignment.

## Output Format Details

### Table sections and row behavior

Rendered order:

1. Header block
   - top border
   - title
   - subtitle
   - top divider
2. OS section
   - `OS`
   - `KERNEL`
   - `ARCH`
   - optional `MODEL`
   - optional `BOARD`
   - optional `BIOS`
3. Network section
   - `HOSTNAME`
   - optional `MACHINE IP`
   - `CLIENT  IP` (`Not connected` if none)
   - `DNS  IP 1..5` rows
   - `USER`
4. CPU section
   - `PROCESSOR`
   - `CORES`
   - optional `CORE TYPE`
   - GPU rows:
     - 1 GPU: `GPU`
     - 2-3 GPUs: `GPU 1`, `GPU 2`, `GPU 3`
     - 4+ GPUs: single `GPUs` row
   - optional `HYPERVISOR`
   - `CPU FREQ`
   - `LOAD  1m`, `LOAD  5m`, `LOAD 15m` bars when available
5. Disk section
   - `VOLUME`
   - `DISK USAGE` bar
   - `ZFS HEALTH` only when available
6. Memory section
   - `MEMORY`
   - optional `RAM SLOTS`
   - `USAGE` bar
7. Session section
   - optional `LAST LOGIN`
   - optional extra row with blank label for `last_login_ip`
   - `UPTIME`
   - optional rows: `SHELL`, `TERMINAL`, `LOCALE`, `BATTERY`
8. Footer

Renderer behavior:
- Strings are padded/truncated to fixed display width with `...` ellipsis.
- Header text is centered and truncated when needed.
- Footer is single-line in current output via `render_footer()`.

Bar behavior (`src/render/bar.rs`):
- Percent is clamped to `0..100`.
- Filled cells are rounded from `(percent / 100) * width`.
- Table bars use width 32.

### JSON output shape

Top-level keys:
- `schema_version`
- `elevated`
- `elevation_unlocks_more`
- `system`
- `os`
- `network`
- `cpu`
- `disk`
- `memory`
- `session`

Important implementation details:
- JSON is manually formatted with `format!`, not serde serialization.
- `escape_json(...)` must escape control characters `0x00..0x1F` as `\u00xx`.
- Optional values are emitted as string literals or `null`.
- `--update --json` is a separate JSON shape implemented in `src/update.rs`.
- Additive nullable keys currently include `system.motherboard`,
  `system.bios`, `os.machine_model`, `cpu.core_topology`, and
  `memory.ram_slots`.

## Collector Behavior By Module

### `os.rs`

Uses `sysinfo::System` static getters for:
- OS name/version
- kernel version
- hostname
- uptime seconds

Architecture defaults to `std::env::consts::ARCH` unless platform collection provides a richer value.

### `cpu.rs`

- Full mode refreshes CPU info with a short delay for stable usage values.
- Brand/frequency come from the first CPU entry.
- Logical cores come from `sys.cpus().len()`.
- Physical cores come from `physical_core_count`, with logical cores as fallback.
- Sockets:
  - Linux: parse `lscpu`
  - Windows: PowerShell CIM query
  - macOS: `sysctl -n hw.packages`
  - fallback: `1`
- Load averages:
  - Unix: `/proc/loadavg` first, then `libc::getloadavg`, converted to percent of cores
  - Windows: current CPU usage for 1m/5m/15m style fields

### `memory.rs`

Uses `sysinfo` memory/swap counters. The main report surfaces RAM total/used/percent.

### `disk.rs`

Uses `sysinfo::Disks`:
- mount point
- filesystem
- total/available/used
- removable flag
- name

Skips disks with `total == 0`.

### `network.rs`

Machine IP:
- Windows: PowerShell `Get-NetIPAddress`, fallback to `ipconfig`
- Linux: `hostname -I`, fallback to `ip route get 1`
- macOS: `ipconfig getifaddr` on common interfaces, then route fallback

Client IP:
- from `SSH_CLIENT` or `SSH_CONNECTION`

DNS servers:
- Windows: PowerShell `Get-DnsClientServerAddress`, fallback `ipconfig /all`
- Linux: `/etc/resolv.conf`, fallback `resolvectl status`
- macOS: `scutil --dns`
- de-duplicated, max 5

The module also contains legacy network interface collection using `sysinfo::Networks`.

### `session.rs`

Collects:
- username
- home dir
- shell
- cwd
- terminal
- last login
- optional login IP

Last-login strategy:
- Linux: `lastlog2`, fallback `lastlog`, fallback `last`
- macOS: `last`
- Windows: PowerShell `Get-LocalUser`, fallback `net user`

### `collectors/platform/*`

Adds OS-specific enrichments, some not currently rendered:
- virtualization/hypervisor signals
- GPU names
- architecture
- machine model
- CPU core topology where available
- motherboard, BIOS, and RAM slot details when elevated Linux `dmidecode`
  can provide them
- ZFS health in full mode where `zpool` is available
- shell and terminal details
- locale
- battery
- display resolution
- desktop/display server/edition/codename/boot mode metadata

Platform implementations:
- `linux.rs`: `/proc`, `lscpu`, `/sys`, `ip`, resolver files, ZFS and elevated `dmidecode` commands where available
- `macos.rs`: `sysctl`, `scutil`, `pmset`, `ioreg`, `system_profiler` JSON in full mode
- `windows.rs`: Win32 APIs and registry first, WMI/PowerShell fallbacks in full mode only

Optional subprocess probes should use `collectors::command` timeout helpers.
Missing tools, timeouts, malformed output, and permission failures should
return `None`/fallback values silently rather than blocking or failing the
whole report.

## Install/Uninstall System

Entry points in `src/install/mod.rs`:
- `install()`
- `uninstall()`
- `uninstall_complete()`
- prompt helpers re-exported from `prompt.rs`

### Interactive uninstall (`--uninstall`)

Prompt options:
- profile-only cleanup
- complete uninstall (profile + binary)
- cancel

Complete uninstall requires explicit confirmation and shows the binary path before deletion. Do not bypass the prompt unless implementing a clearly requested non-interactive variant.

### Unix install (`src/install/unix.rs`)

Profiles touched:
- `~/.bashrc` when present
- `~/.zshrc` when present
- creates `.bashrc` if neither shell profile exists

Injected block:

```bash
# TR-300 Machine Report
alias report='tr300'

# Auto-run on interactive shell
case "$-" in *i*)
    tr300 --fast
    ;; esac
# End TR-300
```

Behavior:
- removes existing TR-300 block first
- uses POSIX-compatible `case "$-"` rather than bash-only `[[ ... ]]`
- idempotent re-install behavior

### Windows install (`src/install/windows.rs`)

Profile target:
- PowerShell `$PROFILE` path queried via PowerShell
- creates profile directory if needed

Injected block:

```powershell
# TR-300 Machine Report
Set-Alias -Name report -Value tr300

# Auto-run on interactive shell
if ($Host.Name -eq 'ConsoleHost') {
    tr300 --fast
}
# End TR-300
```

Behavior:
- removes existing TR-300 block first
- complete uninstall deletes the binary and attempts to remove an empty parent directory when path contains `tr300`

## Self-Update System

`--update` is implemented in `src/update.rs`.

Behavior:
- fetches latest release from `https://api.github.com/repos/QubeTX/qube-machine-report/releases/latest`
- uses `ureq` with a 15 second timeout
- sends `User-Agent: tr300/<current version>`
- strips leading `v` from release tags
- compares semver-like numeric components
- builds the strategy list by detecting install origin:
  - **Windows (v3.15.0+):** `detect_install_origin()` reads `HKCU\Software\TR300\InstallSource` written by all four first-class installers (Global MSI, Corporate MSI, Global EXE, Corporate EXE). Returns one matching strategy — no cross-fallback between installer types. MSI strategies run `msiexec /i /passive /norestart`; EXE strategies run `setup.exe /SILENT /SUPPRESSMSGBOXES /NORESTART`. Path-based fallback (`\Program Files\tr300\` → MsiGlobal, `\AppData\Local\Programs\tr300\` → MsiCorporate, `\.cargo\bin\` → CargoOrInstaller, else → Unknown) handles legacy installs without the marker.
  - **Other origins (`CargoOrInstaller` / `Unknown` on Windows, or non-Windows):** legacy probe-and-retry chain — `cargo install tr300 --force` first when `cargo --version` succeeds, macOS/Linux fallback to cargo-dist shell installer through `curl` then `wget`, Windows fallback to cargo-dist PowerShell installer through `powershell` then `pwsh`.
- runs `rustup update stable` best-effort before the cargo strategy when rustup is available
- records skipped/failed strategy attempts and falls through until one strategy succeeds (only on the legacy chain — MSI/EXE strategies don't cross-fall-back)
- **Cross-method consolidation (`tr300 migrate-cleanup`, v3.17.0+, `src/migrate.rs`):** hidden `#[value(hide=true)]` `Action::MigrateCleanup` + hidden flags. The four installers invoke it (interactive checkboxes + silent self-update, both default ON) to keep one install at a time — removes a shadowing `~\.cargo\bin` copy and/or the other edition. Only deletes `tr300.exe` (allowlist); never cargo/rustup/PATH/`~/Downloads`/the running install; needs-admin → skip, exit 0; advisory (never fails an install). Reuses `detect_install_origin`/`InstallOrigin` (`pub(crate)`). WiX: deferred `Impersonate='yes'` `FileKey` CA (no `WixUtilExtension`). Inno Global EXE omits `--user-profile` (no reliable pre-elevation constant) → process-env fallback. Edition paths/marker strings are in the windows-distribution lockstep.

JSON mode:
- `tr300 update --json`, `tr300 --json update`, and `tr300 --update --json`
  print a single JSON object
- fields include `action`, `success`, `message`, `current_version`, `latest_version`, and `update_available` when available
- **Windows (v3.15.0+):** every response also includes a top-level `install_origin` field (`msi-global`, `msi-corporate`, `exe-global`, `exe-corporate`, `cargo-or-installer`, or `unknown`).
- successful updates include legacy `"method"` plus precise `"strategy"` (the v3.15.0+ values are `msi_global`, `msi_corporate`, `exe_global`, `exe_corporate`; the legacy values are `cargo`, `installer_curl`, `installer_wget`, `installer_powershell`, `installer_pwsh`).
- failed updates include an `"attempts"` array with per-strategy diagnostics

Exit codes:
- `0`: success or already current
- `2`: failure

## Distribution And Release Methods

### Packaging model

Release automation uses cargo-dist and GitHub Actions.

`Cargo.toml` (`[workspace.metadata.dist]`):
- `cargo-dist-version = "0.31.0"`
- `ci = "github"`
- `allow-dirty = ["ci"]` so cargo-dist accepts the checked-in release workflow
  alias-copy step for v3.14.2 updater compatibility
- `installers = ["shell", "powershell", "msi"]`
- targets:
  - `aarch64-apple-darwin`
  - `aarch64-unknown-linux-gnu`
  - `x86_64-apple-darwin`
  - `x86_64-unknown-linux-gnu`
  - `x86_64-unknown-linux-musl`
  - `x86_64-pc-windows-msvc`
- `pr-run-mode = "plan"`
- `install-updater = false`
- `install-path = "CARGO_HOME"`
- `publish-prereleases = false`

### CI workflow (`.github/workflows/release.yml`)

High-level job flow:
1. `plan`
2. `build-local-artifacts`
3. `build-global-artifacts`
4. `host`
5. `announce`

Triggers:
- pull requests run cargo-dist planning
- pushes of semver-like tags matching `**[0-9]+.[0-9]+.[0-9]+*` publish releases

### Installer outputs

cargo-dist publishes via `release.yml`:
- Shell installer (`tr300-installer.sh`) for macOS/Linux
- PowerShell installer (`tr300-installer.ps1`) for Windows
- Legacy shell/PowerShell aliases (`tr-300-installer.sh`,
  `tr-300-installer.ps1`) are copied into GitHub Releases for v3.14.2 updater
  compatibility after the crate name was canonicalized to `tr300`
- Global MSI installer for Windows (`tr300-x86_64-pc-windows-msvc.msi`)

`windows-installers.yml` (v3.15.0+) publishes three additional Windows assets after `release.yml` finishes:
- Corporate MSI (`tr300-x86_64-pc-windows-msvc-corporate.msi`) — perUser scope, no admin, installs to `%LocalAppData%\Programs\tr300\bin\` (built from `wix-corporate/corporate.wxs`)
- Global EXE installer (`tr300-x86_64-pc-windows-msvc-setup.exe`) — Inno Setup, perMachine, same install path as Global MSI (built from `inno/global.iss`)
- Corporate EXE installer (`tr300-x86_64-pc-windows-msvc-corporate-setup.exe`) — Inno Setup, perUser, same install path as Corporate MSI (built from `inno/corporate.iss`)

Total Windows installer surface area per release: 4 first-class installers + 2 legacy installer scripts. Total release asset count: 28 (was 22 pre-v3.15.0).

### crates.io publishing (`.github/workflows/crates-publish.yml`)

The crates.io workflow is intentionally separate from the auto-generated
`release.yml`. It is triggered by `workflow_run` after `CI` completes on
`master`/`main`, checks out the exact CI-tested commit SHA, skips when the
manifest version is already present on crates.io using a descriptive
data-access `User-Agent`, and runs `cargo publish --locked` only when
`CARGO_REGISTRY_TOKEN` is configured as a repository Actions secret. It reruns
`cargo fmt --all -- --check`,
`cargo clippy --all-targets --workspace -- -D warnings`,
`cargo test --workspace --all-targets`, `cargo package --locked --list`, and
`cargo publish --dry-run --locked` before the real publish.

`Cargo.lock` is tracked and included in the crate package because both local
release verification and the GitHub publish workflow use `--locked`. Do not
re-ignore or remove it unless the publish workflow is changed in the same
release.

### MSI specifics (`wix/main.wxs` — Global Edition)

- Product name: `tr300`
- Manufacturer: `Emmett S`
- Install scope: `perMachine` (requires admin/UAC)
- Default folder under Program Files (`tr300` with `bin/tr300.exe`)
- Optional PATH feature included in MSI feature tree (system PATH)
- Upgrade/path GUIDs are defined in `Cargo.toml` metadata (`5CD540A8-…` UpgradeCode, `0F93D599-…` Path component)
- v3.15.0+ writes `HKCU\Software\TR300\InstallSource=msi-global` via the `InstallSourceMarker` Component (GUID `537B3C60-…`) so `tr300 update` can dispatch to the matching installer

### Corporate MSI specifics (`wix-corporate/corporate.wxs` — Corporate Edition, v3.15.0+)

- Product name: `tr300 (Corporate Edition)`
- Manufacturer: `Emmett S`
- Install scope: `perUser` (no admin, no UAC) — via `InstallScope='perUser'` + `ALLUSERS=''` + `MSIINSTALLPERUSER=1`
- Default folder: `%LocalAppData%\Programs\tr300\bin\` (via `LocalAppDataFolder > Programs > tr300 > bin`)
- PATH modification: user PATH only (`Environment ... System='no'`)
- UpgradeCode `93F465CB-…` (DIFFERENT from Global MSI — never collide), Path component `13D45AEB-…`, InstallSourceMarker `405304C3-…`
- Writes `HKCU\Software\TR300\InstallSource=msi-corporate`

### Inno Setup EXE installers (`inno/global.iss` + `inno/corporate.iss`, v3.15.0+)

Built by `windows-installers.yml` using `iscc.exe` (installed via `choco install innosetup` on the CI runner). Inputs: the prebuilt `target/release/tr300.exe` and the corresponding `.iss` script. Outputs: `tr300-x86_64-pc-windows-msvc-setup.exe` (Global) and `tr300-x86_64-pc-windows-msvc-corporate-setup.exe` (Corporate).

- Global: `AppId={{AB14223F-…}`, `PrivilegesRequired=admin`, installs to `{commonpf}\tr300` = `%ProgramFiles%\tr300`, writes system PATH via HKLM, marker = `exe-global`.
- Corporate: `AppId={{76A253EB-…}`, `PrivilegesRequired=lowest`, installs to `{userpf}\tr300` = `%LocalAppData%\Programs\tr300`, writes user PATH via HKCU, marker = `exe-corporate`.
- Both use the canonical `[Code]` block with `EnvAddPath()` / `EnvRemovePath()` for duplicate-safe PATH modification with clean uninstall.
- Both support `/SILENT /SUPPRESSMSGBOXES /NORESTART` (used by `tr300 update`).
- All four installer GUIDs (Global+Corp MSI UpgradeCodes, Global+Corp EXE AppIds) plus the two MSI Component GUIDs are permanent — never regenerate. Renaming breaks user upgrade paths.

### Release checklist

1. Update version in `Cargo.toml`.
2. Update the full documentation set for any user-visible release, install,
   update, or deployment behavior change: `CHANGELOG.md`,
   **`HUMAN_CHANGELOG.md`** (plain-English mirror — see
   `CLAUDE.md` § "HUMAN_CHANGELOG.md (companion changelog)" for the
   strip/keep rules; always update both files in the same commit),
   `README.md`, `CODEX_PROJECT.md`, `AGENTS.md`, `CLAUDE.md`,
   `MASTER_PLAN.md`, `TESTING.md`, and `docs/architecture-decisions.md`
   when rationale or release workflow changes. Update the global Codex
   guide at `/Users/realemmetts/.codex/AGENTS.md` when repo deployment
   workflow changes.
3. Run checks:
   - `cargo fmt --all -- --check`
   - `cargo clippy --all-targets --workspace -- -D warnings`
   - `cargo test --workspace --all-targets`
   - `cargo package --locked --list`
   - `cargo publish --dry-run --locked`
4. Commit with message `release: vX.Y.Z - <summary>`.
5. Push the default branch, **wait for `ci.yml` to go green on the exact
   commit**, then confirm `crates-publish.yml` either published the new
   crates.io version from that same SHA or skipped because it was already
   present.
6. Tag with `git tag vX.Y.Z`.
7. Push tag: `git push origin vX.Y.Z` (do NOT use `git push --tags` for the
   workflow trigger; an explicit single-tag push is sufficient).
8. Wait for `.github/workflows/release.yml` to publish the GitHub Release
   assets and installers, including the `tr300-installer.*` assets and
   `tr-300-installer.*` compatibility aliases.

After changing `[workspace.metadata.dist]`, regenerate the workflow with:

```bash
dist init
```

The binary is `dist`, not `cargo dist`.

### Toolchain pinning (load-bearing — read before touching `rust-toolchain.toml`)

The repo pins the toolchain in two places that MUST move in lockstep when MSRV changes:

1. **`Cargo.toml` `rust-version = "1.95"`** — cargo-side declaration. Produces the user-facing `error: package tr300@X.Y.Z cannot be built because it requires rustc N.M ...` message for users on older toolchains.
2. **`rust-toolchain.toml`**:
   ```toml
   [toolchain]
   channel = "1.95"
   components = ["rustfmt", "clippy"]
   ```
   Rustup-side override. **Both fields are required.**

   **Why both fields:**
   - `channel = "1.95"` is the actual fix for the v3.10.0–v3.13.0 release.yml regression (task #54). The auto-generated cargo-dist v0.31.0 `release.yml` has no rustup setup step — it uses each runner image's pre-installed rustc. As of late April 2026, `ubuntu-22.04`, `ubuntu-22.04-arm`, and `windows-2022` runners ship rustc 1.94.1, below MSRV 1.95. The pin makes rustup auto-install 1.95 before cargo runs.
   - `components = ["rustfmt", "clippy"]` is non-obvious but load-bearing. When rustup honors a `rust-toolchain.toml` it installs only the default profile (rustc + cargo + rust-std) and **ignores any action-level `components:` field** passed to `dtolnay/rust-toolchain@stable` in `ci.yml`. Without the components list in the file, ci.yml's `Format` and `Clippy` jobs fail with `error: 'cargo-fmt' is not installed for the toolchain '1.95-x86_64-unknown-linux-gnu'`. v3.13.1 was published as two commits because the first attempt (`c2e6a65`) shipped only the `channel` pin and tripped this; `086ef0a` added the components.

   **Future MSRV bumps:** edit BOTH `Cargo.toml` `rust-version` AND `rust-toolchain.toml` `channel` in the same commit. The minor pin (e.g. `"1.95"`, not `"1.95.0"` and not `"stable"`) lets rustup install the latest patch in the line without churn for patch releases.

Reference: https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file

## Development Commands

```bash
# Build
cargo build
cargo build --release

# Test
cargo test
cargo test --lib
cargo test --test integration
cargo test <test_name>

# Lint/format
cargo clippy
cargo clippy -- -D warnings
cargo fmt
cargo fmt -- --check

# Run
cargo run
cargo run -- --fast
cargo run -- --ascii
cargo run -- --json
cargo run -- --title "MY TITLE"
cargo run -- --no-color
cargo run -- --update
cargo run -- update
cargo run -- --update --json
cargo run -- --json update
cargo run -- --install
cargo run -- install
cargo run -- --uninstall
cargo run -- uninstall
```

## Error Model

`src/error.rs` defines `AppError` variants:
- `SystemInfo { message }`
- `Platform { message }`
- `Io(std::io::Error)`
- `Config { message }`
- `Display { message }`
- Windows only: `Wmi(wmi::WMIError)`

Type alias:
- `Result<T> = std::result::Result<T, AppError>`

## Dependencies

Core:
- `sysinfo = "0.32"`
- `clap = "4.5"` (derive, env)
- `crossterm = "0.28"`
- `thiserror = "2.0"`
- `dirs = "5.0"`
- `chrono = "0.4"`
- `unicode-width = "0.2"`
- `ureq = "2"` (tls, json)
- `serde_json = "1"`

Build:
- `clap = "4.5"` (derive, env)
- `clap_mangen = "0.2"`

Platform-specific:
- Windows: `wmi = "0.14"`, `serde = "1"`, `winapi = "0.3"`
- Unix: `libc = "0.2"`, `users = "0.11"`

Dev:
- `assert_cmd = "2"`
- `predicates = "3"`

## Tests

`tests/integration.rs` currently validates:
- `--help`
- `--version`
- default output contains main title/subtitle
- ASCII mode output
- JSON mode output structure keys
- custom title injection
- `--no-color`
- expected key report fields

For markdown-only guide edits, Rust tests are not required. For source changes, run at least the affected test scope, and prefer `cargo test` before release work.

## Known Caveats / Drift Risks

- `Config` fields `show_network`, `show_disks`, `width`, and `compact` exist but are not fully wired into report rendering/section toggling.
- `use_colors` currently affects update-flow styling, not the normal table renderer.
- `zfs_health` is populated only on full-mode hosts where `zpool` is available.
- JSON report output is manual; schema changes require careful escaping and integration test updates.
- `--install`, `--uninstall`, `--update`, and their positional equivalents have real user-environment side effects. Treat them carefully in tests and automation.
- Some historical docs/examples may mention older behaviors; source code is authoritative.
- PowerShell can be unavailable or restricted on some Windows environments; Windows collectors and installers should retain graceful fallbacks.

## Extension Patterns

### Add a new report field

1. Add field to `SystemInfo` in `src/collectors/mod.rs`.
2. Collect or derive the value in `SystemInfo::collect_with_mode()`.
3. Preserve `CollectMode::Fast` expectations and avoid slow commands in auto-run paths.
4. Add table row in `generate_table()` (`src/report.rs`) if user-visible.
5. Add JSON field in `generate_json()` (`src/report.rs`) if machine-readable output should expose it.
6. Add or adjust integration tests in `tests/integration.rs`.

### Add or change CLI behavior

1. Update `Cli` in `src/cli.rs`.
2. Keep comments in `src/cli.rs` compatible with `include!` from `build.rs`.
3. Update config/action dispatch in `src/main.rs`.
4. Update man page generation expectations if needed.
5. Add integration coverage for flag behavior.
6. Update README, CLAUDE.md, and AGENTS.md when user-visible behavior changes.

### Add platform-specific detail

1. Extend `PlatformInfo` in `src/collectors/platform/mod.rs`.
2. Implement graceful collection for Linux, macOS, and Windows.
3. Make fast-mode behavior explicit.
4. Surface in table/JSON only when the value is useful and reliable.
5. Prefer fallbacks over hard failures for optional enrichments.

### Change table rendering

1. Preserve fixed-width table layout unless the user explicitly requests a format change.
2. Use display-width-aware truncation/padding from `unicode-width`.
3. Keep ASCII mode equivalent to Unicode mode.
4. Test with long strings and non-ASCII text when alignment changes.

## Maintenance Rule

When editing this guide:
- update `AGENTS.md` and `CLAUDE.md` in the same change when both contain the affected fact
- keep statements tied to current source code, not older README wording
- update the "Last verified against source" date after source verification
- keep `.claude/settings.local.json` unchanged unless intentionally changing Claude Code local behavior
