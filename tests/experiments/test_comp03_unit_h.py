"""Measurement-correctness checks for the Slice 40 Unit H selection rules.

The H1 stop-list rule and the H2 vector-pool rule run here on hand-built
question tables and hand-written per-word effects; none reads the real sweep.
Each fixture's expected list or pool follows from the rule as written.
"""

from __future__ import annotations

import json
import string
from pathlib import Path

import pytest
from hypothesis import given, settings
from hypothesis import strategies as st

from experiments import _lib
from experiments import comp03_answer as c3

CONFIG_DIR = _lib.EXPERIMENTS_DIR / "configs" / "comp-03"
PREF, MULTI, TEMP = "single-session-preference", "multi-session", "temporal-reasoning"

# The engine's STOPWORDS at 5b09afd44: the 51 original words plus the twelve of REQ40-D5.
ENGINE_STOPWORDS_5B09AFD44 = [
    "the", "and", "for", "are", "was", "were", "what", "when", "where", "who", "whom", "which",
    "how", "why", "did", "does", "do", "is", "of", "to", "in", "on", "at", "by", "an", "a", "it",
    "its", "this", "that", "these", "those", "with", "from", "as", "be", "or", "if", "about",
    "into", "over", "than", "then", "they", "them", "their", "you", "your", "we", "our", "i", "s",
    "t", "ll", "re", "ve", "m", "my", "me", "us", "am", "so", "up",
]
CANDIDATES_47 = [
    "many", "much", "have", "had", "has", "can", "could", "would", "should", "some", "any", "recommend",
    "recommendations", "suggest", "suggestions", "give", "tell", "know", "think", "want", "need", "get", "got",
    "total", "number", "different", "also", "been", "being", "just", "still", "currently", "recently", "there",
    "all", "based", "help", "please", "going", "make", "use", "used", "using", "far", "long", "often", "again",
]
N1_CLAUSE_C = (
    "point delta ≥ +1 question AND paired-bootstrap CI lower bound ≥ −1 question "
    "(bootstrap_ci, seed 404, 10,000 resamples) on Recall-all@10 over the selection set"
)


def _h():
    from experiments import comp03_unit_h

    return comp03_unit_h


def _stoplist_config(**rule_overrides) -> dict:
    config = _h().load_stoplist_config(CONFIG_DIR / "stoplist-selection.v1.json")
    config["rule"].update(rule_overrides)
    return config


# --- fake measurement: per-word flips over a hand-built question table ----------------------

class FakeMeasure:
    """Recall-all@10 and Recall-any@5 per question; each word flips named questions.

    ``effects[w] = {"all": {qid: +1|-1}, "any5": {qid: +1|-1}, "ndcg": d, "mrr": d}``.
    A list's outcome is the baseline plus every word's flips, clamped to [0, 1].
    """

    def __init__(self, base: dict, effects: dict, ndcg: float = 0.3265, mrr: float = 0.5312):
        self.base, self.effects, self.ndcg, self.mrr = base, effects, ndcg, mrr
        self.calls: list[list[frozenset]] = []

    def __call__(self, lists):
        self.calls.append([frozenset(words) for words in lists])
        return [self._one(frozenset(words)) for words in lists]

    def _one(self, words):
        questions = {}
        for qid, row in self.base.items():
            out = dict(row)
            for metric, key in (("session_recall_all_at_10", "all"), ("session_recall_any_at_5", "any5")):
                value = row[metric] + sum(self.effects.get(w, {}).get(key, {}).get(qid, 0) for w in words)
                out[metric] = float(min(1, max(0, value)))
            questions[qid] = out
        return {
            "questions": questions,
            "nfcorpus": {"ndcg_at_10": self.ndcg + sum(self.effects.get(w, {}).get("ndcg", 0.0) for w in words),
                         "mrr_at_10": self.mrr + sum(self.effects.get(w, {}).get("mrr", 0.0) for w in words)},
        }


