"""Measurement-correctness checks for the COMP-03 retrieval sweep (Slice 40 Unit D1).

Metric and decision-rule checks use hand-built gold and hand-written CI tables.
Store checks use real FathomDB on a few rounds, on CPU.
"""

from __future__ import annotations

import hashlib
import inspect
import json
import sqlite3
from pathlib import Path
from types import SimpleNamespace

import pytest

from experiments import comp03_answer as c3
from experiments import comp_fts

TYPES = sorted(c3.QUESTION_TYPES)
PREF = "single-session-preference"
RULE = {
    "metric": "session_recall_any_at_5",
    "treatment": "fathomdb-hybrid",
    "control": "fathomdb-fts",
    "focus_type": PREF,
}


def _retrieval():
    from experiments import comp03_retrieval

    return comp03_retrieval


def _require_default_embedder(tmp_path: Path) -> None:
    """Skip only when the installed wheel cannot open the default embedder."""
    from fathomdb import Engine

    try:
        Engine.open(str(tmp_path / "probe.sqlite"), use_default_embedder=True).close()
    except Exception as exc:  # noqa: BLE001 - the reason is printed in the skip
        reason = f"installed fathomdb wheel cannot open the default embedder: {type(exc).__name__}: {exc}"
        print(reason)
        pytest.skip(reason)


def _item(qid: str, qtype: str) -> dict:
    return {
        "question_id": qid,
        "question_type": qtype,
        "question": "What kind of pet did I adopt?",
        "question_date": "2023/06/01 (Thu) 10:00",
        "answer": "a golden retriever",
        "answer_session_ids": ["answer_s2"],
        "haystack_dates": ["2023/04/01 (Sat) 09:00", "2023/05/01 (Mon) 09:00"],
        "haystack_session_ids": ["sharegpt_s1", "answer_s2"],
        "haystack_sessions": [
            [
                {"role": "user", "content": "Recommend a pasta recipe."},
                {"role": "assistant", "content": "Try carbonara with guanciale."},
            ],
            [
                {"role": "user", "content": "I adopted a golden retriever puppy!", "has_answer": True},
                {"role": "assistant", "content": "Congratulations on the new dog."},
            ],
        ],
    }


# --- gold-leak discipline --------------------------------------------------


def test_sweep_reuses_answer_harness_conversion_and_gold():
    sweep = _retrieval()
    assert sweep.convert_question is c3.convert_question
    assert sweep.gold_for is c3.gold_for


# --- metrics -----------------------------------------------------------------


GOLD = {
    # Two answer sessions: s001 (rounds r000, r001) and s004 (round r000).
    "session_relevant": ["s001-r000", "s001-r001", "s004-r000"],
    "turn_relevant": ["s001-r001"],
}


def test_recall_all_needs_every_answer_session_while_recall_any_needs_one():
    sweep = _retrieval()
    only_first_session = ["x0", "s001-r000", "x1", "x2", "x3", "s004-r000"]
    scores = sweep.question_scores(only_first_session, GOLD, ks=(5, 10))
    assert scores["session_recall_any_at_5"] == 1.0
    assert scores["session_recall_all_at_5"] == 0.0
    assert scores["session_recall_any_at_10"] == 1.0
    assert scores["session_recall_all_at_10"] == 1.0
    assert scores["session_mrr_at_10"] == 0.5
    assert scores["turn_recall_any_at_5"] == 0.0
    assert scores["turn_recall_any_at_10"] == 0.0


def test_two_rounds_of_one_session_do_not_satisfy_recall_all():
    sweep = _retrieval()
    scores = sweep.question_scores(["s001-r000", "s001-r001"], GOLD, ks=(5,))
    assert scores["session_recall_any_at_5"] == 1.0
    assert scores["session_recall_all_at_5"] == 0.0
    assert scores["turn_recall_any_at_5"] == 1.0


def test_metrics_are_none_without_relevant_rounds_and_zero_when_nothing_found():
    sweep = _retrieval()
    empty = sweep.question_scores(["a", "b"], {"session_relevant": [], "turn_relevant": []}, ks=(5, 10))
    assert all(value is None for value in empty.values())
    missed = sweep.question_scores(["a", "b"], GOLD, ks=(5, 10))
    assert missed["session_recall_any_at_10"] == 0.0
    assert missed["session_recall_all_at_10"] == 0.0
    assert missed["session_mrr_at_10"] == 0.0


