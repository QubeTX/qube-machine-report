#!/usr/bin/env python3
"""Execute and structurally guard privileged release provenance workflows."""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import tempfile
import textwrap
import zipfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RELEASE_WORKFLOW = ROOT / ".github" / "workflows" / "release.yml"
WINDOWS_WORKFLOW = ROOT / ".github" / "workflows" / "windows-installers.yml"
MACOS_WORKFLOW = ROOT / ".github" / "workflows" / "macos-installer.yml"
CI_WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"
CRATES_WORKFLOW = ROOT / ".github" / "workflows" / "crates-publish.yml"
APPLE_MIGRATION_WORKFLOW = ROOT / ".github" / "workflows" / "apple-secret-migration.yml"
PINNED_INNO_INSTALLER = ROOT / "scripts" / "install-pinned-inno-setup.ps1"
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
MACOS_UPSTREAM_SENTINELS = (
    "dist-manifest.json",
    "tr300-aarch64-apple-darwin.tar.xz",
    "tr300-aarch64-apple-darwin.tar.xz.sha256",
    "tr300-x86_64-apple-darwin.tar.xz",
    "tr300-x86_64-apple-darwin.tar.xz.sha256",
)
MACOS_RELEASE_RUN_ARTIFACTS = (
    "artifacts-build-local-aarch64-apple-darwin",
    "artifacts-build-local-x86_64-apple-darwin",
    "unsigned-artifacts-build-local-aarch64-apple-darwin",
    "unsigned-artifacts-build-local-x86_64-apple-darwin",
)


def require(workflow: str, needle: str, label: str) -> None:
    if needle not in workflow:
        raise AssertionError(f"{label}: missing {needle!r}")


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
        match = re.match(r"^(\s*)run:\s*\|\s*$", lines[index])
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
            if candidate.strip() and len(candidate) - len(candidate.lstrip()) < indentation:
                break
            end += 1
        return "\n".join(lines[index:end]) + "\n"
    raise AssertionError(f"{label}: missing named step {step_name!r}")


def extract_job(workflow: str, job_name: str, label: str) -> str:
    match = re.search(rf"(?m)^  {re.escape(job_name)}:\s*$", workflow)
    if match is None:
        raise AssertionError(f"{label}: missing job {job_name!r}")
    following = re.search(r"(?m)^  [A-Za-z0-9_-]+:\s*$", workflow[match.end() :])
    end = len(workflow) if following is None else match.end() + following.start()
    return workflow[match.start() : end]


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
if [[ -n ${MOCK_GH_LOG:-} ]]; then
    printf '%s\\n' "$endpoint" >> "$MOCK_GH_LOG"
fi
if [[ -n ${MOCK_FAIL_ENDPOINT:-} && $endpoint == "$MOCK_FAIL_ENDPOINT" ]]; then
    exit 88