def _base(n_per_type: int = 10, all_hits: tuple[str, ...] = ()) -> dict:
    """Selection questions at 1 on Recall-any@5 and at 0 on Recall-all@10 except ``all_hits``; one answer-set question."""
    base = {}
    for qtype, prefix in ((PREF, "p"), (MULTI, "m"), (TEMP, "t")):
        for i in range(n_per_type):
            qid = f"{prefix}{i}"
            base[qid] = {"question_type": qtype, "subset": "selection",
                         "session_recall_all_at_10": 1.0 if qid in all_hits else 0.0, "session_recall_any_at_5": 1.0}
    base["a0"] = {"question_type": PREF, "subset": "answer_set",
                  "session_recall_all_at_10": 0.0, "session_recall_any_at_5": 1.0}
    return base


def _queries(*extra: str) -> dict:
    return {"lme:q0": "where did I buy my new tennis racket", **{f"x:{i}": q for i, q in enumerate(extra)}}


def _select(config, measure, queries=None):
    return _h().select_stoplist(config, measure, queries if queries is not None else _queries())


def _small_config(candidates, **rule):
    config = _stoplist_config(**rule)
    config["candidates"] = list(candidates)
    return config


# --- AC40-70: config -----------------------------------------------------------------


def test_stoplist_selection_config_states_the_candidates_and_rule(tmp_path: Path):
    h = _h()
    config = h.load_stoplist_config(CONFIG_DIR / "stoplist-selection.v1.json")
    assert config["schema_version"] == "comp-03.stoplist-selection.v1"
    assert config["candidates"] == CANDIDATES_47 and len(config["candidates"]) == 47
    assert config["engine_stopwords"] == ENGINE_STOPWORDS_5B09AFD44 and len(config["engine_stopwords"]) == 63
    assert "d" not in config["candidates"] and "ve" not in config["candidates"]
    assert not set(config["candidates"]) & set(config["engine_stopwords"])
    assert config["engine_commit"] == "5b09afd44112251ee634e034edf0368bbdce684d"
    assert config["selection"]["expected"] == {"scored": 470, "selection": 414, "answer_set_scored": 56,
                                               "nfcorpus_queries": 323}
    assert config["corpus"]["sha256"] == "d6f21ea9d60a0d56f34a05b609c79c88a451d2ae03597821ea3d5a9678c3a442"
    assert config["arm"] == {"id": "fathomdb-fts", "product_arm": "fathomdb", "method": "Engine.search_text_only",
                             "top_k": 10}
    rule = config["rule"]
    assert N1_CLAUSE_C in rule["text"], "clause (c) is owner ruling N1, verbatim"
    assert "empt" in rule["text"] and "each candidate alone" in rule["text"] and "every list" in rule["text"]
    assert rule["keep_min_delta_all"] == 1
    assert rule["nfcorpus_floor"] == {"ndcg_at_10": {"baseline": 0.3265, "tolerance": 0.002},
                                      "mrr_at_10": {"baseline": 0.5312, "tolerance": 0.005}}
    assert rule["per_type_metric"] == "session_recall_any_at_5" and rule["per_type_tolerance_questions"] == 1
    assert rule["combined_metric"] == "session_recall_all_at_10"
    assert rule["combined_point_min_questions"] == 1 and rule["combined_ci_lower_min_questions"] == -1
    assert rule["bootstrap"] == {"seed": 404, "resamples": 10000}
    bad = dict(config, schema_version="comp-03.stoplist-selection.v0")
    path = tmp_path / "bad.json"
    path.write_text(json.dumps(bad))
    with pytest.raises(h.Comp03UnitHError):
        h.load_stoplist_config(path)
    extra = dict(config, surprise=True)
    path.write_text(json.dumps(extra))
    with pytest.raises(h.Comp03UnitHError):
        h.load_stoplist_config(path)
    wrong_candidates = dict(config, candidates=[*config["candidates"], "the"])
    path.write_text(json.dumps(wrong_candidates))
    with pytest.raises(h.Comp03UnitHError):
        h.load_stoplist_config(path)


# --- AC40-71: simulation ---------------------------------------------------------------

WORDS = st.text(alphabet=string.ascii_lowercase, min_size=1, max_size=6)
QUERY_CHARS = string.ascii_letters + string.digits + " ?'-.,\"*:()"