def test_bootstrap_ci_is_seeded_and_degenerate_on_constant_differences():
    sweep = _retrieval()
    diffs = [1.0, 0.0, -1.0, 1.0, 0.0, 1.0]
    first = sweep.bootstrap_ci(diffs, seed=7, resamples=2000)
    assert first == sweep.bootstrap_ci(diffs, seed=7, resamples=2000)
    assert first[0] <= sum(diffs) / len(diffs) <= first[1]
    assert sweep.bootstrap_ci([0.0] * 5, seed=7, resamples=2000) == [0.0, 0.0]
    assert sweep.bootstrap_ci([1.0] * 5, seed=7, resamples=2000) == [1.0, 1.0]


def test_paired_deltas_are_treatment_minus_control_overall_and_per_type():
    sweep = _retrieval()
    rows = [
        {"question_type": PREF, "arm": "fathomdb-hybrid", "question_id": "p1", "session_recall_any_at_5": 1.0},
        {"question_type": PREF, "arm": "fathomdb-fts", "question_id": "p1", "session_recall_any_at_5": 0.0},
        {"question_type": PREF, "arm": "fathomdb-hybrid", "question_id": "p2", "session_recall_any_at_5": 1.0},
        {"question_type": PREF, "arm": "fathomdb-fts", "question_id": "p2", "session_recall_any_at_5": 1.0},
        {"question_type": "multi-session", "arm": "fathomdb-hybrid", "question_id": "m1", "session_recall_any_at_5": 0.0},
        {"question_type": "multi-session", "arm": "fathomdb-fts", "question_id": "m1", "session_recall_any_at_5": 1.0},
    ]
    deltas = sweep.paired_deltas(rows, "session_recall_any_at_5", treatment="fathomdb-hybrid",
                                 control="fathomdb-fts", seed=1, resamples=1000)
    assert deltas["overall"]["n"] == 3
    assert deltas["overall"]["delta"] == pytest.approx(0.0)
    assert deltas["per_type"][PREF]["delta"] == pytest.approx(0.5)
    assert deltas["per_type"]["multi-session"]["delta"] == pytest.approx(-1.0)
    assert deltas["per_type"]["multi-session"]["ci"] == [-1.0, -1.0]


# --- decision rule -------------------------------------------------------------


def _table(*, pref_ci=(0.01, 0.20), other_ci=(-0.05, 0.05), overall=0.02) -> dict:
    per_type = {t: {"delta": 0.0, "ci": list(other_ci), "n": 10} for t in TYPES}
    per_type[PREF] = {"delta": 0.1, "ci": list(pref_ci), "n": 30}
    return {"overall": {"delta": overall, "ci": [overall - 0.01, overall + 0.01], "n": 470}, "per_type": per_type}


def test_decision_rule_adopts_when_all_three_clauses_hold():
    verdict = _retrieval().evaluate_decision_rule(RULE, _table())
    assert verdict["verdict"] == "adopt"
    assert verdict["clauses"] == {"a": True, "b": True, "c": True}


@pytest.mark.parametrize(
    ("table", "failed"),
    [
        (_table(pref_ci=(-0.01, 0.20)), "a"),
        (_table(pref_ci=(0.0, 0.20)), "a"),
        (_table(other_ci=(-0.20, -0.01)), "b"),
        (_table(overall=-0.001), "c"),
    ],
)
def test_decision_rule_rejects_when_any_clause_fails(table: dict, failed: str):
    verdict = _retrieval().evaluate_decision_rule(RULE, table)
    assert verdict["verdict"] == "reject"
    assert verdict["clauses"][failed] is False
    assert all(ok for clause, ok in verdict["clauses"].items() if clause != failed)


def test_decision_rule_accepts_an_overall_delta_of_exactly_zero():
    table = _table(overall=0.0)
    table["per_type"][PREF]["ci"] = [0.01, 0.30]
    assert _retrieval().evaluate_decision_rule(RULE, table)["verdict"] == "adopt"


# --- config ------------------------------------------------------------------


def test_sweep_config_is_typed_and_states_the_rule_verbatim(tmp_path: Path):
    sweep = _retrieval()
    config = sweep.load_config(sweep.DEFAULT_CONFIG)
    assert config["schema_version"] == "comp-03.retrieval-sweep.v1"
    assert set(config) == sweep.CONFIG_KEYS
    assert [arm["id"] for arm in config["arms"]] == ["fathomdb-fts", "fathomdb-hybrid", "lancedb-fts"]
    assert [arm["product_arm"] for arm in config["arms"]] == ["fathomdb", "fathomdb-hybrid", "lancedb"]
    answer_v1 = c3.load_config(c3.DEFAULT_CONFIG)
    assert config["corpus"] == answer_v1["corpus"]
    assert config["document_unit"] == answer_v1["document_unit"]
    assert config["selection"]["rule"] == "all"
    assert config["retrieval"]["top_k"] == 10
    assert config["metrics"]["ks"] == [5, 10]
    assert isinstance(config["metrics"]["bootstrap"]["seed"], int)
    assert config["metrics"]["bootstrap"]["resamples"] == 10_000
    assert config["decision_rule"] == RULE
    path = tmp_path / "bad.json"
    path.write_text(json.dumps(dict(config, schema_version="comp-03.retrieval-sweep.v0")))
    with pytest.raises(sweep.Comp03RetrievalError):
        sweep.load_config(path)


