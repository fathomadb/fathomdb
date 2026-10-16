"""Slice 40 Unit G: evidence budget, session completion (offline) and recency rendering.

Revised after the Unit G review (findings F1-F10): the policy rule maximises
has-answer rounds in the prompt under an untruncation constraint, the renderer
and the evaluator share one evidence-rounds implementation, and the recency
change is split into two single factors.
"""

from __future__ import annotations

import json

from experiments import comp03_answer as c3

BASELINE = {"per_round_chars": 4000, "total_chars": 20000, "session_expansion": False}
BASE = "4000/20000/off"


def _sweep():
    from experiments import comp03_retrieval

    return comp03_retrieval


def _doc(session: int, position: int, body: str, date: str) -> dict:
    return {"id": f"s{session:03d}-r{position:03d}", "date": date, "roles": ["user"], "body": body}


def _docs() -> list[dict]:
    """Three sessions; session s001 holds the gold, s002 a distractor."""
    return [
        _doc(0, 0, "a" * 30, "2023/01/01"),
        _doc(0, 1, "b" * 30, "2023/01/01"),
        _doc(1, 0, "c" * 30, "2023/03/01"),
        _doc(1, 1, "d" * 30, "2023/03/01"),
        _doc(1, 2, "e" * 30, "2023/03/01"),
        _doc(2, 0, "f" * 30, "2023/02/01"),
        _doc(2, 1, "g" * 30, "2023/02/01"),
    ]


GOLD = {"session_relevant": ["s001-r000", "s001-r001", "s001-r002"], "turn_relevant": ["s001-r002"]}


# --- G1: caps and expansion ----------------------------------------------------------


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


def test_expansion_enumerates_blank_rounds_at_zero_chars():
    docs = _docs()
    docs[3] = {**docs[3], "body": " \n"}  # s001-r001 is whitespace only
    policy = {"per_round_chars": 1000, "total_chars": 1000, "session_expansion": True}
    shown = _sweep().evidence_rounds(["s001-r000"], docs, policy)
    assert shown == [("s001-r000", 30), ("s001-r001", 0), ("s001-r002", 30)]


# --- G1 metrics ------------------------------------------------------------------------


def test_prompt_metrics_count_gold_sessions_and_answer_rounds():
    metrics = _sweep().evidence_metrics([("s001-r000", 30), ("s002-r000", 30)], GOLD, _docs())
    assert metrics == {
        "gold_session_any_in_prompt": 1, "gold_session_all_in_prompt": 1,
        "has_answer_in_prompt": 0, "has_answer_untruncated_in_prompt": 0,
        "gold_chars_shown_fraction": 30 / 90, "rounds_in_prompt": 2, "chars": 60,
    }
    two_sessions = {"session_relevant": ["s001-r000", "s002-r000"], "turn_relevant": ["s002-r000"]}
    partial = _sweep().evidence_metrics([("s001-r000", 30)], two_sessions, _docs())
    assert partial["gold_session_any_in_prompt"] == 1 and partial["gold_session_all_in_prompt"] == 0


def test_untruncated_and_char_fraction_metrics_see_a_cut_answer_round():
    cut = _sweep().evidence_metrics([("s001-r002", 10), ("s002-r000", 0)], GOLD, _docs())
    assert cut["has_answer_in_prompt"] == 1
    assert cut["has_answer_untruncated_in_prompt"] == 0
    assert cut["gold_chars_shown_fraction"] == 10 / 90
    assert cut["rounds_in_prompt"] == 1, "a round shown with 0 characters is not in the prompt"
    full = _sweep().evidence_metrics([("s001-r002", 30)], GOLD, _docs())
    assert full["has_answer_untruncated_in_prompt"] == 1


