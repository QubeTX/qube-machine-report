# TR-300 Testing Guide

This file tracks the manual verification matrix that must pass before each tagged release, plus the automated gates that protect the auto-run UX.

## Automated gates (run by CI on every PR)

`.github/workflows/ci.yml` runs these on every push and pull request:

- **`fmt`** — `cargo fmt --check` (Linux)
- **`clippy`** — `cargo clippy --all-targets --workspace -- -D warnings` (Linux)
- **`test`** — `cargo test --workspace --all-targets` on Linux + macOS ARM + Windows
- **`build`** — release build smoke test on all three platforms (with a `--version` + `--fast --json` invocation to verify the binary runs)
- **`speed`** — 5-run median of `tr300 --fast` on Linux + macOS ARM + Windows, fails if median > 1500 ms (auto-run safety gate). Reports times in the job summary.
- **`audit`** — `cargo audit` against RustSec advisories (advisory-only; doesn't block)
- **`dist-plan`** — verifies cargo-dist config parses, so dist regressions don't surprise us at tag time

To reproduce locally before pushing:

```bash
cargo fmt -- --check
cargo clippy --all-targets --workspace -- -D warnings
cargo test --workspace --all-targets
cargo run -- --json | jq .            # parses without error
cargo run -- --json update | jq .     # update action JSON shape
cargo run -- --fast --json | jq .     # same, fast mode
cargo run -- --ascii                  # visual inspection
```

### Output stability gates

These protect the auto-run experience (open terminal → table renders → prompt is free immediately). A regression here breaks the core UX promise.

- **T.S.1 — Line count**: `report --ascii` must not grow in line count. New rows are only allowed when they're conditional (e.g. ZFS Health only renders when `zpool` exists; battery health enriches an existing row in place; encryption row only on Windows when readable).
- **T.S.2 — Speed**: `tr300 --fast` wall-clock must not regress more than 100 ms on any platform. Capture before/after numbers in the PR description.

## Manual cross-platform matrix

The "Last verified" column tracks which release confirmed each row. Update as part of each PR's documentation block (F-tasks).

| Platform | Required checks | Last verified |
|---|---|---|
| **macOS Intel (Sonoma 14.x)** | OS shows "macOS 14.x"; CPU brand contains "Intel"; uptime present; battery on laptop | — |
| **macOS Apple Silicon M1** | CPU brand "Apple M1/Pro/Max" matches; freq ≠ 0; arch "Apple Silicon"; cores show P/E split | — |
| **macOS Apple Silicon M3 / M4** | CPU brand exact (no "Apple M1" stale); cores P/E; Mac marketing name correct; battery health present | — |
| **macOS Apple Silicon under Rosetta 2** | Arch shows `x86_64 (Apple Silicon, Rosetta 2)` | — |
| **Ubuntu 22.04+ (systemd-resolved)** | DNS row shows upstream resolvers, NOT 127.0.0.53 | — |
| **Debian 12 (no systemd-resolved)** | DNS row shows /etc/resolv.conf contents | — |
| **Fedora / Arch** | Hypervisor "None" on bare metal; terminal detection works for Konsole + GNOME Terminal + Wezterm | — |
| **Alpine in Docker** | Container detected; no panic on missing `lspci` / `lastlog` / systemd | — |
| **Raspberry Pi 4 (aarch64)** | CPU brand from devicetree, not empty | — |
| **AWS EC2 (Graviton or Intel)** | Hypervisor shows "amazon" / "kvm"; cloud detection works | — |
| **WSL2 on Win11** | Hypervisor shows "WSL2"; terminal shows "Windows Terminal" via WT_SESSION | — |
| **Windows 11** | OS shows "Windows 11" (not 10); arch correct; last-login covers session start; battery on laptop | 3.10.0 (footer hint visible; arch / OS / DNS unchanged in PR1) |
| **Windows 11 (BitLocker / Device Encryption ON)** | "Encryption" row shows "BitLocker On" non-admin if readable; full method when elevated | — |
| **Windows 11 (BitLocker OFF)** | "Encryption" row shows "Off" or absent + footer hint when not elevated | — |
| **Windows 11 as Administrator** | Encryption shows full method + protection level; **footer hint absent** | — |
| **Linux as root (sudo)** | Motherboard, BIOS, RAM slot rows present; **footer hint absent** | — |
| **Linux as user (no sudo)** | Motherboard / BIOS / RAM rows absent; one-line footer hint visible (full mode); footer ABSENT in `--fast` | — |
| **Windows 11 ARM** | Arch via IsWow64Process2 correct under both x64 and ARM64 native processes | — |
| **Windows 10 (no Fast Startup)** | No spurious session-suffix on uptime | — |
| **Windows 11 with Fast Startup** | Uptime annotated with `(session: Xh)` when divergent | — |
| **Bare desktop (no battery)** | Battery row absent — must NOT show "0%" or stub row | — |
| **ZFS host** | ZFS Health row appears with "ONLINE"; "DEGRADED" if pool degraded | — |
| **Non-ZFS host** | ZFS Health row absent | 3.10.0 (Windows; absent as expected) |
| **Multi-homed Linux (Ethernet + Wi-Fi + VPN)** | Local IP matches default route, not first in list | — |
| **macOS with VPN active** | Local IP shows VPN tun if it's the primary; DNS shows VPN-pushed servers via scutil | — |
| **Windows with VPN active** | Local IP via GetBestInterfaceEx matches default route | — |
| **RDP session on Windows 11** | Last-login shows session connect time, not stale boot time | — |

## Per-release verification log

Append a section per tagged release noting which matrix rows were re-verified and on which hardware. Lets us catch silent regressions when a row stops being checked.

### v3.10.0 — 2026-04-27

Foundation scaffolding only — no collector changes. Verified:

- Windows 11 (build 26200): footer hint renders below table in full mode; absent in `--fast`; ANSI dim escapes when colors enabled. JSON contains `schema_version: 1`, `elevated: false`, `elevation_unlocks_more: true`. `--no-elevation-hint` suppresses the line cleanly.
- Library tests: 15 passed (8 pre-existing + 7 new for elevation footer logic and schema version).

Pending hardware verification (no collector changes that would affect them, but matrix entries should be stamped on next per-platform PR): macOS Intel/AS, all Linux distros, WSL2.

### v3.14.0 — 2026-05-10

Cross-platform stability and action syntax pass. Verified on local macOS Apple
Silicon during implementation; Linux and Windows hardware-specific behavior is
fixture-covered locally and left to the GitHub Actions matrix / real machines
for runtime validation.

- **Action syntax** — unit tests cover `tr300 update`, `tr300 --update`,
  `tr300 --json update`, `tr300 install`, `tr300 uninstall`, and mixed-action
  rejection.
- **Collector stability** — subprocess helper tests cover success and timeout
  behavior; collector parser fixtures cover macOS battery/sysctl/vm_stat/scutil,
  Linux resolver/route/ZFS/dmidecode paths, and Windows PowerShell fallback
  JSON on Windows CI.
- **Output stability** — integration tests parse JSON with `serde_json`, verify
  fixed-width ASCII table rows, assert `--fast` omits slow conditional rows, and
  confirm help documents both action forms.
- **Local gate** — `cargo fmt -- --check`, `cargo clippy --all-targets
  --workspace -- -D warnings`, `cargo test --workspace --all-targets`, and
  `cargo build --release --workspace` pass on this Mac.
- **Runtime smoke** — `./target/release/tr300 --fast --json | python3 -m
  json.tool` parses successfully, and `./target/release/tr300 --ascii` renders
  the fixed-width report.
- **Fast timing** — sorted 7-run local macOS `--fast` times:
  `0.17, 0.18, 0.20, 0.21, 0.21, 0.23, 0.24` seconds; median `0.21s`.
- **CI verification** — `master` CI run 25642712712 passed fmt, clippy, tests,
  release builds, security audit, dist plan, and auto-run speed gates on macOS
  ARM, Linux, and Windows. Release run 25642853066 passed and published the
  v3.14.0 GitHub Release with 20 assets.
- **Deferred** — admin-only Windows RDP history is not implemented in this pass;
  current Windows elevation wording is limited to BitLocker status.

### v3.14.1 — 2026-05-11

Release confidence patch after the v3.14.0 CI fix-forward. No new runtime
collector or renderer behavior.

- **Latest pre-bump CI verification** — `master` CI run 25643018578 passed on
  commit `5709f9a` across fmt, clippy, tests, release builds, security audit,
  dist plan, and auto-run speed gates on macOS ARM, Linux, and Windows.
- **Local gate before release prep** — `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --workspace -- -D warnings`, and
  `cargo test --workspace --all-targets` passed on this Mac before bumping
  v3.14.1.
- **Release commit local gates** — repeated `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --workspace -- -D warnings`,
  `cargo test --workspace --all-targets`, and
  `cargo build --release --workspace` on the v3.14.1 commit. Runtime smoke
  also passed: `./target/release/tr300 --version` printed `tr300 3.14.1`,
  and `./target/release/tr300 --fast --json | python3 -m json.tool` parsed.
- **CI verification** — `master` CI run 25645894617 passed on commit
  `3328a8e` across fmt, clippy, tests, release builds, security audit, dist
  plan, and auto-run speed gates on macOS ARM, Linux, and Windows.
- **Release verification** — release.yml run 25645999755 passed plan, six
  target artifact builds, global artifacts, host, and announce jobs. The
  v3.14.1 GitHub Release is non-draft, non-prerelease, and published with
  20 cargo-dist assets.

### v3.14.2 — 2026-05-11

Crates.io publication, ND-style updater strategy chain, install/release docs,
and project identity cleanup. Runtime report collection/rendering behavior is
unchanged outside `tr300 update`.

- **Local release gates** — `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --workspace -- -D warnings`,
  `cargo test --workspace --all-targets`, `cargo package --locked --list`,
  `cargo publish --dry-run --locked`, and `~/.cargo/bin/dist plan` passed on
  this Mac before publishing. Package list contained 36 release files and
  excluded `.codex`, `.claude`, `.firecrawl`, `.github`, agent guides, and
  unrelated historical implementation files.
- **Update behavior tests** — unit tests cover cargo-first ordering on Unix and
  Windows, installer fallback ordering when cargo is unavailable, and JSON
  legacy `"method"` mapping vs precise `"strategy"` values.
- **CI verification** — `master` CI run 25647466576 passed on commit
  `a6c3841` across fmt, clippy, tests, release builds, security audit, dist
  plan, and auto-run speed gates on macOS ARM, Linux, and Windows.
- **Crates.io verification** — crates-publish run 25647553585 checked the
  exact CI-tested SHA, reran fmt/clippy/tests/package/dry-run, and published
  `tr-300` 3.14.2 to crates.io with license
  `PolyForm-Noncommercial-1.0.0`, binary `tr300`, and rust-version `1.95`.
  Initial run 25647407638 failed before publishing due the crates.io version
  check missing a descriptive data-access `User-Agent`; follow-up commit
  `a6c3841` fixed the workflow.
- **Release verification** — release.yml run 25647597021 passed plan, six
  target artifact builds, global artifacts, host, and announce jobs. The
  v3.14.2 GitHub Release is non-draft, non-prerelease, and published with
  20 cargo-dist assets.

### v3.14.3 — 2026-05-11

Canonical crates.io package rename from the deleted `tr-300` package name to
`tr300`, plus matching self-update, library import path, installer URL, MSI
name, and release documentation updates.

- **Crates.io availability check** — `https://crates.io/api/v1/crates/tr300`
  returned 404 before release, confirming the corrected package name is
  available for creation.
- **Local release gates** — `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --workspace -- -D warnings`, and
  `cargo test --workspace --all-targets` passed on this Mac after the package
  rename. Test counts: 38 library tests and 18 integration tests.
- **Package verification** — `cargo package --locked --allow-dirty --list`
  listed the expected 36 release files, and
  `cargo publish --dry-run --locked --allow-dirty` packaged and verified
  `tr300 v3.14.3` successfully before the release commit. After committing,
  the strict `cargo package --locked --list` and
  `cargo publish --dry-run --locked` gates also passed.
- **cargo-dist verification** — `dist plan` passed and announced canonical
  `tr300-*` release artifacts, including `tr300-installer.sh`,
  `tr300-installer.ps1`, six platform archives, the Windows MSI, checksums,
  and source tarball. The checked-in release workflow adds legacy
  `tr-300-installer.*` aliases for v3.14.2 updater compatibility.
- **CI verification** — `master` CI run 25648618096 passed on commit
  `25305d8` across fmt, clippy, tests, release builds, security audit, dist
  plan, and auto-run speed gates on macOS ARM, Linux, and Windows.
- **Crates.io verification** — crates-publish run 25648707510 checked the
  exact CI-tested SHA, reran fmt/clippy/tests/package/dry-run, and published
  `tr300` 3.14.3 to crates.io with license
  `PolyForm-Noncommercial-1.0.0`, binary `tr300`, library target `tr300`,
  and rust-version `1.95`.
- **Release verification** — release.yml run 25648740343 passed plan, six
  target artifact builds, global artifacts, host, and announce jobs. The
  v3.14.3 GitHub Release is non-draft, non-prerelease, and published with
  22 cargo-dist assets: canonical `tr300-*` archives/installers/checksums,
  source assets, `dist-manifest.json`, and the legacy
  `tr-300-installer.sh` / `tr-300-installer.ps1` aliases.

### v3.11.0 — 2026-04-27

Windows accuracy + BitLocker (PR #4). Verified on Windows 11 25H2 (build 26200.8246), unelevated user session:

- **OS row** — was `Windows 11 (26200)`, now `Windows 11 25H2`. Registry-based detection working.
- **Kernel row** — was `26200`, now `26200.8246` (full build with UBR).
- **Last-login row** — was `Login tracking unavailable`, now real timestamp `Tue Apr 21 22:12` (matches uptime). WTSLogonTime returned 0 (console session quirk); fell back to GetTickCount64-derived boot time as designed.
- **CPU freq row** — still `1.4 GHz` on this host (machine is power-plan throttled at 1400 MHz; CPUID leaf 16h returns 0 on Meteor Lake; CallNtPowerInformation correctly reports 1400 MaxMhz). Implementation correct; will show higher values on machines with full performance power plan or older Intel chips where leaf 16h works.
- **Hypervisor row** — was `Hypervisor Present`, now `Bare Metal (Hyper-V/VBS)`. CPUID returned `Microsoft Hv` correctly; SMBIOS manufacturer disambiguated to "physical host with VBS active".
- **Encryption row** — absent on this user's unelevated session (Win32_EncryptableVolume returned access-denied as expected). Footer hint covers the gap. Will surface on Win11 Device Encryption laptops and admin sessions.
- **Architecture row** — `x86_64` (unchanged on x64 host running x64 binary; IsWow64Process2 implementation will activate on ARM64 hosts).
- **Footer hint** — still renders correctly with the BitLocker mention; wording was later narrowed to implemented BitLocker-only elevated data.
- Integration tests: 13 passed (1 new for JSON `encryption` key); library tests: 15 passed.

Pending verification (deferred or platform-locked):
- Windows 11 ARM64 host (C.2 IsWow64Process2 emulation annotation)
- Windows 11 with admin shell (BitLocker full method visible)
- Windows 11 with Device Encryption ON, unelevated (BitLocker row should appear)
- Windows 11 in a real Hyper-V VM (CPUID `Microsoft Hv` + Microsoft Corp manufacturer → `Hyper-V`, not `Bare Metal (Hyper-V/VBS)`)
- Windows running inside KVM / VMware / VirtualBox (CPUID-based hypervisor brand detection)

### v3.12.0 — 2026-04-28

Windows accuracy refinements (PR #4b). Verified on Windows 11 25H2 (build 26200.8246), unelevated user session:

- **MACHINE IP / DNS IP rows (C.4)** — `GetBestInterfaceEx`-driven adapter selection working. With no VPN active, `MACHINE IP` resolved to `10.1.0.85` (LAN adapter selected as default route by the kernel — correct). `DNS IP 1` resolved to `10.1.0.1` (LAN gateway DNS — correct). Falls through to legacy first-match order when `get_best_route_interface_index()` returns `None`. JSON output includes `network.machine_ip` and `network.dns_servers[]` populated by the same path.
- **UPTIME row (C.5)** — `detect_fast_startup()` correctly read `HiberbootEnabled = 1` from registry. `last_cold_boot_seconds()` parsed `Win32_OperatingSystem.LastBootUpTime` via `wmi::WMIDateTime` (early hand-written CIM datetime parser was discarded after testing — wmi crate's serde wrapper handles the format natively). On this session the cold-boot time and kernel session age aligned within 1 hour, so the parenthetical annotation correctly stayed dormant. The `(session: …)` annotation will activate on hosts where Shut Down + Boot used Fast Startup hibernation resume (annotation appears when divergence > 1h).
- **JSON `os.session_uptime_seconds` key** — present in every output, nullable per design. New integration test `test_json_includes_session_uptime_seconds_key` pins the contract.
- **`--fast` median timing** — unchanged from v3.11.x baseline (~308 ms). Phase B is full-mode-only; the C.5 WMI cold-boot query is gated on `mode == CollectMode::Full`.
- Integration tests: 14 passed (1 new for `os.session_uptime_seconds`); library tests: 15 passed.

Pending verification (deferred or platform-locked):
- Windows 11 with active VPN client (Tailscale, WireGuard, OpenVPN, Cisco AnyConnect): `MACHINE IP` should swap between LAN address and tunnel address as VPN toggles on/off
- Windows 11 host immediately after Shut Down + Boot (Fast Startup hibernation resume): `UPTIME` row should display `(session: …)` annotation with kernel-session age shorter than cold-boot age
- Windows 11 host immediately after Restart (cold boot, bypasses Fast Startup): annotation should NOT appear (sysinfo uptime ≈ WMI cold-boot time)
- Windows host with `IP Helper` service disabled: `GetBestInterfaceEx` should fail and the function should fall through to legacy first-match adapter ordering

### v3.13.0 — 2026-04-28

PR #5 partial — Windows polish. Verified on Windows 11 25H2 (build 26200.8246), unelevated user session, fully-charged Alienware laptop on AC:

- **BATTERY row (C.10 + C.10b)** — was `100% (Discharging (High))` (legacy WMI `Win32_Battery`'s confusing `BatteryStatus` mapping), now `AC Power` (clean: percentage at full charge is uninformative, just shows AC state). Native `GetSystemPowerStatus` call confirmed working — no COM round-trip overhead. The 5-state model also covers gaming-laptop "PSU undersized for peak GPU draw" → `X% (Plugged in)`, firmware-limited charging (ThinkPad battery longevity) → also `X% (Plugged in)`, and the historical off-AC `X% (Discharging)` / `(Critical)` / `(Low)` states.
- **CORES row (C.9)** — value unchanged (1 socket on this single-package CPU), but now via native `GetLogicalProcessorInformationEx` walking variable-length `SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX` records. ~10x faster than the WMI path it replaced.
- **GPU rows (C.8)** — three hardware adapters detected: Intel Arc Graphics, NVIDIA GeForce RTX 4070 Laptop GPU, Trigger 6 External Graphics. No "Microsoft Basic Render Driver" or other software adapters (registry-prefer path doesn't enumerate them; `filter_software_gpus` name-based filter is the second line of defense).
- **SHELL row (C.11)** — `bash` (we're in Git Bash). PSCore detection fell through correctly (no PowerShell 7+ installed on this host); legacy WinPS-5.x path works as before. The PSCore detection logic was unit-verified by inspecting the `reg query` output format.
- **TERMINAL row (C.12)** — was `Console`, now `Claude Code`. Parent-process walk via Toolhelp32 correctly traversed `tr300.exe → bash.exe → claude.exe` and matched the "Claude Code" label. Verified by manual `Get-Process` parent-walk in PowerShell which produced the same chain.
- **Elevation footer** — the Windows admin hint still renders correctly in v3.13.0 (feature shipped v3.10.0, no regression; wording was later narrowed to BitLocker-only). Suppressed by `--no-elevation-hint` flag and never rendered in `--fast` mode.
- **JSON additive keys** — `cpu.gpus` and existing keys all present; no new top-level keys added in v3.13.0 (no schema bump per MASTER_PLAN §4.9).
- **`--fast` median timing** — 338 ms (sorted-7 middle: 0.332, 0.333, 0.337, 0.338, 0.339, 0.376, 0.394). +30 ms vs v3.11 baseline ~308 ms. Within 100 ms budget per MASTER_PLAN §5; well under 1500 ms CI gate. Slight regression attributed to additional winapi feature bindings linked into the binary; full-mode collectors are equal-or-faster (C.9 saves ~30 ms via native socket count, C.10 saves ~40 ms via native battery, neither on the fast path).
- Integration tests: 14 passed (no new tests in v3.13.0; the existing test suite covers the additive changes); library tests: 15 passed.

Pending verification (deferred to future sessions):
- Windows 11 host with PowerShell 7+ installed (`HKLM\SOFTWARE\Microsoft\PowerShellCore\InstalledVersions\<GUID>` populated): SHELL row should show `PowerShell 7.x.y` instead of falling back to legacy 5.x detection
- Windows 11 host launched directly from Windows Terminal (without env-var inheritance loss): `WT_SESSION` env var path should match before the parent walk runs; verifies the env-var fast path
- Windows 11 host launched from WezTerm / Alacritty / Cursor / Tabby / Hyper / Ghostty / Kitty: parent walk should match the respective terminal label
- Gaming laptop running an active GPU-heavy load that exceeds AC brick wattage: BATTERY row should show `X% (Plugged in)` with percentage decreasing over time (validates the C.10b heuristic for the supplementing-from-battery case)
- ThinkPad / ASUS with battery-longevity firmware mode capping charge at 60-80%: BATTERY row should also show `X% (Plugged in)` (the same heuristic catches both the "PSU undersized" and "firmware limit" cases since they're indistinguishable from a single-snapshot SYSTEM_POWER_STATUS)
- Windows host with admin shell: BATTERY / CORES / GPU rows unchanged (no admin-gated behavior in v3.13.0)

### v3.13.1 — 2026-04-29

Release infrastructure fix (task #54). No runtime behavior changes — the binary is byte-identical to v3.13.0 modulo the version string. Verified on Windows 11 25H2 (build 26200.8246), unelevated user session:

- **Local build sanity** — `cargo fmt --check`, `cargo clippy --all-targets --workspace -- -D warnings`, `cargo test --workspace --all-targets`, `cargo build --release --workspace` all green. `target/release/tr300 --version` reports `3.13.1`. `--fast --json | head -5` parses; `--ascii` renders identically to v3.13.0.
- **`rust-toolchain.toml` doesn't break local development** — the file pins to `1.95`, the same minor that `Cargo.toml`'s `rust-version` already declares. Existing rustup-managed toolchains on the dev host satisfy the pin transparently.

Post-tag verification (run 25096833278 on tag `v3.13.1` → `086ef0a`):

- **`release.yml` succeeded across all 10 jobs** (vs 3/6 on v3.13.0):
  `aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `x86_64-pc-windows-msvc` all green; `plan` + `build-global-artifacts` + `host` + `announce` no longer skipped.
- **`gh release view v3.13.1 -R QubeTX/qube-machine-report`** returns the v3.13.1 release with **20 assets**:
  6 platform binaries as `.tar.xz` + matching `.sha256` (12 files), Windows `.zip` + `.sha256` + `.msi` + `.msi.sha256` (4 files), `source.tar.gz` + `.sha256`, `dist-manifest.json`, `sha256.sum`, `tr-300-installer.sh`, `tr-300-installer.ps1`. **First successful GitHub Release publication since v3.10.0** — the README installer one-liner is unblocked.
- **Fix-forward note:** The first attempt (`c2e6a65`) had only `channel = "1.95"` and CI's Format + Clippy jobs failed with `error: 'cargo-fmt' is not installed for the toolchain '1.95-x86_64-unknown-linux-gnu'`. Root cause: rustup processes a rust-toolchain.toml with only the default profile (rustc + cargo + rust-std) and ignores any action-level `components:` field passed to `dtolnay/rust-toolchain@stable`. Resolved by `086ef0a` which adds `components = ["rustfmt", "clippy"]` to the file. Documented inline in `CLAUDE.md` § "MSRV policy v3.13.1+" and the auto-memory release-process reference. Anyone touching rust-toolchain.toml in the future MUST keep the components list populated unless they also remove the rustfmt/clippy CI jobs.
