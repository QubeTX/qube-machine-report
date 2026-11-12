#!/usr/bin/env python3
"""Regression cases for skipping work and refusing incomplete CI."""
import copy
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import ci_policy as policy


class PolicyTests(unittest.TestCase):
    def test_release_plan_exclusions_match_record_policy(self):
        workflow = (Path(__file__).resolve().parents[1] / ".github/workflows/release.yml").read_text(encoding="utf-8")
        match = re.search(r"  pull_request:\n    paths-ignore:\n(.*?)  push:\n", workflow, re.S)
        self.assertIsNotNone(match)
        ignored = re.findall(r"^      - '([^']+)'$", match.group(1), re.M)
        self.assertEqual(set(ignored), policy.RECORD_FILES | {"docs/**/*.md", ".tasks/**/*.md"})
        self.assertEqual(len(ignored), len(set(ignored)))

    def test_path_allowlist(self):
        for path in ("AGENTS.md", "TESTING.md", "docs/thinking/a.md", ".tasks/tasks/abc.md"):
            self.assertTrue(policy.documentation_path(path), path)
        for path in ("README.md", "LICENSE", "man/report.1", "CHANGELOG.md", "HUMAN_CHANGELOG.md", "docs/run.sh", "src/readme.md", ".agents/skills/release/SKILL.md", ".github/workflows/ci.yml", "docs/../src/a.md", "/docs/a.md", "docs//a.md", "docs/a\nb.md", "docs\\a.md"):
            self.assertFalse(policy.documentation_path(path), path)

    def test_gate_accepts_only_complete_expected_mode(self):
        for mode in ("docs", "full"):
            needs = {name: {"result": "success" if mode == "full" else "skipped"} for name in policy.FULL_JOBS}
            needs["classify"] = {"result": "success", "outputs": {"mode": mode}}
            self.assertEqual(policy.validate_gate(needs), mode)
            for name in needs:
                for result in ("failure", "cancelled", "skipped", "success", "pending", None):
                    if result == needs[name]["result"]:
                        continue
                    bad = copy.deepcopy(needs)
                    bad[name]["result"] = result
                    with self.assertRaises(ValueError):
                        policy.validate_gate(bad)
            for mode_value in ("", "unknown", None):
                bad = copy.deepcopy(needs)
                bad["classify"]["outputs"]["mode"] = mode_value
                with self.assertRaises(ValueError):
                    policy.validate_gate(bad)
            with self.assertRaises(ValueError):
                policy.validate_gate({**needs, "unexpected": {"result": "success"}})
            del needs["test"]
            with self.assertRaises(ValueError):
                policy.validate_gate(needs)

    def test_raw_modes_renames_and_unusual_paths(self):
        for raw in (b":100644 100644 a b M\0docs/a.md\0", b":000000 100644 0 b A\0docs/a.md\0", b":100644 000000 a 0 D\0docs/a.md\0"):
            with patch.object(policy, "git", return_value=raw):
                self.assertIsNotNone(policy.changed_records("a", "b"))
        for raw in (b"", b":120000 120000 a b M\0docs/a.md\0", b":160000 160000 a b M\0docs/a.md\0", b":100644 100755 a b M\0docs/a.md\0", b":100644 100644 a b R100\0src/a.rs\0docs/a.md\0", b":100644 000000 a 0 D\0src/a.rs\0:000000 100644 0 b A\0docs/a.md\0", b":100644 100644 a b M\0docs/a\nb.md\0"):
            with patch.object(policy, "git", return_value=raw):
                self.assertIsNone(policy.changed_records("a", "b"), raw)

    def test_real_git_diff_and_event_bindings(self):
        original = Path.cwd()
        with tempfile.TemporaryDirectory() as directory:
            try:
                os.chdir(directory)
                policy.git("init", "-q")
                policy.git("config", "user.email", "test@example.invalid")
                policy.git("config", "user.name", "CI policy fixture")
                Path("Cargo.toml").write_text('[package]\nname="fixture"\nreadme="README.md"\ninclude=' + json.dumps(sorted(policy.PACKAGE_INCLUDE)) + '\n', encoding="utf-8")
                Path("README.md").write_text("package input\n", encoding="utf-8")
                Path("docs").mkdir()
                Path("docs/a.md").write_text("record\n", encoding="utf-8")
                policy.git("add", ".")
                policy.git("commit", "-qm", "base")
                base = policy.git("rev-parse", "HEAD").decode().strip()
                Path("docs/a.md").write_text("updated record\n", encoding="utf-8")
                policy.git("commit", "-qam", "docs")
                head = policy.git("rev-parse", "HEAD").decode().strip()
                self.assertEqual(policy.classify_diff(base, head), "docs")
                event = {"repository": {"full_name": policy.REPOSITORY}, "before": base, "after": head, "ref": "refs/heads/main"}
                self.assertEqual(policy.event_range(event, "push", head), (base, head))
                for change in ({"forced": True}, {"deleted": True}, {"before": "0" * 40}, {"after": base}, {"ref": "refs/heads/other"}):
                    with self.assertRaises(ValueError):
                        policy.event_range({**event, **change}, "push", head)
                with self.assertRaises(ValueError):
                    policy.event_range(event, "workflow_dispatch", head)
                self.assertEqual(policy.classify_diff(head, head), "full")
                self.assertEqual(policy.classify_diff("f" * 40, head), "full")
                Path("README.md").write_text("changed package\n", encoding="utf-8")
                policy.git("commit", "-qam", "package")
                package_head = policy.git("rev-parse", "HEAD").decode().strip()
                self.assertEqual(policy.classify_diff(base, package_head), "full")
                self.assertEqual(policy.classify_diff(package_head, head), "full")
                # A real synthetic merge proves PR changes against the actual
                # integrated base, while rejecting head checkouts or mismatch.
                policy.git("checkout", "-qb", "feature", package_head)
                Path("docs/feature.md").write_text("feature record\n", encoding="utf-8")
                policy.git("add", ".")
                policy.git("commit", "-qm", "feature docs")
                feature = policy.git("rev-parse", "HEAD").decode().strip()
                policy.git("checkout", "-qb", "integration", package_head)
                Path("README.md").write_text("concurrent base change\n", encoding="utf-8")
                policy.git("commit", "-qam", "base advanced")
                integrated_base = policy.git("rev-parse", "HEAD").decode().strip()
                policy.git("merge", "--no-ff", "-qm", "synthetic merge", "feature")
                merged = policy.git("rev-parse", "HEAD").decode().strip()
                event = {"repository": {"full_name": policy.REPOSITORY}, "pull_request": {"base": {"ref": "main", "sha": package_head, "repo": {"full_name": policy.REPOSITORY}}, "head": {"sha": feature}}}
                self.assertEqual(policy.event_range(event, "pull_request", merged), (integrated_base, merged))
                self.assertEqual(policy.classify_diff(integrated_base, merged), "docs")
                event["pull_request"]["head"]["sha"] = package_head
                with self.assertRaises(ValueError):
                    policy.event_range(event, "pull_request", merged)
                with open("Cargo.toml", "a", encoding="utf-8") as file:
                    file.write('license-file="docs/a.md"\n')
                policy.git("commit", "-qam", "package boundary")
                boundary = policy.git("rev-parse", "HEAD").decode().strip()
                Path("docs/a.md").write_text("packaged record\n", encoding="utf-8")
                policy.git("commit", "-qam", "now packaged docs")
                self.assertEqual(policy.classify_diff(boundary, policy.git("rev-parse", "HEAD").decode().strip()), "full")
            finally:
                os.chdir(original)


if __name__ == "__main__":
    unittest.main()
