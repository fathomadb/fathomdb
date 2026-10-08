#!/usr/bin/env python3
"""Run the frozen alternating S03 installed-Python comparison schedule."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
from time import monotonic, sleep

import slice135_python_s03_pair_block as block


def planned_blocks() -> list[tuple[int, int, str, str]]:
    """Use five order-balanced version pairs at each declared corpus size."""
    return [
        (size, index, role, f"{size}-pair-{index:02}-{role}")
        for size in (32, 256)
        for index in range(1, 6)
        for role in (
            ("baseline", "candidate") if index % 2 else ("candidate", "baseline")
        )
    ]


def run_campaign(
    *,
    spec_path: Path,
    baseline_checkout: Path,
    baseline_wheel: Path,
    baseline_python: Path,
    candidate_checkout: Path,
    candidate_wheel: Path,
    candidate_python: Path,
    output: Path,
) -> dict:
    """Retain every planned attempt, including failures and host warnings."""
    spec = json.loads(spec_path.read_text())
    if (
        spec.get("idle_seconds") != 20
        or spec.get("pairs_per_size") != 5
        or spec.get("campaign_runner_sha256") != block.digest(Path(__file__))
        or spec.get("block_runner_sha256") != block.digest(Path(block.__file__))
        or spec.get("workload_auditor_sha256")
        != block.digest(Path(block.audit.__file__))
    ):
        raise ValueError("S03 paired executable protocol differs from frozen scripts")
    for role in ("baseline", "candidate"):
        identity = spec[role]
        block.validate_spec(
            spec,
            role=role,
            size=32,
            source_sha=identity["source_sha"],
            wheel_sha256=identity["wheel_sha256"],
        )
    inputs = {
        "baseline": (baseline_checkout, baseline_wheel, baseline_python),
        "candidate": (candidate_checkout, candidate_wheel, candidate_python),
    }
    output.mkdir(parents=True, exist_ok=False)
    manifest = {
        "schema_version": 1,
        "status": "FROZEN_S03_PYTHON_PAIRED_CAMPAIGN",
        "protocol_sha256": block.digest(spec_path),
        "campaign_runner_sha256": block.digest(Path(__file__)),
        "block_runner_sha256": block.digest(Path(block.__file__)),
        "workload_runner_sha256": block.digest(block.RUNNER),
        "workload_auditor_sha256": block.digest(Path(block.audit.__file__)),
        "schedule": [name for _, _, _, name in planned_blocks()],
    }
    block.write_json(output / "campaign-manifest.json", manifest)
    records = []
    order = output / "run-order.jsonl"
    for ordinal, (size, index, role, name) in enumerate(planned_blocks(), 1):
        if ordinal > 1:
            sleep(20)
        started = datetime.now(timezone.utc).isoformat()
        start_monotonic = monotonic()
        checkout, wheel, venv_python = inputs[role]
        try:
            attempt = block.run_block(
                spec_path=spec_path,
                role=role,
                checkout=checkout,
                wheel=wheel,
                venv_python=venv_python,
                size=size,
                output=output / name,
            )
            status = attempt["status"]
            invalidators = attempt["invalidators"]
        except Exception as error:
            status = "BLOCK_RUNNER_ERROR"
            invalidators = [f"{type(error).__name__}: {error}"]
        record = {
            "ordinal": ordinal,
            "size": size,
            "pair_index": index,
            "role": role,
            "name": name,
            "started_utc": started,
            "finished_utc": datetime.now(timezone.utc).isoformat(),
            "elapsed_s": monotonic() - start_monotonic,
            "status": status,
            "invalidators": invalidators,
        }
        records.append(record)
        with order.open("a") as stream:
            stream.write(json.dumps(record, sort_keys=True) + "\n")
        print(json.dumps(record, sort_keys=True), flush=True)
    summary = {
        "status": "VALID_S03_PYTHON_PAIRED_CAMPAIGN"
        if all(item["status"] == "VALID_S03_PAIRED_BLOCK" for item in records)
        else "INCOMPLETE_S03_PYTHON_PAIRED_CAMPAIGN",
        "valid_blocks": sum(
            item["status"] == "VALID_S03_PAIRED_BLOCK" for item in records
        ),
        "planned_blocks": len(records),
    }
    block.write_json(output / "campaign-summary.json", summary)
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spec-path", type=Path, required=True)
    parser.add_argument("--baseline-checkout", type=Path, required=True)
    parser.add_argument("--baseline-wheel", type=Path, required=True)
    parser.add_argument("--baseline-python", type=Path, required=True)
    parser.add_argument("--candidate-checkout", type=Path, required=True)
    parser.add_argument("--candidate-wheel", type=Path, required=True)
    parser.add_argument("--candidate-python", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    summary = run_campaign(**vars(args))
    print(json.dumps(summary, sort_keys=True))
    return 0 if summary["status"] == "VALID_S03_PYTHON_PAIRED_CAMPAIGN" else 1


if __name__ == "__main__":
    raise SystemExit(main())
