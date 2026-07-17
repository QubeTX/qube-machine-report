# TR-300 Testing Guide

This file tracks local/hosted release gates and the manual verification matrix.
When the maintainer explicitly defers unavailable personal hardware, the open
matrix row stays visible as post-release patch work rather than being presented
as passed.

## Per-version verification log

### Architecture decision coverage backfill — 2026-07-17

- **Scope:** reconcile the canonical `docs/architecture-decisions.md` against
  current source, workflows, the v4 thinking record, both Mac/Alienware
  handoffs, release/testing ledgers, agent guides, and public v4.0.1 state.
- **Coverage result:** the existing 1,452-line ledger already captured v4
  SemVer and evidence semantics, macOS collection, disk/memory definitions,
  bounded commands/JSON/save/update behavior, manual-only persistence,
  endpoint-policy failure, blocking gates, Apple trust/freeze, personal-
  hardware deferral, toolchain/release policy, Windows accuracy/distribution,
  and installer safety. The backfill adds the previously implicit one-Rust-
  product/collection-budget and terminal/JSON/privacy decisions, the missing
  v3.17 advisory one-install consolidation rationale, a current status index,
  and the exhaustive `main`/checkout-v6 decision record.
- **Evidence rule:** the ADR distinguishes accepted design, observed proof,
  rejected alternatives, consequences, and future revalidation triggers. It
  does not convert the open personal Alienware/AMD/Pi rows into passed evidence
  and contains no credential values.
- **Change boundary:** documentation only—no Rust source, dependencies, Cargo
  metadata, toolchain, installer input, Apple script/workflow/input, tag, crate,
  release asset, or homepage byte is changed.
- **Structural and source audit:** `git diff --check`, ADR GitHub-anchor
  validation (48 headings), relative-link validation across all nine changed
  Markdown files, `cargo fmt --all -- --check`, `actionlint
  .github/workflows/*.yml`, `shellcheck scripts/*.sh`, `bash -n
  scripts/sign-notarize-macos.sh`, protected-surface checks, and credential-
  pattern checks passed. Direct source/installer comparison confirmed every new
  mode, output, save, migration, marker/path, impersonation, and no-op claim.
- **Locked local release gate:** `cargo clippy --locked --all-targets
  --workspace -- -D warnings`; `cargo test --locked --workspace --all-targets`
  (121 library + 19 integration tests); `cargo package --locked --list` (39
  files); `cargo publish --dry-run --locked`; `cargo build --locked --release
  --workspace`; `cargo audit` (1,160 advisories loaded, 221 dependencies
  scanned, zero findings); and `dist plan --output-format=json` (cargo-dist
  0.31.0, six targets) all passed.
- **Compiled runtime smoke:** `tr300 4.0.1`; ordinary full, fast, JSON, and
  compatibility `--no-save` runs created no Markdown report; full/fast JSON
  parsed with schema/mode/privacy invariants; 49 ASCII table lines were exactly
  51 columns; and each of `-r`, `--report`, `-s`, and `--save` created a new,
  valid Markdown report without overwriting an existing file. All four
  deliberately created smoke reports were removed afterward. Non-Windows
  `migrate-cleanup --dry-run --json` returned the documented successful
  single-location no-op.
- **Pre-push live-state audit:** every previously cited run ID resolves to its
  stated SHA and success; GitHub reports `main`, zero open pull requests, and
  four active workflows; v4.0.0/v4.0.1 tags are unchanged; v4.0.1 remains a
  non-draft/non-prerelease 28-asset release with zero empty assets; and crates.io
  still serves unyanked 4.0.1 with checksum
  `55086eb631a3b67c8ab0eaa53b9c3783097044ef77321ec8e6849c30e32275da`.
- **Hosted substantive-commit proof:** ADR backfill commit
  `e38fe2abcffdf6f85d4dac1c12dd294f36604a59` passed all 13 exact-SHA jobs in
  CI run 29560970377 across Linux, Apple Silicon macOS, and Windows, including
  all three fast-mode speed budgets and blocking RustSec. The run had zero
  annotations and no deprecated checkout/Node-runtime log match. Exact-SHA
  crates run 29561137746 found existing 4.0.1 and skipped registry-token access,
  format/clippy/test/package, and publish steps. The follow-up commit records
  this proof only and must itself receive final exact-SHA CI/crates closure.

### Repository default-branch migration — 2026-07-17

- **Scope:** rename the GitHub default branch from `master` to `main` without
  changing product source, dependencies, release tags, public artifacts,
  signing/notarization inputs, or the production homepage.
- **Pre-mutation audit:** `ci.yml` and `crates-publish.yml` already accepted
  `main`; cargo-dist publication is tag-triggered; supplemental Windows
  packaging follows the tag-triggered Release workflow; and GitHub reported no
  open pull requests, branch protection, rulesets, webhooks, deployment
  environments, or other visible policy tied to `master`.
- **Runner-compatibility follow-up:** the first fully green `main` runs exposed
  GitHub's non-failing Node 20 deprecation annotation for
  `actions/checkout@v4`. Both branch workflows now use
  `actions/checkout@v6` and its supported Node 24 runtime, matching the
  previously proven release and supplemental Windows workflows. This changes
  no product, dependency, package, installer, signing, or artifact input.
  Follow-up commit `1714d1fc0b90475d5f0aa590b1ec7d93b24d2eee`
  passed CI run 29559148638 across all 13 jobs with zero check annotations and
  no checkout-v4/Node-20 warning in its logs. Exact-SHA crates run 29559305341
  used checkout v6, detected already-published 4.0.1, skipped token access and
  every check/publish step, and emitted no deprecated-checkout warning.
- **Migration method:** GitHub's atomic branch-rename API moved the unchanged
  `cd3c179540b48770e1c555cbf60c809d702eb999` branch tip to `main`, updated the
  repository default, and removed the old remote branch. The local branch now
  tracks `origin/main`, and `origin/HEAD` resolves to `origin/main`.
- **Local proof:** format, warning-denying locked Clippy, 121 library tests, 19
  integration tests, release build/runtime JSON smoke, exact 39-file package
  list, locked publish dry-run, RustSec audit over 221 dependencies, cargo-dist
  six-target/installer plan, actionlint, shellcheck, Bash syntax, diff checks,
  and protected-input audit all passed. No Rust source, dependency, Cargo
  metadata, Apple signing input/script, `release.yml`, installer source, tag, or
  artifact changed.
- **Hosted branch proof:** migration commit
  `41c30b1e43f8abc5208f0d94702ed12cd91fb7a7` passed CI run 29557626125 across
  all 13 blocking Linux/macOS ARM/Windows jobs on `main`. Downstream Crates.io
  Publish run 29557758673 ran from that exact SHA, found 4.0.1 already
  published, and skipped every token/check/publish step as designed. A fresh
  clone checks out `main` at that SHA; GitHub's old `/tree/master` URL redirects
  to `/tree/main`; the default branch, remote symbolic HEAD, and local upstream
  all resolve to `main`; no remote `master` or open pull request remains.
- **Distribution continuity proof:** all four Actions workflows remain active.
  `v4.0.0` and `v4.0.1` remain at their original SHAs; v4.0.1 remains a public
  non-draft/non-prerelease 28-asset release with no missing, non-uploaded, or
  zero-byte asset. Crates.io still serves unyanked 4.0.1 with checksum
  `55086eb631a3b67c8ab0eaa53b9c3783097044ef77321ec8e6849c30e32275da`.
  The Apple credential/variable names remain configured, and the original
  hosted arm64/x86_64 notarization results remain `Accepted`.
- **Public Mac re-audit:** fresh arm64/x86_64 downloads again matched their
  sidecars, `sha256.sum`, manifest, and known archive hashes
  `b2cd1ecbc86d7f86beddb7b15044ac5839d894a4eae781c1bdfb01a305cf3342`
  and `cbc2800cf4e2dad47d8113db33a8092019c6efeccc0e8ee61cae023fff3cb861`.
  Both extracted binaries report 4.0.1 and pass strict code-sign verification
  with identifier `com.qubetx.tr300`, Team ID `M9D5379H93`, hardened runtime,
  secure timestamp, expected Developer ID authority, correct architecture, and
  leaf-certificate SHA-1 `739B04530883FF9B665C66BD464F98C622971B32`.
- **Homepage continuity proof:** homepage commit
  `d77397479ad2b1189cce86b5402eaf1cc966abdf` remains clean/synchronized on its
  own `main`; lint and production build pass. Production serves the identical
  `index-DghJyecZ.js` bundle (SHA-256
  `edfa6225cdf5d171d68fe3f83f5aab8d395b32a40e9ac8541cce2fa0cfab52ce`),
  identical index, and byte-identical shell/PowerShell install wrappers.

### v4.0.1 — 2026-07-15

- **Fix-forward reason:** immutable tag `v4.0.0` points to
  `c21d5981d4109199fa4bcba15ef8af6285a33d56`. CI run 29389974094 passed and
  crates run 29390118811 published `tr300 4.0.0`, but cargo-dist release run
  29390216481 failed closed before the host job. On clean GitHub Apple runners,
  `security find-identity` saw the imported certificate while `codesign` could
  not resolve its SHA-1 selector because the ephemeral keychain was absent from
  the user search list. No GitHub Release or unsigned Mac artifact was hosted.
