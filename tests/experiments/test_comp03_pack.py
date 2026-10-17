"""Measurement-correctness checks for the 0.8.28 Slice 41 COMP-03 harness steps.

REQ41-Q8 (query stop words through ``Engine.open``, MQ1), the ``pack_context``
evidence path (AC41-27, M1, MF1) and the pack answer-run config (M3). Items,
gold and expected coverage are hand-built; stores are real FathomDB on CPU.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from experiments import comp03_answer as c3
from experiments import comp_fts

WORDS = ["had", "long", "suggest"]
STOP_CONFIG = {"query_stop_words": {"base": "fathomdb", "extra": WORDS}}
CONFIG_DIR = c3.DEFAULT_CONFIG.parent
SSA = "single-session-assistant"


def _pack():
    from experiments import comp03_pack

    return comp03_pack


def _retrieval():
    from experiments import comp03_retrieval

    return comp03_retrieval


def _engine_stopwords() -> list[str]:
    return json.loads((CONFIG_DIR / "stoplist-selection.v1.json").read_text())["engine_stopwords"]


def _set_sha(words) -> str:
    return hashlib.sha256("\n".join(sorted(set(words))).encode("utf-8")).hexdigest()


def _item(qid: str = "q1", qtype: str = "single-session-user", *, question: str = "What pet had I adopted?") -> dict:
    return {
        "question_id": qid,
        "question_type": qtype,
        "question": question,
        "question_date": "2023/06/01 (Thu) 10:00",
        "answer": "a golden retriever",
        "answer_session_ids": ["answer_s1"],
        "haystack_dates": ["2023/04/01 (Sat) 09:00", "2023/05/01 (Mon) 09:00", "2023/05/01 (Mon) 09:00"],
        "haystack_session_ids": ["sharegpt_s0", "answer_s1", "sharegpt_s2"],
        "haystack_sessions": [
            [
                {"role": "user", "content": "Recommend a pasta recipe I had long wanted."},
                {"role": "assistant", "content": "Try carbonara with guanciale. " * 20},
            ],
            [
                {"role": "user", "content": "I adopted a golden retriever puppy!", "has_answer": True},
                {"role": "assistant", "content": "Congratulations on the new dog. " * 30},
                {"role": "user", "content": "   "},
                {"role": "user", "content": "What should I name the pet?"},
                {"role": "assistant", "content": "You could call the pet Biscuit.", "has_answer": True},
            ],
            [
                {"role": "user", "content": "Suggest a long hike."},
                {"role": "assistant", "content": "The ridge trail is long."},
            ],
        ],
    }


def _dataset(tmp_path: Path, items: list[dict]) -> Path:
    path = tmp_path / "lme.json"
    path.write_text(json.dumps(items))
    return path


# --- REQ41-Q8: query stop words through Engine.open -------------------------------------


def test_configured_arm_passes_the_setting_and_leaves_the_registry_arm_alone(tmp_path: Path):
    pytest.importorskip("fathomdb")
    assert comp_fts.configured_arm("fathomdb", None) is comp_fts.ARMS["fathomdb"]
    arm = comp_fts.configured_arm("fathomdb", STOP_CONFIG)
    assert arm is not comp_fts.ARMS["fathomdb"]
    assert arm.settings() == {**comp_fts.ARMS["fathomdb"].settings(), "engine_config": STOP_CONFIG}
    root = tmp_path / "store"
    root.mkdir()
    arm.build(root, [{"id": "d0", "body": "I had a long walk"}])
    handle = arm.open(root)
    try:
        assert handle.open_report().stop_words_sha256 == _set_sha([*_engine_stopwords(), *WORDS])
    finally:
        arm.close(handle)
    default = comp_fts.ARMS["fathomdb"].open(root)
    try:
        assert default.open_report().stop_words_sha256 == _set_sha(_engine_stopwords())
    finally:
        default.close()


def test_configured_arm_refuses_an_unknown_setting_or_a_non_fathomdb_arm():
    with pytest.raises(ValueError):
        comp_fts.configured_arm("fathomdb", {"slow_threshold_ms": 5})
    with pytest.raises(ValueError):
        comp_fts.configured_arm("lancedb", STOP_CONFIG)


def test_retrieve_records_limits_variants_hashes_and_the_mq1_transform(tmp_path: Path):
    pytest.importorskip("fathomdb")
    sweep = _retrieval()
    case = c3.convert_question(_item(question="Suggest the pet I had for a long time?"))
    result = c3.retrieve(
        "fathomdb", case, tmp_path, top_k=2, fathomdb_bin=None, extra_limits=(5,),
        variants=[{"id": "fathomdb-fts-stopwords", "engine_config": STOP_CONFIG}],
        transform={"words": WORDS, "engine_stopwords": _engine_stopwords()},
    )
    assert result["ids_at"]["2"] == result["ids"]
    assert result["ids_at"]["5"][:2] == result["ids"], "FTS is prefix-stable"
    assert result["stop_words_sha256"] == _set_sha(_engine_stopwords())
    variant = result["variants"]["fathomdb-fts-stopwords"]
    assert variant["engine_config"] == STOP_CONFIG
    assert variant["stop_words_sha256"] == _set_sha([*_engine_stopwords(), *WORDS])
    assert set(variant["ids_at"]) == {"2", "5"} and variant["ids_at"]["2"] == variant["ids"]
    from experiments.comp03_unit_h import simulated_query

    assert result["transform"]["query"] == simulated_query(case["question"], WORDS, _engine_stopwords())
    assert result["transform"]["ids"] == variant["ids"], "MQ1: the setting equals the harness transform"
    assert sweep.derived_result(result, "fathomdb-fts-stopwords")["ids"] == variant["ids"]


def test_retrieve_with_engine_config_opens_the_main_store_with_it(tmp_path: Path):
    pytest.importorskip("fathomdb")
    case = c3.convert_question(_item())
    result = c3.retrieve("fathomdb", case, tmp_path, top_k=3, fathomdb_bin=None, engine_config=STOP_CONFIG)
    assert result["engine_config"] == STOP_CONFIG
    assert result["stop_words_sha256"] == _set_sha([*_engine_stopwords(), *WORDS])


def test_v2_sweep_config_adds_stopword_variants_limits_and_mq1():
    sweep = _retrieval()
    config = sweep.load_config(CONFIG_DIR / "retrieval-sweep.v2.json")
    assert config["schema_version"] == "comp-03.retrieval-sweep.v2"
    assert [arm["id"] for arm in config["arms"]] == ["fathomdb-fts", "fathomdb-hybrid", "lancedb-fts"]
    variants = {arm["id"]: [v["id"] for v in arm.get("variants", [])] for arm in config["arms"]}
    assert variants == {"fathomdb-fts": ["fathomdb-fts-stopwords"],
                        "fathomdb-hybrid": ["fathomdb-hybrid-stopwords"], "lancedb-fts": []}
    for arm in config["arms"][:2]:
        assert arm["variants"][0]["engine_config"] == STOP_CONFIG
    assert config["retrieval"]["top_k"] == 10 and config["retrieval"]["extra_limits"] == [30, 50]
    assert config["mq1"]["arm"] == "fathomdb-fts" and config["mq1"]["variant"] == "fathomdb-fts-stopwords"
    assert config["mq1"]["words"] == WORDS
    v1 = sweep.load_config(sweep.DEFAULT_CONFIG)
    assert {k: config[k] for k in ("corpus", "document_unit", "decision_rule", "metrics")} == \
        {k: v1[k] for k in ("corpus", "document_unit", "decision_rule", "metrics")}


def test_mq1_counts_transform_and_simulated_agreement():
    sweep = _retrieval()
    results = {
        "q1": {"transform": {"ids": ["a", "b"]}, "variants": {"v": {"ids": ["a", "b"]}}},
        "q2": {"transform": {"ids": ["a", "b"]}, "variants": {"v": {"ids": ["b", "a"]}}},
        "q3": {"transform": {"ids": ["c"]}, "variants": {"v": {"ids": ["c"]}}},
    }
    table = sweep.mq1_table(results, variant="v", simulated={"q1": ["a", "b"], "q2": ["b", "a"], "q3": ["d"]})
    assert table["questions"] == 3
    assert table["equal_transform"] == 2 and table["differing_transform"] == ["q2"]
    assert table["pass"] is False
    assert table["simulated"] == {"questions": 3, "equal": 2, "informative": True}
    ok = sweep.mq1_table({"q1": results["q1"]}, variant="v", simulated={})
    assert ok["pass"] is True and ok["simulated"]["questions"] == 0


def test_v2_sweep_scores_variants_as_arms_and_writes_mq1(tmp_path: Path, monkeypatch):
    pytest.importorskip("fathomdb")
    sweep = _retrieval()
    items = [_item("q1"), _item("q2", SSA, question="What name did you suggest for the pet I had?")]
    dataset = _dataset(tmp_path, items)
    config = sweep.load_config(CONFIG_DIR / "retrieval-sweep.v2.json")
    config["corpus"]["sha256"] = c3.sha256_file(dataset)
    config["arms"] = [arm for arm in config["arms"] if arm["id"] == "fathomdb-fts"]
    verdict = tmp_path / "verdict.json"
    verdict.write_text(json.dumps({"simulated_fts_ids": {"q1": []}}))
    config["mq1"]["simulated_fts_ids"] = str(verdict)
    attempt = tmp_path / "attempt"
    summary = sweep.sweep(config, dataset_path=dataset, attempt_root=attempt)
    assert set(summary["arms"]) == {"fathomdb-fts", "fathomdb-fts-stopwords"}
    assert summary["arms"]["fathomdb-fts-stopwords"]["overall"]["n"] == 2
    assert summary["arms"]["fathomdb-fts-stopwords"]["stop_words_sha256"] == [_set_sha([*_engine_stopwords(), *WORDS])]
    assert summary["arms"]["fathomdb-fts"]["stop_words_sha256"] == [_set_sha(_engine_stopwords())]
    derived = json.loads((attempt / "questions" / "q2" / "fathomdb-fts-stopwords" / "retrieval.json").read_text())
    parent = json.loads((attempt / "questions" / "q2" / "fathomdb-fts" / "retrieval.json").read_text())
    assert derived["ids"] == parent["variants"]["fathomdb-fts-stopwords"]["ids"]
    assert set(derived["ids_at"]) == {"10", "30", "50"}
    assert summary["mq1"]["questions"] == 2 and summary["mq1"]["pass"] is True
    assert summary["mq1"]["simulated"]["questions"] == 1


def test_mq1_refuses_an_engine_list_that_is_not_the_installed_default(tmp_path: Path):
    pytest.importorskip("fathomdb")
    sweep = _retrieval()
    with pytest.raises(sweep.Comp03RetrievalError):
        sweep.check_engine_stopwords(["the", "and"])
    sweep.check_engine_stopwords(_engine_stopwords())


# --- pack_context evidence (caller items) ------------------------------------------------------


def test_caller_items_have_one_part_per_turn_and_join_to_the_stored_body():
    pack = _pack()
    item = _item()
    items = pack.caller_items(item)
    docs = c3.convert_question(item)["docs"]
    assert [i.id for i in items] == [d["id"] for d in docs]
    for built, doc in zip(items, docs, strict=True):
        assert "\n".join(p.text for p in built.parts) == doc["body"], "REQ41-B2"
        assert [p.role for p in built.parts] == doc["roles"]
        assert built.group == doc["id"][:4] and built.position == int(doc["id"][-3:])
        assert built.order_key == doc["date"] and built.header == f"[Session date: {doc['date']}]"


@pytest.mark.parametrize("ids", [["s001-r000", "s002-r000"], ["s002-r000"], ["s000-r000", "s001-r002"]])
def test_mix_policy_through_pack_context_reproduces_the_slice40_renderer(ids):
    """AC41-27's identity on a hand-built case: expansion, a blank round, date ties, a total cut."""
    pack = _pack()
    item = _item()
    case = c3.convert_question(item)
    mix = {"policy": {"per_round_chars": 4000, "total_chars": 1500, "session_expansion": True},
           "chronological": True}
    expected = c3.evidence_record(case, ids, mix)["text"]
    policy = {**pack.MIX_POLICY, "budget_chars": 1500}
    assert pack.pack_question(item, ids, policy).text == expected


