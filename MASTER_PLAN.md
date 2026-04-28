# TR-300 Master Plan

> **Pickup-ready handoff document.** Read this first on a fresh session, fresh
> machine, or fresh contributor. Tells you exactly what's shipped, what's
> pending, why each decision was made, and how to keep going without
> re-litigating.

**Last updated:** 2026-04-27 (during PR #4 push)
**Current version:** 3.11.0
**Repo:** github.com/QubeTX/qube-machine-report
**Local source of truth:** `C:\Users\hey\Documents\GitHub\qube-machine-report` (Windows host where this work was authored)

---

## 0. Quick orientation for a new session

If you just pulled this repo and want to keep building, read these in order:

1. **`MASTER_PLAN.md`** (this file) — what's shipped, what's pending, where to pick up.
2. **`CLAUDE.md`** § "Development Workflow" — the canonical 7-phase cadence (plan → tasks → implement → docs → verify → commit/push → close out).
3. **`CLAUDE.md`** § "CI" — what `.github/workflows/ci.yml` enforces and how to reproduce it locally.
4. **`TESTING.md`** — the manual cross-platform verification matrix and per-version verification log.
5. **`CHANGELOG.md`** — every shipped change with task-ID cross-references in parens.

The auto-memory at `~/.claude/projects/C--Users-hey-Documents-GitHub-qube-machine-report/memory/` (only valid on the original Windows host) duplicates the workflow notes. Don't recreate it on other machines — `CLAUDE.md` is authoritative.

---

## 1. Status snapshot

### Shipped (committed + pushed to `origin/master`)

| Tag | Commit | Date | Summary |
|---|---|---|---|
| v3.10.0 | `58812cc` | 2026-04-27 | **Foundation** — elevation tier (`is_elevated`, `--no-elevation-hint`, footer hint, JSON `schema_version` + `elevated` + `elevation_unlocks_more`), comprehensive CI (`.github/workflows/ci.yml` with cross-platform fmt/clippy/test/build/speed/audit/dist-plan), Codex plugin config at project scope (`.claude/settings.json`), 7-phase development workflow codified in CLAUDE.md, new `TESTING.md` with manual matrix |
| v3.11.0 | `3a252df` | 2026-04-27 | **Windows accuracy + BitLocker** — registry-based OS detection (Win11 by build ≥ 22000, DisplayVersion + UBR), `IsWow64Process2` arch, `WTSQuerySessionInformation` last-login with `GetTickCount64` boot-time fallback, CPUID leaf 16h + `CallNtPowerInformation` CPU frequency, CPUID leaf `0x40000000` hypervisor brand with VBS disambiguation, BitLocker via `Win32_EncryptableVolume` in `MicrosoftVolumeEncryption` namespace |

**Tag status**: neither tag has been pushed yet. The plan is to wait for `ci.yml` to go green on real Linux / macOS / Windows runners against `3a252df`, then tag both releases:

```bash
git tag v3.10.0 58812cc
git tag v3.11.0 3a252df
git push --tags
```

The tag push triggers cargo-dist's `release.yml` and produces shell + PowerShell + MSI installers across the 6 target platforms.

### Live behavior changes already on master

After a fresh `git pull` and `cargo build --release`, you'll see (verified on Windows 11 25H2 build 26200.8246, unelevated user session):

- `OS` row: `Windows 11 25H2` (was `Windows 11 (26200)`)
- `KERNEL` row: `26200.8246` (was `26200`)
- `LAST LOGIN` row: real timestamp (was `Login tracking unavailable`)
- `HYPERVISOR` row: `Bare Metal (Hyper-V/VBS)` on Win11 with VBS active (was `Hypervisor Present`)
- New footer line below the table on Linux + Windows when unelevated:
  - Linux: `Run with sudo for motherboard, BIOS, and RAM slot details`
  - Windows: `Run as Administrator for BitLocker status and full login history`
  - macOS: no footer (no meaningful elevated unlocks)
- `--no-elevation-hint` flag suppresses the footer
- `--fast` mode never shows the footer (auto-run safety)
- JSON output gains `schema_version: 1`, `elevated`, `elevation_unlocks_more`, `session.encryption`

### Pending (not yet shipped)

- **PR #2** — macOS accuracy (12 sub-tasks) — Apple Silicon CPU brand/freq, Rosetta detection, scutil hostname/IP, AppleLocale, P/E core split, Mac model marketing name, vm_stat memory, battery health
- **PR #3** — Linux accuracy (15 sub-tasks) — systemd-resolved DNS priority, aarch64 CPU brand, in-process systemd-detect-virt port, POSIX locale precedence, power_supply iteration with type filter + battery health, `ip route get` for local IP, terminal env-var priority, last-login fallback chain, ZFS health, dmidecode-backed motherboard/BIOS/RAM (sudo only)
- **PR #4b** — deferred Windows accuracy (2 sub-tasks) — `GetBestInterfaceEx` + `GetAdaptersAddresses` (VPN-aware DNS+IP), Fast Startup uptime annotation
- **PR #5** — Windows polish (10 sub-tasks) — DXGI GPU enumeration, `GetLogicalProcessorInformationEx`, `GetSystemPowerStatus` battery + cycle count, PowerShell 7+ detection, terminal parent-process walk, batched-PowerShell fallback, drop COM/WMI from `--fast` hot path, admin-only RDP login history
- **PR #6** *(optional, deferred unless explicitly requested)* — `--security` flag adding TPM 2.0 + Secure Boot + FileVault + SELinux/AppArmor rows

### Recommended next steps (in order)

1. **Watch CI on `3a252df`**: https://github.com/QubeTX/qube-machine-report/actions — `ci.yml` is brand new, this is its second cross-platform run. If anything fails, fix-forward (don't amend; create a new commit).
2. **Tag both releases** once CI is green (commands above).
3. Pick the next PR. Recommendation:
   - **If validating on Mac** → PR #2 (macOS accuracy). The user's primary platforms are Windows + Apple Silicon; PR #2 touches the most-impactful M-series accuracy bugs (CPU brand "Apple M1" stale on M3/M4, CPU frequency = 0 GHz on Apple Silicon).
   - **If on Windows** → PR #4b (finish Windows network + Fast Startup) or PR #5 (Windows polish — speed wins by replacing remaining WMI calls with Win32 APIs).
   - **If on Linux** → PR #3.

---

## 1.5 What's already best-in-class (do not "fix")

Confirmed correct by research and verified in production. Don't waste effort re-investigating these unless a CHANGED upstream invalidates the source.

- **OS pretty name on Linux** — `/etc/os-release` PRETTY_NAME ([os-release(5)](https://man7.org/linux/man-pages/man5/os-release.5.html))
- **Linux load average** — `/proc/loadavg`
- **Linux uptime** — `/proc/uptime`
- **Linux memory available** — `/proc/meminfo` MemAvailable (kernel 3.14+)
- **Linux disk usage** — sysinfo's statvfs path
- **macOS DNS** — `scutil --dns` (resolv.conf on macOS is decorative — it explicitly says so in its own header)
- **macOS uptime** — `kern.boottime` sysctl (sysinfo uses it)
- **macOS sw_vers** — same data as `kern.osproductversion`
- **Windows hostname** — `GetComputerNameExW` (sysinfo uses it)
- **Windows memory** — `GlobalMemoryStatusEx` (sysinfo uses it)
- **Windows locale** — `GetUserDefaultLocaleName` (already in code)
- **SSH client IP detection** — `SSH_CLIENT` / `SSH_CONNECTION` env vars
- **Cores (physical + logical) on all platforms** — sysinfo handles SMT/HT correctly; Apple Silicon P/E cores need `hw.perflevel*` enrichment (M7) but the basic counts are right.
- **Battery presence/charge** — Linux `/sys/class/power_supply`, macOS `pmset`, Windows WMI `Win32_Battery` (PR #5 C.10 will swap Windows to the faster `GetSystemPowerStatus`).
- **macOS Activity Monitor formula** — `(active + wired + compressed) * page_size` matches sysinfo on most M-series; A.9 only swaps if a divergence is observed.

## 2. Hard constraints (non-negotiable, established by the user)

These shaped every decision and **must** be preserved by future PRs:

1. **Pager rejected.** Don't add a built-in pager / scrollback / `less` wrapper. Modern terminals all have native scrollback; auto-run depends on the prompt being free immediately after the table renders.
2. **`--fast` must stay sub-second.** Current Windows release median: ~308 ms. CI gate: < 1500 ms. New collectors that need subprocesses or > 50 ms must be full-mode-only.
3. **No admin/sudo in the default code path.** Auto-run runs as the user. Admin-only collectors must degrade silently (return `None`, let the elevation footer hint cover the gap). Never panic, prompt, or print errors to stderr.
4. **Report can't grow in line count.** Every PR's integration test must verify line count is unchanged. New rows are only allowed when *conditional* (battery enriched in place; ZFS Health only when zpool exists; ENCRYPTION row only when readable).
5. **Don't "fix" BTRFS subvolume / APFS container disk numbers.** sysinfo's reporting matches what the OS itself shows in Disk Utility / `df`. Changing the aggregation logic would regress against user expectations.

---

## 3. The full plan (consolidated from `~/.claude/plans/take-a-look-around-elegant-stonebraker.md`)

### Section 1 — macOS accuracy fixes (PR #2)

Research grounded in [Apple Developer Forums #652667](https://developer.apple.com/forums/thread/652667), [#664774](https://developer.apple.com/forums/thread/664774), [#671792](https://developer.apple.com/forums/thread/671792), [eclecticlight sysctl reference](https://eclecticlight.co/sysctl-information/), [eclecticlight Oct 2025 frequency table](https://eclecticlight.co/2025/10/28/updating-cpu-frequencies-for-apple-silicon-macs/), [psutil #1892](https://github.com/giampaolo/psutil/issues/1892), [cpufetch #139](https://github.com/Dr-Noob/cpufetch/issues/139).

| ID | Issue | Current source | Better source | File |
|---|---|---|---|---|
| M1 | CPU frequency = 0 GHz on Apple Silicon | sysinfo (returns 0 — `hw.cpufrequency` doesn't exist on M-series) | Chip-name → max-frequency lookup table keyed on `machdep.cpu.brand_string` | `cpu.rs`, `platform/macos.rs` |
| M2 | CPU brand stale on M3/M4 (older sysinfo) | sysinfo | `sysctl -n machdep.cpu.brand_string` direct | `cpu.rs`, `platform/macos.rs` |
| M3 | Architecture wrong under Rosetta 2 | `std::env::consts::ARCH` | Add `sysctl -n sysctl.proc_translated`; show `Apple Silicon (running x86_64 via Rosetta 2)` | `platform/macos.rs` |
| M4 | Hostname doesn't match System Settings | sysinfo `gethostname()` | `scutil --get ComputerName` | `os.rs`, `platform/macos.rs` |
| M5 | Local IP picks wrong interface on multi-homed/VPN | Hardcoded en0/en1/en2 | `scutil --nwi` for primary interface, then `ipconfig getifaddr` | `network.rs`, `platform/macos.rs` |
| M6 | Locale shows shell `LANG` instead of user's region | `LANG` first | `defaults read -g AppleLocale` first; strip `@rg=...` | `platform/macos.rs` |
| M7 | No P-core / E-core breakdown | sysinfo physical/logical only | `hw.perflevel0.physicalcpu` + `hw.perflevel1.physicalcpu`; render `12 cores (8P + 4E)` | `cpu.rs`, `platform/macos.rs` |
| M8 | Memory "used" doesn't match Activity Monitor | sysinfo's used | `host_statistics64` formula `(active + wired + compressed) * page_size`; only swap if sysinfo diverges | `memory.rs`, `platform/macos.rs` |

**Skip:** thermal pressure level (undocumented + Intel-only), serial number (PII risk), time-since-last-power-off (no API distinct from `kern.boottime`).

### Section 2 — Linux accuracy fixes (PR #3)

Research grounded in [systemd-resolved.service(8)](https://man7.org/linux/man-pages/man8/systemd-resolved.service.8.html), [systemd-detect-virt(1)](https://man7.org/linux/man-pages/man1/systemd-detect-virt.1.html), [POSIX locale precedence](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap08.html), [kernel sysfs-class-power](https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-class-power), [raspberrypi/linux #3991](https://github.com/raspberrypi/linux/issues/3991).

| ID | Issue | Better source | File |
|---|---|---|---|
| L1 | DNS reads stub `127.0.0.53` on systemd-resolved | `/run/systemd/resolve/resolv.conf` → `/run/NetworkManager/resolv.conf` → `/etc/resolv.conf` | `network.rs` |
| L2 | CPU brand empty on aarch64 | `/proc/cpuinfo` model name → `/sys/firmware/devicetree/base/model` → `CPU implementer`/`CPU part` decode | `cpu.rs`, `platform/linux.rs` |
| L3 | Hypervisor misses cloud + containers | Port `systemd-detect-virt` logic in-process: cpuid hypervisor flag → DMI sys_vendor → cgroup/`.dockerenv`/`.containerenv`/container env; covers kvm, qemu, vmware, hyperv, parallels, virtualbox, xen, wsl, docker, podman, lxc, systemd-nspawn, aws, azure, gcp | `platform/linux.rs` |
| L4 | Locale ignores `LC_ALL` and `LC_CTYPE` | POSIX precedence: `LC_ALL` > `LC_CTYPE` > `LANG` > `C` | `platform/linux.rs` |
| L5 | Battery hardcodes `BAT0`/`BAT1` | Iterate `/sys/class/power_supply/*` filtering on `type == "Battery"`; add health % via `energy_full / energy_full_design` | `platform/linux.rs` |
| L6 | Local IP wrong on multi-homed hosts | `ip route get 1.1.1.1` parse "src" field | `platform/linux.rs`, `network.rs` |
| L7 | Terminal detection ordering causes misses | Check `KITTY_WINDOW_ID`, `WEZTERM_PANE`, `GHOSTTY_RESOURCES_DIR`, `ALACRITTY_LOG`, `KONSOLE_VERSION`, `VTE_VERSION`, `FOOT_PID`, `TILIX_ID`, `WT_SESSION` first; `TERM_PROGRAM` second; ps-walk fallback | `platform/linux.rs` |
| L8 | WSL2 not detected | Check `/proc/sys/kernel/osrelease` for `microsoft-standard-WSL2` (folded into B.3) | `platform/linux.rs` |
| L9 | `last`/`lastlog` deprecated on glibc 2.40+ | `lastlog2` → `lastlog` → `last -F -1 -w` | `session.rs` |
| E.3 | Motherboard / BIOS only readable as root | When `is_elevated`: parse `dmidecode -s baseboard-{manufacturer,product-name}`, `bios-{vendor,version,release-date}`, `chassis-type`. Render in natural spots; full-mode + elevated only | `platform/linux.rs` |
| E.4 | RAM slot detail only readable as root | When `is_elevated` + full mode: `dmidecode -t memory`, render compact summary like `2x16GB DDR5-5600 SK Hynix` | `platform/linux.rs` |

**Container/musl caveats:** Alpine has no systemd, no `mokutil`, often no `lspci`. Logic must be in-process. Containers inherit host kernel/uptime/loadavg/memory — flag in JSON as `in_container: true` rather than "correcting".

### Section 3 — Windows accuracy fixes

**PR #4 (already shipped — see commit `3a252df`):** C.1 (registry OS), C.2 (IsWow64Process2 arch), C.3 (WTS last-login), C.6 (CPUID 16h + CallNtPowerInformation freq), C.7 (CPUID 0x40000000 hypervisor + VBS disambiguation), E.5 (BitLocker).

**PR #4b (deferred, pending):**

| ID | Fix | File |
|---|---|---|
| C.4 | DNS+IP via `GetBestInterfaceEx` + `GetAdaptersAddresses` (VPN-aware default-route detection); replaces WMI Win32_NetworkAdapterConfiguration | `platform/windows.rs:88-109` |
| C.5 | Fast Startup uptime annotation: registry `HiberbootEnabled` + `GetTickCount64` vs `LastBootUpTime`; append `(session: Xh)` suffix when divergent | `os.rs`, `platform/windows.rs` |

**PR #5 (Windows polish, pending):**

| ID | Fix | File |
|---|---|---|
| C.8 | GPU via DXGI `EnumAdapters1` (filter `DXGI_ADAPTER_FLAG_SOFTWARE` and vendor 0x1414 software adapter) | `platform/windows.rs` |
| C.9 | Cores via `GetLogicalProcessorInformationEx(RelationProcessorPackage, …)` | `cpu.rs`, `platform/windows.rs` |
| C.10 | Battery via `GetSystemPowerStatus` + `DeviceIoControl(\\.\Battery0)` for cycle count + design capacity | `platform/windows.rs` |
| C.11 | PowerShell 7+ detection via `HKLM\SOFTWARE\Microsoft\PowerShellCore\InstalledVersions\*` | `session.rs`, `platform/windows.rs` |
| C.12 | Terminal parent-process walk via `CreateToolhelp32Snapshot` + `Process32NextW` for conhost-only sessions | `platform/windows.rs` |
| C.13 | Batched PowerShell fallback into single JSON call (when WMI/COM init fails) | `platform/windows.rs:88-109` |
| C.14 | Drop COM/WMI init from `--fast` hot path once C.4/C.6/C.9/C.10 land | `platform/windows.rs` |
| E.6 | Admin-only full RDP login history via Security Event Log 4624 (filter `LogonType in [3, 7, 10, 11]`) | `platform/windows.rs` |

**Combined cold-start savings on full mode:** ~150–400 ms by replacing remaining WMI paths with Win32 APIs.

### Section 4 — Cross-platform reliability (mostly done in PR #1)

| ID | Status | Note |
|---|---|---|
| X1 | Pending | ZFS Health row: when `zpool` is on `$PATH`, run `zpool list -H -o health` and aggregate worst-of. Skip in `--fast`. Render only when zpool exists. (Folded into PR #3 B.11.) |
| X2 | Done in PR #1 | BTRFS/APFS volume-semantics note in CLAUDE.md so contributors don't "fix" what isn't broken. |
| X3 | Done in PR #1 | JSON `schema_version: 1` in every output. |

### Section 5 — Speed improvements

| ID | Optimization | Status | Saves |
|---|---|---|---|
| S1 | Linux: collapse 3 sequential `ps` calls into one `ps -e -o pid,ppid,comm` | Pending (PR #3 B.10) | ~20–30 ms |
| S2 | Windows: replace WMI paths with Win32 APIs, eliminate COM init from hot path | Partial (PR #4) — full elimination in PR #5 C.14 | ~150–400 ms cold |
| S3 | Windows: batched PowerShell fallback when WMI fails | Pending (PR #5 C.13) | ~150–250 ms (uncommon path) |
| S4 | macOS: already optimal, no action | N/A | — |

---

## 3.5 Skipped (researched and explicitly rejected)

These were investigated during the planning research and **deliberately not added**. If a future contributor proposes any of them, point them at this section to short-circuit a re-litigation.

- **Built-in pager / scrollback viewer** — modern terminals (Windows Terminal, iTerm2, Wezterm, Konsole, Alacritty, Ghostty) all have native scrollback. Auto-run depends on the prompt being free immediately. **Hard rejected by the user.**
- **Thermal sensors** — opaque on macOS (`machdep.xcpm.cpu_thermal_level` is undocumented and Intel-only), fragile on Linux hwmon (thermal zone naming inconsistent across boards), gone from Windows in modern drivers.
- **Top processes by CPU/RAM** — blows up the table layout, doesn't fit the 51-col TR-200 aesthetic.
- **NIC link speed** — ethtool may not be installed on Linux; signal-to-noise is low for a one-line report.
- **Disk SMART data** — requires elevated privileges on most platforms; smartctl + nvme-cli are platform-fragile.
- **Package count** — unportable; nothing comparable across pacman/apt/dnf/brew/winget/snap/flatpak.
- **Serial number** — privacy-sensitive PII; if a user pastes a TR-300 report into Discord or GitHub they leak their AppleCare-claimable serial. Skip unless gated behind an explicit `--include-serial` flag.
- **Pending reboot indicator (Windows)** — low signal-to-noise for the default report; could land in a future `--security` flag if requested.
- **DPI scaling** — no resolution row currently exists to attach to.
- **LUKS root detection via lsblk parsing** — breaks on ZFS-encryption, fscrypt, stacked setups.
- **CPU temperature** — thermal zone naming inconsistent across boards.
- **Battery time-remaining ETA** — `power_now` lags 30s after AC change; misleading.
- **Last-boot reason** — distro-specific, needs root.

## 4. Implementation task checklist (master)

Status legend: `[x]` done, `[ ]` pending, `[~]` deferred to a later PR than originally planned.

### PR #1 — Foundation scaffolding (✅ shipped as v3.10.0, commit `58812cc`)

- [x] **D.1** Add JSON `schema_version` field
- [x] **E.1** Add `is_elevated` detection (`tr_300::is_elevated()`, manual `IsUserAnAdmin` extern on Windows)
- [x] **E.2** Add `--no-elevation-hint` CLI flag
- [x] **E.7** Footer renderer + `platform_has_elevated_data()` helper
- [x] **E.8** JSON `elevated` + `elevation_unlocks_more` keys
- [x] **F.1** CHANGELOG.md v3.10.0 entry
- [x] **F.2** README.md scaffold update
- [x] **F.3** CLAUDE.md architecture notes
- [x] **F.4** Cargo.toml bump to 3.10.0
- [x] **F.5** Auto memory writes (project / feedback / reference)
- [x] **F.6** Create TESTING.md with manual matrix
- [x] **CI.1** `.github/workflows/ci.yml` cross-platform matrix
- [x] **CI.2** Fix assert_cmd deprecation in integration tests
- [x] **CI.3** Cargo-audit job
- [x] **CI.4** Verify `dist plan` as a gate
- [x] **CI.5** Speed regression check (`--fast` budget gate)
- [x] **CI.6** Document CI in README + CLAUDE.md + TESTING.md
- [x] Tests: footer logic + JSON schema baseline

### PR #2 — macOS accuracy (⏳ pending; needs Apple Silicon + Intel Mac for validation)

- [ ] **A.1** macOS CPU brand via `sysctl machdep.cpu.brand_string`
- [ ] **A.2** Apple Silicon CPU frequency lookup table (M1/M1Pro/Max/Ultra, M2 family, M3 family, M4 family)
- [ ] **A.3** Rosetta 2 architecture detection (`sysctl.proc_translated`)
- [ ] **A.4** `scutil --get ComputerName` for hostname
- [ ] **A.5** `scutil --nwi` for primary interface IP
- [ ] **A.6** Locale via `defaults read -g AppleLocale` first; strip `@rg=...`
- [ ] **A.7** P/E core breakdown via `hw.perflevel0/1.physicalcpu`
- [ ] **A.8** Mac model marketing-name lookup via SIMachineAttributes.plist
- [ ] **A.9** Memory verification + `vm_stat` fallback if sysinfo diverges
- [ ] **A.10** Battery health via `system_profiler SPPowerDataType -json`
- [ ] **F.1–F.5** macOS PR documentation block
- [ ] Tests: macOS unit + manual matrix

### PR #3 — Linux accuracy + dmidecode tier (⏳ pending; needs Linux distros + RPi for validation)

- [ ] **B.1** DNS priority chain: `/run/systemd/resolve/resolv.conf` → NetworkManager → `/etc/resolv.conf`
- [ ] **B.2** aarch64 CPU brand fallback chain
- [ ] **B.3** In-process `systemd-detect-virt` port (covers cloud + containers + WSL1/WSL2)
- [ ] **B.4** POSIX locale precedence
- [ ] **B.5** Iterate `/sys/class/power_supply/*` with type filter + battery health
- [ ] **B.6** Local IP via `ip route get`
- [ ] **B.7** Terminal env-var priority over ps-walk
- [ ] **B.8** WSL1/WSL2 detection (folded into B.3)
- [ ] **B.9** Last-login fallback chain (lastlog2 → lastlog → last -F -1 -w)
- [ ] **B.10** Single ps call for terminal detection (was 3) — speed win S1
- [ ] **B.11** ZFS Health when `zpool` present
- [ ] **E.3** Linux dmidecode motherboard/BIOS (sudo only)
- [ ] **E.4** Linux dmidecode RAM slot summary (sudo only)
- [ ] **F.1–F.5** Linux PR documentation block
- [ ] Tests: Linux unit + manual matrix

### PR #4 — Windows accuracy + BitLocker (✅ shipped as v3.11.0, commit `3a252df`)

- [x] **C.1** Windows OS from registry (Win11 detect by build ≥ 22000, DisplayVersion + UBR)
- [x] **C.2** Windows arch via `IsWow64Process2`
- [x] **C.3** Last-login via `WTSQuerySessionInformation` + boot-time fallback
- [~] **C.4** Windows DNS+IP via `GetBestInterfaceEx` + `GetAdaptersAddresses` — **deferred to PR #4b** (existing WMI path works; this is an accuracy refinement, not a fix)
- [~] **C.5** Fast Startup uptime detection — **deferred to PR #4b** (sysinfo uptime is acceptable today)
- [x] **C.6** CPU freq via CPUID leaf 16h + `CallNtPowerInformation`
- [x] **C.7** Hypervisor via CPUID `0x40000000` with VBS disambiguation
- [x] **E.5** Windows BitLocker collector (try non-admin, escalate)
- [x] **F.1–F.5** Windows accuracy PR documentation
- [x] Tests: Windows unit + manual matrix (user-validatable)

### PR #4b — Deferred Windows accuracy (⏳ pending; user can validate live on Windows)

- [ ] **C.4** DNS+IP via `GetBestInterfaceEx` + `GetAdaptersAddresses`
- [ ] **C.5** Fast Startup uptime annotation
- [ ] **F.1–F.5** PR #4b documentation block
- [ ] Tests: speed regression check (must show no regression vs PR #4 baseline)

### PR #5 — Windows polish (⏳ pending; user can validate live on Windows)

- [ ] **C.8** GPU via DXGI EnumAdapters1
- [ ] **C.9** Cores via `GetLogicalProcessorInformationEx`
- [ ] **C.10** Battery via `GetSystemPowerStatus` + DeviceIoControl
- [ ] **C.11** PowerShell 7+ detection
- [ ] **C.12** Terminal parent-process walk via Toolhelp
- [ ] **C.13** Batched-PowerShell fallback into single JSON call
- [ ] **C.14** Drop COM/WMI init from `--fast` hot path
- [ ] **E.6** Admin-only full RDP login history
- [ ] **F.1–F.5** PR #5 documentation block
- [ ] Tests: Windows polish unit + speed regression (target ~150ms+ improvement on `--fast` cold start)

### PR #6 — Optional `--security` flag (⏸ deferred unless explicitly requested)

- [ ] Add `--security` CLI flag
- [ ] macOS: `fdesetup status`, `csrutil status`
- [ ] Linux: `/sys/class/tpm/tpm0/tpm_version_major`, `/sys/firmware/efi/efivars/SecureBoot-*`, `/sys/kernel/security/lsm`, `/sys/fs/selinux/enforce`
- [ ] Windows: `Win32_Tpm` via CIM (no admin), registry `HKLM\SYSTEM\CurrentControlSet\Control\SecureBoot\State\UEFISecureBootEnabled`
- [ ] Tests + docs

---

## 4.5 Files modified / to-be-modified (full inventory)

Per the implementation plan. Items prefixed `[done]` were touched by PR #1 or #4; everything else is pending.

**Source files:**

- [done] `Cargo.toml` — version bumps; `windows`/`winapi` features as needed
- [done] `src/cli.rs` — new `--no-elevation-hint` flag; future flags will land here
- [done] `src/lib.rs` — `is_elevated()`, `platform_has_elevated_data()`, manual `IsUserAnAdmin` extern
- [done] `src/config.rs` — `no_elevation_hint` field on Config; builder method
- [done] `src/main.rs` — flag → config wiring
- [done] `src/report.rs` — `SCHEMA_VERSION`, `should_render_elevation_footer`, `render_elevation_footer`, JSON additions, BitLocker row, encryption JSON key
- [done] `src/collectors/mod.rs` — `is_elevated`/`encryption` fields on `SystemInfo`; wiring in `collect_with_mode`
- [done] `src/collectors/cpu.rs` — CPUID leaf 16h, `cpu_max_mhz_windows`, manual `CallNtPowerInformation` extern
- [done] `src/collectors/os.rs` — Windows registry override path
- [done] `src/collectors/session.rs` — `wts_query_session_connect_time`, `boot_time_local_string`, manual `WTSQuerySessionInformationW` extern
- [done] `src/collectors/platform/mod.rs` — `encryption` field on `PlatformInfo`
- [done] `src/collectors/platform/windows.rs` — `get_os_info_from_registry`, `cpuid_hypervisor_brand`, `map_hypervisor_vendor`, `get_bitlocker_status`, `format_bitlocker_status`, `bitlocker_method_name`, manual `IsWow64Process2`/`GetCurrentProcess` externs, `IMAGE_FILE_MACHINE_*` constants
- [done, stub] `src/collectors/platform/linux.rs` — `encryption: None` placeholder added; B.1–B.11 + E.3 + E.4 to land in PR #3
- [done, stub] `src/collectors/platform/macos.rs` — `encryption: None` placeholder added; A.1–A.10 to land in PR #2
- [ ] `src/collectors/network.rs` — pending macOS/Linux/Windows VPN-aware default-route detection (M5, L6, C.4)
- [ ] `src/collectors/disk.rs` — battery health additions per platform
- [ ] `src/collectors/memory.rs` — A.9 potential `vm_stat` fallback if sysinfo's used-memory diverges from Activity Monitor

**Documentation:**

- [done] `CHANGELOG.md` — v3.10.0 + v3.11.0 sections at top with task-ID cross-references
- [done] `CLAUDE.md` — § Development Workflow (the canonical 7-phase cadence) + § CI + § Windows accuracy patterns + § Elevation Tier + § JSON Schema Versioning + § Disk volume semantics
- [done] `README.md` — CI badge, flag table, Elevation Tier subsection, updated features list with hypervisor/last-login/encryption notes
- [done] `TESTING.md` — manual matrix + per-version verification log (v3.10.0 + v3.11.0 entries)
- [done] `MASTER_PLAN.md` — this file
- [ ] CHANGELOG/README/CLAUDE.md updates per future PR (F.1–F.3 of each PR's documentation block)

**CI / build:**

- [done] `.github/workflows/ci.yml` — fmt + clippy + cross-platform test/build/speed/audit/dist-plan
- [pre-existing, not hand-edited] `.github/workflows/release.yml` — auto-generated by cargo-dist v0.31.0; regenerate via `dist init` after `[workspace.metadata.dist]` changes
- [done, auto-generated] `man/tr300.1` — regenerated by `build.rs` via `clap_mangen` at build time

**Configuration:**

- [done] `.claude/settings.json` — `extraKnownMarketplaces.openai-codex` + `enabledPlugins.codex@openai-codex` (Codex plugin config at project scope)
- [done] `.gitignore` — `.claude/settings.local.json` (per-machine state)

**Auto memory (only valid on the original Windows host; not committed to repo):**

- `~/.claude/projects/C--Users-hey-Documents-GitHub-qube-machine-report/memory/`
  - [done] `MEMORY.md` — index
  - [done] `project_tr300_overview.md` — what TR-300 is, current version, deployment, constraints
  - [done] `feedback_tr300_constraints.md` — pager rejected, --fast must stay sub-second, no admin, etc.
  - [done] `feedback_tr300_workflow.md` — the 7-phase development cadence
  - [done] `reference_tr300_release_process.md` — release step checklist + CI mechanism
- The plan file at `~/.claude/plans/take-a-look-around-elegant-stonebraker.md` (full original plan, ~1500 lines with citations)

## 4.6 Testing strategy (full reference)

CI runs the gates (`.github/workflows/ci.yml`) automatically. Locally reproduce the same checks before pushing. Manual cross-platform matrix runs before tagging a release.

### Unit tests in the lib (T.U.* — run on every CI target)

- [done] **T.U.23** `elevation_footer_string_logic` (PR #1, in `src/report.rs`)
- [done] **schema_version_is_one** (PR #1)
- [done] **elevation_footer_uses_ansi_dim_when_colors_enabled** (PR #1)
- [done] **elevation_footer_string_is_empty_on_macos** (PR #1)
- [done] **elevation_footer_present_when_unelevated_full_no_optout** (PR #1)
- [done] **elevation_footer_skipped_when_user_opted_out** (PR #1)
- [done] **elevation_footer_skipped_in_fast_mode** (PR #1)
- [done] **elevation_footer_skipped_when_elevated** (PR #1)
- [ ] **T.U.1** `parse_resolvectl_status` — feed known fixture, assert DNS list (PR #3 B.1)
- [ ] **T.U.2** `parse_route_get_default_macos` (PR #2 A.5)
- [ ] **T.U.3** `parse_scutil_nwi` (PR #2 A.5)
- [ ] **T.U.4** `parse_scutil_dns` — regression guard
- [ ] **T.U.5** `parse_devicetree_model` (PR #3 B.2)
- [ ] **T.U.6** `decode_arm_implementer_part` (PR #3 B.2)
- [ ] **T.U.7** `parse_zpool_health` (PR #3 B.11)
- [ ] **T.U.8** `detect_wsl_from_osrelease` (PR #3 B.3 + B.8)
- [ ] **T.U.9** `detect_container_from_proc1` (PR #3 B.3)
- [ ] **T.U.10** `cpuid_hypervisor_vendor_to_name` — Windows currently inlined; could extract for unit testability (PR #4 C.7 — defer cleanup)
- [ ] **T.U.11** `apple_silicon_freq_lookup` (PR #2 A.2)
- [ ] **T.U.12** `posix_locale_precedence` (PR #3 B.4)
- [ ] **T.U.13** `battery_health_ratio` (PR #2 A.10, PR #3 B.5, PR #5 C.10)
- [ ] **T.U.14** `parse_event_log_27_xml` (PR #4b C.5)
- [ ] **T.U.15** `parse_si_machine_attributes_plist` (PR #2 A.8)
- [ ] **T.U.16** `parse_apple_locale_strip_rg` (PR #2 A.6)
- [ ] **T.U.17** `terminal_env_var_priority` (PR #3 B.7)
- [ ] **T.U.18** `parse_hw_perflevel_cores` (PR #2 A.7)
- [ ] **T.U.19** `parse_dmidecode_baseboard_output` (PR #3 E.3)
- [ ] **T.U.20** `parse_dmidecode_memory_table` (PR #3 E.4)
- [ ] **T.U.21** `parse_bitlocker_protection_status` — Windows inlined; could extract (PR #4 E.5 — defer cleanup)
- [ ] **T.U.22** `parse_event_4624_xml_logon_types` (PR #5 E.6)

### Integration tests (`tests/integration.rs`, T.I.*)

- [done] **T.I.1** `--json` schema regression — `schema_version: 1` baseline (PR #1)
- [done] **T.I.2** `--fast --json` produces same key set as `--json` modulo nullables — implicitly via field presence checks; full key-set diff TODO
- [done] **T.I.3** `--ascii` output line count baseline — currently informal (captured in TESTING.md); **TODO**: formalize as a snapshot test
- [done] **T.I.4** `--ascii` structural snapshot (border lines / headers) — currently informal; **TODO**: formalize
- [done] **T.I.5** Speed regression: `tr300 --fast` < 1500 ms median (CI gate); local < 800 ms target — **enforced by `.github/workflows/ci.yml`**
- [done] **T.I.6** `--json | jq .` parses without error on all platforms — implicit via `cargo test`'s integration test for `--json`
- [done] **test_json_includes_schema_version** (PR #1)
- [done] **test_json_includes_elevation_keys** (PR #1)
- [done] **test_no_elevation_hint_flag_accepted** (PR #1)
- [done] **test_fast_mode_no_elevation_footer** (PR #1)
- [done] **test_json_includes_encryption_key** (PR #4)

### Output stability gates (T.S.*)

- [done] **T.S.1** Line-count baseline captured (39 lines for `--ascii` on Windows unelevated full mode); enforce no-growth in future PRs
- [done] **T.S.2** Speed baseline captured (~308 ms median for `--fast` on Windows release build); regression tested in CI

### Manual cross-platform matrix (in `TESTING.md`)

Tracks "Last verified" per row. Each PR appends a `### vX.Y.Z — YYYY-MM-DD` section under "Per-release verification log". Already populated for v3.10.0 and v3.11.0 (Windows 11 25H2 build 26200.8246, unelevated user session). Pending hardware: macOS Intel + Apple Silicon M1/M3/M4, Ubuntu 22.04, Debian 12, Fedora, Arch, Alpine in Docker, Raspberry Pi 4 aarch64, AWS EC2 (Graviton + Intel), WSL2, Win10 (no Fast Startup), Win11 ARM64, Win11 with admin shell.

## 4.7 Phasing & sequencing (the canonical PR order)

Land in this order. PR #1 unblocks every later PR. PR #4 already shipped, but C.4 + C.5 are deferred to PR #4b. Each PR is one commit (using the existing `release: vX.Y.Z - <summary>` convention).

| PR | Status | Commit | Purpose |
|---|---|---|---|
| 1 | ✅ shipped | `58812cc` (v3.10.0) | Foundation: elevation tier scaffolding + CI + Codex config + workflow doc |
| 2 | ⏳ pending | — | macOS accuracy (A.1–A.10) + macOS PR docs (F.1–F.5) |
| 3 | ⏳ pending | — | Linux accuracy (B.1–B.11) + dmidecode tier (E.3–E.4) + Linux PR docs |
| 4 | ✅ shipped | `3a252df` (v3.11.0) | Windows accuracy (C.1, C.2, C.3, C.6, C.7) + BitLocker (E.5) + Windows PR docs |
| 4b | ⏳ pending | — | Deferred Windows accuracy (C.4 + C.5) |
| 5 | ⏳ pending | — | Windows polish (C.8–C.14) + admin RDP login history (E.6) |
| 6 | ⏸ optional | — | `--security` flag with TPM + Secure Boot + FileVault + SELinux/AppArmor |

Within each PR, sub-tasks can be tackled in any order that compiles. Per-PR documentation block (F.1–F.6) runs at the end before commit. Verification runs before the push.

## 4.8 Codex plugin config (committed at project scope)

The repo includes `.claude/settings.json` so any session that clones the repo gets the same Codex review subagent without manual setup:

```json
{
  "extraKnownMarketplaces": {
    "openai-codex": {
      "source": {
        "source": "github",
        "repo": "openai/codex-plugin-cc"
      }
    }
  },
  "enabledPlugins": {
    "codex@openai-codex": true
  }
}
```

`.claude/settings.local.json` (per-machine state like `outputStyle`) is gitignored.

When a teammate or future-you on a fresh machine first opens the folder, Claude Code will prompt to trust the marketplace — accept it. Then the `codex:codex-rescue` subagent is available via the Agent tool (not via `/codex:rescue` slash command nor `Skill(codex:rescue)` — those don't work; only `Agent(subagent_type: "codex:codex-rescue")` does).

**Codex review timing**: dispatch *after* the PR exists on GitHub (Codex's `gh pr diff` path needs a real PR number). On `master`-direct workflows like this repo, dispatch Codex pre-push against the local diff if needed, and accept its review before tagging the release.

## 4.9 JSON schema versioning policy

`report::SCHEMA_VERSION` is currently `1`. Bump rules:

- **Bump on breaking changes**: rename, remove, or change-type any existing key.
- **Do NOT bump on additive changes**: new keys (e.g., adding `cpu.p_cores`/`cpu.e_cores` in PR #2) don't bump the version.

Top-level keys in the JSON output:

- `schema_version: u32` — added in v3.10.0 (currently 1)
- `elevated: bool` — added in v3.10.0
- `elevation_unlocks_more: bool` — added in v3.10.0; true only when platform has elevated-only data AND user is unelevated
- `os: { name, version, kernel, architecture }` — pre-existing
- `network: { hostname, machine_ip?, client_ip?, dns_servers[] }` — pre-existing
- `cpu: { processor, cores, sockets?, hypervisor?, frequency_ghz, load_1m?, load_5m?, load_15m?, gpus[] }` — pre-existing
- `disk: { used_bytes, total_bytes, percent }` — pre-existing
- `memory: { used_bytes, total_bytes, percent }` — pre-existing
- `session: { username, last_login?, uptime_seconds, shell?, terminal?, locale?, battery?, encryption? }` — `encryption` added in v3.11.0

Nullable fields (intentional `null` in JSON) per platform:

- `network.machine_ip` — null when offline
- `network.client_ip` — null when not on SSH (no `SSH_CLIENT` / `SSH_CONNECTION`)
- `cpu.sockets` — null in `--fast` mode (skipped)
- `cpu.hypervisor` — null in `--fast` mode unless platform sets it cheaply
- `cpu.load_1m` / `load_5m` / `load_15m` — null on Windows in `--fast` mode (the 200ms sleep is skipped)
- `session.last_login` — null in `--fast` mode on Windows; otherwise filled
- `session.shell` / `session.terminal` / `session.locale` / `session.battery` — null when unavailable on the platform or skipped in `--fast`
- `session.encryption` — null on macOS + Linux until A/B blocks land; null on Windows when query returns access-denied

## 5. The 7-phase development workflow (canonical — see CLAUDE.md § Development Workflow)

Every non-trivial change follows this cadence. Trivial fixes (typo, single-line patch) can skip phases 1–2 but **never** skip phase 5 (verification) or phase 6 (commit/push gates).

1. **Plan in plan mode.** Read-only, write to `~/.claude/plans/<name>.md`. Spawn parallel `Explore` agents for codebase reconnaissance and parallel `general-purpose` agents (`model: opus`) for authoritative web research (Apple docs, Microsoft Learn, kernel.org, systemd, freedesktop). Cite every source. End with `ExitPlanMode`.
2. **Task tracking upfront.** `TaskCreate` PR-level parents AND every sub-task verbatim from the plan. `TaskUpdate` to `in_progress` *before* starting and `completed` *immediately* on finish — never batched.
3. **Implement one PR at a time.** Read every file before editing. `cargo check` after each meaningful change. After each PR completes, run the full local gate.
4. **Per-PR documentation block (F.1–F.6).** CHANGELOG entry with task IDs in parens · README updates · CLAUDE.md arch notes with citations · Cargo.toml version bump · auto memory updates · TESTING.md verification log entry.
5. **Verification + Codex review.** Re-run the full local gate. For non-trivial PRs, dispatch `Agent({subagent_type: "codex:codex-rescue"})` for an independent review. **Codex's `gh pr diff` path needs the PR to actually exist** — open the PR first, then ask Codex to review.
6. **Commit + push gates.** Local commit → `git-master` agent (no ci-tester). Push to remote → `ci-tester` agent FIRST; if `[FAIL]`, fix root cause (no `--no-verify`); once `[PASS]`, hand off to `git-master` for the push. Tag a release → bump version in F.4, commit + push commits, **wait for CI to go green**, then `git tag vX.Y.Z && git push --tags`.
7. **Close out.** Mark the parent PR task `completed`. Move to next PR. Deferred PRs only run on explicit request.

---

## 6. Reproducing the local gate

```bash
cargo fmt -- --check
cargo clippy --all-targets --workspace -- -D warnings
cargo test --workspace --all-targets
cargo build --release --workspace
./target/release/tr300 --version
./target/release/tr300 --fast --json | head -5
./target/release/tr300 --ascii          # visual smoke test
```

Plus on each platform:

```bash
# 5-run --fast median check (the speed gate)
for i in 1 2 3 4 5; do time ./target/release/tr300 --fast > /dev/null; done
```

CI runs the same checks across Linux, macOS ARM, and Windows. Intel macOS x86_64 was dropped from CI on 2026-04-28 because `macos-13` runner capacity has effectively retired (see `CLAUDE.md` § _Intel macOS coverage policy_); cargo-dist still builds the Intel binary at tag time. Local fmt/clippy/test only validates the host platform; cross-platform issues surface on remote CI runs.

---

## 7. Known limitations / honest notes

- **CPU FREQ on Intel Meteor Lake / Lunar Lake / Arrow Lake** (Core Ultra series): CPUID leaf 16h returns 0 because Intel zeroed it in microcode. `CallNtPowerInformation` reflects the active power-plan ceiling, which on these chips is honest about throttling. The user's machine (Core Ultra 7 155H) shows 1.4 GHz because the active power plan caps it at 1400 MHz — the report is correct, the machine is throttled.
- **Last login on local console sessions**: WTSConnectTime / WTSLogonTime return 0 for the local console session on most modern Windows installs (auto-login + Fast Startup mask the actual logon timestamp). The fallback derives the time from `GetTickCount64` (boot time), which is the meaningful answer for "since when have you been logged in".
- **Hyper-V detection on Win11 with VBS**: CPUID `Microsoft Hv` doesn't necessarily mean a VM — Win11 with Virtualization-Based Security runs the kernel on top of a thin Hyper-V layer. The code disambiguates via SMBIOS manufacturer (real Hyper-V VMs always have "Microsoft Corporation"; physical hosts have OEM strings), reporting `Bare Metal (Hyper-V/VBS)` for the latter.
- **BitLocker without admin**: query may return access-denied on older Win10 / domain-joined configurations. The code returns `None` gracefully and the elevation footer hint covers the gap. On Win11 Device Encryption laptops, the query usually succeeds non-admin.

---

## 7.5 Verification: when is a PR "done"?

A PR is done when ALL of these are true:

1. `cargo fmt -- --check` clean
2. `cargo clippy --all-targets --workspace -- -D warnings` clean
3. `cargo test --workspace` — every test passes (no `--ignore`d, no flaky)
4. `cargo build --release --workspace` produces a working binary on the host platform
5. `tr300 --version` reports the bumped version
6. `tr300 --fast --json | jq .` parses
7. `tr300 --ascii` visually correct
8. CI on GitHub Actions is green across all 3 matrix legs (Linux glibc, macOS ARM, Windows). Intel macOS coverage moved to tag-time-only via cargo-dist (see `CLAUDE.md` § _Intel macOS coverage policy_).
9. The `--fast` median wall-clock on the host platform did not regress > 100 ms vs the prior commit
10. `report --ascii` line count did not grow (verified in `tests/integration.rs`)
11. The user was able to confirm the changed field shows the right value on their actual machine for at least one of: macOS Apple Silicon, Win11, Linux

## 7.6 Authoritative citations (full reference index)

When in doubt about an API or behavior, these are the trusted sources used during the planning research. Don't reach for blog posts or Stack Overflow answers older than ~2 years for these topics.

### Apple

- [Apple Developer Forums #652667](https://developer.apple.com/forums/thread/652667) — detecting Apple Silicon under Rosetta
- [Apple Developer Forums #664774](https://developer.apple.com/forums/thread/664774) — P/E core counts via `hw.perflevel*`
- [Apple Developer Forums #671792](https://developer.apple.com/forums/thread/671792) — Apple Silicon CPU frequency (`hw.cpufrequency` removed)
- [Apple SCDNSConfiguration docs](https://developer.apple.com/library/archive/documentation/Networking/Conceptual/SystemConfigFrameworks/SC_DNSConfiguration/SC_DNSConfiguration.html) — why `scutil --dns` beats resolv.conf on macOS
- [Apple vm_statistics64](https://developer.apple.com/documentation/kernel/vm_statistics64_data_t) — memory formula
- [Apple Activity Monitor docs](https://support.apple.com/guide/activity-monitor/view-memory-usage-actmntr1004/mac) — what "Memory Used" means
- [Apple Support 102888](https://support.apple.com/en-us/102888) — battery cycle count
- [Apple Support 102767](https://support.apple.com/en-us/102767) — IOPlatformSerialNumber privacy
- [Apple Rosetta security guide](https://support.apple.com/guide/security/rosetta-2-on-a-mac-with-apple-silicon-secebb113be1/web)

### Microsoft Learn

- [IsWow64Process2](https://learn.microsoft.com/en-us/windows/win32/api/wow64apiset/nf-wow64apiset-iswow64process2)
- [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex)
- [CallNtPowerInformation](https://learn.microsoft.com/en-us/windows/win32/api/powerbase/nf-powerbase-callntpowerinformation)
- [GlobalMemoryStatusEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-globalmemorystatusex)
- [GetAdaptersAddresses](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses)
- [GetSystemPowerStatus](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getsystempowerstatus)
- [WTSQuerySessionInformationW](https://learn.microsoft.com/en-us/windows/win32/api/wtsapi32/nf-wtsapi32-wtsquerysessioninformationw)
- [Event 4624](https://learn.microsoft.com/en-us/previous-versions/windows/it-pro/windows-10/security/threat-protection/auditing/event-4624)
- [Win32_Tpm](https://learn.microsoft.com/en-us/windows/win32/secprov/win32-tpm)
- [Win32_EncryptableVolume.GetEncryptionMethod](https://learn.microsoft.com/en-us/windows/win32/secprov/getencryptionmethod-win32-encryptablevolume)
- [Confirm-SecureBootUEFI](https://learn.microsoft.com/en-us/powershell/module/secureboot/confirm-securebootuefi)
- [HKLM CurrentVersion key](https://learn.microsoft.com/en-us/windows/win32/com/hkey-local-machine-software-microsoft-windows-nt-currentversion)
- [Microsoft Q&A: Win11 ProductName still says Windows 10](https://learn.microsoft.com/en-us/answers/questions/555857/windows-11-product-name-in-registry)
- [Microsoft Q&A: Fast Startup boot time issue](https://learn.microsoft.com/en-us/answers/questions/1443763/how-to-get-oss-start-time-when-fast-startup-mode-i)
- [DXGI_ADAPTER_DESC1](https://learn.microsoft.com/en-us/windows/win32/api/dxgi/ns-dxgi-dxgi_adapter_desc1)

### Linux / kernel / freedesktop / systemd

- [os-release(5)](https://man7.org/linux/man-pages/man5/os-release.5.html)
- [systemd-resolved.service(8)](https://man7.org/linux/man-pages/man8/systemd-resolved.service.8.html)
- [systemd-detect-virt(1)](https://man7.org/linux/man-pages/man1/systemd-detect-virt.1.html)
- [proc_loadavg(5)](https://man7.org/linux/man-pages/man5/proc_loadavg.5.html)
- [proc_uptime(5)](https://man7.org/linux/man-pages/man5/proc_uptime.5.html)
- [proc_meminfo(5)](https://man7.org/linux/man-pages/man5/proc_meminfo.5.html)
- [Kernel sysfs-class-power](https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-class-power)
- [Kernel cgroup-v2 doc](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html)
- [POSIX locale precedence (Open Group Base Spec §8.2)](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap08.html)

### Other primary sources

- [eclecticlight sysctl reference](https://eclecticlight.co/sysctl-information/) — Apple Silicon sysctl values
- [eclecticlight Oct 2025 frequency table](https://eclecticlight.co/2025/10/28/updating-cpu-frequencies-for-apple-silicon-macs/) — M1/M2/M3/M4 max frequencies
- [eclecticlight: Why Big Sur won't stumble over version numbers](https://eclecticlight.co/2020/07/06/why-big-sur-wont-stumble-over-version-numbers/) — `SYSTEM_VERSION_COMPAT` caveat
- [psutil #1892](https://github.com/giampaolo/psutil/issues/1892) — Apple Silicon CPU freq
- [psutil #1277](https://github.com/giampaolo/psutil/issues/1277) — macOS memory formula
- [cpufetch #139](https://github.com/Dr-Noob/cpufetch/issues/139) — Apple Silicon CPU brand
- [gopsutil PR #999](https://github.com/shirou/gopsutil/pull/999) — Apple Silicon freq fallback
- [scriptingosx — Mac marketing name](https://scriptingosx.com/2017/11/get-the-marketing-name-for-a-mac/)
- [raspberrypi/linux #3991](https://github.com/raspberrypi/linux/issues/3991) — aarch64 /proc/cpuinfo "model name" missing
- [Ubuntu LP#1919478](https://bugs.launchpad.net/ubuntu/+source/linux-raspi/+bug/1919478) — same issue on Ubuntu/RPi
- [LWN: CPUID hypervisor detection](https://lwn.net/Articles/301888/)
- [OSDev wiki: CPUID for hypervisor detection](https://wiki.osdev.org/CPUID#CPUID_for_hypervisor_detection)
- [Intel SDM Vol. 2A, CPUID Leaf 16H](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html) — Processor Frequency Information
- [thewayeye.net — primary interface under VPN](https://thewayeye.net/posts/underlying-physical-network-interface-vpn/)
- [happymacadmin — pmset battery](https://happymacadmin.wordpress.com/2024/10/10/read-a-macs-battery-health-with-the-pmset-utility/)
- [osxdaily Jan 2024 — battery cycle count CLI](https://osxdaily.com/2024/01/03/how-to-check-battery-capacity-cycle-count-from-command-line-on-mac/)
- [alansiu.net — AppleLocale](https://www.alansiu.net/2024/06/01/getting-the-macos-selected-region-via-command-line/)
- [Tabby #702](https://github.com/Eugeny/tabby/issues/702) — terminal env var
- [Warp #6990](https://github.com/warpdotdev/warp/issues/6990)
- [Ghostty discussions #7633](https://github.com/ghostty-org/ghostty/discussions/7633)

### Out-of-band research files (NOT committed; on the original Windows host only)

These are large research dumps from the planning phase. Useful as reference but not authoritative — the inline citations above are what matters.

- `~/.claude/plans/take-a-look-around-elegant-stonebraker.md` — full original plan with sections per platform, sub-task IDs, files to modify, testing strategy, phasing
- `~/.claude/plans/take-a-look-around-elegant-stonebraker-agent-a331823be3c6ba4a3.md` — Linux research dump
- `~/.claude/plans/take-a-look-around-elegant-stonebraker-agent-a93ada108401bdab8.md` — Windows research dump

If you need to reproduce the original research from scratch on a different machine, dispatch parallel `general-purpose` agents (model: `opus`) with WebFetch / WebSearch / Firecrawl / Perplexity access against per-platform briefs. The original briefs are in CLAUDE.md § Development Workflow phase 1 step 3.

## 8. Pickup instructions for a fresh session

1. `git pull origin master` to get the latest commits (`58812cc` and `3a252df` should both be present).
2. Read this file (`MASTER_PLAN.md`) end-to-end.
3. Read `CLAUDE.md` § "Development Workflow" and § "CI".
4. Run the local gate (Section 6 above) — confirm it's green on your machine.
5. Check `gh run list --workflow=ci.yml --limit=3` (or the Actions tab) — confirm CI on `3a252df` has gone green.
6. If not green, fix-forward (don't amend). If green and tags haven't been pushed: `git tag v3.10.0 58812cc && git tag v3.11.0 3a252df && git push --tags`.
7. Pick the next PR (Section 1 "Recommended next steps") and follow the 7-phase workflow (Section 5).
8. Update this file's Section 1 status snapshot when you ship a PR. Append to TESTING.md verification log too.

If you're on a Mac, prioritize PR #2. If you're on Linux, PR #3. If you're on Windows, PR #4b or PR #5.
