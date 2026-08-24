---
name: release
description: Cut a TR-300 release end-to-end — from a protected main-branch commit through crates.io publication and the private-draft GitHub Release chain. Use whenever the user asks to ship, release, deploy, cut a release, tag a release, publish a version, push a version bump to crates.io, or watch a release CI run. Also use when fixing a CI or release workflow failure during an in-flight release. Encodes the full ordered workflow: pre-release local gates, version bump in lockstep across the documentation set, protected PR merge → exact-main CI → explicit OIDC crates publication → tag push → private 24-asset draft → Windows 30-asset assembly and acceptance → macOS 34-asset final publication → post-public smokes → fix-forward loop on failure. Trigger on phrases like "ship v3.14.4", "let's release", "cut the release", "deploy this", "tag and release", "release time", or just "deploy" / "ship it" in this repo's context, even when the user doesn't name a version number.
---

# TR-300 release workflow

Drive a TR-300 release from version-bump through published GitHub Release + crates.io, watching CI at each step and fixing forward when something fails. The canonical source-of-truth for these rules lives in [`AGENTS.md` § "Release checklist"](../../../AGENTS.md) and [`CLAUDE.md` § "Release Process"](../../../CLAUDE.md). This skill encodes the reconciled workflow plus the watch-and-fix-forward loop, so an agent can drive a release with only the user's go-ahead.

The workflow is twelve ordered steps. Skipping or reordering them tends to fail in non-obvious ways — the pitfalls in § 14 are real failures that shipped at some point.

---

## § 1 Pre-flight checks

Before starting, confirm:

