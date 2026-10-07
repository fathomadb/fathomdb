"""Frozen-read schema refusals retain stable fields at the installed boundary."""
from __future__ import annotations

from pathlib import Path

import fathomdb
import fathomdb._fathomdb as native
import pytest


@pytest.mark.parametrize("schema_version", [0, 2])
@pytest.mark.parametrize("boundary", ["native_constructor", "engine_freeze"])
def test_unsupported_read_context_schema_retains_reason_and_path(
    tmp_path: Path, schema_version: int, boundary: str
) -> None:
    database = str(tmp_path / "frozen-refusal.sqlite")
    engine = fathomdb.Engine.open(database, use_default_embedder=False)
    try:
        engine.write([{"kind": "doc", "body": "retained bytes", "logical_id": "retained", "source_id": "frozen-refusal"}])
        with pytest.raises(fathomdb.FrozenReadError, match="unsupported_schema_version at /schemaVersion") as refused:
            if boundary == "native_constructor":
                native.ReadContextV1(schema_version=schema_version)
            else:
                engine.freeze_read_context(fathomdb.ReadContextV1(schema_version=schema_version))
        assert (refused.value.reason, refused.value.field_path) == (
            "unsupported_schema_version", "/schemaVersion"
        )
        assert fathomdb.read.get(engine, "retained").body == "retained bytes"
        assert engine.freeze_read_context(fathomdb.ReadContextV1()).schema_version == 1
    finally:
        engine.close()
    reopened = fathomdb.Engine.open(database, use_default_embedder=False)
    try:
        assert fathomdb.read.get(reopened, "retained").body == "retained bytes"
    finally:
        reopened.close()
