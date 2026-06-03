---
name: tr300-dev-workflow
description: TR-300's canonical 7-phase development workflow for any non-trivial change, plus the CI gates. Use when starting feature work, planning a multi-PR change, asking "how are changes developed/documented here", setting up TaskCreate tracking, running the local gate (fmt/clippy/test/build/speed), or reproducing CI locally. Encodes plan (read-only, parallel Explore + authoritative research) → upfront task tracking → implement one PR at a time → per-PR F.1–F.6 documentation block → verification + Codex review → ci-tester then git-master commit/push → close out, plus what each GitHub Actions workflow (ci.yml, release.yml, crates-publish.yml) enforces and the Intel macOS coverage policy. Triggers on "development workflow", "how do I add a feature", "plan this change", "the F-block", "CI gates", "reproduce CI", "phases". For cutting an actual release (version bump → tag → publish), use the `release` skill instead.
---

# TR-300 development workflow + CI

The canonical cadence for any non-trivial change, and the CI gates that guard it. The actual
release procedure (version bump → tag → GitHub Release → crates.io) is the separate
[`release`](../release/SKILL.md) skill; this skill stops at "commit + push to master." Summary +
pointers are in [`CLAUDE.md`](../../../CLAUDE.md); the historical ledger is in
[`MASTER_PLAN.md`](../../../MASTER_PLAN.md).

## Development Workflow (canonical — follow for every change)

This is the workflow that proved itself during the v3.10.0 cross-platform accuracy pass. Follow it for any non-trivial change. Lightweight one-line fixes (typos, version bumps) can skip phases 1–2 but never skip phase 5.

### Phase 1 — Plan (read-only)

1. Enter Claude Code plan mode. Plans live at `C:\Users\hey\.claude\plans\<descriptive-name>.md` (the runtime tells you the path).
2. **Explore in parallel.** Spawn up to 3 `Explore` agents simultaneously (single message, multiple tool calls) for codebase context. Each agent gets a focused brief: where the field is collected, what the existing pattern is, what's already best-in-class.
3. **Research authoritative sources before designing.** For platform-specific work, dispatch parallel `general-purpose` agents (model: `opus`) with WebFetch / WebSearch / Firecrawl / Perplexity access. Require citations from Apple Developer Forums, Microsoft Learn, kernel.org, systemd man pages, freedesktop specs, sysinfo crate issues. Verdicts: ✅ best-in-class / ⚠️ acceptable / ❌ inaccurate.
4. **Build the plan incrementally.** Sections: Context · What's Already Best-in-Class (don't redo good work) · Per-Platform Fixes · Cross-Platform Reliability · Speed · New Data Points (with skip list) · Files to Modify · Implementation Task Checklist · Testing Strategy · Phasing & Sequencing · Verification.
5. **Phase the work** into PR-sized chunks (typical: 4–6 PRs). PR #1 is always the foundation primitives that later PRs depend on. Each PR has a docs/version block (`F.1`–`F.6`) at the end.
6. End with `ExitPlanMode` — do not text-prompt for plan approval.

### Phase 2 — Task tracking (`TaskCreate` upfront, `TaskUpdate` as you go)

After plan approval, create:

- **Top-level PR tasks** (one per PR), with `addBlocks`/`addBlockedBy` for sequencing.
- **Sub-task per plan ID** (`[PR1] D.1 …`, `[PR2] A.3 …`, etc.) with the spec verbatim from the plan plus LOC estimate. The user uses these to track progress, so be granular.
- **Per-PR doc block tasks** (`F.1`–`F.6`) and **test tasks** (unit + integration + manual matrix).

`TaskUpdate` to `in_progress` *before* starting any sub-task, and to `completed` *immediately* when done — never batch.

### Phase 3 — Implement (one PR at a time, sequentially)

1. **Read first.** Read every file you'll edit before the first `Edit` call. Don't guess at file structure.
2. **Edit minimally.** No drive-by refactors, no comments that explain *what* the code does, no error handling for impossible cases.
3. **`cargo check` after each meaningful change.** Catches issues while context is fresh.
4. **Run the full local gate after each PR completes** *before* moving to the next PR:
   ```bash
   cargo fmt -- --check
   cargo clippy --all-targets --workspace -- -D warnings
   cargo test --workspace
   cargo build --release
   ./target/release/tr300 --version
   ./target/release/tr300 --fast --json | head -5
   ./target/release/tr300 --ascii          # visual smoke test
   ```
