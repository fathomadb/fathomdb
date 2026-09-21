#!/usr/bin/env python3
"""Shared output-local adapter for graph benchmark gauntlet cells."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
from typing import Callable


def run(module: object, argv: list[str] | None = None) -> int:
    """Parse gauntlet paths, invoke one domain runner, and write its result."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--result", type=Path, required=True)
    parser.add_argument("--fathomdb-bin", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.artifact_root.exists():
        raise ValueError(f"artifact root must be new: {args.artifact_root}")
    args.artifact_root.mkdir(parents=True)
    os.environ["FATHOMDB_BIN"] = str(args.fathomdb_bin)
    load: Callable[[Path], object] = getattr(module, "load_config")
    smoke: Callable[..., dict[str, object]] = getattr(module, "run_smoke")
    result = smoke(load(args.config), args.artifact_root, config_path=args.config)
    args.result.parent.mkdir(parents=True, exist_ok=True)
    args.result.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(args.result)
    return 0
