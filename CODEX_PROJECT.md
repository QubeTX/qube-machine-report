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
bridge. v4.3.12 is the current complete trusted-OIDC/GitHub distribution. The
immutable v4.3.0 through v4.3.11 tags failed to complete GitHub distribution:
first on a Windows release-bootstrap guard, then on Bash-4-only
syntax in fresh native Apple signer jobs, then on a checksum guard that did not
model cargo-dist 0.31.0's two-LF raw sidecars, and finally on a tar-mode guard
that did not accept the producer's matching POSIX file-type bits. v4.3.4
repaired that boundary and passed Apple signing/notarization, but its
uncredentialed asset-preparation filter indexed a boolean instead of the
release manifest and stopped before hosting. v4.3.5 repaired that boundary,
created the correct private 24-asset draft, then stopped when GitHub's by-tag
endpoint returned 404 for that draft. v4.3.6 repaired draft discovery and
created its own exact private 24-asset draft, then stopped after hosting when
the managed-installer hash lookup expected a binary-mode marker that default
`sha256sum` does not write. v4.3.7 repaired that lookup and created its exact
private 24-asset draft, but the read-only Windows and macOS downstream tokens
lacked push access and GitHub therefore omitted the draft from their
authenticated release listings; both source resolvers deterministically saw
zero releases and stopped before any downstream build or publication. v4.3.8
repaired that boundary, passed exact-main CI/OIDC, native preflight, and all 13
Release jobs, then created exact private 24-asset draft `376357745`. Its macOS
consumer rejected the trusted freezer's intentional six-file Apple/source
inventory because it still required four archive files; its Windows Inno
Setup attestation step lacked `GH_TOKEN`. Both failed before publication and
the draft remained private at exactly 24 assets. v4.3.9 repaired those two
contracts, passed exact-main CI/OIDC, native preflight, Release, and Windows
assembly, and produced exact private 30-asset draft `376401700`. Its Mac
finalizer then stopped before credential use on locale-sensitive checksum-name
ordering, while private Windows validation asked the production Global worker
to fetch an intentionally private exact-tag URL. v4.3.10 repaired those two
defects and reached exact private 30-asset draft `376446284`; its signed and
notarized Mac artifacts then stopped only because native validation expected
PackageKit to repeat a script-owned human message. PR #28 repaired that exact
assertion. v4.3.11 repaired that boundary and reached an exact private 30-asset
draft; its signed/notarized packages passed the real rejection before test
cleanup could not unlink its fixture from root-owned `/Users`. v4.3.12 fixed
that cleanup and completed the entire release chain for
the unchanged performance/battery/thermal work developed through PR #14.
Alienware Windows evidence is captured;
AMD64 Linux laptop and Raspberry Pi 4 checks remain separate and open.

Start the next session with
[`docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md`](./docs/agents/handoff/2026-07-14-002-v4-release-and-personal-fleet-continuation.md),
then `AGENTS.md`, `CLAUDE.md`, `MASTER_PLAN.md`, and `TESTING.md`.

## Current Status

- Cargo package / binary / library import: `tr300`
- Current crates.io package and complete GitHub distribution: `4.3.12`, exact
  source/tag `19246b76f39c53340e6be62a332cedca9bca766c`. Exact-main CI
  `32869031682`, trusted-OIDC publication `32869029189`, native preflight
  `32869891315`, Release `32869971805`, Windows Installers `32870555353`,
  private Windows validation `32870926989`, macOS finalization `32870555348`,
  and post-public Windows/updater validation `32871841072` passed. Release
  `376540890` has exactly 34 nonempty digest-bearing assets and is `latest`.
  The earlier
  immutable `v4.3.0` through `v4.3.4` tags produced no draft; v4.3.5 and v4.3.6
  own private drafts `376242296` and `376283574`; v4.3.7 owns `376309349`.
  None of the prior twelve tags may
  move, be deleted, or be reused.
