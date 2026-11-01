"""Successful production opens expose only live runtime states."""

from typing import get_args

import pytest

from fathomdb import read
from fathomdb.errors import ProjectionGenerationError
from fathomdb.types import ProjectionRuntimeStateV1, ProjectionRuntimeUnavailabilityReason


def test_runtime_reason_literals_exclude_refused_open_state() -> None:
    assert set(get_args(ProjectionRuntimeStateV1)) == {"absent", "usable"}
    assert set(get_args(ProjectionRuntimeUnavailabilityReason)) == {"none", "no_runtime"}


def test_projection_generation_decoder_rejects_retired_refused_state() -> None:
    with pytest.raises(ProjectionGenerationError) as caught:
        read._projection_enum("refused", read._PROJECTION_RUNTIME_STATES, "/runtimeState")
    assert caught.value.reason == "projection_generation_corrupt"
    assert caught.value.field_path == "/runtimeState"
