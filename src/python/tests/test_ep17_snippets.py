"""EP-17 match-centred snippets through the Python SDK (T-12).

The engine owns snippet selection (``dev/design/agent-access/match-centred-snippets.md``
§ 7.2); these tests check that the Python binding forwards the options and
returns the sidecar unchanged. Offsets in ``SnippetV1`` are UTF-8 byte offsets,
as in Rust.
"""

from __future__ import annotations

import json
from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

import fathomdb
from fathomdb import Engine, SnippetOptions, SnippetV1
from fathomdb.errors import InvalidArgumentError

TERM = "zephyrine"
SOURCE = "py-test:ep17"
# The only query term sits at byte 2990 of a ~4 KiB ASCII record, far past
# any record-start prefix.
FAR_BODY = "alpha beta gamma delta " * 130 + TERM + " " + "omega psi chi " * 80
SHORT_BODY = f"{TERM} short note"


def _writes() -> list[dict[str, str]]:
    return [
        {"kind": "doc", "body": FAR_BODY, "logical_id": "far", "source_id": SOURCE},
        {"kind": "doc", "body": SHORT_BODY, "logical_id": "short", "source_id": SOURCE},
    ]


def _by_id(result: fathomdb.SearchResult) -> dict[str, SnippetV1]:
    assert result.snippets is not None
    assert len(result.snippets) == len(result.results)
    return {hit.id.value: snip for hit, snip in zip(result.results, result.snippets)}


def _assert_far_and_short(result: fathomdb.SearchResult, max_bytes: int) -> None:
    snippets = _by_id(result)
    assert set(snippets) == {"far", "short"}

    far = snippets["far"]
    assert isinstance(far, SnippetV1)
    assert far.anchor == "match"
    assert TERM in far.text
    assert len(far.text.encode("utf-8")) <= max_bytes
    assert far.cut is True
    assert far.spans, "a matched long body carries its body-relative spans"
    start, end = far.spans[0]
    assert FAR_BODY.encode("utf-8")[start:end].decode("utf-8") == TERM
    assert all(isinstance(w, tuple) and len(w) == 2 for w in far.windows)
    assert far.highlights == []

    short = snippets["short"]
    assert short.anchor == "whole"
    assert short.text == SHORT_BODY
    assert short.cut is False
    assert short.windows == [(0, len(SHORT_BODY.encode("utf-8")))]