@settings(max_examples=300, deadline=None)
@given(raw=st.text(alphabet=QUERY_CHARS, max_size=60), words=st.sets(WORDS, max_size=6))
def test_stripping_words_equals_compiling_with_them_as_stopwords(raw, words):
    h = _h()
    d3 = ENGINE_STOPWORDS_5B09AFD44
    stripped = h.strip_words(raw, words)
    assert h.content_tokens(stripped, d3) == h.content_tokens(raw, [*d3, *words])
    if words and h.content_tokens(raw, [*d3, *words]):
        assert h.simulated_query(raw, words, d3) == stripped
    assert h.simulated_query(raw, set(), d3) == raw


def test_content_tokens_mirror_the_engine_compiler():
    h = _h()
    d3 = ENGINE_STOPWORDS_5B09AFD44
    assert h.content_tokens("How many siblings do I have?", d3) == ["many", "siblings", "have"]
    assert h.content_tokens("Vitamin C-Enriched Bacon and vitamin c", d3) == ["vitamin", "c", "enriched", "bacon"]
    assert h.content_tokens("don't", d3) == ["don"]
    assert h.strip_words("How MANY siblings do I have?", {"many", "have"}) == "how siblings do i"


def test_a_query_without_content_under_d3_keeps_the_raw_fallback():
    h = _h()
    d3 = ENGINE_STOPWORDS_5B09AFD44
    assert h.content_tokens("What is it?", d3) == []
    assert not h.empties("What is it?", {"many"}, d3), "already the raw fallback under D3; unchanged by any list"
    assert h.simulated_query("What is it?", {"many"}, d3) == "What is it?"
    assert h.empties("can you help", {"can", "help"}, d3)
    assert not h.empties("can you help", {"can"}, d3)


def test_a_word_that_empties_a_query_is_excluded():
    base = _base()
    effects = {
        "please": {"all": {"p0": 1, "p1": 1}},       # gains, but empties "please" alone
        "can": {"all": {"m0": 1}, "ndcg": -0.0001},  # jointly empties "can you help" with "help"
        "help": {"all": {"m1": 1}, "ndcg": 0.0},
        "many": {"all": {"t0": 1}},
    }
    config = _small_config(["please", "can", "help", "many"])
    queries = _queries("please", "can you help")
    result = _select(config, FakeMeasure(base, effects), queries)
    words = result["per_word"]
    assert words["please"]["excluded_empty"] == ["x:0"] and words["please"]["kept"] is False
    assert words["can"]["excluded_empty"] == [] and words["help"]["excluded_empty"] == []
    assert "can" not in result["list"], "the lower-ΔnDCG word of the jointly emptied query is pruned"
    assert result["list"] == ["help", "many"]
    assert result["pruning"][0] == {"word": "can", "reason": "empties", "queries": ["x:1"]}
    assert result["verdict"] == "adopted"


# --- AC40-72: rule ------------------------------------------------------------------


def test_stoplist_rule_keeps_gaining_words_and_prunes_until_constraints_hold():
    base = _base(all_hits=("t2", "t3"))
    effects = {
        "many": {"all": {"p0": 1, "p1": 1}, "ndcg": -0.0005},
        "have": {"all": {"m0": 1}, "ndcg": -0.0020},   # (a) fails with "many": pruned first
        "some": {"all": {"t0": 1}, "ndcg": 0.0001},
        "just": {"all": {"t1": 1, "t2": -1}},          # Δall 0: not kept
        "tell": {"all": {"t3": -1}},                   # Δall −1: not kept
    }
    config = _small_config(["many", "have", "some", "just", "tell"])
    measure = FakeMeasure(base, effects)
    result = _select(config, measure)
    table = result["per_word"]
    assert {w: table[w]["delta_all"] for w in table} == {"many": 2, "have": 1, "some": 1, "just": 0, "tell": -1}
    assert table["many"]["delta_ndcg"] == pytest.approx(-0.0005)
    assert [w for w in table if table[w]["kept"]] == ["many", "have", "some"]
    assert result["pruning"] == [{"word": "have", "reason": "clause a or b", "failed": ["a"]}]
    assert result["list"] == ["many", "some"]
    assert result["clauses"]["a"] is True and result["clauses"]["b"] is True and result["clauses"]["c"] is True
    assert result["combined"]["point_delta_questions"] == 3
    assert result["verdict"] == "adopted"
    assert measure.calls[0][0] == frozenset(), "the baseline (D3 list) is measured with the per-word step"
    assert set(measure.calls[0][1:]) == {frozenset({w}) for w in config["candidates"]}


