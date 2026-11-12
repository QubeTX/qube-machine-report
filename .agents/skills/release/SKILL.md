---
name: release
description: "Cut a TR-300 release end-to-end — from a reviewed main-branch commit through crates.io publication and the private-draft GitHub Release chain. Use whenever the user asks to ship, release, deploy, cut a release, tag a release, publish a version, push a version bump to crates.io, or watch a release CI run. Also use when fixing a CI or release workflow failure during an in-flight release. Classifies record-only maintenance separately from a new published version; preserves the full release workflow: focused local gates, reviewed PR merge → exact-main CI → automatic OIDC crates publication → tag push → private 24-asset draft → Windows 30-asset assembly and acceptance → macOS 34-asset final publication → post-public smokes → fix-forward loop on failure. Trigger on phrases like \"ship v3.14.4\", \"let's release\", \"cut the release\", \"deploy this\", \"tag and release\", \"release time\", or just \"deploy\" / \"ship it\" in this repo's context, even when the user doesn't name a version number."
---

# TR-300 release workflow

Use the light record-only path for publication records and the full path when publishing new package or installer bytes. Diagnose failures before deciding whether any product change or new version is needed. The canonical source-of-truth for these rules lives in [`AGENTS.md` § "Release checklist"](../../../AGENTS.md) and [`AGENTS.md` § "Release Process"](../../../AGENTS.md). This skill encodes the reconciled workflow plus the watch-and-fix-forward loop, so an agent can drive a release with only the user's go-ahead.

The package/tag/private/native/public ordering below applies to a new release.
Repository documentation and infrastructure maintenance do not automatically
create a release. The canonical classifier is [`scripts/ci_policy.py`](../../../scripts/ci_policy.py):
its positive record-only allowlist is the root `AGENTS.md`, `CLAUDE.md`,
`CODEX_PROJECT.md`, `MASTER_PLAN.md`, `TESTING.md`, plus Markdown beneath
`docs/` and `.tasks/`. Any other path selects full CI, including README,
license/man/changelog files, `.agents`/`.claude` skills, workflows, scripts,
package inputs, and unknown paths. Missing or ambiguous diff evidence selects
full CI. Check both old and new paths of a rename; executable records,
symlinks/submodules, or unsupported file modes also select full.

`CI gate` validates the classifier and heavy-job results. Only after it
succeeds does exactly one mode marker run: `Full CI qualification` or
`Documentation CI qualification`; the other is skipped. Only the full marker can authorize a new crate, tag,
release asset, or native publication. Do not interpret a green documentation
run as release qualification.

---

## § 1 Pre-flight checks

Before starting, confirm:

- Working tree clean on a focused release branch based on current `main`.
- All feature work for the version you're about to cut is already committed.
- MSRV is unchanged — OR — both `Cargo.toml` `rust-version` and `rust-toolchain.toml` `channel` were bumped in the same earlier commit. This is the MSRV lockstep rule from [`AGENTS.md` § "MSRV policy"](../../../AGENTS.md). Touching one without the other will break either `ci.yml` (rustfmt/clippy missing) or `release.yml` (rustc older than required).
- `gh` CLI is authenticated for this repo (`gh auth status`).

You'll never use `--no-verify`, `--no-gpg-sign`, or `git push --tags` anywhere in this workflow. If a pre-commit hook fails, fix the underlying issue and create a new commit — never amend after a hook failure (the commit didn't actually happen, so amend would touch the previous commit).

---

## § 2 Decide whether a published version changes

Keep the version for record-only documentation and repository infrastructure
maintenance unless a new published package or distribution artifact is needed.
Workflow, skill, CI, or evidence updates are not themselves a product release.
A full CI classification means broad verification, not an automatic version bump.

For an authorized new release, choose the bump from the resulting public change:

| Bump | When |
|---|---|
| Major | Public Rust source compatibility breaks |
| Minor | New public fields, flags, behavior, or API surface |
| Patch | A fix that must reach new published package or installer bytes |

