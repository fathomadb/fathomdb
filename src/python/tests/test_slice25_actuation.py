"""0.8.25 Slice 25 atomic actuation Python contract."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

from fathomdb import ActuationBatchV1, Engine
from fathomdb.errors import ActuationError


_FIXTURE_OVERRIDE = os.environ.get("FATHOMDB_SLICE25_FIXTURE")
_FIXTURE_PATH = (
    Path(_FIXTURE_OVERRIDE)
    if _FIXTURE_OVERRIDE is not None
    else Path(__file__).parents[3] / "dev/fixtures/slice25-actuation-conformance-v1.json"
)
SHARED_FIXTURE = json.loads(_FIXTURE_PATH.read_text())


def _snake_wire(value: Any) -> Any:
    keys = {
        "schemaVersion": "schema_version",
        "operationId": "operation_id",
        "decisionPolicyId": "decision_policy_id",
        "expectedWriteBoundary": "expected_write_boundary",
        "sourceId": "source_id",
        "logicalId": "logical_id",
        "validFrom": "valid_from",
        "validUntil": "valid_until",
        "artifactRevisionId": "artifact_revision_id",
        "sourceVersionId": "source_version_id",
        "sourceRevisionId": "source_revision_id",
        "sourceLocator": "source_locator",
        "startInclusive": "start_inclusive",
        "endExclusive": "end_exclusive",
        "canonicalSourceHash": "canonical_source_hash",
        "digestHex": "digest_hex",
        "dependencyId": "dependency_id",
        "derivedRevisionId": "derived_revision_id",
        "expectedCurrentRevisionId": "expected_current_revision_id",
        "toState": "to_state",
    }
    if isinstance(value, list):
        return [_snake_wire(item) for item in value]
    if isinstance(value, dict):
        return {keys.get(key, key): _snake_wire(item) for key, item in value.items()}
    return value


def _request(operation_id: str = "python-actuation") -> ActuationBatchV1:
    return {
        "schema_version": 1,
        "operation_id": operation_id,
        "operations": [
            {
                "type": "put_canonical_node",
                "record": {
                    "kind": "doc",
                    "body": "source body",
                    "source_id": "source-a",
                    "logical_id": "source",
                    "provenance": {
                        "schema_version": 1,
                        "role": "canonical",
                        "artifact_revision_id": "source-r1",
                        "source_version_id": "version-r1",
                    },
                },
            }
        ],
    }


def test_actuation_round_trip_replay_and_decimal_boundaries(tmp_path: Path) -> None:
    db_path = str(tmp_path / "actuation.fathom")
    engine = Engine.open(db_path)
    request = _request()
    receipt = engine.actuate(request)
    assert receipt.outcome == "committed"
    assert receipt.resulting_write_boundary == "1"
    assert receipt.affected_revision_ids == ("source-r1",)
    assert engine.actuate(request) == receipt
    engine.close()


def test_actuation_preserves_embedded_nul_source_identity(tmp_path: Path) -> None:
    engine = Engine.open(str(tmp_path / "actuation-nul.fathom"))
    engine.write([{"kind": "ordinary", "body": "ordinary", "source_id": "source\0id"}])
    request = _request("python-nul-source")
    request["operations"][0]["record"]["source_id"] = "source\0id"  # type: ignore[index,typeddict-item]
    receipt = engine.actuate(request)
    assert receipt.outcome == "committed"
    assert engine.actuate(request) == receipt
    engine.close()


def test_shared_all_variant_fixture_has_exact_receipt_and_digest(tmp_path: Path) -> None:
    engine = Engine.open(str(tmp_path / "shared-actuation.fathom"))
    request = _snake_wire(SHARED_FIXTURE["request"])
    expected = SHARED_FIXTURE["expected"]
    receipt = engine.actuate(request)
    assert receipt.request_sha256 == expected["requestSha256"]
    assert receipt.outcome == expected["outcome"]
    assert list(receipt.affected_revision_ids) == expected["affectedRevisionIds"]
    assert receipt.resulting_write_boundary == expected["resultingWriteBoundary"]
    assert receipt.resulting_dependency_generation == expected["resultingDependencyGeneration"]
    assert list(receipt.pending_projection_write_cursors) == expected["pendingProjectionWriteCursors"]
    assert list(receipt.closure_operation_ids) == expected["closureOperationIds"]
    assert engine.actuate(request) == receipt
    engine.close()


def test_actuation_rejects_unknown_top_level_field_before_engine_call(tmp_path: Path) -> None:
    db_path = str(tmp_path / "actuation.fathom")
    engine = Engine.open(db_path)
    request = _request("unknown-field")
    request["z_unknown"] = True  # type: ignore[typeddict-unknown-key]
    try:
        engine.actuate(request)
    except ActuationError as error:
        assert error.reason == "unknown_field"
        assert error.field_path == "/zUnknown"
    else:
        raise AssertionError("unknown field was accepted")
    engine.close()


def test_actuation_rejects_unknown_nested_record_field(tmp_path: Path) -> None:
    db_path = str(tmp_path / "actuation.fathom")
    engine = Engine.open(db_path)
    request = _request("unknown-record-field")
    record = request["operations"][0]["record"]  # type: ignore[typeddict-item]
    record["z_unknown"] = True
    try:
        engine.actuate(request)
    except ActuationError as error:
        assert error.reason == "unknown_field"
        assert error.field_path == "/operations/0/record/zUnknown"
    else:
        raise AssertionError("unknown nested record field was accepted")
    engine.close()
