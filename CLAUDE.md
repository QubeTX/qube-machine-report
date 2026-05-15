# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **Companion file:** Long-form rationale for major architectural decisions
> (Windows accuracy patterns by version, MSRV / `rust-toolchain.toml` policy,
> auto-rustup self-update reasoning, Intel macOS CI coverage policy) lives in
> [`docs/architecture-decisions.md`](./docs/architecture-decisions.md). This
> file keeps the load-bearing **edit-time rules** — what to do, what not to
> undo, and which constants and APIs are required. Open the decisions doc
> when you need the **why**: rejected alternatives, prior failure modes, and
> historical context.
>
> Forward-looking work tracking (what's shipped, what's pending, who picks up
> next session) is in [`MASTER_PLAN.md`](./MASTER_PLAN.md). Per-version
> verification logs are in [`TESTING.md`](./TESTING.md). Agent-facing
> repository tour and release checklist are in [`AGENTS.md`](./AGENTS.md).

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

`tr300 install` / `tr300 uninstall` modify shell profiles to add/remove the `report` alias and auto-run. The legacy `--install` / `--uninstall` flags are backward compatible aliases. Installation blocks are wrapped in marker comments for idempotent cleanup.

- **Unix/macOS** — `src/install/unix.rs` modifies `~/.bashrc` and/or `~/.zshrc`
- **Windows** — `src/install/windows.rs` modifies PowerShell `$PROFILE`

`--uninstall` is interactive (`src/install/prompt.rs`): the user picks `ProfileOnly`, `Complete` (also deletes the binary), or `Cancel`. The `Complete` path uses `find_binary_location()` + `confirm_complete_uninstall()` to show the path before deleting. Don't bypass the prompt unless the user has explicitly opted into a non-interactive variant.

**Windows execution-policy preflight (v3.14.4+).** `install()` runs `run_execution_policy_preflight()` **before** writing `$PROFILE`. Edit-time rules:
- The preflight is the *minimum-permissions* fix: `Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned -Force` *only* when the current `CurrentUser` policy is `Restricted` or `Undefined`. `RemoteSigned` is the strictest of PowerShell's policies that loads a local unsigned profile — it does **not** weaken protection against downloaded unsigned scripts. Never widen beyond `RemoteSigned`. Never touch `LocalMachine` scope (requires admin and affects all users). Never use persistent `Bypass`.
- **Never silently downgrade `AllSigned`.** That's a deliberate security choice — `policy_state("AllSigned")` returns `PolicyState::BlockedAllSigned`; the preflight prints a notice explaining the auto-run won't fire and leaves the policy alone.
- **Verify after set.** `Set-ExecutionPolicy` can exit 0 while a higher-precedence `MachinePolicy` / `UserPolicy` GPO still wins; `try_set_execution_policy()` re-reads `Get-ExecutionPolicy -Scope CurrentUser` and returns `TrySetResult::StillBlocked` so we can surface a fallback warning with the `LocalMachine`-scope remediation.
- **Failures are non-fatal.** `run_execution_policy_preflight()` returns `()` and never propagates an error to `install()`'s `Result` — the alias write half still succeeds even when the policy can't be fixed, so manual `tr300` invocations from the prompt keep working.
- **Order matters.** Run the preflight first, then write the profile. The reverse surfaces the confusing `UnauthorizedAccess` PSSecurityException at the *next* shell start, far from the install-time context where the user can act on it.
- The policy classification uses an enum (`PolicyState::{BlockedDefault, BlockedAllSigned, Permissive, Unknown}`) so unknown future PowerShell policy strings default to `Permissive` rather than triggering destructive action on values we can't reason about.