Follow an explicit user version request. Otherwise use the smallest correct
bump and keep pre-tag corrections in that candidate. Immutable published
versions, tags, and assets are never moved, replaced, or reused.

## § 3 Update package metadata only for that new version

When publishing a new version, update `Cargo.toml`, its `Cargo.lock` package
entry, generated man-page versions, and affected release documentation in
lockstep. For unchanged-version repository maintenance, leave those version
fields unchanged and continue to the applicable CI path.

## § 4 Update the documentation set

Update the affected reader-facing documents once. A new product release updates both changelogs and its verification record; record-only or infrastructure maintenance does not require a new release-note/version section or edits to every guide. The list below is a routing guide, not a mandatory per-PR ceremony.

| # | File | What goes in |
|---|---|---|
| 1 | `CHANGELOG.md` | Prepend a `## [X.Y.Z] - YYYY-MM-DD` block at the top of the file (above `[Unreleased]` if present, or replacing the `Unreleased` heading). Keep-a-Changelog voice: grouped under `### Added` / `### Changed` / `### Fixed` / `### Internal`. Reference task IDs in parens for traceability. Match the voice of the most recent entry — read it first. |
| 1b | `HUMAN_CHANGELOG.md` | Mirror the same release/date/groupings in plain English. Strip run IDs, SHAs, APIs, paths, GUIDs, task IDs, and dependency identifiers; keep user commands, platforms/installers, behavior, and benefit. Never update only one changelog. |
| 2 | `README.md` | Update flag tables, install snippets, or sample output if anything user-visible changed. Skip if release is purely internal. |
| 3 | `TESTING.md` | Append a `### vX.Y.Z — YYYY-MM-DD` block to the "Per-release verification log" listing local gates that passed, runtime smoke results, and the CI/crates-publish/release.yml run IDs (the IDs come after pushing — fill them in during step 12). Match the format of the most recent entry. |
| 4 | `CODEX_PROJECT.md` | Only if release/install/update/deployment behavior changed. Otherwise skip. |
| 5 | `AGENTS.md` | Update current facts or operating guidance when they changed. |
| 6 | `CLAUDE.md` | Add architectural notes for any new pattern introduced this release. Cite source URLs inline (man pages, Apple docs, Microsoft Learn). Skip if no new pattern. |
| 7 | `MASTER_PLAN.md` | Bump "Last updated" and "Current version" lines. Append or update the "Tag status" bulleted entry **after** the release publishes (the run IDs and asset count come from the actual published runs in step 12). |
| 8 | `docs/architecture-decisions.md` | Only when rationale or release workflow itself changes. Skip otherwise. |
| 9 | Host-specific guides or memory | Optional only within the user-authorized scope; never a release prerequisite. |

Keep the detailed proof in `TESTING.md` and one handoff; other guides should summarize current state and link to that proof rather than requiring further PRs to restate each other.

---

## § 5 Run the local release gates

Choose local checks from the changed behavior. Record-only Markdown needs
content/link/encoding review, `git diff --check`, and classifier tests when its
policy changed; it does not need a Rust build or installer run. Code, workflow,
package, or release changes need their relevant focused tests and independent
review. A new product release requires the full applicable checks below and
hosted native proof. Reuse completed checks for an unchanged exact input;
repeat failed or affected checks and their dependents after a correction,
not unrelated successful work merely to restart a checklist.

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

Before tagging, generate the actual pinned cargo-dist JSON plan and run `python3 scripts/check-dist-runners.py <plan.json>`. Its Apple jobs must use singleton target sets, matching native host triples, `macos-15` / `macos-15-intel`, and no container. The parse-only v4.4.1 gate missed cargo-dist's retired `macos-14` ARM default; preserve that tag and use the v4.4.2 fix-forward. Both CI and the tag Release plan enforce this same validator; never regenerate away the trust-boundary customizations.

