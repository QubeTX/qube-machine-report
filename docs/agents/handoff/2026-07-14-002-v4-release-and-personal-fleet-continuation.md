# Handoff: v4 release and personal fleet continuation

**Date:** 2026-07-14–17 CDT
**Session:** 002
**Agent:** Codex
**Repository:** `QubeTX/qube-machine-report`
**Working directory:** `/Users/realemmetts/Downloads/temp_git/qube-machine-report`
**Default branch:** `main` (GitHub atomically renamed the former `master`
branch on 2026-07-17 without changing the source SHA)
**Release version:** `4.0.1` fix-forward (`v4.0.0` is immutable and failed
closed before GitHub artifact hosting)
**Prior pushed checkpoint:** `553dbd53a50982792030b518d7f5ca48fd3ba7de`
**Release commit:** `b67ad083503d0fff840af8467015d05c659268ea`
**Hosted run IDs:** CI 29391956665; crates 29392101640; cargo-dist 29392185522;
Windows Installers 29392382949
**Task IDs:** `#v400`, `#core`, `#plat`, `#test`, `#docs`, `#winhw`,
`#ship`, `#site`, `#brmain`

This is the exhaustive portable continuation record. The richer SHAUGHV task
board is local and gitignored; a fresh checkout must read this file, `AGENTS.md`,
`CLAUDE.md`, and `TESTING.md` before changing the v4 release.

## Session Narrative

The operator initially requested every safe improvement that would make TR-300
more reliable, stable, precise, and informative on every supported OS, while
keeping one consistent cross-platform Rust implementation. The real personal
fleet is Alienware Windows hardware, Apple Silicon MacBook Air/Pro hardware, an
AMD Linux laptop, and a Raspberry Pi 4. The operator authorized direct delivery
to the repository's default branch and release deployment, while requiring
thorough testing rather than using that authorization to bypass CI.

The preceding Mac checkpoint implemented a broad collector/runtime hardening
pass and proved native arm64 plus Rosetta x86_64 behavior on an M2 MacBook Pro.
It also classified the public Rust API changes as a major-version boundary and
left a tracked continuation in
`docs/agents/handoff/2026-07-14-001-macos-hardening-alienware-continuation.md`.
That checkpoint deliberately deferred the v4 version bump and release until
the other personal machines were available.

The operator then changed the release boundary: the personal Windows, Linux,
and Raspberry Pi verification may happen after v4.0.1, with forward patches as
needed. The managed work Alienware is useful only for the antivirus failure
case; it is not a substitute for personal Windows accuracy/installer evidence.
The local task board was updated so those hardware passes remain explicit
post-release tasks instead of silently becoming release prerequisites.

On the managed Windows machine, automatic report persistence was flagged by
endpoint antivirus because the ordinary report run created temporary/atomic
files. The settled product behavior is therefore:

- ordinary `tr300` and `report` runs only collect and print; they do not save;
- Markdown persistence is manual through `tr300 -r`,
  `tr300 --report`, `report -s`, or `report --save`;
- those four spellings use the existing collision-safe/symlink-safe report
  writer and are restricted to a full table report;
- hidden `--no-save` remains a compatibility no-op for old scripts.

The operator briefly considered a force-update prompt that would bypass atomic
backup behavior when antivirus blocked an update, then explicitly withdrew
that request. The settled updater rule is fail-safe and non-interactive:
probable antivirus, Group Policy, execution-policy, permission, or filesystem
write blocks terminate the strategy chain, preserve the current installation,
clean staging best-effort, exit 2, and provide diagnostics plus official manual
URLs. Do not add a force prompt, direct running-binary replacement, backup
bypass, or another write-heavy fallback after a policy block.

The operator clarified that “complete macOS now” means enforcing the Mac trust
path, not merely passing local tests. No Apple signing/notarization inputs were
previously visible in the repository. With explicit permission to use the
already-authenticated Chrome session, a least-privilege App Store Connect
Developer API key and a long-lived Developer ID Application certificate were
prepared. Credential values were stored only in GitHub Actions secrets and
local secure storage; none enter the repository, task board, handoff, logs, or
memory.

