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
        eligibility=fathomdb.SearchFilter(attributes=(("owner", "alice"),))
    )

    frozen = engine.freeze_read_context(context)
    result = engine.search_frozen("needle", frozen, limit=1)

    assert frozen.schema_version == 1
    assert frozen.context.eligibility == context.eligibility
    assert frozen.context.view.valid_as_of == frozen.effective_valid_at
    assert [hit.id.value for hit in result.results] == ["allowed"]
    engine.close()


def test_frozen_context_rejects_state_drift(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    engine.write([_doc("before", "needle before", "alice")])
    frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
    engine.write([_doc("after", "needle after", "alice")])

    with pytest.raises(fathomdb.errors.FrozenReadError, match="state_drifted"):
        engine.search_frozen("needle", frozen)
    engine.close()


def test_frozen_search_expand_uses_the_same_context(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    engine.write([_doc("root", "expand needle", "alice")])
    frozen = engine.freeze_read_context(
        fathomdb.ReadContextV1(
            eligibility=fathomdb.SearchFilter(attributes=(("owner", "alice"),))
        )
    )

    result = engine.search_expand_frozen("expand needle", frozen, depth=0)

    assert result.all_logical_ids == ["root"]
    assert result.expanded == []
    engine.close()


def test_frozen_authentication_precedes_query_and_range_validation(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
    malformed = fathomdb.FrozenReadContextV1(
        schema_version=frozen.schema_version,
        effective_valid_at=frozen.effective_valid_at,
        context=frozen.context,
        token=frozen.token + "0",
    )

    with pytest.raises(fathomdb.errors.FrozenReadError, match="token_malformed"):
        engine.search_frozen("", malformed, limit=-1)
    with pytest.raises(fathomdb.errors.FrozenReadError, match="token_malformed"):
        engine.search_expand_frozen("", malformed, depth=-1, limit=-1)
    engine.close()
