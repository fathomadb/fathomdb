"""Bounded installed-wheel S02 contention must preserve concurrent state."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_s02_contention as contention  # noqa: E402


def observed() -> dict:
    return {
        "status": "S02_CONTENTION_FUNCTIONAL_OK",
        "writer_ids": [f"contend-{number:02d}" for number in range(8)],
        "reader_cycles": 8,
        "overlap_cycles": 3,
        "reader_errors": [],
        "writer_errors": [],
        "readiness": "ready",
        "evidence": {
            "logical_id": "s02-claim",
            "source_body": '{"summary": "s02 canonical source evidence bytes"}',
        },
        "erasure": {"graph_nodes": 3, "graph_edges": 1, "contended_nodes": 8},
        "before_reopen": {
            "corpus_nodes": 32, "graph_nodes": 0,
            "graph_edges": 0, "contended_nodes": 0,
        },
        "after_reopen": {
            "corpus_nodes": 32, "graph_nodes": 0,
            "graph_edges": 0, "contended_nodes": 0,
        },
        "integrity_check": "ok",
        "bounded_seconds": 60,
        "elapsed_ns": 1000,
    }


def test_observed_contention_requires_overlap_and_reopened_state() -> None:
    contention.validate_observed(observed())
    changed = deepcopy(observed())
    changed["overlap_cycles"] = 0
    with pytest.raises(ValueError, match="overlap"):
        contention.validate_observed(changed)
    changed = deepcopy(observed())
    changed["after_reopen"]["contended_nodes"] = 1
    with pytest.raises(ValueError, match="reopen"):
        contention.validate_observed(changed)


def test_contention_error_cannot_be_labeled_success() -> None:
    changed = deepcopy(observed())
    changed["reader_errors"] = ["search failed"]
    with pytest.raises(ValueError, match="reader"):
        contention.validate_observed(changed)
    changed = deepcopy(observed())
    changed["evidence"]["logical_id"] = "wrong"
    with pytest.raises(ValueError, match="evidence"):
        contention.validate_observed(changed)
