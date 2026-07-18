# Direct universal PKG and legacy DMG bridge

## Status

Active. Direct-PKG source/workflows are implemented; native hosted credentials,
signing/notary/install/update/bridge evidence and publication remain open.

## Activity

- 2026-07-18 — replaced the current DMG-first builder/updater with a signed,
  notarized, stapled direct universal PKG while retaining a signed DMG containing
  the byte-identical package solely for immutable v4.1.x clients (agent: codex)
- 2026-07-18 — added native Intel/ARM direct lifecycle, receipt/file-owner,
  managed-shell takeover, malformed-owner rollback, and v4.1.3 legacy-bridge
  gates; local workflow, shell, Xcode `lipo` ordering, and package-source checks
  pass. Hosted Apple trust and public-byte evidence remain open (agent: codex)

## Intent

Publish the signed/notarized/stapled universal PKG directly because Apple does
not require a DMG around it. Retain a signed compatibility DMG containing the
byte-identical PKG only so immutable v4.1.0-v4.1.3 updaters can cross into the
new release. A physical Mac remains optional and non-blocking unless native CI
exposes a GUI-only defect.

## Verification

- [x] Current updater selects exact-tag PKG and reports precise `mac_pkg`
  strategy while retaining the stable legacy channel ID.
- [x] Builder signs/notarizes/staples direct PKG and compatibility DMG and
  proves nested/direct package byte equality.
- [x] Xcode 16.4 `lipo <file> -verify_arch ...` ordering is preserved.
- [ ] Native Intel and Apple Silicon direct package lifecycle passes.
- [ ] Immutable v4.1.3 DMG updater crosses to the final release on both runners.
- [ ] Direct PKG/sidecar and compatibility DMG/sidecar publish without clobber.
- [ ] Final public checksums, signatures, tickets, receipts, and 34 assets pass.

## Resume

After the clean-tree exact-SHA CI/crates gates pass, tag v4.2.0 and require both
native architectures to finish the complete package and legacy bridge lifecycle
before the publisher attaches the PKG/DMG pairs. Audit fresh public bytes; do not
substitute Windows emulation or a physical Mac for a failed native runner.
