"""Mutation controls for independent TypeScript S01 pilot recomputation."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest


SCRIPT = Path(__file__).resolve().parents[1] / "slice135_ts_s01_audit.py"
SPEC = importlib.util.spec_from_file_location("slice135_ts_s01_audit", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def _attempt(kind: str, latency: int = 100) -> dict:
    return {
        "latency_ns": latency,
        "ids": ["A"] if kind == "text" else ["A", "B"],
        "branches": ["text"] if kind == "text" else ["vector", "vector"],
        "semantic_ok": True,
    }


def _raw() -> dict:
    return {
        "schema_version": 1,
        "source_sha": MODULE.SOURCE_SHA,
        "artifact": {
            "native_sha256": MODULE.NATIVE_SHA256,
            "module_sha256": MODULE.MODULE_SHA256,
            "node_version": "v25.9.0",
        },
        "corpus_size": 32,
        "corpus_sha256": MODULE.CORPUS_SHA256[32],
        "model": MODULE.MODEL,
        "queries": MODULE.QUERIES,
        "warm_samples_per_cell": 100,
        "semantic_failures": 0,
        "started_utc": "2026-10-07T10:00:00.000Z",
        "finished_utc": "2026-10-07T10:01:00.000Z",
        "cells": {
            kind: {
                "session_cold": _attempt(kind),
                "warmup": _attempt(kind),
                "warm": [_attempt(kind) for _ in range(100)],
            }
            for kind in ("text", "vector", "hybrid")
        },
    }


def test_audit_recomputes_every_attempt() -> None:
    report = MODULE.audit_observations(_raw(), 32)
    assert report["checked_attempts"] == 306
    assert report["cells"]["text"]["warm_p95_ns"] == 100


def test_audit_rejects_wrong_text_id() -> None:
    raw = _raw()
    raw["cells"]["text"]["warm"][50]["ids"] = ["B"]
    with pytest.raises(ValueError, match="text anchor"):
        MODULE.audit_observations(raw, 32)


def test_audit_rejects_false_reported_percentile() -> None:
    raw = _raw()
    summary = {
        "checked_attempts": 306,
        "cells": {
            kind: {
                "session_cold_ns": 100,
                "warm_count": 100,
                "warm_p50_ns": 100,
                "warm_p95_ns": 101 if kind == "vector" else 100,
                "warm_max_ns": 100,
                "p99": "unsupported_below_1000_samples",
            }
            for kind in ("text", "vector", "hybrid")
        },
    }
    with pytest.raises(ValueError, match="percentile"):
        MODULE.audit_observations(raw, 32, summary)


def test_audit_rejects_wrong_native_identity() -> None:
    raw = _raw()
    raw["artifact"]["native_sha256"] = "0" * 64
    with pytest.raises(ValueError, match="native"):
        MODULE.audit_observations(raw, 32)
