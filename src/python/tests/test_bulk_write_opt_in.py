"""0.8.32 Slice 30 (`WP-30B`) — Python SDK opt-in bulk write mode parity."""

from __future__ import annotations

from typing import Any

import pytest

import fathomdb
from fathomdb import (
    EndpointValidationMode,
    Engine,
    ScopePolicy,
    ScopeViolationError,
    WriteOptions,
    graph,
    read,
)
from fathomdb.errors import WriteValidationError
from fathomdb.types import EndpointValidationMode as TypesEndpointValidationMode
from fathomdb.types import WriteOptions as TypesWriteOptions


def _node(logical_id: str, body: str, source_id: str = "bulk-src") -> dict[str, Any]:
    return {
        "kind": "note",
        "body": body,
        "logical_id": logical_id,
        "source_id": source_id,
    }


def _edge(
    from_id: str,
    to_id: str,
    *,
    logical_id: str | None = None,
    source_id: str = "bulk-src",
) -> dict[str, Any]:
    edge: dict[str, Any] = {
        "kind": "links",
        "from": from_id,
        "to": to_id,
        "source_id": source_id,
    }
    if logical_id is not None:
        edge["logical_id"] = logical_id
    return {"edge": edge}


def test_write_options_exports_and_defaults() -> None:
    assert WriteOptions is TypesWriteOptions
    assert EndpointValidationMode == TypesEndpointValidationMode
    assert not hasattr(fathomdb, "write")
    default_opts = WriteOptions()
    assert default_opts.endpoint_validation == "strict"
    bulk_opts = WriteOptions(endpoint_validation="bulk_non_strict")
    assert bulk_opts.endpoint_validation == "bulk_non_strict"


def test_strict_default_refuses_dangling_endpoints(db_path: str) -> None:
    engine = Engine.open(db_path)
    try:
        with pytest.raises(WriteValidationError):
            engine.write([_edge("n:a", "n:b")])
        with pytest.raises(WriteValidationError):
            engine.write_with_options([_edge("n:a", "n:b")])
        with pytest.raises(WriteValidationError):
            engine.write_with_options([_edge("n:a", "n:b")], None)
        with pytest.raises(WriteValidationError):
            engine.write_with_options([_edge("n:a", "n:b")], WriteOptions())
        with pytest.raises(WriteValidationError):
            engine.write_with_options(
                [_edge("n:a", "n:b")], WriteOptions(endpoint_validation="strict")
            )
    finally:
        engine.close()


def test_bulk_non_strict_counts_dangling_and_resolves_later(db_path: str) -> None:
    engine = Engine.open(db_path)
    try:
        bulk = WriteOptions(endpoint_validation="bulk_non_strict")

        r1 = engine.write_with_options([_edge("n:a", "n:b", logical_id="e:ab")], bulk)
        assert r1.dangling_edge_endpoints == 2

        r2 = engine.write_with_options(
            [_node("n:a", "alpha"), _edge("n:a", "n:c", logical_id="e:ac")],
            bulk,
        )
        assert r2.dangling_edge_endpoints == 1

        # Node-only batch takes the zero-SQL fast path and reports 0.
        r3 = engine.write_with_options(
            [_node("n:b", "beta"), _node("n:c", "gamma")],
            bulk,
        )
        assert r3.dangling_edge_endpoints == 0

        # Previously admitted edges resolve without rewriting them.
        neighbors = graph.neighbors(engine, "n:a", depth=1)
        assert sorted(n.logical_id for n in neighbors) == ["n:b", "n:c"]
    finally:
        engine.close()


def test_bulk_non_strict_refuses_cross_scope_and_read_only(tmp_path: Any) -> None:
    db = str(tmp_path / "scoped.sqlite")
    bulk = WriteOptions(endpoint_validation="bulk_non_strict")

    scope_a = Engine.open(db, scope_policy=ScopePolicy.single("scope-a"))
    try:
        scope_a.write([_node("shared:a", "alpha in a", source_id="src-a")])
    finally:
        scope_a.close()

    scope_b = Engine.open(db, scope_policy=ScopePolicy.single("scope-b"))
    try:
        with pytest.raises(ScopeViolationError):
            scope_b.write_with_options(
                [
                    _node("local:b", "beta in b", source_id="src-b"),
                    _edge("local:b", "shared:a", source_id="src-b"),
                ],
                bulk,
            )
        # Atomic rollback: local:b must not have landed.
        assert read.get(scope_b, "local:b") is None
    finally:
        scope_b.close()

    ro = Engine.open(db, scope_policy=ScopePolicy.read_only(["scope-a"]))
    try:
        with pytest.raises(ScopeViolationError):
            ro.write_with_options([_node("ro:1", "denied")], bulk)
    finally:
        ro.close()


def test_write_with_options_rejects_invalid_option_types_and_literals(db_path: str) -> None:
    engine = Engine.open(db_path)
    try:
        for bad_opts in ({"endpoint_validation": "bulk_non_strict"}, "bulk_non_strict", 123, True):
            with pytest.raises(TypeError):
                engine.write_with_options([], bad_opts)  # type: ignore[arg-type]

        for bad_mode_type in (123, None, True, ["strict"]):
            with pytest.raises(TypeError):
                engine.write_with_options(
                    [],
                    WriteOptions(endpoint_validation=bad_mode_type),  # type: ignore[arg-type]
                )

        for bad_mode_value in ("", "non_strict", "Strict", "BULK_NON_STRICT", "off"):
            with pytest.raises(ValueError):
                engine.write_with_options(
                    [],
                    WriteOptions(endpoint_validation=bad_mode_value),  # type: ignore[arg-type]
                )

        # Native boundary also validates direct calls.
        with pytest.raises(TypeError):
            engine._native.write_with_options([], 123)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            engine._native.write_with_options([], "unknown_mode")
    finally:
        engine.close()
