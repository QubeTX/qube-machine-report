# Agent Guide (AGENTS.md)

This file is the working guide for AI coding agents in this repository.
Use this file as the canonical source when `AGENTS.md` and `CLAUDE.md` differ.

Companion docs:
- [`CLAUDE.md`](./CLAUDE.md) — edit-time rules, the canonical 7-phase development workflow, CI gates, code patterns.
- [`docs/architecture-decisions.md`](./docs/architecture-decisions.md) — canonical ADR ledger through 2026-07-18: cross-platform/product/output semantics, origin-preserving updates and reusable installer contract, v4 save/Mac trust and evidence boundaries, `main` plus Actions runtime, toolchain/release policy, Windows accuracy/distribution/consolidation, and install safety. Open this before undoing or revising a decision; it records context, rejected alternatives, consequences, evidence, and revalidation triggers.
- [`MASTER_PLAN.md`](./MASTER_PLAN.md) — what's shipped, what's pending, where to pick up next session.
- [`TESTING.md`](./TESTING.md) — manual cross-platform verification matrix + per-release verification log.
- [`docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md`](./docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md) — current v4 release ledger, enforced Mac freeze, and post-release personal-fleet continuation.
- [`docs/agents/handoff/2026-07-14-001-macos-hardening-alienware-continuation.md`](./docs/agents/handoff/2026-07-14-001-macos-hardening-alienware-continuation.md) — historical Mac/shared implementation checkpoint.

Last verified against source: 2026-07-18

## Task management system

This repo uses the SHAUGHV `tasks-*` system locally. The board source of truth is
`.tasks/TASKS.md`; milestones live in `.tasks/MILESTONES.md`, and tasks join one with an
`(ms #id)` tag. Each task's handoff lives at `.tasks/tasks/<id>.md`; keep its `## Status`,
`## Activity`, and `## Verification` current while work is active.

Use indented checkbox subtasks for small required steps visible in the dashboard, with
indented description lines when needed. Keep reasoning, implementation sequence, impact,
acceptance, and resume context in the parent detail file. Large dependent work is a separate
top-level task linked with `(needs #id)`. A task cannot complete over unchecked subtasks or
open verification items, and a milestone cannot complete over open child tasks.

Never put secrets in board or memory files; use environment variables, the OS keychain, or
`.tasks/secure/`, which is gitignored. Resolve the live board from
`.tasks/.board-server.json` and verify its root rather than assuming a port. Relevant skills:
`tasks-start`, `tasks-create`, `tasks-management`, `tasks-update`, `tasks-memory`,
`tasks-boards`, and `tasks-remove`.

## Project Snapshot

- Project: TR-300, a standalone Rust machine-report CLI
- Cargo package name: `tr300`
- Library import path: `tr300`
- Current published version and working manifest: `4.1.3` (`Cargo.toml`).
  v4.1.3 passed exact-SHA CI/crates, signed archives, every Windows package and
  transition job, and universal PKG-in-DMG sign/notary/install/update/uninstall
  gates on native Intel and Apple Silicon. It fixes the immutable v4.1.2
  finding that Restart Manager could terminate a Global installer updater
  before final JSON by using a strict elevated live-image worker. Alienware
  public-binary and installed functionality/hardware validation is real
  evidence; its natural Global MSI completed the v4.0.1 to v4.1.3 same-channel
  UAC transition with one Program Files copy/registration/PATH.
  AMD64 Linux laptop and Raspberry Pi 4 live verification remain open. The
  major v4 boundary
  is required because public Rust records gained fields and collector helpers
  changed signature; the CLI and additive schema-v1 JSON stay compatible.
  Changed public records are `#[non_exhaustive]`.
- Last fully published distribution state: release source
  `c5a25617b8b6438b1e7589e7518a1c1bd305ed64` passed exact-SHA CI/crates,
  both signed Apple archive jobs, all Windows packaging/transitions, and the
  native Intel/ARM universal DMG publication workflow. The public release has
  30 nonempty stable-name assets whose sidecars and versionless `latest`
  entrypoints were audited against the immutable bytes. Homepage commit
  `d77397479ad2b1189cce86b5402eaf1cc966abdf` is live at
  `https://reports.qubetx.com/`; exact evidence is in `TESTING.md` and the
  current handoff.
