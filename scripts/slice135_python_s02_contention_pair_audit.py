#!/usr/bin/env python3
"""Independently audit every installed-Python S02 contention campaign receipt."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics

import slice135_python_s02_contention_audit as run_audit


ROOT = Path(__file__).resolve().parents[1]
PLAN = ROOT / "dev/plans/0.8.27/features/slice-135"
FROZEN = PLAN / "s02-python-contention-comparison-protocol.json"
COMPARISON = PLAN / "s02-python-vector-repaired-comparison-protocol.json"
RUNNER = ROOT / "scripts/slice135_python_s02_contention.py"
AUDITOR = ROOT / "scripts/slice135_python_s02_contention_audit.py"
BLOCK = ROOT / "scripts/slice135_python_s02_contention_block.py"
CAMPAIGN = ROOT / "scripts/slice135_python_s02_contention_campaign.py"


def sha(path: Path) -> str:
    """Hash exact retained bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def nearest_rank(values: list[int], fraction: float) -> int:
    """Return the untrimmed nearest-rank percentile."""
    return sorted(values)[math.ceil(len(values) * fraction) - 1]


def delta_percent(baseline: int, candidate: int) -> float:
    """Report candidate change at the same measured boundary."""
    return (candidate - baseline) * 100 / baseline


def validate_environment(environment: dict) -> list[str]:
    """Recompute host invalidators and warnings from recorded snapshots."""
    start = environment.get("start", {})
    end = environment.get("end", {})
    stable = ("host", "kernel", "cpu", "storage", "governor", "toolchain", "profiler")
    if any(not start.get(key) or start[key] != end.get(key) for key in stable):
        raise ValueError("contention environment identity drift")
    if start.get("competing_jobs") or end.get("competing_jobs"):
        raise ValueError("contention competing jobs")
    if min(start.get("disk_free_bytes", 0), end.get("disk_free_bytes", 0)) < 1_073_741_824:
        raise ValueError("contention disk pressure")
    before, after = start.get("swap_pages"), end.get("swap_pages")
    if type(before) is not int or type(after) is not int or after < before:
        raise ValueError("contention host swap counter invalid")
    warnings = [f"host swap drift: {after - before} pages"] if after > before else []
    if environment.get("invalidators") != []:
        raise ValueError("contention environment invalidators present")
    if not set(warnings).issubset(environment.get("warnings", [])):
        raise ValueError("contention host warning missing")
    return warnings


def validate_order(entries: list[dict], *, pairs: int, minimum_idle_seconds: int) -> None:
    """Reject missing/reordered blocks, failed drivers and insufficient idle."""
    expected = [
        (pair, role, f"pair-{pair:02d}-{role}")
        for pair in range(1, pairs + 1)
        for role in (("baseline", "candidate") if pair % 2 else ("candidate", "baseline"))
    ]
    observed = [(row.get("pair"), row.get("role"), row.get("block")) for row in entries]
    if observed != expected:
        raise ValueError("contention block order or count changed")
    previous_end = None
    for row in entries:
        start, end = row.get("start_monotonic_ns"), row.get("end_monotonic_ns")
        if type(start) is not int or type(end) is not int or end <= start:
            raise ValueError("contention block interval invalid")
        if previous_end is not None and start - previous_end < minimum_idle_seconds * 1_000_000_000:
            raise ValueError("contention block idle interval too short")
        if row.get("exit_code") != 0:
            raise ValueError("contention block driver failed")
        previous_end = end


