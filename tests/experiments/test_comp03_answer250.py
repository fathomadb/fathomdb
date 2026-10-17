"""Measurement-correctness checks for the paired 250-question COMP-03 answer run.

FathomDB FTS through pack_context (T5/A0/k50, query stop words) against
LanceDB FTS on the Slice 40 mix renderer. The selection, reuse rule, cost
ceiling and paired summary are checked on hand-built items with a stub model.
"""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path

import pytest

from experiments import comp03_answer as c3

CONFIG_DIR = c3.DEFAULT_CONFIG.parent
COUNTS = {"single-session-user": 70, "multi-session": 133, "single-session-preference": 30,
          "temporal-reasoning": 133, "knowledge-update": 78, "single-session-assistant": 56}


def _a250():
    from experiments import comp03_answer250

    return comp03_answer250


def _item(qid: str, qtype: str) -> dict:
    return {
        "question_id": qid,
        "question_type": qtype,
        "question": "Where did I move to last spring?",
        "question_date": "2023/06/01 (Thu) 10:00",
        "answer": "Paris",
        "answer_session_ids": ["answer_s1"],
        "haystack_dates": ["2023/04/01 (Sat) 09:00", "2023/05/01 (Mon) 09:00"],
        "haystack_session_ids": ["sharegpt_s0", "answer_s1"],
        "haystack_sessions": [
            [{"role": "user", "content": "Recommend a pasta recipe."},
             {"role": "assistant", "content": "Try carbonara."}],
            [{"role": "user", "content": "I just moved to Paris last spring!", "has_answer": True},
             {"role": "assistant", "content": "Congratulations on the move."}],
        ],
    }


# --- selection ----------------------------------------------------------------------------


def test_quotas_follow_the_full_mix_by_largest_remainder_with_name_ties():
    quotas = _a250().type_quotas(COUNTS, 250)
    assert quotas == {"knowledge-update": 39, "multi-session": 67, "single-session-assistant": 28,
                      "single-session-preference": 15, "single-session-user": 35, "temporal-reasoning": 66}
    assert sum(quotas.values()) == 250


def test_superset_keeps_the_pinned_ids_first_and_fills_each_type_deterministically():
    a = _a250()
    dataset = [_item(f"{t[:3]}{i:02d}", t) for t in ("multi-session", "knowledge-update") for i in range(10)]
    dataset.append(_item("mul99_abs", "multi-session"))
    base = ["mul03", "kno01"]
    quotas = {"multi-session": 4, "knowledge-update": 2}
    picked = a.select_superset(dataset, base, quotas, seed=7)
    assert picked[:2] == base
    assert picked == a.select_superset(dataset, base, quotas, seed=7)
    assert len(picked) == len(set(picked)) == 6
    new = picked[2:]
    assert [q for q in new if q.startswith("kno")] == sorted(q for q in new if q.startswith("kno"))
    assert sum(q.startswith("mul") for q in picked) == 4 and sum(q.startswith("kno") for q in picked) == 2
    assert picked != a.select_superset(dataset, base, quotas, seed=8)
    with pytest.raises(ValueError):
        a.select_superset(dataset, base, {"multi-session": 0, "knowledge-update": 2}, seed=7)