# --- device check ------------------------------------------------------------


def test_device_check_refuses_a_cuda_request_that_resolved_to_cpu():
    from experiments import fathomdb_test_setup as setup

    cpu = SimpleNamespace(effective_device=SimpleNamespace(kind="cpu"), selected_cuda_uuid=None)
    cuda = SimpleNamespace(effective_device=SimpleNamespace(kind="cuda"), selected_cuda_uuid="GPU-1")
    with pytest.raises(RuntimeError):
        setup.require_embed_device(cpu, "cuda:0")
    with pytest.raises(RuntimeError):
        setup.require_embed_device(None, "cuda:0")
    setup.require_embed_device(cuda, "cuda:0")
    setup.require_embed_device(cpu, "cpu")
    setup.require_embed_device(cpu, "auto")


# --- comp_fts hybrid arm -----------------------------------------------------


FATHOM_ARM_SOURCE_SHA256 = "6f0061da4947158dd52a58a6d21f2b89932ecf717b4f11ba175a7680392e22d0"


def test_comp_fts_fts_arm_is_byte_unchanged_and_hybrid_arm_is_registered():
    source = inspect.getsource(comp_fts.FathomArm)
    assert hashlib.sha256(source.encode("utf-8")).hexdigest() == FATHOM_ARM_SOURCE_SHA256
    assert type(comp_fts.ARMS["fathomdb"]) is comp_fts.FathomArm
    assert comp_fts.ARMS["fathomdb"].settings() == {
        "method": "Engine.search_text_only",
        "embedder": "none",
        "query_translation": "native (short-token/stopword removal, OR terms)",
        "dedup": "native body dedup before limit",
        "tokenizer": "FTS5 porter unicode61 remove_diacritics 2",
    }
    hybrid = comp_fts.ARMS["fathomdb-hybrid"]
    assert hybrid.name == "fathomdb-hybrid"
    assert hybrid.settings()["method"] == "Engine.search"
    assert comp_fts.CAMPAIGNS["comp01-hybrid"] == (("fathomdb", "fathomdb-hybrid"), "fts")
    assert comp_fts.CAMPAIGNS["comp01"] == (("fathomdb", "lancedb"), "fts")


DOCS = [
    {"id": "d0", "body": "I adopted a golden retriever puppy"},
    {"id": "d1", "body": "My favourite cuisine is Thai food"},
    {"id": "d2", "body": "The meeting moved to Tuesday"},
]


def _built(arm_name: str, tmp_path: Path) -> Path:
    root = tmp_path / arm_name
    root.mkdir()
    comp_fts.ARMS[arm_name].build(root, DOCS)
    return root


