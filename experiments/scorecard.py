"""Generate the safe, aggregate capability scorecard from run receipts."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

from experiments import _lib


_FIELDS = (
    "configuration",
    "m1_r_at_10",
    "m2_mrr",
    "m2_r_at_1",
    "m2_ndcg_at_10",
    "m4_proxy_temporal_evidence_recall",
    "m6_query_p95_ms",
    "m7_ready_to_search_ms",
)
_METRIC_FIELDS = _FIELDS[1:]
_HEADERS = (
    ("run_id", "run ID"),
    ("configuration", "configuration"),
    ("verdict", "verdict"),
    ("m1_r_at_10", "M1 R@10"),
    ("m2_mrr", "M2 MRR"),
    ("m2_r_at_1", "M2 R@1"),
    ("m2_ndcg_at_10", "M2 nDCG@10"),
    ("m4_proxy_temporal_evidence_recall", "M4 proxy"),
    ("m6_query_p95_ms", "M6 p95 ms"),
    ("m7_ready_to_search_ms", "M7 ready ms"),
)


def _scorecard_metrics(value: object, *, run_id: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != set(_FIELDS):
        actual = set(value) if isinstance(value, dict) else set()
        raise ValueError(
            f"capability_scorecard keys mismatch for {run_id}: "
            f"missing={sorted(set(_FIELDS) - actual)}, unknown={sorted(actual - set(_FIELDS))}"
        )
    configuration = value["configuration"]
    if not isinstance(configuration, str) or not configuration:
        raise ValueError(f"capability_scorecard.configuration must be a non-empty string for {run_id}")
    for field in _METRIC_FIELDS:
        metric = value[field]
        if isinstance(metric, bool) or not isinstance(metric, (int, float)) or not math.isfinite(metric):
            raise ValueError(f"capability_scorecard.{field} must be a finite number for {run_id}")
    return dict(value)


def scorecard_rows(runs_dir: str | Path) -> list[dict[str, Any]]:
    """Return validated aggregate capability rows from content-free run receipts."""
    root = Path(runs_dir)
    rows: list[dict[str, Any]] = []
    if not root.is_dir():
        return rows
    for receipt_path in sorted(root.glob("*/record.json")):
        try:
            data = json.loads(receipt_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            raise ValueError(f"invalid record JSON: {receipt_path}") from exc
        record = _lib.record_from_dict(data)
        if not isinstance(record.metrics, dict) or "capability_scorecard" not in record.metrics:
            continue
        row = _scorecard_metrics(record.metrics["capability_scorecard"], run_id=record.run_id)
        rows.append({"run_id": record.run_id, "verdict": record.verdict, **row})
    return rows


def _cell(value: object) -> str:
    if isinstance(value, float):
        return f"{value:.6g}"
    return str(value).replace("|", "\\|")


def regen_scoreboard(*, runs_dir: str | Path, output_path: str | Path) -> None:
    """Regenerate the Markdown scorecard from safe aggregate receipt fields."""
    rows = scorecard_rows(runs_dir)
    labels = [label for _, label in _HEADERS]
    lines = [
        "# Capability scorecard",
        "",
        "GENERATED FROM content-free `experiments/runs/*/record.json` receipts — do NOT hand-edit.",
        "Raw predictions, logs, and databases remain in external artifact storage.",
        "",
        "| " + " | ".join(labels) + " |",
        "| " + " | ".join("---" for _ in labels) + " |",
    ]
    for row in rows:
        lines.append("| " + " | ".join(_cell(row[key]) for key, _ in _HEADERS) + " |")
    destination = Path(output_path)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    """Regenerate the repository capability scorecard."""
    parser = argparse.ArgumentParser(description="Generate the FathomDB capability scorecard")
    parser.add_argument("--runs-dir", type=Path, default=_lib.EXPERIMENTS_DIR / "runs")
    parser.add_argument("--output", type=Path, default=_lib.EXPERIMENTS_DIR / "SCOREBOARD.md")
    args = parser.parse_args(argv)
    try:
        regen_scoreboard(runs_dir=args.runs_dir, output_path=args.output)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
