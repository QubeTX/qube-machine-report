---
name: tr300-dev-workflow
description: Proportionate TR-300 development, documentation, review, and CI guidance. Use when planning or implementing a change, choosing local checks, reproducing CI, updating evidence, or deciding whether maintenance needs a new version. Keeps full native/release gates for code and release inputs while routing a positive allowlist of record-only Markdown to lightweight CI. For publishing new package or installer bytes, use the release skill.
---

# TR-300 development workflow and CI

Use a short plan for work with dependencies or uncertain behavior, implement a
focused change, test the affected boundary, review it, and merge through the
protected PR. Scale the plan and local checks to the risk. There is no required
runtime-specific plan mode, named agent, seven-phase ceremony, fixed number of
PRs, per-file task explosion, or automatic version/memory update.

Keep `.tasks/TASKS.md` and the active task's status, activity, and verification
accurate when this repository task system is in use. Split dependent work only
when it has a useful independent deliverable. Do not expand the user's scope.

## Choose the CI and version path

[`scripts/ci_policy.py`](../../../scripts/ci_policy.py) is the canonical
fail-closed classifier. Documentation mode is a positive allowlist: root
`AGENTS.md`, `CLAUDE.md`, `CODEX_PROJECT.md`, `MASTER_PLAN.md`, `TESTING.md`,
and Markdown under `docs/` or `.tasks/`. All changed paths must qualify,
including both sides of renames. Unknown, missing, or ambiguous diff inputs
select full mode, as do executable records, symlinks/submodules, or unsupported
file modes. README, license, man pages, changelogs, `.agents`/`.claude`
skills, scripts, workflows, dependencies, and package inputs select full mode.

The stable `CI gate` first validates classifier and heavy-job results. Then
exactly one marker runs: `Full CI qualification` or
`Documentation CI qualification`; the other is skipped. A documentation result cannot authorize a new crate, tag, release
asset, or native publication. Consumer checks bind exact SHA, run, and current
attempt rather than treating any green `CI` conclusion as full qualification.

Keep the version for repository documentation or infrastructure maintenance
unless new published package or distribution bytes are needed. Full CI is a
verification choice, not an automatic version bump. An actual release uses
[`release`](../release/SKILL.md), with the correct semver bump, immutable tag,
and full private/native/public acceptance chain.

## Work and local verification

- Read the changed code and owning architecture decision before editing.
  Research primary platform sources when the design depends on uncertain OS
  behavior. Keep the implementation focused.
- For record-only Markdown, review facts/links/encoding and run
  `git diff --check`. Run the classifier's tests when its policy changes.
  Do not compile Rust or run installers just to record a passed release.
- For Rust changes, run focused meaningful tests and applicable fmt/clippy;
  add release-build/runtime/speed checks when the change affects those paths.
  Full release qualification still requires all supported native CI legs.
- For workflows, installers, or publishing changes, run applicable parser,
  lint, provenance, transaction, and native fixtures. Changed operational
  skills select full CI; being Markdown does not make them record-only.
- Repeat failed or affected checks and their dependents after a change. Reuse
  passed evidence for identical inputs; no rule requires restarting unrelated
  successful work after every edit.
- Self-review and get a focused independent review for security, unsafe,
  cross-platform, installer, or workflow changes. Review record-only updates
  proportionately. Another pass is useful when changes/findings warrant it;
  no specific agent product or model is required.

Update only affected user/agent docs. When writing actual release notes, keep
`CHANGELOG.md` and `HUMAN_CHANGELOG.md` paired under the same version/date;
ordinary evidence or infrastructure work does not require a new section or
version. Never write host-specific memory merely because a checklist says so.
Commit a clear focused change with normal hooks/signing, push the branch, and
merge only its reviewed exact head after required checks and actionable review
threads resolve. Do not tag from this development workflow.

## CI

The full CI path shares locked all-target tests, a release build, and runtime
smokes on each of five existing runners: Linux x86_64 `ubuntu-latest`, Linux
ARM64 `ubuntu-22.04-arm`, macOS ARM `macos-15`, macOS Intel `macos-15-intel`,
and Windows x86_64 `windows-latest`. Linux x86_64, macOS ARM, and Windows run
one warmup and five measured fast-mode samples, requiring median below
1500 ms, before expensive native fixtures. Both native Mac legs retain exact
system-Bash-3.2 Apple staging/provenance and PKG fixtures. Native Intel remains
blocking; Rosetta is supplementary.

