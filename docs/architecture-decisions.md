# TR-300 Architecture Decisions

> Long-form rationale for the load-bearing technical choices in TR-300.
> Day-to-day editing rules live in [`../CLAUDE.md`](../CLAUDE.md); this file
> exists for the **why** — when a contributor is about to undo a decision and
> needs the original reasoning, the rejected alternatives, and the failure mode
> the current code prevents.
>
> Each section is dated by the version that introduced (or substantially
> revised) the decision. Verbatim moves from CLAUDE.md preserved word-for-word
> so `git blame` history continues to make sense.

## Table of contents

- [Toolchain & release](#toolchain--release)
  - [MSRV policy (v3.11.1+, addendum v3.13.1)](#msrv-policy-v3111-addendum-v3131)
  - [Self-update auto-rustup (v3.11.1+)](#self-update-auto-rustup-v3111)
  - [Intel macOS coverage policy (v3.11.2+)](#intel-macos-coverage-policy-v3112)
- [Windows accuracy patterns](#windows-accuracy-patterns)
  - [v3.11.0+ — registry OS, IsWow64Process2, WTS last-login, CPUID hypervisor, BitLocker](#v3110--registry-os-iswow64process2-wts-last-login-cpuid-hypervisor-bitlocker)
  - [v3.12.0+ — VPN-aware default-route, Fast Startup uptime annotation](#v3120--vpn-aware-default-route-fast-startup-uptime-annotation)
  - [v3.13.0+ — 5-state battery, native cores, GPU registry-prefer, PSCore detection, terminal walk](#v3130--5-state-battery-native-cores-gpu-registry-prefer-pscore-detection-terminal-walk)
  - [v3.14.4+ — Windows install execution-policy preflight](#v3144--windows-install-execution-policy-preflight)
  - [v3.14.5+ — Windows install error advisor](#v3145--windows-install-error-advisor)
  - [Windows distribution model (v3.15.0+)](#windows-distribution-model-v3150)
  - [v3.15.1 addendum — Why corporate.wxs lives in `wix-corporate/`, not `wix/`](#v3151-addendum--why-corporatewxs-lives-in-wix-corporate-not-wix)
- [Install / update safety primitives (v3.15.2+)](#install--update-safety-primitives-v3152)
  - [Atomic rc-file writes](#atomic-rc-file-writes)
  - [Marker-balance pre-check](#marker-balance-pre-check)
  - [SHA256 verification of downloaded installers](#sha256-verification-of-downloaded-installers)
  - [Post-install version verification](#post-install-version-verification)
  - [WMI hard-timeout pattern](#wmi-hard-timeout-pattern)
  - [Windows self-EXE delete via detached cleanup](#windows-self-exe-delete-via-detached-cleanup)

---

## Toolchain & release

### MSRV policy (v3.11.1+, addendum v3.13.1)

`rust-version` is pinned in `Cargo.toml` and tracks the GitHub Actions
`stable` toolchain. As of 3.11.1 it's `1.95` because `std::arch::x86::__cpuid`
and `std::arch::x86_64::__cpuid` were reclassified as safe-to-call in
Rust 1.95 (no safety preconditions on x86/x86_64 — CPUID is universally
available), which made our `unsafe { __cpuid(_) }` wrappers in
`src/collectors/cpu.rs` and `src/collectors/platform/windows.rs` trip the
`unused_unsafe` lint. Under our `-D warnings` policy that's a hard build
error. Bump `rust-version` whenever a new stable lint or stdlib change
forces source edits — and at the same release pin so that users running
older toolchains hit cargo's MSRV check, not E0133s deep in collector
modules.

*Why pin MSRV instead of supporting older Rust via shims:* there are three
realistic alternatives, and we considered each.

1. **`#[allow(unused_unsafe)]` on every `unsafe { __cpuid(_) }` block.**
   Compiles on both old and new toolchains. *Rejected* because the `allow`
   is permanent — once added, even Rust toolchains where the lint is
   correct (i.e. the unsafe block really is necessary because someone
   added a genuinely-unsafe call inside it later) will silently swallow
   the warning and we'd miss real safety regressions. It also bloats every
   CPUID callsite with attribute noise that has to be re-justified at
   review time, and it propagates: every future stable lint we want to
   straddle adds another permanent `allow`. Tech debt that compounds.

2. **`#[cfg(rustc_version)]` ladders to gate per Rust version.** Requires
   pulling in `rustversion` or `rustc_version` build-script crates, adds a
   build-time fingerprint to every release, and means our source has two
   parallel implementations of the same logic — one with `unsafe`, one
   without — that have to be kept in lockstep. *Rejected* as
   over-engineering for a tool whose CI deliberately uses
   `dtolnay/rust-toolchain@stable` and ships from a single toolchain.

3. **Pin MSRV to the CI toolchain (this approach).** Cargo's existing
   `rust-version` field already enforces this without any source-level
   shims. Older toolchains get `error: package tr300@3.11.1 cannot be
   built because it requires rustc 1.95.0 or newer, while the currently
   active rustc version is 1.94.0` — clear, actionable, and points at
   exactly the right knob to fix. Combined with auto-rustup in
   `--update`, users on rustup-managed toolchains never see the error at
   all because `tr300 --update` brings their stable forward in lockstep
   with the MSRV pin. Users on distro-managed toolchains see the clear
   error and can update on their own schedule.

The combination — pin in `Cargo.toml`, auto-rustup in `--update`, README
mentions `rustup update stable` ahead of `cargo install tr300` — gives us
a coherent toolchain story across all three install paths (binary
installer, fresh `cargo install`, self-update) without source-level
compatibility shims.

**v3.13.1 addendum — `rust-toolchain.toml` is the fourth leg.** `Cargo.toml`'s
`rust-version` declaration alone is not enough to keep `release.yml` green.
The release workflow is auto-generated by cargo-dist v0.31.0 (see
`.github/workflows/release.yml` line 1: `# This file was autogenerated by
dist`); its only Rust setup is downloading the `dist` binary itself and
immediately invoking `dist build`. There is no `rustup install` step.
Each `build-local-artifacts` job uses whatever rustc the
GitHub-hosted runner image happens to have pre-installed, which on
`ubuntu-22.04`, `ubuntu-22.04-arm`, and `windows-2022` is currently 1.94.1
— below MSRV 1.95. The result was that v3.10.0 / v3.12.0 / v3.13.0 all
tagged but produced no GitHub Release (3/6 build-local-artifacts jobs
failed; cargo-dist is all-or-nothing).

The fix is `rust-toolchain.toml` at repo root with `[toolchain] channel =
"1.95"` AND `components = ["rustfmt", "clippy"]`. Rustup is pre-installed
on every GitHub-hosted runner image, and when cargo runs from a workspace
containing `rust-toolchain.toml`, the rustup proxy auto-installs the
pinned channel before delegating to cargo. This works in `release.yml`
(which never invokes rustup directly), in `ci.yml` (which still calls
`dtolnay/rust-toolchain@stable` but the `rust-toolchain.toml` override
wins — same toolchain in CI as in releases, single source of truth), and
locally for any contributor who clones fresh. See [rustup overrides](https://rust-lang.github.io/rustup/overrides.html)
for the precedence rules.

The `components = ["rustfmt", "clippy"]` half is non-obvious but
load-bearing: when rustup processes a `rust-toolchain.toml` that does
not list components, it installs only the default profile (rustc, cargo,
rust-std) and *ignores* the `components:` field passed to action-level
toolchain installers like `dtolnay/rust-toolchain@stable`. The result is
that the `Format` and `Clippy` CI jobs fail with `error: 'cargo-fmt' is
not installed for the toolchain '1.95-x86_64-unknown-linux-gnu'` and
their clippy equivalent. Listing them in the file is the canonical way
to make rustup install them alongside the channel. Release.yml runners
get the extra ~few MB download too, which is harmless.

The MSRV is now expressed in **two** places that must move together:
`Cargo.toml`'s `rust-version` (the cargo-side declaration that produces
the clear `error: package tr300@X.Y.Z cannot be built because it
requires rustc N.M ...` message for users on older toolchains) and
`rust-toolchain.toml`'s `channel` (the rustup-side override that ensures
GitHub-hosted runners and contributor machines actually pull the right
toolchain). Future MSRV bumps must update both. Channel choice is a
minor pin (`"1.95"`, not `"stable"` or `"1.95.0"`): rustup installs the
latest patch in the 1.95.x line so we benefit from patch releases
without having to bump for them, but we don't float forward across
minors silently.

### Self-update auto-rustup (v3.11.1+)

`src/update.rs` checks `https://api.github.com/repos/QubeTX/qube-machine-report/releases/latest` (15s timeout via `ureq`), compares against `VERSION` from `Cargo.toml`, and runs an ordered probe-and-retry chain:
- `cargo install tr300 --force` first when `cargo --version` succeeds
- macOS/Linux fallback: cargo-dist shell installer via `curl`, then `wget`
- Windows fallback: cargo-dist PowerShell installer via `powershell`, then `pwsh`

`--update --json` emits a single JSON object with `current_version`, `latest_version`, `update_available`, and `success`. Success includes legacy `"method"` plus precise `"strategy"`; failure includes an `"attempts"` array. Exit codes: `0` success, `2` failure.

**v3.14.2 addendum — path detection is retired.** Earlier releases inferred the
update method from the current executable path (`.cargo/.../bin/...` meant
`cargo install`). That turned out to be the wrong signal once cargo-dist was
configured with `install-path = "CARGO_HOME"`: the official shell installer can
also place `tr300` under `.cargo/bin`, so path-based detection could choose
cargo on machines that do not have cargo installed. The updater now probes
tools directly and falls through to installer strategies on any preflight or
runtime failure.

**v3.14.3 addendum — canonical package name is `tr300`.** The crates.io package
name is lowercase `tr300`, matching the binary and Rust library import path.
The cargo strategy therefore runs `cargo install tr300 --force`. GitHub Release
installer assets use `tr300-installer.sh` and `tr300-installer.ps1`; the release
workflow also copies those files to legacy `tr-300-installer.*` aliases so
v3.14.2 binaries, whose fallback URL used the deleted old package name, can
still self-update through the installer path after cargo fails.

**Auto-rustup on the cargo strategy (v3.11.1+).** When the strategy chain tries
`UpdateStrategy::Cargo` it first calls `rustup_update_stable_best_effort()`,
which probes for `rustup` on PATH (via `rustup --version`, redirecting both
stdout and stderr to `Stdio::null()` so the probe is silent) and, if found,
runs `rustup update stable` and prints `Updating Rust toolchain (rustup
update stable)…` so the user sees what's happening. Any failure — rustup
absent, network timeout, locked toolchain, permission error — is *non-fatal*:
we discard the result with `let _ =` and proceed straight to the
`cargo install tr300 --force` call. Installer strategies never touch Rust
because they download a prebuilt binary.

*Why this exists — the failure mode it prevents:* TR-300's MSRV tracks the
GitHub Actions `stable` toolchain and moves whenever Rust ships a stable
release that promotes a lint we trigger or changes safety classifications
on stdlib intrinsics we use (cf. the 1.95 `__cpuid` reclassification that
prompted this change). Without auto-rustup, a user who installed via
`cargo install tr300` on Rust 1.94 and then later runs `tr300 --update`
against a release built with `rust-version = "1.95"` would see cargo print
`error: rustc 1.94.0 is not supported by the following package: tr300@…
requires rustc 1.95`, our `execute_update` would propagate that as a
non-zero exit, and the user would be silently stuck on the stale binary
forever — they'd assume `--update` "doesn't work" and either give up or
manually research the toolchain pin themselves. The 5–30 seconds spent on
a redundant `rustup update stable` (effectively a no-op when already
current — rustup just prints `info: cleaning up downloads & tmp directories`
and exits) is dramatically cheaper than that user-experience failure, and
this pattern means MSRV bumps in future releases stop being a coordination
problem with users.

*Why best-effort instead of "fail loudly if rustup isn't there":* not every
user manages Rust through rustup. Distro packages (Debian's `rustc`/`cargo`,
Homebrew's `rust`, NixOS's nixpkgs Rust), CI environments where rustup is
intentionally absent, and corporate-managed toolchains all install Rust
without putting `rustup` on PATH. Hard-failing in those cases would be
worse than the status quo (we'd block working updates on a tool we don't
need). Probing first and silently skipping when it's missing means we help
the rustup majority while not surprising the minority — they just see the
plain `cargo install` path and any MSRV mismatch surfaces normally, with
the standard cargo error pointing at `rust-version` so they can update
their distro/Homebrew Rust on their own terms.

*Why we don't probe rustc version and conditionally call rustup:* the
naive alternative — parse `rustc --version`, compare against an MSRV
constant, only run rustup when older — adds two new failure modes (parse
errors, drift between the constant and `Cargo.toml`'s `rust-version`) and
saves at most a few seconds. Always running `rustup update stable` is
simpler, idempotent, and self-correcting; rustup itself decides whether
work is needed.

### Intel macOS coverage policy (v3.11.2+)

**What changed.** On 2026-04-28 the `macos-13` matrix entries were removed
from both the `test` and `build` jobs in `.github/workflows/ci.yml`. CI no
longer exercises Intel macOS x86_64. The Intel binary continues to ship
from tag-push releases — `[workspace.metadata.dist].targets` in
`Cargo.toml` still lists `x86_64-apple-darwin`, and cargo-dist's
`release.yml` still builds it on a `macos-13` runner at every `vX.Y.Z`
tag. CI on every commit no longer touches that runner.

**The triggering symptom.** The five CI runs immediately preceding this
change all stalled exclusively on the two Intel macOS jobs while every
other matrix cell finished within minutes. Concrete examples (run IDs +
queue times before manual cancellation): `#25023039347` ("fix(audit):
migrate users → uzers") sat queued **3h 20m+** before being cancelled to
unblock the next push, `#25022743655` (v3.11.1 release) cancelled at 7m
59s, `#25021879374` (docs commit) cancelled at 22m 34s, `#25021109459`
(v3.11.0 release) cancelled at 18m 41s, and `#24978816648` (v3.10.0
release) sat for **15h 50m** before cancellation. The repeated workflow
was: push → wait → realise Intel never picked up → `gh run cancel ...` →
push next commit, which the `concurrency: cancel-in-progress: true`
group then auto-cancelled anyway. Hours of latency per push, no Intel
runtime ever exercised.

**Why it's structural, not a glitch.** `macos-13` is GitHub's last public
Intel x86_64 macOS hosted runner label. There is no `macos-14`,
`macos-15`, or `macos-latest` Intel variant — Apple Silicon is the only
forward path on hosted runners. GitHub has been progressively winding
down the Intel hosted-fleet capacity through 2025 as Apple Silicon
became default; on top of that thin baseline, transient incidents like
the 2026-04-27 16:31 UTC "Actions experiencing degraded performance"
event push queue depth from "slow" to "indefinite." This is not a
problem we can wait out — capacity is not coming back.

**Why dropping CI coverage is acceptable for this project.** Apple
stopped selling Intel Macs in June 2023; the newest hardware Apple
shipped with an Intel CPU (the 2019 Mac Pro / 2020 Intel iMac /
MacBook Pro) is roughly three years old by 2026-04-28 and falling out
of macOS support tiers release-over-release. Critically, dropping Intel
*CI* does not mean dropping Intel *correctness coverage*: the
`#[cfg(target_os = "macos")]` gates in `src/collectors/platform/macos.rs`
are arch-agnostic, so Apple Silicon CI exercises every line of the
macOS path. The only thing ARM CI doesn't catch is genuinely
arch-specific behavior, of which TR-300's macOS path has effectively
none — no inline asm, `format_bytes()` and the table renderer are
arch-agnostic, sysinfo's `System` API hides arch internally, and the
`sysctl`/`scutil`/`pmset`/`ioreg` subprocess calls produce identical
output regardless of CPU. The accuracy delta from losing Intel CI
coverage is close to zero in practice, and any drift would show up at
tag-push time when cargo-dist builds the Intel target anyway.

**Why not just `continue-on-error: true` on the Intel matrix entry.**
Considered. Rejected because it leaves the queued-3-hours-and-then-
cancel UX intact for every push: the workflow's overall conclusion
would be `success`, but the dashboard would still show two perpetually
pending cells, and the run wouldn't be considered "complete" until
either the runner finally picked up (rare) or `cancel-in-progress`
killed it on the next push. That's theatrical coverage — the user
experience is identical to the current pain. Hard removal is the only
fix that actually changes the dashboard state.

**Why not also drop Intel from cargo-dist's release targets.**
Considered. Rejected per maintainer direction. Release-time builds run
on a tag push (cadence: minor/patch releases, ~weekly to monthly), and
that cadence is willing to absorb a multi-hour Intel queue wait — we
don't tag releases on a deadline. The cost of keeping
`x86_64-apple-darwin` in `[workspace.metadata.dist].targets` is one
extra `macos-13` job per *tag*, not per *commit*. Any user still on
2019/2020-era Intel hardware deserves a working binary download
without having to build from source. The contract is: **CI never
blocks on Intel; releases still produce the artifact.**

**What this implies for future contributors.**

1. Don't re-add `macos-13` to `ci.yml` without a concrete reason and a
   discussion of capacity risk. The default state is "Intel is not in
   CI, period."
2. If GitHub ever ships a hosted Intel macOS replacement label
   (unlikely), prefer that label over `macos-13` and re-evaluate
   whether re-adding to CI makes sense at that point.
3. If `release.yml` starts taking longer than ~2 hours at tag time
   because of `macos-13` queue depth, *that* is the signal to revisit
   dropping `x86_64-apple-darwin` from cargo-dist's targets. Until
   then, releases tolerate the wait.
4. If a macOS-arch-specific bug is reported by an Intel user, reproduce
   locally on Intel hardware (or via a one-shot self-hosted runner)
   rather than re-introducing CI coverage. The bug-rate doesn't justify
   the queue cost.

**Why "Builds 6 targets" in `## Release Process` still says 6.** Six
binary targets continue to ship from `release.yml`: Windows x64, macOS
Intel (x86_64), macOS ARM (aarch64), Linux x64 glibc, Linux x64 musl,
Linux ARM64. CI tests three of them (Linux x64 glibc, macOS ARM,
Windows x64). The mismatch between *tested* and *shipped* platforms
is intentional and is exactly what this section documents — see above.

---

## Windows accuracy patterns

### v3.11.0+ — registry OS, IsWow64Process2, WTS last-login, CPUID hypervisor, BitLocker

- **OS detection** reads `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`
  directly (`get_os_info_from_registry`) and overrides sysinfo. Detects Win11
  by `CurrentBuild >= 22000` because the registry `ProductName` is frozen at
  "Windows 10". Adds `DisplayVersion` (release ID like `25H2`) and `UBR` to the
  kernel string for richer output.
- **Architecture detection** (`get_architecture`) calls `IsWow64Process2` via a
  manual `extern "system"` linked against `kernel32`. Returns the host's
  native machine even when the binary itself runs under emulation. Handles
  `IMAGE_FILE_MACHINE_AMD64`, `_ARM64`, `_I386`, `_ARM`. Annotates emulation
  in the form `aarch64 (x86_64 emulation)` when process arch ≠ host arch.
- **CPU frequency** (`cpu.rs::collect`) combines CPUID leaf 16h + Windows
  `CallNtPowerInformation(ProcessorInformation)` + sysinfo, using
  `Iterator::max`. Leaf 16h returns 0 on Intel hybrid (Meteor/Lunar/Arrow Lake)
  — that's a documented Intel microcode change, not a bug. Falls through to
  the next source.
- **Hypervisor detection** (`detect_virtualization_wmi`) calls
  `cpuid_hypervisor_brand()` first (CPUID leaf 0x40000000, 12-byte vendor
  string) and disambiguates the Win11 VBS edge case: if CPUID returns
  `Microsoft Hv` but the SMBIOS manufacturer is a normal OEM (not Microsoft
  Corp), the result is `Bare Metal (Hyper-V/VBS)` instead of `Hyper-V`. Real
  Hyper-V VMs always have Microsoft Corp as manufacturer.
- **Last-login** (`get_last_login_windows`) calls `WTSQuerySessionInformation`
  via a manual `extern "system"` linked against `wtsapi32` (the constants
  `WTS_CURRENT_SESSION = 0xFFFFFFFF`, `WTSLogonTime = 17`,
  `WTSConnectTime = 14` are declared inline). Falls back to a boot-time
  derivation from `GetTickCount64` because Windows leaves the WTS time fields
  at 0 for local console sessions on most modern installs (auto-login + Fast
  Startup mask the actual logon timestamp). The previous `net user`-based
  parsing returned localized strings and "Never" — gone.
- **BitLocker** (`get_bitlocker_status`) queries `Win32_EncryptableVolume` in
  the `ROOT\CIMV2\Security\MicrosoftVolumeEncryption` namespace via the `wmi`
  crate's `WMIConnection::with_namespace_path`. Try-and-degrade pattern: on
  modern Win11 Device Encryption hosts this is readable non-admin and the
  `ENCRYPTION` row renders; on older Win10 / domain configurations the WMI
  call returns access-denied → `None` and the row is gracefully omitted; the
  elevation footer hint covers the unelevated case.

### v3.12.0+ — VPN-aware default-route, Fast Startup uptime annotation

- **VPN-aware default-route detection** (`get_best_route_interface_index` +
  `get_network_info_wmi`). The historical Windows network collector queried
  `Win32_NetworkAdapterConfiguration WHERE IPEnabled = TRUE` and picked the
  first IPv4 address it found — coin-flip behavior on multi-homed hosts and
  on hosts with active VPN tunnels (Tailscale, WireGuard, OpenVPN, Cisco
  AnyConnect). v3.12.0 calls `GetBestInterfaceEx` (manual `extern` linked
  against `iphlpapi`) for `1.1.1.1`; the kernel returns the interface index
  that *would* carry packets to the public internet right now. We then
  reorder the WMI result list so that adapter is picked first, falling back
  transparently to the original first-match logic when the kernel lookup
  fails (IP Helper service disabled, no default route, etc.). The `winapi`
  features `iphlpapi`/`ws2def`/`ws2ipdef`/`winerror`/`inaddr`/`in6addr`/
  `ifdef` were added for this — `SOCKADDR_IN` is declared inline (3 fields,
  layout-stable since Win95) to keep the surface area minimal. Reference:
  [GetBestInterfaceEx](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getbestinterfaceex).
- **Fast Startup uptime annotation** (`detect_fast_startup` +
  `last_cold_boot_seconds` in `platform/windows.rs`, threaded through
  `OsInfo.session_uptime_seconds` and `SystemInfo.uptime_formatted()`).
  Windows 10/11 default to `HiberbootEnabled = 1` — at "shut down" the
  kernel session is hibernated to `hiberfil.sys` and resumed at "boot",
  so `GetTickCount64` (sysinfo's uptime) reports the resumed-session age,
  while `Win32_OperatingSystem.LastBootUpTime` reports the actual cold-boot
  time. They diverge by days on a daily-use laptop. The collector probes
  the registry key `HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\
  Power\HiberbootEnabled` (DWORD), and when enabled AND the WMI cold-boot
  time is >1h older than the kernel session age, swaps `uptime_seconds` to
  the cold-boot value AND populates `session_uptime_seconds` with the
  resumed-session value. The renderer surfaces both as
  `9d 4h 12m (session: 7h 14m)`. Both values are right; surfacing both
  avoids the "wait, I restarted three days ago, why does this say 47 days"
  confusion. The WMI cold-boot query is full-mode-only (~80 ms cost);
  `--fast` uses sysinfo's uptime exclusively. The CIM datetime field is
  deserialized via `wmi::WMIDateTime` (the `wmi` crate's serde-aware
  wrapper around `chrono::DateTime<FixedOffset>`) — manual hand-parsing of
  the `yyyymmddHHMMSS.mmmmmmsUUU` format is unnecessary and was tried-
  and-discarded in early v3.12.0 development. References:
  [Microsoft Q&A: Fast Startup boot time](https://learn.microsoft.com/en-us/answers/questions/1443763/how-to-get-oss-start-time-when-fast-startup-mode-i),
  [Win32_OperatingSystem](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-operatingsystem).

### v3.13.0+ — 5-state battery, native cores, GPU registry-prefer, PSCore detection, terminal walk

- **5-state battery awareness via `GetSystemPowerStatus`** (`get_battery_native`
  in `platform/windows.rs`). Replaces the WMI `Win32_Battery` query (~40 ms,
  COM round-trip) with a single Win32 API call (~1 ms). The output state
  machine layered on top is the v3.13.0 user-visible improvement and was
  expanded mid-implementation (originally a 3-state plan; the user requested
  better gaming-laptop awareness):
    1. `BatteryFlag == 0x80` (no system battery — desktops) → `None`, BATTERY
       row omitted entirely.
    2. `BatteryLifePercent == 0xFF` (unknown charge) → `None`.
    3. `BatteryFlag == 0xFF` (unknown state) → `X% (Unknown)` so the user
       sees something rather than nothing.
    4. `ACLineStatus == 0xFF` (AC status unknown — rare; some VMs,
       hypervisor-passthrough batteries) → `X%` with no AC label, since
       guessing would be misleading.
    5. On AC + Charging bit set → `X% (Charging)`.
    6. On AC + percent ≥ 95 + not charging → `AC Power` with NO percentage.
       The percentage at full charge is uninformative; suppressing it
       eliminates the v3.11.x output `100% (AC Power)` which was redundant.
    7. On AC + percent < 95 + not charging → `X% (Plugged in)`. Covers two
       distinct but indistinguishable scenarios from a single-snapshot API:
       (a) gaming laptop where peak GPU draw exceeds the AC brick's
       wattage (Alienware, ROG, Razer with discrete GPUs); (b) firmware-
       limited charging (ThinkPad / ASUS / Lenovo battery-longevity modes
       capping at 60-80%). Either way, the percentage matters and we can't
       distinguish without time-series sampling — the label is honest about
       the ambiguity.
    8. Off AC + Critical bit → `X% (Critical)`; Low bit → `X% (Low)`;
       otherwise → `X% (Discharging)`. Critical/Low take precedence so
       the urgency is visible.
  The `BATTERY_FLAG_HIGH` bit (0x01, > 66% charge) is intentionally NOT
  surfaced as a label suffix — early v3.13.0 testing on a fully-charged
  laptop on AC produced `100% (Discharging (High))` which was incoherent
  (charging bit OFF + ACLineStatus=1 + High set is the "fully topped up
  on AC" case, not "discharging"). The percentage already conveys charge
  level. Reference:
  [GetSystemPowerStatus](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getsystempowerstatus).
- **Native socket count via `GetLogicalProcessorInformationEx`**
  (`get_socket_count_native` in `platform/windows.rs`). Replaces the
  `Win32_Processor` WMI count (~30 ms COM round-trip) with the standard
  two-call buffer-sizing pattern: first call with `Buffer = null_mut`
  returns FALSE + `ERROR_INSUFFICIENT_BUFFER` and writes
  `returned_length`; allocate exactly that size and call again. Walks the
  resulting variable-length `SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX`
  records by reading the `Size` field at each entry's offset (the records
  aren't fixed-size — different `Relationship` values pack different
  payloads). Counts entries with `Relationship == RelationProcessorPackage`.
  WMI fallback (`get_socket_count_wmi`) retained as a safety net during
  the C.9 → C.14 transition. Reference:
  [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex).
- **GPU enumeration prefers the registry path** (`filter_software_gpus` +
  the existing `get_gpus_fast` registry walk). The `--fast` mode already
  used the `{4d36e968-e325-11ce-bfc1-08002be10318}` Display class registry
  walk because it's COM-free and fast (~5 ms), and it has the additional
  property that it only enumerates hardware adapters — Microsoft Basic
  Render Driver, Microsoft Hyper-V Video, and similar software-only
  adapters don't appear there. Full mode now also prefers it (with WMI /
  PowerShell as fallbacks), and a `filter_software_gpus()` name-based
  filter strips known-bad strings as belt-and-suspenders. This is a
  deliberate simplification of the originally-planned C.8 (full DXGI
  `IDXGIFactory1::EnumAdapters1` COM enumeration) — the simpler approach
  achieves the same user-visible outcome (no software adapters in the GPU
  row) at 25 LOC vs ~100 LOC of unsafe COM. DXGI remains an option if
  vendor/device-ID filtering ever becomes necessary that name-based
  filtering can't do.
- **PowerShell 7+ ("PowerShell Core") detection** (`get_powershell_core_version`
  in `platform/windows.rs`). The historical `get_shell()` only knew about
  Windows PowerShell 5.x via `HKLM\SOFTWARE\Microsoft\PowerShell\3\
  PowerShellEngine\PowerShellVersion`. PowerShell 7+ installs register
  under a different hive: `HKLM\SOFTWARE\Microsoft\PowerShellCore\
  InstalledVersions\<GUID>\SemanticVersion`. We do a recursive
  `reg query /s` and pick the highest `SemanticVersion` value found by
  string-compare (works for 3-tuple semver since each component fits in
  2 digits — no zero-padding needed). Falls back to the legacy 5.x
  detection when no PSCore subkey exists.
- **Terminal parent-process walk via Toolhelp32**
  (`detect_terminal_via_parent_walk` + `match_terminal_name` in
  `platform/windows.rs`). When neither `WT_SESSION` nor `TERM_PROGRAM`
  nor the new Cursor env vars are set (common when launched from a
  desktop shortcut, a fresh subshell that lost the parent's environment,
  or by an AI agent), `get_terminal()` walks the parent-process chain:
  `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)`, then iterate via
  `Process32FirstW` / `Process32NextW`, build a
  `HashMap<pid, (parent_pid, name)>`, then climb from `GetCurrentProcessId()`
  upward (cap 10 levels — defensive against PID-table cycles, which
  happen in practice on Windows when a parent dies and its PID gets
  recycled). Recognizes Windows Terminal, WezTerm, Alacritty, VS Code,
  Cursor, Windsurf, Hyper, Tabby, Ghostty, Kitty, MinTTY, Claude Code,
  Antigravity. Intermediate hosts (`conhost.exe`, `powershell.exe`,
  `pwsh.exe`, `cmd.exe`, `bash.exe`, `sh.exe`, `zsh.exe`, `fish.exe`,
  `nu.exe`, `tr300.exe`, `node.exe`, `python.exe`) are skipped so the
  walk continues through them; unrecognized exes break the walk silently
  and the row falls back to `Console`. Verified on this dev host: when
  TR-300 runs inside Claude Code's Bash tool, the chain
  `tr300.exe → bash.exe → claude.exe → powershell.exe` correctly
  resolves to `Claude Code`. Reference:
  [CreateToolhelp32Snapshot](https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/nf-tlhelp32-createtoolhelp32snapshot).

### v3.14.4+ — Windows install execution-policy preflight

**Failure mode this prevents.** On a fresh Windows machine, `tr300 install`
wrote a valid alias-plus-auto-run block to
`Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1`, but the next
PowerShell session immediately errored:

```
File C:\Users\<user>\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1
cannot be loaded because running scripts is disabled on this system.
    + FullyQualifiedErrorId : UnauthorizedAccess
```

Windows Client defaults `ExecutionPolicy` to `Restricted` at both
`CurrentUser` and `LocalMachine` scopes — and when every scope is
`Undefined`, scope-precedence resolves to `Restricted` too. `Restricted`
blocks every `.ps1` file including `$PROFILE` itself, so the auto-run never
fires on a freshly installed system. The user's prompt-driven `tr300`
invocations still worked because `tr300.exe` is a native binary on PATH;
execution policy governs `.ps1` only. The installer was silently failing
its contract ("auto-run on new interactive shells") because it never
inspected or adjusted execution policy.

**The fix.** `src/install/windows.rs::run_execution_policy_preflight()`
runs before the profile write. It calls `Get-ExecutionPolicy -Scope
CurrentUser`, classifies the result via `policy_state()` into one of
`{BlockedDefault, BlockedAllSigned, Permissive, Unknown}`, and only acts
on `BlockedDefault` (`Restricted` / `Undefined`). For that case it runs
`Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
-Force`, then re-reads the policy to confirm the change took. Three
outcome branches:

1. **Succeeded** — prints a one-line "Set PowerShell CurrentUser execution
   policy: <previous> -> RemoteSigned" followed by the "(required to load
   $PROFILE; only your account, no admin needed)" line, then proceeds with
   the alias write. Verified on the user's exact failure-mode machine.
2. **Set-ExecutionPolicy returned 0 but the policy is still blocking**
   (`TrySetResult::StillBlocked`) — a higher-precedence `MachinePolicy` or
   `UserPolicy` Group Policy wins scope precedence over `CurrentUser`.
   Prints a fallback warning to stderr explaining what we tried, what the
   effective policy still is, and a `LocalMachine`-scope remediation. The
   alias write half still succeeds; only the auto-run UX is degraded.
3. **Set-ExecutionPolicy itself failed** — print the same fallback
   warning with the underlying error message. Same non-fatal posture.

The `BlockedAllSigned` branch never touches `Set-ExecutionPolicy`. It
prints a notice explaining `AllSigned` blocks the unsigned auto-run
snippet, offers the user the choice between signing the block themselves
or relaxing to `RemoteSigned`, and proceeds with the alias write. The
policy is left alone.

**Why `RemoteSigned` and not other policies.** PowerShell's execution
policies, strictest to most permissive:

| Policy | Loads our local profile? | Why we don't use it |
|---|---|---|
| `Restricted` | ❌ | The default — the bug we're fixing. |
| `AllSigned` | ❌ (without code-signing the snippet) | Would require shipping an Authenticode cert. Rejected — it expands deploy surface dramatically for a one-user-facing-snippet win. |
| **`RemoteSigned`** | ✅ | **Minimum policy that loads a local unsigned profile.** Downloaded scripts must still be Authenticode-signed. What we use. |
| `Unrestricted` | ✅ | Loads downloaded scripts with a confirmation prompt. Strictly more permissive than needed. Rejected. |
| `Bypass` | ✅ | Loads everything silently with no warnings. Strictly more permissive than needed. Rejected. |

`RemoteSigned` is the policy [Microsoft's own
documentation](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_execution_policies)
recommends for non-server Windows installs and the policy Scoop,
oh-my-posh, and starship all rely on. Using `Unrestricted` or persistent
`Bypass` would meet the user's explicit constraint ("not overly
permissive") with strictly less safety than `RemoteSigned` gives.

**Why `CurrentUser` scope and not `LocalMachine`.** `CurrentUser`
writes to `HKCU\SOFTWARE\Microsoft\PowerShell\1\ShellIds\Microsoft.PowerShell`
and affects only the current user. It does not require Administrator,
does not affect other accounts on the machine, and matches the rest of
`tr300`'s install footprint (which targets `%LOCALAPPDATA%\Programs\tr300`
and `$PROFILE` — both `CurrentUser` paths). `LocalMachine` would require
an elevated process and would impose our policy choice on every account
on the box; we never want that.

**Why verify after `Set-ExecutionPolicy`.** Scope precedence is
`MachinePolicy > UserPolicy > Process > CurrentUser > LocalMachine`.
Domain-managed Windows machines can set `MachinePolicy` or `UserPolicy`
via GPO, and those scopes are read-only from user processes.
`Set-ExecutionPolicy -Scope CurrentUser` succeeds (exits 0) — it really
does write the HKCU key — but the *effective* policy seen by the next
shell is still whatever GPO enforces. Re-reading
`Get-ExecutionPolicy -Scope CurrentUser` immediately after the set
confirms whether the change actually took effect under the active scope
precedence. Without this check, the `StillBlocked` path would be invisible
and the user would silently get the same `UnauthorizedAccess` error on
the next shell start.

**Why uninstall doesn't roll back execution policy.** The user's pre-install
policy is unknown by the time `tr300 uninstall` runs (we don't persist it
anywhere — that would expand the install footprint to a state file), and
other PowerShell tooling installed on the same machine — Scoop, oh-my-posh,
starship, dev module loaders — typically depends on `RemoteSigned` too.
Reverting to `Restricted` on uninstall would break those tools. Leaving
the policy alone is the least-surprising behavior. Documented in README.

**Why not write to both Windows PowerShell 5.x and PowerShell 7+
profiles.** PowerShell 7+ (`pwsh.exe`) uses
`Documents\PowerShell\Microsoft.PowerShell_profile.ps1`, separate from
WinPS 5.x's `Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1`.
The reporting user was on WinPS 5.x and the current installer only
targets WinPS 5.x via `powershell.exe -NoProfile -Command "$PROFILE"`,
so this fix unblocks their case. PowerShell 7+ profile coverage is real
gap but a separate scope expansion — tracked but not landed here.

**Rejected alternatives.**
- *Always set `Bypass`* — would meet the user's "minimum" constraint with
  strictly less safety than `RemoteSigned`. Rejected.
- *Touch `LocalMachine` instead of `CurrentUser`* — requires admin,
  affects all users on the box. Rejected.
- *Skip the preflight, document the failure in README and point users at
  `Set-ExecutionPolicy`* — every fresh-Windows user runs `tr300 install`
  and hits the failure mode the very first time; the install command's
  job is to make the tool work, not to print a workaround. Rejected.
- *Sign the auto-run snippet ourselves so `AllSigned` works too* —
  requires Authenticode cert infrastructure, ongoing key rotation, and a
  release-time signing step. Not worth it for the AllSigned user count.
- *Write a `.cmd` shim that PowerShell can `Invoke-Item` from a profile
  that's allowed under `Restricted`* — there is no such profile under
  `Restricted`. Rejected.

Reference:
[Microsoft Learn — about_Execution_Policies](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_execution_policies),
[Set-ExecutionPolicy](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.security/set-executionpolicy).

### v3.14.5+ — Windows install error advisor

**Failure mode this prevents.** Prior to v3.14.5, when `tr300 install` hit
a permissions error on a work machine — Intune-managed device, Active
Directory Group Policy lockdown, AppLocker policy, Windows Defender
Application Control (WDAC), antivirus / EDR write-block, or OneDrive
Known Folder Move with offline files — the user saw something like:

```
Error: Platform { message: "Failed to write profile: Access is denied. (os error 5)" }
```

Two things wrong with that output:

1. The Debug-format wrapper (`Platform { message: "..." }`) is `cargo
   run`-grade developer output, not user-grade output. `thiserror` has a
   perfectly good Display impl (`Platform operation failed: ...`) that
   never gets used because `fn main() -> Result<()>` calls Debug on the
   error before exiting non-zero.
2. The message has no remediation. The user is told *what* failed but
   not *why* it likely failed or *what to do next*. On corporate
   machines this is the #1 friction point in install adoption — IT
   doesn't know what to allowlist, and the user has no concrete
   "give this to IT" path.

**The fix.** Two related changes in `src/install/windows.rs` and
`src/main.rs`:

1. **`fail_install(InstallStep, &Path, io::Error) -> AppError`** in
   `src/install/windows.rs`. Every `std::fs` call in install/uninstall
   now goes through this advisor. It:
   - Streams a multi-paragraph guidance block to *stderr* —
     `step.short_description()` → `Path:` + `Cause:` (including the raw
     Windows error code) → step-and-kind-specific "Likely reasons"
     paragraph → "Manual `tr300` still works" reassurance closer.
   - Then returns a concise `AppError::platform("write profile: <io
     err>")` suitable for `main()`'s trailing summary line.
   - The rich output is on stderr so it's never swallowed by code that
     only captures the returned error, and so it interleaves correctly
     with `main()`'s `Error: ...` line.

2. **Dispatch on `(InstallStep, ErrorKind, raw_os_error, path
   inspection)`.** The advisor branches on multiple inputs to pick the
   right remediation paragraph:

   | Condition | Guidance |
   |---|---|
   | `PermissionDenied` / Windows error 5, **OneDrive path** | OneDrive-specific: ensure synced, "Always keep on this device", or ask IT to allow OneDrive-folder writes |
   | `PermissionDenied` / error 5, **redirected (UNC) path** | Network share / folder redirection: check share access, ask IT |
   | `PermissionDenied` / error 5, **plain local path** | Intune / AD GPO / AppLocker / WDAC / antivirus, with `takeown /F "..." /R` example |
   | Sharing violation (error 32) | OneDrive sync engine + EDR + open editor as causes; retry |
   | Storage full / error 112 | Free disk space |
   | Path too long / error 206 | MAX_PATH overflow; suggest LongPathsEnabled |
   | NotFound on ReadProfile | Transient race; retry |

   The OneDrive detection uses `looks_like_onedrive_path()`, a
   case-insensitive segment scan matching both `\OneDrive\` and
   `\OneDrive - <TenantName>\` (OneDrive-for-Business). False positives
   on filenames containing "onedrive" are intentional — the
   user-visible cost is just slightly off-topic advisory text, and
   keeping the predicate simple is worth more than a perfect filter.

**Why we don't put the rich guidance in the AppError message itself.**
That would force the message into a single string with embedded
newlines, which renders poorly in non-terminal consumers (CI logs,
piped to other tools) and forces a hard coupling between the error type
and the user-facing UX. Keeping the advisor as a side effect of
`fail_install()` and the returned `AppError` as a short tag means
programmatic consumers see a clean `"write profile: <io err>"` and
human consumers see the rich stderr stream.

**Why dispatch on `path` and not just `ErrorKind` + raw code.** The
same Windows error 5 has *different remediations* depending on where
the path lives: OneDrive sync state vs. corporate MDM policy vs.
locally-mounted volume with weird ACLs. A one-size-fits-all "permission
denied" paragraph would either be too vague (skip the actionable steps)
or include OneDrive-specific advice for non-OneDrive paths (confusing).
Splitting the branch on `looks_like_onedrive_path()` is the cheapest way
to put the user on the right track.

**Why we never abort install on the execution-policy preflight failure
but we *do* abort on profile-write failure.** The preflight failure is
recoverable manually (the alias write still happens; `tr300` keeps
working as a binary on PATH). The profile-write failure means the
install genuinely did not complete — there is no alias, there is no
auto-run. The user needs to know that, with concrete remediation,
rather than silently continuing.

**Rejected alternatives.**
- *Put the rich guidance into `AppError::Platform::message`* — turns
  the message into a multi-line embedded string and couples the error
  type to the rendering. Rejected.
- *Use a separate error variant per scenario (e.g. `OneDriveBlocked`,
  `IntuneBlocked`)* — would require detection logic that doesn't exist
  (we can't reliably tell Intune from AppLocker from antivirus just
  from an `io::Error`). The "Likely reasons (most common first)"
  presentation is honest about that uncertainty. Rejected.
- *Print only the short error and rely on documentation* — defeats the
  purpose. The user already has the message in front of them; the
  guidance should be there too. Rejected.
Reference:
[`std::io::ErrorKind`](https://doc.rust-lang.org/std/io/enum.ErrorKind.html),
[Windows System Error Codes](https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes),
[OneDrive Known Folder Move](https://learn.microsoft.com/en-us/onedrive/redirect-known-folders),
[Long Paths in Windows](https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation).

### Windows distribution model (v3.15.0+)

**Decision: ship four Windows installer artifacts at every tagged release.** Two MSIs (Global perMachine + Corporate perUser) and two Inno Setup EXE installers (Global perMachine + Corporate perUser). The MSI Global has been shipping since v3.13.1; this release adds the other three.

```
tr300-x86_64-pc-windows-msvc.msi                   (Global MSI,  perMachine, UAC)
tr300-x86_64-pc-windows-msvc-corporate.msi         (Corp.  MSI,  perUser,    no UAC)
tr300-x86_64-pc-windows-msvc-setup.exe             (Global EXE,  perMachine, UAC)
tr300-x86_64-pc-windows-msvc-corporate-setup.exe   (Corp.  EXE,  perUser,    no UAC)
```

**Problem this solves.** Before v3.15.0, Windows install required either `cargo install tr300` (which the project's homepage commands wrap with rustup + MSVC Build Tools — a ~5 GB install chain blocked by every managed-device policy I've checked) or the cargo-dist PowerShell installer `irm | iex` (broken under PowerShell `ExecutionPolicy=Restricted` and frequently blocked by AppLocker / WDAC). The only MSI we shipped was perMachine, which requires admin — exactly the cohort of users (managed corp machines) who most need a friction-free install. Result: users couldn't install on the machines that the tool was most useful on.

**Why two MSIs + two EXEs, not a single dual-purpose installer.**

There IS a "single-package authoring" pattern in WiX that produces a dual-purpose MSI which installs perUser by default and perMachine when run elevated (`ALLUSERS=2` + `MSIINSTALLPERUSER=1`). I rejected this approach for two reasons:

1. **[WiX issue #7137](https://github.com/wixtoolset/issues/issues/7137) documents the pattern is fragile in WiX v4/v5** — the upgrade-detection logic doesn't reliably find an existing install of the same product when the scope differs. Real users have reported v1.0.0 perMachine + v1.0.1 perUser ending up with both installed. The two-separate-MSIs-with-different-UpgradeCodes pattern is the industry workaround.
2. **Different UpgradeCodes are SEMANTICALLY correct.** A perMachine install (everyone on the box sees the binary) and a perUser install (only my account sees it) are different products from the operating system's perspective. Pretending they're one product creates upgrade ambiguity, ARP entry confusion, and uninstall edge cases.

Same reasoning for two EXE installers vs one with a runtime mode switch (`tr300-setup.exe --mode user` vs `--mode admin`). The runtime-switch pattern doesn't actually exist in Inno Setup — `PrivilegesRequired` is a compile-time directive. You'd need two separate `.iss` files anyway; might as well make them obviously separate.

**Why MSI + EXE for the same edition.** End users have well-established preferences. Some prefer `setup.exe` (familiar pattern from decades of Windows desktop apps); others trust `.msi` more (managed IT environments). The marginal cost of building both formats from the same prebuilt `tr300.exe` is small (~80 LOC of Inno Setup script per edition, ~30 LOC of GitHub Actions YAML). The marginal benefit is real: every user gets their preferred format and the install completes.

**Why install both formats to the same path within each edition.** Two alternatives considered:

1. **Same path, document "pick one"** (chosen). MSI Global and EXE Global both install to `C:\Program Files\tr300\bin\`. The first installer to run wins; the second sees the binary already exists and either overwrites or merges. README documents that users should pick ONE format per edition. If they install both anyway, the result is two Add/Remove Programs entries, one binary file, and PATH ordering decides which `tr300` (logically both refer to the same file) runs. Confusing but not broken.
2. **Distinct paths** (`C:\Program Files\tr300\bin\` for MSI, `C:\Program Files\tr300 (Setup)\bin\` for EXE). Rejected because the suffix is ugly, PATH ordering still picks a winner, and the duplicate-install scenario is rare enough that it's not worth uglifying the common case.

The Industry Standard Solution to the duplicate-install problem is **don't let users install both** — installers detect their counterpart and refuse. WiX can do this with custom actions; Inno Setup can do this with `[Code]` checks. I considered it and rejected for v3.15.0 because the user-visible problem is small (annoying ARP duplication) and the engineering cost is high (cross-installer detection logic in both WiX and Inno Setup). May revisit in a future release if real users hit this.

**The registry marker (`HKCU\Software\TR300\InstallSource`).** All four installers write a literal string to this registry value on install: `msi-global`, `msi-corporate`, `exe-global`, or `exe-corporate`. `src/update.rs::detect_install_origin()` reads it to choose which installer to fetch and re-run for in-place upgrades.

Why HKCU and not HKLM:
- `tr300 update` always runs as the user, who reads HKCU naturally.
- Writing to HKLM from a perMachine MSI works but requires elevation-aware Component authoring (`Component KeyPath` semantics get awkward with HKLM writes from a perUser-or-perMachine wrapper).
- The rare "admin installed perMachine MSI on behalf of end user X, user X now runs `tr300 update`" case is covered by the path-based fallback in `classify_install_path()` — `\Program Files\tr300\` in the running binary's path implies MSI Global even without a marker.

The path-based fallback also handles **pre-v3.15.0 installs** that don't have the marker. Those users get a sensible default: Program Files → MSI Global, LocalAppData → MSI Corporate. New installs always set the marker.

**Why no SHA256 verification on downloaded installers.** The existing cargo-dist PowerShell installer also doesn't verify SHA256 — it trusts HTTPS to github.com. Adding SHA256 verification to `try_msi_install()` / `try_exe_install()` would require fetching the `.sha256` sidecar, parsing it, computing the hash, comparing — about 60 LOC for marginal additional security (the actual threat model would require an attacker to compromise GitHub's TLS certificates AND the GitHub Actions runner that built the binary). Tracked as future work; not blocking v3.15.0.

**Why `.github/workflows/windows-installers.yml` triggers on `release: published`, not `push: tags`.** The release.yml workflow (cargo-dist) needs to create the GitHub Release before `gh release upload` from windows-installers.yml can attach assets to it. `release: types: [published]` fires after `release.yml` finishes, which is the correct sequencing. The alternative — `push: tags` with a `needs:` dependency — doesn't work across separate workflow files in GitHub Actions.

**Why Inno Setup, not NSIS or WiX Burn.**

- **WiX Burn** would let us produce a bundle.exe that wraps the existing MSI. Considered. Rejected because WiX 3 Burn is finicky (Bundle.wxs syntax is non-trivial, the build pipeline through `candle.exe` + `light.exe` differs from the MSI flow), and the `cargo-wix` CLI doesn't directly support Burn bundles. We'd need to drop down to raw WiX 3 tooling. Inno Setup is one `.iss` file per edition and a single `iscc` command.
- **NSIS** is also viable but its scripting language is less ergonomic than Inno's Pascal-like syntax, and the resulting installers are slightly larger.
- **Inno Setup** (chosen) is widely used in the Rust CLI ecosystem (gh CLI, deno, bun, uv all use it on Windows). Free, mature, well-documented. The `.iss` files are human-readable and self-contained. The CI install is one `choco install innosetup` line.

**Why two separate Inno Setup .iss files instead of one parameterized.** Inno Setup supports `#define` and `#if` for compile-time conditionals, so a single file with `#define GLOBAL_OR_CORPORATE` would work. Rejected because: (1) the two files differ in enough ways (PrivilegesRequired, DefaultDirName, PATH registry root, AppId, OutputBaseFilename, Code block contents) that the conditional branches would dominate the file; (2) two files is more obvious to readers; (3) the AppIds are forever-distinct, so the files can't truly share an identity anyway. Duplicate-ish code is fine when the products are genuinely separate.

**Why `install-path = "CARGO_HOME"` is still set in `[workspace.metadata.dist]`.** That setting controls where the cargo-dist PowerShell installer (`tr300-installer.ps1`) places the binary on Windows — `%USERPROFILE%\.cargo\bin\tr300.exe`. The new MSI / EXE installers don't use cargo-dist's installer logic at all; they have their own install paths defined in `wix/*.wxs` and `inno/*.iss`. Leaving the cargo-dist setting alone means the PowerShell installer keeps working for users who prefer it (or for users on platforms without an installer story, like ARM Windows once we add that target).

**SmartScreen / unsigned-binary honest accounting.** All four installers are unsigned. First-time download on Win11 24H2+ triggers "Windows protected your PC" — user clicks **More info → Run anyway**. README documents this; we don't pretend it doesn't happen.

Real options for fixing this (not done in v3.15.0):
1. **Azure Trusted Signing** (~$10/mo) — Microsoft's EV-equivalent service that gives instant SmartScreen reputation. Requires verified org identity (we'd need to register QubeTX). Likely the right answer long-term.
2. **OV code-signing cert** (~$80–200/yr) — still requires accumulated download reputation to clear SmartScreen, but reduces "Unknown publisher" friction.
3. **Document the manual override** (chosen for now) — what BleachBit, Teleport, ScreenToGif, and many other open-source Windows tools do.

**Rejected alternatives.**
- *Use a single perUser MSI that auto-detects "in admin? install perMachine. Otherwise perUser."* — see WiX #7137 above. Too unreliable.
- *Drop the perMachine MSI entirely and ship only perUser* — would break IT-managed deployment via Intune / SCCM. Real users need perMachine for managed rollouts.
- *Ship only one EXE format with a runtime `--user / --machine` switch* — not supported by Inno Setup's `PrivilegesRequired` directive. Would require building two binaries anyway.
- *Include code signing in this release* — too much scope. Tracked as future work.
- *Submit to WinGet / Scoop in this release* — high-value follow-up, but adds external dependencies (PR review processes). Defer to v3.16.0.

References:
- [WiX Toolset Issue #7137 — dual-purpose perUser MSI](https://github.com/wixtoolset/issues/issues/7137)
- [Microsoft Learn — WDAC vs AppLocker overview](https://learn.microsoft.com/en-us/windows/security/application-security/application-control/windows-defender-application-control/wdac-and-applocker-overview)
- [Microsoft Learn — about_Execution_Policies](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_execution_policies)
- [Inno Setup documentation](https://jrsoftware.org/ishelp/)
- [cargo-wix project README](https://github.com/volks73/cargo-wix)
- [cargo-dist MSI installer book](https://opensource.axo.dev/cargo-dist/book/installers/msi.html)
- [Azure Trusted Signing](https://learn.microsoft.com/en-us/azure/trusted-signing/)
- [Microsoft Learn — Submit packages to WinGet](https://learn.microsoft.com/en-us/windows/package-manager/package/)

### v3.15.1 addendum — Why corporate.wxs lives in `wix-corporate/`, not `wix/`

v3.15.0's `release.yml` run 25901237669 failed at `build-local-artifacts(x86_64-pc-windows-msvc)` with WiX candle exit code 6. v3.15.0 published to crates.io (it was past the crates-publish stage by then per /release § 10's "tag only after both ci.yml and crates-publish are green") but had no GitHub Release artifacts. v3.15.1 is the fix-forward patch release. Two root causes:

**Root cause 1: `<Property Id='ALLUSERS' Value=''/>` in v3.15.0 `wix/corporate.wxs`.** I'd added this Property because folklore and old WiX 3 examples claim that explicit `ALLUSERS=""` + `MSIINSTALLPERUSER=1` are needed to force a per-user install. WiX 3.11 candle rejects `Value=""` with `CNDL0006: The Property/@Value attribute's value cannot be an empty string` and emits a follow-up `CNDL1006` warning explaining the Property would be ignored anyway. The actual WiX 3 rule: `InstallScope='perUser'` on the Package element is sufficient by itself. The MSI installer treats unset `ALLUSERS` as per-user, which is exactly what we want; declaring it with an empty value is both syntactically rejected by candle AND, even if it parsed, semantically equivalent to not declaring it at all. The v3.15.0 → v3.15.1 fix was to delete both that line and the redundant `MSIINSTALLPERUSER=1` Property.

**Root cause 2: cargo-wix's "compile every `wix/*.wxs` into ONE MSI" default.** Once root cause 1 was fixed, light.exe surfaced `LGHT0089` ("Multiple entry sections '*' and '*'") and a cascade of `LGHT0091/0092` ("Duplicate symbol") errors at the link stage — for `Property:Manufacturer`, `Property:ProductCode`, `Property:ProductLanguage`, `Property:ProductName`, `Property:ProductVersion`, `Property:UpgradeCode`, `Property:DiskPrompt`, `Media:1`, `Directory:APPLICATIONFOLDER`, `Component:binary0`, `Component:Path`, `Component:InstallSourceMarker`. cargo-wix scans `wix/*.wxs` non-recursively and feeds ALL matching files to candle → produces one wixobj per .wxs → light.exe links them ALL into a single output. Two complete Product definitions in the same directory cannot coexist this way; they conflict at every shared symbol.

The fix has two parts: (a) move `corporate.wxs` to a NEW directory `wix-corporate/` so cargo-wix's default scan of `wix/` produces only the Global MSI cleanly. (b) Build the Corporate MSI separately by calling bare `candle.exe` + `light.exe` from `windows-installers.yml`, never through cargo-wix. cargo-wix CLI has `--include <path>` to ADD files but no equivalent "use ONLY this file" flag, so cargo-wix is the wrong tool for compiling one wxs from outside `wix/`.

The bare WiX invocation needs `-sice:ICE38 -sice:ICE64 -sice:ICE91` flags on `light.exe`. Per-user MSIs in WiX 3 want:
- ICE38: every Component installing to a user-profile directory should have an HKCU RegistryValue as its KeyPath (not a File KeyPath).
- ICE64: every Directory in the user profile needs a Component with a `RemoveFolder` element so the directory is cleaned up on uninstall.
- ICE91: WiX warns that per-user files in `[Bin]` won't replicate to other users' profiles even if perMachine was somehow desired (cosmetic).

Making `wix-corporate/corporate.wxs` ICE-clean is a real option, requiring ~5 additional `<Component>` elements per intermediate Directory (each with `<RemoveFolder>` + an `<RegistryValue Root='HKCU'>` KeyPath dummy), changing the existing `Path` and `binary0` Components to add HKCU RegistryValue KeyPaths instead of relying on Environment/File KeyPaths, plus matching `<ComponentRef>` entries in the Feature tree — roughly 40 lines of new WiX boilerplate. Rejected for v3.15.1: the practical impact of suppressing the three ICEs is one empty `%LocalAppData%\Programs\tr300\bin\` folder left in the user profile after uninstall. Install and uninstall both work correctly; PATH modification works; the binary is fully functional. The empty folder is a cosmetic leftover that's acceptable for a CLI tool.

**Verification path used to diagnose this.** Local repro with portable WiX 3.11 binaries from [github.com/wixtoolset/wix3/releases](https://github.com/wixtoolset/wix3/releases/tag/wix3112rtm) (downloaded as `wix311-binaries.zip`, extracted to `/tmp/wix3/`). With cargo-wix installed via `cargo install cargo-wix`, running `cargo wix --no-build --nocapture` against v3.15.0's source surfaced both error classes in sequence — first the CNDL0006 from candle, then (after the Property fix) the LGHT0089/0091 cascade from light.exe trying to link both wixobjs together. cargo-dist's CI invocation suppresses these details by default (it captures candle/light stderr); local reproduction with `--nocapture` was essential.

**Rejected alternative for root cause 2: in-tree wxs file rename.** I considered renaming `wix/corporate.wxs` to `wix/corporate.wxs.bak` in the windows-installers.yml workflow before calling cargo-wix, then restoring. Worked but ugly — the file would be physically renamed at CI runtime, which complicates local builds and confuses git. The directory move (`wix-corporate/`) is structural and explicit.

**Rejected alternative for root cause 1: keep the ALLUSERS Property with a non-empty Value.** WiX docs allow `Value=" "` (single space) as a workaround for the empty-string rule. Tried in an earlier draft; it passes candle but generates a meaningless ALLUSERS Property that doesn't actually affect install scope. The clean answer is to delete the Property entirely.

**Side effect: `Cargo.toml` `include` list gained `/wix-corporate/**`** so the published crate ships with the Corporate WiX source. This is for completeness — the practical Corporate MSI build happens in CI from the Git repo, not from a downloaded crate.

---

## Install / update safety primitives (v3.15.2+)

The v3.15.2 audit + remediation cycle surfaced six related-but-distinct
"silent failure" classes across the install / update / runtime paths.
Each shipped as a focused primitive rather than a one-off fix, with the
explicit intent that future Claude agents see the pattern and reuse it.

### Atomic rc-file writes

**Problem.** `std::fs::write(path, content)` opens the target with
`O_TRUNC` / `CREATE_ALWAYS` (POSIX / Windows respectively) and then
writes. If the process dies between truncate and write completion — `Ctrl-C`
during the install, a power loss, an antivirus quarantining the file
mid-write, OneDrive deciding to rename the file during the open — the
target file is left **truncated or partial**. For files the user has
invested real time in (`~/.bashrc`, `~/.zshrc`, PowerShell `$PROFILE`),
that loss is silent and irrecoverable.

**Fix.** `src/install/mod.rs::atomic_write(path, content)`:
1. Write to a sibling temp file (`.<filename>.tr300-tmp`) in the same
   parent directory.
2. `sync_all()` to flush to disk.
3. `std::fs::rename(temp, target)` — atomic on POSIX, atomic on NTFS
   within a volume.

The temp file in the same parent dir is load-bearing: NTFS guarantees
atomicity for `MoveFileExW(MOVEFILE_REPLACE_EXISTING)` only when both
endpoints are on the same volume, and POSIX `rename(2)` has the same
constraint. Using `std::env::temp_dir()` for the temp would have placed
it on a different filesystem in the common case and silently degraded
to non-atomic copy-then-delete.

On any failure path, the temp file is cleaned up with a best-effort
`fs::remove_file` so we don't leak orphan `.<filename>.tr300-tmp` files.

**Rejected alternative: `tempfile` crate.** The `tempfile::NamedTempFile`
API does roughly the same dance but with a heavier API surface and an
additional dependency. For three call sites + a couple test fixtures,
the hand-rolled ~40 LOC helper wins on transparency and zero added
deps.

**Rejected alternative: write to `$TEMP` first, then copy.** Works but
isn't atomic — the copy-then-delete sequence has the same partial-write
hazard we were trying to avoid, just in a different file. Same-parent
temp is the only path that gets actual atomicity.

**Companion: `backup_once(path)`.** First install also copies the
pre-TR-300 contents to `<path>.tr300-backup` if no backup exists yet.
Idempotent: subsequent installs preserve the ORIGINAL backup. We never
overwrite a backup with a TR-300-modified version — that would silently
destroy the user's last good copy.

### Marker-balance pre-check

**Problem.** The `remove_delimited_block` parser (now in
`src/install/shared.rs`) opens a block on any line containing
`# TR-300 Machine Report` and closes it on any line containing
`# End TR-300`. If a user hand-edited `# End TR-300` out of their rc
file — a real and plausible failure mode when tidying shell config —
the parser silently sets `in_block = true` on the start marker, never
sees the close, and drops every subsequent line through EOF.

The user's `~/.bashrc`, gone, on the next `tr300 install`.

**Fix.** `check_marker_balance(content, start, end)` counts lines
containing each marker. If `count(start) != count(end)`, refuse the
write up-front with an actionable error explaining how to repair the
block by hand. All four call sites (`update_shell_profile`,
`remove_from_profile`, Windows `install_into_profile`, Windows
`uninstall`) call this before any state mutation.

**Why count instead of "in_block at EOF" check.** Counting also catches
the inverse case (an orphan `# End TR-300` line on its own — less common
but possible). And it's an O(n) single-pass over the file we already
have in memory; no performance concern.

### SHA256 verification of downloaded installers

**Problem.** Pre-v3.15.2 the Windows update path (`src/update.rs`)
trusted only TLS to `github.com` and immediately handed the downloaded
MSI / EXE bytes to `msiexec /i` or ran the Inno EXE directly. The
implicit comment was "we trust the cert chain" — but TR-300's target
audience is corporate users on machines with TLS-interception proxies
(Bluecoat, Zscaler, McAfee Web Gateway, etc.) installing a trusted
root certificate in the user store. Those proxies see plaintext
HTTPS traffic and can rewrite installer bytes in flight.

The worst case is the Global EXE: Inno Setup binaries declare
`PrivilegesRequired=admin` at launch, so one UAC click on a tampered
installer gives the attacker SYSTEM-equivalent code execution on
every machine that auto-updates.

**Fix.** After `download_to_file`, fetch `{installer_url}.sha256` in a
SEPARATE request, parse the cargo-dist `<lowercase-64-char-hex>  *<filename>`
format (also tolerant of the asterisk being absent — some sha256sum
invocations emit text mode), compute SHA-256 of the downloaded file
via `sha2::Sha256`, and refuse to launch on mismatch.

**Why this is meaningful uplift even though both requests go through
the same MITM.** The proxy has to tamper with BOTH the installer AND
the sidecar in a way that yields a matching hash. SHA-256 is
preimage-resistant — you can't compute new bytes that hash to a
specific target value. The attacker would have to either:
1. Use a hash collision (currently believed-hard for SHA-256 — no
   published collision attack), or
2. Rewrite both files in a way that doesn't trigger TLS cert
   validation on the second connection (rustls won't downgrade or
   ignore cert errors silently), or
3. Forge the trusted root and serve their own cert (already in scope
   if they're MITM-ing, but now they have to do it consistently for
   two requests instead of one — much higher likelihood of triggering
   the user's other security tooling).

This isn't code-signing protection (we don't verify a signature; we
verify a hash that an attacker who controls the proxy in steady state
could in principle replace alongside the binary). But it raises the
attack cost meaningfully, and it matches the security posture of
cargo-dist's own shell installer (`curl ... | sh`), which the
PowerShell installer one-liner already inherits via cargo-dist.

**Rejected alternative: code signing.** Would be stronger, but
acquiring an EV cert and threading signing through release.yml +
windows-installers.yml is a multi-month project. SHA256 sidecar
verification is the 90% solution that ships in days.

### Post-install version verification

**Problem.** `msiexec /i ... /passive /norestart` exits 0 in several
scenarios where the install didn't materially complete:
- MajorUpgrade matching the *same* version (re-running v3.15.2 install
  with v3.15.2 already installed): exits 0, no file changed.
- Restart Manager scheduled deferred file replace (exit 3010 —
  REBOOT_REQUIRED, file isn't actually swapped until next reboot).
- `/passive` mode: msiexec may exit 0 immediately when another `msiexec`
  is running and the new install is queued.

JSON consumers reading `tr300 update --json` would see
`"success": true, "strategy": "msi_global"` — false positive, fleet
automation thinks the rollout is done when the on-disk binary is still
old.

**Fix.** After `msiexec` / Inno EXE returns success, re-exec
`env::current_exe() --version` via a fresh `Command`, parse the
version, compare against the expected `latest`. Mismatch → return
`StrategyError::Runtime` with a clear message. msiexec exit code 3010
is intercepted BEFORE the version check and surfaces as a dedicated
"Reboot, then verify with `tr300 --version`" message — operators can
distinguish "needs reboot" from "actually failed" from the JSON
`attempts[].message` field.

**Why not re-exec `cargo install --list` or similar.** The cargo path
doesn't apply (these are MSI/EXE strategies, not cargo). And the
on-disk binary's `--version` output is the source of truth: it's
literally the bytes that will run next time the user invokes `tr300`,
so confirming THAT version equals expected is the only verification
that actually matters.

### WMI hard-timeout pattern

**Problem.** Windows WMI queries (`wmi::WMIConnection::query()`) have
no Rust-side timeout. The bounded-subprocess helper in
`src/collectors/command.rs` only protects `Command::output()` calls.
WMI bypasses it entirely. A deadlocked WMI provider — post-Windows-
update misconfig, Group Policy lockdown of the security namespace,
antivirus interfering with `Win32_EncryptableVolume` queries, broken
`Winmgmt` service — could block the report for tens of seconds with
no indication of why.

**Fix.** `src/collectors/platform/windows.rs::with_timeout(budget, f)`:

```rust
fn with_timeout<F, T>(budget: Duration, f: F) -> Option<T>
where
    F: FnOnce() -> Option<T> + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || { let _ = tx.send(f()); });
    rx.recv_timeout(budget).ok().flatten()
}
```

The 5-second `WMI_TIMEOUT` constant is the budget. All four WMI-touching
collectors (`get_bitlocker_status`, `get_socket_count_wmi`,
`get_network_info_wmi`, `last_cold_boot_seconds`) wrap their inner
logic in this. The 5th (`get_battery_wmi(&WMIConnection)`) takes a
non-`'static` reference so can't be wrapped this way — but
`get_battery_native()` via `GetSystemPowerStatus` covers the same
ground in ~1 ms and runs first.

**The detached-thread tradeoff.** Rust intentionally doesn't kill
threads. When `with_timeout` returns `None`, the worker thread is
still blocked on its WMI call and continues running in the background.
There's no resource leak in practice: when our process exits,
Windows tears down all threads. The COMLibrary created inside the
worker thread's closure has the same lifetime as the thread, so it
goes too. The cost is a tiny amount of orphan background work per
timeout-firing WMI call — bounded, paid for by the OS, not our
problem.

**Rejected alternative: tokio + spawn_blocking + timeout.** Async
runtime drag for a single sync wait operation. Not worth pulling in
the entire tokio crate.

**Rejected alternative: a global watchdog thread that kills the
process after N seconds.** Too coarse — would terminate the entire
report even when only one collector is hung, and there's no way to
return the data that WAS collected before the timeout.

### Windows self-EXE delete via detached cleanup

**Problem.** `tr300 uninstall` -> Complete called `fs::remove_file`
on `find_binary_location()`, which prefers `env::current_exe()`. When
the user ran `tr300 uninstall` from `%LocalAppData%\Programs\tr300\
bin\tr300.exe`, that path IS the running EXE. Windows refuses
`DeleteFile` on a running EXE image with raw OS error 5 because the
loader holds an open file handle. The user was left with profile
cleaned, binary still on disk, and a confusing `RemoveBinary` error.

**Fix.** `is_running_binary(target)` canonicalizes both `target` and
`env::current_exe()` via `fs::canonicalize` (handles 8.3 short names,
junctions, drive-letter case) and compares for equality. When the
running binary IS the target, `schedule_self_cleanup` spawns a
detached `cmd.exe` job:

```
cmd /c timeout /t 2 /nobreak > nul & del "<binary>" & rd /q "<parent>"
```

With `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` creation flags so
the child has no console window and doesn't inherit our process
group. The 2-second wait gives our parent process time to exit and
release the file handle; `del` then succeeds.

**Why not `MoveFileEx(path, NULL, MOVEFILE_DELAY_UNTIL_REBOOT)`.**
Works but requires the user to reboot before the binary is actually
gone, and `MOVEFILE_DELAY_UNTIL_REBOOT` requires admin on Windows 11
(MS hardened this in 22H2 to prevent ransomware abuse). The detached
cleanup gives sub-3-second cleanup without elevation.

**Why `cmd.exe` rather than PowerShell.** `cmd.exe` is always
present on Windows; PowerShell 5.1 is usually but not always
(Server Core, stripped images). `cmd` has no startup overhead;
`pwsh -Command` has measurable JIT warmup. For a 2-line cleanup
script, cmd wins.

**Parent-dir cleanup heuristic.** Only `rd /q` the parent if it
contains "tr300" in the name (case-insensitive) — matches the
synchronous-path heuristic, prevents wiping unrelated dirs in
unusual portable-install scenarios.