- MSRV: `1.95` (declared in both `Cargo.toml` `rust-version` AND `rust-toolchain.toml` `channel` — the two-place pin is required; see "Toolchain pinning" below)
- Binary name: `tr300`
- Convenience alias installed by `--install`: `report`
- License: PolyForm-Noncommercial-1.0.0
- Repo: `https://github.com/QubeTX/qube-machine-report`
- Default branch: `main` (renamed atomically from `master` on 2026-07-17;
  `origin/master` no longer exists)
- Actions checkout runtime: all four workflows use `actions/checkout@v6`
  (Node 24). Do not regress branch CI or crates publishing to the deprecated
  Node 20-based v4 action.
- Release tooling: cargo-dist `0.31.0`

The crate exposes both:
- a binary in `src/main.rs`
- a library in `src/lib.rs`, including `generate_report()`, `generate_report_with_config()`, `format_bytes()`, `CollectMode`, `SystemInfo`, `Config`, `AppError`, and `Result`

Keep both surfaces working when refactoring.

For v4 migration notes, distinguish Rust source compatibility from CLI/JSON
compatibility. Direct external struct literals or exhaustive patterns over
the crate's public collector/config/CLI/error records and enums must adapt to
non-exhaustive types; the high-level collection/report APIs and existing JSON
keys remain available. The exact v4 boundary includes `Action`, `Cli`,
`CollectMode`, `CommandTimeout`, all public collector information records,
`Config`, `OutputFormat`, `BoxChars`, `AppError`, `UninstallOption`,
`MigrateOptions`, and `MarkdownSaveOutcome`.

## Repository Map