def test_stoplist_rule_prunes_on_a_per_type_loss_and_breaks_ties_by_delta_all_then_name():
    base = _base()
    effects = {
        "bb": {"all": {"p0": 1, "p1": 1}, "any5": {"m0": -1}, "ndcg": -0.001},
        "aa": {"all": {"p2": 1, "p3": 1}, "any5": {"m1": -1}, "ndcg": -0.001},
        "cc": {"all": {"p4": 1}, "ndcg": -0.001},
        "dd": {"all": {"t0": 1}},
    }
    result = _select(_small_config(["aa", "bb", "cc", "dd"]), FakeMeasure(base, effects))
    # Two multi-session losses fail (b). Lowest ΔnDCG ties at −0.001 among aa, bb, cc; lowest Δall is cc (1).
    # After cc, (b) still fails; aa and bb tie on both, so aa goes first alphabetically.
    assert [step["word"] for step in result["pruning"]] == ["cc", "aa"]
    assert result["list"] == ["bb", "dd"]
    assert result["verdict"] == "adopted"


def test_stoplist_rule_boundaries():
    h = _h()
    rule = _stoplist_config()["rule"]
    floor = rule["nfcorpus_floor"]
    exact = {"ndcg_at_10": floor["ndcg_at_10"]["baseline"] - floor["ndcg_at_10"]["tolerance"],
             "mrr_at_10": floor["mrr_at_10"]["baseline"] - floor["mrr_at_10"]["tolerance"]}
    assert h.clause_a_holds(exact, rule)
    assert not h.clause_a_holds({**exact, "ndcg_at_10": 0.3265 - 0.0021}, rule)
    assert not h.clause_a_holds({**exact, "mrr_at_10": 0.5312 - 0.0051}, rule)
    assert h.clause_b_holds({PREF: 9, MULTI: 10}, {PREF: 10, MULTI: 10}, rule), "one question down passes"
    assert not h.clause_b_holds({PREF: 8, MULTI: 10}, {PREF: 10, MULTI: 10}, rule), "two down prunes"
    # Owner ruling N1: point delta >= +1 question and CI lower bound >= -1 question.
    assert h.clause_c_holds(1, -1.0, rule)
    assert h.clause_c_holds(1, -1.0 + 1e-13, rule)
    assert not h.clause_c_holds(1, -1.0000001, rule)
    assert not h.clause_c_holds(1, -2.0, rule)
    assert not h.clause_c_holds(0, 0.0, rule)
    assert h.clause_c_holds(2, 0.5, rule)
    # The CI is bootstrap_ci over per-question differences, in questions (the mean bound times n).
    diffs = [1.0, 1.0, 1.0, -1.0, -1.0] + [0.0] * 5
    combined = h.combined_clause_c(diffs, rule)
    from experiments import comp03_retrieval

    ci = comp03_retrieval.bootstrap_ci(diffs, seed=404, resamples=10000)
    assert combined["point_delta_questions"] == 1
    assert combined["ci_questions"] == pytest.approx([ci[0] * 10, ci[1] * 10])
    assert combined["ci_questions"][0] < -1 and combined["holds"] is False


