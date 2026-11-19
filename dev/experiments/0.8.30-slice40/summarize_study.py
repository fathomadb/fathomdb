"""Summarize successful latency, total useful rate and observed request stages."""

import argparse
from collections import Counter, defaultdict
import json
import math
from pathlib import Path
from statistics import median

from run_study import load, stage_records

REFUSALS = {"OverloadedError", "ClosingError"}
STATIONARY = (
    "ids",
    "bodies",
    "hits",
    "snippets",
    "explanation",
    "projection_cursor",
    "soft_fallback",
    "degradations",
)


def quantiles(values):
    ordered = sorted(values)
    if not ordered:
        return None
    return {
        f"p{p}": ordered[math.ceil(p * len(ordered) / 100) - 1] for p in (50, 95, 99)
    }


def error_class(row):
    kind = row.get("error_kind")
    error = row.get("error", "")
    if kind in REFUSALS:
        return "refusals"
    if kind == "EmbedderError" and error.endswith("(Timeout))"):
        return "timeouts"
    return "errors"


def metrics(rows, planned, wall_ns=None):
    success = [row for row in rows if row["ok"] is True]
    errors = [row for row in rows if row["ok"] is False]
    outcomes = Counter(error_class(row) for row in errors)
    if len(success) + len(errors) != len(rows) or len(rows) > planned:
        raise ValueError("invalid request outcomes or attempt count")
    return {
        "planned": planned,
        "returned": len(rows),
        "successful": len(success),
        "degraded_successful": sum(
            row.get("degradations", "[]") != "[]"
            or row.get("soft_fallback", "None") != "None"
            for row in success
        ),
        "refusals": outcomes["refusals"],
        "timeouts": outcomes["timeouts"],
        "errors": outcomes["errors"],
        "missing": planned - len(rows),
        "error_kinds": dict(Counter(row.get("error_kind") for row in errors)),
        "latency_ms": quantiles([row["elapsed_ns"] / 1e6 for row in success]),
        "wall_s": wall_ns / 1e9 if wall_ns is not None else None,
        "useful_qps": len(success) * 1e9 / wall_ns if wall_ns else None,
        "returned_utf8_body_bytes": sum(
            len(body.encode()) for row in success for body in row.get("bodies", [])
        ),
    }


def population(result, key, routes, expected, *, partial=False):
    rows = result[key]
    grouped = []
    for route in routes:
        selected = [row for row in rows if row["route"] == route]
        indices = [row["sample_index"] for row in selected]
        query_indices = [row.get("query_index") for row in selected]
        if (
            indices != list(range(len(selected)))
            or query_indices != indices
            or len(selected) > expected
            or (not partial and len(selected) != expected)
        ):
            raise ValueError(
                f"{route} {key}: wrong count or order ({indices}, {query_indices})"
            )
        grouped.append(selected)
    return grouped


def stage_summary(result, rows):
    times = defaultdict(list)
    guards = Counter()
    routes = Counter()
    for row in rows:
        for item in stage_records(result, row):
            phase, facts = item["phase"], item["facts"]
            if item["status"] == "not_run":
                guards[(phase, facts.get("reason", ""))] += 1
            if phase == "slice50_work" and facts.get("route"):
                routes[facts["route"]] += 1
            if item["status"] == "ok" and item["started_ns"] is not None:
                times[phase].append((item["ended_ns"] - item["started_ns"]) / 1e6)
    return {
        "scheduled_rows": len(rows),
        "phases": {
            phase: {"count": len(values), "ms": quantiles(values)}
            for phase, values in times.items()
        },
        "not_run": [
            {"phase": phase, "reason": reason, "count": n}
            for (phase, reason), n in sorted(guards.items())
        ],
        "routes": dict(routes),
    }


def observed_snapshot(result, row):
    captures = [
        record["facts"]
        for record in stage_records(result, row)
        if record["phase"] == "observer_snapshot_capture" and record["status"] == "ok"
    ]
    if (
        len(captures) != 1
        or captures[0]["projection_cursor"] != row["projection_cursor"]
    ):
        raise ValueError("expected one matching reader snapshot")
    return captures[0]


