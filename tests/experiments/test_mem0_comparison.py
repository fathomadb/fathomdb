"""Tests for the safe paired-arm LOCOMO comparison receipt."""

from __future__ import annotations

import json
from datetime import datetime, timezone

import pytest

from experiments import mem0_comparison


TS = datetime(2026, 8, 13, 12, 30, tzinfo=timezone.utc)


def _record(run_id: str, experiment: str, verdict: str = "complete") -> dict:
    return {
        "schema_version": "experiments.record.v1", "run_id": run_id, "experiment": experiment,
        "title": experiment, "verdict": verdict, "read": "ok",
        "code": {"git_sha": "abc", "dirty": False, "branch": "b", "baseline_commit": None},
        "config": {"sha256": "a" * 64, "path": None, "resolved": {
            "harness": {"git_sha": "abc123"},
            "benchmark": {"top_k": 10, "top_k_cutoffs": [10], "conversations": "0", "categories": "1", "predict_only": True, "resume": True, "max_workers": 1, "rpm": 1},
            "corpus": {"raw_sha256": "b" * 64, "normalized_sha256": "c" * 64, "sessions": 1, "eligible_questions": 1},
        }},
        "corpus": {"source": "LOCOMO", "manifest_sha256": "b" * 64, "datasets": []}, "seeds": {},
        "env": {"python": "3.12", "lockfile_sha256": None, "gpu": None, "key_deps": {}},
        "metrics": {"completion": {"complete": verdict == "complete"}}, "cost_usd": None,
        "tdd_evidence": {}, "tests": None, "files_changed": [], "artifacts": [], "review": None, "open_questions": [],
    }


def test_comparison_rejects_incomplete_or_mismatched_arm_receipts(tmp_path):
    native = _record("native", "mem0-oss-locomo-native")
    fathom = _record("fathom", "fathomdb-locomo-official-seam", verdict="incomplete")
    with pytest.raises(ValueError, match="complete"):
        mem0_comparison.write_receipt(native, fathom, ts=TS, base_dir=tmp_path)

    fathom = _record("fathom", "fathomdb-locomo-official-seam")
    fathom["config"]["resolved"]["benchmark"]["top_k"] = 9
    with pytest.raises(ValueError, match="top_k"):
        mem0_comparison.write_receipt(native, fathom, ts=TS, base_dir=tmp_path)


def test_comparison_writes_generic_receipt_and_typed_result(tmp_path):
    run_id, run_dir = mem0_comparison.write_receipt(
        _record("native", "mem0-oss-locomo-native"),
        _record("fathom", "fathomdb-locomo-official-seam"), ts=TS, base_dir=tmp_path,
    )

    result = json.loads((run_dir / "mem0-comparison.result.v1.json").read_text())
    assert result["schema_version"] == "mem0-comparison.result.v1"
    assert result["arms"] == {"fathomdb": "fathom", "mem0_oss": "native"}
    record = json.loads((run_dir / "record.json").read_text())
    assert record["experiment"] == "fathomdb-vs-mem0-locomo-comparison"
    assert record["run_id"] == run_id