def test_policy_grid_is_the_stated_twelve_policies():
    grid = _sweep().evidence_policy_grid()
    assert len(grid) == 12
    assert {p["per_round_chars"] for p in grid} == {4000, 2000, 1000}
    assert {p["total_chars"] for p in grid} == {20000, 30000}
    assert {p["session_expansion"] for p in grid} == {False, True}
    assert _sweep().policy_key(BASELINE) == BASE


# --- G2 rule -----------------------------------------------------------------------------


def _row(answer_in_prompt: int, untruncated_by_type: dict, chars: int) -> dict:
    return {"overall": {"has_answer_in_prompt": answer_in_prompt, "chars": chars},
            "per_type": {t: {"has_answer_untruncated_in_prompt": v} for t, v in untruncated_by_type.items()}}


def test_rule_maximises_has_answer_and_rejects_policies_that_cut_answer_rounds():
    pooled = {
        BASE: _row(1321, {"ms": 300, "pref": 80}, 27_000_000),
        "4000/30000/on": _row(1344, {"ms": 305, "pref": 81}, 41_800_000),
        "1000/30000/on": _row(1369, {"ms": 40, "pref": 10}, 36_200_000),
        "1000/20000/off": _row(1334, {"ms": 40, "pref": 10}, 13_700_000),
    }
    verdict = _sweep().select_evidence_policy(pooled, baseline=BASE)
    assert verdict["policy"] == "4000/30000/on"
    assert verdict["excluded"] == {"1000/30000/on": ["ms", "pref"], "1000/20000/off": ["ms", "pref"]}, \
        "the highest-count and the lowest-chars candidates both lose on untruncation"


def test_pooled_tolerance_is_three_counts_and_fts_tolerance_is_one():
    pooled = {BASE: _row(100, {"ms": 50}, 10), "a": _row(110, {"ms": 47}, 20), "b": _row(120, {"ms": 46}, 20)}
    assert _sweep().select_evidence_policy(pooled, baseline=BASE)["policy"] == "a", "3 allowed, 4 not"
    fts = {BASE: _row(40, {"ms": 20}, 4), "a": _row(42, {"ms": 18}, 7), "b": _row(44, {"ms": 19}, 7)}
    pooled_ok = {BASE: _row(100, {"ms": 50}, 10), "a": _row(110, {"ms": 50}, 20), "b": _row(105, {"ms": 50}, 20)}
    verdict = _sweep().select_evidence_policy(pooled_ok, baseline=BASE, fts=fts)
    assert verdict["policy"] == "b", "a drops two fathomdb-fts questions; one is allowed"
    assert verdict["excluded"] == {"a": ["fathomdb-fts:ms"]}


def test_rule_also_holds_for_fathomdb_fts_alone_and_falls_back_when_the_pooled_winner_breaks_it():
    pooled = {
        BASE: _row(900, {"ms": 300}, 27_000_000),
        "1000/30000/on": _row(1000, {"ms": 300}, 30_000_000),
        "2000/30000/on": _row(960, {"ms": 300}, 33_000_000),
    }
    fts = {
        BASE: _row(300, {"ms": 100}, 9_000_000),
        "1000/30000/on": _row(330, {"ms": 98}, 10_000_000),
        "2000/30000/on": _row(320, {"ms": 99}, 11_000_000),
    }
    verdict = _sweep().select_evidence_policy(pooled, baseline=BASE, fts=fts)
    assert verdict["policy"] == "2000/30000/on", "the pooled winner loses two fts multi-session questions"
    assert verdict["excluded"] == {"1000/30000/on": ["fathomdb-fts:ms"]}


def test_rule_breaks_ties_on_fewer_chars_and_keeps_the_baseline_when_nothing_beats_it():
    tied = {
        BASE: _row(300, {"ms": 100}, 9_000_000),
        "2000/30000/on": _row(340, {"ms": 100}, 13_000_000),
        "4000/30000/on": _row(340, {"ms": 100}, 12_500_000),
    }
    assert _sweep().select_evidence_policy(tied, baseline=BASE)["policy"] == "4000/30000/on"
    worse = {BASE: _row(300, {"ms": 100}, 9_000_000), "1000/20000/off": _row(290, {"ms": 100}, 5_000_000)}
    assert _sweep().select_evidence_policy(worse, baseline=BASE)["policy"] == BASE


