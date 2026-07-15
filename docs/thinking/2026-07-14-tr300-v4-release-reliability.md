# Critical Thinking Session — TR-300 v4 Release Reliability

**Date:** 2026-07-14
**Framework:** Information Triage → Design
**Mode:** Self-check
**Stacked skills:** logical-reasoning

---

## Pre-Flight: Inputs Inspected

### Inputs brought to the session

- Operator requirements: a dense sequence of direct decisions covering cross-platform
  accuracy, macOS completion, report persistence, managed-Windows antivirus behavior,
  release timing, Apple notarization, hardware follow-up, task tracking, and handoff.
- Current repository source and tests: internal implementation artifacts on `master`,
  including a previously pushed macOS hardening checkpoint and current uncommitted
  report/update/release changes.
- Current project guidance: `AGENTS.md`, `CLAUDE.md`, `MASTER_PLAN.md`, `TESTING.md`,
  the architecture decisions, release skill, task board, and tracked handoff.
- Current operational evidence: live macOS arm64/Rosetta checks, the v3.17.0 release
  artifact audit, Apple Developer ID identities, App Store Connect API-key validation,
  and an accepted local Apple Notary Service submission.

### Source pass findings

- The operator's final instructions are the controlling release decision. Older tracked
  docs and local task records that say personal Windows/Linux/Pi evidence blocks v4 are
  stale and must be revised without erasing that deferred evidence requirement.
- Repository guidance is authoritative for process but not proof of current behavior.
  Source, tests, signed artifacts, Apple responses, and exact-SHA hosted runs are the
  evidence required for release claims.
- Live Mac evidence is available on this host. Personal Alienware, AMD Linux laptop, and
  Raspberry Pi 4 evidence is not available in this session and must not be invented.
- The managed work machine's antivirus behavior is valid product input but is not a
  substitute for accuracy testing on the operator's personal Alienware.

### What's already decided (not revisiting)

- Keep a single Rust-first cross-platform architecture with OS-specific collection behind
  platform boundaries.
- Normal `tr300` and `report` runs perform no report-file write. Saving is manual only via
  `-r`, `--report`, `-s`, or `--save`; legacy `--no-save` remains a hidden no-op.
- A likely endpoint-policy block during update stops the fallback chain, preserves the
  current installation, reports actionable diagnostics, and exits unsuccessfully. There
  is no forced direct overwrite or unsafe prompt path.
- The operator authorizes v4.0.0 before personal Alienware/AMD/Pi verification and will
  release follow-up patches if that evidence finds defects.
- Both macOS release archives must be Developer ID signed and accepted by Apple
  notarization in GitHub Actions. Missing credentials or rejection must block hosting;
  there is no unsigned fallback.
- Work is delivered to the repository default branch (`master`) with the full local,
  exact-SHA hosted, crates.io, tag, release-asset, and homepage gates intact.

---

## Working Sections

### Facts

| Fact | Confidence | Source / surfaced at |
|---|---|---|
| v3.17.0's arm64 release binary was ad-hoc signed and had no Apple notarization proof. | High | Direct archive, `codesign`, Gatekeeper, and stapler audit |
| A selected Developer ID Application identity is valid through 2031 and signs the bare CLI with hardened runtime and a trusted timestamp. | High | Local isolated-keychain signing and certificate-chain verification |
| The new App Store Connect API key authenticated successfully and Apple accepted a signed TR-300 test submission. | High | `notarytool history` and accepted submission `7441ddc2-32e7-48bf-bbb3-b23f7fa504ef` |
| A standalone CLI binary has no `.app` or `.pkg` container to staple; the accepted notarization record is bound to the signed bytes online. | High | Apple tooling behavior observed during the local test |
| Ordinary report execution now reaches the existing report writer only when the explicit save flag is true. | High | `src/cli.rs`, `src/main.rs`, and targeted tests |
| Endpoint-policy-like staging/launch failures now have a distinct blocked result and stop subsequent updater strategies. | High | `src/update.rs` and targeted unit tests |
| Personal Windows/Linux/Pi live accuracy remains unverified in this session. | High | Hardware availability and operator direction |
| GitHub Actions secret names are configured without committing secret values. | High | Repository secret/variable name audit |