def test_ac41_27_checks_every_recorded_answer_question_and_arm(tmp_path: Path):
    pack = _pack()
    items = [_item("q1"), _item("q2", SSA)]
    dataset = _dataset(tmp_path, items)
    mix = {"policy": {"per_round_chars": 4000, "total_chars": 30000, "session_expansion": True},
           "chronological": True}
    root = tmp_path / "mix"
    for item, ids in zip(items, (["s001-r000"], ["s002-r000", "s000-r000"]), strict=True):
        for arm in ("fathomdb-fts", "lancedb-fts"):
            record = c3.evidence_record(c3.convert_question(item), ids, mix)
            path = root / "questions" / item["question_id"] / arm / "retrieval.json"
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps({"ids": ids, "evidence_sha256": record["evidence_sha256"]}))
    result = pack.ac41_27(root, dataset)
    assert result["arms"] == {"fathomdb-fts": {"checked": 2, "matched": 2, "mismatched": []},
                              "lancedb-fts": {"checked": 2, "matched": 2, "mismatched": []}}
    assert result["pass"] is True


def test_coverage_reads_answer_turns_from_part_entries():
    pack = _pack()
    item = _item()
    # s001-r002 holds the assistant answer turn (part 1); the compact tier keeps it to 0 characters.
    compact = {**pack.MIX_POLICY, "full_items": 1, "compact_chars": 0, "order": "rank"}
    tight = pack.pack_question(item, ["s001-r000", "s001-r002"], compact)
    row = pack.question_coverage(tight, item)
    assert row == {"gold_all": True, "hasA_full": True, "hasA_allfull": False, "rounds": 2}, \
        "the blank sibling expands but is omitted"
    full = pack.pack_question(item, ["s001-r000", "s001-r002"], {**compact, "full_items": 2})
    assert pack.question_coverage(full, item)["hasA_allfull"] is True
    miss = pack.pack_question(item, ["s000-r000"], {**compact, "expand_groups": False})
    assert pack.question_coverage(miss, item) == {"gold_all": False, "hasA_full": False, "hasA_allfull": False,
                                                  "rounds": 1}