- **Correction:** `scripts/sign-notarize-macos.sh` temporarily prepends only its
  ephemeral keychain to the user search list for the signing call, restores the
  original list immediately and from the cleanup trap, and verifies the leaf
  certificate embedded in the signed Mach-O has the exact resolved SHA-1
  fingerprint. Existing authority, Team ID, identifier, hardened-runtime,
  timestamp, Apple `Accepted`, repack, sidecar, and manifest gates remain.
- **Local proof:** native arm64 and Rosetta x86_64 builds plus 121 library + 19
  integration tests pass; the isolated no-write/four-manual-save/JSON/privacy/
  51-column ASCII smoke passes; format, warning-denying Clippy, RustSec audit,
  39-file package/publish dry-run, cargo-dist plan, actionlint, shellcheck, and
  Bash syntax pass.
- **Real Apple proof:** both actual cargo-dist v4.0.1 archives passed the full
  fixed script. Arm64 submission `52b52e88-8eb9-457b-bb01-6c39f01da913` and
  x86_64 submission `e018b7e1-d16e-4a33-b2a2-5b62512652b5` are `Accepted`.
  Extracted binaries report `tr300 4.0.1`, verify identifier
  `com.qubetx.tr300`, Team ID `M9D5379H93`, expected Developer ID authority,
  hardened runtime, timestamp, and embedded leaf fingerprint. Archive,
  sidecar, and manifest SHA-256 values agree at
  `eb9be3c3afe19a6e6e07f6482f5fb1073e5d2407fd30fc449e362d76c41c59b9`
  (arm64) and
  `5549e3d26ddcd20b0ec74f11e083d94183b30ab6eaf4e80f9f42f3ac9610ec46`
  (x86_64). The original keychain search list matched byte-for-byte after
  success and after injected real timestamp/notary transport failures; those
  transient failures did not repack or claim acceptance.
- **Hosted source/package proof:** release source commit
  `b67ad083503d0fff840af8467015d05c659268ea` passed CI run 29391956665 across
  all 13 blocking jobs. Crates.io Publish run 29392101640 completed from that
  same SHA; crates.io serves unyanked `tr300 4.0.1` with checksum
  `55086eb631a3b67c8ab0eaa53b9c3783097044ef77321ec8e6849c30e32275da`.
- **Hosted release proof:** lightweight tag `v4.0.1` resolves to the release
  source commit without moving `v4.0.0`. Cargo-dist run 29392185522 and Windows
  Installers run 29392382949 completed successfully. Hosted notarization was
  `Accepted` for arm64 submission `97b0c295-89d8-4758-a4c3-1dc345c28f0e`
  and x86_64 submission `09cf1403-e546-4f5e-8de1-9bf92fd602e9` before the
  non-draft/non-prerelease GitHub Release published.
- **Public artifact proof:** the release targets the exact tested SHA and has
  exactly 28 assets. Downloaded arm64 and x86_64 archives matched their
  sidecars, `sha256.sum`, cargo-dist manifest, and GitHub asset digests at
  `b2cd1ecbc86d7f86beddb7b15044ac5839d894a4eae781c1bdfb01a305cf3342`
  and `cbc2800cf4e2dad47d8113db33a8092019c6efeccc0e8ee61cae023fff3cb861`.
  Both extracted binaries report `tr300 4.0.1`, have the correct Mach-O
  architecture, and pass strict Developer ID verification with identifier
  `com.qubetx.tr300`, Team ID `M9D5379H93`, hardened runtime, secure timestamp,
  and embedded certificate SHA-1
  `739B04530883FF9B665C66BD464F98C622971B32`.
- **Installer proof:** canonical and legacy shell aliases are byte-identical
  (SHA-256 `79bdb3ab32bcee155967a8ca1fdfccf955cae612d5d8afee27132788bd9e01b1`),
  as are the PowerShell aliases
  (`7e5f59911fdb73e2405d2354fe24bc1d60b3e39b40c534599ef48ee32899cb66`).
  Supplemental Corporate MSI, Global EXE, and Corporate EXE assets match their
  sidecars/GitHub digests at
  `6ca603d30a13aca11c21aab348ea7aa3ab932c18ebdb58462557fbb7fb771f3d`,
  `f9477c0ea53fd81f7e11fc3d279e884531a8303e9165f565a6dadc321220f47a`,
  and `339cfd02ed7fb0d3741909c07477fd3cbfe803a21bac88237cb519613fe559d3`.
- **Homepage proof:** homepage commit
  `d77397479ad2b1189cce86b5402eaf1cc966abdf` on its default branch deploys
  package 1.13.0 to `https://reports.qubetx.com/`. Lint, Vite production build,
  Bash/PowerShell wrapper syntax, production wrapper equality, release/docs/
  installer link checks, and Chrome desktop/mobile inspection pass. Production
  serves bundle `index-DghJyecZ.js`; both viewport widths have no horizontal
  overflow or site-origin console warning/error, and all 49 sample rows are
  exactly 51 columns. SD-300 and Shaughv OS remain intentionally WIP-delisted.
- **Release-ledger proof:** documentation commit
  `771fd09a90baf94db64f21471482c296acf71d05` records the observed release and
  homepage evidence. CI run 29394204632 passed all 13 jobs on that exact SHA;
  Crates.io Publish run 29394374303 also succeeded and correctly skipped the
  already-published 4.0.1 package.
- **Remaining evidence boundary:** personal Alienware Windows, AMD64 Linux, and
  Raspberry Pi 4 checks remain explicit post-release patch tasks; none is
  retroactively claimed by the hosted release evidence above.

### v4.0.0 — 2026-07-14

- **Scope/state:** manifest and release docs are v4.0.0. macOS collection,
  shared runtime hardening, manual-only persistence, graceful endpoint-policy
  update failure, and fail-closed Mac signing/notarization are release scope.
  The maintainer explicitly approved personal Alienware, AMD64 Linux, and
  Raspberry Pi 4 verification after release with forward patches as needed.
- **SemVer boundary:** final API review found source-breaking additions to
  public Rust records and changed collector-helper signatures. The release is
  therefore v4.0.0, not v3.18.0. CLI behavior and existing schema-v1
  JSON keys remain compatible; affected records are `#[non_exhaustive]`.
- **Host:** MacBook Pro M2 (`Mac14,7`), macOS 26.3.1 build 25D2128, 8 GiB,
  APFS root, FileVault On, integrated battery, internal Retina display.
- **Native Apple Silicon gate:** `cargo fmt --all -- --check`,
  `cargo clippy --locked --all-targets --workspace -- -D warnings`,
  `cargo test --locked --workspace --all-targets`, and
  `cargo build --locked --release` pass. Test count: **121 library + 19
  integration**, zero failures.
- **External Rust consumer:** an isolated temporary crate using TR-300 as a path
  dependency compiled and ran with `SystemInfo::collect_with_mode`,
  `Config::default`, `report::generate`, and a wildcard arm for the
  non-exhaustive `CollectMode` contract.
- **Rosetta gate:** `cargo build --locked --release --target
  x86_64-apple-darwin` and `cargo test --locked --target
  x86_64-apple-darwin --workspace --all-targets` pass. Test count: **121
  library + 19 integration**, zero failures. The executable is confirmed Mach-O
  x86_64 and runs through `/usr/bin/arch -x86_64`.
- **Live native parity:** schema v1 JSON matched `sw_vers` version/build,
  `sysctl` model/physical/logical cores/RAM, `pmset` battery percent, and
  `fdesetup` FileVault state. Reported values included `MacBook Pro (Mac14,7)`,
  Apple M2, `4P + 4E`, arm64, APFS `/`, Normal boot, current logical/native
  display resolution, nonzero available memory, battery health/cycles, and
  Codex terminal detection.
- **Live Rosetta parity:** the same host facts remained populated through the
  native arm64 profiler slice; architecture was exactly `arm64 host / x86_64
  (Rosetta 2)` and both frequency fields were `null` rather than the translated
  2.4 GHz compatibility value.
- **Mode/output contracts:** full/fast JSON parse; fast omits slow display and
  encryption fields; `LC_ALL=C LANG=C` auto-falls back to printable ASCII with
  every table line exactly 51 columns; native and Rosetta JSON expose no path
  containing `serial`, `uuid`, or `udid`; zsh profile install/reinstall/uninstall
  round-trip passes entirely inside a temporary home. Normal full/fast/JSON
  runs create no report file; each of `-r`, `--report`, `-s`, and `--save`
  requests the existing collision-safe manual writer.
- **Updater:** native and Rosetta v4 `tr300 update --json` both returned
  success, `current_version: 4.0.0`, latest hosted `3.17.0`, and
  `update_available: false`; no install was run.
  v4 unit tests classify likely endpoint-policy staging/launch errors as
  `blocked`, stop later strategies, preserve the installed binary, and include
  cleanup/manual-release context without any force/direct-overwrite path.
- **Performance:** final five-run release medians are 0.51s native full, 0.23s
  native fast, 0.72s Rosetta full, and 0.36s Rosetta fast. Both fast paths are
  well below the blocking 1.5s CI budget.
