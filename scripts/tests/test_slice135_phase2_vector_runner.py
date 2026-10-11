"""Phase 2 vector runner fixes corpus selection and isolates the lexical arm."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path

import fathomdb


SCRIPT = Path(__file__).resolve().parents[1] / "slice135_phase2_vector_runner.py"
SPEC = importlib.util.spec_from_file_location("slice135_phase2_vector_runner", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
RUNNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUNNER)


def test_corpus_and_query_selection_is_stable(tmp_path: Path) -> None:
    source_paths = {}
    for name in ("alpha", "beta"):
        path = tmp_path / f"{name}.jsonl"
        rows = [
            {"doc_id": f"{index:02}", "title": f"Title {name} {index}", "body": f"Body {name} {index}. More text."}
            for index in (3, 1, 0, 2)
        ]
        path.write_text("".join(json.dumps(row) + "\n" for row in rows))
        source_paths[name] = path
    documents, queries = RUNNER.select_corpus_and_queries(
        source_paths, per_source=4, query_stride=2
    )
    assert [row["logical_id"] for row in documents] == [
        "alpha-000", "alpha-001", "alpha-002", "alpha-003",
        "beta-000", "beta-001", "beta-002", "beta-003",
    ]
    assert [row["body"] for row in documents[:2]] == [
        json.dumps({"summary": "Body alpha 0. More text."}),
        json.dumps({"summary": "Body alpha 1. More text."}),
    ]
    assert [(q["query_id"], q["text"], q["target_id"]) for q in queries] == [
        ("alpha:000", "Title alpha 0", "alpha-000"),
        ("alpha:002", "Title alpha 2", "alpha-002"),
        ("beta:000", "Title beta 0", "beta-000"),
        ("beta:002", "Title beta 2", "beta-002"),
    ]


def test_lexical_isolation_preserves_canonical_rows(tmp_path: Path) -> None:
    database = tmp_path / "vector.sqlite"
    before = tmp_path / "pre-isolation.sqlite"
    engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
    try:
        engine.write([
            {"kind": "doc", "body": "rare orchard phrase", "logical_id": "A", "source_id": "fixture"},
            {"kind": "doc", "body": "unrelated river text", "logical_id": "B", "source_id": "fixture"},
        ])
        assert [h.id.value for h in engine.search_text_only("orchard").results] == ["A"]
    finally:
        engine.close()
    counts = RUNNER.isolate_lexical_index(database, before)
    assert counts["pre"]["search_index_v2"] == 2
    assert counts["post"]["search_index_v2"] == 0
    assert before.is_file()
    engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
    try:
        assert not engine.search_text_only("orchard").results
        assert fathomdb.read.get(engine, "A").body == "rare orchard phrase"
    finally:
        engine.close()