The cargo-dist release workflow is now fail-closed. Each macOS build signs the
extracted CLI binary with the Developer ID Application identity, hardened
runtime, secure timestamp, and stable identifier; submits that exact binary to
Apple; requires the result `Accepted`; repacks the archive; regenerates its
sidecar; and patches cargo-dist's local manifest checksum before upload.
Missing credentials, signing failure, timeout, a rejected/invalid result, or
checksum disagreement fails the Apple build job. There is no unsigned fallback,
including when a tag is pushed from Windows.

Multiple real Apple submissions proved the path locally. Most importantly, both
actual v4 release binaries completed the final script: arm64 submission
`c2afae62-1873-4337-8c88-1bbfa26c23eb` and x86_64 submission
`fe2dcc67-cfe1-49be-8d4c-59daf8697c61` returned `Accepted`. This final pass used
the SHA-1 fingerprint form stored in the GitHub repository variable, proving
the exact hosted identity input rather than only the human-readable display
name. Their extracted
binaries reported `tr300 4.0.0` and verified the Developer ID chain, expected
Team ID, hardened-runtime flag, and secure timestamp. Archive, sidecar, and
manifest agreed on SHA-256
`b1085dcc6e1bf5ce0e3a2fdeab0342cf4f4ae94506a007c2089a8a3db785a244`
(arm64) and
`703dfe22a8fbdc2b5bcb6e4bafce99b0e558f920ef1f8284774eca5ba2a34f30`
(x86_64). Earlier preliminary submissions
`7441ddc2-32e7-48bf-bbb3-b23f7fa504ef`,
`547d9426-6ab2-43a1-87df-e837aded786b`, and
`eda85d0f-b9ee-43bd-9891-df909fd3724e` were also `Accepted`.

The immutable v4.0.0 hosted run then revealed that local login-keychain state
had masked a clean-runner prerequisite: a new keychain must be on the user
search list for `codesign` even when `security find-identity` can enumerate it.
v4.0.1 fixes that lifecycle and verifies the embedded leaf certificate. Both
actual v4.0.1 cargo-dist archives passed the corrected path: arm64 submission
`52b52e88-8eb9-457b-bb01-6c39f01da913` and x86_64 submission
`e018b7e1-d16e-4a33-b2a2-5b62512652b5` returned `Accepted`. Their binaries
report `tr300 4.0.1`; archive/sidecar/manifest SHA-256 values are
`eb9be3c3afe19a6e6e07f6482f5fb1073e5d2407fd30fc449e362d76c41c59b9`
(arm64) and
`5549e3d26ddcd20b0ec74f11e083d94183b30ab6eaf4e80f9f42f3ac9610ec46`
(x86_64). The embedded leaf fingerprint matches the configured identity, and
the original user keychain search list was restored exactly after success and
after real timestamp/notary transport failures.

Apple does not provide a useful stapling path for a bare CLI binary inside a
tar.gz archive: `stapler` targets supported bundles/packages, and
`spctl --type execute` does not treat a standalone signed CLI as an app bundle.
Do not convert this into a false “must staple” gate. The precise evidence is
Developer ID signature verification on the exact uploaded binary plus Apple's
`Accepted` notarization result in the protected pre-upload job.

The operator was tired and asked for autonomous completion: finish all Mac work,
fully update `AGENTS.md` and `CLAUDE.md`, keep the board exact, push/deploy v4,
update and deploy the TR-300 homepage only after the release exists, preserve a
thorough tracked handoff and durable memory, and leave personal-fleet checks for
the later Alienware session.

## The Plan & Where It Stands

1. **Manual-only report persistence — implemented and locally covered.**
   `src/cli.rs` and `src/main.rs` contain the settled aliases/gates;
   `tests/integration.rs` verifies ordinary reports do not announce a save.
