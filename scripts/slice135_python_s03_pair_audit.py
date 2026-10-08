#!/usr/bin/env python3
"""Independently recompute S03 paired samples, order and persisted state."""

from __future__ import annotations

import argparse
from datetime import datetime
import json
from pathlib import Path
from typing import Any

import slice135_python_s03_audit as sample_audit
import slice135_python_s03_baseline_audit as pilot_audit


ROOT = Path(__file__).resolve().parents[1]


def expected_blocks() -> list[tuple[int, int, str, str]]:
    """Define the order independently of the campaign producer."""
    return [
        (size, index, role, f"{size}-pair-{index:02}-{role}")
        for size in (32, 256)
        for index in range(1, 6)
        for role in (
            ("baseline", "candidate") if index % 2 else ("candidate", "baseline")
        )
    ]


def check_order(records: list[dict[str, Any]]) -> None:
    """Reject missing, reordered or under-idled version blocks."""
    if len(records) != 20:
        raise ValueError("S03 paired schedule must contain twenty records")
    previous_finished = None
    for ordinal, (record, (size, index, role, name)) in enumerate(
        zip(records, expected_blocks()), 1
    ):
        if (
            record.get("ordinal"),
            record.get("size"),
            record.get("pair_index"),
            record.get("role"),
            record.get("name"),
        ) != (ordinal, size, index, role, name):
            raise ValueError("S03 paired block order differs from frozen schedule")
        if "started_utc" not in record or "finished_utc" not in record:
            continue
        started = datetime.fromisoformat(record["started_utc"])
        finished = datetime.fromisoformat(record["finished_utc"])
        if finished <= started:
            raise ValueError(f"{name}: nonpositive block interval")
        if (
            previous_finished is not None
            and (started - previous_finished).total_seconds() < 19.9
        ):
            raise ValueError(f"{name}: idle interval below 20 seconds")
        previous_finished = finished


def _delta_pct(candidate: int, baseline: int) -> float:
    return (candidate / baseline - 1.0) * 100.0