On Windows, also run `scripts/test-managed-installer-transaction.ps1` under
both `pwsh` and Windows PowerShell and parse
`scripts/install-pinned-inno-setup.ps1`. Extract the single `PREINSTALL`
template from `scripts/build-sign-notarize-macos-installer.sh` and run
ShellCheck on that exact script. The native macOS CI matrix is the runtime
authority for clean pass, ordinary and dot-prefixed `/Users` residue plus
environment-independent local Directory Service custom-home standard-path
managed-evidence refusal through a real PKG, negative system-UID handling,
broken links, unlistable/abnormal fixed-path parents, and unchanged evidence. The
preinstall must validate `dscl -plist` output with `plutil -lint`, extract the
escaped attribute's sole array value with `/usr/libexec/PlistBuddy`, and never
use typed `plutil` extraction flags that require macOS 12. Require one absolute
existing home for UID 501–4294967295 accounts, exclude UID 0/negative sentinels,
and defensively scan safely resolved positive lower-UID homes. Reject every
target volume except `/`, before payload or receipt placement. Before accepting
a fixed standard path as absent, require `/Users` itself plus non-recursive enumeration of its home
and present parent levels; symlinked, non-directory, or unlistable intermediates
must fail closed, and fixed ASCII components must have at most one
case-insensitive match. Its refusal
diagnostic must first refresh a runnable managed copy through the managed
installer before receipt-aware Complete uninstall, so an older client cannot
leave a stale receipt. A missing, offline, or otherwise uninspectable mandatory
home deliberately blocks the PKG; recovery is to make it inspectable, repair the
Directory Service home record, or keep using the managed installer, never to
override the gate. The release workflow must also inspect the built PKG's exact
payload/script inventory.

Both public managed wrappers must be rendered from the exact prepared release
artifact with the SHA-256 of their corresponding raw `tr300-dist-installer.*`
asset. The wrapper verifies those downloaded bytes before execution. On macOS,
the shell wrapper must additionally prepend a private `/usr/bin/shasum -a 256`-
backed `sha256sum` shim only for the cargo-dist child so archive verification is
mandatory. The public v4.2.2 lifecycle baseline must receive the same private
shim because that historical cargo-dist script otherwise treats a missing
`sha256sum` as permission to skip archive verification. Native managed-transition
tests bind wrapper, raw script, and target archive IDs/sizes/digests before
execution and require identical records after.
For tagged macOS `workflow_run` or recovery dispatch, require the downstream
workflow-code SHA, protected remote `main`, and resolved tag source SHA to be
identical before any `apple-signing` credential job can run. Tagless credential
preflight remains pinned to current protected `main`.

Native Apple workflow shell compatibility is also a pre-tag contract. GitHub's
macOS runners resolve the system shell to `/bin/bash` 3.2, so affected native
jobs in `release.yml` and `macos-installer.yml` must not use Bash 4-only
`declare -A`, `mapfile`, or `readarray`. Fixed Apple artifact inventories must
fail closed through Python `os.scandir` plus
`entry.stat(follow_symlinks=False)`: require a real directory, the exact
expected name set and count, and nonempty regular files; reject hidden extras,
links, directories, devices, and empty inputs.

Before a tag, both existing `macos-15` and `macos-15-intel` `test` matrix legs
must run
`TR300_TEST_BASH=/bin/bash python3 scripts/test-release-workflow-provenance.py --apple-staging-compatibility`.
That mode must prove the selected shell is Bash 3.2, syntax-check every inline
run block in the affected native Apple jobs plus every classified Bash block in
the tag-only cargo-dist build matrix, execute the exact named release-staging
block for both Apple targets, and execute all three exact macOS-installer
inventory snippets against clean, missing, extra, empty, directory, and symlink
fixtures. It must retain fail-closed negatives for bad sidecars and linked
archive members. These remain steps inside the two native Mac legs of the full five-platform
CI matrix. Consolidation shares each platform's tests, release build, smokes,
and applicable speed check on one runner; it does not remove native proof.

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

