# TR-300

[![CI](https://github.com/QubeTX/qube-machine-report/actions/workflows/ci.yml/badge.svg)](https://github.com/QubeTX/qube-machine-report/actions/workflows/ci.yml)
[![Release](https://github.com/QubeTX/qube-machine-report/actions/workflows/release.yml/badge.svg)](https://github.com/QubeTX/qube-machine-report/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-PolyForm%20Noncommercial-blue.svg)](LICENSE)

Cross-platform system information report with Unicode box-drawing tables.

TR-300 is a standalone Rust CLI for fast, reliable, and readable terminal machine reports.

Latest release: [v3.17.0](https://github.com/QubeTX/qube-machine-report/releases/tag/v3.17.0) (2026-06-08). Windows users get four installer options — Global / Corporate Editions, each in MSI and EXE formats — none of which require Rust on the install machine. macOS / Linux ship via cargo-dist's shell installer. The crates.io package is [`tr300`](https://crates.io/crates/tr300).

Development status: the default branch contains an unreleased macOS accuracy
and reliability checkpoint. v3.17.0 remains the installable release until live
Windows, AMD Linux, and Raspberry Pi 4 validation is complete and v4.0.0 is
explicitly tagged.

The next version is a major release for Rust-library SemVer only: public
information records gained fields and are now non-exhaustive. The `tr300`
command, existing schema-v1 JSON keys, installer names, and update path remain
compatible. Applications that directly construct or exhaustively match public
Rust records should follow the v4 migration note when it is published.

## Features

- Cross-platform support (Windows, macOS, Linux)
- Beautiful Unicode box-drawing tables with ASCII fallback
- ASCII fallback mode for legacy terminals
- Bar graphs for CPU load, memory, and disk usage
- VPN-aware network information on Windows — `MACHINE IP` and `DNS IP` rows reflect the active default route (`GetBestInterfaceEx`-driven) so Tailscale / WireGuard / OpenVPN / corporate VPN tunnels are reported correctly instead of a coin-flip pick
- Hypervisor / virtualization detection (CPUID-based; disambiguates Win11 VBS from real VMs)
- Session info with last-login/current-session tracking; Windows uses WTS APIs with a boot-time fallback
- Disk encryption status (FileVault on macOS, BitLocker on Windows, and root
  encryption signals on Linux when readable)
- Native macOS detail from a single structured snapshot: model, OS build/name,
  real Normal/Safe/Recovery boot state, Apple chip and P/E topology, logical and
  native display resolution, battery health/cycles, and explicit Rosetta host /
  process architecture without leaking hardware identifiers
- Fast Startup-aware uptime on Windows — when the kernel session age and the WMI cold-boot time diverge by >1h (typical on Win10/Win11 laptops with `HiberbootEnabled`), the `UPTIME` row renders both as `9d 4h 12m (session: 7h 14m)`
- 5-state battery awareness on Windows — distinguishes `AC Power` (plugged in, fully topped up), `X% (Charging)`, `X% (Plugged in)` (gaming-laptop case where peak GPU draw exceeds the brick wattage, OR firmware-limited charging modes like ThinkPad battery longevity), `X% (Discharging)`, plus `Critical` / `Low` overrides
- Smart terminal detection on Windows — env-var pre-checks (`WT_SESSION`, `TERM_PROGRAM`, `CURSOR_TRACE_ID`) then a parent-process walk via Toolhelp32 that recognizes Windows Terminal, WezTerm, Alacritty, VS Code, Cursor, Windsurf, Hyper, Tabby, Ghostty, Kitty, MinTTY, Claude Code, and Antigravity even when env vars are absent
- PowerShell 7+ ("PowerShell Core") detection on Windows — reads `HKLM\SOFTWARE\Microsoft\PowerShellCore\InstalledVersions\<GUID>\SemanticVersion` so `pwsh` users see the actual installed version instead of falling back to Windows PowerShell 5.x
- Schema-versioned JSON output for scripting, including collection mode and
  explicit CPU-load, frequency, disk, and memory value definitions
- Collision-safe auto-save of Markdown reports to Downloads on manual runs,
  with `--no-save` for scripts and temporary checks
- Fast mode (`--fast`) for sub-second auto-run startup
- Positional action syntax (`tr300 update`, `tr300 install`, `tr300 uninstall`) with legacy flag compatibility
- Resilient self-update with cargo-first probing and shell/PowerShell installer fallbacks — the cargo path now verifies the new version actually landed (and falls through to the prebuilt installer if crates.io is lagging), so `tr300 update` no longer reports a false success
- Conditional platform detail rows for machine model, CPU core topology, ZFS health, motherboard, BIOS, and RAM slots when the host exposes them
- Self-installation with shell alias and auto-run

## Installation

TR-300 ships **prebuilt binaries** — no Rust toolchain required to install or
run. Pick the installer that matches your operating system and (on Windows)
whether you have admin rights on the machine.

### Windows — Global Edition (recommended for personal machines)

Installs to `C:\Program Files\tr300\bin\tr300.exe` and adds it to **system PATH**
so every terminal on the machine can find it. **Requires admin (UAC prompt).**

Two equivalent download options — **pick one**, not both. Both install to the
same location; installing both creates duplicate Add/Remove Programs entries.

| Format | Direct download | When to pick |
|---|---|---|
| MSI | [tr300-x86_64-pc-windows-msvc.msi](https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr300-x86_64-pc-windows-msvc.msi) | IT-managed deployment (Intune, SCCM, Group Policy) and most personal installs. The default. |
| EXE | [tr300-x86_64-pc-windows-msvc-setup.exe](https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr300-x86_64-pc-windows-msvc-setup.exe) | If you prefer a familiar `setup.exe`. Same outcome as the MSI. |

After install, open a new terminal and run `tr300`. The binary is unsigned, so
SmartScreen will show **"Windows protected your PC"** the first time —
click **More info → Run anyway**. (Code signing is on the roadmap;
see [Releases](https://github.com/QubeTX/qube-machine-report/releases) for
status.)

### Windows — Corporate Edition (recommended for locked-down work machines)

Installs to `%LocalAppData%\Programs\tr300\bin\tr300.exe` and adds it to your
**user PATH** only — no admin, no UAC, no system-wide changes. Use this on
machines managed by Intune / Active Directory / Group Policy when you don't
have admin rights but still need to install software.

| Format | Direct download | When to pick |
|---|---|---|
| MSI | [tr300-x86_64-pc-windows-msvc-corporate.msi](https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr300-x86_64-pc-windows-msvc-corporate.msi) | IT can push this via Intune assignment; works under AppLocker / managed-installer policy when allowlisted. |
| EXE | [tr300-x86_64-pc-windows-msvc-corporate-setup.exe](https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr300-x86_64-pc-windows-msvc-corporate-setup.exe) | Familiar `setup.exe` for end users on restricted machines. |

Same "pick ONE format" rule as the Global Edition. If WDAC / AppLocker blocks
both, your only options are the bare-EXE path below or asking IT to allowlist
one of these installers.

### Choosing between Global and Corporate Edition

| You're on… | Pick |
|---|---|
| Your own laptop / desktop, you have admin | **Global** |
| A work machine where you have admin | **Global** |
| A work machine where you don't have admin but can install apps to your user profile | **Corporate** |
| A locked-down machine that blocks `C:\Program Files` writes | **Corporate** |
| A server, ARM Windows, or anything else | Use the bare EXE / shell installer below |

The v3.17.0+ installers offer to remove an older Cargo copy and the other
edition (both cleanup choices are on by default), keeping one active install.
You can opt out during an interactive install when side-by-side copies are
intentional.

### macOS / Linux

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr300-installer.sh | sh
```

Installs to `~/.cargo/bin/tr300` and modifies your shell's PATH. **Does not
require Rust** — downloads the prebuilt binary from the GitHub Release.

### Alternative install methods

<details>
<summary>Windows PowerShell one-liner (downloads the prebuilt binary, no Rust)</summary>

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr300-installer.ps1 | iex"
```

Installs to `%USERPROFILE%\.cargo\bin\tr300.exe` (the directory is created
automatically — Rust is not required). Modifies user PATH only. Fails if
PowerShell `ExecutionPolicy` is `Restricted` and not overridden; the MSI /
EXE installers above don't have that limitation.
</details>

<details>
<summary>Windows — bare EXE (no installer, no PATH modification)</summary>

Download [tr300-x86_64-pc-windows-msvc.zip](https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr300-x86_64-pc-windows-msvc.zip),
extract `tr300.exe`, place it anywhere you can write to (your Desktop, a
USB stick, `C:\bin`, etc.). Run by typing the full path or by adding the
containing directory to your PATH manually. Useful for portable use, USB
sticks, and scenarios where any installer is blocked.
</details>

<details>
<summary>Build from source — for developers / contributors</summary>

Requires Rust **1.95.0 or later** (`rustup update stable` if needed).
Most users should NOT use these paths — they download and compile the
source instead of using the prebuilt binary, which is slower and requires
the Rust toolchain.

**From crates.io:**
```bash
cargo install tr300
```

**From a specific Git tag:**
```bash
rustup update stable
cargo install --git https://github.com/QubeTX/qube-machine-report.git --tag v3.17.0
```

**Local clone for development:**
```bash
git clone https://github.com/QubeTX/qube-machine-report.git
cd qube-machine-report
cargo build --release
```
</details>

## Usage

```bash
# Display system report (default)
tr300

# Use ASCII characters instead of Unicode
tr300 --ascii

# Output as JSON
tr300 --json

# Display a full table without writing a Markdown file to Downloads
tr300 --no-save

# Custom title
tr300 --title "MY SERVER"

# Disable colors
tr300 --no-color

# Self-update to the latest release
tr300 update
# Legacy form still works:
tr300 --update

# Install to shell profile (adds 'report' alias + auto-run)
tr300 install
# Legacy form still works:
tr300 --install

# Remove from shell profile
tr300 uninstall
# Legacy form still works:
tr300 --uninstall

# Show help
tr300 --help
```

## Example Output

```
┌┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┬┐
├┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┴┤
│            QUBETX DEVELOPER TOOLS                 │
│             TR-300 MACHINE REPORT                 │
├──────────────┬────────────────────────────────────┤
│ OS           │ Windows 11 24H2                    │
│ KERNEL       │ Windows 10.0.26200                 │
├──────────────┼────────────────────────────────────┤
│ HOSTNAME     │ DESKTOP-ABC123                     │
│ MACHINE IP   │ 192.168.1.100                      │
│ CLIENT  IP   │ Not connected                      │
│ DNS  IP 1    │ 192.168.1.1                        │
│ USER         │ emmett                             │
├──────────────┼────────────────────────────────────┤
│ PROCESSOR    │ Intel Core Ultra 7 155H            │
│ CORES        │ 22 vCPU(s) / 1 Socket(s)           │
│ HYPERVISOR   │ Bare Metal                         │
│ CPU FREQ     │ 1.4 GHz                            │
│ LOAD  1m     │ ███████░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
│ LOAD  5m     │ █████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
│ LOAD 15m     │ ████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
├──────────────┼────────────────────────────────────┤
│ VOLUME       │ 848.18/935.54 GB [90.66%]          │
│ DISK USAGE   │ █████████████████████████████░░░░░ │
├──────────────┼────────────────────────────────────┤
│ MEMORY       │ 18.00/31.52 GiB [57.1%]            │
│ USAGE        │ ██████████████████░░░░░░░░░░░░░░░░ │
├──────────────┼────────────────────────────────────┤
│ LAST LOGIN   │ Jul 13 22:41                        │
│ UPTIME       │ 5h 38m                             │
├──────────────┴────────────────────────────────────┤
└──────────────┴────────────────────────────────────┘
```

## Command Line Options

Positional actions can be written without a double dash. They are mutually
exclusive with each other and with the legacy action flags.

| Action | Description |
|--------|-------------|
| `update` | Check for updates and install the latest version |
| `install` | Add to shell profile with alias and auto-run |
| `uninstall` | Remove from shell profile |

| Option | Description |
|--------|-------------|
| `--ascii` | Use ASCII characters instead of Unicode |
| `--json` | Output in JSON format |
| `-t, --title <TITLE>` | Custom title for the report header |
| `--no-color` | Disable colored output |
| `--fast` | Fast mode: skip slow collectors for quick auto-run |
| `--no-save` | Do not auto-save Markdown after a full table run |
| `--no-elevation-hint` | Suppress the optional Linux `sudo` detail hint |
| `--update` | Legacy flag form of `tr300 update` |
| `--install` | Add to shell profile with alias and auto-run |
| `--uninstall` | Remove from shell profile |
| `-h, --help` | Print help information |
| `-V, --version` | Print version information |

## Self-Update

Both action forms update to the latest release:

```bash
tr300 update
tr300 --update
```

`tr300 update` detects how this binary was installed and downloads the matching
installer for an **in-place upgrade** — no manual download required. The update
**preserves your edition and its permissions**: a Corporate (per-user) install
updates as Corporate (same `%LocalAppData%\Programs\tr300` location, user PATH, no
admin prompt), and a Global install updates as Global (Program Files, system PATH,
UAC) — it never silently switches your install style or location.

**Single install at a time (v3.17.0+):** the Windows installers — and silent
self-updates — consolidate to one version/edition: they remove an older
`cargo install` copy and/or the *other* edition if present (two checkboxes, both on
by default; untick to keep). The cleanup only ever removes `tr300` itself — never
your Rust toolchain, your Downloads, or the running copy — and can never cause an
install or update to fail.

**On Windows (v3.15.0+):**

| Installed via… | `tr300 update` does | UAC? |
|---|---|---|
| Global MSI | Downloads the new Global MSI, runs `msiexec /i` | Yes |
| Corporate MSI | Downloads the new Corporate MSI, runs `msiexec /i` | No |
| Global EXE | Downloads the new Global EXE installer, runs `/SILENT` | Yes |
| Corporate EXE | Downloads the new Corporate EXE installer, runs `/SILENT` | No |
| `cargo install` / PowerShell installer | Falls through to the legacy chain (cargo first, then `irm \| iex` PowerShell installer) | No |

Detection uses a `HKCU\Software\TR300\InstallSource` registry marker that the
four first-class installers write at install time. If the marker is missing
(legacy install from before v3.15.0), the updater falls back to inspecting the
running binary's path: `C:\Program Files\tr300\` → MSI Global, `%LocalAppData%
\Programs\tr300\` → MSI Corporate, `~\.cargo\bin\` → legacy chain.

**Security (v3.15.2+):** every downloaded MSI / EXE installer is checked against
its published `.sha256` sidecar before launching. A network MITM (corporate
TLS-inspection proxy with trusted root CA, hostile public WiFi, captive
portal) that swaps the installer bytes is now caught and refused with a
clear error.

**Post-install verification (v3.15.2+):** after the installer reports success,
`tr300 update` re-execs the on-disk binary's `--version` and confirms the
upgrade actually took effect. If Windows Installer's Restart Manager
scheduled a delete-on-reboot (msiexec exit code 3010) rather than replacing
the locked binary in-place, the JSON `attempts[].message` contains an
actionable "Reboot, then verify with `tr300 --version`" rather than a
false-positive success.

**On macOS / Linux:**

1. Checks the latest release on GitHub. If you're already current, exits 0
   without changing anything.
2. Tries `cargo install tr300 --force` first when `cargo --version` succeeds.
   If `rustup` is present, it runs `rustup update stable` best-effort first so
   cargo installs against the current MSRV.
3. Falls through to the cargo-dist shell installer: `curl | sh` then `wget | sh`.

**Manual fallback (any platform):** if `tr300 update` fails, you can always
re-download the installer for your platform from the
[Releases page](https://github.com/QubeTX/qube-machine-report/releases) and
re-run it — WiX `MajorUpgrade` (MSI) and Inno Setup's AppId-based upgrade
detection (EXE) handle in-place upgrades cleanly.

In `--json` mode, every response includes a top-level `install_origin` field
on Windows (`msi-global` / `msi-corporate` / `exe-global` / `exe-corporate` /
`cargo-or-installer` / `unknown`). Successful updates include the legacy
`"method"` field plus a precise `"strategy"` value (`msi_global`,
`msi_corporate`, `exe_global`, `exe_corporate`, or the legacy installer IDs).
Failed updates include an `"attempts"` array with each strategy result and
diagnostic message.

## Release Automation

GitHub Actions handles both release assets and crates.io publishing:

- `CI` runs formatting, clippy, tests, release builds, speed checks, audit, and
  cargo-dist planning on pushes to the default branch.
- `Crates.io Publish` runs only after `CI` succeeds for that default-branch
  commit, checks whether the manifest version is already on crates.io with a
  descriptive data-access `User-Agent`, reruns fmt/clippy/tests/package/dry-run
  with `--locked`, and publishes `tr300` only when the repository
  `CARGO_REGISTRY_TOKEN` Actions secret is configured.
- `Release` is the cargo-dist workflow triggered by an explicit version tag such
  as `v3.14.3`; it builds the cross-platform archives and installers. New
  installer assets use `tr300-installer.*`; the workflow also publishes
  `tr-300-installer.*` compatibility aliases so v3.14.2 binaries can
  self-update after the old package name was removed. The cargo-dist config
  permits that checked-in workflow customization with `allow-dirty = ["ci"]`.

`Cargo.lock` is tracked so the crates.io publish workflow uses the same resolved
dependency set that local release verification used.

## Elevation Tier

TR-300 detects whether it is running with elevated privileges (root on Unix /
Administrator on Windows) and exposes that fact to JSON consumers.

- **Linux, unelevated** — a single dim footer notes that `sudo` may unlock RAM
  module details when `dmidecode` is installed. The hint is never shown during
  `--fast` auto-run.
- **Linux, elevated** — available `dmidecode` details render inline.
- **Windows and macOS** — no blanket elevation promise is shown. Optional
  BitLocker/FileVault data is reported when the OS exposes it; a missing value
  does not prove that Administrator or `sudo` would fix the probe.

To opt out of the hint, run `tr300 --no-elevation-hint` (or wire the flag into your
shell alias). The hint is rendered with ANSI dim, so respects `--no-color` as well.

The JSON output exposes this state under top-level keys `elevated` and
`elevation_unlocks_more` for scripted consumers.

## Installation to Shell Profile

Running `tr300 install` or `tr300 --install` will:

1. **Remove existing TR-300 configuration** (if present)
2. Add a `report` alias so you can type `report` instead of `tr300`
3. Configure auto-run on new interactive shell sessions

This means you can safely run `tr300 install` multiple times without duplicating profile blocks.

**Safety (v3.15.2+):** every modification to your rc files / `$PROFILE`
is written via a temp-then-atomic-rename. A Ctrl-C, power loss, or
antivirus quarantine mid-write can no longer leave your profile
truncated. The first install also writes a one-time `.tr300-backup`
copy of your original profile so you have a recovery option. If the
TR-300 marker block in your profile has been hand-edited into an
unbalanced state (e.g. `# End TR-300` line accidentally removed),
`tr300 install` refuses up-front with an actionable error rather
than silently dropping the rest of your profile.

**Auto-run guards (v3.15.2+):** the injected snippet now guards
the `tr300 --fast` call with `command -v tr300` (POSIX) /
`Get-Command tr300 -ErrorAction SilentlyContinue` (Windows), so a
missing binary no longer prints an error at every new shell start.
A `TR300_AUTORUN_RAN` sentinel prevents recursion in nested shells
(vim `:term`, `bash -i -c …`, Windows Terminal nested tabs). On
Windows, the snippet uses `[Environment]::UserInteractive` instead
of the older `$Host.Name -eq 'ConsoleHost'` check, so scripted
`pwsh -Command "..."` invocations from CI / VS Code / scheduled
tasks no longer dump the table into log streams.

**On Unix/macOS:** Modifies `~/.bashrc` and/or `~/.zshrc`. On a
fresh-account macOS machine where neither file exists, `tr300
install` creates `.zshrc` (macOS has defaulted to zsh since 10.15,
Catalina, 2019). `sudo tr300 install` is refused — TR-300 modifies
your personal shell profile, and running as root either targets
root's profile (no benefit to your shell) or leaves root-owned
files in your home that break subsequent non-sudo installs.

**On Windows:** Modifies the PowerShell `$PROFILE`. When both
Windows PowerShell 5.1 (`powershell`) and PowerShell 7 (`pwsh`)
are installed, `tr300 install` writes to BOTH profile paths
(`Documents\WindowsPowerShell\…` and `Documents\PowerShell\…`)
since the two shell flavors don't cross-source each other.
Pre-v3.15.2, pwsh-only users got a silent no-op install because
the snippet went to a profile their actual shell never read.

Fresh Windows machines default `ExecutionPolicy` to `Restricted`,
which blocks every `.ps1` including the profile itself, so
`tr300 install` runs a preflight that adjusts the `CurrentUser`
scope to `RemoteSigned` when needed. `RemoteSigned` is the minimum
policy that lets the local profile load — it still requires
downloaded `.ps1` files to be Authenticode-signed. The change is
HKCU-only, affects only your user account, and does not require
admin. If you have deliberately set `AllSigned`, `tr300 install`
will not silently downgrade it; it prints a notice explaining the
auto-run will not fire and leaves the policy alone.
See [Microsoft Learn — about_Execution_Policies](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_execution_policies).

To remove these additions, run `tr300 uninstall` or `tr300 --uninstall`.
The uninstall does not roll back your execution policy — other PowerShell
tooling typically relies on `RemoteSigned`, so restoring it would surprise
users.

**`uninstall` → Complete** on Windows handles the self-EXE delete case
(v3.15.2+). When you run `tr300 uninstall` from a Windows install
location (e.g. `%LocalAppData%\Programs\tr300\bin\tr300.exe`) and
choose "Complete", the OS would normally refuse to delete the
running binary. TR-300 detects this and schedules a detached
background job that waits 2 seconds for our process to exit, then
removes the binary and its empty parent folder. You'll see
"Scheduled deferred cleanup of: …" and your shell returns to a
prompt; the file is gone within a couple seconds.

If `tr300 install` (or `uninstall`) hits a permissions error on Windows —
common on work machines managed by Intune, Group Policy, AppLocker, or
WDAC, and on personal machines where Documents is redirected to OneDrive
— the tool prints a multi-paragraph explanation of what likely caused the
failure (OneDrive offline / online-only files, an MDM lockdown, antivirus
blocking the profile edit, a sharing violation, a disk-full condition, or
a MAX_PATH overflow) along with concrete next steps: which path to ask IT
to allowlist, where to add an antivirus exclusion, or how to `takeown`
ownership. Manual `tr300` invocations from the prompt always continue to
work in this case; only the auto-run on new shells is affected, and you
can re-run `tr300 install` once the underlying restriction is addressed.

## Building from Source

Requirements:
- Rust 1.95.0 or later (`rustup update stable`)
- Cargo

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test --workspace --all-targets

# Run clippy
cargo clippy --all-targets --workspace -- -D warnings

# Check formatting
cargo fmt --all -- --check

# Audit the locked dependency graph
cargo audit

# Run with arguments
cargo run -- --ascii
```

### Rust library migration for v4

The high-level APIs remain the preferred path:

```rust,no_run
use tr300::{report, CollectMode, Config, SystemInfo};

let info = SystemInfo::collect_with_mode(CollectMode::Full)?;
let rendered = report::generate(&info, &Config::default());
println!("{rendered}");
# Ok::<(), tr300::AppError>(())
```

Public data records and extensible enums are now `#[non_exhaustive]`. Downstream
code may read public fields, but should stop constructing those records with
external struct literals and should include a wildcard arm when matching public
enums. The CLI and existing schema-v1 JSON keys do not require migration.

### Man Page (Linux/macOS)

A man page is auto-generated during build via `clap_mangen`. After building from source:

```bash
sudo cp man/tr300.1 /usr/local/share/man/man1/
man tr300
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the PolyForm Noncommercial License 1.0.0 - see the [LICENSE](LICENSE) file for details. This license permits noncommercial use, personal use, and use by charitable organizations, educational institutions, and government agencies.
