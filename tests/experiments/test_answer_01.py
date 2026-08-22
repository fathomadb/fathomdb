"""ANSWER-01 typed dry-run and checkpoint contract."""

from __future__ import annotations

import json
import stat
from pathlib import Path

import pytest

from experiments import answer_01


CONFIG = Path("experiments/configs/answer-01/shortlist-scoring.v1.json")


def _corpus() -> list[dict[str, object]]:
    qa = []
    categories = (4, 2, 1)
    for index in range(32):
        qa.append(
            {
                "question": f"secret-question-{index}",
                "answer": f"gold-{index}",
                "category": categories[index % len(categories)],
                "evidence": ["D1:1"],
            }
        )
    return [
        {
            "conversation": {
                "speaker_a": "A",
                "session_1": [
                    {"speaker": "A", "text": "fixture body", "dia_id": "D1:1"}
                ],
            },
            "qa": qa,
        }
    ]


def _subset() -> dict[str, object]:
    return {
        "schema_version": "locomo-fixed-subset.v1",
        "question_ids": [f"locomo-0-q-{index}" for index in range(32)],
    }


def test_config_binds_exact_two_arms_and_live_cost_boundary() -> None:
    config = answer_01.load_config(json.loads(CONFIG.read_text(encoding="utf-8")))

    assert config["program_track"] == "ANSWER-01"
    assert [arm["id"] for arm in config["arms"]] == [
        "a0_turn_fts",
        "hybrid_ce_alpha_10_pool_20",
    ]
    assert config["dry_run"] == {
        "answerer": "stub-deterministic-v1",
        "judge": "stub-grounded-attribution-v1",
        "network": False,
        "question_count": 32,
    }
    assert config["live"]["answerer"]["model"] == "gpt-5.4"
    assert config["live"]["judge"]["model"] == "gemini-3.1-flash-lite"
    assert config["live"]["max_usd"] == 3.0
    assert config["live"]["checkpoint_every"] == 1
    assert config["live"]["max_workers"] == 1
    assert config["live"]["max_retries"] == 1


def test_config_rejects_unknown_fields() -> None:
    raw = json.loads(CONFIG.read_text(encoding="utf-8"))
    raw["surprise"] = True

    with pytest.raises(answer_01.Answer01Error, match="keys mismatch"):
        answer_01.load_config(raw)


def test_fixed_subset_resolves_answers_and_all_reporting_classes() -> None:
    selected = answer_01.select_questions(_corpus(), _subset())

    assert len(selected) == 32
    assert {question.reporting_class for question in selected} == {
        "factoid",
        "temporal",
        "multi_session",
    }
    assert selected[0].answer == "gold-0"


def test_stub_dry_run_checkpoints_every_pair_and_resumes(tmp_path: Path) -> None:
    config = answer_01.load_config(json.loads(CONFIG.read_text(encoding="utf-8")))
    questions = answer_01.select_questions(_corpus(), _subset())
    calls: list[tuple[str, str]] = []

    def retrieve(arm_id: str, question: answer_01.Question) -> list[answer_01.ContextHit]:
        calls.append((arm_id, question.question_id))
        return [
            answer_01.ContextHit(
                logical_id=f"{arm_id}-{question.question_id}",
                source_id="source-1",
                body=f"context containing {question.answer}",
            )
        ]

    checkpoint = tmp_path / "checkpoint.json"
    first = answer_01.score_stub_dry_run(
        config, questions, retrieve=retrieve, checkpoint_path=checkpoint
    )

    assert len(calls) == 64
    assert first["status"] == "dry_run_proof"
    assert first["pair_count"] == 64
    assert first["complete_pair_count"] == 32
    assert first["cost_usd"] == 0.0
    assert set(first["arms"]["a0_turn_fts"]["by_class"]) == {
        "factoid",
        "temporal",
        "multi_session",
    }
    checkpoint_doc = json.loads(checkpoint.read_text(encoding="utf-8"))
    assert len(checkpoint_doc["records"]) == 64
    assert stat.S_IMODE(checkpoint.stat().st_mode) == 0o600

    def must_not_retrieve(_arm_id: str, _question: answer_01.Question):
        raise AssertionError("completed checkpoint cells must not be repeated")

    resumed = answer_01.score_stub_dry_run(
        config, questions, retrieve=must_not_retrieve, checkpoint_path=checkpoint
    )
    assert resumed == first


