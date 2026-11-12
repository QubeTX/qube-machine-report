TT;DR: Simplify this repository's CI and release workflow so documentation records avoid native builds and repeated release work while actual code and release changes keep full acceptance. Implementation is active; hosted qualification and deployment remain.

## Why
Direct operator request on 2026-11-11 to make all release/deployment processes easier, commit and deploy the improvements, and specifically skip expensive builds/installer tests for documentation-only changes. This follows six unallocated ARM retries on the otherwise published v4.4.4 documentation follow-up.

## Scope
Strict positive record-path classifier; stable aggregate and exclusive full/docs qualification; consolidated five-platform tests, release builds and existing three-platform performance checks; no-build existing-version registry proof; exact full-CI checks at every publication boundary; concise nonrecursive release/dev instructions. Preserve all native lifecycle cases, immutable tags/assets/crates, isolated signing and registry custody, current v4.4.4 product bytes, original dirty checkout, and separate hardware tasks.

## Plan
Implement and adversarially test classifier/aggregate/consumer guards, independently review, qualify one full PR, merge and verify exact-main full CI and existing-version no-op. Require the stable observed CI gate on main. Finish with a docs-only receipt PR that demonstrates heavy jobs are deliberately skipped and closes the earlier qualified publication records without another recursive receipt.

## Impact
Full CI allocates five combined native test/build runners instead of thirteen separate test/build/speed runners; docs allocate no native build or installer runners. New publication proof checks prevent docs success from authorizing a tag or new crate. The sensitive risk is misclassifying packaged inputs or accepting skipped/mismatched job evidence; explicit negative tests and independent review own that boundary.

## Acceptance
Functional bar: workflow deployed and real full/docs modes observed; current public distribution unchanged. Evidence bar: classifier/aggregate negative cases, provenance fixtures, workflow lint, independent review, exact-head PR/full main checks, metadata/byte existing-version proof without OIDC, and actual docs-only PR/main runs. Gate ownership: user scope plus reviewed repository policy; native runtime/security acceptance remains mandatory for product/release inputs. Capacity failures warrant diagnosis and bounded targeted retries, never a bypass/new version.

## Verification
- [x] Classifier rejects unknown/package/code/workflow/policy paths, links, renames and unverified comparisons
- [x] Aggregate rejects failed/cancelled/skipped full work and unexpected docs work
- [x] Release/native/crates consumers distinguish full from docs with exact run/attempt binding
- [x] Full local workflow/provenance tests and independent review pass
- [ ] Exact-head PR and exact-main full CI pass after merge
- [ ] Existing-version proof succeeds with no package compilation or registry credential
- [ ] Main requires CI gate; actual docs-only PR/main skip expensive work
- [ ] v4.4.4 immutable public bytes and unrelated hardware work remain unchanged

## Status
Active in C:/Users/hey/git/qube-machine-report-security-pr35. PR #42 passed all 15 allocated full-CI jobs and Release plan, merged as 8d913502e8470945605f79f94c40247f382dd655, and main ruleset27090739 now enforces pull requests plus the Actions CI gate. The first real docs probe43152542124 passed classification/aggregation and skipped heavy jobs, but GitHub's inherited success condition also skipped its documentation marker. That green run is not qualified mode evidence. PR #43 adds explicit status functions to both marker jobs while preserving successful classification/aggregate requirements. Focused regression/lint checks pass; full hosted correction and a fresh docs PR/main round trip remain. No version/tag/public bytes change. sec35/q44/v44 remain open.

## Activity
- 2026-11-11 — Created from direct operator request; split independent publication/registry/process review and implementation, preserving existing isolated checkout.
- 2026-11-11 — Local full provenance suite and independent security reviews passed; corrected private-release fallback and isolated the data-only Python interpreter. Actual v4.4.4 registry bytes/tag/source proof passed without Cargo/OIDC. Product/package inputs remain unchanged; hosted qualification pending.
- 2026-11-11 — PR42 full CI43151114831 passed15 allocated jobs in620seconds; plan43151114880 passed and protected merge8d91350 deployed. Real docs probe43152542124 exposed inherited skip propagation into the mode marker; fixed the explicit condition and retained the unqualified probe as evidence. Main source qualification and the corrected full/docs paths remain tracked.
