TT;DR: Validate TR-300 reporting and channel-preserving updates on the real AMD64 Linux laptop. This remains queued and does not currently block v4.1.0.

## Why

Direct operator order to continue real-hardware verification beyond hosted runners.

## Plan

Exercise full/fast table, ASCII, JSON, save/no-save, shell and Cargo install/update flows, permissions, network/session fields, architecture asset selection, and graceful optional-probe failures. Record non-sensitive hardware evidence and timings.

## Impact

Adds real AMD/Linux evidence and may expose collector or packaging defects that hosted CI misses.

## Acceptance

All functional/report/update rows pass or produce a reproducible defect with a safe fix-forward plan.

## Verification

- [ ] Full/fast/table/ASCII/JSON/save behavior passes on the AMD64 laptop
- [ ] Shell and Cargo updates preserve channel/prefix without duplicate active copies
- [ ] Hardware/network/session/permission fields and optional-probe failures are coherent

## Status

Queued for access to the AMD64 Linux laptop.

## Activity

- 2026-07-18 00:00 — task created from the existing cross-platform hardware continuation (agent: codex)
