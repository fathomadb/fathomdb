"""Successful production opens expose only live runtime states."""

from typing import get_args
from types import SimpleNamespace

import pytest

from fathomdb import read
from fathomdb._sdk_search import _map_structural_explanation
from fathomdb.errors import ProjectionGenerationError
from fathomdb.types import EmbeddingReadinessState, GraphProjectionReadinessV1, StructuralDegradationCodeV1, ProjectionReadinessV1, ProjectionRuntimeStateV1, ProjectionRuntimeUnavailabilityReason


def test_runtime_reason_literals_exclude_refused_open_state() -> None:
    assert set(get_args(ProjectionRuntimeStateV1)) == {"absent", "usable"}
    assert set(get_args(ProjectionRuntimeUnavailabilityReason)) == {"none", "no_runtime"}


def test_projection_generation_decoder_rejects_retired_refused_state() -> None:
    with pytest.raises(ProjectionGenerationError) as caught:
        read._projection_enum("refused", read._PROJECTION_RUNTIME_STATES, "/runtimeState")
    assert caught.value.reason == "projection_generation_corrupt"
    assert caught.value.field_path == "/runtimeState"


def test_readiness_literals_exclude_refused_runtime_deferral() -> None:
    assert set(get_args(ProjectionReadinessV1)) == {"ready", "processing", "blocked", "degraded"}
    assert set(get_args(EmbeddingReadinessState)) == {"ready", "processing", "blocked"}


def test_readiness_decoders_reject_retired_refusal_deferral() -> None:
    with pytest.raises(ProjectionGenerationError) as caught:
        read._projection_enum("deferred", read._PROJECTION_READINESS, "/readiness")
    assert caught.value.field_path == "/readiness"
    with pytest.raises(RuntimeError, match="unknown state"):
        read._embedding_readiness_state("deferred")


def test_graph_and_structural_wire_literals_exclude_refused_runtime_modes() -> None:
    assert "deferred" not in get_args(GraphProjectionReadinessV1)
    assert "projection_deferred" not in get_args(StructuralDegradationCodeV1)


def test_structural_decoder_rejects_retired_projection_deferral() -> None:
    value = SimpleNamespace(
        schema_version=1, inclusion_state="degraded", projection_origin="current_dense_generation",
        dependency_state="not_applicable", lifecycle_state="node_active",
        degradation_codes=["projection_deferred"],
    )
    with pytest.raises(ValueError, match="/structural/degradationCodes/0"):
        _map_structural_explanation(value)