### Assumptions

| Assumption | Status | Surfaced at | Notes |
|---|---|---|---|
| GitHub's current macOS release runners include compatible `security`, `codesign`, `xcrun notarytool`, `jq`, `ditto`, and `shasum`. | open | Design research | Local host proves behavior; hosted tag jobs are the decisive test. |
| Patching the per-target cargo-dist manifest checksum before Post-build is sufficient for global checksum/hosting jobs to consume the exact signed archive. | open | Prototype design | Must be tested against a real cargo-dist manifest locally and then in hosted release jobs. |
| The two Apple matrix entries each expose their target through `matrix.targets` in the generated workflow. | tested | Workflow audit | The workflow joins matrix targets and build jobs are target-specific; YAML validation still required. |
| Endpoint products may return errors outside the initial Windows/raw-OS policy set. | open | Updater design | Unknown errors still fail safely; only policy classification and fallback stopping may be less specific. |
| Releasing before personal-fleet checks may reveal OS-specific defects. | tested | Operator decision | Explicitly accepted as patch-release risk; must remain visible in the board and handoff. |

### Constraints

- Secrets never enter git, task files, reasoning records, logs, or user-facing output.
- The signed executable bytes submitted to Apple must be the exact bytes repacked into
  the hosted archive; any post-notarization mutation invalidates the evidence.
- Apple credential absence, certificate import failure, signing failure, or non-Accepted
  notarization must fail the macOS build-local job before artifact upload.
- Existing cargo-dist artifact names, legacy installer aliases, four Windows installer
  paths, updater origin rules, and crates.io publication ordering remain intact.
- Release claims must distinguish local proof, exact-SHA CI, crates.io, GitHub Release,
  Apple acceptance, and post-release personal hardware work.

### Open questions

- Does the signing script correctly mutate an actual cargo-dist manifest and preserve
  archive checksum agreement? Answer before the release commit.
- Do both hosted macOS tag jobs import the certificate and receive Apple `Accepted`?
  Answer before calling the GitHub Release complete.
- Does the final GitHub Release contain the expected complete asset set, and do both Mac
  archives verify as Developer ID signed? Answer after tag publication.

### Tensions

- The operator needs the release finished now vs. the personal-fleet evidence is
  unavailable. Resolution: ship only after comprehensive Mac/local/hosted gates, disclose
  the evidence boundary, and retain personal hardware work as explicit post-release tasks.
- Managed antivirus objects to unexpected file activity vs. self-update necessarily
  stages installer payloads. Resolution: normal reports stop writing; update remains
  transactional and fails closed on policy-like staging/launch errors without direct
  binary replacement.
- A bare CLI benefits from notarization vs. stapling applies to app/package containers.
  Resolution: require Developer ID signature, hardened runtime, timestamp, and Apple
  `Accepted`; document why no staple is expected.

### Deferred items

- Personal Alienware full/fast/JSON/save/update and field-by-field Windows validation:
  post-release on the operator's personal machine.
- AMD64 Linux laptop and 64-bit Raspberry Pi 4 field-by-field validation: post-release.
- Managed work Windows retest: useful endpoint-policy regression evidence only, not the
  canonical personal Windows accuracy gate.

---

## Framework Steps

### Information Triage: Dense Hand-Back

**Sub-questions asked:** What decisions are required? What is done? What is in flight?
What comes later? What constraints/deadlines control the work? What happens if nothing
else is decided?

**Responses:**

