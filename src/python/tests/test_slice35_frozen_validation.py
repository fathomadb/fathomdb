"""Slice 35 valid-token validation and response-version contracts."""

from __future__ import annotations

from types import SimpleNamespace
from typing import Any, cast

import pytest

import fathomdb
from fathomdb.config import EngineConfig


def test_valid_token_rejects_invalid_query_and_ranking_controls(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    frozen = engine.freeze_read_context(fathomdb.ReadContextV1())

    with pytest.raises(fathomdb.errors.WriteValidationError):
        engine.search_frozen("bad\0query", frozen)
    with pytest.raises(fathomdb.errors.InvalidArgumentError):
        engine.search_frozen("query", frozen, rerank_depth=-1)
    with pytest.raises(fathomdb.errors.InvalidArgumentError):
        engine.search_frozen("query", frozen, pool_n=-1)
    engine.close()


@pytest.mark.parametrize("outer,inner", [(2, 1), (1, 2)])
def test_python_response_reader_rejects_unknown_schema_versions(outer: int, inner: int) -> None:
    class Native:
        def freeze_read_context(self, _context: object) -> object:
            return SimpleNamespace(
                schema_version=outer,
                effective_valid_at=1,
                context=SimpleNamespace(schema_version=inner),
                token="unused",
            )

    engine = fathomdb.Engine(cast(Any, Native()), path="unused", config=EngineConfig())
    with pytest.raises(fathomdb.errors.FrozenReadError, match="unsupported_schema_version"):
        engine.freeze_read_context(fathomdb.ReadContextV1())
