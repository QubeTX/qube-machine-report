# CODEX_PROJECT.md

## TL;DR

TR-300 is a standalone Rust CLI and library that produces compact, fixed-width
machine reports on macOS, Linux, and Windows. The v4 release line hardens
cross-platform facts, makes report persistence explicit-only, fails updates
gracefully under endpoint policy, and enforces Developer ID signing plus Apple
notarization. v4.1 added origin-preserving updates and a native universal
package verified on hosted Apple Silicon and Intel. v4.2.2 is the published
MIC-1 baseline: managed CLI installers are the recommended default, fresh
installer intent is authoritative only within a proven platform transaction,
and the Mac native artifact is a direct PKG with a compatibility-only DMG
bridge. v4.3.0 is an unreleased performance/battery/thermal and release-custody
candidate developed through PR #14. Alienware Windows evidence is captured;
AMD64 Linux laptop and Raspberry Pi 4 checks remain separate and open.

Start the next session with
[`docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md`](./docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md),
then `AGENTS.md`, `CLAUDE.md`, `MASTER_PLAN.md`, and `TESTING.md`.

## Current Status

- Cargo package / binary / library import: `tr300`
- Last fully published distribution: `4.2.2` (2026-07-18), source
  `db0f538c82961569a7118b105a20e967b15476f0`. Exact-SHA CI/crates, both signed
  Apple archives, all Windows packages/transitions, native Intel/Apple Silicon
  direct-PKG/compatibility-DMG lifecycles, and the public 34-asset checksum/
  stable-`latest` audit passed.
- Working manifest / unreleased candidate: `4.3.0` on
  `feature/v4.3.0-battery-perf-thermals`, developed through PR #14. It adds
  deterministic fault-aware hottest-valid Linux CPU/GPU thermals (including
  `soc_thermal`), `*_avg` plus valid signed/zero Linux battery corroboration,
  recognizable-`InternalBattery`-only macOS fallback, mode-bounded NVIDIA GPU
  thermals on Windows while CPU stays absent/JSON `null`, and `-f/--full`.
  Seven alternating full runs measured 5138.6 ms versus 2092.3 ms (~59.3%,
  2.46×); 11 fast runs measured 247.0 ms versus 238.9 ms (-3.3%), so no
  fast-mode gain is claimed. It remains unreleased; AMD64 Linux laptop and
  Raspberry Pi physical acceptance remain open.
- Homepage commit `4829c4430ee917bcb1508c2ea7ac87988ba5e055` is live at
  `https://reports.qubetx.com/` with the v4.2.2 managed/native distribution.
- Personal-fleet evidence: the Alienware Global MSI update and report/hardware
  facts now have real evidence; never claim the AMD laptop or Pi 4 is verified
  until its board task contains real evidence
- Major-version reason: public Rust structs gained fields and selected public
  collector helpers changed signature. CLI and schema-v1 JSON compatibility are
  retained; changed record types are now `#[non_exhaustive]` for safer future
  additive fields.
- MSRV: Rust `1.95`, pinned in both `Cargo.toml` and `rust-toolchain.toml`
- Default branch: `main` (GitHub atomically renamed it from `master` on
  2026-07-17; new clones and release work must use `main`)
- Branch-migration proof: commit
  `41c30b1e43f8abc5208f0d94702ed12cd91fb7a7` passed all 13 CI jobs in run
  29557626125 on `main`; downstream crates run 29557758673 succeeded by safely
  skipping already-published 4.0.1. Tags, public artifacts, Apple proof, and the
  production homepage were re-audited unchanged. The branch CI and crates
  workflows use `actions/checkout@v6` on Node 24, aligned with the release and
  supplemental Windows workflows. Follow-up commit
  `1714d1fc0b90475d5f0aa590b1ec7d93b24d2eee` passed all 13 jobs in CI run
  29559148638 with zero annotations; exact-SHA crates run 29559305341 safely
  skipped already-published 4.0.1 without token or publish access.
