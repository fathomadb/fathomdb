"""Lifecycle contract for the future LOCOMO/PARENT external executor.

These tests deliberately exercise only synthetic files and injected adapters.
They must never acquire a corpus, invoke a model, or create a campaign receipt.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

import pytest

from experiments import locomo_live_executor, trace_projection
from experiments.locomo_provenance import canonical_session_id, canonical_turn_id


CONFIG_PATH = Path("experiments/configs/locomo-01/live-executor.v1.json")


def _sha(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _canonical_turn(raw_turn_id: str) -> str:
    return canonical_turn_id("locomo-1", "session-1", raw_turn_id)


def _canonical_session() -> str:
    return canonical_session_id("locomo-1", "session-1")


def _plan() -> locomo_live_executor.LiveExecutorPlan:
    return locomo_live_executor.load_config(json.loads(CONFIG_PATH.read_text(encoding="utf-8")))


def _release(plan: locomo_live_executor.LiveExecutorPlan, *, action: str) -> dict[str, object]:
    """A structurally complete token; paths are intentionally invalid by default."""
    token: dict[str, object] = {
        "schema_version": "locomo-live-executor.release.v1",
        "release_id": "locomo-parent-reviewed-release",
        "issued_by": "track-runner-coordinator",
        "integrated_git_sha": "a" * 40,
        "phase_b_config_sha256": plan.phase_b_config_sha256,
        "executor_config_sha256": plan.config_sha256,
        "runner_sha256": plan.runner_sha256,
        "independent_review_git_sha": "b" * 40,
        "authorizations": ["seq-249", "seq-250"],
        "approved_actions": ["fixed_subset_dry_run"] if action == "fixed_subset_dry_run" else ["cpu_grid", "gpu_ce_grid"],
        "gpu_policy": {
            "cuda_required": action != "fixed_subset_dry_run", "allow_cpu_fallback": False,
            "selected_device": "cuda:0" if action != "fixed_subset_dry_run" else "cpu",
        },
        "external_roots": {
            "artifact_root": {"path": "/external/artifacts", "binding_sha256": "c" * 64},
            "corpus": {"path": "/external/locomo.json", "sha256": plan.external_input_sha256["corpus"]},
            "turn_provenance": {"path": "/external/turn.json", "sha256": plan.external_input_sha256["turn_provenance"]},
            "session_provenance": {"path": "/external/session.json", "sha256": plan.external_input_sha256["session_provenance"]},
            "dry_run_subset": {"path": "/external/subset.json", "sha256": plan.external_input_sha256["dry_run_subset"]},
            "trace_projection": {"path": "/external/trace.json", "sha256": "d" * 64},
            "parent_relation_proof": {"path": "/external/parent-relations.json", "sha256": "f" * 64},
        },
        "cell_adapter": {"path": "/external/locomo-cell-adapter", "sha256": "e" * 64},
        "review_evidence": {"path": "/external/review-evidence.json", "sha256": "f" * 64},
    }
    token["release_sha256"] = locomo_live_executor.release_sha256(token)
    return token


def test_live_executor_freezes_phase_b_and_action_partitions():
    plan = _plan()

    assert plan.program_tracks == ("LOCOMO-01", "PARENT-01")
    assert plan.action("fixed_subset_dry_run").cell_ids == (
        "turn--fts_only--cpu--cold",
        "turn--hybrid--cpu--steady",
        "session--fts_only--cpu--cold",
        "session--hybrid--cpu--steady",
        "turn--parent_child_turn_session_v1--cpu--cold",
    )
    assert len(plan.action("cpu_grid").cell_ids) == 26
    assert len(plan.action("gpu_ce_grid").cell_ids) == 26
    assert set(plan.action("cpu_grid").cell_ids).isdisjoint(plan.action("gpu_ce_grid").cell_ids)


def test_live_executor_pins_raw_locomo_against_its_normalized_corpus_identity(monkeypatch, tmp_path):
    """The released raw file is compared using the Phase-B canonical corpus identity."""
    raw_corpus = tmp_path / "locomo.json"
    raw_corpus.write_text('{"payload":"raw-bytes-differ"}', encoding="utf-8")
    monkeypatch.setattr(
        locomo_live_executor, "_normalized_corpus_sha256", lambda path: "a" * 64
    )

    assert locomo_live_executor._external_input_sha256("corpus", raw_corpus) == "a" * 64


@pytest.mark.parametrize("action", ["fixed_subset_dry_run", "cpu_grid", "gpu_ce_grid"])
def test_release_requires_an_exact_self_hash_and_one_separate_action_gate(action):
    plan = _plan()
    token = _release(plan, action=action)

    assert locomo_live_executor.release_sha256(token) == token["release_sha256"]
    token["approved_actions"] = ["fixed_subset_dry_run", "cpu_grid"]
    token["release_sha256"] = locomo_live_executor.release_sha256(token)
    with pytest.raises(locomo_live_executor.LiveExecutorError, match="exact separately gated"):
        locomo_live_executor.validate_release_shape(plan, token, action=action)


def test_release_rejects_tampering_before_any_external_adapter_can_start(monkeypatch):
    plan = _plan()
    token = _release(plan, action="fixed_subset_dry_run")
    token["runner_sha256"] = "f" * 64
    started = False

    def unexpected(*_args, **_kwargs):  # noqa: ANN001
        nonlocal started
        started = True
        raise AssertionError("adapter must not start")

    monkeypatch.setattr(locomo_live_executor.subprocess, "run", unexpected)
    with pytest.raises(locomo_live_executor.LiveExecutorError, match="self hash"):
        locomo_live_executor.run_action(plan, token, action="fixed_subset_dry_run")
    assert not started


def test_parent_result_requires_trace_attributed_membership_ordinals_and_bounded_context():
    plan = _plan()
    parent_cell = next(cell for cell in plan.cells if cell.program_track == "PARENT-01")
    result = {
        "schema_version": "locomo-live-executor.cell-result.v1",
        "cell_id": parent_cell.cell_id,
        "mode": "full_grid",
        "external_metrics_ref": "locomo-parent-metrics-v1",
        "external_metrics_sha256": "a" * 64,
        "metric_summary": locomo_live_executor.synthetic_metric_summary(parent=True),
        "parent_hits": [{
                    "child_id": _canonical_turn("turn-2"), "rank": 1,
            "child_provenance": {"parent_session_ids": ["session-1"], "ordinal": 2, "trace_source_id": "source-1"},
            "neighbors": [
                {"id": _canonical_turn("turn-1"), "parent_session_id": "session-1", "ordinal": 1, "trace_source_id": "source-1"},
                {"id": _canonical_turn("turn-3"), "parent_session_id": "session-1", "ordinal": 3, "trace_source_id": "source-1"},
            ],
        }],
    }

    projection = locomo_live_executor.validate_cell_result(
        plan, parent_cell, result, active_trace_source_ids={"source-1"},
        parent_relations={
            _canonical_turn("turn-2"): {
                "parent_session_id": "session-1", "ordinal": 2, "trace_source_id": "source-1",
                "member_ordinals": {1: _canonical_turn("turn-1"), 2: _canonical_turn("turn-2"), 3: _canonical_turn("turn-3")},
            }
        },
    )
    assert projection.parent_context == ({
        "parent_session_id": "session-1", "seed_child_id": _canonical_turn("turn-2"),
        "ordered_neighbor_ids": [_canonical_turn("turn-1"), _canonical_turn("turn-3")], "trace_source_id": "source-1",
    },)

    result["parent_hits"][0]["neighbors"][1]["ordinal"] = 4
    with pytest.raises(locomo_live_executor.LiveExecutorError, match="membership proof"):
        locomo_live_executor.validate_cell_result(
            plan, parent_cell, result, active_trace_source_ids={"source-1"},
            parent_relations={
                _canonical_turn("turn-2"): {
                    "parent_session_id": "session-1", "ordinal": 2, "trace_source_id": "source-1",
                    "member_ordinals": {1: _canonical_turn("turn-1"), 2: _canonical_turn("turn-2"), 3: _canonical_turn("turn-3")},
                }
            },
        )


def test_action_projection_is_content_free_and_not_index_eligible_when_partial(tmp_path):
    plan = _plan()
    action = plan.action("cpu_grid")
    result = locomo_live_executor.CellProjection(
        cell_id=action.cell_ids[0], mode="full_grid", external_metrics_ref="cell-metrics-v1",
        external_metrics_sha256="b" * 64, metric_summary=locomo_live_executor.synthetic_metric_summary(parent=False),
        parent_context=(),
    )
    with pytest.raises(locomo_live_executor.LiveExecutorError, match="complete"):
        locomo_live_executor.write_action_projection(
            tmp_path, release_id="release-1", release_sha="c" * 64, plan=plan, action=action,
            results=[result],
        )


def test_fixed_subset_dispatch_writes_only_one_complete_content_free_projection(tmp_path, monkeypatch):
    plan = _plan()
    artifact_root = tmp_path / "external-artifacts"
    artifact_root.mkdir()
    roots = {key: tmp_path / f"{key}.json" for key in plan.external_input_sha256}
    roots["corpus"].write_text("fixture-only", encoding="utf-8")
    roots["dry_run_subset"].write_text("fixture-only", encoding="utf-8")
    roots["turn_provenance"].write_text(json.dumps({
        "schema_version": "locomo-provenance.v1",
        "entries": [{"fingerprint": "a" * 64, "conversation_id": "locomo-1", "session_id": "session-1", "turn_ids": ["turn-2"]}],
    }), encoding="utf-8")
    roots["session_provenance"].write_text(json.dumps({
        "schema_version": "locomo-provenance.v1",
        "entries": [{"fingerprint": "b" * 64, "conversation_id": "locomo-1", "session_id": "session-1", "turn_ids": ["turn-1", "turn-2", "turn-3"]}],
    }), encoding="utf-8")
    trace = tmp_path / "trace.json"
    current = trace_projection.Source("source-1", "f" * 64)
    trace.write_text(json.dumps(trace_projection.build_trace_projection(
        (current,), (trace_projection.Projection("text-1", "source-1", "f" * 64, "text"),), {"warnings": []},
    )), encoding="utf-8")
    parent_relations = tmp_path / "parent-relations.json"
    parent_relations.write_text(json.dumps({
        "schema_version": "locomo-parent-relation-proof.v2",
        "turn_provenance_sha256": plan.external_input_sha256["turn_provenance"],
        "session_provenance_sha256": plan.external_input_sha256["session_provenance"],
        "entries": [{
                "child_id": _canonical_turn("turn-2"), "parent_session_id": _canonical_session(), "ordinal": 2, "trace_source_id": "source-1",
            "turn_provenance_fingerprint": "a" * 64, "session_provenance_fingerprint": "b" * 64,
            "session_members": [
                {"id": _canonical_turn("turn-1"), "ordinal": 1, "trace_source_id": "source-1"},
                {"id": _canonical_turn("turn-2"), "ordinal": 2, "trace_source_id": "source-1"},
                {"id": _canonical_turn("turn-3"), "ordinal": 3, "trace_source_id": "source-1"},
            ],
        }],
    }), encoding="utf-8")
    adapter = tmp_path / "locomo-cell-adapter"
    adapter.write_text("fixture-only", encoding="utf-8")
    adapter.chmod(0o700)
    review = tmp_path / "review-evidence.json"

    token = _release(plan, action="fixed_subset_dry_run")
    token["integrated_git_sha"] = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], text=True
    ).strip()
    token["external_roots"] = {
        "artifact_root": {"path": str(artifact_root), "binding_sha256": "c" * 64},
        **{
            key: {"path": str(path), "sha256": plan.external_input_sha256[key]}
            for key, path in roots.items()
        },
        "trace_projection": {"path": str(trace), "sha256": "d" * 64},
        "parent_relation_proof": {"path": str(parent_relations), "sha256": "f" * 64},
    }
    token["cell_adapter"] = {"path": str(adapter), "sha256": "e" * 64}
    token["review_evidence"] = {"path": str(review), "sha256": "0" * 64}
    review.write_text(json.dumps({
        "schema_version": "locomo-live-executor.review-evidence.v1", "verdict": "accepted",
        "review_git_sha": token["independent_review_git_sha"], "release_id": token["release_id"],
        "release_binding_sha256": locomo_live_executor.release_binding_sha256(token),
        "integrated_git_sha": token["integrated_git_sha"], "phase_b_config_sha256": token["phase_b_config_sha256"],
        "executor_config_sha256": token["executor_config_sha256"], "runner_sha256": token["runner_sha256"],
    }), encoding="utf-8")
    token["review_evidence"] = {"path": str(review), "sha256": "9" * 64}
    token["release_sha256"] = locomo_live_executor.release_sha256(token)

    def fixture_sha(path: Path) -> str:
        if path == adapter:
            return "e" * 64
        if path == trace:
            return "d" * 64
        if path == parent_relations:
            return "f" * 64
        if path == review:
            return "9" * 64
        for key, root in roots.items():
            if path == root:
                return plan.external_input_sha256[key]
        return _sha(path.read_bytes())

    observed: list[str] = []

    def fixture_run(command, **kwargs):  # noqa: ANN001
        if command[:3] == ["git", "rev-parse", "HEAD"]:
            return subprocess.CompletedProcess(command, 0, stdout=f"{token['integrated_git_sha']}\n", stderr="")
        request = json.loads(kwargs["input"])
        observed.append(request["cell"]["cell_id"])
        parent = request["cell"]["program_track"] == "PARENT-01"
        result = {
            "schema_version": "locomo-live-executor.cell-result.v1",
            "cell_id": request["cell"]["cell_id"], "mode": request["mode"],
            "external_metrics_ref": "fixture-metrics-v1", "external_metrics_sha256": "a" * 64,
            "metric_summary": locomo_live_executor.synthetic_metric_summary(parent=parent),
        }
        if parent:
            result["parent_hits"] = [{
                "child_id": _canonical_turn("turn-2"), "rank": 1,
                    "child_provenance": {"parent_session_ids": [_canonical_session()], "ordinal": 2, "trace_source_id": "source-1"},
                "neighbors": [],
            }]
        return subprocess.CompletedProcess(command, 0, stdout=json.dumps(result), stderr="")

    monkeypatch.setattr(locomo_live_executor, "_file_sha256", fixture_sha)
    monkeypatch.setattr(
        locomo_live_executor,
        "_normalized_corpus_sha256",
        lambda path: plan.external_input_sha256["corpus"],
    )
    monkeypatch.setattr(locomo_live_executor.subprocess, "run", fixture_run)
    projection_path = locomo_live_executor.run_action(plan, token, action="fixed_subset_dry_run")

    projection = json.loads(projection_path.read_text(encoding="utf-8"))
    assert observed == list(plan.action("fixed_subset_dry_run").cell_ids)
    assert projection["result_count"] == 5
    assert projection["receipt_status"] == "dry_run_proof"
    assert projection["index_eligible"] is True
    assert str(artifact_root) not in json.dumps(projection)
    assert "fixture-only" not in json.dumps(projection)


def test_action_resume_persists_only_validated_content_free_cell_receipts(tmp_path, monkeypatch):
    """A bounded host invocation can resume one release without accepting a partial action."""
    plan = _plan()
    artifact_root = tmp_path / "external-artifacts"
    artifact_root.mkdir()
    roots = {key: tmp_path / f"{key}.json" for key in plan.external_input_sha256}
    roots["corpus"].write_text("fixture-only", encoding="utf-8")
    roots["dry_run_subset"].write_text("fixture-only", encoding="utf-8")
    roots["turn_provenance"].write_text(json.dumps({
        "schema_version": "locomo-provenance.v1",
        "entries": [{"fingerprint": "a" * 64, "conversation_id": "locomo-1", "session_id": "session-1", "turn_ids": ["turn-2"]}],
    }), encoding="utf-8")
    roots["session_provenance"].write_text(json.dumps({
        "schema_version": "locomo-provenance.v1",
        "entries": [{"fingerprint": "b" * 64, "conversation_id": "locomo-1", "session_id": "session-1", "turn_ids": ["turn-1", "turn-2", "turn-3"]}],
    }), encoding="utf-8")
    trace = tmp_path / "trace.json"
    current = trace_projection.Source("source-1", "f" * 64)
    trace.write_text(json.dumps(trace_projection.build_trace_projection(
        (current,), (trace_projection.Projection("text-1", "source-1", "f" * 64, "text"),), {"warnings": []},
    )), encoding="utf-8")
    relations = tmp_path / "relations.json"
    relations.write_text(json.dumps({
        "schema_version": "locomo-parent-relation-proof.v2",
        "turn_provenance_sha256": plan.external_input_sha256["turn_provenance"],
        "session_provenance_sha256": plan.external_input_sha256["session_provenance"],
        "entries": [{
            "child_id": _canonical_turn("turn-2"), "parent_session_id": _canonical_session(), "ordinal": 2, "trace_source_id": "source-1",
            "turn_provenance_fingerprint": "a" * 64, "session_provenance_fingerprint": "b" * 64,
            "session_members": [
                {"id": _canonical_turn("turn-1"), "ordinal": 1, "trace_source_id": "source-1"},
                {"id": _canonical_turn("turn-2"), "ordinal": 2, "trace_source_id": "source-1"},
                {"id": _canonical_turn("turn-3"), "ordinal": 3, "trace_source_id": "source-1"},
            ],
        }],
    }), encoding="utf-8")
    adapter = tmp_path / "adapter"
    adapter.write_text("fixture-only", encoding="utf-8")
    adapter.chmod(0o700)
    review = tmp_path / "review.json"
    token = _release(plan, action="fixed_subset_dry_run")
    token["integrated_git_sha"] = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    token["external_roots"] = {
        "artifact_root": {"path": str(artifact_root), "binding_sha256": "c" * 64},
        **{key: {"path": str(path), "sha256": plan.external_input_sha256[key]} for key, path in roots.items()},
        "trace_projection": {"path": str(trace), "sha256": "d" * 64},
        "parent_relation_proof": {"path": str(relations), "sha256": "f" * 64},
    }
    token["cell_adapter"] = {"path": str(adapter), "sha256": "e" * 64}
    token["review_evidence"] = {"path": str(review), "sha256": "0" * 64}
    review.write_text(json.dumps({
        "schema_version": "locomo-live-executor.review-evidence.v1", "verdict": "accepted",
        "review_git_sha": token["independent_review_git_sha"], "release_id": token["release_id"],
        "release_binding_sha256": locomo_live_executor.release_binding_sha256(token),
        "integrated_git_sha": token["integrated_git_sha"], "phase_b_config_sha256": token["phase_b_config_sha256"],
        "executor_config_sha256": token["executor_config_sha256"], "runner_sha256": token["runner_sha256"],
    }), encoding="utf-8")
    token["review_evidence"] = {"path": str(review), "sha256": "9" * 64}
    token["release_sha256"] = locomo_live_executor.release_sha256(token)

    def fixture_sha(path: Path) -> str:
        if path == adapter:
            return "e" * 64
        if path == trace:
            return "d" * 64
        if path == relations:
            return "f" * 64
        if path == review:
            return "9" * 64
        for key, root in roots.items():
            if path == root:
                return plan.external_input_sha256[key]
        return _sha(path.read_bytes())

    observed: list[str] = []

    def fixture_run(command, **kwargs):  # noqa: ANN001
        if command[:3] == ["git", "rev-parse", "HEAD"]:
            return subprocess.CompletedProcess(command, 0, stdout=f"{token['integrated_git_sha']}\n", stderr="")
        request = json.loads(kwargs["input"])
        observed.append(request["cell"]["cell_id"])
        parent = request["cell"]["program_track"] == "PARENT-01"
        result = {
            "schema_version": "locomo-live-executor.cell-result.v1", "cell_id": request["cell"]["cell_id"],
            "mode": request["mode"], "external_metrics_ref": "fixture-metrics-v1", "external_metrics_sha256": "a" * 64,
            "metric_summary": locomo_live_executor.synthetic_metric_summary(parent=parent),
        }
        if parent:
            result["parent_hits"] = [{
                "child_id": _canonical_turn("turn-2"), "rank": 1,
                "child_provenance": {"parent_session_ids": [_canonical_session()], "ordinal": 2, "trace_source_id": "source-1"},
                "neighbors": [],
            }]
        return subprocess.CompletedProcess(command, 0, stdout=json.dumps(result), stderr="")

    monkeypatch.setattr(locomo_live_executor, "_file_sha256", fixture_sha)
    monkeypatch.setattr(locomo_live_executor, "_normalized_corpus_sha256", lambda _path: plan.external_input_sha256["corpus"])
    monkeypatch.setattr(locomo_live_executor.subprocess, "run", fixture_run)

    assert locomo_live_executor.run_action(plan, token, action="fixed_subset_dry_run", max_cells=2) is None
    assert not locomo_live_executor._projection_path(artifact_root, str(token["release_id"]), plan.action("fixed_subset_dry_run")).exists()
    projection_path = locomo_live_executor.run_action(plan, token, action="fixed_subset_dry_run", max_cells=3)

    assert projection_path is not None
    assert observed == list(plan.action("fixed_subset_dry_run").cell_ids)
    assert json.loads(projection_path.read_text(encoding="utf-8"))["result_count"] == 5


def test_existing_action_projection_must_bind_the_exact_release_before_idempotent_return():
    plan = _plan()
    action = plan.action("fixed_subset_dry_run")
    release = {"release_id": "release-1", "release_sha256": "a" * 64}
    results = [
        locomo_live_executor.CellProjection(
            cell_id=cell_id,
            mode=action.mode,
            external_metrics_ref="fixture-metrics-v1",
            external_metrics_sha256="b" * 64,
            metric_summary=locomo_live_executor.synthetic_metric_summary(parent=cell_id.startswith("turn--parent")),
            parent_context=(
                {"parent_session_id": "session-1", "seed_child_id": "turn-1", "ordered_neighbor_ids": [], "trace_source_id": "source-1"},
            ) if cell_id.startswith("turn--parent") else (),
        )
        for cell_id in action.cell_ids
    ]
    projection = locomo_live_executor.action_projection_document(plan, action, release, results)
    projection["release_sha256"] = "c" * 64

    with pytest.raises(locomo_live_executor.LiveExecutorError, match="action projection does not bind"):
        locomo_live_executor._validate_existing_action_projection(plan, action, release, projection)


def test_parent_cell_receipt_rechecks_context_bounds_before_resume():
    plan = _plan()
    action = plan.action("fixed_subset_dry_run")
    cell = next(item for item in plan.cells if item.cell_id == "turn--parent_child_turn_session_v1--cpu--cold")
    release = {"release_id": "release-1", "release_sha256": "a" * 64}
    receipt = locomo_live_executor._cell_receipt_document(
        plan,
        action,
        release,
        locomo_live_executor.CellProjection(
            cell_id=cell.cell_id,
            mode=action.mode,
            external_metrics_ref="fixture-metrics-v1",
            external_metrics_sha256="b" * 64,
            metric_summary=locomo_live_executor.synthetic_metric_summary(parent=True),
            parent_context=tuple(
                {"parent_session_id": f"session-{index}", "seed_child_id": f"turn-{index}", "ordered_neighbor_ids": [], "trace_source_id": "source-1"}
                for index in range(6)
            ),
        ),
    )

    with pytest.raises(locomo_live_executor.LiveExecutorError, match="parent context"):
        locomo_live_executor._validate_cell_receipt(plan, action, release, cell, receipt)


def test_config_rejects_action_or_runner_digest_drift():
    document = json.loads(CONFIG_PATH.read_text(encoding="utf-8"))
    document["runner"]["sha256"] = "0" * 64
    with pytest.raises(locomo_live_executor.LiveExecutorError, match="runner digest"):
        locomo_live_executor.load_config(document)
