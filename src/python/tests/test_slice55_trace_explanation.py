"""Slice 55 RED public trace and structural explanation surface."""

import copy
import json
from types import SimpleNamespace
from typing import Any

import fathomdb
import pytest
from fathomdb import _fathomdb
from fathomdb import engine as engine_module


def _trace_response() -> dict[str, Any]:
    return {
        "schemaVersion": 1,
        "rootRevisionId": "source-r1",
        "direction": "to_dependents",
        "nodes": [
            {
                "schemaVersion": 1,
                "artifactRevisionId": "source-r1",
                "artifactClass": "node",
                "role": "canonical_source",
                "depth": 0,
                "lifecycle": {
                    "schemaVersion": 1,
                    "artifactClass": "node",
                    "state": "active",
                    "superseded": False,
                    "validAtEffective": True,
                },
            },
            {
                "schemaVersion": 1,
                "artifactRevisionId": "derived-r1",
                "artifactClass": "node",
                "role": "derived",
                "depth": 1,
                "lifecycle": {
                    "schemaVersion": 1,
                    "artifactClass": "node",
                    "state": "active",
                    "superseded": False,
                    "validAtEffective": True,
                },
            },
        ],
        "dependencyEdges": [
            {
                "schemaVersion": 1,
                "dependencyId": "dep-1",
                "sourceRevisionId": "source-r1",
                "derivedRevisionId": "derived-r1",
                "registeredDependencyGeneration": "1",
            }
        ],
        "checkedWorkUnits": 2,
        "complete": True,
        "readBoundary": {
            "schemaVersion": 1,
            "effectiveAtEpochS": 1,
            "observedWriteBoundary": "1",
            "dependencyGeneration": "1",
            "projectionGenerationId": "pgen1:00000000000000000000000000000000",
        },
    }


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


def test_slice55_python_request_rejects_bool_schema() -> None:
    with pytest.raises(fathomdb.DependencyTraceError) as caught:
        fathomdb.DependencyTraceRequestV1(
            root_revision_id="source-r1",
            direction="to_dependents",
            context=fathomdb.FrozenReadContextV1(
                effective_valid_at=1,
                context=fathomdb.ReadContextV1(),
                token="opaque",
            ),
            schema_version=True,
        )
    assert caught.value.reason == "unsupported_schema_version"
    assert caught.value.field_path == "/schemaVersion"


@pytest.mark.parametrize(
    ("field", "value", "reason", "path"),
    [
        ("root_revision_id", "!", "trace_root_invalid", "/rootRevisionId"),
        ("direction", "sideways", "trace_direction_invalid", "/direction"),
        ("context", None, "trace_corrupt", "/context"),
    ],
)
def test_slice55_python_request_validates_declared_fields_in_order(
    tmp_path: Any, field: str, value: Any, reason: str, path: str
) -> None:
    engine = fathomdb.Engine.open(
        str(tmp_path / f"slice55-{field}.fathom"), use_default_embedder=False
    )
    try:
        request = fathomdb.DependencyTraceRequestV1(
            root_revision_id="source-r1",
            direction="to_dependents",
            context=fathomdb.FrozenReadContextV1(
            effective_valid_at=1,
            context=fathomdb.ReadContextV1(),
            token="opaque",
            ),
        )
        object.__setattr__(request, field, value)
        with pytest.raises(fathomdb.DependencyTraceError) as caught:
            engine.trace_dependency(request)
        assert caught.value.reason == reason
        assert caught.value.field_path == path
    finally:
        engine.close()


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


@pytest.mark.parametrize(
    ("mutate", "reason", "path"),
    [
        (
            lambda value: value["nodes"][1]["lifecycle"].__setitem__(
                "schemaVersion", 2
            ),
            "unsupported_schema_version",
            "/nodes/1/lifecycle/schemaVersion",
        ),
        (
            lambda value: value["nodes"][1].__setitem__(
                "artifactRevisionId", "source-r1"
            ),
            "trace_corrupt",
            "/nodes/1/artifactRevisionId",
        ),
        (
            lambda value: value["dependencyEdges"][0].__setitem__(
                "registeredDependencyGeneration", "01"
            ),
            "trace_corrupt",
            "/dependencyEdges/0/registeredDependencyGeneration",
        ),
    ],
)
def test_slice55_python_recursively_validates_trace_responses(
    mutate: Any, reason: str, path: str
) -> None:
    value = copy.deepcopy(_trace_response())
    mutate(value)
    with pytest.raises(fathomdb.DependencyTraceError) as caught:
        engine_module._decode_dependency_trace_response(json.dumps(value))
    assert caught.value.reason == reason
    assert caught.value.field_path == path


