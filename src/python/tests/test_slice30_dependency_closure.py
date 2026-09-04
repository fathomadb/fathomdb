"""0.8.25 Slice 30 closure-status and error parity."""

from __future__ import annotations

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
            {"schema_version": 2, "closure_operation_id": "_fdb:c:" + "a" * 64}
        )  # type: ignore[arg-type]
    assert excinfo.value.reason == "unsupported_schema_version"
    assert excinfo.value.field_path == "/schemaVersion"
    engine.close()