5. **Time `--fast`** on the local platform; record before/after numbers in the PR description.

### Phase 4 — Per-PR documentation (the F-block — never skip)

Every PR completes this block before commit:

- `F.1` — `CHANGELOG.md` new `## [X.Y.Z] — YYYY-MM-DD` section at the top, Keep-a-Changelog voice, **reference task IDs in parens** for traceability. **Then mirror the same section into `HUMAN_CHANGELOG.md` rewritten in plain English** — same version header and date, but with the technical noise stripped (no run IDs, SHAs, error codes, function or API names, registry paths, GUIDs, line counts, memory deltas, task IDs, or crate identifiers). See the [`tr300-changelog`](../tr300-changelog/SKILL.md) skill for the full strip/keep rules. **Never update one without the other.**
- `F.2` — `README.md` updates: flag table, sample output, new subsections.
- `F.3` — `CLAUDE.md` architectural notes for any new pattern (cite man pages / Apple docs / Microsoft Learn URLs inline). For a deep subsystem rule, the home is the matching subsystem skill (`windows-install`, `windows-accuracy`, `windows-distribution-and-update`) with a summary + tripwire in `CLAUDE.md`.
- `F.4` — `Cargo.toml` `version =` bump (minor for new fields/flags; patch for pure accuracy fixes).
- `F.5` — Auto memory writes at `C:\Users\hey\.claude\projects\C--Users-hey-git-qube-machine-report\memory\`: keep `project_tr300_overview.md` (project) up-to-date, append to `feedback_tr300_constraints.md` (feedback) when the user adds a hard rule, update `MEMORY.md` index.
- `F.6` — `TESTING.md` append a `### vX.Y.Z — YYYY-MM-DD` log noting which manual matrix rows were re-verified and on which hardware.

### Phase 5 — Verification + independent review

1. Re-run the full local gate (Phase 3 step 4). Must be green.
2. **Codex review** (`Agent` tool, `subagent_type: "codex:codex-rescue"`) for non-trivial PRs. Use it to spot-check cross-platform safety / YAML / unsafe blocks where a second pair of eyes catches things stale eyes miss. Note: Codex's `gh pr diff` review path needs the PR to actually exist — open the PR first, then ask Codex to review it. Don't over-rely on its findings; double-check.
3. Manual matrix run for the platforms touched (`TESTING.md`).

### Phase 6 — Commit + push

- **Local commit**: `git-master` agent. No `ci-tester` needed for local-only operations.
- **Push to remote**: `ci-tester` agent FIRST. If `[FAIL]`, fix the failures — never skip hooks (`--no-verify`), never bypass signing. Once `ci-tester` is `[PASS]`, hand off to `git-master` for the push.
- **Tag a release**: bump version (already done in `F.4`), commit + push commits, wait for `ci.yml` to go green on the exact commit, then `git tag vX.Y.Z && git push origin vX.Y.Z`. The tag push triggers cargo-dist's `release.yml`. Push a single explicit version tag; do not use broad `git push --tags`. (For the full release procedure, use the [`release`](../release/SKILL.md) skill.)

### Phase 7 — Close out

Mark the parent PR task `completed` in `TaskList`. Move on to the next PR's parent task and start phase 3 again. PR #6 (and other "deferred" tasks) only run if the user explicitly asks after the previous PR lands.

## CI

Three GitHub Actions workflows guard release quality and publication:

- **`.github/workflows/ci.yml`** — runs on every push to master and every pull request. Jobs:
  - `fmt` — `cargo fmt --check` (Linux only)
  - `clippy` — `cargo clippy --all-targets --workspace -- -D warnings` (Linux only)
  - `test` — `cargo test --workspace --all-targets` on Linux + macOS ARM + Windows
  - `build` — `cargo build --release` smoke test on every platform, plus `--version` and `--fast --json` invocation to verify the binary actually runs
  - `speed` — measures `tr300 --fast` median wall-clock across 5 runs on Linux/macOS/Windows; fails the build if any platform's median exceeds the 1500 ms budget. Records numbers in the GitHub Actions step summary so PR reviewers see them.
  - `audit` — `cargo audit` against RustSec advisories (advisory-only via `continue-on-error: true`; flagged vulnerabilities should be triaged within one release cycle but don't gate PRs)
  - `dist-plan` — runs `dist plan` to verify cargo-dist config parses; catches dist regressions before they bite at tag time
- **`.github/workflows/release.yml`** — cargo-dist v0.31.0 release workflow. Triggered by tag push (`vX.Y.Z`). Builds 6 targets and produces shell + PowerShell + MSI installers. It also copies `tr300-installer.*` to legacy `tr-300-installer.*` aliases before creating the GitHub Release so v3.14.2 binaries can self-update after the old package name was removed. `Cargo.toml` sets `allow-dirty = ["ci"]` so `dist plan` accepts this checked-in customization. Regenerate via `dist init` after changing `[workspace.metadata.dist]` in `Cargo.toml`, then preserve the alias-copy step if v3.14.2 compatibility is still needed.
- **`.github/workflows/crates-publish.yml`** — runs after successful `CI` workflow runs from pushes to `master`/`main`, checks out the exact CI-tested SHA, skips already-published crate versions using a descriptive crates.io data-access `User-Agent`, reruns fmt/clippy/tests/package/dry-run with `--locked`, and publishes with the repository Actions secret `CARGO_REGISTRY_TOKEN`.

To reproduce the CI gates locally:

```bash
cargo fmt -- --check
cargo clippy --all-targets --workspace -- -D warnings
cargo test --workspace --all-targets
cargo build --release --workspace
# Speed check (rough — CI uses 5-run median):
time ./target/release/tr300 --fast > /dev/null
```

If a CI job fails, click into the job logs first — `clippy` and `test` failures are usually obvious from the diff. Speed regressions print the per-run times and median in the step summary; correlate against the recent change set.

### Intel macOS coverage policy (v3.11.2+)

**Contract: CI never blocks on Intel; releases still produce the artifact.**

- `.github/workflows/ci.yml` — **no `macos-13` entry**. The default state is "Intel is not in CI, period." Tested matrix is Linux x64 glibc + macOS ARM + Windows x64.
- `[workspace.metadata.dist].targets` in `Cargo.toml` — **still includes `x86_64-apple-darwin`**. cargo-dist's `release.yml` still builds it on a `macos-13` runner at every `vX.Y.Z` tag. The mismatch between tested (3) and shipped (6) targets is intentional.

**Don't re-add `macos-13` to ci.yml** without a concrete reason and a capacity-risk discussion. `macos-13` is GitHub's last public Intel x86_64 macOS hosted runner label and capacity is structurally winding down (no `macos-14`/`-15`/`-latest` Intel variant exists). Pre-removal CI runs queued 3h+ to 15h+ on Intel before manual cancellation; `continue-on-error: true` is theatrical coverage — same UX. Hard removal was the only fix that actually changed the dashboard state.

**Don't drop `x86_64-apple-darwin` from cargo-dist targets** unless `release.yml` starts taking >2h at tag time because of `macos-13` queue depth. Tag cadence is weekly-to-monthly, willing to absorb the wait — users on 2019/2020-era Intel hardware deserve a working binary download.

**For Intel-specific bugs**: reproduce locally on Intel hardware (or one-shot self-hosted runner). The arch-agnostic `#[cfg(target_os = "macos")]` paths mean Apple Silicon CI exercises every line of the macOS code path; bug rate doesn't justify the queue cost.

— Full reasoning, the five concrete CI run IDs / queue times that triggered the removal, the rejected `continue-on-error` and target-drop alternatives, the per-architecture correctness analysis: see [`docs/architecture-decisions.md` § "Intel macOS coverage policy (v3.11.2+)"](../../../docs/architecture-decisions.md#intel-macos-coverage-policy-v3112).

## Source of truth

Summary + heading stubs in [`CLAUDE.md`](../../../CLAUDE.md) (§ "Development Workflow", § "CI", § "Intel macOS coverage policy" — preserved as anchors). Historical ledger and the 7-phase cross-reference in [`MASTER_PLAN.md`](../../../MASTER_PLAN.md). If they disagree with this skill, the docs win — fix this skill.
