"""0.8.28 Slice 41 phase 2 unit Q — configurable query stop words, Python SDK.

AC41-36 (open-time refusals), AC41-38 (the hash on the open report and the
explain trace, and ``DEFAULT_STOP_WORDS_SHA256``) and AC41-40 (the shared
small-corpus case). Loads ``src/conformance/query-stop-words-fixtures.json``,
which the Rust and TypeScript suites also load. Real stores, no mocking.
"""

from __future__ import annotations

import json
import struct
from pathlib import Path
from typing import Any

import pytest

import fathomdb
from fathomdb import DEFAULT_STOP_WORDS_SHA256, Engine, EngineConfig, QueryStopWords
from fathomdb.errors import InvalidArgumentError

_FIXTURES = Path(__file__).resolve().parents[2] / "conformance" / "query-stop-words-fixtures.json"
_QUERY = "how long had the dentist suggest flossing"


def _fixtures() -> dict[str, Any]:
    return json.loads(_FIXTURES.read_text(encoding="utf-8"))


def _setting(value: dict[str, Any]) -> QueryStopWords:
    return QueryStopWords(
        base=value.get("base", "fathomdb"),
        extra=tuple(value.get("extra", ())),
        keep=tuple(value.get("keep", ())),
    )


def _bits(score: float) -> int:
    return int(struct.unpack("<Q", struct.pack("<d", score))[0])


def _write_corpus(engine: Engine) -> None:
    engine.write(
        [
            {
                "kind": "doc",
                "body": doc["body"],
                "logical_id": doc["logical_id"],
                "source_id": "slice41q-fixture",
            }
            for doc in _fixtures()["corpus"]
        ]
    )
    engine.drain(timeout_s=30)


def test_default_hash_constant_matches_the_fixture() -> None:
    assert DEFAULT_STOP_WORDS_SHA256 == _fixtures()["default_sha256"]
    assert "DEFAULT_STOP_WORDS_SHA256" in fathomdb.__all__
    assert "QueryStopWords" in fathomdb.__all__


def test_shared_fixture_ranks_as_derived_and_as_the_harness_transform(db_path: str) -> None:
    fixtures = _fixtures()
    default = Engine.open(db_path)
    try:
        _write_corpus(default)
        assert default.open_report().stop_words_sha256 == fixtures["default_sha256"]
        transforms = []
        for query in fixtures["queries"]:
            hits = default.search_text_only(query["query"]).results
            assert [hit.id.value for hit in hits] == query["default_ids"]
            transforms.append(default.search_text_only(query["stripped_query"]).results)
    finally:
        default.close()

    setting = _setting(fixtures["setting"])
    configured = Engine.open(db_path, query_stop_words=setting)
    try:
        assert configured.config.query_stop_words == setting
        assert configured.open_report().stop_words_sha256 == fixtures["setting_sha256"]
        for query, transform in zip(fixtures["queries"], transforms):
            hits = configured.search_text_only(query["query"]).results
            assert [hit.id.value for hit in hits] == query["setting_ids"]
            assert [hit.id.value for hit in transform] == query["setting_ids"]
            assert [_bits(h.score) for h in hits] == [_bits(h.score) for h in transform]
    finally:
        configured.close()


@pytest.mark.parametrize(
    ("setting", "expected"),
    [
        (None, "default_sha256"),
        (QueryStopWords(), "default_sha256"),
        ("fixture", "setting_sha256"),
    ],
)
def test_trace_and_open_report_record_the_hash(
    db_path: str, setting: QueryStopWords | str | None, expected: str
) -> None:
    fixtures = _fixtures()
    if setting == "fixture":
        setting = _setting(fixtures["setting"])
    config = EngineConfig(query_stop_words=setting)  # type: ignore[arg-type]
    engine = Engine.open(db_path, config=config)
    try:
        _write_corpus(engine)
        assert engine.open_report().stop_words_sha256 == fixtures[expected]
        result = engine.search(_QUERY, explain=True)
        assert result.explanation is not None
        assert result.explanation.trace.stop_words_sha256 == fixtures[expected]
        frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
        frozen_result = engine.search_frozen(_QUERY, frozen, explain=True)
        assert frozen_result.explanation is not None
        assert frozen_result.explanation.trace.stop_words_sha256 == fixtures[expected]
    finally:
        engine.close()


@pytest.mark.parametrize("case", _fixtures()["invalid_settings"], ids=lambda c: c["name"])
def test_invalid_settings_are_invalid_argument_at_open(db_path: str, case: dict[str, Any]) -> None:
    with pytest.raises(InvalidArgumentError) as raised:
        Engine.open(db_path, query_stop_words=_setting(case["setting"]))
    assert case["message_contains"] in str(raised.value)
    assert not Path(db_path).exists()


def test_wrong_setting_types_are_refused(db_path: str) -> None:
    with pytest.raises(TypeError):
        Engine.open(db_path, query_stop_words=QueryStopWords(extra="had"))  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        Engine.open(db_path, query_stop_words={"extra": ["had"]})  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        Engine.open(db_path, query_stop_words=QueryStopWords(base="engine"))  # type: ignore[arg-type]