## § 6 Focused independent review and protected PR

Self-review the diff and seek a focused independent review for security,
unsafe/platform code, package, or workflow changes. Give the reviewer the
changed boundary, expected behavior, and relevant evidence. One useful review
is enough unless new findings or changes justify another pass; no particular
agent product, model, tool name, plan-mode UI, or multi-PR sequence is required.
Record-only updates need proportionate content and evidence review.

Merge only a focused, reviewed exact commit whose required checks are green
and whose actionable review threads are resolved. Keep normal hooks and
signing enabled.

## § 7 Commit, push the release branch, and merge the protected PR

Commit:

```bash
git add <specific files>   # avoid `git add -A` — sensitive files have slipped in this way
git commit -m "release: vX.Y.Z - <one-line summary>"
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

For a new release, require `CI gate` and `Full CI qualification` successful
from the exact same run and current attempt; `Documentation CI qualification`
must be skipped. The full path retains fmt, clippy, fresh advisory audit,
validated actual cargo-dist plan, executable workflow/provenance fixtures,
Windows bootstrap and both installer engines, and five platform legs:

- Linux x86_64 glibc (`ubuntu-latest`)
- Linux ARM64 glibc (`ubuntu-22.04-arm`)
- macOS ARM (`macos-15`)
- macOS Intel (`macos-15-intel`)
- Windows x86_64 (`windows-latest`)

Each leg runs locked all-target tests, its release build, and runtime smokes
on the same runner. Linux x86_64, macOS ARM, and Windows retain one warmup plus
five measured fast runs with the 1500 ms median limit, before expensive native
fixtures. Both Mac legs retain system-Bash-3.2 compatibility and native PKG
fixtures. The old 13 separate test/build/speed allocations become five
platform legs, removing eight duplicate allocations. The historical 20-job
CI count is not itself an acceptance rule.
The designed allocation is 12 heavy jobs plus classifier, stable gate, and full
marker (15 total); record-only mode allocates classifier, stable gate, and docs
marker (three). These are workflow structure counts, not measured speed claims.

A record-only follow-up instead needs `CI gate` and only
`Documentation CI qualification`. The Release workflow also ignores those
exact allowlisted PR paths, so a pure record PR does not schedule its plan job;
main/tag qualification and real release inputs retain their full checks. It does not execute the expensive Rust,
packaging, native lifecycle, or audit-tool jobs and cannot authorize a release.

If any job fails:

```bash
gh run view <run-id> --log-failed
```

Then fix through the protected PR (same version — the tag has not been pushed
yet), wait for its exact-head checks, merge, and watch the new exact `main` SHA.
See § 13 for the fix-forward loop.

---

## § 9 Observe automatic crate publication through trusted OIDC

`crates-publish.yml` starts automatically on a same-repository push to `main`.
A read-only preflight waits for that exact SHA's current `CI` attempt and
checks the stable gate plus exclusive qualification marker. Documentation mode
can only prove the existing unyanked version using public checksum/download,
VCS/trusted-publisher metadata, immutable tag ancestry, and unchanged packaged
Git inputs. It performs no Cargo compilation, packaging, dry-run, OIDC, or
registry publication. Full maintenance with those same unchanged inputs may
use that existing-version path too. Otherwise only full CI may enter the
existing exact-package validator and fresh publisher. There is no normal manual
publication step:

```bash
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

Two acceptable outcomes are an exact trusted new publication after full CI,
or a read-only/idempotent existing-version proof against the immutable
historical tag and public bytes. Documentation qualification can reach only
the latter, never the fresh publisher. Any mismatch or ambiguous registry result fails closed.

A **failed** run halts the release. Diagnose with `gh run view <run-id> --log-failed`. Common failures:
- exact `main`/CI/package custody changed before OIDC;
- the `crates-io` protected environment or trusted-publisher tuple is wrong;
- `trustpub_only` is not enabled or the public bytes/provenance do not match;
- Cargo or crates.io returned an indeterminate result — inspect the authoritative
  public metadata/hash before deciding whether a retry is safe.

