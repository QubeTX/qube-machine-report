# TR-300 edit-time reminder hook (PreToolUse: Edit | Write | MultiEdit).
#
# Deterministic tripwire: when an agent is about to edit a sensitive subsystem,
# inject a short reminder naming the skill to load plus the load-bearing
# invariants. This fires on the edit itself, independent of whether the skill's
# description triggered from the prompt — the backstop for the rules that were
# relocated out of CLAUDE.md into per-subsystem skills.
#
# Strictly additive: always exits 0, never returns a block/deny decision. A
# non-matching path, missing field, or parse failure is a silent no-op. The
# reminder is surfaced via the PreToolUse `additionalContext` field; on a
# Claude Code build that does not honor it, the hook degrades to a no-op and
# the CLAUDE.md tripwire table + skill descriptions still cover the gap.
#
# Registered (machine-local, gitignored) in .claude/settings.local.json. The
# matching reference rules live in CLAUDE.md "Edit-time rule skills" and in
# .claude/skills/<name>/SKILL.md.

$ErrorActionPreference = 'SilentlyContinue'

$raw = [Console]::In.ReadToEnd()
if ([string]::IsNullOrWhiteSpace($raw)) { exit 0 }

try { $payload = $raw | ConvertFrom-Json } catch { exit 0 }

$path = $payload.tool_input.file_path
if ([string]::IsNullOrWhiteSpace($path)) { exit 0 }

# Normalize: forward slashes, lower-case, for substring/regex matching.
$p = $path.Replace('\', '/').ToLowerInvariant()
$base = ($p -split '/')[-1]

$msgs = New-Object System.Collections.Generic.List[string]

if ($p -match '(^|/)src/install/') {
    $msgs.Add('editing the install subsystem -> load the windows-install skill. Invariants: route rc-file writes through atomic_write (never std::fs::write); call check_marker_balance before any mutation; run the Windows execution-policy preflight before writing the PowerShell profile; funnel fs failures through fail_install.')
}
if ($p -match '(^|/)src/collectors/platform/windows\.rs') {
    $msgs.Add('editing the Windows collectors -> load the windows-accuracy skill. Invariants: run the WMI batch on a fresh worker thread (COM init mode); pick PSCore version by (u64,u64,u64) semver tuple, not string sort; detect Win11 by CurrentBuild greater-or-equal 22000; do not reintroduce net user parsing for last-login.')
}
if ($p -match '(^|/)wix/' -or $p -match '(^|/)wix-corporate/' -or $p -match '(^|/)inno/' -or $p -match '(^|/)windows-installers\.yml' -or $p -match '(^|/)release\.yml' -or $p -match '(^|/)src/update\.rs') {
    $msgs.Add('editing Windows packaging / self-update -> load the windows-distribution-and-update skill. Invariants: the four product GUIDs are PERMANENT (never regenerate); registry InstallSource marker strings stay in lockstep across installer template / update.rs / JSON; keep the SHA256 + post-install version verify; do not hand-edit auto-generated release.yml outside the allow-dirty zone.')
}
if ($base -eq 'changelog.md' -or $base -eq 'human_changelog.md') {
    $msgs.Add('editing a changelog -> load the tr300-changelog skill. Invariant: update CHANGELOG.md and HUMAN_CHANGELOG.md in the SAME commit, stripping technical noise from the human mirror.')
}
if ($base -eq 'cargo.toml' -or $base -eq 'rust-toolchain.toml') {
    $msgs.Add('MSRV lives in BOTH Cargo.toml (rust-version) and rust-toolchain.toml (channel) -- bump them together, and keep rust-toolchain.toml components = rustfmt + clippy. See the release / tr300-dev-workflow skills.')
}

if ($msgs.Count -eq 0) { exit 0 }

$context = 'TR-300 edit-time reminder: ' + ($msgs -join '  ||  ')

$out = @{
    hookSpecificOutput = @{
        hookEventName     = 'PreToolUse'
        additionalContext = $context
    }
}

$out | ConvertTo-Json -Depth 5 -Compress
exit 0
