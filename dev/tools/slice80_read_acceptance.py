#!/usr/bin/env python3
"""Validate and summarize Slice 80 AC-081a/b acceptance evidence."""

from __future__ import annotations

import hashlib
import json
import re
import shutil
import subprocess
from pathlib import Path
from statistics import median
from typing import Any


MARKER = re.compile(
    r"AC081_NUMBERS sequential_ns=(?P<sequential>\d+) concurrent_ns=(?P<concurrent>\d+) "
    r"sequential_searches=(?P<sequential_searches>\d+) "
    r"concurrent_searches=(?P<concurrent_searches>\d+) threads=(?P<threads>\d+) "
    r"sequential_warning=(?P<sequential_warning>true|false) "
    r"concurrent_warning=(?P<concurrent_warning>true|false) "
    r"sequential_failure=(?P<sequential_failure>true|false) "
    r"concurrent_failure=(?P<concurrent_failure>true|false) ratio=(?P<ratio>[0-9.]+)"
)
REQUIRED_IDENTITY = ("source_sha", "binary_sha256", "input_sha256", "mode")
REQUIRED_ENVIRONMENT = (
    "load_1m",
    "online_cpus",
    "available_memory_percent",
    "pswpin",
    "pswpout",
    "cpu_temp_c",
    "thermal_signal",
    "competing_processes",
    "cpu_affinity",
    "cpu_quota",
    "scaling_governors",
    "scaling_frequencies_khz",
)


def _flag(value: str) -> bool:
    return value == "true"


def parse_run_log(text: str, exit_code: int, identity: dict[str, str]) -> dict[str, Any]:
    matches = list(MARKER.finditer(text))
    if len(matches) != 1:
        raise ValueError("expected exactly one AC081_NUMBERS marker")
    if not all(identity.get(field) for field in REQUIRED_IDENTITY):
        raise ValueError("complete source, binary, input and mode identity is required")

    match = matches[0]
    record: dict[str, Any] = {
        "sequential_ns": int(match["sequential"]),
        "concurrent_ns": int(match["concurrent"]),
        "sequential_searches": int(match["sequential_searches"]),
        "concurrent_searches": int(match["concurrent_searches"]),
        "threads": int(match["threads"]),
        "sequential_warning": _flag(match["sequential_warning"]),
        "concurrent_warning": _flag(match["concurrent_warning"]),
        "sequential_failure": _flag(match["sequential_failure"]),
        "concurrent_failure": _flag(match["concurrent_failure"]),
        "ratio": float(match["ratio"]),
        **identity,
    }
    if record["sequential_searches"] != 1600 or record["concurrent_searches"] != 1600:
        raise ValueError("both arms must execute exactly 1600 searches")
    if record["threads"] != 8:
        raise ValueError("concurrent arm must declare exactly 8 threads")

    expected = {
        "sequential_warning": record["sequential_ns"] >= 200_000_000,
        "concurrent_warning": record["concurrent_ns"] >= 80_000_000,
        "sequential_failure": record["sequential_ns"] > 500_000_000,
        "concurrent_failure": record["concurrent_ns"] > 100_000_000,
    }
    if any(record[field] != value for field, value in expected.items()):
        raise ValueError("AC-081 marker flags disagree with full-precision durations")
    record["numeric_pass"] = not record["sequential_failure"] and not record["concurrent_failure"]

    passed = bool(re.search(r"test result: ok\. 1 passed; 0 failed", text))
    failed = bool(re.search(r"test result: FAILED\. 0 passed; 1 failed", text))
    expected_exit = 0 if record["numeric_pass"] else 101
    if exit_code != expected_exit or (record["numeric_pass"] and not passed) or (
        not record["numeric_pass"] and not failed
    ):
        raise ValueError("test execution status disagrees with the numeric oracle")
    return record


def _cpu_count(affinity: str) -> int:
    count = 0
    for item in affinity.split(","):
        bounds = item.split("-", maxsplit=1)
        count += 1 if len(bounds) == 1 else int(bounds[1]) - int(bounds[0]) + 1
    return count


def _quota_cpus(value: str) -> float:
    quota, period = value.split()
    return float("inf") if quota == "max" else int(quota) / int(period)


