TT;DR: The Apple G2 Developer ID Installer certificate is verified, encrypted, backed up, uploaded to GitHub, and proven by signed disposable packages on native Apple Silicon and Intel runners.

## Why

Direct operator order: Codex should drive the Apple authentication and credential ceremony from the Alienware, pausing only for credentials, 2FA, and native file selection. A Developer ID Installer identity is distinct from the existing Developer ID Application identity and is required to sign the macOS PKG.

The first CSR used RSA 4096 and Apple rejected it with `CSR algorithm/size incorrect. Expected: RSA(2048)`. That request was replaced before certificate issuance. Secret values and key material remain outside the repository and task board.

## Plan

Use Apple G2, not the expiring previous intermediary. Issue Developer ID Installer from the verified RSA-2048 request. Verify the downloaded certificate's type, subject, issuer, validity, public-key match, and full common name. Convert it with the encrypted private key into a password-protected PKCS#12 outside git. Upload only through authenticated GitHub CLI as the Installer certificate/password secrets and set the full common name repository variable. Run a native macOS identity-import proof before packaging.

## Impact

Intended: enables signed system PKGs and the universal PKG-in-DMG workflow. Risks: selecting the wrong intermediary/type, mismatching the certificate and private key, leaking a secret, or publishing before hosted identity validation. The process must fail closed on every mismatch.

## Acceptance

Apple issues a G2 Developer ID Installer certificate matching the encrypted local key; GitHub receives the encrypted secret pair and full common-name variable without log exposure; a native runner imports and identifies it before package construction.

## Verification

- [x] Local CSR reports RSA 2048, SHA-256, valid self-signature, and a public key matching the encrypted private key
- [x] Downloaded certificate is a valid G2 Developer ID Installer certificate matching the CSR/private key
- [x] GitHub Installer certificate/password secrets and full common-name variable exist without credential material in git/logs
- [x] Native GitHub macOS runner proves the imported Installer identity before packaging

## Status

Done. Apple issued the G2 Developer ID Installer certificate. Its RSA-2048 public key, Team ID, Installer EKU, private-key match, and official G2 intermediate chain are verified. The encrypted, Apple-keychain-compatible PKCS#12/password secrets and exact common-name repository variable are uploaded. Redundant private-key generation material was removed; only the encrypted PKCS#12 backup, its Windows-user-encrypted password record, and public certificate copies remain outside git. GitHub Actions run 29637224793 imported that identity and signed/verified a disposable PKG on both native Apple Silicon and Intel.

## Activity

- 2026-07-18 00:00 — Apple rejected the prior RSA-4096 CSR; generated and verified an encrypted RSA-2048 replacement and directed the operator to G2 (agent: codex)
- 2026-07-18 02:33 — Verified the Apple-issued Developer ID Installer certificate against the local key and Apple's official G2 intermediate, created an encrypted PKCS#12, uploaded both GitHub secrets and the full identity variable, and removed redundant key-generation material (agent: codex)
- 2026-07-18 02:35 — Added an Apple Silicon and Intel preflight that imports the identity and signs a disposable PKG before the release DMG build can begin (agent: codex)
- 2026-07-18 03:18 — Run 29637224793 passed the disposable-PKG signing proof on native Apple Silicon job 88061567206 and Intel job 88061567218; no physical Mac was required (agent: codex)