def test_slice55_python_malformed_trace_json_never_leaks_decoder_errors() -> None:
    with pytest.raises(fathomdb.DependencyTraceError) as caught:
        engine_module._decode_dependency_trace_response("{")
    assert caught.value.reason == "trace_corrupt"
    assert caught.value.field_path == ""


def test_slice55_python_rejects_boolean_response_schema() -> None:
    value = _trace_response()
    value["schemaVersion"] = True
    with pytest.raises(fathomdb.DependencyTraceError) as caught:
        engine_module._decode_dependency_trace_response(json.dumps(value))
    assert caught.value.reason == "unsupported_schema_version"
    assert caught.value.field_path == "/schemaVersion"


def test_slice55_python_malformed_nested_frozen_context_is_frozen_error(tmp_path) -> None:
    engine = fathomdb.Engine.open(str(tmp_path / "nested-context.fathom"), use_default_embedder=False)
    try:
        context = engine.freeze_read_context(fathomdb.ReadContextV1())
        object.__setattr__(context.context, "schema_version", 2)
        request = fathomdb.DependencyTraceRequestV1(
            root_revision_id="source-r1", direction="to_dependents", context=context
        )
        with pytest.raises(fathomdb.errors.FrozenReadError) as caught:
            engine.trace_dependency(request)
        assert caught.value.reason == "unsupported_schema_version"
        assert caught.value.field_path == "/context/context/schemaVersion"
    finally:
        engine.close()


@pytest.mark.parametrize(
    ("mutate", "reason", "path"),
    [
        (
            lambda frozen: object.__setattr__(frozen, "token", 7),
            "token_malformed",
            "/context/token",
        ),
        (
            lambda frozen: object.__setattr__(frozen, "effective_valid_at", True),
            "context_invalid",
            "/context/effectiveValidAt",
        ),
        (
            lambda frozen: object.__setattr__(frozen, "context", []),
            "context_invalid",
            "/context/context",
        ),
        (
            lambda frozen: object.__setattr__(frozen.context, "view", []),
            "context_invalid",
            "/context/context/view",
        ),
        (
            lambda frozen: object.__setattr__(frozen.context, "eligibility", []),
            "context_invalid",
            "/context/context/eligibility",
        ),
        (
            lambda frozen: object.__setattr__(
                frozen.context.view, "include_superseded", 1
            ),
            "context_invalid",
            "/context/context/view/includeSuperseded",
        ),
        (
            lambda frozen: object.__setattr__(
                frozen.context.eligibility, "attributes", [["missing-value"]]
            ),
            "context_invalid",
            "/context/context/eligibility/attributes/0",
        ),
    ],
)
def test_slice55_python_validates_complete_frozen_shape(
    tmp_path, mutate: Any, reason: str, path: str
) -> None:
    engine = fathomdb.Engine.open(str(tmp_path / "full-frozen.fathom"), use_default_embedder=False)
    try:
        context = engine.freeze_read_context(fathomdb.ReadContextV1())
        mutate(context)
        request = fathomdb.DependencyTraceRequestV1(
            root_revision_id="source-r1", direction="to_dependents", context=context
        )
        with pytest.raises(fathomdb.errors.FrozenReadError) as caught:
            engine.trace_dependency(request)
        assert caught.value.reason == reason
        assert caught.value.field_path == path
    finally:
        engine.close()


def test_slice55_python_noncanonical_bound_never_leaks_native_type_error(tmp_path) -> None:
    engine = fathomdb.Engine.open(
        str(tmp_path / "slice55-bound-error.fathom"), use_default_embedder=False
    )
    try:
        context = engine.freeze_read_context(fathomdb.ReadContextV1())
        request = fathomdb.DependencyTraceRequestV1(
            root_revision_id="source-r1",
            direction="to_dependents",
            context=context,
        )
        object.__setattr__(request, "max_relations", 1.5)
        with pytest.raises(fathomdb.DependencyTraceError) as caught:
            engine.trace_dependency(request)
        assert caught.value.reason == "trace_limit_invalid"
        assert caught.value.field_path == "/maxRelations"
    finally:
        engine.close()


@pytest.mark.parametrize(
    ("field", "value", "path"),
    [
        ("id", True, "/id"),
        ("arm", "unknown", "/arm"),
        ("text_rank", -1, "/textRank"),
        ("fused_score", float("nan"), "/fusedScore"),
    ],
)
def test_slice55_python_rejects_malformed_explanation_scalars(
    field: str, value: Any, path: str
) -> None:
    native = SimpleNamespace(
        id=1,
        arm="text",
        vector_rank=None,
        text_rank=0,
        graph_rank=None,
        fused_score=1.0,
        ce_score=None,
        blended=1.0,
        importance=None,
        confidence=None,
        structural=None,
    )
    setattr(native, field, value)
    with pytest.raises(ValueError, match=f"invalid explanation response at {path}"):
        engine_module._map_per_hit_explain(native)
