"""REASON-01 held-out retrieval and paid scoring contracts."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments import reason_01_run


def test_checkpoint_is_bound_and_resumes_only_missing_cells(tmp_path: Path) -> None:
    path = tmp_path / "checkpoint.json"
    checkpoint = reason_01_run.Checkpoint.open(
        path,
        config_sha256="a" * 64,
        question_ids=["q1", "q2"],
    )
    checkpoint.put_retrieval("a0", "q1", {"hits": []})

    resumed = reason_01_run.Checkpoint.open(
        path,
        config_sha256="a" * 64,
        question_ids=["q1", "q2"],
    )
    assert resumed.retrieval("a0", "q1") == {"hits": []}
    assert resumed.retrieval("protected_multiquery_v1", "q1") is None
    assert path.stat().st_mode & 0o777 == 0o600

    with pytest.raises(reason_01_run.Reason01RunError, match="does not match"):
        reason_01_run.Checkpoint.open(
            path,
            config_sha256="b" * 64,
            question_ids=["q1", "q2"],
        )


def test_checkpoint_allows_one_explicit_format_recovery_rebind(tmp_path: Path) -> None:
    path = tmp_path / "checkpoint.json"
    original = reason_01_run.Checkpoint.open(
        path,
        config_sha256="a" * 64,
        question_ids=["q1"],
    )
    original.put_retrieval("a0", "q1", {"hits": []})

    recovered = reason_01_run.Checkpoint.open(
        path,
        config_sha256="b" * 64,
        question_ids=["q1"],
        prior_config_sha256="a" * 64,
        amendment="judge JSON-shape recovery",
    )
    assert recovered.retrieval("a0", "q1") == {"hits": []}
    assert recovered.state.config_sha256 == "b" * 64
    assert recovered.state.amendments == [
        {
            "from_config_sha256": "a" * 64,
            "to_config_sha256": "b" * 64,
            "reason": "judge JSON-shape recovery",
        }
    ]


def test_model_response_is_charged_before_semantic_parse(tmp_path: Path) -> None:
    checkpoint = reason_01_run.Checkpoint.open(
        tmp_path / "checkpoint.json",
        config_sha256="a" * 64,
        question_ids=["q1"],
    )
    checkpoint.record_attempt(
        "answers",
        "a0||q1",
        model="deepseek-v4-pro",
        content="not-json",
        prompt_tokens=100,
        completion_tokens=20,
        cost_usd=0.001,
    )

    state = json.loads(checkpoint.path.read_text(encoding="utf-8"))
    assert state["cost_usd"] == pytest.approx(0.001)
    assert state["answers"]["a0||q1"]["attempts"][0]["content"] == "not-json"
    assert "result" not in state["answers"]["a0||q1"]


def test_answer_contract_rejects_unknown_citations() -> None:
    hits = [{"logical_id": "m1", "source_id": "s1", "body": "evidence"}]
    assert reason_01_run.parse_answer(
        '{"answer":"yes","citations":["m1"]}', hits
    ) == {"answer": "yes", "citations": ["m1"]}
    with pytest.raises(reason_01_run.Reason01RunError, match="citation"):
        reason_01_run.parse_answer(
            '{"answer":"yes","citations":["missing"]}', hits
        )


def test_judgment_requires_exact_boolean_contract() -> None:
    assert reason_01_run.parse_judgment(
        '{"answer_correct":true,"grounded":true,"attributed":false}'
    ) == {"answer_correct": True, "grounded": True, "attributed": False}
    with pytest.raises(reason_01_run.Reason01RunError, match="judgment"):
        reason_01_run.parse_judgment(
            '{"answer_correct":true,"grounded":true,"attributed":false,"score":1}'
        )


def test_paired_bootstrap_is_deterministic_and_uses_case_deltas() -> None:
    control = [0.0, 0.5, 1.0, 0.0]
    treatment = [0.5, 0.5, 1.0, 1.0]
    first = reason_01_run.paired_bootstrap(
        control, treatment, draws=1000, seed=20260830
    )
    second = reason_01_run.paired_bootstrap(
        control, treatment, draws=1000, seed=20260830
    )
    assert first == second
    assert first["delta"] == pytest.approx(0.375)
    assert first["one_sided_95_lower"] >= 0.0


def test_retry_after_accepts_seconds_without_truncating() -> None:
    assert reason_01_run.retry_after_seconds({"Retry-After": "217"}, fallback=1.0) == 217.0