- **Apple release trust:** the least-privilege App Store Connect API credential
  and selected Developer ID Application identity were tested without exposing
  values. The script resolves exactly one imported identity in its ephemeral
  keychain and signs by certificate fingerprint, preventing ambiguity when a
  developer login keychain contains the same certificate. Both actual v4
  release binaries completed the full sign/notary/repack/rehash path:
  aarch64 submission `c2afae62-1873-4337-8c88-1bbfa26c23eb` and x86_64
  submission `fe2dcc67-cfe1-49be-8d4c-59daf8697c61` were `Accepted`. This
  final pass supplied the signing identity in the SHA-1 fingerprint form used
  by the repository variable, so it exercised the exact hosted configuration.
  Extracted
  binaries report `tr300 4.0.0` and verify identifier `com.qubetx.tr300`,
  Developer ID authority, Team ID `M9D5379H93`, hardened runtime, and secure
  timestamp. Final local archive/sidecar/manifest SHA-256 values were
  `b1085dcc6e1bf5ce0e3a2fdeab0342cf4f4ae94506a007c2089a8a3db785a244`
  (arm64) and
  `703dfe22a8fbdc2b5bcb6e4bafce99b0e558f920ef1f8284774eca5ba2a34f30`
  (x86_64). `actionlint`, `shellcheck`, and `bash -n` pass.
- **Security/package:** `cargo audit` passes against the locked dependency set;
  `crossbeam-epoch` is 0.9.20. Markdown collision/symlink, updater staging/
  policy-block, and release-manifest tests pass. The final dirty-tree package
  and publish dry-run pass with 39 packaged files (632.9 KiB / 164.7 KiB
  compressed), and `cargo dist plan` contains all six configured targets. The
  same package/dry-run check is repeated without `--allow-dirty` after commit.
- **CI enforcement:** macOS ARM test/build/speed jobs have no
  `continue-on-error`; RustSec audit is blocking. The exact release SHA must be
  green and crates.io publication must settle before `v4.0.0` is tagged.
- **Mac release freeze:** post-release Alienware/Linux/Pi work must not change
  macOS collectors/cfg branches, Apple targets/artifact names,
  `scripts/sign-notarize-macos.sh`, the Apple `release.yml` step, toolchain, or
  signing/notarization secrets/variables. Any shared/dependency/workflow/Apple
  change requires native + Rosetta proof again on a Mac; Apple-input changes
  also require a real archive notary round-trip.
- **Post-release hardware boundary:** hosted Windows/Linux checks are not the
  missing personal-hardware evidence. Alienware/AMD/Pi rows remain open, and
  the managed-work antivirus case is tracked separately from personal Windows
  accuracy.

### v3.17.0 — 2026-06-08

- Release commit/tag: `2d0c0b2470db603aa2e8058fee382b0dcaf0930c` / `v3.17.0`.
- CI run 27116322705, Release run 27116326346, Crates.io Publish run
  27116417286, and Windows Installers run 27116494691 all succeeded.
- crates.io reports newest/max version `3.17.0`.
- GitHub Release is published (not draft/prerelease) with 28 assets, including
  both macOS architectures, Linux ARM64/x86_64 gnu/musl, Windows archive/MSI,
  the three add-on Windows installers, checksums, canonical installer scripts,
  and legacy `tr-300-installer.*` aliases.
- User-visible scope: Windows cross-method install consolidation and edition-
  preserving updates. macOS/Linux runtime collection was unchanged at this tag.

### v3.16.0 — 2026-06-03

