TT;DR: Audit the freshly published v4.1.3 bytes and close every release ledger only after all 30 assets, trust checks, installer channels, and recovery paths are proven.

## Why

Direct operator order: implementation is not done until published artifacts and updater behavior are verified, and the ADR/testing records reflect the actual outcome.

## Plan

Verify crates.io checksum/version, enumerate 30 nonempty GitHub assets, verify sidecars and fresh public bytes, inspect Developer ID/notarization evidence for both Apple architectures and the universal DMG/PKG, exercise all Windows installer families and update JSON/recovery behavior, then append exact run IDs and hashes to TESTING, MASTER_PLAN, ADR status, and the canonical handoff.

## Impact

Closes the evidence gap between accepted design and public reality. It must not rewrite historical tags or assets.

## Acceptance

Every public distribution claim has a matching observable artifact/run/hash, update and recovery behavior is proven per channel, and tracked ledgers contain no pending release-proof claims.

## Verification

- [x] crates.io and all 30 release assets match the intended v4.1.3 release
- [x] Apple signatures/notarization and every Windows installer channel pass public-byte validation
- [x] TESTING, MASTER_PLAN, ADR status, and handoff record exact final evidence

## Status

Done. v4.1.3 targets exact source `c5a25617b8b6438b1e7589e7518a1c1bd305ed64`; CI 29645549130, crates 29645665879, Release 29645718537, Windows packaging 29645855695, native Mac 29645855688, and Windows validation 29645963379 passed. Thirty nonempty assets, twelve sidecars, eight aggregate checksums, representative versionless latest URLs, and the crates.io archive checksum matched. Windows installers remain explicitly Authenticode-unsigned under the existing ADR; Apple trust is signed/notarized/stapled and fail-closed. #w413 separately tracks the user-approved real-machine installation transition.

## Activity

- 2026-07-18 00:00 — task created as the public release closure gate (agent: codex)
- 2026-07-18 13:35 — completed exact public-byte, crates, Apple trust, all-channel Windows, versionless-link, and immutable-release audit; synchronized ADR/testing/plan/handoff (agent: codex)
