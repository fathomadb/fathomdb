#!/usr/bin/env python3
"""Seal the exact Slice 75 CUDA package set to its candidate and preflight."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


SHA = re.compile(r"[0-9a-f]{40}\Z")


def fail(message: str) -> None:
    raise SystemExit(f"seal-slice75-cuda-packages: {message}")


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidate-sha", required=True)
    parser.add_argument("--packages", type=Path, required=True)
    parser.add_argument("--witness", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    if SHA.fullmatch(args.candidate_sha) is None:
        fail("invalid candidate SHA")
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True
    ).stdout.strip()
    if head != args.candidate_sha:
        fail("candidate SHA differs from HEAD")
    if args.output.exists() or args.output.is_symlink():
        fail("output must be new")
    subprocess.run(
        [
            sys.executable,
            "scripts/release/verify-cuda-preflight-witness.py",
            "--witness-dir",
            str(args.witness),
            "--candidate-sha",
            args.candidate_sha,
        ],
        check=True,
    )
    if args.packages.is_symlink() or not args.packages.is_dir():
        fail("package directory must be a non-symlink directory")
    artifacts = sorted(path for path in args.packages.iterdir() if path.is_file())
    wheels = [path for path in artifacts if path.suffix == ".whl"]
    cli = [path for path in artifacts if path.name.endswith("-x86_64-unknown-linux-gnu.tar.gz")]
    napi = [path for path in artifacts if "linux-x64-gnu" in path.name and path.suffix == ".tgz"]
    npm_main = [path for path in artifacts if path.suffix == ".tgz" and path not in napi]
    if not all(len(group) == 1 for group in (wheels, cli, napi, npm_main)) or len(artifacts) != 4:
        fail("package inventory must contain exactly one wheel, CLI, N-API, and npm main artifact")
    if any(path.is_symlink() for path in artifacts):
        fail("package artifacts must not be symlinks")

    witness_receipt = args.witness / "cuda-preflight-witness.json"
    payload = {
        "schema_version": "fathomdb.slice75-cuda-package-set/v1",
        "candidate_sha": args.candidate_sha,
        "preflight_witness_sha256": digest(witness_receipt),
        "artifacts": {path.name: digest(path) for path in artifacts},
    }
    args.output.write_text(
        json.dumps(payload, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n",
        encoding="ascii",
    )
    print(f"seal-slice75-cuda-packages: sealed {len(artifacts)} artifacts")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
