"""Slice 60 FIX-2 RED: execute Python graph wire transport with UTF-8 bytes."""

from __future__ import annotations

import json
import types
from pathlib import Path
from typing import cast

import fathomdb.graph as graph
import fathomdb.types as sdk_types
from fathomdb import Engine

FIXTURE = Path(__file__).parents[3] / "dev" / "fixtures" / "slice60-fix2-unicode-v1.json"


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


def test_python_request_and_native_result_transport_are_raw_utf8_fixture_bytes() -> None:
    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))

    captured: list[str] = []

    class Native:
        def graph_expand(self, request: str) -> str:
            captured.append(request)
            return fixture["result"]

    result = graph.expand(
        cast(Engine, types.SimpleNamespace(_native=Native())), _unicode_request(sdk_types)
    )
    assert [request.encode("utf-8") for request in captured] == [fixture["request"].encode("utf-8")]
    assert "\\u00e9" not in captured[0]
    assert result.seeds[0].logical_id == "café"
