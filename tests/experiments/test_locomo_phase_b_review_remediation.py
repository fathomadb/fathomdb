"""Independent-review regression tests for the LOCOMO/PARENT executor."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments import locomo_phase_b


CONFIG_PATH = Path("experiments/configs/locomo-01/phase-b-execution.v1.json")


def _plan() -> locomo_phase_b.LocomoPhaseBPlan:
    return locomo_phase_b.load_config(json.loads(CONFIG_PATH.read_text(encoding="utf-8")))


def _release(plan: locomo_phase_b.LocomoPhaseBPlan) -> dict[str, object]:
    return {
        "schema_version": "locomo-phase-b.release.v1",
        "release_id": "locomo-01-parent-01-reviewed-release",
        "config_sha256": plan.config_sha256,
        "issued_by": "track-runner-coordinator",
        "independent_review_sha256": "a" * 64,
        "authorizations": ["seq-249", "seq-250"],
    }


def _metrics(*, parent: bool) -> dict[str, object]:
    metrics: dict[str, object] = {
        "m1": {"r_at_10": 1.0},
        "m2": {"mrr": 1.0, "r_at_1": 1.0, "ndcg_at_10": 1.0},
        "m4_proxy": {"temporal_evidence_recall": 1.0},
        "m6": {"facade_query_ms": 1.0, "engine_query_ms": 1.0},
        "m7": {"ingest_ack_ms": 1.0, "ready_to_search_ms": 1.0},
        "class_metrics": {
            "factoid": {"r_at_10": 1.0},
            "temporal": {"r_at_10": 1.0},
            "multi_session": {"r_at_10": 1.0},
        },
    }
    if parent:
        metrics["parent_metrics"] = {
            "child_evidence_recall": 1.0,
            "parent_session_recall": 1.0,
            "duplicate_rate": 0.0,
            "context_expansion_count": 0,
            "class_latency_ms": {"factoid": 1.0, "temporal": 1.0, "multi_session": 1.0},
        }
    return metrics


def _result(cell: locomo_phase_b.GridCell, *, mode: str) -> locomo_phase_b.CellExecutionResult:
    return locomo_phase_b.CellExecutionResult(
        cell_id=cell.cell_id,
        mode=mode,
        external_metrics_ref=f"locomo-phase-b-{cell.cell_id}.v1",
        external_metrics_sha256="b" * 64,
        metric_summary=_metrics(parent=cell.program_track == "PARENT-01"),
    )


def test_execute_dispatches_the_fixed_dry_run_subset_in_frozen_listed_order(tmp_path):
    plan = _plan()
    observed: list[str] = []

    def executor(request: locomo_phase_b.CellExecutionRequest) -> locomo_phase_b.CellExecutionResult:
        observed.append(request.cell.cell_id)
        return _result(request.cell, mode=request.mode)

    results = locomo_phase_b.execute(
        plan, release=_release(plan), external_root=tmp_path, executor=executor, mode="dry_run"
    )

    assert observed == list(plan.dry_run_cell_ids)
    assert [result.cell_id for result in results] == list(plan.dry_run_cell_ids)
    assert {result.mode for result in results} == {"dry_run"}


def test_complete_receipt_fails_closed_on_result_identity_mode_coverage_or_metric_gaps(tmp_path):
    plan = _plan()
    external_root = tmp_path / "external"
    external_root.mkdir()
    complete = [_result(cell, mode="full_grid") for cell in plan.cells]

    missing = complete[:-1]
    with pytest.raises(locomo_phase_b.LocomoPhaseBError, match="coverage"):
        locomo_phase_b.write_safe_receipt(
            plan, release=_release(plan), base_dir=tmp_path / "experiments", external_root=external_root,
            status="complete", result_refs=missing,
        )

    duplicate = complete.copy()
    duplicate[-1] = duplicate[0]
    with pytest.raises(locomo_phase_b.LocomoPhaseBError, match="unique"):
        locomo_phase_b.write_safe_receipt(
            plan, release=_release(plan), base_dir=tmp_path / "experiments", external_root=external_root,
            status="complete", result_refs=duplicate,
        )

    wrong_mode = complete.copy()
    wrong_mode[0] = _result(plan.cells[0], mode="dry_run")
    with pytest.raises(locomo_phase_b.LocomoPhaseBError, match="mode"):
        locomo_phase_b.write_safe_receipt(
            plan, release=_release(plan), base_dir=tmp_path / "experiments", external_root=external_root,
            status="complete", result_refs=wrong_mode,
        )

    incomplete_metrics = complete.copy()
    incomplete_metrics[-1] = locomo_phase_b.CellExecutionResult(
        cell_id=plan.cells[-1].cell_id,
        mode="full_grid",
        external_metrics_ref="locomo-phase-b-parent-metrics.v1",
        external_metrics_sha256="c" * 64,
        metric_summary={key: value for key, value in _metrics(parent=True).items() if key != "parent_metrics"},
    )
    with pytest.raises(locomo_phase_b.LocomoPhaseBError, match="parent metrics"):
        locomo_phase_b.write_safe_receipt(
            plan, release=_release(plan), base_dir=tmp_path / "experiments", external_root=external_root,
            status="complete", result_refs=incomplete_metrics,
        )


def test_parent_mapping_requires_one_provenance_backed_parent_and_immediate_trace_attributed_neighbors():
    hit = {
        "child_id": "turn-2",
        "rank": 2,
        "child_provenance": {
            "parent_session_ids": ["session-b"],
            "ordinal": 2,
            "trace_source_id": "source-b",
        },
        "neighbors": [
            {"id": "turn-1", "parent_session_id": "session-b", "ordinal": 1, "trace_source_id": "source-b"},
            {"id": "turn-3", "parent_session_id": "session-b", "ordinal": 3, "trace_source_id": "source-b"},
        ],
    }

    assert locomo_phase_b.parent_child_bundles([hit]) == [{
        "parent_session_id": "session-b",
        "seed_child_id": "turn-2",
        "ordered_neighbor_ids": ["turn-1", "turn-3"],
        "trace_source_id": "source-b",
    }]

    for mutate, message in (
        (lambda value: value["child_provenance"].__setitem__("parent_session_ids", ["session-a", "session-b"]), "exactly one"),
        (lambda value: value["neighbors"].__setitem__(0, "turn-1"), "compact"),
        (lambda value: value["neighbors"][0].__setitem__("ordinal", 0), "immediately"),
        (lambda value: value["neighbors"][0].__setitem__("parent_session_id", "session-a"), "cross-session"),
        (lambda value: value["neighbors"][0].pop("trace_source_id"), "attribution"),
    ):
        candidate = json.loads(json.dumps(hit))
        mutate(candidate)
        with pytest.raises(locomo_phase_b.LocomoPhaseBError, match=message):
            locomo_phase_b.parent_child_bundles([candidate])


def test_receipt_verdict_and_n_are_honest_for_complete_dry_and_full_execution(tmp_path):
    plan = _plan()
    external_root = tmp_path / "external"
    external_root.mkdir()
    dry_cells = [next(cell for cell in plan.cells if cell.cell_id == cell_id) for cell_id in plan.dry_run_cell_ids]

    dry = locomo_phase_b.write_safe_receipt(
        plan, release=_release(plan), base_dir=tmp_path / "dry", external_root=external_root,
        status="dry_run_proof", result_refs=[_result(cell, mode="dry_run") for cell in dry_cells],
    )
    full = locomo_phase_b.write_safe_receipt(
        plan, release=_release(plan), base_dir=tmp_path / "full", external_root=external_root,
        status="complete", result_refs=[_result(cell, mode="full_grid") for cell in plan.cells],
    )

    dry_record = json.loads((dry.run_dir / "record.json").read_text(encoding="utf-8"))
    full_record = json.loads((full.run_dir / "record.json").read_text(encoding="utf-8"))
    assert dry_record["verdict"] == "complete"
    assert dry_record["metrics"]["status"] == "dry_run_proof"
    assert dry_record["metrics"]["result_count"] == 5
    assert full_record["verdict"] == "complete"
    assert full_record["metrics"]["status"] == "complete"
    assert full_record["metrics"]["result_count"] == 52
