#!/usr/bin/env python3
"""Run canonical LOCOMO A0 and delegate retrieval scoring output-locally."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from experiments import fathomdb_locomo, locomo_metrics  # noqa: E402


def main() -> int:
    """Run the existing LOCOMO facade/harness and metrics APIs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--base-dir", type=Path, required=True)
    parser.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    config = fathomdb_locomo.resolve_config(
        json.loads(args.config.read_text(encoding="utf-8"))
    )
    run_id, receipt_dir, returncode = fathomdb_locomo.run(
        config, base_dir=args.base_dir
    )
    raw_dir = Path(config["output"]["external_root"]) / run_id
    predictions = sorted(raw_dir.glob("predicted_*"))
    provenance = raw_dir / "facade-provenance.v1.json"
    facade_metrics = raw_dir / "facade-metrics.v1.json"
    retrieval_summary = args.result.parent / "locomo-retrieval-summary.json"
    state = "blocked"
    if returncode == 0 and len(predictions) == 1 and provenance.is_file():
        summary = locomo_metrics.summarize_predictions(
            predictions[0], json.loads(provenance.read_text(encoding="utf-8"))
        )
        retrieval_summary.write_text(
            json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        state = "complete"
    result = {
        "schema_version": "fathomdb.performance-gauntlet.locomo/v1",
        "state": state,
        "run_id": run_id,
        "predictions": str(predictions[0]) if len(predictions) == 1 else None,
        "provenance": str(provenance),
        "facade_metrics": str(facade_metrics),
        "retrieval_summary": str(retrieval_summary),
        "receipt_dir": str(receipt_dir),
        "raw_dir": str(raw_dir),
    }
    args.result.parent.mkdir(parents=True, exist_ok=True)
    args.result.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(json.dumps(result, sort_keys=True))
    return 0 if state == "complete" else returncode or 1


if __name__ == "__main__":
    raise SystemExit(main())
