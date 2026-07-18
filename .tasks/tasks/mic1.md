# MIC-1 managed installation behavior

## Status

Active. Source implementation is present; local and hosted transition evidence,
documentation reconciliation, and immutable release proof remain open.

## Activity

- 2026-07-18 — implemented the stable-name managed wrappers, strict transactional
  ownership migration, native-package convergence rules, hosted malformed-owner
  fixtures, and the reusable fresh-install versus self-update contract across
  source, ADR, README, changelogs, testing, plans, and agent guidance (agent: codex)
- 2026-07-18 — froze the v4.2.0 candidate after 164 unit + 19 integration tests,
  warning-denying Clippy, release build, RustSec, cargo-dist, workflow/script,
  rollback, WiX 3.14.1, Inno 6.7.3, and Alienware functionality/hardware gates.
  Inspected the generated cargo-dist 0.31.0 Unix installer and changed the
  published Linux smoke to prove its default persistent PATH setup from a clean
  home and fresh shell. Clean-tree and hosted release evidence remain open
  (agent: codex)
- 2026-07-18 — first exact-SHA CI run 29661664992 stopped before tagging:
  native Intel/ARM Mac rejected one unused Mac-only staging helper under
  `-D warnings`, and hosted Linux actionlint exposed six embedded-shell findings
  that Windows actionlint had not evaluated without ShellCheck on PATH. Removed
  the unused API, made literal-string suppressions explicit, and replaced two
  `! command`/`errexit` checks with ordinary fail-closed conditionals. Windows
  installer source/transition/rollback gates passed in that run; the complete
  exact-SHA matrix must now repeat on the fix-forward commit (agent: codex)
- 2026-07-18 — second exact-SHA CI run 29662005364 proved every native Apple,
  Windows, Linux, performance, audit, and real Windows installer source job,
  but stopped before tagging because ShellCheck directives do not accept a
  trailing prose suffix after `disable=SC2016`. Moved each explanation to its
  own preceding comment and retained the narrowly scoped directive; the final
  exact-SHA matrix still must pass as a whole (agent: codex)
- 2026-07-18 — third exact-SHA CI run 29662168432 again passed every product,
  native Apple, Windows/Linux, performance, audit, and Windows installer source
  job. Hosted actionlint then identified the one remaining intentional literal
  dollar expression in the lifecycle guard (`cmp "$pkg" "$mount/tr300.pkg"`).
  Added the exact SC2016 annotation and locally linted the extracted complete
  guard block before another full exact-SHA run (agent: codex)
- 2026-07-18 — final v4.2.0 source `b61e8b8` passed CI 29662326024 and
  crates run 29662484965; crates.io published 4.2.0. Tagged release run
  29662526880 built all six targets and signed/notarized both Apple archives,
  then failed closed before GitHub Release creation because host assertions
  expected fully expanded URLs that both valid wrappers construct from pinned
  tag/base variables plus asset suffixes; the shell assertion failed first.
  Preserved the v4.2.0 tag/crate,
  bumped to v4.2.1, corrected the assertion, and added full wrapper rendering
  to pre-tag CI (agent: codex)
- 2026-07-18 — v4.2.1 reran locked fmt/Clippy/tests/build, RustSec, cargo-dist,
  actionlint, direct plus embedded ShellCheck, both wrapper transactions, the
  executable synthetic-render lifecycle guard, and WiX 3.14.1/Inno 6.7.3
  Global/Corporate source compiles successfully; clean committed-tree package
  and hosted evidence remain (agent: codex)
- 2026-07-18 — exact v4.2.1 source `b45ec00` passed CI 29663392937,
  crates 29663494252, Release 29663533999, both signed/notarized Apple archive
  jobs, and Windows packaging 29663678096. Native Intel and ARM run 29663678097
  then proved PKG `postinstall` ambiguity rejection occurs after payload
  installation, so all four Mac assets were withheld. Windows transition run
  29663781604 separately proved that selecting a historical baseline against
  the new release's future-only internal script is invalid. v4.2.1 remains an
  immutable 30-asset release; v4.2.2 moves strict dry-run ownership proof into
  PKG `preinstall`, retains transactional cleanup in `postinstall`, makes every
  negative-state assertion explicit, and separates historical/current asset
  contracts (agent: codex)
- 2026-07-18 — v4.2.2 passes local locked fmt/Clippy/164 unit/19 integration/
  release build, RustSec, cargo-dist, workflow/script/parser, extracted Mac
  lifecycle ShellCheck, managed rollback, executable guard, WiX/Inno source,
  historical-baseline resolver, and Alienware candidate functionality gates.
  Clean committed-tree package/publish proof and exact-SHA hosted publication
  remain (agent: codex)
