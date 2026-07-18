# TR-300 Architecture Decisions

> Long-form rationale for the load-bearing technical choices in TR-300.
> Day-to-day editing rules live in [`../CLAUDE.md`](../CLAUDE.md); this file
> exists for the **why** — when a contributor is about to undo a decision and
> needs the original reasoning, the rejected alternatives, and the failure mode
> the current code prevents.
>
> Each section is dated by the version that introduced (or substantially
> revised) the decision. Verbatim moves from CLAUDE.md preserved word-for-word
> so `git blame` history continues to make sense.
>
> **Coverage reconciled through 2026-07-18.** This is the repository's
> canonical ADR ledger: one document organized by decision family rather than
> one file per decision. Accepted decisions remain binding until a later dated
> section explicitly supersedes them. Historical failure evidence is retained
> because it explains why the guardrails exist.

## Table of contents

- [Decision ledger status (through v4.1.2)](#decision-ledger-status-through-v412)
- [Origin-preserving updates and native PKG-in-DMG distribution (v4.1.0)](#origin-preserving-updates-and-native-pkg-in-dmg-distribution-v410)
  - [Latest discovery, immutable installation](#latest-discovery-immutable-installation)
  - [Install channel is product state](#install-channel-is-product-state)
  - [Fresh installer intent and legacy consolidation](#fresh-installer-intent-and-legacy-consolidation)
  - [Why the PKG is inside the DMG](#why-the-pkg-is-inside-the-dmg)
  - [Apple credentials and hosted-native proof](#apple-credentials-and-hosted-native-proof)
  - [Hosted packaging lifecycle and tool syntax (v4.1.1 addendum)](#hosted-packaging-lifecycle-and-tool-syntax-v411-addendum)
  - [Supported macOS ownership proof (v4.1.2 addendum)](#supported-macos-ownership-proof-v412-addendum)
  - [Windows transition runner correctness (v4.1.2 addendum)](#windows-transition-runner-correctness-v412-addendum)
  - [Machine-readable and failure contract](#machine-readable-and-failure-contract)
  - [Reusable contract for other CLI products](#reusable-contract-for-other-cli-products)
- [Cross-platform report semantics (v4.0.0)](#cross-platform-report-semantics-v400)
  - [One Rust product, platform adapters, and two collection budgets](#one-rust-product-platform-adapters-and-two-collection-budgets)
  - [Stable table, JSON, locale, and privacy contracts](#stable-table-json-locale-and-privacy-contracts)
  - [Why this is v4.0.0 rather than v3.18.0](#why-this-is-v400-rather-than-v3180)
  - [Evidence-backed optional values](#evidence-backed-optional-values)
  - [One native macOS snapshot, including Rosetta](#one-native-macos-snapshot-including-rosetta)
  - [Disk and memory value definitions](#disk-and-memory-value-definitions)
  - [Bounded subprocess, save, JSON, and update primitives](#bounded-subprocess-save-json-and-update-primitives)
  - [Why report persistence is explicit-only](#why-report-persistence-is-explicit-only)
  - [Why endpoint-policy update blocks stop the strategy chain](#why-endpoint-policy-update-blocks-stop-the-strategy-chain)
  - [Recovered gates must block](#recovered-gates-must-block)
  - [Enforced Developer ID signing and Apple notarization](#enforced-developer-id-signing-and-apple-notarization)
  - [Post-release personal-hardware evidence boundary](#post-release-personal-hardware-evidence-boundary)
- [Toolchain & release](#toolchain--release)
  - [Default branch is `main` (2026-07-17)](#default-branch-is-main-2026-07-17)
  - [MSRV policy (v3.11.1+, addendum v3.13.1)](#msrv-policy-v3111-addendum-v3131)
  - [Self-update auto-rustup (v3.11.1+)](#self-update-auto-rustup-v3111)
  - [Intel macOS coverage policy (v3.11.2+)](#intel-macos-coverage-policy-v3112)
- [Windows accuracy patterns](#windows-accuracy-patterns)
  - [v3.11.0+ — registry OS, IsWow64Process2, WTS last-login, CPUID hypervisor, BitLocker](#v3110--registry-os-iswow64process2-wts-last-login-cpuid-hypervisor-bitlocker)
  - [v3.12.0+ — VPN-aware default-route, Fast Startup uptime annotation](#v3120--vpn-aware-default-route-fast-startup-uptime-annotation)
  - [v3.13.0+ — 5-state battery, native cores, GPU registry-prefer, PSCore detection, terminal walk](#v3130--5-state-battery-native-cores-gpu-registry-prefer-pscore-detection-terminal-walk)
  - [v4.1.0 — Native Windows hybrid-core topology](#v410--native-windows-hybrid-core-topology)
  - [v3.14.4+ — Windows install execution-policy preflight](#v3144--windows-install-execution-policy-preflight)
  - [v3.14.5+ — Windows install error advisor](#v3145--windows-install-error-advisor)
  - [Windows distribution model (v3.15.0+)](#windows-distribution-model-v3150)
    - [v3.17.0 addendum — advisory one-install consolidation](#v3170-addendum--advisory-one-install-consolidation)
  - [v3.15.1 addendum — Why corporate.wxs lives in `wix-corporate/`, not `wix/`](#v3151-addendum--why-corporatewxs-lives-in-wix-corporate-not-wix)
- [Install / update safety primitives (v3.15.2+)](#install--update-safety-primitives-v3152)
  - [Atomic rc-file writes](#atomic-rc-file-writes)
  - [Marker-balance pre-check](#marker-balance-pre-check)
  - [SHA256 verification of downloaded installers](#sha256-verification-of-downloaded-installers)
  - [Post-install version verification](#post-install-version-verification)
  - [WMI hard-timeout pattern](#wmi-hard-timeout-pattern)
  - [Windows self-EXE delete via detached cleanup](#windows-self-exe-delete-via-detached-cleanup)

---

## Decision ledger status (through v4.1.2)

This reconciliation compared the ledger against current source, all release
and validation workflows, the v4 thinking record, both Mac/Alienware handoffs, the testing
ledger, technical and human changelogs, agent guides, and the public v4.0.1
release state. The result is an accepted decision set, not a claim that every
future hardware row has already been exercised.

| Decision family | Status | Enforcement / source of truth |
|---|---|---|
| One Rust CLI/library with cfg-gated platform adapters | Accepted | `src/collectors/`, `src/install/`, shared `SystemInfo`, report, and JSON paths |
| Full versus fast collection budgets | Accepted | `CollectMode`; fast may omit slow optional evidence but cannot redefine values |
| Evidence-backed nullable facts and named value definitions | Accepted | collectors, schema-v1 JSON, table/Markdown renderers, tests |
| Fixed-width terminal and additive JSON compatibility | Accepted | `unicode-width`, typed `serde_json`, locale/code-page setup before rendering |
| Read-only ordinary reports; explicit-only Markdown persistence | Accepted | four save aliases; hidden `--no-save` compatibility no-op |
| Bounded optional probes and fail-safe endpoint-policy updates | Accepted | command helper, randomized staging, `PolicyBlocked`, no force/direct overwrite |
| Developer ID plus Apple `Accepted` before Mac artifact upload | Accepted and release-blocking | signing script and protected cargo-dist workflow step |
| Frozen Mac surface during personal Windows/Linux/Pi continuation | Accepted | agent guides, current handoff, release workflow, hardware task boundary |
| Personal hardware evidence | Alienware complete; AMD64 laptop and Pi 4 still open | `#win`, `#amd`, `#pi4`, `TESTING.md`; hosted jobs do not impersonate physical machines |
| GitHub default branch `main` and checkout v6/Node 24 | Accepted and hosted-verified | GitHub default metadata, workflow filters/actions, exact-SHA CI/crates runs |
| Two-place Rust 1.95 pin and tag-gated cargo-dist publication | Accepted | `Cargo.toml`, `rust-toolchain.toml`, release skill/workflows |
| Windows native-first facts and four-installer model | Accepted | Windows collector, update origin marker, WiX/Inno sources and supplemental workflow |
| Origin-preserving updates with no cross-channel fallback | Accepted and release-blocking | `src/update.rs`, installer receipts/markers, isolated update fixtures |
| Native universal macOS PKG-in-DMG | Accepted and release-blocking | `macos-installer.yml`, signing script, native ARM/Intel install gates |
| Hosted package inputs survive source checkout; builder and validation tool syntax stay identical | Accepted and release-blocking | checkout-before-download ordering plus Xcode 16.4 `lipo <file> -verify_arch ...` checks |
| macOS installed-package ownership uses supported receipt, file-owner, and Developer ID checks | Accepted and release-blocking | `pkgutil --pkg-info/--files/--file-info`, strict `codesign`, native ARM/Intel install gates |
| Explicit fresh-installer choice supersedes the prior Windows channel | Accepted | WiX/Inno cross-format removal, scoped markers, advisory cross-edition cleanup |
| Physical Mac requirement | Superseded; optional visual smoke only | GitHub `macos-15` and `macos-15-intel` are the required native gates |

The following earlier states are **superseded**, not alternative supported
modes: automatic report saving, `master` as the default branch, non-blocking
Mac/audit CI gates, ad-hoc-signed public Mac binaries, unsigned Mac release
fallbacks, path-only Windows installer inference, the v3.17 cargo-first updater
chain, Intel-as-release-only coverage, the claim that a physical Mac is needed
for release proof, the claim that a bare CLI must not have a PKG, and any
force/direct-running-binary updater bypass. Historical sections retain them
only as failure context.

---

## Origin-preserving updates and native PKG-in-DMG distribution (v4.1.0)

**Status:** Accepted. This section supersedes the older updater fallback,
Intel-runner, bare-binary packaging, and same-edition coexistence decisions
where they conflict. The governing user intent is: an update preserves the
channel that installed the running copy, while an explicit fresh installer
launch is a newer choice and may replace the previous channel.

### Latest discovery, immutable installation

Public commands, links, and asset filenames are versionless. Users download
from `/releases/latest` or `/releases/latest/download/<stable-name>` and never
need to edit a version embedded in a command. The updater follows a stricter
two-stage rule: it queries the GitHub latest-release API once, validates the
resolved tag/version, then constructs every payload and checksum URL from that
immutable tag. Cargo receives the resolved `--version` and `--locked`; shell
and PowerShell installers are fetched from the exact tag even though their
filenames remain stable.

This split is deliberate. A mutable latest URL is the correct human-facing
entry point, but resolving it again during a multi-file update creates a
time-of-check/time-of-use race if a release changes between the version check,
payload download, and sidecar download. Stable filenames plus exact tagged
bytes give both a durable user command and a reproducible update attempt.
Cargo-dist's plan necessarily contains the exact announcement tag, but the
checked-in release workflow normalizes the published release-note install and
download URLs back to `/releases/latest/download/` before creating the release.
The release tag/title still identify immutable history; user-facing artifact
names and copy-paste commands do not embed a version.
Cargo-dist creates the release once, and supplemental Windows and macOS jobs
upload without `--clobber`. A collision or partial prior upload therefore fails
the release; it is fixed forward under a new version instead of changing bytes
or checksums behind an existing tag.

### Install channel is product state

The updater treats install format, edition, scope, and prefix as state rather
than as a preference to rediscover on every run:

| Detected running origin | Only permitted CLI update strategy |
|---|---|
| Global MSI | Global MSI, passive, UAC when required |
| Corporate MSI | Corporate per-user MSI |
| Global EXE | Global Inno Setup EXE |
| Corporate EXE | Corporate per-user Inno Setup EXE |
| cargo-dist PowerShell receipt | Exact-tag PowerShell installer and recorded prefix |
| Cargo metadata | Exact resolved Cargo version with `--force --locked` |
| macOS package receipt | Universal DMG and Apple Installer |
| cargo-dist shell receipt | Exact-tag shell installer and recorded prefix |
| Unknown, portable, or conflicting evidence | No mutation; recovery links only |

Windows writes separate `InstallSourceGlobal` and `InstallSourceCorporate`
values while retaining the legacy `InstallSource` value for compatibility. A
missing marker may be recovered only when Add/Remove Programs registration,
scope, and executable path identify exactly one MSI or Inno product. Cargo uses
Cargo metadata. cargo-dist uses its versioned receipt and recorded install
prefix. macOS uses the stable `pkgutil` receipt
`com.qubetx.tr300.pkg`, but receipt existence alone is not ownership proof. The
receipt identifier, version, install scope, payload list, per-file owner, and
installed binary's valid Developer ID Application identifier, Team ID, and
authority must all agree with the running `/usr/local/bin/tr300`. Path alone
cannot distinguish Windows MSI from EXE and therefore cannot authorize a
mutation.

No strategy crosses into another channel after failure. Preserving the old
working install is more important than “trying something” that creates a
second registration, changes Corporate to Global, changes per-user to
per-machine, or shadows the running binary. This also makes very old versions
safe: legacy evidence is recovered when unambiguous and otherwise stops with a
fresh-installer link instead of guessing.

### Fresh installer intent and legacy consolidation

A CLI update preserves origin; a manually launched fresh installer expresses
newer intent. Within a Windows edition, an MSI detects and removes the matching
Inno registration before writing shared-path files, and Inno enumerates and
removes the matching MSI product before installation. The new installer then
writes its scoped marker. MSI UpgradeCode and Inno AppId continue to handle
ordinary same-channel upgrades across arbitrarily old versions. Both MSI
packages set WiX `MajorUpgrade AllowDowngrades='yes'`: an explicitly launched
older or same-version MSI replaces the registered same-edition MSI instead of
being blocked. This same-version behavior is part of the
[WiX v3 `MajorUpgrade` contract](https://docs.firegiant.com/wix3/xsd/wix/majorupgrade/),
not an inference from the path or product name. `RemoveExistingProducts` is
scheduled after `InstallInitialize`, so a failed replacement can roll the old
package back. The downgrade permission is intentional because a fresh installer
launch is user intent; the automatic updater never selects an older release.
This works for v4.1.0-and-newer MSI packages. An immutable historical MSI owns
its already-published launch policy and may still block when launched against a
newer installation; that safe stop cannot be fixed retroactively and must not
be bypassed by deleting registered product state behind Windows Installer.

The v3.17 advisory `migrate-cleanup` remains the bounded post-install mechanism
for a shadowing Cargo copy and the other Global/Corporate executable. It may
not delete a toolchain or the running install. A no-admin Corporate install
can safely report that a Global copy needs elevated removal rather than
pretending the conflict was resolved. The active new copy and scoped marker
still reflect the latest explicit intent; installer-matrix gates must detect
duplicate active binaries, PATH entries, or product registrations.
The pre-tag Windows gate builds a second MSI with the same version and another
with a synthetic newer version for each edition, then proves a deliberate
newer-to-current-to-same-version sequence converges to one current registration
and executable.

### Why the PKG is inside the DMG

The DMG is not mechanically required; a signed PKG could be published alone.
For TR-300, the nested design is the smallest distribution that satisfies the
whole contract:

- the universal Mach-O supports both `arm64` and `x86_64`;
- the signed system package owns `/usr/local/bin/tr300`, presents Apple's
  normal privileged Installer prompts, and creates the stable receipt needed
  for origin-preserving updates;
- the signed DMG is the familiar versionless downloadable container and can
  include short recovery instructions without inventing an `.app` wrapper;
- the PKG and DMG are independently notarized, stapled, signature-checked, and
  Gatekeeper-assessed before publication.

A loose binary in a DMG was rejected because it gives no package receipt,
privileged install transaction, uninstall inventory, or reliable updater
origin. An artificial GUI `.app` was rejected because TR-300 remains a CLI.
Publishing only the PKG was rejected as a weaker download/recovery experience,
not because Apple requires the DMG.

The updater downloads the exact-version DMG, verifies its SHA-256 sidecar,
validates and mounts it read-only, validates the nested `tr300.pkg`, and invokes
Apple Installer with `open -W`. Success is reported only after Installer
returns, the receipt exists, and `/usr/local/bin/tr300 --version` matches the
resolved version. Later update detection revalidates that the receipt claims
that exact file and version, `pkgutil --file-info` names the same package as
owner, and the binary retains TR-300's Developer ID product identity; a stale
receipt beside a replaced or ad-hoc-signed binary fails closed. Cancellation
or failure leaves the old installation in place and exits 2.

### Apple credentials and hosted-native proof

Developer ID Application and Developer ID Installer are distinct identities.
The Installer private key and CSR may be generated with OpenSSL on the
Alienware; Apple Developer portal authentication, 2FA, CSR upload, and
certificate download do not require a Mac. Apple requires RSA 2048 for this
portal CSR and the current G2 intermediary is preferred over the expiring
previous Sub-CA. The certificate and encrypted private key are converted to a
password-protected, Apple-keychain-compatible PKCS#12 outside the repository,
uploaded through authenticated GitHub CLI, and imported only into an ephemeral
runner keychain. Secret values never enter source, task boards, browser fields,
or logs.

Repository credentials add
`APPLE_INSTALLER_CERTIFICATE_P12_BASE64` and
`APPLE_INSTALLER_CERTIFICATE_PASSWORD`; the corresponding repository variable
is `APPLE_INSTALLER_SIGNING_IDENTITY` and contains the certificate's full
Developer ID Installer common name. Existing application-signing and App
Store Connect credentials remain separate.

Before any release PKG/DMG packaging, both native runner architectures import
the Installer PKCS#12 into an ephemeral keychain and use `pkgbuild` to sign a
disposable package. Verifying that package proves both the certificate and its
matching private key are usable; merely finding the public certificate in a
keychain is not sufficient evidence. Local OpenSSL integrity is also not a
substitute for this gate: PKCS#12 algorithms accepted on Windows can still be
rejected by macOS Keychain. Secret upload must use raw standard input without a
literal `--body -` or newline transformation, and the hosted signing proof is
rerun after any certificate, password, container, variable, or upload change.

GitHub's native `macos-15` Apple Silicon and `macos-15-intel` runners are the
required gate. They build, sign, notarize, mount, install, report, select the
update strategy, and uninstall on the architecture they claim. A physical Mac
is optional for visual inspection of Finder/Installer prompts and becomes
blocking only if CI exposes a GUI-only defect. This avoids a strange
cross-compilation/signing workaround on Windows without making the maintainer
rent or borrow hardware for every release.

### Hosted packaging lifecycle and tool syntax (v4.1.1 addendum)

Source checkout is a workspace lifecycle boundary, not a harmless read. The
checkout action cleans untracked files, so release inputs downloaded into the
repository workspace before checkout may disappear even after their hashes
were successfully verified. Supplemental packaging must therefore check out
the exact immutable tag first and download release assets second (or place the
assets in an explicitly preserved directory outside the checkout). The job
must re-check that each expected input exists at the packaging call boundary.

Workflow event identity is likewise not transitive. A workflow triggered from
another `workflow_run` executes on the default branch; a second workflow chained
from it receives `head_branch=main`, not the tag that triggered the original
release. A downstream validator must bind itself to the successful immediate
upstream run's exact `head_sha`, resolve exactly one immutable GitHub Release
targeting that SHA, validate the semver tag and required assets, and fail closed
on zero or multiple matches. Parsing a display title or assuming the tag
survives another event hop was rejected as ambiguous metadata.

Apple command-line syntax is also part of the release contract. On Xcode 16.4,
universal-architecture verification requires the input operand first:
`lipo <file> -verify_arch arm64 x86_64`. The input-last form failed on a hosted
runner. The universal builder and both post-install architecture gates use the
same input-first form. This is deliberately recorded as a lifecycle invariant,
not merely a one-line correction: a package can be assembled correctly while
a later validation command still rejects the same artifact if the two stages
drift.

v4.1.0 supplied the failure evidence. Its exact-SHA CI, crates publication,
and signed/notarized per-architecture archives succeeded, but supplemental run
29639135342 downloaded and verified both archives, then checkout removed the
`upstream/` directory before the builder ran. The DMG was never created or
uploaded. Supplemental Windows packaging run 29639135337 succeeded, but its
chained validation run 29639224625 skipped because the second-hop
`head_branch` was `main`. Its tag and existing assets remain immutable; v4.1.1
fixes forward with checkout-before-download, input-first `lipo` calls, and
exact-SHA release resolution for the Windows validation hop. This is the same
immutable-release rule used for checksum, signature, or installer defects.

**Rejected alternatives:** re-downloading inside the signing script hides the
workflow ownership boundary and introduces a second network transaction;
disabling checkout cleanup weakens reproducibility; altering or attaching new
bytes to v4.1.0 would make the historical release mutable. None is acceptable.

**Revalidation triggers:** changing checkout action/version or order, changing
the workspace/output location, changing Xcode/macOS runner images, changing
universal-binary tooling, changing a builder/validator command, or adding a
workflow-event hop requires a native hosted packaging run through mount,
install, both architecture checks, immutable upload, and every downstream
validation job.

### Supported macOS ownership proof (v4.1.2 addendum)

Native v4.1.1 packaging run 29639898362 successfully downloaded both exact
archives after checkout, used the input-first Xcode 16.4 `lipo` form, created
and signed the universal binary and PKG, notarized and stapled the PKG and DMG,
mounted the DMG, passed Gatekeeper, and installed the package on both Apple
Silicon and Intel. Both validation jobs then failed at the same legacy call:
current macOS `pkgutil` rejected `--verify` as an unrecognized option. The
current Xcode `pkgutil(1)` command set contains `--pkg-info`, `--files`,
`--file-info`, `--check-signature`, and related receipt operations, but no
installed-package `--verify` operation. This was a validator defect, not an
excuse to weaken origin proof, and v4.1.1's published assets remain immutable.

The supported v4.1.2 ownership proof is conjunctive:

1. `pkgutil --pkg-info com.qubetx.tr300.pkg` must report the exact package ID,
   current executable version, and root volume. A root package may report an
   empty `location:` or `/`; any non-root location is rejected.
2. `pkgutil --files com.qubetx.tr300.pkg` must contain exactly the expected
   relative payload `usr/local/bin/tr300`, and `pkgutil --file-info
   /usr/local/bin/tr300` must name `com.qubetx.tr300.pkg` as its package owner.
3. `codesign --verify --strict /usr/local/bin/tr300` must succeed. Its detailed
   metadata must report identifier `com.qubetx.tr300`, Team ID `M9D5379H93`,
   and a Developer ID Application authority. The installed executable must
   then report the receipt/expected version.

Package-container trust remains independently enforced before installation:
`pkgutil --check-signature` validates the nested PKG signature,
`spctl -a -t install` performs Gatekeeper assessment, and `stapler validate`
checks the notarization ticket. These are not substitutes for installed-file
ownership, just as receipt presence is not a substitute for code identity.
The runtime updater intentionally invokes only supported public commands; it
does not parse undocumented receipt database internals or depend on a removed
compatibility switch.

**Rejected alternatives:** receipt presence alone permits a replaced binary to
inherit a stale channel; version/path alone permits an unsigned or foreign
binary to impersonate the package; requiring `location: /` rejects valid root
receipts observed on current hosted macOS where the value is empty; querying an
obsolete `pkgutil --verify` command makes every genuine PKG install look
unknown. All four violate safe origin preservation.

**Revalidation triggers:** macOS/Xcode runner changes, package identifier or
payload changes, signing identifier/Team/certificate changes, receipt parser
changes, or updater origin changes require both native architectures to repeat
DMG checksum/signature/notary checks, mount, PKG trust checks, install, receipt
and file-owner proof, binary signature/version proof, update selection, and
uninstall cleanup.

### Windows transition runner correctness (v4.1.2 addendum)

The first fully dispatched v4.1.1 Windows matrix proved the exact-SHA resolver
and clean install/update-detection/uninstall paths for both MSIs, both EXEs, and
Cargo. It also exposed three distinct test/transition issues that clean installs
could not reveal:

- Both EXE-over-MSI takeover jobs exited 1 during Inno initialization. The
  first hypothesis was an incorrect `var` declaration for the
  `MsiEnumRelatedProductsW` output buffer. Hosted replay 29640785777 disproved
  that as a sufficient fix: Inno Setup 6.7.1 still access-violated when the
  preallocated string was passed without `var`. The decision is therefore not
  to cross this DLL ABI from Pascal Script at all. The shared include uses
  Inno's supported `RegGetSubkeyNames`/`RegQuery*` APIs and recognizes an MSI
  only from the edition's correct registry scope plus exact display name,
  publisher, `WindowsInstaller=1`, and a syntactically valid GUID product key.
  It collects and bounds the full set before mutation, waits for each exact
  `msiexec /x`, accepts only success/already-absent, and stops before writing
  new files on ambiguity, restart, or removal failure. Because recognition is
  based on durable native registration rather than only the current
  UpgradeCode, the transition also covers older official MSI registrations
  that retain the same product identity.
- Starting legacy `powershell.exe` as a child of `pwsh` inherits the parent's
  `PSModulePath`. On current Windows runners that can make Windows PowerShell
  discover PowerShell 7's incompatible `Microsoft.PowerShell.Security` module
  and exit before cargo-dist installation. Hosted validation executes the
  script in its current `pwsh` host; runtime same-channel update prefers `pwsh`
  when available and retains `powershell.exe` as the same-channel fallback.
  This changes the interpreter, not the installation channel or prefix.
- An already-current portable binary is a successful no-op even though its
  origin remains `unknown`; no mutation is needed. An older portable binary
  with an available update must instead exit 2, remain byte/version unchanged,
  require user action, and return the versionless recovery page. Treating those
  two states as the same assertion either invents a failure or fails to test
  recovery.

The release validation matrix therefore installs the most recent lower stable
release that contains every required Windows family, exercises a real
old-to-new CLI update for each recognized channel, verifies the target version
and preserved marker/registration/PATH, then invokes update again to prove the
already-current no-op. Portable uses that same older release to prove safe
recovery without mutation. Separate jobs still test both directions of
same-edition MSI/EXE fresh takeover because explicit installer intent is a
different state machine from CLI update.

**Rejected alternatives:** accepting a clean install as updater proof; invoking
a different installer family after failure; hard-coding a historical version
into the workflow; treating every unknown-origin invocation as an error;
guessing another Pascal declaration for a Win32 output buffer after two hosted
access violations; or ignoring a nonzero Inno/PowerShell exit. The matrix
resolves immutable releases and stable asset names dynamically and fails before
claiming convergence.

**Revalidation triggers:** changes to the PowerShell cargo-dist template or
launcher order, MSI UpgradeCodes/product scope, Inno registry identity or
takeover code, marker/ARP recovery, update JSON semantics, portable behavior,
or Windows runner/PowerShell images require the full prior-version update,
current-version no-op, cross-format takeover, and uninstall matrix.

### Machine-readable and failure contract

Update JSON stdout is exactly one object; child installer output and progress
go to stderr. Existing keys and Windows `install_origin` remain. Additive keys
are `install_channel`, `recovery_url`, and `requires_user_action`. A failed
known installer channel also carries `exact_installer_url`; this pins the
matching recovery payload while `recovery_url` stays the versionless release
page. Successful same-channel/no-op updates return 0. Cancellation, ambiguity,
restart-required installer exits, blocked staging, failed verification, or installer failure
returns 2 and keeps the old installation.

When the channel is known, failure output includes the exact tagged matching
payload and the versionless latest release page. When origin is unknown, no
payload is guessed and the fresh-installer page is the recovery path. Windows
MSI codes 3010 and 1641 and the equivalent Inno restart exit are never
misreported as a complete in-place update.

### Reusable contract for other CLI products

The v4.1.2 implementation is intended to be copied into other standalone CLI
products. Product names, package identifiers, paths, asset names, registry
keys, and installer GUIDs are parameters; the following invariants are the
portable architecture:

1. **Separate explicit install from self-update.** A deliberately launched
   fresh installer is the user's newest instruction and may select a different
   format, scope, edition, or even an older/same version. A self-update is
   latest-only and preserves the proven format, scope, edition, and prefix.
   Never apply fresh-install downgrade semantics to an automatic update.
2. **Make origin durable product state.** Native product registration and
   package receipts outrank paths. Product-owned receipts are versioned and
   record product identity, channel, prefix, installed version, and write time.
   A path may bound a candidate but cannot distinguish formats that share it.
   Missing state is recoverable only from one unambiguous native registration;
   equal, stale, or conflicting evidence becomes unknown.
3. **Prove receipt ownership, not receipt presence.** For a native package,
   require the package ID, recorded payload path, installed file owner,
   receipt/package version, running executable path, and installed binary's
   native signing identity to agree. Validate the package container's signature,
   notarization, and platform trust separately before installation. Use only
   commands supported by the current platform; an obsolete verification switch
   that always fails is not conservative because it hides every legitimate
   origin. Apply equivalent identity/registration checks on Windows and
   package-manager metadata checks elsewhere.
4. **Resolve mutable discovery once.** Public filenames, download commands, and
   recovery pages remain versionless. The updater resolves `latest` once, then
   pins the tag/version for the payload, sidecar, package-manager command, and
   post-install expectation. Do not fetch one transaction from multiple
   mutable `latest/download` URLs.
   Treat the resolved tag and every attached asset as immutable: supplemental
   upload jobs must refuse collisions rather than clobber public bytes.
5. **Treat replacement as a transaction.** Use private randomized staging,
   payload size limits, cryptographic digest checks, native signature/trust
   checks, and the original installer engine. Do not delete or directly
   overwrite the running installation as a fallback. Report success only after
   the installer exits successfully, durable ownership metadata matches, and
   the intended executable reports the resolved version.
6. **Converge fresh installs before claiming ownership.** An official installer
   removes or upgrades recognized conflicting registrations for the selected
   product before it writes shared-path files. If privilege is insufficient,
   it stops or reports the exact remaining conflict; it never claims a
   one-copy result it did not establish. A package manager with no install-time
   lifecycle hook needs an official wrapper/first-run convergence step or must
   be documented as unable to guarantee cross-method takeover.
7. **Fail closed and remain recoverable.** Cancellation, ambiguity, policy
   blocks, reboot-required exits, checksum/signature failure, or post-install
   mismatch retains the old working copy, returns a stable nonzero exit, and
   supplies both the exact matching tagged asset when one is known and the
   versionless fresh-install/release page. JSON stdout remains exactly one
   backward-compatible object; child progress is stderr-only.
8. **Test transitions, not only clean installs.** Every release gate covers
   old-to-new and very-old-to-new updates, same-version repair, explicit
   downgrade, both directions of cross-format takeover, missing metadata,
   conflicting metadata, cancellation/UAC denial, restart-required exits,
   corrupted downloads, PATH/registration cleanup, uninstall, and the absence
   of duplicate active binaries. Tests deliberately remove markers/receipts so
   recovery behavior is evidence rather than an unexercised branch.
9. **Treat workflow ordering and validator syntax as product code.** Establish
   the immutable source checkout before downloading transient release inputs,
   or preserve those inputs outside the cleaned workspace. Keep every builder
   and post-install validator invocation in lockstep, including operand order,
   and revalidate after runner/Xcode changes. Bind chained workflows to exact
   upstream SHA/release identity instead of assuming branch/tag context crosses
   event hops. A successful checksum in one step does not prove the input still
   exists or that a later tool call or downstream job is valid.

These rules are intentionally stricter than “download the newest binary and
replace the file.” The portable unit is the installation transaction and its
durable ownership evidence, not merely an executable byte stream.

---

## Cross-platform report semantics (v4.0.0)

Substantially revised and released 2026-07-14. Alienware verification completed
2026-07-17; AMD64 Linux laptop and Raspberry Pi 4 live checks remain tracked
continuation and their absence must not be rewritten as proof.

### One Rust product, platform adapters, and two collection budgets

TR-300 is one Cargo package, one `tr300` binary, and one Rust library surface.
macOS, Linux, and Windows are not separate products and must not grow separate
CLI grammars, report schemas, renderers, or release versions. Shared code owns
the `SystemInfo` aggregate, collection orchestration, value definitions,
rendering, JSON, save policy, and error contract. Platform differences live in
cfg-gated collectors and installers, with native APIs/files used first and
bounded subprocess fallbacks used only where they add defensible evidence.

`SystemInfo::collect_with_mode` runs the OS, CPU, memory, disk, network,
session, and platform collectors in scoped parallel work, converts core-thread
panics into a reportable error, and treats platform enrichment as optional.
This keeps one semantic assembly point while allowing each OS to use its own
authoritative sources. A missing tool, denied permission, timeout, malformed
response, or unsupported fact becomes absence/fallback—not a platform fork and
not a whole-report failure.

The same product intentionally has two collection budgets:

- **Full** is the normal detailed report. It may run bounded profiler, WMI,
  resolver, login, encryption, firmware, or health probes.
- **Fast** is the shell-startup path. It avoids slow subprocess-heavy work and
  may leave optional rows/JSON values absent, but values it does emit retain the
  same meaning as full mode. Fast does not substitute guesses for skipped facts.

The install-time auto-run uses `tr300 --fast`; the `report` alias remains a
normal full invocation. Neither mode saves unless the explicit full-table save
flag is present. This separation makes startup latency a contract without
weakening the accuracy contract.

**Rejected alternatives:**

- Separate macOS/Windows/Linux editions: fixes would drift, consumers would
  receive different schemas, and cross-platform comparisons would stop meaning
  the same thing.
- Run every probe in fast mode: shell startup would inherit the slowest WMI,
  profiler, login, resolver, or encryption command and violate the 1.5-second
  hosted budget.
- Give fast mode friendly placeholder values: omission accurately says “not
  collected under this budget”; a placeholder can be mistaken for measured
  hardware state.
- Move platform parsing into the renderer: presentation would become coupled to
  command output and make table/JSON/Markdown disagree.

**Consequences and revalidation:** a platform-local accuracy fix belongs in its
cfg-gated collector plus shared fixtures/tests. A change to shared aggregation,
definitions, schema, renderer, command helper, dependencies, or mode semantics
is cross-platform and reopens the relevant Linux/Windows/Mac gates; during the
Alienware continuation it also reopens native arm64 and Rosetta qualification.

### Stable table, JSON, locale, and privacy contracts

The human table and machine JSON are two views of the same collected record,
not independent implementations. The table remains a compact 51-display-column
surface with a 12-column label and 32-column value field. Display width is
measured with `unicode-width`, not scalar/byte count, so wide characters cannot
break borders. Strings truncate with an ellipsis inside the allotted display
width. ASCII mode changes only presentation characters, not values.

Encoding setup precedes output. On Unix, locale inspection can select ASCII
before any box-drawing character is printed. On a Windows terminal, the output
code page is set to UTF-8 before the report renders. This ordering is
load-bearing: printing a Unicode banner first would make the fallback too late.

Schema-v1 JSON is constructed as a typed `serde_json::Value` and serialized
once. Existing keys and types remain stable; v4 additions are nullable/context
keys. Numeric non-finite values become JSON `null`. CPU, disk, memory, load,
frequency, route, uptime, and availability fields include enough provenance or
definition context that consumers do not need to infer platform-specific
meaning from a label. Table, JSON, and manually saved Markdown must derive from
the same facts.

Privacy is part of the information contract. Useful model, board, firmware,
display, battery, session, and network context may be reported; hardware serial
numbers, platform UUIDs, and similar persistent unique identifiers are not.

**Rejected alternatives:**

- Count `.chars()` for terminal layout: Unicode scalar count is not terminal
  display width.
- Hand-build JSON punctuation/escaping: unusual device/user text and non-finite
  values can produce invalid or ambiguous output.
- Use different field definitions per renderer: a value that changes meaning
  between table and JSON is not cross-platform precision.
- Print Unicode before locale/code-page setup: recovery cannot repair bytes
  already emitted.
- Add serials/UUIDs “for completeness”: they add tracking risk without helping
  the machine-health purpose.

**Consequences and revalidation:** renderer changes require fixed-width ASCII
and Unicode tests; JSON changes require parse/schema compatibility tests and a
consumer review; platform information additions require privacy review. A
shared renderer/schema change also crosses the frozen Mac boundary.

### Why this is v4.0.0 rather than v3.18.0

The command-line and JSON work is additive: existing schema-v1 keys retain
their names and types. The Rust library surface is different. `SystemInfo`,
`Config`, and several public collector records gained fields, and selected
public collector helper signatures changed. A downstream crate that uses a
struct literal or exhaustive pattern can therefore stop compiling. Rust SemVer
treats that as a major change even when ordinary CLI users see no break.

The release is consequently v4.0.0. Changed record types are marked
`#[non_exhaustive]` at the major boundary so future additive fields can remain
minor releases. The v4 migration note must tell library consumers to prefer
collection/default APIs, avoid exhaustive public-record patterns, and update
calls to the changed collector helpers. This is more accurate than either
calling the source break “additive” or falsely presenting JSON as incompatible.

### Evidence-backed optional values

An absent optional field means “TR-300 could not establish this fact through a
reliable, bounded source.” It does **not** mean the negative of the fact.

Examples:

- No virtualization signal is not proof of Bare Metal. A hypervisor label is
  emitted only for positive CPUID, registry, `/proc`, container, WSL, or known
  macOS guest-model evidence.
- A failed BitLocker/FileVault/LUKS probe is not proof that encryption is Off,
  and not proof that elevation would make the probe work.
- Missing physical topology is not replaced with logical processors. SMT and
  vCPU counts must not be relabeled as physical cores.
- Windows has no Unix-style 1m/5m/15m load average. One instantaneous CPU sample
  is not repeated into three fake time windows.
- Rosetta's translated compatibility CPU frequency is not host hardware data;
  `null` is more precise.

**Rejected alternative: friendly defaults such as `Bare Metal`, `1 socket`, or
logical=physical.** These look complete but turn lack of evidence into a false
claim, which is especially damaging in JSON automation. Optional absence is
deliberate information.

### One native macOS snapshot, including Rosetta

Full-mode macOS launches one bounded
`system_profiler -json SPHardwareDataType SPDisplaysDataType SPPowerDataType
SPSoftwareDataType` snapshot. The collector parses only the facts it displays:
model name/identifier, GPU names, current display mode and native pixel size,
battery condition/maximum capacity/cycles, actual boot state, and known guest
virtualization indicators. It deliberately does not surface serial numbers,
hardware UUIDs, platform serials, or other unique device identifiers.

One snapshot was chosen over several profiler invocations because profiler is
the slowest Mac probe, JSON is less locale-sensitive than its text format, and
the fields describe one coherent moment. Missing/malformed sections retain the
existing quick `sysctl`/`ioreg`/`pmset` fallbacks.

When the TR-300 process is translated, `/usr/bin/arch -arm64
/usr/sbin/system_profiler ...` requests the native slice. Live testing showed
the x86_64 slice can return generic model/chip values and omit battery maximum
capacity. The native slice describes the same physical host accurately while
`sysctl.proc_translated` still lets the report name the process scope as
`arm64 host / x86_64 (Rosetta 2)`. If native launch fails, the translated
profiler remains a fallback.

**Rejected alternatives:**

- Run separate profiler commands per data type: slower and can describe
  different moments.
- Parse localized text: brittle across locales and macOS revisions.
- Label Apple Silicon as a boot mode: CPU architecture is unrelated to Normal,
  Safe, or Recovery boot state.
- Trust the translated 2.4 GHz value: it is a compatibility observation, not a
  defensible host-frequency measurement.
- Include hardware serial/UUID for “more information”: unnecessary for a
  machine-health report and creates privacy/tracking risk.

macOS 26 `Tahoe` and macOS 27 `Golden Gate` mappings are checked against
[Apple's current macOS page](https://www.apple.com/os/macos/); unknown future
versions remain unlabeled rather than being guessed.

### Disk and memory value definitions

#### Disk

The report chooses one system/root volume: `/` on Unix or the normalized system
drive on Windows, then a single largest fixed-volume fallback, then a removable
volume only if no fixed volume exists. It never sums every mount. APFS volumes,
BTRFS subvolumes, bind mounts, Windows drive letters, and container mounts can
overlap or describe independent resources; a machine-wide sum is not a coherent
capacity.

On Unix, `statvfs` distinguishes total blocks, all free blocks, and blocks
available to an unprivileged caller. On Windows, `GetDiskFreeSpaceExW` provides
the corresponding total/free/available values. Therefore:

- `disk.used_bytes = total - free` and means allocated bytes.
- `disk.available_bytes` means bytes available to the current caller.
- `disk.percent` is allocated / total.

These values can legitimately make used + available differ from total because
of reserved blocks, quota views, and filesystem policies. JSON names both
definitions so consumers do not infer the wrong equation.

#### Memory

Non-macOS platforms use operating-system available memory and define used as
`total - available`. macOS uses the Activity Monitor-compatible working set
from `vm_stat`: `(active + wired + compressed) * page_size`. Available is
`total - reported_used`, so the displayed pair is internally consistent and
cannot say both “81% used” and “0 available.”

Inactive/cached pages are intentionally not counted as used in that Mac formula.
Swap is reported separately. JSON includes `usage_definition` and
`availability_definition` because “used RAM” is not a universal kernel metric.

**Rejected alternative: expose sysinfo's Mac used and available counters
together.** On the live M2 host, compressor accounting made those independent
values contradictory. A shared definition is more useful than mixing two
legitimate but incompatible accounting views.

### Bounded subprocess, save, JSON, and update primitives

Optional tools are untrusted from a reliability perspective: they may hang,
spawn descendants, block after filling one pipe, or produce unbounded output.
`src/collectors/command.rs` therefore drains stdout/stderr concurrently, caps
captured output at 8 MiB, enforces fast/normal/slow deadlines, kills the Unix
process group on timeout, and uses a Windows Job Object best-effort. A missing
tool, timeout, nonzero status, malformed response, or overflow yields `None`.

**Rejected alternative: `Command::output()` plus a polling timeout.** A child
can block in `write()` after a pipe fills, never reaching the timeout polling
state; killing only the immediate process can also leave descendants alive.

Report JSON is assembled as a typed `serde_json::Value` and serialized once.
This centralizes escaping and non-finite handling. Schema version 1 remains
because every new field is additive; existing key names and types are retained.

Markdown files use an OS Downloads directory when available, unique suffixes,
`create_new`, flush/sync, and cleanup on error. Existing paths and symlinks are
never followed or overwritten. The writer is reachable only through explicit
`-r`/`--report`/`-s`/`--save`; ordinary full/fast/JSON runs are the
side-effect-free default. `--no-save` remains a hidden compatibility no-op.

Windows updater payloads use a `tempfile`-managed private randomized directory,
bounded response sizes, explicit cleanup, and cleanup diagnostics. The SHA256
sidecar detects corruption or a mismatched payload/sidecar pair; it is not
described as an independent signature because both files share release
transport. Post-install `--version` remains the success source of truth.

### Why report persistence is explicit-only

The original convenience behavior automatically wrote a Markdown file after
every full table run. On a managed Windows endpoint, the operator observed the
work antivirus flagging that unexpected report-file activity and freezing the
machine. The data collection and terminal output were healthy; the unsolicited
persistence was the trigger.

The reliable default is therefore read-only: `tr300`, `report`, fast, ASCII,
and JSON modes do not call `save_markdown_report`. Users who want an artifact
make that intent explicit with `-r`/`--report`/`-s`/`--save`. Those flags
conflict with fast/JSON/actions so the saved document retains the established
full-table contract. The existing writer is preserved exactly where it matters:
manual files remain collision-safe, symlink-resistant, flushed, and cleaned on
incomplete writes.

**Rejected alternatives:**

- Keep auto-save and merely document `--no-save`: a safe managed-machine run
  should not depend on remembering an opt-out.
- Remove saving entirely: deliberate support/report workflows still need the
  Markdown artifact.
- Detect “corporate antivirus” dynamically: endpoint products and policies are
  not reliably enumerable, and guessing would reintroduce inconsistent behavior.

### Why endpoint-policy update blocks stop the strategy chain

Self-update necessarily writes a staged installer. A policy product can deny
directory creation, file writes/sync, or process launch through ordinary I/O
errors or Windows policy codes. Treating that like a transient strategy failure
and immediately trying cargo plus multiple installers performs more of the
activity the endpoint just rejected and can compound a workstation freeze.

v4 classifies likely policy errors as `PolicyBlocked`, records `blocked` in
human/JSON diagnostics, stops the fallback chain, explicitly cleans staging,
keeps the current installation, links to official manual downloads, and exits
2. Failed JSON contains `manual_install_url`; blocked JSON also contains the
direct `official_releases_url`, so non-interactive callers get the same recovery
path as terminal users. A verified successful install remains success even if best-effort staging
cleanup later reports a warning; success is still determined by re-running the
installed `tr300 --version`.

**Rejected alternatives:**

- Prompt to force a direct running-binary replacement: antivirus can interrupt
  the overwrite and destroy the only working copy; the user explicitly withdrew
  this idea.
- Continue trying other installers: they require the same denied write/launch
  primitives and increase endpoint noise.
- Label every I/O failure “antivirus”: unknown failures remain normal failures;
  only a conservative error set gets the policy-specific label, while all paths
  still fail without claiming success.

### Recovered gates must block

The macOS ARM test/build/speed jobs were made non-blocking in v3.14.5 after a
short period of hosted-runner zero-second failures. Subsequent local and hosted
evidence recovered, and v4.0.0 adds native/Rosetta coverage plus
substantial Mac-specific behavior. Leaving `continue-on-error` in place would
turn that evidence into dashboard decoration. The exceptions are removed: a
red macOS test, release build, or 1.5s speed result blocks CI.

The same rule applies to `cargo audit`. The only known locked advisory was
resolved by moving `crossbeam-epoch` to 0.9.20; keeping audit advisory-only after
the graph is clean would allow a new known vulnerability through a release.

Intel hosted CI remains a separate capacity decision: per-commit CI tests Apple
Silicon, local Rosetta runs exercise the x86_64 binary for v4.0.0, and
cargo-dist still builds the Intel artifact at tag time. See the existing Intel
macOS coverage policy below.

### Enforced Developer ID signing and Apple notarization

Auditing the public v3.17.0 arm64 archive showed `Signature=adhoc`, no Team ID,
Gatekeeper rejection, and no stapled ticket. Local collector testing cannot
repair a public distribution-trust gap, and an undocumented external setup
cannot enforce future releases initiated from Windows.

v4.0.0 makes the trust path tracked and fail-closed. In publishing Apple matrix
jobs, `release.yml` invokes `scripts/sign-notarize-macos.sh` after
`dist build` and before cargo-dist Post-build/upload. The script uses an
ephemeral keychain and work directory, imports one Developer ID Application
identity, resolves exactly one configured match inside that keychain, and signs
by its certificate fingerprint. This avoids the macOS `codesign` ambiguity that
occurs when the same certificate display name also exists in the login
keychain. It signs with stable identifier `com.qubetx.tr300`, hardened runtime,
and a trusted timestamp; verifies authority, Team ID, identifier, runtime, and
timestamp; submits the binary to Apple with a least-privilege App Store Connect
API key; and requires `Accepted`. It then repacks the exact signed bytes,
regenerates the `.sha256` sidecar, patches the archive checksum in the per-target
cargo-dist manifest, verifies it, and deletes decoded credentials.

The immutable v4.0.0 tag exposed a clean-runner distinction that a developer
Mac with the same certificate in its login keychain had masked: importing an
identity into a newly created keychain does not automatically put that keychain
on the calling user's search list, and `codesign` requires signing identities to
be discoverable there even when `--keychain` narrows lookup. The v4.0.0 workflow
therefore failed closed before upload. v4.0.1 snapshots the original user
search list, prepends the ephemeral keychain only for `codesign`, restores the
list immediately and in the cleanup trap, and extracts the embedded leaf
certificate from the final Mach-O to compare its SHA-1 fingerprint with the
resolved identity. This keeps fingerprint selection unambiguous while making
the clean CI environment behave like the proven local environment.

Repository secrets hold the PKCS#12, its password, API private key, key ID, and
issuer ID; repository variables select the signing identity/team. Only the
names are documented. Pull-request planning does not receive them because the
step additionally requires cargo-dist's publishing output.

This bare-archive conclusion applied to the v4.0 cargo-dist artifacts and is
superseded for the installer-first path by the v4.1.0 PKG-in-DMG decision. The
archives still use Developer ID plus Apple's accepted service record because a
bare Mach-O has no ticket container. The new PKG and DMG are containers and
must both be stapled and Gatekeeper-assessed in addition to the nested binary's
signature verification.

**Rejected alternatives:**

- Manual signing on one Mac after each tag: not enforceable when the maintainer
  pushes from Windows and easy to forget.
- Host first and replace assets later: creates a public unsigned window and a
  race between manifests/checksums.
- Unsigned fallback when secrets are absent: defeats the stated trust contract.
- Wrap the CLI in a GUI app solely for stapling: still rejected. A PKG is now
  accepted for its system-install transaction and stable receipt, not as a
  cosmetic ticket wrapper.

#### Alienware continuation freeze (historical; superseded by v4.1.0)

The v4.0.1 handoff temporarily froze the proven archive path while work moved
back to Windows. v4.1.0 intentionally reopens that boundary for a tracked,
reviewable PKG-in-DMG workflow. Windows remains unsuitable for executing Apple
signing tools, but it is valid for authoring the workflow, generating the
Installer CSR/private key, and managing GitHub credentials. Native hosted Mac
runners now supply the required execution and proof.

A shared or Apple-artifact change still invalidates old Mac evidence, but the
required revalidation is native `macos-15` plus native `macos-15-intel` hosted
CI. A physical Mac is needed only for a defect that cannot be observed through
those runners.

### Post-release personal-hardware evidence boundary

The strongest original plan required personal Alienware, AMD64 Linux laptop,
and Raspberry Pi 4 evidence before v4.0.0. Those machines were unavailable when
the maintainer needed the release finished. The maintainer explicitly chose to
release after comprehensive Mac/local/hosted gates and perform the personal
matrix afterward, shipping forward patches for real findings.

This is a scope decision, not fabricated evidence. Hosted Windows/Linux jobs
prove compilation, unit behavior, and runner smokes; they do not prove OEM GPU/
battery/firmware, personal network, or Pi-specific runtime facts. The task board,
`TESTING.md`, and handoff keep those rows open. The managed work machine's
antivirus incident informs report/update side-effect design but is not the
personal Alienware accuracy test.

**Rejected alternatives:**

- Mark hardware rows waived/passed: false and destroys the evidence ledger.
- Keep the release indefinitely blocked despite direct maintainer risk
  acceptance: conflates a product milestone with unavailable lab scheduling.
- Drop the tasks after release: removes the feedback loop the deferral depends on.

---

## Toolchain & release

### Default branch is `main` (2026-07-17)

**Status:** Accepted, implemented, hosted-verified. This supersedes `master` as
the live default; historical records that name `master` remain factual for the
runs and commits they describe.

#### Context and forces

The operator's other repositories use `main`, and explicitly requested the same
convention here only if CI, crates publication, cargo-dist releases,
supplemental Windows packaging, Apple trust enforcement, and public deployment
could be preserved without ambiguity. A branch rename is therefore a release-
systems decision, not cosmetic repository housekeeping.

Before mutation, the audit established:

- the unchanged default-branch tip was
  `cd3c179540b48770e1c555cbf60c809d702eb999`;
- `ci.yml` and `crates-publish.yml` already accepted `main` during transition;
- `release.yml` publishes only from version-like tags, not from a default-branch
  name;
- `windows-installers.yml` follows a successful named Release workflow through
  `workflow_run`, not `master`;
- no open pull request, branch protection, ruleset, webhook, or deployment
  environment visible through GitHub was bound to `master`;
- release tags, crates.io records, public assets, Apple credentials, and the
  homepage were separate records that did not need recreation.

#### Decision

1. Use GitHub's supported branch-rename operation to atomically rename
   `master` to `main` at the same commit. GitHub's
   [branch-renaming documentation](https://docs.github.com/en/enterprise-cloud@latest/repositories/configuring-branches-and-merges-in-your-repository/managing-branches-in-your-repository/renaming-a-branch)
   describes the platform-managed redirects and branch metadata behavior this
   path provides.
2. Rename the local branch, track `origin/main`, and refresh `origin/HEAD` to
   `origin/main`. The remote exposes `main` only; do not keep a second live
   `master` alias.
3. Narrow live branch filters to `main` only after the atomic rename. A later
   accidental `master` recreation must not become another CI/crates publication
   route.
4. Update canonical release skills, agent guides, project status, testing
   ledger, handoff, changelogs, and planning documents. Preserve historical
   `master` references where rewriting them would falsify old evidence.
5. Keep direct-default-branch delivery subject to the same local and hosted
   quality gates. Branch naming changes the route, not the standard.

#### Workflow topology and exact-SHA boundary

The preserved release topology is:

```text
push to main
  -> CI (13 blocking format/clippy/test/build/speed/audit/dist-plan jobs)
  -> Crates.io Publish workflow_run checks out the CI-tested head SHA
       -> existing version: skip before token/check/package/publish access
       -> new version: rerun locked gates, then publish

explicit vX.Y.Z tag after main CI/crates settle
  -> cargo-dist Release (six targets, aliases, fail-closed Apple jobs)
  -> Windows Installers workflow_run (three installers + sidecars)
```

`crates-publish.yml` checks `github.event.workflow_run.head_repository` and
event type in addition to success, then checks out
`github.event.workflow_run.head_sha`. This prevents a successful unrelated or
fork run from publishing. Its crates.io existence query happens before secret
use; a documentation-only commit at already-published 4.0.1 therefore proves
the chain without accessing the registry token or trying to republish.

Tags remain the only binary-release trigger. A branch/workflow/documentation
change with unchanged `Cargo.toml` does not justify a version bump, retag, or
artifact replacement. Creating a new tag merely to “deploy” a branch rename
would manufacture duplicate product artifacts and unnecessarily rerun Apple and
Windows release machinery.

#### Actions runtime alignment

The first green `main` runs exposed a non-failing annotation: the remaining
`actions/checkout@v4` steps targeted deprecated Node 20 and GitHub was forcing
them onto Node 24. `release.yml` and `windows-installers.yml` already used
checkout v6 successfully, so leaving CI/crates on v4 created needless runtime
drift and a future failure risk.

All four workflows now use `actions/checkout@v6`. The
[official checkout documentation](https://github.com/actions/checkout)
identifies v6 as the Node 24 release and notes the minimum runner version for
self-hosted use. TR-300 currently uses GitHub-hosted runners; their actual
Linux, Apple Silicon, and Windows execution is the decisive compatibility
proof. If a self-hosted runner is introduced later, its runner version becomes
an explicit prerequisite rather than an assumption.

The checkout upgrade was deliberately limited to `ci.yml` and
`crates-publish.yml`; the already-proven generated release workflow and
hand-authored supplemental Windows workflow were not regenerated or otherwise
changed. That scope avoids reopening the frozen Mac artifact path for a cleanup
that provided no Mac benefit.

#### Immutable distribution boundary

The branch and checkout changes do not mutate already-published distribution
records:

- immutable `v4.0.0` remains the historical failed-closed tag;
- immutable `v4.0.1` remains at release source
  `b67ad083503d0fff840af8467015d05c659268ea`;
- crates.io remains unyanked 4.0.1 with its existing checksum;
- the GitHub Release remains non-draft/non-prerelease with 28 uploaded,
  nonempty assets;
- both public Mac archives retain their exact hashes, Developer ID signature,
  Team ID, identifier, hardened runtime, timestamp, and Apple `Accepted`
  submissions;
- the homepage remains deployed from its own `main` commit
  `d77397479ad2b1189cce86b5402eaf1cc966abdf`.

Repository Apple secret **names** and non-secret variables can be audited;
their values must never enter this ADR, git, task memory, or logs.

#### Rejected alternatives

- **Create `main`, push it, change default, then delete `master` manually.**
  Multiple operations create an avoidable split-brain interval and do not gain
  anything over GitHub's supported atomic rename/redirect behavior.
- **Keep both `master` and `main` active.** Two default-like branches invite
  divergent commits and, if filters drift, duplicate CI or publication paths.
- **Rename first and assume integrations follow.** Workflow filters, policies,
  open PR bases, webhooks, environments, local upstreams, release triggers, and
  raw branch URLs all require explicit audit.
- **Rewrite every historical `master` mention.** That would falsify release
  ledgers and old run descriptions rather than improve current instructions.
- **Ignore checkout v4's Node warning because the run is green.** GitHub's
  compatibility shim is not a stable contract; waiting converts a known,
  low-risk maintenance change into a future CI outage.
- **Leave branch CI on v4 while release workflows use v6.** Mixed action
  runtimes make pre-tag validation less representative of release automation.
- **Regenerate `release.yml` merely to align action versions.** It already used
  v6, and regeneration risks losing legacy aliases or the protected Apple gate.
- **Bump or retag 4.0.1 for repository metadata.** No product/package bytes
  changed; immutable release records should remain immutable.

#### Consequences and operating invariants

- New clones, PR defaults, direct development, and release-source work use
  `main`. Existing stale clones may need the normal Git upstream rename steps;
  they must not recreate `origin/master`.
- `ci.yml` and `crates-publish.yml` remain `main`-only. Release remains tag-
  triggered, and supplemental Windows packaging remains Release-workflow-
  triggered.
- Every workflow uses checkout v6/Node 24. A future action-major change requires
  official-source review, `actionlint`, exact-SHA hosted execution on all CI
  operating systems, and a crates workflow proof.
- Direct pushes to `main` are authorized for this maintainer but never bypass
  local gates or exact-SHA hosted verification.
- Branch-only documentation commits legitimately trigger the crates workflow;
  already-published versions must exit successfully through the pre-token skip.
- Any future branch rename, new deployment integration, ruleset, self-hosted
  runner, changed workflow name, or trigger redesign requires reopening this
  decision and auditing the complete chain.

#### Verification evidence

- Migration commit `41c30b1e43f8abc5208f0d94702ed12cd91fb7a7`:
  CI 29557626125 passed all 13 jobs; exact-SHA crates run 29557758673 safely
  skipped existing 4.0.1.
- Migration attestation `bc936ad8450cc4c85b07dfadff0dbe5761ebb237`:
  CI 29558048158 passed all 13 jobs; crates run 29558195163 safely skipped.
- Checkout-v6 commit `1714d1fc0b90475d5f0aa590b1ec7d93b24d2eee`:
  CI 29559148638 passed all 13 jobs with zero check annotations and no
  checkout-v4/Node-20 log match; crates run 29559305341 used v6 and skipped
  token/check/package/publish access for existing 4.0.1.
- Tracked final attestation
  `eb2ce8b362a54b77a8921cd9666e84f69d423b10`: CI 29559379791 passed all
  13 jobs with zero annotations and no deprecated-checkout log match; crates
  run 29559541494 repeated the exact-SHA pre-token safe skip.
- A fresh public clone selected `main` at the final attestation, GitHub's old
  `/tree/master` URL redirected to `/tree/main`, `origin/HEAD` resolved to
  `origin/main`, and no remote `master` remained.
- All four workflows remained active. v4 tags, 28 release assets, crates.io
  checksum, public Mac signature/notary evidence, and production homepage
  bundle/wrappers were re-audited unchanged.
- Architecture-ledger backfill
  `e38fe2abcffdf6f85d4dac1c12dd294f36604a59`: CI 29560970377 passed all 13
  jobs on the exact commit with zero annotations and no deprecated checkout/
  Node-runtime log match. Crates run 29561137746 checked out the same SHA,
  found 4.0.1 already published, and skipped registry-token access plus every
  check/package/publish step. This is the hosted proof for the substantive ADR
  reconciliation; the follow-up commit only records that proof.

### MSRV policy (v3.11.1+, addendum v3.13.1)

`rust-version` is pinned in `Cargo.toml` and tracks the GitHub Actions
`stable` toolchain. As of 3.11.1 it's `1.95` because `std::arch::x86::__cpuid`
and `std::arch::x86_64::__cpuid` were reclassified as safe-to-call in
Rust 1.95 (no safety preconditions on x86/x86_64 — CPUID is universally
available), which made our `unsafe { __cpuid(_) }` wrappers in
`src/collectors/cpu.rs` and `src/collectors/platform/windows.rs` trip the
`unused_unsafe` lint. Under our `-D warnings` policy that's a hard build
error. Bump `rust-version` whenever a new stable lint or stdlib change
forces source edits — and at the same release pin so that users running
older toolchains hit cargo's MSRV check, not E0133s deep in collector
modules.

*Why pin MSRV instead of supporting older Rust via shims:* there are three
realistic alternatives, and we considered each.

1. **`#[allow(unused_unsafe)]` on every `unsafe { __cpuid(_) }` block.**
   Compiles on both old and new toolchains. *Rejected* because the `allow`
   is permanent — once added, even Rust toolchains where the lint is
   correct (i.e. the unsafe block really is necessary because someone
   added a genuinely-unsafe call inside it later) will silently swallow
   the warning and we'd miss real safety regressions. It also bloats every
   CPUID callsite with attribute noise that has to be re-justified at
   review time, and it propagates: every future stable lint we want to
   straddle adds another permanent `allow`. Tech debt that compounds.

2. **`#[cfg(rustc_version)]` ladders to gate per Rust version.** Requires
   pulling in `rustversion` or `rustc_version` build-script crates, adds a
   build-time fingerprint to every release, and means our source has two
   parallel implementations of the same logic — one with `unsafe`, one
   without — that have to be kept in lockstep. *Rejected* as
   over-engineering for a tool whose CI deliberately uses
   `dtolnay/rust-toolchain@stable` and ships from a single toolchain.

3. **Pin MSRV to the CI toolchain (this approach).** Cargo's existing
   `rust-version` field already enforces this without any source-level
   shims. Older toolchains get `error: package tr300@3.11.1 cannot be
   built because it requires rustc 1.95.0 or newer, while the currently
   active rustc version is 1.94.0` — clear, actionable, and points at
   exactly the right knob to fix. Combined with auto-rustup in
   `--update`, users on rustup-managed toolchains never see the error at
   all because `tr300 --update` brings their stable forward in lockstep
   with the MSRV pin. Users on distro-managed toolchains see the clear
   error and can update on their own schedule.

The combination — pin in `Cargo.toml`, auto-rustup in `--update`, README
mentions `rustup update stable` ahead of `cargo install tr300` — gives us
a coherent toolchain story across all three install paths (binary
installer, fresh `cargo install`, self-update) without source-level
compatibility shims.

**v3.13.1 addendum — `rust-toolchain.toml` is the fourth leg.** `Cargo.toml`'s
`rust-version` declaration alone is not enough to keep `release.yml` green.
The release workflow is auto-generated by cargo-dist v0.31.0 (see
`.github/workflows/release.yml` line 1: `# This file was autogenerated by
dist`); its only Rust setup is downloading the `dist` binary itself and
immediately invoking `dist build`. There is no `rustup install` step.
Each `build-local-artifacts` job uses whatever rustc the
GitHub-hosted runner image happens to have pre-installed, which on
`ubuntu-22.04`, `ubuntu-22.04-arm`, and `windows-2022` is currently 1.94.1
— below MSRV 1.95. The result was that v3.10.0 / v3.12.0 / v3.13.0 all
tagged but produced no GitHub Release (3/6 build-local-artifacts jobs
failed; cargo-dist is all-or-nothing).

The fix is `rust-toolchain.toml` at repo root with `[toolchain] channel =
"1.95"` AND `components = ["rustfmt", "clippy"]`. Rustup is pre-installed
on every GitHub-hosted runner image, and when cargo runs from a workspace
containing `rust-toolchain.toml`, the rustup proxy auto-installs the
pinned channel before delegating to cargo. This works in `release.yml`
(which never invokes rustup directly), in `ci.yml` (which still calls
`dtolnay/rust-toolchain@stable` but the `rust-toolchain.toml` override
wins — same toolchain in CI as in releases, single source of truth), and
locally for any contributor who clones fresh. See [rustup overrides](https://rust-lang.github.io/rustup/overrides.html)
for the precedence rules.

The `components = ["rustfmt", "clippy"]` half is non-obvious but
load-bearing: when rustup processes a `rust-toolchain.toml` that does
not list components, it installs only the default profile (rustc, cargo,
rust-std) and *ignores* the `components:` field passed to action-level
toolchain installers like `dtolnay/rust-toolchain@stable`. The result is
that the `Format` and `Clippy` CI jobs fail with `error: 'cargo-fmt' is
not installed for the toolchain '1.95-x86_64-unknown-linux-gnu'` and
their clippy equivalent. Listing them in the file is the canonical way
to make rustup install them alongside the channel. Release.yml runners
get the extra ~few MB download too, which is harmless.

The MSRV is now expressed in **two** places that must move together:
`Cargo.toml`'s `rust-version` (the cargo-side declaration that produces
the clear `error: package tr300@X.Y.Z cannot be built because it
requires rustc N.M ...` message for users on older toolchains) and
`rust-toolchain.toml`'s `channel` (the rustup-side override that ensures
GitHub-hosted runners and contributor machines actually pull the right
toolchain). Future MSRV bumps must update both. Channel choice is a
minor pin (`"1.95"`, not `"stable"` or `"1.95.0"`): rustup installs the
latest patch in the 1.95.x line so we benefit from patch releases
without having to bump for them, but we don't float forward across
minors silently.

### Self-update auto-rustup (v3.11.1+)

**v4.1.0 superseding rule:** the auto-rustup behavior below remains only for a
confirmed Cargo origin, and Cargo is pinned to the resolved release with
`--version <resolved> --force --locked`. The ordered cross-method fallback
chain described historically below is retired. Shell, PowerShell, MSI, EXE,
and PKG/DMG origins each have their own single channel-preserving strategy;
unknown origins do not mutate the machine.

Before v4.1.0, `src/update.rs` checked the same latest-release endpoint,
compared against `VERSION`, and ran this ordered probe-and-retry chain:
- `cargo install tr300 --force` first when `cargo --version` succeeds
- macOS/Linux fallback: cargo-dist shell installer via `curl`, then `wget`
- Windows fallback: cargo-dist PowerShell installer via `powershell`, then `pwsh`

`--update --json` emits a single JSON object with `current_version`, `latest_version`, `update_available`, and `success`. Success includes legacy `"method"` plus precise `"strategy"`; failure includes an `"attempts"` array. Exit codes: `0` success, `2` failure.

**v3.14.2 addendum — path detection is retired.** Earlier releases inferred the
update method from the current executable path (`.cargo/.../bin/...` meant
`cargo install`). That turned out to be the wrong signal once cargo-dist was
configured with `install-path = "CARGO_HOME"`: the official shell installer can
also place `tr300` under `.cargo/bin`, so path-based detection could choose
cargo on machines that do not have cargo installed. The updater now probes
tools directly and falls through to installer strategies on any preflight or
runtime failure.

**v3.14.3 addendum — canonical package name is `tr300`.** The crates.io package
name is lowercase `tr300`, matching the binary and Rust library import path.
The cargo strategy therefore runs `cargo install tr300 --force`. GitHub Release
installer assets use `tr300-installer.sh` and `tr300-installer.ps1`; the release
workflow also copies those files to legacy `tr-300-installer.*` aliases so
v3.14.2 binaries, whose fallback URL used the deleted old package name, can
still self-update through the installer path after cargo fails.

**Auto-rustup on the cargo strategy (v3.11.1+).** When the strategy chain tries
`UpdateStrategy::Cargo` it first calls `rustup_update_stable_best_effort()`,
which probes for `rustup` on PATH (via `rustup --version`, redirecting both
stdout and stderr to `Stdio::null()` so the probe is silent) and, if found,
runs `rustup update stable` and prints `Updating Rust toolchain (rustup
update stable)…` so the user sees what's happening. Any failure — rustup
absent, network timeout, locked toolchain, permission error — is *non-fatal*:
we discard the result with `let _ =` and proceed straight to the
`cargo install tr300 --force` call. Installer strategies never touch Rust
because they download a prebuilt binary.

*Why this exists — the failure mode it prevents:* TR-300's MSRV tracks the
GitHub Actions `stable` toolchain and moves whenever Rust ships a stable
release that promotes a lint we trigger or changes safety classifications
on stdlib intrinsics we use (cf. the 1.95 `__cpuid` reclassification that
prompted this change). Without auto-rustup, a user who installed via
`cargo install tr300` on Rust 1.94 and then later runs `tr300 --update`
against a release built with `rust-version = "1.95"` would see cargo print
`error: rustc 1.94.0 is not supported by the following package: tr300@…
requires rustc 1.95`, our `execute_update` would propagate that as a
non-zero exit, and the user would be silently stuck on the stale binary
forever — they'd assume `--update` "doesn't work" and either give up or
manually research the toolchain pin themselves. The 5–30 seconds spent on
a redundant `rustup update stable` (effectively a no-op when already
current — rustup just prints `info: cleaning up downloads & tmp directories`
and exits) is dramatically cheaper than that user-experience failure, and
this pattern means MSRV bumps in future releases stop being a coordination
problem with users.

*Why best-effort instead of "fail loudly if rustup isn't there":* not every
user manages Rust through rustup. Distro packages (Debian's `rustc`/`cargo`,
Homebrew's `rust`, NixOS's nixpkgs Rust), CI environments where rustup is
intentionally absent, and corporate-managed toolchains all install Rust
without putting `rustup` on PATH. Hard-failing in those cases would be
worse than the status quo (we'd block working updates on a tool we don't
need). Probing first and silently skipping when it's missing means we help
the rustup majority while not surprising the minority — they just see the
plain `cargo install` path and any MSRV mismatch surfaces normally, with
the standard cargo error pointing at `rust-version` so they can update
their distro/Homebrew Rust on their own terms.

*Why we don't probe rustc version and conditionally call rustup:* the
naive alternative — parse `rustc --version`, compare against an MSRV
constant, only run rustup when older — adds two new failure modes (parse
errors, drift between the constant and `Cargo.toml`'s `rust-version`) and
saves at most a few seconds. Always running `rustup update stable` is
simpler, idempotent, and self-correcting; rustup itself decides whether
work is needed.

### Intel macOS coverage policy (v3.11.2+)

**Current policy (v4.1.0; supersedes the remainder of this historical
section).** GitHub now exposes the native `macos-15-intel` runner label used by
this repository. Intel is once again a blocking test/build and PKG-in-DMG
install gate, paired with native Apple Silicon `macos-15`. Do not remove either
matrix cell merely because the old `macos-13` public fleet was unreliable. A
physical Intel Mac and Rosetta emulation are no longer substitutes for the
native hosted Intel gate.

The following records why Intel was temporarily release-only while
`macos-13` was the only available label; it is retained as historical failure
context and is not current instruction.

**What changed.** On 2026-04-28 the `macos-13` matrix entries were removed
from both the `test` and `build` jobs in `.github/workflows/ci.yml`. CI no
longer exercises Intel macOS x86_64. The Intel binary continues to ship
from tag-push releases — `[workspace.metadata.dist].targets` in
`Cargo.toml` still lists `x86_64-apple-darwin`, and cargo-dist's
`release.yml` still builds it on a `macos-13` runner at every `vX.Y.Z`
tag. CI on every commit no longer touches that runner.

**The triggering symptom.** The five CI runs immediately preceding this
change all stalled exclusively on the two Intel macOS jobs while every
other matrix cell finished within minutes. Concrete examples (run IDs +
queue times before manual cancellation): `#25023039347` ("fix(audit):
migrate users → uzers") sat queued **3h 20m+** before being cancelled to
unblock the next push, `#25022743655` (v3.11.1 release) cancelled at 7m
59s, `#25021879374` (docs commit) cancelled at 22m 34s, `#25021109459`
(v3.11.0 release) cancelled at 18m 41s, and `#24978816648` (v3.10.0
release) sat for **15h 50m** before cancellation. The repeated workflow
was: push → wait → realise Intel never picked up → `gh run cancel ...` →
push next commit, which the `concurrency: cancel-in-progress: true`
group then auto-cancelled anyway. Hours of latency per push, no Intel
runtime ever exercised.

**Why it's structural, not a glitch.** `macos-13` is GitHub's last public
Intel x86_64 macOS hosted runner label. There is no `macos-14`,
`macos-15`, or `macos-latest` Intel variant — Apple Silicon is the only
forward path on hosted runners. GitHub has been progressively winding
down the Intel hosted-fleet capacity through 2025 as Apple Silicon
became default; on top of that thin baseline, transient incidents like
the 2026-04-27 16:31 UTC "Actions experiencing degraded performance"
event push queue depth from "slow" to "indefinite." This is not a
problem we can wait out — capacity is not coming back.

**Why dropping CI coverage is acceptable for this project.** Apple
stopped selling Intel Macs in June 2023; the newest hardware Apple
shipped with an Intel CPU (the 2019 Mac Pro / 2020 Intel iMac /
MacBook Pro) is roughly three years old by 2026-04-28 and falling out
of macOS support tiers release-over-release. Critically, dropping Intel
*CI* does not mean dropping Intel *correctness coverage*: the
`#[cfg(target_os = "macos")]` gates in `src/collectors/platform/macos.rs`
are arch-agnostic, so Apple Silicon CI exercises every line of the
macOS path. The only thing ARM CI doesn't catch is genuinely
arch-specific behavior, of which TR-300's macOS path has effectively
none — no inline asm, `format_bytes()` and the table renderer are
arch-agnostic, sysinfo's `System` API hides arch internally, and the
`sysctl`/`scutil`/`pmset`/`ioreg` subprocess calls produce identical
output regardless of CPU. The accuracy delta from losing Intel CI
coverage is close to zero in practice, and any drift would show up at
tag-push time when cargo-dist builds the Intel target anyway.

**Why not just `continue-on-error: true` on the Intel matrix entry.**
Considered. Rejected because it leaves the queued-3-hours-and-then-
cancel UX intact for every push: the workflow's overall conclusion
would be `success`, but the dashboard would still show two perpetually
pending cells, and the run wouldn't be considered "complete" until
either the runner finally picked up (rare) or `cancel-in-progress`
killed it on the next push. That's theatrical coverage — the user
experience is identical to the current pain. Hard removal is the only
fix that actually changes the dashboard state.

**Why not also drop Intel from cargo-dist's release targets.**
Considered. Rejected per maintainer direction. Release-time builds run
on a tag push (cadence: minor/patch releases, ~weekly to monthly), and
that cadence is willing to absorb a multi-hour Intel queue wait — we
don't tag releases on a deadline. The cost of keeping
`x86_64-apple-darwin` in `[workspace.metadata.dist].targets` is one
extra `macos-13` job per *tag*, not per *commit*. Any user still on
2019/2020-era Intel hardware deserves a working binary download
without having to build from source. The contract is: **CI never
blocks on Intel; releases still produce the artifact.**

**What this implies for future contributors.**

1. Don't re-add `macos-13` to `ci.yml` without a concrete reason and a
   discussion of capacity risk. The default state is "Intel is not in
   CI, period."
2. If GitHub ever ships a hosted Intel macOS replacement label
   (unlikely), prefer that label over `macos-13` and re-evaluate
   whether re-adding to CI makes sense at that point.
3. If `release.yml` starts taking longer than ~2 hours at tag time
   because of `macos-13` queue depth, *that* is the signal to revisit
   dropping `x86_64-apple-darwin` from cargo-dist's targets. Until
   then, releases tolerate the wait.
4. If a macOS-arch-specific bug is reported by an Intel user, reproduce
   locally on Intel hardware (or via a one-shot self-hosted runner)
   rather than re-introducing CI coverage. The bug-rate doesn't justify
   the queue cost.

**Why "Builds 6 targets" in `## Release Process` still says 6.** Six
binary targets continue to ship from `release.yml`: Windows x64, macOS
Intel (x86_64), macOS ARM (aarch64), Linux x64 glibc, Linux x64 musl,
Linux ARM64. CI tests three of them (Linux x64 glibc, macOS ARM,
Windows x64). The mismatch between *tested* and *shipped* platforms
is intentional and is exactly what this section documents — see above.

---

## Windows accuracy patterns

### v3.11.0+ — registry OS, IsWow64Process2, WTS last-login, CPUID hypervisor, BitLocker

- **OS detection** reads `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`
  directly (`get_os_info_from_registry`) and overrides sysinfo. Detects Win11
  by `CurrentBuild >= 22000` because the registry `ProductName` is frozen at
  "Windows 10". Adds `DisplayVersion` (release ID like `25H2`) and `UBR` to the
  kernel string for richer output.
- **Architecture detection** (`get_architecture`) calls `IsWow64Process2` via a
  manual `extern "system"` linked against `kernel32`. Returns the host's
  native machine even when the binary itself runs under emulation. Handles
  `IMAGE_FILE_MACHINE_AMD64`, `_ARM64`, `_I386`, `_ARM`. Annotates emulation
  in the form `aarch64 (x86_64 emulation)` when process arch ≠ host arch.
- **CPU frequency** (`cpu.rs::collect`) combines CPUID leaf 16h + Windows
  `CallNtPowerInformation(ProcessorInformation)` + sysinfo, using
  `Iterator::max`. Leaf 16h returns 0 on Intel hybrid (Meteor/Lunar/Arrow Lake)
  — that's a documented Intel microcode change, not a bug. Falls through to
  the next source.
- **Hypervisor detection** (`detect_virtualization_wmi`) calls
  `cpuid_hypervisor_brand()` first (CPUID leaf 0x40000000, 12-byte vendor
  string) and disambiguates the Win11 VBS edge case: if CPUID returns
  `Microsoft Hv` but the SMBIOS manufacturer is a normal OEM (not Microsoft
  Corp), the result is `Bare Metal (Hyper-V/VBS)` instead of `Hyper-V`. Real
  Hyper-V VMs always have Microsoft Corp as manufacturer.
- **Last-login** (`get_last_login_windows`) calls `WTSQuerySessionInformation`
  via a manual `extern "system"` linked against `wtsapi32` (the constants
  `WTS_CURRENT_SESSION = 0xFFFFFFFF`, `WTSLogonTime = 17`,
  `WTSConnectTime = 14` are declared inline). Falls back to a boot-time
  derivation from `GetTickCount64` because Windows leaves the WTS time fields
  at 0 for local console sessions on most modern installs (auto-login + Fast
  Startup mask the actual logon timestamp). The previous `net user`-based
  parsing returned localized strings and "Never" — gone.
- **BitLocker** (`get_bitlocker_status`) queries `Win32_EncryptableVolume` in
  the `ROOT\CIMV2\Security\MicrosoftVolumeEncryption` namespace via the `wmi`
  crate's `WMIConnection::with_namespace_path`. Try-and-degrade pattern: on
  modern Win11 Device Encryption hosts this is readable non-admin and the
  `ENCRYPTION` row renders; on older Win10 / domain configurations the WMI
  call returns access-denied → `None` and the row is gracefully omitted; the
  elevation footer hint covers the unelevated case.

### v3.12.0+ — VPN-aware default-route, Fast Startup uptime annotation

- **VPN-aware default-route detection** (`get_best_route_interface_index` +
  `get_network_info_wmi`). The historical Windows network collector queried
  `Win32_NetworkAdapterConfiguration WHERE IPEnabled = TRUE` and picked the
  first IPv4 address it found — coin-flip behavior on multi-homed hosts and
  on hosts with active VPN tunnels (Tailscale, WireGuard, OpenVPN, Cisco
  AnyConnect). v3.12.0 calls `GetBestInterfaceEx` (manual `extern` linked
  against `iphlpapi`) for `1.1.1.1`; the kernel returns the interface index
  that *would* carry packets to the public internet right now. We then
  reorder the WMI result list so that adapter is picked first, falling back
  transparently to the original first-match logic when the kernel lookup
  fails (IP Helper service disabled, no default route, etc.). The `winapi`
  features `iphlpapi`/`ws2def`/`ws2ipdef`/`winerror`/`inaddr`/`in6addr`/
  `ifdef` were added for this — `SOCKADDR_IN` is declared inline (3 fields,
  layout-stable since Win95) to keep the surface area minimal. Reference:
  [GetBestInterfaceEx](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getbestinterfaceex).
- **Fast Startup uptime annotation** (`detect_fast_startup` +
  `last_cold_boot_seconds` in `platform/windows.rs`, threaded through
  `OsInfo.session_uptime_seconds` and `SystemInfo.uptime_formatted()`).
  Windows 10/11 default to `HiberbootEnabled = 1` — at "shut down" the
  kernel session is hibernated to `hiberfil.sys` and resumed at "boot",
  so `GetTickCount64` (sysinfo's uptime) reports the resumed-session age,
  while `Win32_OperatingSystem.LastBootUpTime` reports the actual cold-boot
  time. They diverge by days on a daily-use laptop. The collector probes
  the registry key `HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\
  Power\HiberbootEnabled` (DWORD), and when enabled AND the WMI cold-boot
  time is >1h older than the kernel session age, swaps `uptime_seconds` to
  the cold-boot value AND populates `session_uptime_seconds` with the
  resumed-session value. The renderer surfaces both as
  `9d 4h 12m (session: 7h 14m)`. Both values are right; surfacing both
  avoids the "wait, I restarted three days ago, why does this say 47 days"
  confusion. The WMI cold-boot query is full-mode-only (~80 ms cost);
  `--fast` uses sysinfo's uptime exclusively. The CIM datetime field is
  deserialized via `wmi::WMIDateTime` (the `wmi` crate's serde-aware
  wrapper around `chrono::DateTime<FixedOffset>`) — manual hand-parsing of
  the `yyyymmddHHMMSS.mmmmmmsUUU` format is unnecessary and was tried-
  and-discarded in early v3.12.0 development. References:
  [Microsoft Q&A: Fast Startup boot time](https://learn.microsoft.com/en-us/answers/questions/1443763/how-to-get-oss-start-time-when-fast-startup-mode-i),
  [Win32_OperatingSystem](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-operatingsystem).

### v3.13.0+ — 5-state battery, native cores, GPU registry-prefer, PSCore detection, terminal walk

- **5-state battery awareness via `GetSystemPowerStatus`** (`get_battery_native`
  in `platform/windows.rs`). Replaces the WMI `Win32_Battery` query (~40 ms,
  COM round-trip) with a single Win32 API call (~1 ms). The output state
  machine layered on top is the v3.13.0 user-visible improvement and was
  expanded mid-implementation (originally a 3-state plan; the user requested
  better gaming-laptop awareness):
    1. `BatteryFlag == 0x80` (no system battery — desktops) → `None`, BATTERY
       row omitted entirely.
    2. `BatteryLifePercent == 0xFF` (unknown charge) → `None`.
    3. `BatteryFlag == 0xFF` (unknown state) → `X% (Unknown)` so the user
       sees something rather than nothing.
    4. `ACLineStatus == 0xFF` (AC status unknown — rare; some VMs,
       hypervisor-passthrough batteries) → `X%` with no AC label, since
       guessing would be misleading.
    5. On AC + Charging bit set → `X% (Charging)`.
    6. On AC + percent ≥ 95 + not charging → `AC Power` with NO percentage.
       The percentage at full charge is uninformative; suppressing it
       eliminates the v3.11.x output `100% (AC Power)` which was redundant.
    7. On AC + percent < 95 + not charging → `X% (Plugged in)`. Covers two
       distinct but indistinguishable scenarios from a single-snapshot API:
       (a) gaming laptop where peak GPU draw exceeds the AC brick's
       wattage (Alienware, ROG, Razer with discrete GPUs); (b) firmware-
       limited charging (ThinkPad / ASUS / Lenovo battery-longevity modes
       capping at 60-80%). Either way, the percentage matters and we can't
       distinguish without time-series sampling — the label is honest about
       the ambiguity.
    8. Off AC + Critical bit → `X% (Critical)`; Low bit → `X% (Low)`;
       otherwise → `X% (Discharging)`. Critical/Low take precedence so
       the urgency is visible.
  The `BATTERY_FLAG_HIGH` bit (0x01, > 66% charge) is intentionally NOT
  surfaced as a label suffix — early v3.13.0 testing on a fully-charged
  laptop on AC produced `100% (Discharging (High))` which was incoherent
  (charging bit OFF + ACLineStatus=1 + High set is the "fully topped up
  on AC" case, not "discharging"). The percentage already conveys charge
  level. Reference:
  [GetSystemPowerStatus](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getsystempowerstatus).
- **Native socket count via `GetLogicalProcessorInformationEx`**
  (`get_socket_count_native` in `platform/windows.rs`). Replaces the
  `Win32_Processor` WMI count (~30 ms COM round-trip) with the standard
  two-call buffer-sizing pattern: first call with `Buffer = null_mut`
  returns FALSE + `ERROR_INSUFFICIENT_BUFFER` and writes
  `returned_length`; allocate exactly that size and call again. Walks the
  resulting variable-length `SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX`
  records by reading the `Size` field at each entry's offset (the records
  aren't fixed-size — different `Relationship` values pack different
  payloads). Counts entries with `Relationship == RelationProcessorPackage`.
  WMI fallback (`get_socket_count_wmi`) retained as a safety net during
  the C.9 → C.14 transition. Reference:
  [GetLogicalProcessorInformationEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex).
- **GPU enumeration prefers the registry path** (`filter_software_gpus` +
  the existing `get_gpus_fast` registry walk). The `--fast` mode already
  used the `{4d36e968-e325-11ce-bfc1-08002be10318}` Display class registry
  walk because it's COM-free and fast (~5 ms), and it has the additional
  property that it only enumerates hardware adapters — Microsoft Basic
  Render Driver, Microsoft Hyper-V Video, and similar software-only
  adapters don't appear there. Full mode now also prefers it (with WMI /
  PowerShell as fallbacks), and a `filter_software_gpus()` name-based
  filter strips known-bad strings as belt-and-suspenders. This is a
  deliberate simplification of the originally-planned C.8 (full DXGI
  `IDXGIFactory1::EnumAdapters1` COM enumeration) — the simpler approach
  achieves the same user-visible outcome (no software adapters in the GPU
  row) at 25 LOC vs ~100 LOC of unsafe COM. DXGI remains an option if
  vendor/device-ID filtering ever becomes necessary that name-based
  filtering can't do.
- **PowerShell 7+ ("PowerShell Core") detection** (`get_powershell_core_version`
  in `platform/windows.rs`). The historical `get_shell()` only knew about
  Windows PowerShell 5.x via `HKLM\SOFTWARE\Microsoft\PowerShell\3\
  PowerShellEngine\PowerShellVersion`. PowerShell 7+ installs register
  under a different hive: `HKLM\SOFTWARE\Microsoft\PowerShellCore\
  InstalledVersions\<GUID>\SemanticVersion`. We do a recursive
  `reg query /s` and pick the highest `SemanticVersion` value found by
  string-compare (works for 3-tuple semver since each component fits in
  2 digits — no zero-padding needed). Falls back to the legacy 5.x
  detection when no PSCore subkey exists.
- **Terminal parent-process walk via Toolhelp32**
  (`detect_terminal_via_parent_walk` + `match_terminal_name` in
  `platform/windows.rs`). When neither `WT_SESSION` nor `TERM_PROGRAM`
  nor the new Cursor env vars are set (common when launched from a
  desktop shortcut, a fresh subshell that lost the parent's environment,
  or by an AI agent), `get_terminal()` walks the parent-process chain:
  `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)`, then iterate via
  `Process32FirstW` / `Process32NextW`, build a
  `HashMap<pid, (parent_pid, name)>`, then climb from `GetCurrentProcessId()`
  upward (cap 10 levels — defensive against PID-table cycles, which
  happen in practice on Windows when a parent dies and its PID gets
  recycled). Recognizes Windows Terminal, WezTerm, Alacritty, VS Code,
  Cursor, Windsurf, Hyper, Tabby, Ghostty, Kitty, MinTTY, Claude Code,
  Antigravity. Intermediate hosts (`conhost.exe`, `powershell.exe`,
  `pwsh.exe`, `cmd.exe`, `bash.exe`, `sh.exe`, `zsh.exe`, `fish.exe`,
  `nu.exe`, `tr300.exe`, `node.exe`, `python.exe`) are skipped so the
  walk continues through them; unrecognized exes break the walk silently
  and the row falls back to `Console`. Verified on this dev host: when
  TR-300 runs inside Claude Code's Bash tool, the chain
  `tr300.exe → bash.exe → claude.exe → powershell.exe` correctly
  resolves to `Claude Code`. Reference:
  [CreateToolhelp32Snapshot](https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/nf-tlhelp32-createtoolhelp32snapshot).

### v4.1.0 — Native Windows hybrid-core topology

Windows hybrid-core reporting uses
`GetLogicalProcessorInformationEx(RelationProcessorCore)` and parses the
variable-sized processor-core records with byte-bounded iteration. The highest
observed nonzero `EfficiencyClass` is treated as performance cores and lower
classes as efficiency cores. If the firmware/OS does not expose useful classes,
the optional topology remains absent rather than inferring P/E counts from
logical processor totals or CPU marketing names.

This replaces a gap exposed on the Alienware m16 R2: Windows correctly reported
16 physical and 22 logical cores, but the older collector could not render the
Core Ultra 7 155H's `6P + 10E` topology. The same native query is cheap enough
for full and fast modes and avoids PowerShell/WMI startup overhead. Record-size
bounds and zero-size guards are load-bearing because the API returns a packed
buffer rather than a Rust slice.

### v3.14.4+ — Windows install execution-policy preflight

**Failure mode this prevents.** On a fresh Windows machine, `tr300 install`
wrote a valid alias-plus-auto-run block to
`Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1`, but the next
PowerShell session immediately errored:

```
File C:\Users\<user>\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1
cannot be loaded because running scripts is disabled on this system.
    + FullyQualifiedErrorId : UnauthorizedAccess
```

Windows Client defaults `ExecutionPolicy` to `Restricted` at both
`CurrentUser` and `LocalMachine` scopes — and when every scope is
`Undefined`, scope-precedence resolves to `Restricted` too. `Restricted`
blocks every `.ps1` file including `$PROFILE` itself, so the auto-run never
fires on a freshly installed system. The user's prompt-driven `tr300`
invocations still worked because `tr300.exe` is a native binary on PATH;
execution policy governs `.ps1` only. The installer was silently failing
its contract ("auto-run on new interactive shells") because it never
inspected or adjusted execution policy.

**The fix.** `src/install/windows.rs::run_execution_policy_preflight()`
runs before the profile write. It calls `Get-ExecutionPolicy -Scope
CurrentUser`, classifies the result via `policy_state()` into one of
`{BlockedDefault, BlockedAllSigned, Permissive, Unknown}`, and only acts
on `BlockedDefault` (`Restricted` / `Undefined`). For that case it runs
`Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
-Force`, then re-reads the policy to confirm the change took. Three
outcome branches:

1. **Succeeded** — prints a one-line "Set PowerShell CurrentUser execution
   policy: <previous> -> RemoteSigned" followed by the "(required to load
   $PROFILE; only your account, no admin needed)" line, then proceeds with
   the alias write. Verified on the user's exact failure-mode machine.
2. **Set-ExecutionPolicy returned 0 but the policy is still blocking**
   (`TrySetResult::StillBlocked`) — a higher-precedence `MachinePolicy` or
   `UserPolicy` Group Policy wins scope precedence over `CurrentUser`.
   Prints a fallback warning to stderr explaining what we tried, what the
   effective policy still is, and a `LocalMachine`-scope remediation. The
   alias write half still succeeds; only the auto-run UX is degraded.
3. **Set-ExecutionPolicy itself failed** — print the same fallback
   warning with the underlying error message. Same non-fatal posture.

The `BlockedAllSigned` branch never touches `Set-ExecutionPolicy`. It
prints a notice explaining `AllSigned` blocks the unsigned auto-run
snippet, offers the user the choice between signing the block themselves
or relaxing to `RemoteSigned`, and proceeds with the alias write. The
policy is left alone.

**Why `RemoteSigned` and not other policies.** PowerShell's execution
policies, strictest to most permissive:

| Policy | Loads our local profile? | Why we don't use it |
|---|---|---|
| `Restricted` | ❌ | The default — the bug we're fixing. |
| `AllSigned` | ❌ (without code-signing the snippet) | Would require shipping an Authenticode cert. Rejected — it expands deploy surface dramatically for a one-user-facing-snippet win. |
| **`RemoteSigned`** | ✅ | **Minimum policy that loads a local unsigned profile.** Downloaded scripts must still be Authenticode-signed. What we use. |
| `Unrestricted` | ✅ | Loads downloaded scripts with a confirmation prompt. Strictly more permissive than needed. Rejected. |
| `Bypass` | ✅ | Loads everything silently with no warnings. Strictly more permissive than needed. Rejected. |

`RemoteSigned` is the policy [Microsoft's own
documentation](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_execution_policies)
recommends for non-server Windows installs and the policy Scoop,
oh-my-posh, and starship all rely on. Using `Unrestricted` or persistent
`Bypass` would meet the user's explicit constraint ("not overly
permissive") with strictly less safety than `RemoteSigned` gives.

**Why `CurrentUser` scope and not `LocalMachine`.** `CurrentUser`
writes to `HKCU\SOFTWARE\Microsoft\PowerShell\1\ShellIds\Microsoft.PowerShell`
and affects only the current user. It does not require Administrator,
does not affect other accounts on the machine, and matches the rest of
`tr300`'s install footprint (which targets `%LOCALAPPDATA%\Programs\tr300`
and `$PROFILE` — both `CurrentUser` paths). `LocalMachine` would require
an elevated process and would impose our policy choice on every account
on the box; we never want that.

**Why verify after `Set-ExecutionPolicy`.** Scope precedence is
`MachinePolicy > UserPolicy > Process > CurrentUser > LocalMachine`.
Domain-managed Windows machines can set `MachinePolicy` or `UserPolicy`
via GPO, and those scopes are read-only from user processes.
`Set-ExecutionPolicy -Scope CurrentUser` succeeds (exits 0) — it really
does write the HKCU key — but the *effective* policy seen by the next
shell is still whatever GPO enforces. Re-reading
`Get-ExecutionPolicy -Scope CurrentUser` immediately after the set
confirms whether the change actually took effect under the active scope
precedence. Without this check, the `StillBlocked` path would be invisible
and the user would silently get the same `UnauthorizedAccess` error on
the next shell start.

**Why uninstall doesn't roll back execution policy.** The user's pre-install
policy is unknown by the time `tr300 uninstall` runs (we don't persist it
anywhere — that would expand the install footprint to a state file), and
other PowerShell tooling installed on the same machine — Scoop, oh-my-posh,
starship, dev module loaders — typically depends on `RemoteSigned` too.
Reverting to `Restricted` on uninstall would break those tools. Leaving
the policy alone is the least-surprising behavior. Documented in README.

**Why not write to both Windows PowerShell 5.x and PowerShell 7+
profiles.** PowerShell 7+ (`pwsh.exe`) uses
`Documents\PowerShell\Microsoft.PowerShell_profile.ps1`, separate from
WinPS 5.x's `Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1`.
The reporting user was on WinPS 5.x and the current installer only
targets WinPS 5.x via `powershell.exe -NoProfile -Command "$PROFILE"`,
so this fix unblocks their case. PowerShell 7+ profile coverage is real
gap but a separate scope expansion — tracked but not landed here.

**Rejected alternatives.**
- *Always set `Bypass`* — would meet the user's "minimum" constraint with
  strictly less safety than `RemoteSigned`. Rejected.
- *Touch `LocalMachine` instead of `CurrentUser`* — requires admin,
  affects all users on the box. Rejected.
- *Skip the preflight, document the failure in README and point users at
  `Set-ExecutionPolicy`* — every fresh-Windows user runs `tr300 install`
  and hits the failure mode the very first time; the install command's
  job is to make the tool work, not to print a workaround. Rejected.
- *Sign the auto-run snippet ourselves so `AllSigned` works too* —
  requires Authenticode cert infrastructure, ongoing key rotation, and a
  release-time signing step. Not worth it for the AllSigned user count.
- *Write a `.cmd` shim that PowerShell can `Invoke-Item` from a profile
  that's allowed under `Restricted`* — there is no such profile under
  `Restricted`. Rejected.

Reference:
[Microsoft Learn — about_Execution_Policies](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_execution_policies),
[Set-ExecutionPolicy](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.security/set-executionpolicy).

### v3.14.5+ — Windows install error advisor

**Failure mode this prevents.** Prior to v3.14.5, when `tr300 install` hit
a permissions error on a work machine — Intune-managed device, Active
Directory Group Policy lockdown, AppLocker policy, Windows Defender
Application Control (WDAC), antivirus / EDR write-block, or OneDrive
Known Folder Move with offline files — the user saw something like:

```
Error: Platform { message: "Failed to write profile: Access is denied. (os error 5)" }
```

Two things wrong with that output:

1. The Debug-format wrapper (`Platform { message: "..." }`) is `cargo
   run`-grade developer output, not user-grade output. `thiserror` has a
   perfectly good Display impl (`Platform operation failed: ...`) that
   never gets used because `fn main() -> Result<()>` calls Debug on the
   error before exiting non-zero.
2. The message has no remediation. The user is told *what* failed but
   not *why* it likely failed or *what to do next*. On corporate
   machines this is the #1 friction point in install adoption — IT
   doesn't know what to allowlist, and the user has no concrete
   "give this to IT" path.

**The fix.** Two related changes in `src/install/windows.rs` and
`src/main.rs`:

1. **`fail_install(InstallStep, &Path, io::Error) -> AppError`** in
   `src/install/windows.rs`. Every `std::fs` call in install/uninstall
   now goes through this advisor. It:
   - Streams a multi-paragraph guidance block to *stderr* —
     `step.short_description()` → `Path:` + `Cause:` (including the raw
     Windows error code) → step-and-kind-specific "Likely reasons"
     paragraph → "Manual `tr300` still works" reassurance closer.
   - Then returns a concise `AppError::platform("write profile: <io
     err>")` suitable for `main()`'s trailing summary line.
   - The rich output is on stderr so it's never swallowed by code that
     only captures the returned error, and so it interleaves correctly
     with `main()`'s `Error: ...` line.

2. **Dispatch on `(InstallStep, ErrorKind, raw_os_error, path
   inspection)`.** The advisor branches on multiple inputs to pick the
   right remediation paragraph:

   | Condition | Guidance |
   |---|---|
   | `PermissionDenied` / Windows error 5, **OneDrive path** | OneDrive-specific: ensure synced, "Always keep on this device", or ask IT to allow OneDrive-folder writes |
   | `PermissionDenied` / error 5, **redirected (UNC) path** | Network share / folder redirection: check share access, ask IT |
   | `PermissionDenied` / error 5, **plain local path** | Intune / AD GPO / AppLocker / WDAC / antivirus, with `takeown /F "..." /R` example |
   | Sharing violation (error 32) | OneDrive sync engine + EDR + open editor as causes; retry |
   | Storage full / error 112 | Free disk space |
   | Path too long / error 206 | MAX_PATH overflow; suggest LongPathsEnabled |
   | NotFound on ReadProfile | Transient race; retry |

   The OneDrive detection uses `looks_like_onedrive_path()`, a
   case-insensitive segment scan matching both `\OneDrive\` and
   `\OneDrive - <TenantName>\` (OneDrive-for-Business). False positives
   on filenames containing "onedrive" are intentional — the
   user-visible cost is just slightly off-topic advisory text, and
   keeping the predicate simple is worth more than a perfect filter.

**Why we don't put the rich guidance in the AppError message itself.**
That would force the message into a single string with embedded
newlines, which renders poorly in non-terminal consumers (CI logs,
piped to other tools) and forces a hard coupling between the error type
and the user-facing UX. Keeping the advisor as a side effect of
`fail_install()` and the returned `AppError` as a short tag means
programmatic consumers see a clean `"write profile: <io err>"` and
human consumers see the rich stderr stream.

**Why dispatch on `path` and not just `ErrorKind` + raw code.** The
same Windows error 5 has *different remediations* depending on where
the path lives: OneDrive sync state vs. corporate MDM policy vs.
locally-mounted volume with weird ACLs. A one-size-fits-all "permission
denied" paragraph would either be too vague (skip the actionable steps)
or include OneDrive-specific advice for non-OneDrive paths (confusing).
Splitting the branch on `looks_like_onedrive_path()` is the cheapest way
to put the user on the right track.

**Why we never abort install on the execution-policy preflight failure
but we *do* abort on profile-write failure.** The preflight failure is
recoverable manually (the alias write still happens; `tr300` keeps
working as a binary on PATH). The profile-write failure means the
install genuinely did not complete — there is no alias, there is no
auto-run. The user needs to know that, with concrete remediation,
rather than silently continuing.

**Rejected alternatives.**
- *Put the rich guidance into `AppError::Platform::message`* — turns
  the message into a multi-line embedded string and couples the error
  type to the rendering. Rejected.
- *Use a separate error variant per scenario (e.g. `OneDriveBlocked`,
  `IntuneBlocked`)* — would require detection logic that doesn't exist
  (we can't reliably tell Intune from AppLocker from antivirus just
  from an `io::Error`). The "Likely reasons (most common first)"
  presentation is honest about that uncertainty. Rejected.
- *Print only the short error and rely on documentation* — defeats the
  purpose. The user already has the message in front of them; the
  guidance should be there too. Rejected.
Reference:
[`std::io::ErrorKind`](https://doc.rust-lang.org/std/io/enum.ErrorKind.html),
[Windows System Error Codes](https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes),
[OneDrive Known Folder Move](https://learn.microsoft.com/en-us/onedrive/redirect-known-folders),
[Long Paths in Windows](https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation).

### Windows distribution model (v3.15.0+)

The four-product model and permanent identities remain current. The v3.15
same-edition coexistence and single legacy marker behavior below are
superseded by v4.1.0: an explicit fresh MSI/EXE removes its same-edition
counterpart, and scoped Global/Corporate marker values preserve coexisting
legacy evidence without authorizing cross-channel updates.

**Decision: ship four Windows installer artifacts at every tagged release.** Two MSIs (Global perMachine + Corporate perUser) and two Inno Setup EXE installers (Global perMachine + Corporate perUser). The MSI Global has been shipping since v3.13.1; this release adds the other three.

```
tr300-x86_64-pc-windows-msvc.msi                   (Global MSI,  perMachine, UAC)
tr300-x86_64-pc-windows-msvc-corporate.msi         (Corp.  MSI,  perUser,    no UAC)
tr300-x86_64-pc-windows-msvc-setup.exe             (Global EXE,  perMachine, UAC)
tr300-x86_64-pc-windows-msvc-corporate-setup.exe   (Corp.  EXE,  perUser,    no UAC)
```

**Problem this solves.** Before v3.15.0, Windows install required either `cargo install tr300` (which the project's homepage commands wrap with rustup + MSVC Build Tools — a ~5 GB install chain blocked by every managed-device policy I've checked) or the cargo-dist PowerShell installer `irm | iex` (broken under PowerShell `ExecutionPolicy=Restricted` and frequently blocked by AppLocker / WDAC). The only MSI we shipped was perMachine, which requires admin — exactly the cohort of users (managed corp machines) who most need a friction-free install. Result: users couldn't install on the machines that the tool was most useful on.

**Why two MSIs + two EXEs, not a single dual-purpose installer.**

There IS a "single-package authoring" pattern in WiX that produces a dual-purpose MSI which installs perUser by default and perMachine when run elevated (`ALLUSERS=2` + `MSIINSTALLPERUSER=1`). I rejected this approach for two reasons:

1. **[WiX issue #7137](https://github.com/wixtoolset/issues/issues/7137) documents the pattern is fragile in WiX v4/v5** — the upgrade-detection logic doesn't reliably find an existing install of the same product when the scope differs. Real users have reported v1.0.0 perMachine + v1.0.1 perUser ending up with both installed. The two-separate-MSIs-with-different-UpgradeCodes pattern is the industry workaround.
2. **Different UpgradeCodes are SEMANTICALLY correct.** A perMachine install (everyone on the box sees the binary) and a perUser install (only my account sees it) are different products from the operating system's perspective. Pretending they're one product creates upgrade ambiguity, ARP entry confusion, and uninstall edge cases.

Same reasoning for two EXE installers vs one with a runtime mode switch (`tr300-setup.exe --mode user` vs `--mode admin`). The runtime-switch pattern doesn't actually exist in Inno Setup — `PrivilegesRequired` is a compile-time directive. You'd need two separate `.iss` files anyway; might as well make them obviously separate.

**Why MSI + EXE for the same edition.** End users have well-established preferences. Some prefer `setup.exe` (familiar pattern from decades of Windows desktop apps); others trust `.msi` more (managed IT environments). The marginal cost of building both formats from the same prebuilt `tr300.exe` is small (~80 LOC of Inno Setup script per edition, ~30 LOC of GitHub Actions YAML). The marginal benefit is real: every user gets their preferred format and the install completes.

**Why install both formats to the same path within each edition.** Two alternatives considered:

1. **Same path, originally document "pick one"** (v3.15 choice, superseded in
   v4.1.0). MSI Global and EXE Global share
   `C:\Program Files\tr300\bin\`; Corporate formats share their per-user path.
   v4.1.0 keeps those paths but makes the newly launched installer remove the
   same-edition counterpart first, so the newest explicit format choice owns
   the binary, marker, PATH, and Add/Remove Programs registration.
2. **Distinct paths** (`C:\Program Files\tr300\bin\` for MSI, `C:\Program Files\tr300 (Setup)\bin\` for EXE). Rejected because the suffix is ugly, PATH ordering still picks a winner, and the duplicate-install scenario is rare enough that it's not worth uglifying the common case.

The v3.15 rejection of cross-installer detection is superseded. v4.1.0 uses
WiX registry search/custom action and Inno's MSI product enumeration to remove
the counterpart transactionally before installing. Failure stops the fresh
install instead of merging two registrations.

**The registry markers.** All four installers retain the legacy
`HKCU\Software\TR300\InstallSource` value and also write one scoped value:
`InstallSourceGlobal` or `InstallSourceCorporate`. Values remain
`msi-global`, `msi-corporate`, `exe-global`, or `exe-corporate`.
`src/update.rs::detect_install_origin()` validates the scoped marker against
the running path before choosing exactly one in-place update strategy.

Why HKCU and not HKLM:
- `tr300 update` always runs as the user, who reads HKCU naturally.
- Writing to HKLM from a perMachine MSI works but requires elevation-aware Component authoring (`Component KeyPath` semantics get awkward with HKLM writes from a perUser-or-perMachine wrapper).
- The marker is accepted only when its Global/Corporate scope matches the
  running path, preventing a stale HKCU value from selecting a coexisting
  install's product.

Marker-free `.cargo\bin` installs are classified only through a matching Cargo
or cargo-dist receipt, not a probe-and-retry chain. Marker-free Program Files
and LocalAppData copies recover an origin only when Add/Remove Programs has one
unambiguous matching registration; otherwise they are `Unknown`. Guessing
would create a cross-format registration, so the updater performs no mutation.

**Checksum evolution.** v3.15.0 initially trusted HTTPS alone. v3.15.4 added
SHA-256 sidecar verification before MSI/EXE launch, and later releases retained
it as a load-bearing corruption/mismatch check. The payload and sidecar share
one release origin, so this is not an independent signature and must not be
described as proof against an origin or TLS-interception compromise.

**Why `.github/workflows/windows-installers.yml` uses `workflow_run`.** The
cargo-dist workflow must create the GitHub Release before supplemental assets
can attach. A `release: published` event created with the default
`GITHUB_TOKEN` does not trigger a downstream workflow because of GitHub's loop
prevention; v3.15.1 exposed that failure. `workflow_run` on successful
completion of the named Release workflow provides the required sequencing, and
`workflow_dispatch` remains the repair path.

**Why Inno Setup, not NSIS or WiX Burn.**

- **WiX Burn** would let us produce a bundle.exe that wraps the existing MSI. Considered. Rejected because WiX 3 Burn is finicky (Bundle.wxs syntax is non-trivial, the build pipeline through `candle.exe` + `light.exe` differs from the MSI flow), and the `cargo-wix` CLI doesn't directly support Burn bundles. We'd need to drop down to raw WiX 3 tooling. Inno Setup is one `.iss` file per edition and a single `iscc` command.
- **NSIS** is also viable but its scripting language is less ergonomic than Inno's Pascal-like syntax, and the resulting installers are slightly larger.
- **Inno Setup** (chosen) is widely used in the Rust CLI ecosystem (gh CLI, deno, bun, uv all use it on Windows). Free, mature, well-documented. The `.iss` files are human-readable and self-contained. The CI install is one `choco install innosetup` line.

**Why two separate Inno Setup .iss files instead of one parameterized.** Inno Setup supports `#define` and `#if` for compile-time conditionals, so a single file with `#define GLOBAL_OR_CORPORATE` would work. Rejected because: (1) the two files differ in enough ways (PrivilegesRequired, DefaultDirName, PATH registry root, AppId, OutputBaseFilename, Code block contents) that the conditional branches would dominate the file; (2) two files is more obvious to readers; (3) the AppIds are forever-distinct, so the files can't truly share an identity anyway. Duplicate-ish code is fine when the products are genuinely separate.

**Why `install-path = "CARGO_HOME"` is still set in `[workspace.metadata.dist]`.** That setting controls where the cargo-dist PowerShell installer (`tr300-installer.ps1`) places the binary on Windows — `%USERPROFILE%\.cargo\bin\tr300.exe`. The new MSI / EXE installers don't use cargo-dist's installer logic at all; they have their own install paths defined in `wix/*.wxs` and `inno/*.iss`. Leaving the cargo-dist setting alone means the PowerShell installer keeps working for users who prefer it (or for users on platforms without an installer story, like ARM Windows once we add that target).

**SmartScreen / unsigned-binary honest accounting.** All four installers are unsigned. First-time download on Win11 24H2+ triggers "Windows protected your PC" — user clicks **More info → Run anyway**. README documents this; we don't pretend it doesn't happen.

Real options for fixing this (not done in v3.15.0):
1. **Azure Trusted Signing** (~$10/mo) — Microsoft's EV-equivalent service that gives instant SmartScreen reputation. Requires verified org identity (we'd need to register QubeTX). Likely the right answer long-term.
2. **OV code-signing cert** (~$80–200/yr) — still requires accumulated download reputation to clear SmartScreen, but reduces "Unknown publisher" friction.
3. **Document the manual override** (chosen for now) — what BleachBit, Teleport, ScreenToGif, and many other open-source Windows tools do.

**Rejected alternatives.**
- *Use a single perUser MSI that auto-detects "in admin? install perMachine. Otherwise perUser."* — see WiX #7137 above. Too unreliable.
- *Drop the perMachine MSI entirely and ship only perUser* — would break IT-managed deployment via Intune / SCCM. Real users need perMachine for managed rollouts.
- *Ship only one EXE format with a runtime `--user / --machine` switch* — not supported by Inno Setup's `PrivilegesRequired` directive. Would require building two binaries anyway.
- *Include Windows Authenticode signing in this release* — too much scope.
  Tracked as future work; this is distinct from v4's Apple Developer ID path.
- *Submit to WinGet / Scoop in this release* — high-value follow-up, but adds external dependencies (PR review processes). Defer to v3.16.0.

References:
- [WiX Toolset Issue #7137 — dual-purpose perUser MSI](https://github.com/wixtoolset/issues/issues/7137)
- [Microsoft Learn — WDAC vs AppLocker overview](https://learn.microsoft.com/en-us/windows/security/application-security/application-control/windows-defender-application-control/wdac-and-applocker-overview)
- [Microsoft Learn — about_Execution_Policies](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_execution_policies)
- [Inno Setup documentation](https://jrsoftware.org/ishelp/)
- [cargo-wix project README](https://github.com/volks73/cargo-wix)
- [cargo-dist MSI installer book](https://opensource.axo.dev/cargo-dist/book/installers/msi.html)
- [Azure Trusted Signing](https://learn.microsoft.com/en-us/azure/trusted-signing/)
- [Microsoft Learn — Submit packages to WinGet](https://learn.microsoft.com/en-us/windows/package-manager/package/)

#### v3.17.0 addendum — advisory one-install consolidation

**Status:** Accepted. This decision backfills the load-bearing rationale for
`src/migrate.rs`, which was already documented in release notes and agent
guides but was missing from this ADR ledger.

Windows supports Cargo/cargo-dist copies in `~\.cargo\bin`, a Global edition in
`%ProgramFiles%\tr300\bin`, and a Corporate edition in
`%LocalAppData%\Programs\tr300\bin`. A user can therefore install a new MSI/EXE
successfully and still run an older Cargo copy because `.cargo\bin` appears
earlier on PATH. Global and Corporate editions can also coexist. The updater's
origin marker prevents guessing the wrong installer for an in-place update, but
it cannot by itself remove a shadowing executable.

The operator policy is one active TR-300 executable/edition at a time. The four
Windows installers therefore own an **advisory** post-install consolidation
step through hidden `tr300 migrate-cleanup`:

- interactive installs expose “remove older Cargo copy” and “remove other
  edition” tasks, both defaulted on;
- silent self-updates invoke the same cleanup without a prompt, also defaulted
  on;
- `--cargo-copy` targets only `~\.cargo\bin\tr300.exe`;
- `--other-edition` selects the opposite Global/Corporate binary directory;
- with neither target flag, the command defaults only to the safer Cargo-copy
  cleanup;
- `--dry-run`, `--json`, `--quiet`, `--user-profile`, and `--cargo-home` exist
  for installer integration, diagnosis, and deterministic resolution;
- macOS/Linux return a clean no-op because their supported methods converge on
  the same Cargo-home binary location rather than separate editions.

The command removes a shadowing executable, not an entire toolchain, PATH entry,
Downloads directory, or Add/Remove Programs registration. A stale installer
registration can be repaired with that product's normal uninstaller; deleting
unrelated registry/product state inside a post-install helper would broaden the
risk substantially.

Hard safety invariants bound every deletion:

1. The filename allowlist contains only `tr300`/`tr300.exe`; `cargo.exe`,
   `rustup.exe`, sibling tools, directories, and arbitrary files cannot match.
2. The running executable's canonicalized directory is always excluded. A
   Cargo-installed process cannot delete itself just because Cargo cleanup was
   requested.
3. Candidate paths are computed only from Cargo home/user profile or the two
   edition roots. No candidate is under Downloads.
4. The helper never edits PATH and never removes `.cargo\bin` itself.
5. It never elevates. A per-user process that cannot remove a per-machine copy
   reports `needs admin`, leaves it intact, and continues.
6. Empty, absent, partial, or needs-admin cleanup remains exit 0 so advisory
   consolidation cannot roll back or falsely fail a successful installer. Only
   a true internal inability to establish the running location is nonzero.

Installer context is part of correctness. WiX custom actions are deferred and
`Impersonate='yes'` so user-profile cleanup runs as the invoking user, and they
use `FileKey`/`ExeCommand` without adding `WixUtilExtension`. Inno Corporate can
pass the user profile directly. Inno Global intentionally relies on process
environment fallback because it has no reliable pre-elevation user-profile
constant for this purpose. Cargo-home and user-profile overrides win over the
helper's environment when available.

The edition directory constants and registry marker strings form one lockstep
across `src/migrate.rs`, `src/update.rs`, `wix/main.wxs`,
`wix-corporate/corporate.wxs`, and both Inno scripts. Changing an install path
in only one surface can make cleanup target the wrong edition or make updater
origin validation reject a legitimate marker.

**Rejected alternatives:**

- **Let PATH precedence decide forever.** Installation can report success while
  the user continues running the old binary, which is a false update outcome.
- **Delete the entire Cargo bin directory or remove it from PATH.** It commonly
  contains cargo, rustup, and unrelated Rust tools; TR-300 has no authority to
  mutate them.
- **Run cleanup automatically on every ordinary report.** Reports are read-only
  by v4 contract, and deleting another install outside explicit installer
  context would be surprising and endpoint-policy-hostile.
- **Make cleanup failure fail the installer.** A Corporate user cannot normally
  remove a Global per-machine copy. The new working install should survive and
  report the remaining conflict rather than be rolled back.
- **Elevate automatically to guarantee removal.** That violates the Corporate
  no-admin purpose and turns a bounded advisory helper into a privilege path.
- **Guess MSI versus EXE from the install directory.** Both formats share an
  edition path; the `InstallSource` marker is required for product identity.
- **Apply the Windows edition model to macOS/Linux.** Those platforms have no
  Global/Corporate split; a no-op preserves the single cross-platform CLI
  contract without inventing foreign paths.

**Consequences and revalidation:** all four installer interactive and silent
paths must exercise the same target defaults; pure path/allowlist/outcome tests
must run on every OS, and live deletion/needs-admin/PATH-shadow tests remain
part of the personal Alienware matrix. Any Windows install path, marker,
product scope, custom-action impersonation, binary name, or additional shipped
binary requires a lockstep migration review. The helper must remain hidden,
advisory, and incapable of deleting the running install.

### v3.15.1 addendum — Why corporate.wxs lives in `wix-corporate/`, not `wix/`

v3.15.0's `release.yml` run 25901237669 failed at `build-local-artifacts(x86_64-pc-windows-msvc)` with WiX candle exit code 6. v3.15.0 published to crates.io (it was past the crates-publish stage by then per /release § 10's "tag only after both ci.yml and crates-publish are green") but had no GitHub Release artifacts. v3.15.1 is the fix-forward patch release. Two root causes:

**Root cause 1: `<Property Id='ALLUSERS' Value=''/>` in v3.15.0 `wix/corporate.wxs`.** I'd added this Property because folklore and old WiX 3 examples claim that explicit `ALLUSERS=""` + `MSIINSTALLPERUSER=1` are needed to force a per-user install. WiX 3.11 candle rejects `Value=""` with `CNDL0006: The Property/@Value attribute's value cannot be an empty string` and emits a follow-up `CNDL1006` warning explaining the Property would be ignored anyway. The actual WiX 3 rule: `InstallScope='perUser'` on the Package element is sufficient by itself. The MSI installer treats unset `ALLUSERS` as per-user, which is exactly what we want; declaring it with an empty value is both syntactically rejected by candle AND, even if it parsed, semantically equivalent to not declaring it at all. The v3.15.0 → v3.15.1 fix was to delete both that line and the redundant `MSIINSTALLPERUSER=1` Property.

**Root cause 2: cargo-wix's "compile every `wix/*.wxs` into ONE MSI" default.** Once root cause 1 was fixed, light.exe surfaced `LGHT0089` ("Multiple entry sections '*' and '*'") and a cascade of `LGHT0091/0092` ("Duplicate symbol") errors at the link stage — for `Property:Manufacturer`, `Property:ProductCode`, `Property:ProductLanguage`, `Property:ProductName`, `Property:ProductVersion`, `Property:UpgradeCode`, `Property:DiskPrompt`, `Media:1`, `Directory:APPLICATIONFOLDER`, `Component:binary0`, `Component:Path`, `Component:InstallSourceMarker`. cargo-wix scans `wix/*.wxs` non-recursively and feeds ALL matching files to candle → produces one wixobj per .wxs → light.exe links them ALL into a single output. Two complete Product definitions in the same directory cannot coexist this way; they conflict at every shared symbol.

The fix has two parts: (a) move `corporate.wxs` to a NEW directory `wix-corporate/` so cargo-wix's default scan of `wix/` produces only the Global MSI cleanly. (b) Build the Corporate MSI separately by calling bare `candle.exe` + `light.exe` from `windows-installers.yml`, never through cargo-wix. cargo-wix CLI has `--include <path>` to ADD files but no equivalent "use ONLY this file" flag, so cargo-wix is the wrong tool for compiling one wxs from outside `wix/`.

The bare WiX invocation needs `-sice:ICE38 -sice:ICE64 -sice:ICE91` flags on `light.exe`. Per-user MSIs in WiX 3 want:
- ICE38: every Component installing to a user-profile directory should have an HKCU RegistryValue as its KeyPath (not a File KeyPath).
- ICE64: every Directory in the user profile needs a Component with a `RemoveFolder` element so the directory is cleaned up on uninstall.
- ICE91: WiX warns that per-user files in `[Bin]` won't replicate to other users' profiles even if perMachine was somehow desired (cosmetic).

Making `wix-corporate/corporate.wxs` ICE-clean is a real option, requiring ~5 additional `<Component>` elements per intermediate Directory (each with `<RemoveFolder>` + an `<RegistryValue Root='HKCU'>` KeyPath dummy), changing the existing `Path` and `binary0` Components to add HKCU RegistryValue KeyPaths instead of relying on Environment/File KeyPaths, plus matching `<ComponentRef>` entries in the Feature tree — roughly 40 lines of new WiX boilerplate. Rejected for v3.15.1: the practical impact of suppressing the three ICEs is one empty `%LocalAppData%\Programs\tr300\bin\` folder left in the user profile after uninstall. Install and uninstall both work correctly; PATH modification works; the binary is fully functional. The empty folder is a cosmetic leftover that's acceptable for a CLI tool.

**Verification path used to diagnose this.** Local repro with portable WiX 3.11 binaries from [github.com/wixtoolset/wix3/releases](https://github.com/wixtoolset/wix3/releases/tag/wix3112rtm) (downloaded as `wix311-binaries.zip`, extracted to `/tmp/wix3/`). With cargo-wix installed via `cargo install cargo-wix`, running `cargo wix --no-build --nocapture` against v3.15.0's source surfaced both error classes in sequence — first the CNDL0006 from candle, then (after the Property fix) the LGHT0089/0091 cascade from light.exe trying to link both wixobjs together. cargo-dist's CI invocation suppresses these details by default (it captures candle/light stderr); local reproduction with `--nocapture` was essential.

**Rejected alternative for root cause 2: in-tree wxs file rename.** I considered renaming `wix/corporate.wxs` to `wix/corporate.wxs.bak` in the windows-installers.yml workflow before calling cargo-wix, then restoring. Worked but ugly — the file would be physically renamed at CI runtime, which complicates local builds and confuses git. The directory move (`wix-corporate/`) is structural and explicit.

**Rejected alternative for root cause 1: keep the ALLUSERS Property with a non-empty Value.** WiX docs allow `Value=" "` (single space) as a workaround for the empty-string rule. Tried in an earlier draft; it passes candle but generates a meaningless ALLUSERS Property that doesn't actually affect install scope. The clean answer is to delete the Property entirely.

**Side effect: `Cargo.toml` `include` list gained `/wix-corporate/**`** so the published crate ships with the Corporate WiX source. This is for completeness — the practical Corporate MSI build happens in CI from the Git repo, not from a downloaded crate.

---

## Install / update safety primitives (v3.15.2+)

The v3.15.2 audit + remediation cycle surfaced six related-but-distinct
"silent failure" classes across the install / update / runtime paths.
Each shipped as a focused primitive rather than a one-off fix, with the
explicit intent that future Claude agents see the pattern and reuse it.

### Atomic rc-file writes

**Problem.** `std::fs::write(path, content)` opens the target with
`O_TRUNC` / `CREATE_ALWAYS` (POSIX / Windows respectively) and then
writes. If the process dies between truncate and write completion — `Ctrl-C`
during the install, a power loss, an antivirus quarantining the file
mid-write, OneDrive deciding to rename the file during the open — the
target file is left **truncated or partial**. For files the user has
invested real time in (`~/.bashrc`, `~/.zshrc`, PowerShell `$PROFILE`),
that loss is silent and irrecoverable.

**Fix.** `src/install/mod.rs::atomic_write(path, content)`:
1. Write to a sibling temp file (`.<filename>.tr300-tmp`) in the same
   parent directory.
2. `sync_all()` to flush to disk.
3. `std::fs::rename(temp, target)` — atomic on POSIX, atomic on NTFS
   within a volume.

The temp file in the same parent dir is load-bearing: NTFS guarantees
atomicity for `MoveFileExW(MOVEFILE_REPLACE_EXISTING)` only when both
endpoints are on the same volume, and POSIX `rename(2)` has the same
constraint. Using `std::env::temp_dir()` for the temp would have placed
it on a different filesystem in the common case and silently degraded
to non-atomic copy-then-delete.

On any failure path, the temp file is cleaned up with a best-effort
`fs::remove_file` so we don't leak orphan `.<filename>.tr300-tmp` files.

**Rejected alternative: `tempfile` crate.** The `tempfile::NamedTempFile`
API does roughly the same dance but with a heavier API surface and an
additional dependency. For three call sites + a couple test fixtures,
the hand-rolled ~40 LOC helper wins on transparency and zero added
deps.

**Rejected alternative: write to `$TEMP` first, then copy.** Works but
isn't atomic — the copy-then-delete sequence has the same partial-write
hazard we were trying to avoid, just in a different file. Same-parent
temp is the only path that gets actual atomicity.

**Companion: `backup_once(path)`.** First install also copies the
pre-TR-300 contents to `<path>.tr300-backup` if no backup exists yet.
Idempotent: subsequent installs preserve the ORIGINAL backup. We never
overwrite a backup with a TR-300-modified version — that would silently
destroy the user's last good copy.

### Marker-balance pre-check

**Problem.** The `remove_delimited_block` parser (now in
`src/install/shared.rs`) opens a block on any line containing
`# TR-300 Machine Report` and closes it on any line containing
`# End TR-300`. If a user hand-edited `# End TR-300` out of their rc
file — a real and plausible failure mode when tidying shell config —
the parser silently sets `in_block = true` on the start marker, never
sees the close, and drops every subsequent line through EOF.

The user's `~/.bashrc`, gone, on the next `tr300 install`.

**Fix.** `check_marker_balance(content, start, end)` counts lines
containing each marker. If `count(start) != count(end)`, refuse the
write up-front with an actionable error explaining how to repair the
block by hand. All four call sites (`update_shell_profile`,
`remove_from_profile`, Windows `install_into_profile`, Windows
`uninstall`) call this before any state mutation.

**Why count instead of "in_block at EOF" check.** Counting also catches
the inverse case (an orphan `# End TR-300` line on its own — less common
but possible). And it's an O(n) single-pass over the file we already
have in memory; no performance concern.

### SHA256 verification of downloaded installers

**Problem.** Pre-v3.15.2 the Windows update path (`src/update.rs`)
trusted only TLS to `github.com` and immediately handed the downloaded
MSI / EXE bytes to `msiexec /i` or ran the Inno EXE directly. The
implicit comment was "we trust the cert chain" — but TR-300's target
audience is corporate users on machines with TLS-interception proxies
(Bluecoat, Zscaler, McAfee Web Gateway, etc.) installing a trusted
root certificate in the user store. Those proxies see plaintext
HTTPS traffic and can rewrite installer bytes in flight.

The worst case is the Global EXE: Inno Setup binaries declare
`PrivilegesRequired=admin` at launch, so one UAC click on a tampered
installer gives the attacker SYSTEM-equivalent code execution on
every machine that auto-updates.

**Fix.** After `download_to_file`, fetch `{installer_url}.sha256` in a
SEPARATE request, parse the cargo-dist `<lowercase-64-char-hex>  *<filename>`
format (also tolerant of the asterisk being absent — some sha256sum
invocations emit text mode), compute SHA-256 of the downloaded file
via `sha2::Sha256`, and refuse to launch on mismatch.

**Why this is meaningful uplift even though both requests go through
the same MITM.** The proxy has to tamper with BOTH the installer AND
the sidecar in a way that yields a matching hash. SHA-256 is
preimage-resistant — you can't compute new bytes that hash to a
specific target value. The attacker would have to either:
1. Use a hash collision (currently believed-hard for SHA-256 — no
   published collision attack), or
2. Rewrite both files in a way that doesn't trigger TLS cert
   validation on the second connection (rustls won't downgrade or
   ignore cert errors silently), or
3. Forge the trusted root and serve their own cert (already in scope
   if they're MITM-ing, but now they have to do it consistently for
   two requests instead of one — much higher likelihood of triggering
   the user's other security tooling).

This isn't code-signing protection (we don't verify a signature; we
verify a hash that an attacker who controls the proxy in steady state
could in principle replace alongside the binary). But it raises the
attack cost meaningfully, and it matches the security posture of
cargo-dist's own shell installer (`curl ... | sh`), which the
PowerShell installer one-liner already inherits via cargo-dist.

**Rejected alternative: Windows Authenticode signing.** Would be stronger, but
acquiring an EV cert and threading signing through release.yml +
windows-installers.yml is a multi-month project. SHA256 sidecar
verification is the 90% solution that ships in days.

### Post-install version verification

**Problem.** `msiexec /i ... /passive /norestart` exits 0 in several
scenarios where the install didn't materially complete:
- MajorUpgrade matching the *same* version (re-running v3.15.2 install
  with v3.15.2 already installed): exits 0, no file changed.
- Restart Manager scheduled deferred file replace (exit 3010 —
  REBOOT_REQUIRED, file isn't actually swapped until next reboot).
- `/passive` mode: msiexec may exit 0 immediately when another `msiexec`
  is running and the new install is queued.

JSON consumers reading `tr300 update --json` would see
`"success": true, "strategy": "msi_global"` — false positive, fleet
automation thinks the rollout is done when the on-disk binary is still
old.

**Fix.** After `msiexec` / Inno EXE returns success, re-exec
`env::current_exe() --version` via a fresh `Command`, parse the
version, compare against the expected `latest`. Mismatch → return
`StrategyError::Runtime` with a clear message. msiexec exit code 3010
is intercepted BEFORE the version check and surfaces as a dedicated
"Reboot, then verify with `tr300 --version`" message — operators can
distinguish "needs reboot" from "actually failed" from the JSON
`attempts[].message` field.

**Why not re-exec `cargo install --list` or similar.** The cargo path
doesn't apply (these are MSI/EXE strategies, not cargo). And the
on-disk binary's `--version` output is the source of truth: it's
literally the bytes that will run next time the user invokes `tr300`,
so confirming THAT version equals expected is the only verification
that actually matters.

### WMI hard-timeout pattern

**Problem.** Windows WMI queries (`wmi::WMIConnection::query()`) have
no Rust-side timeout. The bounded-subprocess helper in
`src/collectors/command.rs` only protects `Command::output()` calls.
WMI bypasses it entirely. A deadlocked WMI provider — post-Windows-
update misconfig, Group Policy lockdown of the security namespace,
antivirus interfering with `Win32_EncryptableVolume` queries, broken
`Winmgmt` service — could block the report for tens of seconds with
no indication of why.

**Fix.** `src/collectors/platform/windows.rs::with_timeout(budget, f)`:

```rust
fn with_timeout<F, T>(budget: Duration, f: F) -> Option<T>
where
    F: FnOnce() -> Option<T> + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || { let _ = tx.send(f()); });
    rx.recv_timeout(budget).ok().flatten()
}
```

The 5-second `WMI_TIMEOUT` constant is the budget. All four WMI-touching
collectors (`get_bitlocker_status`, `get_socket_count_wmi`,
`get_network_info_wmi`, `last_cold_boot_seconds`) wrap their inner
logic in this. The 5th (`get_battery_wmi(&WMIConnection)`) takes a
non-`'static` reference so can't be wrapped this way — but
`get_battery_native()` via `GetSystemPowerStatus` covers the same
ground in ~1 ms and runs first.

**The detached-thread tradeoff.** Rust intentionally doesn't kill
threads. When `with_timeout` returns `None`, the worker thread is
still blocked on its WMI call and continues running in the background.
There's no resource leak in practice: when our process exits,
Windows tears down all threads. The COMLibrary created inside the
worker thread's closure has the same lifetime as the thread, so it
goes too. The cost is a tiny amount of orphan background work per
timeout-firing WMI call — bounded, paid for by the OS, not our
problem.

**Rejected alternative: tokio + spawn_blocking + timeout.** Async
runtime drag for a single sync wait operation. Not worth pulling in
the entire tokio crate.

**Rejected alternative: a global watchdog thread that kills the
process after N seconds.** Too coarse — would terminate the entire
report even when only one collector is hung, and there's no way to
return the data that WAS collected before the timeout.

### Windows self-EXE delete via detached cleanup

**Problem.** `tr300 uninstall` -> Complete called `fs::remove_file`
on `find_binary_location()`, which prefers `env::current_exe()`. When
the user ran `tr300 uninstall` from `%LocalAppData%\Programs\tr300\
bin\tr300.exe`, that path IS the running EXE. Windows refuses
`DeleteFile` on a running EXE image with raw OS error 5 because the
loader holds an open file handle. The user was left with profile
cleaned, binary still on disk, and a confusing `RemoveBinary` error.

**Fix.** `is_running_binary(target)` canonicalizes both `target` and
`env::current_exe()` via `fs::canonicalize` (handles 8.3 short names,
junctions, drive-letter case) and compares for equality. When the
running binary IS the target, `schedule_self_cleanup` spawns a
detached `cmd.exe` job:

```
cmd /c timeout /t 2 /nobreak > nul & del "<binary>" & rd /q "<parent>"
```

With `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` creation flags so
the child has no console window and doesn't inherit our process
group. The 2-second wait gives our parent process time to exit and
release the file handle; `del` then succeeds.

**Why not `MoveFileEx(path, NULL, MOVEFILE_DELAY_UNTIL_REBOOT)`.**
Works but requires the user to reboot before the binary is actually
gone, and `MOVEFILE_DELAY_UNTIL_REBOOT` requires admin on Windows 11
(MS hardened this in 22H2 to prevent ransomware abuse). The detached
cleanup gives sub-3-second cleanup without elevation.

**Why `cmd.exe` rather than PowerShell.** `cmd.exe` is always
present on Windows; PowerShell 5.1 is usually but not always
(Server Core, stripped images). `cmd` has no startup overhead;
`pwsh -Command` has measurable JIT warmup. For a 2-line cleanup
script, cmd wins.

**Parent-dir cleanup heuristic.** Only `rd /q` the parent if it
contains "tr300" in the name (case-insensitive) — matches the
synchronous-path heuristic, prevents wiping unrelated dirs in
unusual portable-install scenarios.