# --- G4: one implementation --------------------------------------------------------------


def test_renderer_shows_exactly_the_evaluator_rounds_with_expansion():
    docs = _docs()
    docs[3] = {**docs[3], "body": " \n"}
    policy = {"per_round_chars": 25, "total_chars": 80, "session_expansion": True}
    ids = ["s001-r000", "s002-r000"]
    shown = _sweep().evidence_rounds(ids, docs, policy)
    by_id = {d["id"]: d for d in docs}
    expected = "\n\n".join(
        f"[Session date: {by_id[rid]['date']}]\n{by_id[rid]['body'][:chars]}" for rid, chars in shown if chars > 0
    )
    rendered = c3.render_evidence(docs, ids, policy=policy)
    assert rendered == expected


def test_render_evidence_refuses_a_policy_together_with_caps():
    policy = {"per_round_chars": 25, "total_chars": 80, "session_expansion": True}
    for caps in ({"max_item_chars": 4000}, {"max_total_chars": 20000},
                 {"max_item_chars": 4000, "max_total_chars": 20000}):
        try:
            c3.render_evidence(_docs(), ["s001-r000"], policy=policy, **caps)
        except ValueError:
            continue
        raise AssertionError(f"a policy with {caps} must be refused")


def test_evidence_record_digests_the_expanded_text_and_counts_it():
    case = {"docs": _docs()}
    config = {"policy": {"per_round_chars": 1000, "total_chars": 1000, "session_expansion": True},
              "chronological": False, "recency_instruction": False}
    record = c3.evidence_record(case, ["s001-r001"], config)
    assert record["rounds_in_prompt"] == 3 and record["chars"] == 90
    import hashlib

    assert record["evidence_sha256"] == hashlib.sha256(record["text"].encode("utf-8")).hexdigest()
    assert record["text"].count("[Session date:") == 3


# --- G3: recency, two single factors -------------------------------------------------------


def test_chronological_rendering_orders_by_session_date_newest_last_and_keeps_dates():
    docs = _docs()
    ids = ["s001-r000", "s000-r000", "s002-r000"]
    text = c3.render_evidence(docs, ids, max_item_chars=4000, max_total_chars=20000, chronological=True)
    blocks = text.split("\n\n")
    assert [b.splitlines()[0] for b in blocks] == [
        "[Session date: 2023/01/01]", "[Session date: 2023/02/01]", "[Session date: 2023/03/01]",
    ]


def test_chronological_rendering_breaks_date_ties_by_session_then_round_position():
    docs = [
        _doc(0, 0, "x" * 5, "2023/05/05"),
        _doc(2, 0, "p" * 5, "2023/05/05"),
        _doc(2, 1, "q" * 5, "2023/05/05"),
    ]
    text = c3.render_evidence(docs, ["s002-r001", "s000-r000", "s002-r000"], max_item_chars=4000,
                              max_total_chars=20000, chronological=True)
    assert [b.splitlines()[1] for b in text.split("\n\n")] == ["xxxxx", "ppppp", "qqqqq"]


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


def _rows(correct: dict[str, list[bool]]) -> list[dict]:
    out = []
    for qtype, values in correct.items():
        out += [{"question_id": f"{qtype}-{i}", "question_type": qtype, "arm": value}
                for i, value in enumerate(values)]
    return out