- Architecture ledger: `docs/architecture-decisions.md` is reconciled through
  2026-08-24. It includes the v4.3 collector candidate, preinstall-only Mac
  managed-conflict refusal and receipt-aware transition, trusted executable/CWD rules, exact stable release provenance,
  protected environments/rulesets, explicit OIDC crates publishing, and the
  private 24→30→validated→34 publication chain.
- Release tooling: cargo-dist `0.31.0`
- Last physical-Mac source verification: 2026-07-15 on a MacBook Pro M2,
  macOS 26.3.1 build 25D2128. Hosted Installer-identity proof and
  documentation/workflow state reconciled 2026-08-24.

### v4.2.2 published baseline and v4.3.0 candidate

- v4.3 Linux battery corroboration accepts standard `_now`/`_avg` voltage,
  current, power, charge, and energy signals, including signed discharge and
  valid zero readings. Thermal selection is fault-aware, deterministic, and
  picks the hottest plausible CPU/GPU candidate including `soc_thermal`.
- v4.3 Windows consolidates WMI/registry/network/process work and applies
  launch-relative probe deadlines. NVIDIA temperature uses bounded
  `nvidia-smi`; Windows CPU temperature remains absent/null because ACPI zones
  cannot be mapped reliably. macOS accepts only a real `InternalBattery` record.
- Seven alternating Windows full-mode runs produced medians of 5138.6 ms before
  and 2092.3 ms after (~3046.3 ms, 59.3%, 2.46×). Eleven alternating fast runs
  produced 247.0 ms and 238.9 ms medians; the apparent -3.3% is background-level
  and supports no fast-mode performance claim.
- The release-security candidate keeps every stable release private at 24 base
  assets, extends it to 30 Windows assets, proves those exact bytes through
  private fresh-install plus authenticated direct prior-to-candidate transition
  checks while public `latest` stays unchanged, and lets only the fresh macOS
  finalizer expose the exact 34-asset result. The real updater-to-candidate
  matrix runs post-public; crates publication is an explicit protected OIDC
  gate.
- Native Apple runners disproved automatic managed-to-PKG takeover: a failing
  `postinstall` can leave the native payload behind even when prior managed
  state restores exactly. v4.3 removes the helper and postinstall, checks
  standard managed binary/receipt paths in every `/Users` home, including
  dot-prefixed unregistered residue, plus all eligible local Directory Service
  homes non-mutatingly in `preinstall`, independent of console or launch
  environment. It enumerates only the fixed parent levels and rejects abnormal
  or unlistable intermediates rather than treating them as absent, with
  pre-macOS-12-compatible plist parsing and a `/`-only PackageKit target. It
  requires managed-installer refresh -> receipt-aware Complete uninstall ->
  PKG. Clean/native upgrades and exact PKG-to-managed takeover remain supported.
- Release-chain hardening head `8ea060f` passed its complete local gate, five
  zero-finding security scans, exact-head CI/release-plan runs, and native Intel
  and Apple Silicon PackageKit fixtures before PR #15 merged as `1ffb0cc`.
  Exact-main CI then passed. The earlier collector-only head passed its recorded
  gates; the integrated PR #14 result requires fresh exact-head proof.

- `tr300 update` preserves MSI/EXE edition and scope, Cargo, cargo-dist
  shell/PowerShell, or macOS PKG origin. Unknown/conflicting origins do not
  mutate the machine.
- Public commands/assets remain versionless; the updater resolves latest once
  and pins every payload, sidecar, installer script, and Cargo version to that
  immutable tag. Generated GitHub release notes are normalized to public
  `latest` links immediately before the release is created.
- MIC-1 recommends `irm .../tr300-installer.ps1 | iex` on Windows and
  `curl .../tr300-installer.sh | sh` on macOS/Linux. The rendered wrappers use
  one exact tag, invoke internal `tr300-dist-installer.*` cargo-dist scripts,
  verify the managed receipt/binary, and converge only exact recognized native
  ownership. Raw Cargo remains advanced/unmanaged because it cannot run a
  TR-300 post-install hook.
