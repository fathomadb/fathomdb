"""Contract tests for the FathomDB implementation of Mem0's OSS seam."""

from __future__ import annotations

from experiments.fathomdb_oss_facade import FathomDBOssStore, render_messages


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

    def write(self, batch):  # noqa: ANN001
        self.writes.append(list(batch))

    def drain(self, *, timeout_s: int) -> None:
        assert timeout_s == 30

    def search_text_only(self, query: str) -> _Result:
        return _Result([_Hit(f"hit for {query}", 0.7, "doc-1")])

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
    assert store.search({"user_id": "locomo_0_test", "query": "alpha", "limit": 10}) == {
        "results": [{"memory": "hit for alpha", "score": 0.7, "id": "doc-1"}]
    }
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
