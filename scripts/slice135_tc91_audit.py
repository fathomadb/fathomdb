#!/usr/bin/env python3
"""Replay exact-candidate projection-commit fault logs and resources."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
from typing import Any


EXPECTED_TESTS = {
    "tc91_panic_terminal_commit_failure_is_reported_and_redispatched",
    "tc91_projection_commit_busy_is_reported_and_redispatched",
    "tc91_projection_commit_failure_at_mean_pin_is_rollback_safe",
    "tc91_projection_commit_failure_survives_stop_and_reopen",
    "tc91_projection_commit_storage_error_is_reported_and_redispatched",
    "tc91_subscriber_panic_after_commit_failure_does_not_wedge_redispatch",
}


def digest(path: Path) -> str:
    """Hash an independently reopened evidence file."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_test_output(stdout: str, stderr: str) -> list[str]:
    """Reject a missing test, changed outcome or missing injected panic."""
    names = re.findall(r"^test (tc91_\w+) \.\.\. ok$", stdout, re.MULTILINE)
    if (
        len(names) != 6
        or set(names) != EXPECTED_TESTS
        or "running 6 tests" not in stdout
        or not re.search(
            r"^test result: ok\. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;",
            stdout,
            re.MULTILINE,
        )
    ):
        raise ValueError("expected six passing tests are not present")
    markers = (
        "intentional first projection panic",
        "intentional subscriber panic after projection commit failure",
    )
    if (
        any(stderr.count(marker) != 1 for marker in markers)
        or stderr.count("panicked at") != 2
    ):
        raise ValueError("expected panic evidence differs from fixture")
    return sorted(names)


def parse_resources(report: str) -> dict[str, int]:
    """Parse the separately captured GNU Time test-process report."""
    labels = {
        "peak_rss_kib": "Maximum resident set size (kbytes)",
        "major_faults": "Major (requiring I/O) page faults",
        "swap_events": "Swaps",
        "exit_status": "Exit status",
    }
    values = {}
    for key, label in labels.items():
        match = re.search(rf"^\s*{re.escape(label)}:\s*(\d+)\s*$", report, re.MULTILINE)
        if match is None:
            raise ValueError(f"missing resource field: {label}")
        values[key] = int(match.group(1))
    if values["swap_events"] or values["exit_status"] or values["major_faults"]:
        raise ValueError("measured test process swapped, faulted or failed")
    return values


def _git(checkout: Path, *args: str) -> str:
    """Read source identity from the exact measured checkout."""
    result = subprocess.run(
        ["git", "-C", str(checkout), *args],
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout.strip()


def audit(directory: Path, checkout: Path) -> dict[str, Any]:
    """Recompute ten fault-run outcomes from retained logs and binary bytes."""
    manifest_path = directory / "manifest.json"
    manifest = json.loads(manifest_path.read_text())
    source_sha = manifest.get("source_sha")
    if (
        manifest.get("schema_version") != 1
        or manifest.get("run_count") != 10
        or len(manifest.get("results", [])) != 10
        or _git(checkout, "rev-parse", "HEAD") != source_sha
        or _git(checkout, "status", "--porcelain", "--untracked-files=all")
    ):
        raise ValueError("source checkout or campaign declaration differs")
    test_source = (
        checkout
        / "src/rust/crates/fathomdb-engine/tests/tc91_projection_commit_hardening.rs"
    )
    binary = Path(manifest["binary"])
    if digest(test_source) != manifest.get("test_source_sha256") or digest(
        binary
    ) != manifest.get("binary_sha256"):
        raise ValueError("test source or measured binary bytes changed")
    results = []
    for index in range(1, 11):
        run = directory / f"run-{index:02}"
        stdout_path = run / "stdout.log"
        stderr_path = run / "stderr.log"
        resource_path = run / "resource.txt"
        names = check_test_output(stdout_path.read_text(), stderr_path.read_text())
        resources = parse_resources(resource_path.read_text())
        if manifest["results"][index - 1] != {
            "index": index,
            "returncode": 0,
            "six_passed": True,
            "expected_panic_markers": True,
        }:
            raise ValueError(f"run {index}: producer status differs")
        results.append(
            {
                "index": index,
                "tests": names,
                "stdout_sha256": digest(stdout_path),
                "stderr_sha256": digest(stderr_path),
                "resource_sha256": digest(resource_path),
                "resources": resources,
            }
        )
    return {
        "status": "AUDITED_TC91_EXACT_CANDIDATE_FAULT_CAMPAIGN",
        "source_sha": source_sha,
        "test_source_sha256": digest(test_source),
        "binary_sha256": digest(binary),
        "manifest_sha256": digest(manifest_path),
        "run_count": len(results),
        "results": results,
    }


def main() -> None:
    """Print the independently recomputed campaign record."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--checkout", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(audit(args.input, args.checkout), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
