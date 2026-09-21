"""Shared pure contracts for the graph benchmark cells."""

from __future__ import annotations

import hashlib
import json
import math
import subprocess
import sys
from collections.abc import Mapping, Sequence
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


class GraphBenchmarkError(ValueError):
    """Report a malformed benchmark contract or observation."""


def exact_mapping(value: object, label: str, keys: set[str]) -> dict[str, Any]:
    """Return a string-keyed mapping containing exactly ``keys``."""
    if not isinstance(value, dict) or any(not isinstance(key, str) for key in value):
        raise GraphBenchmarkError(f"{label} must be an object")
    actual = set(value)
    missing = keys - actual
    unknown = actual - keys
    if missing or unknown:
        raise GraphBenchmarkError(
            f"{label} keys drifted: missing={sorted(missing)}, unknown={sorted(unknown)}"
        )
    return value


def canonical_json(value: object) -> bytes:
    """Encode one identity-bearing value as canonical UTF-8 JSON."""
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def canonical_sha256(value: object) -> str:
    """Return the canonical JSON SHA-256 for ``value``."""
    return hashlib.sha256(canonical_json(value)).hexdigest()


def sha256_file(path: str | Path) -> str:
    """Return the SHA-256 digest of one regular file."""
    candidate = Path(path)
    if not candidate.is_file():
        raise GraphBenchmarkError(f"required file is unavailable: {candidate}")
    digest = hashlib.sha256()
    with candidate.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def percentile(values: Sequence[float], probability: float) -> float:
    """Return the pinned linear interpolation at ``p * (n - 1)``."""
    if not values or not 0.0 <= probability <= 1.0:
        raise GraphBenchmarkError("percentile inputs are invalid")
    ordered = sorted(float(value) for value in values)
    position = probability * (len(ordered) - 1)
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def reconcile_denominators(
    *, attempted: int, completed: int, errors: int, typed_refusals: int
) -> dict[str, int]:
    """Validate and return the common attempted-outcome accounting."""
    values = (attempted, completed, errors, typed_refusals)
    if any(
        isinstance(value, bool) or not isinstance(value, int) or value < 0
        for value in values
    ):
        raise GraphBenchmarkError("denominators must be non-negative integers")
    if attempted != completed + errors + typed_refusals:
        raise GraphBenchmarkError("denominators do not reconcile")
    return {
        "attempted": attempted,
        "completed": completed,
        "errors": errors,
        "typed_refusals": typed_refusals,
    }


def stable_receipt_identity(record: Mapping[str, Any]) -> dict[str, Any]:
    """Project a run receipt onto replay-stable standalone/gauntlet identity."""
    config = record.get("config")
    code = record.get("code")
    env = record.get("env")
    corpus = record.get("corpus")
    if not all(isinstance(value, Mapping) for value in (config, code, env, corpus)):
        raise GraphBenchmarkError("receipt identity sections are unavailable")
    resolved = config.get("resolved")
    workload = (
        resolved.get("workload_identity") if isinstance(resolved, Mapping) else None
    )
    return {
        "schema_version": record.get("schema_version"),
        "experiment": record.get("experiment"),
        "source_commit": code.get("git_sha"),
        "runtime": env.get("key_deps"),
        "config_sha256": config.get("sha256"),
        "workload_identity": workload,
        "corpus": {
            "manifest_sha256": corpus.get("manifest_sha256"),
            "datasets": corpus.get("datasets"),
        },
    }


def write_benchmark_record(
    *,
    experiment: str,
    config: Mapping[str, Any],
    metrics: Mapping[str, Any],
    base_dir: str | Path,
    config_path: str | Path,
    corpus_manifest_sha256: str,
    datasets: Sequence[str],
) -> Path:
    """Write one graph result through the existing experiment registry."""
    from experiments import _lib

    repo = Path(__file__).resolve().parent.parent

    def git(*args: str) -> str:
        result = subprocess.run(
            ["git", *args],
            cwd=repo,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
        )
        return result.stdout.strip()

    resolved = dict(config)
    resolved["workload_identity"] = {
        "cell": experiment,
        "config_sha256": canonical_sha256(config),
        "corpus_manifest_sha256": corpus_manifest_sha256,
    }
    run_id, run_dir = _lib.write_record(
        experiment,
        ts=datetime.now(UTC),
        config_obj=resolved,
        metrics=dict(metrics),
        verdict="complete",
        read="directional benchmark observation",
        code={
            "git_sha": git("rev-parse", "HEAD"),
            "dirty": bool(git("status", "--porcelain")),
            "branch": git("branch", "--show-current"),
            "baseline_commit": None,
        },
        corpus={
            "source": "synthetic" if not datasets else "pinned",
            "manifest_sha256": corpus_manifest_sha256,
            "datasets": list(datasets),
        },
        seeds={},
        env={
            "python": sys.version.split()[0],
            "lockfile_sha256": sha256_file(repo / "Cargo.lock"),
            "gpu": None,
            "key_deps": {},
        },
        cost_usd=0.0,
        config_path=str(Path(config_path).resolve()),
        base_dir=base_dir,
        index_path=Path(base_dir) / "index.jsonl",
    )
    del run_id
    return run_dir / "record.json"