```text
.claude/
  settings.local.json         # Claude Code local output style setting

.github/workflows/
  ci.yml                      # cross-platform fmt/clippy/test/build/speed/audit/dist-plan
  crates-publish.yml          # publishes new crates.io versions after successful default-branch CI
  release.yml                 # cargo-dist workflow + aliases + fail-closed Apple signing/notary step
  windows-installers.yml      # hand-authored; builds Corporate MSI + both Inno Setup EXEs after release.yml (v3.15.0+)

docs/
  agents/handoff/              # tracked cross-machine continuation records
  architecture-decisions.md   # long-form rationale for MSRV policy, Windows accuracy patterns, etc.
  thinking/                    # tracked release/design reasoning canvases

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
  migrate.rs                  # hidden Windows cross-install cleanup action
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
    shared.rs                 # marker parsing, atomic write, backup helpers
    unix.rs                   # bash/zsh profile edits
    windows.rs                # PowerShell profile edits

tests/
  integration.rs

scripts/
  sign-notarize-macos.sh       # fail-closed Developer ID + Apple notary archive gate

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
4. Apply `--json`, `--no-color`, `--no-elevation-hint`, and custom `--title`.
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
11. Only when the CLI supplied `-r`/`--report`/`-s`/`--save`, save the full
    table as a collision-safe Markdown report in the OS Downloads directory and
    print the path or warning to stderr. Ordinary runs never call the writer.

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
- `-r, --report` with visible aliases `-s, --save` -> manually persist a full
  table Markdown report; conflicts with fast/JSON/action modes
- `--no-save` -> hidden backwards-compatible no-op; reports are not saved by default
- `--no-elevation-hint` -> suppress the optional Linux elevation footer

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
- `os::collect(mode)`
- `cpu::collect(mode)`
- `memory::collect_with_mode(mode)`
- `disk::collect()`
- `network::collect_network_info(mode)`
- `session::collect(mode)`
- `platform::collect(mode)`

Thread panic behavior:
- Core collector thread panics are converted to `AppError::SystemInfo`.
- Platform collector panics fall back to `PlatformInfo::default()`.

After collection it:
- Selects one root/system volume (`/` or normalized Windows system drive), then
  the largest fixed volume, then a removable volume only when no fixed volume
  exists. It never sums potentially overlapping or unrelated mounts.
- Computes disk percent from allocated blocks and memory percent from each
  platform's explicitly named used-memory definition.
- Preserves virtualization as optional positive evidence. Absence stays `None`;
  it is not converted to `Bare Metal`.
- Builds final `SystemInfo` with the collection mode stored in `mode`.

### Fast mode (`CollectMode::Fast`)

`--fast` is intended for shell startup auto-run and avoids slow subprocess-heavy checks where possible.

Install profile auto-run uses `tr300 --fast`; the `report` alias still runs full mode. Exact skipped work varies by platform collector, so check `src/collectors/platform/{linux,macos,windows}.rs` before changing fast-mode behavior.

### Rendering flow (`src/report.rs`)

- `Config::format == OutputFormat::Table`: render fixed-width terminal table.
- `Config::format == OutputFormat::Json`: build a typed `serde_json::Value` and
  serialize it once.
- Full table mode calls `save_markdown_report(info)` only when the CLI supplied
  an explicit save alias. Normal full/fast/JSON runs perform no report-file write.

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
   - optional `EDITION`, `CODENAME`, `BUILD`
   - `KERNEL`
   - `ARCH`
   - optional `MODEL`
   - optional `BOARD`
   - optional `BIOS`
   - optional `BOOT MODE`, `DESKTOP`, `SESSION`, `DISPLAY`
3. Network section
   - `HOSTNAME`
   - optional `DEFAULT IP`
   - `SSH CLIENT` (`Not an SSH session` if none)
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
   - optional `MAX FREQ` or `REPORTED FREQ`
   - optional `CPU USAGE`
   - `LOAD/CPU 1m`, `LOAD/CPU 5m`, `LOAD/CPU 15m` bars when available
5. Disk section
   - `VOLUME`
   - `DISK USAGE` bar
   - `ZFS HEALTH` only when available
6. Memory section
   - `MEMORY`
   - `AVAILABLE`
   - optional `SWAP`
   - optional `RAM SLOTS`
   - `USAGE` bar
7. Session section
   - optional `LAST LOGIN`
   - optional `LOGIN ORIGIN`
   - `UPTIME`
   - optional rows: `LOGIN SHELL`, `TERMINAL`, `LOCALE`, `BATTERY`,
     `BAT HEALTH`, `ENCRYPTION`
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
- `collection_mode`
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
- JSON is assembled through `serde_json::json!`; do not reintroduce manual
  punctuation/escaping. The compatibility `escape_json(...)` helper delegates
  to `serde_json` and is test-only in normal builds.
- Optional values are emitted as string literals or `null`.
- `--update --json` is a separate JSON shape implemented in `src/update.rs`.
- Existing `cpu.cores` remains the logical processor count for compatibility;
  `cpu.logical_processors` and `cpu.physical_cores` make the distinction explicit.
- Normalized `cpu.load_*` values mean percent of logical CPU capacity. Unix-only
  `cpu.load_raw_*` values mean runnable-queue average; Windows leaves both sets
  absent when it cannot establish time-windowed loads.
- `disk.used_definition`, `disk.available_definition`,
  `memory.usage_definition`, and `memory.availability_definition` are part of
  the accuracy contract. Do not change their meaning without an architecture
  decision and consumer review.
- Additive nullable/context keys also include OS build/codename/session uptime,
  boot/display details, default-route/SSH scopes, frequency provenance, root
  mount/filesystem, available/swap bytes, and exact uptime seconds.

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
- Physical cores come from `physical_core_count`; failure remains unknown (`0`)
  rather than falling back to logical threads.
- Sockets:
  - Linux: sysfs physical package IDs, fallback locale-stable `lscpu`
  - Windows: native logical-processor API, fallback bounded WMI
  - macOS: `sysctl -n hw.packages`
  - otherwise: `None`
- Load averages:
  - Unix: `/proc/loadavg` first, then `libc::getloadavg`; retain raw queue
    averages and derive percent of logical capacity separately
  - Windows: `None`; one instantaneous usage sample must not be copied into
    1m/5m/15m windows
- Physical topology remains `0`/unknown when the OS cannot establish it; never
  relabel logical threads or vCPUs as physical cores.
- On Apple Silicon under Rosetta, CPU frequency is intentionally unavailable
  because the translated compatibility value is not a host measurement.

### `memory.rs`

Uses `sysinfo` memory/swap totals. Non-macOS used memory is
`total - operating-system-available`. macOS parses `vm_stat` and defines used as
active + wired + compressed; available is the exact total-minus-used complement.
The table/JSON/Markdown surfaces available memory and swap as well as the
definitions.

### `disk.rs`

Uses `sysinfo::Disks` for enumeration, then queries per-mount capacity with
`statvfs` on Unix or `GetDiskFreeSpaceExW` on Windows:
- mount point
- filesystem
- total/free/available/used (used = total - all-free; available is caller view)
- removable flag
- name

Skips disks with `total == 0`.

### `network.rs`

Machine/default-route IP:
- Windows: WMI/native default-route-aware platform batch, fallback `ipconfig`
- Linux: `ip route get 1.1.1.1` source, fallback `hostname -I`
- macOS: `scutil --nwi` primary address/interface, then `ipconfig`/route fallback

Client IP:
- from `SSH_CLIENT` or `SSH_CONNECTION`

DNS servers:
- Windows: default-route-aware platform WMI/native batch, fallback `ipconfig /all`
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
- Windows: current WTS session data first, then bounded PowerShell fallback;
  address parsing follows the `WTS_CLIENT_ADDRESS` family/layout contract

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
- `macos.rs`: quick `sysctl`/`sw_vers`/environment/`pmset`/`ioreg` probes plus
  one full-mode `system_profiler` JSON snapshot for hardware, displays, power,
  and software. Under Rosetta it requests the native arm64 profiler slice, then
  falls back to translated output. It never surfaces serial/UUID fields.
- `windows.rs`: Win32 APIs and registry first, WMI/PowerShell fallbacks in full mode only

Optional subprocess probes should use `collectors::command` timeout helpers.
Missing tools, timeouts, malformed output, and permission failures should
return `None`/fallback values silently rather than blocking or failing the
whole report. The helper drains stdout/stderr concurrently, caps captured
output under the shared 8 MiB budget, and terminates Unix process groups or a
best-effort Windows Job Object on timeout. Do not replace it with
`Command::output()` plus polling.

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
- creates `.zshrc` on macOS or `.bashrc` on other Unix when neither exists

Injected block:

```bash
# TR-300 Machine Report
alias report='tr300'

