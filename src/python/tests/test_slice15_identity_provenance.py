"""0.8.25 Slice 15 Python provenance-wire parity."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from fathomdb import Engine
from fathomdb.errors import ProvenanceError, WriteValidationError

FIXTURE = json.loads(
    (Path(__file__).parents[2] / "conformance" / "provenance-v1.json").read_text()
)
SNAKE = FIXTURE["snake"]


def test_python_consumes_shared_success_fixture_and_keeps_receipt_shape(
    db_path: str,
) -> None:
    engine = Engine.open(db_path)
    receipt = engine.write(
        [
            {
                "kind": "doc",
                "body": FIXTURE["canonicalBody"],
                "source_id": "source-1",
                "logical_id": "source-logical",
                "provenance": SNAKE["canonical"],
            },
            {
                "kind": "entity",
                "body": "derived whole body",
                "source_id": "source-1",
                "logical_id": "derived-whole",
                "provenance": SNAKE["derivedWholeBody"],
            },
            {
                "kind": "mentions",
                "from": "source-logical",
                "to": "derived-whole",
                "body": "é",
                "source_id": "source-1",
                "logical_id": "derived-bytes",
                "provenance": SNAKE["derivedUtf8Bytes"],
            },
        ]
    )
    assert set(vars(receipt)) == {
        "cursor",
        "row_cursors",
        "dangling_edge_endpoints",
    }
    assert receipt.row_cursors == (1, 2, 3)
    engine.close()


@pytest.mark.parametrize(
    "case",
    SNAKE["errors"],
    ids=[case["name"] for case in SNAKE["errors"]],
)
@pytest.mark.parametrize("wrapped", [False, True], ids=["direct", "wrapped"])
def test_python_consumes_shared_error_fixture_with_reason_path_parity(
    db_path: str, case: dict[str, Any], wrapped: bool
) -> None:
    engine = Engine.open(db_path)
    if case.get("seedCanonical"):
        engine.write(
            [
                {
                    "kind": "doc",
                    "body": FIXTURE["canonicalBody"],
                    "source_id": "source-1",
                    "logical_id": "fixture-source",
                    "provenance": SNAKE["canonical"],
                }
            ]
        )
    with pytest.raises(ProvenanceError) as excinfo:
        node = {
            "kind": "doc",
            "body": "body",
            "source_id": "source-1",
            "provenance": case["provenance"],
        }
        engine.write([{"node": node} if wrapped else node])
    assert excinfo.value.reason == case["reason"]
    assert excinfo.value.field_path == case["fieldPath"]
    engine.close()


@pytest.mark.parametrize(
    "provenance",
    [
        {"schema_version": 1, "role": "future"},
        {
            "schema_version": 1,
            "role": "future",
            "artifact_revision_id": None,
            "source_version_id": 7,
        },
    ],
)
def test_python_unknown_role_precedes_missing_or_malformed_shared_ids(
    db_path: str, provenance: dict[str, Any]
) -> None:
    engine = Engine.open(db_path)
    with pytest.raises(ProvenanceError) as excinfo:
        engine.write(
            [
                {
                    "kind": "doc",
                    "body": "body",
                    "source_id": "source-1",
                    "provenance": provenance,
                }
            ]
        )
    assert excinfo.value.reason == "role_invalid"
    assert excinfo.value.field_path == "/provenance/role"
    engine.close()


@pytest.mark.parametrize("wrapped", [False, True])
@pytest.mark.parametrize("entity", ["node", "edge"])
@pytest.mark.parametrize("bad_id", ["bad\x00revision", "bad\ud800revision"])
def test_python_provenance_encoding_errors_are_typed_for_direct_and_wrapped_entities(
    db_path: str, wrapped: bool, entity: str, bad_id: str
) -> None:
    engine = Engine.open(db_path)
    provenance = dict(SNAKE["canonical"])
    provenance["artifact_revision_id"] = bad_id
    if entity == "node":
        inner: dict[str, Any] = {
            "kind": "doc",
            "body": "body",
            "source_id": "source-1",
            "provenance": provenance,
        }
        item = {"node": inner} if wrapped else inner
    else:
        provenance["role"] = "derived"
        provenance["source_revision_id"] = "missing-source"
        provenance["source_locator"] = {"kind": "whole_body"}
        provenance["canonical_source_hash"] = SNAKE["derivedWholeBody"][
            "canonical_source_hash"
        ]
        inner = {
            "kind": "mentions",
            "from": "a",
            "to": "b",
            "body": "body",
            "source_id": "source-1",
            "provenance": provenance,
        }
        item = {"edge": inner}
        if not wrapped:
            item = inner
    with pytest.raises(ProvenanceError) as excinfo:
        engine.write([item])
    assert excinfo.value.reason == "revision_id_invalid"
    assert excinfo.value.field_path == "/provenance/artifactRevisionId"
    engine.close()


@pytest.mark.parametrize("entity", ["node", "edge"])
@pytest.mark.parametrize("wrapped", [False, True])
def test_python_versioned_body_must_be_a_string(
    db_path: str, wrapped: bool, entity: str
) -> None:
    engine = Engine.open(db_path)
    if entity == "node":
        inner: dict[str, Any] = {
            "kind": "doc",
            "body": {"not": "a string"},
            "source_id": "source-1",
            "provenance": SNAKE["canonical"],
        }
        item = {"node": inner} if wrapped else inner
    else:
        inner = {
            "kind": "mentions",
            "from": "a",
            "to": "b",
            "body": {"not": "a string"},
            "source_id": "source-1",
            "provenance": SNAKE["derivedWholeBody"],
        }
        item = {"edge": inner}
    with pytest.raises(WriteValidationError):
        engine.write([item])
    engine.close()


def test_python_canonical_role_is_illegal_for_edges(db_path: str) -> None:
    engine = Engine.open(db_path)
    with pytest.raises(ProvenanceError) as excinfo:
        engine.write(
            [
                {
                    "edge": {
                        "kind": "mentions",
                        "from": "a",
                        "to": "b",
                        "body": "body",
                        "source_id": "source-1",
                        "provenance": SNAKE["canonical"],
                    }
                }
            ]
        )
    assert excinfo.value.reason == "role_invalid"
    assert excinfo.value.field_path == "/provenance/role"
    engine.close()
