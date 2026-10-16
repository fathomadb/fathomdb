"""Slice 40 Unit G: evidence budget, session completion (offline) and recency rendering."""

from __future__ import annotations

import json


from experiments import comp03_answer as c3

BASELINE = {"per_round_chars": 4000, "total_chars": 20000, "session_expansion": False}


def _sweep():
    from experiments import comp03_retrieval

    return comp03_retrieval


def _docs() -> list[dict]:
    """Three sessions; session s001 holds the gold, s002 a distractor."""
    def doc(session: int, position: int, body: str, date: str) -> dict:
        return {"id": f"s{session:03d}-r{position:03d}", "date": date, "roles": ["user"], "body": body}

    return [
        doc(0, 0, "a" * 30, "2023/01/01"),
        doc(0, 1, "b" * 30, "2023/01/01"),
        doc(1, 0, "c" * 30, "2023/03/01"),
        doc(1, 1, "d" * 30, "2023/03/01"),
        doc(1, 2, "e" * 30, "2023/03/01"),
        doc(2, 0, "f" * 30, "2023/02/01"),
        doc(2, 1, "g" * 30, "2023/02/01"),
    ]


GOLD = {"session_relevant": ["s001-r000", "s001-r001", "s001-r002"], "turn_relevant": ["s001-r002"]}


# --- G1: caps ---------------------------------------------------------------------


def test_caps_apply_per_round_then_total_in_rank_order():
    policy = {"per_round_chars": 20, "total_chars": 50, "session_expansion": False}
    shown = _sweep().evidence_rounds(["s002-r000", "s001-r001", "s000-r000"], _docs(), policy)
    assert shown == [("s002-r000", 20), ("s001-r001", 20), ("s000-r000", 10)]


def test_the_baseline_policy_matches_the_answer_harness_renderer():
    docs = [{**d, "body": d["body"] * 300} for d in _docs()]
    ids = ["s001-r000", "s002-r000", "s000-r000", "s001-r002", "s000-r001", "s002-r001"]
    shown = _sweep().evidence_rounds(ids, docs, BASELINE)
    rendered = c3.render_evidence(docs, ids, max_item_chars=4000, max_total_chars=20000)
    assert len(shown) == rendered.count("[Session date:")
    assert sum(chars for _, chars in shown) == 20000


# --- G2: session expansion ---------------------------------------------------------


def test_expansion_appends_remaining_rounds_of_hit_sessions_by_best_hit_rank():
    policy = {"per_round_chars": 1000, "total_chars": 1000, "session_expansion": True}
    shown = _sweep().evidence_rounds(["s002-r001", "s001-r001", "s002-r000"], _docs(), policy)
    assert [round_id for round_id, _ in shown] == [
        "s002-r001", "s001-r001", "s002-r000",   # the top-k, rank order
        "s001-r000", "s001-r002",                  # s002 is complete; then s001 by position
    ]


def test_expansion_stops_at_the_total_cap():
    policy = {"per_round_chars": 30, "total_chars": 100, "session_expansion": True}
    shown = _sweep().evidence_rounds(["s001-r001"], _docs(), policy)
    assert shown == [("s001-r001", 30), ("s001-r000", 30), ("s001-r002", 30)]
    capped = _sweep().evidence_rounds(["s001-r001", "s000-r000"], _docs(), policy)
    assert capped == [("s001-r001", 30), ("s000-r000", 30), ("s001-r000", 30), ("s001-r002", 10)]


# --- G1/G2 metrics and the rule ------------------------------------------------------


def test_prompt_metrics_count_gold_sessions_and_answer_rounds():
    metrics = _sweep().evidence_metrics([("s001-r000", 30), ("s002-r000", 30)], GOLD)
    assert metrics == {"gold_session_any_in_prompt": 1, "gold_session_all_in_prompt": 1,
                       "has_answer_in_prompt": 0, "rounds_in_prompt": 2, "chars": 60}
    two_sessions = {"session_relevant": ["s001-r000", "s002-r000"], "turn_relevant": ["s002-r000"]}
    partial = _sweep().evidence_metrics([("s001-r000", 30)], two_sessions)
    assert partial["gold_session_any_in_prompt"] == 1 and partial["gold_session_all_in_prompt"] == 0


