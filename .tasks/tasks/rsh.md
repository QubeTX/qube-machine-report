TT;DR: Commit and push the completed v4.1.0 release candidate, wait for exact-SHA CI/crates, then push only the immutable v4.1.0 tag and watch all packaging workflows.

## Why

Direct operator order to implement and release the approved plan. The repository release workflow requires main CI and crates publication from the same commit before tagging; Apple/archive or installer failures must fix forward rather than mutate a tag.

## Plan

#c8r is complete. Rerun all local gates from a clean committed tree, commit the exact intended files, push main, verify exact-SHA CI and crates workflow, then create/push only `v4.1.0`. Watch cargo-dist, supplemental Windows installers, native Mac PKG-in-DMG, and disposable Windows installer validation. Diagnose and fix forward if any gate fails.

## Impact

Publishes a new crate version and immutable release assets. Premature tagging or skipped gates could strand a broken public version, so order is load-bearing.

## Acceptance

The exact release commit passes CI/crates; the immutable tag points at that SHA; all release workflows succeed without unsigned or cross-channel fallbacks.

## Verification

- [x] Clean-tree local release gates pass without `--allow-dirty`
- [ ] Exact release SHA passes CI and crates publication
- [ ] Tag `v4.1.0` points at that exact SHA and every release workflow succeeds

## Status

Active. #c8r's hosted identity proof passed in run 29637224793 on both native architectures. Receipt/ADR review, the candidate commit, and clean-tree package/publish dry runs now pass. Amend this evidence into the candidate, reprove the final clean SHA, and push main. Never tag before exact-SHA CI and crates publication.

## Activity

- 2026-07-18 00:00 — task created from the release skill's exact-SHA sequence; waiting on Installer credentials (agent: codex)
- 2026-07-18 03:18 — Apple credential preflight passed on native ARM and Intel; task unblocked for final candidate commit and exact-SHA gates (agent: codex)
- 2026-07-18 04:00 — final review added exact PKG file-ownership proof, same-version MSI fixtures, immutable supplemental uploads, and the portable ADR contract; fmt, Clippy, 150 unit + 19 integration tests, release build, audit, package/publish dry run, dist plan, actionlint, Bash syntax, and diff checks passed (agent: codex)
- 2026-07-18 04:10 — committed the reviewed candidate locally and passed `cargo package --locked --list` plus `cargo publish --locked --dry-run` from a clean tree with no `--allow-dirty` bypass (agent: codex)
