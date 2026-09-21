#!/usr/bin/env python3
"""Compare replay-stable graph receipt identity across execution paths."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from experiments.graph_benchmarks import stable_receipt_identity


CELLS = ("graph-evidence01", "graph-expand01", "graph-retrieval01")


def _record(root: Path, cell: str) -> dict[str, object]:
    matches = []
    for path in root.rglob("record.json"):
        value = json.loads(path.read_text())
        if value.get("experiment") == cell:
            matches.append(value)
    if len(matches) != 1:
        raise ValueError(
            f"expected one {cell} receipt beneath {root}, found {len(matches)}"
        )
    return matches[0]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--standalone-root", type=Path, required=True)
    parser.add_argument("--gauntlet-root", type=Path, required=True)
    args = parser.parse_args()
    plan = json.loads((args.gauntlet_root / "execution-plan.json").read_text())
    raw_roots = {item["cell"]: Path(item["outputs"]["artifact_root"]) for item in plan}
    for cell in CELLS:
        standalone = stable_receipt_identity(_record(args.standalone_root, cell))
        gauntlet = stable_receipt_identity(_record(raw_roots[cell], cell))
        if standalone != gauntlet:
            raise ValueError(f"stable receipt identity differs for {cell}")
    print(json.dumps({"state": "complete", "cells": list(CELLS)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
