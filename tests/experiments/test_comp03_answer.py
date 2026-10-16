"""Measurement-correctness checks for the COMP-03 answer comparison."""

from __future__ import annotations

import copy
import json
from pathlib import Path

import pytest

from experiments import comp03_answer as c3

TYPES = sorted(c3.QUESTION_TYPES)


def _item(qid: str, qtype: str, *, answer: str = "Paris") -> dict:
    return {
        "question_id": qid,
        "question_type": qtype,
        "question": "Where did I move to last spring?",
        "question_date": "2023/06/01 (Thu) 10:00",
        "answer": answer,
        "answer_session_ids": ["answer_s2"],
        "haystack_dates": ["2023/04/01 (Sat) 09:00", "2023/05/01 (Mon) 09:00"],
        "haystack_session_ids": ["sharegpt_s1", "answer_s2"],
        "haystack_sessions": [
            [
                {"role": "user", "content": "Recommend a pasta recipe."},
                {"role": "assistant", "content": "Try carbonara with guanciale."},
            ],
            [
                {"role": "user", "content": "I just moved to Paris last spring!", "has_answer": True},
                {"role": "assistant", "content": "Congratulations on the move."},
                {"role": "user", "content": "Any museum tips?"},
                {"role": "assistant", "content": "Visit the Orsay early."},
            ],
        ],
    }


def test_selection_takes_ten_per_type_sorted_and_is_seed_stable():
    dataset = [_item(f"t{n}-{i:02d}", t) for n, t in enumerate(TYPES) for i in range(12)]
    picked = c3.select_questions(dataset, per_type=10, seed=7)
    assert len(picked) == 60
    by_type = {t: [q for q in picked if q.startswith(f"t{n}-")] for n, t in enumerate(TYPES)}
    assert all(len(ids) == 10 and ids == sorted(ids) for ids in by_type.values())
    assert c3.select_questions(list(reversed(dataset)), per_type=10, seed=7) == picked
    assert c3.select_questions(dataset, per_type=10, seed=8) != picked


def test_rounds_group_user_turn_with_following_assistant_turns():
    case = c3.convert_question(_item("q1", "single-session-user"))
    assert [d["id"] for d in case["docs"]] == ["s000-r000", "s001-r000", "s001-r001"]
    assert case["docs"][1]["body"] == "I just moved to Paris last spring!\nCongratulations on the move."
    assert case["docs"][1]["date"] == "2023/05/01 (Mon) 09:00"


def test_gold_changes_leave_index_evidence_and_answer_prompt_byte_identical():
    base = _item("q1", "single-session-user")
    changed = copy.deepcopy(base)
    changed["answer"] = "Lyon"
    changed["answer_session_ids"] = ["sharegpt_s1"]
    changed["haystack_session_ids"] = ["answer_zz", "sharegpt_zz"]
    for session in changed["haystack_sessions"]:
        for turn in session:
            turn["has_answer"] = not turn.get("has_answer", False)

    def outputs(item: dict) -> tuple[str, str, str]:
        case = c3.convert_question(item)
        ids = [d["id"] for d in case["docs"]]
        evidence = c3.render_evidence(case["docs"], ids, max_item_chars=4000, max_total_chars=20000)
        messages = c3.answer_messages(case, evidence)
        return json.dumps(case["docs"]), evidence, json.dumps(messages)

    assert outputs(base) == outputs(changed)


def test_gold_marks_session_and_turn_relevant_rounds():
    gold = c3.gold_for(_item("q1", "single-session-user"))
    assert gold["session_relevant"] == ["s001-r000", "s001-r001"]
    assert gold["turn_relevant"] == ["s001-r000"]
    assert gold["abstention"] is False
    assert c3.gold_for(_item("q1_abs", "multi-session"))["abstention"] is True


def test_evidence_truncation_is_per_item_then_total():
    docs = [{"id": "a", "body": "x" * 50, "date": "d1"}, {"id": "b", "body": "y" * 50, "date": "d2"}]
    text = c3.render_evidence(docs, ["b", "a"], max_item_chars=10, max_total_chars=15)
    assert text.startswith("[Session date: d2]\n" + "y" * 10)
    assert "x" * 5 in text and "x" * 6 not in text


def test_retrieval_scores_match_hand_computed_values():
    scores = c3.retrieval_scores(["n1", "a2", "n3", "a1"], ["a1", "a2", "a9"], ["a1"], k=5)
    assert scores == {
        "session_recall_any_at_5": 1.0,
        "session_mrr_at_5": 0.5,
        "turn_recall_any_at_5": 1.0,
        "turn_mrr_at_5": 0.25,
    }
    assert c3.retrieval_scores(["n1"], ["a1"], [], k=5)["turn_recall_any_at_5"] is None


