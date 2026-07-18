TT;DR: Preserve the partial v4.1.0/v4.1.1 releases, qualify supported Mac ownership proof and real Windows update transitions as v4.1.2, then publish and audit the complete immutable distribution.

## Why

Direct operator order to implement and release the approved plan. The repository release workflow requires main CI and crates publication from the same commit before tagging; Apple/archive or installer failures must fix forward rather than mutate a tag.

## Plan

#c8r is complete. v4.1.1 fixed checkout, Xcode `lipo`, and second-hop SHA identity and passed exact-SHA CI/crates, signed archives, and Windows packaging. Native DMG run 29639898362 built/notarized/installed both architectures, then failed on unsupported `pkgutil --verify`; Windows run 29639998787 exposed PowerShell host, already-current portable, and Inno MSI buffer transition defects. Keep v4.1.0/v4.1.1 immutable. Rerun local gates from a clean committed tree, push the v4.1.2 supported-validator/transition fix, verify exact-SHA CI and crates, then create/push only `v4.1.2`. Watch cargo-dist, supplemental Windows installers, native Mac PKG-in-DMG, and disposable Windows installer validation.

## Impact

Publishes a new crate version and immutable release assets. Premature tagging or skipped gates could strand a broken public version, so order is load-bearing.

## Acceptance

The exact release commit passes CI/crates; the immutable tag points at that SHA; all release workflows succeed without unsigned or cross-channel fallbacks.

## Verification

- [x] Clean-tree local release gates pass without `--allow-dirty`
- [x] v4.1.0 release SHA passed CI/crates and its supplemental DMG failure was retained as immutable evidence
- [ ] Exact v4.1.2 release SHA passes CI and crates publication
- [ ] Tag `v4.1.2` points at that exact SHA and every release workflow succeeds

## Status

Active. #c8r's hosted identity proof passed in run 29637224793 on both native architectures. v4.1.0 exact source `5b4e18d5928e602452a0030a9f5b130dc611d3c9` and v4.1.1 exact source `09afdc6ae5cbff1a497e6cec07c4cf1b36d2557b` remain immutable. v4.1.2 replaces removed Mac verification syntax with supported receipt/file-owner/code-signature proof, corrects Inno's Win32 output buffer declaration, and makes hosted Windows validation exercise a prior-version same-channel update plus no-op/recovery/takeover/uninstall.

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
- 2026-07-18 05:30 — v4.1.0 exact-SHA CI 29638735899, crates 29638873747, and signed archive Release 29638940801 succeeded; supplemental DMG run 29639135342 failed before packaging because checkout cleaned the downloaded `upstream/` inputs (agent: codex)
- 2026-07-18 05:35 — cross-project Xcode 16.4 hosted evidence exposed input-last `lipo -verify_arch`; v4.1.1 now places checkout before download and keeps input-first architecture checks in builder/validator lockstep (agent: codex)
- 2026-07-18 05:50 — v4.1.1 local gates passed: fmt, actionlint/ShellCheck/Bash syntax, warning-denying Clippy, 150 + 19 tests, release build/smokes, audit, dist plan, package list, and publish dry-run. Added a CI semantic guard for checkout/download and `lipo` order (agent: codex)
- 2026-07-18 05:55 — v4.1.0 Windows packaging run 29639135337 succeeded, but chained validation 29639224625 skipped because second-hop `workflow_run` exposed `head_branch=main`; v4.1.1 now resolves exactly one release from upstream `head_sha`, with the resolver replayed successfully against v4.1.0 (agent: codex)
- 2026-07-18 06:40 — v4.1.1 exact source `09afdc6` passed CI 29639632790, crates 29639731682, Release 29639767064, and Windows packaging 29639898355. Mac run 29639898362 completed universal build/sign/notary/staple/mount/install on both architectures, then both validators rejected unsupported `pkgutil --verify`; no DMG published (agent: codex)
- 2026-07-18 06:55 — Windows matrix 29639998787 proved exact-SHA resolution and five clean channels, then exposed PowerShell child-host inheritance, an already-current portable assertion, and Inno's `MsiEnumRelatedProductsW` Pascal `var` buffer failure. v4.1.2 codifies supported replacements and real prior-release transitions in source, workflow, ADR, testing, handoff, and board (agent: codex)
- 2026-07-18 07:20 — v4.1.2 local candidate passed fmt, actionlint 1.7.12, ShellCheck 0.11.0, Git Bash syntax, warning-denying Clippy, 151 + 19 tests, release build/smokes, RustSec audit, cargo-dist plan, 39-file package list, publish dry-run, diff checks, and credential-material scan (agent: codex)