def audit(directory: Path, spec_path: Path) -> dict[str, Any]:
    """Audit twenty raw blocks without trusting their producer summaries."""
    spec = json.loads(spec_path.read_text())
    if (
        spec.get("schema_version") != 1
        or spec.get("status") != "FROZEN_S03_PYTHON_PAIRED"
        or spec.get("sizes") != [32, 256]
        or spec.get("samples_per_case") != 100
        or spec.get("pairs_per_size") != 5
        or spec.get("idle_seconds") != 20
    ):
        raise ValueError("S03 comparison protocol declaration changed")
    for role in ("baseline", "candidate"):
        identity = spec.get(role, {})
        if (
            len(identity.get("source_sha", "")) != 40
            or len(identity.get("wheel_sha256", "")) != 64
        ):
            raise ValueError(f"{role} source or wheel identity malformed")
    expected_files = {
        "input_sha256": {
            str(path.relative_to(ROOT)): sample_audit.digest(path)
            for path in (
                sample_audit.RUNNER,
                sample_audit.FIXTURE,
                sample_audit.S01,
                sample_audit.S02,
                ROOT / "scripts/slice135_python_s01_block.py",
            )
        },
        "block_runner_sha256": sample_audit.digest(
            ROOT / "scripts/slice135_python_s03_pair_block.py"
        ),
        "campaign_runner_sha256": sample_audit.digest(
            ROOT / "scripts/slice135_python_s03_pair_campaign.py"
        ),
        "workload_auditor_sha256": sample_audit.digest(
            ROOT / "scripts/slice135_python_s03_audit.py"
        ),
        "paired_auditor_sha256": sample_audit.digest(Path(__file__)),
        "resource_auditor_sha256": sample_audit.digest(Path(pilot_audit.__file__)),
    }
    if any(spec.get(key) != value for key, value in expected_files.items()):
        raise ValueError("frozen executable or fixture bytes differ")
    manifest = json.loads((directory / "campaign-manifest.json").read_text())
    if (
        manifest.get("status") != "FROZEN_S03_PYTHON_PAIRED_CAMPAIGN"
        or manifest.get("protocol_sha256") != sample_audit.digest(spec_path)
        or manifest.get("schedule") != [name for _, _, _, name in expected_blocks()]
        or manifest.get("campaign_runner_sha256") != spec["campaign_runner_sha256"]
        or manifest.get("block_runner_sha256") != spec["block_runner_sha256"]
        or manifest.get("workload_auditor_sha256") != spec["workload_auditor_sha256"]
        or manifest.get("workload_runner_sha256")
        != spec["input_sha256"]["scripts/slice135_python_s03.py"]
    ):
        raise ValueError("S03 campaign manifest differs from frozen protocol")
    records = [
        json.loads(line)
        for line in (directory / "run-order.jsonl").read_text().splitlines()
    ]
    check_order(records)
    if json.loads((directory / "campaign-summary.json").read_text()) != {
        "status": "VALID_S03_PYTHON_PAIRED_CAMPAIGN",
        "valid_blocks": 20,
        "planned_blocks": 20,
    }:
        raise ValueError("producer summary differs from twenty valid blocks")
    result_blocks = []
    samples: dict[str, dict[str, dict[str, list[int]]]] = {
        "32": {},
        "256": {},
    }
    pairs: dict[str, dict[int, dict[str, dict[str, dict[str, int | str]]]]] = {
        "32": {},
        "256": {},
    }
    for record, (size, index, role, name) in zip(records, expected_blocks()):
        block_dir = directory / name
        attempt = json.loads((block_dir / "attempt.json").read_text())
        identity = spec[role]
        if (
            record.get("status") != "VALID_S03_PAIRED_BLOCK"
            or attempt.get("status") != record["status"]
            or record.get("invalidators") != []
            or attempt.get("invalidators") != []
            or attempt.get("source_sha") != identity["source_sha"]
            or attempt.get("wheel_sha256") != identity["wheel_sha256"]
            or attempt.get("comparison_protocol_sha256")
            != sample_audit.digest(spec_path)
        ):
            raise ValueError(f"{name}: invalid block or identity")
        block_protocol_path = block_dir / "block-protocol.json"
        block_protocol = json.loads(block_protocol_path.read_text())
        if (
            block_protocol.get("role") != role
            or block_protocol.get("size") != size
            or block_protocol.get("samples_per_case") != 100
            or block_protocol.get("source_sha") != identity["source_sha"]
            or block_protocol.get("wheel_sha256") != identity["wheel_sha256"]
            or block_protocol.get("comparison_protocol_sha256")
            != sample_audit.digest(spec_path)
            or block_protocol.get("runner_sha256")
            != spec["input_sha256"]["scripts/slice135_python_s03.py"]
            or block_protocol.get("block_runner_sha256") != spec["block_runner_sha256"]
            or block_protocol.get("auditor_sha256") != spec["workload_auditor_sha256"]
            or attempt.get("block_protocol_sha256")
            != sample_audit.digest(block_protocol_path)
        ):
            raise ValueError(f"{name}: block protocol differs from freeze")
        recomputed = sample_audit.audit(
            block_dir / "workload",
            size=size,
            source_sha=identity["source_sha"],
            wheel_sha256=identity["wheel_sha256"],
        )
        retained = json.loads((block_dir / "independent-audit.json").read_text())
        if retained != recomputed or attempt.get(
            "summary_sha256"
        ) != sample_audit.digest(block_dir / "independent-audit.json"):
            raise ValueError(f"{name}: independently recomputed result changed")
        if any(cell["count"] != 100 for cell in recomputed["cases"].values()):
            raise ValueError(f"{name}: sample count below frozen floor")
        resources = pilot_audit._resources(block_dir)
        warnings = pilot_audit._environment(block_dir)
        if warnings != attempt.get("warnings"):
            raise ValueError(f"{name}: warning claim changed")
        raw = json.loads((block_dir / "workload/raw.json").read_text())
        for cell, values in raw["cases"].items():
            durations = [
                sample_audit.check_attempt(cell, item, size) for item in values
            ]
            samples[str(size)].setdefault(cell, {}).setdefault(role, []).extend(
                durations
            )
        pairs[str(size)].setdefault(index, {})[role] = recomputed["cases"]
        result_blocks.append(
            {
                "name": name,
                "raw_sha256": recomputed["raw_sha256"],
                "database_sha256": recomputed["database_sha256"],
                "resources": resources,
                "warnings": warnings,
            }
        )
    results = {}
    for size, cells in samples.items():
        results[size] = {}
        for cell, roles in cells.items():
            if set(roles) != {"baseline", "candidate"} or any(
                len(values) != 500 for values in roles.values()
            ):
                raise ValueError(f"{size}/{cell}: incomplete pooled samples")
            baseline = sample_audit.summarize_durations(roles["baseline"])
            candidate = sample_audit.summarize_durations(roles["candidate"])
            pair_results = []
            for index in range(1, 6):
                pair = pairs[size][index]
                pair_results.append(
                    {
                        "index": index,
                        "p50_delta_pct": _delta_pct(
                            int(pair["candidate"][cell]["p50_ns"]),
                            int(pair["baseline"][cell]["p50_ns"]),
                        ),
                        "p95_delta_pct": _delta_pct(
                            int(pair["candidate"][cell]["p95_ns"]),
                            int(pair["baseline"][cell]["p95_ns"]),
                        ),
                    }
                )
            results[size][cell] = {
                "baseline": baseline,
                "candidate": candidate,
                "pooled_p50_delta_pct": _delta_pct(
                    int(candidate["p50_ns"]), int(baseline["p50_ns"])
                ),
                "pooled_p95_delta_pct": _delta_pct(
                    int(candidate["p95_ns"]), int(baseline["p95_ns"])
                ),
                "pairs": pair_results,
            }
    return {
        "status": "AUDITED_S03_PYTHON_PAIRED_SUBSET",
        "protocol_sha256": sample_audit.digest(spec_path),
        "manifest_sha256": sample_audit.digest(directory / "campaign-manifest.json"),
        "run_order_sha256": sample_audit.digest(directory / "run-order.jsonl"),
        "blocks": result_blocks,
        "results": results,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--spec-path", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(audit(args.input, args.spec_path), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