def qualify_environment(start: dict[str, Any], end: dict[str, Any]) -> dict[str, Any]:
    reasons: list[str] = []
    for phase, sample in (("start", start), ("end", end)):
        missing = [field for field in REQUIRED_ENVIRONMENT if field not in sample]
        if missing:
            reasons.append(f"{phase} missing: {','.join(missing)}")
    if reasons:
        return {"applicable": False, "reasons": reasons}

    if max(start["load_1m"], end["load_1m"]) / min(start["online_cpus"], end["online_cpus"]) > 0.5:
        reasons.append("load")
    if min(start["available_memory_percent"], end["available_memory_percent"]) < 25:
        reasons.append("memory")
    if end["pswpin"] - start["pswpin"] != 0 or end["pswpout"] - start["pswpout"] != 0:
        reasons.append("swap")
    if max(start["cpu_temp_c"], end["cpu_temp_c"]) > 90:
        reasons.append("temperature")
    if start["thermal_signal"] != "k10temp:Tctl" or end["thermal_signal"] != "k10temp:Tctl":
        reasons.append("thermal signal")
    if start["competing_processes"] or end["competing_processes"]:
        reasons.append("competing process")
    if start["cpu_affinity"] != end["cpu_affinity"] or _cpu_count(start["cpu_affinity"]) < 8:
        reasons.append("affinity")
    try:
        if start["cpu_quota"] != end["cpu_quota"] or _quota_cpus(start["cpu_quota"]) < 8:
            reasons.append("quota")
    except (ValueError, ZeroDivisionError):
        reasons.append("quota")
    governors = start["scaling_governors"]
    if governors != end["scaling_governors"] or not governors or set(governors) != {"performance"}:
        reasons.append("governor")
    return {"applicable": not reasons, "reasons": reasons}


def validate_campaign(observations: list[dict[str, Any]], identity: dict[str, str]) -> None:
    if len(observations) != 7:
        raise ValueError("campaign requires exactly seven observations")
    labels = [item.get("label") for item in observations]
    if len(set(labels)) != 7:
        raise ValueError("campaign observation labels must be unique")
    for item in observations:
        if any(item.get(field) != identity.get(field) for field in REQUIRED_IDENTITY):
            raise ValueError("campaign identity drift")
        if not item.get("numeric_pass") or not item.get("environment_applicable"):
            raise ValueError("every campaign observation must be numerically passing and applicable")


def summarize(observations: list[dict[str, Any]]) -> dict[str, Any]:
    warnings = [
        item["label"]
        for item in observations
        if item.get("sequential_warning") or item.get("concurrent_warning")
    ]
    applicable = all(item.get("environment_applicable") for item in observations)
    numeric = all(item.get("numeric_pass") for item in observations)
    if not applicable:
        status = "ENVIRONMENT_INVALID"
    elif not numeric:
        status = "FAIL"
    elif warnings:
        status = "PASS_WITH_WARNING"
    else:
        status = "PASS"
    return {
        "status": status,
        "warning_labels": warnings,
        "sequential_median_ns": median(item["sequential_ns"] for item in observations),
        "concurrent_median_ns": median(item["concurrent_ns"] for item in observations),
        "observations": observations,
    }


def render_human_summary(summary: dict[str, Any]) -> str:
    warning = ",".join(summary["warning_labels"]) or "none"
    return (
        f"AC-081a/b {summary['status']}: sequential median "
        f"{summary['sequential_median_ns']} ns; concurrent median "
        f"{summary['concurrent_median_ns']} ns; WARNING runs={warning}"
    )


def select_binary(build_json: Path, destination: Path) -> dict[str, str]:
    candidates = []
    for line in build_json.read_text(encoding="utf-8").splitlines():
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        target = record.get("target", {})
        executable = record.get("executable")
        if record.get("profile", {}).get("test") and target.get("name") == "perf_gates" and executable:
            candidates.append(Path(executable))
    if len(candidates) != 1:
        raise ValueError(f"expected one perf_gates test executable, found {len(candidates)}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        raise ValueError(f"sealed destination already exists: {destination}")
    shutil.copy2(candidates[0], destination)
    digest = hashlib.sha256(destination.read_bytes()).hexdigest()
    return {"binary": str(destination), "binary_sha256": digest}


def check_test_list(binary: Path) -> None:
    output = subprocess.run([str(binary), "--list"], check=True, capture_output=True, text=True).stdout
    if output.count("ac_081_absolute_read_performance: test") != 1:
        raise ValueError("AC-081 selector must occur exactly once")
    for name in (
        "ac_020_reads_do_not_serialize_on_a_single_reader_connection",
        "ac_020_reads_do_not_serialize_on_a_single_reader_connection_diagnostics",
    ):
        if output.count(f"{name}: test") != 1:
            raise ValueError(f"retired selector missing from compiled history: {name}")
        retired = subprocess.run(
            [str(binary), "--exact", name, "--nocapture", "--test-threads=1"],
            check=True,
            capture_output=True,
            text=True,
            env={**__import__("os").environ, "AGENT_LONG": "1"},
        )
        combined = retired.stdout + retired.stderr
        if not re.search(r"0 passed; 0 failed; 1 ignored", combined):
            raise ValueError(f"retired selector remains active: {name}")


def main() -> int:
    import argparse

    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    select = commands.add_parser("select-binary")
    select.add_argument("--build-json", type=Path, required=True)
    select.add_argument("--copy-to", type=Path, required=True)
    listing = commands.add_parser("check-test-list")
    listing.add_argument("--binary", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "select-binary":
        print(json.dumps(select_binary(args.build_json, args.copy_to), sort_keys=True))
    else:
        check_test_list(args.binary)
        print("AC-081 selector and retired AC-020 registrations present")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
