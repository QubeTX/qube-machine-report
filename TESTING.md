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
| **Windows 11 as Administrator** | Encryption shows full method + protection level; full RDP login history visible; **footer hint absent** | — |
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

### v3.11.0 — 2026-04-27

Windows accuracy + BitLocker (PR #4). Verified on Windows 11 25H2 (build 26200.8246), unelevated user session:

- **OS row** — was `Windows 11 (26200)`, now `Windows 11 25H2`. Registry-based detection working.
- **Kernel row** — was `26200`, now `26200.8246` (full build with UBR).
- **Last-login row** — was `Login tracking unavailable`, now real timestamp `Tue Apr 21 22:12` (matches uptime). WTSLogonTime returned 0 (console session quirk); fell back to GetTickCount64-derived boot time as designed.
- **CPU freq row** — still `1.4 GHz` on this host (machine is power-plan throttled at 1400 MHz; CPUID leaf 16h returns 0 on Meteor Lake; CallNtPowerInformation correctly reports 1400 MaxMhz). Implementation correct; will show higher values on machines with full performance power plan or older Intel chips where leaf 16h works.
- **Hypervisor row** — was `Hypervisor Present`, now `Bare Metal (Hyper-V/VBS)`. CPUID returned `Microsoft Hv` correctly; SMBIOS manufacturer disambiguated to "physical host with VBS active".
- **Encryption row** — absent on this user's unelevated session (Win32_EncryptableVolume returned access-denied as expected). Footer hint covers the gap. Will surface on Win11 Device Encryption laptops and admin sessions.
- **Architecture row** — `x86_64` (unchanged on x64 host running x64 binary; IsWow64Process2 implementation will activate on ARM64 hosts).
- **Footer hint** — still renders correctly with the BitLocker mention; on hosts where the encryption row appears non-admin, the hint is harmless extra info about RDP login history (also admin-gated per E.6, deferred to PR #5).
- Integration tests: 13 passed (1 new for JSON `encryption` key); library tests: 15 passed.

Pending verification (deferred or platform-locked):
- Windows 11 ARM64 host (C.2 IsWow64Process2 emulation annotation)
- Windows 11 with admin shell (BitLocker full method visible; E.6 RDP history would land in PR #5)
- Windows 11 with Device Encryption ON, unelevated (BitLocker row should appear)
- Windows 11 in a real Hyper-V VM (CPUID `Microsoft Hv` + Microsoft Corp manufacturer → `Hyper-V`, not `Bare Metal (Hyper-V/VBS)`)
- Windows running inside KVM / VMware / VirtualBox (CPUID-based hypervisor brand detection)
