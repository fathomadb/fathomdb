#!/usr/bin/env python3
"""Fail-closed verifier for a retained CUDA preflight witness."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "fathomdb.cuda-preflight-witness/v1"
WITNESS_NAME = "cuda-preflight-witness.json"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
COMMIT_SHA = re.compile(r"[0-9a-f]{40}\Z")
REQUIRED_EVIDENCE = frozenset(
    {
        "environment.txt",
        "manylinux-build.txt",
        "dynamic-dependencies.txt",
        "python-auditwheel.txt",
        "driverless-python-cpu-smoke.txt",
        "driverless-napi-cpu-smoke.txt",
        "gpu-python-cuda-witness.txt",
        "gpu-node-cuda-witness.txt",
        "gpu-node-cuda-smoke.txt",
    }
)


def fail(message: str) -> None:
    print(f"cuda-preflight-witness: {message}", file=sys.stderr)
    raise SystemExit(1)


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode("utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--witness-dir", required=True, type=Path)
    parser.add_argument("--candidate-sha", required=True)
    return parser.parse_args()


def load_witness(path: Path) -> tuple[dict[str, Any], bytes]:
    if path.is_symlink():
        fail(f"witness must not be a symlink: {path}")
    try:
        raw = path.read_bytes()
    except OSError as error:
        fail(f"cannot read witness {path}: {error}")
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as error:
        fail(f"malformed witness JSON: {error}")
    if not isinstance(value, dict):
        fail("witness must be a JSON object")
    if raw != canonical_json(value):
        fail("witness JSON is not canonical")
    return value, raw


def validate(witness_dir: Path, candidate_sha: str) -> None:
    if not COMMIT_SHA.fullmatch(candidate_sha):
        fail("requested candidate SHA must be a lowercase 40-hex commit")
    if witness_dir.is_symlink():
        fail(f"witness directory must not be a symlink: {witness_dir}")
    if not witness_dir.is_dir():
        fail(f"witness directory is absent: {witness_dir}")

    witness, _ = load_witness(witness_dir / WITNESS_NAME)
    if set(witness) != {"schema_version", "candidate_sha", "outcome", "evidence_sha256"}:
        fail("witness has missing or unknown top-level fields")
    if witness["schema_version"] != SCHEMA_VERSION:
        fail("witness schema version is unsupported")
    if witness["candidate_sha"] != candidate_sha:
        fail("witness candidate SHA does not match the requested candidate")
    if witness["outcome"] != "passed":
        fail("witness outcome is not passed")

    evidence = witness["evidence_sha256"]
    if not isinstance(evidence, dict) or set(evidence) != REQUIRED_EVIDENCE:
        fail("witness evidence inventory is incomplete or contains unknown evidence")
    for name, expected_digest in evidence.items():
        if not isinstance(expected_digest, str) or SHA256.fullmatch(expected_digest) is None:
            fail(f"witness has an invalid SHA-256 for {name}")
        path = witness_dir / name
        if path.is_symlink():
            fail(f"required evidence must not be a symlink: {name}")
        if not path.is_file():
            fail(f"required evidence is absent: {name}")
        contents = path.read_bytes()
        if not contents:
            fail(f"required evidence is empty: {name}")
        actual_digest = hashlib.sha256(contents).hexdigest()
        if actual_digest != expected_digest:
            fail(f"evidence digest mismatch: {name}")


def main() -> None:
    args = parse_args()
    validate(args.witness_dir, args.candidate_sha)
    print("cuda-preflight-witness: pass")


if __name__ == "__main__":
    main()
