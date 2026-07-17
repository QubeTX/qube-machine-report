---
name: release
description: Cut a TR-300 release end-to-end — from a main-branch commit through GitHub Release + crates.io publication. Use whenever the user asks to ship, release, deploy, cut a release, tag a release, publish a version, push a version bump to crates.io, or watch a release CI run. Also use when fixing a CI or release.yml failure during an in-flight release. Encodes the full ordered workflow: pre-release local gates, version bump in lockstep across the documentation set, main push → wait for ci.yml green → wait for crates-publish.yml → tag push → watch release.yml → fix-forward loop on failure. Trigger on phrases like "ship v3.14.4", "let's release", "cut the release", "deploy this", "tag and release", "release time", or just "deploy" / "ship it" in this repo's context, even when the user doesn't name a version number.
---

# TR-300 release workflow

Drive a TR-300 release from version-bump through published GitHub Release + crates.io, watching CI at each step and fixing forward when something fails. The canonical source-of-truth for these rules lives in [`AGENTS.md` § "Release checklist"](../../../AGENTS.md) and [`AGENTS.md` § "Release Process"](../../../AGENTS.md). This skill encodes the reconciled workflow plus the watch-and-fix-forward loop, so an agent can drive a release with only the user's go-ahead.

The workflow is twelve ordered steps. Skipping or reordering them tends to fail in non-obvious ways — the pitfalls in § 14 are real failures that shipped at some point.

---

## § 1 Pre-flight checks

Before starting, confirm:

- Working tree clean on `main` (`git status` returns clean).
- All feature work for the version you're about to cut is already committed.
- MSRV is unchanged — OR — both `Cargo.toml` `rust-version` and `rust-toolchain.toml` `channel` were bumped in the same earlier commit. This is the MSRV lockstep rule from [`AGENTS.md` § "MSRV policy"](../../../AGENTS.md). Touching one without the other will break either `ci.yml` (rustfmt/clippy missing) or `release.yml` (rustc older than required).
- `gh` CLI is authenticated for this repo (`gh auth status`).

You'll never use `--no-verify`, `--no-gpg-sign`, or `git push --tags` anywhere in this workflow. If a pre-commit hook fails, fix the underlying issue and create a new commit — never amend after a hook failure (the commit didn't actually happen, so amend would touch the previous commit).

---

## § 2 Pick the version bump

| Bump | When | Example from history |
|---|---|---|
| Major (`X.Y.Z → (X+1).0.0`) | Public Rust source compatibility breaks even if CLI/JSON remain compatible | v3.17.0 → v4.0.0 (public records gained fields/signatures changed) |
| Minor (`X.Y.0 → X.(Y+1).0`) | New user-visible flags, fields, behavior, or surface | v3.13.1 → v3.14.0 (positional `update`/`install`/`uninstall` actions) |
| Patch (`X.Y.Z → X.Y.(Z+1)`) | Accuracy fixes, release-infra fixes, doc-only releases | v3.13.0 → v3.13.1 (`rust-toolchain.toml` MSRV pin) |

If the user said "patch bump" / "minor bump" explicitly, follow their direction. Otherwise propose the bump that matches the diff and ask if there's any reason it should be different.

---

## § 3 Bump `Cargo.toml`

Update `version = "X.Y.Z"` in `Cargo.toml`. Single edit.

---

## § 4 Update the documentation set

The reconciled canonical list is ten files. The first three are mandatory on any user-visible release. Files 4–8 update when the noted condition applies. Files 9–10 are host-specific.

