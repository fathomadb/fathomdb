"""0.8.25 Slice 25 atomic actuation Python contract."""

from __future__ import annotations

from pathlib import Path

from fathomdb import ActuationBatchV1, Engine
from fathomdb.errors import ActuationError


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
