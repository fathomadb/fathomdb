"""Independent-review regressions for the LOCOMO/PARENT live executor.

All fixtures are synthetic and content-free.  They exercise future release
validation only; no corpus, GPU, adapter, or campaign artifact is used.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from experiments import locomo_live_executor, trace_projection


def _sha(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _sidecar() -> dict[str, object]:
    prior = trace_projection.Source("source-prior", _sha(b"prior"))
    current = trace_projection.Source("source-current", _sha(b"current"))
    return trace_projection.build_trace_projection(
        (prior, current),
        (
            trace_projection.Projection("text-prior", prior.source_id, prior.source_sha256, "text"),
            trace_projection.Projection("text-current", current.source_id, current.source_sha256, "text"),
        ),
        {"warnings": [{
            "kind": "supersedes", "source_doc_id": current.source_id, "prior_body": "prior", "supersedes_hint": "changed",
        }]},
    )


def _manifest(*, entries: list[dict[str, object]]) -> dict[str, object]:
    return {"schema_version": "locomo-provenance.v1", "entries": entries}


def _entry(*, fingerprint: str, session_id: str, turn_ids: list[str]) -> dict[str, object]:
    return {
        "fingerprint": fingerprint, "conversation_id": "locomo-1", "session_id": session_id, "turn_ids": turn_ids,
    }


def _relations(*, turn_sha: str, session_sha: str) -> dict[str, object]:
    return {
        "schema_version": "locomo-parent-relation-proof.v1",
        "turn_provenance_sha256": turn_sha,
        "session_provenance_sha256": session_sha,
        "entries": [{
            "child_id": "turn-2", "parent_session_id": "session-1", "ordinal": 1,
            "trace_source_id": "source-current", "turn_provenance_fingerprint": "a" * 64,
            "session_provenance_fingerprint": "b" * 64,
            "session_members": [
                {"id": "turn-1", "ordinal": 0, "trace_source_id": "source-current"},
                {"id": "turn-2", "ordinal": 1, "trace_source_id": "source-current"},
                {"id": "turn-3", "ordinal": 2, "trace_source_id": "source-current"},
            ],
        }],
    }


def _action_projection(
    plan: locomo_live_executor.LiveExecutorPlan, action_id: str, release: dict[str, object]
) -> dict[str, object]:
    action = plan.action(action_id)
    results = [
        locomo_live_executor.CellProjection(
            cell_id=cell_id, mode=action.mode, external_metrics_ref=f"metrics-{index}",
            external_metrics_sha256=f"{index:064x}",
            metric_summary=locomo_live_executor.synthetic_metric_summary(
                parent=next(cell for cell in plan.cells if cell.cell_id == cell_id).program_track == "PARENT-01"
            ), parent_context=(),
        )
        for index, cell_id in enumerate(action.cell_ids)
    ]
    return locomo_live_executor.action_projection_document(plan, action, release, results)


def test_trace_lifecycle_proof_requires_full_trace_01_schema_and_semantics(tmp_path):
    full = _sidecar()
    full_path = tmp_path / "trace.json"
    full_path.write_text(json.dumps(full), encoding="utf-8")

    assert locomo_live_executor.load_trace_lifecycle_proof(full_path, _sha(full_path.read_bytes())) == {"source-current"}

    minimal = tmp_path / "minimal-trace.json"
    minimal.write_text(json.dumps({
        "schema_version": "trace-projection.v1",
        "sources": [{"source_id": "source-current", "source_sha256": "a" * 64, "lifecycle": "active"}],
    }), encoding="utf-8")
    with pytest.raises(locomo_live_executor.LiveExecutorError, match="TRACE lifecycle proof"):
        locomo_live_executor.load_trace_lifecycle_proof(minimal, _sha(minimal.read_bytes()))


def test_parent_relations_are_cryptographically_and_structurally_bound_to_canonical_manifests(tmp_path):
    turn = _manifest(entries=[_entry(fingerprint="a" * 64, session_id="session-1", turn_ids=["turn-2"])])
    session = _manifest(entries=[_entry(fingerprint="b" * 64, session_id="session-1", turn_ids=["turn-1", "turn-2", "turn-3"])])
    turn_path, session_path, relation_path = tmp_path / "turn.json", tmp_path / "session.json", tmp_path / "relations.json"
    turn_path.write_text(json.dumps(turn), encoding="utf-8")
    session_path.write_text(json.dumps(session), encoding="utf-8")
    relation = _relations(turn_sha=_sha(turn_path.read_bytes()), session_sha=_sha(session_path.read_bytes()))
    relation_path.write_text(json.dumps(relation), encoding="utf-8")

    bindings = locomo_live_executor.load_parent_relation_proof(
        relation_path, _sha(relation_path.read_bytes()), turn_path, _sha(turn_path.read_bytes()),
        session_path, _sha(session_path.read_bytes()), active_trace_source_ids={"source-current"},
    )
    assert bindings["turn-2"]["member_ordinals"] == {0: "turn-1", 1: "turn-2", 2: "turn-3"}

    relation["entries"][0]["parent_session_id"] = "invented-session"
    relation_path.write_text(json.dumps(relation), encoding="utf-8")
    with pytest.raises(locomo_live_executor.LiveExecutorError, match="canonical provenance"):
        locomo_live_executor.load_parent_relation_proof(
            relation_path, _sha(relation_path.read_bytes()), turn_path, _sha(turn_path.read_bytes()),
            session_path, _sha(session_path.read_bytes()), active_trace_source_ids={"source-current"},
        )


def test_full_grid_combiner_requires_same_release_config_and_exactly_52_unique_cells(tmp_path):
    plan = locomo_live_executor.load_config(json.loads(Path("experiments/configs/locomo-01/live-executor.v1.json").read_text()))
    release = {"release_id": "release-1", "release_sha256": "a" * 64, "executor_config_sha256": plan.config_sha256}
    cpu = _action_projection(plan, "cpu_grid", release)
    gpu = _action_projection(plan, "gpu_ce_grid", release)

    combined = locomo_live_executor.combine_full_grid_projections(plan, release, cpu, gpu)
    assert combined["receipt_status"] == "complete"
    assert combined["index_eligible"] is True
    assert combined["result_count"] == 52

    gpu["release_sha256"] = "b" * 64
    with pytest.raises(locomo_live_executor.LiveExecutorError, match="same-release"):
        locomo_live_executor.combine_full_grid_projections(plan, release, cpu, gpu)


def test_gpu_action_requires_actual_selected_cuda_device_and_adapter_attestation(monkeypatch):
    def no_gpu(*_args, **_kwargs):  # noqa: ANN001
        return type("Completed", (), {"returncode": 0, "stdout": "1\n", "stderr": ""})()

    monkeypatch.setattr(locomo_live_executor.subprocess, "run", no_gpu)
    with pytest.raises(locomo_live_executor.LiveExecutorError, match="selected CUDA device"):
        locomo_live_executor.require_cuda_device("cuda:0")

    with pytest.raises(locomo_live_executor.LiveExecutorError, match="attestation"):
        locomo_live_executor.validate_gpu_attestation(
            {"device": "cpu", "cuda_available": True}, selected_device="cuda:0"
        )


def test_adapter_json_rejects_duplicate_keys_before_result_validation():
    with pytest.raises(locomo_live_executor.LiveExecutorError, match="duplicate keys"):
        locomo_live_executor.parse_adapter_json('{"cell_id":"one","cell_id":"two"}')


def test_review_sha_must_resolve_to_accepted_evidence_bound_to_release_and_runner(tmp_path):
    record = {
        "schema_version": "locomo-live-executor.review-evidence.v1",
        "verdict": "accepted",
        "review_git_sha": "a" * 40,
        "release_id": "release-1",
        "release_binding_sha256": "b" * 64,
        "integrated_git_sha": "c" * 40,
        "phase_b_config_sha256": "d" * 64,
        "executor_config_sha256": "e" * 64,
        "runner_sha256": "f" * 64,
    }
    path = tmp_path / "review-evidence.json"
    path.write_text(json.dumps(record), encoding="utf-8")
    release = {
        "release_id": "release-1", "independent_review_git_sha": "a" * 40,
        "integrated_git_sha": "c" * 40, "phase_b_config_sha256": "d" * 64,
        "executor_config_sha256": "e" * 64, "runner_sha256": "f" * 64,
    }

    locomo_live_executor.validate_review_evidence(path, _sha(path.read_bytes()), release, release_binding_sha256="b" * 64)
    record["verdict"] = "request_changes"
    path.write_text(json.dumps(record), encoding="utf-8")
    with pytest.raises(locomo_live_executor.LiveExecutorError, match="accepted"):
        locomo_live_executor.validate_review_evidence(
            path, _sha(path.read_bytes()), release, release_binding_sha256="b" * 64
        )
