#!/usr/bin/env python3
"""Independently recompute the installed-TypeScript S02 contention pairs."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics

import slice135_ts_s02_contention_audit as run_audit


ROOT = Path(__file__).resolve().parents[1]
PLAN = ROOT / "dev/plans/0.8.27/features/slice-135"
FROZEN = PLAN / "s02-ts-contention-comparison-protocol.json"
PRIOR = PLAN / "s02-ts-vector-repaired-comparison-protocol.json"
RUNNER = ROOT / "scripts/slice135_ts_s02_contention.mjs"
AUDITOR = ROOT / "scripts/slice135_ts_s02_contention_audit.py"
BLOCK = ROOT / "scripts/slice135_ts_s02_contention_block.py"
CAMPAIGN = ROOT / "scripts/slice135_ts_s02_contention_campaign.py"
S01_HELPER = ROOT / "scripts/slice135_ts_s01.mjs"
S02_HELPER = ROOT / "scripts/slice135_ts_s02.mjs"
STABLE = (
    "host", "kernel", "cpu", "storage", "governor", "toolchain", "profiler",
    "resource_tool", "node_version",
)


def sha(path: Path) -> str:
    """Hash exact retained bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def nearest_rank(values: list[int], fraction: float) -> int:
    """Compute the declared untrimmed percentile."""
    return sorted(values)[math.ceil(len(values) * fraction) - 1]


def delta_percent(baseline: int, candidate: int) -> float:
    """Compute candidate change at the same product boundary."""
    return (candidate - baseline) * 100 / baseline


