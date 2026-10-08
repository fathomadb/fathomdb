"""Prove the independent S02 auditor rejects corrupted timed receipts."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_ts_s02_audit as audit  # noqa: E402


ROOT = Path(__file__).resolve().parents[2]
FEASIBILITY = (
    ROOT
    / "dev/plans/0.8.27/features/slice-135/results/2026-10-07-ts-s02-feasibility/candidate.json"
)


def sample() -> dict:
    raw = json.loads(FEASIBILITY.read_text())
    for phase in ("after_erasure", "after_reopen"):
        del raw["observed"][phase]["canonical_counts"]
    raw["status"] = "UNFROZEN_TS_S02_PRODUCT_TIMING_FEASIBILITY"
    raw["whole_product_ns"] = raw.pop("whole_sequence_including_checks_ns")
    raw["verification_ns"] = 1_000_000
    raw["final_canonical_counts"] = {
        "graph_nodes": 0,
        "graph_edges": 0,
        "corpus_nodes": 32,
    }
    return raw


def test_raw_auditor_accepts_materialized_state_then_rejects_tampering() -> None:
    raw = sample()
    assert audit.audit_raw(raw) == raw["whole_product_ns"]
    altered = deepcopy(raw)
    altered["final_canonical_counts"]["graph_edges"] = 1
    with pytest.raises(ValueError, match="canonical"):
        audit.audit_raw(altered)
    altered = deepcopy(raw)
    altered["observed"]["graph"]["edge_revision"] = "wrong"
    with pytest.raises(ValueError, match="graph"):
        audit.audit_raw(altered)


def test_raw_auditor_rejects_interleaved_sqlite_and_timer_shortening() -> None:
    raw = sample()
    altered = deepcopy(raw)
    altered["observed"]["after_erasure"]["canonical_counts"] = raw[
        "final_canonical_counts"
    ]
    with pytest.raises(ValueError, match="interleaved"):
        audit.audit_raw(altered)
    altered = deepcopy(raw)
    altered["whole_product_ns"] = 1
    with pytest.raises(ValueError, match="stage sum"):
        audit.audit_raw(altered)


def test_archived_command_keeps_block_identity_after_relocation() -> None:
    command = ["node", "runner.mjs", "--timing-mode", "product", "--output",
               "/original/archive/block-01/sample-001.json"]
    audit.check_command(command, "block-01", "sample-001")
    bad = command[:-1] + ["/original/archive/block-02/sample-001.json"]
    with pytest.raises(ValueError, match="command"):
        audit.check_command(bad, "block-01", "sample-001")
