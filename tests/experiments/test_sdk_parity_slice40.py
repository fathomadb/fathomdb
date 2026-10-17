"""Slice 40 Unit E: cross-SDK parity cases for the verbs Slice 40 changed.

The coordinator compares every SDK's ordered (id, score, branch) hits with a
Python-SDK oracle run on the same database file. These tests pin the fixture
builders, the comparison, the registry dispositions and the Python driver's hit
output; the TypeScript and Rust drivers are exercised by their own suites once
their artifacts are built from the slice head.
"""

from __future__ import annotations

import hashlib
import re
from pathlib import Path

import pytest

from experiments import sdk_benchmark, sdk_registry
from experiments.sdk_drivers import python_driver

D3_WORDS = {"s", "t", "ll", "re", "ve", "m", "my", "me", "us", "am", "so", "up"}


def _tokens(text: str) -> set[str]:
    return {token for token in re.split(r"[^0-9a-z]+", text.lower()) if token}


def _require_default_embedder(tmp_path: Path) -> None:
    from fathomdb import Engine

    try:
        Engine.open(str(tmp_path / "probe.sqlite"), use_default_embedder=True).close()
    except Exception as exc:  # noqa: BLE001 - the reason is printed in the skip
        reason = f"installed fathomdb wheel cannot open the default embedder: {type(exc).__name__}: {exc}"
        print(reason)
        pytest.skip(reason)


# --- comparison ------------------------------------------------------------------


def _hit(identifier: str, score: float, branch: str = "text") -> dict:
    return {"id": identifier, "score": score, "branch": branch}


def test_parity_requires_exact_id_score_and_branch_per_hit():
    expected = {"q1": [_hit("l:a", 1.5), _hit("l:b", 1.25)], "q2": []}
    assert sdk_benchmark.compare_parity(expected, {"q1": [_hit("l:a", 1.5), _hit("l:b", 1.25)], "q2": []}) == []
    nudged = 1.25 + 2 ** -50
    cases = {
        "order": {"q1": [_hit("l:b", 1.25), _hit("l:a", 1.5)], "q2": []},
        "score bit": {"q1": [_hit("l:a", 1.5), _hit("l:b", nudged)], "q2": []},
        "branch": {"q1": [_hit("l:a", 1.5), _hit("l:b", 1.25, "vector")], "q2": []},
        "length": {"q1": [_hit("l:a", 1.5)], "q2": []},
        "query set": {"q1": [_hit("l:a", 1.5), _hit("l:b", 1.25)]},
    }
    for label, observed in cases.items():
        assert sdk_benchmark.compare_parity(expected, observed), label


# --- fixtures ----------------------------------------------------------------------


def test_text_only_parity_fixture_carries_the_slice_40_query_shapes():
    fixture = sdk_benchmark.build_parity_fixture("text-only-v1")
    assert fixture["operation_id"] == "engine.search_text_only"
    bodies = [row["body"] for row in fixture["corpus"]]
    assert len(bodies) - len(set(bodies)) >= 2, "at least two duplicate-body rows"
    query_tokens = set().union(*(_tokens(q["text"]) for q in fixture["queries"]))
    assert D3_WORDS <= query_tokens, "every D3 word appears in some query"
    assert {"d", "c", "d3", "1", "ii"} <= query_tokens, "short content tokens appear"
    assert any("'" in q["text"] for q in fixture["queries"]), "a possessive or contraction"
    assert fixture == sdk_benchmark.build_parity_fixture("text-only-v1"), "deterministic"


def test_projected_parity_fixture_puts_a_bm25_tie_group_across_the_limit(tmp_path: Path):
    pytest.importorskip("fathomdb")
    from fathomdb import Engine, SearchFilter

    fixture = sdk_benchmark.build_parity_fixture("projected-v1")
    assert fixture["operation_id"] == "engine.search_projected_text"
    assert {case["filter"] is None for case in fixture["filters"]} == {True, False}
    database = tmp_path / "projected.sqlite"
    sdk_benchmark.populate_parity_database(database, fixture)
    engine = Engine.open(str(database), use_default_embedder=False)
    try:
        for query in fixture["queries"]:
            hits = engine.search_projected_text(query["text"], fixture["projection"], None, limit=100).results
            limit = fixture["limit"]
            assert len(hits) > limit
            assert hits[limit - 1].score == hits[limit].score, "a tie group straddles the limit"
        cohort = next(case["filter"] for case in fixture["filters"] if case["filter"] is not None)
        filtered = engine.search_projected_text(
            fixture["queries"][0]["text"], fixture["projection"],
            SearchFilter(attributes=tuple(tuple(pair) for pair in cohort["attributes"])), limit=100,
        ).results
        assert 0 < len(filtered) < len(hits)
    finally:
        engine.close()


