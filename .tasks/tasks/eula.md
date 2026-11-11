Replace the MSI placeholder license with the existing repository LICENSE.

## Scope
Both WiX editions share one RTF rendering of LICENSE. Preserve existing v4.4.0
work, product identities, install actions, and release state. Inspect macOS
PKG/DMG for an existing license surface; do not add a new acceptance flow.

## Status
Done (2026-09-24): both published v4.4.0 MSI editions embed the exact complete
PolyForm Noncommercial 1.0.0 license and Required Notice. The public MSI database
readback is recorded in `target/public-v4.4.0-msi-proof.json`. macOS component
PKG/compatibility DMG inspection found no EULA surface or placeholder resource.

## Verification
- [x] Verify the complete RTF text against LICENSE, including Required Notice.
- [x] Compile both MSI editions and inspect their embedded license controls.
- [x] Verify published release MSI controls after the combined v4.4.0 release.
- [x] Inspect macOS packaging: pkgbuild component package, DMG contains PKG and
  README only; no distribution license resource or disk-image EULA.

## Activity
- 2026-09-24 — Traced placeholder to absent WixUILicenseRtf overrides; added
  explicit shared RTF bindings and preserved unrelated in-progress changes.
- 2026-09-24 — Windows RichTextBox and exact compiled MSI control readbacks
  passed for both editions. Independent review found no EULA defects.

- 2026-09-24 — Exact public MSI license-control readbacks and both command inventories passed for release `396235243`; task completed.