def test_coverage_table_weights_per_type_rates():
    pack = _pack()
    rows = [
        {"question_type": SSA, "gold_all": True, "hasA_full": True, "hasA_allfull": True, "rounds": 4},
        {"question_type": SSA, "gold_all": True, "hasA_full": True, "hasA_allfull": False, "rounds": 2},
        {"question_type": "multi-session", "gold_all": False, "hasA_full": True, "hasA_allfull": True, "rounds": 6},
    ]
    table = pack.coverage_table(rows)
    assert table["per_type"][SSA] == {"n": 2, "gold_all": 1.0, "hasA_full": 1.0, "hasA_allfull": 0.5, "rounds": 3.0}
    assert table["weighted"]["hasA_allfull"] == pytest.approx(0.112 * 0.5 + 0.266 * 1.0)
    assert table["n"] == 3 and table["rounds"] == pytest.approx(4.0)


def test_coverage_config_freezes_the_grid_rule_and_mf1():
    pack = _pack()
    config = pack.load_config(pack.DEFAULT_CONFIG)
    grid = {row["name"]: (row["full_items"], row["compact_chars"], row["k"]) for row in config["grid"]}
    assert grid == {"T3/A300/k30": (3, 300, 30), "T3/A0/k30": (3, 0, 30), "T5/A300/k50": (5, 300, 50),
                    "T5/A0/k50": (5, 0, 50)}
    assert config["control"] == {"name": "mix", "full_items": 10**6, "compact_chars": 300, "k": 10,
                                 "order": "chronological"}
    assert config["base_policy"] == {"budget_chars": 30000, "item_chars": 4000, "keep_roles": ["user"],
                                     "expand_groups": True}
    assert config["weights"] == pack.WEIGHTS
    assert config["guard"] == {"ssa_min": 0.89, "other_tolerance": 0.02}
    assert config["rule"]["min_gain"] == 0.05 and config["rule"]["fallback"] == "T3/A300/k30"
    assert [row["name"] for row in config["mf1"]["rows"]] == ["T3/A0/k30", "(c) rank-1 session <= 10",
                                                              "(b) assistant reference -> T8"]