- Update JSON stays one stdout object and adds `install_channel`,
  `recovery_url`, and `requires_user_action` without removing existing fields;
  known-channel failures also expose the immutable `exact_installer_url`.
- Current Windows user-scoped channels rename the live image to a private
  sibling before replacement, verify the original path, restore on failure,
  and use the new binary for delayed backup cleanup. Legacy updater failure is
  valid only when it retains the old binary and returns recovery; a fresh exact
  same-channel install must then converge to one current registration.
- A deliberately launched fresh installer is the user's newest channel intent
  only inside a safe platform transaction; automatic updates remain latest-
  only. Same-edition MSI/Inno format changes remove the exact competing product.
  Opposite-edition native packages stop before mutation and point to the managed
  PowerShell path, which can request UAC for exact cross-scope convergence.
- The native macOS artifact is the direct universal signed, notarized, stapled
  `tr300-universal-apple-darwin.pkg`. It owns `/usr/local/bin/tr300` and the
  `com.qubetx.tr300.pkg` receipt. The DMG remains only for immutable v4.1.x
  clients and contains a byte-identical PKG. Current updaters download the
  direct exact-tag PKG/sidecar and wait for Apple Installer. The package ships
  only `preinstall`; standard-path managed binary or receipt evidence stops it
  before payload placement.
- Native GitHub `macos-15` and `macos-15-intel` runners are release gates; a
  physical Mac is optional visual smoke testing unless CI exposes a GUI-only
  defect. Installer-identity preflight run 29637224793 signed and verified a
  disposable PKG successfully on both architectures.
- Alienware validation confirmed the existing Global MSI v3.17.0 upgraded in
  place through v4.0.1 to v4.1.3 at the same Program Files path/registration;
  corrected hybrid topology reports `6P + 10E`, 16 physical, 22 logical cores.
- v4.2.2 published 34 stable-name assets after the full local/clean-tree gate
  set, exact-SHA CI/crates, disposable Windows managed/native matrices, both
  Apple-native direct-PKG/bridge lifecycles, public-byte audit, and homepage
  update passed. Exact run IDs and hashes are recorded in `TESTING.md`.
- The v4.3.0 release target remains 34 stable-name assets. Required pending
  evidence is the clean exact-commit local/hosted gate, explicit trusted-OIDC
  crates publication, private Windows byte/matrix proof, both Apple-native
  lifecycles/finalization, post-public smokes, public-byte audit, and homepage
  update. Physical AMD64 Linux and Pi qualification stay separately open.

### v4.0.0 feature set, released through the v4.0.1 fix-forward

- A single structured full-mode macOS snapshot supplies model, display, GPU,
  battery, boot-state, and virtualization facts with graceful fallbacks.
- Native arm64 and Rosetta x86_64 report the same hardware semantics; Rosetta
  is labeled explicitly and does not expose the translated 2.4 GHz compatibility
  value as a real CPU frequency.
- APFS root-volume and macOS memory figures use explicit, internally consistent
  definitions. Used plus available RAM equals total RAM.
- FileVault, battery, display, terminal, OS build/codename, core topology, locale,
  and last-login parsing have live and fixture coverage.
- JSON is built through `serde_json` while preserving schema version 1 and adds
  nullable/context fields without renaming existing keys.
- Optional commands drain both pipes, cap output, time out, and terminate their
  process tree/group best-effort.
- Ordinary reports create no report file. `-r`/`--report`/`-s`/`--save`
  invoke the existing collision-safe, symlink-resistant writer; `--no-save` is
  a hidden compatibility no-op.
- Updater payloads use private randomized staging, bounded downloads, explicit
  cleanup, and post-install version verification. Likely antivirus/Group Policy
  write or launch blocks stop the fallback chain, retain the current install,
  and return actionable failure without a direct-overwrite escape hatch.