# Auto-run on interactive shell; guards prevent spam-on-every-prompt
# when the binary is missing, and recursion in nested shells.
case "$-" in *i*)
    if command -v tr300 >/dev/null 2>&1 && [ -z "${TR300_AUTORUN_RAN-}" ]; then
        export TR300_AUTORUN_RAN=1
        tr300 --fast
    fi
    ;;
esac
# End TR-300
```

Behavior:
- validates marker balance before mutation, takes a one-time backup, removes the
  existing TR-300 block, and atomically replaces the real target (including a
  symlinked profile target)
- uses POSIX-compatible `case "$-"` rather than bash-only `[[ ... ]]`
- idempotent re-install behavior
- refuses `sudo`/root profile installation

### Windows install (`src/install/windows.rs`)

Profile targets:
- queries both Windows PowerShell and PowerShell 7 `$PROFILE` paths and updates
  each distinct installed shell flavor
- creates profile directories if needed

Injected block:

```powershell
# TR-300 Machine Report
Set-Alias -Name report -Value tr300

# Auto-run on interactive shell; also guard missing binaries and recursion.
if (
    (Get-Command tr300 -ErrorAction SilentlyContinue) -and
    -not $env:TR300_AUTORUN_RAN -and
    [Environment]::UserInteractive
) {
    $env:TR300_AUTORUN_RAN = '1'
    tr300 --fast
}
# End TR-300
```

Behavior:
- performs the conservative CurrentUser execution-policy preflight, then uses
  the same marker-balance/backup/atomic-write pipeline as Unix
- complete uninstall deletes the binary and attempts to remove an empty parent directory when path contains `tr300`

## Self-Update System

`--update` is implemented in `src/update.rs`.

Behavior:
- fetches latest release from `https://api.github.com/repos/QubeTX/qube-machine-report/releases/latest`
- uses `ureq` with a 15 second timeout
- sends `User-Agent: tr300/<current version>`
- reuses a nonempty caller-provided `GITHUB_TOKEN` or `GH_TOKEN` for hosted
  release discovery without printing or persisting it; ordinary public updates
  remain unauthenticated
- strips leading `v` from release tags
- compares semver-like numeric components
- resolves the release once, then builds exact-tag asset/sidecar URLs with
  stable versionless filenames; public recovery remains `/releases/latest`
