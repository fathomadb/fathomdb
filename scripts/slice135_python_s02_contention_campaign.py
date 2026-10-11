#!/usr/bin/env python3
"""Run a frozen alternating 0.8.26/0.8.27 Python S02 contention campaign."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parents[1]
BLOCK = ROOT / "scripts/slice135_python_s02_contention_block.py"
RUNNER = ROOT / "scripts/slice135_python_s02_contention.py"
AUDITOR = ROOT / "scripts/slice135_python_s02_contention_audit.py"


def sha(path: Path) -> str:
    """Hash exact artifact bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def order_for_pairs(pairs: int) -> list[tuple[int, str]]:
    """Alternate the first version in each preassigned pair."""
    if pairs < 1:
        raise ValueError("at least one version pair required")
    return [
        (number, role)
        for number in range(1, pairs + 1)
        for role in (
            ("baseline", "candidate") if number % 2 else ("candidate", "baseline")
        )
    ]


def validate_freeze(protocol: dict, values: dict[str, Path]) -> None:
    """Reject changed runners, package bytes or declared sample rules."""
    if protocol.get("status") != "FROZEN_S02_PYTHON_CONTENTION_PAIRED":
        raise ValueError("contention comparison protocol not frozen")
    if protocol.get("pairs") != 5 or protocol.get("samples_per_block") != 20:
        raise ValueError("contention sample rule changed")
    if protocol.get("pair_order") != [list(item) for item in order_for_pairs(5)]:
        raise ValueError("contention pair order changed")
    for key, artifact in values.items():
        if protocol["frozen_sha256"].get(key) != sha(artifact):
            raise ValueError(f"contention frozen {key} changed")


def run_campaign(
    *, protocol_path: Path, output: Path,
    baseline_checkout: Path, baseline_python: Path, baseline_wheel: Path,
    candidate_checkout: Path, candidate_python: Path, candidate_wheel: Path,
) -> dict:
    """Keep ten blocks serial, each with fresh databases and a bounded wait."""
    protocol = json.loads(protocol_path.read_text())
    comparison = ROOT / "dev/plans/0.8.27/features/slice-135/s02-python-vector-repaired-comparison-protocol.json"
    validate_freeze(protocol, {
        "runner": RUNNER,
        "auditor": AUDITOR,
        "block_runner": BLOCK,
        "campaign_runner": Path(__file__),
        "comparison_protocol": comparison,
        "baseline_wheel": baseline_wheel,
        "candidate_wheel": candidate_wheel,
    })
    roles = {
        "baseline": (baseline_checkout, baseline_python, baseline_wheel),
        "candidate": (candidate_checkout, candidate_python, candidate_wheel),
    }
    output.mkdir(parents=True, exist_ok=False)
    order_path = output / "run-order.jsonl"
    results = []
    last_end = None
    for pair, role in order_for_pairs(protocol["pairs"]):
        if last_end is not None:
            remaining = protocol["minimum_idle_seconds"] - (time.monotonic() - last_end)
            if remaining > 0:
                time.sleep(remaining)
        checkout, python, wheel = roles[role]
        block_output = output / f"pair-{pair:02d}-{role}"
        command = [
            sys.executable, str(BLOCK),
            "--role", role, "--checkout", str(checkout),
            "--python", str(python), "--wheel", str(wheel),
            "--comparison-protocol", str(comparison),
            "--samples", str(protocol["samples_per_block"]),
            "--output", str(block_output),
        ]
        start_ns = time.monotonic_ns()
        start_utc = datetime.now(timezone.utc).isoformat()
        try:
            child = subprocess.run(
                command, capture_output=True, text=True, check=False,
                timeout=protocol["block_timeout_seconds"],
            )
            stdout, stderr, exit_code = child.stdout, child.stderr, child.returncode
        except subprocess.TimeoutExpired as error:
            stdout = (error.stdout or b"").decode(errors="replace")
            stderr = (error.stderr or b"").decode(errors="replace")
            exit_code = 124
        end_ns = time.monotonic_ns()
        last_end = time.monotonic()
        (output / f"pair-{pair:02d}-{role}.driver.stdout").write_text(stdout)
        (output / f"pair-{pair:02d}-{role}.driver.stderr").write_text(stderr)
        (output / f"pair-{pair:02d}-{role}.driver.command.json").write_text(json.dumps(command) + "\n")
        summary_path = block_output / "summary.json"
        row = {
            "pair": pair, "role": role, "start_monotonic_ns": start_ns,
            "end_monotonic_ns": end_ns, "start_utc": start_utc,
            "exit_code": exit_code, "block": block_output.name,
            "summary_sha256": sha(summary_path) if summary_path.is_file() else None,
        }
        with order_path.open("a") as stream:
            stream.write(json.dumps(row) + "\n")
        results.append(row)
        if exit_code != 0 or not summary_path.is_file() or json.loads(summary_path.read_text()).get("status") != "VALID_S02_PYTHON_CONTENTION_BLOCK":
            break
    valid = len(results) == 10 and all(row["exit_code"] == 0 for row in results)
    summary = {
        "status": "VALID_S02_PYTHON_CONTENTION_CAMPAIGN" if valid else "INVALID_S02_PYTHON_CONTENTION_CAMPAIGN",
        "protocol_sha256": sha(protocol_path),
        "run_order_sha256": sha(order_path),
        "blocks": len(results), "pairs": protocol["pairs"],
        "samples_per_block": protocol["samples_per_block"],
    }
    (output / "campaign-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    return summary


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    for role in ("baseline", "candidate"):
        for name in ("checkout", "python", "wheel"):
            parser.add_argument(f"--{role}-{name}", required=True, type=Path)
    arguments = parser.parse_args()
    summary = run_campaign(
        protocol_path=arguments.protocol.resolve(), output=arguments.output.absolute(),
        baseline_checkout=arguments.baseline_checkout.resolve(),
        baseline_python=arguments.baseline_python.absolute(),
        baseline_wheel=arguments.baseline_wheel.resolve(),
        candidate_checkout=arguments.candidate_checkout.resolve(),
        candidate_python=arguments.candidate_python.absolute(),
        candidate_wheel=arguments.candidate_wheel.resolve(),
    )
    print(summary["status"])
    if summary["status"] != "VALID_S02_PYTHON_CONTENTION_CAMPAIGN":
        sys.exit(1)


if __name__ == "__main__":
    main()
