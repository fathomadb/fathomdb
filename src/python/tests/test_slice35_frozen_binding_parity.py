"""Slice 35 frozen methods retain the ordinary Python argument contract."""

from __future__ import annotations

import pytest

import fathomdb


def _malformed_context(engine: fathomdb.Engine) -> fathomdb.FrozenReadContextV1:
    frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
    return fathomdb.FrozenReadContextV1(
        schema_version=frozen.schema_version,
        effective_valid_at=frozen.effective_valid_at,
        context=frozen.context,
        token=frozen.token + "0",
    )


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


@pytest.mark.parametrize(
    "kwargs",
    [
        {"rerank_depth": 1.5},
        {"rerank_depth": 1 << 100},
        {"rerank_depth": "one"},
        {"alpha": "high"},
        {"pool_n": 1.5},
        {"pool_n": 1 << 100},
        {"limit": 1.5},
        {"limit": 1 << 100},
    ],
)
def test_frozen_search_authenticates_before_native_argument_conversion(
    db_path: str,
    kwargs: dict[str, object],
) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    malformed = _malformed_context(engine)

    with pytest.raises(fathomdb.errors.FrozenReadError, match="token_malformed"):
        engine.search_frozen("query", malformed, **kwargs)  # type: ignore[arg-type]
    engine.close()


@pytest.mark.parametrize(
    ("depth", "limit"),
    [(1.5, 10), (1 << 100, 10), ("one", 10), (0, 1.5), (0, 1 << 100)],
)
def test_frozen_expansion_authenticates_before_native_argument_conversion(
    db_path: str,
    depth: object,
    limit: object,
) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    malformed = _malformed_context(engine)

    with pytest.raises(fathomdb.errors.FrozenReadError, match="token_malformed"):
        engine.search_expand_frozen(  # type: ignore[arg-type]
            "query", malformed, depth, limit=limit
        )
    engine.close()