- **Stability & cross-platform hardening pass**, shipped as seven reviewed, individually-CI-green PRs (PR1 output/build robustness; PR2 macOS Tahoe codename + ARM-Linux CPU frequency; PR3 Windows MODEL row + GPU/boot/socket-count; PR4 Linux battery-units/lspci/ZFS; PR5 symlink-safe install + updater temp cleanup; PR6 Unicode-table-width + checksum tests; PR7 self-update cargo-path verify + rate-limit messaging), merged to master in order. Two audit findings (A3 macOS host-arch, D4 macOS battery wording) were verified non-issues after review and made no code change — see `MASTER_PLAN.md`.
- Full local gate green on the Windows authoring host: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --workspace -- -D warnings`, `cargo test --locked --workspace --all-targets`, `cargo build --locked --release`. **104 lib + 18 integration tests pass** (was 98+18 at v3.15.3; +6 lib tests: macOS codename, Linux cpufreq parse, Windows compose_machine_model, Linux battery-health/zfs-rank/lspci, Unicode table width, SHA256 checksum_verdict, http_status_message, post_install_version_ok).
- Cross-platform compile + tests verified by **per-PR CI** on Linux x64 + macOS ARM + Windows x64 runners; every PR's CI was green before merge.
- `tr300 --version` reports `3.16.0`. Windows host smoke: `tr300 --json` is valid JSON; `os.machine_model` populates (`"Alienware m16 R2"`); `cpu.gpus` contains only hardware adapters; `session.last_login` is a real value or `null` (never the old sentence). `tr300 --ascii` renders.
- **CI verification** — master CI run 26868530816 (commit `ed664d5`) succeeded across all 13 gates: fmt + clippy (Linux), test on Linux/macOS-ARM/Windows, release build on the same three, speed gate, audit, dist-plan.
- **Crates.io verification** — crates-publish run 26868660180 published `tr300` 3.16.0 to crates.io after rerunning fmt/clippy/tests/package/dry-run against the exact CI-tested SHA. `curl https://crates.io/api/v1/crates/tr300` confirms `newest_version=3.16.0`.
- **Release verification** — release.yml run 26868722226 succeeded across all 10 jobs (plan + 6 build-local-artifacts + build-global-artifacts + host + announce); GitHub Release v3.16.0 published 2026-06-03T06:57:28Z. windows-installers.yml run 26868901275 (triggered via workflow_run after release.yml) succeeded — its F20 pre-flight passed and it uploaded the 6 additional installer assets (Corporate MSI + Global EXE setup + Corporate EXE setup + 3 `.sha256` sidecars). **Final GitHub Release v3.16.0: 28 assets** verified via `gh release view v3.16.0`.
- **Manual verification required on release-candidate hardware (deferred to the user's machines):**
  - **macOS (Apple Silicon + Intel)**: confirm the OS row shows the correct codename ("Tahoe" on macOS 26); confirm battery row unchanged (D4 was intentionally not touched). Confirm `tr300 update` from an older build updates cleanly or reports an honest failure (U1 — no silent "Updated to vX" no-op).
  - **Linux ARM (Raspberry Pi)**: confirm the CPU FREQ row shows a real frequency, not `0.00 GHz` (A2). Confirm a dotfiles-symlinked `~/.bashrc` survives `tr300 install` as a symlink (E3).
  - **Linux x86_64**: confirm GPU detection doesn't list an HDMI "Display Audio" controller (D5); confirm battery health % is sane (D3).

### v3.15.3 — 2026-05-23

- Deferred-audit-findings follow-up release. Resolves the three v3.15.2 audit findings (F17, F20, F22) that the original audit explicitly deferred. All audit work from the May 2026 cycle is now landed.
- Full local gate green on Windows host: `cargo fmt -- --check`, `cargo clippy --all-targets --workspace -- -D warnings`, `cargo test --workspace --all-targets`, `cargo package --locked --list`, `cargo publish --dry-run --locked`, `cargo build --release`.
- 98 lib + 18 integration tests pass (same as v3.15.2). No new test infrastructure required — F17 is verified by integration manual matrix, F20 is verified by the next real release, F22 is verified by manual on-Windows smoke + a one-off COM-mode harness if needed.
- `tr300 --version` reports `3.15.3`. `tr300 --fast --json | head -5` produces valid JSON header.
- **CI verification** — master CI run 26342102096 (commit `e030baa`) succeeded across all 13 gates: fmt + clippy (Linux), test on Linux/macOS-ARM/Windows, release build on the same three, speed gate, audit, dist-plan. Follow-up CI run 26342326674 (commit `a70ed50`, F20 pre-flight `--repo` fix-forward) also succeeded across all 13 gates.
- **Crates.io verification** — crates-publish run 26342172080 published `tr300` 3.15.3 to crates.io after rerunning fmt/clippy/tests/package/dry-run against the exact CI-tested SHA. `curl https://crates.io/api/v1/crates/tr300` confirms `newest_version=3.15.3`.
- **Release verification** — release.yml run 26342193045 succeeded across all 10 jobs (plan + 6 build-local-artifacts + build-global-artifacts + host + announce). windows-installers.yml run 26342293613 (triggered via workflow_run after release.yml completion) **FAILED** at the new F20 pre-flight step — `gh release view` couldn't infer the repo because the step runs BEFORE actions/checkout. Fix-forward: commit `a70ed50` adds `--repo ${{ github.repository }}` to the `gh release view` call. Workflow_dispatch retry windows-installers.yml run 26342329480 then succeeded with the master-resident fix and uploaded the 6 additional installer assets (Corporate MSI + Global EXE setup + Corporate EXE setup + 3 `.sha256` sidecars). **Final GitHub Release v3.15.3: 28 assets** verified via `gh release view v3.15.3`, published 2026-05-23T20:05:52Z.
- **Manual verification required on release-candidate machines:**
  - **Linux**: write `alias report='echo hi'` into a temp `~/.bashrc` (back up real one first), run `./target/release/tr300 install`, confirm the alias-collision stderr note prints with the file:line of the existing alias (F17). Then on a clean account without any pre-existing `report`, confirm install runs silently with no F17 note.
  - **macOS**: same Unix probe as Linux but in `~/.zshrc` (F17). Confirm the install message style matches the project's existing voice.
  - **Windows**: drop `Set-Alias -Name report -Value notepad` into `$PROFILE`, run `tr300 install`, confirm the F17 stderr note prints with the profile-file path and line. On a clean profile, confirm no note. Run a manual smoke of `cargo run --release` and confirm Windows Edition / Virtualization / GPUs / Battery rows still populate correctly (F22 — the WMI batch now runs in a worker thread). Manual F20 test: `gh workflow run "Windows Installers" -f tag=v3.15.3` should pass the pre-flight after the v3.15.3 release publishes; a synthetic test with a non-existent tag should fail with the actionable "missing assets" message.

### v3.15.2 — 2026-05-18

- Cross-platform audit + remediation release. 19 of 22 audit findings fixed (F1–F19+F21); 3 deferred (F17, F20, F22).
- **CI verification** — master CI run 26020803537 (commit `0d975e2`, after fix-forward `0d975e2` for a Linux-clippy `unused_imports` finding on `run_stdout` / `run_output` in the macOS-cfg-gated import lines from the initial release commit `c1f9d52` / failed CI run 26020553352) passed all 13 gates: fmt + clippy (Linux), test on Linux/macOS-ARM/Windows, release build on the same three, speed gate, audit, dist-plan.
- **Crates.io verification** — crates-publish run 26020958768 published `tr300` 3.15.2 to crates.io at 2026-05-18T07:59Z after rerunning fmt/clippy/tests/package/dry-run against the exact CI-tested SHA.
- **Release verification** — release.yml run 26021005159 succeeded across all 10 jobs (plan + 6 build-local-artifacts + build-global-artifacts + host + announce). windows-installers.yml run 26021208515 (triggered via workflow_run after release.yml completion) added the 6 additional installer assets. **Final GitHub Release v3.15.2: 28 assets** verified via `gh release view v3.15.2`, published 2026-05-18T08:04:44Z. Contains all four first-class Windows installers (Global MSI, Corporate MSI, Global EXE setup, Corporate EXE setup) + 6 platform binary archives + cargo-dist installer scripts (canonical + legacy `tr-300-*` aliases) + source tarball + dist-manifest + 3 SHA256 sidecars for the new Windows installer assets.
- Full local gate green on Windows host: `cargo fmt -- --check`, `cargo clippy --all-targets --workspace -- -D warnings`, `cargo test --workspace --all-targets`, `cargo build --release`.
- 98 lib + 18 integration tests pass (up from 72 lib in v3.15.1). 26 new tests cover atomic-write semantics, marker-balance check, install-snippet content pinning (Unix + Windows), `strip_prerelease_metadata` / `is_newer` (prerelease + build-metadata handling), `parse_sha256_sidecar` (cargo-dist format variants), `compute_sha256` (RFC empty-input vector), and `escape_json` round-trips.
- `tr300 --version` reports `3.15.2`. `tr300 --fast --json | head -5` produces valid JSON header.
- New dependency: `sha2 = "0.10"` (Windows-only). Confirmed non-Windows builds compile cleanly.
- **Manual verification required on release-candidate machines** (split by platform):
  - **Linux**: `sudo ./target/release/tr300 install` should refuse with the new actionable error (F11). `LC_MESSAGES=de_DE.UTF-8 ./target/release/tr300 --fast` should still report the right socket count (F19). On a fresh-account host, `tr300` should still report "Never logged in" regardless of `LC_MESSAGES`.
  - **macOS**: on a fresh user account with neither `.bashrc` nor `.zshrc`, `tr300 install` creates `.zshrc` (F12). Atomic-write durability under Ctrl-C between truncate-and-write (F1).
  - **Windows**: marker-block rejection on hand-mutilated `.bashrc` (F2). OneDrive-redirected `$PROFILE` install on Windows 11. After install, `rm $(which tr300)` then open a new PowerShell — no error spam (F4). Nested `pwsh -Command "Get-Module"` inside an installed shell — confirm no table render (F4). Dual-shell install with both `powershell` and `pwsh` — confirm both profile paths modified (F5). `chcp 437` → `tr300` → another CP-437 tool — confirm CP restored (F10). Junction-mounted `C:\mnt\D` — confirm disk row reports C: usage (F18). Self-EXE uninstall from `%LocalAppData%\Programs\tr300\bin\tr300.exe` → Complete — confirm deferred cleanup (F13). Hung WMI test via `net stop winmgmt` — full-mode report should finish in ≤5s per WMI site (F14). Tampered-MSI test: edit one byte of a real v3.15.2 MSI, run `tr300 update` against a mock manifest — confirm refusal with SHA256 mismatch (F3). Reboot-required test: hold an open file handle to `tr300.exe` during `tr300 update`, observe msiexec 3010, confirm clear "reboot, then verify" message (F8). Prerelease ordering: install a `v3.15.x-rc.1` prerelease, publish stable, run `tr300 update` — confirm stable detected as newer (F7). First-PATH-entry Inno uninstall: install Corporate EXE on fresh user with empty HKCU\Environment\Path — confirm `tr300\bin` at index 1 → uninstall → PATH empty, no orphan (F9).



## Automated gates (run by CI on every PR)

`.github/workflows/ci.yml` runs these on every push and pull request:

- **`fmt`** — `cargo fmt --check` (Linux)
- **`clippy`** — `cargo clippy --all-targets --workspace -- -D warnings` (Linux)
- **`test`** — `cargo test --workspace --all-targets` on Linux + macOS ARM + Windows
- **`build`** — release build smoke test on all three platforms (with a `--version` + `--fast --json` invocation to verify the binary runs)
- **`speed`** — 5-run median of `tr300 --fast` on Linux + macOS ARM + Windows, fails if median > 1500 ms (auto-run safety gate). Reports times in the job summary.
- **`audit`** — blocking `cargo audit` against RustSec advisories
- **`dist-plan`** — verifies cargo-dist config parses, so dist regressions don't surprise us at tag time

To reproduce locally before pushing:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --workspace -- -D warnings
cargo test --locked --workspace --all-targets
cargo audit
cargo run -- --json | jq .            # parses; normal runs never save
cargo run -- --json update | jq .     # update action JSON shape
cargo run -- --fast --json | jq .     # same, fast mode
cargo run -- --ascii                  # visual inspection; no file write
actionlint .github/workflows/*.yml
shellcheck scripts/sign-notarize-macos.sh
```

For a release, additionally prove native/Rosetta release binaries, isolated
manual-save aliases, no-write ordinary runs, and a real cargo-dist
sign/notary/repack/checksum round-trip as described in `MASTER_PLAN.md`.

### Output stability gates

These protect the auto-run experience (open terminal → table renders → prompt is free immediately). A regression here breaks the core UX promise.

- **T.S.1 — Line count**: `report --ascii` must not grow in line count. New rows are only allowed when they're conditional (e.g. ZFS Health only renders when `zpool` exists; battery health enriches an existing row in place; encryption row only on Windows when readable).
- **T.S.2 — Speed**: `tr300 --fast` wall-clock must not regress more than 100 ms on any platform. Capture before/after numbers in the PR description.

## Manual cross-platform matrix

The "Last verified" column tracks which release confirmed each row. Update as part of each PR's documentation block (F-tasks).

| Platform | Required checks | Last verified |
|---|---|---|
| **macOS Intel (Sonoma 14.x)** | OS shows "macOS 14.x"; CPU brand contains "Intel"; uptime present; battery on laptop | — |
| **macOS Apple Silicon M1** | CPU brand "Apple M1/Pro/Max" matches; nonzero native frequency when the OS exposes one; arm64 arch; cores show P/E split | — |
| **macOS Apple Silicon M2** | Native full/fast table+JSON; model/build/boot/display/FileVault/battery parity; P/E topology; memory definitions; privacy keys; speed | v4.0.0 — MacBook Pro M2, Mac14,7, macOS 26.3.1 (25D2128) |
| **macOS Apple Silicon M3 / M4** | CPU brand exact (no "Apple M1" stale); cores P/E; Mac marketing name correct; battery health present | — |
| **macOS Apple Silicon under Rosetta 2** | Arch shows host+process scopes; native profiler preserves model/display/battery; translated compatibility frequency stays null | v4.0.0 — x86_64 release binary under Rosetta on Mac14,7 |
| **Ubuntu 22.04+ (systemd-resolved)** | DNS row shows upstream resolvers, NOT 127.0.0.53 | — |
| **Debian 12 (no systemd-resolved)** | DNS row shows /etc/resolv.conf contents | — |
| **Fedora / Arch** | Hypervisor "None" on bare metal; terminal detection works for Konsole + GNOME Terminal + Wezterm | — |
| **Alpine in Docker** | Container detected; no panic on missing `lspci` / `lastlog` / systemd | — |
| **Raspberry Pi 4 (aarch64)** | CPU brand from devicetree, not empty | Post-v4.0.1 personal-hardware task open |
| **AWS EC2 (Graviton or Intel)** | Hypervisor shows "amazon" / "kvm"; cloud detection works | — |
| **WSL2 on Win11** | Hypervisor shows "WSL2"; terminal shows "Windows Terminal" via WT_SESSION | — |
| **Windows 11** | OS shows "Windows 11" (not 10); arch correct; last-login covers session start; battery on laptop | Personal Alienware post-v4.0.1 retest open; prior 3.10.0 evidence remains historical |
| **Windows 11 (BitLocker / Device Encryption ON)** | "Encryption" row shows "BitLocker On" non-admin if readable; full method when elevated | — |
| **Windows 11 (BitLocker OFF)** | "Encryption" row shows an evidence-backed Off state or remains absent; no promise that elevation alone unlocks it | — |
| **Windows 11 as Administrator** | Encryption shows evidence-backed method + protection level when available; no blanket elevation footer | — |
| **Linux as root (sudo)** | Motherboard, BIOS, RAM slot rows present; **footer hint absent** | — |
| **Linux as user (no sudo)** | Motherboard / BIOS / RAM rows absent; one-line footer hint visible (full mode); footer ABSENT in `--fast` | — |
| **Windows 11 ARM** | Arch via IsWow64Process2 correct under both x64 and ARM64 native processes | — |
| **Windows 10 (no Fast Startup)** | No spurious session-suffix on uptime | — |
| **Windows 11 with Fast Startup** | Uptime annotated with `(session: Xh)` when divergent | — |
| **Bare desktop (no battery)** | Battery row absent — must NOT show "0%" or stub row | — |
| **ZFS host** | ZFS Health row appears with "ONLINE"; "DEGRADED" if pool degraded | — |
| **Non-ZFS host** | ZFS Health row absent | 3.10.0 (Windows; absent as expected) |
| **Multi-homed Linux (Ethernet + Wi-Fi + VPN)** | Local IP matches default route, not first in list | — |
| **macOS with VPN active** | Local IP shows VPN tun if it's the primary; DNS shows VPN-pushed servers via scutil | — |
| **Windows with VPN active** | Local IP via GetBestInterfaceEx matches default route | — |
| **RDP session on Windows 11** | Last-login shows session connect time, not stale boot time | — |

## Per-release verification log

Append a section per tagged release noting which matrix rows were re-verified and on which hardware. Lets us catch silent regressions when a row stops being checked.

### v3.10.0 — 2026-04-27

Foundation scaffolding only — no collector changes. Verified:

- Windows 11 (build 26200): footer hint renders below table in full mode; absent in `--fast`; ANSI dim escapes when colors enabled. JSON contains `schema_version: 1`, `elevated: false`, `elevation_unlocks_more: true`. `--no-elevation-hint` suppresses the line cleanly.
- Library tests: 15 passed (8 pre-existing + 7 new for elevation footer logic and schema version).

Pending hardware verification (no collector changes that would affect them, but matrix entries should be stamped on next per-platform PR): macOS Intel/AS, all Linux distros, WSL2.

### v3.14.0 — 2026-05-10

Cross-platform stability and action syntax pass. Verified on local macOS Apple
Silicon during implementation; Linux and Windows hardware-specific behavior is
fixture-covered locally and left to the GitHub Actions matrix / real machines
for runtime validation.

- **Action syntax** — unit tests cover `tr300 update`, `tr300 --update`,
  `tr300 --json update`, `tr300 install`, `tr300 uninstall`, and mixed-action
  rejection.
- **Collector stability** — subprocess helper tests cover success and timeout
  behavior; collector parser fixtures cover macOS battery/sysctl/vm_stat/scutil,
  Linux resolver/route/ZFS/dmidecode paths, and Windows PowerShell fallback
  JSON on Windows CI.
- **Output stability** — integration tests parse JSON with `serde_json`, verify
  fixed-width ASCII table rows, assert `--fast` omits slow conditional rows, and
  confirm help documents both action forms.
- **Local gate** — `cargo fmt -- --check`, `cargo clippy --all-targets
  --workspace -- -D warnings`, `cargo test --workspace --all-targets`, and
  `cargo build --release --workspace` pass on this Mac.
- **Runtime smoke** — `./target/release/tr300 --fast --json | python3 -m
  json.tool` parses successfully, and `./target/release/tr300 --ascii` renders
  the fixed-width report.
- **Fast timing** — sorted 7-run local macOS `--fast` times:
  `0.17, 0.18, 0.20, 0.21, 0.21, 0.23, 0.24` seconds; median `0.21s`.
- **CI verification** — `master` CI run 25642712712 passed fmt, clippy, tests,
  release builds, security audit, dist plan, and auto-run speed gates on macOS
  ARM, Linux, and Windows. Release run 25642853066 passed and published the
  v3.14.0 GitHub Release with 20 assets.
- **Deferred** — admin-only Windows RDP history is not implemented in this pass;
  current Windows elevation wording is limited to BitLocker status.

### v3.14.1 — 2026-05-11

Release confidence patch after the v3.14.0 CI fix-forward. No new runtime
collector or renderer behavior.

- **Latest pre-bump CI verification** — `master` CI run 25643018578 passed on
  commit `5709f9a` across fmt, clippy, tests, release builds, security audit,
  dist plan, and auto-run speed gates on macOS ARM, Linux, and Windows.
- **Local gate before release prep** — `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --workspace -- -D warnings`, and
  `cargo test --workspace --all-targets` passed on this Mac before bumping
  v3.14.1.
- **Release commit local gates** — repeated `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --workspace -- -D warnings`,
  `cargo test --workspace --all-targets`, and
  `cargo build --release --workspace` on the v3.14.1 commit. Runtime smoke
  also passed: `./target/release/tr300 --version` printed `tr300 3.14.1`,
  and `./target/release/tr300 --fast --json | python3 -m json.tool` parsed.
- **CI verification** — `master` CI run 25645894617 passed on commit
  `3328a8e` across fmt, clippy, tests, release builds, security audit, dist
  plan, and auto-run speed gates on macOS ARM, Linux, and Windows.
- **Release verification** — release.yml run 25645999755 passed plan, six
  target artifact builds, global artifacts, host, and announce jobs. The
  v3.14.1 GitHub Release is non-draft, non-prerelease, and published with
  20 cargo-dist assets.

### v3.14.2 — 2026-05-11

Crates.io publication, ND-style updater strategy chain, install/release docs,
and project identity cleanup. Runtime report collection/rendering behavior is
unchanged outside `tr300 update`.

- **Local release gates** — `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --workspace -- -D warnings`,
  `cargo test --workspace --all-targets`, `cargo package --locked --list`,
  `cargo publish --dry-run --locked`, and `~/.cargo/bin/dist plan` passed on
  this Mac before publishing. Package list contained 36 release files and
  excluded `.codex`, `.claude`, `.firecrawl`, `.github`, agent guides, and
  unrelated historical implementation files.
- **Update behavior tests** — unit tests cover cargo-first ordering on Unix and
  Windows, installer fallback ordering when cargo is unavailable, and JSON
  legacy `"method"` mapping vs precise `"strategy"` values.
- **CI verification** — `master` CI run 25647466576 passed on commit
  `a6c3841` across fmt, clippy, tests, release builds, security audit, dist
  plan, and auto-run speed gates on macOS ARM, Linux, and Windows.
- **Crates.io verification** — crates-publish run 25647553585 checked the
  exact CI-tested SHA, reran fmt/clippy/tests/package/dry-run, and published
  `tr-300` 3.14.2 to crates.io with license
  `PolyForm-Noncommercial-1.0.0`, binary `tr300`, and rust-version `1.95`.
  Initial run 25647407638 failed before publishing due the crates.io version
  check missing a descriptive data-access `User-Agent`; follow-up commit
  `a6c3841` fixed the workflow.
- **Release verification** — release.yml run 25647597021 passed plan, six
  target artifact builds, global artifacts, host, and announce jobs. The
  v3.14.2 GitHub Release is non-draft, non-prerelease, and published with
  20 cargo-dist assets.

### v3.15.1 — 2026-05-15

Patch — fix-forward of v3.15.0 release.yml WiX build failure. No runtime
behavior changes from v3.15.0.

**Local gates verified (Windows 11 25H2 build 26200.8457):**
- `cargo fmt --all -- --check` — clean
- `cargo clippy --all-targets --workspace -- -D warnings` — clean
- `cargo test --workspace --all-targets` — 79 passed (61 lib + 18 integration)
- `cargo package --locked --list` — 38 files (+1 from v3.15.0's 37 = `wix-corporate/corporate.wxs`)
- `cargo publish --dry-run --locked` — clean
- `cargo build --release --locked` — clean
- `./target/release/tr300 --version` → `tr300 3.15.1`

**WiX MSI builds verified locally with portable WiX 3.11.2.4516** —
downloaded `wix311-binaries.zip` from
[github.com/wixtoolset/wix3/releases/tag/wix3112rtm](https://github.com/wixtoolset/wix3/releases/tag/wix3112rtm),
extracted to a portable directory (no admin install):

- **Global MSI** via cargo-wix: `cargo wix --no-build --nocapture` →
  produces `target/wix/tr300-3.15.1-x86_64.msi` (1.9 MB), exit 0.
- **Corporate MSI** via bare WiX:
  `candle.exe -arch x64 -dVersion=3.15.1 -dCargoTargetBinDir=target/release ... wix-corporate/corporate.wxs`
  then `light.exe -sice:ICE38 -sice:ICE64 -sice:ICE91 -ext WixUIExtension ...`
  → produces `tr300-x86_64-pc-windows-msvc-corporate.msi` (1.9 MB),
  both exit codes 0.

**Path-classification unit tests** (from v3.15.0, unchanged): 4 tests
covering `classify_install_path()` for Program Files / LocalAppData /
.cargo\\bin / random paths all pass.

**Post-release CI verification (all five workflow runs green):**
- **master CI run 25902114185** on commit `b37e783` — fmt + clippy +
  tests + builds + speed gates green across Linux + macOS ARM + Windows
  (macOS ARM test flake non-blocking via existing `continue-on-error`
  knob from v3.14.5).
- **crates-publish run 25902206318** — published `tr300` 3.15.1 to
  crates.io at 2026-05-15 05:35:19 UTC from the CI-tested SHA.
- **release.yml run 25902253637** on tag `v3.15.1` — all 10 jobs green
  (plan + 6 build-local-artifacts + build-global-artifacts + host +
  announce); GitHub Release published 05:41:10 UTC with the initial 22
  cargo-dist assets (the Windows MSI build that failed for v3.15.0
  succeeded cleanly this time).
- **windows-installers.yml run 25902607841** — FAILED on the first
  workflow_dispatch retry with `candle.exe CNDL0103` because PowerShell
  parsed `-dVersion=3.15.1` as two tokens at the dots. Fixed in
  commit `5883627` by quoting all `-D` / `/D` define args.
- **windows-installers.yml run 25902740025** — SUCCESS on the second
  workflow_dispatch retry. Built and uploaded the 6 additional assets
  (Corporate MSI + Global EXE + Corporate EXE + their `.sha256`
  sidecars).

**Final v3.15.1 GitHub Release: 28 assets** (verified via
`gh release view v3.15.1`). All four first-class Windows installers
present (Global MSI, Corporate MSI, Global EXE setup, Corporate EXE
setup) plus 6 platform binary archives plus the cargo-dist
installer scripts plus legacy `tr-300-installer.*` aliases plus
source tarball plus metadata.

**Three fix-forward CI commits on master after the v3.15.1 release
commit `b37e783`:**
- `7715b93` — `windows-installers.yml` trigger swap (release.published
  → workflow_run + workflow_dispatch). Driven by v3.15.1 hitting the
  GITHUB_TOKEN loop-prevention rule that suppresses release.published
  downstream events.
- `5883627` — PowerShell `-D`/`/D` argument quoting fix. Driven by run
  25902607841 catching candle on `.15.1`.
- (Final docs commit recording all of the above)

**Pending verification (real Windows hardware):**
Same matrix as v3.15.0 (the 15-row install/upgrade/`tr300 update` grid
below). Now testable with actual installer downloads from the v3.15.1
GitHub Release.

### v3.15.0 — 2026-05-14

Four-installer Windows distribution model — adds Corporate MSI plus Global and
Corporate EXE installers (Inno Setup) to every release, with `tr300 update`
now MSI/EXE-aware via a `HKCU\Software\TR300\InstallSource` registry marker.
No runtime collector or renderer changes; the binary's behavior is identical
to v3.14.5 outside the update flow.

**Local gates verified:**
- `cargo check --all-targets` — clean (added `winreg = "0.52"` dependency).
- `cargo clippy --all-targets --workspace -- -D warnings` — clean.
- `cargo test --workspace --all-targets` — 61 library tests + 18 integration
  tests + 0 doc tests = 79 passed. New tests in `src/update.rs`:
  `install_origin_classify_program_files_is_msi_global`,
  `install_origin_classify_localappdata_is_msi_corporate`,
  `install_origin_classify_cargo_bin_is_cargo_or_installer`,
  `install_origin_classify_random_path_is_unknown`,
  `install_origin_json_ids_are_kebab_case`,
  `new_strategies_have_stable_json_ids`,
  `new_strategies_have_distinct_labels`, plus extended assertions in
  `json_method_maps_to_legacy_taxonomy` covering the four new variants.
- `cargo fmt --all -- --check` — clean.

**Manual install/upgrade matrix (Windows 11 25H2 build 26200.8457):**
The 4-installer × {fresh install, in-place upgrade, `tr300 update`} grid.
Rows marked "Pending" need a published v3.15.0 release to test against (the
update flow needs a `latest` to compare to). The local-build cases verify the
installer formats can be produced and run end-to-end on this host before
release.

| # | Scenario | Expected | Status |
|---|---|---|---|
| 1 | MSI Global fresh install (admin) | UAC → `C:\Program Files\tr300\bin\`. ARP shows "tr300 3.15.0". Registry marker = `msi-global`. | Pending hardware test |
| 2 | MSI Corporate fresh install (non-admin) | No UAC → `%LocalAppData%\Programs\tr300\bin\`. ARP shows "tr300 (Corporate Edition) 3.15.0". Marker = `msi-corporate`. | Pending hardware test |
| 3 | EXE Global fresh install (admin) | UAC → `C:\Program Files\tr300\bin\`. ARP shows "tr300 3.15.0" (Inno Setup product). Marker = `exe-global`. | Pending hardware test |
| 4 | EXE Corporate fresh install (non-admin) | No UAC → `%LocalAppData%\Programs\tr300\bin\`. ARP shows "tr300 (Corporate Edition) 3.15.0". Marker = `exe-corporate`. | Pending hardware test |
| 5–8 | In-place upgrade (3.15.0 → 3.15.1) of each | WiX `MajorUpgrade` (MSI) / Inno Setup AppId match (EXE) silently uninstalls old + installs new. | Pending v3.15.1 |
| 9–12 | `tr300 update` from each install | Reads registry → matching strategy → downloads installer → runs it. | Pending v3.15.1 |
| 13 | `tr300 update` on `cargo install` install (regression) | Path-based fallback returns CargoOrInstaller; legacy chain runs. Unchanged behavior. | ✓ unit-verified (`install_origin_classify_cargo_bin_is_cargo_or_installer` test) |
| 14 | Coexistence (install MSI + EXE of same edition) | Two ARP entries, last-installed marker wins. Documented in README as "pick one". | Pending hardware test |
| 15 | CI green | ci.yml passes on the release commit pre-tag. After tag push: release.yml uploads 22 cargo-dist assets; windows-installers.yml uploads 6 more (Corp MSI + Global EXE + Corp EXE + their .sha256). Total 28 release assets. | Pending tag push |

**Path-classification unit test coverage** (`src/update.rs::classify_install_path`):

```rust
classify("C:\\Program Files\\tr300\\bin\\tr300.exe")              → MsiGlobal
classify("c:\\PROGRAM FILES\\tr300\\BIN\\tr300.exe")              → MsiGlobal (case-insensitive)
classify("C:\\Users\\alice\\AppData\\Local\\Programs\\tr300\\bin\\tr300.exe") → MsiCorporate
classify("C:\\Users\\alice\\.cargo\\bin\\tr300.exe")              → CargoOrInstaller
classify("D:\\portable\\tr300\\tr300.exe")                         → Unknown
classify("C:\\Users\\alice\\Downloads\\tr300.exe")                 → Unknown
```

**Pending verification (post-release, real Windows hardware):**
- All four installers actually produce valid Windows Installer / Inno Setup
  artifacts in the CI build. Local verification deferred to the first CI run
  on the feature branch.
- UAC prompt behavior matches scope on real hardware (Global → prompts,
  Corporate → does not).
- SmartScreen "Windows protected your PC" on a fresh download of each EXE
  installer (unsigned binary). User clicks "More info → Run anyway" path
  documented in README.
- `gh release upload --clobber` re-run idempotency in the windows-installers.yml
  workflow (verified by simulating a failed-mid-flight upload).
- WiX MajorUpgrade across the v3.14.x perMachine MSI → v3.15.0 perMachine MSI
  upgrade. The UpgradeCode `5CD540A8-AD16-4B0F-8CE4-51FF641DE181` is unchanged
  from v3.13.1+, so MajorUpgrade should silently replace.

### v3.14.5 — 2026-05-14

Windows install error advisor + Display-formatted main()-level errors.
Verified on the same Windows 11 25H2 (build 26200.8457) host as v3.14.4,
unelevated user session.

- **Permission-denied write path.** Reproduced via
  `attrib +R "$PROFILE"` to force `fs::write` to fail with Windows
  error 5. `tr300 install` output (excerpt):
  ```
  tr300 install: Can't write to your PowerShell profile.

    Path:  C:\Users\hey\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1
    Cause: Access is denied. (os error 5) (Windows error 5)

  Likely reasons (most common first):
    - Your organization restricts writes via Intune MDM, Active Directory Group
        Policy, AppLocker, or Windows Defender Application Control (WDAC). Ask
        IT to allow writes to:
            C:\Users\hey\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1
    - Antivirus / EDR (Defender, CrowdStrike, SentinelOne, etc.) is treating the
        profile edit as suspicious. Add an exclusion for the path above.
    - The file or folder is owned by another user or by SYSTEM. From an admin
        PowerShell you can re-take ownership:
            takeown /F "..." /R

  Manual `tr300` still works from the prompt; only the auto-run on new shells is
  affected. After addressing the cause above, re-run `tr300 install`.

  Error: Platform operation failed: write profile: Access is denied. (os error 5)
  ```
  Exit code: 1. After `attrib -R` the next `tr300 install` succeeded.
- **OneDrive-vs-plain path branch.** Pure-function unit tests verify
  `looks_like_onedrive_path()` matches `\OneDrive\`,
  `\OneDrive - Acme Corp\`, and case-variant forms; non-OneDrive paths
  return false. The dispatch logic in `fail_install()` is direct
  conditional logic from there.
- **Happy path didn't regress.** `tr300 install` with the file writable
  produced the same "Modified PowerShell profile:" / "Installation
  complete!" output as v3.14.4, exit code 0.
- **Local gates.** `cargo fmt -- --check`,
  `cargo clippy --all-targets --workspace -- -D warnings`,
  `cargo test --workspace` (54 lib + 18 integration + 1 doc — up from
  v3.14.4's 47 lib via 7 new path-inspection tests) all passed on this
  host.
- **Release pipeline (with macOS ARM workaround).** Six commits between
  the initial release commit and the green CI:
  - `06161b2` initial release commit — CI failed on Release Build (macOS
    ARM) at 0s, Test (macOS ARM) passed
  - `ff199d5` empty-commit retrigger — Test + Release Build (macOS ARM)
    both failed at 0s
  - `be8f2a0` empty-commit retrigger — Test (macOS ARM) failed at 0s,
    Release Build (macOS ARM) succeeded
  - `158dc2e` reverted the v3.14.5 main.rs Display-rendering change to
    isolate whether it was code-correlated; same Test (macOS ARM) failed
    at 0s pattern, Release Build (macOS ARM) succeeded — confirmed
    infrastructure, not code
  - `667e466` added `continue-on-error: ${{ matrix.os == 'macos-latest'
    }}` to ci.yml test + build matrix entries; workflow conclusion
    still failed because Auto-run speed (macOS ARM) also failed
    without the knob
  - `a21a4d1` extended the same knob to the speed job; **final green
    CI** (workflow conclusion = success despite macOS ARM Test +
    Auto-run-speed individual failures)
  - CI run 25850693664 succeeded; crates-publish run 25850823118
    published `tr300 3.14.5` to crates.io; tag `v3.14.5` push triggered
    release.yml run 25850864213 which built all 22 artifacts. GitHub
    Release v3.14.5 published non-draft non-prerelease.
  - The 0-second cargo abort pattern (cache restore succeeds, then
    cargo itself exits instantly without producing compile output) is
    structurally not a code-compilation failure. Linux + Windows green
    on every retry. v3.14.4 on a structurally similar tree passed
    cleanly hours earlier.

### v3.14.4 — 2026-05-14

Windows `tr300 install` execution-policy preflight. Verified on the same Windows
11 25H2 (build 26200.8457) host the user reported the original failure on,
under a non-admin session.

- **Reproduce the broken state** — `powershell -NoProfile -Command "Get-ExecutionPolicy -List"`
  returned `Undefined` at every scope (the resolved effective policy is
  `Restricted` on Windows Client when all scopes are Undefined). This matches
  the exact fresh-machine state the user reported.
- **Auto-fix path** — `./target/release/tr300.exe install` printed
  `Set PowerShell CurrentUser execution policy: Undefined -> RemoteSigned`,
  followed by the existing "Modified PowerShell profile:" /
  "Installation complete!" output. `Get-ExecutionPolicy -Scope CurrentUser`
  returned `RemoteSigned` after the install.
- **Fresh shell loads the profile** — `powershell -Command "exit 0"` (full
  profile load, no `-NoProfile`) printed the TR-300 fast-mode auto-run table
  with no `UnauthorizedAccess` / PSSecurityException. The exact failure mode
  the user reported is fixed end-to-end.
- **Idempotency** — re-running `tr300 install` with the policy already at
  `RemoteSigned` produced no policy-change message and no duplicated
  `# TR-300` markers in `$PROFILE`.
- **AllSigned not-downgraded** — set `CurrentUser` to `AllSigned`, ran
  `tr300 install`, observed the warning text ("PowerShell CurrentUser
  execution policy is 'AllSigned' — TR-300 will not change this." plus the
  remediation options) and confirmed `Get-ExecutionPolicy -Scope CurrentUser`
  was *still* `AllSigned` afterwards. The alias-write half still succeeded
  ("Installation complete!" printed); the auto-run won't fire under
  `AllSigned` without signing, as documented.
- **Local gates** — `cargo fmt -- --check`,
  `cargo clippy --all-targets --workspace -- -D warnings`, and
  `cargo test --workspace` (47 lib + 18 integration + 1 doc) passed on this
  Windows host after the change. The clippy pass required moving the
  pre-existing `powershell_fallback_tests` module to the end of
  `src/collectors/platform/windows.rs` to satisfy
  `clippy::items_after_test_module`; the previous structure had the test
  module mid-file with ~270 lines of non-test items after it, and that lint
  had never tripped CI because clippy runs Linux-only and the file is
  `#[cfg(target_os = "windows")]`-gated.
- **GPO-locked path** — not verified on this non-domain machine. The
  fallback warning text is exercised at the unit level via the AllSigned
  path (same `TrySetResult::StillBlocked` rendering).
- **Release pipeline verified end-to-end.** CI run 25848439537 succeeded
  on commit `ac3fd34` across all 13 jobs (fmt, clippy, audit, dist-plan,
  tests on three platforms, release builds on three platforms, auto-run
  speed gates on three platforms). The previous commit `35fb65a` (a
  docs-only commit) had failed on macOS ARM in a flaky way; my commit's
  green macOS ARM result confirmed it was transient runner noise, not a
  code regression. Crates.io publish run 25848562250 then published
  `tr300 3.14.4` to crates.io from that same SHA. Tag `v3.14.4` push
  triggered release.yml run 25848716551 which built six target binaries
  (Linux x64 gnu/musl, Linux ARM64 gnu, macOS Intel, macOS ARM, Windows
  x64) plus the MSI installer and the shell/PowerShell installer
  scripts. GitHub Release published non-draft, non-prerelease with 22
  assets including the legacy `tr-300-installer.*` aliases for
  v3.14.2-and-earlier updater compatibility. `tr300 update --json` from
  the local v3.14.4 binary reported `latest_version=3.14.4` /
  `update_available=false`, confirming the release is discoverable via
  the GitHub API.

### v3.14.3 — 2026-05-11

Canonical crates.io package rename from the deleted `tr-300` package name to
`tr300`, plus matching self-update, library import path, installer URL, MSI
name, and release documentation updates.

- **Crates.io availability check** — `https://crates.io/api/v1/crates/tr300`
  returned 404 before release, confirming the corrected package name is
  available for creation.
- **Local release gates** — `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --workspace -- -D warnings`, and
  `cargo test --workspace --all-targets` passed on this Mac after the package
  rename. Test counts: 38 library tests and 18 integration tests.
- **Package verification** — `cargo package --locked --allow-dirty --list`
  listed the expected 36 release files, and
  `cargo publish --dry-run --locked --allow-dirty` packaged and verified
  `tr300 v3.14.3` successfully before the release commit. After committing,
  the strict `cargo package --locked --list` and
  `cargo publish --dry-run --locked` gates also passed.
- **cargo-dist verification** — `dist plan` passed and announced canonical
  `tr300-*` release artifacts, including `tr300-installer.sh`,
  `tr300-installer.ps1`, six platform archives, the Windows MSI, checksums,
  and source tarball. The checked-in release workflow adds legacy
  `tr-300-installer.*` aliases for v3.14.2 updater compatibility.
- **CI verification** — `master` CI run 25648618096 passed on commit
  `25305d8` across fmt, clippy, tests, release builds, security audit, dist
  plan, and auto-run speed gates on macOS ARM, Linux, and Windows.
- **Crates.io verification** — crates-publish run 25648707510 checked the
  exact CI-tested SHA, reran fmt/clippy/tests/package/dry-run, and published
  `tr300` 3.14.3 to crates.io with license
  `PolyForm-Noncommercial-1.0.0`, binary `tr300`, library target `tr300`,
  and rust-version `1.95`.
- **Release verification** — release.yml run 25648740343 passed plan, six
  target artifact builds, global artifacts, host, and announce jobs. The
  v3.14.3 GitHub Release is non-draft, non-prerelease, and published with
  22 cargo-dist assets: canonical `tr300-*` archives/installers/checksums,
  source assets, `dist-manifest.json`, and the legacy
  `tr-300-installer.sh` / `tr-300-installer.ps1` aliases.

### v3.11.0 — 2026-04-27

Windows accuracy + BitLocker (PR #4). Verified on Windows 11 25H2 (build 26200.8246), unelevated user session:

- **OS row** — was `Windows 11 (26200)`, now `Windows 11 25H2`. Registry-based detection working.
- **Kernel row** — was `26200`, now `26200.8246` (full build with UBR).
- **Last-login row** — was `Login tracking unavailable`, now real timestamp `Tue Apr 21 22:12` (matches uptime). WTSLogonTime returned 0 (console session quirk); fell back to GetTickCount64-derived boot time as designed.
- **CPU freq row** — still `1.4 GHz` on this host (machine is power-plan throttled at 1400 MHz; CPUID leaf 16h returns 0 on Meteor Lake; CallNtPowerInformation correctly reports 1400 MaxMhz). Implementation correct; will show higher values on machines with full performance power plan or older Intel chips where leaf 16h works.
- **Hypervisor row** — was `Hypervisor Present`, now `Bare Metal (Hyper-V/VBS)`. CPUID returned `Microsoft Hv` correctly; SMBIOS manufacturer disambiguated to "physical host with VBS active".
- **Encryption row** — absent on this user's unelevated session (Win32_EncryptableVolume returned access-denied as expected). Footer hint covers the gap. Will surface on Win11 Device Encryption laptops and admin sessions.
- **Architecture row** — `x86_64` (unchanged on x64 host running x64 binary; IsWow64Process2 implementation will activate on ARM64 hosts).
- **Footer hint** — still renders correctly with the BitLocker mention; wording was later narrowed to implemented BitLocker-only elevated data.
- Integration tests: 13 passed (1 new for JSON `encryption` key); library tests: 15 passed.

Pending verification (deferred or platform-locked):
- Windows 11 ARM64 host (C.2 IsWow64Process2 emulation annotation)
- Windows 11 with admin shell (BitLocker full method visible)
- Windows 11 with Device Encryption ON, unelevated (BitLocker row should appear)
- Windows 11 in a real Hyper-V VM (CPUID `Microsoft Hv` + Microsoft Corp manufacturer → `Hyper-V`, not `Bare Metal (Hyper-V/VBS)`)
- Windows running inside KVM / VMware / VirtualBox (CPUID-based hypervisor brand detection)

### v3.12.0 — 2026-04-28

Windows accuracy refinements (PR #4b). Verified on Windows 11 25H2 (build 26200.8246), unelevated user session:

- **MACHINE IP / DNS IP rows (C.4)** — `GetBestInterfaceEx`-driven adapter selection working. With no VPN active, `MACHINE IP` resolved to `10.1.0.85` (LAN adapter selected as default route by the kernel — correct). `DNS IP 1` resolved to `10.1.0.1` (LAN gateway DNS — correct). Falls through to legacy first-match order when `get_best_route_interface_index()` returns `None`. JSON output includes `network.machine_ip` and `network.dns_servers[]` populated by the same path.
- **UPTIME row (C.5)** — `detect_fast_startup()` correctly read `HiberbootEnabled = 1` from registry. `last_cold_boot_seconds()` parsed `Win32_OperatingSystem.LastBootUpTime` via `wmi::WMIDateTime` (early hand-written CIM datetime parser was discarded after testing — wmi crate's serde wrapper handles the format natively). On this session the cold-boot time and kernel session age aligned within 1 hour, so the parenthetical annotation correctly stayed dormant. The `(session: …)` annotation will activate on hosts where Shut Down + Boot used Fast Startup hibernation resume (annotation appears when divergence > 1h).
- **JSON `os.session_uptime_seconds` key** — present in every output, nullable per design. New integration test `test_json_includes_session_uptime_seconds_key` pins the contract.
- **`--fast` median timing** — unchanged from v3.11.x baseline (~308 ms). Phase B is full-mode-only; the C.5 WMI cold-boot query is gated on `mode == CollectMode::Full`.
- Integration tests: 14 passed (1 new for `os.session_uptime_seconds`); library tests: 15 passed.

Pending verification (deferred or platform-locked):
- Windows 11 with active VPN client (Tailscale, WireGuard, OpenVPN, Cisco AnyConnect): `MACHINE IP` should swap between LAN address and tunnel address as VPN toggles on/off
- Windows 11 host immediately after Shut Down + Boot (Fast Startup hibernation resume): `UPTIME` row should display `(session: …)` annotation with kernel-session age shorter than cold-boot age
- Windows 11 host immediately after Restart (cold boot, bypasses Fast Startup): annotation should NOT appear (sysinfo uptime ≈ WMI cold-boot time)
- Windows host with `IP Helper` service disabled: `GetBestInterfaceEx` should fail and the function should fall through to legacy first-match adapter ordering

### v3.13.0 — 2026-04-28

PR #5 partial — Windows polish. Verified on Windows 11 25H2 (build 26200.8246), unelevated user session, fully-charged Alienware laptop on AC:

- **BATTERY row (C.10 + C.10b)** — was `100% (Discharging (High))` (legacy WMI `Win32_Battery`'s confusing `BatteryStatus` mapping), now `AC Power` (clean: percentage at full charge is uninformative, just shows AC state). Native `GetSystemPowerStatus` call confirmed working — no COM round-trip overhead. The 5-state model also covers gaming-laptop "PSU undersized for peak GPU draw" → `X% (Plugged in)`, firmware-limited charging (ThinkPad battery longevity) → also `X% (Plugged in)`, and the historical off-AC `X% (Discharging)` / `(Critical)` / `(Low)` states.
- **CORES row (C.9)** — value unchanged (1 socket on this single-package CPU), but now via native `GetLogicalProcessorInformationEx` walking variable-length `SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX` records. ~10x faster than the WMI path it replaced.
- **GPU rows (C.8)** — three hardware adapters detected: Intel Arc Graphics, NVIDIA GeForce RTX 4070 Laptop GPU, Trigger 6 External Graphics. No "Microsoft Basic Render Driver" or other software adapters (registry-prefer path doesn't enumerate them; `filter_software_gpus` name-based filter is the second line of defense).
- **SHELL row (C.11)** — `bash` (we're in Git Bash). PSCore detection fell through correctly (no PowerShell 7+ installed on this host); legacy WinPS-5.x path works as before. The PSCore detection logic was unit-verified by inspecting the `reg query` output format.
- **TERMINAL row (C.12)** — was `Console`, now `Claude Code`. Parent-process walk via Toolhelp32 correctly traversed `tr300.exe → bash.exe → claude.exe` and matched the "Claude Code" label. Verified by manual `Get-Process` parent-walk in PowerShell which produced the same chain.
- **Elevation footer** — the Windows admin hint still renders correctly in v3.13.0 (feature shipped v3.10.0, no regression; wording was later narrowed to BitLocker-only). Suppressed by `--no-elevation-hint` flag and never rendered in `--fast` mode.
- **JSON additive keys** — `cpu.gpus` and existing keys all present; no new top-level keys added in v3.13.0 (no schema bump per MASTER_PLAN §4.9).
- **`--fast` median timing** — 338 ms (sorted-7 middle: 0.332, 0.333, 0.337, 0.338, 0.339, 0.376, 0.394). +30 ms vs v3.11 baseline ~308 ms. Within 100 ms budget per MASTER_PLAN §5; well under 1500 ms CI gate. Slight regression attributed to additional winapi feature bindings linked into the binary; full-mode collectors are equal-or-faster (C.9 saves ~30 ms via native socket count, C.10 saves ~40 ms via native battery, neither on the fast path).
- Integration tests: 14 passed (no new tests in v3.13.0; the existing test suite covers the additive changes); library tests: 15 passed.

Pending verification (deferred to future sessions):
- Windows 11 host with PowerShell 7+ installed (`HKLM\SOFTWARE\Microsoft\PowerShellCore\InstalledVersions\<GUID>` populated): SHELL row should show `PowerShell 7.x.y` instead of falling back to legacy 5.x detection
- Windows 11 host launched directly from Windows Terminal (without env-var inheritance loss): `WT_SESSION` env var path should match before the parent walk runs; verifies the env-var fast path
- Windows 11 host launched from WezTerm / Alacritty / Cursor / Tabby / Hyper / Ghostty / Kitty: parent walk should match the respective terminal label
- Gaming laptop running an active GPU-heavy load that exceeds AC brick wattage: BATTERY row should show `X% (Plugged in)` with percentage decreasing over time (validates the C.10b heuristic for the supplementing-from-battery case)
- ThinkPad / ASUS with battery-longevity firmware mode capping charge at 60-80%: BATTERY row should also show `X% (Plugged in)` (the same heuristic catches both the "PSU undersized" and "firmware limit" cases since they're indistinguishable from a single-snapshot SYSTEM_POWER_STATUS)
- Windows host with admin shell: BATTERY / CORES / GPU rows unchanged (no admin-gated behavior in v3.13.0)

### v3.13.1 — 2026-04-29

Release infrastructure fix (task #54). No runtime behavior changes — the binary is byte-identical to v3.13.0 modulo the version string. Verified on Windows 11 25H2 (build 26200.8246), unelevated user session:

- **Local build sanity** — `cargo fmt --check`, `cargo clippy --all-targets --workspace -- -D warnings`, `cargo test --workspace --all-targets`, `cargo build --release --workspace` all green. `target/release/tr300 --version` reports `3.13.1`. `--fast --json | head -5` parses; `--ascii` renders identically to v3.13.0.
- **`rust-toolchain.toml` doesn't break local development** — the file pins to `1.95`, the same minor that `Cargo.toml`'s `rust-version` already declares. Existing rustup-managed toolchains on the dev host satisfy the pin transparently.

Post-tag verification (run 25096833278 on tag `v3.13.1` → `086ef0a`):

- **`release.yml` succeeded across all 10 jobs** (vs 3/6 on v3.13.0):
  `aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `x86_64-pc-windows-msvc` all green; `plan` + `build-global-artifacts` + `host` + `announce` no longer skipped.
- **`gh release view v3.13.1 -R QubeTX/qube-machine-report`** returns the v3.13.1 release with **20 assets**:
  6 platform binaries as `.tar.xz` + matching `.sha256` (12 files), Windows `.zip` + `.sha256` + `.msi` + `.msi.sha256` (4 files), `source.tar.gz` + `.sha256`, `dist-manifest.json`, `sha256.sum`, `tr-300-installer.sh`, `tr-300-installer.ps1`. **First successful GitHub Release publication since v3.10.0** — the README installer one-liner is unblocked.
- **Fix-forward note:** The first attempt (`c2e6a65`) had only `channel = "1.95"` and CI's Format + Clippy jobs failed with `error: 'cargo-fmt' is not installed for the toolchain '1.95-x86_64-unknown-linux-gnu'`. Root cause: rustup processes a rust-toolchain.toml with only the default profile (rustc + cargo + rust-std) and ignores any action-level `components:` field passed to `dtolnay/rust-toolchain@stable`. Resolved by `086ef0a` which adds `components = ["rustfmt", "clippy"]` to the file. Documented inline in `CLAUDE.md` § "MSRV policy v3.13.1+" and the auto-memory release-process reference. Anyone touching rust-toolchain.toml in the future MUST keep the components list populated unless they also remove the rustfmt/clippy CI jobs.
