"""0.8.25 Slice 15 Python provenance-wire parity."""

from __future__ import annotations

import pytest

from fathomdb import Engine
from fathomdb.errors import ProvenanceError


def test_python_canonical_write_keeps_receipt_shape(db_path: str) -> None:
    engine = Engine.open(db_path)
    receipt = engine.write(
        [
            {
                "kind": "doc",
                "body": "AéB",
                "source_id": "source-1",
                "logical_id": "source-logical",
                "provenance": {
                    "schema_version": 1,
                    "role": "canonical",
                    "artifact_revision_id": "source-revision-1",
                    "source_version_id": "source-v1",
                },
            }
        ]
    )
    assert set(vars(receipt)) == {
        "cursor",
        "row_cursors",
        "dangling_edge_endpoints",
    }
    engine.close()


def test_python_versioned_object_is_closed_and_typed(db_path: str) -> None:
    engine = Engine.open(db_path)
    with pytest.raises(ProvenanceError) as excinfo:
        engine.write(
            [
                {
                    "kind": "doc",
                    "body": "body",
                    "source_id": "source-1",
                    "provenance": {
                        "schema_version": 1,
                        "role": "canonical",
                        "artifact_revision_id": "revision-1",
                        "source_version_id": "source-v1",
                        "future_field": True,
                    },
                }
            ]
        )
    assert excinfo.value.reason == "unknown_field"
    assert excinfo.value.field_path == "/provenance/futureField"
    engine.close()
