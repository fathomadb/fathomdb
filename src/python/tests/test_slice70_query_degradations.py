"""Per-call absence through the installed production Python binding."""

from pathlib import Path

import pytest

from fathomdb import Engine, ReadContextV1, graph

QUERY = "sliceseventyneedle"


@pytest.mark.parametrize("route", ["search", "frozen", "expand", "frozen_expand"])
def test_none_provider_reports_per_call_absence(tmp_path: Path, route: str) -> None:
    """Each requested hybrid route preserves the missing vector-arm reason."""
    engine = Engine.open(str(tmp_path / f"{route}.sqlite"), use_default_embedder=False)
    try:
        engine.write(
            [
                {
                    "kind": "note",
                    "body": f"{QUERY} note",
                    "logical_id": "s70-note",
                    "source_id": "s70-python",
                }
            ]
        )
        text = engine.search_text_only(QUERY)
        assert text.degradations is None
        assert [hit.id.value for hit in text.results] == ["s70-note"]
        frozen = engine.freeze_read_context(ReadContextV1())
        if route == "search":
            result = engine.search(QUERY)
            hits = result.results
        elif route == "frozen":
            result = engine.search_frozen(QUERY, frozen)
            hits = result.results
        elif route == "expand":
            expanded = graph.search_expand(engine, QUERY, 1)
            hits = expanded.search_hits
            result = expanded
        else:
            expanded = engine.search_expand_frozen(QUERY, frozen, 1)
            hits = expanded.search_hits
            result = expanded
        words = (
            None
            if result.degradations is None
            else [(entry.arm, str(entry.reason)) for entry in result.degradations]
        )
        assert words == [("vector", "embedder_absent")]
        assert [hit.id.value for hit in hits] == ["s70-note"]
        assert all(hit.branch == "text" for hit in hits)
        assert hits[0].score == 3.0 / 31.0
    finally:
        engine.close()
