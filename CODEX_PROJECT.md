# CODEX_PROJECT.md

## TL;DR

TR-300 is a standalone Rust CLI and library that produces compact, fixed-width
machine reports on macOS, Linux, and Windows. The v4 release line hardens
cross-platform facts, makes report persistence explicit-only, fails updates
gracefully under endpoint policy, and enforces Developer ID signing plus Apple
notarization. v4.1 added origin-preserving updates and a native universal
package verified on hosted Apple Silicon and Intel. v4.2 is the MIC-1 candidate:
managed CLI installers are the recommended default, fresh installer intent is
authoritative, and the Mac native artifact is a direct PKG with a compatibility-
only DMG bridge. Alienware Windows evidence is captured; AMD64 Linux laptop and
Raspberry Pi 4 checks remain open.

Start the next session with
[`docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md`](./docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md),
then `AGENTS.md`, `CLAUDE.md`, `MASTER_PLAN.md`, and `TESTING.md`.

## Current Status

- Cargo package / binary / library import: `tr300`
- Published version: `4.1.3` (2026-07-18); working manifest / candidate:
  `4.2.0`. v4.1.3 passed exact-SHA CI/crates, signed archives, all Windows
  package/transition jobs, the native Intel/ARM PKG-in-DMG lifecycle, a
  30-asset public audit, and the Alienware's real Global MSI v4.0.1→v4.1.3 UAC
  transition. Its release source is
  `c5a25617b8b6438b1e7589e7518a1c1bd305ed64`. v4.2.0 is not published and
  must not inherit that evidence without its own exact-SHA gates.
- Homepage commit `d77397479ad2b1189cce86b5402eaf1cc966abdf` is live at
  `https://reports.qubetx.com/` with the v4.0.1 persistence, accuracy,
  update-failure, and Mac trust contract.
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
  2026-07-18. It covers the complete accepted v4/session decision surface and
  explicitly backfills the one-product/mode/output contracts, the historical
  Windows advisory cleanup and MIC-1's strict superseding behavior, plus the
  exhaustive `main`/checkout-v6 rationale and evidence.
- Release tooling: cargo-dist `0.31.0`
- Last physical-Mac source verification: 2026-07-15 on a MacBook Pro M2,
  macOS 26.3.1 build 25D2128. Hosted Installer-identity proof and
  documentation/workflow state reconciled 2026-07-18.

### v4.2.0 managed-install/direct-PKG candidate

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
- The v4.2.0 target is 34 stable-name assets. Required pending evidence is the
  full locked local gate set, exact-SHA CI/crates, disposable Windows managed/
  native matrices, both Apple-native direct-PKG/bridge lifecycles, public-byte
  audit, and homepage update.

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

1. Keep `Cargo.toml`, `Cargo.lock`, generated man page, and the full docs set
   synchronized at `4.2.0` while clearly labeling it a candidate.
2. Run locked fmt, clippy, tests, native Apple Silicon/Intel release builds and smokes,
   package list, publish dry-run, security audit, cargo-dist plan, actionlint,
   shellcheck, Windows installer fixtures, and archive plus direct-PKG/DMG
   sign/notary/staple/install proof.
3. Commit and push `main`; wait for `.github/workflows/ci.yml` to pass on the
   exact commit and for `crates-publish.yml` to publish that same SHA.
4. Create and push only tag `v4.2.0` after CI/crates settle. Existing immutable
   v4 tags must not move.
5. Require both hosted Apple jobs to sign and receive Notary `Accepted`; verify
   extracted signatures/checksums from both public Mac archives.
6. Verify cargo-dist's GitHub Release, both supplemental installer workflows,
   all four Windows installers, managed/legacy wrappers, direct PKG, bridge DMG,
   and expected 34 assets.
7. Only then update, test, commit, and push the homepage repository.
8. Keep AMD/Pi tasks open and patch forward from real findings.

Observed v4.0.1 runs: CI 29391956665, crates 29392101640, cargo-dist
29392185522, and Windows Installers 29392382949. The public tag/release targets
the exact source SHA above; the complete hashes and Apple submission IDs live
in `TESTING.md` and the tracked handoff.

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