- Working manifest and current release: `4.3.12`, carrying the
  product code merged through PR #14 as `2f997d2`. It adds
  deterministic fault-aware hottest-valid Linux CPU/GPU thermals (including
  `soc_thermal`), `*_avg` plus valid signed/zero Linux battery corroboration,
  recognizable-`InternalBattery`-only macOS fallback, mode-bounded NVIDIA GPU
  thermals on Windows while CPU stays absent/JSON `null`, and `-f/--full`.
  Seven alternating full runs per version measured 5146.9 ms versus 2120.2 ms
  (~58.8%, 2.43×); 11 fast runs per version measured 257.3 ms versus 260.4 ms
  (+1.2%), so no fast-mode gain is claimed. The v4.3.1 release bootstrap is
  shared with a live Windows Server 2022 pre-tag CI job and rejects actual
  reparse points without rejecting ordinary hard links. v4.3.2 additionally
  removes Bash-4-only staging constructs from native Apple jobs and exercises
  their exact staging/syntax contract under macOS system Bash before any tag is
  eligible. v4.3.3 validates cargo-dist's exact raw checksum bytes before
  canonicalizing the public sidecars and aggregate. v4.3.4 accepts only the
  producer's safe full POSIX or equivalent permission-only tar modes in both
  Apple extraction boundaries. v4.3.5 preserves the validated cargo-dist plan
  object through release-metadata predicates instead of allowing boolean
  filter context to replace it. v4.3.6 binds the created draft by its immutable
  release ID rather than a public-only by-tag lookup. v4.3.7 corrects the
  managed-installer checksum-record lookup without weakening any draft or asset
  invariant. v4.3.8 isolates push-capable private-draft discovery/freezing in
  fresh no-checkout jobs while keeping every checked-out builder and candidate
  execution job read-only. v4.3.9 reconciles the downstream Apple consumer
  with the freezer's six-file inventory and restores read-only `github.token`
  only to the Windows Inno attestation step, then removes and verifies removal
  of `GH_TOKEN` before launching the third-party installer. v4.3.10 replaces
  the locale-sensitive Mac checksum-name comparison with an order-independent
  exact parser, proves frozen Global repairs while the draft is private, and
  runs the real network-backed Global worker only after publication. PR #28
  then binds the script-owned managed-conflict guidance directly and accepts
  PackageKit's generic preinstall failure only when unchanged-state checks also
  pass. The automated
  PR-to-public chain and its least-privilege boundaries remain intact. AMD64
  Linux laptop and Raspberry Pi physical acceptance remain open.
- Homepage commit `4829c4430ee917bcb1508c2ea7ac87988ba5e055` is live at
  `https://reports.qubetx.com/` with the v4.2.2 managed/native distribution.
- Personal-fleet evidence: Alienware report/hardware and v4.1.3 same-channel
  evidence are real; the natural v4.1.3 → v4.2.2 UAC update remains open. Never
  claim the AMD laptop or Pi 4 is verified until its board task contains real
  evidence.
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
  2026-08-25. It includes the v4.3 collector candidate, preinstall-only Mac
  managed-conflict refusal and receipt-aware transition, trusted executable/CWD rules, exact stable release provenance,
  protected environments, automatic OIDC crates publishing, and the
  private 24→30→validated→34 publication chain.
- Release tooling: cargo-dist `0.31.0`
- Last physical-Mac source verification: 2026-07-15 on a MacBook Pro M2,
  macOS 26.3.1 build 25D2128. Hosted Installer-identity proof and
  documentation/workflow state reconciled 2026-08-25.

### v4.2.2 complete-distribution baseline and v4.3.12 fix-forward

- v4.3 Linux battery corroboration accepts standard `_now`/`_avg` voltage,
  current, power, charge, and energy signals, including signed discharge and
  valid zero readings. Thermal selection is fault-aware, deterministic, and
  picks the hottest plausible CPU/GPU candidate including `soc_thermal`.
- v4.3 Windows consolidates WMI/registry/network/process work and applies
  launch-relative probe deadlines. NVIDIA temperature uses bounded
  `nvidia-smi`; Windows CPU temperature remains absent/null because ACPI zones
  cannot be mapped reliably. macOS accepts only a real `InternalBattery` record.
