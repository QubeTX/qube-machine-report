#!/usr/bin/env python3
"""Require native, supported Apple runners in an actual cargo-dist JSON plan."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


APPLE_RUNNERS = {
    "aarch64-apple-darwin": "macos-15",
    "x86_64-apple-darwin": "macos-15-intel",
}


def validate_manifest(manifest: object) -> None:
    try:
        matrix = manifest["ci"]["github"]["artifacts_matrix"]
    except (KeyError, TypeError) as error:
        raise ValueError("cargo-dist plan has no GitHub artifacts matrix") from error
    if not isinstance(matrix, dict) or set(matrix) != {"include"}:
        raise ValueError("cargo-dist artifacts matrix must contain only explicit include entries")
    entries = matrix["include"]
    if not isinstance(entries, list):
        raise ValueError("cargo-dist artifacts matrix must be a list")

    seen = set()
    for entry in entries:
        if not isinstance(entry, dict):
            raise ValueError("cargo-dist matrix entry must be an object")
        targets = entry.get("targets")
        if not isinstance(targets, list) or not all(isinstance(target, str) for target in targets):
            raise ValueError("cargo-dist matrix entry must have a target list")
        for target in targets:
            if not target.endswith("-apple-darwin"):
                continue
            if target not in APPLE_RUNNERS:
                raise ValueError(f"unreviewed Apple target: {target}")
            if target in seen:
                raise ValueError(f"duplicate Apple target: {target}")
            if targets != [target]:
                raise ValueError(f"{target}: native builds must have their own matrix entry")
            if entry.get("runner") != APPLE_RUNNERS[target]:
                raise ValueError(f"{target}: runner must be {APPLE_RUNNERS[target]}")
            if entry.get("host") != target:
                raise ValueError(f"{target}: runner host must match the native target")
            if entry.get("container") is not None:
                raise ValueError(f"{target}: container execution is not permitted")
            seen.add(target)

    missing = APPLE_RUNNERS.keys() - seen
    if missing:
        raise ValueError(f"missing Apple targets: {', '.join(sorted(missing))}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args()
    try:
        validate_manifest(json.loads(args.manifest.read_text(encoding="utf-8-sig")))
    except (OSError, ValueError) as error:
        print(f"cargo-dist runner validation failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
    print("cargo-dist Apple targets use the required native runners")


if __name__ == "__main__":
    main()
