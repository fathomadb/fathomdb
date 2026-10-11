#!/usr/bin/env python3
"""Run five S03 baseline-only noise blocks at each declared corpus size."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
from time import monotonic, sleep

import slice135_python_s03_block as block


def planned_blocks() -> list[tuple[int, int, str]]:
    """Return the baseline-only schedule fixed before paired timing."""
    return [
        (size, index, f"{size}-block-{index:02}")
        for size in (32, 256)
        for index in range(1, 6)
    ]


def run_campaign(
    *,
    checkout: Path,
    wheel: Path,
    wheel_sha256: str,
    venv_python: Path,
    output: Path,
    idle_seconds: int,
) -> dict:
    """Keep every block and its order even when one is invalid."""
    if idle_seconds < 20:
        raise ValueError(
            "S03 baseline pilot requires at least 20 seconds between blocks"
        )
    output.mkdir(parents=True, exist_ok=False)
    manifest = {
        "schema_version": 1,
        "status": "S03_BASELINE_NOISE_PILOT_NOT_FROZEN_COMPARISON",
        "baseline_source_sha": block.BASELINE_SOURCE,
        "wheel_sha256": wheel_sha256,
        "block_runner_sha256": block.digest(Path(block.__file__)),
        "auditor_sha256": block.digest(Path(block.audit.__file__)),
        "workload_runner_sha256": block.digest(block.RUNNER),
        "campaign_runner_sha256": block.digest(Path(__file__)),
        "repetitions_per_cell": 100,
        "idle_seconds": idle_seconds,
        "blocks": [name for _, _, name in planned_blocks()],
    }
    block.write_json(output / "pilot-manifest.json", manifest)
    records = []
    order = output / "run-order.jsonl"
    for ordinal, (size, index, name) in enumerate(planned_blocks(), 1):
        if ordinal > 1:
            sleep(idle_seconds)
        started = datetime.now(timezone.utc).isoformat()
        start_monotonic = monotonic()
        try:
            attempt = block.run_block(
                checkout=checkout,
                wheel=wheel,
                wheel_sha256=wheel_sha256,
                venv_python=venv_python,
                size=size,
                repetitions=100,
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
            "block_index": index,
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
        "status": "VALID_BASELINE_NOISE_PILOT"
        if all(item["status"] == "VALID_BASELINE_S03_PILOT_BLOCK" for item in records)
        else "INCOMPLETE_BASELINE_NOISE_PILOT",
        "valid_blocks": sum(
            item["status"] == "VALID_BASELINE_S03_PILOT_BLOCK" for item in records
        ),
        "planned_blocks": len(records),
    }
    block.write_json(output / "campaign-summary.json", summary)
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", type=Path, required=True)
    parser.add_argument("--wheel", type=Path, required=True)
    parser.add_argument("--wheel-sha256", required=True)
    parser.add_argument("--venv-python", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--idle-seconds", type=int, default=20)
    args = parser.parse_args()
    summary = run_campaign(**vars(args))
    print(json.dumps(summary, sort_keys=True))
    return 0 if summary["status"] == "VALID_BASELINE_NOISE_PILOT" else 1


if __name__ == "__main__":
    raise SystemExit(main())
