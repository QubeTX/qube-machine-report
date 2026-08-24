# CODEX_PROJECT.md

## TL;DR

TR-300 is a standalone Rust CLI and library that produces compact, fixed-width
machine reports on macOS, Linux, and Windows. The v4 release line hardens
cross-platform facts, makes report persistence explicit-only, fails updates
gracefully under endpoint policy, and enforces Developer ID signing plus Apple
notarization. v4.1 added origin-preserving updates and a native universal
package verified on hosted Apple Silicon and Intel. v4.2.2 is the completed
MIC-1 release: managed CLI installers are the recommended default, fresh
installer intent is authoritative, and the Mac native artifact is a direct PKG
with a compatibility-only DMG bridge. v4.3.0 is an unreleased candidate in open
PR #14. Repairs plus repaired-code local/hosted PR gates and review pass, and
the operator requires the PR to remain open after validation.
Alienware Windows evidence is captured; AMD64 Linux laptop and Raspberry Pi 4
checks remain separate and open.

Start the next session with
[`docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md`](./docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md),
then `AGENTS.md`, `CLAUDE.md`, `MASTER_PLAN.md`, and `TESTING.md`.

## Current Status

- Cargo package / binary / library import: `tr300`
- Complete GitHub distribution and crates.io package: `4.2.2` (2026-07-18).
  Exact source `db0f538c82961569a7118b105a20e967b15476f0` passed exact-SHA
  CI/crates, all six release targets, signed Apple archives, every Windows
  package/transition job, native Intel/ARM direct-PKG and compatibility-DMG
  lifecycles, and the fresh 34-asset public-byte audit.
- Working manifest / candidate: `4.3.0` on
  `feature/v4.3.0-battery-perf-thermals`, open PR #14. It is unreleased and
  repaired-code local gates pass; head `56b92d7` passed CI 32680492930 and
  PR-mode Release plan 32680492910 with all four review threads resolved. The
  operator has withheld merge authority even after validation.
- The v4.3 candidate adds deterministic, fault-aware hottest-valid Linux
  CPU/GPU thermals (including `soc_thermal`), mode-bounded NVIDIA GPU thermals
  on Windows while Windows CPU stays absent/JSON `null`, no macOS thermals, and
  stricter Linux/macOS battery identity. Linux battery corroboration includes
  `*_avg` and valid signed current/power readings; the historical Raspberry Pi
  "30%" source is still unproven. Seven alternating Windows full runs produced
  medians of 5138.6 ms versus 2092.3 ms (~3046.3 ms, 59.3%, 2.46×); 11
  alternating fast runs at 247.0 ms versus 238.9 ms (-3.3%) are background-
  level and carry no performance-gain claim.
- Homepage commit `4829c4430ee917bcb1508c2ea7ac87988ba5e055` is live at
  `https://reports.qubetx.com/` for the published v4.2.2 distribution. Do not
  advertise v4.3.0 before its release gates and publication complete.
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
  2026-08-23. It distinguishes the accepted, published v4.2.2 decisions from
  the unreleased v4.3 candidate's thermal, battery, and bounded-probe rationale,
  and retains the one-product/mode/output, MIC-1, `main`, checkout-v6, and
  evidence-boundary contracts.
- Release tooling: cargo-dist `0.31.0`
- Last physical-Mac source verification: 2026-07-15 on a MacBook Pro M2,
  macOS 26.3.1 build 25D2128. Published v4.2.2 hosted Installer-identity and
  workflow evidence was reconciled 2026-07-18; v4.3 native Mac gates remain
  pending and the candidate has no thermal collection.

### v4.3.0 open thermal/battery/latency candidate

- Linux thermal collection is pure sysfs in both modes. It deterministically
  chooses the hottest plausible healthy reading within the preferred sensor
  class, rejects faulted/malformed channels, recognizes `soc_thermal`, and
  preserves absence when no trusted CPU/GPU sensor answers.
