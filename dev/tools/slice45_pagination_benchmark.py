#!/usr/bin/env python3
"""Run and analyze the preregistered Slice 45 pagination overhead campaign."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import random
import statistics
import subprocess
from pathlib import Path
from typing import Any

TEST_NAME = "measure_slice45_pagination_overhead"
RSS_ARMS = (
    "exact_page",
    "frozen_page",
    "mint_plus_page",
    "continuation_page",
    "current_state",
    "frozen_state",
)
LATENCY_PAIRS = (
    ("exact_page", "frozen_page"),
    ("preminted_page", "mint_plus_page"),
    ("first_page", "continuation_page"),
    ("current_state", "frozen_state"),
)
COLD_PAIRS = LATENCY_PAIRS
COLD_ARMS = tuple(dict.fromkeys(arm for pair in COLD_PAIRS for arm in pair))
RSS_PAIRS = (
    ("exact_page", "frozen_page"),
    ("frozen_page", "mint_plus_page"),
    ("frozen_page", "continuation_page"),
    ("current_state", "frozen_state"),
)
BOOTSTRAP_SEED = 450_825
BOOTSTRAP_DRAWS = 10_000


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def command_output(*command: str) -> str:
    return subprocess.run(command, check=True, text=True, capture_output=True).stdout.strip()


def invoke(
    binary: Path,
    database: Path,
    rows: int,
    samples: int,
    mode: str,
    treatment_first: bool = False,
    seed_only: bool = False,
    cold_arm: str | None = None,
    frozen_fixture: dict[str, Any] | None = None,
) -> dict[str, Any]:
    environment = os.environ.copy()
    environment.update(
        {
            "FATHOM_SLICE45_DATABASE": str(database),
            "FATHOM_SLICE45_ROWS": str(rows),
            "FATHOM_SLICE45_SAMPLES": str(samples),
            "FATHOM_SLICE45_MODE": mode,
        }
    )
    if treatment_first:
        environment["FATHOM_SLICE45_TREATMENT_FIRST"] = "1"
    if seed_only:
        environment["FATHOM_SLICE45_SEED_ONLY"] = "1"
    if cold_arm is not None:
        environment["FATHOM_SLICE45_COLD_ARM"] = cold_arm
    if frozen_fixture is not None:
        environment.update(
            {
                "FATHOM_SLICE45_FROZEN_EFFECTIVE": str(frozen_fixture["effective_valid_at"]),
                "FATHOM_SLICE45_FROZEN_TOKEN": frozen_fixture["token"],
                "FATHOM_SLICE45_FROZEN_CONTINUATION": frozen_fixture["continuation"],
            }
        )
    completed = subprocess.run(
        [
            "taskset",
            "-c",
            "0",
            str(binary),
            "--ignored",
            "--exact",
            TEST_NAME,
            "--nocapture",
        ],
        check=True,
        text=True,
        capture_output=True,
        env=environment,
    )
    payloads = []
    for line in completed.stdout.splitlines():
        start = line.find("{")
        end = line.rfind("}")
        if start >= 0 and end > start:
            payloads.append(json.loads(line[start : end + 1]))
    if len(payloads) != 1:
        raise RuntimeError(f"expected one JSON payload, got {len(payloads)}: {completed.stdout}")
    return payloads[0]


def percentile(values: list[float], quantile: float) -> float:
    ordered = sorted(values)
    index = round((len(ordered) - 1) * quantile)
    return ordered[index]


def bootstrap_median_ci(deltas: list[float]) -> tuple[float, float]:
    generator = random.Random(BOOTSTRAP_SEED)
    draws = []
    for _ in range(BOOTSTRAP_DRAWS):
        sample = [generator.choice(deltas) for _ in deltas]
        draws.append(statistics.median(sample))
    return percentile(draws, 0.025), percentile(draws, 0.975)


def analyze_latency(records: list[dict[str, Any]]) -> list[dict[str, Any]]:
    outcomes = []
    for baseline, treatment in LATENCY_PAIRS:
        baseline_values = [record["result"][baseline][1] for record in records]
        treatment_values = [record["result"][treatment][1] for record in records]
        deltas = [right - left for left, right in zip(baseline_values, treatment_values)]
        baseline_median = statistics.median(baseline_values)
        treatment_median = statistics.median(treatment_values)
        baseline_throughput = statistics.median(
            record["result"][baseline][2] for record in records
        )
        treatment_throughput = statistics.median(
            record["result"][treatment][2] for record in records
        )
        delta = statistics.median(deltas)
        percent = delta / baseline_median * 100.0
        low, high = bootstrap_median_ci(deltas)
        outcomes.append(
            {
                "baseline": baseline,
                "treatment": treatment,
                "baseline_median_p95_ms": baseline_median,
                "treatment_median_p95_ms": treatment_median,
                "baseline_median_throughput_ops_s": baseline_throughput,
                "treatment_median_throughput_ops_s": treatment_throughput,
                "throughput_delta_percent": (
                    (treatment_throughput - baseline_throughput)
                    / baseline_throughput
                    * 100.0
                ),
                "median_paired_delta_ms": delta,
                "median_paired_delta_percent": percent,
                "paired_bootstrap_95_ci_ms": [low, high],
                "material": delta > 0.25 and percent > 10.0,
            }
        )
    return outcomes


def analyze_rss(records: list[dict[str, Any]]) -> list[dict[str, Any]]:
    by_arm: dict[str, list[float]] = {arm: [] for arm in RSS_ARMS}
    for record in records:
        by_arm[record["result"]["arm"]].append(record["peak_rss_kib"] / 1024.0)
    outcomes = []
    for baseline, treatment in RSS_PAIRS:
        baseline_median = statistics.median(by_arm[baseline])
        treatment_median = statistics.median(by_arm[treatment])
        delta = treatment_median - baseline_median
        percent = delta / baseline_median * 100.0
        outcomes.append(
            {
                "baseline": baseline,
                "treatment": treatment,
                "baseline_median_peak_rss_mib": baseline_median,
                "treatment_median_peak_rss_mib": treatment_median,
                "delta_mib": delta,
                "delta_percent": percent,
                "material": delta > 8.0 and percent > 5.0,
            }
        )
    return outcomes


def analyze_cold(records: list[dict[str, Any]]) -> list[dict[str, Any]]:
    outcomes = []
    by_arm = {
        arm: sorted(
            (record for record in records if record["arm"] == arm),
            key=lambda record: record["repetition"],
        )
        for arm in COLD_ARMS
    }
    for baseline, treatment in COLD_PAIRS:
        baseline_records = by_arm[baseline]
        treatment_records = by_arm[treatment]
        baseline_values = [record["operation_ms"] for record in baseline_records]
        treatment_values = [record["operation_ms"] for record in treatment_records]
        deltas = [right - left for left, right in zip(baseline_values, treatment_values)]
        baseline_median = statistics.median(baseline_values)
        treatment_median = statistics.median(treatment_values)
        delta = statistics.median(deltas)
        percent = delta / baseline_median * 100.0
        outcomes.append(
            {
                "baseline": baseline,
                "treatment": treatment,
                "baseline_median_ms": baseline_median,
                "treatment_median_ms": treatment_median,
                "median_paired_delta_ms": delta,
                "median_paired_delta_percent": percent,
                "baseline_median_open_ms": statistics.median(
                    record["open_ms"] for record in baseline_records
                ),
                "treatment_median_open_ms": statistics.median(
                    record["open_ms"] for record in treatment_records
                ),
                "material": delta > 0.25 and percent > 10.0,
            }
        )
    return outcomes


def write_markdown(result: dict[str, Any], path: Path) -> None:
    lines = [
        "# Slice 45 pagination performance result",
        "",
        "Material means both >10% and >0.25 ms median paired p95 latency, or both",
        ">10% and >0.25 ms median paired cold-operation latency, or both >5% and",
        ">8 MiB median peak RSS.",
        "",
        "| Scale | Comparison | Baseline p95 ms | Treatment p95 ms | Delta ms | Delta % | Material |",
        "|---:|---|---:|---:|---:|---:|---|",
    ]
    for scale in result["scales"]:
        for item in scale["latency"]:
            lines.append(
                f"| {scale['rows']} | {item['baseline']} → {item['treatment']} | "
                f"{item['baseline_median_p95_ms']:.4f} | {item['treatment_median_p95_ms']:.4f} | "
                f"{item['median_paired_delta_ms']:.4f} | {item['median_paired_delta_percent']:.2f} | "
                f"{str(item['material']).lower()} |"
            )
    lines.extend(
        [
            "",
            "| Scale | Comparison | Baseline ops/s | Treatment ops/s | Delta % |",
            "|---:|---|---:|---:|---:|",
        ]
    )
    for scale in result["scales"]:
        for item in scale["latency"]:
            lines.append(
                f"| {scale['rows']} | {item['baseline']} → {item['treatment']} | "
                f"{item['baseline_median_throughput_ops_s']:.1f} | "
                f"{item['treatment_median_throughput_ops_s']:.1f} | "
                f"{item['throughput_delta_percent']:.2f} |"
            )
    lines.extend(
        [
            "",
            "| Scale | RSS comparison | Baseline MiB | Treatment MiB | Delta MiB | Delta % | Material |",
            "|---:|---|---:|---:|---:|---:|---|",
        ]
    )
    for scale in result["scales"]:
        for item in scale["rss"]:
            lines.append(
                f"| {scale['rows']} | {item['baseline']} → {item['treatment']} | "
                f"{item['baseline_median_peak_rss_mib']:.2f} | "
                f"{item['treatment_median_peak_rss_mib']:.2f} | {item['delta_mib']:.2f} | "
                f"{item['delta_percent']:.2f} | {str(item['material']).lower()} |"
            )
    lines.extend(
        [
            "",
            "| Scale | Cold comparison | Baseline ms | Treatment ms | Delta ms | Delta % | Baseline open ms | Treatment open ms | Material |",
            "|---:|---|---:|---:|---:|---:|---:|---:|---|",
        ]
    )
    for scale in result["scales"]:
        for item in scale["cold"]:
            lines.append(
                f"| {scale['rows']} | {item['baseline']} → {item['treatment']} | "
                f"{item['baseline_median_ms']:.4f} | {item['treatment_median_ms']:.4f} | "
                f"{item['median_paired_delta_ms']:.4f} | "
                f"{item['median_paired_delta_percent']:.2f} | "
                f"{item['baseline_median_open_ms']:.4f} | "
                f"{item['treatment_median_open_ms']:.4f} | "
                f"{str(item['material']).lower()} |"
            )
    lines.extend(
        [
            "",
            "| Scale | Mint context p95 ms | Mint snapshot p95 ms | Mint binding p95 ms | Mint codec p95 ms | Page token auth p95 ms | Page binding p95 ms | Cursor auth p95 ms |",
            "|---:|---:|---:|---:|---:|---:|---:|---:|",
        ]
    )
    for scale in result["scales"]:
        stages = scale["stage_medians"]
        lines.append(
            f"| {scale['rows']} | {stages['mint_context_validation_p95_ms']:.4f} | "
            f"{stages['mint_snapshot_validation_p95_ms']:.4f} | "
            f"{stages['mint_binding_p95_ms']:.4f} | "
            f"{stages['mint_token_codec_p95_ms']:.4f} | "
            f"{stages['token_authentication_p95_ms']:.4f} | "
            f"{stages['snapshot_binding_p95_ms']:.4f} | "
            f"{stages['cursor_authentication_p95_ms']:.4f} |"
        )
    lines.extend(
        [
            "",
            "| Scale | Public list p95 ms | Public list ops/s | Full walk ms | Pages | Items | Walk items/s |",
            "|---:|---:|---:|---:|---:|---:|---:|",
        ]
    )
    for scale in result["scales"]:
        public_list = scale["public_list_medians"]
        full_walk = scale["full_walk_medians"]
        lines.append(
            f"| {scale['rows']} | {public_list['p95_ms']:.4f} | "
            f"{public_list['throughput_ops_s']:.1f} | {full_walk['elapsed_ms']:.2f} | "
            f"{full_walk['pages']:.0f} | {full_walk['items']:.0f} | "
            f"{full_walk['items_per_second']:.1f} |"
        )
    lines.extend(["", f"Overall material: **{str(result['material']).lower()}**", ""])
    path.write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--database-dir", type=Path, required=True)
    parser.add_argument("--samples", type=int, default=1_000)
    parser.add_argument("--latency-repetitions", type=int, default=10)
    parser.add_argument("--cold-repetitions", type=int, default=3)
    parser.add_argument("--rss-repetitions", type=int, default=5)
    parser.add_argument("--scales", type=int, nargs="+", default=[10_000, 50_000])
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    args.database_dir.mkdir(parents=True, exist_ok=True)
    raw_path = args.output_dir / "raw.ndjson"
    raw_path.write_text("", encoding="utf-8")
    all_records: dict[int, dict[str, list[dict[str, Any]]]] = {}

    with raw_path.open("a", encoding="utf-8") as raw:
        for rows in args.scales:
            database = args.database_dir / f"slice45-{rows}.fathom.sqlite3"
            seed = invoke(args.binary, database, rows, args.samples, "latency", seed_only=True)
            raw.write(json.dumps({"phase": "seed", **seed}, sort_keys=True) + "\n")
            groups = {"latency": [], "cold": [], "rss": []}
            fixture = invoke(args.binary, database, rows, 1, "mint_fixture")
            for repetition in range(args.latency_repetitions):
                record = invoke(
                    args.binary,
                    database,
                    rows,
                    args.samples,
                    "latency",
                    treatment_first=repetition % 2 == 1,
                )
                record.update({"phase": "steady", "repetition": repetition})
                groups["latency"].append(record)
                raw.write(json.dumps(record, sort_keys=True) + "\n")
                raw.flush()
            for repetition in range(args.cold_repetitions):
                for baseline, treatment in COLD_PAIRS:
                    arms = (treatment, baseline) if repetition % 2 == 1 else (baseline, treatment)
                    for arm in arms:
                        record = invoke(
                            args.binary,
                            database,
                            rows,
                            1,
                            "cold",
                            cold_arm=arm,
                            frozen_fixture=fixture,
                        )
                        record.update({"phase": "cold", "repetition": repetition})
                        groups["cold"].append(record)
                        raw.write(json.dumps(record, sort_keys=True) + "\n")
                        raw.flush()
            for arm in RSS_ARMS:
                for repetition in range(args.rss_repetitions):
                    record = invoke(args.binary, database, rows, args.samples, arm)
                    record.update({"phase": "rss", "repetition": repetition})
                    groups["rss"].append(record)
                    raw.write(json.dumps(record, sort_keys=True) + "\n")
                    raw.flush()
            all_records[rows] = groups

    scales = []
    for rows, groups in all_records.items():
        first_stage_keys = groups["latency"][0]["result"]["first_page_stages"]
        mint_stage_keys = groups["latency"][0]["result"]["mint_stages"]
        scales.append(
            {
                "rows": rows,
                "latency": analyze_latency(groups["latency"]),
                "rss": analyze_rss(groups["rss"]),
                "cold": analyze_cold(groups["cold"]),
                "stage_medians": {
                    **{
                        key: statistics.median(
                            record["result"][
                                "continuation_stages"
                                if key.startswith("cursor_authentication_")
                                else "first_page_stages"
                            ][key]
                            for record in groups["latency"]
                        )
                        for key in first_stage_keys
                    },
                    **{
                        f"mint_{key}": statistics.median(
                            record["result"]["mint_stages"][key]
                            for record in groups["latency"]
                        )
                        for key in mint_stage_keys
                    },
                },
                "public_list_medians": {
                    "p95_ms": statistics.median(
                        record["result"]["public_list"][1]
                        for record in groups["latency"]
                    ),
                    "throughput_ops_s": statistics.median(
                        record["result"]["public_list"][2]
                        for record in groups["latency"]
                    ),
                },
                "full_walk_medians": {
                    key: statistics.median(
                        record["result"][
                            "full_walk"
                        ][key]
                        for record in groups["latency"]
                    )
                    for key in ("elapsed_ms", "pages", "items", "items_per_second")
                },
            }
        )
    material = any(
        item["material"]
        for scale in scales
        for family in ("latency", "cold", "rss")
        for item in scale[family]
    )
    result = {
        "schema_version": "slice45-pagination-campaign.v1",
        "bootstrap_seed": BOOTSTRAP_SEED,
        "bootstrap_draws": BOOTSTRAP_DRAWS,
        "samples_per_process": args.samples,
        "latency_repetitions": args.latency_repetitions,
        "cold_repetitions": args.cold_repetitions,
        "rss_repetitions_per_arm": args.rss_repetitions,
        "scales": scales,
        "material": material,
    }
    result_path = args.output_dir / "result.json"
    result_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(result, args.output_dir / "result.md")
    try:
        output_relative = args.output_dir.resolve().relative_to(Path.cwd().resolve())
    except ValueError:
        output_relative = None
    diff_command = ["git", "diff", "--binary", "--", "."]
    if output_relative is not None:
        diff_command.append(f":(exclude){output_relative}/**")
    manifest = {
        "schema_version": "slice45-pagination-manifest.v1",
        "binary": str(args.binary.resolve()),
        "binary_sha256": sha256(args.binary),
        "runner_sha256": sha256(Path(__file__)),
        "raw_sha256": sha256(raw_path),
        "result_sha256": sha256(result_path),
        "git_head": command_output("git", "rev-parse", "HEAD"),
        "git_diff_sha256": hashlib.sha256(
            command_output(*diff_command).encode("utf-8")
        ).hexdigest(),
        "rustc": command_output("rustc", "--version"),
        "kernel": command_output("uname", "-srmo"),
        "cpu_affinity": "0",
        "databases": {
            str(rows): {
                "path": str((args.database_dir / f"slice45-{rows}.fathom.sqlite3").resolve()),
                "sha256": sha256(args.database_dir / f"slice45-{rows}.fathom.sqlite3"),
            }
            for rows in args.scales
        },
    }
    (args.output_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(json.dumps({"material": material, "result": str(result_path)}, sort_keys=True))


if __name__ == "__main__":
    main()