def test_dense_parity_fixture_store_embeds_every_document(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("FATHOMDB_EMBED_DEVICE", "cpu")
    _require_default_embedder(tmp_path)
    import sqlite3

    from fathomdb import Engine

    fixture = sdk_benchmark.build_parity_fixture("dense-v1")
    assert fixture["operation_id"] == "engine.search"
    database = tmp_path / "dense.sqlite"
    sdk_benchmark.populate_parity_database(database, fixture)
    with sqlite3.connect(database) as connection:
        vectors = connection.execute(
            "SELECT COUNT(DISTINCT write_cursor) FROM _fathomdb_vector_rows WHERE kind = 'doc'"
        ).fetchone()[0]
    assert vectors == len(fixture["corpus"])
    engine = Engine.open(str(database), use_default_embedder=True)
    try:
        branches = {hit.branch for q in fixture["queries"] for hit in engine.search(q["text"], limit=10).results}
    finally:
        engine.close()
    assert "vector" in branches


# --- registry and drivers ------------------------------------------------------------


def test_registry_measures_the_three_slice_40_verbs_with_ordered_hits():
    registry = sdk_registry.load_registry()
    for operation_id in ("engine.search_text_only", "engine.search_projected_text", "engine.search"):
        entry = registry[operation_id]
        assert entry["disposition"] == "measure", operation_id
        assert entry["normalizer"] in {"ordered_ids", "ordered_hits"}
    assert registry["engine.search_projected_text"]["normalizer"] == "ordered_hits"
    assert registry["engine.search"]["normalizer"] == "ordered_hits"


def _native_sha() -> str:
    from fathomdb import _fathomdb

    return hashlib.sha256(Path(_fathomdb.__file__).read_bytes()).hexdigest()


def _request(database: Path, fixture: dict, arguments: dict) -> dict:
    return {
        "schema_version": "sdk.driver.v1",
        "mode": "run-repetition",
        "sdk": "python",
        "operation_id": fixture["operation_id"],
        "operation_version": 1,
        "database_path": str(database),
        "queries": [{"id": q["id"], "text": q["text"]} for q in fixture["queries"]],
        "arguments": arguments,
        "normalizer": "ordered_hits",
        "warmup_per_query": 0,
        "timeout_ms": 30000,
        "expected_native_sha256": _native_sha(),
    }


@pytest.mark.parametrize("case_id", ["text-only-v1", "projected-v1"])
def test_python_driver_hits_equal_the_python_oracle_on_the_same_database(case_id: str, tmp_path: Path):
    pytest.importorskip("fathomdb")
    fixture = sdk_benchmark.build_parity_fixture(case_id)
    database = tmp_path / "parity.sqlite"
    sdk_benchmark.populate_parity_database(database, fixture)
    arguments = {"limit": fixture["limit"], "view": {"valid_as_of": 1700000000}}
    if case_id == "projected-v1":
        arguments.update(name=fixture["projection"], filter=None)
    response = python_driver.execute(_request(database, fixture, arguments))
    assert response["status"] == "complete", response["error"]
    observed = {sample["query_id"]: sample["hits"] for sample in response["samples"]}
    expected = sdk_benchmark.python_oracle(database, fixture, filter=None)
    assert sdk_benchmark.compare_parity(expected, observed) == []
    assert all(set(hit) == {"id", "score", "branch"} for hits in observed.values() for hit in hits)


def test_python_driver_dense_hits_equal_the_python_oracle(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("FATHOMDB_EMBED_DEVICE", "cpu")
    _require_default_embedder(tmp_path)
    fixture = sdk_benchmark.build_parity_fixture("dense-v1")
    database = tmp_path / "dense.sqlite"
    sdk_benchmark.populate_parity_database(database, fixture)
    arguments = {"limit": fixture["limit"], "view": {"valid_as_of": 1700000000}, "use_default_embedder": True}
    response = python_driver.execute(_request(database, fixture, arguments))
    assert response["status"] == "complete", response["error"]
    observed = {sample["query_id"]: sample["hits"] for sample in response["samples"]}
    assert sdk_benchmark.compare_parity(sdk_benchmark.python_oracle(database, fixture, filter=None), observed) == []
    assert any(hit["branch"] == "vector" for hits in observed.values() for hit in hits)


def test_pre_existing_requests_without_a_normalizer_still_return_ids_only(tmp_path: Path):
    pytest.importorskip("fathomdb")
    fixture = sdk_benchmark.build_parity_fixture("text-only-v1")
    database = tmp_path / "ids.sqlite"
    sdk_benchmark.populate_parity_database(database, fixture)
    request = _request(database, fixture, {"limit": 10, "view": {"valid_as_of": 1700000000}})
    del request["normalizer"]
    response = python_driver.execute(request)
    assert response["status"] == "complete"
    assert all(set(sample) == {"query_id", "elapsed_ns", "ids"} for sample in response["samples"])
