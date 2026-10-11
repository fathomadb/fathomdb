"""Vector-only fidelity must remove duplicate body fusion and rank inversions."""

from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path

import pytest


SCRIPTS = Path(__file__).resolve().parents[1]


def _module(name: str):
    path = SCRIPTS / f"{name}.py"
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


RUNNER = _module("slice135_phase2_vector_runner")
SCORER = _module("slice135_phase2_vector_audit")


def _hash(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def test_corpus_selection_replaces_duplicate_bodies_from_same_and_other_sources(
    tmp_path: Path,
) -> None:
    source_paths = {}
    rows = {
        "alpha": [("00", "shared"), ("01", "shared"), ("02", "alpha two"),
                  ("03", "alpha three"), ("04", "alpha four")],
        "beta": [("00", "shared"), ("01", "beta one"), ("02", "beta two"),
                 ("03", "beta three")],
    }
    for source, values in rows.items():
        path = tmp_path / f"{source}.jsonl"
        path.write_text("".join(json.dumps({
            "doc_id": doc_id, "title": f"Query {source} {doc_id}", "body": body,
        }) + "\n" for doc_id, body in values))
        source_paths[source] = path
    documents, queries = RUNNER.select_corpus_and_queries(
        source_paths, per_source=3, query_stride=1
    )
    assert [(row["logical_id"], row["source_doc_id"]) for row in documents] == [
        ("alpha-000", "00"), ("alpha-001", "02"), ("alpha-002", "03"),
        ("beta-000", "01"), ("beta-001", "02"), ("beta-002", "03"),
    ]
    assert len({row["body"] for row in documents}) == 6
    assert len(queries) == 6


def test_fidelity_score_rejects_nonmonotonic_vector_hit_order() -> None:
    documents = [
        {"logical_id": "A", "body_sha256": _hash("A"), "vector": [0.0, 0.0]},
        {"logical_id": "B", "body_sha256": _hash("B"), "vector": [0.1, 0.0]},
        {"logical_id": "C", "body_sha256": _hash("C"), "vector": [0.2, 0.0]},
        {"logical_id": "D", "body_sha256": _hash("D"), "vector": [0.3, 0.0]},
    ]
    receipt = {
        "documents": documents,
        "queries": [{
            "query_id": "q1", "target_body_sha256": _hash("A"),
            "vector": [0.0, 0.0], "text_only_count": 0,
            "hits": [
                {"logical_id": "A", "body_sha256": _hash("A"), "branch": "vector"},
                {"logical_id": "C", "body_sha256": _hash("C"), "branch": "vector"},
                {"logical_id": "B", "body_sha256": _hash("B"), "branch": "vector"},
                {"logical_id": "D", "body_sha256": _hash("D"), "branch": "vector"},
            ],
        }],
    }
    with pytest.raises(ValueError, match="rank inversion"):
        SCORER.score_version(receipt, ["q1"], k=2, dimension=2)