def _policy_row(all_overall: int, any_by_type: dict, chars: int) -> dict:
    return {"overall": {"gold_session_all_in_prompt": all_overall, "chars": chars},
            "per_type": {t: {"gold_session_any_in_prompt": v} for t, v in any_by_type.items()}}


def test_rule_picks_the_highest_all_in_prompt_without_losing_more_than_one_question_per_type():
    base = "4000/20000/off"
    table = {
        base: _policy_row(300, {"ms": 100, "pref": 25}, 9_000_000),
        "2000/30000/on": _policy_row(340, {"ms": 101, "pref": 24}, 13_000_000),
        "1000/30000/on": _policy_row(360, {"ms": 98, "pref": 25}, 12_000_000),
    }
    verdict = _sweep().select_evidence_policy(table, baseline=base)
    assert verdict["policy"] == "2000/30000/on", "1000/30000/on loses two multi-session questions"
    assert verdict["excluded"] == {"1000/30000/on": ["ms"]}


def test_rule_breaks_ties_on_fewer_chars_and_keeps_the_baseline_when_nothing_beats_it():
    base = "4000/20000/off"
    tied = {
        base: _policy_row(300, {"ms": 100}, 9_000_000),
        "2000/30000/on": _policy_row(340, {"ms": 100}, 13_000_000),
        "4000/30000/on": _policy_row(340, {"ms": 100}, 12_500_000),
    }
    assert _sweep().select_evidence_policy(tied, baseline=base)["policy"] == "4000/30000/on"
    worse = {base: _policy_row(300, {"ms": 100}, 9_000_000), "1000/20000/off": _policy_row(290, {"ms": 100}, 5_000_000)}
    assert _sweep().select_evidence_policy(worse, baseline=base)["policy"] == base


def test_policy_grid_is_the_stated_twelve_policies():
    grid = _sweep().evidence_policy_grid()
    assert len(grid) == 12
    assert {p["per_round_chars"] for p in grid} == {4000, 2000, 1000}
    assert {p["total_chars"] for p in grid} == {20000, 30000}
    assert {p["session_expansion"] for p in grid} == {False, True}
    assert _sweep().policy_key(BASELINE) == "4000/20000/off"


# --- G3: recency rendering -------------------------------------------------------------


def test_chronological_rendering_orders_by_session_date_newest_last_and_keeps_dates():
    docs = _docs()
    ids = ["s001-r000", "s000-r000", "s002-r000"]
    text = c3.render_evidence(docs, ids, max_item_chars=4000, max_total_chars=20000, chronological=True)
    blocks = text.split("\n\n")
    assert [b.splitlines()[0] for b in blocks] == [
        "[Session date: 2023/01/01]", "[Session date: 2023/02/01]", "[Session date: 2023/03/01]",
    ]


def test_chronological_rendering_selects_the_same_rounds_as_rank_order_under_the_cap():
    docs = _docs()
    ids = ["s001-r000", "s000-r000", "s002-r000"]
    rank = c3.render_evidence(docs, ids, max_item_chars=4000, max_total_chars=45)
    chrono = c3.render_evidence(docs, ids, max_item_chars=4000, max_total_chars=45, chronological=True)
    assert sorted(rank.split("\n\n")) == sorted(chrono.split("\n\n"))


def test_recency_sentence_is_added_only_when_enabled_and_v2_rendering_is_unchanged():
    case = c3.convert_question({
        "question_id": "q1", "question_type": "knowledge-update", "question": "Where do I live?",
        "question_date": "2023/06/01", "answer": "Lyon", "answer_session_ids": [],
        "haystack_dates": ["2023/01/01"], "haystack_session_ids": ["s"],
        "haystack_sessions": [[{"role": "user", "content": "I live in Paris."}]],
    })
    evidence = "[Session date: 2023/01/01]\nI live in Paris."
    plain = c3.answer_messages(case, evidence)
    assert c3.answer_messages(case, evidence, recency_instruction=False) == plain
    assert c3.RECENCY_SENTENCE not in plain[0]["content"]
    with_recency = c3.answer_messages(case, evidence, recency_instruction=True)
    assert c3.RECENCY_SENTENCE in with_recency[0]["content"]
    assert with_recency[0]["content"].replace(" " + c3.RECENCY_SENTENCE, "") == plain[0]["content"]
    assert c3.RECENCY_SENTENCE == (
        "When the history states different values for the same fact at different dates, "
        "use the most recent one."
    )
    docs = _docs()
    ids = ["s001-r000", "s000-r000"]
    assert c3.render_evidence(docs, ids, max_item_chars=4000, max_total_chars=20000, chronological=False) == \
        c3.render_evidence(docs, ids, max_item_chars=4000, max_total_chars=20000)


