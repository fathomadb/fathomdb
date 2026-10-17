"""Check one benchmark attempt against the known-run baseline manifest.

Each manifest metric names a JSON Pointer into the attempt's ``summary.json``
or ``report.json``, an expected value, and a tolerance: ``{"abs": x}``,
``{"rel": x}`` (a fraction of the expected value) or ``{"questions": k, "n": n}``
(k questions out of n, for per-type rates). Every metric prints PASS, DRIFT or
MISSING with the measured value, expected value and tolerance; the command exits
non-zero on any DRIFT or MISSING.

    python -m experiments.baseline_check MANIFEST BENCHMARK_ID SUMMARY
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Mapping, Sequence

from experiments import _lib

DEFAULT_MANIFEST = _lib.EXPERIMENTS_DIR / "configs" / "baselines" / "slice-40-baseline.v1.json"
SCHEMA = "fathomdb.baseline.v1"


def load_manifest(path: Path) -> dict[str, Any]:
    manifest = json.loads(Path(path).read_text(encoding="utf-8"))
    if manifest.get("schema_version") != SCHEMA:
        raise ValueError(f"baseline manifest schema must be {SCHEMA}")
    ids = [benchmark["id"] for benchmark in manifest["benchmarks"]]
    if len(ids) != len(set(ids)):
        raise ValueError("duplicate benchmark id")
    return manifest


def allowed_delta(tolerance: Mapping[str, Any], *, expected: float) -> float:
    """The largest absolute difference the tolerance accepts around ``expected``."""
    if set(tolerance) == {"abs"}:
        return float(tolerance["abs"])
    if set(tolerance) == {"rel"}:
        return abs(expected) * float(tolerance["rel"])
    if set(tolerance) == {"questions", "n"}:
        return float(tolerance["questions"]) / float(tolerance["n"])
    raise ValueError(f"unknown tolerance {dict(tolerance)}")


def _resolve(value: Any, pointer: str) -> Any:
    for raw in pointer[1:].split("/"):
        token = raw.replace("~1", "/").replace("~0", "~")
        if isinstance(value, dict) and token in value:
            value = value[token]
        elif isinstance(value, list) and token.isdigit() and int(token) < len(value):
            value = value[int(token)]
        else:
            return None
    return value


def check(entry: Mapping[str, Any], summary: Any) -> list[dict[str, Any]]:
    """One row per manifest metric: PASS, DRIFT, or MISSING."""
    rows = []
    for metric in entry["metrics"]:
        measured = _resolve(summary, metric["pointer"])
        expected = metric["expected"]
        row = {"name": metric["name"], "expected": expected, "measured": measured,
               "tolerance": metric["tolerance"], "gate": metric.get("gate", True)}
        if isinstance(measured, bool) or not isinstance(measured, (int, float)):
            row["measured"] = None
            row["status"] = "MISSING"
        else:
            delta = allowed_delta(metric["tolerance"], expected=expected)
            # A tiny epsilon keeps a value exactly on the boundary inside it.
            row["status"] = "PASS" if abs(measured - expected) <= delta + 1e-12 else "DRIFT"
        rows.append(row)
    return rows


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("benchmark")
    parser.add_argument("summary", type=Path)
    args = parser.parse_args(argv)
    manifest = load_manifest(args.manifest)
    entry = next((b for b in manifest["benchmarks"] if b["id"] == args.benchmark), None)
    if entry is None:
        parser.error(f"unknown benchmark {args.benchmark}")
    rows = check(entry, json.loads(args.summary.read_text(encoding="utf-8")))
    failing = 0
    for row in rows:
        gate = "" if row["gate"] else " (not a gate)"
        print(f"{row['status']:<7} {row['name']}: measured {row['measured']} expected {row['expected']} "
              f"tolerance {json.dumps(row['tolerance'])}{gate}")
        failing += row["gate"] and row["status"] != "PASS"
    return 1 if failing else 0


if __name__ == "__main__":
    raise SystemExit(main())