def test_stoplist_rule_rejects_a_list_whose_ci_lower_bound_is_below_minus_one_question():
    base = _base()
    effects = {"many": {"all": {"p0": 1, "p1": 1, "p2": 1, "m0": -1, "m1": -1}}}
    base_ = {**base, "m0": {**base["m0"], "session_recall_all_at_10": 1.0},
             "m1": {**base["m1"], "session_recall_all_at_10": 1.0}}
    result = _select(_small_config(["many"]), FakeMeasure(base_, effects))
    assert result["per_word"]["many"]["delta_all"] == 1 and result["per_word"]["many"]["kept"] is True
    assert result["combined"]["point_delta_questions"] == 1
    assert result["combined"]["ci_questions"][0] < -1
    assert result["clauses"]["c"] is False and result["verdict"] == "not_adopted"


def test_stoplist_rule_is_not_adopted_when_nothing_gains():
    base = _base(all_hits=("p1", "m0"))
    effects = {"many": {"all": {"p0": 1, "p1": -1}}, "have": {"all": {"m0": -1}}}
    measure = FakeMeasure(base, effects)
    result = _select(_small_config(["many", "have"]), measure)
    assert result["verdict"] == "not_adopted" and result["list"] == []
    assert result["reason"] == "no word kept"
    assert len(measure.calls) == 1, "no combined list is measured when K is empty"
    pruned_to_nothing = _select(_small_config(["many"]),
                                FakeMeasure(base, {"many": {"all": {"p0": 1}, "ndcg": -0.01}}))
    assert pruned_to_nothing["verdict"] == "not_adopted" and pruned_to_nothing["reason"] == "K emptied by pruning"


# --- AC40-73: verdict file ----------------------------------------------------------


def test_stoplist_verdict_names_its_inputs():
    h = _h()
    config = _small_config(["many"])
    base = _base()
    result = _select(config, FakeMeasure(base, {"many": {"all": {"p0": 1, "p1": 1}}}))
    inputs = {"dataset_sha256": "d" * 64, "nfcorpus_files": {"corpus.json": "c" * 64},
              "answer_config": "experiments/configs/comp-03/hermes-answer.v2.json", "answer_config_sha256": "a" * 64,
              "engine": {"commit": "5" * 40, "native_sha256": "n" * 64, "version": "0.8.27"},
              "questions": {"scored": 470, "selection": 414, "answer_set_scored": 56, "nfcorpus_queries": 323},
              "runner": "implementer, CPU", "summary_sha256": "s" * 64}
    verdict = h.stoplist_verdict(config, result, inputs)
    assert verdict["schema_version"] == "comp-03.stoplist-verdict.v1"
    assert verdict["verdict"] == "adopted" and verdict["words"] == ["many"]
    assert verdict["inputs"]["questions"] == {"scored": 470, "selection": 414, "answer_set_scored": 56,
                                              "nfcorpus_queries": 323}
    assert verdict["inputs"]["engine"]["commit"] == "5" * 40 and verdict["inputs"]["runner"] == "implementer, CPU"
    assert verdict["per_word"]["many"]["delta_all"] == 2
    assert verdict["rule"] == config["rule"]["text"]
    assert {"baseline", "final"} <= set(verdict["combined_metrics"])
    assert "answer_set" in verdict["combined_metrics"]["final"], "the answer set is reported, not used"


def test_stoplist_measurement_scores_the_selection_and_answer_sets_separately():
    h = _h()
    gold = {"q1": {"abstention": False, "session_relevant": ["s000-r000"], "turn_relevant": []},
            "q2": {"abstention": False, "session_relevant": ["s001-r000"], "turn_relevant": []},
            "q3_abs": {"abstention": True, "session_relevant": [], "turn_relevant": []}}
    types = {"q1": PREF, "q2": MULTI, "q3_abs": MULTI}
    scored = h.score_questions({"q1": ["s000-r000"], "q2": ["x"], "q3_abs": ["x"]}, gold, types, answer_ids={"q2"})
    assert set(scored) == {"q1", "q2"}
    assert scored["q1"]["subset"] == "selection" and scored["q2"]["subset"] == "answer_set"
    assert scored["q1"]["session_recall_all_at_10"] == 1.0 and scored["q2"]["session_recall_any_at_5"] == 0.0


# --- AC40-77: pool config --------------------------------------------------------------


def _pool_config():
    return _h().load_pool_config(CONFIG_DIR / "vector-pool-selection.v1.json")