2. **Policy-blocked updater behavior — implemented and locally covered.**
   `src/update.rs` has typed blocked attempts, conservative detection,
   terminal chain behavior, cleanup context, human/JSON diagnostics, and
   injected-executor tests.
3. **Fail-closed Developer ID signing/notarization — implemented and proven
   locally; clean-runner discovery fix in v4.0.1.**
   `scripts/sign-notarize-macos.sh` and `.github/workflows/release.yml` contain
   the enforcement. v4.0.0's hosted attempt proved fail-closed behavior but
   exposed that a new keychain must be on the user search list for `codesign`.
   v4.0.1 temporarily adds/restores that keychain and verifies the embedded
   leaf-certificate fingerprint.
4. **Synchronize release docs, agent rules, release skills, board, and handoff —
   complete.** The full tracked doc set and local board describe the behavior,
   observed release outcome, deployed homepage, frozen Mac path, and remaining
   personal-hardware work.
5. **Run the comprehensive pre-release gate — complete.** Native and Rosetta
   Rust gates, runtime parity/privacy/save smokes, package/publish dry-run,
   audit, cargo-dist plan, workflow/script lint, Windows cross-check, external
   consumer smoke, and secret scans pass. The clean release commit repeated the
   39-file locked package/publish dry-run without changing the worktree.
6. **Push exact release commit and prove hosted source gates — complete.**
   v4.0.1 source commit `b67ad083503d0fff840af8467015d05c659268ea`
   passed CI 29391956665 and crates run 29392101640; crates.io serves the
   unyanked package from that exact SHA.
7. **Publish and verify release artifacts — complete.** Explicit tag `v4.0.1`
   points to the tested SHA. Cargo-dist 29392185522 and Windows Installers
   29392382949 succeeded, both hosted Apple jobs were `Accepted`, both public
   Mac archives passed signature/checksum/version checks, and the public release
   contains exactly 28 verified assets.
8. **Update/deploy the TR-300 homepage — complete.** Homepage commit
   `d77397479ad2b1189cce86b5402eaf1cc966abdf` is pushed to its default branch;
   production at `https://reports.qubetx.com/` serves the verified v4.0.1
   content with passing lint/build/link and Chrome desktop/mobile checks.
9. **Finalize board, handoff, durable memory, and cleanup — in progress only
   for closing bookkeeping.** Keep personal hardware tasks open, write the one
   requested ad-hoc memory note without secrets, remove only known temporary
   credential/test paths, and release the controlled Chrome tab.
10. **Personal fleet verification — explicitly post-release.** Retest personal
    Alienware Windows installers/collectors/update behavior, then the AMD Linux
    laptop and 64-bit Raspberry Pi 4. Patch forward from real findings.

## What Was Accomplished

### Product behavior

- `src/cli.rs` adds `-r/--report` plus visible `-s/--save` aliases,
  rejects fast/JSON/action combinations, and retains hidden `--no-save`.
- `src/main.rs` makes report persistence opt-in. Ordinary full, fast, JSON,
  ASCII, and alias runs no longer touch the report-save path.
- `tests/integration.rs` no longer needs `--no-save` for isolation and asserts
  the normal CLI does not report a saved file.
- `src/update.rs` adds `PolicyBlocked` and `AttemptKind::Blocked`, stops the
  strategy chain on likely endpoint/filesystem policy denial, preserves exit
  code 2, reports `"result": "blocked"` in JSON, adds official/manual URLs,
  and exposes cleanup diagnostics without promising an unsafe fallback.
- `src/collectors/disk.rs` contains a narrow documented Clippy allowance around
  libc filesystem-field casts whose concrete integer types differ by Unix
  target; this fixes the prior hosted Linux lint failure without weakening the
  workspace lint gate.

### Apple release trust

