# Repository Guidelines

## Project Status

This repository is currently a staging area for a modern successor. The only implementation present today is the legacy reference in `TR200-OLD/`. Expect this guide to evolve once the new codebase is added.

## Project Structure & Module Organization

- `TR200-OLD/` contains the original program (shell/PowerShell scripts, npm packaging, and docs).
- `.github/` is reserved for automation; it is empty at the moment.
- `.gitignore` is the only root-level configuration file.

## Build, Test, and Development Commands

- There are no repo-wide build or test commands yet.
- Legacy build/packaging commands are documented under `TR200-OLD/README.md` and `TR200-OLD/tools/`; use them only if you are working on the legacy copy.

## Coding Style & Naming Conventions

- No formatting or linting tooling is configured at the root yet.
- If editing legacy files, follow the existing style and keep line endings consistent with the file.

## Testing Guidelines

- No test framework is configured in the root.
- The legacy folder does not include automated tests; verify changes manually if you touch it.

## Commit & Pull Request Guidelines

- Recent commit history uses short, imperative messages (for example “Update README …”). Keep that style.
- PRs should include a brief summary and note whether changes affect legacy only or future work.

## Legacy Reference

`TR200-OLD/` is the source of truth for current behavior and output formatting. Treat it as reference material unless the change request explicitly targets legacy.
