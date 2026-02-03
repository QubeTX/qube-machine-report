# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Status

This repository is a **staging area** for a modern successor to TR-200 Machine Report. The new implementation does not exist yet—only the legacy reference code is present.

## Repository Structure

```
qube-machine-report/
├── TR200-OLD/           # Legacy TR-200 implementation (reference only)
│   ├── machine_report.sh        # Unix/macOS bash script
│   ├── WINDOWS/                 # PowerShell implementation
│   │   └── TR-200-MachineReport.ps1
│   ├── bin/tr200.js             # npm CLI wrapper
│   ├── install.sh               # Unix installer
│   └── ...
├── AGENTS.md            # Guidelines for this staging repo
├── CLAUDE.md            # This file
└── .gitignore
```

## Development Commands

**None configured at the repository root.** This is a staging area awaiting the new implementation.

### Legacy Commands (TR200-OLD only)

If working on legacy code, use from within `TR200-OLD/`:
```bash
# Run directly
./machine_report.sh

# Build release zip
./tools/package_release.sh

# Windows PowerShell
powershell -File WINDOWS/TR-200-MachineReport.ps1
```

## Guidelines

### When to Work on Legacy (`TR200-OLD/`)
Only when the change request explicitly targets the legacy implementation. Treat `TR200-OLD/` as archived reference material.

### For New Development
- Wait for new codebase to be added
- The new implementation will replace `TR200-OLD/` eventually

### Commit Style
Short, imperative messages (e.g., "Update README", "Fix array indexing").

## Legacy Reference

The `TR200-OLD/CLAUDE.md` contains detailed guidance for the legacy scripts including:
- Line-by-line architecture of `machine_report.sh`
- PowerShell script structure
- Customization locations
- Cross-platform testing requirements

Consult it only when working on legacy code.
