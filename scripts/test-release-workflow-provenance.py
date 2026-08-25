#!/usr/bin/env python3
"""Execute and structurally guard privileged release provenance workflows."""

from __future__ import annotations

import hashlib
import io
import json
import lzma
import os
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import textwrap
import zipfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RELEASE_WORKFLOW = ROOT / ".github" / "workflows" / "release.yml"
WINDOWS_WORKFLOW = ROOT / ".github" / "workflows" / "windows-installers.yml"
MACOS_WORKFLOW = ROOT / ".github" / "workflows" / "macos-installer.yml"
MACOS_INSTALLER_BUILDER = ROOT / "scripts" / "build-sign-notarize-macos-installer.sh"
CI_WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"
CRATES_WORKFLOW = ROOT / ".github" / "workflows" / "crates-publish.yml"
PINNED_INNO_INSTALLER = ROOT / "scripts" / "install-pinned-inno-setup.ps1"
PINNED_WINDOWS_CARGO_DIST_INSTALLER = (
    ROOT / "scripts" / "install-pinned-cargo-dist.ps1"
)
APPLE_STAGING_STEP_NAME = "Validate and safely stage the unsigned Apple archive"
NATIVE_APPLE_JOBS = (
    ("release.yml", "sign-apple-artifacts"),
    ("macos-installer.yml", "prepare-installer-inputs"),
    ("macos-installer.yml", "credential-preflight"),
    ("macos-installer.yml", "build"),
    ("macos-installer.yml", "validate"),
    ("macos-installer.yml", "legacy-bridge"),
)
NATIVE_APPLE_BUILD_STEPS = (
    "enable windows longpaths",
    "Reject unreviewed cargo-dist container runners",
    "Install pinned cargo-dist on Unix",
    "Install dependencies",
    "Build artifacts",
    "Post-build",
)
BASH_4_ONLY_PATTERNS = (
    (
        re.compile(r"\b(?:declare|typeset|readonly)[ \t]+-[A-Za-z]*A[A-Za-z]*\b"),
        "associative-array declaration",
    ),
    (
        re.compile(r"\b(?:declare|typeset)[ \t]+-[A-Za-z]*g[A-Za-z]*\b"),
        "global declaration",
    ),
    (
        re.compile(r"\b(?:declare|typeset|local)[ \t]+-[A-Za-z]*n[A-Za-z]*\b"),
        "nameref declaration",
    ),
    (re.compile(r"\b(?:mapfile|readarray)\b"), "array-reading builtin"),
    (re.compile(r"\bwait[ \t]+-[A-Za-z]*n[A-Za-z]*\b"), "wait -n"),
    (
        re.compile(r"\bshopt[ \t]+-s[^\n]*(?:globstar|lastpipe)\b"),
        "Bash-4-only shopt",
    ),
)
CI_JOB_IDS = (
    "release-bootstrap-windows",
    "fmt",
    "clippy",
    "test",
    "build",
    "speed",
    "audit",
    "dist-plan",
    "workflow-validation",
    "windows-installer-sources",
)
CI_MATRIX_OSES = {
    "test": (
        "ubuntu-latest",
        "ubuntu-22.04-arm",
        "macos-15",
        "macos-15-intel",
        "windows-latest",
    ),
    "build": (
        "ubuntu-latest",
        "ubuntu-22.04-arm",
        "macos-15",
        "macos-15-intel",
        "windows-latest",
    ),
    "speed": ("ubuntu-latest", "macos-latest", "windows-latest"),
}
INNO_MSI_BRIDGE = ROOT / "inno" / "remove-conflicting-msi.pas"
MANAGED_WINDOWS_INSTALLER = (
    ROOT / "scripts" / "managed-installers" / "tr300-installer.ps1"
)
UPDATE_SOURCE = ROOT / "src" / "update.rs"
WINDOWS_VALIDATION_WORKFLOW = (
    ROOT / ".github" / "workflows" / "windows-installer-validation.yml"
)
STABLE_TAG_PATTERN = (
    r"^v(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)$"
)
REPOSITORY = "QubeTX/qube-machine-report"
TRUSTED_SHA = "a" * 40
OTHER_SHA = "b" * 40
TAG_OBJECT_SHA = "c" * 40
RELEASE_RUN_ID = "29664688035"
RELEASE_RUN_ATTEMPT = "1"
RELEASE_WORKFLOW_ID = "229917047"
MACOS_WORKFLOW_ID = "229917048"
WINDOWS_WORKFLOW_ID = "229917049"
WINDOWS_VALIDATION_WORKFLOW_ID = "229917050"
REPOSITORY_ID = "987654321"
DOWNLOAD_ARTIFACT_ACTION = (
    "actions/download-artifact@37930b1c2abaa49bbe596cd826c3c89aef350131 # v7"
)
UPLOAD_ARTIFACT_ACTION = (
    "actions/upload-artifact@b7c566a772e6b6bfb58ed0dc250532a479d7789f # v6"
)
CHECKOUT_ACTION = (
    "actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803 # v6.1.0"
)
CRATES_AUTH_ACTION = (
    "rust-lang/crates-io-auth-action@c6f97d42243bad5fab37ca0427f495c86d5b1a18 # v1.0.5"
)
CARGO_DIST_SH_SHA256 = (
    "e79d87e418b9d2cbe992d014985457c28a5a7c553add3da4ed1047e161c928f4"
)
CARGO_DIST_PS1_SHA256 = (
    "ffec5b52cfbe29465d831150b01f8a254668fc271e5102fab7aea7da5d51ec69"
)
WINDOWS_RELEASE_ARTIFACT = "windows-release-assets"
WINDOWS_RELEASE_PAYLOADS = (
    "tr300-x86_64-pc-windows-msvc-corporate.msi",
    "tr300-x86_64-pc-windows-msvc-setup.exe",
    "tr300-x86_64-pc-windows-msvc-corporate-setup.exe",
)
WINDOWS_RELEASE_ASSETS = tuple(
    name for payload in WINDOWS_RELEASE_PAYLOADS for name in (payload, f"{payload}.sha256")
)
WINDOWS_UPSTREAM_SENTINELS = (
    "dist-manifest.json",
    "tr300-x86_64-pc-windows-msvc.msi",
)
MACOS_RELEASE_PAYLOADS = (
    "tr300-universal-apple-darwin.pkg",
    "tr300-universal-apple-darwin.dmg",
)
MACOS_RELEASE_ASSETS = tuple(
    name for payload in MACOS_RELEASE_PAYLOADS for name in (payload, f"{payload}.sha256")
)
MACOS_RELEASE_RUN_ARTIFACTS = (
    "artifacts-build-local-aarch64-apple-darwin",
    "artifacts-build-local-x86_64-apple-darwin",
    "unsigned-artifacts-build-local-aarch64-apple-darwin",
    "unsigned-artifacts-build-local-x86_64-apple-darwin",
)
INITIAL_RELEASE_ASSETS = (
    "dist-manifest.json",
    "sha256.sum",
    "source.tar.gz",
    "source.tar.gz.sha256",
    "tr-300-installer.ps1",
    "tr-300-installer.sh",
    "tr300-aarch64-apple-darwin.tar.xz",
    "tr300-aarch64-apple-darwin.tar.xz.sha256",
    "tr300-aarch64-unknown-linux-gnu.tar.xz",
    "tr300-aarch64-unknown-linux-gnu.tar.xz.sha256",
    "tr300-dist-installer.ps1",
    "tr300-dist-installer.sh",
    "tr300-installer.ps1",
    "tr300-installer.sh",
    "tr300-x86_64-apple-darwin.tar.xz",
    "tr300-x86_64-apple-darwin.tar.xz.sha256",
    "tr300-x86_64-pc-windows-msvc.msi",
    "tr300-x86_64-pc-windows-msvc.msi.sha256",
    "tr300-x86_64-pc-windows-msvc.zip",
    "tr300-x86_64-pc-windows-msvc.zip.sha256",
    "tr300-x86_64-unknown-linux-gnu.tar.xz",
    "tr300-x86_64-unknown-linux-gnu.tar.xz.sha256",
    "tr300-x86_64-unknown-linux-musl.tar.xz",
    "tr300-x86_64-unknown-linux-musl.tar.xz.sha256",
)
PREPARED_RELEASE_ARTIFACT_MEMBERS = (
    *INITIAL_RELEASE_ASSETS,
    "__tr300-release-metadata.json",
    "__tr300-notes.txt",
    "__tr300-asset-sha256s",
)


def require(workflow: str, needle: str, label: str) -> None:
    if needle not in workflow:
        raise AssertionError(f"{label}: missing {needle!r}")


def require_exact_step(step: str, expected: str, label: str) -> None:
    actual_lines = textwrap.dedent(step).strip().splitlines()
    expected_lines = textwrap.dedent(expected).strip().splitlines()
    actual = "\n".join(line.rstrip() for line in actual_lines)
    wanted = "\n".join(line.rstrip() for line in expected_lines)
    if actual != wanted:
        raise AssertionError(
            f"{label}: step must match the exact fail-closed contract\n"
            f"expected:\n{wanted}\nactual:\n{actual}"
        )


def require_exact_line_sequence(text: str, expected: str, label: str) -> None:
    lines = textwrap.dedent(text).strip().splitlines()
    wanted = textwrap.dedent(expected).strip().splitlines()
    matches = sum(
        lines[index : index + len(wanted)] == wanted
        for index in range(len(lines) - len(wanted) + 1)
    )
    if matches != 1:
        raise AssertionError(
            f"{label}: expected one exact active line sequence, found {matches}: "
            f"{wanted!r}"
        )


def require_direct_step_contract(
    step: str,
    expected_fields: tuple[str, ...],
    exact_scalars: dict[str, str],
    label: str,
) -> None:
    normalized = textwrap.dedent(step).strip()
    fields = tuple(
        match.group(1)
        for line in normalized.splitlines()
        if (match := re.match(r"^  ([A-Za-z0-9_-]+):", line)) is not None
    )
    if fields != expected_fields:
        raise AssertionError(
            f"{label}: direct step fields must be {expected_fields!r}, found {fields!r}"
        )
    for key, expected in exact_scalars.items():
        matches = [
            match.group(1).strip()
            for line in normalized.splitlines()
            if (match := re.match(rf"^  {re.escape(key)}:\s*(.*?)\s*$", line))
            is not None
        ]
        if matches != [expected]:
            raise AssertionError(
                f"{label}: {key!r} must be exactly {expected!r}, found {matches!r}"
            )


def locate_bash() -> str | None:
    configured = os.environ.get("TR300_TEST_BASH")
    if configured:
        candidate = Path(configured)
        if not candidate.is_file():
            raise AssertionError(f"TR300_TEST_BASH is not a file: {configured}")
        return str(candidate)
    if os.name == "nt":
        program_files = os.environ.get("ProgramFiles", r"C:\Program Files")
        git_bash = Path(program_files) / "Git" / "bin" / "bash.exe"
        if git_bash.is_file():
            return str(git_bash)
        return None
    return shutil.which("bash")


def extract_run_blocks(workflow: str) -> list[str]:
    lines = workflow.splitlines()
    blocks: list[str] = []
    index = 0
    while index < len(lines):
        match = re.match(r"^(\s*)(?:-\s+)?run:\s*\|[+-]?\s*$", lines[index])
        if match is None:
            index += 1
            continue
        indentation = len(match.group(1))
        block: list[str] = []
        index += 1
        while index < len(lines):
            line = lines[index]
            if line.strip() and len(line) - len(line.lstrip()) <= indentation:
                break
            block.append(line)
            index += 1
        blocks.append(textwrap.dedent("\n".join(block)).strip() + "\n")
    return blocks


def extract_named_run(workflow: str, step_name: str, label: str) -> str:
    lines = workflow.splitlines()
    step_index = -1
    step_indent = -1
    for index, line in enumerate(lines):
        match = re.match(r"^(\s*)- name:\s*(.+?)\s*$", line)
        if match is not None and match.group(2) == step_name:
            step_index = index
            step_indent = len(match.group(1))
            break
    if step_index < 0:
        raise AssertionError(f"{label}: missing named step {step_name!r}")

    for index in range(step_index + 1, len(lines)):
        line = lines[index]
        indentation = len(line) - len(line.lstrip())
        if line.strip() and indentation <= step_indent:
            break
        match = re.match(r"^(\s*)run:\s*\|\s*$", line)
        if match is None:
            continue
        run_indent = len(match.group(1))
        block: list[str] = []
        for body_line in lines[index + 1 :]:
            body_indent = len(body_line) - len(body_line.lstrip())
            if body_line.strip() and body_indent <= run_indent:
                break
            block.append(body_line)
        return textwrap.dedent("\n".join(block)).strip() + "\n"
    raise AssertionError(f"{label}: named step {step_name!r} has no block run body")


def extract_unique_named_run(workflow: str, step_name: str, label: str) -> str:
    matches = [
        line
        for line in workflow.splitlines()
        if (match := re.match(r"^\s*- name:\s*(.+?)\s*$", line)) is not None
        and match.group(1) == step_name
    ]
    if len(matches) != 1:
        raise AssertionError(
            f"{label}: expected one named step {step_name!r}, found {len(matches)}"
        )
    return extract_named_run(workflow, step_name, label)


def extract_unique_id_run(workflow: str, step_id: str, label: str) -> str:
    lines = workflow.splitlines()
    matches = [
        index
        for index, line in enumerate(lines)
        if (match := re.match(r"^(\s*)- id:\s*(.+?)\s*$", line)) is not None
        and match.group(2) == step_id
    ]
    if len(matches) != 1:
        raise AssertionError(
            f"{label}: expected one step id {step_id!r}, found {len(matches)}"
        )
    step_index = matches[0]
    step_indent = len(lines[step_index]) - len(lines[step_index].lstrip())
    for index in range(step_index + 1, len(lines)):
        line = lines[index]
        indentation = len(line) - len(line.lstrip())
        if line.strip() and indentation <= step_indent:
            break
        match = re.match(r"^(\s*)run:\s*\|[+-]?\s*$", line)
        if match is None:
            continue
        run_indent = len(match.group(1))
        block: list[str] = []
        for body_line in lines[index + 1 :]:
            body_indent = len(body_line) - len(body_line.lstrip())
            if body_line.strip() and body_indent <= run_indent:
                break
            block.append(body_line)
        return textwrap.dedent("\n".join(block)).strip() + "\n"
    raise AssertionError(f"{label}: step id {step_id!r} has no literal run body")


def extract_named_step(workflow: str, step_name: str, label: str) -> str:
    lines = workflow.splitlines()
    for index, line in enumerate(lines):
        match = re.match(r"^(\s*)- name:\s*(.+?)\s*$", line)
        if match is None or match.group(2) != step_name:
            continue
        indentation = len(match.group(1))
        end = index + 1
        while end < len(lines):
            candidate = lines[end]
            if re.match(rf"^\s{{{indentation}}}- (?:name:|uses:)", candidate):
                break
            if candidate.strip() and len(candidate) - len(candidate.lstrip()) <= indentation:
                break
            end += 1
        return "\n".join(lines[index:end]) + "\n"
    raise AssertionError(f"{label}: missing named step {step_name!r}")


def extract_unique_named_step(workflow: str, step_name: str, label: str) -> str:
    matches = [
        line
        for line in workflow.splitlines()
        if (match := re.match(r"^\s*- name:\s*(.+?)\s*$", line)) is not None
        and match.group(1) == step_name
    ]
    if len(matches) != 1:
        raise AssertionError(
            f"{label}: expected one named step {step_name!r}, found {len(matches)}"
        )
    return extract_named_step(workflow, step_name, label)


def extract_job(workflow: str, job_name: str, label: str) -> str:
    match = re.search(rf"(?m)^  {re.escape(job_name)}:\s*$", workflow)
    if match is None:
        raise AssertionError(f"{label}: missing job {job_name!r}")
    following = re.search(r"(?m)^  [A-Za-z0-9_-]+:\s*$", workflow[match.end() :])
    end = len(workflow) if following is None else match.end() + following.start()
    return workflow[match.start() : end]


def check_ci_job_inventory(ci: str) -> None:
    jobs_match = re.search(r"(?m)^jobs:\s*$", ci)
    if jobs_match is None:
        raise AssertionError("ci.yml: missing jobs mapping")
    job_ids = tuple(
        re.findall(r"(?m)^  ([A-Za-z0-9_-]+):\s*$", ci[jobs_match.end() :])
    )
    if job_ids != CI_JOB_IDS:
        raise AssertionError(
            f"ci.yml: logical job inventory must be {CI_JOB_IDS!r}, found {job_ids!r}"
        )

    expanded_jobs = 0
    for job_id in CI_JOB_IDS:
        job = extract_job(ci, job_id, CI_WORKFLOW.name)
        expected_oses = CI_MATRIX_OSES.get(job_id)
        matrix_lines = re.findall(r"(?m)^      matrix:\s*$", job)
        if expected_oses is None:
            if matrix_lines:
                raise AssertionError(f"ci.yml:{job_id}: unexpected matrix expansion")
            expanded_jobs += 1
            continue
        if len(matrix_lines) != 1:
            raise AssertionError(
                f"ci.yml:{job_id}: expected one matrix mapping, found {len(matrix_lines)}"
            )
        matrix_start = re.search(r"(?m)^      matrix:\s*$", job)
        steps_start = re.search(r"(?m)^    steps:\s*$", job)
        if matrix_start is None or steps_start is None or matrix_start.end() >= steps_start.start():
            raise AssertionError(f"ci.yml:{job_id}: malformed matrix/steps ordering")
        matrix = job[matrix_start.end() : steps_start.start()]
        active_oses = tuple(
            re.findall(r"(?m)^          - os:\s*([A-Za-z0-9._-]+)\s*$", matrix)
        )
        if active_oses != expected_oses:
            raise AssertionError(
                f"ci.yml:{job_id}: matrix OS inventory must be {expected_oses!r}, "
                f"found {active_oses!r}"
            )
        expanded_jobs += len(active_oses)
    if expanded_jobs != 20:
        raise AssertionError(
            f"ci.yml: expected exactly 20 expanded jobs, found {expanded_jobs}"
        )


def native_apple_job_bodies(release: str, macos: str) -> list[tuple[str, str]]:
    workflows = {
        "release.yml": release,
        "macos-installer.yml": macos,
    }
    return [
        (
            f"{workflow_name}:{job_name}",
            extract_job(workflows[workflow_name], job_name, workflow_name),
        )
        for workflow_name, job_name in NATIVE_APPLE_JOBS
    ]


def native_apple_build_blocks(release: str) -> list[tuple[str, str]]:
    build = extract_job(release, "build-local-artifacts", RELEASE_WORKFLOW.name)
    declarations = re.findall(r"(?m)^\s+(?:-\s+)?run:\s*.*$", build)
    if len(declarations) != len(NATIVE_APPLE_BUILD_STEPS) + 1:
        raise AssertionError(
            "release.yml:build-local-artifacts: every run step must be classified "
            "for native Apple Bash compatibility"
        )
    windows_install = extract_named_step(
        build,
        "Install pinned cargo-dist on Windows",
        "release.yml:build-local-artifacts",
    )
    require_exact_step(
        windows_install,
        """
        - name: Install pinned cargo-dist on Windows
          if: runner.os == 'Windows'
          shell: pwsh
          run: ./scripts/install-pinned-cargo-dist.ps1
        """,
        "release.yml:build-local-artifacts Windows-only exception",
    )
    blocks = [
        (
            f"release.yml:build-local-artifacts:{step_name}",
            extract_unique_named_run(build, step_name, "release.yml:build-local-artifacts"),
        )
        for step_name in NATIVE_APPLE_BUILD_STEPS
        if step_name != "Post-build"
    ]
    blocks.append(
        (
            "release.yml:build-local-artifacts:Post-build",
            extract_unique_id_run(build, "cargo-dist", "release.yml:build-local-artifacts"),
        )
    )
    return blocks


def mask_shell_noncode(source: str) -> list[tuple[int, str]]:
    """Mask quoted strings and here-doc bodies while preserving line positions."""
    masked_lines: list[tuple[int, str]] = []
    quote: str | None = None
    heredoc: str | None = None
    for line_number, line in enumerate(source.splitlines(), start=1):
        if heredoc is not None:
            if line.strip() == heredoc:
                heredoc = None
            masked_lines.append((line_number, ""))
            continue

        quote_at_start = quote
        masked: list[str] = []
        index = 0
        while index < len(line):
            char = line[index]
            if quote is not None:
                masked.append(" ")
                if quote == '"' and char == "\\" and index + 1 < len(line):
                    index += 1
                    masked.append(" ")
                elif char == quote:
                    quote = None
            elif char in ("'", '"'):
                quote = char
                masked.append(" ")
            elif char == "\\" and index + 1 < len(line):
                masked.append(char)
                index += 1
                masked.append(" ")
            elif char == "#" and (
                index == 0 or line[index - 1].isspace() or line[index - 1] in ";|&(){}"
            ):
                masked.extend(" " * (len(line) - index))
                break
            else:
                masked.append(char)
            index += 1
        masked_lines.append((line_number, "".join(masked)))

        if quote_at_start is None:
            operator = re.search(r"(?<!<)<<(?!<)-?[ \t]*", masked_lines[-1][1])
            if operator is not None:
                marker = re.match(
                    r"<<-?[ \t]*(['\"])([A-Za-z_][A-Za-z0-9_]*)\1"
                    r"(?=$|[ \t;|&])",
                    line[operator.start() :],
                )
                if marker is None:
                    raise AssertionError(
                        f"line {line_number}: native Apple run blocks require a "
                        "quoted simple here-doc delimiter; ambiguous << is forbidden"
                    )
                heredoc = marker.group(2)
    return masked_lines


def bash_test_closing_suffix(
    lines: list[tuple[int, str]], start: int, label: str
) -> str:
    for _, line in lines[start:]:
        closing = re.search(r"(?<!\S)\]\](?=$|[ \t;&|])", line)
        if closing is not None:
            return line[closing.end() :].strip()
    raise AssertionError(
        f"{label}:{lines[start][0]}: unterminated Bash compound command"
    )


def bash_subshell_closing_suffix(
    lines: list[tuple[int, str]], start: int, label: str
) -> tuple[str, int, int]:
    depth = 0
    for offset, (_, line) in enumerate(lines[start:], start=start):
        begin = line.find("(") if offset == start else 0
        for position in range(begin, len(line)):
            if line[position] == "(":
                depth += 1
            elif line[position] == ")":
                depth -= 1
                if depth == 0:
                    return line[position + 1 :].strip(), offset, position
                if depth < 0:
                    break
    raise AssertionError(
        f"{label}:{lines[start][0]}: unterminated Bash subshell command"
    )


def reject_unhandled_bash3_compound_guards(source: str, label: str) -> None:
    """Enforce canonical fail-closed compound commands for macOS Bash 3.2."""
    lines = mask_shell_noncode(source)
    for index, (line_number, line) in enumerate(lines):
        stripped = line.lstrip()
        if not stripped:
            continue

        control = re.match(
            r"^(if|elif|while|until)[ \t]+(?:![ \t]+)?(\[\[|\(\()", stripped
        )
        if control is not None:
            if control.group(2) == "((":
                raise AssertionError(
                    f"{label}:{line_number}: use [[ ]] for native Apple arithmetic "
                    "control conditions so the Bash 3.2 contract stays unambiguous"
                )
            suffix = bash_test_closing_suffix(lines, index, label)
            expected = "; then" if control.group(1) in ("if", "elif") else "; do"
            if suffix != expected:
                raise AssertionError(
                    f"{label}:{line_number}: native Apple compound control must end "
                    f"canonically with `{expected}`"
                )
            continue

        if stripped.startswith("[["):
            suffix = bash_test_closing_suffix(lines, index, label)
            if suffix != "|| exit 1":
                raise AssertionError(
                    f"{label}:{line_number}: standalone Bash compound guard must "
                    "end with exact `|| exit 1` for macOS Bash 3.2"
                )
            continue

        if stripped.startswith("(("):
            raise AssertionError(
                f"{label}:{line_number}: standalone Bash arithmetic conditions must "
                "use explicit if/while control flow for macOS Bash 3.2"
            )

        if stripped.startswith("("):
            suffix, closing_index, closing_position = bash_subshell_closing_suffix(
                lines, index, label
            )
            if suffix != "|| exit 1":
                raise AssertionError(
                    f"{label}:{line_number}: standalone Bash subshell must end "
                    "with exact `|| exit 1` for macOS Bash 3.2"
                )
            if stripped == "(":
                for interior_index in range(index + 1, closing_index + 1):
                    interior = lines[interior_index][1]
                    if interior_index == closing_index:
                        interior = interior[:closing_position]
                    interior = interior.strip()
                    if not interior or interior.endswith("\\"):
                        continue
                    if not interior.endswith("|| exit 1"):
                        raise AssertionError(
                            f"{label}:{lines[interior_index][0]}: every complete "
                            "command in a multiline Bash 3.2 subshell must end "
                            "with exact `|| exit 1`"
                        )
            else:
                opening_position = line.find("(")
                if closing_index == index:
                    fragments = [line[opening_position + 1 : closing_position]]
                else:
                    fragments = [line[opening_position + 1 :]]
                    fragments.extend(
                        candidate
                        for _, candidate in lines[index + 1 : closing_index]
                    )
                    fragments.append(lines[closing_index][1][:closing_position])
                interior = "\n".join(fragments)
                if re.search(r";|\|\||(?<!\|)\|(?!\|)|(?<!&)&(?!&)", interior):
                    raise AssertionError(
                        f"{label}:{line_number}: inline Bash 3.2 subshells must "
                        "contain one command or a status-preserving && list"
                    )
            continue

        prefixed_compound = re.search(
            r"(?:[;|&!]|\{|\)|\bthen|\bdo|\belse)[ \t]*"
            r"(?:\[\[|\(\(|\((?![<(]))",
            stripped,
        )
        timed_compound = re.search(
            r"(?:^|[;|&])[ \t]*(?:![ \t]+)*time(?:[ \t]+-p)?"
            r"(?:[ \t]+!)*[ \t]+"
            r"(?:\[\[|\(\(|\((?![<(]))",
            stripped,
        )
        if prefixed_compound or timed_compound:
            raise AssertionError(
                f"{label}:{line_number}: Bash compound commands must use one "
                "canonical control or fail-closed command per line"
            )


def check_bash3_compound_guard_scanner() -> None:
    reject_unhandled_bash3_compound_guards(
        "if [[ x == x ]]; then\n  :\nfi\n"
        "while [[ x == x ]]; do\n  break\ndone\n"
        "value=$((1 + 1))\n"
        "grep -E '^[[:space:]]+$' input\n"
        "key=${line#*\\\"}\n"
        "jq '.value and\n  (.nested | type == \"string\")' input\n"
        "cat <<'SH'\n[[ ignored ]]\n(ignored)\nSH\n"
        "(false) || exit 1\n(\n  false || exit 1\n) || exit 1",
        "scanner valid control-flow fixture",
    )
    reject_unhandled_bash3_compound_guards(
        '[[ "$hash" =~ ^[[:xdigit:]]{40}$ &&\n'
        '   "$size" == 1 ]] || exit 1',
        "scanner valid fixture",
    )
    invalid = (
        "[[ x == y ]]",
        "[[ x == y ]] # bypass\n[[ x == x ]] || exit 1",
        "[[ x == y ]]; true\n[[ x == x ]] || exit 1",
        "[[ x ==\n   y ]] # bypass\n[[ x == x ]] || exit 1",
        "(( 1 == 0 )) || exit 1",
        ":; [[ x == y ]] || exit 1",
        "! [[ x == y ]] || exit 1",
        "{ [[ x == y ]] || exit 1; }",
        "true && [[ x == y ]] || exit 1",
        "[[ x == y ]] || exit 256",
        "if [[ x == x ]]; then :; fi; [[ x == y ]]",
        "if [[ x == x ]]; then [[ x == y ]]; fi",
        "while [[ x == x ]]; do [[ x == y ]]; done",
        "else [[ x == y ]] || exit 1",
        "case x in x) [[ x == y ]] || exit 1 ;; esac",
        "(false)",
        "(\n  false\n)",
        "true; (false) || exit 1",
        "(false; true) || exit 1",
        "(\n  false\n  true\n) || exit 1",
        "true | [[ x == y ]] || exit 1",
        "true & [[ x == y ]] || exit 1",
        "time [[ x == y ]]; true",
        "time -p (false); true",
        "! time [[ x == y ]]; true",
        "! ! time -p (false); true",
        "printf '%s' '<<EOF'\n[[ x == y ]]",
        "cat <<EOF-END\nignored\nEOF-END\n[[ x == y ]]",
        "cat <<EOF.END\nignored\nEOF.END\n[[ x == y ]]",
        "x=$((1 << BITS))\n[[ x == y ]]",
        "true;# <<'EOF'\n[[ x == y ]]",
        "(\n  (\n    false\n  ) || exit 1\n)\ntrue",
        "(\n  value=$(true) || exit 1\n  false\n)",
    )
    for case_number, source in enumerate(invalid, start=1):
        try:
            reject_unhandled_bash3_compound_guards(
                source, f"scanner invalid fixture {case_number}"
            )
        except AssertionError:
            continue
        raise AssertionError(
            f"Bash 3.2 compound-guard scanner accepted invalid fixture {case_number}"
        )


def check_native_apple_bash_contract(release: str, macos: str) -> None:
    check_bash3_compound_guard_scanner()
    for workflow_name, workflow in (
        (RELEASE_WORKFLOW.name, release),
        (MACOS_WORKFLOW.name, macos),
    ):
        if re.search(r"(?m)^defaults:\s*$", workflow):
            raise AssertionError(
                f"{workflow_name}: top-level shell defaults require explicit native "
                "Apple compatibility review"
            )
    apple_staging_step = extract_unique_named_step(
        release, APPLE_STAGING_STEP_NAME, RELEASE_WORKFLOW.name
    )
    require_direct_step_contract(
        apple_staging_step,
        ("env", "shell", "run"),
        {"shell": "bash", "run": "|"},
        "Apple signer staging step",
    )
    require_exact_line_sequence(
        apple_staging_step,
        """
        - name: Validate and safely stage the unsigned Apple archive
          env:
            APPLE_TARGET: ${{ matrix.target }}
            RELEASE_TAG: ${{ needs.plan.outputs.tag }}
            STAGING_DIRECTORY: ${{ runner.temp }}/tr300-apple-signing-input
          shell: bash
        """,
        "Apple signer staging environment",
    )
    apple_staging = extract_unique_named_run(
        release, APPLE_STAGING_STEP_NAME, RELEASE_WORKFLOW.name
    )
    require(apple_staging, "with os.scandir(directory)", "Apple input file-set scan")
    require(
        apple_staging,
        "stat.S_ISREG(metadata.st_mode)",
        "Apple input regular-file validation",
    )
    require_exact_line_sequence(
        apple_staging,
        r"""
        if ! [[ "$RELEASE_TAG" =~ ^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
          echo "unsupported Apple signing tag: $RELEASE_TAG" >&2
          exit 64
        fi
        """,
        "Apple signer explicit stable-tag rejection",
    )
    require_exact_line_sequence(
        apple_staging,
        r"""
        if ! [[ "$archive_sha" =~ ^[0-9a-f]{64}$ ]]; then
          echo "unsigned Apple archive checksum is malformed" >&2
          exit 1
        fi
        if ! python3 -c 'import pathlib, sys; path, expected = sys.argv[1:]; raise SystemExit(pathlib.Path(path).read_bytes() != (expected + "\n\n").encode("utf-8"))' \
          "unsigned/$sidecar" "$expected_sidecar"; then
          echo "unsigned Apple checksum sidecar is malformed or does not match" >&2
          exit 1
        fi
        """,
        "Apple signer explicit sidecar rejection",
    )
    native_jobs = native_apple_job_bodies(release, macos)
    native_build_blocks = native_apple_build_blocks(release)
    for label, job in native_jobs:
        for block_number, block in enumerate(extract_run_blocks(job), start=1):
            reject_unhandled_bash3_compound_guards(
                block, f"{label}:run-{block_number}"
            )
    for label, block in native_build_blocks:
        reject_unhandled_bash3_compound_guards(block, label)

    native_sources = native_jobs + native_build_blocks
    for label, job in native_sources:
        explicit_shells = re.findall(r"(?m)^\s+shell:\s*(.+?)\s*$", job)
        unsupported_shells = [shell for shell in explicit_shells if shell != "bash"]
        if unsupported_shells:
            raise AssertionError(
                f"{label}: native macOS run steps must use Bash; found explicit "
                f"shells {unsupported_shells!r}"
            )
        for pattern, description in BASH_4_ONLY_PATTERNS:
            if match := pattern.search(job):
                raise AssertionError(
                    f"{label}: native macOS job must run under system Bash 3.2; "
                    f"found unsupported {description} {match.group(0)!r}"
                )


def check_native_apple_bash_syntax(release: str, macos: str, bash: str) -> None:
    check_native_apple_bash_contract(release, macos)
    for label, job in native_apple_job_bodies(release, macos):
        blocks = extract_run_blocks(job)
        if not blocks:
            raise AssertionError(f"{label}: expected at least one inline run block")
        declarations = re.findall(r"(?m)^\s+(?:-\s+)?run:\s*.*$", job)
        if len(blocks) != len(declarations):
            raise AssertionError(
                f"{label}: native Apple syntax validation requires every run block "
                "to use a literal YAML scalar"
            )
        for index, block in enumerate(blocks, start=1):
            result = subprocess.run(
                [bash, "--noprofile", "--norc", "-n", "-s"],
                input=block,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=30,
                check=False,
            )
            if result.returncode != 0:
                raise AssertionError(
                    f"{label}: run block {index} is not valid for the selected Bash "
                    f"(exit {result.returncode})\nstdout:\n{result.stdout}\n"
                    f"stderr:\n{result.stderr}"
                )
    for label, block in native_apple_build_blocks(release):
        result = subprocess.run(
            [bash, "--noprofile", "--norc", "-n", "-s"],
            input=block,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=30,
            check=False,
        )
        if result.returncode != 0:
            raise AssertionError(
                f"{label}: Apple matrix block is not valid for the selected Bash "
                f"(exit {result.returncode})\nstdout:\n{result.stdout}\n"
                f"stderr:\n{result.stderr}"
            )


def bash_major_minor(bash: str) -> str:
    result = subprocess.run(
        [
            bash,
            "--noprofile",
            "--norc",
            "-c",
            'printf "%s.%s" "${BASH_VERSINFO[0]}" "${BASH_VERSINFO[1]}"',
        ],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=10,
        check=False,
    )
    if result.returncode != 0 or not re.fullmatch(r"[0-9]+\.[0-9]+", result.stdout):
        raise AssertionError(
            f"could not identify selected Bash version (exit {result.returncode}): "
            f"{result.stderr}"
        )
    return result.stdout


def write_apple_staging_fixture(
    case_dir: Path, target: str, *, release_tag: str, mutation: str | None
) -> None:
    unsigned = case_dir / "unsigned"
    unsigned.mkdir(exist_ok=True)
    archive_name = f"tr300-{target}.tar.xz"
    archive_path = unsigned / archive_name
    root = f"tr300-{target}"
    members = (
        (f"{root}/LICENSE", b"fixture license\n", 0o644),
        (f"{root}/CHANGELOG.md", b"fixture changelog\n", 0o644),
        (f"{root}/README.md", b"fixture readme\n", 0o644),
        (f"{root}/tr300", b"#!/bin/sh\nexit 0\n", 0o755),
    )
    raw_archive = io.BytesIO()
    with tarfile.open(fileobj=raw_archive, mode="w", format=tarfile.GNU_FORMAT) as archive:
        directory = tarfile.TarInfo(root)
        directory.type = tarfile.DIRTYPE
        directory.mode = 0o755
        directory.mtime = 0
        archive.addfile(directory)
        for name, contents, mode in members:
            member = tarfile.TarInfo(name)
            member.mode = mode
            member.mtime = 0
            if mutation in ("archive-symlink", "archive-hardlink") and name.endswith(
                "/tr300"
            ):
                member.type = (
                    tarfile.SYMTYPE
                    if mutation == "archive-symlink"
                    else tarfile.LNKTYPE
                )
                member.linkname = f"{root}/README.md"
                archive.addfile(member)
                continue
            member.size = len(contents)
            archive.addfile(member, io.BytesIO(contents))

    raw_bytes = bytearray(raw_archive.getvalue())
    if mutation != "archive-permission-only-modes":
        encoded_modes = {root: stat.S_IFDIR | 0o755}
        encoded_modes.update(
            {
                name: stat.S_IFREG | mode
                for name, _contents, mode in members
            }
        )
        binary_name = f"{root}/tr300"
        if mutation == "archive-mode-type-mismatch":
            encoded_modes[binary_name] = stat.S_IFDIR | 0o755
        elif mutation == "archive-setuid":
            encoded_modes[binary_name] = stat.S_IFREG | 0o4755

        offset = 0
        patched_names: set[str] = set()
        while offset + 512 <= len(raw_bytes):
            header = raw_bytes[offset : offset + 512]
            if not any(header):
                break
            name = (
                bytes(header[0:100])
                .split(b"\0", 1)[0]
                .decode("utf-8")
                .rstrip("/")
            )
            size_field = bytes(header[124:136]).rstrip(b"\0 ") or b"0"
            size = int(size_field, 8)
            if name in encoded_modes:
                patched_names.add(name)
                raw_mode = f"{encoded_modes[name]:07o}\0".encode("ascii")
                if len(raw_mode) != 8:
                    raise AssertionError(f"fixture mode is not tar-compatible: {raw_mode!r}")
                raw_bytes[offset + 100 : offset + 108] = raw_mode
                raw_bytes[offset + 148 : offset + 156] = b"        "
                checksum = sum(raw_bytes[offset : offset + 512])
                raw_checksum = f"{checksum:06o}\0 ".encode("ascii")
                raw_bytes[offset + 148 : offset + 156] = raw_checksum
            offset += 512 + ((size + 511) // 512) * 512
        if patched_names != set(encoded_modes):
            raise AssertionError(
                f"fixture did not patch every expected tar mode: {patched_names!r}"
            )

    archive_path.write_bytes(lzma.compress(bytes(raw_bytes), format=lzma.FORMAT_XZ))

    digest = hashlib.sha256(archive_path.read_bytes()).hexdigest()
    sidecar_name = f"{archive_name}.sha256"
    (unsigned / sidecar_name).write_text(
        f"{digest} *{archive_name}\n\n", encoding="utf-8", newline="\n"
    )
    manifest_name = f"{target}-dist-manifest.json"
    (unsigned / manifest_name).write_text(
        json.dumps(
            {
                "dist_version": "0.31.0",
                "announcement_tag": release_tag,
                "upload_files": [archive_name, sidecar_name],
                "artifacts": {archive_name: {"checksums": {"sha256": digest}}},
            }
        )
        + "\n",
        encoding="utf-8",
        newline="\n",
    )
    if mutation == "extra":
        (unsigned / ".unexpected").write_text("reject me\n", encoding="utf-8")
    elif mutation == "symlink":
        sidecar = unsigned / sidecar_name
        sidecar.unlink()
        try:
            sidecar.symlink_to(archive_name)
        except OSError:
            if os.name != "nt":
                raise
            # Unprivileged Windows hosts may prohibit symlink creation. A
            # directory exercises the same non-regular-input rejection locally;
            # both native macOS CI legs still exercise the real symlink case.
            sidecar.mkdir()
    elif mutation == "sidecar":
        (unsigned / sidecar_name).write_text(
            f"{'0' * 64} *{archive_name}\n\n", encoding="utf-8", newline="\n"
        )
    elif mutation == "sidecar-single-newline":
        (unsigned / sidecar_name).write_text(
            f"{digest} *{archive_name}\n", encoding="utf-8", newline="\n"
        )
    elif mutation == "sidecar-extra-newline":
        (unsigned / sidecar_name).write_text(
            f"{digest} *{archive_name}\n\n\n", encoding="utf-8", newline="\n"
        )
    elif mutation == "sidecar-crlf":
        (unsigned / sidecar_name).write_bytes(
            f"{digest} *{archive_name}\r\n\r\n".encode("utf-8")
        )
    elif mutation == "sidecar-same-length-tail":
        (unsigned / sidecar_name).write_bytes(
            f"{digest} *{archive_name}\nX".encode("utf-8")
        )
    elif mutation in (
        "archive-symlink",
        "archive-hardlink",
        "archive-permission-only-modes",
        "archive-mode-type-mismatch",
        "archive-setuid",
    ):
        pass
    elif mutation is not None:
        raise AssertionError(f"unknown Apple staging mutation: {mutation}")


def run_apple_staging_compatibility_fixture(release: str, bash: str) -> None:
    block = extract_unique_named_run(
        release, APPLE_STAGING_STEP_NAME, RELEASE_WORKFLOW.name
    )
    for target, mutation, release_tag, expected_success in (
        ("aarch64-apple-darwin", None, "v99.99.99", True),
        ("x86_64-apple-darwin", None, "v99.99.99", True),
        (
            "aarch64-apple-darwin",
            "archive-permission-only-modes",
            "v99.99.99",
            True,
        ),
        ("aarch64-apple-darwin", "extra", "v99.99.99", False),
        ("aarch64-apple-darwin", "symlink", "v99.99.99", False),
        ("aarch64-apple-darwin", "sidecar", "v99.99.99", False),
        ("aarch64-apple-darwin", "sidecar-single-newline", "v99.99.99", False),
        ("aarch64-apple-darwin", "sidecar-extra-newline", "v99.99.99", False),
        ("aarch64-apple-darwin", "sidecar-crlf", "v99.99.99", False),
        ("aarch64-apple-darwin", "sidecar-same-length-tail", "v99.99.99", False),
        ("aarch64-apple-darwin", "archive-symlink", "v99.99.99", False),
        ("aarch64-apple-darwin", "archive-hardlink", "v99.99.99", False),
        ("aarch64-apple-darwin", "archive-mode-type-mismatch", "v99.99.99", False),
        ("aarch64-apple-darwin", "archive-setuid", "v99.99.99", False),
        ("aarch64-apple-darwin", None, "v99.99.99-rc.1", False),
    ):
        with tempfile.TemporaryDirectory(prefix="tr300-apple-staging-") as case_raw:
            case_dir = Path(case_raw)
            write_apple_staging_fixture(
                case_dir, target, release_tag=release_tag, mutation=mutation
            )
            staging = case_dir / "staging"
            environment = os.environ.copy()
            fixture_block = block
            if os.name == "nt":
                compat_bin = case_dir / "bin"
                compat_bin.mkdir()
                write_windows_mkdir_compat(compat_bin)
                write_windows_shasum_compat(compat_bin)
                write_fixture_jq_compat(compat_bin)
                conversion_environment = environment.copy()
                conversion_environment["TR300_COMPAT_BIN"] = str(compat_bin)
                path_probe = subprocess.run(
                    [
                        bash,
                        "--noprofile",
                        "--norc",
                        "-c",
                        'printf "%s\\n%s" "$(cygpath -u "$TR300_COMPAT_BIN")" "$PATH"',
                    ],
                    env=conversion_environment,
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    timeout=10,
                    check=False,
                )
                converted = path_probe.stdout.splitlines()
                if path_probe.returncode != 0 or len(converted) != 2:
                    raise AssertionError(
                        "could not prepare Git Bash fixture PATH: "
                        f"{path_probe.stderr}"
                    )
                environment["TR300_COMPAT_BIN"] = converted[0]
                fixture_block = (
                    'mkdir() { "$TR300_COMPAT_BIN/mkdir" "$@"; }\n'
                    'shasum() { "$TR300_COMPAT_BIN/shasum" "$@"; }\n'
                    'jq() { "$TR300_COMPAT_BIN/jq" "$@"; }\n'
                    + block
                )
            environment.update(
                {
                    "APPLE_TARGET": target,
                    "RELEASE_TAG": release_tag,
                    "STAGING_DIRECTORY": str(staging),
                }
            )
            result = subprocess.run(
                [bash, "--noprofile", "--norc", "-e", "-o", "pipefail", "-s"],
                cwd=case_dir,
                env=environment,
                input=fixture_block,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=30,
                check=False,
            )
            succeeded = result.returncode == 0
            if succeeded != expected_success:
                raise AssertionError(
                    f"Apple staging compatibility target={target} mutation={mutation} "
                    f"release_tag={release_tag} "
                    f"returned {result.returncode}\nstdout:\n{result.stdout}\n"
                    f"stderr:\n{result.stderr}"
                )
            staged_binary = staging / f"tr300-{target}" / "tr300"
            if expected_success:
                if not staged_binary.is_file() or not os.access(staged_binary, os.X_OK):
                    raise AssertionError("Apple staging did not produce an executable binary")
            elif mutation in (
                "archive-symlink",
                "archive-hardlink",
                "archive-mode-type-mismatch",
                "archive-setuid",
            ):
                if staged_binary.exists():
                    raise AssertionError(
                        "unsafe Apple archive fixture produced a staged binary"
                    )
            elif staging.exists():
                raise AssertionError("rejected Apple staging fixture created output")


def run_macos_archive_mode_compatibility_fixtures(macos: str) -> None:
    prepare = extract_unique_named_run(
        macos,
        "Safely prepare a fixed data-only universal installer payload",
        MACOS_WORKFLOW.name,
    )
    program = extract_unique_python_heredoc(
        prepare,
        'for target in ("aarch64-apple-darwin", "x86_64-apple-darwin"):',
        "macOS universal input archive extraction",
    )
    for mutation, expected_success in (
        (None, True),
        ("archive-permission-only-modes", True),
        ("archive-mode-type-mismatch", False),
        ("archive-setuid", False),
        ("archive-symlink", False),
        ("archive-hardlink", False),
    ):
        with tempfile.TemporaryDirectory(prefix="tr300-macos-archive-") as case_raw:
            case_dir = Path(case_raw)
            for target in ("aarch64-apple-darwin", "x86_64-apple-darwin"):
                write_apple_staging_fixture(
                    case_dir,
                    target,
                    release_tag="v99.99.99",
                    mutation=mutation,
                )
            output = case_dir / "output"
            output.mkdir()
            result = subprocess.run(
                [sys.executable, "-", str(case_dir / "unsigned"), str(output)],
                input=program,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=30,
                check=False,
            )
            succeeded = result.returncode == 0
            if succeeded != expected_success:
                raise AssertionError(
                    f"macOS archive mode mutation={mutation} returned "
                    f"{result.returncode}, expected_success={expected_success}\n"
                    f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
                )
            binaries = tuple(output.glob("*/tr300"))
            if expected_success:
                if len(binaries) != 2 or not all(path.is_file() for path in binaries):
                    raise AssertionError(
                        "macOS archive fixture did not produce both architecture binaries"
                    )
            elif binaries:
                raise AssertionError(
                    "rejected macOS archive fixture produced an architecture binary"
                )


def extract_unique_python_heredoc(block: str, marker: str, label: str) -> str:
    matches = [
        textwrap.dedent(match.group(1)).strip() + "\n"
        for match in re.finditer(r"<<'PY'\n(.*?)\nPY(?:\n|$)", block, re.DOTALL)
        if marker in match.group(1)
    ]
    if len(matches) != 1:
        raise AssertionError(
            f"{label}: expected one Python heredoc containing {marker!r}, "
            f"found {len(matches)}"
        )
    return matches[0]


def run_release_sidecar_normalization_fixtures(release: str) -> None:
    prepare = extract_unique_named_run(
        release,
        "Render and validate the fixed 24-asset initial release",
        RELEASE_WORKFLOW.name,
    )
    program = extract_unique_python_heredoc(
        prepare,
        "raw_cargo_dist_payloads = {",
        "release sidecar normalization",
    )
    payload_names = (
        "source.tar.gz",
        "tr300-aarch64-apple-darwin.tar.xz",
        "tr300-aarch64-unknown-linux-gnu.tar.xz",
        "tr300-x86_64-apple-darwin.tar.xz",
        "tr300-x86_64-pc-windows-msvc.msi",
        "tr300-x86_64-pc-windows-msvc.zip",
        "tr300-x86_64-unknown-linux-gnu.tar.xz",
        "tr300-x86_64-unknown-linux-musl.tar.xz",
    )
    raw_payload_names = {
        "source.tar.gz",
        "tr300-aarch64-unknown-linux-gnu.tar.xz",
        "tr300-x86_64-pc-windows-msvc.msi",
        "tr300-x86_64-pc-windows-msvc.zip",
        "tr300-x86_64-unknown-linux-gnu.tar.xz",
        "tr300-x86_64-unknown-linux-musl.tar.xz",
    }

    for mutation, expected_success in (
        (None, True),
        ("raw-single-newline", False),
        ("raw-triple-newline", False),
        ("raw-crlf", False),
        ("raw-same-length-tail", False),
        ("signed-double-newline", False),
        ("aggregate-single-newline", False),
        ("aggregate-triple-newline", False),
        ("aggregate-mismatch", False),
    ):
        with tempfile.TemporaryDirectory(prefix="tr300-release-sidecars-") as case_raw:
            case_dir = Path(case_raw)
            records: dict[str, bytes] = {}
            for index, payload_name in enumerate(payload_names, start=1):
                payload = f"fixture payload {index}: {payload_name}\n".encode()
                (case_dir / payload_name).write_bytes(payload)
                record = (
                    f"{hashlib.sha256(payload).hexdigest()} *{payload_name}".encode()
                )
                records[payload_name] = record
                ending = b"\n\n" if payload_name in raw_payload_names else b"\n"
                (case_dir / f"{payload_name}.sha256").write_bytes(record + ending)

            canonical_checksum = b"".join(
                records[payload_name] + b"\n" for payload_name in payload_names
            )
            checksum_path = case_dir / "sha256.sum"
            checksum_path.write_bytes(canonical_checksum + b"\n")

            raw_name = "source.tar.gz"
            raw_sidecar = case_dir / f"{raw_name}.sha256"
            signed_name = "tr300-aarch64-apple-darwin.tar.xz"
            signed_sidecar = case_dir / f"{signed_name}.sha256"
            if mutation == "raw-single-newline":
                raw_sidecar.write_bytes(records[raw_name] + b"\n")
            elif mutation == "raw-triple-newline":
                raw_sidecar.write_bytes(records[raw_name] + b"\n\n\n")
            elif mutation == "raw-crlf":
                raw_sidecar.write_bytes(records[raw_name] + b"\r\n\r\n")
            elif mutation == "raw-same-length-tail":
                raw_sidecar.write_bytes(records[raw_name] + b"\nX")
            elif mutation == "signed-double-newline":
                signed_sidecar.write_bytes(records[signed_name] + b"\n\n")
            elif mutation == "aggregate-single-newline":
                checksum_path.write_bytes(canonical_checksum)
            elif mutation == "aggregate-triple-newline":
                checksum_path.write_bytes(canonical_checksum + b"\n\n")
            elif mutation == "aggregate-mismatch":
                checksum_path.write_bytes(b"0" + canonical_checksum[1:] + b"\n")
            elif mutation is not None:
                raise AssertionError(f"unknown release sidecar mutation: {mutation}")

            before = {
                path.name: path.read_bytes() for path in case_dir.iterdir() if path.is_file()
            }
            result = subprocess.run(
                [sys.executable, "-", str(case_dir)],
                input=program,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=15,
                check=False,
            )
            succeeded = result.returncode == 0
            if succeeded != expected_success:
                raise AssertionError(
                    f"release sidecar normalization mutation={mutation} returned "
                    f"{result.returncode}, expected_success={expected_success}\n"
                    f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
                )
            if expected_success:
                for payload_name in payload_names:
                    actual = (case_dir / f"{payload_name}.sha256").read_bytes()
                    expected = records[payload_name] + b"\n"
                    if actual != expected:
                        raise AssertionError(
                            f"release sidecar was not canonicalized: {payload_name}"
                        )
                if checksum_path.read_bytes() != canonical_checksum:
                    raise AssertionError("release aggregate checksum was not canonicalized")
            else:
                after = {
                    path.name: path.read_bytes()
                    for path in case_dir.iterdir()
                    if path.is_file()
                }
                if after != before:
                    raise AssertionError(
                        f"rejected release sidecar mutation changed files: {mutation}"
                    )


def run_release_manifest_preservation_fixtures(release: str, bash: str) -> None:
    prepare = extract_unique_named_run(
        release,
        "Render and validate the fixed 24-asset initial release",
        RELEASE_WORKFLOW.name,
    )
    start_marker = "printf '%s\\n' \"$PLAN_MANIFEST\" | jq -e '"
    end_marker = '  > "$OUTPUT_DIRECTORY/__tr300-release-metadata.json"\n'
    start = prepare.find(start_marker)
    end = prepare.find(end_marker, start)
    if start < 0 or end < 0:
        raise AssertionError("release manifest preservation fragment was not found")
    fragment = prepare[start : end + len(end_marker)]

    with tempfile.TemporaryDirectory(prefix="tr300-release-manifest-") as case_raw:
        case_dir = Path(case_raw)
        environment = os.environ.copy()
        fixture_prefix = "set -euo pipefail\n"
        if os.name == "nt":
            compat_bin = case_dir / "bin"
            compat_bin.mkdir()
            write_fixture_jq_compat(compat_bin)
            conversion_environment = environment.copy()
            conversion_environment["TR300_COMPAT_BIN"] = str(compat_bin)
            path_probe = subprocess.run(
                [
                    bash,
                    "--noprofile",
                    "--norc",
                    "-c",
                    'cygpath -u "$TR300_COMPAT_BIN"',
                ],
                env=conversion_environment,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=10,
                check=False,
            )
            if path_probe.returncode != 0 or not path_probe.stdout.strip():
                raise AssertionError(
                    "could not prepare release manifest fixture jq path: "
                    f"{path_probe.stderr}"
                )
            environment["TR300_COMPAT_BIN"] = path_probe.stdout.strip()
            fixture_prefix += 'jq() { "$TR300_COMPAT_BIN/jq" "$@"; }\n'
        elif shutil.which("jq") is None:
            raise AssertionError("jq is required for release manifest fixtures")

        for mutation, expected_success in (
            (None, True),
            ("prerelease", False),
            ("missing-title", False),
            ("empty-title", False),
            ("nonstring-title", False),
            ("missing-body", False),
            ("empty-body", False),
            ("nonstring-body", False),
            ("boolean-root", False),
            ("null-root", False),
        ):
            manifest: object = {
                "dist_version": "0.31.0",
                "announcement_tag": "v99.99.99",
                "announcement_is_prerelease": False,
                "announcement_title": "99.99.99 - fixture",
                "announcement_github_body": "fixture release notes",
                "artifacts": {"fixture": {"kind": "sentinel"}},
                "releases": [{"app_name": "tr300", "display": True}],
                "fixture_marker": {"preserve": [True, 7, "all fields"]},
            }
            if mutation == "boolean-root":
                manifest = True
            elif mutation == "null-root":
                manifest = None
            else:
                assert isinstance(manifest, dict)
                if mutation == "prerelease":
                    manifest["announcement_is_prerelease"] = True
                elif mutation == "missing-title":
                    del manifest["announcement_title"]
                elif mutation == "empty-title":
                    manifest["announcement_title"] = ""
                elif mutation == "nonstring-title":
                    manifest["announcement_title"] = 7
                elif mutation == "missing-body":
                    del manifest["announcement_github_body"]
                elif mutation == "empty-body":
                    manifest["announcement_github_body"] = ""
                elif mutation == "nonstring-body":
                    manifest["announcement_github_body"] = ["not", "a", "string"]
                elif mutation is not None:
                    raise AssertionError(
                        f"unknown release manifest mutation: {mutation}"
                    )

            output = case_dir / f"output-{mutation or 'valid'}"
            output.mkdir()
            output_environment = environment.copy()
            output_path = str(output)
            if os.name == "nt":
                conversion_environment = output_environment.copy()
                conversion_environment["TR300_OUTPUT_DIRECTORY"] = output_path
                output_probe = subprocess.run(
                    [
                        bash,
                        "--noprofile",
                        "--norc",
                        "-c",
                        'cygpath -u "$TR300_OUTPUT_DIRECTORY"',
                    ],
                    env=conversion_environment,
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    timeout=10,
                    check=False,
                )
                if output_probe.returncode != 0 or not output_probe.stdout.strip():
                    raise AssertionError(
                        "could not prepare release manifest fixture output path: "
                        f"{output_probe.stderr}"
                    )
                output_path = output_probe.stdout.strip()
            output_environment.update(
                {
                    "PLAN_MANIFEST": json.dumps(manifest, separators=(",", ":")),
                    "OUTPUT_DIRECTORY": output_path,
                    "RELEASE_TAG": "v99.99.99",
                    "SOURCE_SHA": "a" * 40,
                }
            )
            result = subprocess.run(
                [bash, "--noprofile", "--norc", "-s"],
                cwd=case_dir,
                env=output_environment,
                input=fixture_prefix + fragment,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=15,
                check=False,
            )
            succeeded = result.returncode == 0
            if succeeded != expected_success:
                raise AssertionError(
                    f"release manifest mutation={mutation} returned "
                    f"{result.returncode}, expected_success={expected_success}\n"
                    f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
                )
            metadata_path = output / "__tr300-release-metadata.json"
            if expected_success:
                persisted = json.loads((output / "dist-manifest.json").read_text())
                if persisted != manifest or not isinstance(persisted, dict):
                    raise AssertionError(
                        "release manifest validation did not preserve the complete object"
                    )
                metadata = json.loads(metadata_path.read_text())
                expected_metadata = {
                    "tag": "v99.99.99",
                    "source_sha": "a" * 40,
                    "title": manifest["announcement_title"],
                    "body": manifest["announcement_github_body"],
                    "prerelease": False,
                }
                if metadata != expected_metadata:
                    raise AssertionError(
                        f"release metadata projection changed: {metadata!r}"
                    )
            elif metadata_path.exists():
                raise AssertionError(
                    f"rejected release manifest produced metadata: mutation={mutation}"
                )


def write_release_host_gh_fixture(bin_dir: Path) -> None:
    mock = bin_dir / "gh"
    mock.write_text(
        r'''#!/usr/bin/env python3
import json
import os
import re
import sys
from pathlib import Path

arguments = sys.argv[1:]
config_path = Path(os.environ["MOCK_RELEASE_HOST_CONFIG"])
log_path = Path(os.environ["MOCK_RELEASE_HOST_LOG"])
config = json.loads(config_path.read_text(encoding="utf-8"))

with log_path.open("a", encoding="utf-8", newline="\n") as log:
    log.write(json.dumps(arguments, separators=(",", ":")) + "\n")

if arguments[:2] == ["release", "create"]:
    created = config.get("created", [])
    if not created:
        raise SystemExit(45)
    pages = config.setdefault("pages", [[]])
    if not pages:
        pages.append([])
    pages[0].extend(created)
    config_path.write_text(
        json.dumps(config, separators=(",", ":")), encoding="utf-8", newline="\n"
    )
    print(created[0]["html_url"])
    raise SystemExit(0)

if not arguments or arguments[0] != "api":
    raise SystemExit(97)
endpoint = next((value for value in arguments[1:] if value.startswith("repos/")), None)
if endpoint is None:
    raise SystemExit(98)
if endpoint.endswith("/releases?per_page=100"):
    print(json.dumps(config.get("pages", []), separators=(",", ":")))
    raise SystemExit(0)
if "/releases/tags/" in endpoint:
    # GitHub's REST tag endpoint does not expose private draft releases.
    raise SystemExit(44)
match = re.search(r"/releases/([1-9][0-9]*)$", endpoint)
if match:
    release_id = int(match.group(1))
    matches = [
        release
        for page in config.get("pages", [])
        for release in page
        if release.get("id") == release_id
    ]
    if len(matches) != 1:
        raise SystemExit(45)
    print(json.dumps(matches[0], separators=(",", ":")))
    raise SystemExit(0)
raise SystemExit(99)
''',
        encoding="utf-8",
        newline="\n",
    )
    mock.chmod(mock.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def run_release_host_draft_fixtures(release: str, bash: str) -> None:
    host = extract_job(release, "host", RELEASE_WORKFLOW.name)
    publisher = extract_named_run(
        host, "Rebind protected source and publish the fixed 24 assets", RELEASE_WORKFLOW.name
    )
    start = publisher.find("public_assets=(")
    if start < 0:
        raise AssertionError("release host draft fragment was not extractable")
    fragment = publisher[start:]
    array_end = fragment.find("\n)", fragment.find("public_assets=("))
    if array_end < 0:
        raise AssertionError("release host public asset array was not extractable")
    array_body = fragment[fragment.find("public_assets=(") + len("public_assets=(") : array_end]
    public_assets = array_body.split()
    if len(public_assets) != 24 or len(set(public_assets)) != 24:
        raise AssertionError(f"release host expected 24 unique assets, found {public_assets!r}")

    tag = "v99.99.99"
    source_sha = "a" * 40
    title = "99.99.99 - fixture"
    created_url = "https://github.com/QubeTX/qube-machine-report/releases/tag/untagged-fixture"

    def release_record(release_id: int = 901) -> dict[str, object]:
        return {
            "id": release_id,
            "tag_name": tag,
            "target_commitish": source_sha,
            "draft": True,
            "prerelease": False,
            "name": title,
            "html_url": created_url,
            "assets": [{"name": name, "size": 1} for name in public_assets],
        }

    def clone(record: dict[str, object]) -> dict[str, object]:
        return json.loads(json.dumps(record))

    valid = release_record()
    duplicate = release_record(902)
    wrong_target = clone(valid)
    wrong_target["target_commitish"] = "b" * 40
    public = clone(valid)
    public["draft"] = False
    prerelease = clone(valid)
    prerelease["prerelease"] = True
    missing_asset = clone(valid)
    assert isinstance(missing_asset["assets"], list)
    missing_asset["assets"].pop()
    extra_asset = clone(valid)
    assert isinstance(extra_asset["assets"], list)
    extra_asset["assets"].append({"name": "unexpected.bin", "size": 1})
    zero_asset = clone(valid)
    assert isinstance(zero_asset["assets"], list)
    zero_asset["assets"][0]["size"] = 0

    cases = (
        ("valid private draft", [[], []], [valid], True, 1),
        ("preexisting draft on page two", [[], [valid]], [valid], False, 0),
        ("duplicate created drafts", [[], []], [valid, duplicate], False, 1),
        ("wrong target", [[], []], [wrong_target], False, 1),
        ("public release", [[], []], [public], False, 1),
        ("prerelease", [[], []], [prerelease], False, 1),
        ("missing asset", [[], []], [missing_asset], False, 1),
        ("extra asset", [[], []], [extra_asset], False, 1),
        ("zero-byte asset", [[], []], [zero_asset], False, 1),
    )

    for name, pages, created, expected_success, expected_creates in cases:
        with tempfile.TemporaryDirectory(prefix="tr300-release-host-") as case_raw:
            directory = Path(case_raw)
            bin_dir = directory / "bin"
            assets = directory / "assets"
            runner_temp = directory / "runner"
            bin_dir.mkdir()
            assets.mkdir()
            runner_temp.mkdir()
            write_release_host_gh_fixture(bin_dir)
            write_fixture_jq_compat(bin_dir)
            for asset in public_assets:
                (assets / asset).write_bytes(b"fixture\n")
            (assets / "__tr300-release-metadata.json").write_text(
                json.dumps(
                    {
                        "tag": tag,
                        "source_sha": source_sha,
                        "title": title,
                        "body": "fixture release notes",
                        "prerelease": False,
                    }
                ),
                encoding="utf-8",
                newline="\n",
            )
            (assets / "__tr300-notes.txt").write_text(
                "fixture release notes\n", encoding="utf-8", newline="\n"
            )
            managed_installer_sha256 = hashlib.sha256(b"fixture\n").hexdigest()
            (assets / "__tr300-asset-sha256s").write_text(
                f"{managed_installer_sha256}  tr300-installer.sh\n",
                encoding="utf-8",
                newline="\n",
            )
            config_path = directory / "config.json"
            log_path = directory / "gh-calls.jsonl"
            github_output = directory / "github-output"
            config_path.write_text(
                json.dumps({"pages": pages, "created": created}, separators=(",", ":")),
                encoding="utf-8",
                newline="\n",
            )
            log_path.write_text("", encoding="utf-8", newline="\n")
            environment = os.environ.copy()
            environment.update(
                {
                    "ASSET_DIRECTORY": assets.as_posix(),
                    "RUNNER_TEMP": runner_temp.as_posix(),
                    "EXPECTED_REPOSITORY": "QubeTX/qube-machine-report",
                    "RELEASE_TAG": tag,
                    "EXPECTED_SHA": source_sha,
                    "GH_TOKEN": "fixture-token",
                    "GITHUB_OUTPUT": github_output.as_posix(),
                    "MOCK_RELEASE_HOST_CONFIG": config_path.as_posix(),
                    "MOCK_RELEASE_HOST_LOG": log_path.as_posix(),
                    "TR300_RELEASE_HOST_BIN": bin_dir.as_posix(),
                }
            )
            environment["PATH"] = str(bin_dir) + os.pathsep + environment.get("PATH", "")
            result = subprocess.run(
                [bash, "--noprofile", "--norc", "-s"],
                cwd=directory,
                env=environment,
                input=(
                    ("set -euxo pipefail\n" if os.environ.get("TR300_TEST_XTRACE") == "1" else "set -euo pipefail\n")
                    + fragment
                ),
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=20,
                check=False,
            )
            succeeded = result.returncode == 0
            calls = [
                json.loads(line)
                for line in log_path.read_text(encoding="utf-8").splitlines()
                if line
            ]
            create_count = sum(call[:2] == ["release", "create"] for call in calls)
            if succeeded != expected_success or create_count != expected_creates:
                raise AssertionError(
                    f"release host draft case={name!r} returned {result.returncode}, "
                    f"expected_success={expected_success}, creates={create_count}, "
                    f"expected_creates={expected_creates}\nstdout:\n{result.stdout}\n"
                    f"stderr:\n{result.stderr}"
                )
            if expected_success:
                if github_output.read_text(encoding="utf-8") != (
                    f"managed_installer_sha256={managed_installer_sha256}\n"
                ):
                    raise AssertionError(
                        "release host fixture did not propagate the managed installer digest"
                    )
                by_tag = subprocess.run(
                    [
                        bash,
                        "--noprofile",
                        "--norc",
                        "-c",
                        'gh api "repos/$EXPECTED_REPOSITORY/releases/tags/$RELEASE_TAG"',
                    ],
                    cwd=directory,
                    env=environment,
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    timeout=10,
                    check=False,
                )
                if by_tag.returncode == 0:
                    raise AssertionError("release host fixture exposed a private draft by tag")


def run_macos_inventory_compatibility_fixtures(macos: str) -> None:
    custody = extract_job(macos, "source-custody", MACOS_WORKFLOW.name)
    prepare = extract_unique_named_run(
        macos,
        "Safely prepare a fixed data-only universal installer payload",
        MACOS_WORKFLOW.name,
    )
    build = extract_unique_named_run(
        macos,
        "Download and revalidate the exact prepared artifact before credential use",
        MACOS_WORKFLOW.name,
    )
    trusted_input_names = (
        "tr300-installer.sh",
        "tr300-dist-installer.sh",
        "tr300-aarch64-apple-darwin.tar.xz",
        "tr300-aarch64-apple-darwin.tar.xz.sha256",
        "tr300-x86_64-apple-darwin.tar.xz",
        "tr300-x86_64-apple-darwin.tar.xz.sha256",
    )
    prepared_names = ("tr300-universal", "preinstall", "PROVENANCE", "SHA256SUMS")
    trusted_input_array = "expected=(\n" + "\n".join(
        f"  {name}" for name in trusted_input_names
    ) + "\n)"
    prepared_array = "expected=(\n" + "\n".join(
        f"  {name}" for name in prepared_names
    ) + "\n)"
    require_exact_line_sequence(
        prepare, trusted_input_array, "prepare upstream expected-name binding"
    )
    require_exact_line_sequence(
        prepare,
        'python3 - upstream "${expected[@]}" <<\'PY\'',
        "prepare upstream invocation binding",
    )
    require_exact_line_sequence(
        prepare,
        'python3 - "$OUTPUT_DIRECTORY" \\\n'
        "  tr300-universal preinstall PROVENANCE SHA256SUMS <<'PY'",
        "prepare output invocation binding",
    )
    require_exact_line_sequence(
        build, prepared_array, "build prepared expected-name binding"
    )
    require_exact_line_sequence(
        build,
        'python3 - "$PREPARED_DIRECTORY" "${expected[@]}" <<\'PY\'',
        "build prepared invocation binding",
    )

    trusted_upload = extract_unique_named_step(
        custody,
        "Upload normalized trusted Apple managed inputs",
        f"{MACOS_WORKFLOW.name} source custody",
    )
    producer_prefix = "${{ runner.temp }}/trusted-apple-inputs/"
    producer_names = tuple(
        match.group(1)
        for line in trusted_upload.splitlines()
        if (
            match := re.fullmatch(
                rf"\s+{re.escape(producer_prefix)}([A-Za-z0-9._-]+)\s*", line
            )
        )
        is not None
    )
    if producer_names != trusted_input_names:
        raise AssertionError(
            f"{MACOS_WORKFLOW.name}: source-custody producer inventory must be "
            f"{trusted_input_names!r}, found {producer_names!r}"
        )

    consumer_match = re.search(
        r"(?m)^expected=\(\n((?:  [A-Za-z0-9._-]+\n)+)\)", prepare
    )
    if consumer_match is None:
        raise AssertionError(
            f"{MACOS_WORKFLOW.name}: missing prepare consumer inventory"
        )
    consumer_names = tuple(
        line.strip() for line in consumer_match.group(1).splitlines()
    )
    if consumer_names != trusted_input_names:
        raise AssertionError(
            f"{MACOS_WORKFLOW.name}: prepare consumer inventory must be "
            f"{trusted_input_names!r}, found {consumer_names!r}"
        )

    scans = (
        (
            "source-custody to prepare upstream inventory",
            extract_unique_python_heredoc(
                prepare, "upstream input is not a real directory", "prepare upstream"
            ),
            producer_names,
            consumer_names,
        ),
        (
            "prepare output inventory",
            extract_unique_python_heredoc(
                prepare, "prepared output is not a real directory", "prepare output"
            ),
            prepared_names,
            prepared_names,
        ),
        (
            "credentialed build input inventory",
            extract_unique_python_heredoc(
                build, "prepared input is not a real directory", "build input"
            ),
            prepared_names,
            prepared_names,
        ),
    )
    for label, program, fixture_names, expected_names in scans:
        for mutation, expected_success in (
            (None, True),
            ("missing", False),
            ("extra", False),
            ("empty", False),
            ("nonregular", False),
            ("symlink", False),
            ("root-symlink", False),
        ):
            with tempfile.TemporaryDirectory(prefix="tr300-macos-inventory-") as raw:
                directory = Path(raw) / "inventory"
                fixture_directory = directory
                if mutation == "root-symlink":
                    fixture_directory = Path(raw) / "real-inventory"
                    fixture_directory.mkdir()
                    try:
                        directory.symlink_to(fixture_directory, target_is_directory=True)
                    except OSError:
                        if os.name != "nt":
                            raise
                        directory.write_bytes(b"not a directory\n")
                else:
                    directory.mkdir()
                for name in fixture_names:
                    (fixture_directory / name).write_bytes(b"fixture\n")
                if mutation == "extra":
                    (fixture_directory / ".unexpected").write_bytes(b"reject\n")
                elif mutation == "missing":
                    (fixture_directory / fixture_names[0]).unlink()
                elif mutation == "empty":
                    (fixture_directory / fixture_names[0]).write_bytes(b"")
                elif mutation == "nonregular":
                    path = fixture_directory / fixture_names[0]
                    path.unlink()
                    path.mkdir()
                elif mutation == "symlink":
                    path = fixture_directory / fixture_names[0]
                    path.unlink()
                    try:
                        path.symlink_to(fixture_names[1])
                    except OSError:
                        if os.name != "nt":
                            raise
                        path.mkdir()
                result = subprocess.run(
                    [sys.executable, "-", str(directory), *expected_names],
                    input=program,
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    timeout=10,
                    check=False,
                )
                if (result.returncode == 0) != expected_success:
                    raise AssertionError(
                        f"{label} mutation={mutation} returned {result.returncode}\n"
                        f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
                    )


def write_mock_gh(bin_dir: Path) -> None:
    mock = bin_dir / "gh"
    mock.write_bytes(
        b"""#!/usr/bin/env bash
set -euo pipefail
if [[ ${1:-} == release && ${2:-} == view && $# -ge 3 ]]; then
    tag=$3
    if [[ $tag == "$MOCK_RELEASE_TAG" ]]; then
        printf '%s\\n' "$MOCK_CURRENT_ASSETS"
    elif [[ $tag == "$MOCK_PREVIOUS_TAG" ]]; then
        printf '%s\\n' "$MOCK_PREVIOUS_ASSETS"
    else
        exit 45
    fi
    exit 0
fi
if [[ ${1:-} == release && ${2:-} == upload && $# -ge 3 ]]; then
    if [[ -n ${MOCK_GH_LOG:-} ]]; then
        for argument in "$@"; do
            printf 'release-upload:%s\n' "$argument" >> "$MOCK_GH_LOG"
        done
    fi
    [[ ${MOCK_UPLOAD_FAIL:-false} != true ]] || exit 46
    exit 0
fi
[[ ${1:-} == api && $# -ge 2 ]] || exit 97
endpoint=$2
if [[ $endpoint == --paginate && ${3:-} == --slurp ]]; then
    endpoint=${4:-}
fi
if [[ -n ${MOCK_GH_LOG:-} ]]; then
    printf '%s\\n' "$endpoint" >> "$MOCK_GH_LOG"
fi
if [[ -n ${MOCK_FAIL_ENDPOINT:-} && $endpoint == "$MOCK_FAIL_ENDPOINT" ]]; then
    exit 88
fi
case "$endpoint" in
    repos/*/actions/workflows/ci.yml/runs\\?*)
        if [[ -n ${MOCK_CI_RUNS_AFTER:-} ]]; then
            ci_marker="$MOCK_CI_STATE_DIR/ci-runs-seen"
            if [[ ! -e $ci_marker ]]; then
                : > "$ci_marker"
                cat "$MOCK_CI_RUNS_JSON"
            else
                cat "$MOCK_CI_RUNS_AFTER"
            fi
        else
            cat "$MOCK_CI_RUNS_JSON"
        fi
        ;;
    repos/*/actions/workflows/ci.yml)
        printf '%s\\n' "${MOCK_CI_WORKFLOW_ID:-333333}"
        ;;
    repos/*/actions/workflows/macos-installer.yml)
        printf '%s\\n' "${MOCK_MACOS_WORKFLOW_ID:-229917048}"
        ;;
    repos/*/actions/workflows/windows-installers.yml)
        printf '%s\\n' "${MOCK_WINDOWS_WORKFLOW_ID:-229917049}"
        ;;
    repos/*/actions/workflows/windows-installer-validation.yml)
        printf '%s\\n' "${MOCK_WINDOWS_VALIDATION_WORKFLOW_ID:-229917050}"
        ;;
    repos/*/actions/workflows/release.yml/runs\\?*)
        [[ -z ${MOCK_RELEASE_RUN_IDS:-} ]] || printf '%s\\n' "$MOCK_RELEASE_RUN_IDS"
        ;;
    repos/*/actions/workflows/release.yml)
        printf '%s\\n' "${MOCK_TRUSTED_RELEASE_WORKFLOW_ID:-229917047}"
        ;;
    repos/*/actions/runs/*/artifacts\\?*)
        inventory_marker="$MOCK_GH_STATE_DIR/artifact-inventory-seen"
        if [[ ! -e $inventory_marker ]]; then
            : > "$inventory_marker"
            cat "$MOCK_ARTIFACTS_BEFORE"
        else
            cat "$MOCK_ARTIFACTS_AFTER"
        fi
        ;;
    repos/*/actions/runs/*)
        if [[ $* == *'@tsv'* ]]; then
            printf '%s\\t%s\\t%s\\t%s\\t%s\\t%s\\t%s\\t%s\\t%s\\t%s\\t%s\\t%s\\n' \
                "$MOCK_RELEASE_RUN_ID" "$MOCK_RELEASE_WORKFLOW_ID" \
                "$MOCK_RELEASE_RUN_NAME" "$MOCK_RELEASE_RUN_PATH" \
                "$MOCK_RELEASE_RUN_EVENT" "$MOCK_RELEASE_RUN_STATUS" \
                "$MOCK_RELEASE_RUN_CONCLUSION" "$MOCK_RELEASE_RUN_REPOSITORY" \
                "$MOCK_RELEASE_RUN_HEAD_REPOSITORY" "$MOCK_RELEASE_RUN_TAG" \
                "$MOCK_RELEASE_RUN_SHA" "$MOCK_RELEASE_RUN_ATTEMPT"
        elif [[ ! -e $MOCK_GH_STATE_DIR/run-json-seen ]]; then
            : > "$MOCK_GH_STATE_DIR/run-json-seen"
            cat "$MOCK_RUN_BEFORE"
        else
            cat "$MOCK_RUN_AFTER"
        fi
        ;;
    repos/*/actions/artifacts/*/zip)
        case "$endpoint" in
            */101/zip) cat "$MOCK_ARM_ARTIFACT_ZIP" ;;
            */102/zip) cat "$MOCK_X86_ARTIFACT_ZIP" ;;
            */103/zip) cat "$MOCK_PREPARED_RELEASE_ARTIFACT_ZIP" ;;
            *)
                [[ -n ${MOCK_PUBLISHER_ARTIFACT_ZIP:-} ]] || exit 89
                cat "$MOCK_PUBLISHER_ARTIFACT_ZIP"
                ;;
        esac
        ;;
    repos/*/actions/artifacts/*)
        if [[ -n ${MOCK_SOURCE_ARTIFACT_JSON:-} &&
              $endpoint == */${MOCK_SOURCE_ARTIFACT_ID:-0} ]]; then
            cat "$MOCK_SOURCE_ARTIFACT_JSON"
        else
            cat "$MOCK_DIRECT_ARTIFACT_JSON"
        fi
        ;;
    repos/*/git/ref/heads/main)
        printf '%s\\n' "$MOCK_DEFAULT_SHA"
        ;;
    repos/*/git/ref/tags/*)
        printf '%s\\t%s\\n' "$MOCK_REF_TYPE" "$MOCK_REF_SHA"
        ;;
    repos/*/git/tags/*)
        printf '%s\\t%s\\n' "$MOCK_TAG_TYPE" "$MOCK_TAG_SHA"
        ;;
    repos/*/releases/[1-9]*)
        [[ ${MOCK_RELEASE_PRESENT:-true} == true ]] || exit 44
        requested_id=${endpoint##*/}
        [[ "$requested_id" == "${MOCK_RELEASE_ID:-777777}" ]] || exit 44
        printf '{"id":%s,"tag_name":"%s","target_commitish":"%s","draft":%s,"prerelease":%s,"assets":[' \
            "${MOCK_RELEASE_ID:-777777}" "$MOCK_RELEASE_TAG" "$MOCK_RELEASE_TARGET" \
            "${MOCK_RELEASE_DRAFT:-true}" "${MOCK_RELEASE_PRERELEASE:-false}"
        asset_count=${MOCK_RELEASE_ASSET_COUNT:-30}
        asset_index=0
        while IFS= read -r asset_name; do
            [[ -n "$asset_name" ]] || continue
            asset_index=$((asset_index + 1))
            [[ $asset_index -eq 1 ]] || printf ','
            printf '{"id":%s,"name":"%s","size":1,"digest":"sha256:%064d"}' \
                "$((900000 + asset_index))" "$asset_name" 0
        done <<< "${MOCK_CURRENT_ASSETS:-}"
        while ((asset_index < asset_count)); do
            asset_index=$((asset_index + 1))
            [[ $asset_index -eq 1 ]] || printf ','
            printf '{"id":%s,"name":"fixture-%s","size":1,"digest":"sha256:%064d"}' \
                "$((900000 + asset_index))" "$asset_index" 0
        done
        printf ']}\\n'
        ;;
    repos/*/releases/tags/*)
        [[ ${MOCK_RELEASE_PRESENT:-true} == true ]] || exit 44
        if [[ $* == *'(.assets | length)'* ]]; then
            printf '%s\\t%s\\t%s\\n' "${MOCK_RELEASE_DRAFT:-true}" \
                "${MOCK_RELEASE_PRERELEASE:-false}" "${MOCK_RELEASE_ASSET_COUNT:-34}"
        elif [[ $* == *'@tsv'* ]]; then
            printf '%s\\t%s\\t%s\\n' "$MOCK_RELEASE_TARGET" \
                "${MOCK_RELEASE_DRAFT:-true}" "${MOCK_RELEASE_PRERELEASE:-false}"
        else
            printf '%s\\n' "$MOCK_RELEASE_TARGET"
        fi
        ;;
    repos/*/releases/latest)
        printf '%s\\n' "${MOCK_LATEST_TAG:-v4.2.2}"
        ;;
    repos/*/releases\\?per_page=100)
        if [[ ${2:-} == --paginate && ${3:-} == --slurp ]]; then
            printf '[['
            record_index=0
            if [[ ${MOCK_RELEASE_PRESENT:-true} == true ]]; then
                while IFS= read -r release_tag; do
                    [[ -n "$release_tag" ]] || continue
                    [[ $record_index -eq 0 ]] || printf ','
                    printf '{"id":%s,"tag_name":"%s","target_commitish":"%s","draft":%s,"prerelease":%s,"assets":[' \
                        "$(( ${MOCK_RELEASE_ID:-777777} + record_index ))" "$release_tag" \
                        "$MOCK_RELEASE_TARGET" "${MOCK_RELEASE_DRAFT:-true}" \
                        "${MOCK_RELEASE_PRERELEASE:-false}"
                    asset_count=${MOCK_RELEASE_ASSET_COUNT:-30}
                    for ((asset_index=1; asset_index<=asset_count; asset_index++)); do
                        [[ $asset_index -eq 1 ]] || printf ','
                        printf '{"id":%s,"name":"fixture-%s","size":1,"digest":"sha256:%064d"}' \
                            "$((900000 + asset_index))" "$asset_index" 0
                    done
                    printf ']}'
                    record_index=$((record_index + 1))
                done <<< "${MOCK_RELEASE_TAGS_FOR_SHA-$MOCK_RELEASE_TAG}"
            fi
            printf ']]\\n'
        elif [[ $* == *target_commitish* ]]; then
            [[ -z $MOCK_RELEASE_TAGS_FOR_SHA ]] ||
                printf '%s\\n' "$MOCK_RELEASE_TAGS_FOR_SHA"
        else
            [[ -z $MOCK_PREVIOUS_TAGS ]] ||
                printf '%s\\n' "$MOCK_PREVIOUS_TAGS"
        fi
        ;;
    repos/*/commits/*)
        printf '%s\\n' "$MOCK_DEFAULT_SHA"
        ;;
    repos/*)
        if [[ $* == *'.id'* ]]; then
            printf '%s\\n' "$MOCK_REPOSITORY_ID"
        else
            printf '%s\\n' "$MOCK_DEFAULT_BRANCH"
        fi
        ;;
    *)
        printf 'unexpected gh api endpoint: %s\\n' "$endpoint" >&2
        exit 98
        ;;
esac
"""
    )
    mock.chmod(mock.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def write_windows_mkdir_compat(bin_dir: Path) -> None:
    """Keep Git Bash's NTFS mkdir from rejecting otherwise valid -m fixtures."""

    if os.name != "nt":
        return
    mock = bin_dir / "mkdir"
    mock.write_text(
        """#!/usr/bin/env python3
import os
import sys

parents = False
paths = []
arguments = iter(sys.argv[1:])
for argument in arguments:
    if argument == "-p" or argument == "--parents":
        parents = True
    elif argument == "-m" or argument == "--mode":
        next(arguments, None)
    elif argument.startswith("-m") or argument.startswith("--mode="):
        pass
    elif argument.startswith("-"):
        raise SystemExit(f"fixture mkdir rejects option: {argument}")
    else:
        paths.append(argument)
for path in paths:
    if parents:
        os.makedirs(path, exist_ok=True)
    else:
        os.mkdir(path)
""",
        encoding="utf-8",
        newline="\n",
    )
    mock.chmod(mock.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def write_windows_shasum_compat(bin_dir: Path) -> None:
    """Provide the macOS shasum invocation used by native custody fixtures."""

    if os.name != "nt":
        return
    mock = bin_dir / "shasum"
    mock.write_text(
        """#!/usr/bin/env python3
import hashlib
import pathlib
import sys

arguments = sys.argv[1:]
if len(arguments) == 4 and arguments[:3] == ["-a", "256", "-c"]:
    sidecar = pathlib.Path(arguments[3])
    lines = sidecar.read_text(encoding="utf-8").splitlines()
    if len(lines) != 1:
        raise SystemExit("fixture shasum requires one checksum record")
    fields = lines[0].split(maxsplit=1)
    if len(fields) != 2 or len(fields[0]) != 64:
        raise SystemExit("fixture shasum rejects malformed checksum record")
    name = fields[1].removeprefix("*")
    path = pathlib.Path(name)
    if path.name != name or not path.is_file():
        raise SystemExit("fixture shasum rejects unsafe checksum target")
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != fields[0]:
        raise SystemExit(f"{name}: FAILED")
    print(f"{name}: OK")
    raise SystemExit(0)
if len(arguments) != 3 or arguments[:2] != ["-a", "256"]:
    raise SystemExit(f"fixture shasum rejects arguments: {arguments!r}")
path = pathlib.Path(arguments[2])
print(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path}")
""",
        encoding="utf-8",
        newline="\n",
    )
    mock.chmod(mock.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def write_fixture_jq_compat(bin_dir: Path) -> None:
    """Provide the exact JSON predicates used by executable fixtures on Windows.

    Hosted Ubuntu CI still executes the workflow filters with real jq. Git Bash on
    Windows has no declared jq dependency, so this fail-closed adapter implements
    only the fixed custody predicates below and rejects every unknown filter.
    """

    if os.name != "nt":
        return
    mock = bin_dir / "jq"
    mock.write_text(
        r'''#!/usr/bin/env python3
import json
import re
import sys

sys.stdout.reconfigure(newline="\n")
arguments = sys.argv[1:]
variables = {}
raw = False
raw_input = False
slurp = False
index = 0
while index < len(arguments):
    argument = arguments[index]
    if re.fullmatch(r"-[erc]+", argument):
        raw = raw or "r" in argument
        index += 1
        continue
    if argument in ("-e", "-r", "-c"):
        raw = raw or argument == "-r"
        index += 1
        continue
    if argument in ("-R", "--raw-input"):
        raw_input = True
        index += 1
        continue
    if argument in ("-s", "--slurp"):
        slurp = True
        index += 1
        continue
    if argument in ("--arg", "--argjson"):
        if index + 2 >= len(arguments):
            raise SystemExit(2)
        name, value = arguments[index + 1], arguments[index + 2]
        variables[name] = json.loads(value) if argument == "--argjson" else value
        index += 3
        continue
    break
if index >= len(arguments) or index + 2 < len(arguments):
    print(f"fixture jq rejects arguments: {arguments!r}", file=sys.stderr)
    raise SystemExit(3)
query = arguments[index]
if raw_input:
    data = sys.stdin.read()
elif slurp:
    contents = sys.stdin.read()
    decoder = json.JSONDecoder()
    data = []
    cursor = 0
    while cursor < len(contents):
        while cursor < len(contents) and contents[cursor].isspace():
            cursor += 1
        if cursor >= len(contents):
            break
        value, cursor = decoder.raw_decode(contents, cursor)
        data.append(value)
elif index + 1 == len(arguments):
    data = json.load(sys.stdin)
else:
    path = arguments[index + 1]
    with open(path, "r", encoding="utf-8") as source:
        data = json.load(source)
normalized = " ".join(query.split())

def nested(record, *parts):
    value = record
    for part in parts:
        if not isinstance(value, dict) or part not in value:
            return None
        value = value[part]
    return value

def emit(value):
    if isinstance(value, bool):
        print("true" if value else "false")
    elif isinstance(value, (dict, list)):
        print(json.dumps(value, separators=(",", ":")))
    elif isinstance(value, str) and not raw:
        print(json.dumps(value, separators=(",", ":")))
    elif value is not None:
        print(value)

def require(condition):
    raise SystemExit(0 if condition else 1)

if raw_input and normalized == ".":
    for line in data.splitlines():
        emit(line)
elif slurp and normalized == "sort":
    emit(sorted(data))
elif (
    "release manifest metadata is malformed" in normalized and
    normalized.startswith("if .announcement_is_prerelease == false and") and
    "then . else error(" in normalized and normalized.endswith("end")
):
    valid = (
        data.get("announcement_is_prerelease") is False and
        isinstance(data.get("announcement_title"), str) and
        len(data["announcement_title"]) > 0 and
        isinstance(data.get("announcement_github_body"), str) and
        len(data["announcement_github_body"]) > 0
    ) if isinstance(data, dict) else False
    if not valid:
        raise SystemExit(5)
    emit(data)
elif (
    normalized ==
    "{ tag: $tag, source_sha: $source_sha, title: .announcement_title, "
    "body: .announcement_github_body, prerelease: .announcement_is_prerelease }"
):
    emit({
        "tag": variables.get("tag"),
        "source_sha": variables.get("source_sha"),
        "title": data.get("announcement_title"),
        "body": data.get("announcement_github_body"),
        "prerelease": data.get("announcement_is_prerelease"),
    })
elif normalized == (
    ".tag == $tag and .source_sha == $sha and .prerelease == false and "
    "(.title | type == \"string\" and length > 0)"
):
    require(
        data.get("tag") == variables.get("tag") and
        data.get("source_sha") == variables.get("sha") and
        data.get("prerelease") is False and
        isinstance(data.get("title"), str) and len(data["title"]) > 0
    )
elif normalized == ".title":
    emit(data.get("title"))
elif normalized == ".total_count":
    emit(data.get("total_count"))
elif normalized == ".artifacts | length":
    emit(len(data.get("artifacts", [])))
elif normalized == '.artifacts[].name | select(test("apple-darwin"))':
    for artifact in data.get("artifacts", []):
        name = artifact.get("name")
        if isinstance(name, str) and "apple-darwin" in name:
            emit(name)
elif "expected exactly one artifact named" in normalized:
    records = [
        artifact for artifact in data.get("artifacts", [])
        if artifact.get("name") == variables.get("name")
    ]
    if len(records) != 1:
        raise SystemExit(5)
    artifact = records[0]
    emit("\t".join(str(value).lower() if isinstance(value, bool) else str(value) for value in (
        artifact.get("id"), artifact.get("digest"), artifact.get("expired"),
        artifact.get("size_in_bytes"), nested(artifact, "workflow_run", "id"),
        nested(artifact, "workflow_run", "repository_id"),
        nested(artifact, "workflow_run", "head_repository_id"),
        nested(artifact, "workflow_run", "head_branch"),
        nested(artifact, "workflow_run", "head_sha"),
    )))
elif ".id == $run_id" in normalized and ".workflow_id == $workflow_id" in normalized:
    require(
        data.get("id") == variables.get("run_id") and
        data.get("workflow_id") == variables.get("workflow_id") and
        data.get("name") == "Release" and
        data.get("path") == ".github/workflows/release.yml" and
        data.get("event") == "push" and data.get("status") == "completed" and
        data.get("conclusion") == "success" and
        nested(data, "repository", "full_name") == variables.get("repository") and
        nested(data, "head_repository", "full_name") == variables.get("repository") and
        data.get("head_branch") == variables.get("tag") and
        data.get("head_sha") == variables.get("sha") and
        data.get("run_attempt") == variables.get("run_attempt")
    )
elif normalized == (
    "[.id, .name, .digest, .expired, .size_in_bytes, .workflow_run.id, "
    ".workflow_run.repository_id, .workflow_run.head_repository_id, "
    ".workflow_run.head_sha] | @tsv"
):
    emit("\t".join(
        str(value).lower() if isinstance(value, bool) else str(value)
        for value in (
            data.get("id"), data.get("name"), data.get("digest"),
            data.get("expired"), data.get("size_in_bytes"),
            nested(data, "workflow_run", "id"),
            nested(data, "workflow_run", "repository_id"),
            nested(data, "workflow_run", "head_repository_id"),
            nested(data, "workflow_run", "head_sha"),
        )
    ))
elif ".id == $id" in normalized and ".workflow_run.id == $run" in normalized:
    name_match = re.search(r'\.name == "([^"]+)"', normalized)
    expected_name = name_match.group(1) if name_match else None
    size = data.get("size_in_bytes")
    condition = (
        data.get("id") == variables.get("id") and data.get("name") == expected_name and
        data.get("expired") is False and isinstance(size, int) and 0 < size <= 268435456 and
        data.get("digest") == variables.get("digest") and
        nested(data, "workflow_run", "id") == variables.get("run") and
        nested(data, "workflow_run", "repository_id") == variables.get("repository") and
        nested(data, "workflow_run", "head_repository_id") == variables.get("repository")
    )
    if "$sha" in normalized:
        condition = condition and nested(data, "workflow_run", "head_sha") == variables.get("sha")
    require(condition)
elif (
    ".workflow_runs[]" in normalized and
    '(.conclusion // "__pending__")' in normalized and
    "| @tsv" in normalized
):
    for run in data.get("workflow_runs", []):
        if (
            run.get("workflow_id") == variables.get("workflow_id") and
            run.get("name") == "CI" and
            run.get("path") == ".github/workflows/ci.yml" and
            run.get("event") == "push" and
            nested(run, "repository", "full_name") == variables.get("repo") and
            nested(run, "head_repository", "full_name") == variables.get("repo") and
            run.get("head_branch") == "main" and
            run.get("head_sha") == variables.get("sha")
        ):
            emit("\t".join(str(value) for value in (
                run.get("id"), run.get("status"),
                run.get("conclusion") or "__pending__", run.get("run_attempt"),
            )))
elif ".workflow_runs[]" in normalized and "| .id" in normalized:
    expected_name = "Windows Installers" if 'name == "Windows Installers"' in normalized else "CI"
    expected_path = (
        ".github/workflows/windows-installers.yml"
        if expected_name == "Windows Installers" else ".github/workflows/ci.yml"
    )
    expected_event = "workflow_run" if expected_name == "Windows Installers" else "push"
    for run in data.get("workflow_runs", []):
        if (
            run.get("workflow_id") == variables.get("workflow_id") and
            run.get("name") == expected_name and run.get("path") == expected_path and
            run.get("event") == expected_event and run.get("status") == "completed" and
            run.get("conclusion") == "success" and
            nested(run, "repository", "full_name") == variables.get("repository") and
            nested(run, "head_repository", "full_name") == variables.get("repository") and
            run.get("head_branch") == "main" and run.get("head_sha") == variables.get("sha")
        ):
            emit(run.get("id"))
elif ".dist_version == \"0.31.0\"" in normalized:
    archive = variables.get("archive")
    sidecar = variables.get("sidecar")
    upload_files = data.get("upload_files", [])
    basenames = sorted(str(value).replace("\\", "/").split("/")[-1] for value in upload_files)
    require(
        data.get("dist_version") == "0.31.0" and
        data.get("announcement_tag") == variables.get("tag") and len(upload_files) == 2 and
        basenames == sorted([archive, sidecar]) and
        nested(data, "artifacts", archive, "checksums", "sha256") == variables.get("hash")
    )
elif normalized == '.artifacts[] | select(.name == "windows-release-assets" and .expired == false) | [.id, .digest] | @tsv':
    for artifact in data.get("artifacts", []):
        if artifact.get("name") == "windows-release-assets" and artifact.get("expired") is False:
            emit(f"{artifact.get('id')}\t{artifact.get('digest')}")
elif normalized == ".assets[].name":
    for asset in data.get("assets", []):
        emit(asset.get("name"))
elif normalized == ".id":
    emit(data.get("id"))
elif normalized == "[.[][] | select(.tag_name == $tag)] | length == 0":
    matches = [
        release
        for page in data
        for release in page
        if release.get("tag_name") == variables.get("tag")
    ]
    require(len(matches) == 0)
elif "created release is not the unique exact private draft" in normalized:
    matches = [
        release
        for page in data
        for release in page
        if release.get("tag_name") == variables.get("tag")
    ]
    expected_assets = variables.get("expected_assets")
    valid = len(matches) == 1
    if valid:
        release = matches[0]
        assets = release.get("assets", [])
        valid = (
            isinstance(release.get("id"), int) and release["id"] > 0 and
            release.get("target_commitish") == variables.get("sha") and
            release.get("draft") is True and release.get("prerelease") is False and
            release.get("name") == variables.get("title") and
            release.get("html_url") == variables.get("url") and
            sorted(asset.get("name") for asset in assets) == expected_assets and
            all(asset.get("size", 0) > 0 for asset in assets)
        )
    if not valid:
        raise SystemExit(5)
    emit(release["id"])
elif ".id == $id and .tag_name == $tag" in normalized and ".html_url == $url" in normalized:
    assets = data.get("assets", [])
    require(
        data.get("id") == variables.get("id") and
        data.get("tag_name") == variables.get("tag") and
        data.get("target_commitish") == variables.get("sha") and
        data.get("draft") is True and data.get("prerelease") is False and
        data.get("name") == variables.get("title") and
        data.get("html_url") == variables.get("url") and
        sorted(asset.get("name") for asset in assets) == variables.get("expected_assets") and
        all(asset.get("size", 0) > 0 for asset in assets)
    )
elif "expected exactly one authenticated release" in normalized:
    matches = [
        release
        for page in data
        for release in page
        if release.get("tag_name") == variables.get("tag")
    ]
    if len(matches) != 1:
        raise SystemExit(5)
    release = matches[0]
    if not (
        isinstance(release.get("id"), int) and release["id"] > 0 and
        release.get("target_commitish") == variables.get("sha") and
        release.get("draft") is True and release.get("prerelease") is False
    ):
        raise SystemExit(5)
    emit(release["id"])
elif ".[][] | select(.tag_name == $tag)" in normalized and "| @tsv" in normalized:
    for page in data:
        for release in page:
            if release.get("tag_name") != variables.get("tag"):
                continue
            fields = [release.get("id")]
            if "[.id, .tag_name," in normalized:
                fields.append(release.get("tag_name"))
            fields.extend((
                release.get("target_commitish"), release.get("draft"),
                release.get("prerelease"),
            ))
            emit("\t".join(
                str(value).lower() if isinstance(value, bool) else str(value)
                for value in fields
            ))
elif (
    "select(.target_commitish == $sha" in normalized and
    "[.id, .tag_name, .target_commitish, .draft, .prerelease] | @tsv" in normalized
):
    for page in data:
        for release in page:
            if not (
                release.get("target_commitish") == variables.get("sha") and
                release.get("draft") is True and release.get("prerelease") is False
            ):
                continue
            emit("\t".join(
                str(value).lower() if isinstance(value, bool) else str(value)
                for value in (
                    release.get("id"), release.get("tag_name"),
                    release.get("target_commitish"), release.get("draft"),
                    release.get("prerelease"),
                )
            ))
elif ".id == $id and .tag_name == $tag" in normalized:
    condition = (
        data.get("id") == variables.get("id") and
        data.get("tag_name") == variables.get("tag") and
        data.get("target_commitish") == variables.get("sha") and
        data.get("draft") is True and data.get("prerelease") is False
    )
    if "(.assets | length) == 30" in normalized:
        condition = condition and len(data.get("assets", [])) == 30
    require(condition)
elif ".target_commitish == $sha" in normalized:
    assets = data.get("assets", [])
    condition = (
        data.get("target_commitish") == variables.get("sha") and
        data.get("draft") is (".draft == true" in normalized) and
        data.get("prerelease") is False
    )
    if "([.assets[].name] | sort) == $names" in normalized:
        condition = condition and sorted(asset.get("name") for asset in assets) == variables.get("names")
    if "([.assets[].size > 0] | all)" in normalized:
        condition = condition and all(asset.get("size", 0) > 0 for asset in assets)
    require(condition)
else:
    print(f"fixture jq rejects unknown filter: {query!r}", file=sys.stderr)
    raise SystemExit(3)
''',
        encoding="utf-8",
        newline="\n",
    )
    mock.chmod(mock.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def fixture_environment(mock_bin: Path, output: Path, log: Path) -> dict[str, str]:
    environment = os.environ.copy()
    environment.update(
        {
            "PATH": str(mock_bin) + os.pathsep + environment.get("PATH", ""),
            "GH_TOKEN": "fixture-token",
            "GITHUB_OUTPUT": str(output),
            "RUNNER_TEMP": output.parent.as_posix(),
            "MOCK_GH_STATE_DIR": output.parent.as_posix(),
            "EVENT_NAME": "workflow_run",
            "DISPATCH_MODE": "private",
            "DISPATCH_TAG": "",
            "DISPATCH_RUN_ID": RELEASE_RUN_ID,
            "DISPATCH_UPSTREAM_RUN_ID": RELEASE_RUN_ID,
            "PREFLIGHT_ONLY": "false",
            "WORKFLOW_REF": "refs/heads/main",
            "WORKFLOW_SHA": TRUSTED_SHA,
            "ACTOR_ID": "30877743",
            "EXPECTED_REPOSITORY": REPOSITORY,
            "UPSTREAM_CONCLUSION": "success",
            "UPSTREAM_EVENT": "push",
            "UPSTREAM_WORKFLOW_NAME": "Release",
            "UPSTREAM_REPOSITORY": REPOSITORY,
            "UPSTREAM_SHA": TRUSTED_SHA,
            "UPSTREAM_TAG": "v4.3.0",
            "UPSTREAM_RUN_ID": RELEASE_RUN_ID,
            "UPSTREAM_RUN_ATTEMPT": RELEASE_RUN_ATTEMPT,
            "MOCK_REF_TYPE": "commit",
            "MOCK_REF_SHA": TRUSTED_SHA,
            "MOCK_TAG_TYPE": "commit",
            "MOCK_TAG_SHA": TRUSTED_SHA,
            "MOCK_RELEASE_PRESENT": "true",
            "MOCK_RELEASE_ID": "777777",
            "MOCK_RELEASE_TARGET": TRUSTED_SHA,
            "MOCK_RELEASE_DRAFT": "true",
            "MOCK_RELEASE_PRERELEASE": "false",
            "MOCK_DEFAULT_BRANCH": "main",
            "MOCK_DEFAULT_SHA": TRUSTED_SHA,
            "MOCK_REPOSITORY_ID": REPOSITORY_ID,
            "MOCK_RELEASE_WORKFLOW_ID": RELEASE_WORKFLOW_ID,
            "MOCK_RELEASE_RUN_ID": RELEASE_RUN_ID,
            "MOCK_RELEASE_RUN_ATTEMPT": RELEASE_RUN_ATTEMPT,
            "MOCK_RELEASE_RUN_NAME": "Release",
            "MOCK_RELEASE_RUN_PATH": ".github/workflows/release.yml",
            "MOCK_RELEASE_RUN_EVENT": "push",
            "MOCK_RELEASE_RUN_STATUS": "completed",
            "MOCK_RELEASE_RUN_CONCLUSION": "success",
            "MOCK_RELEASE_RUN_REPOSITORY": REPOSITORY,
            "MOCK_RELEASE_RUN_HEAD_REPOSITORY": REPOSITORY,
            "MOCK_RELEASE_RUN_TAG": "v4.3.0",
            "MOCK_RELEASE_RUN_SHA": TRUSTED_SHA,
            "MOCK_RELEASE_RUN_IDS": RELEASE_RUN_ID,
            "MOCK_RELEASE_TAG": "v4.3.0",
            "MOCK_PREVIOUS_TAG": "v4.2.2",
            "MOCK_RELEASE_TAGS_FOR_SHA": "v4.3.0",
            "MOCK_PREVIOUS_TAGS": "v4.2.2",
            "MOCK_CURRENT_ASSETS": "\n".join(
                (
                    "dist-manifest.json",
                    "tr300-x86_64-pc-windows-msvc.msi",
                    "tr300-x86_64-pc-windows-msvc-corporate.msi",
                    "tr300-x86_64-pc-windows-msvc-setup.exe",
                    "tr300-x86_64-pc-windows-msvc-corporate-setup.exe",
                    "tr300-installer.ps1",
                    "tr300-dist-installer.ps1",
                    "tr300-x86_64-pc-windows-msvc.zip",
                )
            ),
            "MOCK_PREVIOUS_ASSETS": "\n".join(
                (
                    "tr300-x86_64-pc-windows-msvc.msi",
                    "tr300-x86_64-pc-windows-msvc-corporate.msi",
                    "tr300-x86_64-pc-windows-msvc-setup.exe",
                    "tr300-x86_64-pc-windows-msvc-corporate-setup.exe",
                    "tr300-installer.ps1",
                    "tr300-x86_64-pc-windows-msvc.zip",
                )
            ),
            "MOCK_FAIL_ENDPOINT": "",
            "MOCK_UPLOAD_FAIL": "false",
            "MOCK_GH_LOG": str(log),
        }
    )
    return environment


def parse_outputs(path: Path) -> dict[str, str]:
    outputs: dict[str, str] = {}
    if not path.exists():
        return outputs
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" not in line:
            raise AssertionError(f"malformed GITHUB_OUTPUT fixture line: {line!r}")
        key, value = line.split("=", 1)
        outputs[key] = value
    return outputs


def run_case(
    *,
    bash: str,
    mock_bin: Path,
    block: str,
    name: str,
    overrides: dict[str, str] | None = None,
    expected_outputs: dict[str, str] | None = None,
    expected_success: bool,
) -> list[str]:
    with tempfile.TemporaryDirectory(prefix="tr300-provenance-case-") as case_dir_raw:
        case_dir = Path(case_dir_raw)
        output = case_dir / "github-output"
        log = case_dir / "gh-calls"
        environment = fixture_environment(mock_bin, output, log)
        if overrides:
            environment.update(overrides)
        script = case_dir / "workflow-run.sh"
        script.write_text(block, encoding="utf-8", newline="\n")
        bash_arguments = [bash]
        if os.environ.get("TR300_TEST_XTRACE") == "1":
            bash_arguments.append("-x")
        bash_arguments.append(str(script))
        result = subprocess.run(
            bash_arguments,
            cwd=ROOT,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=15,
            check=False,
        )
        succeeded = result.returncode == 0
        if succeeded != expected_success:
            raise AssertionError(
                f"{name}: return code {result.returncode}, expected_success="
                f"{expected_success}\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
            )
        outputs = parse_outputs(output)
        if expected_outputs is not None and outputs != expected_outputs:
            raise AssertionError(
                f"{name}: outputs {outputs!r} != {expected_outputs!r}\n"
                f"stderr:\n{result.stderr}"
            )
        return log.read_text(encoding="utf-8").splitlines() if log.exists() else []


def cargo_dist_apple_archive_bytes(target: str) -> bytes:
    return f"fixture signed cargo-dist archive for {target}\n".encode()


def write_cargo_dist_apple_artifact(
    path: Path,
    target: str,
    *,
    extra_member: bool = False,
    bad_checksum: bool = False,
) -> str:
    archive_name = f"tr300-{target}.tar.xz"
    sidecar_name = f"{archive_name}.sha256"
    manifest_name = f"{target}-dist-manifest.json"
    archive_bytes = cargo_dist_apple_archive_bytes(target)
    archive_hash = hashlib.sha256(archive_bytes).hexdigest()
    sidecar_hash = "0" * 64 if bad_checksum else archive_hash
    sidecar_bytes = f"{sidecar_hash} *{archive_name}\n".encode()
    manifest = {
        "dist_version": "0.31.0",
        "announcement_tag": "v4.3.0",
        "upload_files": [
            f"/Users/runner/work/qube-machine-report/qube-machine-report/target/distrib/{archive_name}",
            f"/Users/runner/work/qube-machine-report/qube-machine-report/target/distrib/{sidecar_name}",
        ],
        "artifacts": {archive_name: {"checksums": {"sha256": archive_hash}}},
    }
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr(archive_name, archive_bytes)
        archive.writestr(sidecar_name, sidecar_bytes)
        archive.writestr(manifest_name, json.dumps(manifest, sort_keys=True).encode())
        if extra_member:
            archive.writestr("unexpected", b"unexpected")
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_prepared_release_artifact(
    path: Path, mutation: str | None
) -> tuple[str, dict[str, tuple[str, int]]]:
    payloads = {
        name: f"prepared Release fixture for {name}\n".encode()
        for name in INITIAL_RELEASE_ASSETS
    }
    for target in ("aarch64-apple-darwin", "x86_64-apple-darwin"):
        payloads[f"tr300-{target}.tar.xz"] = cargo_dist_apple_archive_bytes(target)
    dist_installer = b"#!/bin/sh\nprintf '%s\\n' 'fixture dist installer'\n"
    payloads["tr300-dist-installer.sh"] = dist_installer
    dist_installer_sha256 = hashlib.sha256(dist_installer).hexdigest()
    embedded_dist_sha256 = (
        "0" * 64
        if mutation == "prepared-dist-wrapper-pin-mismatch"
        else dist_installer_sha256
    )
    payloads["tr300-installer.sh"] = (
        "#!/bin/sh\n"
        "tr300_tag='v4.3.0'\n"
        f"tr300_dist_installer_sha256='{embedded_dist_sha256}'\n"
        "printf '%s\\n' \"$tr300_tag\"\n"
    ).encode()
    if mutation == "prepared-canonical-divergence":
        payloads["tr300-aarch64-apple-darwin.tar.xz"] += (
            b"internally valid but different prepared Release bytes\n"
        )
    manifest = b"".join(
        f"{hashlib.sha256(payloads[name]).hexdigest()}  {name}\n".encode()
        for name in INITIAL_RELEASE_ASSETS
    )
    if mutation == "prepared-wrapper-hash-mismatch":
        payloads["tr300-installer.sh"] += b"# changed after manifest\n"
    elif mutation == "prepared-dist-hash-mismatch":
        payloads["tr300-dist-installer.sh"] += b"# changed after manifest\n"
    elif mutation == "prepared-archive-hash-mismatch":
        payloads["tr300-aarch64-apple-darwin.tar.xz"] += b"changed after manifest\n"
    members = {
        **payloads,
        "__tr300-release-metadata.json": json.dumps(
            {"tag": "v4.3.0", "source_sha": TRUSTED_SHA}
        ).encode(),
        "__tr300-notes.txt": b"fixture release notes\n",
        "__tr300-asset-sha256s": manifest,
    }
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_STORED) as archive:
        for name in PREPARED_RELEASE_ARTIFACT_MEMBERS:
            archive.writestr(name, members[name])
        if mutation == "prepared-duplicate-member":
            archive.writestr("tr300-installer.sh", members["tr300-installer.sh"])
        elif mutation == "prepared-extra-member":
            archive.writestr("unexpected", b"unexpected\n")
    custody_assets = {
        name: (hashlib.sha256(payloads[name]).hexdigest(), len(payloads[name]))
        for name in (
            "tr300-installer.sh",
            "tr300-dist-installer.sh",
            "tr300-aarch64-apple-darwin.tar.xz",
            "tr300-x86_64-apple-darwin.tar.xz",
        )
    }
    return (
        hashlib.sha256(path.read_bytes()).hexdigest(),
        custody_assets,
    )


def release_run_json(**overrides: object) -> dict[str, object]:
    record: dict[str, object] = {
        "id": int(RELEASE_RUN_ID),
        "workflow_id": int(RELEASE_WORKFLOW_ID),
        "name": "Release",
        "path": ".github/workflows/release.yml",
        "event": "push",
        "status": "completed",
        "conclusion": "success",
        "repository": {"full_name": REPOSITORY},
        "head_repository": {"full_name": REPOSITORY},
        "head_branch": "v4.3.0",
        "head_sha": TRUSTED_SHA,
        "run_attempt": int(RELEASE_RUN_ATTEMPT),
    }
    record.update(overrides)
    return record


def release_artifact_record(
    *, name: str, artifact_id: int, digest: str, **overrides: object
) -> dict[str, object]:
    record: dict[str, object] = {
        "id": artifact_id,
        "name": name,
        "digest": f"sha256:{digest}",
        "expired": False,
        "size_in_bytes": 1024,
        "workflow_run": {
            "id": int(RELEASE_RUN_ID),
            "repository_id": int(REPOSITORY_ID),
            "head_repository_id": int(REPOSITORY_ID),
            "head_branch": "v4.3.0",
            "head_sha": TRUSTED_SHA,
        },
    }
    record.update(overrides)
    return record


def write_macos_release_run_fixture(
    directory: Path, mutation: str | None
) -> dict[str, str]:
    arm_zip = directory / "arm.zip"
    x86_zip = directory / "x86.zip"
    arm_digest = write_cargo_dist_apple_artifact(
        arm_zip,
        "aarch64-apple-darwin",
        extra_member=mutation == "extra-zip-member",
        bad_checksum=mutation == "bad-checksum",
    )
    x86_digest = write_cargo_dist_apple_artifact(
        x86_zip, "x86_64-apple-darwin"
    )
    prepared_zip = directory / "prepared-release.zip"
    prepared_digest, prepared_custody_assets = (
        write_prepared_release_artifact(prepared_zip, mutation)
    )

    run_before = release_run_json()
    run_after = release_run_json()
    artifacts: list[dict[str, object]] = [
        release_artifact_record(
            name=MACOS_RELEASE_RUN_ARTIFACTS[0],
            artifact_id=101,
            digest=("0" * 64 if mutation == "digest-mismatch" else arm_digest),
        ),
        release_artifact_record(
            name=MACOS_RELEASE_RUN_ARTIFACTS[1], artifact_id=102, digest=x86_digest
        ),
        release_artifact_record(
            name=MACOS_RELEASE_RUN_ARTIFACTS[2], artifact_id=201, digest="d" * 64
        ),
        release_artifact_record(
            name=MACOS_RELEASE_RUN_ARTIFACTS[3], artifact_id=202, digest="e" * 64
        ),
        release_artifact_record(
            name="tr300-prepared-release-assets",
            artifact_id=103,
            digest=(
                "0" * 64
                if mutation == "prepared-digest-mismatch"
                else prepared_digest
            ),
        ),
        release_artifact_record(
            name="artifacts-build-local-x86_64-pc-windows-msvc",
            artifact_id=301,
            digest="f" * 64,
        ),
    ]
    artifacts_after = json.loads(json.dumps(artifacts))

    if mutation == "wrong-run-event":
        run_before["event"] = run_after["event"] = "pull_request"
    elif mutation == "wrong-run-repository":
        run_before["repository"] = run_after["repository"] = {
            "full_name": "attacker/fork"
        }
    elif mutation == "wrong-run-head-repository":
        run_before["head_repository"] = run_after["head_repository"] = {
            "full_name": "attacker/fork"
        }
    elif mutation == "wrong-run-sha":
        run_before["head_sha"] = run_after["head_sha"] = OTHER_SHA
    elif mutation == "wrong-run-path":
        run_before["path"] = run_after["path"] = ".github/workflows/attacker.yml"
    elif mutation == "wrong-workflow-id":
        run_before["workflow_id"] = run_after["workflow_id"] = 42
    elif mutation == "wrong-attempt":
        run_before["run_attempt"] = run_after["run_attempt"] = 2
    elif mutation == "midflight-attempt":
        run_after["run_attempt"] = 2
    elif mutation == "extra-apple-artifact":
        artifacts.append(
            release_artifact_record(
                name="attacker-apple-darwin", artifact_id=401, digest="1" * 64
            )
        )
        artifacts_after = json.loads(json.dumps(artifacts))
    elif mutation == "missing-canonical-artifact":
        artifacts = artifacts[1:]
        artifacts_after = json.loads(json.dumps(artifacts))
    elif mutation == "duplicate-canonical-artifact":
        duplicate = json.loads(json.dumps(artifacts[0]))
        duplicate["id"] = 999
        artifacts.append(duplicate)
        artifacts_after = json.loads(json.dumps(artifacts))
    elif mutation == "expired-artifact":
        artifacts[0]["expired"] = True
        artifacts_after = json.loads(json.dumps(artifacts))
    elif mutation == "wrong-artifact-repository":
        artifacts[0]["workflow_run"]["repository_id"] = 42  # type: ignore[index]
        artifacts_after = json.loads(json.dumps(artifacts))
    elif mutation == "midflight-artifact-replacement":
        artifacts_after[0]["digest"] = f"sha256:{'9' * 64}"
    elif mutation == "missing-prepared-artifact":
        artifacts = [artifact for artifact in artifacts if artifact["id"] != 103]
        artifacts_after = json.loads(json.dumps(artifacts))
    elif mutation == "duplicate-prepared-artifact":
        duplicate = json.loads(
            json.dumps(next(artifact for artifact in artifacts if artifact["id"] == 103))
        )
        duplicate["id"] = 998
        artifacts.append(duplicate)
        artifacts_after = json.loads(json.dumps(artifacts))
    elif mutation == "midflight-prepared-replacement":
        for artifact in artifacts_after:
            if artifact["id"] == 103:
                artifact["digest"] = f"sha256:{'8' * 64}"
                break

    run_before_path = directory / "run-before.json"
    run_after_path = directory / "run-after.json"
    artifacts_before_path = directory / "artifacts-before.json"
    artifacts_after_path = directory / "artifacts-after.json"
    direct_artifact_path = directory / "direct-artifact.json"
    run_before_path.write_text(json.dumps(run_before), encoding="utf-8")
    run_after_path.write_text(json.dumps(run_after), encoding="utf-8")
    artifacts_before_path.write_text(
        json.dumps({"total_count": len(artifacts), "artifacts": artifacts}),
        encoding="utf-8",
    )
    artifacts_after_path.write_text(
        json.dumps(
            {"total_count": len(artifacts_after), "artifacts": artifacts_after}
        ),
        encoding="utf-8",
    )
    direct_artifact_path.write_text(json.dumps(artifacts[0]), encoding="utf-8")
    return {
        "MOCK_GH_STATE_DIR": directory.as_posix(),
        "MOCK_RUN_BEFORE": run_before_path.as_posix(),
        "MOCK_RUN_AFTER": run_after_path.as_posix(),
        "MOCK_ARTIFACTS_BEFORE": artifacts_before_path.as_posix(),
        "MOCK_ARTIFACTS_AFTER": artifacts_after_path.as_posix(),
        "MOCK_DIRECT_ARTIFACT_JSON": direct_artifact_path.as_posix(),
        "MOCK_ARM_ARTIFACT_ZIP": arm_zip.as_posix(),
        "MOCK_X86_ARTIFACT_ZIP": x86_zip.as_posix(),
        "MOCK_PREPARED_RELEASE_ARTIFACT_ZIP": prepared_zip.as_posix(),
        "MOCK_PREPARED_WRAPPER_SHA256": prepared_custody_assets[
            "tr300-installer.sh"
        ][0],
        "MOCK_PREPARED_WRAPPER_SIZE": str(
            prepared_custody_assets["tr300-installer.sh"][1]
        ),
        "MOCK_PREPARED_DIST_SHA256": prepared_custody_assets[
            "tr300-dist-installer.sh"
        ][0],
        "MOCK_PREPARED_DIST_SIZE": str(
            prepared_custody_assets["tr300-dist-installer.sh"][1]
        ),
        "MOCK_PREPARED_ARM_SHA256": prepared_custody_assets[
            "tr300-aarch64-apple-darwin.tar.xz"
        ][0],
        "MOCK_PREPARED_ARM_SIZE": str(
            prepared_custody_assets["tr300-aarch64-apple-darwin.tar.xz"][1]
        ),
        "MOCK_PREPARED_INTEL_SHA256": prepared_custody_assets[
            "tr300-x86_64-apple-darwin.tar.xz"
        ][0],
        "MOCK_PREPARED_INTEL_SIZE": str(
            prepared_custody_assets["tr300-x86_64-apple-darwin.tar.xz"][1]
        ),
    }


def run_macos_source_custody_case(
    *,
    bash: str,
    mock_bin: Path,
    block: str,
    name: str,
    mutation: str | None,
    expected_success: bool,
) -> None:
    with tempfile.TemporaryDirectory(prefix="tr300-macos-custody-") as raw:
        directory = Path(raw)
        output = directory / "normalized"
        github_output = directory / "github-output"
        log = directory / "gh-calls"
        environment = fixture_environment(mock_bin, github_output, log)
        environment.update(write_macos_release_run_fixture(directory, mutation))
        environment.update(
            {
                "RUNNER_TEMP": directory.as_posix(),
                "OUTPUT_DIRECTORY": output.as_posix(),
                "RELEASE_TAG": "v4.3.0",
                "EXPECTED_SHA": TRUSTED_SHA,
                "RELEASE_ID": "777777",
                "RELEASE_RUN_ID": RELEASE_RUN_ID,
                "RELEASE_RUN_ATTEMPT": RELEASE_RUN_ATTEMPT,
            }
        )
        execution_block = block
        if os.name == "nt":
            # NTFS/Git Bash cannot enforce POSIX modes and its /usr/bin takes
            # precedence over fixture PATH. Keep the original hosted block's
            # mode-bearing commands structurally required, but execute the
            # same custody flow locally without unsupported chmod requests.
            if 'mkdir -m 700 "$OUTPUT_DIRECTORY"' not in block:
                raise AssertionError(f"{name}: hosted private-directory mode guard changed")
            execution_block = execution_block.replace("mkdir -m 700 ", "mkdir ")
        script = directory / "workflow-run.sh"
        script.write_text(execution_block, encoding="utf-8", newline="\n")
        result = subprocess.run(
            [bash, str(script)],
            cwd=ROOT,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=30,
            check=False,
        )
        succeeded = result.returncode == 0
        if succeeded != expected_success:
            raise AssertionError(
                f"{name}: return code {result.returncode}, expected_success="
                f"{expected_success}\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
            )
        if mutation == "prepared-canonical-divergence" and (
            "prepared Release archive differs from the canonical signed artifact: "
            "tr300-aarch64-apple-darwin.tar.xz"
            not in result.stderr
        ):
            raise AssertionError(
                f"{name}: divergent valid archives did not reach the equality guard\n"
                f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
            )
        if expected_success:
            expected = {
                "tr300-installer.sh",
                "tr300-dist-installer.sh",
                "tr300-aarch64-apple-darwin.tar.xz",
                "tr300-aarch64-apple-darwin.tar.xz.sha256",
                "tr300-x86_64-apple-darwin.tar.xz",
                "tr300-x86_64-apple-darwin.tar.xz.sha256",
            }
            if {path.name for path in output.iterdir()} != expected:
                raise AssertionError(f"{name}: normalized inventory changed")
            for asset_name, digest_key, size_key in (
                (
                    "tr300-installer.sh",
                    "MOCK_PREPARED_WRAPPER_SHA256",
                    "MOCK_PREPARED_WRAPPER_SIZE",
                ),
                (
                    "tr300-dist-installer.sh",
                    "MOCK_PREPARED_DIST_SHA256",
                    "MOCK_PREPARED_DIST_SIZE",
                ),
            ):
                payload = (output / asset_name).read_bytes()
                if hashlib.sha256(payload).hexdigest() != environment[digest_key]:
                    raise AssertionError(f"{name}: normalized {asset_name} digest changed")
                if len(payload) != int(environment[size_key]):
                    raise AssertionError(f"{name}: normalized {asset_name} size changed")
            expected_outputs = {
                "managed_installer_sha256": environment[
                    "MOCK_PREPARED_WRAPPER_SHA256"
                ],
                "managed_installer_size": environment["MOCK_PREPARED_WRAPPER_SIZE"],
                "dist_installer_sha256": environment["MOCK_PREPARED_DIST_SHA256"],
                "dist_installer_size": environment["MOCK_PREPARED_DIST_SIZE"],
                "aarch64_archive_sha256": environment["MOCK_PREPARED_ARM_SHA256"],
                "aarch64_archive_size": environment["MOCK_PREPARED_ARM_SIZE"],
                "x86_64_archive_sha256": environment[
                    "MOCK_PREPARED_INTEL_SHA256"
                ],
                "x86_64_archive_size": environment["MOCK_PREPARED_INTEL_SIZE"],
            }
            if parse_outputs(github_output) != expected_outputs:
                raise AssertionError(f"{name}: managed-wrapper custody outputs changed")
        calls = log.read_text(encoding="utf-8").splitlines()
        if any("/releases/" in call for call in calls):
            raise AssertionError(f"{name}: source custody consulted mutable Release assets")


def write_macos_native_build_artifact_fixture(
    directory: Path, mutation: str | None
) -> dict[str, str]:
    source_directory = directory / "trusted-source-inputs"
    source_directory.mkdir()
    source_payloads = {
        "tr300-installer.sh": b"#!/bin/sh\n# managed source fixture\n",
        "tr300-dist-installer.sh": b"#!/bin/sh\n# raw cargo-dist source fixture\n",
        "tr300-aarch64-apple-darwin.tar.xz": b"arm64 source archive fixture\n",
        "tr300-x86_64-apple-darwin.tar.xz": b"x86_64 source archive fixture\n",
    }
    source_digests: dict[str, str] = {}
    source_sizes: dict[str, int] = {}
    for asset_name, payload in source_payloads.items():
        (source_directory / asset_name).write_bytes(payload)
        source_digests[asset_name] = hashlib.sha256(payload).hexdigest()
        source_sizes[asset_name] = len(payload)
    for archive_name in (
        "tr300-aarch64-apple-darwin.tar.xz",
        "tr300-x86_64-apple-darwin.tar.xz",
    ):
        (source_directory / f"{archive_name}.sha256").write_text(
            f"{source_digests[archive_name]} *{archive_name}\n",
            encoding="utf-8",
            newline="\n",
        )
    source_zip = directory / "trusted-source-inputs.zip"
    with zipfile.ZipFile(source_zip, "w", compression=zipfile.ZIP_STORED) as archive:
        for path in sorted(source_directory.iterdir()):
            archive.write(path, arcname=path.name)
    source_artifact_digest = hashlib.sha256(source_zip.read_bytes()).hexdigest()
    source_artifact_id = 502
    source_artifact_record: dict[str, object] = {
        "id": source_artifact_id,
        "name": "tr300-trusted-apple-release-inputs",
        "digest": f"sha256:{source_artifact_digest}",
        "expired": False,
        "size_in_bytes": source_zip.stat().st_size,
        "workflow_run": {
            "id": int(RELEASE_RUN_ID),
            "repository_id": int(REPOSITORY_ID),
            "head_repository_id": int(REPOSITORY_ID),
            "head_sha": TRUSTED_SHA,
        },
    }
    source_metadata = directory / "trusted-source-artifact.json"
    source_metadata.write_text(json.dumps(source_artifact_record), encoding="utf-8")

    artifact_zip = directory / "native-build.zip"
    expected = (
        "tr300-universal-apple-darwin.pkg",
        "tr300-universal-apple-darwin.pkg.sha256",
        "tr300-universal-apple-darwin.dmg",
        "tr300-universal-apple-darwin.dmg.sha256",
    )
    with zipfile.ZipFile(artifact_zip, "w", compression=zipfile.ZIP_STORED) as archive:
        for name in expected:
            archive.writestr(name, f"native build fixture for {name}\n".encode())
        if mutation == "native-extra-member":
            archive.writestr("unexpected", b"unexpected\n")
        elif mutation == "native-unsafe-path":
            archive.writestr("../escape", b"escape\n")
    zip_digest = hashlib.sha256(artifact_zip.read_bytes()).hexdigest()
    expected_digest = "0" * 64 if mutation == "native-zip-digest-mismatch" else zip_digest
    artifact_record: dict[str, object] = {
        "id": 501,
        "name": "tr300-universal-macos-installer",
        "digest": f"sha256:{expected_digest}",
        "expired": False,
        "size_in_bytes": artifact_zip.stat().st_size,
        "workflow_run": {
            "id": int(RELEASE_RUN_ID),
            "repository_id": int(REPOSITORY_ID),
            "head_repository_id": int(REPOSITORY_ID),
            "head_sha": TRUSTED_SHA,
        },
    }
    if mutation == "native-wrong-id":
        artifact_record["id"] = 502
    elif mutation == "native-api-digest-mismatch":
        artifact_record["digest"] = f"sha256:{'f' * 64}"
    elif mutation == "native-expired":
        artifact_record["expired"] = True
    elif mutation == "native-wrong-run":
        artifact_record["workflow_run"]["id"] = 42  # type: ignore[index]
    elif mutation == "native-wrong-repository":
        artifact_record["workflow_run"]["repository_id"] = 42  # type: ignore[index]
    elif mutation == "native-wrong-head-repository":
        artifact_record["workflow_run"]["head_repository_id"] = 42  # type: ignore[index]
    elif mutation == "native-wrong-sha":
        artifact_record["workflow_run"]["head_sha"] = OTHER_SHA  # type: ignore[index]

    metadata = directory / "native-build-artifact.json"
    metadata.write_text(json.dumps(artifact_record), encoding="utf-8")
    return {
        "RUNNER_TEMP": directory.as_posix(),
        "REPOSITORY": REPOSITORY,
        "EXPECTED_SHA": TRUSTED_SHA,
        "CURRENT_RUN_ID": RELEASE_RUN_ID,
        "CURRENT_RUN_ATTEMPT": RELEASE_RUN_ATTEMPT,
        "MATRIX_ARCH": "x86_64",
        "BUILD_ARTIFACT_ID": "501",
        "BUILD_ARTIFACT_DIGEST": expected_digest,
        "SOURCE_ARTIFACT_ID": str(source_artifact_id),
        "SOURCE_ARTIFACT_DIGEST": source_artifact_digest,
        "TRUSTED_SOURCE_DIRECTORY": source_directory.as_posix(),
        "SOURCE_MANAGED_INSTALLER_SHA256": source_digests["tr300-installer.sh"],
        "SOURCE_MANAGED_INSTALLER_SIZE": str(source_sizes["tr300-installer.sh"]),
        "SOURCE_DIST_INSTALLER_SHA256": source_digests["tr300-dist-installer.sh"],
        "SOURCE_DIST_INSTALLER_SIZE": str(source_sizes["tr300-dist-installer.sh"]),
        "SOURCE_AARCH64_ARCHIVE_SHA256": source_digests[
            "tr300-aarch64-apple-darwin.tar.xz"
        ],
        "SOURCE_AARCH64_ARCHIVE_SIZE": str(
            source_sizes["tr300-aarch64-apple-darwin.tar.xz"]
        ),
        "SOURCE_X86_64_ARCHIVE_SHA256": source_digests[
            "tr300-x86_64-apple-darwin.tar.xz"
        ],
        "SOURCE_X86_64_ARCHIVE_SIZE": str(
            source_sizes["tr300-x86_64-apple-darwin.tar.xz"]
        ),
        "VALIDATION_PROOF": (directory / "native-proof.json").as_posix(),
        "MOCK_DIRECT_ARTIFACT_JSON": metadata.as_posix(),
        "MOCK_SOURCE_ARTIFACT_ID": str(source_artifact_id),
        "MOCK_SOURCE_ARTIFACT_JSON": source_metadata.as_posix(),
        "MOCK_PUBLISHER_ARTIFACT_ZIP": artifact_zip.as_posix(),
    }


def run_macos_native_build_custody_case(
    *,
    bash: str,
    mock_bin: Path,
    block: str,
    name: str,
    mutation: str | None,
    expected_success: bool,
) -> None:
    with tempfile.TemporaryDirectory(prefix="tr300-macos-native-custody-") as raw:
        directory = Path(raw)
        output = directory / "github-output"
        log = directory / "gh-calls"
        environment = fixture_environment(mock_bin, output, log)
        environment.update(write_macos_native_build_artifact_fixture(directory, mutation))
        environment["RELEASE_ID"] = "777777"
        script = directory / "native-custody.sh"
        script.write_text(block, encoding="utf-8", newline="\n")
        result = subprocess.run(
            [bash, str(script)],
            cwd=ROOT,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=20,
            check=False,
        )
        succeeded = result.returncode == 0
        if succeeded != expected_success:
            raise AssertionError(
                f"{name}: return code {result.returncode}, expected_success="
                f"{expected_success}\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
            )


def write_windows_release_artifact(directory: Path) -> None:
    directory.mkdir()
    for index, payload_name in enumerate(WINDOWS_RELEASE_PAYLOADS, start=1):
        payload = directory / payload_name
        payload.write_bytes(f"fixture installer {index}\n".encode())
        digest = hashlib.sha256(payload.read_bytes()).hexdigest()
        (directory / f"{payload_name}.sha256").write_text(
            f"{digest} *{payload_name}", encoding="utf-8"
        )


def run_windows_publisher_case(
    *,
    bash: str,
    mock_bin: Path,
    block: str,
    name: str,
    overrides: dict[str, str] | None = None,
    artifact_mutation: str | None = None,
    expected_success: bool,
) -> None:
    with tempfile.TemporaryDirectory(prefix="tr300-publisher-case-") as case_dir_raw:
        case_dir = Path(case_dir_raw)
        artifact = case_dir / "artifact-source"
        extracted = case_dir / WINDOWS_RELEASE_ARTIFACT
        output = case_dir / "github-output"
        log = case_dir / "gh-calls"
        write_windows_release_artifact(artifact)

        if artifact_mutation == "extra":
            (artifact / "unexpected.asset").write_bytes(b"unexpected")
        elif artifact_mutation == "missing":
            (artifact / WINDOWS_RELEASE_ASSETS[-1]).unlink()
        elif artifact_mutation == "empty":
            (artifact / WINDOWS_RELEASE_PAYLOADS[0]).write_bytes(b"")
        elif artifact_mutation == "directory":
            target = artifact / WINDOWS_RELEASE_PAYLOADS[0]
            target.unlink()
            target.mkdir()
        elif artifact_mutation == "bad-checksum":
            (artifact / f"{WINDOWS_RELEASE_PAYLOADS[0]}.sha256").write_text(
                f"{'0' * 64} *{WINDOWS_RELEASE_PAYLOADS[0]}", encoding="utf-8"
            )
        elif artifact_mutation is not None:
            raise AssertionError(f"unknown artifact mutation: {artifact_mutation}")

        artifact_zip = case_dir / "windows-release-assets.zip"
        with zipfile.ZipFile(artifact_zip, "w", compression=zipfile.ZIP_STORED) as archive:
            for path in sorted(artifact.iterdir()):
                if path.is_dir():
                    archive.writestr(f"{path.name}/", b"")
                else:
                    archive.write(path, path.name)
        artifact_digest = hashlib.sha256(artifact_zip.read_bytes()).hexdigest()
        current_run_id = "555001"
        artifact_id = "501"
        direct_artifact = case_dir / "direct-artifact.json"
        direct_artifact.write_text(
            json.dumps(
                {
                    "id": int(artifact_id),
                    "name": WINDOWS_RELEASE_ARTIFACT,
                    "expired": False,
                    "size_in_bytes": artifact_zip.stat().st_size,
                    "digest": f"sha256:{artifact_digest}",
                    "workflow_run": {
                        "id": int(current_run_id),
                        "repository_id": int(REPOSITORY_ID),
                        "head_repository_id": int(REPOSITORY_ID),
                        "head_sha": TRUSTED_SHA,
                    },
                }
            ),
            encoding="utf-8",
        )
        ci_runs = case_dir / "ci-runs.json"
        ci_runs.write_text(
            json.dumps(
                {
                    "workflow_runs": [
                        {
                            "id": 777,
                            "workflow_id": 333333,
                            "name": "CI",
                            "path": ".github/workflows/ci.yml",
                            "event": "push",
                            "status": "completed",
                            "conclusion": "success",
                            "repository": {"full_name": REPOSITORY},
                            "head_repository": {"full_name": REPOSITORY},
                            "head_branch": "main",
                            "head_sha": TRUSTED_SHA,
                        }
                    ]
                }
            ),
            encoding="utf-8",
        )

        environment = fixture_environment(mock_bin, output, log)
        environment.update(
            {
                "RELEASE_TAG": "v4.3.0",
                "EXPECTED_SHA": TRUSTED_SHA,
                "RELEASE_ID": "777777",
                "ASSET_DIRECTORY": extracted.as_posix(),
                "CURRENT_RUN_ID": current_run_id,
                "INTERNAL_ARTIFACT_ID": artifact_id,
                "INTERNAL_ARTIFACT_DIGEST": artifact_digest,
                "WORKFLOW_SHA": TRUSTED_SHA,
                "RUNNER_TEMP": case_dir.as_posix(),
                "MOCK_GH_STATE_DIR": case_dir.as_posix(),
                "MOCK_DIRECT_ARTIFACT_JSON": direct_artifact.as_posix(),
                "MOCK_PUBLISHER_ARTIFACT_ZIP": artifact_zip.as_posix(),
                "MOCK_CI_RUNS_JSON": ci_runs.as_posix(),
                "MOCK_CI_WORKFLOW_ID": "333333",
                "MOCK_CURRENT_ASSETS": "\n".join(WINDOWS_UPSTREAM_SENTINELS),
            }
        )
        if overrides:
            environment.update(overrides)
        script = case_dir / "workflow-run.sh"
        script.write_text(block, encoding="utf-8", newline="\n")
        bash_arguments = [bash]
        if os.environ.get("TR300_TEST_XTRACE") == "1":
            bash_arguments.append("-x")
        bash_arguments.append(str(script))
        result = subprocess.run(
            bash_arguments,
            cwd=ROOT,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=15,
            check=False,
        )
        succeeded = result.returncode == 0
        if succeeded != expected_success:
            raise AssertionError(
                f"{name}: return code {result.returncode}, expected_success="
                f"{expected_success}\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
            )

        calls = log.read_text(encoding="utf-8").splitlines() if log.exists() else []
        upload_arguments = [
            line.removeprefix("release-upload:")
            for line in calls
            if line.startswith("release-upload:")
        ]
        if expected_success:
            expected_arguments = [
                "release",
                "upload",
                "v4.3.0",
                "--repo",
                REPOSITORY,
                *(f"{extracted.as_posix()}/{asset}" for asset in WINDOWS_RELEASE_ASSETS),
            ]
            if upload_arguments != expected_arguments:
                raise AssertionError(
                    f"{name}: upload arguments {upload_arguments!r} != "
                    f"{expected_arguments!r}"
                )
        elif upload_arguments:
            raise AssertionError(f"{name}: rejected case reached release upload")


def payloads_with_sidecars(names: tuple[str, ...], label: str) -> dict[str, bytes]:
    payloads: dict[str, bytes] = {}
    for index, name in enumerate(names, start=1):
        content = f"fixture {label} payload {index} for {name}\n".encode()
        payloads[name] = content
        payloads[f"{name}.sha256"] = (
            f"{hashlib.sha256(content).hexdigest()} *{name}\n".encode()
        )
    return payloads


def initial_release_asset_payloads() -> dict[str, bytes]:
    payloads = {
        name: f"fixture initial Release asset for {name}\n".encode()
        for name in INITIAL_RELEASE_ASSETS
        if name != "sha256.sum" and not name.endswith(".sha256")
    }
    checksum_lines = []
    for sidecar in (name for name in INITIAL_RELEASE_ASSETS if name.endswith(".sha256")):
        payload_name = sidecar.removesuffix(".sha256")
        content = payloads[payload_name]
        sidecar_content = (
            f"{hashlib.sha256(content).hexdigest()} *{payload_name}\n".encode()
        )
        payloads[sidecar] = sidecar_content
        checksum_lines.append(sidecar_content)
    payloads["sha256.sum"] = b"".join(checksum_lines)
    if set(payloads) != set(INITIAL_RELEASE_ASSETS):
        raise AssertionError("initial Release fixture inventory drifted")
    return payloads


def write_payload_directory(directory: Path, payloads: dict[str, bytes]) -> None:
    directory.mkdir()
    for name, content in payloads.items():
        (directory / name).write_bytes(content)


def write_payload_zip(path: Path, payloads: dict[str, bytes]) -> str:
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_STORED) as archive:
        for name in sorted(payloads):
            archive.writestr(name, payloads[name])
    return hashlib.sha256(path.read_bytes()).hexdigest()


def release_asset_records(
    payloads: dict[str, bytes], *, first_id: int
) -> list[dict[str, object]]:
    return [
        {
            "id": first_id + index,
            "name": name,
            "size": len(payloads[name]),
            "digest": f"sha256:{hashlib.sha256(payloads[name]).hexdigest()}",
        }
        for index, name in enumerate(sorted(payloads))
    ]


def artifact_record(
    *,
    artifact_id: int,
    name: str,
    digest: str,
    size: int,
    run_id: int,
) -> dict[str, object]:
    return {
        "id": artifact_id,
        "name": name,
        "digest": f"sha256:{digest}",
        "expired": False,
        "size_in_bytes": size,
        "workflow_run": {
            "id": run_id,
            "repository_id": int(REPOSITORY_ID),
            "head_repository_id": int(REPOSITORY_ID),
            "head_sha": TRUSTED_SHA,
        },
    }


def write_macos_publisher_jq(bin_dir: Path) -> None:
    mock = bin_dir / "jq"
    mock.write_text(
        textwrap.dedent(
            r'''
            #!/usr/bin/env python3
            import json
            import re
            import sys

            sys.stdout.reconfigure(newline="\n")
            arguments = sys.argv[1:]
            variables = {}
            raw_output = False
            raw_input = False
            slurp = False
            index = 0
            while index < len(arguments):
                argument = arguments[index]
                if argument in ("-e", "-r", "-c", "-R", "-s", "-er", "-re"):
                    raw_output = raw_output or "r" in argument
                    raw_input = raw_input or argument == "-R"
                    slurp = slurp or argument == "-s"
                    index += 1
                    continue
                if argument in ("--arg", "--argjson", "--slurpfile"):
                    if index + 2 >= len(arguments):
                        raise SystemExit(2)
                    name, value = arguments[index + 1], arguments[index + 2]
                    if argument == "--argjson":
                        variables[name] = json.loads(value)
                    elif argument == "--slurpfile":
                        with open(value, "r", encoding="utf-8") as source:
                            variables[name] = [json.load(source)]
                    else:
                        variables[name] = value
                    index += 3
                    continue
                break

            if index >= len(arguments):
                raise SystemExit(2)
            query = arguments[index]
            index += 1

            def normalized_query(value):
                parts = re.split(r'("(?:\\.|[^"\\])*")', value)
                return "".join(
                    part if part_index % 2 else re.sub(r"\s+", " ", part)
                    for part_index, part in enumerate(parts)
                ).strip()

            normalized = normalized_query(query)

            if raw_input:
                if query != "." or index != len(arguments):
                    raise SystemExit("publisher jq fixture rejects raw-input filter")
                for line in sys.stdin.read().splitlines():
                    print(json.dumps(line))
                raise SystemExit(0)
            if slurp:
                if normalized != "sort" or index != len(arguments):
                    raise SystemExit("publisher jq fixture rejects slurp filter")
                values = [json.loads(line) for line in sys.stdin.read().splitlines() if line]
                print(json.dumps(sorted(values), separators=(",", ":")))
                raise SystemExit(0)
            if index + 1 != len(arguments):
                raise SystemExit(f"publisher jq fixture rejects arguments: {arguments!r}")
            path = arguments[index]
            with open(path, "r", encoding="utf-8") as source:
                data = json.load(source)

            def nested(record, *parts):
                value = record
                for part in parts:
                    if not isinstance(value, dict) or part not in value:
                        return None
                    value = value[part]
                return value

            def emit(value):
                if raw_output and isinstance(value, str):
                    print(value)
                elif isinstance(value, bool):
                    print("true" if value else "false")
                elif value is not None:
                    print(json.dumps(value, separators=(",", ":")))

            def emit_tsv(values):
                emit("\t".join(
                    str(value).lower() if isinstance(value, bool) else str(value)
                    for value in values
                ))

            def require(condition):
                raise SystemExit(0 if condition else 1)

            def artifact_projection(records):
                return sorted(
                    [
                        {
                            "id": record.get("id"),
                            "name": record.get("name"),
                            "size": record.get("size"),
                            "digest": record.get("digest"),
                        }
                        for record in records
                    ],
                    key=lambda record: record["name"],
                )

            def bound_release_asset(records, name, digest, size):
                matches = [record for record in records if record.get("name") == name]
                return (
                    len(matches) == 1
                    and matches[0].get("digest") == digest
                    and matches[0].get("size") == size
                )

            native_proof_inventory_query = normalized_query("""
                [.artifacts[] | select(.name == $name)] |
                if length != 1 then error("expected exactly one native proof artifact named " + $name)
                else .[0] |
                  [.id, .name, .digest, .expired, .size_in_bytes,
                   .workflow_run.id, .workflow_run.repository_id,
                   .workflow_run.head_repository_id, .workflow_run.head_sha] | @tsv
                end
            """)
            direct_artifact_query = normalized_query("""
                [.id, .name, .digest, .expired, .size_in_bytes,
                 .workflow_run.id, .workflow_run.repository_id,
                 .workflow_run.head_repository_id, .workflow_run.head_sha] | @tsv
            """)
            artifact_validator_queries = {
                normalized_query("""
                    .id == $id and .name == "tr300-universal-macos-installer" and
                    .expired == false and (.size_in_bytes > 0 and .size_in_bytes <= 268435456) and
                    .digest == $digest and .workflow_run.id == $run and
                    .workflow_run.repository_id == $repository and
                    .workflow_run.head_repository_id == $repository
                """): ("tr300-universal-macos-installer", 268435456, False),
                normalized_query("""
                    .id == $id and .name == "windows-release-assets" and .expired == false and
                    (.size_in_bytes > 0 and .size_in_bytes <= 268435456) and .digest == $digest and
                    .workflow_run.id == $run and .workflow_run.repository_id == $repository and
                    .workflow_run.head_repository_id == $repository and .workflow_run.head_sha == $sha
                """): ("windows-release-assets", 268435456, True),
                normalized_query("""
                    .id == $id and .name == "windows-installer-validation-proof" and
                    .expired == false and (.size_in_bytes > 0 and .size_in_bytes <= 1048576) and
                    .digest == $digest and .workflow_run.id == $run and
                    .workflow_run.repository_id == $repository and
                    .workflow_run.head_repository_id == $repository and .workflow_run.head_sha == $sha
                """): ("windows-installer-validation-proof", 1048576, True),
                normalized_query("""
                    .id == $id and .name == "windows-validation-inputs" and .expired == false and
                    (.size_in_bytes > 0 and .size_in_bytes <= 1073741824) and .digest == $digest and
                    .workflow_run.id == $run and .workflow_run.repository_id == $repository and
                    .workflow_run.head_repository_id == $repository and .workflow_run.head_sha == $sha
                """): ("windows-validation-inputs", 1073741824, True),
            }
            native_proof_query = normalized_query("""
                .schema_version == 1 and .tag == $tag and .source_sha == $sha and
                .workflow_run_id == $run and .workflow_run_attempt == $attempt and
                .architecture == $arch and .build_artifact_id == $build_id and
                .build_artifact_digest == $build_digest and
                .managed_installer == {digest: $managed_digest, size: $managed_size} and
                .dist_installer == {digest: $dist_digest, size: $dist_size} and
                .aarch64_archive == {digest: $arm_digest, size: $arm_size} and
                .x86_64_archive == {digest: $intel_digest, size: $intel_size} and
                (keys | sort) == (["aarch64_archive", "architecture", "build_artifact_digest",
                  "build_artifact_id", "dist_installer", "managed_installer", "schema_version",
                  "source_sha", "tag", "workflow_run_attempt", "workflow_run_id",
                  "x86_64_archive"] | sort)
            """)
            workflow_run_queries = {
                normalized_query("""
                    .workflow_runs[] |
                    select(.workflow_id == $workflow_id and .name == "Windows Installers" and
                           .path == ".github/workflows/windows-installers.yml" and
                           .event == "workflow_run" and .status == "completed" and
                           .conclusion == "success" and .repository.full_name == $repository and
                           .head_repository.full_name == $repository and .head_branch == "main" and
                           .head_sha == $sha) | .id
                """): ("Windows Installers", ".github/workflows/windows-installers.yml", "workflow_run"),
                normalized_query("""
                    .workflow_runs[] |
                    select(.workflow_id == $workflow_id and .name == "Windows Installer Validation" and
                           .path == ".github/workflows/windows-installer-validation.yml" and
                           .event == "workflow_run" and .status == "completed" and
                           .conclusion == "success" and .repository.full_name == $repository and
                           .head_repository.full_name == $repository and .head_branch == "main" and
                           .head_sha == $sha) | .id
                """): ("Windows Installer Validation", ".github/workflows/windows-installer-validation.yml", "workflow_run"),
                normalized_query("""
                    .workflow_runs[] |
                    select(.workflow_id == $workflow_id and .name == "CI" and
                           .path == ".github/workflows/ci.yml" and .event == "push" and
                           .status == "completed" and .conclusion == "success" and
                           .repository.full_name == $repository and
                           .head_repository.full_name == $repository and
                           .head_branch == "main" and .head_sha == $sha) | .id
                """): ("CI", ".github/workflows/ci.yml", "push"),
            }
            windows_artifact_inventory_query = normalized_query("""
                .artifacts[] | select(.name == "windows-release-assets" and .expired == false) |
                [.id, .digest] | @tsv
            """)
            validation_artifact_inventory_query = normalized_query("""
                .artifacts[] |
                select(.name == "windows-installer-validation-proof" and .expired == false) |
                [.id, .digest] | @tsv
            """)
            validation_proof_query = normalized_query("""
                .schema_version == 1 and .result == "success" and .tag == $tag and
                .source_sha == $sha and .windows_run_id == $windows_run and
                .windows_run_attempt == $windows_attempt and
                .windows_artifact_id == $windows_artifact and
                .windows_artifact_digest == $windows_digest and
                .validation_run_id == $validation_run and
                .validation_run_attempt == $validation_attempt and
                (.validation_input_artifact_id | type == "number") and
                (.validation_input_artifact_digest | test("^sha256:[0-9a-f]{64}$"))
            """)
            validation_manifest_query = normalized_query("""
                .schema_version == 1 and .validation_mode == "private" and
                .tag == $tag and .source_sha == $sha and .upstream_run_id == $windows_run and
                .upstream_run_attempt == $windows_attempt and
                .windows_artifact_id == $windows_artifact and
                .windows_artifact_digest == $windows_digest and
                (.release_assets | length) == 30 and
                ([.release_assets[] | select(.name == "tr300-installer.sh")] | length) == 1 and
                ([.release_assets[] | select(.name == "tr300-installer.sh")][0] |
                  .digest == $managed_digest and .size == $managed_size) and
                ([.release_assets[] | select(.name == "tr300-dist-installer.sh")] | length) == 1 and
                ([.release_assets[] | select(.name == "tr300-dist-installer.sh")][0] |
                  .digest == $dist_digest and .size == $dist_size) and
                ([.release_assets[] | select(.name == "tr300-aarch64-apple-darwin.tar.xz")] |
                  length) == 1 and
                ([.release_assets[] | select(.name == "tr300-aarch64-apple-darwin.tar.xz")][0] |
                  .digest == $arm_digest and .size == $arm_size) and
                ([.release_assets[] | select(.name == "tr300-x86_64-apple-darwin.tar.xz")] |
                  length) == 1 and
                ([.release_assets[] | select(.name == "tr300-x86_64-apple-darwin.tar.xz")][0] |
                  .digest == $intel_digest and .size == $intel_size) and
                ([.release_assets[] | (.id | type == "number") and (.size > 0) and
                  (.digest | type == "string" and test("^sha256:[0-9a-f]{64}$"))] | all)
            """)
            validated_draft_query = normalized_query("""
                .id == $release_id and .tag_name == $tag and
                .target_commitish == $sha and
                .draft == true and .prerelease == false and
                ([.assets[] | select(.name == "tr300-installer.sh")] | length) == 1 and
                ([.assets[] | select(.name == "tr300-installer.sh")][0] |
                  .digest == $managed_digest and .size == $managed_size) and
                ([.assets[] | select(.name == "tr300-dist-installer.sh")] | length) == 1 and
                ([.assets[] | select(.name == "tr300-dist-installer.sh")][0] |
                  .digest == $dist_digest and .size == $dist_size) and
                ([.assets[] | select(.name == "tr300-aarch64-apple-darwin.tar.xz")] |
                  length) == 1 and
                ([.assets[] | select(.name == "tr300-aarch64-apple-darwin.tar.xz")][0] |
                  .digest == $arm_digest and .size == $arm_size) and
                ([.assets[] | select(.name == "tr300-x86_64-apple-darwin.tar.xz")] |
                  length) == 1 and
                ([.assets[] | select(.name == "tr300-x86_64-apple-darwin.tar.xz")][0] |
                  .digest == $intel_digest and .size == $intel_size) and
                ([.assets[] | {id, name, size, digest}] | sort_by(.name)) ==
                  ($manifest[0].release_assets | sort_by(.name))
            """)
            release_manifest_query = normalized_query("""
                .id == $release_id and .tag_name == $tag and
                .target_commitish == $sha and
                .draft == true and .prerelease == false and
                ([.assets[] | {id, name, size, digest}] | sort_by(.name)) ==
                  ($manifest[0].release_assets | sort_by(.name))
            """)
            draft_final_inventory_query = normalized_query("""
                .id == $release_id and .tag_name == $tag and
                .target_commitish == $sha and
                .draft == true and .prerelease == false and
                ([.assets[].name] | sort) == $names and ([.assets[].size > 0] | all) and
                ([.assets[].digest | type == "string" and test("^sha256:[0-9a-f]{64}$")] | all)
            """)
            public_final_inventory_query = normalized_query("""
                .target_commitish == $sha and .draft == false and .prerelease == false and
                ([.assets[].name] | sort) == $names and ([.assets[].size > 0] | all) and
                ([.assets[].digest | type == "string" and test("^sha256:[0-9a-f]{64}$")] | all)
            """)
            validated_subset_query = normalized_query("""
                ($manifest[0].release_assets | map(.name)) as $validated_names |
                ([.assets[] | select(.name as $name | $validated_names | index($name)) |
                  {id, name, size, digest}] | sort_by(.name)) ==
                  ($manifest[0].release_assets | sort_by(.name))
            """)

            if normalized == ".total_count":
                emit(data.get("total_count"))
            elif normalized == ".artifacts | length":
                emit(len(data.get("artifacts", [])))
            elif normalized == native_proof_inventory_query:
                records = [
                    record for record in data.get("artifacts", [])
                    if record.get("name") == variables.get("name")
                ]
                if len(records) != 1:
                    raise SystemExit(5)
                record = records[0]
                emit_tsv((
                    record.get("id"), record.get("name"), record.get("digest"),
                    record.get("expired"), record.get("size_in_bytes"),
                    nested(record, "workflow_run", "id"),
                    nested(record, "workflow_run", "repository_id"),
                    nested(record, "workflow_run", "head_repository_id"),
                    nested(record, "workflow_run", "head_sha"),
                ))
            elif normalized == direct_artifact_query:
                emit_tsv((
                    data.get("id"), data.get("name"), data.get("digest"),
                    data.get("expired"), data.get("size_in_bytes"),
                    nested(data, "workflow_run", "id"),
                    nested(data, "workflow_run", "repository_id"),
                    nested(data, "workflow_run", "head_repository_id"),
                    nested(data, "workflow_run", "head_sha"),
                ))
            elif normalized in artifact_validator_queries:
                expected_name, size_limit, require_sha = artifact_validator_queries[normalized]
                size = data.get("size_in_bytes")
                condition = (
                    data.get("id") == variables.get("id")
                    and data.get("name") == expected_name
                    and data.get("expired") is False
                    and isinstance(size, int) and 0 < size <= size_limit
                    and data.get("digest") == variables.get("digest")
                    and nested(data, "workflow_run", "id") == variables.get("run")
                    and nested(data, "workflow_run", "repository_id") == variables.get("repository")
                    and nested(data, "workflow_run", "head_repository_id") == variables.get("repository")
                )
                if require_sha:
                    condition = condition and nested(data, "workflow_run", "head_sha") == variables.get("sha")
                require(condition)
            elif normalized == native_proof_query:
                expected = {
                    "schema_version": 1,
                    "tag": variables.get("tag"),
                    "source_sha": variables.get("sha"),
                    "workflow_run_id": variables.get("run"),
                    "workflow_run_attempt": variables.get("attempt"),
                    "architecture": variables.get("arch"),
                    "build_artifact_id": variables.get("build_id"),
                    "build_artifact_digest": variables.get("build_digest"),
                    "managed_installer": {
                        "digest": variables.get("managed_digest"),
                        "size": variables.get("managed_size"),
                    },
                    "dist_installer": {
                        "digest": variables.get("dist_digest"),
                        "size": variables.get("dist_size"),
                    },
                    "aarch64_archive": {
                        "digest": variables.get("arm_digest"),
                        "size": variables.get("arm_size"),
                    },
                    "x86_64_archive": {
                        "digest": variables.get("intel_digest"),
                        "size": variables.get("intel_size"),
                    },
                }
                require(data == expected)
            elif normalized in workflow_run_queries:
                expected_name, expected_path, expected_event = workflow_run_queries[normalized]
                for run in data.get("workflow_runs", []):
                    if (
                        run.get("workflow_id") == variables.get("workflow_id")
                        and run.get("name") == expected_name
                        and run.get("path") == expected_path
                        and run.get("event") == expected_event
                        and run.get("status") == "completed"
                        and run.get("conclusion") == "success"
                        and nested(run, "repository", "full_name") == variables.get("repository")
                        and nested(run, "head_repository", "full_name") == variables.get("repository")
                        and run.get("head_branch") == "main"
                        and run.get("head_sha") == variables.get("sha")
                    ):
                        emit(run.get("id"))
            elif normalized == windows_artifact_inventory_query:
                for record in data.get("artifacts", []):
                    if record.get("name") == "windows-release-assets" and record.get("expired") is False:
                        emit_tsv((record.get("id"), record.get("digest")))
            elif normalized == validation_artifact_inventory_query:
                for record in data.get("artifacts", []):
                    if record.get("name") == "windows-installer-validation-proof" and record.get("expired") is False:
                        emit_tsv((record.get("id"), record.get("digest")))
            elif normalized == ".validation_input_artifact_id":
                emit(data.get("validation_input_artifact_id"))
            elif normalized == ".validation_input_artifact_digest":
                emit(data.get("validation_input_artifact_digest"))
            elif normalized == validation_proof_query:
                require(
                    data.get("schema_version") == 1
                    and data.get("result") == "success"
                    and data.get("tag") == variables.get("tag")
                    and data.get("source_sha") == variables.get("sha")
                    and data.get("windows_run_id") == variables.get("windows_run")
                    and data.get("windows_run_attempt") == variables.get("windows_attempt")
                    and data.get("windows_artifact_id") == variables.get("windows_artifact")
                    and data.get("windows_artifact_digest") == variables.get("windows_digest")
                    and data.get("validation_run_id") == variables.get("validation_run")
                    and data.get("validation_run_attempt") == variables.get("validation_attempt")
                    and isinstance(data.get("validation_input_artifact_id"), int)
                    and re.fullmatch(r"sha256:[0-9a-f]{64}", str(data.get("validation_input_artifact_digest"))) is not None
                )
            elif normalized == validation_manifest_query:
                records = data.get("release_assets", [])
                require(
                    data.get("schema_version") == 1
                    and data.get("validation_mode") == "private"
                    and data.get("tag") == variables.get("tag")
                    and data.get("source_sha") == variables.get("sha")
                    and data.get("upstream_run_id") == variables.get("windows_run")
                    and data.get("upstream_run_attempt") == variables.get("windows_attempt")
                    and data.get("windows_artifact_id") == variables.get("windows_artifact")
                    and data.get("windows_artifact_digest") == variables.get("windows_digest")
                    and len(records) == 30
                    and bound_release_asset(records, "tr300-installer.sh", variables.get("managed_digest"), variables.get("managed_size"))
                    and bound_release_asset(records, "tr300-dist-installer.sh", variables.get("dist_digest"), variables.get("dist_size"))
                    and bound_release_asset(records, "tr300-aarch64-apple-darwin.tar.xz", variables.get("arm_digest"), variables.get("arm_size"))
                    and bound_release_asset(records, "tr300-x86_64-apple-darwin.tar.xz", variables.get("intel_digest"), variables.get("intel_size"))
                    and all(
                        isinstance(record.get("id"), int)
                        and isinstance(record.get("size"), int) and record.get("size") > 0
                        and re.fullmatch(r"sha256:[0-9a-f]{64}", str(record.get("digest"))) is not None
                        for record in records
                    )
                )
            elif normalized == ".release_assets[] | [.name, .digest, .size] | @tsv":
                for record in data.get("release_assets", []):
                    emit_tsv((record.get("name"), record.get("digest"), record.get("size")))
            elif normalized in (validated_draft_query, release_manifest_query):
                assets = data.get("assets", [])
                manifest_assets = variables["manifest"][0].get("release_assets", [])
                condition = (
                    data.get("id") == variables.get("release_id")
                    and data.get("tag_name") == variables.get("tag")
                    and data.get("target_commitish") == variables.get("sha")
                    and data.get("draft") is True
                    and data.get("prerelease") is False
                    and artifact_projection(assets) == artifact_projection(manifest_assets)
                )
                if normalized == validated_draft_query:
                    condition = condition and all((
                        bound_release_asset(assets, "tr300-installer.sh", variables.get("managed_digest"), variables.get("managed_size")),
                        bound_release_asset(assets, "tr300-dist-installer.sh", variables.get("dist_digest"), variables.get("dist_size")),
                        bound_release_asset(assets, "tr300-aarch64-apple-darwin.tar.xz", variables.get("arm_digest"), variables.get("arm_size")),
                        bound_release_asset(assets, "tr300-x86_64-apple-darwin.tar.xz", variables.get("intel_digest"), variables.get("intel_size")),
                    ))
                require(condition)
            elif normalized in (draft_final_inventory_query, public_final_inventory_query):
                assets = data.get("assets", [])
                expected_draft = normalized == draft_final_inventory_query
                require(
                    (not expected_draft or data.get("id") == variables.get("release_id"))
                    and (not expected_draft or data.get("tag_name") == variables.get("tag"))
                    and data.get("target_commitish") == variables.get("sha")
                    and data.get("draft") is expected_draft
                    and data.get("prerelease") is False
                    and sorted(record.get("name") for record in assets) == variables.get("names")
                    and all(isinstance(record.get("size"), int) and record.get("size") > 0 for record in assets)
                    and all(re.fullmatch(r"sha256:[0-9a-f]{64}", str(record.get("digest"))) is not None for record in assets)
                )
            elif normalized == validated_subset_query:
                assets = data.get("assets", [])
                manifest_assets = variables["manifest"][0].get("release_assets", [])
                validated_names = {record.get("name") for record in manifest_assets}
                selected = [record for record in assets if record.get("name") in validated_names]
                require(artifact_projection(selected) == artifact_projection(manifest_assets))
            elif normalized == ".assets[].name":
                for record in data.get("assets", []):
                    emit(record.get("name"))
            elif normalized == ".assets[] | [.name, .digest, .size] | @tsv":
                for record in data.get("assets", []):
                    emit_tsv((record.get("name"), record.get("digest"), record.get("size")))
            elif normalized == ".id":
                emit(data.get("id"))
            else:
                print(f"publisher jq fixture rejects unknown filter: {query!r}", file=sys.stderr)
                raise SystemExit(3)
            '''
        ).lstrip(),
        encoding="utf-8",
        newline="\n",
    )
    mock.chmod(mock.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def write_macos_publisher_gh(bin_dir: Path) -> None:
    mock = bin_dir / "gh"
    mock.write_text(
        textwrap.dedent(
            r'''
            #!/usr/bin/env python3
            import copy
            import hashlib
            import json
            import os
            import shutil
            import sys
            from pathlib import Path

            sys.stdout.reconfigure(newline="\n")
            state = Path(os.environ["MOCK_MACOS_PUBLISHER_STATE"])
            config_path = state / "config.json"
            release_path = state / "release.json"
            log_path = state / "gh-calls.jsonl"
            arguments = sys.argv[1:]
            with log_path.open("a", encoding="utf-8", newline="\n") as log:
                log.write(json.dumps(arguments, separators=(",", ":")) + "\n")
            config = json.loads(config_path.read_text(encoding="utf-8"))

            def write_json(value):
                sys.stdout.write(json.dumps(value, separators=(",", ":")) + "\n")

            def load_release():
                return json.loads(release_path.read_text(encoding="utf-8"))

            def save_release(value):
                release_path.write_text(
                    json.dumps(value, separators=(",", ":")),
                    encoding="utf-8",
                    newline="\n",
                )

            def artifact_bytes(artifact_id):
                path = config["artifact_zips"].get(str(artifact_id))
                if path is None:
                    raise SystemExit(89)
                sys.stdout.buffer.write(Path(path).read_bytes())

            def direct_artifact(artifact_id):
                record = copy.deepcopy(config["artifacts"].get(str(artifact_id)))
                if record is None:
                    raise SystemExit(89)
                counter_path = state / f"artifact-{artifact_id}-reads"
                count = int(counter_path.read_text(encoding="utf-8")) if counter_path.exists() else 0
                counter_path.write_text(str(count + 1), encoding="utf-8", newline="\n")
                if (
                    config.get("mutation") == "macos-artifact-replacement"
                    and artifact_id == config["macos_artifact_id"]
                    and count >= 1
                ):
                    record["digest"] = "sha256:" + "0" * 64
                write_json(record)

            if arguments[:2] == ["release", "upload"]:
                tag = arguments[2]
                if tag != config["tag"] or arguments[3:5] != ["--repo", config["repository"]]:
                    raise SystemExit(45)
                sources = [Path(value) for value in arguments[5:]]
                if [source.name for source in sources] != config["macos_asset_names"]:
                    raise SystemExit(46)
                release = load_release()
                existing = {record["name"] for record in release["assets"]}
                if any(source.name in existing for source in sources):
                    raise SystemExit(46)
                next_id = max(record["id"] for record in release["assets"]) + 1
                release_directory = state / "release-assets"
                for offset, source in enumerate(sources):
                    destination = release_directory / source.name
                    shutil.copyfile(source, destination)
                    content = destination.read_bytes()
                    release["assets"].append({
                        "id": next_id + offset,
                        "name": source.name,
                        "size": len(content),
                        "digest": "sha256:" + hashlib.sha256(content).hexdigest(),
                    })
                save_release(release)
                raise SystemExit(0)

            if arguments[:2] == ["release", "download"]:
                tag = arguments[2]
                if tag != config["tag"]:
                    raise SystemExit(45)
                destination = None
                patterns = []
                index = 3
                while index < len(arguments):
                    if arguments[index] == "--repo":
                        if arguments[index + 1] != config["repository"]:
                            raise SystemExit(45)
                        index += 2
                    elif arguments[index] == "--dir":
                        destination = Path(arguments[index + 1])
                        index += 2
                    elif arguments[index] == "--pattern":
                        patterns.append(arguments[index + 1])
                        index += 2
                    else:
                        raise SystemExit(45)
                if destination is None:
                    raise SystemExit(45)
                release = load_release()
                selected = [
                    record for record in release["assets"]
                    if not patterns or record["name"] in patterns
                ]
                if patterns and {record["name"] for record in selected} != set(patterns):
                    raise SystemExit(45)
                destination.mkdir(parents=True, exist_ok=True)
                for record in selected:
                    shutil.copyfile(
                        state / "release-assets" / record["name"],
                        destination / record["name"],
                    )
                raise SystemExit(0)

            if not arguments or arguments[0] != "api":
                raise SystemExit(97)
            index = 1
            method = "GET"
            if arguments[index:index + 2] == ["--method", "PATCH"]:
                method = "PATCH"
                index += 2
            endpoint = arguments[index]
            options = arguments[index + 1:]
            repository_prefix = f"repos/{config['repository']}"

            if method == "PATCH":
                if endpoint != f"{repository_prefix}/releases/{config['release_id']}":
                    raise SystemExit(98)
                if "draft=false" not in options or "make_latest=true" not in options:
                    raise SystemExit(98)
                release = load_release()
                release["draft"] = False
                save_release(release)
                write_json(release)
                raise SystemExit(0)

            if endpoint == repository_prefix:
                print(config["repository_id"])
            elif endpoint == f"{repository_prefix}/git/ref/tags/{config['tag']}":
                print(f"commit\t{config['sha']}")
            elif endpoint == f"{repository_prefix}/git/ref/heads/main":
                print(config["sha"])
            elif endpoint.startswith(f"{repository_prefix}/actions/workflows/") and "/runs?" not in endpoint:
                workflow = endpoint.rsplit("/", 1)[-1]
                print(config["workflow_ids"][workflow])
            elif endpoint.startswith(f"{repository_prefix}/actions/workflows/") and "/runs?" in endpoint:
                workflow = endpoint.split("/actions/workflows/", 1)[1].split("/runs?", 1)[0]
                write_json({"workflow_runs": config["workflow_runs"][workflow]})
            elif endpoint.startswith(f"{repository_prefix}/actions/runs/") and endpoint.endswith("/artifacts?per_page=100"):
                run_id = endpoint.split("/actions/runs/", 1)[1].split("/", 1)[0]
                write_json(config["run_artifacts"][run_id])
            elif endpoint.startswith(f"{repository_prefix}/actions/runs/"):
                run_id = endpoint.rsplit("/", 1)[-1]
                run = config["runs"].get(run_id)
                if run is None:
                    raise SystemExit(89)
                if options[-2:] == ["--jq", ".event"]:
                    print(run["event"])
                else:
                    fields = (
                        run["id"], run["workflow_id"], run["name"], run["path"],
                        run["event"], run["status"], run["conclusion"],
                        run["repository"]["full_name"], run["head_repository"]["full_name"],
                        run["head_branch"], run["head_sha"], run["run_attempt"],
                    )
                    print("\t".join(str(value) for value in fields))
            elif endpoint.startswith(f"{repository_prefix}/actions/artifacts/"):
                artifact_tail = endpoint.split("/actions/artifacts/", 1)[1]
                if artifact_tail.endswith("/zip"):
                    artifact_bytes(int(artifact_tail.removesuffix("/zip")))
                else:
                    direct_artifact(int(artifact_tail))
            elif endpoint == f"{repository_prefix}/releases/{config['release_id']}":
                write_json(load_release())
            elif endpoint == f"{repository_prefix}/releases/tags/{config['tag']}":
                write_json(load_release())
            else:
                print(f"unexpected publisher gh endpoint: {endpoint}", file=sys.stderr)
                raise SystemExit(98)
            '''
        ).lstrip(),
        encoding="utf-8",
        newline="\n",
    )
    mock.chmod(mock.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def publisher_run_record(
    *, run_id: int, workflow_id: int, name: str, path: str, event: str
) -> dict[str, object]:
    return {
        "id": run_id,
        "workflow_id": workflow_id,
        "name": name,
        "path": path,
        "event": event,
        "status": "completed",
        "conclusion": "success",
        "repository": {"full_name": REPOSITORY},
        "head_repository": {"full_name": REPOSITORY},
        "head_branch": "main",
        "head_sha": TRUSTED_SHA,
        "run_attempt": 1,
    }


def write_macos_publisher_fixture(
    directory: Path, mutation: str | None
) -> tuple[dict[str, str], Path, dict[str, bytes]]:
    state = directory / "publisher-state"
    state.mkdir()
    fixture_bin = directory / "publisher-bin"
    fixture_bin.mkdir()
    write_macos_publisher_jq(fixture_bin)
    write_macos_publisher_gh(fixture_bin)
    write_windows_mkdir_compat(fixture_bin)
    write_windows_shasum_compat(fixture_bin)

    current_run_id = 41001
    windows_run_id = 41002
    validation_run_id = 41003
    ci_run_id = 41004
    macos_artifact_id = 51001
    arm_proof_artifact_id = 51002
    intel_proof_artifact_id = 51003
    windows_artifact_id = 61001
    validation_proof_artifact_id = 71001
    validation_input_artifact_id = 71002
    release_id = 81001

    initial_payloads = initial_release_asset_payloads()
    windows_payloads = payloads_with_sidecars(
        WINDOWS_RELEASE_PAYLOADS, "Windows supplement"
    )
    macos_payloads = payloads_with_sidecars(MACOS_RELEASE_PAYLOADS, "macOS")
    release_payloads = {**initial_payloads, **windows_payloads}
    if mutation == "destination-collision":
        del release_payloads["tr-300-installer.ps1"]
        release_payloads[MACOS_RELEASE_ASSETS[0]] = b"pre-existing macOS destination\n"

    release_directory = state / "release-assets"
    write_payload_directory(release_directory, release_payloads)
    release_records = release_asset_records(release_payloads, first_id=91000)
    release = {
        "id": release_id,
        "tag_name": "v4.2.2" if mutation == "release-tag-mismatch" else "v4.3.0",
        "target_commitish": TRUSTED_SHA,
        "draft": True,
        "prerelease": False,
        "assets": release_records,
    }
    (state / "release.json").write_text(
        json.dumps(release, separators=(",", ":")),
        encoding="utf-8",
        newline="\n",
    )

    macos_zip = state / "macos-assets.zip"
    macos_zip_digest = write_payload_zip(macos_zip, macos_payloads)
    macos_artifact = artifact_record(
        artifact_id=macos_artifact_id,
        name="tr300-universal-macos-installer",
        digest=macos_zip_digest,
        size=macos_zip.stat().st_size,
        run_id=current_run_id,
    )

    source_assets = {
        "managed": release_payloads["tr300-installer.sh"],
        "dist": release_payloads["tr300-dist-installer.sh"],
        "arm": release_payloads["tr300-aarch64-apple-darwin.tar.xz"],
        "intel": release_payloads["tr300-x86_64-apple-darwin.tar.xz"],
    }
    source_digests = {
        name: hashlib.sha256(content).hexdigest()
        for name, content in source_assets.items()
    }
    source_sizes = {name: len(content) for name, content in source_assets.items()}

    proof_artifacts: list[dict[str, object]] = []
    artifact_zips: dict[str, str] = {str(macos_artifact_id): macos_zip.as_posix()}
    artifacts: dict[str, dict[str, object]] = {
        str(macos_artifact_id): macos_artifact
    }
    for arch, proof_id in (
        ("arm64", arm_proof_artifact_id),
        ("x86_64", intel_proof_artifact_id),
    ):
        proof = {
            "schema_version": 1,
            "tag": "v4.3.0",
            "source_sha": TRUSTED_SHA,
            "workflow_run_id": current_run_id,
            "workflow_run_attempt": (
                2 if mutation == "stale-native-attempt" and arch == "arm64" else 1
            ),
            "architecture": arch,
            "build_artifact_id": macos_artifact_id,
            "build_artifact_digest": f"sha256:{macos_zip_digest}",
            "managed_installer": {
                "digest": f"sha256:{source_digests['managed']}",
                "size": source_sizes["managed"],
            },
            "dist_installer": {
                "digest": f"sha256:{source_digests['dist']}",
                "size": source_sizes["dist"],
            },
            "aarch64_archive": {
                "digest": f"sha256:{source_digests['arm']}",
                "size": source_sizes["arm"],
            },
            "x86_64_archive": {
                "digest": f"sha256:{source_digests['intel']}",
                "size": source_sizes["intel"],
            },
        }
        proof_name = f"tr300-macos-native-validation-{arch}.json"
        proof_zip = state / f"{arch}-proof.zip"
        proof_zip_digest = write_payload_zip(
            proof_zip,
            {
                proof_name: (
                    json.dumps(proof, separators=(",", ":")) + "\n"
                ).encode()
            },
        )
        proof_artifact = artifact_record(
            artifact_id=proof_id,
            name=f"tr300-macos-native-validation-{arch}-1",
            digest=proof_zip_digest,
            size=proof_zip.stat().st_size,
            run_id=current_run_id,
        )
        proof_artifacts.append(proof_artifact)
        artifacts[str(proof_id)] = proof_artifact
        artifact_zips[str(proof_id)] = proof_zip.as_posix()

    windows_zip = state / "windows-assets.zip"
    windows_zip_digest = write_payload_zip(windows_zip, windows_payloads)
    windows_artifact = artifact_record(
        artifact_id=windows_artifact_id,
        name="windows-release-assets",
        digest=windows_zip_digest,
        size=windows_zip.stat().st_size,
        run_id=windows_run_id,
    )
    artifacts[str(windows_artifact_id)] = windows_artifact
    artifact_zips[str(windows_artifact_id)] = windows_zip.as_posix()

    validation_input_content = b"fixture exact Windows validation inputs\n"
    validation_input_digest = hashlib.sha256(validation_input_content).hexdigest()
    validation_input_artifact = artifact_record(
        artifact_id=validation_input_artifact_id,
        name="windows-validation-inputs",
        digest=validation_input_digest,
        size=len(validation_input_content),
        run_id=validation_run_id,
    )
    artifacts[str(validation_input_artifact_id)] = validation_input_artifact

    validation_manifest_records = json.loads(json.dumps(release_records))
    if mutation == "manifest-draft-divergence":
        source_record = next(
            record
            for record in validation_manifest_records
            if record["name"] == "source.tar.gz"
        )
        source_record["digest"] = f"sha256:{'f' * 64}"
    validation_manifest = {
        "schema_version": 1,
        "validation_mode": "private",
        "tag": "v4.3.0",
        "source_sha": TRUSTED_SHA,
        "upstream_run_id": windows_run_id,
        "upstream_run_attempt": 1,
        "windows_artifact_id": windows_artifact_id,
        "windows_artifact_digest": f"sha256:{windows_zip_digest}",
        "release_assets": validation_manifest_records,
    }
    validation_proof = {
        "schema_version": 1,
        "result": "success",
        "tag": "v4.3.0",
        "source_sha": TRUSTED_SHA,
        "windows_run_id": windows_run_id,
        "windows_run_attempt": 1,
        "windows_artifact_id": windows_artifact_id,
        "windows_artifact_digest": f"sha256:{windows_zip_digest}",
        "validation_run_id": validation_run_id,
        "validation_run_attempt": 1,
        "validation_input_artifact_id": validation_input_artifact_id,
        "validation_input_artifact_digest": f"sha256:{validation_input_digest}",
    }
    validation_proof_zip = state / "windows-validation-proof.zip"
    validation_proof_zip_digest = write_payload_zip(
        validation_proof_zip,
        {
            "windows-installer-validation.json": (
                json.dumps(validation_proof, separators=(",", ":")) + "\n"
            ).encode(),
            "validation-input-manifest.json": (
                json.dumps(validation_manifest, separators=(",", ":")) + "\n"
            ).encode(),
        },
    )
    validation_proof_artifact = artifact_record(
        artifact_id=validation_proof_artifact_id,
        name="windows-installer-validation-proof",
        digest=validation_proof_zip_digest,
        size=validation_proof_zip.stat().st_size,
        run_id=validation_run_id,
    )
    artifacts[str(validation_proof_artifact_id)] = validation_proof_artifact
    artifact_zips[str(validation_proof_artifact_id)] = (
        validation_proof_zip.as_posix()
    )

    windows_run = publisher_run_record(
        run_id=windows_run_id,
        workflow_id=int(WINDOWS_WORKFLOW_ID),
        name="Windows Installers",
        path=".github/workflows/windows-installers.yml",
        event="workflow_run",
    )
    validation_run = publisher_run_record(
        run_id=validation_run_id,
        workflow_id=int(WINDOWS_VALIDATION_WORKFLOW_ID),
        name="Windows Installer Validation",
        path=".github/workflows/windows-installer-validation.yml",
        event="workflow_run",
    )
    ci_run = publisher_run_record(
        run_id=ci_run_id,
        workflow_id=333333,
        name="CI",
        path=".github/workflows/ci.yml",
        event="push",
    )

    config = {
        "mutation": mutation,
        "repository": REPOSITORY,
        "repository_id": int(REPOSITORY_ID),
        "tag": "v4.3.0",
        "sha": TRUSTED_SHA,
        "release_id": release_id,
        "macos_artifact_id": macos_artifact_id,
        "macos_asset_names": list(MACOS_RELEASE_ASSETS),
        "workflow_ids": {
            "ci.yml": 333333,
            "windows-installers.yml": int(WINDOWS_WORKFLOW_ID),
            "windows-installer-validation.yml": int(
                WINDOWS_VALIDATION_WORKFLOW_ID
            ),
        },
        "workflow_runs": {
            "ci.yml": [ci_run],
            "windows-installers.yml": [windows_run],
            "windows-installer-validation.yml": [validation_run],
        },
        "runs": {
            str(windows_run_id): windows_run,
            str(validation_run_id): validation_run,
        },
        "run_artifacts": {
            str(current_run_id): {
                "total_count": 3,
                "artifacts": [macos_artifact, *proof_artifacts],
            },
            str(windows_run_id): {
                "total_count": 1,
                "artifacts": [windows_artifact],
            },
            str(validation_run_id): {
                "total_count": 2,
                "artifacts": [validation_proof_artifact, validation_input_artifact],
            },
        },
        "artifacts": artifacts,
        "artifact_zips": artifact_zips,
    }
    (state / "config.json").write_text(
        json.dumps(config, separators=(",", ":")),
        encoding="utf-8",
        newline="\n",
    )

    environment = {
        "MOCK_MACOS_PUBLISHER_STATE": state.as_posix(),
        "PUBLISHER_FIXTURE_BIN": fixture_bin.as_posix(),
        "RELEASE_TAG": "v4.3.0",
        "EXPECTED_SHA": TRUSTED_SHA,
        "RELEASE_ID": str(release_id),
        "REPOSITORY": REPOSITORY,
        "ASSET_DIRECTORY": (directory / "macos-release-assets").as_posix(),
        "CURRENT_RUN_ID": str(current_run_id),
        "CURRENT_RUN_ATTEMPT": "1",
        "MACOS_ARTIFACT_ID": str(macos_artifact_id),
        "MACOS_ARTIFACT_DIGEST": macos_zip_digest,
        "SOURCE_MANAGED_INSTALLER_SHA256": source_digests["managed"],
        "SOURCE_MANAGED_INSTALLER_SIZE": str(source_sizes["managed"]),
        "SOURCE_DIST_INSTALLER_SHA256": source_digests["dist"],
        "SOURCE_DIST_INSTALLER_SIZE": str(source_sizes["dist"]),
        "SOURCE_AARCH64_ARCHIVE_SHA256": source_digests["arm"],
        "SOURCE_AARCH64_ARCHIVE_SIZE": str(source_sizes["arm"]),
        "SOURCE_X86_64_ARCHIVE_SHA256": source_digests["intel"],
        "SOURCE_X86_64_ARCHIVE_SIZE": str(source_sizes["intel"]),
        "EVENT_NAME": "workflow_run",
        "DISPATCH_WINDOWS_RUN_ID": "",
        "DISPATCH_WINDOWS_VALIDATION_RUN_ID": "",
        "WORKFLOW_SHA": TRUSTED_SHA,
        "RUNNER_TEMP": directory.as_posix(),
    }
    return environment, state, macos_payloads


def publisher_gh_mutation(arguments: list[str]) -> str | None:
    release_mutations = {"create", "delete", "edit", "upload"}
    if "release" in arguments and any(
        operation in arguments for operation in release_mutations
    ):
        operation = next(
            operation for operation in release_mutations if operation in arguments
        )
        return f"release {operation}"

    if "api" not in arguments:
        return None
    methods: list[str] = []
    has_fields = False
    for index, argument in enumerate(arguments):
        if argument in ("--method", "-X"):
            if index + 1 < len(arguments):
                methods.append(arguments[index + 1].upper())
        elif argument.startswith("--method="):
            methods.append(argument.split("=", 1)[1].upper())
        elif argument.startswith("-X") and len(argument) > 2:
            methods.append(argument[2:].upper())
        elif argument in ("-f", "-F", "--field", "--raw-field", "--input"):
            has_fields = True
        elif argument.startswith(("--field=", "--raw-field=", "--input=")):
            has_fields = True
    if not methods:
        methods.append("POST" if has_fields else "GET")
    mutating = [
        method for method in methods if method in {"POST", "PUT", "PATCH", "DELETE"}
    ]
    return f"api {mutating[0]}" if mutating else None


def publisher_gh_read_allowed(arguments: list[str], config: dict[str, object]) -> bool:
    repository = str(config["repository"])
    repository_prefix = f"repos/{repository}"
    tag = str(config["tag"])
    sha = str(config["sha"])

    if arguments[:2] == ["release", "download"]:
        if len(arguments) < 7 or arguments[2] != tag:
            return False
        repository_value = None
        destination = None
        patterns: list[str] = []
        index = 3
        while index < len(arguments):
            option = arguments[index]
            if option == "--repo" and index + 1 < len(arguments):
                repository_value = arguments[index + 1]
                index += 2
            elif option == "--dir" and index + 1 < len(arguments):
                destination = arguments[index + 1]
                index += 2
            elif option == "--pattern" and index + 1 < len(arguments):
                patterns.append(arguments[index + 1])
                index += 2
            else:
                return False
        return (
            repository_value == repository
            and bool(destination)
            and (
                not patterns
                or (
                    len(patterns) == len(WINDOWS_RELEASE_ASSETS)
                    and set(patterns) == set(WINDOWS_RELEASE_ASSETS)
                )
            )
        )

    if not arguments or arguments[0] != "api" or len(arguments) < 2:
        return False
    if publisher_gh_mutation(arguments) is not None:
        return False
    endpoint = arguments[1]
    options = arguments[2:]
    run_projection = (
        "[.id, .workflow_id, .name, .path, .event, .status, .conclusion, "
        ".repository.full_name, .head_repository.full_name, .head_branch, "
        ".head_sha, .run_attempt] | @tsv"
    )

    exact_endpoints: dict[str, list[str]] = {
        repository_prefix: ["--jq", ".id"],
        f"{repository_prefix}/git/ref/tags/{tag}": [
            "--jq",
            ".object | [.type, .sha] | @tsv",
        ],
        f"{repository_prefix}/git/ref/heads/main": ["--jq", ".object.sha"],
        f"{repository_prefix}/releases/tags/{tag}": [],
        f"{repository_prefix}/releases/{config['release_id']}": [],
    }
    for workflow in config["workflow_ids"]:
        exact_endpoints[f"{repository_prefix}/actions/workflows/{workflow}"] = [
            "--jq",
            ".id",
        ]
    exact_endpoints.update(
        {
            f"{repository_prefix}/actions/workflows/windows-installers.yml/runs?head_sha={sha}&status=success&per_page=100": [],
            f"{repository_prefix}/actions/workflows/windows-installer-validation.yml/runs?head_sha={sha}&status=success&per_page=100": [],
            f"{repository_prefix}/actions/workflows/ci.yml/runs?event=push&head_sha={sha}&per_page=100": [],
        }
    )
    for run_id in config["runs"]:
        exact_endpoints[f"{repository_prefix}/actions/runs/{run_id}"] = [
            "--jq",
            run_projection,
        ]
    for run_id in config["run_artifacts"]:
        exact_endpoints[
            f"{repository_prefix}/actions/runs/{run_id}/artifacts?per_page=100"
        ] = []
    for artifact_id in config["artifacts"]:
        exact_endpoints[f"{repository_prefix}/actions/artifacts/{artifact_id}"] = []
    for artifact_id in config["artifact_zips"]:
        exact_endpoints[f"{repository_prefix}/actions/artifacts/{artifact_id}/zip"] = []
    return endpoint in exact_endpoints and options == exact_endpoints[endpoint]


def validate_publisher_gh_calls(
    *,
    calls: list[list[str]],
    config: dict[str, object],
    expected_mutations: list[list[str]],
    name: str,
) -> None:
    mutations: list[list[str]] = []
    unknown: list[list[str]] = []
    for call in calls:
        if publisher_gh_mutation(call) is not None:
            mutations.append(call)
        elif not publisher_gh_read_allowed(call, config):
            unknown.append(call)
    if unknown:
        raise AssertionError(f"{name}: non-allowlisted gh invocation(s): {unknown!r}")
    if mutations != expected_mutations:
        described_mutations = [
            (publisher_gh_mutation(call), call) for call in mutations
        ]
        raise AssertionError(
            f"{name}: mutating gh invocation(s) {described_mutations!r} != "
            f"{expected_mutations!r}"
        )


def run_macos_publisher_case(
    *,
    bash: str,
    mock_bin: Path,
    block: str,
    name: str,
    mutation: str | None,
    expected_success: bool,
) -> None:
    with tempfile.TemporaryDirectory(prefix="tr300-macos-publisher-") as case_dir_raw:
        case_dir = Path(case_dir_raw)
        output = case_dir / "github-output"
        log = case_dir / "gh-calls"
        environment = fixture_environment(mock_bin, output, log)
        fixture_environment_values, state, macos_payloads = (
            write_macos_publisher_fixture(case_dir, mutation)
        )
        fixture_bin = fixture_environment_values.pop("PUBLISHER_FIXTURE_BIN")
        environment.update(fixture_environment_values)
        environment["PATH"] = fixture_bin + os.pathsep + environment["PATH"]
        execution_block = block
        if os.name == "nt":
            if "mkdir -m 700 " not in execution_block:
                raise AssertionError(f"{name}: hosted private-directory guard changed")
            execution_block = execution_block.replace("mkdir -m 700 ", "mkdir ")
        script = case_dir / "workflow-run.sh"
        script.write_text(execution_block, encoding="utf-8", newline="\n")
        bash_arguments = [bash]
        if os.environ.get("TR300_TEST_XTRACE") == "1":
            bash_arguments.append("-x")
        bash_arguments.append(str(script))
        result = subprocess.run(
            bash_arguments,
            cwd=ROOT,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            # The full publisher performs dozens of fixed archive/hash checks.
            # Git Bash process startup on Windows can legitimately exceed the
            # former 45-second fixture budget without reaching a polling wait.
            timeout=90,
            check=False,
        )
        succeeded = result.returncode == 0
        if succeeded != expected_success:
            raise AssertionError(
                f"{name}: return code {result.returncode}, expected_success="
                f"{expected_success}\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
            )

        publisher_log = state / "gh-calls.jsonl"
        calls = [
            json.loads(line)
            for line in publisher_log.read_text(encoding="utf-8").splitlines()
        ]
        config = json.loads(
            (state / "config.json").read_text(encoding="utf-8")
        )
        asset_directory = Path(environment["ASSET_DIRECTORY"])
        expected_upload = [
            "release",
            "upload",
            "v4.3.0",
            "--repo",
            REPOSITORY,
            *(f"{asset_directory.as_posix()}/{asset}" for asset in MACOS_RELEASE_ASSETS),
        ]
        expected_patch = [
            "api",
            "--method",
            "PATCH",
            f"repos/{REPOSITORY}/releases/{config['release_id']}",
            "-F",
            "draft=false",
            "-f",
            "make_latest=true",
        ]
        validate_publisher_gh_calls(
            calls=calls,
            config=config,
            expected_mutations=[expected_upload, expected_patch]
            if expected_success
            else [],
            name=name,
        )
        uploads = [call for call in calls if call[:2] == ["release", "upload"]]
        patches = [
            call
            for call in calls
            if call[:3] == ["api", "--method", "PATCH"]
        ]
        api_endpoints = [
            call[1]
            for call in calls
            if len(call) >= 2 and call[0] == "api" and call[1] != "--method"
        ]
        download_calls = [
            call for call in calls if call[:2] == ["release", "download"]
        ]
        if expected_success:
            if uploads != [expected_upload]:
                raise AssertionError(
                    f"{name}: upload calls {uploads!r} != {[expected_upload]!r}"
                )
            if patches != [expected_patch]:
                raise AssertionError(f"{name}: draft promotion call changed: {patches!r}")
            release = json.loads(
                (state / "release.json").read_text(encoding="utf-8")
            )
            if release.get("draft") is not False or len(release.get("assets", [])) != 34:
                raise AssertionError(f"{name}: final release state changed: {release!r}")
            release_names = {record["name"] for record in release["assets"]}
            expected_names = {
                *INITIAL_RELEASE_ASSETS,
                *WINDOWS_RELEASE_ASSETS,
                *MACOS_RELEASE_ASSETS,
            }
            if release_names != expected_names:
                raise AssertionError(f"{name}: final release inventory changed")
            for asset_name, expected_bytes in macos_payloads.items():
                actual = (state / "release-assets" / asset_name).read_bytes()
                if actual != expected_bytes:
                    raise AssertionError(f"{name}: uploaded bytes changed for {asset_name}")
        elif uploads or patches:
            raise AssertionError(
                f"{name}: rejected case reached release mutation: "
                f"uploads={uploads!r}, patches={patches!r}"
            )
        if mutation == "stale-native-attempt":
            if not any(endpoint.endswith("/actions/artifacts/51002/zip") for endpoint in api_endpoints):
                raise AssertionError(f"{name}: stale proof case did not reach proof validation")
            if any("actions/workflows/windows-installers.yml" in endpoint for endpoint in api_endpoints):
                raise AssertionError(f"{name}: stale proof escaped into Windows custody")
        elif mutation == "manifest-draft-divergence":
            if not any("/releases/81001" in endpoint for endpoint in api_endpoints):
                raise AssertionError(f"{name}: divergent manifest did not reach draft equality")
            if download_calls:
                raise AssertionError(f"{name}: divergent manifest reached draft byte download")
        elif mutation == "macos-artifact-replacement":
            macos_metadata_endpoint = (
                f"repos/{REPOSITORY}/actions/artifacts/{environment['MACOS_ARTIFACT_ID']}"
            )
            if api_endpoints.count(macos_metadata_endpoint) != 2:
                raise AssertionError(f"{name}: replacement did not reach the metadata reread")
        elif mutation == "destination-collision":
            if len(download_calls) != 1 or not any(
                "actions/workflows/ci.yml/runs?" in endpoint for endpoint in api_endpoints
            ):
                raise AssertionError(f"{name}: collision did not reach the frozen 30-asset inventory")


def check_macos_publisher_jq_literal_fidelity(publisher: str) -> None:
    marker = '.name == "Windows Installers"'
    marker_index = publisher.find(marker)
    if marker_index < 0 or publisher.find(marker, marker_index + 1) >= 0:
        raise AssertionError("macOS publisher jq string-literal marker changed")
    query_start = publisher.rfind("'", 0, marker_index)
    query_end = publisher.find("'", marker_index)
    if query_start < 0 or query_end < 0:
        raise AssertionError("macOS publisher Windows-run jq filter was not extractable")
    query = publisher[query_start + 1 : query_end]
    if not query.strip().startswith(".workflow_runs[] |") or not query.strip().endswith(
        ") | .id"
    ):
        raise AssertionError("macOS publisher Windows-run jq filter boundary changed")

    with tempfile.TemporaryDirectory(prefix="tr300-publisher-jq-fidelity-") as raw:
        directory = Path(raw)
        write_macos_publisher_jq(directory)
        input_path = directory / "windows-runs.json"
        input_path.write_text(
            json.dumps(
                {
                    "workflow_runs": [
                        publisher_run_record(
                            run_id=41002,
                            workflow_id=int(WINDOWS_WORKFLOW_ID),
                            name="Windows Installers",
                            path=".github/workflows/windows-installers.yml",
                            event="workflow_run",
                        )
                    ]
                },
                separators=(",", ":"),
            ),
            encoding="utf-8",
            newline="\n",
        )
        arguments = [
            sys.executable,
            str(directory / "jq"),
            "-r",
            "--argjson",
            "workflow_id",
            WINDOWS_WORKFLOW_ID,
            "--arg",
            "repository",
            REPOSITORY,
            "--arg",
            "sha",
            TRUSTED_SHA,
        ]
        accepted = subprocess.run(
            [*arguments, query, str(input_path)],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=10,
            check=False,
        )
        if accepted.returncode != 0 or accepted.stdout.strip() != "41002":
            raise AssertionError(
                "macOS publisher jq fixture rejected the exact production filter: "
                f"return code {accepted.returncode}, stdout={accepted.stdout!r}, "
                f"stderr={accepted.stderr!r}"
            )

        mutated_query = query.replace(marker, '.name == "Windows  Installers"', 1)
        rejected = subprocess.run(
            [*arguments, mutated_query, str(input_path)],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=10,
            check=False,
        )
        if (
            rejected.returncode != 3
            or rejected.stdout
            or "publisher jq fixture rejects unknown filter" not in rejected.stderr
        ):
            raise AssertionError(
                "macOS publisher jq fixture collapsed bytes inside a string literal: "
                f"return code {rejected.returncode}, stdout={rejected.stdout!r}, "
                f"stderr={rejected.stderr!r}"
            )


def check_actual_resolvers(
    release: str,
    windows: str,
    macos: str,
    crates: str,
    windows_validation: str,
    bash: str,
    mock_bin: Path,
) -> None:
    release_guard = extract_named_run(
        release, "Require a supported stable release tag", RELEASE_WORKFLOW.name
    )
    windows_resolver = extract_named_run(
        windows,
        "Require a stable repository tag bound to the release commit",
        WINDOWS_WORKFLOW.name,
    )
    windows_publisher = extract_named_run(
        windows,
        "Revalidate custody and publish the fixed six assets",
        WINDOWS_WORKFLOW.name,
    )
    macos_resolver = extract_named_run(
        macos,
        "Resolve a trusted source before Apple credential use",
        MACOS_WORKFLOW.name,
    )
    macos_source_custody = extract_named_run(
        macos,
        "Download by immutable artifact ID and normalize fixed Apple inputs",
        MACOS_WORKFLOW.name,
    )
    macos_native_validation = extract_named_run(
        macos,
        "Validate, install, and exercise the universal package",
        MACOS_WORKFLOW.name,
    )
    native_custody_marker = 'pkg="$artifact_directory/tr300-universal-apple-darwin.pkg"'
    native_custody_end = macos_native_validation.index(native_custody_marker) + len(
        native_custody_marker
    )
    macos_native_custody = macos_native_validation[:native_custody_end] + "\n"
    if os.name == "nt":
        macos_native_custody = macos_native_custody.replace("mkdir -m 700 ", "mkdir ")
    macos_publisher = extract_named_run(
        macos,
        "Bind exact supplements, upload macOS assets, and publish the draft",
        MACOS_WORKFLOW.name,
    )
    if macos_publisher.count("sleep 30") != 2:
        raise AssertionError("macOS publisher polling-delay contract changed")
    macos_publisher_fixture = macos_publisher.replace("sleep 30", "sleep 0")
    windows_validation_resolver = extract_named_run(
        windows_validation,
        "Resolve tag and require every Windows family",
        WINDOWS_VALIDATION_WORKFLOW.name,
    )
    crates_ci_waiter = extract_named_run(
        crates,
        "Require successful CI for this exact main commit",
        CRATES_WORKFLOW.name,
    )

    ci_fixture_directory = mock_bin.parent / "crates-ci-waiter"
    ci_fixture_directory.mkdir()
    ci_state = ci_fixture_directory / "state"
    ci_state.mkdir()
    ci_runs_before = ci_fixture_directory / "before.json"
    ci_runs_after = ci_fixture_directory / "after.json"
    ci_run_id = "444444"
    publish_run_id = "555555"

    def ci_run(status: str, conclusion: str | None) -> dict[str, object]:
        return {
            "id": int(ci_run_id),
            "workflow_id": 333333,
            "name": "CI",
            "path": ".github/workflows/ci.yml",
            "event": "push",
            "status": status,
            "conclusion": conclusion,
            "repository": {"full_name": REPOSITORY},
            "head_repository": {"full_name": REPOSITORY},
            "head_branch": "main",
            "head_sha": TRUSTED_SHA,
            "run_attempt": 1,
        }

    ci_runs_before.write_text(
        json.dumps({"workflow_runs": [ci_run("in_progress", None)]}),
        encoding="utf-8",
        newline="\n",
    )
    ci_runs_after.write_text(
        json.dumps({"workflow_runs": [ci_run("completed", "success")]}),
        encoding="utf-8",
        newline="\n",
    )
    fixture_waiter = crates_ci_waiter.replace("sleep 15", "sleep 0")
    if fixture_waiter == crates_ci_waiter:
        raise AssertionError("crates CI waiter fixture could not bound its retry delay")
    run_case(
        bash=bash,
        mock_bin=mock_bin,
        block=fixture_waiter,
        name="crates publisher waits for in-progress exact CI",
        overrides={
            "EVENT_NAME": "push",
            "SOURCE_REF": "refs/heads/main",
            "SOURCE_SHA": TRUSTED_SHA,
            "PUBLISH_WORKFLOW_RUN_ID": publish_run_id,
            "CI_STATE_PATH": (ci_fixture_directory / "ci-runs.json").as_posix(),
            "MOCK_CI_RUNS_JSON": ci_runs_before.as_posix(),
            "MOCK_CI_RUNS_AFTER": ci_runs_after.as_posix(),
            "MOCK_CI_STATE_DIR": ci_state.as_posix(),
            "MOCK_RELEASE_RUN_ID": ci_run_id,
            "MOCK_RELEASE_WORKFLOW_ID": "333333",
            "MOCK_RELEASE_RUN_NAME": "CI",
            "MOCK_RELEASE_RUN_PATH": ".github/workflows/ci.yml",
            "MOCK_RELEASE_RUN_EVENT": "push",
            "MOCK_RELEASE_RUN_STATUS": "completed",
            "MOCK_RELEASE_RUN_CONCLUSION": "success",
            "MOCK_RELEASE_RUN_TAG": "main",
            "MOCK_RELEASE_RUN_SHA": TRUSTED_SHA,
            "MOCK_RELEASE_RUN_ATTEMPT": "1",
        },
        expected_success=True,
        expected_outputs={
            "source_sha": TRUSTED_SHA,
            "workflow_run_id": publish_run_id,
            "ci_run_id": ci_run_id,
        },
    )

    for tag, succeeds in (
        ("v4.3.0", True),
        ("v4.3.0-rc.1", False),
        ("v4.3.0+build.1", False),
        ("4.3.0", False),
        ("v04.3.0", False),
        ("v4.3.0$(touch /tmp/tr300-release-injection)", False),
        ("v4.3.0'; exit 0; #", False),
    ):
        run_case(
            bash=bash,
            mock_bin=mock_bin,
            block=release_guard,
            name=f"release guard {tag!r}",
            overrides={"RELEASE_TAG": tag},
            expected_success=succeeds,
        )

    windows_outputs = {
        "tag": "v4.3.0",
        "version": "4.3.0",
        "source_sha": TRUSTED_SHA,
        "release_id": "777777",
        "release_run_id": RELEASE_RUN_ID,
        "release_run_attempt": RELEASE_RUN_ATTEMPT,
    }
    windows_cases = [
        ("lightweight automatic", {}, True, windows_outputs),
        (
            "annotated automatic",
            {
                "MOCK_REF_TYPE": "tag",
                "MOCK_REF_SHA": TAG_OBJECT_SHA,
                "MOCK_TAG_TYPE": "commit",
                "MOCK_TAG_SHA": TRUSTED_SHA,
            },
            True,
            windows_outputs,
        ),
        (
            "trusted manual recovery",
            {
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_TAG": "v4.3.0",
                "DISPATCH_RUN_ID": RELEASE_RUN_ID,
            },
            True,
            windows_outputs,
        ),
        ("fork automatic", {"UPSTREAM_REPOSITORY": "attacker/fork"}, False, None),
        ("pull request automatic", {"UPSTREAM_EVENT": "pull_request"}, False, None),
        ("failed upstream", {"UPSTREAM_CONCLUSION": "failure"}, False, None),
        ("upstream SHA mismatch", {"UPSTREAM_SHA": OTHER_SHA}, False, None),
        (
            "prerelease manual",
            {"EVENT_NAME": "workflow_dispatch", "DISPATCH_TAG": "v4.3.0-rc.1"},
            False,
            None,
        ),
        (
            "malicious manual",
            {"EVENT_NAME": "workflow_dispatch", "DISPATCH_TAG": "v4.3.0$(id)"},
            False,
            None,
        ),
        (
            "quote and semicolon manual injection",
            {
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_TAG": "v4.3.0'; exit 0; #",
            },
            False,
            None,
        ),
        (
            "tag API failure",
            {"MOCK_FAIL_ENDPOINT": f"repos/{REPOSITORY}/git/ref/tags/v4.3.0"},
            False,
            None,
        ),
        ("missing release", {"MOCK_RELEASE_PRESENT": "false"}, False, None),
        ("branch release target", {"MOCK_RELEASE_TARGET": "main"}, False, None),
        ("mismatched release target", {"MOCK_RELEASE_TARGET": OTHER_SHA}, False, None),
    ]
    for name, overrides, succeeds, outputs in windows_cases:
        run_case(
            bash=bash,
            mock_bin=mock_bin,
            block=windows_resolver,
            name=f"Windows resolver: {name}",
            overrides=overrides,
            expected_success=succeeds,
            expected_outputs=outputs,
        )

    publisher_cases = [
        ("trusted lightweight publish", {}, None, True),
        (
            "trusted annotated publish",
            {
                "MOCK_REF_TYPE": "tag",
                "MOCK_REF_SHA": TAG_OBJECT_SHA,
                "MOCK_TAG_TYPE": "commit",
                "MOCK_TAG_SHA": TRUSTED_SHA,
            },
            None,
            True,
        ),
        (
            "quote and semicolon tag injection",
            {"RELEASE_TAG": "v4.3.0'; exit 0; #"},
            None,
            False,
        ),
        ("malformed expected SHA", {"EXPECTED_SHA": "not-a-sha"}, None, False),
        ("retargeted tag", {"MOCK_REF_SHA": OTHER_SHA}, None, False),
        (
            "tag API failure",
            {"MOCK_FAIL_ENDPOINT": f"repos/{REPOSITORY}/git/ref/tags/v4.3.0"},
            None,
            False,
        ),
        ("missing release", {"MOCK_RELEASE_PRESENT": "false"}, None, False),
        ("branch release target", {"MOCK_RELEASE_TARGET": "main"}, None, False),
        ("retargeted release", {"MOCK_RELEASE_TARGET": OTHER_SHA}, None, False),
        ("extra artifact entry", {}, "extra", False),
        ("missing artifact entry", {}, "missing", False),
        ("empty artifact payload", {}, "empty", False),
        ("non-regular artifact entry", {}, "directory", False),
        ("mismatched checksum sidecar", {}, "bad-checksum", False),
        ("missing upstream sentinel", {"MOCK_CURRENT_ASSETS": ""}, None, False),
        (
            "existing destination collision",
            {
                "MOCK_CURRENT_ASSETS": "\n".join(
                    (*WINDOWS_UPSTREAM_SENTINELS, WINDOWS_RELEASE_ASSETS[0])
                )
            },
            None,
            False,
        ),
    ]
    for name, overrides, mutation, succeeds in publisher_cases:
        run_windows_publisher_case(
            bash=bash,
            mock_bin=mock_bin,
            block=windows_publisher,
            name=f"Windows publisher: {name}",
            overrides=overrides,
            artifact_mutation=mutation,
            expected_success=succeeds,
        )

    validation_outputs = {
        "tag": "v4.3.0",
        "version": "4.3.0",
        "previous_tag": "v4.2.2",
        "previous_version": "4.2.2",
        "source_sha": TRUSTED_SHA,
        "release_id": "777777",
        "validation_mode": "private",
        "upstream_run_id": RELEASE_RUN_ID,
        "upstream_run_attempt": RELEASE_RUN_ATTEMPT,
    }
    private_validation_run = {
        "UPSTREAM_EVENT": "workflow_run",
        "UPSTREAM_WORKFLOW_NAME": "Windows Installers",
        "MOCK_RELEASE_WORKFLOW_ID": WINDOWS_WORKFLOW_ID,
        "MOCK_WINDOWS_WORKFLOW_ID": WINDOWS_WORKFLOW_ID,
        "MOCK_RELEASE_RUN_NAME": "Windows Installers",
        "MOCK_RELEASE_RUN_PATH": ".github/workflows/windows-installers.yml",
        "MOCK_RELEASE_RUN_EVENT": "workflow_run",
        "MOCK_RELEASE_RUN_TAG": "main",
        "MOCK_RELEASE_DRAFT": "true",
        "MOCK_RELEASE_PRERELEASE": "false",
        "MOCK_RELEASE_ASSET_COUNT": "30",
        "MOCK_LATEST_TAG": "v4.2.2",
    }
    public_validation_outputs = {
        **validation_outputs,
        "release_id": "",
        "validation_mode": "public",
    }
    public_validation_run = {
        "UPSTREAM_EVENT": "workflow_run",
        "UPSTREAM_WORKFLOW_NAME": "macOS Universal Package",
        "MOCK_RELEASE_WORKFLOW_ID": MACOS_WORKFLOW_ID,
        "MOCK_MACOS_WORKFLOW_ID": MACOS_WORKFLOW_ID,
        "MOCK_RELEASE_RUN_NAME": "macOS Universal Package",
        "MOCK_RELEASE_RUN_PATH": ".github/workflows/macos-installer.yml",
        "MOCK_RELEASE_RUN_EVENT": "workflow_run",
        "MOCK_RELEASE_RUN_TAG": "main",
        "MOCK_RELEASE_DRAFT": "false",
        "MOCK_RELEASE_PRERELEASE": "false",
        "MOCK_RELEASE_ASSET_COUNT": "34",
        "MOCK_LATEST_TAG": "v4.3.0",
    }
    validation_cases = [
        (
            "lightweight automatic private draft",
            private_validation_run,
            True,
            validation_outputs,
        ),
        (
            "annotated automatic private draft",
            {
                **private_validation_run,
                "MOCK_REF_TYPE": "tag",
                "MOCK_REF_SHA": TAG_OBJECT_SHA,
                "MOCK_TAG_TYPE": "commit",
                "MOCK_TAG_SHA": TRUSTED_SHA,
            },
            True,
            validation_outputs,
        ),
        (
            "trusted manual private recovery",
            {
                **private_validation_run,
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_MODE": "private",
                "DISPATCH_TAG": "v4.3.0",
                "DISPATCH_UPSTREAM_RUN_ID": RELEASE_RUN_ID,
            },
            True,
            validation_outputs,
        ),
        (
            "automatic public updater smoke",
            public_validation_run,
            True,
            public_validation_outputs,
        ),
        (
            "trusted manual public updater smoke",
            {
                **public_validation_run,
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_MODE": "public",
                "DISPATCH_TAG": "v4.3.0",
                "DISPATCH_UPSTREAM_RUN_ID": RELEASE_RUN_ID,
            },
            True,
            public_validation_outputs,
        ),
        (
            "fork automatic chain",
            {
                **private_validation_run,
                "UPSTREAM_REPOSITORY": "attacker/fork",
            },
            False,
            None,
        ),
        (
            "direct push cannot impersonate the two-hop chain",
            {**private_validation_run, "UPSTREAM_EVENT": "push"},
            False,
            None,
        ),
        (
            "failed upstream packaging",
            {**private_validation_run, "UPSTREAM_CONCLUSION": "failure"},
            False,
            None,
        ),
        (
            "malformed upstream SHA",
            {**private_validation_run, "UPSTREAM_SHA": "not-a-sha"},
            False,
            None,
        ),
        (
            "upstream SHA mismatch",
            {**private_validation_run, "UPSTREAM_SHA": OTHER_SHA},
            False,
            None,
        ),
        (
            "no release for upstream SHA",
            {**private_validation_run, "MOCK_RELEASE_TAGS_FOR_SHA": ""},
            False,
            None,
        ),
        (
            "ambiguous releases for upstream SHA",
            {
                **private_validation_run,
                "MOCK_RELEASE_TAGS_FOR_SHA": "v4.3.0\nv4.2.2",
            },
            False,
            None,
        ),
        (
            "prerelease manual validation",
            {
                **private_validation_run,
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_MODE": "private",
                "DISPATCH_TAG": "v4.3.0-rc.1",
                "DISPATCH_UPSTREAM_RUN_ID": RELEASE_RUN_ID,
            },
            False,
            None,
        ),
        (
            "malicious manual validation",
            {
                **private_validation_run,
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_MODE": "private",
                "DISPATCH_TAG": "v4.3.0$(id)",
                "DISPATCH_UPSTREAM_RUN_ID": RELEASE_RUN_ID,
            },
            False,
            None,
        ),
        (
            "quote and semicolon manual validation injection",
            {
                **private_validation_run,
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_MODE": "private",
                "DISPATCH_TAG": "v4.3.0'; exit 0; #",
                "DISPATCH_UPSTREAM_RUN_ID": RELEASE_RUN_ID,
            },
            False,
            None,
        ),
        (
            "tag API failure",
            {
                **private_validation_run,
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_MODE": "private",
                "DISPATCH_TAG": "v4.3.0",
                "DISPATCH_UPSTREAM_RUN_ID": RELEASE_RUN_ID,
                "MOCK_FAIL_ENDPOINT": f"repos/{REPOSITORY}/git/ref/tags/v4.3.0",
            },
            False,
            None,
        ),
        (
            "missing release target",
            {
                **private_validation_run,
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_MODE": "private",
                "DISPATCH_TAG": "v4.3.0",
                "DISPATCH_UPSTREAM_RUN_ID": RELEASE_RUN_ID,
                "MOCK_RELEASE_PRESENT": "false",
            },
            False,
            None,
        ),
        (
            "branch release target",
            {
                **private_validation_run,
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_MODE": "private",
                "DISPATCH_TAG": "v4.3.0",
                "DISPATCH_UPSTREAM_RUN_ID": RELEASE_RUN_ID,
                "MOCK_RELEASE_TARGET": "main",
            },
            False,
            None,
        ),
        (
            "mismatched release target",
            {
                **private_validation_run,
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_MODE": "private",
                "DISPATCH_TAG": "v4.3.0",
                "DISPATCH_UPSTREAM_RUN_ID": RELEASE_RUN_ID,
                "MOCK_RELEASE_TARGET": OTHER_SHA,
            },
            False,
            None,
        ),
        (
            "wrong private upstream workflow identity",
            {**private_validation_run, "MOCK_RELEASE_WORKFLOW_ID": "42"},
            False,
            None,
        ),
        (
            "failed private upstream run",
            {**private_validation_run, "MOCK_RELEASE_RUN_CONCLUSION": "failure"},
            False,
            None,
        ),
        (
            "wrong private upstream run attempt",
            {**private_validation_run, "UPSTREAM_RUN_ATTEMPT": "2"},
            False,
            None,
        ),
        (
            "private draft already public",
            {
                **private_validation_run,
                "MOCK_RELEASE_DRAFT": "false",
                "MOCK_RELEASE_ASSET_COUNT": "34",
            },
            False,
            None,
        ),
        (
            "public smoke before publication",
            {
                **public_validation_run,
                "MOCK_RELEASE_DRAFT": "true",
                "MOCK_RELEASE_ASSET_COUNT": "30",
                "MOCK_LATEST_TAG": "v4.2.2",
            },
            False,
            None,
        ),
        (
            "unauthorized manual actor",
            {
                **private_validation_run,
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_MODE": "private",
                "DISPATCH_TAG": "v4.3.0",
                "DISPATCH_UPSTREAM_RUN_ID": RELEASE_RUN_ID,
                "ACTOR_ID": "1",
            },
            False,
            None,
        ),
    ]
    for name, overrides, succeeds, outputs in validation_cases:
        run_case(
            bash=bash,
            mock_bin=mock_bin,
            block=windows_validation_resolver,
            name=f"Windows validation resolver: {name}",
            overrides=overrides,
            expected_success=succeeds,
            expected_outputs=outputs,
        )

    validation_function = re.search(
        r"(?ms)^validate_windows_validation_run\(\) \{.*?^\}\n",
        macos_publisher,
    )
    if validation_function is None:
        raise AssertionError("macOS publisher: Windows validation function was not extractable")
    validation_function_block = "set -euo pipefail\n" + validation_function.group(0) + """
validate_windows_validation_run "$TEST_VALIDATION_RUN_ID" "$TEST_VALIDATION_EVENT"
printf 'attempt=%s\n' "$validation_run_attempt" >> "$GITHUB_OUTPUT"
"""
    mac_validation_run = {
        "REPOSITORY": REPOSITORY,
        "EXPECTED_SHA": TRUSTED_SHA,
        "TEST_VALIDATION_RUN_ID": RELEASE_RUN_ID,
        "TEST_VALIDATION_EVENT": "workflow_run",
        "MOCK_RELEASE_WORKFLOW_ID": WINDOWS_VALIDATION_WORKFLOW_ID,
        "MOCK_WINDOWS_VALIDATION_WORKFLOW_ID": WINDOWS_VALIDATION_WORKFLOW_ID,
        "MOCK_RELEASE_RUN_NAME": "Windows Installer Validation",
        "MOCK_RELEASE_RUN_PATH": ".github/workflows/windows-installer-validation.yml",
        "MOCK_RELEASE_RUN_EVENT": "workflow_run",
        "MOCK_RELEASE_RUN_TAG": "main",
    }
    mac_validation_cases = [
        ("trusted exact validation", {}, True),
        ("missing validation run", {"TEST_VALIDATION_RUN_ID": "999999999"}, False),
        ("wrong validation workflow ID", {"MOCK_RELEASE_WORKFLOW_ID": "42"}, False),
        ("wrong validation workflow path", {"MOCK_RELEASE_RUN_PATH": ".github/workflows/ci.yml"}, False),
        ("failed validation run", {"MOCK_RELEASE_RUN_CONCLUSION": "failure"}, False),
        ("skipped validation run", {"MOCK_RELEASE_RUN_CONCLUSION": "skipped"}, False),
        ("wrong validation event", {"MOCK_RELEASE_RUN_EVENT": "push"}, False),
        ("wrong validation repository", {"MOCK_RELEASE_RUN_REPOSITORY": "attacker/fork"}, False),
        ("wrong validation head repository", {"MOCK_RELEASE_RUN_HEAD_REPOSITORY": "attacker/fork"}, False),
        ("wrong validation SHA", {"MOCK_RELEASE_RUN_SHA": OTHER_SHA}, False),
        ("malformed validation attempt", {"MOCK_RELEASE_RUN_ATTEMPT": "0"}, False),
    ]
    for name, overrides, succeeds in mac_validation_cases:
        run_case(
            bash=bash,
            mock_bin=mock_bin,
            block=validation_function_block,
            name=f"macOS finalizer validation binding: {name}",
            overrides={**mac_validation_run, **overrides},
            expected_success=succeeds,
            expected_outputs={"attempt": RELEASE_RUN_ATTEMPT} if succeeds else None,
        )

    macos_publisher_cases = (
        ("exact 30+4 asset publication", None, True),
        ("private release tag mutation", "release-tag-mismatch", False),
        ("stale native proof attempt", "stale-native-attempt", False),
        (
            "validation manifest and private draft divergence",
            "manifest-draft-divergence",
            False,
        ),
        (
            "macOS build artifact digest replacement",
            "macos-artifact-replacement",
            False,
        ),
        ("pre-existing macOS destination collision", "destination-collision", False),
    )
    check_macos_publisher_jq_literal_fidelity(macos_publisher)
    for name, mutation, succeeds in macos_publisher_cases:
        run_macos_publisher_case(
            bash=bash,
            mock_bin=mock_bin,
            block=macos_publisher_fixture,
            name=f"macOS full publisher: {name}",
            mutation=mutation,
            expected_success=succeeds,
        )

    rejection_reach_marker = (
        'gh api "repos/$REPOSITORY/releases/$RELEASE_ID" \\\n'
        '  > "$work_directory/validated-draft.json"\n'
    )
    if macos_publisher.count(rejection_reach_marker) != 1:
        raise AssertionError("macOS publisher rejection reach marker changed")
    adversarial_publisher = macos_publisher_fixture.replace(
        rejection_reach_marker,
        rejection_reach_marker
        + 'gh release delete "$RELEASE_TAG" --repo "$REPOSITORY" --yes\n',
        1,
    )
    try:
        run_macos_publisher_case(
            bash=bash,
            mock_bin=mock_bin,
            block=adversarial_publisher,
            name="macOS full publisher: adversarial rejected-path mutation",
            mutation="manifest-draft-divergence",
            expected_success=False,
        )
    except AssertionError as error:
        if "mutating gh invocation(s)" not in str(error) or "release delete" not in str(error):
            raise AssertionError(
                "macOS publisher mutation self-test failed for the wrong reason"
            ) from error
    else:
        raise AssertionError(
            "macOS publisher accepted an injected rejected-path release deletion"
        )

    macos_outputs = {
        "build": "true",
        "tag": "v4.3.0",
        "version": "4.3.0",
        "source_sha": TRUSTED_SHA,
        "release_id": "777777",
        "run_id": RELEASE_RUN_ID,
        "run_attempt": RELEASE_RUN_ATTEMPT,
        "custody_required": "true",
    }
    macos_cases = [
        ("lightweight automatic", {}, True, macos_outputs),
        (
            "annotated automatic",
            {
                "MOCK_REF_TYPE": "tag",
                "MOCK_REF_SHA": TAG_OBJECT_SHA,
                "MOCK_TAG_TYPE": "commit",
                "MOCK_TAG_SHA": TRUSTED_SHA,
            },
            True,
            macos_outputs,
        ),
        (
            "trusted tagged manual publish",
            {
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_TAG": "v4.3.0",
                "PREFLIGHT_ONLY": "false",
                "DISPATCH_RUN_ID": RELEASE_RUN_ID,
            },
            True,
            macos_outputs,
        ),
        (
            "trusted tagged manual preflight",
            {
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_TAG": "v4.3.0",
                "PREFLIGHT_ONLY": "true",
                "DISPATCH_RUN_ID": "",
            },
            True,
            {**macos_outputs, "build": "false"},
        ),
        (
            "tagless credential preflight",
            {
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_TAG": "",
                "PREFLIGHT_ONLY": "true",
            },
            True,
            {
                "build": "false",
                "tag": "",
                "version": "",
                "source_sha": TRUSTED_SHA,
                "release_id": "",
                "run_id": "",
                "run_attempt": "",
                "custody_required": "false",
            },
        ),
        (
            "tagless publish",
            {
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_TAG": "",
                "PREFLIGHT_ONLY": "false",
            },
            False,
            None,
        ),
        (
            "quote and semicolon tagged manual injection",
            {
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_TAG": "v4.3.0'; exit 0; #",
                "PREFLIGHT_ONLY": "false",
            },
            False,
            None,
        ),
        ("fork automatic", {"UPSTREAM_REPOSITORY": "attacker/fork"}, False, None),
        ("upstream SHA mismatch", {"UPSTREAM_SHA": OTHER_SHA}, False, None),
        (
            "automatic source no longer protected-main tip",
            {"MOCK_DEFAULT_SHA": OTHER_SHA, "WORKFLOW_SHA": OTHER_SHA},
            False,
            None,
        ),
        (
            "automatic downstream workflow not protected-main tip",
            {"WORKFLOW_SHA": OTHER_SHA},
            False,
            None,
        ),
        (
            "tagged dispatch source no longer protected-main tip",
            {
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_TAG": "v4.3.0",
                "PREFLIGHT_ONLY": "true",
                "DISPATCH_RUN_ID": RELEASE_RUN_ID,
                "MOCK_DEFAULT_SHA": OTHER_SHA,
                "WORKFLOW_SHA": OTHER_SHA,
            },
            False,
            None,
        ),
        (
            "tagless preflight workflow not protected-main tip",
            {
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_TAG": "",
                "PREFLIGHT_ONLY": "true",
                "WORKFLOW_SHA": OTHER_SHA,
            },
            False,
            None,
        ),
        (
            "automatic run-attempt mismatch",
            {"MOCK_RELEASE_RUN_ATTEMPT": "2"},
            False,
            None,
        ),
        (
            "wrong Release workflow identity",
            {"MOCK_RELEASE_WORKFLOW_ID": "42"},
            False,
            None,
        ),
        (
            "wrong Release workflow path",
            {"MOCK_RELEASE_RUN_PATH": ".github/workflows/attacker.yml"},
            False,
            None,
        ),
        (
            "wrong Release run event",
            {"MOCK_RELEASE_RUN_EVENT": "pull_request"},
            False,
            None,
        ),
        (
            "wrong Release run repository",
            {"MOCK_RELEASE_RUN_REPOSITORY": "attacker/fork"},
            False,
            None,
        ),
        (
            "wrong Release run head repository",
            {"MOCK_RELEASE_RUN_HEAD_REPOSITORY": "attacker/fork"},
            False,
            None,
        ),
        (
            "failed exact Release run",
            {"MOCK_RELEASE_RUN_CONCLUSION": "failure"},
            False,
            None,
        ),
        (
            "tagged publication omits exact run ID",
            {
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_TAG": "v4.3.0",
                "PREFLIGHT_ONLY": "false",
                "DISPATCH_RUN_ID": "",
            },
            False,
            None,
        ),
        (
            "ambiguous manual preflight runs",
            {
                "EVENT_NAME": "workflow_dispatch",
                "DISPATCH_TAG": "v4.3.0",
                "PREFLIGHT_ONLY": "true",
                "DISPATCH_RUN_ID": "",
                "MOCK_RELEASE_RUN_IDS": f"{RELEASE_RUN_ID}\n42",
            },
            False,
            None,
        ),
        ("missing release", {"MOCK_RELEASE_PRESENT": "false"}, False, None),
        ("branch release target", {"MOCK_RELEASE_TARGET": "main"}, False, None),
        ("mismatched release target", {"MOCK_RELEASE_TARGET": OTHER_SHA}, False, None),
    ]
    for name, overrides, succeeds, outputs in macos_cases:
        calls = run_case(
            bash=bash,
            mock_bin=mock_bin,
            block=macos_resolver,
            name=f"macOS resolver: {name}",
            overrides=overrides,
            expected_success=succeeds,
            expected_outputs=outputs,
        )
        if name == "tagless credential preflight" and any(
            "/releases/tags/" in call for call in calls
        ):
            raise AssertionError("tagless macOS preflight unexpectedly queried a release")

    custody_cases = [
        ("trusted real-shape absolute manifests", None, True),
        ("wrong run event", "wrong-run-event", False),
        ("wrong run repository", "wrong-run-repository", False),
        ("wrong run head repository", "wrong-run-head-repository", False),
        ("wrong run SHA", "wrong-run-sha", False),
        ("wrong workflow path", "wrong-run-path", False),
        ("wrong workflow ID", "wrong-workflow-id", False),
        ("wrong run attempt", "wrong-attempt", False),
        ("midflight rerun attempt", "midflight-attempt", False),
        ("extra Apple artifact", "extra-apple-artifact", False),
        ("missing canonical artifact", "missing-canonical-artifact", False),
        ("duplicate canonical artifact", "duplicate-canonical-artifact", False),
        ("expired canonical artifact", "expired-artifact", False),
        ("wrong artifact repository", "wrong-artifact-repository", False),
        ("API digest mismatch", "digest-mismatch", False),
        ("extra ZIP member", "extra-zip-member", False),
        ("checksum mismatch", "bad-checksum", False),
        ("midflight artifact replacement", "midflight-artifact-replacement", False),
        ("missing prepared Release artifact", "missing-prepared-artifact", False),
        ("duplicate prepared Release artifact", "duplicate-prepared-artifact", False),
        ("prepared Release API digest mismatch", "prepared-digest-mismatch", False),
        ("prepared Release extra ZIP member", "prepared-extra-member", False),
        ("prepared Release duplicate ZIP member", "prepared-duplicate-member", False),
        ("prepared wrapper hash mismatch", "prepared-wrapper-hash-mismatch", False),
        ("prepared dist-installer hash mismatch", "prepared-dist-hash-mismatch", False),
        ("prepared Apple archive hash mismatch", "prepared-archive-hash-mismatch", False),
        (
            "prepared wrapper dist-installer pin mismatch",
            "prepared-dist-wrapper-pin-mismatch",
            False,
        ),
        (
            "prepared and canonical Apple archives diverge while both remain internally valid",
            "prepared-canonical-divergence",
            False,
        ),
        (
            "midflight prepared Release artifact replacement",
            "midflight-prepared-replacement",
            False,
        ),
    ]
    for name, mutation, succeeds in custody_cases:
        run_macos_source_custody_case(
            bash=bash,
            mock_bin=mock_bin,
            block=macos_source_custody,
            name=f"macOS exact-run source custody: {name}",
            mutation=mutation,
            expected_success=succeeds,
        )

    native_custody_cases = [
        ("trusted exact build artifact", None, True),
        ("wrong immutable artifact ID", "native-wrong-id", False),
        ("API digest mismatch", "native-api-digest-mismatch", False),
        ("expired build artifact", "native-expired", False),
        ("wrong workflow run", "native-wrong-run", False),
        ("wrong repository", "native-wrong-repository", False),
        ("wrong head repository", "native-wrong-head-repository", False),
        ("wrong source SHA", "native-wrong-sha", False),
        ("ZIP digest mismatch", "native-zip-digest-mismatch", False),
        ("extra ZIP member", "native-extra-member", False),
        ("unsafe ZIP path", "native-unsafe-path", False),
    ]
    for name, mutation, succeeds in native_custody_cases:
        run_macos_native_build_custody_case(
            bash=bash,
            mock_bin=mock_bin,
            block=macos_native_custody,
            name=f"macOS native build custody: {name}",
            mutation=mutation,
            expected_success=succeeds,
        )

def check_workflow_contract(path: Path, workflow: str) -> None:
    label = path.name
    require(workflow, "github.event.workflow_run.conclusion == 'success'", label)
    require(workflow, "github.event.workflow_run.event == 'push'", label)
    require(
        workflow,
        "github.event.workflow_run.head_repository.full_name == github.repository",
        label,
    )
    require(workflow, STABLE_TAG_PATTERN, label)
    require(workflow, "git/ref/tags/$tag", label)
    require(workflow, "git/tags/$source_sha", label)
    require(workflow, "gh api --paginate --slurp", label)
    require(workflow, "releases?per_page=100", label)
    require(workflow, "release_id", label)
    require(workflow, ".target_commitish", label)
    if "startsWith(github.event.workflow_run.head_branch, 'v')" in workflow:
        raise AssertionError(f"{label}: v-prefix-only workflow gate returned")

    executable = "\n".join(extract_run_blocks(workflow))
    if "${{ vars." in executable:
        raise AssertionError(
            f"{label}: repository variable interpolated directly into executable shell"
        )
    for expression in (
        "${{ inputs.tag }}",
        "${{ github.event.workflow_run.head_branch }}",
        "${{ github.event.workflow_run.head_sha }}",
    ):
        if expression in executable:
            raise AssertionError(
                f"{label}: untrusted event expression interpolated into executable shell: "
                f"{expression}"
            )


def check_windows_validation_provenance(workflow: str) -> None:
    label = WINDOWS_VALIDATION_WORKFLOW.name
    resolve = extract_job(workflow, "resolve", label)
    prepare = extract_job(workflow, "prepare-validation-inputs", label)
    channels = extract_job(workflow, "channels", label)
    managed = extract_job(workflow, "managed-cli-choice", label)
    fresh_format = extract_job(workflow, "fresh-format-choice", label)

    # GitHub omits draft releases from callers without push access. Keep this
    # tiny model executable so fixtures cannot silently assume that a
    # read-scoped GITHUB_TOKEN sees private candidates.
    release_records = [{"tag": "v-test", "draft": True}, {"tag": "v-old", "draft": False}]
    read_only_visible = [record for record in release_records if not record["draft"]]
    push_visible = list(release_records)
    if any(record["draft"] for record in read_only_visible) or not any(
        record["draft"] for record in push_visible
    ):
        raise AssertionError(f"{label}: private-draft visibility model drifted")

    if workflow.count("contents: write") != 2:
        raise AssertionError(
            f"{label}: contents:write must be limited to the no-checkout draft "
            "resolver/freezer"
        )
    for job, job_name in (
        (resolve, "resolve"),
        (prepare, "prepare-validation-inputs"),
    ):
        require(
            job,
            "permissions:\n      actions: read\n      contents: write",
            f"{label} {job_name} draft visibility",
        )
        if "actions/checkout" in job:
            raise AssertionError(
                f"{label}: write-scoped {job_name} must not check out repository source"
            )
    for job, job_name in ((channels, "channels"), (managed, "managed-cli-choice")):
        require(
            job,
            "permissions:\n      actions: read\n      contents: read",
            f"{label} {job_name} read-only execution",
        )
        if "contents: write" in job:
            raise AssertionError(
                f"{label}: executed candidate job {job_name} gained release write access"
            )
        if "actions/checkout" in job:
            raise AssertionError(f"{label}: {job_name} unexpectedly checks out source")
    if "contents: write" in fresh_format:
        raise AssertionError(f"{label}: offline format-choice job gained release write access")
    for job, job_name in (
        (channels, "channels"),
        (managed, "managed-cli-choice"),
        (fresh_format, "fresh-format-choice"),
    ):
        for forbidden in (
            'releases/$RELEASE_ID',
            'releases/$release_id',
            'gh release download "$RELEASE_TAG"',
        ):
            if forbidden in job:
                raise AssertionError(
                    f"{label}: read-only {job_name} still queries private release state"
                )
    require(workflow, "github.event.workflow_run.conclusion == 'success'", label)
    require(
        workflow,
        'workflows: ["Windows Installers", "macOS Universal Package"]',
        label,
    )
    require(workflow, "github.event.workflow_run.event == 'workflow_run'", label)
    if re.search(
        r"github\.event\.workflow_run\.name == 'Windows Installers'\s*&&\s*"
        r"\(github\.event\.workflow_run\.event == 'workflow_run'\s*\|\|\s*"
        r"github\.event\.workflow_run\.event == 'workflow_dispatch'\)",
        resolve,
    ) is None:
        raise AssertionError(f"{label}: Windows automatic/recovery event gate drifted")
    if re.search(
        r"github\.event\.workflow_run\.name == 'macOS Universal Package'\s*&&\s*"
        r"github\.event\.workflow_run\.event == 'workflow_run'",
        resolve,
    ) is None:
        raise AssertionError(f"{label}: macOS preflight exclusion gate drifted")
    require(
        workflow,
        "github.event.workflow_run.head_repository.full_name == github.repository",
        label,
    )
    require(workflow, STABLE_TAG_PATTERN, label)
    require(workflow, "git/ref/tags/$tag", label)
    require(workflow, "git/tags/$source_sha", label)
    require(workflow, "releases?per_page=100", label)
    require(workflow, "release_id: ${{ steps.release.outputs.release_id }}", label)
    require(workflow, ".target_commitish", label)
    for needle in (
        "github.ref == 'refs/heads/main'",
        "github.actor_id == '30877743'",
        "github.repository == 'QubeTX/qube-machine-report'",
        "validation_mode: ${{ steps.release.outputs.validation_mode }}",
        "upstream_run_id: ${{ steps.release.outputs.upstream_run_id }}",
        "upstream_run_attempt: ${{ steps.release.outputs.upstream_run_attempt }}",
        "windows-validation-inputs",
        "validation-input-manifest.json",
        "windows-installer-validation-proof",
        "validation_input_artifact_id",
        "validation_input_artifact_digest",
        "windows_artifact_id",
        "windows_artifact_digest",
        "artifact-ids: ${{ needs.prepare-validation-inputs.outputs.artifact_id }}",
        f"uses: {UPLOAD_ARTIFACT_ACTION}",
        f"uses: {DOWNLOAD_ARTIFACT_ACTION}",
        'gh api "repos/$repo/releases/$release_id"',
        '.id == $id and .tag_name == $tag and .target_commitish == $sha',
        '[[ "$release_draft" == false && "$release_prerelease" == false && "$asset_count" == 34 ]]',
        "direct authenticated candidate transition",
        "private draft leaked into public updater discovery",
    ):
        require(workflow, needle, f"{label} private/public custody")
    attest = extract_job(workflow, "attest-private-validation", label)
    freeze = extract_named_run(
        prepare,
        "Download, verify, and bind the exact release inventory",
        f"{label} frozen release bytes",
    )
    for needle in (
        "releases/assets/$asset_id",
        "Accept: application/octet-stream",
        "sort_by(.name)[] | [.id, .name] | @tsv",
    ):
        require(freeze, needle, f"{label} numeric draft asset download")
    if 'gh release download "$RELEASE_TAG"' in freeze:
        raise AssertionError(f"{label}: freezer returned to draft-blind tag downloads")
    downloaded_match_index = freeze.index(
        '[[ "$digest" =~ ^sha256:([0-9a-f]{64})$ ]]'
    )
    downloaded_capture_index = freeze.index(
        "downloaded_digest=${BASH_REMATCH[1]}", downloaded_match_index
    )
    downloaded_size_index = freeze.index(
        '[[ "$size" =~ ^[1-9][0-9]*$ ]]', downloaded_capture_index
    )
    downloaded_compare_index = freeze.index(
        '[[ "$actual" == "$downloaded_digest" ]]', downloaded_size_index
    )
    if not (
        downloaded_match_index
        < downloaded_capture_index
        < downloaded_size_index
        < downloaded_compare_index
    ):
        raise AssertionError(f"{label}: downloaded digest capture is out of order")
    if (
        '[[ "$digest" =~ ^sha256:([0-9a-f]{64})$ &&' in freeze
        or '[[ "$actual" == "${BASH_REMATCH[1]}" ]]' in freeze
    ):
        raise AssertionError(f"{label}: downloaded-byte loop clobbers BASH_REMATCH")
    channel_verify = extract_named_step(
        workflow,
        "Verify reports, channel, registration, PATH, and safe update result",
        label,
    )
    require(
        channel_verify,
        "VALIDATION_MODE: ${{ needs.resolve.outputs.validation_mode }}",
        f"{label} private/public channel dispatch",
    )
    require(
        channel_verify,
        "$env:VALIDATION_MODE -eq 'private'",
        f"{label} private/public channel dispatch",
    )
    for needle in (
        "TR300_DIST_INSTALLER_PATH",
        "TR300_DOWNLOAD_URL",
        "candidateDirectoryUri.IsFile",
        "@('TR300_GITHUB_TOKEN', 'GITHUB_TOKEN', 'GH_TOKEN')",
        "SetEnvironmentVariable($name, $null, 'Process')",
    ):
        require(channel_verify, needle, f"{label} token-free local candidate execution")
    managed_execution = extract_named_step(
        managed,
        "Install native channel, then make fresh PowerShell intent authoritative",
        label,
    )
    for needle in (
        "TR300_DIST_INSTALLER_PATH",
        "TR300_DOWNLOAD_URL",
        "candidateDirectoryUri.IsFile",
    ):
        require(managed_execution, needle, f"{label} managed local candidate execution")
    if "TR300_GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}" in managed_execution:
        raise AssertionError(f"{label}: managed candidate code still inherits a GitHub token")
    for needle in (
        "windows-release-assets",
        ".workflow_run.id == $run",
        ".workflow_run.head_sha == $sha",
        "release_assets: ([.assets[] | {id, name, size, digest}]",
        "cmp \"$work_directory/windows-assets/$name\" \"$OUTPUT_DIRECTORY/$name\"",
    ):
        require(prepare, needle, f"{label} normalized byte custody")
    require(attest, "needs:\n      - resolve\n      - prepare-validation-inputs\n      - channels", label)
    require(attest, 'result: "success"', f"{label} proof result")
    if "([+-].*)?" in workflow:
        raise AssertionError(
            f"{label}: prerelease/build suffix acceptance returned to the stable chain"
        )

    executable = extract_named_run(
        workflow,
        "Resolve tag and require every Windows family",
        label,
    )
    for expression in (
        "${{ inputs.tag }}",
        "${{ inputs.validation_mode }}",
        "${{ inputs.upstream_run_id }}",
        "${{ github.event_name }}",
        "${{ github.repository }}",
        "${{ github.event.workflow_run.conclusion }}",
        "${{ github.event.workflow_run.event }}",
        "${{ github.event.workflow_run.head_repository.full_name }}",
        "${{ github.event.workflow_run.head_sha }}",
    ):
        if expression in executable:
            raise AssertionError(
                f"{label}: untrusted event expression interpolated into executable shell: "
                f"{expression}"
            )


def check_windows_publish_boundary(workflow: str) -> None:
    label = WINDOWS_WORKFLOW.name
    jobs = re.search(r"(?m)^jobs:\s*$", workflow)
    if jobs is None:
        raise AssertionError(f"{label}: workflow is missing its jobs boundary")
    workflow_scope = workflow[: jobs.start()]
    if "GH_TOKEN" in workflow_scope or "${{ github.token }}" in workflow_scope:
        raise AssertionError(
            f"{label}: read-only GitHub token escaped to workflow scope"
        )
    resolver = extract_job(workflow, "resolve-release", label)
    build = extract_job(workflow, "build-windows-installers", label)
    publisher = extract_job(workflow, "publish-windows-installers", label)

    if workflow.count("contents: write") != 2:
        raise AssertionError(
            f"{label}: contents:write must be limited to the no-checkout "
            "draft resolver and publisher"
        )
    require(
        resolver,
        "permissions:\n      actions: read\n      contents: write",
        f"{label} private-draft resolver",
    )
    if "actions/checkout" in resolver:
        raise AssertionError(f"{label}: write-scoped resolver must not check out source")
    for needle in (
        'gh api "repos/$EXPECTED_REPOSITORY/releases/$release_id"',
        "dist-manifest.json",
        "tr300-x86_64-pc-windows-msvc.msi",
        "upstream cargo-dist release is incomplete; missing asset",
    ):
        require(resolver, needle, f"{label} no-checkout F20 guard")
    require(build, "permissions:\n      contents: read", f"{label} build job")
    if "contents: write" in build or "gh release upload" in build:
        raise AssertionError(f"{label}: build job regained release write capability")
    if "releases/" in build or "gh release" in build:
        raise AssertionError(f"{label}: read-only build job still queries private Release state")
    checkout = extract_unique_named_step(build, "Checkout tag", f"{label} build job")
    require(checkout, f"uses: {CHECKOUT_ACTION}", f"{label} exact checkout")
    require(checkout, "persist-credentials: false", f"{label} exact checkout")

    inno_install = extract_unique_named_step(
        build,
        "Install the pinned official Inno Setup 6.7.3",
        f"{label} build job",
    )
    require(
        inno_install,
        "env:\n          GH_TOKEN: ${{ github.token }}",
        f"{label} Inno attestation step",
    )
    if inno_install.count("GH_TOKEN:") != 1:
        raise AssertionError(
            f"{label}: Inno attestation step must receive exactly one GitHub token"
        )
    steps = re.search(r"(?m)^    steps:\s*$", build)
    if steps is None:
        raise AssertionError(f"{label}: build job is missing its steps boundary")
    if "GH_TOKEN" in build[: steps.start()]:
        raise AssertionError(f"{label}: build job exposes GH_TOKEN at job scope")
    if build.count("GH_TOKEN:") != 1 or build.count("${{ github.token }}") != 1:
        raise AssertionError(
            f"{label}: read-only GitHub token must be isolated to the Inno "
            "attestation step"
        )
    if "${{ secrets.GITHUB_TOKEN }}" in build:
        raise AssertionError(
            f"{label}: Inno attestation must use the job's read-only github.token"
        )
    require(
        build,
        f"uses: {UPLOAD_ARTIFACT_ACTION}",
        f"{label} build job",
    )
    require(build, "artifact_id: ${{ steps.windows-upload.outputs.artifact-id }}", label)
    require(build, "artifact_digest: ${{ steps.windows-upload.outputs.artifact-digest }}", label)
    if build.count(f"name: {WINDOWS_RELEASE_ARTIFACT}") != 1:
        raise AssertionError(f"{label}: build must upload one fixed internal artifact")
    for asset in WINDOWS_RELEASE_ASSETS:
        require(
            build,
            f"/windows-release-assets/{asset}",
            f"{label} fixed artifact upload",
        )

    require(publisher, "permissions:\n      actions: read\n      contents: write", f"{label} publisher")
    require(publisher, "actions: read", f"{label} publisher artifact custody")
    require(publisher, "environment: release-publishing", f"{label} publisher environment")
    if "actions/checkout" in publisher:
        raise AssertionError(f"{label}: publisher must not check out repository source")
    if "actions/upload-artifact" in publisher or "actions/download-artifact" in publisher:
        raise AssertionError(f"{label}: publisher must use exact artifact API custody")
    uses = [
        line.strip().removeprefix("- ")
        for line in publisher.splitlines()
        if "uses:" in line
    ]
    if uses:
        raise AssertionError(f"{label}: unexpected publisher actions: {uses!r}")
    if publisher.count("GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}") != 1:
        raise AssertionError(f"{label}: publisher token must exist on one upload step")
    if publisher.count("${{ secrets.GITHUB_TOKEN }}") != 1 or "${{ github.token }}" in publisher:
        raise AssertionError(f"{label}: publisher token escaped the upload step")
    if len(extract_run_blocks(publisher)) != 1:
        raise AssertionError(f"{label}: publisher must contain one audited run block")

    executable = extract_named_run(
        publisher,
        "Revalidate custody and publish the fixed six assets",
        f"{label} publisher",
    )
    require(executable, STABLE_TAG_PATTERN, f"{label} publisher")
    require(executable, "EXPECTED_SHA", f"{label} publisher")
    require(executable, "git/ref/tags/$tag", f"{label} publisher")
    require(executable, "git/tags/$source_sha", f"{label} publisher")
    require(executable, "releases/$RELEASE_ID", f"{label} publisher")
    require(executable, '.id == $id and .tag_name == $tag', f"{label} publisher")
    require(executable, "actions/artifacts/$INTERNAL_ARTIFACT_ID/zip", f"{label} exact artifact ID")
    require(executable, '"sha256:$INTERNAL_ARTIFACT_DIGEST"', f"{label} REST digest")
    require(executable, '[[ "$zip_hash" == "$INTERNAL_ARTIFACT_DIGEST" ]]', f"{label} ZIP digest")
    require(executable, ".workflow_run.id == $run", f"{label} run custody")
    require(executable, 'git/ref/heads/main', f"{label} current main rebind")
    require(executable, 'actions/workflows/ci.yml/runs?event=push', f"{label} exact CI rebind")
    require(executable, ".draft == true", f"{label} private draft only")
    require(executable, "find \"$ASSET_DIRECTORY\"", f"{label} publisher")
    require(executable, "sha256sum -- \"$payload\"", f"{label} publisher")
    require(executable, "gh release upload", f"{label} publisher")
    if "--clobber" in executable or '"$ASSET_DIRECTORY"/*' in executable:
        raise AssertionError(f"{label}: publisher upload is not immutable/fixed-path")
    for asset in WINDOWS_RELEASE_ASSETS:
        require(
            executable,
            f'"$ASSET_DIRECTORY/{asset}"',
            f"{label} fixed release upload",
        )
    for forbidden in (
        "actions/checkout",
        "cargo ",
        "scripts/",
        "Start-Process",
        "Invoke-Expression",
        "powershell",
        "pwsh",
    ):
        if forbidden in publisher:
            raise AssertionError(
                f"{label}: publisher unexpectedly executes repository/build content: "
                f"{forbidden!r}"
            )


def check_macos_publish_boundary(workflow: str) -> None:
    label = MACOS_WORKFLOW.name
    builder = MACOS_INSTALLER_BUILDER.read_text(encoding="utf-8")
    ci_workflow = CI_WORKFLOW.read_text(encoding="utf-8")
    resolver = extract_job(workflow, "resolve-release", label)
    custody = extract_job(workflow, "source-custody", label)
    prepare = extract_job(workflow, "prepare-installer-inputs", label)
    preflight = extract_job(workflow, "credential-preflight", label)
    build = extract_job(workflow, "build", label)
    validate = extract_job(workflow, "validate", label)
    publisher = extract_job(workflow, "publish", label)

    for needle in (
        "for home in /Users/* /Users/.[!.]* /Users/..?*; do",
        "minimum_human_uid=501",
        "maximum_human_uid=4294967295",
        'while IFS= read -r account; do',
        '/usr/bin/dscl -plist . -read "/Users/${account}" UniqueID',
        "'dsAttrTypeStandard:NFSHomeDirectory'",
        '/usr/bin/plutil -lint "$plist_path"',
        '/usr/libexec/PlistBuddy -c "Print ${plist_entry}:0"',
        '/usr/libexec/PlistBuddy -c "Print ${plist_entry}:1"',
        'case "${3-}" in',
        'supports only the current system volume (/)',
        'fail_uninspectable_home() {',
        'Make the declared home available and inspectable',
        'account_uid_sign=negative',
        'account_uid_digits=$(printf \'%s\' "$account_uid" | /usr/bin/wc -c',
        'if [ "$account_uid_digits" -gt 10 ]; then',
        'if [ "$account_uid" -gt "$maximum_human_uid" ]; then',
        'UID outside the supported unsigned 32-bit range',
        'inspection_required=0',
        '[ "$account_uid" -ge "$minimum_human_uid" ]',
        'inspect_home_once "$account_home" "$inspection_required"',
        'directory_listing="${preinstall_state}/directory-listing"',
        'entry_matches="${preinstall_state}/entry-matches"',
        "list_standard_directory() {",
        "standard_entry_present() {",
        "standard_leaf_present() {",
        'if [ -L /Users ] || [ ! -d /Users ]; then',
        'LC_ALL=C /bin/ls -1A /Users > "$directory_listing"',
        'LC_ALL=C /bin/ls -1A "$listed_directory"',
        'LC_ALL=C /usr/bin/grep -Fix -e "$entry_name" "$directory_listing"',
        '> "$entry_matches"',
        'entry_match_count=$(/usr/bin/wc -l < "$entry_matches" |',
        'if [ "$entry_match_count" -ne 1 ]; then',
        'entry_match_status=$?',
        'if [ "$entry_match_status" -ne 1 ]; then',
        "found multiple ASCII-case-insensitive entries matching",
        "plist_capture_sentinel='__TR300_PLIST_CAPTURE_END__'",
        "plist_framed=$(",
        'printf \'%s\' "$plist_capture_sentinel"',
        'plist_with_terminator=${plist_framed%"$plist_capture_sentinel"}',
        'single_plist_value=${plist_with_terminator%"$plist_newline"}',
        'account_uid=$single_plist_value',
        'account_home=$single_plist_value',
    ):
        require(builder, needle, f"{label} preinstall home resolution")
    users_type_index = builder.index('if [ -L /Users ] || [ ! -d /Users ]; then')
    users_listing_index = builder.index(
        'LC_ALL=C /bin/ls -1A /Users > "$directory_listing"'
    )
    users_glob_index = builder.index(
        "for home in /Users/* /Users/.[!.]* /Users/..?*; do"
    )
    if not users_type_index < users_listing_index < users_glob_index:
        raise AssertionError(
            f"{label}: /Users type/list custody does not precede conventional-home globs"
        )
    directory_list_index = builder.index("list_standard_directory() {")
    entry_present_index = builder.index("standard_entry_present() {")
    leaf_present_index = builder.index("standard_leaf_present() {")
    entry_grep_index = builder.index(
        'LC_ALL=C /usr/bin/grep -Fix -e "$entry_name" "$directory_listing"'
    )
    entry_count_index = builder.index(
        'entry_match_count=$(/usr/bin/wc -l < "$entry_matches" |'
    )
    entry_status_index = builder.index("entry_match_status=$?", entry_count_index)
    if not (
        directory_list_index
        < entry_present_index
        < entry_grep_index
        < entry_count_index
        < entry_status_index
        < leaf_present_index
    ):
        raise AssertionError(f"{label}: fixed-depth entry classification is out of order")
    plist_sentinel_index = builder.index(
        "plist_capture_sentinel='__TR300_PLIST_CAPTURE_END__'"
    )
    plist_framed_index = builder.index("plist_framed=$(")
    plist_terminator_index = builder.index(
        'plist_with_terminator=${plist_framed%"$plist_capture_sentinel"}'
    )
    plist_global_index = builder.index(
        'single_plist_value=${plist_with_terminator%"$plist_newline"}'
    )
    uid_read_index = builder.index(
        "read_single_plist_string \"$record_plist\" \\\n"
        "        'dsAttrTypeStandard:UniqueID'"
    )
    uid_assignment_index = builder.index("account_uid=$single_plist_value", uid_read_index)
    home_read_index = builder.index(
        "read_single_plist_string \"$record_plist\" \\\n"
        "        'dsAttrTypeStandard:NFSHomeDirectory'",
        uid_assignment_index,
    )
    home_assignment_index = builder.index(
        "account_home=$single_plist_value", home_read_index
    )
    if not (
        plist_sentinel_index
        < plist_framed_index
        < plist_terminator_index
        < plist_global_index
        < uid_read_index
        < uid_assignment_index
        < home_read_index
        < home_assignment_index
    ):
        raise AssertionError(f"{label}: sentinel-framed plist assignment is out of order")
    uid_normalize_index = builder.index(
        "account_uid=$(printf '%s\\n' \"$account_uid\" | /usr/bin/sed 's/^0*//;s/^$/0/')"
    )
    uid_digits_index = builder.index(
        'account_uid_digits=$(printf \'%s\' "$account_uid" | /usr/bin/wc -c',
        uid_normalize_index,
    )
    uid_width_branch_index = builder.index(
        'if [ "$account_uid_digits" -gt 10 ]; then', uid_digits_index
    )
    uid_range_branch_index = builder.index(
        'if [ "$account_uid" -gt "$maximum_human_uid" ]; then',
        uid_width_branch_index,
    )
    uid_inspection_index = builder.index(
        'inspection_required=0', uid_range_branch_index
    )
    if not (
        uid_normalize_index
        < uid_digits_index
        < uid_width_branch_index
        < uid_range_branch_index
        < uid_inspection_index
    ):
        raise AssertionError(f"{label}: unsigned UID validation is out of order")
    uid_range_diagnostic = "UID outside the supported unsigned 32-bit range"
    if builder.count(uid_range_diagnostic) != 2:
        raise AssertionError(
            f"{label}: both unsupported-positive-UID branches must fail closed"
        )
    uid_validation_slice = builder[uid_digits_index:uid_inspection_index]
    if "continue" in uid_validation_slice or uid_validation_slice.count("fail_closed") != 2:
        raise AssertionError(
            f"{label}: unsupported positive UIDs can bypass fail-closed validation"
        )
    for forbidden_assignment in (
        "account_uid=$(read_single_plist_string",
        "account_home=$(read_single_plist_string",
    ):
        if forbidden_assignment in builder:
            raise AssertionError(
                f"{label}: plist value lost through nested command substitution"
            )
    for forbidden in (
        "SUDO_USER",
        "SUDO_UID",
        "console_user=",
        "/usr/bin/awk '{print $2}'",
        "plutil -extract",
        " -expect ",
    ):
        if forbidden not in builder:
            continue
        raise AssertionError(
            f"{label}: preinstall relies on unsafe invocation/home parsing: {forbidden!r}"
        )

    ci_test = extract_job(ci_workflow, "test", CI_WORKFLOW.name)
    preinstall_behavior = extract_named_step(
        ci_test,
        "Validate direct-PKG preinstall behavior",
        f"{CI_WORKFLOW.name} macOS preinstall behavior",
    )
    for needle in (
        "custom_fixture_user=tr300pkgcustom",
        "custom_fixture_uid=3000000000",
        "The project supports systems older than macOS 12",
        "test -x /usr/libexec/PlistBuddy",
        "preinstall uses plist flags unavailable before macOS 12",
        'custom_fixture_home=$(mktemp -d "/private/tmp/tr300 pkg custom home.XXXXXXXX")',
        'NFSHomeDirectory "$custom_fixture_home"',
        '/usr/bin/dscl -plist . -read "/Users/$custom_fixture_user"',
        '/usr/bin/plutil -lint "$custom_fixture_plist"',
        "'Print :dsAttrTypeStandard\\:NFSHomeDirectory:0'",
        "'Print :dsAttrTypeStandard\\:NFSHomeDirectory:1'",
        'pkgbuild --root "$fixture_payload" --scripts "$fixture_scripts"',
        'cmp "$preinstall" "$fixture_expanded/Scripts/preinstall"',
        '/usr/bin/env -u SUDO_USER -u SUDO_UID',
        '/usr/sbin/installer -pkg "$fixture_pkg" -target /',
        '/usr/sbin/installer -pkg "$fixture_pkg" -target / -dumplog',
        "custom-home managed evidence was incorrectly accepted by PackageKit",
        'managed receipt or link evidence at \\"${custom_fixture_home}\\"',
        "Rerun the managed installer to refresh this copy to a receipt-aware version",
        'test ! -e "$fixture_install_root/payload-marker"',
        'pkgutil --pkg-info "$fixture_identifier"',
        "The same exact package must be otherwise viable",
        'sudo pkgutil --forget "$fixture_identifier"',
        'hdiutil create -size 64m -fs HFS+ -volname TR300PkgAltTarget',
        '/usr/sbin/installer -pkg "$fixture_pkg" -target "$fixture_alt_mount"',
        'fixture_phase=\'alternate-volume rejection\'',
        "alternate-volume package target was incorrectly accepted",
        "supports only the current system volume (/)",
        'pkgutil --volume "$fixture_alt_mount" --pkg-info "$fixture_identifier"',
        "alternate-volume rejection left a target receipt behind",
        "uninspectable eligible home was incorrectly accepted by PackageKit",
        'could not inspect eligible local home',
        "Make the declared home available and inspectable",
        "uninspectable-home rejection left its fixture receipt behind",
        'grep -Eq \'^nobody[[:space:]]+-2$\' "$runner_uid_inventory"',
        'sudo /usr/bin/dscl . -delete "/Users/$custom_fixture_user"',
        'rmdir "$custom_fixture_home/.config/tr300"',
        "test -x /usr/bin/sandbox-exec",
        '(deny file-read-data (literal "/Users"))',
        "unlistable /Users root was incorrectly accepted by preinstall",
        'hidden_home="/Users/.tr300-pkg-preinstall-',
        'double_dot_home="/Users/..tr300-pkg-preinstall-',
        '"$hidden_home|$hidden_receipt|$hidden_log"',
        '"$double_dot_home|$double_dot_receipt|$double_dot_log"',
        "dot-prefixed home was incorrectly accepted",
        "broken managed parent was incorrectly accepted by preinstall",
        "inaccessible managed parent was incorrectly accepted by preinstall",
        "case-variant managed binary was incorrectly accepted",
        "case-variant managed receipt was incorrectly accepted",
        "hdiutil create -size 64m -fs HFSX -volname TR300CaseFold",
        'mkdir -p "$casefold_home/.cargo/bin" "$casefold_home/.CARGO/BIN"',
        "dual folded-name managed owner was incorrectly accepted",
        'found multiple ASCII-case-insensitive entries matching ".cargo"',
        'test "$(shasum -a 256 "$casefold_binary" | awk \'{print $1}\')" =',
        "fixture_newline_framed=$(printf '\\nx')",
        'trailing_fixture_home="${custom_fixture_home}${fixture_newline}"',
        'NFSHomeDirectory "$trailing_fixture_home"',
        "fixture_plist_sentinel='__TR300_CI_PLIST_CAPTURE_END__'",
        'fixture_plist_with_terminator=${fixture_plist_framed%"$fixture_plist_sentinel"}',
        '"${trailing_fixture_home}${fixture_newline}"',
        "trailing-newline eligible home was incorrectly accepted by PackageKit",
        '"$custom_fixture_newline_log"',
        'test "$(shasum -a 256 "$custom_fixture_receipt" | awk \'{print $1}\')" =',
        "trailing-newline rejection left its fixture receipt behind",
        '"4294967296:$custom_fixture_out_of_range_log"',
        '"10000000000:$custom_fixture_too_wide_uid_log"',
        'UniqueID "$malformed_uid"',
        "malformed positive UID $malformed_uid was incorrectly accepted by PackageKit",
        "UID outside the supported unsigned 32-bit range",
        "malformed UID $malformed_uid left its fixture receipt behind",
        "PKInstallErrorDomain Code=112",
        "NSFilePath=./preinstall",
        "incorrectly accepted by extracted preinstall",
        "diagnose_direct_pkg_failure() {",
        'trap \'diagnose_direct_pkg_failure "$?" "$LINENO" "$BASH_COMMAND"\' ERR',
        "trap - ERR",
        "refusing unexpected diagnostic log path",
        '/usr/bin/tail -n 200 "$fixture_phase_log"',
        "/usr/bin/sed 's/^/[installer] /'",
        'exit "$failure_status"',
        "fixture_phase='custom-home ownership rejection'",
        'fixture_phase_log="$custom_fixture_log"',
        "fixture_phase='custom fixture account cleanup'",
    ):
        require(preinstall_behavior, needle, f"{CI_WORKFLOW.name} custom-home PackageKit fixture")
    dumplog_command_lines = [
        line
        for line in preinstall_behavior.splitlines()
        if "-dumplog" in line and not line.lstrip().startswith("#")
    ]
    if len(dumplog_command_lines) != 5:
        raise AssertionError(
            f"{CI_WORKFLOW.name}: every rejecting PackageKit fixture must retain detailed logs"
        )
    if preinstall_behavior.count("PKInstallErrorDomain Code=112") != 5:
        raise AssertionError(
            f"{CI_WORKFLOW.name}: every rejecting PackageKit fixture must prove packaged-script failure"
        )
    if preinstall_behavior.count("NSFilePath=./preinstall") != 5:
        raise AssertionError(
            f"{CI_WORKFLOW.name}: every rejecting PackageKit fixture must bind failure to preinstall"
        )
    if preinstall_behavior.count("incorrectly accepted by extracted preinstall") != 5:
        raise AssertionError(
            f"{CI_WORKFLOW.name}: every rejecting PackageKit fixture must bind its direct preinstall reason"
        )
    for (
        phase,
        phase_log,
        direct_target,
        direct_redirect,
        direct_diagnostic,
        installer_target,
        installer_redirect,
        postconditions,
        phase_end,
    ) in (
        (
            "fixture_phase='custom-home ownership rejection'",
            'fixture_phase_log="$custom_fixture_log"',
            '/bin/sh "$preinstall" package-path / /',
            '> "$custom_fixture_log" 2>&1; then',
            'managed receipt or link evidence at \\"${custom_fixture_home}\\"',
            '/usr/sbin/installer -pkg "$fixture_pkg" -target /',
            '>> "$custom_fixture_log" 2>&1; then',
            (
                'shasum -a 256 "$custom_fixture_receipt"',
                'test ! -e "$fixture_install_root/payload-marker"',
                'test ! -L "$fixture_install_root/payload-marker"',
                'pkgutil --pkg-info "$fixture_identifier"',
            ),
            "failed custom-home package left its fixture receipt behind",
        ),
        (
            "fixture_phase='trailing-newline home rejection'",
            'fixture_phase_log="$custom_fixture_newline_log"',
            '/bin/sh "$preinstall" package-path / /',
            '> "$custom_fixture_newline_log" 2>&1; then',
            "could not inspect eligible local home",
            '/usr/sbin/installer -pkg "$fixture_pkg" -target /',
            '>> "$custom_fixture_newline_log" 2>&1; then',
            (
                'shasum -a 256 "$custom_fixture_receipt"',
                'test ! -e "$fixture_install_root/payload-marker"',
                'test ! -L "$fixture_install_root/payload-marker"',
                'pkgutil --pkg-info "$fixture_identifier"',
            ),
            "trailing-newline rejection left its fixture receipt behind",
        ),
        (
            "fixture_phase='missing declared home rejection'",
            'fixture_phase_log="$custom_fixture_missing_home_log"',
            '/bin/sh "$preinstall" package-path / /',
            '> "$custom_fixture_missing_home_log" 2>&1; then',
            'could not inspect eligible local home \\"${custom_fixture_home}\\"',
            '/usr/sbin/installer -pkg "$fixture_pkg" -target /',
            '>> "$custom_fixture_missing_home_log" 2>&1; then',
            (
                'test ! -e "$fixture_install_root/payload-marker"',
                'test ! -L "$fixture_install_root/payload-marker"',
                'pkgutil --pkg-info "$fixture_identifier"',
            ),
            "uninspectable-home rejection left its fixture receipt behind",
        ),
        (
            'fixture_phase="malformed UID $malformed_uid rejection"',
            'fixture_phase_log="$malformed_uid_log"',
            '/bin/sh "$preinstall" package-path / /',
            '> "$malformed_uid_log" 2>&1; then',
            "UID outside the supported unsigned 32-bit range",
            '/usr/sbin/installer -pkg "$fixture_pkg" -target /',
            '>> "$malformed_uid_log" 2>&1; then',
            (
                'test ! -e "$fixture_install_root/payload-marker"',
                'test ! -L "$fixture_install_root/payload-marker"',
                'pkgutil --pkg-info "$fixture_identifier"',
            ),
            "malformed UID $malformed_uid left its fixture receipt behind",
        ),
        (
            "fixture_phase='alternate-volume rejection'",
            'fixture_phase_log="$fixture_alt_log"',
            '/bin/sh "$preinstall" package-path / "$fixture_alt_mount"',
            '> "$fixture_alt_log" 2>&1; then',
            "supports only the current system volume (/)",
            '/usr/sbin/installer -pkg "$fixture_pkg" -target "$fixture_alt_mount"',
            '>> "$fixture_alt_log" 2>&1; then',
            (
                'test ! -e "$alternate_payload"',
                'test ! -L "$alternate_payload"',
                'test ! -e "$fixture_install_root/payload-marker"',
                'test ! -L "$fixture_install_root/payload-marker"',
                'pkgutil --volume "$fixture_alt_mount" --pkg-info "$fixture_identifier"',
                'pkgutil --pkg-info "$fixture_identifier"',
            ),
            "alternate-volume rejection left a host receipt behind",
        ),
    ):
        phase_index = preinstall_behavior.index(phase)
        phase_end_index = preinstall_behavior.index(phase_end, phase_index)
        phase_block = preinstall_behavior[phase_index:phase_end_index]
        phase_marker_index = phase_block.index(phase)
        phase_log_index = phase_block.index(phase_log)
        direct_index = phase_block.index(direct_target, phase_log_index)
        direct_redirect_match = re.search(
            rf"(?m)^[ \t]*{re.escape(direct_redirect)}[ \t]*$", phase_block
        )
        if direct_redirect_match is None:
            raise AssertionError(
                f"{CI_WORKFLOW.name}: direct rejection must truncate its phase log: {phase}"
            )
        direct_redirect_index = direct_redirect_match.start()
        direct_diagnostic_index = phase_block.index(direct_diagnostic, direct_redirect_index)
        installer_index = phase_block.index(installer_target, direct_diagnostic_index)
        dumplog_index = phase_block.index("-dumplog", installer_index)
        installer_redirect_match = re.search(
            rf"(?m)^[ \t]*{re.escape(installer_redirect)}[ \t]*$", phase_block
        )
        if installer_redirect_match is None:
            raise AssertionError(
                f"{CI_WORKFLOW.name}: PackageKit rejection must append its phase log: {phase}"
            )
        installer_redirect_index = installer_redirect_match.start()
        package_failure_index = phase_block.index(
            "PKInstallErrorDomain Code=112", installer_redirect_index
        )
        package_preinstall_index = phase_block.index(
            "NSFilePath=./preinstall", package_failure_index
        )
        postcondition_index = package_preinstall_index
        for postcondition in postconditions:
            postcondition_index = phase_block.index(
                postcondition, postcondition_index + 1
            )
        if not (
            phase_marker_index
            < phase_log_index
            < direct_index
            < direct_redirect_index
            < direct_diagnostic_index
            < installer_index
            < dumplog_index
            < installer_redirect_index
            < package_failure_index
            < package_preinstall_index
            < postcondition_index
        ):
            raise AssertionError(
                f"{CI_WORKFLOW.name}: direct and packaged rejection phase is out of order: {phase}"
            )
    newline_value_index = preinstall_behavior.index(
        "fixture_newline_framed=$(printf '\\nx')"
    )
    newline_ds_index = preinstall_behavior.index(
        'NFSHomeDirectory "$trailing_fixture_home"', newline_value_index
    )
    newline_sentinel_index = preinstall_behavior.index(
        "fixture_plist_sentinel='__TR300_CI_PLIST_CAPTURE_END__'",
        newline_ds_index,
    )
    newline_install_index = preinstall_behavior.index(
        '"$custom_fixture_newline_log" 2>&1', newline_sentinel_index
    )
    newline_hash_index = preinstall_behavior.index(
        'shasum -a 256 "$custom_fixture_receipt"', newline_install_index
    )
    newline_receipt_index = preinstall_behavior.index(
        "trailing-newline rejection left its fixture receipt behind",
        newline_hash_index,
    )
    if not (
        newline_value_index
        < newline_ds_index
        < newline_sentinel_index
        < newline_install_index
        < newline_hash_index
        < newline_receipt_index
    ):
        raise AssertionError(f"{CI_WORKFLOW.name}: trailing-newline fixture is out of order")
    valid_uid_index = preinstall_behavior.index("custom_fixture_uid=3000000000")
    valid_uid_write_index = preinstall_behavior.index(
        'UniqueID "$custom_fixture_uid"', valid_uid_index
    )
    valid_uid_read_index = preinstall_behavior.index(
        'test "$(/usr/bin/id -u "$custom_fixture_user")" = "$custom_fixture_uid"',
        valid_uid_write_index,
    )
    valid_uid_packagekit_index = preinstall_behavior.index(
        '/usr/sbin/installer -pkg "$fixture_pkg" -target /', valid_uid_read_index
    )
    malformed_uid_loop_index = preinstall_behavior.index(
        "for malformed_uid_case in \\", valid_uid_packagekit_index
    )
    malformed_uid_write_index = preinstall_behavior.index(
        'UniqueID "$malformed_uid"', malformed_uid_loop_index
    )
    malformed_uid_direct_index = preinstall_behavior.index(
        '/bin/sh "$preinstall" package-path / /', malformed_uid_write_index
    )
    malformed_uid_diagnostic_index = preinstall_behavior.index(
        "UID outside the supported unsigned 32-bit range",
        malformed_uid_direct_index,
    )
    malformed_uid_packagekit_index = preinstall_behavior.index(
        '/usr/sbin/installer -pkg "$fixture_pkg" -target /',
        malformed_uid_diagnostic_index,
    )
    malformed_uid_receipt_index = preinstall_behavior.index(
        "malformed UID $malformed_uid left its fixture receipt behind",
        malformed_uid_packagekit_index,
    )
    malformed_uid_cleanup_index = preinstall_behavior.index(
        'sudo /usr/bin/dscl . -delete "/Users/$custom_fixture_user"',
        malformed_uid_receipt_index,
    )
    if not (
        valid_uid_index
        < valid_uid_write_index
        < valid_uid_read_index
        < valid_uid_packagekit_index
        < malformed_uid_loop_index
        < malformed_uid_write_index
        < malformed_uid_direct_index
        < malformed_uid_diagnostic_index
        < malformed_uid_packagekit_index
        < malformed_uid_receipt_index
        < malformed_uid_cleanup_index
    ):
        raise AssertionError(f"{CI_WORKFLOW.name}: UInt32 UID fixtures are out of order")

    resolver_executable = extract_named_run(
        resolver,
        "Resolve a trusted source before Apple credential use",
        f"{label} resolver",
    )
    for needle in (
        "WORKFLOW_SHA: ${{ github.sha }}",
        'remote_main=$(gh api \\',
        '"repos/$EXPECTED_REPOSITORY/git/ref/heads/main" --jq .object.sha)',
        '[[ "$remote_main" =~ ^[0-9a-f]{40}$ ]]',
        '[[ "$WORKFLOW_SHA" == "$remote_main" && "$source_sha" == "$remote_main" ]]',
        "refusing source outside the protected main tip",
    ):
        require(resolver, needle, f"{label} protected-main source binding")
    protected_main_index = resolver_executable.index(
        'remote_main=$(gh api \\\n'
        '  "repos/$EXPECTED_REPOSITORY/git/ref/heads/main" --jq .object.sha)'
    )
    source_output_index = resolver_executable.index('echo "source_sha=$source_sha"')
    if protected_main_index > source_output_index:
        raise AssertionError(f"{label}: protected-main equality runs after custody outputs")
    require(
        preflight,
        "needs: [resolve-release, source-custody]",
        f"{label} credential preflight resolver dependency",
    )
    require(
        build,
        "needs: [resolve-release, source-custody, prepare-installer-inputs, credential-preflight]",
        f"{label} build resolver dependency",
    )

    if workflow.count("contents: write") != 2:
        raise AssertionError(
            f"{label}: contents:write must be limited to the no-checkout "
            "draft resolver and publisher"
        )
    require(
        resolver,
        "permissions:\n      actions: read\n      contents: write",
        f"{label} private-draft resolver",
    )
    if "actions/checkout" in resolver:
        raise AssertionError(f"{label}: write-scoped resolver must not check out source")
    require(
        validate,
        "permissions:\n      actions: read\n      contents: read",
        f"{label} read-only native validator",
    )
    if "actions/checkout" in validate:
        raise AssertionError(f"{label}: native validator unexpectedly checks out source")
    if "contents: write" in validate:
        raise AssertionError(f"{label}: native validator regained release write access")
    for forbidden in (
        'releases/$RELEASE_ID',
        'gh release download "$RELEASE_TAG"',
        'validate_release_identity "$candidate_managed_release"',
    ):
        if forbidden in validate:
            raise AssertionError(
                f"{label}: read-only native validator still queries private release state"
            )
    if "This job keeps the token read-only" in validate:
        raise AssertionError(f"{label}: native validator token comment contradicts permissions")
    require(
        build,
        "source_sha: ${{ needs.resolve-release.outputs.source_sha }}",
        f"{label} build custody output",
    )
    require(build, "artifact_id: ${{ steps.macos-upload.outputs.artifact-id }}", label)
    require(build, "artifact_digest: ${{ steps.macos-upload.outputs.artifact-digest }}", label)
    if "contents: write" in build or "gh release upload" in build:
        raise AssertionError(f"{label}: build job regained release write capability")

    require(
        custody,
        "artifact_digest: ${{ steps.trusted-upload.outputs.artifact-digest }}",
        f"{label} normalized artifact digest output",
    )
    for needle in (
        "managed_installer_sha256: ${{ steps.custody.outputs.managed_installer_sha256 }}",
        "managed_installer_size: ${{ steps.custody.outputs.managed_installer_size }}",
        "dist_installer_sha256: ${{ steps.custody.outputs.dist_installer_sha256 }}",
        "dist_installer_size: ${{ steps.custody.outputs.dist_installer_size }}",
        "aarch64_archive_sha256: ${{ steps.custody.outputs.aarch64_archive_sha256 }}",
        "aarch64_archive_size: ${{ steps.custody.outputs.aarch64_archive_size }}",
        "x86_64_archive_sha256: ${{ steps.custody.outputs.x86_64_archive_sha256 }}",
        "x86_64_archive_size: ${{ steps.custody.outputs.x86_64_archive_size }}",
        "prepared_artifact_name=tr300-prepared-release-assets",
        'artifact_record "$work_directory/artifacts-before.json" "$prepared_artifact_name"',
        '"$prepared_run_id" == "$RELEASE_RUN_ID"',
        '"$prepared_repository_id" == "$repository_id"',
        '"$prepared_head_repository_id" == "$repository_id"',
        '"$prepared_head_branch" == "$RELEASE_TAG"',
        '"$prepared_head_sha" == "$EXPECTED_SHA"',
        "actions/artifacts/$prepared_artifact_id/zip",
        '[[ "sha256:$prepared_zip_sha" == "$prepared_artifact_digest" ]]',
        "selected_names = {",
        '"tr300-installer.sh",',
        '"tr300-dist-installer.sh",',
        '"tr300-aarch64-apple-darwin.tar.xz",',
        '"tr300-x86_64-apple-darwin.tar.xz",',
        '"__tr300-asset-sha256s",',
        "unexpected prepared Release artifact members",
        "os.O_EXCL",
        "os.O_NOFOLLOW",
        "stat.S_ISLNK(mode)",
        "__tr300-asset-sha256s",
        "'^[0-9a-f]{64}  tr300-installer\\.sh$'",
        "'^[0-9a-f]{64}  tr300-dist-installer\\.sh$'",
        'managed_manifest_sha256=$(awk',
        '$2 == "tr300-installer.sh" { print $1 }',
        '$2 == "tr300-dist-installer.sh" { print $1 }',
        'managed_installer_sha256=$(sha256sum -- "$managed_installer")',
        '"$managed_installer_sha256" == "$managed_manifest_sha256"',
        'dist_installer_sha256=$(sha256sum -- "$dist_installer")',
        '"$dist_installer_sha256" == "$dist_manifest_sha256"',
        'grep -Fq "tr300_dist_installer_sha256=\'${dist_installer_sha256}\'"',
        'archive_manifest_sha256=$(awk -v name="$archive_name"',
        '"$archive_sha256" == "$archive_manifest_sha256"',
        'prepared_after=$(artifact_record',
        '[[ "$prepared_after" == "$prepared_before" ]]',
        'echo "managed_installer_sha256=$managed_installer_sha256"',
        'echo "managed_installer_size=$managed_installer_size"',
        'echo "dist_installer_sha256=$dist_installer_sha256"',
        'echo "dist_installer_size=$dist_installer_size"',
        'echo "aarch64_archive_sha256=$aarch64_archive_sha256"',
        'echo "aarch64_archive_size=$aarch64_archive_size"',
        'echo "x86_64_archive_sha256=$x86_64_archive_sha256"',
        'echo "x86_64_archive_size=$x86_64_archive_size"',
        'install -m 0644 "$managed_installer"',
        '"$OUTPUT_DIRECTORY/tr300-installer.sh"',
        'install -m 0644 "$dist_installer"',
        '"$OUTPUT_DIRECTORY/tr300-dist-installer.sh"',
        '[[ ${#normalized[@]} -eq 6 ]]',
        "trusted-apple-inputs/tr300-installer.sh",
        "trusted-apple-inputs/tr300-dist-installer.sh",
    ):
        require(custody, needle, f"{label} managed-wrapper source custody")
    for asset in PREPARED_RELEASE_ARTIFACT_MEMBERS:
        require(custody, asset, f"{label} prepared Release inventory")
    require(prepare, f"uses: {CHECKOUT_ACTION}", f"{label} read-only prep")
    require(prepare, f"uses: {DOWNLOAD_ARTIFACT_ACTION}", f"{label} read-only prep")
    require(prepare, f"uses: {UPLOAD_ARTIFACT_ACTION}", f"{label} read-only prep")
    if "${{ secrets." in prepare or "environment: apple-signing" in prepare:
        raise AssertionError(f"{label}: read-only prep gained Apple credentials")
    require(
        prepare,
        "artifact-ids: ${{ needs.source-custody.outputs.artifact_id }}",
        f"{label} exact normalized input ID",
    )
    require(prepare, '[[ "$TRUSTED_INPUT_DIGEST" =~ ^[0-9a-f]{64}$ ]]',
            f"{label} raw upload digest contract")

    for job_name, job in (("preflight", preflight), ("build", build)):
        require(job, "environment: apple-signing", f"{label} {job_name} environment")
        if "actions/checkout" in job:
            raise AssertionError(f"{label}: {job_name} checks out repository code")
        if "run: scripts/" in job or "run: ./" in job:
            raise AssertionError(f"{label}: {job_name} executes repository code")
    if "uses:" in preflight:
        raise AssertionError(f"{label}: credential preflight must be action-free")
    for needle in (
        "APPLE_CERTIFICATE_P12_BASE64",
        "APPLE_INSTALLER_CERTIFICATE_P12_BASE64",
        "APPLE_API_KEY_P8_BASE64",
        "codesign --force",
        "pkgbuild --root",
        "xcrun notarytool history",
    ):
        require(preflight, needle, f"{label} complete credential preflight")
    build_uses = [line.strip().removeprefix("- ") for line in build.splitlines() if "uses:" in line]
    if build_uses != [f"uses: {UPLOAD_ARTIFACT_ACTION}"]:
        raise AssertionError(f"{label}: unexpected credentialed build actions: {build_uses!r}")
    build_custody = extract_named_step(
        build,
        "Download and revalidate the exact prepared artifact before credential use",
        f"{label} build",
    )
    for needle in (
        '[[ "$PREPARED_ARTIFACT_DIGEST" =~ ^[0-9a-f]{64}$ ]]',
        '--arg digest "sha256:$PREPARED_ARTIFACT_DIGEST"',
        '[[ "$zip_hash" == "$PREPARED_ARTIFACT_DIGEST" ]]',
        '.workflow_run.id == $run_id',
        'name == "tr300-prepared-universal-installer-inputs"',
        "unexpected prepared artifact members",
        "shasum -a 256 -c SHA256SUMS",
        "lipo \"$PREPARED_DIRECTORY/tr300-universal\" -verify_arch arm64 x86_64",
    ):
        require(build_custody, needle, f"{label} prepared input custody")
    secret_step = extract_named_step(build, "Build direct PKG and legacy DMG bridge", label)
    if build.index(build_custody) > build.index(secret_step):
        raise AssertionError(f"{label}: Apple secrets precede prepared byte custody")
    for needle in (
        "codesign --force --identifier com.qubetx.tr300",
        "pkgbuild --root",
        "xcrun notarytool submit",
        'install -m 0755 "$PREPARED_DIRECTORY/preinstall" "$package_scripts/preinstall"',
        'pkgutil --expand-full "$direct_package" "$bound_package"',
        'cmp "$PREPARED_DIRECTORY/preinstall" "$bound_package/Scripts/preinstall"',
    ):
        require(secret_step, needle, f"{label} inline system-tool builder")
    for forbidden in (
        "tr300-pkg-rollback",
        "tr300-migration-probe",
        "macos-pkg-rollback",
        "$package_scripts/postinstall",
    ):
        if forbidden in prepare or forbidden in build:
            raise AssertionError(
                f"{label}: direct PKG retained post-payload migration input {forbidden!r}"
            )

    native_validation = extract_named_step(
        validate,
        "Validate, install, and exercise the universal package",
        f"{label} native validation",
    )
    require(validate, "needs: [build, source-custody]", f"{label} native wrapper custody needs")
    require(validate, f"uses: {DOWNLOAD_ARTIFACT_ACTION}", f"{label} source input download")
    require(
        validate,
        "artifact-ids: ${{ needs.source-custody.outputs.artifact_id }}",
        f"{label} immutable source input ID",
    )
    if "name: tr300-trusted-apple-release-inputs" in validate:
        raise AssertionError(f"{label}: native validation uses mutable artifact-name selection")
    if validate.count(f"uses: {UPLOAD_ARTIFACT_ACTION}") != 1:
        raise AssertionError(f"{label}: native validation must emit two matrix proof artifacts")
    for arch in ("arm64", "x86_64"):
        require(validate, f"arch: {arch}", f"{label} native validation architecture")
    for needle in (
        "GH_TOKEN: ${{ github.token }}",
        "CURRENT_RUN_ID: ${{ github.run_id }}",
        "CURRENT_RUN_ATTEMPT: ${{ github.run_attempt }}",
        "MATRIX_ARCH: ${{ matrix.arch }}",
        "BUILD_ARTIFACT_ID: ${{ needs.build.outputs.artifact_id }}",
        "BUILD_ARTIFACT_DIGEST: ${{ needs.build.outputs.artifact_digest }}",
        "SOURCE_ARTIFACT_ID: ${{ needs.source-custody.outputs.artifact_id }}",
        "SOURCE_ARTIFACT_DIGEST: ${{ needs.source-custody.outputs.artifact_digest }}",
        "TRUSTED_SOURCE_DIRECTORY: ${{ runner.temp }}/tr300-trusted-apple-managed-inputs",
        "VALIDATION_PROOF: ${{ runner.temp }}/tr300-macos-native-validation-${{ matrix.arch }}.json",
        '[[ "$(uname -m)" == "$MATRIX_ARCH" ]]',
        "validate_build_artifact() {",
        '.id == $id and .name == "tr300-universal-macos-installer"',
        '.workflow_run.id == $run',
        '.workflow_run.repository_id == $repository',
        '.workflow_run.head_repository_id == $repository',
        '.workflow_run.head_sha == $sha',
        'actions/artifacts/$BUILD_ARTIFACT_ID"',
        'actions/artifacts/$BUILD_ARTIFACT_ID/zip"',
        '[[ "$artifact_zip_sha256" == "$BUILD_ARTIFACT_DIGEST" ]]',
        "validate_source_artifact() {",
        '.id == $id and .name == "tr300-trusted-apple-release-inputs"',
        'actions/artifacts/$SOURCE_ARTIFACT_ID"',
        'source_record_before=$(source_artifact_record "$source_artifact_before")',
        'source_record_after=$(source_artifact_record "$source_artifact_after")',
        '[[ "$source_record_after" == "$source_record_before" ]]',
        '"tr300-installer.sh",',
        '"tr300-dist-installer.sh",',
        '"tr300-aarch64-apple-darwin.tar.xz",',
        '"tr300-x86_64-apple-darwin.tar.xz",',
        "unexpected trusted Apple source input inventory",
        "unexpected native-validation artifact members",
        "native-validation artifact expands beyond the custody limit",
        "os.O_EXCL",
        "os.O_NOFOLLOW",
        "stat.S_ISLNK(mode)",
        'build_record_before=$(build_artifact_record "$build_artifact_before")',
        'build_record_after=$(build_artifact_record "$build_artifact_after")',
        '[[ "$build_record_after" == "$build_record_before" ]]',
        "schema_version: 1",
        "workflow_run_attempt: $attempt",
        "build_artifact_digest: $build_digest",
        "managed_installer: {digest: $managed_digest, size: $managed_size}",
        "dist_installer: {digest: $dist_digest, size: $dist_size}",
        "aarch64_archive: {digest: $arm_digest, size: $arm_size}",
        "x86_64_archive: {digest: $intel_digest, size: $intel_size}",
        'pkgutil --expand-full "$pkg" "$expanded_pkg"',
        'lsbom -s -f "$expanded_pkg/Bom"',
        "unexpected direct-PKG payload inventory",
        "unexpected direct-PKG Scripts inventory",
        'test -f "$expanded_pkg/Scripts/preinstall"',
        'test ! -L "$expanded_pkg/Scripts/preinstall"',
        "postinstall tr300-pkg-rollback tr300-migration-probe",
        'rm -rf "$expanded_pkg"',
        'custom_fixture_home=$(mktemp -d \\',
        '"/private/tmp/tr300 signed pkg custom home.XXXXXXXX")',
        "custom_fixture_uid=3000000000",
        'UniqueID "$custom_fixture_uid"',
        'test "$(/usr/bin/id -u "$custom_fixture_user")" = "$custom_fixture_uid"',
        '/usr/bin/dscl -plist . -read "/Users/$custom_fixture_user"',
        '/usr/bin/plutil -lint "$custom_fixture_plist"',
        "'Print :dsAttrTypeStandard\\:NFSHomeDirectory:0'",
        '/usr/bin/env -u SUDO_USER -u SUDO_UID',
        '/usr/sbin/installer -pkg "$pkg" -target /',
        "signed PKG incorrectly accepted custom-home managed evidence",
        "custom-home rejection left the signed payload behind",
        "custom-home rejection left the signed package receipt behind",
        'sudo /usr/bin/dscl . -delete "/Users/$custom_fixture_user"',
        'rmdir "$custom_fixture_home/.config/tr300"',
        'baseline_managed_directory="$RUNNER_TEMP/tr300-managed-v422"',
        "baseline_managed_digest=eed23f5e71bd3fa455bd439e4313f596b99067d5145fbd5e01652ce4cb1c1751",
        "baseline_managed_size=14904",
        "baseline_dist_digest=497b238d725d61145888d430a1137cb5eb2362341fe0b3e4af0b8b81bd890444",
        "baseline_dist_size=55485",
        "baseline_archive_digest=4b4ab3e05dc3719cac9d1e07a3202616db359f9d3c8221625362d805ef3f226c",
        "baseline_archive_size=1390748",
        "baseline_archive_digest=127a8a52c3b5c5db257dcfc81b0ab96972bf3a8589fc8a3af7cc00cef96ec739",
        "baseline_archive_size=1563320",
        'gh api "repos/$REPOSITORY/releases/tags/v4.2.2"',
        "db0f538c82961569a7118b105a20e967b15476f0 false",
        'baseline_wrapper_before=$(release_asset_record',
        '"$baseline_managed_release" tr300-installer.sh)',
        'baseline_dist_before=$(release_asset_record',
        '"$baseline_managed_release" tr300-dist-installer.sh)',
        'baseline_archive_before=$(release_asset_record',
        '"$baseline_managed_release" "$baseline_archive_name")',
        '"$baseline_wrapper_id" =~ ^[1-9][0-9]*$',
        '"$baseline_dist_record_digest" == "sha256:$baseline_dist_digest"',
        '"$baseline_archive_record_digest" == "sha256:$baseline_archive_digest"',
        '--pattern tr300-installer.sh --dir "$baseline_managed_directory"',
        '[[ "$baseline_managed_actual" == "$baseline_managed_digest" ]]',
        'baseline_checksum_bin="$baseline_managed_directory/checksum-bin"',
        'baseline_checksum_shim="$baseline_checksum_bin/sha256sum"',
        'set -C',
        'exec /usr/bin/shasum -a 256 "$@"',
        '[[ -f "$baseline_checksum_shim" && ! -L "$baseline_checksum_shim"',
        '"$baseline_checksum_shim" -b "$checksum_probe"',
        '"$shim_probe" =~ ^[0-9a-f]{64}$',
        '"$resolved_checksum_shim" == "$baseline_checksum_shim"',
        'PATH="$baseline_checksum_bin:$PATH" sh "$baseline_managed_installer"',
        'baseline_managed_after="$baseline_managed_directory/release-after.json"',
        '"$baseline_managed_after" tr300-dist-installer.sh)',
        '"$baseline_managed_after" "$baseline_archive_name")',
        '"$baseline_wrapper_after" == "$baseline_wrapper_before"',
        '"$baseline_dist_after" == "$baseline_dist_before"',
        '"$baseline_archive_after" == "$baseline_archive_before"',
        'test "$("$HOME/.cargo/bin/tr300" --version)" = \'tr300 4.2.2\'',
        "Rerun the managed installer to refresh this copy to a receipt-aware version",
        'sh "$managed_installer"',
        'test "$("$HOME/.cargo/bin/tr300" --version)" = "tr300 $RELEASE_VERSION"',
        "printf '2\\ny\\n' | \"$HOME/.cargo/bin/tr300\" uninstall",
        "gh release download v4.2.2",
        "baseline_pkg_sha256=717f233eedfac679a507bc9ce2b16ba195f050e289295665a8c68d83ba10c979",
        "baseline_pkg_size=7571628",
        "baseline_pkg_sidecar_sha256=1530d1f9fd73a7d7ab84de30fe4929b3ab5ecacc0715f8c461cd0ce2c76b5aec",
        "baseline_pkg_sidecar_size=99",
        '.targetCommitish == "db0f538c82961569a7118b105a20e967b15476f0"',
        '.isDraft == false and .isPrerelease == false',
        'select(.name == "tr300-universal-apple-darwin.pkg")',
        'select(.name == "tr300-universal-apple-darwin.pkg.sha256")',
        '.digest == $pkg_digest and .size == $pkg_size',
        '.digest == $sidecar_digest and .size == $sidecar_size',
        '"$baseline_pkg_actual_sha256" == "$baseline_pkg_sha256"',
        '"$baseline_sidecar_actual_sha256" == "$baseline_pkg_sidecar_sha256"',
        'sudo installer -pkg "$baseline_pkg" -target /',
        'test "$(/usr/local/bin/tr300 --version)" = \'tr300 4.2.2\'',
        "SOURCE_MANAGED_INSTALLER_SHA256: ${{ needs.source-custody.outputs.managed_installer_sha256 }}",
        "SOURCE_MANAGED_INSTALLER_SIZE: ${{ needs.source-custody.outputs.managed_installer_size }}",
        "SOURCE_DIST_INSTALLER_SHA256: ${{ needs.source-custody.outputs.dist_installer_sha256 }}",
        "SOURCE_DIST_INSTALLER_SIZE: ${{ needs.source-custody.outputs.dist_installer_size }}",
        "SOURCE_AARCH64_ARCHIVE_SHA256: ${{ needs.source-custody.outputs.aarch64_archive_sha256 }}",
        "SOURCE_AARCH64_ARCHIVE_SIZE: ${{ needs.source-custody.outputs.aarch64_archive_size }}",
        "SOURCE_X86_64_ARCHIVE_SHA256: ${{ needs.source-custody.outputs.x86_64_archive_sha256 }}",
        "SOURCE_X86_64_ARCHIVE_SIZE: ${{ needs.source-custody.outputs.x86_64_archive_size }}",
        "release_asset_record() {",
        '[.id, .digest, .size] | @tsv',
        '.tag_name == $tag and .target_commitish == $sha',
        '.draft == $draft and .prerelease == false',
        'managed_installer="$trusted_managed_installer"',
        'candidate_dist_installer="$trusted_dist_installer"',
        'candidate_archive="$TRUSTED_SOURCE_DIRECTORY/$candidate_archive_name"',
        '[[ -f "$candidate_dist_installer" && ! -L "$candidate_dist_installer"',
        '[[ -f "$candidate_archive" && ! -L "$candidate_archive"',
        'grep -Fq "tr300_dist_installer_sha256=\'${SOURCE_DIST_INSTALLER_SHA256}\'"',
        '/usr/bin/env -u TR300_GITHUB_TOKEN -u GITHUB_TOKEN -u GH_TOKEN',
        'TR300_DIST_INSTALLER_PATH="$candidate_dist_installer"',
        'TR300_DOWNLOAD_URL="file://$TRUSTED_SOURCE_DIRECTORY"',
    ):
        require(native_validation, needle, f"{label} native package inventory/upgrade")
    native_uint32_uid_index = native_validation.index("custom_fixture_uid=3000000000")
    native_uint32_write_index = native_validation.index(
        'UniqueID "$custom_fixture_uid"', native_uint32_uid_index
    )
    native_uint32_read_index = native_validation.index(
        'test "$(/usr/bin/id -u "$custom_fixture_user")" = "$custom_fixture_uid"',
        native_uint32_write_index,
    )
    native_uint32_packagekit_index = native_validation.index(
        '/usr/sbin/installer -pkg "$pkg" -target /', native_uint32_read_index
    )
    if not (
        native_uint32_uid_index
        < native_uint32_write_index
        < native_uint32_read_index
        < native_uint32_packagekit_index
    ):
        raise AssertionError(f"{label}: native UInt32 UID fixture is out of order")
    if re.search(r'^sh "\$baseline_managed_installer"$', native_validation, re.MULTILINE):
        raise AssertionError(f"{label}: historical wrapper escaped its checksum shim")
    for needle in (
        "name: Upload exact native validation proof",
        "name: tr300-macos-native-validation-${{ matrix.arch }}-${{ github.run_attempt }}",
        "path: ${{ runner.temp }}/tr300-macos-native-validation-${{ matrix.arch }}.json",
    ):
        require(validate, needle, f"{label} native matrix proof upload")
    source_artifact_before_index = native_validation.index(
        'actions/artifacts/$SOURCE_ARTIFACT_ID"'
    )
    source_byte_index = native_validation.index(
        '"$SOURCE_MANAGED_INSTALLER_SHA256" ]] || exit 1',
        source_artifact_before_index,
    )
    build_artifact_before_index = native_validation.index(
        'actions/artifacts/$BUILD_ARTIFACT_ID"'
    )
    build_zip_index = native_validation.index(
        'actions/artifacts/$BUILD_ARTIFACT_ID/zip"', build_artifact_before_index
    )
    package_trust_index = native_validation.index(
        "shasum -a 256 -c tr300-universal-apple-darwin.pkg.sha256",
        build_zip_index,
    )
    build_artifact_after_index = native_validation.index(
        'build_record_after=$(build_artifact_record "$build_artifact_after")',
        package_trust_index,
    )
    source_artifact_after_index = native_validation.index(
        'source_record_after=$(source_artifact_record "$source_artifact_after")',
        build_artifact_after_index,
    )
    proof_write_index = native_validation.index(
        "schema_version: 1", source_artifact_after_index
    )
    if not (
        source_artifact_before_index
        < source_byte_index
        < build_artifact_before_index
        < build_zip_index
        < package_trust_index
        < build_artifact_after_index
        < source_artifact_after_index
        < proof_write_index
    ):
        raise AssertionError(f"{label}: native build custody/proof sequence is out of order")
    if native_validation.index(
        'pkgutil --expand-full "$pkg" "$expanded_pkg"'
    ) > native_validation.index('sudo installer -pkg "$pkg" -target /'):
        raise AssertionError(f"{label}: native package inventory runs after installation")
    managed_baseline_index = native_validation.index('sh "$baseline_managed_installer"')
    baseline_record_before_index = native_validation.index(
        'baseline_wrapper_before=$(release_asset_record'
    )
    baseline_shim_index = native_validation.index(
        'baseline_checksum_shim="$baseline_checksum_bin/sha256sum"',
        baseline_record_before_index,
    )
    baseline_record_after_index = native_validation.index(
        'baseline_managed_after="$baseline_managed_directory/release-after.json"',
        managed_baseline_index,
    )
    if not (
        baseline_record_before_index
        < baseline_shim_index
        < managed_baseline_index
        < baseline_record_after_index
    ):
        raise AssertionError(
            f"{label}: v4.2.2 wrapper/raw/archive record sandwich is out of order"
        )
    managed_rejection_index = native_validation.index(
        'sudo installer -pkg "$pkg" -target /', managed_baseline_index
    )
    managed_refresh_index = native_validation.index(
        'sh "$managed_installer"', managed_rejection_index
    )
    candidate_local_path_index = native_validation.index(
        'managed_installer="$trusted_managed_installer"',
        managed_rejection_index,
    )
    candidate_tokenless_index = native_validation.index(
        '/usr/bin/env -u TR300_GITHUB_TOKEN -u GITHUB_TOKEN -u GH_TOKEN',
        candidate_local_path_index,
    )
    managed_complete_index = native_validation.index(
        "printf '2\\ny\\n' | \"$HOME/.cargo/bin/tr300\" uninstall",
        managed_refresh_index,
    )
    fresh_pkg_index = native_validation.index(
        'sudo installer -pkg "$pkg" -target /', managed_complete_index
    )
    if not (
        managed_baseline_index
        < managed_rejection_index
        and source_byte_index
        < candidate_local_path_index
        < candidate_tokenless_index
        < managed_refresh_index
        < managed_complete_index
        < fresh_pkg_index
    ):
        raise AssertionError(
            f"{label}: public managed baseline recovery does not precede fresh PKG install"
        )
    baseline_install_index = native_validation.index('sudo installer -pkg "$baseline_pkg" -target /')
    baseline_metadata_index = native_validation.index(
        "baseline_pkg_sha256=717f233eedfac679a507bc9ce2b16ba195f050e289295665a8c68d83ba10c979"
    )
    baseline_download_index = native_validation.index(
        "gh release download v4.2.2", baseline_metadata_index
    )
    baseline_bytes_index = native_validation.index(
        '"$baseline_pkg_actual_sha256" == "$baseline_pkg_sha256"',
        baseline_download_index,
    )
    baseline_sidecar_check_index = native_validation.index(
        "shasum -a 256 -c tr300-universal-apple-darwin.pkg.sha256",
        baseline_bytes_index,
    )
    baseline_signature_index = native_validation.index(
        'baseline_signature=$(pkgutil --check-signature "$baseline_pkg"',
        baseline_sidecar_check_index,
    )
    baseline_notary_index = native_validation.index(
        'xcrun stapler validate "$baseline_pkg"', baseline_signature_index
    )
    if not (
        baseline_metadata_index
        < baseline_download_index
        < baseline_bytes_index
        < baseline_sidecar_check_index
        < baseline_signature_index
        < baseline_notary_index
        < baseline_install_index
    ):
        raise AssertionError(
            f"{label}: v4.2.2 native PKG pins do not precede trust and install checks"
        )
    candidate_upgrade_index = native_validation.index(
        'sudo installer -pkg "$pkg" -target /', baseline_install_index
    )
    same_version_repair_index = native_validation.index(
        'native_hash=$(shasum -a 256 /usr/local/bin/tr300'
    )
    if not baseline_install_index < candidate_upgrade_index < same_version_repair_index:
        raise AssertionError(f"{label}: same-version repair precedes the v4.2.2 native upgrade")

    require(publisher, "permissions:\n      actions: read\n      contents: write", f"{label} publisher")
    require(publisher, "actions: read", f"{label} publisher artifact custody")
    require(publisher, "environment: release-publishing", f"{label} publisher environment")
    require(
        publisher,
        "needs: [build, validate, source-custody]",
        f"{label} publisher managed-wrapper custody needs",
    )
    require(
        publisher,
        "SOURCE_MANAGED_INSTALLER_SHA256: ${{ needs.source-custody.outputs.managed_installer_sha256 }}",
        f"{label} publisher source-bound wrapper hash",
    )
    require(
        publisher,
        "SOURCE_MANAGED_INSTALLER_SIZE: ${{ needs.source-custody.outputs.managed_installer_size }}",
        f"{label} publisher source-bound wrapper size",
    )
    for needle in (
        "SOURCE_DIST_INSTALLER_SHA256: ${{ needs.source-custody.outputs.dist_installer_sha256 }}",
        "SOURCE_DIST_INSTALLER_SIZE: ${{ needs.source-custody.outputs.dist_installer_size }}",
        "SOURCE_AARCH64_ARCHIVE_SHA256: ${{ needs.source-custody.outputs.aarch64_archive_sha256 }}",
        "SOURCE_AARCH64_ARCHIVE_SIZE: ${{ needs.source-custody.outputs.aarch64_archive_size }}",
        "SOURCE_X86_64_ARCHIVE_SHA256: ${{ needs.source-custody.outputs.x86_64_archive_sha256 }}",
        "SOURCE_X86_64_ARCHIVE_SIZE: ${{ needs.source-custody.outputs.x86_64_archive_size }}",
    ):
        require(publisher, needle, f"{label} publisher transitive source custody")
    require(
        publisher,
        "CURRENT_RUN_ATTEMPT: ${{ github.run_attempt }}",
        f"{label} publisher native-proof attempt binding",
    )
    if "actions/checkout" in publisher:
        raise AssertionError(f"{label}: publisher must not check out repository source")
    uses = [
        line.strip().removeprefix("- ")
        for line in publisher.splitlines()
        if "uses:" in line
    ]
    if uses:
        raise AssertionError(f"{label}: unexpected publisher actions: {uses!r}")
    if publisher.count("GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}") != 1:
        raise AssertionError(f"{label}: publisher token must exist on one upload step")
    if publisher.count("${{ secrets.GITHUB_TOKEN }}") != 1 or "${{ github.token }}" in publisher:
        raise AssertionError(f"{label}: publisher token escaped the upload step")
    if len(extract_run_blocks(publisher)) != 1:
        raise AssertionError(f"{label}: publisher must contain one audited run block")

    executable = extract_named_run(
        publisher,
        "Bind exact supplements, upload macOS assets, and publish the draft",
        f"{label} publisher",
    )
    previous_digest_compare_index = -1
    for digest_variable in ("validated_digest", "published_digest"):
        digest_match_index = executable.index(
            '[[ "$digest" =~ ^sha256:([0-9a-f]{64})$ ]]',
            previous_digest_compare_index + 1,
        )
        digest_capture_index = executable.index(
            f"{digest_variable}=${{BASH_REMATCH[1]}}", digest_match_index
        )
        digest_size_index = executable.index(
            '[[ "$size" =~ ^[1-9][0-9]*$ ]]', digest_capture_index
        )
        digest_compare_index = executable.index(
            f'[[ "$actual" == "${digest_variable}" ]]', digest_size_index
        )
        if not (
            digest_match_index
            < digest_capture_index
            < digest_size_index
            < digest_compare_index
        ):
            raise AssertionError(f"{label}: {digest_variable} capture is out of order")
        previous_digest_compare_index = digest_compare_index
    if (
        '[[ "$digest" =~ ^sha256:([0-9a-f]{64})$ &&' in executable
        or '[[ "$actual" == "${BASH_REMATCH[1]}" ]]' in executable
    ):
        raise AssertionError(f"{label}: publisher byte loop clobbers BASH_REMATCH")
    require(executable, STABLE_TAG_PATTERN, f"{label} publisher")
    require(executable, "EXPECTED_SHA", f"{label} publisher")
    require(executable, "git/ref/tags/$tag", f"{label} publisher")
    require(executable, "git/tags/$source_sha", f"{label} publisher")
    require(executable, "releases/$RELEASE_ID", f"{label} publisher")
    require(
        executable,
        ".id == $release_id and .tag_name == $tag",
        f"{label} publisher",
    )
    require(executable, "actions/artifacts/$MACOS_ARTIFACT_ID/zip", f"{label} own artifact ID")
    require(executable, '"sha256:$MACOS_ARTIFACT_DIGEST"', f"{label} own REST digest")
    require(executable, '[[ "$macos_zip_hash" == "$MACOS_ARTIFACT_DIGEST" ]]', f"{label} own ZIP digest")
    require(executable, "actions/workflows/windows-installers.yml", f"{label} Windows run identity")
    require(executable, "actions/artifacts/$windows_artifact_id/zip", f"{label} Windows artifact ID")
    require(executable, '[[ "sha256:$windows_zip_hash" == "$windows_artifact_digest" ]]', f"{label} Windows ZIP digest")
    for needle in (
        "actions/workflows/windows-installer-validation.yml",
        '"$run_name" == "Windows Installer Validation"',
        '"$run_path" == .github/workflows/windows-installer-validation.yml',
        '"$event" == "$expected_event"',
        "windows-installer-validation-proof",
        "validation-proof-artifact-before.json",
        "validation-input-artifact-pre-publish.json",
        "windows-installer-validation.json",
        "validation-input-manifest.json",
        ".validation_run_attempt == $validation_attempt",
        ".validation_input_artifact_id | type == \"number\"",
        "([.assets[] | {id, name, size, digest}] | sort_by(.name))",
        "($manifest[0].release_assets | sort_by(.name))",
        '[[ "$validation_run_attempt" == "$bound_validation_run_attempt" ]]',
        '[[ "$validated_windows_run_attempt" == "$windows_run_attempt" ]]',
        "release-patch-response.json",
        "patch_status=$?",
        "release-published.json",
        '--arg managed_digest "sha256:$SOURCE_MANAGED_INSTALLER_SHA256"',
        '--argjson managed_size "$SOURCE_MANAGED_INSTALLER_SIZE"',
        '--arg dist_digest "sha256:$SOURCE_DIST_INSTALLER_SHA256"',
        '--argjson dist_size "$SOURCE_DIST_INSTALLER_SIZE"',
        '--arg arm_digest "sha256:$SOURCE_AARCH64_ARCHIVE_SHA256"',
        '--argjson arm_size "$SOURCE_AARCH64_ARCHIVE_SIZE"',
        '--arg intel_digest "sha256:$SOURCE_X86_64_ARCHIVE_SHA256"',
        '--argjson intel_size "$SOURCE_X86_64_ARCHIVE_SIZE"',
        '([.release_assets[] | select(.name == "tr300-installer.sh")] | length) == 1',
        '.digest == $managed_digest and .size == $managed_size',
        '([.release_assets[] | select(.name == "tr300-dist-installer.sh")] | length) == 1',
        '.digest == $dist_digest and .size == $dist_size',
        'select(.name == "tr300-aarch64-apple-darwin.tar.xz")',
        '.digest == $arm_digest and .size == $arm_size',
        'select(.name == "tr300-x86_64-apple-darwin.tar.xz")',
        '.digest == $intel_digest and .size == $intel_size',
        '([.assets[] | select(.name == "tr300-installer.sh")] | length) == 1',
        '([.assets[] | select(.name == "tr300-dist-installer.sh")] | length) == 1',
        'actions/runs/$CURRENT_RUN_ID/artifacts?per_page=100',
        "proof_artifact_record() {",
        "direct_proof_record() {",
        "validate_native_proof() {",
        'proof_name="tr300-macos-native-validation-${arch}-${CURRENT_RUN_ATTEMPT}"',
        'proof_file="tr300-macos-native-validation-${arch}.json"',
        'actions/artifacts/$proof_id"',
        'actions/artifacts/$proof_id/zip"',
        '[[ "sha256:$proof_zip_sha" == "$proof_digest" ]]',
        "unexpected native proof members",
        "unsafe native proof member",
        '.workflow_run_id == $run and .workflow_run_attempt == $attempt',
        '.build_artifact_id == $build_id',
        '.build_artifact_digest == $build_digest',
        '.managed_installer == {digest: $managed_digest, size: $managed_size}',
        '.dist_installer == {digest: $dist_digest, size: $dist_size}',
        '.aarch64_archive == {digest: $arm_digest, size: $arm_size}',
        '.x86_64_archive == {digest: $intel_digest, size: $intel_size}',
        'validate_native_proof "$proof_directory/$proof_file" "$arch"',
        '[[ "$(direct_proof_record "$proof_after_metadata")" == "$proof_before" ]]',
    ):
        require(executable, needle, f"{label} Windows acceptance custody")
    proof_inventory_index = executable.index(
        'actions/runs/$CURRENT_RUN_ID/artifacts?per_page=100'
    )
    proof_validation_index = executable.index(
        'validate_native_proof "$proof_directory/$proof_file" "$arch"',
        proof_inventory_index,
    )
    manifest_custody_index = executable.index(
        '([.release_assets[] | select(.name == "tr300-installer.sh")] | length) == 1'
    )
    manifest_transitive_indices = [
        executable.index(
            f'([.release_assets[] | select(.name == "{asset}")]',
            manifest_custody_index,
        )
        for asset in (
            "tr300-dist-installer.sh",
            "tr300-aarch64-apple-darwin.tar.xz",
            "tr300-x86_64-apple-darwin.tar.xz",
        )
    ]
    draft_custody_index = executable.index(
        '([.assets[] | select(.name == "tr300-installer.sh")] | length) == 1'
    )
    draft_transitive_indices = [
        executable.index(
            f'([.assets[] | select(.name == "{asset}")]', draft_custody_index
        )
        for asset in (
            "tr300-dist-installer.sh",
            "tr300-aarch64-apple-darwin.tar.xz",
            "tr300-x86_64-apple-darwin.tar.xz",
        )
    ]
    upload_index = executable.index("gh release upload")
    publish_index = executable.index("-F draft=false -f make_latest=true")
    if not (
        proof_inventory_index
        < proof_validation_index
        < manifest_custody_index
        < max(manifest_transitive_indices)
        < draft_custody_index
        < max(draft_transitive_indices)
        < upload_index
        < publish_index
    ):
        raise AssertionError(
            f"{label}: managed-wrapper manifest/draft custody does not precede publication"
        )
    require(executable, "actions/workflows/ci.yml/runs?event=push", f"{label} exact CI rebind")
    if executable.count("(.release_assets | length) == 30") != 1:
        raise AssertionError(
            f"{label}: publisher must require exactly 30 manifest release assets"
        )
    require(executable, '[[ ${#existing_assets[@]} -eq 30 ]]', f"{label} draft inventory")
    require(executable, '[[ ${#final_entries[@]} -eq 34 ]]', f"{label} final inventory")
    require(executable, "sha256sum -c sha256.sum", f"{label} final checksums")
    require(executable, "-F draft=false -f make_latest=true", f"{label} atomic promotion")
    require(executable, "find \"$ASSET_DIRECTORY\"", f"{label} publisher")
    require(executable, "sha256sum -- \"$payload\"", f"{label} publisher")
    require(executable, "gh release upload", f"{label} publisher")
    if "--clobber" in executable or '"$ASSET_DIRECTORY"/*' in executable:
        raise AssertionError(f"{label}: publisher upload is not immutable/fixed-path")
    for asset in MACOS_RELEASE_ASSETS:
        require(
            executable,
            f'"$ASSET_DIRECTORY/{asset}"',
            f"{label} fixed release upload",
        )
    for forbidden in (
        "actions/checkout",
        "cargo ",
        "scripts/",
        "sudo ",
        "powershell",
        "pwsh",
    ):
        if forbidden in publisher:
            raise AssertionError(
                f"{label}: publisher unexpectedly executes repository/package content: "
                f"{forbidden!r}"
            )


def check_release_token_boundary(workflow: str) -> None:
    label = RELEASE_WORKFLOW.name
    build_local = extract_job(workflow, "build-local-artifacts", label)
    apple_signer = extract_job(workflow, "sign-apple-artifacts", label)
    prepare = extract_job(workflow, "prepare-host-assets", label)
    host = extract_job(workflow, "host", label)
    announce = extract_job(workflow, "announce", label)

    if "${{ secrets.APPLE_" in build_local or "scripts/sign-notarize" in build_local:
        raise AssertionError(f"{label}: cargo-dist build runner regained Apple secrets")
    require(apple_signer, "environment: apple-signing", f"{label} Apple environment")
    if "actions/checkout" in apple_signer or "run: scripts/" in apple_signer:
        raise AssertionError(f"{label}: Apple signer executes repository checkout/code")
    signer_uses = [
        line.strip().removeprefix("- ")
        for line in apple_signer.splitlines()
        if "uses:" in line
    ]
    if signer_uses != [
        f"uses: {DOWNLOAD_ARTIFACT_ACTION}",
        f"uses: {UPLOAD_ARTIFACT_ACTION}",
    ]:
        raise AssertionError(f"{label}: unexpected fresh Apple signer actions: {signer_uses!r}")

    require(prepare, "permissions:\n      actions: read\n      contents: read", label)
    require(prepare, f"uses: {CHECKOUT_ACTION}", f"{label} read-only preparation")
    require(prepare, f"uses: {DOWNLOAD_ARTIFACT_ACTION}", f"{label} read-only preparation")
    require(prepare, f"uses: {UPLOAD_ARTIFACT_ACTION}", f"{label} read-only preparation")
    require(prepare, "artifact_id: ${{ steps.prepared-upload.outputs.artifact-id }}", label)
    require(prepare, "artifact_digest: ${{ steps.prepared-upload.outputs.artifact-digest }}", label)
    require(prepare, "Render and validate the fixed 24-asset initial release", label)
    require(
        prepare,
        "raw_cargo_dist_payloads = {",
        f"{label} raw cargo-dist sidecar inventory",
    )
    require(
        prepare,
        'expected_ending = b"\\n\\n" if payload_name in raw_cargo_dist_payloads else b"\\n"',
        f"{label} exact raw cargo-dist sidecar bytes",
    )
    require(
        prepare,
        'checksum_path.read_bytes() != canonical_checksum + b"\\n"',
        f"{label} exact raw cargo-dist aggregate bytes",
    )
    require(
        prepare,
        "sidecar.write_bytes(contents)",
        f"{label} canonical public sidecar rewrite",
    )
    require(
        prepare,
        "checksum_path.write_bytes(canonical_checksum)",
        f"{label} canonical aggregate rewrite",
    )
    require(prepare, "[[ ${#prepared[@]} -eq $((${#public_assets[@]} + 3)) ]]", label)
    if "contents: write" in prepare or "GH_TOKEN" in prepare:
        raise AssertionError(f"{label}: source preparation gained release write capability")

    host_header = host[: host.index("    steps:")]
    if "GH_TOKEN" in host_header:
        raise AssertionError(f"{label}: host regained a job-wide write token")
    require(host, "environment: release-publishing", f"{label} publisher environment")
    require(host, "permissions:\n      actions: read\n      contents: write", label)
    if "uses:" in host or "actions/checkout" in host or "scripts/" in host or "dist host" in host:
        raise AssertionError(f"{label}: fresh publisher executes source/actions/cargo-dist")
    if host.count("GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}") != 2:
        raise AssertionError(f"{label}: publisher token scope changed")
    require(host, "managed_installer_sha256: ${{ steps.publish.outputs.managed_installer_sha256 }}", label)
    custody = extract_named_run(
        host, "Download and verify the exact prepared release artifact", label
    )
    publisher = extract_named_run(
        host, "Rebind protected source and publish the fixed 24 assets", label
    )
    for needle in (
        "actions/artifacts/$ARTIFACT_ID/zip",
        '"sha256:$ARTIFACT_DIGEST"',
        '[[ "$zip_sha" == "$ARTIFACT_DIGEST" ]]',
        ".workflow_run.id == $run",
        "unexpected prepared release members",
        "sha256sum -c __tr300-asset-sha256s",
    ):
        require(custody, needle, f"{label} exact prepared custody")
    for needle in (
        STABLE_TAG_PATTERN,
        "git/ref/heads/main",
        "actions/workflows/ci.yml/runs?event=push",
        "gh api --paginate --slurp",
        "releases?per_page=100",
        "[.[][] | select(.tag_name == $tag)] | length == 0",
        "gh release create",
        "--draft --verify-tag --target",
        "created release is not the unique exact private draft",
        'gh api "repos/$EXPECTED_REPOSITORY/releases/$release_id"',
        "([.assets[].name] | sort) == $expected_assets",
        'awk \'$2 == "tr300-installer.sh" { print $1 }\'',
    ):
        require(publisher, needle, f"{label} fixed draft publisher")
    if "releases/tags/$RELEASE_TAG" in publisher:
        raise AssertionError(f"{label}: private draft publisher uses draft-blind tag lookup")
    if '"$ASSET_DIRECTORY"/*' in publisher or "--clobber" in publisher:
        raise AssertionError(f"{label}: publisher regained glob/clobber upload")
    if "smoke-published-managed-linux:" in workflow:
        raise AssertionError(f"{label}: public smoke reintroduced before draft finalization")
    require(announce, "permissions: {}", f"{label} private handoff")
    require(announce, "Confirm private draft handoff", f"{label} private handoff")


def check_crates_token_boundary(workflow: str) -> None:
    label = CRATES_WORKFLOW.name
    trigger = workflow[: workflow.index("permissions:")]
    for needle in (
        "push:",
        "branches: [main]",
    ):
        require(trigger, needle, f"{label} automatic main-push trigger")
    if "workflow_run:" in trigger or "github.event.workflow_run" in workflow:
        raise AssertionError(
            f"{label}: crates.io trusted OIDC does not support workflow_run events"
        )
    require(
        workflow,
        "if: github.event_name == 'push' && github.ref == 'refs/heads/main' && github.repository == 'QubeTX/qube-machine-report'",
        f"{label} automatic trusted-main publish gate",
    )
    probe_job = extract_job(workflow, "probe-trusted-publisher", label)
    validation_job = extract_job(workflow, "validate-package", label)
    publish_job = extract_job(workflow, "publish", label)
    validation_package = extract_named_step(
        validation_job, "Package and registry-credential-free dry-run", label
    )
    source_gate = extract_named_step(
        validation_job, "Require successful CI for this exact main commit", label
    )
    candidate = extract_named_step(
        publish_job, "Repackage without execution and prove exact candidate bytes", label
    )
    auth = extract_named_step(
        publish_job, "Mint the short-lived crates.io publish token", label
    )
    publish_step = extract_named_step(
        publish_job, "Publish without executing package code", label
    )
    adjudication = extract_named_step(
        publish_job, "Adjudicate public bytes without the publish token", label
    )

    require(workflow, "cancel-in-progress: false", f"{label} irreversible publish lock")
    for job_name, job in (("probe", probe_job), ("publish", publish_job)):
        require(job, "environment: crates-io", f"{label} {job_name} environment")
        require(job, "runs-on: ubuntu-24.04", f"{label} {job_name} runner")

    for forbidden in (
        "configure-trusted-publisher",
        "enable-trusted-publishing-only",
        "${{ secrets.CARGO_REGISTRY_TOKEN }}",
        "configure_trusted_publisher",
    ):
        if forbidden in workflow:
            raise AssertionError(f"{label}: completed one-time bootstrap remained: {forbidden}")

    probe_uses = [line.strip() for line in probe_job.splitlines() if "uses:" in line]
    if probe_uses != [f"uses: {CRATES_AUTH_ACTION}"]:
        raise AssertionError(f"{label}: unexpected OIDC probe actions: {probe_uses!r}")
    require(probe_job, "id-token: write", f"{label} OIDC probe permission")
    require(workflow, "probe_trusted_publisher", f"{label} post-deletion OIDC probe mode")

    if "id-token: write" in validation_job or "CARGO_REGISTRY_TOKEN" in validation_job:
        raise AssertionError(f"{label}: validation regained a credential")
    for needle in (
        "EVENT_NAME: ${{ github.event_name }}",
        "SOURCE_REF: ${{ github.ref }}",
        "SOURCE_SHA: ${{ github.sha }}",
        "PUBLISH_WORKFLOW_RUN_ID: ${{ github.run_id }}",
        "CI_STATE_PATH: ${{ runner.temp }}/ci-runs.json",
        '"$EVENT_NAME" == push',
        '"$EXPECTED_REPOSITORY" == QubeTX/qube-machine-report',
        '"$SOURCE_REF" == refs/heads/main',
        '"$PUBLISH_WORKFLOW_RUN_ID" =~ ^[1-9][0-9]*$',
        "actions/workflows/ci.yml",
        "runs?event=push&head_sha=$SOURCE_SHA&per_page=100",
        'select(.workflow_id == $workflow_id and .name == "CI"',
        '.path == ".github/workflows/ci.yml" and .event == "push"',
        ".repository.full_name == $repo",
        ".head_repository.full_name == $repo",
        '.head_branch == "main" and .head_sha == $sha',
        '(.conclusion // "__pending__")',
        "[[ ${#rows[@]} -le 1 ]]",
        '"$status" == completed',
        '"$conclusion" == success',
        "actions/runs/$ci_run_id",
        '"$actual_id" == "$ci_run_id"',
        '"$actual_workflow_id" == "$workflow_id"',
        '"$run_name" == CI',
        '"$run_path" == .github/workflows/ci.yml',
        '"$event" == push',
        '"$actual_status" == completed',
        '"$actual_conclusion" == success',
        '"$repository" == "$EXPECTED_REPOSITORY"',
        '"$head_repository" == "$EXPECTED_REPOSITORY"',
        '"$head_branch" == main',
        '"$source_sha" == "$SOURCE_SHA"',
        '"$actual_run_attempt" == "$run_attempt"',
        "for attempt in {1..120}",
        "sleep 15",
        'echo "source_sha=$SOURCE_SHA"',
        'echo "workflow_run_id=$PUBLISH_WORKFLOW_RUN_ID"',
        'echo "ci_run_id=$ci_run_id"',
        'git/ref/heads/main',
    ):
        require(source_gate, needle, f"{label} exact automatic publish gate")
    if source_gate.count('git/ref/heads/main') != 2:
        raise AssertionError(f"{label}: current-main bind must occur before and after CI")
    for needle in (
        "rustup toolchain install 1.95",
        "cargo\\ 1\\.95",
        "package --manifest-path",
        "publish --registry crates-io",
        "--dry-run --locked",
        "/usr/bin/env -i",
        'cd "$CLEAN_CWD"',
    ):
        require(validation_package if "package" in needle or "publish" in needle or "env -i" in needle or 'cd "' in needle else validation_job,
                needle, f"{label} deterministic validator")
    if "--no-verify" in validation_package:
        raise AssertionError(f"{label}: registry-credential-free validator stopped building packaged bytes")

    require(publish_job, "environment: crates-io", f"{label} publish environment")
    require(publish_job, "id-token: write", f"{label} publish OIDC")
    if "actions: read" in publish_job:
        raise AssertionError(f"{label}: fresh publisher retained unused Actions access")
    uses = [line.strip().removeprefix("- ") for line in publish_job.splitlines() if "uses:" in line]
    if uses != [f"uses: {CHECKOUT_ACTION}", f"uses: {CRATES_AUTH_ACTION}"]:
        raise AssertionError(f"{label}: unexpected privileged publisher actions: {uses!r}")
    require(publish_job, "persist-credentials: false", f"{label} exact checkout")
    require(
        publish_job,
        "ref: ${{ needs.validate-package.outputs.source_sha }}",
        f"{label} exact checkout",
    )
    require(publish_job, "submodules: false", f"{label} exact checkout")
    require(publish_job, "lfs: false", f"{label} exact checkout")

    for needle in (
        "ls-files -s -z",
        "100644|100755",
        "metadata.st_nlink != 1",
        "symlink forbidden at publish boundary",
        'package.get("readme") != "README.md"',
        '"license-file" in package',
        'package.get("build") not in (None, "build.rs")',
        'manifest.get("bin") != [{"name": "tr300", "path": "src/main.rs"}]',
        'pure.is_absolute() or ".." in pure.parts',
        '"$SOURCE_DIRECTORY/.cargo/config"',
        '"$ancestor/.cargo/config.toml"',
        "/usr/bin/env -i",
        'cd "$CLEAN_CWD"',
        '[[ "$repackaged_sha" == "$EXPECTED_SHA256" ]]',
        "duplicate crate archive entries",
        "compressed crate exceeds the publish limit",
        'if ".cargo" in path.parts',
        ".cargo_vcs_info.json",
        'vcs.get("git", {}).get("sha1") != source_sha',
        "chmod -R a-w",
    ):
        require(candidate, needle, f"{label} fresh candidate custody")

    if publish_job.index(candidate) > publish_job.index(auth):
        raise AssertionError(f"{label}: OIDC minted before candidate byte proof")
    require(
        publish_job,
        "name: Rebind current main immediately before OIDC",
        f"{label} current-main rebind",
    )
    if publish_job.index("name: Rebind current main immediately before OIDC") > publish_job.index(auth):
        raise AssertionError(f"{label}: current-main rebind must precede OIDC")

    outside_publish = publish_job.replace(publish_step, "", 1)
    if "CARGO_REGISTRY_TOKEN" in outside_publish:
        raise AssertionError(f"{label}: short-lived token escaped the Cargo-only step")
    require(
        publish_step,
        "CARGO_REGISTRY_TOKEN: ${{ steps.auth.outputs.token }}",
        f"{label} short-lived token",
    )
    for forbidden in ("curl ", "gh ", "git ", "python", "sleep ", "scripts/"):
        if forbidden in publish_step:
            raise AssertionError(
                f"{label}: token-bearing Cargo step regained tool {forbidden!r}"
            )
    for needle in (
        'cd "$PUBLISH_CWD" || exit 65',
        "/usr/bin/env -i",
        "publish --registry crates-io",
        "--locked --no-verify",
        'echo "status=$cargo_status"',
    ):
        require(publish_step, needle, f"{label} Cargo-only publish")

    if "CARGO_REGISTRY_TOKEN" in adjudication:
        raise AssertionError(f"{label}: public adjudication retained publish token")
    for needle in (
        "published-crate-policy.json",
        '.crate.name == "tr300" and .crate.trustpub_only == true',
        ".version.checksum == $expected",
        '.version.trustpub_data.provider == "github"',
        '.version.trustpub_data.repository == "QubeTX/qube-machine-report"',
        ".version.trustpub_data.sha == $sha",
        ".version.trustpub_data.run_id | tostring",
        '[[ "$published_sha" == "$EXPECTED_SHA256" ]]',
    ):
        require(adjudication, needle, f"{label} exact public OIDC proof")
    version_check = adjudication[adjudication.index("for attempt in") :]
    if ".crate.trustpub_only" in version_check:
        raise AssertionError(f"{label}: version endpoint incorrectly expected crate policy shape")

    registry = extract_named_step(publish_job, "Recheck immutable crates.io state", label)
    for needle in (
        '.crate.name == "tr300" and .crate.trustpub_only == true',
        '[[ "$metadata_checksum" =~ ^[0-9a-f]{64}$ && "$metadata_checksum" == "$public_sha" ]]',
        "prove_public_bytes_match_published_tag",
        "refs/tags/v$VERSION",
        "cargo\\ 1\\.95",
        '[[ "$packaged_sha" == "$public_sha" ]]',
        "published tag crate does not bind its resolved commit",
        '[[ "$tag_after" == "$published_tag_sha"',
    ):
        require(registry, needle, f"{label} post-release drift proof")


def check_privileged_working_directory_contract() -> None:
    inno = INNO_MSI_BRIDGE.read_text(encoding="utf-8")
    managed = MANAGED_WINDOWS_INSTALLER.read_text(encoding="utf-8")
    updater = UPDATE_SOURCE.read_text(encoding="utf-8")

    require(
        inno,
        "Exec(ExpandConstant('{sys}\\msiexec.exe'), Args,\n"
        "        ExpandConstant('{sys}'), SW_HIDE,",
        INNO_MSI_BRIDGE.name,
    )
    if re.search(
        r"Exec\(ExpandConstant\('\{sys\}\\msiexec\.exe'\),\s*Args,\s*''\s*,",
        inno,
    ):
        raise AssertionError(f"{INNO_MSI_BRIDGE.name}: empty msiexec CWD returned")
    require(
        updater,
        "info.lpDirectory = worker_directory.as_ptr();",
        UPDATE_SOURCE.name,
    )
    require(
        managed,
        "Invoke-Tr300Process $msiexec @('/x', $Product.ProductCode, "
        "'/passive', '/norestart') $Product.Elevated $systemDirectory",
        MANAGED_WINDOWS_INSTALLER.name,
    )
    require(
        managed,
        "Invoke-Tr300Process $uninstaller @('/VERYSILENT', '/SUPPRESSMSGBOXES', "
        "'/NORESTART') $Product.Elevated $uninstallerDirectory",
        MANAGED_WINDOWS_INSTALLER.name,
    )


def check_ci_lifecycle_guard(ci: str, macos: str) -> None:
    guard = extract_named_run(
        ci,
        "Guard supplemental packaging lifecycle invariants",
        CI_WORKFLOW.name,
    )
    stable_jobs = (
        "source-custody",
        "prepare-installer-inputs",
        "credential-preflight",
        "build",
    )
    for job in stable_jobs:
        require(macos, f"  {job}:\n", f"{MACOS_WORKFLOW.name} lifecycle")
        require(guard, f"^  {job}:$", f"{CI_WORKFLOW.name} lifecycle guard")
    if "Checkout exact release tag" in guard or "Checkout verified release source" in guard:
        raise AssertionError(
            f"{CI_WORKFLOW.name}: lifecycle guard regressed to a display-name match"
        )
    positions = [macos.index(f"  {job}:\n") for job in stable_jobs]
    if positions != sorted(positions):
        raise AssertionError(f"{MACOS_WORKFLOW.name}: lifecycle job order changed")
    require(
        guard,
        "needs: [resolve-release, source-custody, prepare-installer-inputs, credential-preflight]",
        f"{CI_WORKFLOW.name} lifecycle needs",
    )


def check_msiexec_contract(path: Path, workflow: str) -> int:
    label = path.name
    trusted_directory = (
        "$msiexecDirectory = "
        "[IO.Path]::GetFullPath([Environment]::SystemDirectory)"
    )
    trusted_path = (
        "$msiexecPath = [IO.Path]::GetFullPath((Join-Path "
        "$msiexecDirectory 'msiexec.exe'))"
    )
    if re.search(
        r"(?i)Start-Process\s+(?:-FilePath\s+)?['\"]?msiexec(?:\.exe)?['\"]?(?:\s|$)",
        workflow,
    ):
        raise AssertionError(f"{label}: bare msiexec process launch returned")

    assignments = [
        line.strip()
        for line in workflow.splitlines()
        if re.match(r"^\s*\$msiexecPath\s*=", line, flags=re.IGNORECASE)
    ]
    directory_assignments = [
        line.strip()
        for line in workflow.splitlines()
        if re.match(r"^\s*\$msiexecDirectory\s*=", line, flags=re.IGNORECASE)
    ]
    if not assignments or not directory_assignments:
        raise AssertionError(f"{label}: no trusted msiexec path assignments found")
    for assignment in assignments:
        if assignment != trusted_path:
            raise AssertionError(
                f"{label}: msiexec path is not derived from SystemDirectory: "
                f"{assignment!r}"
            )
    for assignment in directory_assignments:
        if assignment != trusted_directory:
            raise AssertionError(
                f"{label}: msiexec directory is not SystemDirectory: {assignment!r}"
            )
    if len(assignments) != len(directory_assignments):
        raise AssertionError(
            f"{label}: msiexec path/directory assignment counts do not match"
        )

    launches: list[str] = []
    for block in extract_run_blocks(workflow):
        block_launches = [
            line.strip()
            for line in block.splitlines()
            if "Start-Process" in line and "$msiexecPath" in line
        ]
        if not block_launches:
            continue
        require(block, trusted_directory, f"{label} MSI run block")
        require(block, trusted_path, f"{label} MSI run block")
        launches.extend(block_launches)
    if not launches:
        raise AssertionError(f"{label}: no MSI process launches found")
    for launch in launches:
        if (
            "-FilePath $msiexecPath" not in launch
            or "-WorkingDirectory $msiexecDirectory" not in launch
        ):
            raise AssertionError(
                f"{label}: MSI launch lacks trusted executable/CWD binding: {launch!r}"
            )
    return len(launches)


def check_structural_contract(
    release: str,
    windows: str,
    macos: str,
    ci: str,
    crates: str,
    windows_validation: str,
    windows_cargo_dist_installer: str,
) -> None:
    check_native_apple_bash_contract(release, macos)
    check_ci_job_inventory(ci)
    ci_test = extract_job(ci, "test", CI_WORKFLOW.name)
    apple_compatibility = extract_unique_named_step(
        ci_test,
        "Validate Apple release staging under system Bash",
        f"{CI_WORKFLOW.name}:test",
    )
    require_exact_step(
        apple_compatibility,
        """
        - name: Validate Apple release staging under system Bash
          if: runner.os == 'macOS'
          env:
            TR300_TEST_BASH: /bin/bash
          shell: bash
          run: >-
            python3 scripts/test-release-workflow-provenance.py
            --apple-staging-compatibility
        """,
        "native Apple staging compatibility step",
    )

    for workflow_path in sorted((ROOT / ".github" / "workflows").glob("*.yml")):
        workflow_text = workflow_path.read_text(encoding="utf-8")
        for line_number, line in enumerate(workflow_text.splitlines(), start=1):
            match = re.match(r"^\s*-?\s*uses:\s*([^\s#]+)", line)
            if match is None:
                continue
            action = match.group(1)
            if action.startswith("./"):
                continue
            if re.fullmatch(r"[^@\s]+@[0-9a-f]{40}", action) is None:
                raise AssertionError(
                    f"{workflow_path.name}:{line_number}: external action is not "
                    f"pinned to an immutable 40-hex commit: {action!r}"
                )

        for block in extract_run_blocks(workflow_text):
            if re.search(
                r"(?is)\b(?:curl|irm|Invoke-RestMethod)\b[^\n|]*\|\s*"
                r"(?:sh|bash|iex|Invoke-Expression)\b",
                block,
            ):
                raise AssertionError(
                    f"{workflow_path.name}: network response is piped directly "
                    "to an executable shell"
                )

    require(release, CARGO_DIST_SH_SHA256, "release.yml cargo-dist sh pin")
    require(ci, CARGO_DIST_SH_SHA256, "ci.yml cargo-dist sh pin")
    for label, workflow_text in (("release.yml", release), ("ci.yml", ci)):
        require(workflow_text, "command -v sha256sum", f"{label} cargo-dist checksum tool")
        require(workflow_text, "cargo-dist 0.31.0", f"{label} cargo-dist version")
    for label, workflow_text in (("release.yml", release), ("ci.yml", ci)):
        require(
            workflow_text,
            "./scripts/install-pinned-cargo-dist.ps1",
            f"{label} pinned Windows cargo-dist helper",
        )
    require(
        windows_cargo_dist_installer,
        CARGO_DIST_PS1_SHA256,
        "Windows cargo-dist helper ps1 pin",
    )
    require(
        windows_cargo_dist_installer,
        "cargo-dist 0.31.0",
        "Windows cargo-dist helper version",
    )
    require(
        windows_cargo_dist_installer,
        "-NoLogo -NoProfile -NonInteractive -File $installer",
        "Windows cargo-dist helper fresh installer process",
    )
    require(
        windows_cargo_dist_installer,
        "[IO.FileAttributes]::ReparsePoint",
        "Windows cargo-dist helper reparse-point guard",
    )
    if "\n& $installer\n" in windows_cargo_dist_installer:
        raise AssertionError(
            "Windows cargo-dist helper must not inherit StrictMode into the "
            "official installer"
        )
    if ".LinkType" in windows_cargo_dist_installer:
        raise AssertionError(
            "Windows cargo-dist helper must not reject ordinary NTFS hard links"
        )
    if "matrix.install_dist.run" in release:
        raise AssertionError(
            "release.yml: generated mutable matrix.install_dist.run execution returned"
        )
    if "sh.rustup.rs" in release:
        raise AssertionError("release.yml: mutable rustup bootstrap returned")
    if "choco install innosetup" in ci.lower() or "choco install innosetup" in windows.lower():
        raise AssertionError("mutable Chocolatey Inno Setup installation returned")
    require(
        ci,
        "& scripts/install-pinned-inno-setup.ps1",
        "ci.yml pinned Inno Setup helper",
    )
    require(
        windows,
        "& scripts/install-pinned-inno-setup.ps1",
        "windows-installers.yml pinned Inno Setup helper",
    )
    inno_installer = PINNED_INNO_INSTALLER.read_text(encoding="utf-8")
    for needle in (
        "is-6_7_3",
        "innosetup-6.7.3.exe",
        "10592232L",
        "9c73c3bae7ed48d44112a0f48e66742c00090bdb5bef71d9d3c056c66e97b732",
        "$ghPath release verify-asset",
        "jrsoftware/issrc",
        "Get-AuthenticodeSignature",
        "CN=Pyrsys B.V., O=Pyrsys B.V., S=Noord-Holland, C=NL",
        "ProductVersion",
        "-WorkingDirectory $downloadDirectory",
        "Join-Path $isccDirectory 'ISCC.exe'",
        "#if Ver != EncodeVer($majorVersion, $minorVersion, $revisionVersion)",
        "& $iscc '/Qp'",
    ):
        require(inno_installer, needle, "pinned Inno Setup helper")
    iscc_validation = """$isccSignature = Get-AuthenticodeSignature -LiteralPath $iscc
if ($isccSignature.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or
    $null -eq $isccSignature.SignerCertificate -or
    $isccSignature.SignerCertificate.Subject -cne $expectedSigner) {
    throw "the Inno Setup compiler has an invalid publisher signature: $($isccSignature.Status)"
}
"""
    iscc_validation_start = inno_installer.index(iscc_validation)
    iscc_validation_end = iscc_validation_start + len(iscc_validation)
    iscc_execution = inno_installer.index("& $iscc '/Qp'")
    if iscc_validation_end > iscc_execution:
        raise AssertionError(
            "pinned Inno Setup helper executes ISCC before completing its "
            "publisher-signature validation"
        )
    require(ci, "cargo install --locked cargo-audit --version 0.22.2", "ci cargo-audit pin")
    require(ci, "cargo-audit-audit 0.22.2", "ci cargo-audit version proof")

    check_workflow_contract(WINDOWS_WORKFLOW, windows)
    check_workflow_contract(MACOS_WORKFLOW, macos)
    check_windows_validation_provenance(windows_validation)
    check_windows_publish_boundary(windows)
    check_macos_publish_boundary(macos)
    check_release_token_boundary(release)
    check_crates_token_boundary(crates)
    check_privileged_working_directory_contract()
    check_ci_lifecycle_guard(ci, macos)

    require(windows, "permissions:\n  contents: read", WINDOWS_WORKFLOW.name)
    require(windows, "needs: resolve-release", WINDOWS_WORKFLOW.name)
    require(
        windows,
        "ref: ${{ needs.resolve-release.outputs.source_sha }}",
        WINDOWS_WORKFLOW.name,
    )

    require(macos, "needs: resolve-release", MACOS_WORKFLOW.name)
    require(
        macos,
        "ref: ${{ needs.resolve-release.outputs.source_sha }}",
        MACOS_WORKFLOW.name,
    )
    if "ref: ${{ github.event" in macos:
        raise AssertionError("macos-installer.yml: Apple-secret checkout returned to event data")
    if macos.index("resolve-release:") > macos.index("credential-preflight:"):
        raise AssertionError("macos-installer.yml: credential job precedes provenance resolver")

    require(release, STABLE_TAG_PATTERN, RELEASE_WORKFLOW.name)
    require(release, "if: github.event_name == 'push'", RELEASE_WORKFLOW.name)
    require(
        release,
        'permissions:\n  "contents": "read"',
        RELEASE_WORKFLOW.name,
    )
    if release.count("contents: write") != 1:
        raise AssertionError(
            "release.yml: contents:write must be granted only to the tag-only host job"
        )
    host = extract_job(release, "host", RELEASE_WORKFLOW.name)
    require(
        host,
        "permissions:\n      actions: read\n      contents: write",
        "release.yml host job",
    )
    guard = release.index("name: Require a supported stable release tag")
    checkout = release.index(f"uses: {CHECKOUT_ACTION}")
    install_dist = release.index("name: Install dist")
    if not guard < checkout < install_dist:
        raise AssertionError("release.yml: stable-tag guard must precede checkout and dist")

    msi_launches = check_msiexec_contract(CI_WORKFLOW, ci)
    msi_launches += check_msiexec_contract(
        WINDOWS_VALIDATION_WORKFLOW, windows_validation
    )
    if msi_launches != 16:
        raise AssertionError(
            f"expected 16 trusted MSI process launches, found {msi_launches}"
        )


def main() -> None:
    release = RELEASE_WORKFLOW.read_text(encoding="utf-8")
    windows = WINDOWS_WORKFLOW.read_text(encoding="utf-8")
    macos = MACOS_WORKFLOW.read_text(encoding="utf-8")
    ci = CI_WORKFLOW.read_text(encoding="utf-8")
    crates = CRATES_WORKFLOW.read_text(encoding="utf-8")
    windows_validation = WINDOWS_VALIDATION_WORKFLOW.read_text(encoding="utf-8")
    windows_cargo_dist_installer = PINNED_WINDOWS_CARGO_DIST_INSTALLER.read_text(
        encoding="utf-8"
    )
    bash = locate_bash()
    if not bash:
        raise AssertionError("bash is required to execute workflow provenance fixtures")

    check_structural_contract(
        release,
        windows,
        macos,
        ci,
        crates,
        windows_validation,
        windows_cargo_dist_installer,
    )
    run_release_manifest_preservation_fixtures(release, bash)
    run_release_host_draft_fixtures(release, bash)
    run_release_sidecar_normalization_fixtures(release)
    run_macos_archive_mode_compatibility_fixtures(macos)
    run_macos_inventory_compatibility_fixtures(macos)
    with tempfile.TemporaryDirectory(prefix="tr300-provenance-gh-") as fixture_raw:
        mock_bin = Path(fixture_raw) / "bin"
        mock_bin.mkdir()
        write_mock_gh(mock_bin)
        write_windows_mkdir_compat(mock_bin)
        write_windows_shasum_compat(mock_bin)
        write_fixture_jq_compat(mock_bin)
        check_actual_resolvers(
            release, windows, macos, crates, windows_validation, bash, mock_bin
        )

    print("actual release resolver fixtures and structural guards passed")


if __name__ == "__main__":
    if sys.argv[1:] == ["--apple-staging-compatibility"]:
        release = RELEASE_WORKFLOW.read_text(encoding="utf-8")
        macos = MACOS_WORKFLOW.read_text(encoding="utf-8")
        fixture_bash = locate_bash()
        if not fixture_bash:
            raise AssertionError("bash is required for Apple staging compatibility")
        fixture_bash_version = bash_major_minor(fixture_bash)
        if sys.platform == "darwin" and fixture_bash_version != "3.2":
            raise AssertionError(
                "native Apple compatibility must use system Bash 3.2; selected "
                f"Bash {fixture_bash_version}"
            )
        check_native_apple_bash_syntax(release, macos, fixture_bash)
        run_apple_staging_compatibility_fixture(release, fixture_bash)
        run_release_sidecar_normalization_fixtures(release)
        run_macos_archive_mode_compatibility_fixtures(macos)
        run_macos_inventory_compatibility_fixtures(macos)
        print(
            "Apple release staging and native job syntax are compatible with "
            f"selected Bash {fixture_bash_version}"
        )
    elif sys.argv[1:]:
        raise SystemExit(f"unsupported arguments: {sys.argv[1:]!r}")
    else:
        main()