def test_recency_rule_adopts_unless_ku_plus_temporal_drops_or_overall_drops_by_more_than_one():
    base = _rows({"knowledge-update": [True, False], "temporal-reasoning": [True, True], "multi-session": [True, True]})
    better = _rows({"knowledge-update": [True, True], "temporal-reasoning": [True, True], "multi-session": [True, False]})
    worse_ku = _rows({"knowledge-update": [False, False], "temporal-reasoning": [True, True], "multi-session": [True, True]})
    worse_all = _rows({"knowledge-update": [True, False], "temporal-reasoning": [True, True], "multi-session": [False, False]})
    assert c3.evaluate_recency_rule(base, better, arm="arm", seed=0)["verdict"] == "adopt"
    assert c3.evaluate_recency_rule(base, worse_ku, arm="arm", seed=0)["verdict"] == "reject"
    assert c3.evaluate_recency_rule(base, worse_all, arm="arm", seed=0)["verdict"] == "reject"
    report = c3.evaluate_recency_rule(base, better, arm="arm", seed=0)
    assert set(report) >= {"verdict", "ku_temporal_delta", "overall_delta", "overall_ci", "per_type", "provisional"}
    assert report["provisional"] is True
    assert report["per_type"]["knowledge-update"] == {"base": 1, "new": 2}


def test_recency_rule_boundaries():
    base = _rows({"knowledge-update": [True, False], "temporal-reasoning": [True, True], "multi-session": [True, True]})
    overall_minus_one = _rows({"knowledge-update": [True, False], "temporal-reasoning": [True, True],
                               "multi-session": [True, False]})
    swap = _rows({"knowledge-update": [True, True], "temporal-reasoning": [True, False], "multi-session": [True, True]})
    assert c3.evaluate_recency_rule(base, overall_minus_one, arm="arm", seed=0)["verdict"] == "adopt"
    assert c3.evaluate_recency_rule(base, swap, arm="arm", seed=0)["verdict"] == "adopt"


# --- configs, the verdict file and the mix ----------------------------------------------------


def test_v3_v4a_v4b_v4c_configs_add_one_factor_each():
    configs = {name: c3.load_config(c3.DEFAULT_CONFIG.parent / f"hermes-answer.{name}.json")
               for name in ("v2", "v3", "v4a", "v4b", "v4c")}
    v2, v3, v4a, v4b, v4c = (configs[k] for k in ("v2", "v3", "v4a", "v4b", "v4c"))
    assert {c["schema_version"] for c in (v3, v4a, v4b, v4c)} == {"comp-03.answer.v3"}
    for key in ("corpus", "selection", "document_unit", "arms", "retrieval", "live"):
        assert v3[key] == v2[key] == v4a[key] == v4b[key] == v4c[key], key
    assert set(v3["evidence"]["policy"]) == {"per_round_chars", "total_chars", "session_expansion"}
    assert len(v3["evidence"]["policy_verdict_sha256"]) == 64
    assert (v3["evidence"]["recency_instruction"], v3["evidence"]["chronological"]) == (False, False)
    assert v4a["evidence"] == {**v3["evidence"], "recency_instruction": True}
    assert v4b["evidence"] == {**v4a["evidence"], "chronological": True}
    assert v4c["evidence"] == {**v3["evidence"], "chronological": True}
    assert [c["decision_rule"]["compare_to"] for c in (v3, v4a, v4b, v4c)] == [
        "hermes-answer.v2", "hermes-answer.v3", "hermes-answer.v4a", "hermes-answer.v3",
    ]
    for config in (v3, v4a, v4b, v4c):
        assert config["decision_rule"]["provisional"] is True
        assert config["decision_rule"]["mde_net_discordant"] == 6
        assert config["decision_rule"]["arm_of_record"] == "fathomdb-fts"


