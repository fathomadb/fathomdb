"""TDD checks for the LongMemEval external acquisition contract.

The live acquire step has network I/O. These tests pin its pure configuration
and metadata accounting against a tiny, non-payload fixture.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _config import add_config_cli, config_from_dict, load_config, resolve_config  # noqa: E402
from acquire_longmemeval import (  # noqa: E402
    DEFAULT_FILES,
    LongMemEvalConfig,
    summarize_instances,
)


CONFIG_PATH = Path(__file__).resolve().parent / "configs/acquire-longmemeval.yaml"


def _instance(question_type: str, *, answer: object = "answer") -> dict[str, object]:
    return {
        "question_id": f"fixture-{question_type}",
        "question_type": question_type,
        "question": "fixture question",
        "answer": answer,
        "question_date": "2026-01-01",
        "haystack_session_ids": ["session-1"],
        "haystack_dates": ["2025-12-31"],
        "haystack_sessions": [[{
            "role": "user", "content": "fixture evidence", "has_answer": True,
        }]],
        "answer_session_ids": ["session-1"],
    }


def test_summary_counts_question_classes_and_evidence_fields():
    summary = summarize_instances([
        _instance("temporal-reasoning"),
        _instance("knowledge-update"),
        _instance("knowledge-update", answer=["one", "two"]),
    ])

    assert summary == {
        "instances": 3,
        "question_type_counts": {
            "knowledge-update": 2,
            "temporal-reasoning": 1,
        },
        "with_answer_session_ids": 3,
        "with_turn_evidence": 3,
    }


def test_summary_rejects_an_instance_without_required_retrieval_evidence():
    incomplete = _instance("temporal-reasoning")
    incomplete.pop("answer_session_ids")

    with pytest.raises(ValueError, match="answer_session_ids"):
        summarize_instances([incomplete])


def test_config_defaults_and_baked_config_acquire_s_and_oracle_only():
    assert LongMemEvalConfig().files == list(DEFAULT_FILES)
    assert load_config(LongMemEvalConfig, CONFIG_PATH) == LongMemEvalConfig()


def test_config_rejects_unknown_or_unreleased_file():
    with pytest.raises(ValueError, match="unknown config keys"):
        config_from_dict(LongMemEvalConfig, {"filez": []})
    with pytest.raises(ValueError, match="files"):
        LongMemEvalConfig(files=["not-a-release.json"]).validate()


def test_config_cli_override_is_explicit_and_validated():
    parser = argparse.ArgumentParser()
    add_config_cli(parser)
    args = parser.parse_args([
        "--override",
        'files=["longmemeval_oracle.json"]',
    ])

    assert resolve_config(LongMemEvalConfig, args, LongMemEvalConfig()).files == [
        "longmemeval_oracle.json"
    ]
