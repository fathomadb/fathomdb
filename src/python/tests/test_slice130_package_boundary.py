"""Characterize the installed Python-facing boundary before SDK decomposition."""

from __future__ import annotations

import inspect
import json
from pathlib import Path

import fathomdb
from fathomdb import admin, errors, graph, read
from fathomdb import engine as engine_module


BASELINE = (
    Path(__file__).resolve().parents[3]
    / "dev/plans/0.8.27/features/slice-130/pre-move-python-surface.json"
)


def test_package_boundary_matches_pre_move_baseline() -> None:
    baseline = json.loads(BASELINE.read_text())
    expected_exports = {row["path"] for row in baseline["python_package_exports"]}
    assert len(fathomdb.__all__) == len(expected_exports) == 119
    assert set(fathomdb.__all__) == expected_exports
    for name in expected_exports:
        assert getattr(fathomdb, name) is not None

    assert fathomdb.Engine is engine_module.Engine
    assert fathomdb.read is read
    assert fathomdb.graph is graph
    assert fathomdb.admin is admin
    assert fathomdb.errors is errors
    assert fathomdb.FrozenReadError is errors.FrozenReadError
    assert fathomdb.EvidenceError is errors.EvidenceError
    assert fathomdb.GraphExpansionError is errors.GraphExpansionError
    assert fathomdb.DependencyTraceError is errors.DependencyTraceError
    assert str(inspect.signature(fathomdb.Engine.search)) == (
        "(self, query: 'str', filter: 'SearchFilter | Filter | None' = None, *, "
        "rerank_depth: 'int' = 0, use_graph_arm: 'bool' = False, "
        "alpha: 'float | None' = None, pool_n: 'int | None' = None, "
        "explain: 'bool' = False, view: 'ReadView | None' = None, "
        "limit: 'int' = 10) -> 'SearchResult'"
    )


def test_candidate_sources_are_from_this_checkout() -> None:
    checkout = Path(__file__).resolve().parents[3]
    assert Path(fathomdb.__file__).resolve().is_relative_to(checkout)
    assert Path(inspect.getfile(fathomdb.Engine)).resolve().is_relative_to(checkout)
