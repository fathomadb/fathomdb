"""Slice 20 FIX-1 stored-identity grammar parity."""

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


def test_consistently_invalid_stored_source_version_fails_as_storage(db_path: str) -> None:
    engine = Engine.open(db_path)
    _seed_and_register(engine)
    engine.close()

    with sqlite3.connect(db_path) as connection:
        connection.execute(
            "UPDATE _fathomdb_source_versions SET source_version_id='bad version'"
        )
        connection.execute("UPDATE _fathomdb_source_links SET source_version_id='bad version'")

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