- detects one install channel and never crosses into another:
  - **Windows MSI/EXE:** scoped `InstallSourceGlobal` /
    `InstallSourceCorporate` markers, then the legacy marker; a missing marker
    is recoverable only from one unambiguous matching Add/Remove Programs
    registration and running path. Global/Corporate, MSI/EXE, and scope are
    preserved exactly.
  - **cargo-dist PowerShell/shell:** matching receipt plus recorded
    `install_prefix`; invokes the exact-tag installer through the same shell.
  - **Cargo:** current executable plus Cargo metadata; runs
    `cargo install tr300 --version <resolved> --force --locked`, with
    best-effort `rustup update stable` first when rustup exists.
  - **macOS PKG/DMG:** `/usr/local/bin/tr300` plus the
    `com.qubetx.tr300.pkg` receipt whose ID/version/scope, payload path,
    per-file owner, and Developer ID product identity match the running binary;
    verifies the DMG/sidecar, mounts read-only, validates the nested PKG, and
    waits for Apple Installer.
  - **unknown, conflicting, or portable:** no mutation; exact recovery guidance
    only
- records skipped/failed/blocked attempts but does not cross-fallback between
  channels
- downloads installer payloads/sidecars with explicit size caps into a private
  randomized staging directory that is explicitly removed on success or
  failure; cleanup failure is appended to the diagnostic. Checksum comparison
  detects corruption/mismatch but is not an independent signature because
  payload and sidecar share transport
- classifies likely antivirus/Group Policy/filesystem blocks during create,
  write, sync, or launch as `PolicyBlocked`, stops the fallback chain rather
  than making more write-heavy attempts, retains the current install, reports
  the official release/manual URL, and exits 2. Do not add a force prompt,
  atomic-backup bypass, or direct running-binary replacement fallback
- verifies the installed binary's `--version` before claiming success
- on Windows user-scoped Cargo, cargo-dist PowerShell, Corporate MSI, and
  Corporate EXE channels, atomically renames the running image to a randomized
  private sibling, installs/verifies the replacement at `tr300.exe`, restores
  the old image on strategy failure, and asks only the verified new binary to
  delete the backup after the old process exits. The hidden cleanup action
  accepts only an absolute same-parent numeric private name; another update
  best-effort removes a stale backup left by an interrupted helper.
- on Windows Global MSI/EXE, the non-elevated parent resolves and pins the
  exact channel/version, then uses native `ShellExecuteExW` `runas` with a
  returned process handle for one UAC prompt. The hidden worker accepts only
  `msi_global`/`exe_global`, a plain three-part version, matching origin, and a
  strict unused Program Files sibling; elevated, it performs the same rename,
  install, verify, rollback, and delayed-cleanup transaction. The parent stays
  alive and alone emits final JSON. Never replace this with a PowerShell
  elevation/quoting boundary or let the worker rediscover/switch channels.
- **Fresh-installer intent:** a manually launched Windows installer is the
  user's newest channel choice. WiX removes the same-edition Inno product
  before MSI file installation; Inno enumerates/removes the same-edition MSI
  before Setup proceeds. Ordinary MSI UpgradeCode/Inno AppId upgrades stay
  within format. The hidden advisory `migrate-cleanup` still removes a
  shadowing Cargo executable and/or opposite-edition executable without ever
  deleting the running install, toolchain, Cargo PATH, or Downloads.

JSON mode:
- `tr300 update --json`, `tr300 --json update`, and `tr300 --update --json`
  print exactly one JSON object to stdout; child progress is stderr-only
- fields include `action`, `success`, `message`, `current_version`,
  `latest_version`, `update_available`, `install_channel`, `recovery_url`, and
  `requires_user_action` when applicable
- **Windows (v3.15.0+):** every response also includes a top-level `install_origin` field (`msi-global`, `msi-corporate`, `exe-global`, `exe-corporate`, `cargo-or-installer`, or `unknown`).
- successful updates include legacy `"method"` plus precise `"strategy"`
  (`msi_global`, `msi_corporate`, `exe_global`, `exe_corporate`, `cargo`,
  `installer_powershell`, `installer_pwsh`, `installer_curl`,
  `installer_wget`, or `mac_dmg_pkg`); `install_channel` is the stable
  launcher-independent channel identity
- failed updates include an `"attempts"` array with per-strategy diagnostics;
  `result` is `skipped`, `failed`, or `blocked`; every failed JSON response
  includes `manual_install_url`; a known installer channel also includes its
  immutable `exact_installer_url`, and a blocked response includes
  `official_releases_url`

Exit codes:
- `0`: success or already current
- `2`: failure, cancellation, ambiguity, or installer restart required

## Distribution And Release Methods

### Packaging model

Release automation uses cargo-dist and GitHub Actions.

### Enforced macOS archive signing and native PKG-in-DMG

