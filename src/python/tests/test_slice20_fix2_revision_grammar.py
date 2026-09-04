"""Slice 20 FIX-2 persisted revision-identity grammar parity."""

from __future__ import annotations

import hashlib
import sqlite3

import pytest

from fathomdb import Engine
from fathomdb.errors import StorageError


def _seed_and_register(engine: Engine) -> None:
    body = "source bytes"
    engine.write(
        [
            {
                "kind": "doc",
                "body": body,
                "source_id": "bucket",
                "logical_id": "source",
                "provenance": {
                    "schema_version": 1,
                    "role": "canonical",
                    "artifact_revision_id": "source-r1",
                    "source_version_id": "v1",
                },
            },
            {
                "kind": "fact",
                "body": "derived",
                "source_id": "bucket",
                "logical_id": "derived",
                "provenance": {
                    "schema_version": 1,
                    "role": "derived",
                    "artifact_revision_id": "derived-r1",
                    "source_version_id": "v1",
                    "source_revision_id": "source-r1",
                    "source_locator": {"kind": "whole_body"},
                    "canonical_source_hash": {
                        "algorithm": "sha256",
                        "digest_hex": hashlib.sha256(body.encode()).hexdigest(),
                    },
                },
            },
        ]
    )
    engine.register_source_dependency(
        {
            "schema_version": 1,
            "dependency_id": "dep-1",
            "source_revision_id": "source-r1",
            "derived_revision_id": "derived-r1",
        }
    )


def test_consistent_invalid_canonical_source_revision_maps_to_storage(db_path: str) -> None:
    engine = Engine.open(db_path)
    _seed_and_register(engine)
    engine.close()
    with sqlite3.connect(db_path) as connection:
        connection.execute(
            "UPDATE _fathomdb_artifact_revisions SET revision_id='_bad-rev' "
            "WHERE revision_id='source-r1'"
        )
        connection.execute(
            "UPDATE _fathomdb_source_versions SET source_revision_id='_bad-rev' "
            "WHERE source_revision_id='source-r1'"
        )
        connection.execute(
            "UPDATE _fathomdb_source_links "
            "SET artifact_revision_id='_bad-rev', source_revision_id='_bad-rev' "
            "WHERE artifact_revision_id='source-r1'"
        )
        connection.execute(
            "UPDATE _fathomdb_source_links SET source_revision_id='_bad-rev' "
            "WHERE artifact_revision_id='derived-r1'"
        )

    reopened = Engine.open(db_path)
    try:
        with pytest.raises(StorageError):
            reopened.dependency_for_derived(
                {"schema_version": 1, "derived_revision_id": "derived-r1"}
            )
        with pytest.raises(StorageError):
            reopened.register_source_dependency(
                {
                    "schema_version": 1,
                    "dependency_id": "dep-1",
                    "source_revision_id": "source-r1",
                    "derived_revision_id": "derived-r1",
                }
            )
    finally:
        reopened.close()
