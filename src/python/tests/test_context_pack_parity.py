"""0.8.28 Slice 41 — context pack through the Python SDK (AC41-10/11/20/21).

Loads the shared hand-derived fixture ``src/conformance/context-pack-fixtures.json``
that the Rust (``fathomdb-engine/tests/slice41_context_pack.rs``,
``fathomdb-sdk/tests/slice41_context_pack.rs``) and TypeScript
(``src/ts/tests/context-pack.test.ts``) suites load, and compares the rendered
text and every provenance field. Errors are ``InvalidArgumentError`` whose
message names the field.
"""

from __future__ import annotations

import json
import time
from dataclasses import asdict, fields
from pathlib import Path
from typing import Any

import pytest

import fathomdb
from fathomdb import (
    ContextItem,
    ContextPack,
    ContextPart,
    ContextPolicy,
    Engine,
    context_item_from_hit,
    decode_round_body,
    encode_round_body,
    pack_context,
)
from fathomdb.errors import InvalidArgumentError

_FIXTURES = Path(__file__).resolve().parents[2] / "conformance" / "context-pack-fixtures.json"


def _fixtures() -> dict[str, Any]:
    return json.loads(_FIXTURES.read_text(encoding="utf-8"))


def _part(value: dict[str, Any]) -> ContextPart:
    return ContextPart(role=value["role"], text=value["text"])


def _item(value: dict[str, Any]) -> ContextItem:
    return ContextItem(
        id=value["id"],
        parts=tuple(_part(p) for p in value["parts"]),
        group=value.get("group"),
        position=value.get("position"),
        order_key=value.get("order_key"),
        header=value.get("header"),
    )


def _policy(value: dict[str, Any] | None) -> ContextPolicy:
    value = dict(value or {})
    if "keep_roles" in value:
        value["keep_roles"] = tuple(value["keep_roles"])
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


@pytest.mark.parametrize("case", _fixtures()["pack_cases"], ids=lambda c: c["name"])
def test_fixture_case_matches_through_python(case: dict[str, Any]) -> None:
    pack = _call(case["request"])
    assert _as_wire(pack) == case["expected"]
    assert pack.text_chars == len(pack.text)


def test_scalar_counts_differ_from_utf16_length() -> None:
    case = next(
        c for c in _fixtures()["pack_cases"] if c["name"] == "scalar_values_on_non_ascii_text"
    )
    pack = _call(case["request"])
    assert len(pack.text) == 17
    assert len(pack.text.encode("utf-16-le")) // 2 == case["utf16_length"]


def test_default_policy_matches_documented_defaults() -> None:
    defaults = _fixtures()["defaults"]
    policy = ContextPolicy()
    got = {f.name: getattr(policy, f.name) for f in fields(policy)}
    got["keep_roles"] = list(got["keep_roles"])
    assert got == defaults


@pytest.mark.parametrize(
    "case",
    [c for c in _fixtures()["pack_errors"] if not c.get("json_only")],
    ids=lambda c: c["name"],
)
def test_invalid_input_is_invalid_argument_naming_the_field(case: dict[str, Any]) -> None:
    with pytest.raises(InvalidArgumentError) as raised:
        _call(case["request"])
    assert case["message_contains"] in str(raised.value)


def test_lone_surrogate_is_invalid_argument() -> None:
    with pytest.raises(InvalidArgumentError):
        pack_context([ContextItem(id="a", parts=(ContextPart(role="user", text="\ud800"),))])
    with pytest.raises(InvalidArgumentError):
        decode_round_body("\ud800")


def test_wrong_argument_types_are_type_errors() -> None:
    with pytest.raises(TypeError):
        pack_context([{"id": "a", "parts": []}])  # type: ignore[list-item]
    with pytest.raises(TypeError):
        pack_context([], policy={"budget_chars": 5})  # type: ignore[arg-type]


@pytest.mark.parametrize("case", _fixtures()["round_bodies"], ids=lambda c: c["name"])
def test_round_body_fixture(case: dict[str, Any]) -> None:
    parts = tuple(_part(p) for p in case["parts"])
    body = encode_round_body(parts)
    assert body == case["body"]
    assert decode_round_body(body) == parts


@pytest.mark.parametrize("case", _fixtures()["untagged_bodies"], ids=lambda c: c["name"])
def test_untagged_body_decodes_to_one_role_none_part(case: dict[str, Any]) -> None:
    assert decode_round_body(case["body"]) == tuple(_part(p) for p in case["parts"])


@pytest.mark.parametrize("case", _fixtures()["encode_errors"], ids=lambda c: c["name"])
def test_encode_errors(case: dict[str, Any]) -> None:
    with pytest.raises(InvalidArgumentError) as raised:
        encode_round_body([_part(p) for p in case["parts"]])
    assert case["message_contains"] in str(raised.value)


@pytest.mark.parametrize("case", _fixtures()["decode_errors"], ids=lambda c: c["name"])
def test_decode_errors(case: dict[str, Any]) -> None:
    with pytest.raises(InvalidArgumentError) as raised:
        decode_round_body(case["body"])
    assert case["message_contains"] in str(raised.value)


def test_operations_are_package_level_exports() -> None:
    for name in ("pack_context", "encode_round_body", "decode_round_body", "context_item_from_hit"):
        assert name in fathomdb.__all__
        assert callable(getattr(fathomdb, name))


def _search(engine: Engine, query: str) -> list[Any]:
    deadline = time.monotonic() + 10.0
    while time.monotonic() < deadline:
        hits = list(engine.search_text_only(query).results)
        if hits:
            return hits
        time.sleep(0.02)
    return []


def test_context_item_from_hit_decodes_tagged_and_keeps_untagged(db_path: str) -> None:
    tagged = encode_round_body(
        [
            ContextPart(role="user", text="my cat is named tom"),
            ContextPart(role="assistant", text="tom is a fine name"),
        ]
    )
    engine = Engine.open(db_path)
    try:
        engine.write(
            [
                {
                    "kind": "round",
                    "body": tagged,
                    "logical_id": "round:1",
                    "source_id": "session-7",
                },
                {
                    "kind": "round",
                    "body": "plain kettle note",
                    "logical_id": "round:2",
                    "source_id": "session-7",
                },
                {
                    "kind": "round",
                    "body": "\u001eXbroken kazoo",
                    "logical_id": "round:3",
                    "source_id": "session-7",
                },
            ]
        )
        hits = _search(engine, "cat")
        assert len(hits) == 1
        item = context_item_from_hit(
            hits[0],
            group="session-7",
            position=1,
            order_key="2023/01/02",
            header="[Session date: 2023/01/02]",
        )
        assert item.id == "l:round:1"
        assert item.parts == (
            ContextPart(role="user", text="my cat is named tom"),
            ContextPart(role="assistant", text="tom is a fine name"),
        )
        policy = ContextPolicy(full_items=0, compact_chars=3)
        assert (
            pack_context([item], policy).text
            == "[Session date: 2023/01/02]\nmy cat is named tom\ntom"
        )

        plain = context_item_from_hit(_search(engine, "kettle")[0])
        assert plain.parts == (ContextPart(role=None, text="plain kettle note"),)
        assert pack_context([plain], policy).text == "plain kettle note"

        with pytest.raises(InvalidArgumentError):
            context_item_from_hit(_search(engine, "kazoo")[0])
    finally:
        engine.close()
