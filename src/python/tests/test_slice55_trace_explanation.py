"""Slice 55 RED public trace and structural explanation surface."""

import fathomdb


def test_slice55_trace_types_are_public() -> None:
    request = fathomdb.DependencyTraceRequestV1(
        root_revision_id="source-r1",
        direction="to_dependents",
        context=fathomdb.FrozenReadContextV1(
            effective_valid_at=1,
            context=fathomdb.ReadContextV1(),
            token="opaque",
        ),
    )
    assert request.schema_version == 1
    assert fathomdb.DependencyTraceError.code == "FDB_DEPENDENCY_TRACE"


def test_slice55_doctor_surface_remains_absent() -> None:
    assert not hasattr(fathomdb.Engine, "check_data_plane_integrity")
    assert not hasattr(fathomdb.Engine, "doctor")