The trusted-publisher tuple and `trustpub_only=true` are steady-state
configuration. `operation=probe_trusted_publisher` is diagnostics only; it
must never become a required release click or substitute for the automatic
run. Apple signing likewise consumes the existing `apple-signing` environment;
its branch policy has no reviewer or wait-timer ceremony. A tagless native
ARM/Intel credential preflight is optional diagnostics before a risky signing
change, not a normal release step.

Don't push the tag until this resolves.

---

## § 10 Tag and push the single tag

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

**Never** `git push --tags`. The explicit single-tag push is the documented trigger for `release.yml`; a broad `--tags` push can drag stale tags along and is one of the destructive operations called out in [`AGENTS.md`](../../../AGENTS.md).

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
managed-conflict refusal before payload/receipt creation, receipt-aware
managed-to-PKG preparation beginning from the checksum-bound public v4.2.2
managed wrapper, published-native-to-candidate upgrade, same-version repair,
PKG-to-managed takeover, update, and uninstall gates. Its sole fresh `release-publishing`
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

## § 12 Record evidence and close once

After public gates pass, record the release source/tag, exact PR/main CI and
crate results, Release/Windows/private/native/public run attempts, immutable
proof IDs/digests, 34-asset byte audit, and relevant runtime smokes in
`TESTING.md` plus one handoff. Summarize current status in `MASTER_PLAN.md`
and update only affected guides. Never mark an unrun gate complete.

Use one protected evidence/closure update. Its allowlisted record-only paths
use documentation CI and the read-only existing-version proof; no new version,
installer campaign, or repeated product release is needed. Record that
follow-up's actual checks through its PR/check links and the task receipt.
Do not open another PR merely to record the previous record PR's own CI, then
repeat that cycle. Task/milestone completion still waits for the required
checks and real evidence, including any specifically pending process-change
qualification. Ordinary independent review, branch protection, and exact-input
checks remain in force.

## § 13 Fix-forward loop

Things fail. The recovery path depends on whether the tag has been pushed yet.

### Pre-tag (between § 7 push and § 10 tag)

If `ci.yml` or the automatic `crates-publish.yml` run fails:

1. `gh run view <run-id> --log-failed` to diagnose.
2. Fix the issue on the release branch with a new commit. **Keep the same
   version** — the tag has not moved yet, so `Cargo.toml` `version` stays as set
   in § 3.
3. Push the branch and wait for the protected PR's exact-head checks.
4. Merge and watch the push-triggered CI and crates runs again from § 8. The
   crates run must remain without a registry credential until its exact-SHA CI poll succeeds.

Repeat until exact-main `ci.yml` and the automatic crates publication are proven,
then proceed to § 10.

### Post-tag (after § 10 push)

Tags are immutable. If `release.yml`, Windows assembly, private Windows
validation, or macOS finalization fails non-transiently after the tag is
pushed:

1. `gh run view <run-id> --log-failed` to diagnose.
2. Establish whether a product/package/artifact correction is required. If
   published bytes need changing, fix forward with a new patch after the full
   candidate gates. Keep the existing tag/assets immutable.
3. Repository-only infrastructure or evidence maintenance does not
   automatically bump the product version. Qualify that change through full
   CI where required and preserve all exact-source/custody constraints; do not
   run unqualified newer workflow code against an older tag.

If a post-public smoke exposes a reproducible product defect, treat it as an
incident and publish the validated correction as a new version. Diagnose a
service or capacity failure before deciding that the product is defective. Never delete or move the tag,
replace published assets, or manually toggle draft visibility to work around a
failed chain stage.

