# CODEX_PROJECT.md

## TL;DR

TR-300 is a standalone Rust CLI and library that produces compact, fixed-width
machine reports on macOS, Linux, and Windows. The v4 release line hardens
cross-platform facts, makes report persistence explicit-only, fails updates
gracefully under endpoint policy, and enforces Developer ID signing plus Apple
notarization for both Mac archives. macOS is comprehensively verified on Apple
Silicon and under Rosetta. Personal Alienware, AMD64 Linux laptop, and Raspberry
Pi 4 checks remain explicit post-release patch work by maintainer decision.

Start the next session with
[`docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md`](./docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md),
then `AGENTS.md`, `CLAUDE.md`, `MASTER_PLAN.md`, and `TESTING.md`.

## Current Status

- Cargo package / binary / library import: `tr300`
- Release/manifest version: `4.0.1` (2026-07-15). v4.0.0 published to
  crates.io but its immutable tag failed closed before GitHub artifact hosting;
  v4.0.1 is the deployed keychain-search fix-forward. Release source
  `b67ad083503d0fff840af8467015d05c659268ea` passed exact-SHA CI/crates,
  both hosted Apple jobs, the 28-asset public audit, and supplemental Windows
  packaging.
- Homepage commit `d77397479ad2b1189cce86b5402eaf1cc966abdf` is live at
  `https://reports.qubetx.com/` with the v4.0.1 persistence, accuracy,
  update-failure, and Mac trust contract.
- Personal-fleet evidence: post-release; never claim the Alienware, AMD laptop,
  or Pi 4 is verified until its board task contains real evidence
- Major-version reason: public Rust structs gained fields and selected public
  collector helpers changed signature. CLI and schema-v1 JSON compatibility are
  retained; changed record types are now `#[non_exhaustive]` for safer future
  additive fields.
- MSRV: Rust `1.95`, pinned in both `Cargo.toml` and `rust-toolchain.toml`
- Default branch: `main` (GitHub atomically renamed it from `master` on
  2026-07-17; new clones and release work must use `main`)
- Release tooling: cargo-dist `0.31.0`
- Last source/docs verification: 2026-07-15 on a MacBook Pro M2, macOS 26.3.1
  build 25D2128

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
   synchronized at `4.0.1`.
2. Run locked fmt, clippy, tests, native/Rosetta release builds and smokes,
   package list, publish dry-run, security audit, cargo-dist plan, actionlint,
   shellcheck, and real Mac archive sign/notary/repack proof.
3. Commit and push `main`; wait for `.github/workflows/ci.yml` to pass on the
   exact commit and for `crates-publish.yml` to publish that same SHA.
4. Create and push only tag `v4.0.1` after CI/crates settle. The immutable
   `v4.0.0` tag records the failed-closed first hosting attempt and must not move.
5. Require both hosted Apple jobs to sign and receive Notary `Accepted`; verify
   extracted signatures/checksums from both public Mac archives.
6. Verify cargo-dist's GitHub Release, the Windows Installers workflow, all four
   Windows installers, legacy aliases, and expected 28 assets.
7. Only then update, test, commit, and push the homepage repository.
8. Keep personal Alienware/AMD/Pi tasks open and patch forward from real findings.

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
│   ├── release.yml
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
│   └── global.iss
├── man/tr300.1
├── rust-toolchain.toml
├── scripts
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