def _tables(**weighted_by_policy):
    """Per-arm M1 tables: every type's allfull equals the weighted value unless overridden."""
    types = sorted(c3.QUESTION_TYPES)
    out = {}
    for name, value in weighted_by_policy.items():
        allfull, ssa = value if isinstance(value, tuple) else (value, value)
        per_type = {t: {"hasA_allfull": allfull} for t in types}
        per_type[SSA] = {"hasA_allfull": ssa}
        out[name] = {"per_type": per_type, "weighted": {"hasA_allfull": allfull}}
    return out


def test_m1_rule_guards_then_maximises_mean_weighted_allfull_with_tie_breaks():
    pack = _pack()
    config = pack.load_config(pack.DEFAULT_CONFIG)
    fts = _tables(mix=0.82, **{"T3/A300/k30": 0.92, "T3/A0/k30": 0.95, "T5/A300/k50": 0.95,
                               "T5/A0/k50": (0.99, 0.80)})
    hyb = _tables(mix=0.82, **{"T3/A300/k30": 0.92, "T3/A0/k30": 0.95, "T5/A300/k50": 0.95, "T5/A0/k50": 0.99})
    choice = pack.select_policy(config, {"fts": fts, "hybrid": hyb})
    assert choice["excluded"] == {"T5/A0/k50": ["fts:SSA"]}
    assert choice["policy"] == "T3/A0/k30", "tie on mean: the smaller k wins"
    assert choice["adopted"] is True
    tie_k = _tables(mix=0.82, **{"T3/A300/k30": 0.95, "T3/A0/k30": 0.95, "T5/A300/k50": 0.9, "T5/A0/k50": 0.9})
    assert pack.select_policy(config, {"fts": tie_k, "hybrid": tie_k})["policy"] == "T3/A300/k30", \
        "same k: the larger compact_chars wins"