def test_vector_pool_selection_config_is_typed_and_states_the_rule(tmp_path: Path, monkeypatch):
    h = _h()
    config = _pool_config()
    assert config["schema_version"] == "comp-03.vector-pool-selection.v1"
    assert config["candidates"] == [10, 20, 50, 100] and config["reference"] == 10
    assert config["arm"]["product_arm"] == "fathomdb-hybrid" and config["arm"]["retrieval"] == "comp03_answer.retrieve"
    assert config["ks"] == [5, 10, 20]
    assert config["constraints"] == {
        "per_type": ["session_recall_any_at_5", "session_recall_any_at_10"],
        "overall": ["session_recall_any_at_5", "session_recall_any_at_10", "session_recall_all_at_10"],
        "tolerance_questions": 1,
    }
    assert "largest" in config["rule"] and "minus 1" in config["rule"]
    path = tmp_path / "bad.json"
    path.write_text(json.dumps(dict(config, reference=20)))
    with pytest.raises(h.Comp03UnitHError):
        h.load_pool_config(path)

    # The pool command calls the sweep's own retrieval function at top_k = P, for every P.
    item = {
        "question_id": "q1", "question_type": PREF, "question": "What pet did I adopt?",
        "question_date": "2023/06/01 (Thu) 10:00", "answer": "a dog", "answer_session_ids": ["answer_s1"],
        "haystack_dates": ["2023/05/01 (Mon) 09:00"], "haystack_session_ids": ["answer_s1"],
        "haystack_sessions": [[{"role": "user", "content": "I adopted a dog.", "has_answer": True}]],
    }
    dataset = tmp_path / "data.json"
    dataset.write_text(json.dumps([item]))
    config = dict(config, corpus=dict(config["corpus"], sha256=c3.sha256_file(dataset)))
    calls = []

    def fake_retrieve(arm_name, case, workdir, *, top_k, fathomdb_bin, store_label=None):
        calls.append((arm_name, case["question_id"], top_k))
        return {"ids": [f"s000-r{i:03d}" for i in range(top_k)], "documents": 1, "vector_rows": 1}

    monkeypatch.setattr(c3, "retrieve", fake_retrieve)
    rows = h.run_pool(config, dataset_path=dataset, attempt_root=tmp_path / "attempt")
    assert sorted(calls) == [("fathomdb-hybrid", "q1", p) for p in (10, 20, 50, 100)]
    assert rows["q1"]["ids"]["100"][:3] == ["s000-r000", "s000-r001", "s000-r002"]
    calls.clear()
    h.run_pool(config, dataset_path=dataset, attempt_root=tmp_path / "attempt")
    assert calls == [], "resume does no new work"


# --- AC40-78: pool metrics and rule -------------------------------------------------------


def _pool_rows():
    """Six selection questions, two per type; pool 100 brings a second session in late."""
    gold = {}
    rows = {}
    for qid, qtype in (("p0", PREF), ("p1", PREF), ("m0", MULTI), ("m1", MULTI), ("t0", TEMP), ("t1", TEMP)):
        gold[qid] = {"abstention": False, "session_relevant": ["s001-r000", "s002-r000"], "turn_relevant": []}
        rows[qid] = {"question_type": qtype, "subset": "selection", "ids": {}}
    return gold, rows


def test_pool_metrics_use_prefixes_of_the_limit_p_ranking():
    h = _h()
    gold, rows = _pool_rows()
    rows["p0"]["ids"] = {"10": ["x"] * 5 + ["s001-r000"] + ["x"] * 4,
                         "20": ["s001-r000"] + ["x"] * 4 + ["s002-r000"] + ["x"] * 14}
    table = h.pool_table({"p0": rows["p0"]}, {"p0": gold["p0"]}, candidates=[10, 20], ks=[5, 10, 20])
    assert table["10"]["overall"]["session_recall_any_at_5"] == 0
    assert table["10"]["overall"]["session_recall_any_at_10"] == 1
    assert table["20"]["overall"]["session_recall_any_at_5"] == 1
    assert table["20"]["overall"]["session_recall_all_at_10"] == 1, "first ten of the limit-20 list"
    assert table["20"]["per_type"][PREF]["session_recall_any_at_5"] == 1
    assert table["10"]["overall"]["session_recall_all_at_20"] == 0, "k = 20 on the limit-10 list sees ten ids"


