#!/usr/bin/env python3
"""Fail-closed CI classification and aggregation; no third-party dependencies."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib

REPOSITORY = "QubeTX/qube-machine-report"
RECORD_FILES = frozenset({"AGENTS.md", "CLAUDE.md", "CODEX_PROJECT.md", "MASTER_PLAN.md", "TESTING.md"})
PACKAGE_INCLUDE = frozenset({"/Cargo.lock", "/Cargo.toml", "/LICENSE", "/README.md", "/build.rs", "/man/**", "/src/**", "/tests/**", "/wix/**", "/wix-corporate/**"})
FULL_JOBS = ("release-bootstrap-windows", "fmt", "clippy", "test", "audit", "dist-plan", "workflow-validation", "windows-installer-sources")
SHA = re.compile(r"[0-9a-f]{40}\Z")


def git(*args: str) -> bytes:
    return subprocess.check_output(["git", *args], stderr=subprocess.PIPE)


def documentation_path(path: str) -> bool:
    # Paths are Git bytes decoded strictly, not OS-normalized paths. Reject
    # ambiguity before checking the small positive list.
    if not path or "\\" in path or any(ord(c) < 32 or ord(c) == 127 for c in path):
        return False
    parts = path.split("/")
    if any(p in ("", ".", "..") for p in parts):
        return False
    return path in RECORD_FILES or (parts[0] in ("docs", ".tasks") and len(parts) > 1 and path.endswith(".md"))


def changed_records(base: str, head: str) -> list[tuple[str, str]] | None:
    """Return (path, status), or None if any change needs full CI.

    --no-renames makes a move two checked paths, so moving code into docs
    cannot hide its old path. Raw modes reject links and submodules even
    when their names look like Markdown. NUL framing handles unusual names.
    """
    raw = git("diff", "--raw", "-z", "--no-renames", "--no-ext-diff", "--no-textconv", base, head, "--")
    fields = raw.split(b"\0")
    if fields[-1] != b"":
        return None
    fields.pop()
    if not fields or len(fields) % 2:
        return None
    records = []
    for header, filename in zip(fields[::2], fields[1::2]):
        match = re.fullmatch(rb":(\d{6}) (\d{6}) [0-9a-f]+ [0-9a-f]+ ([AMD])", header)
        if not match:
            return None
        old, new, status = (part.decode("ascii") for part in match.groups())
        if old not in ("000000", "100644", "100755") or new not in ("000000", "100644", "100755"):
            return None
        # Executable docs or a mode change deserve full review/validation.
        if "100755" in (old, new):
            return None
        path = filename.decode("utf-8", errors="strict")
        if not documentation_path(path):
            return None
        records.append((path, status))
    return records


def package_boundary_unchanged(head: str) -> bool:
    manifest = tomllib.loads(git("show", f"{head}:Cargo.toml").decode("utf-8"))
    package = manifest["package"]
    return (set(package.get("include", [])) == PACKAGE_INCLUDE
            and package.get("readme") == "README.md"
            and not package.get("license-file"))


def classify_diff(base: str, head: str) -> str:
    if not SHA.fullmatch(base) or not SHA.fullmatch(head):
        return "full"
    try:
        git("merge-base", "--is-ancestor", base, head)
        if not package_boundary_unchanged(head):
            return "full"
        return "docs" if changed_records(base, head) is not None else "full"
    except (subprocess.CalledProcessError, UnicodeError, ValueError, KeyError):
        return "full"


def event_range(event: dict, event_name: str, head: str) -> tuple[str, str]:
    if not SHA.fullmatch(head) or git("rev-parse", "HEAD").decode().strip() != head:
        raise ValueError("checkout is not the exact event SHA")
    if event.get("repository", {}).get("full_name") != REPOSITORY:
        raise ValueError("unexpected repository")
    if event_name == "push":
        base = event.get("before", "")
        if (event.get("ref") != "refs/heads/main" or event.get("after") != head
                or event.get("deleted") or event.get("forced") or not SHA.fullmatch(base) or base == "0" * 40):
            raise ValueError("unverifiable main push range")
    elif event_name == "pull_request":
        pr = event["pull_request"]
        if pr["base"]["ref"] != "main" or pr["base"]["repo"]["full_name"] != REPOSITORY:
            raise ValueError("unexpected PR base")
        parents = git("show", "-s", "--format=%P", head).decode().strip().split()
        if len(parents) != 2 or parents[1] != pr["head"]["sha"]:
            raise ValueError("checkout is not the expected synthetic PR merge")
        # GitHub's merge first parent is the actual integrated base, including
        # changes since the event's base SHA. Validate that relationship.
        base = parents[0]
        git("merge-base", "--is-ancestor", pr["base"]["sha"], base)
    else:
        raise ValueError("unsupported event")
    git("merge-base", "--is-ancestor", base, head)
    return base, head


def validate_records(base: str, head: str) -> None:
    for path, status in changed_records(base, head) or []:
        if status == "D":
            continue
        data = git("show", f"{head}:{path}")
        text = data.decode("utf-8", errors="strict")
        if "\0" in text or text.startswith("\ufeff"):
            raise ValueError(f"{path}: Markdown must be UTF-8 text without NUL/BOM")


def validate_gate(needs: dict) -> str:
    if set(needs) != {"classify", *FULL_JOBS}:
        raise ValueError("aggregate dependency inventory differs from policy")
    classification = needs["classify"]
    mode = classification.get("outputs", {}).get("mode")
    if classification.get("result") != "success" or mode not in ("docs", "full"):
        raise ValueError("classification did not succeed with a known mode")
    expected = "success" if mode == "full" else "skipped"
    for name in FULL_JOBS:
        if needs[name].get("result") != expected:
            raise ValueError(f"{mode} CI requires {name}={expected}; got {needs[name].get('result')}")
    return mode


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("classify", "gate"))
    args = parser.parse_args()
    if args.operation == "gate":
        mode = validate_gate(json.loads(os.environ["CI_NEEDS"]))
        print(f"CI gate passed ({mode})")
        return
    mode, reason = "full", "unverified comparison uses full CI"
    try:
        event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8"))
        base, head = event_range(event, os.environ["GITHUB_EVENT_NAME"], os.environ["GITHUB_SHA"])
        mode = classify_diff(base, head)
        if mode == "docs":
            validate_records(base, head)
        reason = f"{base}..{head}"
    except (KeyError, ValueError, OSError, subprocess.CalledProcessError) as error:
        # Unknown history is never a reason to skip actual validation. Avoid
        # printing arbitrary event fields or subprocess contents.
        mode = "full"
        reason = f"unverified comparison ({type(error).__name__}); using full CI"
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"mode={mode}\n")
    print(f"CI mode: {mode}; {reason}")
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as summary:
            summary.write(f"## CI scope\n\nMode: **{mode}**. Comparison: `{reason}`.\n")


if __name__ == "__main__":
    main()