def test_m1_rule_needs_a_gain_of_five_points_on_both_arms_else_falls_back():
    pack = _pack()
    config = pack.load_config(pack.DEFAULT_CONFIG)
    fts = _tables(mix=0.82, **{"T3/A300/k30": 0.90, "T3/A0/k30": 0.88, "T5/A300/k50": 0.86, "T5/A0/k50": 0.86})
    hyb = _tables(mix=0.86, **{"T3/A300/k30": 0.90, "T3/A0/k30": 0.88, "T5/A300/k50": 0.86, "T5/A0/k50": 0.86})
    choice = pack.select_policy(config, {"fts": fts, "hybrid": hyb})
    assert choice["best"] == "T3/A300/k30" and choice["adopted"] is False
    assert choice["policy"] == "T3/A300/k30" and choice["reason"].startswith("no policy gains")


def test_m1_guard_holds_other_types_to_the_mix_minus_two_points():
    pack = _pack()
    config = pack.load_config(pack.DEFAULT_CONFIG)
    fts = _tables(mix=0.90, **{"T3/A300/k30": 0.95, "T3/A0/k30": 0.95, "T5/A300/k50": 0.95, "T5/A0/k50": 0.95})
    fts["T3/A0/k30"]["per_type"]["multi-session"]["hasA_allfull"] = 0.879
    choice = pack.select_policy(config, {"fts": fts, "hybrid": fts})
    assert choice["excluded"]["T3/A0/k30"] == ["fts:multi-session", "hybrid:multi-session"]


def test_m1_evaluates_each_policy_at_its_own_k_on_both_subsets(tmp_path: Path):
    pack = _pack()
    items = [_item("q1"), _item("q2", SSA), _item("q3_abs")]
    dataset = _dataset(tmp_path, items)
    root = tmp_path / "sweep"
    for item in items:
        for arm in ("fathomdb-fts", "fathomdb-hybrid"):
            path = root / "questions" / item["question_id"] / arm / "retrieval.json"
            path.parent.mkdir(parents=True)
            ids_at = {"10": ["s001-r000"], "30": ["s001-r000", "s001-r002"], "50": ["s001-r002", "s001-r000"]}
            path.write_text(json.dumps({"ids": ids_at["10"], "ids_at": ids_at}))
    config = pack.load_config(pack.DEFAULT_CONFIG)
    config["arms"] = {"fts": "fathomdb-fts", "hybrid": "fathomdb-hybrid"}
    result = pack.evaluate_m1(config, root, dataset, answer_ids=["q2"])
    assert result["questions"] == {"scored": 2, "selection": 1, "answer_set": 1, "abstention_skipped": 1}
    sel = result["tables"]["selection"]["fts"]
    assert set(sel) == {"mix", "T3/A300/k30", "T3/A0/k30", "T5/A300/k50", "T5/A0/k50"}
    assert sel["mix"]["n"] == 1 and result["tables"]["answer_set"]["hybrid"]["mix"]["n"] == 1
    assert result["inputs"]["dataset_sha256"] == c3.sha256_file(dataset)
    assert "policy" in result["choice"]


