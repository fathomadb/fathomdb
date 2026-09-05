"""Slice 40 projection-generation public contract."""

from __future__ import annotations

import fathomdb
from fathomdb import read
from fathomdb.types import ProjectionRole, ProjectionSpec


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


def test_receipt_keyed_mutation_status_round_trips_through_python(db_path: str) -> None:
    configuring = fathomdb.Engine.open(db_path, use_default_embedder=True)
    configuring.actuate(
        {
            "schema_version": 1,
            "operation_id": "slice40-python-status-seed",
            "operations": [
                {
                    "type": "put_canonical_node",
                    "record": {
                        "kind": "doc",
                        "body": "seed body",
                        "logical_id": "seed-node",
                        "source_id": "source:slice40-python-status-seed",
                        "provenance": {
                            "schema_version": 1,
                            "artifact_revision_id": "slice40-python-status-seed-r1",
                            "source_version_id": "slice40-python-status-seed-v1",
                            "role": "canonical",
                        },
                    },
                }
            ],
        }
    )
    configuring.configure_projections(
        [
            ProjectionSpec(
                name="memory",
                roles=frozenset({ProjectionRole.SEARCHABLE}),
                vector=True,
            )
        ]
    )
    configuring.drain(timeout_s=10)
    configuring.close()

    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    receipt = engine.actuate(
        {
            "schema_version": 1,
            "operation_id": "slice40-python-mutation-status",
            "operations": [
                {
                    "type": "put_canonical_node",
                    "record": {
                        "kind": "doc",
                        "body": "body",
                        "logical_id": "node",
                        "source_id": "source:slice40-python-status",
                        "provenance": {
                            "schema_version": 1,
                            "artifact_revision_id": "slice40-python-status-r1",
                            "source_version_id": "slice40-python-status-v1",
                            "role": "canonical",
                        },
                    },
                }
            ],
        }
    )
    assert receipt.projection_generation_id is not None
    assert len(receipt.pending_projection_write_cursors) == 1
    status = read.mutation_projection_status(
        engine,
        {
            "schemaVersion": 1,
            "operationId": receipt.operation_id,
            "writeCursor": receipt.pending_projection_write_cursors[0],
            "expectedGenerationId": receipt.projection_generation_id,
        },
    )
    assert status.operation_id == receipt.operation_id
    assert status.write_cursor == receipt.pending_projection_write_cursors[0]
    assert status.generation_id == receipt.projection_generation_id
    assert status.readiness == "blocked"
    assert status.runtime_state == "absent"
    assert status.pending_count == "1"
    engine.close()
