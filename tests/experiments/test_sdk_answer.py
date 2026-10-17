"""Synthetic contract checks for the shared SDK answer stage."""

import copy
import hashlib
import json

import pytest

from experiments import sdk_answer


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def sha(value):
    return hashlib.sha256(encoded(value)).hexdigest()


def fixture(
    attempt="attempt-one", sdk="python", text="Paris is the capital of France."
):
    gold = {
        "schema_version": "sdk.answer-gold.v1",
        "questions": [
            {
                "id": "q1",
                "question": "What is the capital of France?",
                "answer": "Paris is the capital of France.",
                "source_ids": ["s1"],
                "labels": ["factoid"],
            },
            {
                "id": "q2",
                "question": "What is the capital of Germany?",
                "answer": "Berlin is the capital of Germany.",
                "source_ids": ["s2"],
                "labels": ["factoid"],
            },
        ],
    }
    config = {
        "schema_version": "sdk.answer-config.v1",
        "upstream": {"case_id": "retrieve", "arm_relation": "same_sdk"},
        "evidence_transform": "ordered_snippets_v1",
        "answerer": "first_sentence_v1",
        "prompt": sdk_answer.PROMPT,
        "context": {"max_hits": 2, "max_chars": 100},
        "decoding": {"temperature": 0, "seed": 0, "max_output_chars": 100},
        "judge": "exact_token_f1_grounding_v1",
        "gold_sha256": sha(gold),
        "question_order": ["q1", "q2"],
        "cost_ceiling_usd": 0,
    }
    retrieval = {
        "schema_version": "sdk.result.v1",
        "suite_attempt_id": attempt,
        "case_id": "retrieve",
        "sdk": sdk,
        "family": "retrieval",
        "status": "complete",
        "ordered_ids": {"q1": ["h1"], "q2": ["h2"]},
    }
    evidence = {
        "schema_version": "sdk.retrieval-evidence.v1",
        "suite_attempt_id": attempt,
        "case_id": "retrieve",
        "sdk": sdk,
        "questions": [
            {"id": "q1", "hits": [{"id": "h1", "source_id": "s1", "text": text}]},
            {
                "id": "q2",
                "hits": [
                    {
                        "id": "h2",
                        "source_id": "s2",
                        "text": "Berlin is the capital of Germany.",
                    }
                ],
            },
        ],
    }
    binding = {
        "suite_attempt_id": attempt,
        "case_id": "retrieve",
        "sdk": sdk,
        "retrieval_result_sha256": sha(retrieval),
        "evidence_sha256": sha(evidence),
    }
    return config, gold, retrieval, evidence, binding


def run(parts):
    config, gold, retrieval, evidence, binding = parts
    ticks = iter(range(0, 100000000, 1000000))
    return sdk_answer.run_answer_case(
        config=config,
        gold_bytes=encoded(gold),
        retrieval_result_bytes=encoded(retrieval),
        evidence_bytes=encoded(evidence),
        binding=binding,
        suite_id="synthetic-suite",
        suite_attempt_id=binding["suite_attempt_id"],
        sdk=binding["sdk"],
        case_id="answer",
        clock_ns=lambda: next(ticks),
    )


def test_same_config_binds_each_attempt_and_scores_frozen_pipeline():
    first = run(fixture())
    second = run(
        fixture(
            attempt="attempt-two",
            text="Lyon is in France. Paris is the capital of France.",
        )
    )
    assert first["config_sha256"] == second["config_sha256"]
    assert first["witness"]["upstream"] != second["witness"]["upstream"]
    assert first["result"]["metrics"] == {
        "answer_accuracy": 1.0,
        "answer_f1": 1.0,
        "groundedness": 1.0,
        "attribution": 1.0,
        "cost_usd": 0.0,
        "answer_ms": 2.0,
        "judge_ms": 2.0,
    }
    assert second["result"]["metrics"]["answer_accuracy"] == 0.5
    assert first["result"]["upstream"] == first["witness"]["upstream"]
    assert first["result"]["family"] == "answer"
    assert first["result"]["producer"] == "answer_pipeline"


@pytest.mark.parametrize(
    "change",
    [
        "missing",
        "cross_attempt",
        "wrong_sdk",
        "blocked",
        "altered_evidence",
        "ids_mismatch",
    ],
)
def test_dependency_must_be_completed_same_attempt_and_hash_checked(change):
    parts = list(fixture())
    if change == "missing":
        parts[2] = None
    elif change == "cross_attempt":
        parts[2]["suite_attempt_id"] = "other-attempt"
        parts[4]["retrieval_result_sha256"] = sha(parts[2])
    elif change == "wrong_sdk":
        parts[3]["sdk"] = "rust"
        parts[4]["evidence_sha256"] = sha(parts[3])
    elif change == "blocked":
        parts[2]["status"] = "blocked"
        parts[4]["retrieval_result_sha256"] = sha(parts[2])
    elif change == "altered_evidence":
        parts[3]["questions"][0]["hits"][0]["text"] = "Altered"
    elif change == "ids_mismatch":
        parts[3]["questions"][0]["hits"][0]["id"] = "wrong"
        parts[4]["evidence_sha256"] = sha(parts[3])
    with pytest.raises(ValueError):
        run(parts)


def test_retrieval_only_gold_cannot_score_answers():
    parts = list(fixture())
    parts[1]["questions"][0].pop("answer")
    parts[0]["gold_sha256"] = sha(parts[1])
    with pytest.raises(ValueError, match="answer gold"):
        run(parts)


@pytest.mark.parametrize(
    "field,value",
    [
        ("prompt", "Changed {question} {context}"),
        ("answerer", "other-model"),
        ("judge", "other-judge"),
        ("context", {"max_hits": 1, "max_chars": 100}),
        ("decoding", {"temperature": 1, "seed": 0, "max_output_chars": 100}),
    ],
)
def test_changed_answer_contract_refuses_pairing(field, value):
    config, *_ = fixture()
    altered = copy.deepcopy(config)
    altered[field] = value
    with pytest.raises(ValueError):
        sdk_answer.assert_pairable(config, altered)


def test_gold_and_question_order_are_pinned():
    parts = list(fixture())
    parts[1]["questions"].reverse()
    with pytest.raises(ValueError):
        run(parts)


def test_same_evidence_across_arms_and_missing_evidence_affect_scores():
    python = run(fixture())
    rust = run(fixture(sdk="rust"))
    assert python["result"]["metrics"] == rust["result"]["metrics"]
    assert python["config_sha256"] == rust["config_sha256"]

    parts = list(fixture())
    parts[2]["ordered_ids"]["q2"] = []
    parts[3]["questions"][1]["hits"] = []
    parts[4]["retrieval_result_sha256"] = sha(parts[2])
    parts[4]["evidence_sha256"] = sha(parts[3])
    empty = run(parts)["result"]["metrics"]
    assert empty["answer_accuracy"] == 0.5
    assert empty["groundedness"] == 0.5
    assert empty["attribution"] == 0.5


def test_witness_result_digest_matches_persisted_bytes():
    output = run(fixture())
    assert (
        output["witness"]["result_sha256"]
        == hashlib.sha256(sdk_answer.artifact_bytes(output["result"])).hexdigest()
    )