def fidelity(result, fixed_at=None):
    outputs = []
    for row in result["samples"]:
        if not row["ok"]:
            outputs.append((row["route"], row["query_index"], row["error_kind"]))
            continue
        capture = observed_snapshot(result, row)
        if fixed_at is not None and capture["validity_binding_epoch_s"] != fixed_at:
            raise ValueError("unexpected fixed validity authority")
        # The qualified dense store has no authored validity windows. Dynamic
        # read timestamps can differ across process opens on this unchanged data.
        authority = {
            key: capture[key]
            for key in (
                "visibility_generation",
                "visibility_nonce",
                "dependency_generation",
                "projection_cursor",
                "read_view",
            )
        }
        outputs.append(
            (
                row["route"],
                row["query_index"],
                {key: row[key] for key in STATIONARY},
                authority,
            )
        )
    return outputs


def natural_proof(
    result, original_cursors, reopened, *, before=None, initial_high=100_000
):
    latest = dict(original_cursors)
    receipts = {batch["before_sample"]: batch for batch in result["batches"]}
    probes = {probe["before_sample"]: probe for probe in result["probes"]}
    previous_seen = None
    high = initial_high
    for row in result["samples"]:
        index = row["sample_index"]
        if index in receipts:
            batch, probe = receipts[index], probes[index]
            current = probe["point_probe"]["write_cursor"]
            if (
                not probe["marker_current"]
                or current <= high
                or current != batch["after_write_cursor"]
            ):
                raise ValueError(f"invalid current marker for batch before {index}")
            marker = probe["text_probe"]
            if marker["hits"][0]["write_cursor"] != current:
                raise ValueError(
                    "text marker did not bind to the current committed cursor"
                )
            observed_snapshot(result, marker)
            latest[f"l:{probe['logical_id']}"] = current
            high = current
        for hit in row["hits"]:
            cursor = hit["write_cursor"]
            if cursor > high or (hit["id"] in latest and cursor != latest[hit["id"]]):
                raise ValueError(f"ineligible cursor in natural read {index}")
        generation = observed_snapshot(result, row)["visibility_generation"]
        if previous_seen is not None and generation < previous_seen:
            raise ValueError("natural read visibility regressed")
        previous_seen = generation
    if reopened is None:
        raise ValueError("written store lacked a valid SDK reopen")
    for i, probe in enumerate(result["probes"]):
        marker = next(
            row
            for row in reopened["samples"]
            if row["route"] == "text" and row["query_index"] == i
        )
        if [(hit["id"], hit["write_cursor"]) for hit in marker["hits"]] != [
            (f"l:{probe['logical_id']}", probe["point_probe"]["write_cursor"])
        ]:
            raise ValueError(
                "reopened marker does not name the current committed cursor"
            )
        if marker["bodies"] != probe["text_probe"]["bodies"]:
            raise ValueError("reopened marker body changed")
    for row in reopened["samples"]:
        snapshot = observed_snapshot(reopened, row)
        if (
            row["projection_cursor"] < high
            or snapshot["visibility_generation"] < previous_seen
        ):
            raise ValueError("reopen lacked current reader snapshot authority")
        if any(
            hit["write_cursor"] > high
            or (hit["id"] in latest and hit["write_cursor"] != latest[hit["id"]])
            for hit in row["hits"]
        ):
            raise ValueError("reopen exposed an ineligible cursor")
    original_term_checks = 0
    if before is not None:
        for index, probe in enumerate(result["probes"]):
            reference = f"l:{probe['logical_id']}"
            position = len(result["probes"]) + index
            for route in ("hybrid", "text"):
                original = next(
                    row
                    for row in before["samples"]
                    if row["route"] == route and row["query_index"] == index
                )
                after = next(
                    row
                    for row in reopened["samples"]
                    if row["route"] == route and row["query_index"] == position
                )
                if (
                    observed_snapshot(before, original)["projection_cursor"]
                    != initial_high
                ):
                    raise ValueError(
                        "pre-write original-body query did not use baseline authority"
                    )
                originally_seen = [
                    hit["write_cursor"]
                    for hit in original["hits"]
                    if hit["id"] == reference
                ]
                now_seen = [
                    hit["write_cursor"]
                    for hit in after["hits"]
                    if hit["id"] == reference
                ]
                if originally_seen != [original_cursors[reference]] or now_seen != [
                    latest[reference]
                ]:
                    raise ValueError(
                        f"original body query did not discriminate superseded {reference} via {route}"
                    )
                original_term_checks += 1
    if high <= initial_high or not reopened["fixtures"][0]["qualified_dense_fixture"]:
        raise ValueError(
            "reopened subject did not reach the ready dense state after writes"
        )
    if not any(row["route"] == "hybrid" and row["ok"] for row in reopened["samples"]):
        raise ValueError("reopened subject did not complete hybrid search")
    return {
        "governed_batches": len(receipts),
        "joined_reads": len(result["samples"]),
        "last_cursor": high,
        "reopened_snapshot_joins": len(reopened["samples"]),
        "discriminating_original_body_queries": original_term_checks,
        "reopen_ready": reopened["fixtures"][0]["qualified_dense_fixture"],
    }