| # | File | What goes in |
|---|---|---|
| 1 | `CHANGELOG.md` | Prepend a `## [X.Y.Z] - YYYY-MM-DD` block at the top of the file (above `[Unreleased]` if present, or replacing the `Unreleased` heading). Keep-a-Changelog voice: grouped under `### Added` / `### Changed` / `### Fixed` / `### Internal`. Reference task IDs in parens for traceability. Match the voice of the most recent entry — read it first. |
| 1b | `HUMAN_CHANGELOG.md` | Mirror the same release/date/groupings in plain English. Strip run IDs, SHAs, APIs, paths, GUIDs, task IDs, and dependency identifiers; keep user commands, platforms/installers, behavior, and benefit. Never update only one changelog. |
| 2 | `README.md` | Update flag tables, install snippets, or sample output if anything user-visible changed. Skip if release is purely internal. |
| 3 | `TESTING.md` | Append a `### vX.Y.Z — YYYY-MM-DD` block to the "Per-release verification log" listing local gates that passed, runtime smoke results, and the CI/crates-publish/release.yml run IDs (the IDs come after pushing — fill them in during step 12). Match the format of the most recent entry. |
| 4 | `CODEX_PROJECT.md` | Only if release/install/update/deployment behavior changed. Otherwise skip. |
| 5 | `AGENTS.md` | Bump the "Last verified against source" date. Update any drifted fact (current version line, MSRV, dependency versions). |
| 6 | `CLAUDE.md` | Add architectural notes for any new pattern introduced this release. Cite source URLs inline (man pages, Apple docs, Microsoft Learn). Skip if no new pattern. |
| 7 | `MASTER_PLAN.md` | Bump "Last updated" and "Current version" lines. Append or update the "Tag status" bulleted entry **after** the release publishes (the run IDs and asset count come from the actual published runs in step 12). |
| 8 | `docs/architecture-decisions.md` | Only when rationale or release workflow itself changes. Skip otherwise. |
| 9 | `/Users/realemmetts/.codex/AGENTS.md` | Only when repo deployment workflow changes AND the path exists on the current host. This is the original macOS author's global Codex guide; Windows hosts silently skip. Check `Test-Path` (or `test -f`) before editing. |
| 10 | Auto-memory at `~/.Codex/projects/<host-flavored-path>/memory/` | Host-specific and optional. `MASTER_PLAN.md` § 0 says: "Don't recreate it on other machines — `AGENTS.md` is authoritative." Never block a release on this. |

Why so many files: TR-300's docs are organized around different reader audiences (agents, contributors, users, future-you-on-a-fresh-machine) and the project has chosen redundancy over a single source. The release commit is the natural moment to flush updates across all of them so they don't drift.

---

## § 5 Run the local release gates