# --- MF1 harness rules ------------------------------------------------------------------------------


def test_rank1_session_rule_promotes_siblings_beyond_the_full_tier_up_to_the_cap():
    pack = _pack()
    ids = ["s004-r001", "s002-r000", "s004-r003", "s004-r000", "s001-r000", "s004-r002", "s004-r005"]
    assert pack.rank1_session_full_ids(ids, full_items=2, cap=4) == ["s004-r003", "s004-r000"]
    assert pack.rank1_session_full_ids(ids, full_items=3, cap=10) == ["s004-r000", "s004-r002", "s004-r005"]
    assert pack.rank1_session_full_ids([], full_items=3, cap=10) == []


def test_assistant_reference_rule_raises_full_items_only_when_the_question_refers_to_the_assistant():
    pack = _pack()
    rule = {"name": "assistant_reference", "full_items": 8}
    base = {**pack.MIX_POLICY, "full_items": 3}
    policy, full_ids = pack.apply_rule(rule, base, ["s001-r000"], "What did you suggest for the pet name?")
    assert policy["full_items"] == 8 and full_ids == []
    policy, _ = pack.apply_rule(rule, base, ["s001-r000"], "What pet did I adopt?")
    assert policy["full_items"] == 3
    assert pack.apply_rule(None, base, ["s001-r000"], "anything") == (base, [])


def test_rank1_session_rule_renders_the_promoted_sibling_in_full_and_marks_it_forced():
    pack = _pack()
    item = _item(question="What name did you suggest?")
    base = {**pack.MIX_POLICY, "full_items": 1, "compact_chars": 0, "order": "rank"}
    rule = {"name": "rank1_session", "cap": 10}
    ids = ["s001-r000", "s000-r000", "s001-r002"]
    policy, full_ids = pack.apply_rule(rule, base, ids, item["question"])
    assert full_ids == ["s001-r002"]
    packed = pack.pack_question(item, ids, policy, full_ids=full_ids)
    entry = next(e for e in packed.items if e.id == "s001-r002")
    assert entry.tier == "full" and entry.forced is True
    assert pack.question_coverage(packed, item)["hasA_allfull"] is True


def test_mf1_reads_a_recorded_top_100_file_and_reports_its_rows(tmp_path: Path):
    pack = _pack()
    items = [_item("q1", SSA, question="What name did you suggest for the pet?"), _item("q2")]
    dataset = _dataset(tmp_path, items)
    records = tmp_path / "sweep100.jsonl"
    records.write_text("\n".join(json.dumps({"qid": i["question_id"], "ids100": ["s001-r000", "s000-r000", "s001-r002"]})
                                 for i in items) + "\n")
    config = pack.load_config(pack.DEFAULT_CONFIG)
    ids = pack.load_ids_jsonl(records, "ids100")
    result = pack.evaluate_mf1(config, ids, dataset, answer_ids=["q2"])
    rows = result["tables"]["selection"]
    assert set(rows) == {"T3/A0/k30", "(c) rank-1 session <= 10", "(b) assistant reference -> T8"}
    assert rows["(c) rank-1 session <= 10"]["per_type"][SSA]["n"] == 1
    assert result["heuristics"] == "harness-side rules (REQ41-F6), not SDK behaviour"
    assert set(result["expected"]) == {"(c) rank-1 session <= 10", "(b) assistant reference -> T8"}


# --- M3: the pack answer-run config ------------------------------------------------------------


def _m1_choice(policy="T3/A0/k30"):
    return {"policy": policy, "adopted": True, "grid": {"T3/A0/k30": {"full_items": 3, "compact_chars": 0, "k": 30}}}


