"""0.8.25 Slice 30 closure-status and error parity."""

from __future__ import annotations

import hashlib

from typing import Any, cast

import pytest

from fathomdb import Engine
from fathomdb.errors import DependencyClosureError


def test_closure_lookup_is_closed_and_absence_is_not_disclosure(db_path: str) -> None:
    engine = Engine.open(db_path)
    assert (
        engine.read_dependency_closure(
            {"schema_version": 1, "closure_operation_id": "_fdb:c:" + "a" * 64}
        )
        is None
    )
    with pytest.raises(DependencyClosureError) as excinfo:
        engine.read_dependency_closure(
            cast(
                Any,
                {"schema_version": 2, "closure_operation_id": "_fdb:c:" + "a" * 64},
            )
        )
    assert excinfo.value.reason == "unsupported_schema_version"
    assert excinfo.value.field_path == "/schemaVersion"
    engine.close()


def test_committed_closure_status_survives_reopen(db_path: str) -> None:
    source_body = "closure source body"
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        engine.write(
            [
                {
                    "kind": "doc",
                    "body": source_body,
                    "source_id": "closure-source",
                    "logical_id": "source",
                    "provenance": {
                        "schema_version": 1,
                        "role": "canonical",
                        "artifact_revision_id": "source-r1",
                        "source_version_id": "source-v1",
                    },
                },
                {
                    "kind": "fact",
                    "body": "derived body",
                    "source_id": "closure-source",
                    "logical_id": "derived",
                    "provenance": {
                        "schema_version": 1,
                        "role": "derived",
                        "artifact_revision_id": "derived-r1",
                        "source_version_id": "source-v1",
                        "source_revision_id": "source-r1",
                        "source_locator": {"kind": "whole_body"},
                        "canonical_source_hash": {
                            "algorithm": "sha256",
                            "digest_hex": hashlib.sha256(source_body.encode()).hexdigest(),
                        },
                    },
                },
            ]
        )
        engine.register_source_dependency(
            {
                "schema_version": 1,
                "dependency_id": "source-derived",
                "source_revision_id": "source-r1",
                "derived_revision_id": "derived-r1",
            }
        )
        receipt = engine.actuate(
            {
                "schema_version": 1,
                "operation_id": "python-closure-status",
                "operations": [
                    {
                        "type": "transition_lifecycle",
                        "logical_id": "source",
                        "expected_current_revision_id": "source-r1",
                        "to_state": "deleted",
                    }
                ],
            }
        )
        assert receipt.outcome == "committed_closure_pending"
        assert len(receipt.closure_operation_ids) == 1
        request = {
            "schema_version": 1,
            "closure_operation_id": receipt.closure_operation_ids[0],
        }
        status = engine.read_dependency_closure(request)
        assert status is not None
        assert status.phase == "complete"
        assert status.cause == "soft_deleted"
        assert status.root == {"type": "source_revision", "source_revision_id": "source-r1"}
        assert status.proof is not None
        assert status.proof.current_active_dependent_nodes == "0"
    finally:
        engine.close()
    reopened = Engine.open(db_path, use_default_embedder=False)
    try:
        assert reopened.read_dependency_closure(request) == status
    finally:
        reopened.close()