def pair_summary(baseline, candidate):
    def pairs(key):
        return [
            right[key] / left[key]
            for left, right in zip(baseline, candidate, strict=True)
        ]

    pairs_p50 = [
        (left["latency_ms"]["p50"], right["latency_ms"]["p50"])
        for left, right in zip(baseline, candidate, strict=True)
    ]
    shifts = [(left - right) / left for left, right in pairs_p50]
    return {
        "pairs": len(shifts),
        "relative_p50_improvements": shifts,
        "median_relative_p50_improvement": median(shifts),
        "median_absolute_p50_improvement_ms": median(
            [left - right for left, right in pairs_p50]
        ),
        "paired_observed_bounds": [min(shifts), max(shifts)],
        "median_useful_qps_ratio": median(pairs("useful_qps")),
        "median_peak_rss_ratio": median(pairs("peak_rss_kib")),
        "median_cpu_per_success_ratio": median(pairs("cpu_seconds_per_success")),
    }


def read_jsonl(path, interrupted_outputs):
    if not path.exists():
        return []
    lines = path.read_bytes().splitlines(keepends=True)
    rows = []
    for i, line in enumerate(lines):
        try:
            rows.append(json.loads(line))
        except (json.JSONDecodeError, UnicodeDecodeError):
            if i != len(lines) - 1 or line.endswith(b"\n"):
                raise
            interrupted_outputs.append(f"{path.name}: incomplete final row")
    return rows


