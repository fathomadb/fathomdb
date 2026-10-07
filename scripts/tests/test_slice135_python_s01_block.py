"""Negative controls for the installed Python S01 block receipt."""

from __future__ import annotations

import copy
import importlib.util
from pathlib import Path
import sys

import pytest


WRAPPER = Path(__file__).resolve().parents[1] / "slice135_python_s01_block.py"
SPEC = importlib.util.spec_from_file_location("slice135_python_s01_block", WRAPPER)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

SOURCE = "a" * 40
WHEEL = "b" * 64
CORPUS = "c" * 64


def _attempt(kind: str) -> dict:
    if kind == "text":
        ids, branches = ["A"], ["text"]
    else:
        ids, branches = ["A", "B"], ["vector", "vector"]
    return {
        "latency_ns": 1000,
        "ids": ids,
        "branches": branches,
        "semantic_ok": True,
    }


def _raw(samples: int = 2) -> dict:
    return {
        "schema_version": 1,
        "source_sha": SOURCE,
        "artifact": {"wheel_sha256": WHEEL},
        "corpus_size": 32,
        "corpus_sha256": CORPUS,
        "model": "fathomdb-bge-small-en-v1.5 default embedder",
        "queries": {
            "text": "brightblue",
            "vector": "boat schedule across water",
            "hybrid": "harbor ferry",
        },
        "warm_samples_per_cell": samples,
        "semantic_failures": 0,
        "cells": {
            kind: {
                "session_cold": _attempt(kind),
                "warmup": _attempt(kind),
                "warm": [_attempt(kind) for _ in range(samples)],
            }
            for kind in ("text", "vector", "hybrid")
        },
    }


def test_block_checker_recomputes_counts_and_rejects_wrong_output() -> None:
    summary = MODULE.check_raw(
        _raw(),
        size=32,
        samples=2,
        source_sha=SOURCE,
        wheel_sha256=WHEEL,
        corpus_sha256=CORPUS,
    )
    assert summary["cells"]["text"]["warm_count"] == 2
    corrupted = _raw()
    corrupted["cells"]["text"]["warm"][0]["ids"] = ["B"]
    with pytest.raises(ValueError, match="text result"):
        MODULE.check_raw(
            corrupted,
            size=32,
            samples=2,
            source_sha=SOURCE,
            wheel_sha256=WHEEL,
            corpus_sha256=CORPUS,
        )


def test_block_checker_rejects_identity_and_count_mismatch() -> None:
    with pytest.raises(ValueError, match="source identity"):
        MODULE.check_raw(
            _raw(),
            size=32,
            samples=2,
            source_sha="d" * 40,
            wheel_sha256=WHEEL,
            corpus_sha256=CORPUS,
        )
    with pytest.raises(ValueError, match="sample count"):
        MODULE.check_raw(
            _raw(),
            size=32,
            samples=3,
            source_sha=SOURCE,
            wheel_sha256=WHEEL,
            corpus_sha256=CORPUS,
        )


def test_block_checker_rejects_claimed_semantic_failure_and_wrong_corpus() -> None:
    failed = _raw()
    failed["cells"]["vector"]["warm"][0]["semantic_ok"] = False
    with pytest.raises(ValueError, match="semantic failure"):
        MODULE.check_raw(
            failed,
            size=32,
            samples=2,
            source_sha=SOURCE,
            wheel_sha256=WHEEL,
            corpus_sha256=CORPUS,
        )
    changed = copy.deepcopy(_raw())
    changed["corpus_sha256"] = "d" * 64
    with pytest.raises(ValueError, match="corpus identity"):
        MODULE.check_raw(
            changed,
            size=32,
            samples=2,
            source_sha=SOURCE,
            wheel_sha256=WHEEL,
            corpus_sha256=CORPUS,
        )


def test_block_checker_rejects_malformed_artifact_as_invalid_data() -> None:
    malformed = _raw()
    malformed["artifact"] = []
    with pytest.raises(ValueError, match="wheel identity"):
        MODULE.check_raw(
            malformed,
            size=32,
            samples=2,
            source_sha=SOURCE,
            wheel_sha256=WHEEL,
            corpus_sha256=CORPUS,
        )


def test_nearest_rank_has_declared_indices() -> None:
    assert MODULE.nearest_rank(list(range(1, 101)), 0.50) == 50
    assert MODULE.nearest_rank(list(range(1, 101)), 0.95) == 95


def test_venv_executable_keeps_its_symlink_path(tmp_path: Path) -> None:
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    venv_python = bin_dir / "python"
    venv_python.symlink_to(sys.executable)
    assert MODULE.venv_executable(venv_python) == venv_python.absolute()
