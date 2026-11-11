TT;DR: Resolve PR #35 against current main and ship v4.4.1 to fix Windows managed updater launcher hijacking. Implementation and release verification are active.

## Why
Direct operator request on 2026-11-10: fix PR #35 conflicts and deploy the security fix; expanded in the same session to the concerns in closed PRs #1 and #2.

## Scope
Integrate current main, preserve both command aliases and origin/receipt/checksum/rollback contracts, restrict PowerShell launch to the native system-directory host, add adversarial coverage, patch remaining Unix GPU helper resolution from PR #2, audit PR #1 against existing private staging, update vulnerable TLS dependencies, and publish through the existing protected release chain. Preserve unrelated dirty work in the original checkout by using a separate worktree. Physical hardware acceptance stays in its existing tasks.

## Plan
Resolve changelog conflicts while preserving both histories; prepare patch v4.4.1; run local gates and independent review; push the PR branch and pass hosted gates; merge only its reviewed exact head; verify exact-main CI and OIDC; tag that SHA once; follow private 24/30/34 asset assembly and public smokes. Diagnose failures and fix forward under the release skill without bypassing gates or moving tags.

## Impact
Windows managed self-updates use the OS-owned Windows PowerShell executable and working directory instead of a PATH-selected host. Missing trusted hosts fail before launch. Existing install channels, receipt prefixes, installer integrity checks, and both report/tr300 commands remain supported.

## Acceptance
Functional bar: current main contains the fix and the complete immutable v4.4.1 distribution is public, with the crate and both command aliases verified.
Evidence bar: adversarial Windows launcher test, full local gates, exact-head independent review/hosted checks, exact-main CI/OIDC, signed native publication, exact public inventory and smokes.
Gate ownership: repository release skill and protected branch/release workflows.
Valid bounded outcomes: complete only with all required evidence; otherwise active/partial with exact failing gate and next action recorded.

## Evidence
| Criterion | Oracle | Result | Interpretation | Limitation | Status |
|---|---|---|---|---|---|
| Initial state | GitHub PR #35 and current remote main | PR ced1ea5 based on pre-v4.4.0; current main 3f0e047 | Two changelog conflicts; code integration automatic | Initial state only | PASS |
| Independent initial review | review_security agent | Remove dead pwsh variant; reuse robust OS directory resolver; set trusted cwd | Findings addressed in candidate | Final exact commit review pending | PARTIAL |

## Verification
- [ ] Local release gates and adversarial Windows launcher probe pass
- [ ] Exact-head independent review and hosted CI/Release plan pass; PR merged
- [ ] Exact-main CI and trusted-OIDC crate publication pass before immutable tag
- [ ] Complete native release chain, public asset audit, and post-public smokes pass
- [ ] Protected follow-up documentation records the final evidence

## Status
Active in C:/Users/hey/git/qube-machine-report-security-pr35. Merged origin/main with changelog conflicts resolved. Candidate implementation uses the shared bounded native system-directory resolver, removes obsolete pwsh strategy, sets trusted host cwd, and adds command/probe tests. Windows cargo check, fmt, strict clippy, all-target tests, and the hostile-PATH host probe pass. Advisory audit and standalone script/workflow lints plus managed transactions pass. Provenance fixtures and release build are running; package gates, Unix hosted tests, exact commit review, merge, and publication remain. Public baseline remains v4.4.0.

## Activity
- 2026-11-10 — Started isolated worktree; inspected PR and current main; preserved original checkout's dirty work (agent: codex).
- 2026-11-10 — Resolved both changelog conflicts and prepared v4.4.1; incorporated initial independent review findings; began local verification (agent: codex).

- 2026-11-10 — User added PRs #1/#2. Confirmed #1 protections already superseded by randomized private staging plus exclusive create; Windows #2 probe uses native registry. Patching remaining Unix GPU helpers. Audit found six TLS advisories; targeted updates to rustls 0.23.46, webpki 0.103.16, and roots 1.0.10 now pass audit (agent: codex).