| Bucket | Result |
|---|---|
| Decisions needed | None from the operator. Release authorization, manual-save behavior, graceful updater failure, no force path, and automatic Apple notarization are settled. |
| Done — FYI | macOS collector hardening and prior native/Rosetta proof; opt-in save implementation; updater policy-block implementation; Apple API key/certificate setup; one accepted local notarization submission. |
| In flight | release-workflow enforcement, actual manifest/script test, version/docs/board synchronization, comprehensive gates, ordered publish/deploy. |
| Coming later | personal Alienware, AMD Linux, Pi 4, and managed-work endpoint retests; patches as evidence requires. |
| Constraints | no secrets in source; no unsigned Mac fallback; no unsafe direct updater overwrite; default-branch and exact-SHA release discipline; user is tired, so continue autonomously. |

**Insights:** The stale hardware-blocking release language is the only buried policy
conflict. It must be corrected everywhere before release. The exit ramp is Design: finish
the fail-closed release mechanism and validate it.

**Mode:** Convergent

### Design Step 1: Empathize

**Sub-questions asked:** Who is affected? What pain exists? How is it handled now? How
might the design work for a future tired operator releasing from Windows?

**Responses:**

- Emmett runs TR-300 across MacBooks, personal Alienware hardware, an AMD Linux laptop,
  Raspberry Pi 4, and a managed Windows machine with aggressive antivirus.
- Unexpected automatic report writes can freeze the managed host. A release pushed from
  Windows must not depend on manually returning to a Mac to sign/notarize.
- Users need ordinary reports to be read-only, manual saves to preserve existing safety,
  updates to retain the working install on policy failure, and Mac downloads to carry
  real Apple trust evidence.
- The future tired operator needs a tag-triggered workflow that either produces signed,
  Apple-accepted archives or stops visibly before hosting.

**Insights:** Reliability here means both accurate data and controlled side effects.
Release automation must encode the Mac quality gate instead of relying on memory.

**Mode:** Convergent

### Design Step 2: Define

**Sub-questions asked:** What exact problem is being solved? What requirements,
limitations, legal/security considerations, and success criteria apply? Is it feasible?

**Responses:**

- Problem: current release automation can host ad-hoc-signed Mac binaries, normal report
  runs unexpectedly write files, and update staging failures do not distinguish policy
  blocks clearly enough.
- Requirements: opt-in-only persistence; compatibility alias; safe failure with current
  install retained; both Apple targets Developer ID signed; Apple `Accepted`; exact signed
  bytes hosted; missing credentials block; full local/hosted proof; no secret exposure.
- Limitations: bare CLI cannot be stapled; non-Mac hardware is unavailable now; cargo-dist
  generates the release workflow and manifest contract; Windows installer paths must not
  regress.
- Feasibility: source behavior and local Apple signing/notary prototype already pass.
  Remaining uncertainty is workflow/manifest integration and hosted execution.

**Insights:** “Complete macOS now” requires an enforced build gate, not a note that a
local binary once passed.

**Mode:** Convergent

### Design Step 3: Research

**Sub-questions asked:** What existing mechanisms exist? Where did the current path fail?
What tools/processes are available? What does the solution remain limited by?

**Responses:**

- Existing cargo-dist builds both Mac archives but the audited v3.17.0 executable was
  ad-hoc signed.
- GitHub Actions can hold the certificate/API key as encrypted secrets; App Store Connect
  API credentials allow non-interactive notarization independent of the push machine.
- A locally tested ephemeral keychain can import the existing Developer ID certificate,
  sign with hardened runtime/timestamp, and submit with `notarytool`.
- The per-target archive and manifest can be updated before cargo-dist Post-build selects
  upload paths; the exact behavior still needs a real-manifest test.

**Insights:** Signing after hosting is too late. The correct insertion point is after
`dist build` and before cargo-dist Post-build/upload.

**Mode:** Convergent

### Design Step 4: Ideate and Select

**Sub-questions asked:** What different solutions exist, including the worst idea? Which
is useful, least complicated, reproducible, and compatible with Windows-origin releases?

