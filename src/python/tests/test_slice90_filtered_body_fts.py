"""Filtered direct body FTS through the real native binding, without a provider."""

import json

import pytest

from fathomdb import Engine, ProjectionRole, ProjectionSpec, ReadView, SearchFilter
from fathomdb.errors import InvalidFilterError, WriteValidationError


def test_filter_precedes_cap_and_keeps_positional_view(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        engine.configure_projections([
            ProjectionSpec(name="group", roles=frozenset({ProjectionRole.FILTERABLE})),
        ])
        engine.write([
            {"kind": "doc", "logical_id": f"n{i}", "source_id": "slice90",
             "body": json.dumps({"text": "s90binding", "group": "other", "i": i})}
            for i in range(120)
        ] + [{"kind": "note", "logical_id": "target", "source_id": "slice90",
              "body": json.dumps({"text": "s90binding " + "padding " * 100,
                                  "group": "wanted"})}])
        view = ReadView(valid_as_of=1_700_000_000)
        assert len(engine.search_text_only("s90binding", view).results) == 10
        result = engine.search_text_only("s90binding", view, limit=1, filter=SearchFilter(
            source_type="note", kind="note", attributes=(("group", "wanted"),)))
        assert [h.id.value for h in result.results] == ["target"]
        assert result.soft_fallback is None
        empty = engine.search_text_only("s90binding", view, filter=SearchFilter())
        assert empty == engine.search_text_only("s90binding", view)
        for filter_ in (SearchFilter(status=""), SearchFilter(created_after=0)):
            assert engine.search_text_only("s90binding", filter=filter_).results == []
        with pytest.raises(InvalidFilterError):
            engine.search_text_only("absent", filter=SearchFilter(attributes=(("unknown", "x"),)))
        with pytest.raises(TypeError):
            engine.search_text_only("s90binding", filter={"kind": "note"})  # type: ignore[arg-type]
        with pytest.raises(WriteValidationError):
            engine.search_text_only("s90binding", filter=SearchFilter(kind="bad\x00kind"))
    finally:
        engine.close()