def summarize(root):
    populations = ("samples", "warmups", "first_calls")
    grouped = defaultdict(lambda: defaultdict(list))
    walls = defaultdict(list)
    cells = {}
    for directory in sorted(path for path in root.iterdir() if path.is_dir()):
        config = load(directory / "config.json")
        attempt = load(directory / "attempt.json")
        result_path = directory / "native/result.json"
        interrupted_outputs = []
        try:
            result = load(result_path) if result_path.exists() else {}
        except (json.JSONDecodeError, UnicodeDecodeError):
            interrupted_outputs.append("result.json: incomplete native summary")
            result = {}
        complete = attempt["exit"] == 0 and result.get("complete") is True
        actual = result or {
            key: read_jsonl(
                directory / f"native/{key.replace('_', '-')}.jsonl", interrupted_outputs
            )
            for key in populations
        }
        planned = {
            "samples": config["samples"],
            "warmups": config["warmup"],
            "first_calls": int(
                config.get("record_first_call", True) and config["family"] != "G-W"
            ),
        }
        groups = [
            population(
                actual, key, config["routes"], planned[key], partial=not complete
            )
            for key in populations
        ]
        pass_by_route = {
            item["route"]: item["wall_ns"] for item in result.get("passes", [])
        }
        resource_path = directory / "resources.tsv"
        resources = (
            resource_path.read_text().splitlines() if resource_path.exists() else []
        )
        if resources and len(resources[-1].split()) == 6:
            cpu_u, cpu_s, rss, blocks_in, blocks_out, elapsed = map(
                float, resources[-1].split()
            )
            resource_data = {
                "cpu_seconds": [cpu_u, cpu_s],
                "peak_rss_kib": int(rss),
                "fs_input_operations": int(blocks_in),
                "fs_output_operations": int(blocks_out),
                "process_elapsed_s": elapsed,
            }
        else:
            resource_data = {}
        rows_summary = {}
        for i, route in enumerate(config["routes"]):
            rows, warm, first = (group[i] for group in groups)
            wall = pass_by_route.get(route)
            data = metrics(rows, planned["samples"], wall)
            if len(config["routes"]) == 1 and resource_data and data["successful"]:
                data.update(
                    cpu_seconds_per_success=(cpu_u + cpu_s) / data["successful"],
                    peak_rss_kib=int(rss),
                )
            data["warmup"] = metrics(warm, planned["warmups"])
            data["first_call"] = metrics(first, planned["first_calls"])
            if config["observe"] and complete:
                data["diagnostic_stages"] = stage_summary(result, rows)
            if directory.name.startswith("read-"):
                for key, rows_in_population in zip(
                    populations, (rows, warm, first), strict=True
                ):
                    grouped[route][key].extend(rows_in_population)
                walls[route].append(wall)
            rows_summary[route] = data
        cells[directory.name] = {
            "complete": complete,
            "routes": rows_summary,
            "attempt": attempt,
            "interrupted_outputs": interrupted_outputs,
            **resource_data,
            "raw_persistence": result.get("raw_persistence"),
        }
        if config["family"] == "G-W":
            batches = (
                result["batches"]
                if result
                else read_jsonl(directory / "native/batches.jsonl", interrupted_outputs)
            )
            probes = (
                result["probes"]
                if result
                else read_jsonl(
                    directory / "native/g-marker-probes.jsonl", interrupted_outputs
                )
            )
            cells[directory.name]["governed"] = {
                "planned_batches": math.ceil(
                    config["samples"] / config["searches_per_batch"]
                ),
                "recorded_batches": len(batches),
                "recorded_marker_probes": len(probes),
                "current_marker_probes": sum(
                    probe["marker_current"] for probe in probes
                ),
            }
    routes = {}
    for route, retained in grouped.items():
        wall = (
            sum(walls[route])
            if len(walls[route]) == 4 and None not in walls[route]
            else None
        )
        routes[route] = {
            **metrics(retained["samples"], 480, wall),
            "warmup": metrics(retained["warmups"], 80),
            "first_call": metrics(retained["first_calls"], 4),
        }
    return {"cells": cells, "ordinary_baseline_routes": routes}


