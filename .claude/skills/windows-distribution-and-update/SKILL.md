---
name: windows-distribution-and-update
description: TR-300 Windows packaging + self-update edit-time rules. Load BEFORE editing the installers, the release packaging, or the updater — `wix/main.wxs`, `wix-corporate/corporate.wxs`, `inno/global.iss`, `inno/corporate.iss`, `.github/workflows/windows-installers.yml`, `.github/workflows/release.yml`, the `Cargo.toml` dist / allow-dirty / include config, or `src/update.rs` (install-origin detection, the registry `InstallSource` marker, MSI/EXE update strategies, SHA256 verification, post-install version check, `is_newer` semver, auto-rustup, macOS/Linux update chain). Covers the four-installer model, the four PERMANENT product GUIDs, and the registry-marker contract that ties installers to the updater. Triggers on "installer", "MSI", "EXE installer", "WiX", "candle", "light", "Inno Setup", "self-update", "tr300 update", "install origin", "the four GUIDs", "release.yml", "windows-installers.yml". These contracts span three files each and break in-place upgrades if desynced — do not regenerate GUIDs or rename marker strings.
---

# TR-300 Windows distribution + self-update edit-time rules

Operational rules for how TR-300 is packaged on Windows and how it updates itself. Packaging and
self-update live in one skill because they share the registry `InstallSource` marker contract:
the installers **write** it, `src/update.rs` **reads** it to decide which installer to re-fetch.
Long-form **why** (why two MSIs, why two EXEs, rejected alternatives, the v3.15.0→v3.15.1
post-mortem) is in
[`docs/architecture-decisions.md` § "Windows distribution model (v3.15.0+)"](../../../docs/architecture-decisions.md#windows-distribution-model-v3150)
and § "Install / update safety primitives"; the tripwire summary is in [`CLAUDE.md`](../../../CLAUDE.md).

## Self-Update (`tr300 update` / `--update`)

`src/update.rs` checks `https://api.github.com/repos/QubeTX/qube-machine-report/releases/latest` (15s timeout via `ureq`), compares against `VERSION` from `Cargo.toml`, then dispatches by detected install origin.

**Windows (v3.15.0+).** `detect_install_origin()` reads `HKCU\Software\TR300\InstallSource` (written by all four first-class installers — MSI Global, MSI Corporate, EXE Global, EXE Corporate) and returns a single matching strategy. No cross-fallback: re-running a different product would create coexistence problems (two ARP entries, PATH ordering wins). The MSI strategies download the matching `.msi` to `%TEMP%` and run `msiexec /i /passive /norestart`; the EXE strategies download the matching Inno Setup `.exe` and run it with `/SILENT /SUPPRESSMSGBOXES /NORESTART`. If the marker is missing (legacy pre-v3.15.0 install), `classify_install_path()` falls back to substring-matching the running EXE path: `\Program Files\tr300\` → `MsiGlobal`, `\AppData\Local\Programs\tr300\` → `MsiCorporate`, `\.cargo\bin\` → `CargoOrInstaller` (legacy chain), anything else → `Unknown` (legacy chain). Each install path is mirrored exactly between MSI and EXE for that edition; the registry marker is the only way to distinguish them on update.

**macOS/Linux.** Legacy probe-and-retry chain (unchanged):
- `cargo install tr300 --force` first when `cargo --version` succeeds
- Fallback: cargo-dist shell installer via `curl`, then `wget`

The legacy Windows chain (`powershell` → `pwsh` running `irm $URL | iex`) still runs for `CargoOrInstaller` / `Unknown` install origins — users who installed via the PowerShell one-liner or `cargo install` keep updating the way they always have.

Do not restore executable-path detection as the **primary** discriminator. The registry marker is authoritative; path-based detection is the legacy fallback only.

`tr300 update --json`, `tr300 --json update`, and `tr300 --update --json` emit a single JSON object with `current_version`, `latest_version`, `update_available`, `success`, and (on Windows) a top-level `install_origin` field. Values: `msi-global`, `msi-corporate`, `exe-global`, `exe-corporate`, `cargo-or-installer`, `unknown`. Successful updates include legacy `"method"` (`cargo` / `installer`) plus precise `"strategy"` (`msi_global`, `msi_corporate`, `exe_global`, `exe_corporate`, or the legacy IDs); failures include an `"attempts"` array. Exit codes: `0` success, `2` failure.

**Auto-rustup on the cargo strategy (v3.11.1+).** Before `cargo install tr300 --force`, `try_strategy(UpdateStrategy::Cargo)` calls `rustup_update_stable_best_effort()`: probe `rustup --version` with stdout/stderr → `Stdio::null()`; if found, run `rustup update stable` and print `Updating Rust toolchain (rustup update stable)…`. **Any failure is non-fatal** — `let _ =` the result, fall through to the cargo install. Installer strategies never touch Rust because they download prebuilt binaries. Don't replace this best-effort pattern with hard-failing or with `rustc --version` probing + conditional rustup — the rationale (failure-mode prevented, distro-managed toolchain compatibility, simplicity) is in the decisions doc.

**SHA256 verification + post-install version check (v3.15.4+).** The Windows MSI / EXE update paths now have two load-bearing safety checks:

- `verify_checksum(installer_path, installer_url)` runs after `download_to_file` and before invoking msiexec / the Inno EXE. It fetches `{installer_url}.sha256` in a separate request (cargo-dist publishes the sidecar in `<lowercase-64-char-hex>  *<filename>` format — the parallel implementation in `.github/workflows/windows-installers.yml:213-220` writes the same form), parses via `parse_sha256_sidecar` (tolerant of the asterisk being present or absent, case-insensitive), computes SHA-256 of the downloaded file via `sha2::Sha256`, and refuses to launch the installer on mismatch. This defends against TLS-interception proxies with a trusted root CA (corporate IT), captive portals, and CDN tampering. Don't skip the verify — TLS-to-github.com alone is insufficient against the corporate proxy threat model.
- `verify_post_install(latest)` runs after the installer reports exit code 0. It re-execs `env::current_exe() --version`, parses the version, compares against the expected `latest` (both stripped of prerelease/build-metadata suffix via `strip_prerelease_metadata`). A mismatch produces `StrategyError::Runtime` rather than a false-positive success. The most common cause of mismatch is Windows Installer's Restart Manager scheduling a delete-on-reboot rather than replacing the locked tr300.exe in-place — for that case, msiexec returns exit code `MSI_EXIT_REBOOT_REQUIRED = 3010` and we surface a dedicated "Reboot, then verify with `tr300 --version`" error message *before* the post-install verify runs.

**Don't relax these to "trust the installer's exit code."** The whole point of the v3.15.4 work is that exit-code-0-from-msiexec is NOT a guarantee that the running binary's on-disk image was actually replaced.

**Cargo-path post-install verify (v3.16.0+, U1).** The v3.15.4 verify above was Windows-MSI/EXE-only. The **cargo** strategy (`cargo install tr300 --force`, used on macOS/Linux and Windows cargo-installs) now has the same check via the shared, cross-platform `reexec_installed_version()` + `post_install_version_ok()` (extracted from the Windows `verify_post_install`) wrapped in `verify_cargo_post_install(latest)`. `cargo install` reports success (exit 0) even when crates.io still serves the OLD version — a publish lag right after a GitHub release, or a failed `crates-publish` run — which previously made `tr300 update` print "Updated to vX" while the binary was unchanged, then loop. On a version mismatch, `verify_cargo_post_install` returns `StrategyError::Runtime` so `execute_update` **falls through to the prebuilt GitHub-release installer** (which always carries `latest`). **Don't** add the same check to the terminal curl/wget/PowerShell installer strategies: they're last in the chain and install to a default bindir (`~/.cargo/bin` / `~/.local/bin`) that may differ from `current_exe()`, so a check there could **false-fail with no further fallback**. **Temp-installer cleanup (v3.16.0+, G1):** `try_msi_install` / `try_exe_install` best-effort `remove_file` the downloaded `%TEMP%` installer on success or failure — always *after* `verify_checksum` + `verify_post_install`, once the installer process has exited. Cleanup never gates the result and never relaxes the verifies.

**Rate-limit messaging (v3.16.0+, U2).** `fetch_latest_version` maps a `ureq::Error::Status(403, …)` with `x-ratelimit-remaining: 0` to an explicit "GitHub API rate limit exceeded (60/hour unauthenticated)" message via the pure, testable `http_status_message`, instead of an opaque "Request failed" — the unauthenticated 60/hr cap is a common intermittent cause of "update didn't work."

**`is_newer` handles prereleases per semver (v3.15.4+).** The hand-rolled comparator strips any `-…` / `+…` suffix via `strip_prerelease_metadata` before the numeric-vec compare, then handles the same-triple case: a stable release is treated as newer than its own prerelease (`3.15.2 > 3.15.2-rc.1`), a prerelease of a higher triple is newer than a lower stable (`3.15.2-rc.1 > 3.15.1`), two prereleases of the same triple compare equal (theoretical case — GitHub's `/releases/latest` filters prereleases out). Don't replace with a naive `Vec<u64>` comparison or pull in the `semver` crate without a stronger reason than this — the current heuristic catches the common failure modes that bit pre-v3.15.4 users and has 7 unit tests pinning the contract.

— Full reasoning, the rustup-managed-vs-distro-managed split, the rejected `rustc --version` probe alternative, the failure-mode this prevents: see [`docs/architecture-decisions.md` § "Self-update auto-rustup (v3.11.1+)"](../../../docs/architecture-decisions.md#self-update-auto-rustup-v3111).

## Windows distribution model (v3.15.0+)

> **v3.15.0 → v3.15.1 fix narrative (read before editing the WiX / Inno files).**
> v3.15.0 introduced the four-installer model but its `release.yml` run failed
> at `build-local-artifacts(x86_64-pc-windows-msvc)` with WiX candle exit
> code 6 — the GitHub Release was never published. v3.15.1 is the patch that
> fixed it. **Two root causes**, both real and both required for a working
> Corporate MSI build:
>
> 1. `wix/corporate.wxs` declared `<Property Id='ALLUSERS' Value=''/>` and
>    `<Property Id='MSIINSTALLPERUSER' Value='1'/>`. WiX 3.11 candle rejects
>    empty-string Property `Value=` attributes with `CNDL0006`. **Fix:**
>    delete both Property elements. `InstallScope='perUser'` on the Package
>    element is sufficient on WiX 3.11+ — the MSI installer treats unset
>    `ALLUSERS` as per-user, which is what we want.
> 2. cargo-wix compiles ALL `.wxs` files in `wix/` and links them into ONE
>    MSI. Two complete Product definitions in the same directory hit
>    `LGHT0089` ("Multiple entry sections") + `LGHT0091/0092` ("Duplicate
>    symbol") at link time. **Fix:** move the Corporate template to a NEW
>    directory `wix-corporate/corporate.wxs` so cargo-wix's `wix/` scan only
>    sees `main.wxs`. Build the Corporate MSI separately via bare
>    `candle.exe` + `light.exe` from `windows-installers.yml`.
>
> An interim diagnosis attempt during the v3.15.0 → v3.15.1 work-out
> speculated that the `InstallSourceMarker` Components writing HKCU from a
> perMachine MSI tripped ICE57 and triggered the failure. **That was wrong**
> — ICE runs in `light`, not `candle`, and the actual error was `CNDL0006`
> in corporate.wxs. The marker Components are unchanged between v3.15.0
> and v3.15.1; both Global and Corporate MSIs write
> `HKCU\Software\TR300\InstallSource` on install. Future agents diagnosing
> WiX failures should reproduce locally with portable WiX 3.11 binaries
> (https://github.com/wixtoolset/wix3/releases) BEFORE hypothesizing about
> ICE/CNDL/LGHT codes — cargo-dist captures candle/light stderr by
> default, so the real error class is invisible in CI logs without
> `--nocapture`.

Four first-class Windows installers ship at every tagged release. Two MSIs (built by cargo-dist's `release.yml` and the hand-authored `.github/workflows/windows-installers.yml`) and two Inno Setup EXE installers (built by the same hand-authored workflow). The shape of the four-product matrix:

| Product | Source template | Install scope | Install path | PATH scope | UAC? | ARP entry | InstallSource marker |
|---|---|---|---|---|---|---|---|
| Global MSI | `wix/main.wxs` | perMachine | `C:\Program Files\tr300\bin\` | System | Yes | `tr300` | `msi-global` |
| Corporate MSI | `wix-corporate/corporate.wxs` | perUser | `%LocalAppData%\Programs\tr300\bin\` | User | No | `tr300 (Corporate Edition)` | `msi-corporate` |
| Global EXE | `inno/global.iss` | perMachine | `C:\Program Files\tr300\bin\` (same as MSI Global) | System | Yes | `tr300` | `exe-global` |
| Corporate EXE | `inno/corporate.iss` | perUser | `%LocalAppData%\Programs\tr300\bin\` (same as MSI Corporate) | User | No | `tr300 (Corporate Edition)` | `exe-corporate` |

Edit-time rules:

- **The Corporate MSI source lives at `wix-corporate/corporate.wxs`, NOT `wix/corporate.wxs`.** This is intentional: cargo-wix's default behavior compiles every `.wxs` in `wix/` and links them into ONE MSI. Putting two complete Product definitions in the same directory hits link-stage errors LGHT0089 ("Multiple entry sections") and LGHT0091/0092 ("Duplicate symbol"). cargo-dist's `release.yml` MSI build (which uses cargo-wix internally) only sees `wix/main.wxs` because of this directory separation. v3.15.0 → v3.15.1 was a fix-forward driven by exactly this issue.
- **The four GUIDs identifying each product are PERMANENT.** They live in `wix/main.wxs` (Product `UpgradeCode='5CD540A8-…'`), `wix-corporate/corporate.wxs` (`UpgradeCode='93F465CB-…'`), `inno/global.iss` (`AppId={{AB14223F-…}`), and `inno/corporate.iss` (`AppId={{76A253EB-…}`). **Never regenerate these.** Changing any of them breaks in-place upgrades for users of that product (Windows Installer / Inno Setup treat the new GUID as a different product, so the old version isn't removed). Two more GUIDs live in the registry-marker Components — also permanent.
- **Same install paths between MSI and EXE within each edition.** Global → `C:\Program Files\tr300\bin\`. Corporate → `%LocalAppData%\Programs\tr300\bin\`. Don't suffix the EXE path with `-setup` or `(Setup)` — coexistence is documented in README as a "pick one" rule, not engineered around. The registry marker (`HKCU\Software\TR300\InstallSource`) records whichever was installed most recently; `tr300 update` re-runs that one.
- **Registry marker values are literal strings**: `msi-global`, `msi-corporate`, `exe-global`, `exe-corporate`. These appear in three places per product (the installer template that writes them, `src/update.rs::read_install_source_marker()` that matches them, and the JSON output's `install_origin` field). All three must stay in lockstep. Tests in `src/update.rs` pin the contract.
- **`src/update.rs::detect_install_origin()` is the only place that decides which installer to fetch.** If install paths change in `wix/main.wxs`, `wix-corporate/corporate.wxs`, or `inno/*.iss`, update `classify_install_path()` in lockstep. The path matcher uses lowercased substring containment (handles drive-letter case and "Program Files" capitalization variations) — it's intentionally not regex.
- **The Corporate MSI is built by bare `candle.exe` + `light.exe`, NOT through cargo-wix.** `.github/workflows/windows-installers.yml` invokes WiX directly: `candle.exe -arch x64 -dVersion=... -dCargoTargetBinDir=target/release ... wix-corporate/corporate.wxs` then `light.exe -sice:ICE38 -sice:ICE64 -sice:ICE91 -ext WixUIExtension ...`. cargo-wix can't be told to compile a single `.wxs` from outside `wix/` while ignoring the directory; bare WiX gives full control. The ICE suppressions are required because per-user MSI conventions in WiX 3 want HKCU RegistryValue KeyPaths on Components and RemoveFolder entries on Directories — both real correctness gaps for a "real" per-user MSI, but cosmetic for our single-binary install. The only real consequence: an empty `%LocalAppData%\Programs\tr300\bin\` folder left after uninstall.
- **`.github/workflows/windows-installers.yml` is hand-authored.** It uses `workflow_run` triggered by the `Release` workflow completion — NOT `release: types: [published]`. The `release: published` event does not fire downstream workflows when the upstream release was created via `gh release create` with the default `GITHUB_TOKEN` (GitHub's loop-prevention rule, [documented here](https://docs.github.com/en/actions/security-for-github-actions/security-guides/automatic-token-authentication#using-the-github_token-in-a-workflow)). cargo-dist's `release.yml` uses GITHUB_TOKEN, so the `release: published` approach silently didn't fire — v3.15.1 was the canary. `workflow_run` is the same pattern `crates-publish.yml` uses to chain off `CI`. The workflow also accepts `workflow_dispatch` with a `tag` input so it can be manually re-fired for any release: `gh workflow run "Windows Installers" -f tag=v3.15.1`. The workflow installs Inno Setup via Chocolatey (`choco install innosetup -y`) and uses the WiX 3 preinstalled at `$env:WIX\bin\` on the `windows-2022` runner. It uploads 6 assets (the Corporate MSI, both EXE installers, and their `.sha256` sidecars), bringing the total release count to 28 assets.
- **PowerShell `-D`-style argument values must be quoted** in the workflow's `& candle.exe` and `& iscc.exe` invocations. Write `"-dVersion=${{ steps.tag.outputs.version }}"` and `"/DMyAppVersion=${{ steps.tag.outputs.version }}"`, not the bare `-dVersion=${{ ... }}`. PowerShell's bareword tokenizer splits `-dVersion=3.15.1` into two tokens at the `.` characters — candle then sees `.15.1` as a positional source filename and errors with `CNDL0103`. v3.15.1 hit this on the first `workflow_dispatch` retry (run 25902607841); fixed in commit `5883627`. Any future `-D...=<value-with-dots>` arg added to this workflow must be wrapped in double quotes.
- **Don't touch `.github/workflows/release.yml`.** That file is auto-generated by cargo-dist v0.31.0. Only the small legacy installer-alias step in the allow-dirty zone is permitted. New install-related work goes in `windows-installers.yml`.
- **HKCU is intentional for the registry marker, even for perMachine installs.** Two reasons: (1) `tr300 update` always runs as the user, who reads HKCU naturally; (2) writing to HKLM from a perMachine MSI requires special handling and risks privilege issues. The rare "admin pushed via Intune, end-user is different" case is covered by the path-based fallback in `classify_install_path()` (the Program Files path implies MSI Global even without a marker).
- **Inno Setup PATH management uses the canonical `[Code]` block pattern**: `EnvAddPath()` / `EnvRemovePath()` with `;` padding for substring matching. Don't replace with Inno Setup's higher-level `Tasks` or `Run` sections — the manual approach handles the duplicate-detection and clean-uninstall cases that the high-level shortcuts get wrong.
- **`Cargo.toml` `allow-dirty = ["ci", "msi"]`** is required: `"ci"` for the legacy installer alias step in the auto-generated `release.yml`, `"msi"` for the customized `wix/main.wxs` Component additions. Both flags must stay.
- **`Cargo.toml` `include` list contains both `/wix/**` and `/wix-corporate/**`** so the published crate ships with both templates. Anyone running `cargo install tr300` and then `cargo wix` locally gets the Global MSI by default (only `wix/main.wxs` is scanned); they can build the Corporate MSI by running bare `candle.exe` + `light.exe` against `wix-corporate/corporate.wxs`.

— Full reasoning, why two MSIs instead of a dual-purpose MSI, why two EXE installers instead of a WiX Burn bundle, the rejected alternatives (WiX Burn, NSIS, single-product with `--mode user/admin` switch), the coexistence-vs-distinct-paths trade-off, SmartScreen / unsigned-binary UX honest accounting, the registry-marker contract and why HKCU: see [`docs/architecture-decisions.md` § "Windows distribution model (v3.15.0+)"](../../../docs/architecture-decisions.md#windows-distribution-model-v3150). The full v3.15.0 → v3.15.1 post-mortem (both root causes, the misattributed ICE57 hypothesis, the rejected fix alternatives, the local-WiX repro path used to diagnose it) lives at [`docs/architecture-decisions.md` § "v3.15.1 addendum"](../../../docs/architecture-decisions.md#v3151-addendum--why-corporatewxs-lives-in-wix-corporate-not-wix).

## Source of truth

Summary + tripwire in [`CLAUDE.md`](../../../CLAUDE.md); long-form rationale in [`docs/architecture-decisions.md`](../../../docs/architecture-decisions.md); the release procedure that ships these assets is the [`release`](../release/SKILL.md) skill. If they disagree with this skill, the docs win — fix this skill.
