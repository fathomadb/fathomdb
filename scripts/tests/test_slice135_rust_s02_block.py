"""Reject an incomplete Rust SDK S02 timing receipt before sampling."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_rust_s02_block as block  # noqa: E402


def valid_receipt() -> dict:
    return {
        "schema_version": 1,
        "status": "TIMED_RUST_SDK_S02_FUNCTIONAL_OK",
        "source_sha": "3f29d649d0213e595c0dab251a449d92fd625792",
        "whole_product_ns": 200,
        "stage_ns": {name: 1 for name in block.STAGES},
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


def test_parse_receipt_accepts_complete_product_timer() -> None:
    raw = valid_receipt()
    assert block.parse_receipt("SLICE135_RUST_S02 " + json.dumps(raw) + "\n") == raw


def test_parse_receipt_rejects_false_reopened_state_and_missing_stage() -> None:
    raw = valid_receipt()
    changed = deepcopy(raw)
    changed["observed"]["reopened_graph_absent"] = False
    with pytest.raises(ValueError, match="observed state"):
        block.parse_receipt("SLICE135_RUST_S02 " + json.dumps(changed) + "\n")
    changed = deepcopy(raw)
    del changed["stage_ns"]["reopen"]
    with pytest.raises(ValueError, match="stage"):
        block.parse_receipt("SLICE135_RUST_S02 " + json.dumps(changed) + "\n")
