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

## Bounded follow-up options

1. Profile the Intel direct-PKG fixture by case before changing its assertions. Remove redundant setup and reuse read-only fixture preparation while retaining every ownership-refusal and native install oracle.
2. Review repeated read-only builds between PR, exact-main CI, crate validation, and release. Any reuse must bind the exact source, toolchain, target, artifact identity, and digest. Keep the fresh credentialed publishers and their data-only inputs.
3. Evaluate whether data-only archive signing and duplicate Installer-identity preflights need architecture-specific runners. The universal package builder already signs on Intel. Any change needs an architecture-decision update and native qualification; retain fresh credential isolation and both architectures’ real build/install/update acceptance. This is a proposal, not validation of a changed runner matrix.
4. Design a fail-closed docs-only path as a CI-only PR. Include branch-protection, the strict 20-job source validator, and the idempotent crates publisher in that design; a path filter alone is insufficient under the current contracts.

Keep native Intel/ARM acceptance, installer transitions, integrity checks, and signing/notarization gates. Larger CI maintenance does not require a product version bump unless it changes a published payload or an immutable release must be fixed forward. The current security release follows the established qualified chain.

The public Mac legacy follow-up also exposed opaque failure reporting: the old client hid stapler stdout, requiring closed diagnostic PR #40 to distinguish byte/path hypotheses from an actual Apple CloudKit lost connection. Better retained failure diagnostics and decoupling independently bound post-public platform smokes are candidates for a separate CI-only review. Existing equality/custody guards remain in force.