- Seven alternating Windows full-mode runs per version produced medians of
  5146.9 ms before and 2120.2 ms after (~3026.6 ms, 58.8%, 2.43×). Eleven
  alternating fast runs per version produced 257.3 ms and 260.4 ms medians; the
  apparent +1.2% is background-level and supports no fast-mode performance
  claim.
- The release-security candidate keeps every stable release private at 24 base
  assets, extends it to 30 Windows assets, proves those exact bytes through
  private fresh-install plus authenticated direct prior-to-candidate transition
  checks while public `latest` stays unchanged, and lets only the fresh macOS
  finalizer expose the exact 34-asset result. The real updater-to-candidate
  matrix runs post-public; crates publication automatically follows successful
  exact-main CI through protected OIDC.
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
  Exact-main CI then passed. PR #14 head `8f5919b` passed the complete local,
  hosted, security, benchmark, and independent-review gates before merge as
  `2f997d2`; exact-main CI run `32766014047` passed all 19 jobs.
- PR #18 exact head `4979636` passed all 20 hosted CI jobs in `32799513518`,
  release plan `32799513464`, independent review, and every review thread. It
  merged as `07e0e3ae`; exact-main CI `32800131846`, automatic trusted-OIDC
  crates publication `32800131893`, and native credential preflight
  `32800830054` passed. Immutable tag `v4.3.1` then exposed the Apple system-
  Bash compatibility gap in Release `32800944635` before any draft existed.

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
- The v4.3.12 release has 34 stable-name assets. Exact trusted-OIDC crates
  publication, private Windows byte/matrix proof, both Apple-native lifecycles,
  finalization, post-public smokes, and the public-byte audit passed. The live
  homepage already uses versionless `releases/latest` links. Physical AMD64
  Linux and Pi qualification stay separately open.

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

1. Preserve `4.3.12` as the current complete GitHub/crates boundary and
   `v4.3.0` through `v4.3.11` as immutable failed GitHub-distribution tags. The
   first five produced no draft; v4.3.5 through v4.3.8 produced exact private
   24-asset drafts, and v4.3.9/v4.3.10/v4.3.11 produced exact private 30-asset
   drafts. Keep
   `Cargo.toml`, `Cargo.lock`, generated man page, and the full docs set
   synchronized at `4.3.12`; date the release-note blocks on the final
   release source commit while keeping status ledgers explicit.
2. Run locked fmt, clippy, tests, native Apple Silicon/Intel release builds and smokes,
   package list, publish dry-run, security audit, cargo-dist plan, actionlint,
   shellcheck, Windows installer fixtures, and archive plus direct-PKG/DMG
   sign/notary/staple/install proof.
3. Prepare the final release-source correction on a focused PR, rerun the
   complete local gate, and merge only after every required check and review
   thread passes. Earlier-head checks are not evidence for the merge result.
4. Let the exact-main push start `CI` and `crates-publish.yml`; the publisher
   must wait without a registry credential for that exact CI result before exact package bytes
   plus trusted OIDC/public provenance are accepted.
5. For a future release, create and push only its new tag after exact-main
   CI/crates proof and native Apple Bash-3.2 staging preflight. Existing
   immutable `v*` tags, including v4.3.12, must not move.
6. Require `release.yml` to create the private 24-asset draft, Windows Installers
   to produce 30, and private Windows Installer Validation to attest those exact
   bytes and pass every channel/transition gate.
7. Require both native Apple jobs and exact proof custody before the macOS
   finalizer adds four assets and solely publishes 34. Then require public
   Windows updater and published Linux/macOS smokes plus the public-byte audit.
8. Update the homepage only when its source needs a release-specific change;
   its current versionless `releases/latest` links required no v4.3.12 source
   deployment. Keep AMD/Pi physical tasks open and patch forward from findings.