def test_judge_prompt_uses_type_and_abstention_templates():
    temporal = c3.judge_messages("temporal-reasoning", "Q", "18 days", "19 days", abstention=False)
    assert "off-by-one" in temporal[0]["content"]
    abstain = c3.judge_messages("multi-session", "Q", "not mentioned", "I don't know", abstention=True)
    assert "unanswerable" in abstain[0]["content"]
    assert c3.parse_verdict("Yes.") is True and c3.parse_verdict("no") is False


def test_paired_summary_counts_discordant_pairs():
    rows = [
        {"question_type": "t1", "fathomdb": True, "lancedb": True},
        {"question_type": "t1", "fathomdb": True, "lancedb": False},
        {"question_type": "t2", "fathomdb": False, "lancedb": True},
        {"question_type": "t2", "fathomdb": False, "lancedb": False},
    ]
    summary = c3.paired_summary(rows, ("fathomdb", "lancedb"), seed=1)
    assert summary["n"] == 4
    assert summary["accuracy"] == {"fathomdb": 0.5, "lancedb": 0.5}
    assert summary["discordant"] == {"fathomdb_only": 1, "lancedb_only": 1}
    assert summary["difference"] == 0.0
    assert summary["per_type"]["t1"]["fathomdb"]["correct"] == 2


@pytest.mark.parametrize("arm", ["fathomdb", "lancedb"])
def test_real_store_retrieves_the_answer_round_first(arm: str, tmp_path: Path):
    pytest.importorskip(arm)
    case = c3.convert_question(_item("q1", "single-session-user"))
    case["question"] = "moved Paris"
    result = c3.retrieve(arm, case, tmp_path, top_k=5, fathomdb_bin=None)
    assert result["ids"][0] == "s001-r000"
    assert result["latency_ms"] > 0 and len(result["repeat_latency_ms"]) == c3.REPEATS


class _StubClient:
    def __init__(self) -> None:
        self.calls: list[str] = []

    def complete(self, model: str, messages: list[dict[str, str]], max_tokens: int, **_: object) -> c3.Reply:
        self.calls.append(model)
        content = "yes" if "Answer yes or no only" in messages[-1]["content"] else "Paris"
        return c3.Reply(content, prompt_tokens=100, completion_tokens=2)


def test_run_writes_record_and_resumes_without_new_calls(tmp_path: Path):
    pytest.importorskip("fathomdb")
    pytest.importorskip("lancedb")
    dataset = tmp_path / "lme.json"
    dataset.write_text(json.dumps([_item("q1", "single-session-user"), _item("q2_abs", "multi-session")]))
    config = c3.load_config(c3.DEFAULT_CONFIG)
    config["corpus"]["sha256"] = c3.sha256_file(dataset)
    config["selection"]["question_ids"] = ["q1", "q2_abs"]
    client = _StubClient()
    attempt = tmp_path / "attempt"
    kwargs = {"dataset_path": dataset, "attempt_root": attempt, "client": client, "base_dir": tmp_path / "exp",
              "fathomdb_bin": None, "config_path": None}
    run_id, run_dir = c3.run(config, **kwargs)
    assert len(client.calls) == 8
    record = json.loads((run_dir / "record.json").read_text())
    assert record["metrics"]["answer"]["n"] == 2
    assert record["metrics"]["answer"]["accuracy"] == {"fathomdb": 1.0, "lancedb": 1.0}
    assert len((tmp_path / "exp" / "index.jsonl").read_text().splitlines()) == 1
    c3.run(config, **kwargs)
    assert len(client.calls) == 8


# --- Slice 40 Unit D2: v2 config, three arms, hybrid gate ---------------------

V2_CONFIG = c3.DEFAULT_CONFIG.parent / "hermes-answer.v2.json"


def test_v2_config_keeps_v1_inputs_and_widens_to_three_arms_at_k10():
    v1 = c3.load_config(c3.DEFAULT_CONFIG)
    v2 = c3.load_config(V2_CONFIG)
    assert v2["schema_version"] == "comp-03.answer.v2"
    assert v2["selection"]["question_ids"] == v1["selection"]["question_ids"]
    for key in ("corpus", "document_unit", "evidence", "live", "metrics"):
        assert v2[key] == v1[key], key
    assert v2["retrieval"]["top_k"] == 10
    assert [arm["id"] for arm in v2["arms"]] == ["fathomdb-fts", "fathomdb-hybrid", "lancedb-fts"]
    assert [arm["product_arm"] for arm in v2["arms"]] == ["fathomdb", "fathomdb-hybrid", "lancedb"]
    gated = [arm["id"] for arm in v2["arms"] if arm.get("requires_sweep_verdict") == "adopt"]
    assert gated == ["fathomdb-hybrid"]


