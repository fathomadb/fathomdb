"""Slice 25 FIX-1 binding path-preservation regressions."""

from __future__ import annotations

from pathlib import Path

from fathomdb import Engine
from fathomdb.errors import ActuationError


def test_nested_provenance_decode_preserves_canonical_pointer(tmp_path: Path) -> None:
    engine = Engine.open(str(tmp_path / "provenance.fathom"))
    request = {
        "schema_version": 1,
        "operation_id": "nested-provenance",
        "operations": [
            {
                "type": "put_canonical_node",
                "record": {
                    "kind": "doc",
                    "body": "body",
                    "source_id": "source-a",
                    "provenance": {
                        "schema_version": 1,
                        "role": "canonical",
                        "source_version_id": "version-r1",
                    },
                },
            }
        ],
    }

    try:
        engine.actuate(request)  # type: ignore[arg-type]
    except ActuationError as error:
        assert error.reason == "nested_request_invalid"
        assert error.field_path == "/operations/0/record/provenance/artifactRevisionId"
    else:
        raise AssertionError("malformed nested provenance was accepted")


def test_nested_dependency_decode_preserves_canonical_pointer(tmp_path: Path) -> None:
    engine = Engine.open(str(tmp_path / "dependency.fathom"))
    request = {
        "schema_version": 1,
        "operation_id": "nested-dependency",
        "operations": [
            {
                "type": "register_source_dependency",
                "dependency": {
                    "schema_version": 1,
                    "dependency_id": "dep-r1",
                    "source_revision_id": "source-r1",
                },
            }
        ],
    }

    try:
        engine.actuate(request)  # type: ignore[arg-type]
    except ActuationError as error:
        assert error.reason == "nested_request_invalid"
        assert error.field_path == "/operations/0/dependency/derivedRevisionId"
    else:
        raise AssertionError("malformed nested dependency was accepted")