v4.0.0 makes the Mac trust path explicit and fail-closed. In each publishing
Apple matrix job, `.github/workflows/release.yml` runs
`scripts/sign-notarize-macos.sh` after `dist build` and before cargo-dist
Post-build/upload. The script:

1. creates an ephemeral keychain and private work directory;
2. imports the Developer ID Application PKCS#12, resolves exactly one matching
   identity inside that keychain, temporarily adds that private keychain to the
   user search list, and signs by its certificate fingerprint so a duplicate
   display name in the login keychain cannot make signing ambiguous;
3. signs `tr300` with identifier `com.qubetx.tr300`, hardened runtime, and a
   trusted timestamp; immediately restores the original keychain search list;
   then verifies the embedded leaf-certificate fingerprint, authority, Team ID,
   identifier, runtime, and timestamp;
4. submits the exact binary to Apple Notary Service and requires `Accepted`;
5. repacks those exact signed bytes, regenerates the `.sha256` sidecar, patches
   the per-target cargo-dist manifest checksum, and verifies the sidecar; and
6. restores the original keychain search list from cleanup as a fallback and
   deletes the keychain and decoded credentials on every exit path.

The bare cargo-dist archives retain that Developer ID/notary contract. The
installer-first path is `.github/workflows/macos-installer.yml` plus
`scripts/build-sign-notarize-macos-dmg.sh`: it combines the two exact tagged
archives into one universal Mach-O, signs/notarizes it, builds the signed
system-wide `com.qubetx.tr300.pkg`, embeds that PKG and recovery instructions in
`tr300-universal-apple-darwin.dmg`, then signs, notarizes, staples, mounts, and
Gatekeeper-checks the nested distribution. A PKG is inside the DMG because the
PKG owns `/usr/local/bin/tr300` and supplies the stable updater receipt; the DMG
is the versionless download container, not a GUI app.

Hosted packaging order is load-bearing: check out the exact release tag before
downloading signed archives into the workspace because checkout cleans
untracked inputs. Xcode 16.4 architecture verification must use
`lipo <file> -verify_arch arm64 x86_64` in both the builder and installed-binary
gate. Do not restore input-last syntax. A validator chained from the
supplemental Windows `workflow_run` cannot trust `head_branch` to retain the
original tag; it resolves exactly one immutable release from the immediate
upstream run's exact SHA and fails closed on ambiguity.

Repository Actions configuration uses these names (values are secrets and must
never enter git, logs, tasks, handoffs, or docs):

- secrets: `APPLE_CERTIFICATE_P12_BASE64`,
  `APPLE_CERTIFICATE_PASSWORD`,
  `APPLE_INSTALLER_CERTIFICATE_P12_BASE64`,
  `APPLE_INSTALLER_CERTIFICATE_PASSWORD`, `APPLE_API_KEY_P8_BASE64`,
  `APPLE_API_KEY_ID`, `APPLE_API_ISSUER_ID`;
- variables: `APPLE_SIGNING_IDENTITY`, the full Developer ID Installer common
  name in `APPLE_INSTALLER_SIGNING_IDENTITY`, and `APPLE_TEAM_ID`.

The least-privilege App Store Connect key has Developer role. Installer-key RSA
material/CSR may be generated with OpenSSL on Windows, issued through the Apple
Developer portal, converted to an encrypted PKCS#12 outside the repository, and
uploaded with authenticated GitHub CLI. Apple requires an RSA-2048 CSR for this
certificate. Upload secret bytes through raw stdin (omit `--body`; do not use
literal `--body -`) and prove the resulting PKCS#12 by signing a disposable PKG
on both hosted architectures. Never print, commit, or place secret values in
tasks/docs/browser fields.

Native `macos-15` Apple Silicon and `macos-15-intel` jobs are blocking for every
shared/macOS release change. The DMG workflow additionally installs and
exercises the universal package on both architectures before publication. A
physical Mac is optional visual smoke testing and becomes required only for a
GUI-only defect the hosted runners cannot expose. Windows alone cannot execute
Apple signing tools, but it can author the workflow and conduct the credential
ceremony; do not invent a cross-signing workaround.

`Cargo.toml` (`[workspace.metadata.dist]`):
- `cargo-dist-version = "0.31.0"`
- `ci = "github"`
- `allow-dirty = ["ci", "msi"]` so cargo-dist accepts the checked-in release
  workflow alias-copy/latest-link steps and customized WiX MSI source
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
2. `build-local-artifacts` — Apple publishing jobs sign and notarize after
   `dist build`, before Post-build/upload; any Apple failure blocks hosting
