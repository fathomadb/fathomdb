from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[2]
RUNNER = ROOT / "scripts/perf-experiments/run_gauntlet.py"


def _runner():
    spec = importlib.util.spec_from_file_location("graph_gauntlet_runner", RUNNER)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_graph_suite_is_optional_and_does_not_change_default() -> None:
    runner = _runner()

    assert runner.parse_cell_selection(None) == runner.DEFAULT_CELLS
    assert runner.DEFAULT_CELLS == (
        "ac076",
        "ac072",
        "ac081",
        "ac073",
        "ac075",
        "scale02",
        "protected-writes",
        "ce-profile",
        "search01",
        "locomo",
    )
    assert runner.parse_suite_selection("graph", None) == (
        "graph-evidence01",
        "graph-expand01",
        "graph-retrieval01",
    )


def test_suite_and_cells_are_mutually_exclusive() -> None:
    runner = _runner()

    with pytest.raises(runner.GauntletArgumentError, match="mutually exclusive"):
        runner.parse_suite_selection("graph", "graph-evidence01")


def test_graph_cells_have_canonical_order_after_existing_optional_cell() -> None:
    runner = _runner()

    assert runner.ALL_CELLS[-4:] == (
        "ac013-scale-matrix",
        "graph-evidence01",
        "graph-expand01",
        "graph-retrieval01",
    )