def test_pack_answer_config_runs_pack_arms_with_their_k_and_labels_every_renderer():
    pack = _pack()
    mix = c3.load_config(CONFIG_DIR / "hermes-answer.mix.json")
    config = pack.build_pack_answer_config(mix, _m1_choice(), control_reuse=True, engine_config=STOP_CONFIG)
    assert config["schema_version"] == "comp-03.answer.v4"
    assert [arm["id"] for arm in config["arms"]] == ["fathomdb-fts-pack", "fathomdb-hybrid-pack"]
    fts = config["arms"][0]
    assert fts["product_arm"] == "fathomdb" and fts["top_k"] == 30 and fts["engine_config"] == STOP_CONFIG
    assert fts["evidence"]["renderer"] == "pack_context" and fts["evidence"]["item_source"] == "caller"
    assert fts["evidence"]["pack"] == {"budget_chars": 30000, "item_chars": 4000, "full_items": 3, "compact_chars": 0,
                                       "keep_roles": ["user"], "expand_groups": True, "order": "chronological"}
    assert fts["renderer"] == "pack_context T3/A0/k30 (caller items)"
    assert config["selection"] == mix["selection"] and config["live"] == mix["live"]
    assert "requires_sweep_verdict" not in config["arms"][1]
    assert c3.load_config_obj(config) == config
    both = pack.build_pack_answer_config(mix, _m1_choice(), control_reuse=False, engine_config=None)
    assert [arm["id"] for arm in both["arms"]] == ["fathomdb-fts-mix", "fathomdb-hybrid-mix",
                                                   "fathomdb-fts-pack", "fathomdb-hybrid-pack"]
    assert both["arms"][0]["top_k"] == 10 and both["arms"][0]["evidence"]["pack"]["full_items"] == 10**6
    assert both["arms"][0]["renderer"] == "pack_context Slice 40 mix (caller items)"
    assert "engine_config" not in both["arms"][0]
    assert not any(arm["product_arm"] == "lancedb" for arm in both["arms"]), "LanceDB is not re-run (R0 Q1)"


def test_pack_answer_run_uses_each_arms_k_and_pack_renderer(tmp_path: Path, monkeypatch):
    pytest.importorskip("fathomdb")
    pack = _pack()
    items = [_item("q1", SSA, question="What name did you suggest for the pet?")]
    dataset = _dataset(tmp_path, items)
    mix = c3.load_config(CONFIG_DIR / "hermes-answer.mix.json")
    config = pack.build_pack_answer_config(mix, _m1_choice(), control_reuse=True, engine_config=STOP_CONFIG)
    config["arms"] = config["arms"][:1]
    config["corpus"]["sha256"] = c3.sha256_file(dataset)
    config["selection"]["question_ids"] = ["q1"]
    calls: list[dict] = []
    real = c3.retrieve

    def recording(arm_name, case, workdir, **kwargs):
        calls.append({"arm": arm_name, **{k: kwargs.get(k) for k in ("top_k", "engine_config")}})
        return real(arm_name, case, workdir, **kwargs)

    monkeypatch.setattr(c3, "retrieve", recording)

    class Client:
        def complete(self, model, messages, max_tokens, **_):
            return c3.Reply("yes" if "Answer yes or no only" in messages[-1]["content"] else "Biscuit", 10, 1)

    run_id, run_dir = c3.run(config, dataset_path=dataset, attempt_root=tmp_path / "attempt", client=Client(),
                             base_dir=tmp_path / "exp", fathomdb_bin=None, config_path=None)
    assert calls == [{"arm": "fathomdb", "top_k": 30, "engine_config": STOP_CONFIG}]
    retrieval = json.loads((tmp_path / "attempt" / "questions" / "q1" / "fathomdb-fts-pack" / "retrieval.json").read_text())
    expected = pack.pack_question(items[0], retrieval["ids"], config["arms"][0]["evidence"]["pack"]).text
    assert retrieval["evidence_sha256"] == hashlib.sha256(expected.encode("utf-8")).hexdigest()
    metrics = json.loads((run_dir / "record.json").read_text())["metrics"]
    assert metrics["renderer_by_arm"] == {"fathomdb-fts-pack": "pack_context T3/A0/k30 (caller items)"}
    assert metrics["timing"]["fathomdb-fts-pack"]["renderer"] == "pack_context T3/A0/k30 (caller items)"
    assert metrics["retrieval"]["fathomdb-fts-pack"]["renderer"] == "pack_context T3/A0/k30 (caller items)"
