"""Slice 40 projection-generation public contract."""

from __future__ import annotations

import fathomdb
from fathomdb import read


def test_fresh_generation_status_is_typed_and_stable(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    first = read.projection_generation_status(engine)
    assert first.schema_version == 1
    assert first.generation_id.startswith("pgen1:")
    assert first.origin == "fresh"
    assert first.readiness == "ready"
    assert first.runtime_state == "absent"
    engine.close()

    reopened = fathomdb.Engine.open(db_path, use_default_embedder=False)
    assert read.projection_generation_status(reopened).generation_id == first.generation_id
    reopened.close()


def test_actuation_receipt_exposes_nullable_generation(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    receipt = engine.actuate(
        {
            "schema_version": 1,
            "operation_id": "slice40-python",
            "operations": [
                {
                    "type": "put_canonical_node",
                    "record": {
                        "kind": "doc",
                        "body": "body",
                        "logical_id": "node",
                        "source_id": "source:slice40-python",
                        "provenance": {
                            "schema_version": 1,
                            "artifact_revision_id": "slice40-python-r1",
                            "source_version_id": "slice40-python-v1",
                            "role": "canonical",
                        },
                    },
                }
            ],
        }
    )
    assert receipt.projection_generation_id is None
    engine.close()