- `scripts/sign-notarize-macos.sh` is executable and Bash-3-compatible. It:
  validates every required input; creates a randomized private work directory
  and ephemeral keychain; imports only the release certificate; restricts the
  key partition list; resolves exactly one configured identity inside that
  keychain and signs by certificate fingerprint (avoiding duplicate-name
  ambiguity with a login keychain); extracts the cargo-dist archive; signs the
  `tr300` binary with identifier `com.qubetx.tr300`, hardened runtime, and
  timestamp; verifies authority, Team ID, identifier, runtime, and timestamp;
  submits with `xcrun notarytool`; requires `Accepted`; repacks
  exact archive contents; regenerates SHA-256; updates the matching
  `dist-manifest.json` checksum; verifies all three values; and deletes decoded
  credential material plus the ephemeral keychain on success or failure.
- `.github/workflows/release.yml` invokes that script after `dist build` and
  before cargo-dist chooses upload files for each Apple target. The step runs
  only for a publishing release and receives credentials from GitHub secrets
  and identity metadata from repository variables.
- Repository Actions secret names:
  `APPLE_CERTIFICATE_P12_BASE64`, `APPLE_CERTIFICATE_PASSWORD`,
  `APPLE_API_KEY_P8_BASE64`, `APPLE_API_KEY_ID`, and
  `APPLE_API_ISSUER_ID`. Repository variable names:
  `APPLE_SIGNING_IDENTITY` and `APPLE_TEAM_ID`. Values must never be printed,
  committed, copied into issues, or stored in board/memory files.
- The configured signing identity is a Developer ID Application certificate
  for team `M9D5379H93` with a 2031 expiry. The least-privilege Developer API
  key can submit notarizations. Do not rotate, revoke, recreate, or expose
  either from the Alienware unless the operator explicitly asks and the current
  hosted release proves the credentials invalid.
- `actionlint .github/workflows/release.yml`, `shellcheck
  scripts/sign-notarize-macos.sh`, and `bash -n
  scripts/sign-notarize-macos.sh` passed after implementation.
- A first final-script retry exposed that signing by display name was ambiguous
  when the same certificate existed in both the ephemeral and login keychains.
  No archive was repacked or submitted on that failed attempt. Resolving the
  sole ephemeral identity and signing by its SHA-1 certificate fingerprint
  fixed the issue. A final audit then caught that GitHub stores the configured
  identity itself as that fingerprint, not the display name. The script now
  validates either representation against its single imported identity, signs
  by the resolved fingerprint, and verifies the resolved authority. Both v4
  architecture submissions then passed with the exact GitHub variable form.

### Documentation and operating contract

- `CHANGELOG.md` and `HUMAN_CHANGELOG.md` contain matching v4.0.0 feature and
  v4.0.1 fix-forward technical/plain-English groupings.
- `README.md` documents manual report saving, fail-safe policy blocks, and
  Developer ID/notarized Mac releases.
- `AGENTS.md` and `CLAUDE.md` define the exact save/update behavior, Apple
  secret/variable names, no-unsigned-fallback contract, Mac freeze, release
  sequence, and post-release personal-hardware boundary.
- `CODEX_PROJECT.md`, `MASTER_PLAN.md`, `TESTING.md`, and
  `docs/architecture-decisions.md` are synchronized with v4 scope and
  rationale.
- `.agents/skills/release/SKILL.md` and
  `.claude/skills/release/SKILL.md` require audit/workflow/script gates,
  both Apple `Accepted` results, signature/checksum verification of both
  public Mac archives, the supplemental Windows workflow, and 28 final assets.
- `/Users/realemmetts/.codex/AGENTS.md` was updated with the same global TR-300
  release contract without any credential values.
- `docs/thinking/2026-07-14-tr300-v4-release-reliability.md` preserves the
  critical-thinking working canvas, formal release predicates, assumptions,
  alternatives, steelman, and evidence checkpoints used during the work.
- The local SHAUGHV board at `.tasks/` is live through the board identity file;
  it separates release gates from post-release personal Alienware/AMD/Pi tasks.

