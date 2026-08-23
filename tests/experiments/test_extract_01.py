"""Contract tests for the bounded EXTRACT-01 comparison."""

from __future__ import annotations

import json
import sqlite3
from pathlib import Path

import pytest

from experiments import extract_01


CONFIG = Path("experiments/configs/extract-01/knowledge-update.v1.json")


def _corpus_row(question_id: str = "q1") -> dict[str, object]:
    return {
        "question_id": question_id,
        "question_type": "knowledge-update",
        "question": "Where does the user live now?",
        "answer": "Austin",
        "question_date": "2025/01/03 (Fri) 10:00",
        "haystack_session_ids": ["s1", "s2"],
        "haystack_dates": ["2025/01/01 (Wed) 10:00", "2025/01/02 (Thu) 10:00"],
        "haystack_sessions": [
            [{"role": "user", "content": "I live in Boston.", "has_answer": True}],
            [{"role": "user", "content": "I moved to Austin.", "has_answer": True}],
        ],
        "answer_session_ids": ["s1", "s2"],
    }


def test_committed_config_fixes_the_smallest_defensible_comparison():
    config = extract_01.load_config(CONFIG)

    assert config["program_track"] == "EXTRACT-01"
    assert config["scope"] == {
        "question_type": "knowledge-update",
        "question_count": 78,
    }
    assert [arm["id"] for arm in config["arms"]] == ["raw_fts", "raw_plus_elps_fts"]
    assert config["profile"] == {
        "id": "a0_turn_fts_stream_default",
        "retrieval": "fts",
        "top_k": 10,
        "embedder": "none",
        "embed_device": "cpu",
        "reranker": "none",
        "rerank_device": "cpu",
    }
    assert config["live"]["max_usd"] == 20.0
    assert config["extraction"]["max_facts_per_document"] == 24
    assert config["live"]["extractor"]["model"] == "gemini-3.1-flash-lite"
    assert config["live"]["extractor"]["reasoning_effort"] == "none"
    assert config["live"]["extractor"]["min_call_interval_seconds"] == 8.5
    assert config["live"]["answerer"]["model"] == "gemini-3.1-flash-lite"
    assert config["live"]["answerer"]["reasoning_effort"] == "none"
    assert config["live"]["answerer"]["min_call_interval_seconds"] == 8.5
    assert config["live"]["judge"]["model"] == "claude-sonnet"
    assert config["live"]["judge"]["reasoning_effort"] == "none"
    assert config["live"]["judge"]["min_call_interval_seconds"] == 0.0
    assert config["claim_boundary"]["extraction_precision"] == "unscored_no_human_gold"


def test_config_rejects_unknown_fields():
    document = json.loads(CONFIG.read_text(encoding="utf-8"))
    document["tuning"] = True

    with pytest.raises(extract_01.Extract01Error, match="keys mismatch"):
        extract_01.resolve_config(document)


def test_question_blind_extraction_chunks_strip_all_gold_labels():
    case = extract_01.case_from_row(_corpus_row())
    chunks = extract_01.extraction_documents(case, max_chars=1000)
    serialized = json.dumps(chunks)

    assert len(chunks) == 1
    assert chunks[0]["source_doc_id"] == "q1:chunk-000"
    assert "Where does the user live now" not in serialized
    assert 'Austin"' not in serialized
    assert "has_answer" not in serialized
    assert "answer_session_ids" not in serialized
    assert "2025/01/01" in chunks[0]["body"]
    assert "I moved to Austin" in chunks[0]["body"]


def test_numeric_reference_answer_is_preserved_as_text():
    row = _corpus_row()
    row["answer"] = 120

    assert extract_01.case_from_row(row).answer == "120"


