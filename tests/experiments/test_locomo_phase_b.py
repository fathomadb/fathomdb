"""Human-intended safety and lifecycle tests for the LOCOMO/PARENT executor."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments import locomo_phase_b


CONFIG_PATH = Path("experiments/configs/locomo-01/phase-b-execution.v1.json")


def _document() -> dict[str, object]:
    return json.loads(CONFIG_PATH.read_text(encoding="utf-8"))


def _plan() -> locomo_phase_b.LocomoPhaseBPlan:
    return locomo_phase_b.load_config(_document())


def _release(plan: locomo_phase_b.LocomoPhaseBPlan) -> dict[str, object]:
    return {
        "schema_version": "locomo-phase-b.release.v1",
        "release_id": "locomo-01-parent-01-reviewed-release",
        "config_sha256": plan.config_sha256,
        "issued_by": "track-runner-coordinator",
        "independent_review_sha256": "a" * 64,
        "authorizations": ["seq-249", "seq-250"],
    }


def _result(cell: locomo_phase_b.GridCell, *, mode: str) -> locomo_phase_b.CellExecutionResult:
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
    if cell.program_track == "PARENT-01":
        metrics["parent_metrics"] = {
            "child_evidence_recall": 1.0,
            "parent_session_recall": 1.0,
            "duplicate_rate": 0.0,
            "context_expansion_count": 0,
            "class_latency_ms": {"factoid": 1.0, "temporal": 1.0, "multi_session": 1.0},
        }
    return locomo_phase_b.CellExecutionResult(
        cell_id=cell.cell_id,
        mode=mode,
        external_metrics_ref=f"locomo-phase-b-{cell.cell_id}.v1",
        external_metrics_sha256="c" * 64,
        metric_summary=metrics,
    )


def test_phase_b_freezes_authorized_grid_parent_treatment_and_metric_families():
    plan = _plan()

    assert plan.program_track == "LOCOMO-01"
    assert len(plan.cells) == 52
    assert len({cell.cell_id for cell in plan.cells}) == 52
    assert {cell.program_track for cell in plan.cells} == {"LOCOMO-01", "PARENT-01"}
    assert sum(cell.program_track == "PARENT-01" for cell in plan.cells) == 4
    assert plan.dry_run_cell_ids == (
        "turn--fts_only--cpu--cold",
        "turn--hybrid--cpu--steady",
        "session--fts_only--cpu--cold",
        "session--hybrid--cpu--steady",
        "turn--parent_child_turn_session_v1--cpu--cold",
    )
    assert plan.metric_families == {
        "m1": "r_at_10_paired_one_sided_95",
        "m2": ["mrr", "r_at_1", "ndcg_at_10"],
        "m4_proxy": "temporal_evidence_recall",
        "m6": ["facade_query_ms", "engine_query_ms"],
        "m7": ["ingest_ack_ms", "ready_to_search_ms"],
        "class_metrics": ["factoid", "temporal", "multi_session"],
    }

    parent = next(cell for cell in plan.cells if cell.program_track == "PARENT-01")
    assert parent.ingest_unit == "turn"
    assert parent.retrieval == "hybrid"
    assert parent.runtime == {"device": "cpu", "cache_state": "cold"}
    assert parent.parent_child == {
        "version": "parent_child_turn_session_v1",
        "parent_relation": "exact_enclosing_session",
        "child_top_k": 10,
        "parent_bundle_limit": 5,
        "parent_rank": "best_child_original_rank_then_parent_session_id",
        "fusion": "none",
        "neighbors_each_side": 1,
        "max_turns_per_bundle": 3,
        "max_turns_total": 15,
        "safe_attribution": ["parent_session_id", "seed_child_id", "ordered_neighbor_ids", "trace_source_id"],
    }


@pytest.mark.parametrize(
    "mutate",
    [
        lambda document: document["execution"].__setitem__("default_mode", "execute"),
        lambda document: document["external_inputs"]["corpus"].__setitem__("storage", "repository"),
        lambda document: document["cells"].pop(),
        lambda document: document["cells"].__setitem__(0, {**document["cells"][0], "program_track": "PARENT-01"}),
        lambda document: next(cell for cell in document["cells"] if cell["program_track"] == "PARENT-01")["parent_child"].__setitem__("fusion", "rrf"),
        lambda document: document["receipt"]["historical_output_paths"].append("experiments/runs/old/record.json"),
    ],
)
def test_phase_b_fails_closed_on_unsafe_or_semantically_drifting_config(mutate):
    document = _document()
    mutate(document)

    with pytest.raises(locomo_phase_b.LocomoPhaseBError):
        locomo_phase_b.load_config(document)


def test_preview_is_content_free_and_execution_requires_independent_coordinator_release(tmp_path):
    plan = _plan()
    preview = locomo_phase_b.preview(plan)

    assert preview["schema_version"] == "locomo-phase-b.preview.v1"
    assert preview["execution"] == {"default_mode": "preview_only", "release_required": True}
    assert preview["cell_count"] == 52
    assert str(Path.cwd()) not in json.dumps(preview)

    called = False

    def executor(request: locomo_phase_b.CellExecutionRequest) -> locomo_phase_b.CellExecutionResult:
        nonlocal called
        called = True
        return _result(request.cell, mode=request.mode)

    with pytest.raises(locomo_phase_b.LocomoPhaseBError, match="release"):
        locomo_phase_b.execute(plan, release=None, external_root=tmp_path, executor=executor)
    assert not called

    with pytest.raises(locomo_phase_b.LocomoPhaseBError, match="outside the repository"):
        locomo_phase_b.execute(plan, release=_release(plan), external_root=Path.cwd(), executor=executor)
    assert not called


def test_parent_bundle_selection_is_rank_preserving_bounded_and_fails_closed():
    bundles = locomo_phase_b.parent_child_bundles(
        [
            {"child_id": "turn-2", "rank": 2, "child_provenance": {"parent_session_ids": ["session-b"], "ordinal": 2, "trace_source_id": "source-b"}, "neighbors": [{"id": "turn-1", "parent_session_id": "session-b", "ordinal": 1, "trace_source_id": "source-b"}, {"id": "turn-3", "parent_session_id": "session-b", "ordinal": 3, "trace_source_id": "source-b"}]},
            {"child_id": "turn-4", "rank": 3, "child_provenance": {"parent_session_ids": ["session-b"], "ordinal": 4, "trace_source_id": "source-b"}, "neighbors": [{"id": "turn-3", "parent_session_id": "session-b", "ordinal": 3, "trace_source_id": "source-b"}, {"id": "turn-5", "parent_session_id": "session-b", "ordinal": 5, "trace_source_id": "source-b"}]},
            {"child_id": "turn-9", "rank": 2, "child_provenance": {"parent_session_ids": ["session-a"], "ordinal": 9, "trace_source_id": "source-a"}, "neighbors": []},
        ]
    )

    assert bundles == [
        {"parent_session_id": "session-a", "seed_child_id": "turn-9", "ordered_neighbor_ids": [], "trace_source_id": "source-a"},
        {"parent_session_id": "session-b", "seed_child_id": "turn-2", "ordered_neighbor_ids": ["turn-1", "turn-3"], "trace_source_id": "source-b"},
    ]

    with pytest.raises(locomo_phase_b.LocomoPhaseBError, match="cross-session"):
        locomo_phase_b.parent_child_bundles([
            {"child_id": "turn-2", "rank": 1,
             "child_provenance": {"parent_session_ids": ["session-b"], "ordinal": 2, "trace_source_id": "source-b"},
             "neighbors": [{"id": "turn-x", "parent_session_id": "session-a", "ordinal": 1, "trace_source_id": "source-b"}]},
        ])


def test_safe_receipt_uses_common_record_index_contract_and_never_exposes_external_root(tmp_path):
    plan = _plan()
    external_root = tmp_path / "external-results"
    external_root.mkdir()
    receipt = locomo_phase_b.write_safe_receipt(
        plan,
        release=_release(plan),
        base_dir=tmp_path / "experiments",
        external_root=external_root,
        status="dry_run_proof",
        result_refs=[
            _result(next(cell for cell in plan.cells if cell.cell_id == cell_id), mode="dry_run")
            for cell_id in plan.dry_run_cell_ids
        ],
    )

    record = json.loads((receipt.run_dir / "record.json").read_text(encoding="utf-8"))
    index = json.loads((tmp_path / "experiments" / "index.jsonl").read_text(encoding="utf-8"))
    assert record["schema_version"] == "experiments.record.v1"
    assert index["schema_version"] == "experiments.index-row.v1"
    assert record["config"]["resolved"]["program_track"] == "LOCOMO-01"
    assert len(record["artifacts"]) == 5
    assert all(artifact["sha256"] == "c" * 64 for artifact in record["artifacts"])
    serialized = json.dumps(record)
    assert str(external_root) not in serialized
    assert "turn-2" not in serialized
