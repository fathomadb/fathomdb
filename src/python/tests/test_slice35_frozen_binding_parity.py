"""Slice 35 frozen methods retain the ordinary Python argument contract."""

from __future__ import annotations

import pytest

import fathomdb


@pytest.mark.parametrize(
    ("kwargs", "error"),
    [
        ({"rerank_depth": True}, TypeError),
        ({"use_graph_arm": 1}, TypeError),
        ({"alpha": True}, TypeError),
        ({"pool_n": True}, TypeError),
        ({"explain": 1}, TypeError),
        ({"limit": True}, TypeError),
    ],
)
def test_frozen_search_rejects_python_coercions_after_authentication(
    db_path: str,
    kwargs: dict[str, object],
    error: type[Exception],
) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
    with pytest.raises(error):
        engine.search_frozen("query", frozen, **kwargs)  # type: ignore[arg-type]
    engine.close()


@pytest.mark.parametrize(("depth", "limit"), [(True, 10), (0, True)])
def test_frozen_expansion_rejects_python_coercions_after_authentication(
    db_path: str,
    depth: object,
    limit: object,
) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
    with pytest.raises(TypeError):
        engine.search_expand_frozen(  # type: ignore[arg-type]
            "query", frozen, depth, limit=limit
        )
    engine.close()