def test_recency_rule_adopts_unless_ku_plus_temporal_drops_or_overall_drops_by_more_than_one():
    def rows(correct: dict[str, list[bool]]) -> list[dict]:
        out = []
        for qtype, values in correct.items():
            out += [{"question_id": f"{qtype}-{i}", "question_type": qtype, "arm": value}
                    for i, value in enumerate(values)]
        return out

    v3 = rows({"knowledge-update": [True, False], "temporal-reasoning": [True, True], "multi-session": [True, True]})
    better = rows({"knowledge-update": [True, True], "temporal-reasoning": [True, True], "multi-session": [True, False]})
    worse_ku = rows({"knowledge-update": [False, False], "temporal-reasoning": [True, True], "multi-session": [True, True]})
    worse_all = rows({"knowledge-update": [True, False], "temporal-reasoning": [True, True], "multi-session": [False, False]})
    assert c3.evaluate_recency_rule(v3, better, arm="arm", seed=0)["verdict"] == "adopt"
    assert c3.evaluate_recency_rule(v3, worse_ku, arm="arm", seed=0)["verdict"] == "reject"
    assert c3.evaluate_recency_rule(v3, worse_all, arm="arm", seed=0)["verdict"] == "reject"
    report = c3.evaluate_recency_rule(v3, better, arm="arm", seed=0)
    assert set(report) >= {"verdict", "ku_temporal_delta", "overall_delta", "overall_ci"}


# --- answer configs v3 and v4 -------------------------------------------------------------


def test_v3_and_v4_configs_take_the_policy_from_the_verdict_and_differ_only_in_recency():
    v2 = c3.load_config(c3.DEFAULT_CONFIG.parent / "hermes-answer.v2.json")
    v3 = c3.load_config(c3.DEFAULT_CONFIG.parent / "hermes-answer.v3.json")
    v4 = c3.load_config(c3.DEFAULT_CONFIG.parent / "hermes-answer.v4.json")
    assert v3["schema_version"] == v4["schema_version"] == "comp-03.answer.v3"
    for key in ("corpus", "selection", "document_unit", "arms", "retrieval", "live"):
        assert v3[key] == v2[key], key
    policy = v3["evidence"]["policy"]
    assert set(policy) == {"per_round_chars", "total_chars", "session_expansion"}
    assert len(v3["evidence"]["policy_verdict_sha256"]) == 64
    assert v3["evidence"]["chronological"] is False and v3["evidence"]["recency_instruction"] is False
    assert v4["evidence"] == {**v3["evidence"], "chronological": True, "recency_instruction": True}
    assert {k: v for k, v in v4.items() if k not in {"evidence", "decision_rule", "claim_boundary"}} == \
        {k: v for k, v in v3.items() if k not in {"evidence", "decision_rule", "claim_boundary"}}
    assert v3["decision_rule"]["compare_to"] == "comp-03.answer.v2"
    assert v4["decision_rule"]["compare_to"] == "comp-03.answer.v3"


def test_evidence_verdict_file_names_its_inputs():
    path = c3.DEFAULT_CONFIG.parent / "evidence-policy-verdict.v1.json"
    verdict = json.loads(path.read_text())
    assert verdict["baseline"] == "4000/20000/off"
    assert verdict["policy"] in {_sweep().policy_key(p) for p in _sweep().evidence_policy_grid()}
    assert verdict["sweep_summary_sha256"] and verdict["arms"] and verdict["questions"] == 500
    assert len(verdict["table"]) == 12