3. `build-global-artifacts`
4. `host`
5. `announce`

Triggers:
- pull requests run cargo-dist planning
- pushes of semver-like tags matching `**[0-9]+.[0-9]+.[0-9]+*` publish releases
- Apple secrets are consumed only when `needs.plan.outputs.publishing == 'true'`;
  pull-request planning never receives them

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

`macos-installer.yml` runs after a successful tagged `release.yml`, builds the
universal signed PKG-in-DMG on native Intel, validates install/update/uninstall
on native Intel and Apple Silicon, and publishes:
- `tr300-universal-apple-darwin.dmg`
- `tr300-universal-apple-darwin.dmg.sha256`

Total Windows installer surface area per release: 4 first-class installers + 2
legacy installer scripts. Complete release asset count: 30 (28 existing assets
plus DMG and sidecar).

### crates.io publishing (`.github/workflows/crates-publish.yml`)

The crates.io workflow is intentionally separate from the auto-generated
`release.yml`. It is triggered by `workflow_run` after `CI` completes on
`main`, checks out the exact CI-tested commit SHA, skips when the
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
- `MajorUpgrade AllowDowngrades='yes'` treats a fresh older or same-version MSI
  launch as the user's newest instruction. The automatic updater still resolves
  only the latest release. Historical pre-v4.1.0 MSI files are immutable and
  may safely block a downgrade because their old policy cannot be rewritten.
- writes legacy `InstallSource=msi-global` plus scoped
  `InstallSourceGlobal=msi-global` via the permanent marker Component so
  `tr300 update` can dispatch only to the matching installer

### Corporate MSI specifics (`wix-corporate/corporate.wxs` — Corporate Edition, v3.15.0+)

- Product name: `tr300 (Corporate Edition)`
- Manufacturer: `Emmett S`
- Install scope: `perUser` (no admin, no UAC) via
  `Package InstallScope='perUser'`; do not add redundant empty `ALLUSERS` or
  `MSIINSTALLPERUSER` properties
