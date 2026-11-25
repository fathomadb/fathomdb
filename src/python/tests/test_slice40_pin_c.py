"""Slice 40 WP-40A2 Pin C Python SDK tests: PutLatestState, EP-07, EP-01."""

from __future__ import annotations

import os
from pathlib import Path

import pytest

import fathomdb
from fathomdb import (
    DatabaseNotFoundError,
    Engine,
    EngineConfig,
    ScopePolicy,
    VisibilityStateV1,
    read,
)
from fathomdb.errors import ActuationError, ClosingError


def _register_collection(engine: Engine, name: str, kind: str, schema_json: str) -> None:
    engine.write(
        [
            {
                "admin_schema": {
                    "name": name,
                    "kind": kind,
                    "schema_json": schema_json,
                    "retention_json": "{}",
                }
            }
        ]
    )


def test_put_latest_state_in_actuation_batch_commits_refuses_and_replays(
    db_path: str,
) -> None:
    schema = '{"type":"object","required":["step"],"properties":{"step":{"type":"integer"}}}'
    engine = Engine.open(db_path)
    try:
        _register_collection(engine, "checkpoints", "latest_state", schema)
        _register_collection(engine, "events", "append_only_log", "{}")

        # Atomic commit paired with canonical node
        batch = {
            "schema_version": 1,
            "operation_id": "py-pin-c-op-1",
            "operations": [
                {
                    "type": "put_canonical_node",
                    "record": {
                        "kind": "note",
                        "body": "python checkpointed node",
                        "source_id": "src-py-pin-c",
                        "logical_id": "py-node-1",
                        "provenance": {
                            "schema_version": 1,
                            "role": "canonical",
                            "artifact_revision_id": "py-rev-1",
                            "source_version_id": "py-ver-1",
                        },
                    },
                },
                {
                    "type": "put_latest_state",
                    "collection": "checkpoints",
                    "record_key": "run-1",
                    "payload_json": '{"step":10}',
                },
            ],
        }
        receipt = engine.actuate(batch)  # type: ignore[arg-type]
        assert receipt.outcome == "committed"
        assert receipt.affected_revision_ids == ("py-rev-1",)
        assert receipt.resulting_write_boundary is not None

        row = read.operational_state(engine, "checkpoints", "run-1")
        assert row is not None
        assert row.payload == '{"step":10}'
        assert row.schema_id == "checkpoints"

        # Exact replay returns identical receipt
        replay = engine.actuate(batch)  # type: ignore[arg-type]
        assert replay == receipt

        # Conflict on differing payload_json
        conflict = {
            "schema_version": 1,
            "operation_id": "py-pin-c-op-1",
            "operations": [
                batch["operations"][0],
                {
                    "type": "put_latest_state",
                    "collection": "checkpoints",
                    "record_key": "run-1",
                    "payload_json": '{"step":99}',
                },
            ],
        }
        with pytest.raises(ActuationError) as excinfo:
            engine.actuate(conflict)  # type: ignore[arg-type]
        assert excinfo.value.reason == "operation_id_conflict"

        # Refusal: unregistered collection -> /operations/0/collection
        r_unreg = engine.actuate(
            {
                "schema_version": 1,
                "operation_id": "py-pin-c-unreg",
                "operations": [
                    {
                        "type": "put_latest_state",
                        "collection": "missing_col",
                        "record_key": "k1",
                        "payload_json": "{}",
                    }
                ],
            }
        )
        assert r_unreg.outcome == "refused"
        assert r_unreg.reason_codes == ("write_refused",)
        assert r_unreg.refused_field_path == "/operations/0/collection"

        # Refusal: wrong kind -> /operations/0/collection
        r_kind = engine.actuate(
            {
                "schema_version": 1,
                "operation_id": "py-pin-c-kind",
                "operations": [
                    {
                        "type": "put_latest_state",
                        "collection": "events",
                        "record_key": "k1",
                        "payload_json": "{}",
                    }
                ],
            }
        )
        assert r_kind.outcome == "refused"
        assert r_kind.refused_field_path == "/operations/0/collection"

        # Refusal: schema violation -> /operations/0/payloadJson
        r_schema = engine.actuate(
            {
                "schema_version": 1,
                "operation_id": "py-pin-c-schema",
                "operations": [
                    {
                        "type": "put_latest_state",
                        "collection": "checkpoints",
                        "record_key": "k1",
                        "payload_json": '{"step":"bad"}',
                    }
                ],
            }
        )
        assert r_schema.outcome == "refused"
        assert r_schema.refused_field_path == "/operations/0/payloadJson"
    finally:
        engine.close()


def test_ep07_create_if_missing_false_and_database_not_found_error(tmp_path: Path) -> None:
    missing = tmp_path / "missing.sqlite"
    lock_file = Path(f"{missing}.lock")

    with pytest.raises(DatabaseNotFoundError) as excinfo:
        Engine.open(str(missing), create_if_missing=False)
    assert isinstance(excinfo.value, fathomdb.errors.EngineError)
    assert excinfo.value.path is not None
    assert not missing.exists()
    assert not lock_file.exists()

    # Non-bool rejected with TypeError
    with pytest.raises(TypeError):
        Engine.open(str(missing), create_if_missing="no")  # type: ignore[arg-type]

    # Zero-length file -> DatabaseNotFoundError, file untouched
    zero_file = tmp_path / "zero.sqlite"
    zero_file.write_bytes(b"")
    with pytest.raises(DatabaseNotFoundError):
        Engine.open(str(zero_file), create_if_missing=False)
    assert zero_file.stat().st_size == 0
    assert not Path(f"{zero_file}.lock").exists()

    # Existing DB opens cleanly with create_if_missing=False
    existing = tmp_path / "existing.sqlite"
    e1 = Engine.open(str(existing))
    e1.close()
    e2 = Engine.open(str(existing), create_if_missing=False)
    e2.close()


def test_ep01_read_visibility_state_tracks_writes_reopen_and_close(db_path: str) -> None:
    engine = Engine.open(db_path)
    try:
        s0 = read.visibility_state(engine)
        assert isinstance(s0, VisibilityStateV1)
        assert s0.schema_version == 1
        assert len(s0.database_id) == 32
        assert len(s0.nonce) == 64
        assert s0.canonical_write_boundary == 0
        assert s0.projection_cursor == 0
        assert read.visibility_state(engine) == s0
    finally:
        engine.close()

    with pytest.raises(ClosingError):
        read.visibility_state(engine)

    # Reopen preserves visibility state when no writes happened
    reopened = Engine.open(db_path, create_if_missing=False)
    try:
        assert read.visibility_state(reopened) == s0
        reopened.write(
            [
                {
                    "type": "node",
                    "kind": "note",
                    "body": "vis test",
                    "source_id": "src-vis",
                    "logical_id": "n1",
                }
            ]
        )
        s1 = read.visibility_state(reopened)
        assert s1.database_id == s0.database_id
        assert s1.generation > s0.generation
        assert s1.nonce != s0.nonce
        assert s1.canonical_write_boundary == 1
    finally:
        reopened.close()

    # Scoped engine can also read visibility state cleanly
    scoped = Engine.open(
        db_path,
        create_if_missing=False,
        config=EngineConfig(scope_policy=ScopePolicy.read_only(["alpha"])),
    )
    try:
        assert read.visibility_state(scoped) == s1
    finally:
        scoped.close()
    assert os.path.exists(db_path)