The canonical example is v3.13.0 → v3.13.1: v3.13.0's `release.yml` failed because three runner images shipped rustc 1.94.1 (below MSRV 1.95). The fix was adding `rust-toolchain.toml`. Instead of deleting and re-pushing the v3.13.0 tag, v3.13.1 was cut as a fresh patch release. The v3.13.0 tag remains in git as a historic record of the failure.

**Never** delete and re-push a tag. That's a destructive shared-state operation; it confuses anyone who has already fetched the tag and breaks Cargo's expectation that semver versions are immutable.

### When CI is flaky vs broken

Read the failed job's logs and annotations first. Distinguish a runner that
never received an assignment, a service/network failure with unchanged bytes,
and a reproducible product or workflow defect. Preserve successful native
proof and retry only the failed job and required dependents using the same
immutable source/artifacts (for example `gh run rerun <run-id> --failed`).
Repeated capacity or Apple ticket-service errors are not proof that a new
product version fixes anything. Capture targeted diagnostics before changing
code; never skip signature, notarization, native lifecycle, or public gates.

---

## § 14 Pitfalls (load-bearing rules that aren't obvious from the step list)

These have all bitten previous releases. They're the load-bearing why-not-this-instead rules.

- **MSRV bump → two files in lockstep.** Both `Cargo.toml` `rust-version` AND `rust-toolchain.toml` `channel` must change in the same commit. Additionally, `rust-toolchain.toml` must list `components = ["rustfmt", "clippy"]` — when rustup honors a `rust-toolchain.toml` it ignores any action-level `components:` field in `ci.yml`, so the list has to live in the toolchain file. v3.13.1 shipped as two commits (`c2e6a65` + `086ef0a`) specifically because the first attempt missed the components line.

- **Never `git push --tags`.** Always explicit `git push origin vX.Y.Z`. Broad tag pushes can drag stale local tags into the remote and trigger spurious release.yml runs.

- **Don't tag before** exact-main `ci.yml` is green AND the automatic trusted-
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

- **macOS Actions runners use system Bash 3.2.** Do not introduce `declare -A`,
  `mapfile`, or `readarray` into the fresh Apple signer or native installer jobs;
  Bash syntax lint on a newer developer shell does not prove those commands can
  execute. v4.3.1's immutable tag reached all six platform builds and then both
  fresh Apple signers failed before signing for exactly this reason. Preserve
  both native Mac compatibility-fixture legs and the fail-closed Python
  `os.scandir` inventories when regenerating or editing release workflows.

- **`Cargo.lock` is tracked.** Both local `cargo package --locked` and the CI publish workflow use `--locked`. Keep `Cargo.lock` in git — don't add it to `.gitignore` and don't delete it before a release.

- **`Cargo.toml` has `allow-dirty = ["ci", "msi"]`** in `[workspace.metadata.dist]`. `"ci"` permits the legacy-alias and Apple-trust workflow customizations; `"msi"` permits the customized WiX source. Preserve both.

- **`gh run watch` blocks until completion**, but it doesn't return useful info on failure. Pair it with `gh run view <run-id> --log-failed` to actually see the error. The `--exit-status` flag on `gh run watch` will make it exit non-zero on failure, which is useful for scripting.

---

## § 15 Source-of-truth pointers

When a step is ambiguous or this skill seems out of date, the canonical sources are:

- **[`AGENTS.md` § "Release checklist"](../../../AGENTS.md)** — the canonical release ordering and proportionate documentation routing.
- **[`CLAUDE.md` § "Release Process"](../../../CLAUDE.md)** + **§ "MSRV policy"** — same procedure plus the MSRV lockstep rule with rationale.
- **[`MASTER_PLAN.md` § "Status snapshot"](../../../MASTER_PLAN.md)** — historical ledger of what's shipped, the recommended-next-steps queue, and the v3.13.1 narrative that shows fix-forward by patch.

If those three disagree with this skill, **fix this skill** — those four files are the source of truth and this skill is a derivative that exists for convenience. The skill consolidates them so an agent can act on them without re-reading four documents; it doesn't replace them.