## Key Decisions

- **Keep one Rust product.** OS-specific code remains under cfg-gated
  collectors/installers; do not fork platform editions.
- **Release v4 now, patch from personal-hardware findings later.** This is an
  explicit operator re-scope. The remaining personal fleet tasks stay visible
  and are not implied to have passed.
- **Managed work antivirus is failure-mode evidence, not accuracy evidence.**
  Its policy block motivated read-only default reporting and safe updater
  failure, but it cannot close the personal Alienware hardware matrix.
- **Saving is explicit.** Even a non-atomic default write would still be an
  unexpected side effect and could trigger endpoint policy. All existing save
  safety remains behind the four explicit aliases.
- **`--no-save` parses but does nothing.** Removing it would break existing
  scripts for no user benefit; advertising it would misstate the new default.
- **Policy blocks are terminal.** Retrying alternate installers after a
  security product denies a staged write is more likely to cause repeated
  freezes/alerts than to help. The current installation wins over aggressive
  self-update.
- **No force/bypass path.** The operator explicitly rescinded that idea.
  Manual installation through official release/docs is the recovery path.
- **Apple release protection is pre-upload and fail-closed.** Local signing
  alone cannot ensure releases pushed from Windows are trusted. The workflow
  must enforce signature + `Accepted` on each Mac artifact before cargo-dist
  exposes it to the host job.
- **Notarize the exact CLI binary and repack.** Submitting an artificial app
  wrapper would notarize different bytes and would not prove the distributed
  tarball executable.
- **No stapling requirement for the tarred bare CLI.** Developer ID signature
  verification plus Apple's accepted submission is the accurate gate.
- **Do not regenerate cargo-dist casually.** `release.yml` has intentional
  compatibility aliases and Apple enforcement. Any regeneration must preserve
  both, then repeat native Mac proof and workflow linting.
- **Public Rust API changes justify 4.0.0.** CLI spellings and schema-v1 JSON
  remain compatible/additive, but direct struct literals/exhaustive patterns
  over newly non-exhaustive public types may need adaptation.
- **No secret material in git or durable task/memory systems.** Only secret and
  variable names, non-secret team identity, and Apple submission IDs may be
  recorded.

## How It Works

### Explicit report saving

Clap maps all four save spellings to `Cli::save_report`. Conflicts reject fast,
JSON, install/uninstall/update/migration, or hidden `--no-save` combinations.
`main::run_report` always collects/renders/prints first. It calls
`report::save_markdown_report` only when `save_report` is true, collection mode
is full, and output format is table. The existing save implementation discovers
Downloads, uses collision suffixes and create-new semantics, rejects/fails
safely around symlinks, cleans incomplete files, and reports a cwd fallback.

The alias name `report` is not used to distinguish behavior. Both executable
names invoke the same Clap parser, so `report -s` and `tr300 -s` technically
both work; the documented recommended pairs match the operator's request.

### Policy-blocked updater

Each strategy returns success, ordinary preflight/runtime failure, or
`PolicyBlocked`. Conservative classifiers recognize policy-oriented OS errors
and messages. A blocked result is recorded separately from failure, the
remaining strategy list is not executed, the randomized staging guard cleans
best-effort, and human/JSON output tells the user the installed version was
left untouched. Ordinary non-policy failures retain the existing bounded
fallback chain.

This does not make self-update transactional against an installer that mutates
state and then fails; that behavior belongs to the selected Cargo/MSI/EXE/cargo-
dist installer. TR-300 itself never directly replaces its running executable.

### macOS release enforcement

On an Apple cargo-dist job:

1. `dist build` creates the target archive, sidecar, and local manifest.
2. The workflow decodes credentials only into a randomized private directory
   and an ephemeral keychain.
3. The script extracts the archive and signs its `tr300` executable with the
   configured Developer ID Application identity, hardened runtime, stable
   identifier, and secure timestamp.
