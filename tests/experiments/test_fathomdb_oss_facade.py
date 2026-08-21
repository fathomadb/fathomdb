"""Contract tests for the FathomDB implementation of Mem0's OSS seam."""

from __future__ import annotations

import json

import pytest

from experiments import fathomdb_oss_facade as facade
from experiments.fathomdb_oss_facade import FathomDBOssStore, render_messages
from experiments.locomo_provenance import ProvenanceEntry, ProvenanceMap, payload_fingerprint


class _Hit:
    def __init__(self, body: str, score: float, value: str) -> None:
        self.body = body
        self.score = score
        self.id = type("Id", (), {"value": value})()


class _Result:
    def __init__(self, hits: list[_Hit]) -> None:
        self.results = hits


class _Engine:
    def __init__(self, _path: str) -> None:
        self.writes: list[list[dict]] = []
        self.closed = False
        self.logical_id = "doc-1"

    def write(self, batch):  # noqa: ANN001
        self.writes.append(list(batch))
        self.logical_id = batch[0]["logical_id"]

    def drain(self, *, timeout_s: int) -> None:
        assert timeout_s == 30

    def search_text_only(self, query: str) -> _Result:
        return _Result([_Hit(f"hit for {query}", 0.7, self.logical_id)])

    def close(self) -> None:
        self.closed = True


def test_render_messages_is_stable_and_preserves_roles():
    assert render_messages([
        {"role": "user", "content": "hello"},
        {"role": "assistant", "content": "hi"},
    ]) == "user: hello\nassistant: hi"


def test_store_add_search_and_delete_are_user_isolated(tmp_path):
    stores: list[_Engine] = []

    def engine_factory(path: str) -> _Engine:
        engine = _Engine(path)
        stores.append(engine)
        return engine

    store = FathomDBOssStore(tmp_path, engine_factory=engine_factory)
    response = store.add({
        "user_id": "locomo_0_test",
        "messages": [{"role": "user", "content": "alpha"}],
        "timestamp": 123,
    })

    assert response == {"results": []}
    written = stores[0].writes[0][0]
    assert written["kind"] == "locomo_message_chunk"
    assert written["body"] == "user: alpha"
    assert written["source_id"].startswith("mem0-oss:")
    assert "locomo_0_test" not in written["source_id"]
    assert written["logical_id"] == f"{written['source_id']}:0"
    search = store.search({"user_id": "locomo_0_test", "query": "alpha", "limit": 10})
    assert search["results"][0]["memory"] == "hit for alpha"
    assert search["results"][0]["score"] == 0.7
    assert store.search({"user_id": "other", "query": "alpha", "limit": 10}) == {"results": []}

    store.delete_user("locomo_0_test")
    assert stores[0].closed is True


def test_store_rejects_a_limit_that_fathomdb_fts_cannot_honor(tmp_path):
    store = FathomDBOssStore(tmp_path, engine_factory=_Engine)
    try:
        store.search({"user_id": "missing", "query": "x", "limit": 11})
    except ValueError as exc:
        assert "limit" in str(exc)
    else:  # pragma: no cover - assertion guard
        raise AssertionError("limit > 10 must be rejected")


def test_store_rejects_unmapped_ingest_and_returns_safe_provenance_and_timings(tmp_path):
    payload = {"user_id": "locomo_0_run", "messages": [{"role": "user", "content": "alpha"}]}
    provenance = ProvenanceMap([ProvenanceEntry(
        payload_fingerprint(payload), "locomo-0", "session-1", ("D1:1",),
    )])
    store = FathomDBOssStore(tmp_path, engine_factory=_Engine, provenance=provenance)

    assert store.add(payload) == {"results": []}
    result = store.search({"user_id": "locomo_0_run", "query": "alpha", "limit": 10})

    assert result["results"][0]["evaluation_provenance"] == {
        "conversation_id": "locomo-0", "session_id": "session-1", "turn_ids": ["D1:1"],
    }
    metrics = store.metrics_snapshot()
    assert metrics["facade_query_ms"]["n"] == 1
    assert metrics["engine_query_ms"]["n"] == 1
    assert metrics["ingest_ack_ms"]["n"] == 1
    assert metrics["ready_to_search_ms"]["n"] == 1
    provenance_sidecar = store.provenance_snapshot()
    encoded = json.dumps(provenance_sidecar)
    assert provenance_sidecar["schema_version"] == "locomo-facade-provenance.v1"
    assert len(provenance_sidecar["requests"]) == 1
    assert "alpha" not in encoded
    assert provenance_sidecar["requests"][next(iter(provenance_sidecar["requests"]))] == [{
        "conversation_id": "locomo-0", "session_id": "session-1", "turn_ids": ["D1:1"],
    }]

    with pytest.raises(ValueError, match="unmapped"):
        store.add({"user_id": "locomo_0_run", "messages": [{"role": "user", "content": "unknown"}]})


def test_facade_main_requires_a_content_free_provenance_manifest(tmp_path):
    with pytest.raises(SystemExit):
        facade.main(["--root", str(tmp_path)])
