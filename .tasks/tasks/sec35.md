TT;DR: Resolve PR #35 against current main and ship one consolidated v4.4.3 candidate to fix Windows managed updater launcher hijacking. Implementation and release verification are active.

## Why
Direct operator request on 2026-11-10: fix PR #35 conflicts and deploy the security fix; expanded in the same session to the concerns in closed PRs #1 and #2.

## Scope
Integrate current main, preserve both command aliases and origin/receipt/checksum/rollback contracts, restrict PowerShell launch to the native system-directory host, add adversarial coverage, patch remaining Unix GPU helper resolution from PR #2, audit PR #1 against existing private staging, update vulnerable TLS dependencies, and publish through the existing protected release chain. Preserve unrelated dirty work in the original checkout by using a separate worktree. Physical hardware acceptance stays in its existing tasks.

## Plan
Complete the bounded collector, maintenance-child-chain, and staging inventories, then consolidate every confirmed fix into unpublished v4.4.3. Preserve source/channel/receipt/rollback and foreign-command ownership semantics. Run one complete local gate and independent review of the final diff, pass exact-head native hosted checks and actual cargo-dist plan, merge the qualified exact head, verify main CI/OIDC, and tag that source once. Follow private 24/30/34 asset assembly and public smokes; record final evidence through the protected docs PR. The user permits a necessary new version but rejects release-driven diagnosis and unnecessary CI-only version bumps.

## Impact
Windows managed self-updates use the OS-owned Windows PowerShell executable and working directory instead of a PATH-selected host. Missing trusted hosts fail before launch. Existing install channels, receipt prefixes, installer integrity checks, and both report/tr300 commands remain supported.

## Acceptance
Functional bar: current main contains the fix and the complete immutable v4.4.3 distribution is public, with the crate and both command aliases verified.
Evidence bar: adversarial Windows launcher test, full local gates, exact-head independent review/hosted checks, exact-main CI/OIDC, signed native publication, exact public inventory and smokes.
Gate ownership: repository release skill and protected branch/release workflows.
Valid bounded outcomes: complete only with all required evidence; otherwise active/partial with exact failing gate and next action recorded.

## Attempts
| Obligation | Observation | Decision | Re-entry |
|---|---|---|---|
| Exact-main CI | Linux x86_64 existing report-copy integration test failed spawning a just-copied executable with ETXTBSY; 230 library tests including new security probe passed, PR matrix had passed | One bounded retry permitted by release skill; no tag | Repeated on attempt 2; preparing test-only bounded ETXTBSY handling and explicit writable-file regression before release |

## Evidence
| Criterion | Oracle | Result | Interpretation | Limitation | Status |
|---|---|---|---|---|---|
| Initial state | GitHub PR #35 and current remote main | PR ced1ea5 based on pre-v4.4.0; current main 3f0e047 | Two changelog conflicts; code integration automatic | Initial state only | PASS |
| Independent review | review_security and release_gate_review agents | Exact heads 8c8df9a and 578e301a reviewed without blockers | Security and fixture findings addressed | Hosted gates remain separately required | PASS |

## Verification
- [ ] Current consolidated candidate passes local release gates and adversarial child-chain/staging probes
- [ ] Consolidated exact head passes independent review and hosted CI/Release plan; PR merged
- [ ] Consolidated exact-main CI and trusted-OIDC crate publication pass before immutable tag
- [ ] Complete native release chain, public asset audit, and post-public smokes pass
- [ ] Protected follow-up documentation records the final evidence

## Status
Active implementation; publication is stopped pending the complete bounded audit fixes. v4.4.3 is local and unpublished. The collector, maintenance-chain, and staging audits are complete; their confirmed fixes are being consolidated in this candidate before another merge or tag. v4.4.1 and v4.4.2 crates/tags remain immutable; v4.4.2 Release 42974726577 passed but its 24-asset draft 441328198 remains private, with downstream Windows/macOS jobs cancelled. Public latest remains v4.4.0. User explicitly authorized a necessary new version and normal publishing after the work is complete, and asked to reduce avoidable CI repetition.

## Activity
- 2026-11-10 — Started isolated worktree; inspected PR and current main; preserved original checkout's dirty work (agent: codex).
- 2026-11-10 — Resolved both changelog conflicts and prepared v4.4.1; incorporated initial independent review findings; began local verification (agent: codex).

