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

Active. #c8r's hosted identity proof passed in run 29637224793 on both native architectures. Exact-SHA runs 29638116741 and 29638544600 failed closed before tagging. Their direct fixes now pass all local and clean-tree gates, including cross-platform checksum tests and version-independent shell syntax. Push the new exact SHA. Never tag before CI/crates publication.

## Activity

- 2026-07-18 00:00 — task created from the release skill's exact-SHA sequence; waiting on Installer credentials (agent: codex)
- 2026-07-18 03:18 — Apple credential preflight passed on native ARM and Intel; task unblocked for final candidate commit and exact-SHA gates (agent: codex)
- 2026-07-18 04:00 — final review added exact PKG file-ownership proof, same-version MSI fixtures, immutable supplemental uploads, and the portable ADR contract; fmt, Clippy, 150 unit + 19 integration tests, release build, audit, package/publish dry run, dist plan, actionlint, Bash syntax, and diff checks passed (agent: codex)
- 2026-07-18 04:10 — committed the reviewed candidate locally and passed `cargo package --locked --list` plus `cargo publish --locked --dry-run` from a clean tree with no `--allow-dirty` bypass (agent: codex)
- 2026-07-18 04:20 — pushed `eb5d212`; exact-SHA CI run 29638116741 failed before tagging on Unix/macOS dead-code warnings plus actionlint 1.7.7 runner-label/shell findings. Retained the full channel taxonomy with explicit lint boundaries, target-gated download constants, upgraded actionlint to 1.7.12, and locally passed actionlint 1.7.12 + ShellCheck 0.11.0 (agent: codex)
- 2026-07-18 04:30 — isolated the Windows pipe-drain test's busy-host PowerShell cold-start flake; gave only the test a 10-second custom budget while retaining all production budgets/caps. Three isolated repetitions and the full 150 + 19 suite passed (agent: codex)
- 2026-07-18 04:40 — fix-forward candidate passed release build, audit, cargo-dist plan, 39-file package list, and `cargo publish --locked --dry-run`; the clean-tree package/publish run used no dirty-tree bypass (agent: codex)
- 2026-07-18 04:50 — exact-SHA run 29638544600 confirmed current actionlint recognized native Intel, then failed on Linux checksum helpers whose tests were Windows-gated and an Ubuntu ShellCheck SC2015; cancelled the already-doomed run, made checksum tests cross-platform, and replaced the notary log boolean chain with explicit control flow (agent: codex)
- 2026-07-18 05:00 — second fix-forward passed fmt, warning-denying Clippy, 150 + 19 tests, release build, actionlint, ShellCheck, Bash syntax, audit, dist plan, and clean-tree package/publish dry runs (agent: codex)