fi
case "$endpoint" in
    repos/*/actions/workflows/ci.yml/runs\\?*)
        cat "$MOCK_CI_RUNS_JSON"
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
            *)
                [[ -n ${MOCK_PUBLISHER_ARTIFACT_ZIP:-} ]] || exit 89
                cat "$MOCK_PUBLISHER_ARTIFACT_ZIP"
                ;;
        esac
        ;;
    repos/*/actions/artifacts/*)
        cat "$MOCK_DIRECT_ARTIFACT_JSON"
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
        if [[ $* == *target_commitish* ]]; then
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
index = 0
while index < len(arguments):
    argument = arguments[index]
    if argument in ("-e", "-r", "-c"):
        raw = raw or argument == "-r"
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
if index + 2 != len(arguments):
    print(f"fixture jq rejects arguments: {arguments!r}", file=sys.stderr)
    raise SystemExit(3)
query = arguments[index]
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
    elif value is not None:
        print(value)

def require(condition):
    raise SystemExit(0 if condition else 1)

if normalized == ".total_count":
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
    archive_bytes = f"fixture signed cargo-dist archive for {target}\n".encode()
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
        if expected_success:
            expected = {
                "tr300-aarch64-apple-darwin.tar.xz",
                "tr300-aarch64-apple-darwin.tar.xz.sha256",
                "tr300-x86_64-apple-darwin.tar.xz",
                "tr300-x86_64-apple-darwin.tar.xz.sha256",
            }
            if {path.name for path in output.iterdir()} != expected:
                raise AssertionError(f"{name}: normalized inventory changed")
        calls = log.read_text(encoding="utf-8").splitlines()
        if any("/releases/" in call for call in calls):
            raise AssertionError(f"{name}: source custody consulted mutable Release assets")


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


def write_macos_release_artifact(directory: Path) -> None:
    directory.mkdir()
    for index, payload_name in enumerate(MACOS_RELEASE_PAYLOADS, start=1):
        payload = directory / payload_name
        payload.write_bytes(f"fixture macOS package {index}\n".encode())
        digest = hashlib.sha256(payload.read_bytes()).hexdigest()
        (directory / f"{payload_name}.sha256").write_bytes(
            f"{digest} *{payload_name}\n".encode()
        )


def run_macos_publisher_case(
    *,
    bash: str,
    mock_bin: Path,
    block: str,
    name: str,
    overrides: dict[str, str] | None = None,
    artifact_mutation: str | None = None,
    expected_success: bool,
) -> None:
    with tempfile.TemporaryDirectory(prefix="tr300-macos-publisher-") as case_dir_raw:
        case_dir = Path(case_dir_raw)
        artifact = case_dir / "artifact"
        output = case_dir / "github-output"
        log = case_dir / "gh-calls"
        write_macos_release_artifact(artifact)

        if artifact_mutation == "extra":
            (artifact / "unexpected.asset").write_bytes(b"unexpected")
        elif artifact_mutation == "missing":
            (artifact / MACOS_RELEASE_ASSETS[-1]).unlink()
        elif artifact_mutation == "empty":
            (artifact / MACOS_RELEASE_PAYLOADS[0]).write_bytes(b"")
        elif artifact_mutation == "directory":
            target = artifact / MACOS_RELEASE_PAYLOADS[0]
            target.unlink()
            target.mkdir()
        elif artifact_mutation == "bad-checksum":
            (artifact / f"{MACOS_RELEASE_PAYLOADS[0]}.sha256").write_bytes(
                f"{'0' * 64} *{MACOS_RELEASE_PAYLOADS[0]}\n".encode()
            )
        elif artifact_mutation is not None:
            raise AssertionError(f"unknown macOS artifact mutation: {artifact_mutation}")

        environment = fixture_environment(mock_bin, output, log)
        environment.update(
            {
                "RELEASE_TAG": "v4.3.0",
                "EXPECTED_SHA": TRUSTED_SHA,
                "REPOSITORY": REPOSITORY,
                "ASSET_DIRECTORY": artifact.as_posix(),
                "MOCK_CURRENT_ASSETS": "\n".join(MACOS_UPSTREAM_SENTINELS),
            }
        )
        if overrides:
            environment.update(overrides)
        script = case_dir / "workflow-run.sh"
        script.write_text(block, encoding="utf-8", newline="\n")
        result = subprocess.run(
            [bash, str(script)],
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
                *(f"{artifact.as_posix()}/{asset}" for asset in MACOS_RELEASE_ASSETS),
            ]
            if upload_arguments != expected_arguments:
                raise AssertionError(
                    f"{name}: upload arguments {upload_arguments!r} != "
                    f"{expected_arguments!r}"
                )
        elif upload_arguments:
            raise AssertionError(f"{name}: rejected case reached release upload")


def check_actual_resolvers(
    release: str,
    windows: str,
    macos: str,
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
    macos_publisher = extract_named_run(
        macos,
        "Bind exact supplements, upload macOS assets, and publish the draft",
        MACOS_WORKFLOW.name,
    )
    windows_validation_resolver = extract_named_run(
        windows_validation,
        "Resolve tag and require every Windows family",
        WINDOWS_VALIDATION_WORKFLOW.name,
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

    macos_outputs = {
        "build": "true",
        "tag": "v4.3.0",
        "version": "4.3.0",
        "source_sha": TRUSTED_SHA,
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
    require(workflow, "releases/tags/$tag", label)
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
    require(workflow, "github.event.workflow_run.conclusion == 'success'", label)
    require(
        workflow,
        'workflows: ["Windows Installers", "macOS Universal Package"]',
        label,
    )
    require(workflow, "github.event.workflow_run.event == 'workflow_run'", label)
    require(
        workflow,
        "github.event.workflow_run.head_repository.full_name == github.repository",
        label,
    )
    require(workflow, STABLE_TAG_PATTERN, label)
    require(workflow, "git/ref/tags/$tag", label)
    require(workflow, "git/tags/$source_sha", label)
    require(workflow, "releases/tags/$tag", label)
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
        '[[ "$release_draft" == true && "$release_prerelease" == false && "$asset_count" == 30 ]]',
        '[[ "$release_draft" == false && "$release_prerelease" == false && "$asset_count" == 34 ]]',
        "direct authenticated candidate transition",
        "private draft leaked into public updater discovery",
    ):
        require(workflow, needle, f"{label} private/public custody")
    prepare = extract_job(workflow, "prepare-validation-inputs", label)
    attest = extract_job(workflow, "attest-private-validation", label)
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
    build = extract_job(workflow, "build-windows-installers", label)
    publisher = extract_job(workflow, "publish-windows-installers", label)

    if workflow.count("contents: write") != 1:
        raise AssertionError(
            f"{label}: contents:write must be granted only to the publisher"
        )
    require(build, "permissions:\n      contents: read", f"{label} build job")
    if "contents: write" in build or "gh release upload" in build:
        raise AssertionError(f"{label}: build job regained release write capability")
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
    require(executable, "releases/tags/$RELEASE_TAG", f"{label} publisher")
    require(executable, "actions/artifacts/$INTERNAL_ARTIFACT_ID/zip", f"{label} exact artifact ID")
    require(executable, '"sha256:$INTERNAL_ARTIFACT_DIGEST"', f"{label} REST digest")
    require(executable, '[[ "$zip_hash" == "$INTERNAL_ARTIFACT_DIGEST" ]]', f"{label} ZIP digest")
    require(executable, ".workflow_run.id == $run", f"{label} run custody")
    require(executable, 'git/ref/heads/main', f"{label} current main rebind")
    require(executable, 'actions/workflows/ci.yml/runs?event=push', f"{label} exact CI rebind")
    require(executable, '"$release_draft" == true', f"{label} private draft only")
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
    custody = extract_job(workflow, "source-custody", label)
    prepare = extract_job(workflow, "prepare-installer-inputs", label)
    preflight = extract_job(workflow, "credential-preflight", label)
    build = extract_job(workflow, "build", label)
    publisher = extract_job(workflow, "publish", label)

    if workflow.count("contents: write") != 1:
        raise AssertionError(
            f"{label}: contents:write must be granted only to the publisher"
        )
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
        "tr300-pkg-rollback",
        "tr300-migration-probe",
    ):
        require(secret_step, needle, f"{label} inline system-tool builder")

    require(publisher, "permissions:\n      actions: read\n      contents: write", f"{label} publisher")
    require(publisher, "actions: read", f"{label} publisher artifact custody")
    require(publisher, "environment: release-publishing", f"{label} publisher environment")
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
    require(executable, STABLE_TAG_PATTERN, f"{label} publisher")
    require(executable, "EXPECTED_SHA", f"{label} publisher")
    require(executable, "git/ref/tags/$tag", f"{label} publisher")
    require(executable, "git/tags/$source_sha", f"{label} publisher")
    require(executable, "releases/tags/$RELEASE_TAG", f"{label} publisher")
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
    ):
        require(executable, needle, f"{label} Windows acceptance custody")
    require(executable, "actions/workflows/ci.yml/runs?event=push", f"{label} exact CI rebind")
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
        "releases/tags/$RELEASE_TAG",
        "[[ \"$http_status\" == 404 ]]",
        "gh release create",
        "--draft --verify-tag --target",
        "([.assets[].name] | sort) == $expected_assets",
    ):
        require(publisher, needle, f"{label} fixed draft publisher")
    if '"$ASSET_DIRECTORY"/*' in publisher or "--clobber" in publisher:
        raise AssertionError(f"{label}: publisher regained glob/clobber upload")
    if "smoke-published-managed-linux:" in workflow:
        raise AssertionError(f"{label}: public smoke reintroduced before draft finalization")
    require(announce, "permissions: {}", f"{label} private handoff")
    require(announce, "Confirm private draft handoff", f"{label} private handoff")


def check_crates_token_boundary(workflow: str) -> None:
    label = CRATES_WORKFLOW.name
    trigger = workflow[: workflow.index("permissions:")]
    if re.search(r"(?m)^\s{2}push:\s*$", trigger):
        raise AssertionError(
            f"{label}: routine crates publication must remain an explicit dispatch gate"
        )
    require(
        workflow,
        "if: github.event_name == 'workflow_dispatch' && inputs.operation == 'publish' && github.ref == 'refs/heads/main' && github.repository == 'QubeTX/qube-machine-report' && github.actor_id == '30877743'",
        f"{label} explicit publish dispatch",
    )
    configure_job = extract_job(workflow, "configure-trusted-publisher", label)
    probe_job = extract_job(workflow, "probe-trusted-publisher", label)
    enable_job = extract_job(workflow, "enable-trusted-publishing-only", label)
    validation_job = extract_job(workflow, "validate-package", label)
    publish_job = extract_job(workflow, "publish", label)
    bootstrap = extract_named_step(
        configure_job, "Create the exact trusted-publisher configuration", label
    )
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
    for job_name, job in (
        ("configure", configure_job),
        ("probe", probe_job),
        ("enable", enable_job),
        ("publish", publish_job),
    ):
        require(job, "environment: crates-io", f"{label} {job_name} environment")
        require(job, "runs-on: ubuntu-24.04", f"{label} {job_name} runner")

    if "uses:" in configure_job or "actions/checkout" in configure_job:
        raise AssertionError(f"{label}: bootstrap must be checkout/action free")
    require(configure_job, "permissions: {}", f"{label} bootstrap permissions")
    require(
        bootstrap,
        "CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}",
        f"{label} bootstrap token",
    )
    for needle in (
        "Authorization: Bearer $CARGO_REGISTRY_TOKEN",
        '"environment":"crates-io"',
        ".github_config.crate",
        ".github_configs[0].crate",
        ".meta.total == 1",
        ".github_configs | length",
        "repository_owner_id == 230946269",
        "configuration_count\" == 0",
        "configuration_count\" == 1",
        "commits/main",
        "EXPECTED_ACTOR_ID\" == 30877743",
    ):
        require(bootstrap, needle, f"{label} idempotent exact bootstrap")
    if ".krate" in bootstrap or "/api/v1/tokens/current" in workflow:
        raise AssertionError(f"{label}: invalid crates.io API contract returned")
    if workflow.count("${{ secrets.CARGO_REGISTRY_TOKEN }}") != 2:
        raise AssertionError(f"{label}: legacy token must remain in bootstrap/disable-only jobs")

    probe_uses = [line.strip() for line in probe_job.splitlines() if "uses:" in line]
    if probe_uses != [f"uses: {CRATES_AUTH_ACTION}"]:
        raise AssertionError(f"{label}: unexpected OIDC probe actions: {probe_uses!r}")
    require(probe_job, "id-token: write", f"{label} OIDC probe permission")
    require(workflow, "probe_trusted_publisher", f"{label} post-deletion OIDC probe mode")
    if "actions/checkout" in enable_job or "uses:" in enable_job:
        raise AssertionError(f"{label}: trusted-only transition must be checkout/action free")
    for needle in (
        "needs.probe-trusted-publisher.result == 'success'",
        "CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}",
        '--request PATCH',
        "{\"crate\":{\"trustpub_only\":true}}",
        '.crate.trustpub_only == true',
        "commits/main",
    ):
        require(enable_job, needle, f"{label} trusted-only transition")

    if "id-token: write" in validation_job or "CARGO_REGISTRY_TOKEN" in validation_job:
        raise AssertionError(f"{label}: validation regained a credential")
    for needle in (
        "EVENT_NAME: ${{ github.event_name }}",
        "OPERATION: ${{ inputs.operation }}",
        "ACTOR_ID: ${{ github.actor_id }}",
        '"$EVENT_NAME" == workflow_dispatch',
        '"$OPERATION" == publish',
        '"$ACTOR_ID" == 30877743',
        '"$EXPECTED_REPOSITORY" == QubeTX/qube-machine-report',
    ):
        require(source_gate, needle, f"{label} owner-authorized publish gate")
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
    require(publish_job, "ref: ${{ github.sha }}", f"{label} exact checkout")
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


def check_apple_migration_boundary(workflow: str) -> None:
    label = APPLE_MIGRATION_WORKFLOW.name
    migrate = extract_job(workflow, "migrate", label)
    require(workflow, "permissions: {}", label)
    require(migrate, "environment: apple-signing", label)
    if "uses:" in migrate or "actions/checkout" in migrate:
        raise AssertionError(f"{label}: one-time migration must be action/checkout free")
    for needle in (
        "github.repository == 'QubeTX/qube-machine-report'",
        "github.actor_id == '30877743'",
        "github.ref == 'refs/heads/main'",
        "git/ref/heads/main",
        "deployment-branch-policies",
        "$'branch\\tmain'",
        "$'tag\\tv*'",
        "RELEASE_SECRET_MIGRATION_TOKEN",
        "APPLE_CERTIFICATE_P12_BASE64",
        "APPLE_INSTALLER_CERTIFICATE_P12_BASE64",
        "APPLE_API_KEY_P8_BASE64",
        'gh secret set "$name" --env apple-signing',
    ):
        require(migrate, needle, label)


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
    apple_migration: str,
) -> None:
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
    require(release, CARGO_DIST_PS1_SHA256, "release.yml cargo-dist ps1 pin")
    require(ci, CARGO_DIST_SH_SHA256, "ci.yml cargo-dist sh pin")
    for label, workflow_text in (("release.yml", release), ("ci.yml", ci)):
        require(workflow_text, "command -v sha256sum", f"{label} cargo-dist checksum tool")
        require(workflow_text, "cargo-dist 0.31.0", f"{label} cargo-dist version")
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
    ):
        require(inno_installer, needle, "pinned Inno Setup helper")
    require(ci, "cargo install --locked cargo-audit --version 0.22.2", "ci cargo-audit pin")
    require(ci, "cargo-audit-audit 0.22.2", "ci cargo-audit version proof")

    check_workflow_contract(WINDOWS_WORKFLOW, windows)
    check_workflow_contract(MACOS_WORKFLOW, macos)
    check_windows_validation_provenance(windows_validation)
    check_windows_publish_boundary(windows)
    check_macos_publish_boundary(macos)
    check_release_token_boundary(release)
    check_crates_token_boundary(crates)
    check_apple_migration_boundary(apple_migration)
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
    apple_migration = APPLE_MIGRATION_WORKFLOW.read_text(encoding="utf-8")
    bash = locate_bash()
    if not bash:
        raise AssertionError("bash is required to execute workflow provenance fixtures")

    check_structural_contract(
        release, windows, macos, ci, crates, windows_validation, apple_migration
    )
    with tempfile.TemporaryDirectory(prefix="tr300-provenance-gh-") as fixture_raw:
        mock_bin = Path(fixture_raw) / "bin"
        mock_bin.mkdir()
        write_mock_gh(mock_bin)
        write_windows_mkdir_compat(mock_bin)
        write_fixture_jq_compat(mock_bin)
        check_actual_resolvers(
            release, windows, macos, windows_validation, bash, mock_bin
        )

    print("actual release resolver fixtures and structural guards passed")


if __name__ == "__main__":
    main()
