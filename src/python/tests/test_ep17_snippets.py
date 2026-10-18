"""EP-17 match-centred snippets through the Python SDK (T-12).

The engine owns snippet selection (``dev/design/agent-access/match-centred-snippets.md``
§ 7.2); these tests check that the Python binding forwards the options and
returns the sidecar unchanged. Offsets in ``SnippetV1`` are UTF-8 byte offsets,
as in Rust.
"""

from __future__ import annotations

import fathomdb
from fathomdb import Engine, SnippetOptions, SnippetV1

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