The other full jobs retain fmt, clippy, fresh advisory audit, actual validated
cargo-dist plan, executable workflow/provenance fixtures, pinned Windows
bootstrap, and both MSI/Inno source/transition tests. Consolidation removes
eight duplicate allocations (13 split test/build/speed allocations become
five platform legs): the designed full path has 12 heavy jobs plus
classifier, `CI gate`, and full qualification (15 allocations); record-only
mode has classifier, gate, and documentation qualification (three). These
counts describe the workflow design, not measured elapsed-time improvements.

Record-only mode skips expensive build/installer jobs, and the Release
workflow ignores the same exact allowlisted PR paths for its plan job. The crate preflight can
only prove an already published unyanked version using public bytes, VCS and
trusted-publisher metadata, immutable tag ancestry, and unchanged packaged Git
inputs. It performs no Cargo compilation/packaging/dry-run and has no OIDC or
registry publication. Full maintenance with unchanged package inputs may use
the same fast existing-version proof; only full qualification may reach the
normal fresh publishing path.

The six persistent workflows retain their separate responsibilities:

- `ci.yml`: classify and qualify the exact PR/main input.
- `crates-publish.yml`: prove an existing version read-only, or after full CI
  validate exact package bytes without credentials and pass data to a fresh
  protected OIDC publisher that executes no package code while credentialed.
- `release.yml`: require full exact-main qualification and the proven crate,
  build cargo-dist inputs, sign/notarize Apple archives in fresh checkout-free
  jobs, and create the private exact 24-asset draft.
- `windows-installers.yml`: add six Windows assets through the fresh publisher,
  producing 30 assets.
- `windows-installer-validation.yml`: attest the exact private 30-asset fresh,
  transition, rollback, scope, uninstall, and cleanup matrix; after publication,
  run the actual public updater matrix.
- `macos-installer.yml`: retain both native Apple acceptance jobs and exact
  proof custody; its fresh finalizer alone adds four assets and publishes the
  verified 34-asset result, followed by public smokes.

Signing/publication jobs retain fresh credential boundaries; checked-out
builders and candidate execution remain read-only. System Bash 3.2,
fail-closed fixed inventories, exact IDs/digests, mandatory checksums, scope
and rollback policy, and both native architectures remain requirements.

## Diagnose and close once

Read failures before changing code. Runner-allocation/capacity annotations or
an Apple/network-service error with identical artifacts call for targeted
unchanged-source retries and focused diagnostics. Retry failed jobs and their
required dependents without discarding successful proof. A reproducible bug
needs a code fix; a service outage does not justify another product version.
Never bypass the native/public gates to finish faster.

Keep one verification record and handoff, link the authoritative PR/run checks,
and record one evidence/closure receipt after the required results exist.
Do not open a new PR only to record the preceding record PR's checks, then
repeat. Do not complete a task or milestone over genuinely pending gates.

### Intel macOS coverage policy (v4.1.0 current; supersedes v3.11.2)

**Contract: native Apple Silicon and native Intel are blocking for shared/macOS
release changes.**

- `.github/workflows/ci.yml` uses `macos-15` and `macos-15-intel` for native
  ARM64/x86_64 test and release-build coverage.
- `.github/workflows/macos-installer.yml` installs and exercises the universal
  PKG-in-DMG on those same architectures before publication.
- `[workspace.metadata.dist].targets` keeps both Apple target triples.
- Both native runner labels expose Apple's system `/bin/bash` 3.2 for workflow
  `bash` steps. Keep the ARM and Intel compatibility fixtures blocking: each
  executes the exact release-staging path and Bash-3.2 syntax coverage before a
  tag exists. A successful parse under a newer local Bash does not substitute
  for those runtime legs.

The old `macos-13` capacity exception and Rosetta-as-primary-Intel proof are
historical only. Do not remove `macos-15-intel` based on the old queue record;
GitHub now provides the explicit native Intel label. Rosetta may supplement but
never replace it. A physical Mac is optional unless hosted CI exposes a GUI-only
defect.

— Full reasoning, the five concrete CI run IDs / queue times that triggered the removal, the rejected `continue-on-error` and target-drop alternatives, the per-architecture correctness analysis: see [`docs/architecture-decisions.md` § "Intel macOS coverage policy (v3.11.2+)"](../../../docs/architecture-decisions.md#intel-macos-coverage-policy-v3112).

## Source of truth

[`AGENTS.md`](../../../AGENTS.md) defines repository contracts,
[`CLAUDE.md`](../../../CLAUDE.md) routes edit-time guidance, and
[`docs/architecture-decisions.md`](../../../docs/architecture-decisions.md)
records why. The classifier and workflow are authoritative for exact paths,
marker names, and jobs. `MASTER_PLAN.md` and `TESTING.md` retain release
history; historical job counts and the retired runtime-specific ceremony do
not override the current proportionate process.
