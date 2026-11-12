"""Freeze, run and summarize the bounded Slice30 provider-free campaign."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from campaign import make_plan


def freeze(cells, corpus, queries, binary, out):
    """Freeze actual input/native/source identities and the balanced cell plan."""
    cells, corpus, queries, binary, out = [Path(p).resolve() for p in (cells, corpus, queries, binary, out)]
    source = Path(__file__).resolve().parents[3]
    def identify(path):
        return {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
    def input_record(path):
        return dict(identify(path), count=len(path.read_text().splitlines()))
    base = json.loads(cells.read_text())
    plan = make_plan([dict(cell, corpus=str(corpus), queries=str(queries)) for cell in base], 2)
    manifest = {
        "schema": "slice30-p/v1", "measurement_layer": "public-direct-text", "gated": False,
        "source_sha": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=source, text=True).strip(),
        "source_dirty": subprocess.check_output(["git", "status", "--porcelain"], cwd=source, text=True).splitlines(),
        "binary": identify(binary), "corpus": input_record(corpus), "queries": input_record(queries),
        "cells": identify(cells), "lock": identify(source / "Cargo.lock"),
        "build_command": "cargo build --locked --release -p fathomdb-sdk --example slice30_measure",
        "format": 37, "provider": "none", "profile": "release", "features": [],
        "cell_seconds": 900, "campaign_seconds": 14400, "active_bytes": 16 * 1024**3,
        "repeat_count": 2, "warmup_policy": "separate records; not included in timed statistics",
        "source_observation": "freeze records actual dirty state; qualifying timing requires clean source",
    }
    if manifest["corpus"]["count"] < max(cell["n"] for cell in plan):
        raise ValueError("corpus is smaller than a selected cell")
    out.mkdir(parents=True, exist_ok=False)
    for name, value in (("manifest.json", manifest), ("plan.json", plan)):
        path = out / name
        path.write_text(json.dumps(value, indent=2) + "\n")
        path.chmod(0o444)
    return manifest


def main():
    """Operate an explicitly selected local campaign; never build in timings."""
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    freeze_parser = sub.add_parser("freeze")
    for field in ("cells", "corpus", "queries", "binary", "out"):
        freeze_parser.add_argument("--" + field, type=Path, required=True)
    args = parser.parse_args()
    freeze(args.cells, args.corpus, args.queries, args.binary, args.out)


if __name__ == "__main__":
    main()