def test_evidence_verdict_file_names_its_inputs():
    verdict = json.loads((c3.DEFAULT_CONFIG.parent / "evidence-policy-verdict.v1.json").read_text())
    assert verdict["baseline"] == BASE
    assert verdict["policy"] in {_sweep().policy_key(p) for p in _sweep().evidence_policy_grid()}
    assert verdict["questions"] == {"scored": 470, "selection": 414, "answer_set_ids": 60, "answer_set_scored": 56}
    assert len(verdict["dataset_sha256"]) == 64 and len(verdict["sweep_summary_sha256"]) == 64
    assert verdict["arms"] == ["fathomdb-fts", "fathomdb-hybrid", "lancedb-fts"]
    for subset in ("selection", "answer_set"):
        assert len(verdict["tables"][subset]["pooled"]) == 12
        assert set(verdict["tables"][subset]["per_arm"]) == set(verdict["arms"])
    expectation = verdict["answer_set_expectation"]
    assert expectation["policy"] == verdict["policy"] and expectation["baseline"] == BASE
    assert set(expectation["pooled"]) >= {"has_answer_in_prompt", "has_answer_untruncated_in_prompt", "chars"}
    assert set(expectation["per_arm"]) == set(verdict["arms"])


def _factor(fts: str, others: str = "adopt", sha: str = "2") -> dict:
    return {"sha256": sha * 64, "arms": {"fathomdb-fts": {"verdict": fts},
                                         "fathomdb-hybrid": {"verdict": others},
                                         "lancedb-fts": {"verdict": others}}}


V3 = {"schema_version": "comp-03.answer.v3", "selection": {"question_ids": ["a"]},
      "evidence": {"policy": dict(BASELINE), "policy_verdict_sha256": "0" * 64,
                   "chronological": False, "recency_instruction": False},
      "decision_rule": {"compare_to": "hermes-answer.v2"}}
POLICY = {"policy": "4000/30000/on", "sha256": "1" * 64}


def _flags(config: dict) -> tuple[bool, bool]:
    return config["evidence"]["recency_instruction"], config["evidence"]["chronological"]


def test_mix_config_follows_the_three_verdicts():
    both = c3.build_mix_config(V3, POLICY, _factor("adopt"), _factor("adopt"))
    assert both["evidence"]["policy"] == {"per_round_chars": 4000, "total_chars": 30000, "session_expansion": True}
    assert _flags(both) == (True, True)
    assert both["decision_rule"]["compare_to"] == "hermes-answer.v2"
    assert both["decision_rule"]["arm_of_record"] == "fathomdb-fts"
    assert set(both["decision_rule"]["sources"]) == {"policy_verdict", "v4a_verdict", "v4b_verdict"}
    assert _flags(c3.build_mix_config(V3, POLICY, _factor("adopt"), _factor("reject"))) == (True, False)
    assert _flags(c3.build_mix_config(V3, POLICY, _factor("reject"), _factor("reject"))) == (False, False)
    assert V3["evidence"]["recency_instruction"] is False, "the input config is not mutated"


def test_mix_config_follows_the_arm_of_record():
    mix = c3.build_mix_config(V3, POLICY, _factor("adopt", others="reject"), _factor("adopt", others="reject"))
    assert _flags(mix) == (True, True), "fathomdb-fts adopts; the other arms are reported, not used"
    lance = c3.build_mix_config(V3, POLICY, _factor("adopt", others="reject"), _factor("adopt", others="reject"),
                                arm="lancedb-fts")
    assert _flags(lance) == (False, False) and lance["decision_rule"]["arm_of_record"] == "lancedb-fts"


def test_mix_config_needs_v4c_for_chronological_order_alone():
    try:
        c3.build_mix_config(V3, POLICY, _factor("reject"), _factor("adopt"))
    except ValueError:
        pass
    else:
        raise AssertionError("sentence rejected but chronological adopted needs a v4c verdict")
    alone = c3.build_mix_config(V3, POLICY, _factor("reject"), _factor("adopt"), _factor("adopt", sha="4"))
    assert _flags(alone) == (False, True)
    assert set(alone["decision_rule"]["sources"]) == {"policy_verdict", "v4a_verdict", "v4b_verdict", "v4c_verdict"}
    assert _flags(c3.build_mix_config(V3, POLICY, _factor("reject"), _factor("adopt"), _factor("reject"))) == \
        (False, False)