def test_safe_summary_excludes_corpus_and_model_text(tmp_path: Path) -> None:
    config = answer_01.load_config(json.loads(CONFIG.read_text(encoding="utf-8")))
    questions = answer_01.select_questions(_corpus(), _subset())

    def retrieve(arm_id: str, question: answer_01.Question) -> list[answer_01.ContextHit]:
        return [answer_01.ContextHit(f"{arm_id}-id", "source-1", question.answer)]

    summary = answer_01.score_stub_dry_run(
        config,
        questions,
        retrieve=retrieve,
        checkpoint_path=tmp_path / "checkpoint.json",
    )
    encoded = json.dumps(summary)
    assert "secret-question" not in encoded
    assert "context containing" not in encoded
    assert "gold-" not in encoded


def test_live_scoring_checkpoints_each_model_call_and_resumes(tmp_path: Path) -> None:
    config = answer_01.load_config(json.loads(CONFIG.read_text(encoding="utf-8")))
    questions = answer_01.select_questions(_corpus(), _subset())

    def retrieve(arm_id: str, question: answer_01.Question) -> list[answer_01.ContextHit]:
        return [answer_01.ContextHit(f"{arm_id}-id", "source-1", question.answer)]

    dry_checkpoint = tmp_path / "dry.json"
    answer_01.score_stub_dry_run(
        config, questions, retrieve=retrieve, checkpoint_path=dry_checkpoint
    )
    calls: list[str] = []

    def complete(role: str, _model: str, _messages: object, _max_tokens: int):
        calls.append(role)
        if role == "answerer":
            content = '{"answer":"fixture answer","citations":["source-1"]}'
        else:
            content = '{"answer_correct":true,"grounded":true,"attributed":true}'
        return answer_01.ModelReply(content, prompt_tokens=100, completion_tokens=20)

    live_checkpoint = tmp_path / "live.json"
    first = answer_01.score_live(
        config,
        questions,
        dry_checkpoint_path=dry_checkpoint,
        checkpoint_path=live_checkpoint,
        complete=complete,
    )

    assert first["status"] == "complete"
    assert first["decision_eligible"] is True
    assert first["pair_count"] == 64
    assert first["complete_pair_count"] == 32
    assert first["live_calls"] == 128
    assert first["cost_usd"] > 0
    assert len(calls) == 128

    def must_not_complete(*_args: object):
        raise AssertionError("completed live model calls must not be repeated")

    resumed = answer_01.score_live(
        config,
        questions,
        dry_checkpoint_path=dry_checkpoint,
        checkpoint_path=live_checkpoint,
        complete=must_not_complete,
    )
    assert resumed == first


def test_live_scoring_stops_before_exceeding_cost_cap(tmp_path: Path) -> None:
    config = answer_01.load_config(json.loads(CONFIG.read_text(encoding="utf-8")))
    config["live"]["max_usd"] = 0.000001
    questions = answer_01.select_questions(_corpus(), _subset())

    def retrieve(arm_id: str, question: answer_01.Question) -> list[answer_01.ContextHit]:
        return [answer_01.ContextHit(f"{arm_id}-id", "source-1", question.answer)]

    dry_checkpoint = tmp_path / "dry.json"
    answer_01.score_stub_dry_run(
        config, questions, retrieve=retrieve, checkpoint_path=dry_checkpoint
    )
    calls: list[str] = []

    def complete(*_args: object):
        calls.append("called")
        return answer_01.ModelReply("{}", prompt_tokens=1, completion_tokens=1)

    result = answer_01.score_live(
        config,
        questions,
        dry_checkpoint_path=dry_checkpoint,
        checkpoint_path=tmp_path / "live.json",
        complete=complete,
    )

    assert result["status"] == "cost_cap"
    assert result["decision_eligible"] is False
    assert result["live_calls"] == 0
    assert calls == []
