"""IR runner input mapping must preserve human gold IDs and denominators."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path

import pytest


PATH = Path(__file__).resolve().parents[1] / "slice135_phase2_ir_runner.py"
SPEC = importlib.util.spec_from_file_location("slice135_phase2_ir_runner", PATH)
assert SPEC is not None and SPEC.loader is not None
RUNNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUNNER)


def _sources(tmp_path: Path) -> dict[str, Path]:
    sources = {}
    for source, rows in {
        "enronqa": [("b", "Body B"), ("a", "Body A")],
        "qmsum": [("c", "Body C")],
    }.items():
        path = tmp_path / f"{source}.jsonl"
        path.write_text("".join(json.dumps({"doc_id": doc_id, "body": body}) + "\n"
                                for doc_id, body in rows))
        sources[source] = path
    return sources


def _gold() -> dict[str, object]:
    return {"queries": [
        {"query_id": "enronqa:q1", "source": "enronqa", "query_class": "exact_fact",
         "query": "Who?", "expected_top_k_doc_ids": ["a"],
         "required_evidence": [{"doc_id": "a", "necessity": "required"}]},
        {"query_id": "qmsum:q2", "source": "qmsum", "query_class": "exploratory",
         "query": "What happened?", "expected_top_k_doc_ids": ["c"],
         "required_evidence": [{"doc_id": "c", "necessity": "required"}]},
        {"query_id": "enronqa:q3", "source": "enronqa", "query_class": "negative",
         "query": "Unanswerable?", "expected_top_k_doc_ids": [], "required_evidence": []},
    ]}


def test_sorted_documents_and_positive_gold_keep_source_id_mapping(tmp_path: Path) -> None:
    documents = RUNNER.build_documents(_sources(tmp_path))
    assert [row["logical_id"] for row in documents] == [
        "enronqa:a", "enronqa:b", "qmsum:c",
    ]
    assert [json.loads(row["body"])["summary"] for row in documents] == [
        "Body A", "Body B", "Body C",
    ]
    positive, omitted = RUNNER.select_positive_queries(
        _gold(), {row["logical_id"] for row in documents}
    )
    assert [row["query_id"] for row in positive] == ["enronqa:q1", "qmsum:q2"]
    assert [row["required_id"] for row in positive] == ["enronqa:a", "qmsum:c"]
    assert omitted == 1


def test_missing_gold_document_and_duplicate_source_id_are_rejected(tmp_path: Path) -> None:
    documents = RUNNER.build_documents(_sources(tmp_path))
    gold = _gold()
    gold["queries"][0]["expected_top_k_doc_ids"] = ["absent"]
    gold["queries"][0]["required_evidence"] = [{"doc_id": "absent", "necessity": "required"}]
    with pytest.raises(ValueError, match="required"):
        RUNNER.select_positive_queries(gold, {row["logical_id"] for row in documents})
    sources = _sources(tmp_path)
    with sources["enronqa"].open("a") as out:
        out.write(json.dumps({"doc_id": "a", "body": "Duplicate"}) + "\n")
    with pytest.raises(ValueError, match="duplicate"):
        RUNNER.build_documents(sources)


def test_duplicate_gold_query_id_and_disagreeing_evidence_are_rejected(tmp_path: Path) -> None:
    documents = RUNNER.build_documents(_sources(tmp_path))
    gold = _gold()
    gold["queries"][1]["query_id"] = gold["queries"][0]["query_id"]
    with pytest.raises(ValueError, match="duplicate"):
        RUNNER.select_positive_queries(gold, {row["logical_id"] for row in documents})
    gold = _gold()
    gold["queries"][0]["required_evidence"] = [{"doc_id": "b", "necessity": "required"}]
    with pytest.raises(ValueError, match="required"):
        RUNNER.select_positive_queries(gold, {row["logical_id"] for row in documents})
