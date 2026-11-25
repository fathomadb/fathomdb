"""0.8.32 Slice 20 (`WP-20D`) — Python SDK caller-scope isolation & view parity."""

from __future__ import annotations

import pytest

from fathomdb import (
    Engine,
    EngineConfig,
    ReadView,
    ScopePolicy,
    ScopeViolationError,
    graph,
    read,
)
from fathomdb.errors import InvalidArgumentError


def _node(
    logical_id: str,
    body: str,
    source_id: str,
    *,
    valid_from: int | None = None,
    valid_until: int | None = None,
) -> dict:
    item: dict[str, object] = {
        "kind": "note",
        "body": body,
        "logical_id": logical_id,
        "source_id": source_id,
    }
    if valid_from is not None:
        item["valid_from"] = valid_from
    if valid_until is not None:
        item["valid_until"] = valid_until
    return item


def _edge(logical_id: str, from_id: str, to_id: str, source_id: str) -> dict:
    return {
        "edge": {
            "kind": "links",
            "from": from_id,
            "to": to_id,
            "logical_id": logical_id,
            "source_id": source_id,
        }
    }


def test_invalid_scope_policy_open_rejections(db_path: str) -> None:
    invalid_policies = [
        ScopePolicy.read_only([]),
        ScopePolicy.single(""),
        ScopePolicy.single("*"),
        ScopePolicy.read_write(["agent-a"], "agent-b"),
    ]
    for policy in invalid_policies:
        with pytest.raises(InvalidArgumentError):
            Engine.open(db_path, scope_policy=policy)

    with pytest.raises(TypeError):
        ScopePolicy.single(123)  # type: ignore[arg-type]


def test_two_callers_are_isolated_and_read_only_raises_scope_violation(db_path: str) -> None:
    a = Engine.open(db_path, config=EngineConfig(scope_policy=ScopePolicy.single("a")))
    a.write([_node("a:only", "scopedneedle alpha private", "src-a")])
    a.drain(timeout_s=10)
    a.close()

    b = Engine.open(db_path, scope_policy=ScopePolicy.single("b"))
    b.write([_node("b:only", "scopedneedle beta private", "src-b")])
    b.drain(timeout_s=10)
    assert read.get(b, "a:only") is None
    assert graph.search_expand(b, "scopedneedle", 1).all_logical_ids == ["b:only"]
    b.close()

    a = Engine.open(db_path, config=EngineConfig(scope_policy=ScopePolicy.single("a")))
    assert read.get(a, "b:only") is None
    assert [hit.id.value for hit in a.search_text_only("scopedneedle").results] == ["a:only"]
    a.close()

    ro = Engine.open(db_path, scope_policy=ScopePolicy.read_only(["a"]))
    try:
        assert read.get(ro, "a:only") is not None
        assert read.get(ro, "b:only") is None
        with pytest.raises(ScopeViolationError):
            ro.write([_node("a:blocked", "cannot write", "src-ro")])
    finally:
        ro.close()


def test_graph_search_expand_view_honors_caller_scope(db_path: str) -> None:
    a = Engine.open(db_path, scope_policy=ScopePolicy.single("a"))
    a.write(
        [
            _node("a:root", "expandanchor alpha", "src-a"),
            _node("a:past", "past leaf alpha", "src-a", valid_from=1, valid_until=2),
            _edge("e:a", "a:root", "a:past", "src-a"),
        ]
    )
    a.drain(timeout_s=10)
    a.close()

    b = Engine.open(db_path, scope_policy=ScopePolicy.single("b"))
    b.write(
        [
            _node("b:root", "expandanchor beta", "src-b"),
            _node("b:past", "past leaf beta", "src-b", valid_from=1, valid_until=2),
            _edge("e:b", "b:root", "b:past", "src-b"),
        ]
    )
    b.drain(timeout_s=10)
    b.close()

    a = Engine.open(db_path, scope_policy=ScopePolicy.single("a"))
    try:
        strict = graph.search_expand(a, "expandanchor", 1)
        assert strict.all_logical_ids == ["a:root"]

        relaxed = graph.search_expand(
            a,
            "expandanchor",
            1,
            view=ReadView(include_out_of_window=True),
        )
        assert [hit.id.value for hit in relaxed.search_hits] == ["a:root"]
        assert [item.node.logical_id for item in relaxed.expanded] == ["a:past"]
        assert not any(logical_id.startswith("b:") for logical_id in relaxed.all_logical_ids)
    finally:
        a.close()