Published v4.2.2 runs: CI 29664547910, crates 29664653519, Release 29664688035,
native macOS 29664824418, Windows packaging 29664824432, and Windows transition
validation 29664948031. v4.3.0 OIDC/CI evidence is `32794371283`/
`32794371259`; failed GitHub Release run `32795846831` created no draft. The
v4.3.1 exact source `07e0e3ae` passed CI `32800131846`, automatic OIDC crates
run `32800131893`, and native credential preflight `32800830054`; its immutable
tag then triggered failed Release `32800944635`, which also created no draft.
The v4.3.2 exact source `c246ded2` passed CI `32809616793`, automatic trusted-
OIDC crates publication `32809616807`, and native preflight `32810343902`.
Its immutable tag then triggered Release `32810420213`; both Apple staging jobs
failed before credential use because the guard expected one LF instead of the
  pinned producer's two-LF sidecar, and no draft exists. The v4.3.3 exact source
  `cf1ac838` passed CI `32814328977`, automatic trusted-OIDC crates publication
  `32814329178`, and native preflight `32815263338`. Its immutable tag then
  triggered Release `32815338720`; both Apple staging jobs rejected the
  producer's safe embedded POSIX file-type bits before credential use, and no
  draft exists. PR #21 exact head `a631e971` passed CI `32819974330` and
  Release plan `32819974234`, then merged as exact source `ed22545e`. Exact-main
  CI `32820725614`, automatic trusted-OIDC v4.3.4 publication `32820725583`,
  and native preflight `32821497928` passed; the unyanked public crate checksum
  is `b74d1aec64b44f5f7f56d284dcb89dd44f1d6b2aa18400e49adb2acb27cbd304`.
  Immutable Release `32821575317` passed its plan, all six builds, both Apple
  signer/notary jobs, and global artifacts, then its uncredentialed 24-asset
  preparation failed on a jq boolean-context error before upload, hosting, or
  draft creation. PR #22 exact head `f9aac4f7` passed CI `32823394621` and
  Release plan `32823394660`, then merged as exact source `42879b4a`. Exact-main
  CI `32823922777`, automatic trusted-OIDC v4.3.5 publication `32823922726`,
  and native preflight `32824834401` passed. Immutable Release `32824925889`
  created correct private 24-asset draft `376242296`, then host verification
  failed because the by-tag endpoint returns 404 for drafts. Every downstream
  workflow skipped and the draft remains private. PR #23 exact head `ebb4f76a`
  passed CI `32829236226` and Release plan `32829236145`, then merged as exact
  source `9ab392f4`. Exact-main CI `32830286853`, automatic trusted-OIDC v4.3.6
  publication `32830286932`, and native preflight `32830369613` passed.
  Immutable Release `32831292249` passed all builds, signers, global assembly,
  and preparation; host job `97751759346` created exact private draft
  `376283574`, then the managed-installer checksum lookup rejected default
  `sha256sum` field-two syntax. Every downstream workflow skipped and the draft
  remains private. PR #24 exact head `eebce78c` passed CI `32833632116` and
  Release plan `32833632174`, then merged as exact source `b9aa1285`.
  Exact-main CI `32834399658`, automatic trusted-OIDC v4.3.7 publication
  `32834399800`, and native preflight `32835377627` passed. Immutable Release
  `32835470143` passed all 13 jobs and created exact private 24-asset draft
  `376309349`. Windows `32835918254` and macOS `32835918239` failed before
  downstream build/publication because their read-only tokens lacked push
  access and could not see that draft in the authenticated release collection.
  The draft remains private. v4.3.8 then created exact private 24-asset draft
  `376357745`; its macOS run `32843468883` failed on six-file versus four-file
  inventory parity and its Windows run `32843468892` failed because the Inno
  attestation step lacked `GH_TOKEN`. Both publishers skipped and nothing
  became public. v4.3.9 then passed exact-main CI/OIDC, native preflight, all 13
  Release jobs, and Windows assembly, creating exact private 30-asset draft
  `376401700`. Its Mac finalizer stopped before credential use on locale-
  sensitive checksum-name ordering, and private Windows validation stopped the
  Global production worker because the exact-tag URL was not public. The
  v4.3.10 and v4.3.11 drafts remain private and immutable. v4.3.12 completed
  release at exact source/tag `19246b76f39c53340e6be62a332cedca9bca766c`.

Never publish locally merely because a credential exists. Never tag before
exact-current-main CI and its automatic trusted-OIDC crates run settle.

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
