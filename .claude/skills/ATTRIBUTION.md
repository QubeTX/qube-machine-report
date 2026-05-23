# Project-Scope Skills — Attribution

The four skill folders here are **vendored copies** of skills originally
distributed via Claude Code plugins. They were copied into the repo so any
agent working on TR-300 has access to them without depending on the user's
plugin configuration.

| Skill folder | Upstream source |
|---|---|
| [`brainstorming/`](./brainstorming/) | `superpowers` plugin (Anthropic-distributed Claude Code plugin) — [github.com/anthropics/claude-plugins](https://github.com/anthropics) |
| [`critical-thinking/`](./critical-thinking/) | `anthropic-skills` plugin (Anthropic-distributed) |
| [`architecture/`](./architecture/) | `engineering` plugin (Anthropic-distributed) |
| [`system-design/`](./system-design/) | `engineering` plugin (Anthropic-distributed) |

## Deviations from upstream

- **`brainstorming/scripts/`** is intentionally NOT copied. Upstream ships
  a small Node.js web server for the optional "visual companion" feature.
  TR-300 is a Rust CLI with no Node dependency; the visual companion
  gracefully falls back to text-only mode when the server isn't available
  (per the upstream SKILL.md). Both `visual-companion.md` and
  `spec-document-reviewer-prompt.md` are still here.
- **`architecture/SKILL.md`** has one line edited — the upstream reference
  to `../../CONNECTORS.md` (a plugin-relative path) was rewritten to point
  at the sibling `./CONNECTORS.md`. The connector categories table is
  retained for reference but most of it doesn't apply to a standalone CLI.

## Why vendored, not referenced

Different contributors run different plugin configurations. Vendoring
makes the skills part of the repo: anyone who clones TR-300 and runs
Claude Code in the project gets the same thinking-and-design toolkit,
without needing to install the upstream plugins first.

## Keeping in sync

These are point-in-time copies. If upstream improves a skill, manually
re-copy the relevant files. Don't extend or fork the skills here unless
the change is genuinely TR-300-specific — better to contribute back
upstream.

## How agents discover these

Claude Code auto-discovers `.claude/skills/<name>/SKILL.md` in the
project root. The four skills appear in the available-skills list under
their bare names (`brainstorming`, `critical-thinking`, `architecture`,
`system-design`) and take precedence over any plugin skill with the same
name.