**Windows install error advisor (v3.14.5+).** Every fallible `std::fs` call in the install/uninstall flow funnels through `fail_install(InstallStep, &Path, io::Error)` instead of the old `map_err(|e| AppError::platform(format!("Failed to ...: {}", e)))` pattern. Edit-time rules:
- **Print the rich guidance to stderr, then return a concise `AppError`.** `fail_install()` streams a multi-paragraph advisory to stderr *before* returning, so it's never swallowed by anything that only captures the returned error. The returned `AppError::platform` is a short tag (`"write profile: ...err..."`) suitable for `main()`'s trailing `Error: ...` line — keep it short so the rich content above stays the focal point.
- **Dispatch on `(InstallStep, io::ErrorKind, raw_os_error, path_inspection)`.** The combination matters: `PermissionDenied` on a OneDrive-redirected path gets OneDrive-specific text (sync state, "always keep on this device"); the same error on a non-OneDrive path gets AD/Intune/AppLocker/WDAC + antivirus + `takeown` guidance. Don't collapse these into one generic block — each cohort needs different remediation.
- **`looks_like_onedrive_path()` checks for an "onedrive" path segment (case-insensitive)** so it catches both `\OneDrive\` and `\OneDrive - <TenantName>\` (OneDrive-for-Business). It will also match `\onedrive-migration.ps1\` etc.; the false-positive harms only the advisory text and is intentionally accepted to keep the predicate simple.
- **Always close with "Manual `tr300` still works from the prompt".** Install failures don't break the binary's basic functionality; the user needs to know what they CAN still do while they sort out the underlying restriction.
- **Don't move the rich output to stdout.** Stdout is for the normal install messages; rich error guidance belongs on stderr so it interleaves correctly with the trailing `Error: ...` line and so callers that capture stdout (e.g. CI scripts) still see the explanation.


### Windows accuracy patterns (v3.11.0+)

Edit-time rules:
- **OS** — `get_os_info_from_registry` reads `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion` directly and overrides sysinfo. Detect Win11 by `CurrentBuild >= 22000` (registry `ProductName` is frozen at "Windows 10"). Append `DisplayVersion` + `UBR` to the kernel string.
- **Arch** — `get_architecture` calls `IsWow64Process2` via manual `extern "system"` against `kernel32`. Returns native machine even under emulation. Handles AMD64 / ARM64 / I386 / ARM. Annotate emulation as `aarch64 (x86_64 emulation)` when process arch ≠ host arch.
- **CPU freq** — combine CPUID leaf 16h + `CallNtPowerInformation(ProcessorInformation)` + sysinfo via `Iterator::max`. Leaf 16h returns 0 on Intel hybrid (Meteor/Lunar/Arrow Lake) — Intel microcode change, not a bug; the fallthrough is intentional.
- **Hypervisor** — `cpuid_hypervisor_brand()` (CPUID leaf 0x40000000, 12-byte vendor string) first. Disambiguate the Win11 VBS edge case: CPUID returns `Microsoft Hv` AND SMBIOS manufacturer is a normal OEM → `Bare Metal (Hyper-V/VBS)`, not `Hyper-V`. Real Hyper-V VMs always have Microsoft Corp as manufacturer.
- **Last-login** — `WTSQuerySessionInformation` via manual extern against `wtsapi32` with inline `WTS_CURRENT_SESSION = 0xFFFFFFFF`, `WTSLogonTime = 17`, `WTSConnectTime = 14` constants. Falls back to `GetTickCount64`-derived boot time because WTS time fields are 0 for local console sessions on modern installs (auto-login + Fast Startup mask the actual logon timestamp). Don't reintroduce `net user` parsing — localized strings, returned "Never".
- **BitLocker** — `Win32_EncryptableVolume` in `ROOT\CIMV2\Security\MicrosoftVolumeEncryption` namespace via `wmi::WMIConnection::with_namespace_path`. Try-and-degrade: readable non-admin on Win11 Device Encryption → `ENCRYPTION` row renders; access-denied on older Win10 / domain configs → `None` and row is omitted; elevation footer covers the gap.

— Full reasoning, every constant's purpose, the prior-`net user` failure mode, the VBS disambiguation logic in detail: see [`docs/architecture-decisions.md` § "v3.11.0+ — registry OS, IsWow64Process2, WTS last-login, CPUID hypervisor, BitLocker"](./docs/architecture-decisions.md#v3110--registry-os-iswow64process2-wts-last-login-cpuid-hypervisor-bitlocker).

### Windows accuracy patterns (v3.13.0+)

Edit-time rules:
- **Battery** — `get_battery_native` calls `GetSystemPowerStatus` (~1 ms vs ~40 ms WMI). 8-state output machine: `BatteryFlag == 0x80` → row omitted (desktop); `0xFF` → `X% (Unknown)`; AC+charging → `X% (Charging)`; AC+≥95%+not charging → `AC Power` (no percent — uninformative at full); AC+<95%+not charging → `X% (Plugged in)` (gaming-laptop PSU-undersized OR firmware battery-longevity, indistinguishable from a single snapshot); off-AC + Critical/Low → labels with that priority; off-AC default → `X% (Discharging)`; `ACLineStatus == 0xFF` → bare `X%`. **Never surface `BATTERY_FLAG_HIGH` as a label suffix** — early v3.13.0 produced `100% (Discharging (High))` which was incoherent.
- **Cores** — `get_socket_count_native` uses `GetLogicalProcessorInformationEx` two-call buffer-sizing pattern (null buffer → `ERROR_INSUFFICIENT_BUFFER` writes required size; allocate, call again). **Walk the variable-length `SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX` records via `u32::from_le_bytes` on the `Size` field** — records aren't fixed-size, raw casts are technically UB. Count `Relationship == RelationProcessorPackage`. WMI fallback (`get_socket_count_wmi`) kept as safety net.
- **GPU** — full mode prefers the registry path (`get_gpus_fast` walks the `{4d36e968-…}` Display class) before WMI/PowerShell; software adapters (Microsoft Basic Render Driver, Hyper-V Video) don't appear there. `filter_software_gpus()` strips known-bad names as belt-and-suspenders. **Don't replace this with full DXGI `IDXGIFactory1::EnumAdapters1`** unless name-based filtering proves insufficient — it's 25 LOC vs ~100 LOC of unsafe COM for the same user-visible outcome.
- **PowerShell 7+ ("PSCore")** — `get_powershell_core_version` does recursive `reg query /s` on `HKLM\SOFTWARE\Microsoft\PowerShellCore\InstalledVersions\<GUID>\SemanticVersion`. **Pick the highest version via `(u64, u64, u64)` semver tuple comparison, NOT string sort** — string-compare puts `"7.10.0" < "7.9.0"` (caught in Codex review). Fall through to legacy WinPS-5.x detection on miss.
- **Terminal** — env-var pre-checks (`WT_SESSION`, `TERM_PROGRAM`, `CURSOR_TRACE_ID`/`CURSOR_AGENT`) THEN `detect_terminal_via_parent_walk` via `CreateToolhelp32Snapshot` + `Process32FirstW`/`Process32NextW`. Build `HashMap<pid, (parent_pid, name)>`, climb from `GetCurrentProcessId()` upward, **cap at 10 levels** (defensive against PID-table cycles when parent dies and PID gets recycled). Skip intermediate hosts (`conhost.exe`, `powershell.exe`, `pwsh.exe`, `cmd.exe`, shells, `tr300.exe`, `node.exe`, `python.exe`). Recognized: Windows Terminal, WezTerm, Alacritty, VS Code, Cursor, Windsurf, Hyper, Tabby, Ghostty, Kitty, MinTTY, Claude Code, Antigravity. Unrecognized → `Console`.

— Full reasoning, all 8 battery-state branches with rationale, the gaming-laptop vs firmware-limit ambiguity, the alignment-safe walk justification, the DXGI vs registry trade-off, the PSCore semver-sort bug catch, the Toolhelp32 PID-cycle defense: see [`docs/architecture-decisions.md` § "v3.13.0+ — 5-state battery, native cores, GPU registry-prefer, PSCore detection, terminal walk"](./docs/architecture-decisions.md#v3130--5-state-battery-native-cores-gpu-registry-prefer-pscore-detection-terminal-walk).

### Windows accuracy patterns (v3.12.0+)

Edit-time rules:
- **VPN-aware default-route** — `get_best_route_interface_index` calls `GetBestInterfaceEx` (manual extern, `iphlpapi`) for `1.1.1.1`; reorders the WMI `Win32_NetworkAdapterConfiguration` result list so the kernel-preferred adapter wins. Falls back transparently to first-match when the kernel lookup fails (IP Helper service disabled, no default route). **Don't reintroduce naive first-match** — coin-flip behavior on multi-homed hosts and on Tailscale / WireGuard / OpenVPN / Cisco AnyConnect tunnels. **`SOCKADDR_IN` is declared inline** (3 fields, layout-stable since Win95) to keep the unsafe surface minimal — don't pull in additional winapi features for it.
- **Fast Startup uptime** — `detect_fast_startup` reads `HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Power\HiberbootEnabled` (DWORD); `last_cold_boot_seconds` queries `Win32_OperatingSystem.LastBootUpTime`. When enabled AND cold-boot is >1h older than `GetTickCount64` (sysinfo uptime, which is hibernation-resumed session age), swap `uptime_seconds` to cold-boot AND populate `session_uptime_seconds` with the resumed value. Renderer outputs `9d 4h 12m (session: 7h 14m)`. **Full-mode only** (~80 ms WMI cost); `--fast` keeps sysinfo's uptime. **Use `wmi::WMIDateTime` for the CIM datetime field** — manual hand-parsing of `yyyymmddHHMMSS.mmmmmmsUUU` was tried and discarded in early v3.12.0.

— Full reasoning, the inline `SOCKADDR_IN` justification, the `wmi::WMIDateTime` discovery: see [`docs/architecture-decisions.md` § "v3.12.0+ — VPN-aware default-route, Fast Startup uptime annotation"](./docs/architecture-decisions.md#v3120--vpn-aware-default-route-fast-startup-uptime-annotation).

### Self-Update (`tr300 update` / `--update`)

`src/update.rs` checks `https://api.github.com/repos/QubeTX/qube-machine-report/releases/latest` (15s timeout via `ureq`), compares against `VERSION` from `Cargo.toml`, then dispatches by detected install origin.