**Responses — divergent options:**

1. Keep ad-hoc signing and document a warning. This does not satisfy the requirement.
2. Manually sign/notarize on this Mac after every tag. This cannot enforce releases
   initiated from Windows and depends on operator memory.
3. Host unsigned artifacts, then replace them after a separate signing workflow. This
   creates a race and a period of unsafe public assets.
4. Package the CLI as an app/pkg solely to staple a ticket. This changes distribution
   semantics and is unnecessary for a command-line executable.
5. Add a fail-closed per-target script immediately after `dist build`, repack the exact
   signed/notarized bytes, update checksums/manifests, and continue only on Apple
   `Accepted`.
6. The “worst possible idea”: decode long-lived credentials into the repository or print
   them for debugging. This is explicitly prohibited.

**Responses — convergence:** Option 5 uniquely meets reproducibility, Windows-origin
release, no unsigned window, cargo-dist compatibility, and secret-handling requirements.
The script uses an ephemeral keychain/work directory and no fallback.

**Mode:** Divergent → Convergent

### Design Steps 5–8: Prototype, Test, Refine, Release

**Sub-questions asked:** What prototype exists? What constitutes success? What did the
first test reveal? What remains before release and post-launch feedback?

**Responses:**

- Prototype: `scripts/sign-notarize-macos.sh`, plus planned workflow insertion between
  Build artifacts and Post-build.
- Initial test: isolated certificate import, Developer ID signing, hardened runtime,
  timestamp, and Apple notarization all succeeded locally. The certificate export needed
  OpenSSL legacy PKCS#12 wrapping for macOS import; the final secret uses that compatible
  format.
- Success criteria: shell/YAML checks; real cargo-dist manifest mutation; regenerated
  sidecar agreement; extracted binary signature verification; Apple `Accepted`; both
  hosted Apple jobs pass; both hosted archives verify after release.
- Refinement still required: insert workflow step, test the script against a real archive
  and manifest, synchronize release docs, and run every code/package/security gate.
- Release feedback loop: preserve personal-fleet tasks and ship patches for real defects,
  without rewriting the frozen Mac release path from Windows.

**Mode:** Convergent

---

## Logical Audit of the Release Claim

**Mode:** Construct / Prove

**Goal:** establish when the claim “v4.0.0 is permitted to release” follows, without
mistaking partial evidence for completion.

**Dictionary:**

- `D` = the operator explicitly authorizes release before personal-fleet checks.
- `M` = both Mac targets are locally and hosted-proven Developer ID signed and Apple
  accepted.
- `C` = the comprehensive source/package/security validation suite passes.
- `W` = exact release-SHA default-branch CI and crates publication gates pass.
- `R` = v4.0.0 is permitted to proceed to tag/release.

**Argument:**

1. `(D ∧ M ∧ C ∧ W) → R` — release policy premise.
2. `D` — direct operator instruction.
3. `M` — not yet established; local prototype evidence is necessary but hosted
   per-target evidence is still pending.
4. `C` — not yet established for the final versioned tree.
5. `W` — not yet established for the final release commit.

**Verdict:** `R` does not yet follow. Once `M`, `C`, and `W` are proven, conjunction
introduction yields `D ∧ M ∧ C ∧ W`; Modus Ponens with line 1 then yields `R`. This
prevents the invalid shortcut “local notarization succeeded, therefore the release is
complete.”

---

## Steel-Manned Dissent

- **The case against:** Releasing before personal Alienware/AMD/Pi evidence weakens the
  original “every OS” confidence claim; the safest choice is to keep v4 unreleased.
- **What would have to be true for it to be correct:** The release must promise verified
  correctness on those exact machines now, or discovered defects would be too damaging
  for patch releases.
- **How it was handled:** Modified the release claim and plan. v4 may ship under explicit
  operator risk acceptance only after Mac/local/hosted gates; docs and handoff must state
  that personal-fleet validation is post-release and cannot be presented as completed.
