"""0.8.25 Slice 15 Python provenance-wire parity."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from fathomdb import Engine
from fathomdb.errors import ProvenanceError

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
    assert receipt.row_cursors == [1, 2, 3]
    engine.close()


@pytest.mark.parametrize(
    "case",
    SNAKE["errors"],
    ids=[case["name"] for case in SNAKE["errors"]],
)
def test_python_consumes_shared_error_fixture_with_reason_path_parity(
    db_path: str, case: dict[str, Any]
) -> None:
    engine = Engine.open(db_path)
    with pytest.raises(ProvenanceError) as excinfo:
        engine.write(
            [
                {
                    "kind": "doc",
                    "body": "body",
                    "source_id": "source-1",
                    "provenance": case["provenance"],
                }
            ]
        )
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