def _rows_with(per_p: dict[int, dict[str, list[str]]]):
    gold, rows = _pool_rows()
    for p, by_q in per_p.items():
        for qid, ids in by_q.items():
            rows[qid]["ids"][str(p)] = ids
    return gold, rows


HIT = ["s001-r000"] + ["x"] * 99
MISS = ["x"] * 100


def _fill(hits: set[str], p: int) -> dict[str, list[str]]:
    return {q: (HIT if q in hits else MISS)[:p] for q in ("p0", "p1", "m0", "m1", "t0", "t1")}


def test_pool_rule_picks_the_largest_pool_that_keeps_recall():
    h = _h()
    everyone = {"p0", "p1", "m0", "m1", "t0", "t1"}
    gold, rows = _rows_with({10: _fill(everyone, 10), 20: _fill(everyone, 20),
                             50: _fill(everyone - {"m0"}, 50), 100: _fill(everyone - {"m0", "m1"}, 100)})
    choice = h.select_pool(_pool_config(), h.pool_table(rows, gold, candidates=[10, 20, 50, 100], ks=[5, 10, 20]))
    assert choice["pool"] == 50, "100 loses two multi-session questions; 50 loses one"
    assert choice["excluded"] == {"100": ["per_type multi-session session_recall_any_at_5",
                                          "per_type multi-session session_recall_any_at_10",
                                          "overall session_recall_any_at_5",
                                          "overall session_recall_any_at_10"]}


def test_pool_rule_boundaries():
    h = _h()
    everyone = {"p0", "p1", "m0", "m1", "t0", "t1"}
    gold, rows = _rows_with({10: _fill(everyone, 10), 20: _fill(everyone - {"p0"}, 20),
                             50: _fill(everyone - {"p0", "p1"}, 50), 100: _fill(everyone - {"p0", "m0", "t0"}, 100)})
    table = h.pool_table(rows, gold, candidates=[10, 20, 50, 100], ks=[5, 10, 20])
    choice = h.select_pool(_pool_config(), table)
    assert choice["pool"] == 20, "one question down on a type passes; two on a type (50) or overall (100) fails"
    assert set(choice["excluded"]) == {"50", "100"}
    gold2, rows2 = _rows_with({10: _fill(everyone, 10), 20: _fill(set(), 20), 50: _fill(set(), 50),
                               100: _fill(set(), 100)})
    assert h.select_pool(_pool_config(), h.pool_table(rows2, gold2, candidates=[10, 20, 50, 100],
                                                      ks=[5, 10, 20]))["pool"] == 10, "the reference always passes"


def test_pool_verdict_records_the_table_ids_and_runner():
    h = _h()
    everyone = {"p0", "p1", "m0", "m1", "t0", "t1"}
    gold, rows = _rows_with({p: _fill(everyone, p) for p in (10, 20, 50, 100)})
    rows["a0"] = {"question_type": PREF, "subset": "answer_set", "ids": {str(p): HIT[:p] for p in (10, 20, 50, 100)}}
    gold["a0"] = gold["p0"]
    config = _pool_config()
    verdict = h.pool_verdict(config, rows, gold, {"runner": "orchestrator, cuda:0", "engine": {"commit": "5" * 40},
                                                   "device": {"requested": "cuda:0", "effective": "cuda"}})
    assert verdict["schema_version"] == "comp-03.vector-pool-verdict.v1"
    assert verdict["pool"] == 100 and verdict["inputs"]["runner"] == "orchestrator, cuda:0"
    assert set(verdict["tables"]) == {"selection", "answer_set"}
    assert verdict["tables"]["selection"]["100"]["overall"]["n"] == 6
    assert verdict["ids"]["p0"]["100"] == HIT[:20], "ids recorded per P, to the deepest k the rule reads"