- **Confidence in the handling:** High — the scope boundary is explicit and testable.

---

## Checkpoint — 2026-07-14

### Status snapshot

| State | Items |
|---|---|
| Settled | Manual-only save; graceful updater policy failure; no direct overwrite; v4 timing; no unsigned Mac fallback; personal hardware deferred. |
| Proven | Targeted source tests; certificate/API-key function; local Developer ID signature; one Apple-accepted notarization test; board server identity. |
| Open | Workflow insertion; real-manifest script test; final docs/version; comprehensive gates; exact-SHA CI/crates; tag/release assets; homepage. |
| Deferred | Personal Alienware, AMD64 Linux, Pi 4, and managed-work endpoint retests. |
| In tension | Release-now authorization vs. absent personal-fleet evidence, resolved through explicit evidence boundaries and patch follow-up. |

### Exit state

**Directed:** insert and validate the fail-closed signing/notarization step before
synchronizing the v4.0.0 release tree.

---

## Checkpoint — final local release tree

**Mode:** Verify

The earlier open implementation predicates are now locally discharged:

- final native and Rosetta suites pass with 121 library + 19 integration tests
  per architecture;
- native/Rosetta release binaries pass full/fast/table/JSON/ASCII/privacy/parity
  smokes, all four manual-save aliases, zero-write defaults, and updater no-op;
- five-run medians are 0.51s/0.23s native full/fast and 0.72s/0.36s Rosetta;
- Windows MSVC xwin check and warning-denying Clippy pass;
- format, Clippy, tests, audit, dist plan, actionlint, shellcheck, Bash syntax,
  external Rust consumer, dirty-tree package list, and publish dry-run pass;
- signing by display name initially failed safely because the same certificate
  existed in the ephemeral and login keychains. Resolving the single ephemeral
  identity and signing by its certificate fingerprint removed the ambiguity;
- the actual v4 arm64 and x86_64 binaries both completed Developer ID signing,
  expected identity/team/runtime/timestamp checks, Apple notarization
  (submissions `c2afae62-1873-4337-8c88-1bbfa26c23eb` and
  `fe2dcc67-cfe1-49be-8d4c-59daf8697c61`, both `Accepted` while using the
  fingerprint-form identity configured in GitHub), repacking, and
  archive/sidecar/manifest checksum verification.

The clean-tree package/dry-run gate follows the release commit. In the logical
audit, local `M` and `C` are established; hosted `M` and `W` remain open until
the exact-SHA CI/crates and tag workflows prove them. Therefore `R` still must
not be claimed before those hosted observations.

---

## Checkpoint — v4.0.1 immutable-tag fix-forward

The hosted v4.0.0 observation corrected an environmental confounder in local
`M`: the developer login keychain contained the same certificate, so local
fingerprint signing succeeded even though the ephemeral keychain itself was not
on the user search list. Clean GitHub runners imported and enumerated the
identity, then correctly failed before upload when `codesign` could not resolve
it. CI and crates publication had passed, and the tag was already shared;
therefore moving v4.0.0 would violate the immutable-tag premise. A v4.0.1 patch
is the only valid fix-forward.

The correction snapshots the original user keychain list, prepends the private
keychain only for the signing call, restores the list immediately and in the
cleanup trap, and compares the certificate extracted from the signed Mach-O
with the exact resolved SHA-1 fingerprint. Local v4.0.1 cargo-dist proofs now
establish this stronger `M`: submissions
`52b52e88-8eb9-457b-bb01-6c39f01da913` (arm64) and
`e018b7e1-d16e-4a33-b2a2-5b62512652b5` (x86_64) were `Accepted`; both search
list restoration and fail-closed behavior were observed under real timestamp/
notary transport failures. Hosted `M` remains open until the v4.0.1 tag jobs
repeat that result on clean runners.
