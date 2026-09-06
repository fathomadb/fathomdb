"""Slice 55 RED public trace and structural explanation surface."""

import fathomdb
import pytest
from fathomdb import _fathomdb


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


def test_slice55_dependency_trace_error_is_the_native_exception() -> None:
    assert fathomdb.DependencyTraceError is _fathomdb.DependencyTraceError
    assert fathomdb.DependencyTraceError.code == "FDB_DEPENDENCY_TRACE"


def test_slice55_python_request_rejects_schema_before_semantics() -> None:
    with pytest.raises(fathomdb.DependencyTraceError) as caught:
        fathomdb.DependencyTraceRequestV1(
            root_revision_id="!",
            direction="not-a-direction",  # type: ignore[arg-type]
            context=fathomdb.FrozenReadContextV1(
                effective_valid_at=1,
                context=fathomdb.ReadContextV1(),
                token="opaque",
            ),
            schema_version=2,
        )
    assert caught.value.reason == "unsupported_schema_version"
    assert caught.value.field_path == "/schemaVersion"


def test_slice55_python_catches_actual_native_trace_refusal(tmp_path) -> None:
    engine = fathomdb.Engine.open(
        str(tmp_path / "slice55-native-error.fathom"), use_default_embedder=False
    )
    try:
        context = engine.freeze_read_context(fathomdb.ReadContextV1())
        request = fathomdb.DependencyTraceRequestV1(
            root_revision_id="source-r1",
            direction="invalid",  # type: ignore[arg-type]
            context=context,
        )
        with pytest.raises(fathomdb.DependencyTraceError) as caught:
            engine.trace_dependency(request)
        assert caught.value.reason == "trace_direction_invalid"
        assert caught.value.field_path == "/direction"
    finally:
        engine.close()
