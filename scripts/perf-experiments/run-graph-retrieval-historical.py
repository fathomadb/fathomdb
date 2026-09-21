#!/usr/bin/env python3
"""Run qualified historical MuSiQue through native graph expansion."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from experiments.graph_retrieval_01 import load_config, run_historical


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--extractions", type=Path, required=True)
    parser.add_argument("--cohort", type=Path, required=True)
    parser.add_argument("--seed-manifest", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--result", type=Path, required=True)
    parser.add_argument("--fathomdb-bin", type=Path, required=True)
    args = parser.parse_args()
    if args.artifact_root.exists():
        raise ValueError(f"artifact root must be new: {args.artifact_root}")
    args.artifact_root.mkdir(parents=True)
    result = run_historical(
        load_config(args.config),
        corpus_path=args.corpus,
        extractions_path=args.extractions,
        cohort_path=args.cohort,
        seed_manifest_path=args.seed_manifest,
        artifact_root=args.artifact_root,
        config_path=args.config,
        fathomdb_bin=args.fathomdb_bin,
    )
    args.result.parent.mkdir(parents=True, exist_ok=True)
    args.result.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(args.result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
