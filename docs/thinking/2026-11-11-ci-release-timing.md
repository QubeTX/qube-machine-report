# CI and release timing — 2026-11-11

The repository builds Rust commands, platform archives, Windows installers, and Mac packages on hosted operating-system runners. It does not build container images. Native installer acceptance and hosted-runner availability account for much of the elapsed time.

| Observed run | Elapsed time | Interpretation |
| --- | --- | --- |
| [v4.4.2 main CI](https://github.com/QubeTX/qube-machine-report/actions/runs/42972929754) | 11.13 min | Intel direct-PKG fixture consumed 7.95 min. |
| [Matching crate publication](https://github.com/QubeTX/qube-machine-report/actions/runs/42972929890) | 13.17 min | 11.27 min waited for that CI; adding both totals double-counts the wait. |
| Same CI security audit | 3.25 min tool installation; 0.05 min audit | Rebuilding the pinned tool was avoidable setup work. |
| [First v4.4.4 PR CI](https://github.com/QubeTX/qube-machine-report/actions/runs/43003014006) audit job | 20 sec total; 3 sec fresh audit | Reusing the pinned audit binary worked. Tool-install step completed within one second. |
| [Qualified v4.4.4 PR CI](https://github.com/QubeTX/qube-machine-report/actions/runs/43005013711) | 7.93 min | Intel job 7.87 min; ARM Mac 4.38 min; Windows installer source gate 2.42 min. |
| v4.4.3 archive signer, first attempt | 18 min queued, then hosted-capacity cancellation | No runner was assigned. A retry of the same immutable source passed; no new version was needed for this external failure. |

The [v4.4.4 tagged Release](https://github.com/QubeTX/qube-machine-report/actions/runs/43007443270) took 57.40 minutes through attempt 3. All six builds passed; the ARM archive signer was cancelled twice before assignment with explicit hosted-capacity annotations, then passed unchanged. This elapsed duration is dominated by allocation waits rather than image builds or repeated code diagnosis.

These are individual measurements, not guaranteed durations or an isolated benchmark of the cache change. Parallel jobs mean saving runner minutes need not shorten the critical path by the same amount.

## Changes made during this security release

- Reuse cargo-audit 0.22.2 through the existing pinned Rust cache action, check the tool version before use, and run a fresh advisory scan every time. Cache writes are restricted to protected main.
- Build the real paired Windows commands before the transaction test under both PowerShell hosts. Test direct and nested invocation, stale caller status, wrong version, and actual nonzero exits. The pre-release CI caught the nested-scope defect before another immutable tag.
- Inspect compact job summaries while waiting; retrieve detailed logs for failures. Waiting is elapsed time, while repeated log retrieval and diagnosis consume tokens.

## CI workflow implementation

[PR #42](https://github.com/QubeTX/qube-machine-report/pull/42) merged the
reviewed scope classifier, consolidated native jobs, and existing-version
registry proof without changing v4.4.4 product inputs or published bytes.
Its exact candidate `acdd74bf8779236b259411f931a88b1dc600e151` passed
[full CI](https://github.com/QubeTX/qube-machine-report/actions/runs/43151114831)
on the first attempt: 15 allocated jobs succeeded, the documentation marker
was skipped, and the separate
[Release plan](https://github.com/QubeTX/qube-machine-report/actions/runs/43151114880)
passed. CI elapsed time was 620 seconds (10 minutes 20 seconds).

The measured result proves the smaller job inventory, not a full-run elapsed
speedup. Native Intel installer fixtures remain on the critical path, and the
first consolidated runners also built their release caches. Future timings
must distinguish cache state and allocation delay from actual test duration.

- The five existing platform legs now share tests, a release build, and paired
  command smokes. Linux x86_64, macOS ARM, and Windows reuse that release binary
  for the existing warmup/five-sample/1500 ms performance gate. This replaces
  thirteen separate test/build/speed allocations with five.
- The positive record-only policy is in
  [`scripts/ci_policy.py`](../../scripts/ci_policy.py). It permits the five named
  root record files and Markdown under `docs/` and `.tasks/`; package inputs,
  operational skills, workflows/scripts, unknown paths and ambiguous history
  select full validation. The separate Release PR plan uses matching record
  exclusions, while the required CI workflow always starts.
- `CI gate` validates all required results. Its full/documentation markers
  identify the exact mode; release consumers require full qualification from
  the same run and attempt. Main ruleset
  [27090739](https://github.com/QubeTX/qube-machine-report/rules/27090739)
  enforces pull requests and the GitHub Actions `CI gate` against current main.
- Registry preflight can prove an existing immutable public version from
  metadata, downloaded checksum/VCS/trusted-publisher provenance, tag ancestry,
  and unchanged packaged Git inputs without Cargo or OIDC. New/changed package
  work retains the original full validator and fresh publisher. A private or
  nonfinal release selects that original path in full mode and is refused in
  documentation mode.

The first actual documentation-only probe,
[CI 43152542124](https://github.com/QubeTX/qube-machine-report/actions/runs/43152542124),
correctly skipped all heavy jobs and passed classification/aggregation, but
GitHub also skipped the documentation marker through its inherited default
success condition. Its green workflow conclusion therefore was not qualified
publication evidence. Both marker jobs now explicitly override inherited
skips with `always()` while still requiring successful classification, the
exact mode, and a successful aggregate. The provenance and lightweight policy
tests guard that condition. Full workflow qualification and a fresh actual
documentation PR/main round trip remain required before completion.

## Remaining bounded follow-up options

1. Profile the Intel direct-PKG fixture by case before changing its assertions. Remove redundant setup and reuse read-only fixture preparation while retaining every ownership-refusal and native install oracle.
2. Review repeated read-only builds between PR, exact-main CI, crate validation, and release. Any reuse must bind the exact source, toolchain, target, artifact identity, and digest. Keep the fresh credentialed publishers and their data-only inputs.
3. Evaluate whether data-only archive signing and duplicate Installer-identity preflights need architecture-specific runners. The universal package builder already signs on Intel. Any change needs an architecture-decision update and native qualification; retain fresh credential isolation and both architectures’ real build/install/update acceptance. This is a proposal, not validation of a changed runner matrix.
4. Measure the new documentation route and full-run cache behavior. Preserve its explicit classifier, required aggregate, exact-mode publication guards, and existing-version byte/source proof; broad path filters alone remain insufficient.

Keep native Intel/ARM acceptance, installer transitions, integrity checks, and signing/notarization gates. Larger CI maintenance does not require a product version bump unless it changes a published payload or an immutable release must be fixed forward. The current security release follows the established qualified chain.

The public Mac legacy follow-up also exposed opaque failure reporting: the old client hid stapler stdout, requiring closed diagnostic PR #40 to distinguish byte/path hypotheses from an actual Apple CloudKit lost connection. Better retained failure diagnostics and decoupling independently bound post-public platform smokes are candidates for a separate CI-only review. Existing equality/custody guards remain in force.
