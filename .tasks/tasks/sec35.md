TT;DR: v4.4.4 is published with the reported security fixes and all required native/public runtime gates passed. Protected process/documentation qualification and the completion receipt are complete.

## Why
Direct operator request on 2026-11-10: fix PR #35 conflicts and deploy the security fix; expanded in the same session to the concerns in closed PRs #1 and #2.

## Scope
Integrate current main, preserve both command aliases and origin/receipt/checksum/rollback contracts, restrict PowerShell launch to the native system-directory host, add adversarial coverage, patch remaining Unix GPU helper resolution from PR #2, audit PR #1 against existing private staging, update vulnerable TLS dependencies, and publish through the existing protected release chain. Preserve unrelated dirty work in the original checkout by using a separate worktree. Physical hardware acceptance stays in its existing tasks.

## Plan
Implementation, source/publication/runtime acceptance, full process qualification, and actual documentation PR/main/no-op proof are complete. Version 4.4.4 and immutable public bytes are unchanged; physical hardware remains separately owned.

## Impact
Windows managed self-updates use the OS-owned Windows PowerShell executable and working directory instead of a PATH-selected host. Missing trusted hosts fail before launch. Existing install channels, receipt prefixes, installer integrity checks, and both report/tr300 commands remain supported.

## Acceptance
Functional bar: current main contains the fix and the complete immutable v4.4.4 distribution is public, with the crate and both command aliases verified.
Evidence bar: adversarial Windows launcher test, full local gates, exact-head independent review/hosted checks, exact-main CI/OIDC, signed native publication, exact public inventory and smokes.
Gate ownership: repository release skill and protected branch/release workflows.
Valid bounded outcomes: complete only with all required evidence; otherwise active/partial with exact failing gate and next action recorded.

## Attempts
| Obligation | Observation | Decision | Re-entry |
|---|---|---|---|
| Exact-main CI | Linux x86_64 existing report-copy integration test failed spawning a just-copied executable with ETXTBSY; 230 library tests including new security probe passed, PR matrix had passed | One bounded retry permitted by release skill; no tag | Repeated on attempt 2; preparing test-only bounded ETXTBSY handling and explicit writable-file regression before release |
| Public legacy Mac bridge | Two Intel runs returned stapler exit 68; retry accepted the DMG before the inner-PKG failure. Direct public PKG and DMG inner PKG are byte-identical, with ticket trailer preserved | Freeze repeated Intel retries; use a separate unmerged CI-only native verbose probe of direct, mounted, and copied PKG contexts | Need native tool stdout/stderr and actual-path hashes before any version/producer change; current evidence does not prove an artifact defect |

## Evidence
| Criterion | Oracle | Result | Interpretation | Limitation | Status |
|---|---|---|---|---|---|
| Initial state | GitHub PR #35 and current remote main | PR ced1ea5 based on pre-v4.4.0; current main 3f0e047 | Two changelog conflicts; code integration automatic | Initial state only | PASS |
| Independent review | review_security and release_gate_review agents | Exact heads 8c8df9a and 578e301a reviewed without blockers | Security and fixture findings addressed | Hosted gates remain separately required | PASS |
| v4.4.4 source/crate | Exact-head/main CI and trusted OIDC; independent registry download | PR 39, CI 43005013711/43006272757; publisher43006272686; checksum/VCS in TESTING.md | Qualified source and public crate match | No broad exploit claim beyond audited scope | PASS |
| Private/native publication | Exact workflow/run proofs and release API | Release43007443270a3 13/13; Win43015387367 3/3; private43015777726 16/16; Mac43015387253a6 12/12 | Complete34asset publication from9f | Native signature evidence is CI, not Windows codesign | PASS |
| Public bytes/runtime | Anonymous downloads, hash/sidecar/archive/wrapper comparison; actual public lifecycle jobs | target/public-v4.4.4/audit.json; publicWin43030855472 9/9; native/public hashes and both legacy bridge results | Public latest and both commands verified | Physical hardware/UAC remains separately open | PASS |
| Documentation closure | Protected evidence PR, exact-main CI, idempotent existing-version publisher | Full CI 43154206746/proof 43154206867; docs PR 44/main CI 43154266882/43155508207/proof 43155508130 | Full/docs paths and existing publication verified | Physical hardware remains separate | PASS |

## Verification
- [x] Current consolidated candidate passes local release gates and adversarial child-chain/staging probes
- [x] Consolidated exact head passes independent review and hosted CI/Release plan; PR merged
- [x] Consolidated exact-main CI and trusted-OIDC crate publication pass before immutable tag
- [x] Complete native release chain, public asset audit, and post-public smokes pass
- [x] Protected follow-up documentation records the final evidence

## Status
Complete (2026-11-11). Exact v4.4.4 source/tag
9f5110a3e8665f5fa2eb2e1744778c8025dc241f remains the verified unyanked crate and
public 34-asset release 441625880. All source/private/native/public gates passed;
PRs #42/#43 and actual docs PR 44/main now complete process qualification and
the read-only existing-version proofs. TESTING.md and the November11 handoff
contain the final evidence. No further version or recursive record PR is
needed. Personal hardware/UAC tasks remain open.

## Activity
- 2026-11-11 — All product/runtime gates complete: Mac attempt6 passed12/12 and public Windows43030855472 passed9/9. Prepared publication evidence, current guides/handoff, release-note security summary, and measured CI report. This remains open pending the protected documentation PR/main/idempotent proof and final completion receipt (agent: codex).

- 2026-11-11 — Draft PR #40 first head 24a8589 passed all 20 and actual plan; both native verbose public trust probes pass all four contexts. Re-entered original legacy once with that new evidence: Mac attempt 5 ARM passes, Intel repeats inner-PKG stapler68. Source audit rules out baseline postinstall/environment mutation; next isolated variables are raw/canonical path spelling and exact default/stdout-null launch, with no new payload/version (agent: codex).