def test_the_250_config_pins_its_ids_and_digest_and_reuses_the_pinned_60():
    config = c3.load_config(CONFIG_DIR / "hermes-answer.250.json")
    mix = c3.load_config(CONFIG_DIR / "hermes-answer.mix.json")
    sel = config["selection"]
    assert config["schema_version"] == "comp-03.answer.v5"
    assert len(sel["question_ids"]) == 250 == len(set(sel["question_ids"]))
    assert sel["question_ids"][:60] == mix["selection"]["question_ids"]
    assert sel["question_ids_sha256"] == hashlib.sha256("\n".join(sel["question_ids"]).encode()).hexdigest()
    assert sel["quotas"] == _a250().type_quotas(COUNTS, 250)
    assert [arm["id"] for arm in config["arms"]] == ["fathomdb-fts-pack", "lancedb-fts"]
    fathom, lance = config["arms"]
    assert fathom["top_k"] == 50 and fathom["evidence"]["pack"]["full_items"] == 5
    assert fathom["evidence"]["pack"]["compact_chars"] == 0
    assert fathom["engine_config"] == {"query_stop_words": {"base": "fathomdb", "extra": ["had", "long", "suggest"]}}
    assert lance["top_k"] == 10 and lance["evidence"]["policy"] == mix["evidence"]["policy"]
    assert lance["evidence"]["chronological"] is True
    assert config["live"]["ceiling_usd"] == 11.5
    assert {r["arm"] for r in config["reuse"]} == {"fathomdb-fts-pack", "lancedb-fts"}
    broken = copy.deepcopy(config)
    broken["selection"]["question_ids"][0], broken["selection"]["question_ids"][1] = \
        broken["selection"]["question_ids"][1], broken["selection"]["question_ids"][0]
    with pytest.raises(c3.Comp03Error):
        c3.load_config_obj(broken)


# --- reuse rule ---------------------------------------------------------------------------


def test_reuse_needs_the_same_reader_judge_seed_prompts_and_evidence_policy():
    config = c3.load_config(CONFIG_DIR / "hermes-answer.250.json")
    mix = c3.load_config(CONFIG_DIR / "hermes-answer.mix.json")
    c3.check_reuse(config, "lancedb-fts", mix, "lancedb-fts")
    m3 = copy.deepcopy(config)
    m3["schema_version"] = "comp-03.answer.v4"
    c3.check_reuse(config, "fathomdb-fts-pack", m3, "fathomdb-fts-pack")
    for change in (
        lambda s: s["live"]["judge"].update(model="other"),
        lambda s: s["live"]["answerer"].update(max_tokens=10),
        lambda s: s["live"].update(seed=1),
        lambda s: s["evidence"].update(recency_instruction=True),
        lambda s: s["evidence"]["policy"].update(total_chars=20000),
        lambda s: s["retrieval"].update(top_k=5),
    ):
        source = copy.deepcopy(mix)
        change(source)
        with pytest.raises(c3.Comp03Error):
            c3.check_reuse(config, "lancedb-fts", source, "lancedb-fts")
    other = copy.deepcopy(m3)
    other["arms"][0]["evidence"]["pack"]["full_items"] = 3
    with pytest.raises(c3.Comp03Error):
        c3.check_reuse(config, "fathomdb-fts-pack", other, "fathomdb-fts-pack")
    other = copy.deepcopy(m3)
    del other["arms"][0]["engine_config"]
    with pytest.raises(c3.Comp03Error):
        c3.check_reuse(config, "fathomdb-fts-pack", other, "fathomdb-fts-pack")


class _Client:
    def __init__(self) -> None:
        self.calls = 0

    def complete(self, model, messages, max_tokens, **_):
        self.calls += 1
        content = "yes" if "Answer yes or no only" in messages[-1]["content"] else "Paris"
        return c3.Reply(content, prompt_tokens=1000, completion_tokens=10)


def _small(tmp_path: Path, qids: list[str], *, reuse_root: Path | None = None) -> tuple[dict, Path]:
    items = [_item(q, "single-session-user" if i % 2 else "multi-session") for i, q in enumerate(qids)]
    dataset = tmp_path / "lme.json"
    dataset.write_text(json.dumps(items))
    config = c3.load_config(CONFIG_DIR / "hermes-answer.250.json")
    config["corpus"]["sha256"] = c3.sha256_file(dataset)
    config["selection"]["question_ids"] = qids
    config["selection"]["question_ids_sha256"] = hashlib.sha256("\n".join(qids).encode()).hexdigest()
    config["selection"]["base_count"] = 1
    if reuse_root is None:
        config["reuse"] = []
    else:
        for entry in config["reuse"]:
            entry["source_attempt_root"] = str(reuse_root)
            entry["source_arm"] = entry["arm"]
    return config, dataset