def test_arm_order_rotation_reduces_to_the_v1_alternation_for_two_arms():
    two = ("fathomdb", "lancedb")
    for index in range(6):
        assert tuple(c3.arm_order(index, two)) == (two if index % 2 == 0 else two[::-1])


def test_arm_order_rotation_puts_each_of_three_arms_in_each_position_once():
    three = ("fathomdb-fts", "fathomdb-hybrid", "lancedb-fts")
    for start in range(0, 9, 3):
        orders = [tuple(c3.arm_order(index, three)) for index in range(start, start + 3)]
        for position in range(3):
            assert sorted(order[position] for order in orders) == sorted(three)


def _require_default_embedder(tmp_path: Path) -> None:
    from fathomdb import Engine

    try:
        Engine.open(str(tmp_path / "probe.sqlite"), use_default_embedder=True).close()
    except Exception as exc:  # noqa: BLE001 - the reason is printed in the skip
        reason = f"installed fathomdb wheel cannot open the default embedder: {type(exc).__name__}: {exc}"
        print(reason)
        pytest.skip(reason)


def test_gold_changes_leave_hybrid_retrieval_byte_identical(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("FATHOMDB_EMBED_DEVICE", "cpu")
    _require_default_embedder(tmp_path)
    base = _item("q1", "single-session-user")
    changed = copy.deepcopy(base)
    changed["answer"] = "Lyon"
    changed["answer_session_ids"] = ["sharegpt_s1"]
    changed["haystack_session_ids"] = ["answer_zz", "sharegpt_zz"]
    for session in changed["haystack_sessions"]:
        for turn in session:
            turn["has_answer"] = not turn.get("has_answer", False)
    results = []
    for name, item in (("base", base), ("changed", changed)):
        case = c3.convert_question(item)
        results.append(c3.retrieve("fathomdb-hybrid", case, tmp_path / name, top_k=10, fathomdb_bin=None)["ids"])
    assert results[0] == results[1]
    assert results[0], "the hybrid arm returns rounds"


def _gate_run(tmp_path: Path, verdict: str) -> tuple[_StubClient, Path]:
    dataset = tmp_path / "lme.json"
    dataset.write_text(json.dumps([_item("q1", "single-session-user"), _item("q2", "multi-session")]))
    config = c3.load_config(V2_CONFIG)
    config["corpus"]["sha256"] = c3.sha256_file(dataset)
    config["selection"]["question_ids"] = ["q1", "q2"]
    config["arms"] = [arm for arm in config["arms"] if arm["id"] != "lancedb-fts"]
    summary = tmp_path / "sweep-summary.json"
    summary.write_text(json.dumps({"decision": {"verdict": verdict, "clauses": {"a": True, "b": True, "c": True}}}))
    client = _StubClient()
    attempt = tmp_path / "attempt"
    c3.run(config, dataset_path=dataset, attempt_root=attempt, client=client, base_dir=tmp_path / "exp",
           fathomdb_bin=None, config_path=None, sweep_summary=summary)
    return client, attempt


def test_hybrid_arm_is_skipped_unless_the_sweep_verdict_is_adopt(tmp_path: Path):
    pytest.importorskip("fathomdb")
    client, attempt = _gate_run(tmp_path, "reject")
    assert len(client.calls) == 4, "two questions x one arm x answer and judge"
    assert not list(attempt.glob("questions/*/fathomdb-hybrid"))
    retrieval = json.loads((attempt / "questions" / "q1" / "fathomdb-fts" / "retrieval.json").read_text())
    assert len(retrieval["ids"]) <= 10 and retrieval["arm"] == "fathomdb"


def test_hybrid_arm_runs_when_the_sweep_verdict_is_adopt(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("FATHOMDB_EMBED_DEVICE", "cpu")
    _require_default_embedder(tmp_path)
    client, attempt = _gate_run(tmp_path, "adopt")
    assert len(client.calls) == 8, "two questions x two arms x answer and judge"
    assert (attempt / "questions" / "q1" / "fathomdb-hybrid" / "retrieval.json").exists()