Run these in order. Each must pass before moving on. If any fails, fix and re-run from the start — don't push known-failing code and expect CI to be the gate.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --workspace -- -D warnings
cargo test --workspace --all-targets
cargo package --locked --list
cargo publish --dry-run --locked
cargo build --release --workspace
cargo audit
actionlint .github/workflows/*.yml
shellcheck scripts/sign-notarize-macos.sh
```

Then runtime smoke on the freshly-built binary:

```bash
./target/release/tr300 --version             # expect "tr300 X.Y.Z" — the version you just set
./target/release/tr300 --fast --json | python3 -m json.tool   # parses cleanly
./target/release/tr300 --ascii               # visual smoke; table renders
```

After any shared/macOS/release-workflow change, also run native Apple Silicon
and Rosetta all-target tests, both release builds, full/fast/JSON/ASCII/
manual-save/no-write smokes, and privacy/parity checks. After Apple credential,
script, archive, or workflow-input changes, run a real cargo-dist archive
Developer ID signing → Apple `Accepted` → repack → sidecar/manifest checksum
test. Do not tag from Windows/Linux-only evidence after reopening the Mac gate.

Why both `cargo package --locked --list` and `cargo publish --dry-run --locked` — they catch different classes of issue. `package --list` shows what files end up in the crate (look for accidentally-included `.env` / `target/` / fixtures; the release commit is the cheapest time to catch them). `publish --dry-run` runs the full publish pipeline minus the upload — it catches the `Cargo.lock` resolver mismatches, version conflicts, and credential issues that bite at the publish step otherwise.

`Cargo.lock` is intentionally tracked — both `cargo package --locked` and the CI publish workflow use `--locked` to guarantee the same resolved dependency graph everywhere.

---

## § 6 Optional Codex review

For non-trivial diffs (cross-platform `unsafe` code, workflow YAML changes, anything you'd want a second pair of eyes on), invoke Codex review:

```
Agent tool with subagent_type: "codex:codex-rescue"
```

For release commits this is usually a self-review of the diff. The full Codex review is more appropriate on feature commits earlier in the cycle. If you want Codex's `gh pr diff` review path, the PR must exist first — open it, then ask Codex to review.

The TR-300 project ships release commits direct to `main` with no PR (see recent history: `62a2006`, `8ab4aa4`, `25305d8`). Skip this step on routine releases unless something in the diff makes you want a second opinion.

---

## § 7 Commit and push main

Commit:

```bash
git add <specific files>   # avoid `git add -A` — sensitive files have slipped in this way
git commit -m "release: vX.Y.Z - <one-line summary>"
```

For multi-line bodies, use a HEREDOC so the formatting survives:

```bash
git commit -m "$(cat <<'EOF'
release: vX.Y.Z - <one-line summary>

<longer body if needed>

Co-Authored-By: Codex Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Then push:

```bash
git push origin main
```

Never `--no-verify`. Never `--amend` after a hook failure (the commit didn't happen, so amend would silently modify the previous commit).

---

## § 8 Watch `ci.yml` on the exact release commit

```bash
gh run list --branch main --limit 5
# Identify the run for your commit SHA — look at the "DISPLAY TITLE" column
gh run watch <run-id>
```

Green means all of these passed:
- `fmt` (cargo fmt --check on Linux)
- `clippy` (cargo clippy -D warnings on Linux)
- `test` (cargo test --workspace --all-targets on Linux + macOS ARM + Windows)
- `build` (release build smoke on all three platforms)
- `speed` (5-run median of `tr300 --fast` < 1500 ms on all three platforms)
- `audit` (cargo audit, blocking)
- `dist-plan` (cargo-dist config parses)

If any job fails:

```bash
gh run view <run-id> --log-failed
```

Then fix on `main` (same version — the tag hasn't been pushed yet), commit, push, watch again. See § 13 for the fix-forward loop.

---

## § 9 Wait for `crates-publish.yml`

`.github/workflows/crates-publish.yml` is triggered automatically via `workflow_run` after `ci.yml` succeeds on `main`. It checks out the exact CI-tested SHA, reruns fmt/clippy/tests/package/dry-run, and publishes to crates.io.

```bash
gh run list --workflow=crates-publish.yml --branch main --limit 3
gh run watch <run-id>
```

Two acceptable outcomes:
- **Published** — `tr300 X.Y.Z` is now on crates.io.
- **Skipped** — the workflow ran but found the version was already published (the version check uses a descriptive `User-Agent` against `crates.io/api/v1/crates/tr300/X.Y.Z`). This happens on re-runs or on doc-only releases where the version didn't actually change.

A **failed** run halts the release. Diagnose with `gh run view <run-id> --log-failed`. Common failures:
- Missing `CARGO_REGISTRY_TOKEN` secret (configuration issue — talk to repo admin).
- `cargo publish --dry-run --locked` regression (something changed that wasn't caught locally).
- crates.io API transient — re-run with `gh run rerun <run-id>`.

Don't push the tag until this resolves.

---

## § 10 Tag and push the single tag

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

**Never** `git push --tags`. The explicit single-tag push is the documented trigger for `release.yml`; a broad `--tags` push can drag stale tags along and is one of the destructive operations called out in [`AGENTS.md`](../../../AGENTS.md).

The tag name has a `v` prefix (`v3.14.4`), but the cargo version is bare (`3.14.4`). This matches the `**[0-9]+.[0-9]+.[0-9]+*` glob in [`release.yml`](../../../.github/workflows/release.yml).

---

## § 11 Watch `release.yml`

```bash
gh run list --workflow=release.yml --limit 3
gh run watch <run-id>
```

`release.yml` is generated by cargo-dist v0.31.0 and checked in with the
documented compatibility-alias and fail-closed Apple signing/notarization
customizations. Success looks like 10 jobs passing:

- `plan` (1 job)
- `build-local-artifacts` (6 jobs — one per target: `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`)
- `build-global-artifacts` (1 job)
- `host` (1 job)
- `announce` (1 job)

For both Apple build jobs, verify that the pre-upload signing step completed,
Apple returned `Accepted`, and cargo-dist uploaded the repacked signed archive.
Missing credentials, signing failure, or any non-`Accepted` result must fail the
job; there is deliberately no unsigned fallback.

After `release.yml` succeeds, watch the matching `windows-installers.yml` run.
It downloads the release's Windows binary, builds the Corporate MSI plus Global
and Corporate Inno Setup EXEs, and uploads those three installers and their
three SHA-256 sidecars.

The complete published GitHub Release has **28 assets for v4.0.0+**:
- 6 platform binaries (tarballs / zip)
- 1 MSI installer for Windows
- 1 source tarball
- 2 canonical installer scripts (`tr300-installer.sh`, `tr300-installer.ps1`)
- 2 legacy alias installer scripts (`tr-300-installer.sh`, `tr-300-installer.ps1`) — preserved for v3.14.2 updater compatibility
- Cargo-dist metadata and checksums (22 cargo-dist assets total)
- 1 Corporate MSI plus 2 Inno Setup EXEs
- 3 SHA-256 sidecars for those additional Windows installers

Download both public macOS archives and their sidecars. Verify each checksum,
extract each binary, and confirm its version plus Developer ID signature,
Team ID, timestamp, hardened-runtime flag, and Apple trust chain with
`codesign`. The notarization proof is the `Accepted` workflow log; bare CLI
archives are not expected to carry a stapled app ticket.

If `release.yml` fails, you can't re-tag (tags are immutable). See § 13 for the fix-forward path.

---

## § 12 Append post-release verification log entries

After the GitHub Release publishes successfully, collect:
- `main` CI run ID (from § 8)
- crates-publish run ID (from § 9)
- release.yml run ID (from § 11)
- windows-installers.yml run ID
- both Apple `Accepted` results and public macOS signature/checksum evidence
- GitHub Release asset count (should be 28 for v4.0.0+)

Then update two files:

**`TESTING.md`** — append a new `### vX.Y.Z — YYYY-MM-DD` block to the "Per-release verification log" section. Match the format of the most recent entry. Include:
- One-paragraph summary of what shipped
- Local gates pass note (the cargo commands from § 5)
- Runtime smoke results
- "**CI verification** — `main` CI run <id> passed..."
- "**Crates.io verification** — crates-publish run <id> published `tr300` X.Y.Z..."
- "**Release verification** — release.yml run <id> passed, both Apple jobs were accepted and signed archives verified, windows-installers.yml run <id> passed, and the GitHub Release published N assets"

**`MASTER_PLAN.md`** — locate the "Tag status (as of YYYY-MM-DD)" bulleted list and append a new entry for `vX.Y.Z`:
```
- `vX.Y.Z` (`<commit-sha>`): tagged + pushed; CI run <id> succeeded across fmt/clippy/test/build/audit/dist-plan/speed gates; crates-publish run <id> published `tr300` X.Y.Z to crates.io after rerunning fmt/clippy/tests/package/dry-run; release.yml run <id> succeeded across plan + six target artifact builds + global artifacts + host + announce, including Accepted notarization and verified Developer ID signatures for both macOS archives; windows-installers.yml run <id> published the three supplemental Windows installers and sidecars; GitHub Release published with 28 assets, including canonical `tr300-installer.*` installers and `tr-300-installer.*` compatibility aliases.
```

Also bump the "Last updated" and "Current version" lines at the top of `MASTER_PLAN.md`.

Then commit and push these doc updates as a follow-up:

```bash
git add CHANGELOG.md TESTING.md MASTER_PLAN.md   # whichever you touched
git commit -m "docs: record vX.Y.Z publication status"
git push origin main
```

This follow-up commit kicks off another `ci.yml` run, but it's doc-only, so it should be green by default. It also kicks off another `crates-publish.yml` run that will SKIP (the version is already published). Both of those are fine.

The release is now complete.

---

## § 13 Fix-forward loop

Things fail. The recovery path depends on whether the tag has been pushed yet.

### Pre-tag (between § 7 push and § 10 tag)

If `ci.yml` or `crates-publish.yml` fails:

1. `gh run view <run-id> --log-failed` to diagnose.
2. Fix the issue on `main` with a new commit. **Keep the same version** — the tag hasn't moved yet, so `Cargo.toml` `version` stays as you set it in § 3.
3. Push the fix: `git push origin main`.
4. Watch CI again from § 8.

Repeat until both `ci.yml` and `crates-publish.yml` are green/skipped, then proceed to § 10.

### Post-tag (after § 10 push)

Tags are immutable. If `release.yml` fails after the tag is pushed:

1. `gh run view <run-id> --log-failed` to diagnose.
2. Fix the issue and **bump the version as a patch** (`vX.Y.Z → vX.Y.(Z+1)`).
3. Go back to § 3 and run the workflow again for the patch version.

The canonical example is v3.13.0 → v3.13.1: v3.13.0's `release.yml` failed because three runner images shipped rustc 1.94.1 (below MSRV 1.95). The fix was adding `rust-toolchain.toml`. Instead of deleting and re-pushing the v3.13.0 tag, v3.13.1 was cut as a fresh patch release. The v3.13.0 tag remains in git as a historic record of the failure.

**Never** delete and re-push a tag. That's a destructive shared-state operation; it confuses anyone who has already fetched the tag and breaks Cargo's expectation that semver versions are immutable.

### When CI is flaky vs broken

If a run fails for what looks like a transient reason (network blip, GitHub Actions queue, crates.io API hiccup), `gh run rerun <run-id>` is the lightweight retry. If the rerun also fails with the same error, treat it as a real bug and fix-forward.

---

## § 14 Pitfalls (load-bearing rules that aren't obvious from the step list)

These have all bitten previous releases. They're the load-bearing why-not-this-instead rules.

- **MSRV bump → two files in lockstep.** Both `Cargo.toml` `rust-version` AND `rust-toolchain.toml` `channel` must change in the same commit. Additionally, `rust-toolchain.toml` must list `components = ["rustfmt", "clippy"]` — when rustup honors a `rust-toolchain.toml` it ignores any action-level `components:` field in `ci.yml`, so the list has to live in the toolchain file. v3.13.1 shipped as two commits (`c2e6a65` + `086ef0a`) specifically because the first attempt missed the components line.

- **Never `git push --tags`.** Always explicit `git push origin vX.Y.Z`. Broad tag pushes can drag stale local tags into the remote and trigger spurious release.yml runs.

- **Don't tag before** `ci.yml` is green AND `crates-publish.yml` has resolved (published or skipped — not failed). If you tag while either is still in-flight, you risk publishing a release whose crate hasn't yet made it to crates.io, which breaks the installer's `cargo install tr300` fallback.

- **The crate name is `tr300`** (lowercase, no hyphen) since v3.14.3. `cargo install tr300`, library import path `tr300`, installer URLs `tr300-installer.*`. The legacy `tr-300` crate name is GONE from crates.io (recreated under the corrected name in v3.14.3). Legacy `tr-300-installer.*` aliases stay in releases for v3.14.2 updater compatibility, but new install instructions point at `tr300`.

- **`cargo-dist` regenerates `release.yml`** via `dist init` (the binary is named `dist`, not `cargo dist`). After regeneration, preserve both the legacy installer alias copy step and the fail-closed Apple signing/notarization step. Losing either breaks an established compatibility or trust contract and requires another Mac gate.

- **A newly created signing keychain is not automatically searchable by `codesign`.** Preserve the v4.0.1 script sequence: capture the user search list, temporarily prepend the ephemeral keychain for the fingerprint-based signing call, restore the list immediately and from cleanup, then compare the embedded leaf-certificate fingerprint. Removing that sequence recreates v4.0.0's clean-runner failure even though `security find-identity` succeeds.

- **`Cargo.lock` is tracked.** Both local `cargo package --locked` and the CI publish workflow use `--locked`. Keep `Cargo.lock` in git — don't add it to `.gitignore` and don't delete it before a release.

- **`Cargo.toml` has `allow-dirty = ["ci", "msi"]`** in `[workspace.metadata.dist]`. `"ci"` permits the legacy-alias and Apple-trust workflow customizations; `"msi"` permits the customized WiX source. Preserve both.

- **`gh run watch` blocks until completion**, but it doesn't return useful info on failure. Pair it with `gh run view <run-id> --log-failed` to actually see the error. The `--exit-status` flag on `gh run watch` will make it exit non-zero on failure, which is useful for scripting.

---

## § 15 Source-of-truth pointers

When a step is ambiguous or this skill seems out of date, the canonical sources are:

- **[`AGENTS.md` § "Release checklist"](../../../AGENTS.md)** — the canonical ordered procedure with the full 10-file doc list.
- **[`AGENTS.md` § "Release Process"](../../../AGENTS.md)** + **§ "MSRV policy"** — same procedure plus the MSRV lockstep rule with rationale.
- **[`MASTER_PLAN.md` § "Status snapshot"](../../../MASTER_PLAN.md)** — historical ledger of what's shipped, the recommended-next-steps queue, and the v3.13.1 narrative that shows fix-forward by patch.

If those three disagree with this skill, **fix this skill** — those four files are the source of truth and this skill is a derivative that exists for convenience. The skill consolidates them so an agent can act on them without re-reading four documents; it doesn't replace them.