def test_extraction_output_requires_valid_source_links_and_confidence():
    valid = {
        "entities": [
            {
                "name": "User",
                "type": "Person",
                "aliases": [],
                "source_doc_id": "q1:chunk-000",
            },
            {
                "name": "Austin",
                "type": "Place",
                "aliases": [],
                "source_doc_id": "q1:chunk-000",
            },
        ],
        "edges": [
            {
                "from_entity": "User",
                "to_entity": "Austin",
                "relation": "lives_in",
                "body": "I moved to Austin.",
                "t_valid": "2025-01-02T10:00:00Z",
                "t_invalid": None,
                "confidence": 0.9,
                "source_doc_id": "q1:chunk-000",
                "source_span": None,
            }
        ],
        "warnings": [],
    }
    assert extract_01.validate_elps_result(valid, {"q1:chunk-000"}) == valid

    invalid = json.loads(json.dumps(valid))
    invalid["edges"][0]["source_doc_id"] = "invented"
    with pytest.raises(extract_01.Extract01Error, match="source_doc_id"):
        extract_01.validate_elps_result(invalid, {"q1:chunk-000"})
    invalid = json.loads(json.dumps(valid))
    invalid["edges"][0]["confidence"] = 1.1
    with pytest.raises(extract_01.Extract01Error, match="confidence"):
        extract_01.validate_elps_result(invalid, {"q1:chunk-000"})


def test_single_document_adapter_repairs_provider_shape_before_native_ingest():
    raw = {
        "entities": [
            {"name": "User", "type": "person:human", "aliases": None},
            {"name": "Austin", "type": "place", "aliases": ["ATX"]},
        ],
        "edges": [
            {
                "from_entity": "User",
                "to_entity": "Austin",
                "relation": "lives:in",
                "body": "I moved to Austin.",
                "t_valid": "null",
                "t_invalid": "2025-02-30T00:00:00Z",
                "confidence": 4,
                "source_doc_id": "invented",
            },
            {
                "from_entity": {"name": "User"},
                "to_entity": "Austin",
                "relation": "lives_in",
                "body": "malformed endpoint",
            },
        ],
    }

    repaired = extract_01.normalize_elps_result(raw, "q1:chunk-000")

    assert repaired["entities"][0]["type"] == "person_human"
    assert repaired["entities"][0]["aliases"] == []
    assert repaired["edges"][0]["relation"] == "lives_in"
    assert repaired["edges"][0]["confidence"] == 1.0
    assert repaired["edges"][0]["t_valid"] is None
    assert repaired["edges"][0]["t_invalid"] is None
    assert repaired["edges"][0]["source_doc_id"] == "q1:chunk-000"
    assert len(repaired["edges"]) == 1
    assert extract_01.validate_elps_result(repaired, {"q1:chunk-000"}) == repaired


def test_answer_adapter_maps_source_citations_and_drops_unknown_ids():
    hits = [
        {"id": "logical-1", "source_id": "source-1", "body": "one"},
        {"id": "logical-2", "source_id": "source-2", "body": "two"},
    ]

    answer = extract_01.normalize_answer_reply(
        {"answer": "Austin", "cited_ids": ["source-1", "logical-2", "invented"]},
        hits,
    )

    assert answer == {"answer": "Austin", "cited_ids": ["logical-1", "logical-2"]}


def test_semantic_retry_is_bounded_and_returns_first_valid_value():
    replies = iter(["broken", "still broken", '{"ok": true}'])
    calls: list[int] = []

    value = extract_01.semantic_retry(
        2,
        lambda: calls.append(1) or next(replies),
        lambda text: json.loads(text),
    )

    assert value == {"ok": True}
    assert len(calls) == 3

    with pytest.raises(extract_01.Extract01Error, match="semantic retries"):
        extract_01.semantic_retry(
            1,
            lambda: "broken",
            lambda text: json.loads(text),
        )

    failed_calls: list[int] = []

    def cost_failure():
        failed_calls.append(1)
        raise extract_01.Extract01Error("cost cap")

    with pytest.raises(extract_01.Extract01Error, match="cost cap"):
        extract_01.semantic_retry(2, cost_failure, lambda value: value)
    assert len(failed_calls) == 1


def test_cost_guard_reserves_before_call_and_counts_actual_usage(tmp_path):
    state = extract_01.RunState.new("sha", ["q1"])
    ledger = extract_01.Checkpoint(tmp_path / "checkpoint.json", state)
    model = {"input_usd_per_million": 1.0, "output_usd_per_million": 2.0}

    with pytest.raises(extract_01.Extract01Error, match="cost cap"):
        ledger.reserve(
            model, [{"role": "user", "content": "x" * 300}], 100, max_usd=0.0001
        )

    ledger.charge("q1:extract", model, prompt_tokens=100, completion_tokens=10)
    assert ledger.state.cost_usd == pytest.approx(0.00012)
    assert json.loads(ledger.path.read_text())["cost_usd"] == pytest.approx(0.00012)