- Working tree clean on a focused release branch based on current `main`.
- All feature work for the version you're about to cut is already committed.
- MSRV is unchanged — OR — both `Cargo.toml` `rust-version` and `rust-toolchain.toml` `channel` were bumped in the same earlier commit. This is the MSRV lockstep rule from [`CLAUDE.md` § "MSRV policy"](../../../CLAUDE.md). Touching one without the other will break either `ci.yml` (rustfmt/clippy missing) or `release.yml` (rustc older than required).
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
| 1b | `HUMAN_CHANGELOG.md` | Mirror the same release block in plain English — same `## [X.Y.Z] - YYYY-MM-DD` header and date, same `### Added` / `### Changed` / `### Fixed` / `### Internal` groupings. Strip CI run IDs, commit SHAs, error codes, function and API names, registry paths, file paths, GUIDs, line counts, memory deltas, task IDs, and dependency identifiers. Keep platform names, edition names (Global / Corporate), installer types, the command names and flags users actually type, and what the user-facing benefit is. One short paragraph per change. See `CLAUDE.md` § "HUMAN_CHANGELOG.md (companion changelog)" for the full strip/keep rules. **Never update `CHANGELOG.md` without also updating this file in the same commit.** |
| 2 | `README.md` | Update flag tables, install snippets, or sample output if anything user-visible changed. Skip if release is purely internal. |
| 3 | `TESTING.md` | Append a `### vX.Y.Z — YYYY-MM-DD` block to the "Per-release verification log" listing local gates that passed, runtime smoke results, and the CI/crates-publish/release.yml run IDs (the IDs come after pushing — fill them in during step 12). Match the format of the most recent entry. |
| 4 | `CODEX_PROJECT.md` | Only if release/install/update/deployment behavior changed. Otherwise skip. |
| 5 | `AGENTS.md` | Bump the "Last verified against source" date. Update any drifted fact (current version line, MSRV, dependency versions). |
| 6 | `CLAUDE.md` | Add architectural notes for any new pattern introduced this release. Cite source URLs inline (man pages, Apple docs, Microsoft Learn). Skip if no new pattern. |
| 7 | `MASTER_PLAN.md` | Bump "Last updated" and "Current version" lines. Append or update the "Tag status" bulleted entry **after** the release publishes (the run IDs and asset count come from the actual published runs in step 12). |
| 8 | `docs/architecture-decisions.md` | Only when rationale or release workflow itself changes. Skip otherwise. |
| 9 | `/Users/realemmetts/.codex/AGENTS.md` | Only when repo deployment workflow changes AND the path exists on the current host. This is the original macOS author's global Codex guide; Windows hosts silently skip. Check `Test-Path` (or `test -f`) before editing. |
| 10 | Auto-memory at `~/.claude/projects/<host-flavored-path>/memory/` | Host-specific and optional. `MASTER_PLAN.md` § 0 says: "Don't recreate it on other machines — `CLAUDE.md` is authoritative." Never block a release on this. |

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
shellcheck -x scripts/*.sh scripts/managed-installers/*.sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/test-release-workflow-provenance.py
scripts/test-managed-installer-transaction.sh
```

On Windows, also run `scripts/test-managed-installer-transaction.ps1` under
both `pwsh` and Windows PowerShell and parse
`scripts/install-pinned-inno-setup.ps1`. On Unix with a C compiler, run
`scripts/test-macos-pkg-rollback.sh`; on macOS it must enforce the native
metadata cases as CI does.

Then runtime smoke on the freshly-built binary:

```bash
./target/release/tr300 --version             # expect "tr300 X.Y.Z" — the version you just set
./target/release/tr300 --fast --json | python3 -m json.tool   # parses cleanly
./target/release/tr300 --ascii               # visual smoke; table renders
```

After any shared/macOS/release-workflow change, also run native Apple Silicon
and native Intel all-target tests, both release builds, full/fast/JSON/ASCII/
manual-save/no-write smokes, and privacy/parity checks. After Apple credential,
script, archive, or workflow-input changes, run a real cargo-dist archive
Developer ID signing → Apple `Accepted` → repack → sidecar/manifest checksum
test. Rosetta is supplemental only and never substitutes for native Intel. Do
not tag from Windows/Linux-only evidence after reopening the Mac gate.

Why both `cargo package --locked --list` and `cargo publish --dry-run --locked` — they catch different classes of issue. `package --list` shows what files end up in the crate (look for accidentally-included `.env` / `target/` / fixtures; the release commit is the cheapest time to catch them). `publish --dry-run` runs the full publish pipeline minus the upload — it catches the `Cargo.lock` resolver mismatches, version conflicts, and credential issues that bite at the publish step otherwise.

`Cargo.lock` is intentionally tracked — both `cargo package --locked` and the CI publish workflow use `--locked` to guarantee the same resolved dependency graph everywhere.

---

## § 6 Codex review and protected PR

For non-trivial diffs (cross-platform `unsafe` code, workflow YAML changes, anything you'd want a second pair of eyes on), invoke Codex review:

```
Agent tool with subagent_type: "codex:codex-rescue"
```

For release commits this starts with a self-review of the diff. Open a focused
feature/release PR and run the full review path for any non-trivial code,
workflow, dependency, installer, or platform change. Repository ruleset
`21268055` protects `main`: release work does not bypass the PR, resolved-thread,
or strict required-check contracts.

---

## § 7 Commit, push the release branch, and merge the protected PR

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

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Then push the focused branch, open the PR, and wait for every required exact-head
check and review thread:

```bash
git push -u origin <release-branch>
gh pr create --base main --head <release-branch> --title "release: vX.Y.Z" --body-file <body-file>
gh pr checks <pr-number> --watch
```

Merge only after the protected PR is eligible, then record the exact merge SHA on
`main`. Never push a release commit directly to `main`, use `--no-verify`, or
`--amend` after a hook failure (the commit did not happen, so amend would silently
modify the previous commit).

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
- `test` (Linux AMD64/ARM64, native macOS ARM/Intel, and Windows)
- `build` (release build smoke on those same five platform runners)
- `speed` (5-run median of `tr300 --fast` < 1500 ms on all three platforms)
- `audit` (cargo audit, blocking)
- `dist-plan` (cargo-dist config parses)
- `workflow-validation` (actionlint, ShellCheck, executable provenance, managed
  transaction, and macOS rollback/lifecycle guards)
- `windows-installer-sources` (managed PowerShell transaction plus both WiX and
  Inno source/transition gates)

That is the strict 19-context inventory recorded in the protected-main
ruleset. Do not treat a subset or a still-running matrix leg as a green gate.

If any job fails:

```bash
gh run view <run-id> --log-failed
```

Then fix through the protected PR (same version — the tag has not been pushed
yet), wait for its exact-head checks, merge, and watch the new exact `main` SHA.
See § 13 for the fix-forward loop.

---

## § 9 Explicitly publish the crate through trusted OIDC

`crates-publish.yml` is deliberately manual so merging a candidate cannot race
the irreversible release decision. After `ci.yml` succeeds on the exact current
`main` SHA, dispatch the owner-only `publish` operation:

```bash
gh workflow run crates-publish.yml --ref main -f operation=publish
gh run list --workflow=crates-publish.yml --branch main --limit 3
gh run watch <run-id>
```

The read-only validator, which has no registry credential, builds and dry-runs
the normalized package with exact Cargo 1.95, records the exact `.crate` hash,
and passes only data to a fresh
`crates-io` environment job. That job executes no package code while an OIDC
credential exists: it repackages with `--no-verify`, requires byte identity,
mints a short-lived token, publishes, and verifies the public checksum plus
crates.io trusted-publisher provenance for this repository, run, and SHA. The
crate must already enforce `trustpub_only=true`.

Two acceptable outcomes are an exact trusted publication or an idempotent
existing-version proof whose public bytes match the independently reproduced
historical tag package. Any mismatch or ambiguous registry result fails closed.

A **failed** run halts the release. Diagnose with `gh run view <run-id> --log-failed`. Common failures:
- exact `main`/CI/package custody changed before OIDC;
- the `crates-io` protected environment or trusted-publisher tuple is wrong;
- `trustpub_only` is not enabled or the public bytes/provenance do not match;
- Cargo or crates.io returned an indeterminate result — inspect the authoritative
  public metadata/hash before deciding whether a retry is safe.

One-time migration only: dispatch `operation=configure_trusted_publisher` from
protected `main`. It idempotently creates the exact crates.io configuration,
proves OIDC, and enables `trustpub_only` using the existing scoped legacy token.
Once that operation succeeds and the public policy is confirmed, the operator
must revoke that exact token in the authenticated crates.io UI, delete the
GitHub `CARGO_REGISTRY_TOKEN` secret, and merge a protected follow-up PR that
removes the bootstrap job and every legacy-token reference. Then dispatch
`operation=probe_trusted_publisher` to prove OIDC still works without the
legacy secret; the real version publication remains a later explicit release
gate. Never retain the token until that publication or restore automatic push
publication.

Apple cutover is separate. Create a short-lived fine-grained personal access
token restricted to `QubeTX/qube-machine-report`, with repository permissions
`Contents: read`, `Actions: read`, and `Environments: write` only. Install its
value as the `apple-signing` environment secret named exactly
`RELEASE_SECRET_MIGRATION_TOKEN` from an authenticated workstation with
`gh secret set RELEASE_SECRET_MIGRATION_TOKEN --env apple-signing --repo QubeTX/qube-machine-report`;
paste the value only at the command's stdin prompt so it does not enter shell
history. Run `apple-secret-migration.yml` to copy and inventory the exact names,
then require fresh native ARM and Intel credential preflights to prove both
Developer ID identities plus read-only notary authentication from
`apple-signing`. Delete the repository Apple secrets and
`RELEASE_SECRET_MIGRATION_TOKEN`, rerun environment-only preflight, and remove
the migration workflow in the protected follow-up. The copy workflow neither
deletes credentials nor supplies native proof itself.

Don't push the tag until this resolves.

---

## § 10 Tag and push the single tag

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

**Never** `git push --tags`. The explicit single-tag push is the documented trigger for `release.yml`; a broad `--tags` push can drag stale tags along and is one of the destructive operations called out in [`CLAUDE.md`](../../../CLAUDE.md).

The tag name has a `v` prefix (`v3.14.4`), but the cargo version is bare
(`3.14.4`). TR-300 supports only exact stable `vX.Y.Z` tags: no prerelease
suffix, build metadata, package prefix, or alternate tag shape. The generated
`**[0-9]+.[0-9]+.[0-9]+*` trigger remains deliberately broad so the workflow's
first plan step visibly rejects an unsupported push tag before checkout or any
dist/release work instead of silently creating only part of the 34-asset chain.

---

## § 11 Watch `release.yml`

```bash
gh run list --workflow=release.yml --limit 3
gh run watch <run-id>
```

`release.yml` is generated by cargo-dist v0.31.0 and checked in with the
documented stable-tag, artifact-custody, compatibility-alias, private-host, and
fresh Apple signing/notarization customizations. Success looks like 13 jobs:

- `plan` (1 job)
- `build-local-artifacts` (6 jobs — one per target: `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`)
- `sign-apple-artifacts` (2 fresh checkout-free jobs)
- `build-global-artifacts` (1 job)
- `prepare-host-assets` (1 fixed-inventory job with read-only GitHub
  authorization and no Release write credential)
- `host` (1 job)
- `announce` (1 job)

For both fresh Apple signing jobs, verify that Apple returned `Accepted` and the
canonical signed artifact was uploaded with its fixed manifest and sidecar.
Missing credentials, signing failure, or any non-`Accepted` result must fail the
job; there is deliberately no unsigned fallback.

`release.yml` does not immediately expose a stable release. Its fresh,
checkout-free publisher creates a private draft with 24 exact assets after the
two Apple archives are signed/notarized in fresh `apple-signing` jobs.

Watch the matching `windows-installers.yml` run. Its read-only builder creates
the Corporate MSI plus Global and Corporate Inno Setup EXEs; its fresh
`release-publishing` job attaches those three installers and their sidecars to
the private draft, bringing it to 30 assets.

Next require the **private** `windows-installer-validation.yml` run to pass its
complete exact-byte fresh-install, authenticated direct prior-to-candidate
transition, uninstall, scope, format-transition, rollback, and cleanup matrix
while public `latest` remains the prior release. The immutable proof binds the
30 draft assets, Windows build/run attempt, internal artifact ID/digest, and
validation results. This is not the real updater-to-candidate gate; that runs
only after publication.

Only then may `macos-installer.yml` use that exact proof. Native Intel and Apple
Silicon jobs must pass signature, notarization, staple, Gatekeeper, install,
update, takeover, and uninstall gates. Its sole fresh `release-publishing`
finalizer rebinds the draft and proof, adds the universal direct PKG and v4.1.x
compatibility DMG pairs, verifies the exact 34-asset inventory and every asset
byte, and changes the draft to a public stable release. A failed supplement
remains private.

Finally, require the chained **public** Windows updater smoke and the published
Linux/macOS smoke gates. Public discovery and homepage/final closure occur only
after this post-public evidence.

The complete published GitHub Release has **34 assets for v4.2.0+**:
- 6 platform archive pairs (archive plus SHA-256 sidecar): 12
- source archive plus sidecar: 2
- Global MSI plus sidecar: 2
- canonical, legacy-alias, and internal raw shell/PowerShell installers: 6
- `dist-manifest.json` plus `sha256.sum`: 2
- Corporate MSI and both Inno Setup EXEs plus their sidecars: 6
- universal direct PKG and compatibility DMG plus their sidecars: 4

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
- private windows-installer-validation.yml run ID and exact proof artifact
- macos-installer.yml run ID, including native Intel/Apple Silicon lifecycle results
- post-public windows-installer-validation.yml updater-smoke run ID
- both Apple `Accepted` results and public macOS signature/checksum evidence
- GitHub Release asset count (should be 34 for v4.2.0+)

Then update two files:

**`TESTING.md`** — append a new `### vX.Y.Z — YYYY-MM-DD` block to the "Per-release verification log" section. Match the format of the most recent entry. Include:
- One-paragraph summary of what shipped
- Local gates pass note (the cargo commands from § 5)
- Runtime smoke results
- "**CI verification** — `main` CI run <id> passed..."
- "**Crates.io verification** — crates-publish run <id> published `tr300` X.Y.Z..."
- "**Release verification** — release.yml run <id> created the private 24-asset draft, windows-installers.yml run <id> assembled 30 assets, private windows-installer-validation.yml run <id> attested the exact draft bytes, macos-installer.yml run <id> passed both native Apple gates and published the exact 34-asset release, and public windows-installer-validation.yml run <id> passed updater smokes"

**`MASTER_PLAN.md`** — locate the "Tag status (as of YYYY-MM-DD)" bulleted list and append a new entry for `vX.Y.Z`:
```
- `vX.Y.Z` (`<commit-sha>`): tagged + pushed after exact-main CI and explicit trusted-OIDC crates publication; release.yml run <id> created the private 24-asset draft after fresh Apple signing/notary gates; windows-installers.yml run <id> assembled the exact 30-asset draft; private windows-installer-validation.yml run <id> attested those bytes and passed direct prior-to-candidate transitions while public latest remained prior; macos-installer.yml run <id> added the four direct-PKG/compatibility-DMG assets and solely published the exact 34-asset release; public windows-installer-validation.yml run <id> and published Linux/macOS smokes passed.
```

Also bump the "Last updated" and "Current version" lines at the top of `MASTER_PLAN.md`.

Then commit and push these doc updates as a follow-up:

```bash
git add CHANGELOG.md TESTING.md MASTER_PLAN.md   # whichever you touched
git commit -m "docs: record vX.Y.Z publication status"
git push -u origin <docs-branch>
# open/merge a protected docs PR
```

The follow-up PR must still pass the protected `ci.yml` contexts. It does not
automatically dispatch crates publication.

The release is now complete.

---

## § 13 Fix-forward loop

Things fail. The recovery path depends on whether the tag has been pushed yet.

### Pre-tag (between § 7 push and § 10 tag)

If `ci.yml` or the explicit `crates-publish.yml` run fails:

1. `gh run view <run-id> --log-failed` to diagnose.
2. Fix the issue on the release branch with a new commit. **Keep the same
   version** — the tag has not moved yet, so `Cargo.toml` `version` stays as set
   in § 3.
3. Push the branch and wait for the protected PR's exact-head checks.
4. Merge, watch exact-main CI again from § 8, and explicitly redispatch crates
   publication only when that SHA is ready.

Repeat until exact-main `ci.yml` and the explicit crates publication are proven,
then proceed to § 10.

### Post-tag (after § 10 push)

Tags are immutable. If `release.yml`, Windows assembly, private Windows
validation, or macOS finalization fails non-transiently after the tag is
pushed:

1. `gh run view <run-id> --log-failed` to diagnose.
2. Fix the issue and **bump the version as a patch** (`vX.Y.Z → vX.Y.(Z+1)`).
3. Go back to § 3 and run the workflow again for the patch version.

If a post-public updater or Linux/macOS smoke fails, treat the visible release
as an incident and fix forward with a patch. Never delete or move the tag,
replace published assets, or manually toggle draft visibility to work around a
failed chain stage.

The canonical example is v3.13.0 → v3.13.1: v3.13.0's `release.yml` failed because three runner images shipped rustc 1.94.1 (below MSRV 1.95). The fix was adding `rust-toolchain.toml`. Instead of deleting and re-pushing the v3.13.0 tag, v3.13.1 was cut as a fresh patch release. The v3.13.0 tag remains in git as a historic record of the failure.

**Never** delete and re-push a tag. That's a destructive shared-state operation; it confuses anyone who has already fetched the tag and breaks Cargo's expectation that semver versions are immutable.

### When CI is flaky vs broken

If a run fails for what looks like a transient reason (network blip, GitHub Actions queue, crates.io API hiccup), `gh run rerun <run-id>` is the lightweight retry. If the rerun also fails with the same error, treat it as a real bug and fix-forward.

---

## § 14 Pitfalls (load-bearing rules that aren't obvious from the step list)

These have all bitten previous releases. They're the load-bearing why-not-this-instead rules.

- **MSRV bump → two files in lockstep.** Both `Cargo.toml` `rust-version` AND `rust-toolchain.toml` `channel` must change in the same commit. Additionally, `rust-toolchain.toml` must list `components = ["rustfmt", "clippy"]` — when rustup honors a `rust-toolchain.toml` it ignores any action-level `components:` field in `ci.yml`, so the list has to live in the toolchain file. v3.13.1 shipped as two commits (`c2e6a65` + `086ef0a`) specifically because the first attempt missed the components line.

- **Never `git push --tags`.** Always explicit `git push origin vX.Y.Z`. Broad tag pushes can drag stale local tags into the remote and trigger spurious release.yml runs.

- **Don't tag before** exact-main `ci.yml` is green AND the explicit trusted-
  OIDC crates publication has been proven. If you tag while either is still in
  flight, you risk a GitHub candidate whose Cargo fallback is unavailable.

- **Do not publish a partial stable release.** The GitHub chain is a private
  24-asset draft, a 30-asset Windows draft, exact pre-public Windows acceptance,
  and a single macOS finalizer that publishes only after the exact 34 assets and
  all their bytes match.
  Failed supplements remain private; do not manually flip the draft flag.

- **The crate name is `tr300`** (lowercase, no hyphen) since v3.14.3. `cargo install tr300`, library import path `tr300`, installer URLs `tr300-installer.*`. The legacy `tr-300` crate name is GONE from crates.io (recreated under the corrected name in v3.14.3). Legacy `tr-300-installer.*` aliases stay in releases for v3.14.2 updater compatibility, but new install instructions point at `tr300`.

- **`cargo-dist` regenerates `release.yml`** via `dist init` (the binary is named
  `dist`, not `cargo dist`). After regeneration, preserve the first-step stable-
  tag guard, workflow-level read-only permission plus tag-only host write
  permission, legacy installer alias copy step, and fail-closed Apple signing/
  notarization step. Losing any one breaks the complete publication or trust
  contract and requires release-workflow revalidation (plus the Mac gate when
  Apple behavior changes).

- **A newly created signing keychain is not automatically searchable by `codesign`.** Preserve the v4.0.1 script sequence: capture the user search list, temporarily prepend the ephemeral keychain for the fingerprint-based signing call, restore the list immediately and from cleanup, then compare the embedded leaf-certificate fingerprint. Removing that sequence recreates v4.0.0's clean-runner failure even though `security find-identity` succeeds.

- **`Cargo.lock` is tracked.** Both local `cargo package --locked` and the CI publish workflow use `--locked`. Keep `Cargo.lock` in git — don't add it to `.gitignore` and don't delete it before a release.

- **`Cargo.toml` has `allow-dirty = ["ci", "msi"]`** in `[workspace.metadata.dist]`. `"ci"` permits the legacy-alias and Apple-trust workflow customizations; `"msi"` permits the customized WiX source. Preserve both.

- **`gh run watch` blocks until completion**, but it doesn't return useful info on failure. Pair it with `gh run view <run-id> --log-failed` to actually see the error. The `--exit-status` flag on `gh run watch` will make it exit non-zero on failure, which is useful for scripting.

---

## § 15 Source-of-truth pointers

When a step is ambiguous or this skill seems out of date, the canonical sources are:

- **[`AGENTS.md` § "Release checklist"](../../../AGENTS.md)** — the canonical ordered procedure with the full 10-file doc list.
- **[`CLAUDE.md` § "Release Process"](../../../CLAUDE.md)** + **§ "MSRV policy"** — same procedure plus the MSRV lockstep rule with rationale.
- **[`MASTER_PLAN.md` § "Status snapshot"](../../../MASTER_PLAN.md)** — historical ledger of what's shipped, the recommended-next-steps queue, and the v3.13.1 narrative that shows fix-forward by patch.

If those three disagree with this skill, **fix this skill** — those four files are the source of truth and this skill is a derivative that exists for convenience. The skill consolidates them so an agent can act on them without re-reading four documents; it doesn't replace them.
