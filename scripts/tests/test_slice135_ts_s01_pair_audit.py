"""Negative controls for independently recomputed TypeScript S01 pairs."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/slice135_ts_s01_pair_audit.py"
SPEC = importlib.util.spec_from_file_location("slice135_ts_s01_pair_audit", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
PROTOCOL = json.loads(
    (
        ROOT / "dev/plans/0.8.27/features/slice-135/s01-ts-comparison-protocol.json"
    ).read_text()
)


def _attempt(kind: str, latency: int) -> dict:
    return {
        "latency_ns": latency,
        "ids": ["A"] if kind == "text" else ["A", "B"],
        "branches": ["text"] if kind == "text" else ["vector", "vector"],
        "semantic_ok": True,
    }


def _raw(role: str = "baseline") -> dict:
    return {
        "schema_version": 1,
        "source_sha": PROTOCOL[role]["source_sha"],
        "artifact": {
            "native_sha256": MODULE.NATIVE_SHA256[role],
            "module_sha256": MODULE.MODULE_SHA256[role],
            "node_version": "v25.9.0",
        },
        "corpus_size": 32,
        "corpus_sha256": PROTOCOL["corpus_sha256_by_size"]["32"],
        "model": PROTOCOL["model"],
        "queries": PROTOCOL["queries"],
        "warm_samples_per_cell": 1000,
        "semantic_failures": 0,
        "started_utc": "2026-10-07T10:00:00.000Z",
        "finished_utc": "2026-10-07T10:01:00.000Z",
        "cells": {
            kind: {
                "session_cold": _attempt(kind, 10),
                "warmup": _attempt(kind, 20),
                "warm": [_attempt(kind, index + 1) for index in range(1000)],
            }
            for kind in ("text", "vector", "hybrid")
        },
    }


def test_recomputes_p50_p95_p99_from_all_calls() -> None:
    report = MODULE.audit_observations(
        _raw(), role="baseline", size=32, specification=PROTOCOL
    )
    assert report["checked_attempts"] == 3006
    assert report["cells"]["text"]["warm_p50_ns"] == 500
    assert report["cells"]["text"]["warm_p95_ns"] == 950
    assert report["cells"]["text"]["warm_p99_ns"] == 990


def test_rejects_mutated_text_id() -> None:
    raw = _raw()
    raw["cells"]["text"]["warm"][999]["ids"] = ["B"]
    with pytest.raises(ValueError, match="text anchor"):
        MODULE.audit_observations(raw, role="baseline", size=32, specification=PROTOCOL)


def test_rejects_false_tail_summary() -> None:
    raw = _raw()
    report = MODULE.audit_observations(
        raw, role="baseline", size=32, specification=PROTOCOL
    )
    summary = {"checked_attempts": report["checked_attempts"], "cells": report["cells"]}
    summary["cells"]["hybrid"]["warm_p99_ns"] += 1
    with pytest.raises(ValueError, match="p99"):
        MODULE.audit_observations(
            raw, role="baseline", size=32, specification=PROTOCOL, summary=summary
        )


def test_rejects_wrong_candidate_native_artifact() -> None:
    raw = _raw("candidate")
    raw["artifact"]["native_sha256"] = MODULE.NATIVE_SHA256["baseline"]
    with pytest.raises(ValueError, match="native"):
        MODULE.audit_observations(
            raw, role="candidate", size=32, specification=PROTOCOL
        )
