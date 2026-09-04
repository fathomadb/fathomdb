"""0.8.25 Slice 20 dependency-wire parity."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

import pytest

from fathomdb import Engine, SourceDependencyRegistrationV1
from fathomdb.errors import DependencyError

FIXTURE = json.loads(
    (Path(__file__).parents[3] / "dev/fixtures/slice20-dependency-conformance-v1.json").read_text()
)


def _seed(engine: Engine) -> None:
    body = "source bytes"
    digest = hashlib.sha256(body.encode()).hexdigest()
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
                    "canonical_source_hash": {"algorithm": "sha256", "digest_hex": digest},
                },
            },
        ]
    )


def test_dependency_round_trip_and_decimal_generation(db_path: str) -> None:
    engine = Engine.open(db_path)
    _seed(engine)
    request: SourceDependencyRegistrationV1 = {
        "schema_version": 1,
        "dependency_id": "dep-1",
        "source_revision_id": "source-r1",
        "derived_revision_id": "derived-r1",
    }
    registered = engine.register_source_dependency(request)
    assert registered.registered_dependency_generation == "1"
    assert engine.register_source_dependency(request) == registered
    assert engine.dependencies_for_source(
        {"schema_version": 1, "source_revision_id": "source-r1"}
    ).items == (registered,)
    assert engine.dependency_for_derived(
        {"schema_version": 1, "derived_revision_id": "derived-r1"}
    ) == registered
    engine.close()


@pytest.mark.parametrize("case", FIXTURE["failures"])
def test_shared_fixture_validation_precedence_and_paths(
    db_path: str, case: dict[str, Any]
) -> None:
    engine = Engine.open(db_path)
    request = {
        {
            "schemaVersion": "schema_version",
            "dependencyId": "dependency_id",
            "sourceRevisionId": "source_revision_id",
            "derivedRevisionId": "derived_revision_id",
        }.get(key, key): value
        for key, value in case["request"].items()
    }
    with pytest.raises(DependencyError) as excinfo:
        engine.register_source_dependency(request)  # type: ignore[arg-type]
    assert excinfo.value.reason == case["reason"]
    assert excinfo.value.field_path == case["fieldPath"]
    engine.close()