- `scripts/sign-notarize-macos.sh` signs both cargo-dist Mac binaries with
  Developer ID/hardened runtime/timestamp, temporarily exposes only its
  ephemeral keychain to `codesign`, verifies the embedded certificate
  fingerprint, requires Apple `Accepted` before upload, repacks the exact
  bytes, and regenerates manifest/sidecar checksums.
- CI's macOS test/build/speed legs and RustSec audit are blocking again.
- Native and Rosetta final evidence includes complete suites, release binaries,
  full/fast JSON and table smokes, a 51-column non-UTF ASCII fallback, privacy,
  explicit-save/no-write behavior, updater checks, and a real archive
  Developer ID/notary/repack round-trip. Exact counts and run IDs live in
  `TESTING.md`.

### Post-release — do not mark complete without hardware evidence

- Live Windows report/install/update verification on the user's personal
  Alienware. Managed-work antivirus behavior is a separate endpoint-policy case.
- Live Linux AMD64 and Raspberry Pi 4/aarch64 report verification.
- SD-300 and Shaughv OS remain intentionally WIP-delisted on the homepage; do
  not restore their marketing links until their separate work is ready.

## Product and Architecture

The crate exposes both a binary (`src/main.rs`) and a public library
(`src/lib.rs`). `SystemInfo::collect_with_mode()` runs seven scoped collectors
in parallel, merges platform enrichments, then `src/report.rs` renders table,
JSON, or Markdown output. The terminal table remains 51 display columns wide
and uses `unicode-width` for alignment.

`CollectMode::Fast` is the shell-startup path. It keeps quick native/environment
facts and skips slow optional probes. The installed profile block invokes
`tr300 --fast`; the `report` alias and plain `tr300` use full mode. Optional
collector failure is represented as absence, not a fabricated value or a whole
report failure.

JSON schema version 1 is stable. Additive keys are allowed; key removal, rename,
or type change requires a schema bump. Current JSON names value provenance for
CPU load/frequency, disk used/available, and memory used/available so consumers
do not have to infer platform semantics.

## Release Contract

1. Preserve `4.2.2` as the last published boundary. Keep `Cargo.toml`,
   `Cargo.lock`, generated man page, and the full docs set synchronized at
   `4.3.0`, clearly labeled `Unreleased` until publication.
2. Run locked fmt, clippy, tests, native Apple Silicon/Intel release builds and smokes,
   package list, publish dry-run, security audit, cargo-dist plan, actionlint,
   shellcheck, Windows installer fixtures, and archive plus direct-PKG/DMG
   sign/notary/staple/install proof.
3. Resolve PR #14 on the current merge-result SHA, rerun the complete local
   gate, and merge only through protected `main` after every strict required
   check and review thread passes. Earlier-head checks are not evidence for the
   merge result; never push release work directly to `main`.
4. Explicitly dispatch the owner-only `crates-publish.yml operation=publish` and
   require exact package bytes plus trusted OIDC/public provenance. No push or
   `workflow_run` automatically publishes a crate.
5. Create and push only tag `v4.3.0` after exact-main CI/crates proof. Existing
   immutable `v*` tags must not move.
6. Require `release.yml` to create the private 24-asset draft, Windows Installers
   to produce 30, and private Windows Installer Validation to attest those exact
   bytes and pass every channel/transition gate.
7. Require both native Apple jobs and exact proof custody before the macOS
   finalizer adds four assets and solely publishes 34. Then require public
   Windows updater and published Linux/macOS smokes plus the public-byte audit.
8. Only then update/test/push the homepage through its own protected workflow;
   keep AMD/Pi physical tasks open and patch forward from real findings.

Published v4.2.2 runs: CI 29664547910, crates 29664653519, Release 29664688035,
native macOS 29664824418, Windows packaging 29664824432, and Windows transition
validation 29664948031. The current v4.3.0 candidate has no publication run or
release evidence yet.

Never publish locally merely because a credential exists. Never tag before
exact-current-main CI and the explicit trusted-OIDC crates operation settle.