4. Local `codesign` verification must pass.
5. `notarytool submit --wait --output-format json` submits that exact executable
   and must report `Accepted`.
6. The archive is deterministically repacked around the exact original member
   paths, its `.sha256` sidecar is regenerated, and the matching artifact
   checksum in `dist-manifest.json` is updated.
7. Archive, sidecar, and manifest checksums are compared.
8. Only then does cargo-dist calculate upload paths. The decoded key/certificate,
   working files, and ephemeral keychain are removed by a trap.

Both `aarch64-apple-darwin` and `x86_64-apple-darwin` jobs independently run
that gate. A release is incomplete until the hosted logs show both accepted and
the downloaded public archives verify.

## Known Issues & Limitations

- Personal Alienware Windows accuracy/installer/update tests, AMD64 Linux
  hardware tests, and Raspberry Pi 4 aarch64 tests are intentionally not part
  of the v4.0.1 pre-release evidence. They are post-release tasks approved by
  the operator.
- The managed Windows endpoint may still block official installers or Cargo
  writes. TR-300 can fail safely and explain the manual path; it cannot override
  enterprise security policy.
- Antivirus/policy classification is necessarily heuristic. It is conservative
  and only changes behavior when the evidence resembles a write/launch/policy
  denial; unknown errors remain ordinary fallthrough failures.
- Apple notarization is online and credential-dependent. A release correctly
  fails if Apple is unavailable or credentials expire. Fix credentials or
  forward-bump; never weaken the gate to ship.
- The CLI tarballs do not carry a stapled app-bundle ticket. This is expected
  for the current artifact shape and is documented rather than hidden.
- GitHub Actions variables/secrets prove future Windows-triggered releases only
  when the checked-in workflow remains intact. Any edit to the Apple step,
  script, cargo-dist archive naming/manifest format, toolchain, dependency
  graph, or Mac target list reopens the Mac release gate.
- v4 is a Rust source migration for consumers constructing/matching the
  non-exhaustive public types directly. The high-level APIs, CLI commands, and
  existing schema-v1 JSON keys remain available.

## Important Context for Future Sessions

### Absolute macOS freeze for the personal Alienware continuation

The v4.0.1 Mac path is complete only as the combination of source behavior,
native/Rosetta evidence, Developer ID signing, Apple acceptance, and the
checked-in enforcement. From Windows, **do not change any of the following**
merely to clean up, generalize, regenerate, or make Windows/Linux code look
uniform:

- `src/collectors/platform/macos.rs`;
- macOS cfg branches in `src/collectors/cpu.rs`,
  `src/collectors/memory.rs`, `src/collectors/network.rs`,
  `src/collectors/os.rs`, `src/collectors/session.rs`,
  `src/install/unix.rs`, or `src/main.rs`;
- `scripts/sign-notarize-macos.sh`;
- the Apple step or any other release customization in
  `.github/workflows/release.yml`;
- the repository Actions secrets/variables, App Store Connect API key,
  Developer ID certificate, keychain identities, or Apple account settings;
- the `aarch64-apple-darwin` / `x86_64-apple-darwin` targets, archive names,
  shell installer names, cargo-dist configuration, `rust-toolchain.toml`,
  package/signing identifiers, hardened-runtime/timestamp flags, or checksum/
  manifest rewrite contract;
- shared runtime, renderer, schema, command-helper, dependency, build-script,
  toolchain, or distribution changes without treating them as Mac-impacting.

Windows/Linux work should stay inside their cfg-gated collectors, installers,
packaging sources, and tests. Release bookkeeping and Windows/Linux evidence
may be updated without reopening Mac proof.

Anything broader requires stopping before a new tag, returning to a Mac, and
rerunning: native arm64 and Rosetta x86_64 full tests/builds/smokes; workflow
and script lint; a real archive-through-script Apple submission; hosted macOS
CI; and, for a release, both hosted Apple signing/notarization jobs plus public
archive signature/checksum checks. Windows/Linux green alone is insufficient.

