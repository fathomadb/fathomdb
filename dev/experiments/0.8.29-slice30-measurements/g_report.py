"""Report every frozen G obligation and retained current-program populations."""

import json
from pathlib import Path

from metrics import aggregate, paired_difference
from g_spans import summarize_spans


def summarize(root):
    """Keep raw/errors/failed cells visible; no whole30 or cross-family clearance."""
    root = Path(root)
    manifest = json.loads((root / "manifest.json").read_text())
    path = root / "results/g-outcomes.json"
    outcomes = {row["id"]: row for row in json.loads(path.read_text())} if path.exists() else {}
    cells, paired = [], {}
    for cell in manifest["plan"]:
        outcome = outcomes.get(cell["id"])
        report = {"id": cell["id"], "stage": cell["stage"], "n": cell["n"],
                  "status": outcome["status"] if outcome else "not_run", "populations": []}
        cells.append(report)
        if not outcome:
            continue
        report["outcome"] = outcome
        if not outcome.get("attempt") or outcome["status"].startswith("not_run_"):
            continue
        try:
            actual = json.loads(Path(outcome["attempt"], "result.json").read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            report["issues"] = ["missing_or_partial_result"]
            continue
        report["actual_result_path"] = str(Path(outcome["attempt"], "result.json"))
        report["qualification_issues"] = actual.get("qualification_issues", [])
        report["phase_summaries"] = []
        for step in actual.get("steps", []):
            result = step.get("result") or {}
            phases = {}
            span_issues = []
            for batch in result.get("observation_batches", []):
                tree = summarize_spans(batch.get("records", []), dropped_records=batch.get("dropped_records", 0),
                                       issues=batch.get("issues", []))
                span_issues.extend(tree["issues"])
                for record in tree["spans"]:
                    entry = phases.setdefault(record["phase"], {"count": 0, "inclusive_ns": [], "remainders": [], "non_additive": 0})
                    entry["count"] += 1
                    if record["inclusive_ns"] is not None:
                        entry["inclusive_ns"].append(record["inclusive_ns"])
                    if record["exclusive_remainder_ns"] is not None:
                        entry["remainders"].append(record["exclusive_remainder_ns"])
                    entry["non_additive"] += record["additivity"] == "non_additive"
            if phases:
                report["phase_summaries"].append({"output": step["output"], "issues": sorted(set(span_issues)),
                    "phases": {key: {"count": value["count"], "timed_count": len(value["inclusive_ns"]),
                                      "inclusive_mean_ns": sum(value["inclusive_ns"]) / len(value["inclusive_ns"]) if value["inclusive_ns"] else None,
                                      "demonstrated_remainder_count": len(value["remainders"]),
                                      "demonstrated_remainder_mean_ns": sum(value["remainders"]) / len(value["remainders"]) if value["remainders"] else None,
                                      "non_additive_count": value["non_additive"]}
                               for key, value in phases.items()},
                    "scope": "inclusive host intervals; nested/overlapping phases are nonadditive; per-sample raw trees retained"})
            for item in result.get("passes", []):
                rows = [row for row in result.get("samples", [])
                        if row["adapter"] == item["adapter"] and row["route"] == item["route"]]
                statistics = aggregate(rows, item["wall_ns"])
                if cell["stage"] == "G-O" or len(rows) < 1000:
                    for category in ("successful_latency", "error_latency"):
                        statistics[category] = {key: value for key, value in statistics[category].items()
                                                if key in ("count", "mean_ms")}
                report["populations"].append({"adapter": item["adapter"], "route": item["route"],
                                              "statistics": statistics})
                if cell["stage"] == "G-R" and outcome["status"] in ("complete", "reused_complete"):
                    paired.setdefault(f"n{cell['n']}/{item['route']}", []).extend(rows)
    invocation = json.loads((root / "last-run.json").read_text()) if (root / "last-run.json").exists() else None
    return {"invocation": invocation, "scope": "G only; no D/M/F/whole30/40/50/Block1 clearance", "manifest": manifest,
            "complete": bool(cells) and all(row["status"] == "complete" for row in cells)
            and invocation is not None and invocation.get("environment_ok") is True,
            "cells": cells, "paired_adapters": {key: paired_difference(rows) for key, rows in paired.items()}}
