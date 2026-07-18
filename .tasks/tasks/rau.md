TT;DR: Audit the freshly published v4.1.2 bytes and close every release ledger only after all 30 assets, trust checks, installer channels, and recovery paths are proven.

## Why

Direct operator order: implementation is not done until published artifacts and updater behavior are verified, and the ADR/testing records reflect the actual outcome.

## Plan

Verify crates.io checksum/version, enumerate 30 nonempty GitHub assets, verify sidecars and fresh public bytes, inspect Developer ID/notarization evidence for both Apple architectures and the universal DMG/PKG, exercise all Windows installer families and update JSON/recovery behavior, then append exact run IDs and hashes to TESTING, MASTER_PLAN, ADR status, and the canonical handoff.

## Impact

Closes the evidence gap between accepted design and public reality. It must not rewrite historical tags or assets.

## Acceptance

Every public distribution claim has a matching observable artifact/run/hash, update and recovery behavior is proven per channel, and tracked ledgers contain no pending release-proof claims.

## Verification

- [ ] crates.io and all 30 release assets match the intended v4.1.2 release
- [ ] Apple signatures/notarization and every Windows installer channel pass public-byte validation
- [ ] TESTING, MASTER_PLAN, ADR status, and handoff record exact final evidence

## Status

Blocked on #rsh. v4.1.0 and v4.1.1 are intentionally excluded from the 30-asset success claim because their DMG workflows failed before publication. Resume only after all v4.1.2 publication workflows finish.

## Activity

- 2026-07-18 00:00 — task created as the public release closure gate (agent: codex)
