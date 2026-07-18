<!-- tasks-bootstrap: done -->
> Secrets: never stored here or in memory/. See .tasks/secure/ (gitignored), or env/keychain.

# Memory

## Me

Emmett maintains TR-300 and drives releases with Codex across Windows, hosted macOS/Linux runners, and personal hardware.

## People

| Who | Role |
|-----|------|
| **Emmett** | Maintainer and release operator |

## Terms

| Term | Meaning |
|------|---------|
| **TR-300** | Cross-platform Rust machine-report CLI (`tr300`) |
| **Global** | Per-machine Windows installer edition |
| **Corporate** | Per-user Windows installer edition |
| **Origin-preserving update** | Update through the same detected install channel, edition, scope, and prefix |
| **PKG-in-DMG** | Signed macOS Installer package nested in the versionless DMG distribution |

## Projects

| Name | What |
|------|------|
| **TR-300 v4.1.1** | Immutable fix-forward for the origin-preserving updater, native universal Mac installer, and cross-platform validation release |

## Preferences

- Prefer durable native mechanisms and exact evidence over path-based guesses.
- Preserve unrelated work and fail safely when installer origin is ambiguous.
- Never put credentials or private key material in git, tasks, docs, logs, or memory.