**Windows (v3.15.0+).** `detect_install_origin()` reads `HKCU\Software\TR300\InstallSource` (written by all four first-class installers — MSI Global, MSI Corporate, EXE Global, EXE Corporate) and returns a single matching strategy. No cross-fallback: re-running a different product would create coexistence problems (two ARP entries, PATH ordering wins). The MSI strategies download the matching `.msi` to `%TEMP%` and run `msiexec /i /passive /norestart`; the EXE strategies download the matching Inno Setup `.exe` and run it with `/SILENT /SUPPRESSMSGBOXES /NORESTART`. If the marker is missing (legacy pre-v3.15.0 install), `classify_install_path()` falls back to substring-matching the running EXE path: `\Program Files\tr300\` → `MsiGlobal`, `\AppData\Local\Programs\tr300\` → `MsiCorporate`, `\.cargo\bin\` → `CargoOrInstaller` (legacy chain), anything else → `Unknown` (legacy chain). Each install path is mirrored exactly between MSI and EXE for that edition; the registry marker is the only way to distinguish them on update.

**macOS/Linux.** Legacy probe-and-retry chain (unchanged):
- `cargo install tr300 --force` first when `cargo --version` succeeds
- Fallback: cargo-dist shell installer via `curl`, then `wget`

The legacy Windows chain (`powershell` → `pwsh` running `irm $URL | iex`) still runs for `CargoOrInstaller` / `Unknown` install origins — users who installed via the PowerShell one-liner or `cargo install` keep updating the way they always have.

Do not restore executable-path detection as the **primary** discriminator. The registry marker is authoritative; path-based detection is the legacy fallback only.

`tr300 update --json`, `tr300 --json update`, and `tr300 --update --json` emit a single JSON object with `current_version`, `latest_version`, `update_available`, `success`, and (on Windows) a top-level `install_origin` field. Values: `msi-global`, `msi-corporate`, `exe-global`, `exe-corporate`, `cargo-or-installer`, `unknown`. Successful updates include legacy `"method"` (`cargo` / `installer`) plus precise `"strategy"` (`msi_global`, `msi_corporate`, `exe_global`, `exe_corporate`, or the legacy IDs); failures include an `"attempts"` array. Exit codes: `0` success, `2` failure.

**Auto-rustup on the cargo strategy (v3.11.1+).** Before `cargo install tr300 --force`, `try_strategy(UpdateStrategy::Cargo)` calls `rustup_update_stable_best_effort()`: probe `rustup --version` with stdout/stderr → `Stdio::null()`; if found, run `rustup update stable` and print `Updating Rust toolchain (rustup update stable)…`. **Any failure is non-fatal** — `let _ =` the result, fall through to the cargo install. Installer strategies never touch Rust because they download prebuilt binaries. Don't replace this best-effort pattern with hard-failing or with `rustc --version` probing + conditional rustup — the rationale (failure-mode prevented, distro-managed toolchain compatibility, simplicity) is in the decisions doc.

— Full reasoning, the rustup-managed-vs-distro-managed split, the rejected `rustc --version` probe alternative, the failure-mode this prevents: see [`docs/architecture-decisions.md` § "Self-update auto-rustup (v3.11.1+)"](./docs/architecture-decisions.md#self-update-auto-rustup-v3111).

### Windows distribution model (v3.15.0+)

> **v3.15.0 → v3.15.1 fix narrative (read before editing the WiX / Inno files).**
> v3.15.0 introduced the four-installer model but its `release.yml` run failed
> at `build-local-artifacts(x86_64-pc-windows-msvc)` with WiX candle exit
> code 6 — the GitHub Release was never published. v3.15.1 is the patch that
> fixed it. **Two root causes**, both real and both required for a working
> Corporate MSI build:
>
> 1. `wix/corporate.wxs` declared `<Property Id='ALLUSERS' Value=''/>` and
>    `<Property Id='MSIINSTALLPERUSER' Value='1'/>`. WiX 3.11 candle rejects
>    empty-string Property `Value=` attributes with `CNDL0006`. **Fix:**
>    delete both Property elements. `InstallScope='perUser'` on the Package
>    element is sufficient on WiX 3.11+ — the MSI installer treats unset
>    `ALLUSERS` as per-user, which is what we want.
> 2. cargo-wix compiles ALL `.wxs` files in `wix/` and links them into ONE
>    MSI. Two complete Product definitions in the same directory hit
>    `LGHT0089` ("Multiple entry sections") + `LGHT0091/0092` ("Duplicate
>    symbol") at link time. **Fix:** move the Corporate template to a NEW
>    directory `wix-corporate/corporate.wxs` so cargo-wix's `wix/` scan only
>    sees `main.wxs`. Build the Corporate MSI separately via bare
>    `candle.exe` + `light.exe` from `windows-installers.yml`.
>
> An interim diagnosis attempt during the v3.15.0 → v3.15.1 work-out
> speculated that the `InstallSourceMarker` Components writing HKCU from a
> perMachine MSI tripped ICE57 and triggered the failure. **That was wrong**
> — ICE runs in `light`, not `candle`, and the actual error was `CNDL0006`
> in corporate.wxs. The marker Components are unchanged between v3.15.0
> and v3.15.1; both Global and Corporate MSIs write
> `HKCU\Software\TR300\InstallSource` on install. Future agents diagnosing
> WiX failures should reproduce locally with portable WiX 3.11 binaries
> (https://github.com/wixtoolset/wix3/releases) BEFORE hypothesizing about
> ICE/CNDL/LGHT codes — cargo-dist captures candle/light stderr by
> default, so the real error class is invisible in CI logs without
> `--nocapture`.

Four first-class Windows installers ship at every tagged release. Two MSIs (built by cargo-dist's `release.yml` and the hand-authored `.github/workflows/windows-installers.yml`) and two Inno Setup EXE installers (built by the same hand-authored workflow). The shape of the four-product matrix:

| Product | Source template | Install scope | Install path | PATH scope | UAC? | ARP entry | InstallSource marker |
|---|---|---|---|---|---|---|---|
| Global MSI | `wix/main.wxs` | perMachine | `C:\Program Files\tr300\bin\` | System | Yes | `tr300` | `msi-global` |
| Corporate MSI | `wix-corporate/corporate.wxs` | perUser | `%LocalAppData%\Programs\tr300\bin\` | User | No | `tr300 (Corporate Edition)` | `msi-corporate` |
| Global EXE | `inno/global.iss` | perMachine | `C:\Program Files\tr300\bin\` (same as MSI Global) | System | Yes | `tr300` | `exe-global` |
| Corporate EXE | `inno/corporate.iss` | perUser | `%LocalAppData%\Programs\tr300\bin\` (same as MSI Corporate) | User | No | `tr300 (Corporate Edition)` | `exe-corporate` |

Edit-time rules:

- **The Corporate MSI source lives at `wix-corporate/corporate.wxs`, NOT `wix/corporate.wxs`.** This is intentional: cargo-wix's default behavior compiles every `.wxs` in `wix/` and links them into ONE MSI. Putting two complete Product definitions in the same directory hits link-stage errors LGHT0089 ("Multiple entry sections") and LGHT0091/0092 ("Duplicate symbol"). cargo-dist's `release.yml` MSI build (which uses cargo-wix internally) only sees `wix/main.wxs` because of this directory separation. v3.15.0 → v3.15.1 was a fix-forward driven by exactly this issue.
- **The four GUIDs identifying each product are PERMANENT.** They live in `wix/main.wxs` (Product `UpgradeCode='5CD540A8-…'`), `wix-corporate/corporate.wxs` (`UpgradeCode='93F465CB-…'`), `inno/global.iss` (`AppId={{AB14223F-…}`), and `inno/corporate.iss` (`AppId={{76A253EB-…}`). **Never regenerate these.** Changing any of them breaks in-place upgrades for users of that product (Windows Installer / Inno Setup treat the new GUID as a different product, so the old version isn't removed). Two more GUIDs live in the registry-marker Components — also permanent.
- **Same install paths between MSI and EXE within each edition.** Global → `C:\Program Files\tr300\bin\`. Corporate → `%LocalAppData%\Programs\tr300\bin\`. Don't suffix the EXE path with `-setup` or `(Setup)` — coexistence is documented in README as a "pick one" rule, not engineered around. The registry marker (`HKCU\Software\TR300\InstallSource`) records whichever was installed most recently; `tr300 update` re-runs that one.
- **Registry marker values are literal strings**: `msi-global`, `msi-corporate`, `exe-global`, `exe-corporate`. These appear in three places per product (the installer template that writes them, `src/update.rs::read_install_source_marker()` that matches them, and the JSON output's `install_origin` field). All three must stay in lockstep. Tests in `src/update.rs` pin the contract.
- **`src/update.rs::detect_install_origin()` is the only place that decides which installer to fetch.** If install paths change in `wix/main.wxs`, `wix-corporate/corporate.wxs`, or `inno/*.iss`, update `classify_install_path()` in lockstep. The path matcher uses lowercased substring containment (handles drive-letter case and "Program Files" capitalization variations) — it's intentionally not regex.
- **The Corporate MSI is built by bare `candle.exe` + `light.exe`, NOT through cargo-wix.** `.github/workflows/windows-installers.yml` invokes WiX directly: `candle.exe -arch x64 -dVersion=... -dCargoTargetBinDir=target/release ... wix-corporate/corporate.wxs` then `light.exe -sice:ICE38 -sice:ICE64 -sice:ICE91 -ext WixUIExtension ...`. cargo-wix can't be told to compile a single `.wxs` from outside `wix/` while ignoring the directory; bare WiX gives full control. The ICE suppressions are required because per-user MSI conventions in WiX 3 want HKCU RegistryValue KeyPaths on Components and RemoveFolder entries on Directories — both real correctness gaps for a "real" per-user MSI, but cosmetic for our single-binary install. The only real consequence: an empty `%LocalAppData%\Programs\tr300\bin\` folder left after uninstall.
- **`.github/workflows/windows-installers.yml` is hand-authored.** It uses `workflow_run` triggered by the `Release` workflow completion — NOT `release: types: [published]`. The `release: published` event does not fire downstream workflows when the upstream release was created via `gh release create` with the default `GITHUB_TOKEN` (GitHub's loop-prevention rule, [documented here](https://docs.github.com/en/actions/security-for-github-actions/security-guides/automatic-token-authentication#using-the-github_token-in-a-workflow)). cargo-dist's `release.yml` uses GITHUB_TOKEN, so the `release: published` approach silently didn't fire — v3.15.1 was the canary. `workflow_run` is the same pattern `crates-publish.yml` uses to chain off `CI`. The workflow also accepts `workflow_dispatch` with a `tag` input so it can be manually re-fired for any release: `gh workflow run "Windows Installers" -f tag=v3.15.1`. The workflow installs Inno Setup via Chocolatey (`choco install innosetup -y`) and uses the WiX 3 preinstalled at `$env:WIX\bin\` on the `windows-2022` runner. It uploads 6 assets (the Corporate MSI, both EXE installers, and their `.sha256` sidecars), bringing the total release count to 28 assets.
- **Don't touch `.github/workflows/release.yml`.** That file is auto-generated by cargo-dist v0.31.0. Only the small legacy installer-alias step in the allow-dirty zone is permitted. New install-related work goes in `windows-installers.yml`.
- **HKCU is intentional for the registry marker, even for perMachine installs.** Two reasons: (1) `tr300 update` always runs as the user, who reads HKCU naturally; (2) writing to HKLM from a perMachine MSI requires special handling and risks privilege issues. The rare "admin pushed via Intune, end-user is different" case is covered by the path-based fallback in `classify_install_path()` (the Program Files path implies MSI Global even without a marker).
- **Inno Setup PATH management uses the canonical `[Code]` block pattern**: `EnvAddPath()` / `EnvRemovePath()` with `;` padding for substring matching. Don't replace with Inno Setup's higher-level `Tasks` or `Run` sections — the manual approach handles the duplicate-detection and clean-uninstall cases that the high-level shortcuts get wrong.
- **`Cargo.toml` `allow-dirty = ["ci", "msi"]`** is required: `"ci"` for the legacy installer alias step in the auto-generated `release.yml`, `"msi"` for the customized `wix/main.wxs` Component additions. Both flags must stay.
- **`Cargo.toml` `include` list contains both `/wix/**` and `/wix-corporate/**`** so the published crate ships with both templates. Anyone running `cargo install tr300` and then `cargo wix` locally gets the Global MSI by default (only `wix/main.wxs` is scanned); they can build the Corporate MSI by running bare `candle.exe` + `light.exe` against `wix-corporate/corporate.wxs`.

— Full reasoning, why two MSIs instead of a dual-purpose MSI, why two EXE installers instead of a WiX Burn bundle, the rejected alternatives (WiX Burn, NSIS, single-product with `--mode user/admin` switch), the coexistence-vs-distinct-paths trade-off, SmartScreen / unsigned-binary UX honest accounting, the registry-marker contract and why HKCU: see [`docs/architecture-decisions.md` § "Windows distribution model (v3.15.0+)"](./docs/architecture-decisions.md#windows-distribution-model-v3150). The full v3.15.0 → v3.15.1 post-mortem (both root causes, the misattributed ICE57 hypothesis, the rejected fix alternatives, the local-WiX repro path used to diagnose it) lives at [`docs/architecture-decisions.md` § "v3.15.1 addendum"](./docs/architecture-decisions.md#v3151-addendum--why-corporatewxs-lives-in-wix-corporate-not-wix).

### MSRV policy (v3.11.1+)

MSRV pinned in **two places that move in lockstep on every bump**:

1. **`Cargo.toml` `rust-version = "1.95"`** — cargo-side declaration. Produces the user-facing `error: package tr300@X.Y.Z cannot be built because it requires rustc N.M ...` for users on older toolchains.
2. **`rust-toolchain.toml`** at repo root:
   ```toml
   [toolchain]
   channel = "1.95"
   components = ["rustfmt", "clippy"]
   ```
   Rustup-side override. **Both fields are required.**

   - `channel = "1.95"` — `release.yml` is auto-generated by cargo-dist v0.31.0 and has NO rustup setup step; without this, runners use their pre-installed rustc (currently 1.94.1 on `ubuntu-22.04`, `ubuntu-22.04-arm`, `windows-2022`) and the build fails with `error: rustc 1.94.1 is not supported`. This is the v3.13.1 fix for task #54 — no GitHub Release artifact had been published since v3.10.0 because of this.
   - `components = ["rustfmt", "clippy"]` — non-obvious but load-bearing. When rustup honors a `rust-toolchain.toml`, it installs only the default profile (rustc + cargo + rust-std) and **ignores any action-level `components:` field** passed to `dtolnay/rust-toolchain@stable`. Without the components list, ci.yml's `Format` and `Clippy` jobs fail with `error: 'cargo-fmt' is not installed for the toolchain '1.95-x86_64-unknown-linux-gnu'`. v3.13.1 was published in two commits because the first attempt missed this.

**Why pinned MSRV** rather than `#[allow(unused_unsafe)]` shims or `#[cfg(rustc_version)]` ladders: cargo's `rust-version` field already enforces this without source-level shims; combined with auto-rustup in `--update` (above), rustup-managed users never see the error and distro-managed users get a clear actionable message.

**Channel format**: minor pin (`"1.95"`, not `"1.95.0"` and not `"stable"`). Rustup installs the latest patch in the 1.95.x line — patch releases land without a bump; minor floats stay explicit.

**On every MSRV bump**: edit `Cargo.toml` `rust-version` AND `rust-toolchain.toml` `channel` in the same commit. See [rustup overrides](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file) for the precedence rules.

— Full reasoning, the three rejected alternatives (allow / cfg ladder / no-pin), the v3.10.0–v3.13.0 partial-release history, the v3.13.1 two-commit fix-forward narrative: see [`docs/architecture-decisions.md` § "MSRV policy"](./docs/architecture-decisions.md#msrv-policy-v3111-addendum-v3131).

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
- **Tag a release**: bump version (already done in `F.4`), commit + push commits, wait for `ci.yml` to go green on the exact commit, then `git tag vX.Y.Z && git push origin vX.Y.Z`. The tag push triggers cargo-dist's `release.yml`. Push a single explicit version tag; do not use broad `git push --tags`.

### Phase 7 — Close out

Mark the parent PR task `completed` in `TaskList`. Move on to the next PR's parent task and start phase 3 again. PR #6 (and other "deferred" tasks) only run if the user explicitly asks after the previous PR lands.

## CI

Three GitHub Actions workflows guard release quality and publication:

- **`.github/workflows/ci.yml`** — runs on every push to master and every pull request. Jobs:
  - `fmt` — `cargo fmt --check` (Linux only)
  - `clippy` — `cargo clippy --all-targets --workspace -- -D warnings` (Linux only)
  - `test` — `cargo test --workspace --all-targets` on Linux + macOS ARM + Windows
  - `build` — `cargo build --release` smoke test on every platform, plus `--version` and `--fast --json` invocation to verify the binary actually runs
  - `speed` — measures `tr300 --fast` median wall-clock across 5 runs on Linux/macOS/Windows; fails the build if any platform's median exceeds the 1500 ms budget. Records numbers in the GitHub Actions step summary so PR reviewers see them.
  - `audit` — `cargo audit` against RustSec advisories (advisory-only via `continue-on-error: true`; flagged vulnerabilities should be triaged within one release cycle but don't gate PRs)
  - `dist-plan` — runs `dist plan` to verify cargo-dist config parses; catches dist regressions before they bite at tag time
- **`.github/workflows/release.yml`** — cargo-dist v0.31.0 release workflow. Triggered by tag push (`vX.Y.Z`). Builds 6 targets and produces shell + PowerShell + MSI installers. It also copies `tr300-installer.*` to legacy `tr-300-installer.*` aliases before creating the GitHub Release so v3.14.2 binaries can self-update after the old package name was removed. `Cargo.toml` sets `allow-dirty = ["ci"]` so `dist plan` accepts this checked-in customization. Regenerate via `dist init` after changing `[workspace.metadata.dist]` in `Cargo.toml`, then preserve the alias-copy step if v3.14.2 compatibility is still needed.
- **`.github/workflows/crates-publish.yml`** — runs after successful `CI` workflow runs from pushes to `master`/`main`, checks out the exact CI-tested SHA, skips already-published crate versions using a descriptive crates.io data-access `User-Agent`, reruns fmt/clippy/tests/package/dry-run with `--locked`, and publishes with the repository Actions secret `CARGO_REGISTRY_TOKEN`.

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

**Contract: CI never blocks on Intel; releases still produce the artifact.**

- `.github/workflows/ci.yml` — **no `macos-13` entry**. The default state is "Intel is not in CI, period." Tested matrix is Linux x64 glibc + macOS ARM + Windows x64.
- `[workspace.metadata.dist].targets` in `Cargo.toml` — **still includes `x86_64-apple-darwin`**. cargo-dist's `release.yml` still builds it on a `macos-13` runner at every `vX.Y.Z` tag. The mismatch between tested (3) and shipped (6) targets is intentional.

**Don't re-add `macos-13` to ci.yml** without a concrete reason and a capacity-risk discussion. `macos-13` is GitHub's last public Intel x86_64 macOS hosted runner label and capacity is structurally winding down (no `macos-14`/`-15`/`-latest` Intel variant exists). Pre-removal CI runs queued 3h+ to 15h+ on Intel before manual cancellation; `continue-on-error: true` is theatrical coverage — same UX. Hard removal was the only fix that actually changed the dashboard state.

**Don't drop `x86_64-apple-darwin` from cargo-dist targets** unless `release.yml` starts taking >2h at tag time because of `macos-13` queue depth. Tag cadence is weekly-to-monthly, willing to absorb the wait — users on 2019/2020-era Intel hardware deserve a working binary download.

**For Intel-specific bugs**: reproduce locally on Intel hardware (or one-shot self-hosted runner). The arch-agnostic `#[cfg(target_os = "macos")]` paths mean Apple Silicon CI exercises every line of the macOS code path; bug rate doesn't justify the queue cost.

— Full reasoning, the five concrete CI run IDs / queue times that triggered the removal, the rejected `continue-on-error` and target-drop alternatives, the per-architecture correctness analysis: see [`docs/architecture-decisions.md` § "Intel macOS coverage policy (v3.11.2+)"](./docs/architecture-decisions.md#intel-macos-coverage-policy-v3112).

## Release Process

Uses **cargo-dist** (v0.31.0) for fully automated cross-platform releases.

**Every release requires ALL of these steps:**

1. Bump `version` in `Cargo.toml`
2. Update the full documentation set for any user-visible release, install,
   update, or deployment behavior change: `CHANGELOG.md`, `README.md`,
   `CODEX_PROJECT.md`, `AGENTS.md`, `CLAUDE.md`, `MASTER_PLAN.md`,
   `TESTING.md`, and `docs/architecture-decisions.md` when rationale or
   release workflow changes. Update `/Users/realemmetts/.codex/AGENTS.md`
   when repo deployment workflow changes.
3. Commit with message `release: vX.Y.Z - <summary>`
4. Push the commit and wait for `ci.yml` to pass on that exact commit
5. Confirm `crates-publish.yml` published the new crates.io version from that same SHA or skipped because it already existed
6. Create git tag: `git tag vX.Y.Z`
7. Push the single tag explicitly: `git push origin vX.Y.Z`
8. Wait for the cargo-dist release workflow to publish GitHub Release assets, including canonical `tr300-installer.*` assets and legacy `tr-300-installer.*` aliases

The tag push triggers GitHub Actions to build all 6 targets (Windows x64, macOS Intel/ARM, Linux x64 glibc/musl, Linux ARM64) and generate shell/PowerShell/MSI installers.

`crates-publish.yml` is separate from generated cargo-dist release automation. It runs only after the `CI` workflow succeeds on `master`/`main`, checks out the exact CI-tested SHA, skips already-published versions, and requires `CARGO_REGISTRY_TOKEN` to publish. `Cargo.lock` is intentionally tracked and included in the package because both local verification and the publish workflow use `cargo publish --locked`.

### Regenerating CI Workflow

After changing `[workspace.metadata.dist]` in Cargo.toml:
```bash
dist init    # Regenerates .github/workflows/release.yml
```

Note: The binary is `dist`, not `cargo dist` — it installs as a standalone command.