- Default folder: `%LocalAppData%\Programs\tr300\bin\` (via `LocalAppDataFolder > Programs > tr300 > bin`)
- PATH modification: user PATH only (`Environment ... System='no'`)
- UpgradeCode `93F465CB-…` (DIFFERENT from Global MSI — never collide), Path component `13D45AEB-…`, InstallSourceMarker `405304C3-…`
- Same fresh-install `AllowDowngrades='yes'` policy as the Global MSI.
- Writes legacy `InstallSource=msi-corporate` plus scoped
  `InstallSourceCorporate=msi-corporate`

### Inno Setup EXE installers (`inno/global.iss` + `inno/corporate.iss`, v3.15.0+)

Built by `windows-installers.yml` using `iscc.exe` (installed via `choco install innosetup` on the CI runner). Inputs: the prebuilt `target/release/tr300.exe` and the corresponding `.iss` script. Outputs: `tr300-x86_64-pc-windows-msvc-setup.exe` (Global) and `tr300-x86_64-pc-windows-msvc-corporate-setup.exe` (Corporate).

- Global: `AppId={{AB14223F-…}`, `PrivilegesRequired=admin`, installs to `{commonpf}\tr300` = `%ProgramFiles%\tr300`, writes system PATH via HKLM, marker = `exe-global`.
- Corporate: `AppId={{76A253EB-…}`, `PrivilegesRequired=lowest`, installs to `{userpf}\tr300` = `%LocalAppData%\Programs\tr300`, writes user PATH via HKCU, marker = `exe-corporate`.
- Both use the canonical `[Code]` block with `EnvAddPath()` / `EnvRemovePath()` for duplicate-safe PATH modification with clean uninstall.
- Both support `/SILENT /SUPPRESSMSGBOXES /NORESTART` (used by `tr300 update`).
- An explicit MSI removes its same-edition Inno registration before writing
  shared-path files; an explicit Inno install enumerates/removes its
  same-edition MSI product first. This implements newest fresh-installer intent
  without changing Global/Corporate scope silently.
- All four installer GUIDs (Global+Corp MSI UpgradeCodes, Global+Corp EXE AppIds) plus the two MSI Component GUIDs are permanent — never regenerate. Renaming breaks user upgrade paths.

### Release checklist

The v4.1.0 release includes real Alienware Windows evidence. AMD64 Linux laptop
and Raspberry Pi 4 checks remain tracked continuation work. Every hosted-native
Mac/local/hosted gate below remains blocking for future releases.

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
   - `actionlint .github/workflows/*.yml` and `shellcheck` on changed shell scripts
   - after shared/Mac/release changes: native Apple Silicon + native Intel tests
     and smokes; after Apple-input changes, real archive and PKG-in-DMG
     sign/notary/staple/install/checksum round-trips
4. Commit with message `release: vX.Y.Z - <summary>`.
5. Push the default branch, **wait for `ci.yml` to go green on the exact
   commit**, then confirm `crates-publish.yml` either published the new
   crates.io version from that same SHA or skipped because it was already
   present.
6. Tag with `git tag vX.Y.Z`.
7. Push tag: `git push origin vX.Y.Z` (do NOT use `git push --tags` for the
   workflow trigger; an explicit single-tag push is sufficient).
8. Wait for `.github/workflows/release.yml` to publish the GitHub Release
   assets and installers. Require both Apple jobs to report Developer ID
   signing and Apple `Accepted` before host; then extract both Mac archives and
   verify their signatures/checksums.
9. Wait for `windows-installers.yml` and verify all four first-class Windows
   installers and both legacy installer aliases.
10. Wait for `macos-installer.yml`; require native Intel and Apple Silicon
    PKG/DMG install gates, then verify the expected 30-asset release.
11. Only after the deployed release is verified, update/test/push the TR-300
    homepage and record exact run IDs/evidence in `TESTING.md` and the handoff.

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
cargo test --locked --workspace --all-targets
cargo test --lib
cargo test --test integration
cargo test <test_name>
cargo test --locked --target x86_64-apple-darwin --workspace --all-targets

# Lint/format
cargo clippy
cargo clippy --locked --all-targets --workspace -- -D warnings
cargo fmt
cargo fmt --all -- --check
cargo audit

# Run
cargo run
cargo run -- --fast
cargo run -- --ascii
cargo run -- --json
cargo run -- --title "MY TITLE"
cargo run -- --no-color
cargo run -- --report
cargo run -- -r
cargo run -- --save
cargo run -- -s
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
- Windows: `wmi = "0.14"`, `serde = "1"`, `winapi = "0.3"`,
  `winreg = "0.52"`, `sha2 = "0.10"`, `tempfile = "3"`
- Unix: `libc = "0.2"`, `users = "0.11"`

Dev:
- `assert_cmd = "2"`
- `predicates = "3"`
- `tempfile = "3"`

## Tests

`tests/integration.rs` currently validates:
- `--help`
- `--version`
- default output contains main title/subtitle
- ASCII mode output
- JSON mode output structure keys
- structured JSON parses, preserves schema v1, includes additive context, and
  keeps non-finite values valid
- custom title injection
- `--no-color`
- default/full/fast/JSON runs do not emit a save message or create a report file
- `-r`/`--report`/`-s`/`--save` invoke isolated manual persistence
- fast mode omits slow conditional rows and elevation footer
- exact 51-column ASCII table width
- expected key report fields

The library suite also covers bounded subprocess success/timeout/large-output/
overflow, save collisions/symlinks, zsh profile round-trip, macOS structured
parsers/privacy/Rosetta/boot/display/battery/FileVault/terminal/locale,
disk/root selection, memory parsing, session parsers, updater staging/checksum/
version/policy-block semantics, release signing/notary manifest behavior, and
cross-platform fallback helpers. Record exact final v4 counts/evidence in
`TESTING.md` rather than leaving stale counts here.

For markdown-only guide edits, Rust tests are not required. For source changes, run at least the affected test scope, and prefer `cargo test` before release work.

## Known Caveats / Drift Risks

- `Config` fields `show_network`, `show_disks`, `width`, and `compact` exist but are not fully wired into report rendering/section toggling.
- `use_colors` currently affects update-flow styling, not the normal table renderer.
- `zfs_health` is populated only on full-mode hosts where `zpool` is available.
- JSON report output is typed through `serde_json`, but schema changes still
  require compatibility review and integration test updates.
- Native `macos-15-intel` and `macos-15` coverage is blocking. A physical Mac
  is optional unless a GUI-only defect cannot be reproduced on hosted runners.
- Alienware evidence exists for the v4.1.0 work. AMD Linux/Pi 4 evidence remains
  open; hosted/cross-compiled evidence is not personal-hardware proof.
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