- 2026-11-11 — Mac attempt 4 concluded: Intel legacy stapler failure repeated; ARM legacy again cancelled with no runner. Paused that repeated validation route pending the native verbose diagnostic rather than launching another identical check or version. The release stays public/unchanged; full closure remains partial (agent: codex).

- 2026-11-11 — Intel legacy retry again hit stapler 68, after DMG Gatekeeper acceptance; froze repeat retries and audited producer/order/old-client path. Both independent reviews found no byte-level producer defect. Existing protected NVIDIA 7-Zip extracted only the public inner PKG; byte-for-byte equality, size, SHA, and ticket trailer match the direct public PKG. Created a separate unmerged diagnostic worktree/branch for a read-only native verbose direct/mounted/copied probe; no version, main, credentials, or public asset change (agent: codex).

- 2026-11-11 — Both native Mac PKG lifecycles passed; Mac finalizer published exact 34-asset v4.4.4 release 441625880 at 12:29:18Z. Independent anonymous audit passed 34 digests/sizes, 13 sidecars, eight aggregate records, six paired archives, aliases, exact-source rendered wrappers/Windows ZIP binding, and registry source/hash; downloaded Windows pair runtime smokes pass. Linux public smoke passed. Intel legacy bridge hit xcrun stapler exit 68 despite both native pre-public stapler checks passing; ARM bridge queued. Job retry is unavailable while this workflow is active; wait for its conclusion, then retry the exact failed job (agent: codex).

- 2026-11-11 — Windows assembly 43015387367 passed all three jobs; private draft 441625880 has exact 30 assets while public latest remains v4.4.0. Private validation 43015777726 passed all 16, including all four fresh managed takeovers; proof artifact 13853142041 binds source/assets. Mac workflow 43015387253 ARM credential-preflight attempts 1/2 were cancelled before assignment with the same hosted-capacity annotations; targeted unchanged-source attempt 3 is queued. Its package build/native validators/publication have not yet run (agent: codex).

- 2026-11-11 — Release 43007443270 built all six targets and passed Intel archive signing/notarization. ARM signer attempts 1/2 were cancelled with runner_id 0 and explicit hosted macOS ARM capacity annotations before any signing code. Targeted unchanged-source attempt 3 is queued; no extra version/tag or credential-policy change. Complete GitHub publication remains pending (agent: codex).

- 2026-11-11 — Qualified exact head ab014df in PR #39, merged as 9f5110a, passed main CI 43006272757 and OIDC 43006272686, independently verified public crate bytes/source, and pushed only immutable v4.4.4. Watching the native chain before public closure (agent: codex).

- 2026-11-11 — Reproduced the Actions invocation: script-local status shadows the automatic global native result. All five managed verification reads now explicitly use the real global result; both hosts pass direct and nested invocations, including the exact Actions dual-host block/footer and true exit-37 negatives. Independent scoped review passed. Fresh audit also surfaced informational rustix unsoundness warning RUSTSEC-2026-0486; targeted 1.1.6 update clears it within the same unpublished candidate (agent: codex).

- 2026-11-11 — PR #39 exact head 97ff014 passed local gates and independent review; Release plan 43003013700 passed. CI 43003014006 caught the real managed-version check failing in the hosted PowerShell invocation before any merge/crate/tag. Investigating invocation-scope differences within the same unpublished version. The warmed audit job took 20 seconds, including a three-second fresh audit; cached tool installation completed within the same second (agent: codex).

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

- 2026-11-11 — Opened PR #38 at reviewed db5d0a4; final raw-child TMPDIR binding/fixture committed as 2535c72. Release plan 42988722165 passed; CI 42988722084 passed 18/20, with both native Mac Rust suites green but the new fixture failing on Apple mktemp OS-private-base semantics. Correcting that platform assertion before merge; version remains 4.4.3 (agent: codex).

- 2026-11-11 — Final reviewed head 932341c passed CI 42990549773 (20/20) and plan 42990549754. Resolved bot review after a CLI Bash-version/startup-hook regression; PR #38 merged as exact 452d60f. Watching main CI 42991858691 and automatic OIDC 42991858741 before a single v4.4.3 tag (agent: codex).

- 2026-11-11 — Exact-main CI 42991858691 passed all 20; trusted-OIDC publisher 42991858741 succeeded. Verified public crate checksum 365c676267ecd8aed35a3d46828f0f71b484ecf82e5286eb00ead911ad049834 and VCS source452d60f, then pushed only immutable v4.4.3 at that source. Native chain pending (agent: codex).

- 2026-11-11 — Complete v4.4.3 private acceptance audit found one wrapper defect behind four fresh takeover failures; all other completed Windows cases and both native Mac PKG lifecycles passed. Reproduced with real binaries under PS7/WinPS5; fixed complete output/exit capture and added required build-backed pre-tag fixtures, including native exit37 despite matching version text. Preparing one unpublished v4.4.4 candidate; no new tag yet (agent: codex).

- 2026-11-11 — v4.4.4 local Rust 251 tests/fmt/clippy/package/dry-run/release/audit, workflow/shell/provenance, actualpaired-command PS7/WinPS5 suites, and runtime alias/JSON/ASCII checks all pass. Independent scoped review confirms no weakened guard. Hosted currentcandidate proof pending (agent: codex).
- 2026-11-11 — Completed real full/docs qualification: corrected full main CI 43154206746 and proof 43154206867 pass; docs PR 44 CI 43154266882 and main CI 43155508207/proof 43155508130 pass with heavy work and OIDC skipped. Public v4.4.4 inventory/crate unchanged; one completion receipt, separate hardware tasks preserved (agent: codex).