Do not delete legacy `tr-300-installer.*` aliases, regenerate the permanent MSI
UpgradeCodes/component GUIDs or Inno AppIds, merge Global and Corporate install
origins, or change the 28-asset release contract casually. Those are established
upgrade paths.

### Branch, release, board, and credential context

- Work directly on `main`, the remote default branch since 2026-07-17. The
  rename used GitHub's atomic branch-rename operation after confirming CI and
  crates publishing already accepted `main`, tag-driven release workflows did
  not depend on the old name, and no open PR, protection, ruleset, webhook, or
  deployment policy targeted `master`. Preserve the full local/hosted gate.
- Release sequence is source commit -> exact-SHA `ci.yml` ->
  exact-SHA `crates-publish.yml` / crates.io -> explicit `v4.0.1` tag ->
  `release.yml` -> `windows-installers.yml` -> public artifact audit ->
  homepage deployment -> release-ledger documentation and attestation.
- Never use `git push --tags`. Never move/delete a published tag. If tag-time
  release fails, inspect the partial state and use the documented fix-forward
  path.
- `v4.0.0` is exactly that partial state: its immutable tag and crates.io
  package exist, but release run 29390216481 failed both Apple jobs before
  Post-build/upload and skipped host/announce. Never move or reuse it.
- `.tasks/` is local/gitignored. Resolve the board from
  `.tasks/.board-server.json` and verify its root. Do not assume port 4319 on
  another host.
- Credential values exist in GitHub Actions and local secure storage. Future
  Windows work should need neither Apple browser access nor local Apple keys.
  Do not print `gh secret` values or try to export them.
- The official repo is
  `https://github.com/QubeTX/qube-machine-report`; the release is
  `https://github.com/QubeTX/qube-machine-report/releases/tag/v4.0.1` after
  publication; crates.io is `https://crates.io/crates/tr300/4.0.1`.
- The homepage repo is
  `/Users/realemmetts/Downloads/temp_git/qube-machine-report-homepage`.
  Commit `d77397479ad2b1189cce86b5402eaf1cc966abdf` is deployed at
  `https://reports.qubetx.com/`; SD-300 and Shaughv OS remain intentionally
  WIP-delisted and must not be re-linked yet.

## Release Ledger

This section records observed hosted state, never expectation.

- v4.0.0 partial state: commit/tag
  `c21d5981d4109199fa4bcba15ef8af6285a33d56` / `v4.0.0`; CI run 29389974094
  passed; crates run 29390118811 published an unyanked package; release run
  29390216481 failed closed in both Apple jobs before upload/host; no GitHub
  Release assets were published
- v4.0.1 release source commit:
  `b67ad083503d0fff840af8467015d05c659268ea`
- Default-branch CI: run 29391956665, success across all 13 blocking jobs on
  that exact SHA
- Crates publish: run 29392101640, success on the same SHA; crates.io version
  4.0.1 is unyanked with checksum
  `55086eb631a3b67c8ab0eaa53b9c3783097044ef77321ec8e6849c30e32275da`
- Tag: lightweight `v4.0.1` resolves locally/remotely to the release SHA;
  immutable `v4.0.0` remains at its original SHA
- Cargo-dist: run 29392185522, success
- Apple arm64 hosted submission/result:
  `97b0c295-89d8-4758-a4c3-1dc345c28f0e` / `Accepted`
- Apple x86_64 hosted submission/result:
  `09cf1403-e546-4f5e-8de1-9bf92fd602e9` / `Accepted`
- Windows supplemental installers: run 29392382949, success
- GitHub Release: non-draft/non-prerelease, exact tested target, 28 assets
- Public arm64 archive: SHA-256
  `b2cd1ecbc86d7f86beddb7b15044ac5839d894a4eae781c1bdfb01a305cf3342`;
  archive/sidecar/aggregate/manifest agree; signed/hardened/timestamped arm64
  binary reports 4.0.1 with the expected identifier/team/certificate
