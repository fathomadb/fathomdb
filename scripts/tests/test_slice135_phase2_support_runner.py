"""The support campaign rejects altered or incomplete derived gold."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest


PATH = Path(__file__).resolve().parents[1] / "slice135_phase2_support_runner.py"
SPEC = importlib.util.spec_from_file_location("slice135_phase2_support_runner", PATH)
assert SPEC is not None and SPEC.loader is not None
RUNNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUNNER)


def _data() -> dict:
    return {
        "dataset": "musique",
        "documents": [
            {"logical_id": "a", "body": "one", "source_id": "source"},
            {"logical_id": "b", "body": "two", "source_id": "source"},
        ],
        "queries": [{"query_id": "q", "query_class": "two_hop", "text": "question?",
                     "required_ids": ["a", "b"], "answers": ["answer"]}],
    }


def test_complete_input_accepts_named_support_and_denominator() -> None:
    data = _data()
    assert RUNNER.validate_inputs(data, {"documents": 2, "queries": 1,
                                         "by_class": {"two_hop": 1}}) == data


@pytest.mark.parametrize("alter", [
    lambda data: data["queries"][0]["required_ids"].append("missing"),
    lambda data: data["queries"].append(data["queries"][0]),
    lambda data: data["documents"].append(data["documents"][0]),
    lambda data: data["documents"][0].update(body=""),
])
def test_wrong_support_or_duplicate_corpus_rejected(alter) -> None:
    data = _data()
    alter(data)
    with pytest.raises(ValueError):
        RUNNER.validate_inputs(data, {"documents": 2, "queries": 1,
                                      "by_class": {"two_hop": 1}})