def test_airlock_client_paces_calls_to_the_same_model(monkeypatch):
    client = extract_01.AirlockClient("http://127.0.0.1:4000", "key", 0)
    ticks = iter([10.0, 14.0])
    sleeps: list[float] = []
    monkeypatch.setattr(extract_01.time, "monotonic", lambda: next(ticks))
    monkeypatch.setattr(extract_01.time, "sleep", sleeps.append)

    client._pace("gemini-3.1-flash-lite", 8.5)
    client._pace("gemini-3.1-flash-lite", 8.5)

    assert sleeps == [4.5]


def test_airlock_client_omits_openai_reasoning_field_for_claude(monkeypatch):
    client = extract_01.AirlockClient("http://127.0.0.1:4000", "key", 0)
    captured: dict[str, object] = {}

    def request(path, payload):
        captured.update(payload)
        return {
            "choices": [{"message": {"content": "yes"}}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1},
        }

    monkeypatch.setattr(client, "_request", request)

    reply = client.complete(
        "claude-sonnet",
        [{"role": "user", "content": "judge"}],
        10,
        json_mode=False,
        reasoning_effort="none",
        min_call_interval_seconds=0.0,
    )

    assert reply.content == "yes"
    assert "reasoning_effort" not in captured


def test_resume_skips_completed_cells_and_requires_full_pairs(tmp_path):
    state = extract_01.RunState.new("sha", ["q1"])
    checkpoint = extract_01.Checkpoint(tmp_path / "checkpoint.json", state)
    checkpoint.put("extract:q1:chunk-000", {"done": True})
    calls: list[str] = []

    value = extract_01.complete_once(
        checkpoint,
        "extract:q1:chunk-000",
        lambda: calls.append("called") or {"done": False},
    )

    assert value == {"done": True}
    assert calls == []
    with pytest.raises(extract_01.Extract01Error, match="incomplete"):
        extract_01.summarize(checkpoint.state, ["q1"])


def test_storage_measurement_uses_logical_sqlite_pages(tmp_path):
    database = tmp_path / "measurement.sqlite"
    connection = sqlite3.connect(database)
    try:
        connection.execute("CREATE TABLE payload (body TEXT NOT NULL)")
        connection.executemany(
            "INSERT INTO payload(body) VALUES (?)",
            [("x" * 2048,) for _ in range(40)],
        )
        connection.commit()

        measured = extract_01.logical_database_bytes(connection)
        page_count = connection.execute("PRAGMA page_count").fetchone()[0]
        page_size = connection.execute("PRAGMA page_size").fetchone()[0]
    finally:
        connection.close()

    assert measured == page_count * page_size
    assert measured > 0


def test_old_case_measurement_is_refreshed_without_invalidating_paid_cells():
    assert extract_01.case_measurement_is_current({"storage_bytes": 100}) is False
    assert (
        extract_01.case_measurement_is_current(
            {"measurement_revision": "sqlite-logical-pages.v1"}
        )
        is True
    )


def test_summary_reports_paired_quality_provenance_lifecycle_and_cost():
    state = extract_01.RunState.new("sha", ["q1"])
    state.cells.update(
        {
            "case:q1": {
                "raw": {"correct": False, "evidence_hit": False, "answer": "Boston"},
                "treatment": {
                    "correct": True,
                    "evidence_hit": True,
                    "answer": "Austin",
                },
                "source_link_total": 3,
                "source_link_valid": 3,
                "extracted_rows": 3,
                "raw_rows": 2,
                "ingest_ms": 8.0,
                "storage_bytes": 1200,
            },
            "lifecycle": {
                "conflict_detected": True,
                "supersession_applied": False,
                "erasure_absent": True,
                "orphan_count": 0,
            },
        }
    )
    state.cost_usd = 1.25

    summary = extract_01.summarize(state, ["q1"])

    assert summary["status"] == "complete"
    assert summary["arms"]["raw_fts"]["answer_accuracy"] == 0.0
    assert summary["arms"]["raw_plus_elps_fts"]["answer_accuracy"] == 1.0
    assert summary["paired_answer_accuracy_delta"] == 1.0
    assert summary["source_link_completeness"] == 1.0
    assert summary["lifecycle"]["supersession_applied"] is False
    assert summary["extraction_precision"] == "unscored_no_human_gold"
    assert summary["failed_extraction_document_count"] == 0
    assert summary["cost_usd"] == 1.25
