"""Slice 60 FIX-2 RED: execute Python graph wire transport with UTF-8 bytes."""

from __future__ import annotations

import importlib.util
import json
import sys
import types
from pathlib import Path


ROOT = Path(__file__).parents[1] / "fathomdb"
FIXTURE = Path(__file__).parents[3] / "dev" / "fixtures" / "slice60-fix2-unicode-v1.json"


def _load_graph(monkeypatch):
    package = types.ModuleType("fathomdb")
    package.__path__ = [str(ROOT)]
    monkeypatch.setitem(sys.modules, "fathomdb", package)

    types_spec = importlib.util.spec_from_file_location("fathomdb.types", ROOT / "types.py")
    assert types_spec and types_spec.loader
    sdk_types = importlib.util.module_from_spec(types_spec)
    monkeypatch.setitem(sys.modules, "fathomdb.types", sdk_types)
    types_spec.loader.exec_module(sdk_types)

    native = types.ModuleType("fathomdb._fathomdb")
    native.NodeRecord = object
    native.SearchHit = object
    native.graph_neighbors = lambda *args, **kwargs: None
    native.search_expand = lambda *args, **kwargs: None
    monkeypatch.setitem(sys.modules, "fathomdb._fathomdb", native)

    read = types.ModuleType("fathomdb.read")
    read._to_native_view = lambda value: value
    monkeypatch.setitem(sys.modules, "fathomdb.read", read)

    errors = types.ModuleType("fathomdb.errors")

    class GraphExpansionError(Exception):
        def __init__(self, message: str, *, reason: str, field_path: str) -> None:
            super().__init__(message)
            self.reason = reason
            self.field_path = field_path

    errors.GraphExpansionError = GraphExpansionError
    monkeypatch.setitem(sys.modules, "fathomdb.errors", errors)

    graph_spec = importlib.util.spec_from_file_location("fathomdb.graph", ROOT / "graph.py")
    assert graph_spec and graph_spec.loader
    graph = importlib.util.module_from_spec(graph_spec)
    monkeypatch.setitem(sys.modules, "fathomdb.graph", graph)
    graph_spec.loader.exec_module(graph)
    return graph, sdk_types


def _unicode_request(sdk_types):
    return sdk_types.GraphExpandRequestV1(
        schema_version=1,
        seed=sdk_types.GraphQuerySeedV1(
            schema_version=1, type="query", text="café", ranked_limit=1
        ),
        direction="outgoing",
        edge_kinds=(),
        target_kinds=(),
        context=sdk_types.CurrentGraphReadContextV1(
            schema_version=1,
            type="current",
            context=sdk_types.ReadContextV1(
                view=sdk_types.ReadView(valid_as_of=1_700_000_000),
                eligibility=sdk_types.SearchFilter(),
            ),
        ),
        max_depth=0,
        result_limit=1,
        max_work_units="1",
        include_explanation=False,
    )


def test_python_request_and_native_result_transport_are_raw_utf8_fixture_bytes(monkeypatch) -> None:
    graph, sdk_types = _load_graph(monkeypatch)
    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))

    captured: list[str] = []

    class Native:
        def graph_expand(self, request: str) -> str:
            captured.append(request)
            return fixture["result"]

    engine = types.ModuleType("fathomdb.engine")
    engine._map_native_graph_expand_result = lambda response: response
    monkeypatch.setitem(sys.modules, "fathomdb.engine", engine)
    result = graph.expand(types.SimpleNamespace(_native=Native()), _unicode_request(sdk_types))
    assert [request.encode("utf-8") for request in captured] == [fixture["request"].encode("utf-8")]
    assert "\\u00e9" not in captured[0]
    assert result.seeds[0].logical_id == "café"