def _run(config: dict, dataset: Path, attempt: Path, client: _Client, tmp_path: Path):
    return c3.run(config, dataset_path=dataset, attempt_root=attempt, client=client, base_dir=tmp_path / "exp",
                  fathomdb_bin=None, config_path=None)


def test_reused_questions_make_no_calls_are_marked_and_cost_nothing(tmp_path: Path):
    pytest.importorskip("fathomdb")
    pytest.importorskip("lancedb")
    source_config, dataset = _small(tmp_path, ["q1", "q2"])
    source = tmp_path / "source"
    first = _Client()
    _run(source_config, dataset, source, first, tmp_path)
    assert first.calls == 8
    config, dataset = _small(tmp_path, ["q1", "q2"], reuse_root=source)
    client = _Client()
    _, run_dir = _run(config, dataset, tmp_path / "attempt", client, tmp_path)
    assert client.calls == 4, "q1 is reused on both arms; only q2 is called"
    for arm in ("fathomdb-fts-pack", "lancedb-fts"):
        call = json.loads((tmp_path / "attempt" / "questions" / "q1" / arm / "answer-call.json").read_text())
        assert call["reused_from"] == {"attempt_root": str(source), "arm": arm}
    metrics = json.loads((run_dir / "record.json").read_text())["metrics"]
    assert metrics["reuse"]["fathomdb-fts-pack"]["questions"] == ["q1"]
    assert metrics["reuse"]["lancedb-fts"]["usd"] > 0
    expected = 2 * (1000 * 2.5 + 10 * 15.0) / 1e6 * 2
    assert metrics["cost_usd"] == pytest.approx(expected), "reused calls are not this run's spend"
    answer = metrics["answer"]
    assert answer["n"] == 2 and set(answer["accuracy"]) == {"fathomdb-fts-pack", "lancedb-fts"}
    assert set(answer["discordant"]) == {"fathomdb-fts-pack_only", "lancedb-fts_only"}
    assert len(answer["difference_bootstrap_95"]) == 2 and set(answer["per_type"]) == {"multi-session",
                                                                                      "single-session-user"}


def test_reuse_refuses_a_source_run_whose_config_differs(tmp_path: Path):
    pytest.importorskip("fathomdb")
    pytest.importorskip("lancedb")
    source_config, dataset = _small(tmp_path, ["q1", "q2"])
    source = tmp_path / "source"
    source.mkdir()
    changed = copy.deepcopy(source_config)
    changed["live"]["judge"]["model"] = "other"
    (source / "config.resolved.json").write_text(json.dumps(changed))
    config, dataset = _small(tmp_path, ["q1", "q2"], reuse_root=source)
    with pytest.raises(c3.Comp03Error):
        _run(config, dataset, tmp_path / "attempt", _Client(), tmp_path)


def test_cost_ceiling_stops_before_a_question_and_the_run_resumes(tmp_path: Path):
    pytest.importorskip("fathomdb")
    pytest.importorskip("lancedb")
    config, dataset = _small(tmp_path, ["q1", "q2", "q3"])
    per_question = 2 * 2 * (1000 * 2.5 + 10 * 15.0) / 1e6
    config["live"]["question_usd_estimate"] = per_question
    config["live"]["ceiling_usd"] = per_question * 2.5
    attempt = tmp_path / "attempt"
    client = _Client()
    with pytest.raises(c3.Comp03CostCeiling):
        _run(config, dataset, attempt, client, tmp_path)
    assert client.calls == 8, "q3 would pass the ceiling, so it is not started"
    assert not (attempt / "questions" / "q3").exists()
    config["live"]["ceiling_usd"] = per_question * 3.5
    _, run_dir = _run(config, dataset, attempt, client, tmp_path)
    assert client.calls == 12
    assert json.loads((run_dir / "record.json").read_text())["metrics"]["answer"]["n"] == 3