def test_hybrid_arm_store_has_one_vector_row_per_document_and_vector_hits(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("FATHOMDB_EMBED_DEVICE", "cpu")
    _require_default_embedder(tmp_path)
    hybrid = comp_fts.ARMS["fathomdb-hybrid"]
    root = tmp_path / "hybrid"
    root.mkdir()
    build = hybrid.build(root, DOCS)
    assert build["build_return_s"] >= build["ingest_ack_s"]
    with sqlite3.connect(root / "fathomdb.sqlite") as connection:
        cursors = connection.execute(
            "SELECT COUNT(DISTINCT write_cursor) FROM _fathomdb_vector_rows WHERE kind = 'doc'"
        ).fetchone()[0]
        bodies = [row[0] for row in connection.execute("SELECT body FROM canonical_nodes ORDER BY write_cursor")]
    assert cursors == len(DOCS)
    assert bodies == [doc["body"] for doc in DOCS], "the hybrid store holds the fts arm's plain bodies"
    assert build["vector_rows"] == len(DOCS)
    handle = hybrid.open(root)
    try:
        hits = handle.search("what pet do I have", limit=3).results
        assert any(hit.branch == "vector" for hit in hits), [hit.branch for hit in hits]
        assert hybrid.query(handle, "what pet do I have", limit=3)[0] == "d0"
    finally:
        hybrid.close(handle)
    fts = comp_fts.ARMS["fathomdb"]
    handle = fts.open(_built("fathomdb", tmp_path))
    try:
        assert fts.query(handle, "what pet do I have", limit=3) == [], "a vocabulary gap FTS cannot cross"
    finally:
        fts.close(handle)


def test_vector_coverage_check_refuses_a_store_without_vectors(tmp_path: Path, monkeypatch):
    pytest.importorskip("fathomdb")
    sweep = _retrieval()
    root = _built("fathomdb", tmp_path)
    with pytest.raises(sweep.Comp03RetrievalError):
        sweep.require_vector_coverage(root / "fathomdb.sqlite", documents=len(DOCS))
    monkeypatch.setenv("FATHOMDB_EMBED_DEVICE", "cpu")
    _require_default_embedder(tmp_path)
    hybrid_root = _built("fathomdb-hybrid", tmp_path)
    assert sweep.require_vector_coverage(hybrid_root / "fathomdb.sqlite", documents=len(DOCS)) == len(DOCS)


# --- sweep run, resume, all questions ------------------------------------------


def _dataset(tmp_path: Path, items: list[dict]) -> Path:
    path = tmp_path / "lme.json"
    path.write_text(json.dumps(items))
    return path


def _fts_only_config(sweep, dataset: Path) -> dict:
    config = sweep.load_config(sweep.DEFAULT_CONFIG)
    config["corpus"]["sha256"] = c3.sha256_file(dataset)
    config["arms"] = [arm for arm in config["arms"] if arm["id"] == "fathomdb-fts"]
    return config


def test_sweep_covers_every_question_and_resume_does_no_new_work(tmp_path: Path, monkeypatch):
    pytest.importorskip("fathomdb")
    sweep = _retrieval()
    items = [_item("q1", "single-session-user"), _item("q2", PREF), _item("q3_abs", "multi-session")]
    dataset = _dataset(tmp_path, items)
    config = _fts_only_config(sweep, dataset)
    arm = comp_fts.ARMS["fathomdb"]
    builds: list[int] = []
    original = arm.build

    def counting(root, docs):
        builds.append(len(docs))
        return original(root, docs)

    monkeypatch.setattr(arm, "build", counting)
    attempt = tmp_path / "attempt"
    summary = sweep.sweep(config, dataset_path=dataset, attempt_root=attempt)
    assert len(builds) == 3, "every question, abstentions included, is retrieved"
    for qid in ("q1", "q2", "q3_abs"):
        result = json.loads((attempt / "questions" / qid / "fathomdb-fts" / "retrieval.json").read_text())
        assert len(result["ids"]) <= 10
        assert result["latency_ms"] > 0 and result["build_return_s"] >= 0
        assert result["arm"] == "fathomdb"
        assert result["settings"] == comp_fts.FathomArm().settings(), "the v1 fts settings record"
    assert summary["arms"]["fathomdb-fts"]["overall"]["n"] == 2, "abstentions are not scored"
    assert summary["decision"]["verdict"] == "not_evaluated"
    sweep.sweep(config, dataset_path=dataset, attempt_root=attempt)
    assert len(builds) == 3, "finished questions do no work on rerun"


def test_sweep_refuses_a_dataset_whose_digest_differs(tmp_path: Path):
    sweep = _retrieval()
    dataset = _dataset(tmp_path, [_item("q1", "single-session-user")])
    config = sweep.load_config(sweep.DEFAULT_CONFIG)
    with pytest.raises(sweep.Comp03RetrievalError):
        sweep.sweep(config, dataset_path=dataset, attempt_root=tmp_path / "attempt")


def test_sweep_hybrid_arm_records_the_resolved_device_and_writes_a_verdict(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("FATHOMDB_EMBED_DEVICE", "cpu")
    _require_default_embedder(tmp_path)
    sweep = _retrieval()
    dataset = _dataset(tmp_path, [_item("q1", PREF), _item("q2", "single-session-user")])
    config = sweep.load_config(sweep.DEFAULT_CONFIG)
    config["corpus"]["sha256"] = c3.sha256_file(dataset)
    config["arms"] = [arm for arm in config["arms"] if arm["id"] in {"fathomdb-fts", "fathomdb-hybrid"}]
    summary = sweep.sweep(config, dataset_path=dataset, attempt_root=tmp_path / "attempt")
    assert summary["device"] == {"requested": "cpu", "effective": "cpu"}
    assert summary["decision"]["verdict"] in {"adopt", "reject"}
    assert set(summary["decision"]["clauses"]) == {"a", "b", "c"}
    hybrid = json.loads((tmp_path / "attempt" / "questions" / "q1" / "fathomdb-hybrid" / "retrieval.json").read_text())
    assert hybrid["vector_rows"] == hybrid["documents"]
    assert hybrid["drain_s"] >= 0, "embedding time is recorded apart from query latency"
