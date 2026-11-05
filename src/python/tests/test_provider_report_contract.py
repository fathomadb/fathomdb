"""Successful production opens expose only live runtime states."""

from typing import get_args

import pytest

from fathomdb import read
from fathomdb.errors import ProjectionGenerationError
from fathomdb.types import EmbeddingReadinessState, ProjectionReadinessV1, ProjectionRuntimeStateV1, ProjectionRuntimeUnavailabilityReason


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
