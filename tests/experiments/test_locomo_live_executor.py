"""Lifecycle contract for the future LOCOMO/PARENT external executor.

These tests deliberately exercise only synthetic files and injected adapters.
They must never acquire a corpus, invoke a model, or create a campaign receipt.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from experiments import locomo_live_executor


CONFIG_PATH = Path("experiments/configs/locomo-01/live-executor.v1.json")


def _sha(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


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
        "approved_actions": [action],
        "gpu_policy": {"cuda_required": action == "gpu_ce_grid", "allow_cpu_fallback": False},
        "external_roots": {
            "artifact_root": {"path": "/external/artifacts", "binding_sha256": "c" * 64},
            "corpus": {"path": "/external/locomo.json", "sha256": plan.external_input_sha256["corpus"]},
            "turn_provenance": {"path": "/external/turn.json", "sha256": plan.external_input_sha256["turn_provenance"]},
            "session_provenance": {"path": "/external/session.json", "sha256": plan.external_input_sha256["session_provenance"]},
            "dry_run_subset": {"path": "/external/subset.json", "sha256": plan.external_input_sha256["dry_run_subset"]},
            "trace_projection": {"path": "/external/trace.json", "sha256": "d" * 64},
        },
        "cell_adapter": {"path": "/external/locomo-cell-adapter", "sha256": "e" * 64},
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


@pytest.mark.parametrize("action", ["fixed_subset_dry_run", "cpu_grid", "gpu_ce_grid"])
def test_release_requires_an_exact_self_hash_and_one_separate_action_gate(action):
    plan = _plan()
    token = _release(plan, action=action)

    assert locomo_live_executor.release_sha256(token) == token["release_sha256"]
    token["approved_actions"] = ["cpu_grid", "gpu_ce_grid"]
    with pytest.raises(locomo_live_executor.LiveExecutorError, match="one action"):
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
            "child_id": "turn-2", "rank": 1,
            "child_provenance": {"parent_session_ids": ["session-1"], "ordinal": 2, "trace_source_id": "source-1"},
            "neighbors": [
                {"id": "turn-1", "parent_session_id": "session-1", "ordinal": 1, "trace_source_id": "source-1"},
                {"id": "turn-3", "parent_session_id": "session-1", "ordinal": 3, "trace_source_id": "source-1"},
            ],
        }],
    }

    projection = locomo_live_executor.validate_cell_result(
        plan, parent_cell, result, active_trace_source_ids={"source-1"}
    )
    assert projection.parent_context == ({
        "parent_session_id": "session-1", "seed_child_id": "turn-2",
        "ordered_neighbor_ids": ["turn-1", "turn-3"], "trace_source_id": "source-1",
    },)

    result["parent_hits"][0]["neighbors"][1]["ordinal"] = 4
    with pytest.raises(locomo_live_executor.LiveExecutorError, match="immediately"):
        locomo_live_executor.validate_cell_result(plan, parent_cell, result, active_trace_source_ids={"source-1"})


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


def test_config_rejects_action_or_runner_digest_drift():
    document = json.loads(CONFIG_PATH.read_text(encoding="utf-8"))
    document["runner"]["sha256"] = "0" * 64
    with pytest.raises(locomo_live_executor.LiveExecutorError, match="runner digest"):
        locomo_live_executor.load_config(document)