def validate_order(entries: list[dict], *, pairs: int, minimum_idle_seconds: int) -> None:
    """Reject missing, reordered, failed or too-closely spaced blocks."""
    expected = [
        (pair, role, f"pair-{pair:02d}-{role}")
        for pair in range(1, pairs + 1)
        for role in (("baseline", "candidate") if pair % 2 else ("candidate", "baseline"))
    ]
    if [(row.get("pair"), row.get("role"), row.get("block")) for row in entries] != expected:
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
    """Use all raw measurements for pooled and paired percentiles."""
    required = {
        f"pair-{pair:02d}-{role}"
        for pair in range(1, pairs + 1)
        for role in ("baseline", "candidate")
    }
    if set(by_block) != required or any(
        len(values) != samples_per_block
        or any(type(value) is not int or value <= 0 for value in values)
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


def resource_report(path: Path) -> dict:
    """Parse GNU Time's child-only report without producer helpers."""
    fields = dict(line.split("=", 1) for line in path.read_text().splitlines() if "=" in line)
    expected = {"user_s", "system_s", "peak_rss_kib", "fs_inputs", "fs_outputs",
                "major_faults", "swap_events"}
    if set(fields) != expected:
        raise ValueError("contention resource fields invalid")
    report = {
        key: float(value) if key in {"user_s", "system_s"} else int(value)
        for key, value in fields.items()
    }
    if any(value < 0 for value in report.values()) or report["peak_rss_kib"] < 1 or report["swap_events"]:
        raise ValueError("contention child resources invalid or swapped")
    return report


def validate_resource_claim(claim: dict, raw: dict) -> None:
    """Reject a producer's resource claim that differs from GNU Time bytes."""
    expected = {
        "scope": "measured-workload-child-process",
        "method": "gnu-time",
        "user_cpu_s": raw["user_s"],
        "system_cpu_s": raw["system_s"],
        "peak_rss_kib": raw["peak_rss_kib"],
        "fs_inputs": raw["fs_inputs"],
        "fs_outputs": raw["fs_outputs"],
        "major_faults": raw["major_faults"],
        "swap_events": raw["swap_events"],
        "unsupported": [],
    }
    if claim != expected or any(type(claim.get(key)) is not type(expected[key]) for key in expected):
        raise ValueError("contention resource claim changed")


def check_environment(environment: dict, resources: dict[str, dict]) -> list[str]:
    """Recompute host warnings and reject environment invalidators."""
    start, end = environment.get("start", {}), environment.get("end", {})
    if any(not start.get(key) or start[key] != end.get(key) for key in STABLE):
        raise ValueError("contention environment identity drift")
    if start.get("competing_jobs") or end.get("competing_jobs"):
        raise ValueError("contention competing jobs")
    if min(start.get("disk_free_bytes", 0), end.get("disk_free_bytes", 0)) < 1_073_741_824:
        raise ValueError("contention disk pressure")
    before, after = start.get("swap_pages"), end.get("swap_pages")
    if type(before) is not int or type(after) is not int or after < before:
        raise ValueError("contention host swap counter invalid")
    if environment.get("invalidators") != []:
        raise ValueError("contention environment invalidators present")
    warnings = []
    if after > before:
        warnings.append(f"host swap drift: {after - before} pages; child swap events: 0")
    for report in resources.values():
        if report["major_faults"]:
            warnings.append(f"child major faults: {report['major_faults']}")
    warnings = sorted(set(warnings))
    if environment.get("warnings") != warnings:
        raise ValueError("contention environment warnings changed")
    return warnings


def audit_block(
    directory: Path, *, role: str, protocol: dict,
    install_root: Path, main_archive: Path, platform_archive: Path,
    reference_manifest: Path,
) -> tuple[list[int], list[str], dict]:
    """Reopen every raw process and database without trusting block summaries."""
    summary = json.loads((directory / "summary.json").read_text())
    identity = json.loads((directory / "identity.json").read_text())
    environment = json.loads((directory / "environment.json").read_text())
    attempts = json.loads((directory / "attempts.json").read_text())
    for name in ("identity", "environment", "attempts"):
        if summary.get(f"{name}_sha256") != sha(directory / f"{name}.json"):
            raise ValueError(f"{directory.name} {name} hash changed")
    frozen = protocol["frozen_sha256"]
    if (
        summary.get("status") != "VALID_S02_TS_CONTENTION_BLOCK"
        or summary.get("role") != role
        or summary.get("samples") != protocol["samples_per_block"]
        or summary.get("warmups") != 1
        or summary.get("valid_samples") != protocol["samples_per_block"]
        or summary.get("invalidators") != []
        or identity.get("role") != role
        or identity.get("source_sha") != protocol[role]["source_sha"]
        or identity.get("prior_protocol_sha256") != sha(PRIOR)
        or identity.get("contention_protocol_sha256") != sha(FROZEN)
        or identity.get("main_archive_sha256") != sha(main_archive)
        or identity.get("platform_archive_sha256") != sha(platform_archive)
        or identity.get("reference_manifest_sha256") != sha(reference_manifest)
        or identity.get("runner_sha256") != frozen["runner"]
        or identity.get("s01_helper_sha256") != frozen["s01_helper"]
        or identity.get("s02_helper_sha256") != frozen["s02_helper"]
        or identity.get("auditor_sha256") != frozen["auditor"]
        or identity.get("block_runner_sha256") != frozen["block_runner"]
        or identity.get("node_version") != json.loads(PRIOR.read_text())["node_version"]
    ):
        raise ValueError(f"{directory.name} identity or validity invalid")
    bundle = directory / "bundle"
    for key, name in (("runner", RUNNER.name), ("s01_helper", S01_HELPER.name),
                      ("s02_helper", S02_HELPER.name)):
        if sha(bundle / name) != frozen[key]:
            raise ValueError(f"{directory.name} bundled {key} changed")
    labels = ["warmup", *(f"sample-{number:03d}"
                          for number in range(1, protocol["samples_per_block"] + 1))]
    if [row.get("label") for row in attempts] != labels or any(not row.get("valid") for row in attempts):
        raise ValueError(f"{directory.name} attempt count, order or validity invalid")
    resources = {}
    values = []
    peaks = []
    overlaps = []
    for number, (label, attempt) in enumerate(zip(labels, attempts)):
        stem = directory / label
        raw = Path(str(stem) + ".json")
        database = Path(str(stem) + ".sqlite")
        resource = Path(str(stem) + ".resource")
        audit_path = Path(str(stem) + ".audit.json")
        resources[label] = resource_report(resource)
        validate_resource_claim(attempt.get("resource", {}), resources[label])
        checked = run_audit.audit_run(
            raw_path=raw, database=database, runner=bundle / RUNNER.name,
            s01_helper=bundle / S01_HELPER.name,
            s02_helper=bundle / S02_HELPER.name,
            comparison_protocol=PRIOR, reference_manifest=reference_manifest,
            install_root=install_root, main_archive=main_archive,
            platform_archive=platform_archive,
        )
        if (
            attempt.get("exit_code") != 0
            or json.loads(audit_path.read_text()) != checked
            or attempt.get("elapsed_ns") != checked["elapsed_ns"]
            or attempt.get("raw_sha256") != sha(raw)
            or attempt.get("database_sha256") != sha(database)
            or attempt.get("resource_sha256") != sha(resource)
            or attempt.get("audit_sha256") != sha(audit_path)
            or (stem.with_suffix(".stdout")).read_text().splitlines() != ["S02_TS_CONTENTION_FUNCTIONAL_OK"]
            or (stem.with_suffix(".stderr")).read_text()
        ):
            raise ValueError(f"{directory.name}/{label} raw receipt changed")
        if number:
            values.append(checked["elapsed_ns"])
            peaks.append(resources[label]["peak_rss_kib"])
            overlaps.append(checked["actual_call_overlap_cycles"])
    warnings = check_environment(environment, resources)
    if (
        summary.get("warnings") != warnings
        or summary.get("p50_ns") != nearest_rank(values, .5)
        or summary.get("p95_ns") != nearest_rank(values, .95)
        or summary.get("min_ns") != min(values)
        or summary.get("max_ns") != max(values)
    ):
        raise ValueError(f"{directory.name} block statistics changed")
    return values, warnings, {
        "samples": len(values), "p50_ns": nearest_rank(values, .5),
        "p95_ns": nearest_rank(values, .95),
        "peak_rss_kib_p50": nearest_rank(peaks, .5),
        "actual_call_overlap_cycles_min": min(overlaps),
        "warnings": warnings, "summary_sha256": sha(directory / "summary.json"),
    }


def audit_campaign(
    directory: Path, *, baseline_install: Path, baseline_main: Path,
    baseline_platform: Path, baseline_reference: Path,
    candidate_install: Path, candidate_main: Path,
    candidate_platform: Path, candidate_reference: Path,
) -> dict:
    """Verify frozen order, every raw process and paired statistics."""
    protocol = json.loads(FROZEN.read_text())
    files = {
        "runner": RUNNER, "auditor": AUDITOR, "block_runner": BLOCK,
        "campaign_runner": CAMPAIGN, "s01_helper": S01_HELPER,
        "s02_helper": S02_HELPER, "prior_protocol": PRIOR,
        "baseline_main_archive": baseline_main,
        "baseline_platform_archive": baseline_platform,
        "baseline_reference_manifest": baseline_reference,
        "candidate_main_archive": candidate_main,
        "candidate_platform_archive": candidate_platform,
        "candidate_reference_manifest": candidate_reference,
    }
    if protocol.get("status") != "FROZEN_S02_TS_CONTENTION_PAIRED" or any(
        protocol["frozen_sha256"].get(key) != sha(path) for key, path in files.items()
    ):
        raise ValueError("contention frozen source or archive changed")
    order_path = directory / "run-order.jsonl"
    entries = [json.loads(line) for line in order_path.read_text().splitlines()]
    validate_order(entries, pairs=protocol["pairs"],
                   minimum_idle_seconds=protocol["minimum_idle_seconds"])
    campaign = json.loads((directory / "campaign-summary.json").read_text())
    if (
        campaign.get("status") != "VALID_S02_TS_CONTENTION_CAMPAIGN"
        or campaign.get("protocol_sha256") != sha(FROZEN)
        or campaign.get("run_order_sha256") != sha(order_path)
        or campaign.get("blocks") != len(entries)
        or campaign.get("pairs") != protocol["pairs"]
        or campaign.get("samples_per_block") != protocol["samples_per_block"]
    ):
        raise ValueError("contention campaign summary invalid")
    roles = {
        "baseline": (baseline_install, baseline_main, baseline_platform, baseline_reference),
        "candidate": (candidate_install, candidate_main, candidate_platform, candidate_reference),
    }
    by_block = {}
    warnings = {}
    reports = {}
    for entry in entries:
        name, role = entry["block"], entry["role"]
        block_dir = directory / name
        if entry.get("summary_sha256") != sha(block_dir / "summary.json"):
            raise ValueError(f"{name} summary hash changed")
        install, main, platform, reference = roles[role]
        values, block_warnings, report = audit_block(
            block_dir, role=role, protocol=protocol, install_root=install,
            main_archive=main, platform_archive=platform,
            reference_manifest=reference,
        )
        by_block[name] = values
        warnings[name] = block_warnings
        reports[name] = report
    result = summarize(by_block, warnings, pairs=protocol["pairs"],
                       samples_per_block=protocol["samples_per_block"])
    result.update({
        "schema_version": 1,
        "status": "VALID_S02_TS_CONTENTION_PAIRED_AUDIT",
        "protocol_sha256": sha(FROZEN),
        "campaign_summary_sha256": sha(directory / "campaign-summary.json"),
        "run_order_sha256": sha(order_path),
        "source_sha": {role: protocol[role]["source_sha"] for role in roles},
        "samples_per_version": protocol["pairs"] * protocol["samples_per_block"],
        "blocks": reports,
    })
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--campaign", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    for role in ("baseline", "candidate"):
        for name in ("install", "main", "platform", "reference"):
            parser.add_argument(f"--{role}-{name}", required=True, type=Path)
    args = parser.parse_args()
    result = audit_campaign(
        args.campaign,
        baseline_install=args.baseline_install, baseline_main=args.baseline_main,
        baseline_platform=args.baseline_platform,
        baseline_reference=args.baseline_reference,
        candidate_install=args.candidate_install, candidate_main=args.candidate_main,
        candidate_platform=args.candidate_platform,
        candidate_reference=args.candidate_reference,
    )
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(result["status"])


if __name__ == "__main__":
    main()