- Public x86_64 archive: SHA-256
  `cbc2800cf4e2dad47d8113db33a8092019c6efeccc0e8ee61cae023fff3cb861`;
  archive/sidecar/aggregate/manifest agree; signed/hardened/timestamped x86_64
  binary reports 4.0.1 with the expected identifier/team/certificate
- Installer aliases: canonical/legacy shell hash
  `79bdb3ab32bcee155967a8ca1fdfccf955cae612d5d8afee27132788bd9e01b1`;
  canonical/legacy PowerShell hash
  `7e5f59911fdb73e2405d2354fe24bc1d60b3e39b40c534599ef48ee32899cb66`
- Supplemental Windows hashes: Corporate MSI
  `6ca603d30a13aca11c21aab348ea7aa3ab932c18ebdb58462557fbb7fb771f3d`,
  Global EXE
  `f9477c0ea53fd81f7e11fc3d279e884531a8303e9165f565a6dadc321220f47a`,
  Corporate EXE
  `339cfd02ed7fb0d3741909c07477fd3cbfe803a21bac88237cb519613fe559d3`
- Homepage: commit `d77397479ad2b1189cce86b5402eaf1cc966abdf`, package 1.13.0,
  production `https://reports.qubetx.com/`, bundle `index-DghJyecZ.js`;
  lint/build/wrapper/
  link checks and Chrome desktop/mobile inspection pass with zero site-origin
  console errors, no horizontal overflow, and 49 exact 51-column sample rows
- Release-ledger documentation commit:
  `771fd09a90baf94db64f21471482c296acf71d05`; CI run 29394204632 passed all
  13 jobs on that exact SHA. Crates.io Publish run 29394374303 succeeded and
  correctly skipped the already-published 4.0.1 package. This subsequent
  docs-only attestation commit records those necessarily post-commit facts.
- Default-branch migration: GitHub atomically renamed `master` to `main` at
  unchanged pre-migration tip `cd3c179540b48770e1c555cbf60c809d702eb999`.
  Alignment commit `41c30b1e43f8abc5208f0d94702ed12cd91fb7a7`
  passed all 13 jobs in CI run 29557626125 on `main`; exact-SHA downstream
  crates run 29557758673 succeeded and skipped already-published 4.0.1. Fresh
  clones use `main`, the old GitHub branch URL redirects, `origin/master` is
  absent, all workflows remain active, and tags/releases/Apple proof/crates.io/
  production homepage bytes were re-audited unchanged.
- Branch-runner follow-up: `ci.yml` and `crates-publish.yml` now use
  `actions/checkout@v6` on Node 24, matching the already-proven release and
  supplemental Windows workflows. This removes checkout v4's Node 20
  deprecation annotation without touching product, dependency, distribution,
  Apple, installer, tag, or artifact inputs.

## What's Next

The release and homepage are complete. The next machine must begin with the
post-release hardware queue, not reopen or regenerate the Mac release path:

1. On the personal Alienware, run task `#winhw`: compare full/fast/table/JSON/
   manual-save output to native Windows tools and exercise the installed
   edition's updater/installer-origin path. Record evidence and patch forward.
2. On the personal AMD64 Linux laptop, run the Linux portion of `#plat` against
   native topology, disk/memory, network/DNS, battery/encryption, login, and
   elevated/unelevated facts.
3. On the 64-bit Raspberry Pi 4, finish `#plat` with devicetree CPU/model,
   frequency, load, filesystem, network, graceful-missing-tool, installer, and
   fast-budget checks.
4. Preserve the absolute Mac freeze above. Windows/Linux-only cfg code and
   tests may change; any shared/dependency/build/dist/workflow/Apple change
   reopens the stated native arm64 + Rosetta + real notary gates before a tag.
5. Keep SD-300 and Shaughv OS WIP-delisted on the homepage until their separate
   work is actually ready.