def test_search_text_only_returns_snippet_sidecar(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        engine.write(_writes())
        with_snippets = engine.search_text_only(TERM, snippet=SnippetOptions(max_bytes=120))
        _assert_far_and_short(with_snippets, 120)

        plain = engine.search_text_only(TERM)
        assert plain.snippets is None
        assert plain.results == with_snippets.results
    finally:
        engine.close()


def test_search_returns_snippet_sidecar(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        engine.write(_writes())
        with_snippets = engine.search(TERM, snippet=SnippetOptions(max_bytes=120))
        _assert_far_and_short(with_snippets, 120)

        plain = engine.search(TERM)
        assert plain.snippets is None
        assert plain.results == with_snippets.results
    finally:
        engine.close()


def test_search_frozen_returns_snippet_sidecar(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        engine.write(_writes())
        frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
        with_snippets = engine.search_frozen(TERM, frozen, snippet=SnippetOptions(max_bytes=120))
        _assert_far_and_short(with_snippets, 120)

        # Snippet options are presentation: the same frozen context serves
        # both calls and the results do not change.
        plain = engine.search_frozen(TERM, frozen)
        assert plain.snippets is None
        assert plain.results == with_snippets.results
    finally:
        engine.close()


Search = Callable[[Engine, object], fathomdb.SearchResult]

SURFACES: dict[str, Search] = {
    "search": lambda engine, options: engine.search(TERM, snippet=options),  # type: ignore[arg-type]
    "search_text_only": lambda engine, options: engine.search_text_only(
        TERM,
        snippet=options,  # type: ignore[arg-type]
    ),
    "search_frozen": lambda engine, options: engine.search_frozen(
        TERM,
        engine.freeze_read_context(fathomdb.ReadContextV1()),
        snippet=options,  # type: ignore[arg-type]
    ),
}

# Each value is out of its documented range (match-centred-snippets.md
# § 7.2), including values past the native integer types.
OUT_OF_RANGE = [
    SnippetOptions(max_bytes=39),
    SnippetOptions(max_bytes=4001),
    SnippetOptions(max_bytes=-1),
    SnippetOptions(max_bytes=2**40),
    SnippetOptions(max_windows=0),
    SnippetOptions(max_windows=4),
    SnippetOptions(max_windows=2**40),
    SnippetOptions(max_bytes=100, head_bytes=51),
    SnippetOptions(head_bytes=-1),
    SnippetOptions(max_point_queries=256),
    SnippetOptions(max_point_queries=-1),
]

IN_RANGE = [
    SnippetOptions(max_bytes=40, max_windows=3, head_bytes=20, max_point_queries=0),
    SnippetOptions(max_bytes=4000, highlights=True, max_point_queries=255),
]


@pytest.mark.parametrize("surface", sorted(SURFACES))
def test_snippet_options_are_validated_on_every_search_surface(db_path: str, surface: str) -> None:
    run = SURFACES[surface]
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        engine.write(_writes())
        for options in IN_RANGE:
            result = run(engine, options)
            assert result.snippets is not None
            assert len(result.snippets) == len(result.results) == 2
        for options in OUT_OF_RANGE:
            with pytest.raises(InvalidArgumentError):
                run(engine, options)
        with pytest.raises(TypeError):
            run(engine, {"max_bytes": 300})
    finally:
        engine.close()


@pytest.mark.parametrize(
    "fields",
    [
        {"max_bytes": True},
        {"max_bytes": 300.0},
        {"max_windows": "1"},
        {"head_bytes": None},
        {"max_point_queries": False},
        {"highlights": 1},
    ],
)
def test_snippet_options_reject_wrong_field_types(fields: dict[str, object]) -> None:
    with pytest.raises(TypeError):
        SnippetOptions(**fields)  # type: ignore[arg-type]


# Shared with the TypeScript test and the fathomdb-py Rust fixture test.
PARITY_FIXTURE = (
    Path(__file__).resolve().parents[3] / "src" / "conformance" / "snippet-for-parity.json"
)
PARITY_CASES: list[dict[str, Any]] = json.loads(PARITY_FIXTURE.read_text(encoding="utf-8"))["cases"]


def _pairs(values: list[list[int]]) -> list[tuple[int, int]]:
    return [(a, b) for a, b in values]


@pytest.mark.parametrize("case", PARITY_CASES, ids=[c["name"] for c in PARITY_CASES])
def test_snippet_for_matches_the_shared_parity_fixture(case: dict[str, Any]) -> None:
    expected = case["expected"]
    got = fathomdb.snippet_for(
        case["body"], _pairs(case["spans"]), SnippetOptions(**case["options"])
    )
    assert got == SnippetV1(
        text=expected["text"],
        anchor=expected["anchor"],
        windows=_pairs(expected["windows"]),
        spans=_pairs(expected["spans"]),
        highlights=_pairs(expected["highlights"]),
        cut=expected["cut"],
    )


def test_snippet_for_defaults_and_argument_types() -> None:
    assert fathomdb.snippet_for("short body", []) == SnippetV1(
        text="short body", anchor="whole", windows=[(0, 10)], cut=False
    )
    with pytest.raises(TypeError):
        fathomdb.snippet_for(b"short body", [])  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        fathomdb.snippet_for("short body", [(0,)])  # type: ignore[list-item]
    with pytest.raises(TypeError):
        fathomdb.snippet_for("short body", [(0, True)])
    with pytest.raises(TypeError):
        fathomdb.snippet_for("short body", [], {"max_bytes": 40})  # type: ignore[arg-type]


# Python ints are unbounded; values past i64 must keep the contract:
# search refuses them, snippet_for drops or clamps them.
BEYOND_I64 = [2**63, 2**64, -(2**63) - 1, -(2**64)]


@pytest.mark.parametrize("surface", sorted(SURFACES))
def test_snippet_options_beyond_i64_are_invalid_argument(db_path: str, surface: str) -> None:
    run = SURFACES[surface]
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        engine.write(_writes())
        for value in BEYOND_I64:
            for field_name in ("max_bytes", "max_windows", "head_bytes", "max_point_queries"):
                options = SnippetOptions(**{field_name: value})  # type: ignore[arg-type]
                with pytest.raises(InvalidArgumentError, match="out of range"):
                    run(engine, options)
        # A value that fits i64 but not its field type names the range
        # problem, not the native integer width.
        with pytest.raises(InvalidArgumentError, match="out of range") as raised:
            run(engine, SnippetOptions(max_point_queries=256))
        assert "4294967295" not in str(raised.value)
    finally:
        engine.close()


def test_snippet_for_never_raises_for_ints_beyond_i64() -> None:
    whole = SnippetV1(text="short body", anchor="whole", windows=[(0, 10)], cut=False)
    for value in BEYOND_I64:
        assert fathomdb.snippet_for("short body", [(0, value)]) == whole
        assert fathomdb.snippet_for("short body", [(value, 3)]) == whole

    body = "word " * 1000
    clamped_high = fathomdb.snippet_for(body, [], SnippetOptions(max_bytes=4000))
    clamped_low = fathomdb.snippet_for(body, [], SnippetOptions(max_bytes=40))
    for value in (2**63, 2**64):
        options = SnippetOptions(
            max_bytes=value, max_windows=value, head_bytes=value, max_point_queries=value
        )
        assert fathomdb.snippet_for(body, [], options).text == clamped_high.text
    for value in (-(2**63) - 1, -(2**64)):
        assert fathomdb.snippet_for(body, [], SnippetOptions(max_bytes=value)) == clamped_low
