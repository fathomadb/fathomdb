#!/usr/bin/env python3
"""One-time CUDA-only GRAPH-RETRIEVAL-01 seed materializer."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from experiments.graph_retrieval_01 import load_config, materialize_seed_manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--cohort", type=Path, required=True)
    parser.add_argument("--historical-observations", type=Path, required=True)
    parser.add_argument("--model-cache", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--fathomdb-bin", type=Path, required=True)
    parser.add_argument("--cuda-uuid", required=True)
    args = parser.parse_args()
    materialize_seed_manifest(
        load_config(args.config),
        corpus_path=args.corpus,
        cohort_path=args.cohort,
        historical_observations_path=args.historical_observations,
        model_cache=args.model_cache,
        artifact_root=args.artifact_root,
        output_path=args.output,
        fathomdb_bin=args.fathomdb_bin,
        cuda_uuid=args.cuda_uuid,
    )
    print(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
