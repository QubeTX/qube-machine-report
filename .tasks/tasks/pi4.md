TT;DR: Validate the native ARM64 release and updater behavior on Raspberry Pi 4. This remains queued and does not currently block v4.1.0.

## Why

Direct operator order to prove ARM64 Linux behavior on actual Raspberry Pi hardware rather than hosted emulation alone.

## Plan

Verify the ARM64 asset, full/fast/table/ASCII/JSON/save behavior, shell and Cargo updates, permissions, network/session fields, performance, and graceful handling of missing optional utilities.

## Impact

Adds real ARM Linux evidence and can reveal architecture/package/probe assumptions absent from hosted CI.

## Acceptance

The native asset and all core modes/update channels pass, or any defect is captured with reproducible evidence and a fix-forward task.

## Verification

- [ ] Native ARM64 asset and core report modes pass on Raspberry Pi 4
- [ ] Shell and Cargo updates preserve channel/prefix without duplicate active copies
- [ ] Architecture/network/session/permission fields and optional-probe failures are coherent

## Status

Queued for Raspberry Pi 4 access.

## Activity

- 2026-07-18 00:00 — task created from the existing cross-platform hardware continuation (agent: codex)
