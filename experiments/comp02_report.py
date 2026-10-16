"""Summarize COMP-02a scale cells and COMP-02b filtered cells (side plan).

    python -m experiments.comp02_report scale --out DIR [DIR ...]
    python -m experiments.comp02_report filtered --out DIR
"""

from __future__ import annotations

import argparse
import json
import math
import statistics
from pathlib import Path
from typing import Any


def pct(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    return ordered[max(0, math.ceil(len(ordered) * fraction) - 1)]


def _cells(out: Path) -> list[dict[str, Any]]:
    return [json.loads(p.read_text(encoding="utf-8")) for p in sorted((out / "cells").glob("*.json"))]


def scale_rows(out: Path) -> list[dict[str, Any]]:
    rows = []
    for cell in _cells(out):
        steady = [r["latency_ms"] for r in cell["steady"]]
        cold = [r["latency_ms"] for r in cell["cold"]]
        rows.append({
            "arm": cell["arm"], "rep": cell["repetition"], "documents": cell["documents"],
            "ingest_ack_s": cell["ingest_ack_s"], "build_return_s": cell["build_return_s"],
            "query_ready_s": cell["query_ready_s"], "probe_failing": len(cell["probe_failing"]),
            "reopen_s": cell["reopen_s"], "first_query_after_reopen_ms": cell["first_query_after_reopen_s"] * 1000,
            "sentinel_after_reopen": cell["sentinel_found_after_reopen"],
            "cold_p50_ms": pct(cold, 0.5), "steady_p50_ms": pct(steady, 0.5),
            "steady_p95_ms": pct(steady, 0.95), "steady_p99_ms": pct(steady, 0.99),
            "empty_steady": sum(not r["ids"] for r in cell["steady"]),
            "disk_mib": cell["disk_bytes"] / 2**20, "peak_rss_mib": cell["peak_rss_mib"],
        })
    return rows


def filtered_rows(out: Path) -> list[dict[str, Any]]:
    rows = []
    for cell in _cells(out):
        for cohort, data in cell["passes"].items():
            lat = [r["latency_ms"] for r in data["results"]]
            rows.append({
                "arm": cell["arm"], "rep": cell["repetition"], "cohort": cohort,
                "selectivity": data["matching"] / cell["documents"], "violations": data["violations"],
                "short_results": data["short_results"], "p50_ms": pct(lat, 0.5), "p95_ms": pct(lat, 0.95),
                "p99_ms": pct(lat, 0.99),
            })
    return rows


def medians(rows: list[dict[str, Any]], keys: tuple[str, ...]) -> list[dict[str, Any]]:
    groups: dict[tuple[Any, ...], list[dict[str, Any]]] = {}
    for row in rows:
        groups.setdefault(tuple(row[k] for k in keys), []).append(row)
    out = []
    for group_key, members in sorted(groups.items()):
        merged: dict[str, Any] = dict(zip(keys, group_key))
        merged["reps"] = len(members)
        for name, value in members[0].items():
            if name in keys or name == "rep":
                continue
            values = [m[name] for m in members]
            if all(isinstance(v, bool) for v in values):
                merged[name] = all(values)
            elif all(isinstance(v, (int, float)) for v in values):
                merged[name] = statistics.median(values)
                merged[f"{name}_range"] = [min(values), max(values)]
        out.append(merged)
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("kind", choices=("scale", "filtered"))
    parser.add_argument("--out", type=Path, nargs="+", required=True)
    args = parser.parse_args()
    if args.kind == "scale":
        rows = [row for out in args.out for row in scale_rows(out)]
        report = medians(rows, ("documents", "arm"))
    else:
        rows = [row for out in args.out for row in filtered_rows(out)]
        report = medians(rows, ("cohort", "arm"))
    print(json.dumps({"cells": rows, "medians": report}, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
