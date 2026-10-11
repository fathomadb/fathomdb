"""Negative controls for installed-TypeScript S01 receipt validation."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest


SCRIPT = Path(__file__).resolve().parents[1] / "slice135_ts_s01_block.py"
SPEC = importlib.util.spec_from_file_location("slice135_ts_s01_block", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def _attempt(kind: str) -> dict:
    if kind == "text":
        ids, branches = ["A"], ["text"]
    else:
        ids, branches = ["A", "B"], ["vector", "vector"]
    return {"latency_ns": 100, "ids": ids, "branches": branches, "semantic_ok": True}


def _raw(samples: int = 100) -> dict:
    cells = {
        kind: {
            "session_cold": _attempt(kind),
            "warmup": _attempt(kind),
            "warm": [_attempt(kind) for _ in range(samples)],
        }
        for kind in ("text", "vector", "hybrid")
    }
    return {
        "schema_version": 1,
        "source_sha": "a" * 40,
        "artifact": {"native_sha256": "b" * 64},
        "corpus_size": 32,
        "corpus_sha256": MODULE.CORPUS_SHA256[32],
        "model": MODULE.MODEL,
        "queries": MODULE.QUERIES,
        "warm_samples_per_cell": samples,
        "semantic_failures": 0,
        "cells": cells,
    }


def _check(raw: dict, samples: int = 100) -> dict:
    return MODULE.check_raw(
        raw,
        size=32,
        samples=samples,
        source_sha="a" * 40,
        native_sha256="b" * 64,
    )


def test_recomputes_all_attempts_and_declared_percentiles() -> None:
    summary = _check(_raw())
    assert summary["checked_attempts"] == 306
    assert summary["cells"]["text"]["warm_p95_ns"] == 100
    assert summary["cells"]["text"]["p99"] == "unsupported_below_1000_samples"


def test_rejects_wrong_seeded_text_id() -> None:
    raw = _raw()
    raw["cells"]["text"]["warm"][25]["ids"] = ["B"]
    with pytest.raises(ValueError, match="seeded exact match"):
        _check(raw)


def test_rejects_missing_vector_branch_and_truncated_samples() -> None:
    raw = _raw()
    raw["cells"]["vector"]["warm"][0]["branches"] = ["text", "text"]
    with pytest.raises(ValueError, match="vector branch absent"):
        _check(raw)
    raw = _raw()
    raw["cells"]["vector"]["warm"].pop()
    with pytest.raises(ValueError, match="sample count"):
        _check(raw)


def test_rejects_artifact_or_corpus_identity_mismatch() -> None:
    raw = _raw()
    raw["artifact"]["native_sha256"] = "c" * 64
    with pytest.raises(ValueError, match="native artifact"):
        _check(raw)
    raw = _raw()
    raw["corpus_sha256"] = "d" * 64
    with pytest.raises(ValueError, match="corpus"):
        _check(raw)


def _comparison() -> dict:
    return {
        "schema_version": 1,
        "status": "FROZEN_TS_S01_PAIRED",
        "baseline": {
            "source_sha": "a" * 40,
            "main_archive_sha256": "b" * 64,
            "platform_archive_sha256": "c" * 64,
        },
        "candidate": {
            "source_sha": "d" * 40,
            "main_archive_sha256": "e" * 64,
            "platform_archive_sha256": "f" * 64,
        },
        "rows": [32, 256],
        "warm_samples_per_cell": 1000,
        "node_version": "v25.9.0",
        "model": MODULE.MODEL,
        "queries": MODULE.QUERIES,
        "corpus_sha256_by_size": {str(k): v for k, v in MODULE.CORPUS_SHA256.items()},
        "workload_runner_sha256": MODULE.sha256(MODULE.WORKLOAD.read_bytes()),
    }


def test_paired_protocol_rejects_wrong_artifact_and_sample_count() -> None:
    specification = _comparison()
    MODULE.validate_comparison_protocol(
        specification,
        role="baseline",
        size=32,
        samples=1000,
        source_sha="a" * 40,
        main_sha256="b" * 64,
        platform_sha256="c" * 64,
    )
    with pytest.raises(ValueError, match="artifact"):
        MODULE.validate_comparison_protocol(
            specification,
            role="baseline",
            size=32,
            samples=1000,
            source_sha="a" * 40,
            main_sha256="0" * 64,
            platform_sha256="c" * 64,
        )
    with pytest.raises(ValueError, match="sample count"):
        MODULE.validate_comparison_protocol(
            specification,
            role="baseline",
            size=32,
            samples=100,
            source_sha="a" * 40,
            main_sha256="b" * 64,
            platform_sha256="c" * 64,
        )