- Windows thermal collection is NVIDIA GPU-only through `nvidia-smi`, gated on
  detected NVIDIA hardware and bounded by the active collection mode. Windows
  CPU temperature remains absent/JSON `null`; ACPI thermal zones are rejected
  because their instance identity cannot prove a CPU sensor. macOS reports no
  thermals.
- Linux battery selection recognizes `*_avg` attributes and signed current or
  power readings while rejecting contradictory device/absence evidence. The
  macOS `pmset` fallback requires `-InternalBattery-N`. These are hardening
  decisions, not proof of the historical Raspberry Pi symptom's source.
- Seven alternating Windows full-mode runs produced medians of 5138.6 ms before
  and 2092.3 ms after (~3046.3 ms, 59.3%, 2.46×). Eleven alternating fast runs
  produced 247.0 ms and 238.9 ms medians; the apparent -3.3% is background-level
  and supports no fast-mode performance claim.
- PR #14 is open and must remain unmerged by direct operator instruction. The
  repaired candidate's local/hosted gates and review pass. Physical AMD64 Linux
  and Raspberry Pi checks are independent evidence debts.

### v4.2.2 managed-install/direct-PKG release closure

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
- A deliberately launched fresh installer is the user's newest channel intent,
  including same-version or downgrade repair; automatic updates remain latest-
  only. Same-edition MSI/Inno format changes remove the exact competing product.
  Opposite-edition native packages stop before mutation and point to the managed
  PowerShell path, which can request UAC for exact cross-scope convergence.
- The native macOS artifact is the direct universal signed, notarized, stapled
  `tr300-universal-apple-darwin.pkg`. It owns `/usr/local/bin/tr300` and the
  `com.qubetx.tr300.pkg` receipt. The DMG remains only for immutable v4.1.x
  clients and contains a byte-identical PKG. Current updaters download the
  direct exact-tag PKG/sidecar and wait for Apple Installer.
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
   `Cargo.lock`, generated man page, and the approved docs synchronized at
   `4.3.0`, with both changelogs explicitly labeled `Unreleased` until the
   release date is real.
2. Preserve the completed review repairs on the feature branch. PR #14 must
   remain unmerged by operator instruction; an earlier green run is not evidence
   for the repaired head.
3. Keep the complete locked local gate set green on the repaired tree: fmt, warnings-
   denied Clippy, all-target tests, package list, publish dry-run, release build,
   audit, cargo-dist plan, workflow/script checks, and required runtime smokes.
   These gates passed on exact repaired source/docs head `56b92d7`.
4. Exact repaired-head CI 32680492930 and PR-mode Release plan 32680492910 pass,
   and all four review threads are resolved. Leave the PR open. Merge/default-
   branch/release work requires separate operator authorization and remains in
   task `#r43`.
5. Create and push only tag `v4.3.0` after main CI and crates publication settle.
   Existing immutable v4 tags must not move.
6. Require both hosted Apple jobs to sign and receive Notary `Accepted`; verify
   extracted signatures/checksums, then require supplemental native Mac and
   Windows installer/transition workflows.
7. Audit the final public assets, stable `latest` entrypoints, rendered wrappers,
   and release metadata. Only then update, test, commit, and push the homepage.
8. Keep AMD64 Linux/Pi physical tasks open and report them as open evidence
   rather than inferring hardware success from fixtures or hosted runners.

Published v4.2.2 runs: CI 29664547910, crates 29664653519, Release 29664688035,
native macOS 29664824418, Windows packaging 29664824432, and Windows transition
validation 29664948031. The current v4.3.0 candidate has no publication run or
release evidence yet.

Never publish locally merely because a token exists. Never tag before the
default-branch CI and crates workflow settle.

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
│   ├── managed-installers
│   │   ├── tr300-installer.ps1
│   │   └── tr300-installer.sh
│   └── sign-notarize-macos.sh
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
