"""Reject source and artifact drift in paired TypeScript S02 raw evidence."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_ts_s02_pair_audit as audit  # noqa: E402


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
        "graph_nodes": 0, "graph_edges": 0, "corpus_nodes": 32,
    }
    return raw


def test_pair_raw_binding_rejects_source_and_native_drift() -> None:
    raw = sample()
    expected = {
        "source_sha": raw["source_sha"],
        "timed_runner_sha256": raw["runner_sha256"],
        "s01_helper_sha256": raw["s01_helper_sha256"],
        "corpus_sha256": raw["corpus_sha256"],
        "graph_records_sha256": raw["graph_records_sha256"],
        "node_version": raw["artifact"]["node_version"],
        "module_sha256": raw["artifact"]["module_sha256"],
        "package_sha256": raw["artifact"]["package_sha256"],
        "native_sha256": raw["artifact"]["native_sha256"],
    }
    assert audit.validate_raw_binding(raw, expected) == raw["whole_product_ns"]
    altered = deepcopy(raw)
    altered["source_sha"] = "0" * 40
    with pytest.raises(ValueError, match="source"):
        audit.validate_raw_binding(altered, expected)
    altered = deepcopy(raw)
    altered["artifact"]["native_sha256"] = "0" * 64
    with pytest.raises(ValueError, match="native"):
        audit.validate_raw_binding(altered, expected)


def test_pair_delta_uses_version_order_and_rejects_zero_baseline() -> None:
    assert audit.pair_delta_pct(100, 105) == 5.0
    with pytest.raises(ValueError, match="baseline"):
        audit.pair_delta_pct(0, 105)