The v4 release notes must include a concise Rust migration section: downstream
code should obtain `SystemInfo`/`Config` through collection/default APIs rather
than external struct literals, avoid exhaustive public-record patterns, and
account for the changed collector-helper return/signature contracts. Do not
misdescribe the CLI or additive JSON schema as breaking.

## Project Tree

Generated/ephemeral `.git/`, `target/`, and local ignored `.tasks/` contents are
excluded. The tracked project tree is:

```text
.
├── .agents
│   └── skills
│       └── release
│           └── SKILL.md
├── .claude
│   ├── hooks
│   │   └── edit-time-reminder.ps1
│   ├── settings.json
│   ├── settings.local.json
│   └── skills
│       ├── ATTRIBUTION.md
│       ├── architecture
│       │   ├── CONNECTORS.md
│       │   └── SKILL.md
│       ├── brainstorming
│       │   ├── SKILL.md
│       │   ├── spec-document-reviewer-prompt.md
│       │   └── visual-companion.md
│       ├── critical-thinking
│       │   ├── SKILL.md
│       │   └── references
│       ├── release/SKILL.md
│       ├── system-design/SKILL.md
│       ├── tr300-changelog/SKILL.md
│       ├── tr300-dev-workflow/SKILL.md
│       ├── windows-accuracy/SKILL.md
│       ├── windows-distribution-and-update/SKILL.md
│       └── windows-install/SKILL.md
├── .codex/config.toml
├── .firecrawl/polyform-nc-1.0.0.md
├── .github/workflows
│   ├── apple-secret-migration.yml
│   ├── ci.yml
│   ├── crates-publish.yml
│   ├── macos-installer.yml
│   ├── release.yml
│   ├── windows-installer-validation.yml
│   └── windows-installers.yml
├── .gitignore
├── AGENTS.md
├── CHANGELOG.md
├── CLAUDE.md
├── CODEX_PROJECT.md
├── Cargo.lock
├── Cargo.toml
├── HUMAN_CHANGELOG.md
├── LICENSE
├── MASTER_PLAN.md
├── README.md
├── TESTING.md
├── build.rs
├── docs
│   ├── agents/handoff
│   │   ├── 2026-07-14-001-macos-hardening-alienware-continuation.md
│   │   └── 2026-07-14-002-v4-release-and-personal-fleet-continuation.md
│   ├── architecture-decisions.md
│   └── thinking
│       └── 2026-07-14-tr300-v4-release-reliability.md
├── inno
│   ├── corporate.iss
│   ├── global.iss
│   └── remove-conflicting-msi.pas
├── man/tr300.1
├── rust-toolchain.toml
├── scripts
│   ├── build-sign-notarize-macos-installer.sh
│   ├── install-pinned-inno-setup.ps1
│   ├── managed-installers
│   │   ├── tr300-installer.ps1
│   │   └── tr300-installer.sh
│   ├── sign-notarize-macos.sh
│   ├── test-managed-installer-transaction.ps1
│   ├── test-managed-installer-transaction.sh
│   ├── test-release-workflow-provenance.py
│   └── verify-apple-installer-identity.sh
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
│   │   ├── shared.rs
│   │   ├── unix.rs
│   │   └── windows.rs
│   ├── lib.rs
│   ├── main.rs
│   ├── migrate.rs
│   ├── render
│   │   ├── bar.rs
│   │   ├── mod.rs
│   │   └── table.rs
│   ├── report.rs
│   └── update.rs
├── tests/integration.rs
├── wix/main.wxs
└── wix-corporate/corporate.wxs
```

## Local Task Board

The SHAUGHV board is intentionally local and gitignored at `.tasks/`. Its live
root is recorded in `.tasks/.board-server.json`; do not assume a port. It must
retain separate post-release tasks for personal Alienware and Linux/Raspberry
Pi validation, distinguish managed-work antivirus evidence, and keep release/
homepage status exact. The tracked handoff duplicates all resume-critical state
so a fresh Alienware clone does not depend on the ignored board directory and
clearly freezes the enforced Mac signing/notary path.
