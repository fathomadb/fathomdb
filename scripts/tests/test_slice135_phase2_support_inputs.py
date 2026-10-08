"""Qualified support-set inputs preserve source labels and complete sets."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path

import pytest


PATH = Path(__file__).resolve().parents[1] / "slice135_phase2_support_inputs.py"
SPEC = importlib.util.spec_from_file_location("slice135_phase2_support_inputs", PATH)
assert SPEC is not None and SPEC.loader is not None
INPUTS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INPUTS)


def _musique_rows() -> list[dict[str, object]]:
    rows = [
        {"id": "q1", "answerable": True, "hop_count": 2, "question": "Question one?",
         "answer": "A", "paragraphs": [
             {"idx": 0, "text": "shared support", "is_supporting": True},
             {"idx": 1, "text": "first second support", "is_supporting": True},
             {"idx": 2, "text": "first distractor", "is_supporting": False},
         ]},
        {"id": "q2", "answerable": True, "hop_count": 2, "question": "Question two?",
         "answer": "B", "paragraphs": [
             {"idx": 0, "text": "shared support", "is_supporting": True},
             {"idx": 1, "text": "second support", "is_supporting": True},
             {"idx": 2, "text": "second distractor", "is_supporting": False},
         ]},
        {"id": "q3", "answerable": False, "hop_count": 2, "question": "Absent?",
         "answer": "C", "paragraphs": [
             {"idx": 0, "text": "negative context", "is_supporting": False},
         ]},
    ]
    for row in rows:
        for paragraph in row["paragraphs"]:
            paragraph["title"] = "Fixture title"
    return rows


def test_musique_uses_answerable_support_labels_and_deduplicates_body(tmp_path: Path) -> None:
    path = tmp_path / "musique.jsonl"
    path.write_text("".join(json.dumps(row) + "\n" for row in _musique_rows()))
    data = INPUTS.build_musique(path, quotas={2: 2}, salt="fixture")
    assert len(data["documents"]) == 5
    assert len(data["queries"]) == 2
    assert {q["query_class"] for q in data["queries"]} == {"two_hop"}
    assert all(len(q["required_ids"]) == 2 for q in data["queries"])
    assert len(set(data["queries"][0]["required_ids"]) &
               set(data["queries"][1]["required_ids"])) == 1
    assert {q["query_id"] for q in data["queries"]} == {"q1", "q2"}


def test_musique_incomplete_support_set_is_rejected(tmp_path: Path) -> None:
    rows = _musique_rows()
    rows[0]["paragraphs"][1]["is_supporting"] = False
    path = tmp_path / "musique.jsonl"
    path.write_text("".join(json.dumps(row) + "\n" for row in rows))
    with pytest.raises(ValueError, match="support"):
        INPUTS.build_musique(path, quotas={2: 2}, salt="fixture")


def test_musique_preserves_source_title_in_evidence_body(tmp_path: Path) -> None:
    rows = _musique_rows()
    for row in rows:
        for paragraph in row["paragraphs"]:
            paragraph["title"] = "Source title"
    path = tmp_path / "musique.jsonl"
    path.write_text("".join(json.dumps(row) + "\n" for row in rows))
    data = INPUTS.build_musique(path, quotas={2: 2}, salt="fixture")
    assert all(doc["body"].startswith("Source title\n") for doc in data["documents"])


def test_locomo_session_mapping_and_strict_multisession_flag(tmp_path: Path) -> None:
    path = tmp_path / "locomo.json"
    path.write_text(json.dumps([{
        "sample_id": "conv-x",
        "conversation": {
            "session_1": [{"speaker": "Alex", "text": "Lives in Austin."}],
            "session_2": [{"speaker": "Alex", "text": "Moved to Denver."}],
        },
        "qa": [
            {"category": 1, "question": "Where before and after?", "answer": "Austin then Denver",
             "evidence": ["D1:1", "D2:1"]},
            {"category": 2, "question": "Where now?", "answer": "Denver",
             "evidence": ["D2:1"]},
            {"category": 3, "question": "General knowledge?", "answer": "World",
             "evidence": ["D1:1"]},
        ],
    }]))
    data = INPUTS.build_locomo(path)
    assert len(data["documents"]) == 2
    assert len(data["queries"]) == 2
    assert {doc["conversation"] for doc in data["documents"]} == {"conv-x"}
    assert {q["scope_conversation"] for q in data["queries"]} == {"conv-x"}
    multi = next(q for q in data["queries"] if q["query_class"] == "multi_session")
    assert multi["strict_multi_session"] is True
    assert multi["required_ids"] == ["conv-x:session_1", "conv-x:session_2"]
    temporal = next(q for q in data["queries"] if q["query_class"] == "temporal")
    assert temporal["required_ids"] == ["conv-x:session_2"]
