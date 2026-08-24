"""Tests for the safe paired-arm LOCOMO comparison receipt."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

import pytest

from experiments import mem0_comparison


TS = datetime(2026, 8, 13, 12, 30, tzinfo=timezone.utc)


def _record(run_id: str, experiment: str, verdict: str = "complete") -> dict:
    program_track = "MEMORY-01"
    return {
        "schema_version": "experiments.record.v1", "run_id": run_id, "experiment": experiment,
        "title": experiment, "verdict": verdict, "read": "ok",
        "code": {"git_sha": "abc", "dirty": False, "branch": "b", "baseline_commit": None},
        "config": {"sha256": "a" * 64, "path": None, "resolved": {
            "program_track": program_track,
            "harness": {"git_sha": "abc123"},
            "benchmark": {"top_k": 10, "top_k_cutoffs": [10], "conversations": "0", "categories": "1", "predict_only": True, "resume": True, "max_workers": 1, "rpm": 1},
            "corpus": {"raw_sha256": "b" * 64, "normalized_sha256": "c" * 64, "sessions": 1, "eligible_questions": 1},
        }},
        "corpus": {"source": "LOCOMO", "manifest_sha256": "b" * 64, "datasets": []}, "seeds": {},
        "env": {"python": "3.12", "lockfile_sha256": None, "gpu": None, "key_deps": {}},
        "metrics": {"completion": {"complete": verdict == "complete"}}, "cost_usd": None,
        "tdd_evidence": {}, "tests": None, "files_changed": [], "artifacts": [], "review": None, "open_questions": [],
    }


def _write_scores(root: Path, scores: list[tuple[str, int, float]]) -> None:
    root.mkdir(parents=True, exist_ok=True)
    for question_id, category, score in scores:
        (root / f"{question_id}.json").write_text(json.dumps({
            "question_id": question_id,
            "category": category,
            "cutoff_results": {"top_10": {"score": score}},
        }))


def test_comparison_rejects_incomplete_or_mismatched_arm_receipts(tmp_path):
    native = _record("native", "mem0-oss-locomo-native")
    fathom = _record("fathom", "fathomdb-locomo-official-seam", verdict="incomplete")
    with pytest.raises(ValueError, match="complete"):
        mem0_comparison.write_receipt(native, fathom, ts=TS, base_dir=tmp_path)

    fathom = _record("fathom", "fathomdb-locomo-official-seam")
    fathom["config"]["resolved"]["benchmark"]["top_k"] = 9
    with pytest.raises(ValueError, match="top_k"):
        mem0_comparison.write_receipt(native, fathom, ts=TS, base_dir=tmp_path)

    native = _record("native", "mem0-oss-locomo-native")
    native["config"]["resolved"]["program_track"] = "LOCOMO-01"
    with pytest.raises(ValueError, match="program_track"):
        mem0_comparison.write_receipt(native, fathom, ts=TS, base_dir=tmp_path)


def test_comparison_writes_generic_receipt_and_typed_result(tmp_path):
    native = _record("native", "mem0-oss-locomo-native")
    fathom = _record("fathom", "fathomdb-locomo-official-seam")
    fathom["config"]["resolved"]["benchmark"]["max_workers"] = 10
    fathom["config"]["resolved"]["benchmark"]["rpm"] = 600
    run_id, run_dir = mem0_comparison.write_receipt(
        native, fathom, ts=TS, base_dir=tmp_path,
    )

    result = json.loads((run_dir / "mem0-comparison.result.v1.json").read_text())
    assert result["schema_version"] == "mem0-comparison.result.v1"
    assert result["arms"] == {"fathomdb": "fathom", "mem0_oss": "native"}
    assert result["execution_controls"] == {
        "fathomdb": {"max_workers": 10, "rpm": 600},
        "mem0_oss": {"max_workers": 1, "rpm": 1},
    }
    record = json.loads((run_dir / "record.json").read_text())
    assert record["experiment"] == "fathomdb-vs-mem0-locomo-comparison"
    assert record["run_id"] == run_id
    assert record["config"]["resolved"]["program_track"] == "MEMORY-01"


def test_scored_comparison_writes_safe_paired_metrics(tmp_path):
    native = _record("native", "mem0-oss-locomo-native")
    fathom = _record("fathom", "fathomdb-locomo-official-seam")
    native["config"]["resolved"]["corpus"]["eligible_questions"] = 4
    fathom["config"]["resolved"]["corpus"]["eligible_questions"] = 4
    native["config"]["resolved"]["benchmark"]["categories"] = "1,2"
    fathom["config"]["resolved"]["benchmark"]["categories"] = "1,2"
    native_output = tmp_path / "native"
    fathom_output = tmp_path / "fathom"
    _write_scores(native_output, [
        ("conv0_q0", 1, 0.0), ("conv0_q1", 1, 0.0),
        ("conv0_q2", 2, 0.0), ("conv0_q3", 2, 0.0),
    ])
    _write_scores(fathom_output, [
        ("conv0_q0", 1, 1.0), ("conv0_q1", 1, 1.0),
        ("conv0_q2", 2, 1.0), ("conv0_q3", 2, 1.0),
    ])

    run_id, run_dir = mem0_comparison.write_scored_receipt(
        native, fathom,
        native_output_dir=native_output,
        fathom_output_dir=fathom_output,
        ts=TS,
        base_dir=tmp_path,
        bootstrap_seed=7,
        bootstrap_resamples=100,
    )

    result = json.loads((run_dir / "mem0-comparison.result.v1.json").read_text())
    assert result["run_id"] == run_id
    assert result["verdict"] == "pass"
    assert result["paired"]["overall"] == {
        "questions": 4,
        "mem0_oss_accuracy": 0.0,
        "fathomdb_accuracy": 1.0,
        "fathomdb_minus_mem0": 1.0,
    }
    assert result["paired"]["by_category"]["1"]["questions"] == 2
    assert result["paired"]["one_sided_95_lower_bound"] == 1.0
    assert result["paired"]["bootstrap"] == {"resamples": 100, "seed": 7}
    assert set(result["score_manifests"]) == {"fathomdb", "mem0_oss"}
    assert "question_id" not in json.dumps(result).lower()
    assert "generated_answer" not in json.dumps(result).lower()
    record = json.loads((run_dir / "record.json").read_text())
    assert record["metrics"]["phase"] == "paired_answer_scoring"
    assert record["verdict"] == "pass"


def test_scored_comparison_rejects_incomplete_or_mismatched_outputs(tmp_path):
    native = _record("native", "mem0-oss-locomo-native")
    fathom = _record("fathom", "fathomdb-locomo-official-seam")
    native["config"]["resolved"]["corpus"]["eligible_questions"] = 2
    fathom["config"]["resolved"]["corpus"]["eligible_questions"] = 2
    native["config"]["resolved"]["benchmark"]["categories"] = "1,2"
    fathom["config"]["resolved"]["benchmark"]["categories"] = "1,2"
    native_output = tmp_path / "native"
    fathom_output = tmp_path / "fathom"
    _write_scores(native_output, [("conv0_q0", 1, 0.0), ("conv0_q1", 1, 1.0)])
    _write_scores(fathom_output, [("conv0_q0", 1, 1.0)])
    with pytest.raises(ValueError, match="expected 2 scored questions"):
        mem0_comparison.write_scored_receipt(
            native, fathom,
            native_output_dir=native_output,
            fathom_output_dir=fathom_output,
            ts=TS,
            base_dir=tmp_path,
        )

    _write_scores(fathom_output, [("conv0_q1", 2, 1.0)])
    with pytest.raises(ValueError, match="category mismatch"):
        mem0_comparison.write_scored_receipt(
            native, fathom,
            native_output_dir=native_output,
            fathom_output_dir=fathom_output,
            ts=TS,
            base_dir=tmp_path,
        )


def test_scored_comparison_records_failed_near_parity_rule(tmp_path):
    native = _record("native", "mem0-oss-locomo-native")
    fathom = _record("fathom", "fathomdb-locomo-official-seam")
    native["config"]["resolved"]["corpus"]["eligible_questions"] = 2
    fathom["config"]["resolved"]["corpus"]["eligible_questions"] = 2
    native_output = tmp_path / "native"
    fathom_output = tmp_path / "fathom"
    _write_scores(native_output, [("conv0_q0", 1, 1.0), ("conv0_q1", 1, 1.0)])
    _write_scores(fathom_output, [("conv0_q0", 1, 0.0), ("conv0_q1", 1, 0.0)])

    _, run_dir = mem0_comparison.write_scored_receipt(
        native, fathom,
        native_output_dir=native_output,
        fathom_output_dir=fathom_output,
        ts=TS,
        base_dir=tmp_path,
        bootstrap_seed=7,
        bootstrap_resamples=100,
    )

    result = json.loads((run_dir / "mem0-comparison.result.v1.json").read_text())
    assert result["verdict"] == "fail"
    assert result["paired"]["one_sided_95_lower_bound"] == -1.0
