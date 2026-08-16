"""Tests for the non-executable LOCOMO-01 Phase-A grid contract."""

from __future__ import annotations

import copy
import json
from pathlib import Path

import pytest

from experiments import locomo_phase_a


CONFIG_PATH = Path("experiments/configs/locomo-01/phase-a-grid.v1.json")


def _document() -> dict[str, object]:
    return json.loads(CONFIG_PATH.read_text(encoding="utf-8"))


def test_phase_a_grid_freezes_a0_provenance_and_measurement_preregistration():
    config = locomo_phase_a.resolve_config(_document())

    assert config["program_track"] == "LOCOMO-01"
    assert config["execution"] == {"mode": "plan_only", "live_execution": "forbidden"}
    assert config["baseline"]["canonical_id"] == "A0"
    assert config["baseline"]["record_run_id"] == "locomo-capability-a0-baseline-20260814T2311Z-d4a71071"
    assert config["provenance"]["turn"]["sha256"] == "43453c5d1b865dd1721bd0892f1af7f965e2c8484fea999eefa5cddf9da78f66"
    assert config["provenance"]["session"]["sha256"] == "46e928b1d4afa865e59689682101a56a024dea46348154ba37e60da97e66c6e4"
    assert config["measurement"]["m1"]["bootstrap"] == {"seed": 20260814, "resamples": 10000}
    assert config["measurement"]["m1"]["margin"] == 0.02405130733344985


def test_phase_a_grid_expands_each_treatment_and_runtime_semantic_cell():
    cells = locomo_phase_a.grid_cells(locomo_phase_a.resolve_config(_document()))

    assert len(cells) == 48
    assert len({cell["id"] for cell in cells}) == 48
    assert {cell["ingest_unit"] for cell in cells} == {"turn", "session"}
    assert {(cell["device"], cell["cache_state"]) for cell in cells} == {
        ("cpu", "cold"), ("cpu", "steady"), ("gpu", "cold"), ("gpu", "steady"),
    }
    assert {cell["treatment"] for cell in cells} == {
        "fts_only", "hybrid", "hybrid_ce_alpha_03_pool_10", "hybrid_ce_alpha_10_pool_10",
        "hybrid_ce_alpha_10_pool_20", "fts_bounded_neighbor",
    }


def test_phase_a_grid_identity_is_invariant_to_axis_ordering():
    original = _document()
    reordered = copy.deepcopy(original)
    grid = reordered["grid"]
    assert isinstance(grid, dict)
    grid["ingest_units"].reverse()
    grid["treatments"].reverse()
    grid["runtime_cells"].reverse()

    original_ids = {cell["id"] for cell in locomo_phase_a.grid_cells(locomo_phase_a.resolve_config(original))}
    reordered_ids = {cell["id"] for cell in locomo_phase_a.grid_cells(locomo_phase_a.resolve_config(reordered))}

    assert reordered_ids == original_ids


def test_phase_a_config_rejects_live_execution_and_runner_always_blocks():
    live = _document()
    live["execution"] = {"mode": "execute", "live_execution": "allowed"}

    with pytest.raises(ValueError, match="plan_only"):
        locomo_phase_a.resolve_config(live)
    with pytest.raises(RuntimeError, match="not authorized"):
        locomo_phase_a.run(locomo_phase_a.resolve_config(_document()))