- 2026-07-18 — first v4.2.2 exact-SHA CI 29664322349 passed every product,
  native-platform, audit, performance, cargo-dist, and real Windows installer
  source job, but the lifecycle job stopped because hosted ShellCheck reports a
  trap-only rollback body as SC2317 while the newer local build reports SC2329.
  Scoped the callback annotation to both version-dependent diagnostics and kept
  its executable rollback fixture. No crate, tag, or release was produced
  (agent: codex)

## Intent

Make the managed prebuilt CLI installer the advertised default on every
supported platform while keeping native packages as first-class options.
`tr300 update` preserves the proven channel. A deliberately launched fresh
managed/native installer is authoritative for upgrade, reinstall, or downgrade;
it either converges recognized prior ownership or fails visibly without
guessing. Raw Cargo stays advanced/unmanaged because it has no post-install
hook.

## Implementation

- Public stable-name PowerShell/shell wrappers retain cargo-dist's generated
  installer transactions under stable internal exact-tag asset names.
- Windows PowerShell verifies its new binary/receipt before enumerating the two
  exact MSI UpgradeCodes and two exact Inno AppIds and invoking real
  uninstallers.
- macOS shell takeover requires exact receipt, payload owner, and Developer ID
  proof. PKG `preinstall` uses its embedded exact candidate for strict dry-run
  ownership proof before payload mutation; `postinstall` performs the reverse
  allowlisted Cargo binary/receipt cleanup.
- Cross-edition native Windows packages stop before mutation when the other
  scope is registered or its exact native binary remains; same-edition format
  changes remain automatic only after a strict non-mutating managed-ownership
  preflight.
- Stable human entrypoints use `latest`; every wrapper/updater payload remains
  pinned to the one resolved immutable tag.
- Both wrappers snapshot and restore the prior managed/Cargo binary plus receipt
  when a fresh native takeover cannot commit; isolated rollback fixtures run in
  CI.
- Once an external native uninstall/receipt retirement commits and has no
  supported inverse, the wrappers retain the already-verified managed owner and
  fail with partial-state recovery evidence rather than risking zero copies.
- Current MSI/EXE/PKG integrations always use `migrate-cleanup --strict` with no
  task/checkbox opt-out. Strict mode prevalidates and transactionally
  quarantines/restores an exact Cargo-path binary plus cargo-dist receipt;
  ambiguous evidence fails before mutation. Legacy no-`--strict` calls stay
  advisory for compatibility.
- Inno extracts the candidate binary and dry-runs that strict ownership check
  during `PrepareToInstall`, before removing an MSI or writing native state,
  then reconfirms the transaction after its own registration exists. Hosted
  malformed-receipt gates require all four Windows packages and the Mac PKG to
  preserve prior bytes/receipt and leave no rejected native owner.
- Historical transition discovery evaluates old releases against their own
  established artifact family; only the current release must contain newly
  introduced internal wrapper assets.

## Verification

- [x] Rust migration/updater unit tests (including strict pair
  preflight/transaction), both wrapper rollback fixtures, and
  PowerShell/shell parsers pass locally.
- [x] Alienware read-only wrapper discovery identifies exactly the natural
  Global MSI product through its immutable UpgradeCode.
- [x] WiX 3.14.1 and Inno 6.7.3 compile both editions locally; only the
  intentional WiX `AllowDowngrades` ICE61 remains. Hosted source jobs own real
  takeover, cross-edition, and malformed-receipt rollback proof.
- [x] Full local locked Rust gates, RustSec, cargo-dist plan/generate-check,
  actionlint/ShellCheck, wrapper fixtures, and Alienware candidate
  functionality/hardware validation pass.
- [x] Extracted PKG lifecycle scripts lint independently, and the Windows
  release resolver selects v4.1.3 under the historical contract without
  weakening the v4.2.2 current contract.
- [ ] Hosted Windows exact-release jobs prove all four native-to-IRM takeovers.
- [ ] Native Intel/ARM jobs prove shell-to-PKG and PKG-to-shell takeovers.
- [ ] Published AMD64 Linux wrapper installs, receipts, and no-op-updates.
- [ ] Final docs and 34-asset release audit agree with MIC-1.

## Resume

Run the v4.2.2 full clean-tree release sequence only after local fmt/clippy/tests/build,
package/publish dry runs, audit, actionlint/shellcheck, WiX, and Inno gates pass.
Do not use the Alienware's active Global MSI as a takeover fixture; disposable
hosted Windows runners own that destructive matrix.