def summarize(
    by_block: dict[str, list[int]], warnings: dict[str, list[str]],
    *, pairs: int, samples_per_block: int,
) -> dict:
    """Recompute pooled and paired distributions from every measured sample."""
    required = {
        f"pair-{pair:02d}-{role}"
        for pair in range(1, pairs + 1)
        for role in ("baseline", "candidate")
    }
    if set(by_block) != required or any(
        len(values) != samples_per_block or any(type(value) is not int or value <= 0 for value in values)
        for values in by_block.values()
    ):
        raise ValueError("contention sample count or values invalid")
    pooled = {
        role: [value for pair in range(1, pairs + 1)
               for value in by_block[f"pair-{pair:02d}-{role}"]]
        for role in ("baseline", "candidate")
    }
    distributions = {
        role: {
            "count": len(values), "p50_ns": nearest_rank(values, .5),
            "p95_ns": nearest_rank(values, .95),
            "min_ns": min(values), "max_ns": max(values),
        }
        for role, values in pooled.items()
    }
    pair_rows = []
    for pair in range(1, pairs + 1):
        baseline = f"pair-{pair:02d}-baseline"
        candidate = f"pair-{pair:02d}-candidate"
        base_p50 = nearest_rank(by_block[baseline], .5)
        cand_p50 = nearest_rank(by_block[candidate], .5)
        base_p95 = nearest_rank(by_block[baseline], .95)
        cand_p95 = nearest_rank(by_block[candidate], .95)
        pair_rows.append({
            "pair": pair,
            "baseline_p50_ns": base_p50, "candidate_p50_ns": cand_p50,
            "p50_delta_percent": delta_percent(base_p50, cand_p50),
            "baseline_p95_ns": base_p95, "candidate_p95_ns": cand_p95,
            "p95_delta_percent": delta_percent(base_p95, cand_p95),
            "warnings": sorted(set(warnings.get(baseline, []) + warnings.get(candidate, []))),
        })
    deltas = [row["p50_delta_percent"] for row in pair_rows]
    clean = [row["p50_delta_percent"] for row in pair_rows if not row["warnings"]]
    return {
        "pooled": distributions,
        "pooled_p50_delta_percent": delta_percent(
            distributions["baseline"]["p50_ns"], distributions["candidate"]["p50_ns"]
        ),
        "pooled_p95_delta_percent": delta_percent(
            distributions["baseline"]["p95_ns"], distributions["candidate"]["p95_ns"]
        ),
        "pairs": pair_rows,
        "median_pair_p50_delta_percent": statistics.median(deltas),
        "pair_p50_delta_range_percent": [min(deltas), max(deltas)],
        "warned_pairs": [row["pair"] for row in pair_rows if row["warnings"]],
        "warning_free_median_pair_p50_delta_percent": statistics.median(clean) if clean else None,
    }


def audit_block(directory: Path, *, role: str, protocol: dict, wheel: Path) -> tuple[list[int], list[str], dict]:
    """Reopen raw processes and databases rather than trusting block summaries."""
    summary = json.loads((directory / "summary.json").read_text())
    attempts = json.loads((directory / "attempts.json").read_text())
    environment = json.loads((directory / "environment.json").read_text())
    count = protocol["samples_per_block"]
    if (
        summary.get("status") != "VALID_S02_PYTHON_CONTENTION_BLOCK"
        or summary.get("identity") != {
            "source_sha": protocol[role]["source_sha"], "role": role,
            "wheel_sha256": protocol[role]["wheel_sha256"],
            "comparison_protocol_sha256": sha(COMPARISON),
            "runner_sha256": sha(RUNNER), "auditor_sha256": sha(AUDITOR),
            "block_runner_sha256": sha(BLOCK),
        }
        or summary.get("samples") != count or summary.get("warmups") != 1
        or summary.get("valid_samples") != count
        or summary.get("attempts_sha256") != sha(directory / "attempts.json")
        or summary.get("environment_sha256") != sha(directory / "environment.json")
        or summary.get("invalidators") != []
        or environment.get("invalidators") != []
    ):
        raise ValueError(f"{directory.name} block identity or validity invalid")
    host_warnings = validate_environment(environment)
    labels = ["warmup", *(f"sample-{number:03d}" for number in range(1, count + 1))]
    if [row.get("label") for row in attempts] != labels or any(not row.get("valid") for row in attempts):
        raise ValueError(f"{directory.name} attempt count, order or validity invalid")
    values = []
    peak_rss = []
    overlaps = []
    resource_warnings = []
    for index, (label, attempt) in enumerate(zip(labels, attempts)):
        stem = directory / label
        raw = Path(str(stem) + ".json")
        database = Path(str(stem) + ".sqlite")
        resource = Path(str(stem) + ".resource")
        audit_path = Path(str(stem) + ".audit.json")
        checked = run_audit.audit_run(
            raw_path=raw, database=database, wheel=wheel,
            protocol=COMPARISON, runner=RUNNER,
            stdout=Path(str(stem) + ".stdout"),
            stderr=Path(str(stem) + ".stderr"), resource=resource,
        )
        faults = run_audit.resource_report(resource)["major_faults"]
        if faults:
            resource_warnings.append(f"{label} child major faults: {faults}")
        if (
            json.loads(audit_path.read_text()) != checked
            or attempt.get("elapsed_ns") != checked["elapsed_ns"]
            or attempt.get("audit_sha256") != sha(audit_path)
            or attempt.get("raw_sha256") != sha(raw)
            or attempt.get("database_sha256") != sha(database)
            or attempt.get("resource_sha256") != sha(resource)
        ):
            raise ValueError(f"{directory.name}/{label} raw receipt changed")
        if index:
            values.append(checked["elapsed_ns"])
            peak_rss.append(checked["peak_rss_kib"])
            overlaps.append(checked["actual_call_overlap_cycles"])
    if (
        summary.get("p50_ns") != nearest_rank(values, .5)
        or summary.get("p95_ns") != nearest_rank(values, .95)
        or summary.get("min_ns") != min(values)
        or summary.get("max_ns") != max(values)
        or summary.get("warnings") != environment.get("warnings")
        or summary.get("warnings") != sorted(set(host_warnings + resource_warnings))
    ):
        raise ValueError(f"{directory.name} block statistics or environment changed")
    return values, summary["warnings"], {
        "samples": count, "p50_ns": nearest_rank(values, .5),
        "p95_ns": nearest_rank(values, .95),
        "peak_rss_kib_p50": nearest_rank(peak_rss, .5),
        "actual_call_overlap_cycles_min": min(overlaps),
        "warnings": summary["warnings"],
        "summary_sha256": sha(directory / "summary.json"),
    }