def compare(root, report):
    def read(name):
        return load(root / name / "native/result.json")

    failures = load(root / "failures.json")
    if failures:
        return {"promotion_guardrails_pass": False, "incomplete_cells": failures}
    evidence = {}
    verdicts = []
    for schedule, block_count in (("G-R", 8), ("G-W", 4)):
        kept = {"A": [], "B": []}
        samples = {"A": [], "B": []}
        first = {"A": [], "B": []}
        for i in range(block_count):
            for arm in ("A", "B"):
                name = f"pair-{schedule}-{i}-{arm}"
                kept[arm].append(report["cells"][name]["routes"]["hybrid"])
                result = read(name)
                samples[arm].extend(result["samples"])
                first[arm].extend(result["first_calls"])
        paired = pair_summary(kept["A"], kept["B"])
        totals = {
            arm: metrics(
                samples[arm],
                len(samples[arm]),
                round(sum(cell["wall_s"] for cell in kept[arm]) * 1e9),
            )
            for arm in ("A", "B")
        }
        tails = {
            tail: totals["B"]["latency_ms"][tail] / totals["A"]["latency_ms"][tail]
            for tail in ("p95", "p99")
        }
        guards = {
            "tails": max(tails.values()) <= 1.05,
            "useful_throughput": paired["median_useful_qps_ratio"] >= 0.97,
            "cpu": paired["median_cpu_per_success_ratio"] <= 1.05,
            "peak_rss": paired["median_peak_rss_ratio"] <= 1.10,
            "no_missing_or_failed_returns": all(
                cell["planned"] == cell["successful"] for cell in totals.values()
            ),
            "no_degraded_successes": all(
                cell["degraded_successful"] == 0 for cell in totals.values()
            ),
        }
        if schedule == "G-R":
            a_first = [row["elapsed_ns"] for row in first["A"]]
            b_first = [row["elapsed_ns"] for row in first["B"]]
            paired["median_first_call_ratio"] = median(
                b / a for a, b in zip(a_first, b_first, strict=True)
            )
            guards["first_call_review"] = paired["median_first_call_ratio"] <= 1.15
            guards["primary_benefit"] = (
                paired["median_relative_p50_improvement"] >= 0.10
                and paired["median_absolute_p50_improvement_ms"] >= 5
                and paired["paired_observed_bounds"][0] > 0
            )
            guards["stationary_utf8_body_bytes"] = (
                totals["A"]["returned_utf8_body_bytes"]
                == totals["B"]["returned_utf8_body_bytes"]
            )
        else:
            guards["natural_write_p50"] = (
                paired["median_relative_p50_improvement"] >= -0.05
            )
        verdicts.extend(guards.values())
        evidence[schedule] = {
            "paired": paired,
            "totals": totals,
            "tail_ratios": tails,
            "sampled_wal_bytes_per_arm": {
                arm: [
                    report["cells"][f"pair-{schedule}-{i}-{arm}"]["attempt"][
                        "wal_peak_sampled_bytes"
                    ]
                    for i in range(block_count)
                ]
                for arm in ("A", "B")
            },
            "guardrails": guards,
            "first_calls": {
                arm: metrics(first[arm], len(first[arm])) for arm in ("A", "B")
            },
        }
    same_state = fidelity(read("fidelity-A"), 1800000000) == fidelity(
        read("fidelity-B"), 1800000000
    )
    attributed = {
        arm: report["cells"][f"fidelity-{arm}"]["routes"]["hybrid"]["diagnostic_stages"]
        for arm in ("A", "B")
    }
    removed_work = (
        attributed["A"]["phases"]["all_nodes_directly_eligible"]["count"] == 12
        and "all_nodes_directly_eligible" not in attributed["B"]["phases"]
        and all(
            attributed[arm]["routes"].get("winner_hydration") == 12
            for arm in ("A", "B")
        )
    )
    natural = {
        arm: natural_proof(
            read(f"natural-proof-{arm}"),
            {"l:s30doc-0": 1, "l:s30doc-1": 2},
            read(f"natural-proof-{arm}-reopen"),
            before=read(f"natural-proof-{arm}-before")
            if (root / f"natural-proof-{arm}-before").is_dir()
            else None,
        )
        for arm in ("A", "B")
    }
    return {
        "quantitative_and_oracle_guards_pass": all(verdicts)
        and same_state
        and removed_work
        and all(
            item["discriminating_original_body_queries"] == 4
            for item in natural.values()
        ),
        "measured_work_removed": removed_work,
        "exact_stationary_rows": same_state,
        "natural_proof": natural,
        "schedules": evidence,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("--compare", action="store_true")
    args = parser.parse_args()
    report = summarize(args.root)
    if args.compare:
        report["comparison"] = compare(args.root, report)
    print(json.dumps(report, indent=2))
