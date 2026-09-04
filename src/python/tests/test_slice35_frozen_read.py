"""Slice 35 frozen-read parity tests for the public Python SDK."""

from __future__ import annotations

import pytest

import fathomdb


def _doc(logical_id: str, body: str, owner: str) -> dict[str, object]:
    return {
        "kind": "doc",
        "body": body,
        "logical_id": logical_id,
        "source_id": f"slice35:{logical_id}",
        "properties": {"owner": owner},
    }


def test_frozen_search_preserves_context_and_filters_before_ranking(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    engine.write(
        [
            _doc("allowed", "needle allowed", "alice"),
            _doc("excluded", "needle excluded", "bob"),
        ]
    )
    context = fathomdb.ReadContextV1(
        eligibility=fathomdb.SearchFilter(attributes=[("owner", "alice")])
    )

    frozen = engine.freeze_read_context(context)
    result = engine.search_frozen("needle", frozen, limit=1)

    assert frozen.schema_version == 1
    assert frozen.context == context
    assert [hit.id.value for hit in result.results] == ["allowed"]
    engine.close()


def test_frozen_context_rejects_state_drift(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    engine.write([_doc("before", "needle before", "alice")])
    frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
    engine.write([_doc("after", "needle after", "alice")])

    with pytest.raises(fathomdb.errors.FrozenReadError, match="snapshot_drift"):
        engine.search_frozen("needle", frozen)
    engine.close()


def test_frozen_search_expand_uses_the_same_context(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    engine.write([_doc("root", "expand needle", "alice")])
    frozen = engine.freeze_read_context(
        fathomdb.ReadContextV1(
            eligibility=fathomdb.SearchFilter(attributes=[("owner", "alice")])
        )
    )

    result = engine.search_expand_frozen("expand needle", frozen, depth=0)

    assert result.all_logical_ids == ["root"]
    assert result.expanded == []
    engine.close()
