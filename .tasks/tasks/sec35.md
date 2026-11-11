TT;DR: Resolve PR #35 against current main and ship v4.4.2 to fix Windows managed updater launcher hijacking. Implementation and release verification are active.

## Why
Direct operator request on 2026-11-10: fix PR #35 conflicts and deploy the security fix; expanded in the same session to the concerns in closed PRs #1 and #2.

## Scope
Integrate current main, preserve both command aliases and origin/receipt/checksum/rollback contracts, restrict PowerShell launch to the native system-directory host, add adversarial coverage, patch remaining Unix GPU helper resolution from PR #2, audit PR #1 against existing private staging, update vulnerable TLS dependencies, and publish through the existing protected release chain. Preserve unrelated dirty work in the original checkout by using a separate worktree. Physical hardware acceptance stays in its existing tasks.

## Plan
Resolve changelog conflicts while preserving both histories; prepare patch v4.4.1; run local gates and independent review; push the PR branch and pass hosted gates; merge only its reviewed exact head; verify exact-main CI and OIDC; tag that SHA once; follow private 24/30/34 asset assembly and public smokes. Diagnose failures and fix forward under the release skill without bypassing gates or moving tags.

## Impact
Windows managed self-updates use the OS-owned Windows PowerShell executable and working directory instead of a PATH-selected host. Missing trusted hosts fail before launch. Existing install channels, receipt prefixes, installer integrity checks, and both report/tr300 commands remain supported.

## Acceptance
Functional bar: current main contains the fix and the complete immutable v4.4.2 distribution is public, with the crate and both command aliases verified.
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
- [ ] Local release gates and adversarial Windows launcher probe pass
- [ ] Exact-head independent review and hosted CI/Release plan pass; PR merged
- [ ] Exact-main CI and trusted-OIDC crate publication pass before immutable tag
- [ ] Complete native release chain, public asset audit, and post-public smokes pass
- [ ] Protected follow-up documentation records the final evidence

## Status
Fix-forward candidate is v4.4.2. Immutable v4.4.1 remains at `b4024e69e7a2def24da3ba2b42ed19a4dcd2b6a1`, with exact unyanked crate checksum `45345d6727f2a4e4999f23921c8f743c0d066c7f45c9dd05d1502414ed95f4f2`. Release `42970145859` cancelled its ARM Apple job before runner assignment because pinned cargo-dist generated retired `macos-14`; five other builds passed, all signers/host skipped, no draft. Do not move/reuse that tag. Candidate v4.4.2 explicitly maps Apple builders to native 15/15-intel and validates the actual JSON matrix in CI and Release with negative fixtures. Local gates and independent review are running before a new protected PR; public GitHub baseline remains v4.4.0.

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
