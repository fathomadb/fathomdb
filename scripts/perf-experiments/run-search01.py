#!/usr/bin/env python3
"""Run the existing EARP IR-C characterization into an external result root."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from datetime import UTC, datetime
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
sys.path.append(str(ROOT / "src" / "python"))

from eval.earp.characterize import RunVerdict, run_characterization  # noqa: E402
import fathomdb  # noqa: E402


GOLD_SHA256 = "4caabddf7ce55f417e639e3c169fe2035b09c231f36d2f39d293a596373de2bb"
CORPUS_HASH = "fe973fcd49fbbda083158f69fe720f17858ab8528e171fa2188eec84131c7d4e"


def sha256(path: Path) -> str:
    """Return one input file's SHA-256 identity."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    """Validate the frozen inputs and delegate one SEARCH-01 run to EARP."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-root", type=Path, required=True)
    parser.add_argument("--snapshot", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--gold", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--expected-version", required=True)
    parser.add_argument("--experiment", default="search-01-ir-c")
    args = parser.parse_args()
    if fathomdb.__version__ != args.expected_version:
        parser.error(
            "SEARCH-01 FathomDB version drift: "
            f"expected {args.expected_version}, got {fathomdb.__version__}"
        )
    if sha256(args.gold) != GOLD_SHA256:
        parser.error("SEARCH-01 gold identity drifted")
    if args.output_root.exists():
        parser.error("SEARCH-01 output root must be new")
    result = run_characterization(
        data_root=args.data_root,
        snapshot_path=args.snapshot,
        gold_path=args.gold,
        gold_sha256=GOLD_SHA256,
        corpus_hash=CORPUS_HASH,
        qrels_version="ir-c-reused-v2",
        experiments_root=args.output_root,
        experiment=args.experiment,
        ts=datetime.now(UTC),
        evidence_recall_k=(5, 10),
        manifest_path=args.manifest,
    )
    summary = {
        "schema_version": "fathomdb.performance-gauntlet.search01/v1",
        "verdict": result.verdict.value,
        "run_id": result.run_id,
        "run_dir": str(result.run_dir),
        "ingested": result.ingested,
        "retrievals": result.retrievals,
        "fanout_used": result.fanout_used,
        "fathomdb_version": fathomdb.__version__,
        "blockers": [blocker.code for blocker in result.blockers],
    }
    print(json.dumps(summary, sort_keys=True))
    return 0 if result.verdict is RunVerdict.COMPLETE else 1


if __name__ == "__main__":
    raise SystemExit(main())