- 2026-11-10 — User added PRs #1/#2. Confirmed #1 protections already superseded by randomized private staging plus exclusive create; Windows #2 probe uses native registry. Patching remaining Unix GPU helpers. Audit found six TLS advisories; targeted updates to rustls 0.23.46, webpki 0.103.16, and roots 1.0.10 now pass audit (agent: codex).

- 2026-11-10 — Committed and independently reviewed 8c8df9a; all local release gates pass. Pushed PR #35, refreshed its description, marked ready; waiting on exact-head CI 42965125140 and plan 42965125037 (agent: codex).

- 2026-11-10 — Exact-head CI passed 20/20 and plan passed; PR #35 merged as f9e307c. Watching exact-main CI 42966014094 and automatic crates publisher 42966014268 before tagging (agent: codex).

- 2026-11-10 — Main Linux x86_64 job 130343312951 hit ETXTBSY in existing copied-report integration fixture; security tests passed. Waiting for the remaining first-attempt jobs, then retrying only the failed job once under release policy (agent: codex).

- 2026-11-10 — Single Linux retry repeated ETXTBSY. Preparing a reviewed test-only fix-forward: at most 1s of retries for Linux ExecutableFileBusy alone, with an explicit temporarily writable executable fixture; all original security assertions remain (agent: codex).

- 2026-11-10 — Pushed reviewed fixture fix as PR #36. Both Linux architectures pass the explicit writable-executable regression; plan 42967669724 passed and remaining exact-head CI 42967669636 is running (agent: codex).

- 2026-11-10 — PR #36 passed 20/20 and plan, then merged as b4024e6. Watching new exact-main CI 42968538009 and automatic publisher 42968537979; version remains 4.4.1, no tag yet (agent: codex).

- 2026-11-11 — Exact-main CI 42968538009 passed all 20 jobs at b4024e6. Automatic crates publisher 42968537979 is past its CI gate and validating/package-checking the exact source (agent: codex).

- 2026-11-11 — Automatic OIDC publication 42968537979 passed. Independently verified unyanked 4.4.1 checksum/source, then pushed only immutable tag v4.4.1 at b4024e6. Release 42970145859 is running (agent: codex).

- 2026-11-11 — v4.4.1 release stopped deterministically on retired macos-14 before signing/host. Per release policy, preserved tag/crate and prepared v4.4.2 global runner overrides plus actual-plan guard/negative fixtures. Local pinned dist plan now selects native macos-15 and macos-15-intel (agent: codex).

- 2026-11-11 — Local v4.4.2 gates and independent exact-bc6911b review passed; pushed PR #37. Actual Release plan 42971809550 passes the native matrix guard; CI 42971809468 is running (agent: codex).

- 2026-11-11 — PR #37 passed all 20 and actual Release plan, then merged as e868bb1. Watching new exact-main CI 42972929754 and trusted-OIDC publisher 42972929890 before creating the new immutable v4.4.2 tag (agent: codex).

- 2026-11-11 — v4.4.2 exact-main CI 42972929754 passed 20/20 including both native Apple gates and actual-plan validation. OIDC workflow 42972929890 is packaging/dry-running (agent: codex).

- 2026-11-11 — Automatic publisher 42972929890 passed; independently verified unyanked 4.4.2 checksum/source. Revalidated actual dist plan and pushed only tag v4.4.2 at e868bb1. Release 42974726577 is running (agent: codex).

- 2026-11-11 — Stopped publication after end-to-end review found remaining implicit helper/module search, raw Windows archive binding, and installer custody gaps. Finished all three bounded inventories before implementation; consolidated fixes into unpublished v4.4.3. No further tags until complete qualification (agent: codex).
- 2026-11-11 — Measured CI: main 11.13 min, Intel PKG fixture 7.95 min; audit-tool install 3.25 min vs audit 0.05 min; crate workflow waits on CI for 11.27 of 13.17 min. Added reuse of the pinned audit binary, retaining a fresh audit and protected-main-only cache saves. No container image builds were found (agent: codex).

- 2026-11-11 — Consolidated local gates pass: 220 library + two companion + 29 integration tests, strict clippy/fmt, audit, 44-file package inventory/dry-run/release build, whole-fast child poisoning, runtime PS module reset, archive digest/retained guard, staging owner/DACL/handle tests, original PATH ownership checks, shell and both PS transaction suites, actual dist runner plan, workflow/shell/provenance gates. Runtime aliases/help match; fast JSON median 318.1 ms in five smoke runs. Independent component reviews found no remaining blocker (agent: codex).
