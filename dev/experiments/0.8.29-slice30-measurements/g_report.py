"""Report every frozen G obligation and retained current-program populations."""

import json
from pathlib import Path

from metrics import aggregate, paired_difference


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
        if not outcome.get("attempt"):
            continue
        try:
            actual = json.loads(Path(outcome["attempt"], "result.json").read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            report["issues"] = ["missing_or_partial_result"]
            continue
        report["actual"] = actual
        for step in actual.get("steps", []):
            result = step.get("result") or {}
            for item in result.get("passes", []):
                rows = [row for row in result.get("samples", [])
                        if row["adapter"] == item["adapter"] and row["route"] == item["route"]]
                report["populations"].append({"adapter": item["adapter"], "route": item["route"],
                                              "statistics": aggregate(rows, item["wall_ns"])})
                if cell["stage"] == "G-R" and outcome["status"] == "complete":
                    paired.setdefault(f"n{cell['n']}/{item['route']}", []).extend(rows)
    return {"scope": "G only; no D/M/F/whole30/40/50/Block1 clearance", "manifest": manifest,
            "complete": bool(cells) and all(row["status"] == "complete" for row in cells),
            "cells": cells, "paired_adapters": {key: paired_difference(rows) for key, rows in paired.items()}}
