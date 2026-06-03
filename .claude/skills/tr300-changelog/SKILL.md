---
name: tr300-changelog
description: TR-300 changelog rules — keep CHANGELOG.md and HUMAN_CHANGELOG.md in lockstep. Load BEFORE editing `CHANGELOG.md` or `HUMAN_CHANGELOG.md`, or when writing release notes for a version bump. CHANGELOG.md is Keep-a-Changelog and authoritative for agents/contributors; HUMAN_CHANGELOG.md is its plain-English mirror updated in the SAME commit (same `## [X.Y.Z] - YYYY-MM-DD` header, same Added/Changed/Fixed/Internal groupings). Encodes the full strip list (CI run IDs, SHAs, error codes, function/API names, registry paths, file paths, GUIDs, LOC/memory deltas, task IDs, MSRV strings) and keep list (platform names, edition names, installer types, user-typed commands and flags, the user-facing benefit). Triggers on "changelog", "release notes", "human changelog", "update the changelog", "what shipped". Never update one changelog without the other.
---

# TR-300 changelog rules (CHANGELOG.md ↔ HUMAN_CHANGELOG.md)

The strip/keep contract for keeping the two changelogs in lockstep. Summary + tripwire are in
[`CLAUDE.md` § "HUMAN_CHANGELOG.md (companion changelog)"](../../../CLAUDE.md) (heading preserved
as an anchor); the `release` and `tr300-dev-workflow` skills both reference this file for the
F.1 doc step. For the broader task of *bootstrapping* a human changelog in a new repo, the
global `shaughv-code:human-changelog` skill applies — this skill is the TR-300-specific contract.

## HUMAN_CHANGELOG.md (companion changelog)

`HUMAN_CHANGELOG.md` at the repo root is a plain-English mirror of
`CHANGELOG.md`. Every version block in `CHANGELOG.md` has a corresponding
block in `HUMAN_CHANGELOG.md` with the same `## [X.Y.Z] - YYYY-MM-DD`
header and the same `### Added` / `### Changed` / `### Fixed` / `### Internal`
groupings, but with the technical noise stripped so a non-technical
reader can answer "what shipped and why should I care?" in 30 seconds.
`CHANGELOG.md` stays as-is — it's the authoritative source for agents
and contributors.

**Edit-time rules:**

- **Lockstep with `CHANGELOG.md`.** When you add a release entry to
  `CHANGELOG.md`, add the same release entry to `HUMAN_CHANGELOG.md`
  in the same commit. When you amend a `CHANGELOG.md` entry
  post-release (e.g. recording publication status), update the
  human mirror too. Never let one drift ahead of the other.

- **Strip these from the human version:** CI run IDs, commit SHAs,
  error codes (WiX `CNDL…` / `LGHT…`, Clippy lint names,
  `RUSTSEC-…`), function or method names (`detect_install_origin()`
  etc.), Win32 / WMI / POSIX API names (`GetBestInterfaceEx`,
  `WTSQuerySessionInformation`, `geteuid`), registry paths, file
  paths under `src/` / `wix/` / `.github/`, GUIDs, dependency crate
  identifiers used as identifiers (`winreg = "0.52"`), memory or
  LOC measurements (`~30 LOC`, `45 KB → 31.5 KB`, `~80 ms WMI cost`),
  per-task IDs (`(C.10)`, `(E.5)`, `(F.1)`), MSRV / toolchain
  version strings.

- **Keep these in the human version:** the `## [X.Y.Z] - YYYY-MM-DD`
  header, platform names (Windows / macOS / Linux), edition names
  (Global / Corporate), installer types (MSI, EXE, shell installer,
  PowerShell installer, `cargo install`), command names and flags
  users actually type (`tr300 update`, `--fast`, `--ascii`,
  `--no-elevation-hint`), user-visible feature names, what the
  user-facing benefit is (faster, more accurate, VPN-aware, more
  reliable on locked-down machines), security and crash-fix
  implications, release-publication failure narratives compressed
  to a single sentence.

- **One short paragraph per change is the target length.** If a
  `CHANGELOG.md` entry needs ten lines of API reasoning, the human
  mirror needs one or two sentences. The technical detail stays in
  `CHANGELOG.md` for agents and contributors.

- **Voice: declarative, user-facing.** "CPU socket detection on
  Windows is now faster." not "Replaced the WMI count path with a
  native API."

- **Version numbers in headers stay.** The strip rule on "numbers"
  applies to noise inside entries (run IDs, line counts, GUIDs) —
  not to release identifiers users search for.

## Source of truth

The heading stub + tripwire in [`CLAUDE.md`](../../../CLAUDE.md) points here. The `release` skill
(§ 4, file 1b) and `tr300-dev-workflow` (F.1) both defer to this contract. If they disagree with
this skill, those files win — fix this skill.
