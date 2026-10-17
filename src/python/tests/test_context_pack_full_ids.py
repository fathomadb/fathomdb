"""0.8.28 Slice 41 phase 2 — ``ContextPolicy.full_ids`` through the Python SDK.

Loads the hand-derived ``full_ids_cases`` and ``full_ids_errors`` from
``src/conformance/context-pack-fixtures.json`` (the Rust and TypeScript suites
load the same cases) and compares the text and every provenance field,
``forced`` included (AC41-28..31, AC41-33).
"""

from __future__ import annotations

import json
from dataclasses import asdict
from pathlib import Path
from typing import Any

import pytest

from fathomdb import ContextItem, ContextPack, ContextPart, ContextPolicy, pack_context
from fathomdb.errors import InvalidArgumentError

_FIXTURES = Path(__file__).resolve().parents[2] / "conformance" / "context-pack-fixtures.json"


def _fixtures() -> dict[str, Any]:
    return json.loads(_FIXTURES.read_text(encoding="utf-8"))


def _item(value: dict[str, Any]) -> ContextItem:
    return ContextItem(
        id=value["id"],
        parts=tuple(ContextPart(role=p["role"], text=p["text"]) for p in value["parts"]),
        group=value.get("group"),
        position=value.get("position"),
        order_key=value.get("order_key"),
        header=value.get("header"),
    )


def _policy(value: dict[str, Any] | None) -> ContextPolicy:
    value = dict(value or {})
    for name in ("keep_roles", "full_ids"):
        if name in value:
            value[name] = tuple(value[name])
    return ContextPolicy(**value)


def _call(request: dict[str, Any]) -> ContextPack:
    return pack_context(
        [_item(i) for i in request.get("hits", [])],
        _policy(request.get("policy")),
        [_item(i) for i in request.get("context", [])],
    )


def _as_wire(pack: ContextPack) -> dict[str, Any]:
    wire = asdict(pack)
    for entry in wire["items"]:
        entry["parts"] = list(entry["parts"])
    wire["items"] = list(wire["items"])
    return wire


@pytest.mark.parametrize("case", _fixtures()["full_ids_cases"], ids=lambda c: c["name"])
def test_every_full_ids_case_matches_through_python(case: dict[str, Any]) -> None:
    pack = _call(case["request"])
    assert _as_wire(pack) == case["expected"]


@pytest.mark.parametrize(
    "case",
    [c for c in _fixtures()["full_ids_errors"] if not c.get("json_only")],
    ids=lambda c: c["name"],
)
def test_invalid_full_ids_are_invalid_argument_naming_the_field(case: dict[str, Any]) -> None:
    with pytest.raises(InvalidArgumentError) as raised:
        _call(case["request"])
    assert case["message_contains"] in str(raised.value)
    if "message_excludes" in case:
        assert case["message_excludes"] not in str(raised.value)


def test_default_full_ids_is_empty_tuple() -> None:
    assert ContextPolicy().full_ids == ()


def test_bare_str_full_ids_is_a_type_error() -> None:
    hits = [ContextItem(id="h1", parts=(ContextPart(role="user", text="x"),))]
    with pytest.raises(TypeError):
        pack_context(hits, ContextPolicy(full_ids="h1"))  # type: ignore[arg-type]


def test_forced_is_false_without_full_ids() -> None:
    hits = [ContextItem(id=f"h{i}", parts=(ContextPart(role="user", text="x"),)) for i in range(5)]
    pack = pack_context(hits, ContextPolicy(full_items=1))
    assert [entry.forced for entry in pack.items] == [False] * 5
