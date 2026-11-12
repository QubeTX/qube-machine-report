#!/usr/bin/env python3
"""Smoke-test both built commands; benchmark the existing release binary."""
import json
import os
from pathlib import Path
import statistics
import subprocess
import time

suffix = ".exe" if os.name == "nt" else ""
canonical = Path("target/release") / f"tr300{suffix}"
alias = Path("target/release") / f"report{suffix}"
versions = [subprocess.check_output([str(binary), "--version"], text=True, timeout=30).strip() for binary in (canonical, alias)]
if versions[0] != versions[1]:
    raise SystemExit("the two commands disagree on version")
for binary in (canonical, alias):
    report = json.loads(subprocess.check_output([str(binary), "--fast", "--json"], timeout=30))
    if report.get("schema_version") != 1 or report.get("collection_mode") != "fast":
        raise SystemExit(f"{binary}: unexpected fast report")
print(f"Both commands passed version/fast JSON smoke: {versions[0]}")

if os.environ.get("CI_BENCHMARK") == "true":
    def run():
        start = time.perf_counter_ns()
        subprocess.run([str(canonical), "--fast"], stdout=subprocess.DEVNULL, check=True, timeout=30)
        return (time.perf_counter_ns() - start) / 1_000_000

    run()  # Warm-up uses the same existing release binary as the five samples.
    samples = [run() for _ in range(5)]
    median = statistics.median(samples)
    budget = 1500
    summary = f"Fast-mode runs (ms): {', '.join(f'{s:.1f}' for s in samples)}\n\nMedian: **{median:.1f} ms**, budget: {budget} ms.\n"
    print(summary)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as output:
            output.write("## Fast-mode performance\n\n" + summary)
    if median > budget:
        raise SystemExit(f"--fast median {median:.1f} ms exceeds {budget} ms budget")