def audit_campaign(directory: Path, *, baseline_wheel: Path, candidate_wheel: Path) -> dict:
    """Verify frozen source, every process and paired timing from raw bytes."""
    protocol = json.loads(FROZEN.read_text())
    expected_hashes = {
        "runner": RUNNER, "auditor": AUDITOR, "block_runner": BLOCK,
        "campaign_runner": CAMPAIGN, "comparison_protocol": COMPARISON,
        "baseline_wheel": baseline_wheel, "candidate_wheel": candidate_wheel,
    }
    if protocol.get("status") != "FROZEN_S02_PYTHON_CONTENTION_PAIRED" or any(
        protocol["frozen_sha256"].get(key) != sha(path)
        for key, path in expected_hashes.items()
    ):
        raise ValueError("contention protocol or frozen artifact changed")
    campaign = json.loads((directory / "campaign-summary.json").read_text())
    order_path = directory / "run-order.jsonl"
    entries = [json.loads(line) for line in order_path.read_text().splitlines()]
    validate_order(entries, pairs=protocol["pairs"],
                   minimum_idle_seconds=protocol["minimum_idle_seconds"])
    if (
        campaign.get("status") != "VALID_S02_PYTHON_CONTENTION_CAMPAIGN"
        or campaign.get("protocol_sha256") != sha(FROZEN)
        or campaign.get("run_order_sha256") != sha(order_path)
        or campaign.get("blocks") != len(entries)
        or campaign.get("pairs") != protocol["pairs"]
        or campaign.get("samples_per_block") != protocol["samples_per_block"]
    ):
        raise ValueError("contention campaign summary invalid")
    by_block = {}
    warnings = {}
    block_reports = {}
    for entry in entries:
        name, role = entry["block"], entry["role"]
        block_dir = directory / name
        if entry.get("summary_sha256") != sha(block_dir / "summary.json"):
            raise ValueError(f"{name} summary hash changed")
        values, block_warnings, report = audit_block(
            block_dir, role=role, protocol=protocol,
            wheel=baseline_wheel if role == "baseline" else candidate_wheel,
        )
        by_block[name] = values
        warnings[name] = block_warnings
        block_reports[name] = report
    result = summarize(by_block, warnings, pairs=protocol["pairs"],
                       samples_per_block=protocol["samples_per_block"])
    result.update({
        "schema_version": 1,
        "status": "VALID_S02_PYTHON_CONTENTION_PAIRED_AUDIT",
        "protocol_sha256": sha(FROZEN),
        "campaign_summary_sha256": sha(directory / "campaign-summary.json"),
        "run_order_sha256": sha(order_path),
        "source_sha": {role: protocol[role]["source_sha"] for role in ("baseline", "candidate")},
        "wheel_sha256": {role: protocol[role]["wheel_sha256"] for role in ("baseline", "candidate")},
        "samples_per_version": protocol["pairs"] * protocol["samples_per_block"],
        "blocks": block_reports,
    })
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--campaign", required=True, type=Path)
    parser.add_argument("--baseline-wheel", required=True, type=Path)
    parser.add_argument("--candidate-wheel", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    result = audit_campaign(
        args.campaign, baseline_wheel=args.baseline_wheel,
        candidate_wheel=args.candidate_wheel,
    )
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(result["status"])


if __name__ == "__main__":
    main()
