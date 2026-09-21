#!/usr/bin/env python3
"""Generate a strict graph-suite config and invoke the existing gauntlet."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", required=True)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    args = parser.parse_args()
    source = args.source_root.resolve(strict=True)
    output = args.output_root.resolve(strict=False)
    if output.exists():
        raise ValueError(f"output root must be new: {output}")
    external = output.parent / f"{output.name}-graph-raw"
    if external.exists():
        raise ValueError(f"external graph output root must be new: {external}")
    external.mkdir(parents=True)
    commit = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=source,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout.strip()
    configs = source / "experiments/configs"
    config_path = output.parent / f".{output.name}-graph-gauntlet-config.json"
    document = {
        "schema_version": "fathomdb.performance-gauntlet.config/v1",
        "release": args.release,
        "source": {"root": str(source), "commit": commit},
        "output_root": str(output),
        "runtime": {
            "fathomdb_cli": str(source / "target/debug/fathomdb"),
            "python": str(Path(sys.executable).absolute()),
            "wheel": None,
            "native_extension": None,
            "virtualenv": None,
        },
        "configs": {
            "ac072": None,
            "ac073": None,
            "ac075": None,
            "scale02": None,
            "protected_writes": None,
            "ce_profile": None,
            "search01": None,
            "locomo": None,
            "graph_evidence": str(configs / "graph-evidence-01/exact-evidence.v1.json"),
            "graph_expand": str(configs / "graph-expand-01/bounded-traversal.v1.json"),
            "graph_retrieval": str(
                configs / "graph-retrieval-01/musique-native-expand.v1.json"
            ),
        },
        "assets": {
            "ir_c": None,
            "ac073": None,
            "tc5": None,
            "locomo": None,
            "ce_profile": None,
            "graph_output": {"external_output_root": str(external)},
            "musique": None,
            "stark": None,
        },
        "gpu": {"enabled": False, "cuda_uuid": None},
        "cells": ["graph-evidence01", "graph-expand01", "graph-retrieval01"],
        "timeouts_s": {"default": 900, "cells": {}},
    }
    config_path.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n")
    runner = source / "scripts/perf-experiments/run_gauntlet.py"
    environment = os.environ.copy()
    environment["FATHOMDB_GRAPH_SMOKE_ALLOW_DIRTY"] = "1"
    result = subprocess.run(
        [
            sys.executable,
            str(runner),
            "--release",
            args.release,
            "--source-root",
            str(source),
            "--config",
            str(config_path),
            "--output-root",
            str(output),
            "--suite",
            "graph",
        ],
        cwd=source,
        env=environment,
        check=False,
    )
    if output.exists():
        retained = output / "graph-gauntlet-input.json"
        retained.write_bytes(config_path.read_bytes())
    config_path.unlink(missing_ok=True)
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
