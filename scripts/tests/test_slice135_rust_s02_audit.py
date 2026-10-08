"""Independently reject altered Rust SDK S02 timing and state observations."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_rust_s02_audit as audit  # noqa: E402


def raw() -> dict:
    return {
        "schema_version": 1,
        "status": "TIMED_RUST_SDK_S02_FUNCTIONAL_OK",
        "source_sha": "3f29d649d0213e595c0dab251a449d92fd625792",
        "whole_product_ns": 200,
        "stage_ns": {name: 1 for name in audit.STAGES},
        "observed": {
            "corpus_nodes": 32,
            "erased_graph_nodes": 3,
            "erased_graph_edges": 1,
            "reopened_graph_absent": True,
            "reopened_anchor_retained": True,
            "evidence_resolved": True,
            "projection_ready": True,
        },
    }


def test_auditor_accepts_complete_source_bound_receipt() -> None:
    assert audit.validate_raw(raw()) == 200


def test_auditor_rejects_missing_graph_evidence_and_short_whole_timer() -> None:
    changed = deepcopy(raw())
    changed["observed"]["evidence_resolved"] = False
    with pytest.raises(ValueError, match="evidence"):
        audit.validate_raw(changed)
    changed = deepcopy(raw())
    changed["whole_product_ns"] = 1
    with pytest.raises(ValueError, match="whole"):
        audit.validate_raw(changed)


def test_auditor_rejects_bool_integer_coercion() -> None:
    changed = deepcopy(raw())
    changed["observed"]["evidence_resolved"] = 1
    with pytest.raises(ValueError, match="evidence"):
        audit.validate_raw(changed)
