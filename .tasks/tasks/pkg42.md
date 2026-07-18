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
- 2026-07-18 — v4.2.1 native Intel/ARM run 29663678097 passed identity
  preflights and the universal PKG/DMG sign, notary, staple, mount, and trust
  build, then both direct-PKG validators proved an ambiguous managed receipt
  can leave the package payload after `postinstall` fails. Publication withheld
  the PKG/DMG pairs. v4.2.2 embeds the exact signed universal candidate as a
  `preinstall` migration probe, rejects ambiguous ownership with strict
  `--dry-run` before payload mutation, and retains actual cleanup/rollback in
  `postinstall` (agent: codex)
- 2026-07-18 — the v4.2.2 embedded strict preflight and transactional
  postinstall extract and pass ShellCheck independently; the full executable
  workflow guard also passes locally. Native Apple Installer proof remains the
  release-blocking gate (agent: codex)

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
- [x] Strict ownership ambiguity is evaluated by an exact embedded candidate in
  `preinstall`, before Apple Installer can place the package payload.
- [x] Both generated package lifecycle scripts receive independent pre-tag
  ShellCheck instead of hiding inside quoted builder heredocs.
- [ ] Native Intel and Apple Silicon direct package lifecycle passes.
- [ ] Immutable v4.1.3 DMG updater crosses to the final release on both runners.
- [ ] Direct PKG/sidecar and compatibility DMG/sidecar publish without clobber.
- [ ] Final public checksums, signatures, tickets, receipts, and 34 assets pass.

## Resume

After the clean-tree exact-SHA CI/crates gates pass, tag v4.2.2 and require both
native architectures to finish the complete package and legacy bridge lifecycle
before the publisher attaches the PKG/DMG pairs. Audit fresh public bytes; do not
substitute Windows emulation or a physical Mac for a failed native runner.
