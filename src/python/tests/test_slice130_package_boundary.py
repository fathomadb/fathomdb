"""Characterize the installed Python-facing boundary before SDK decomposition."""

from __future__ import annotations

import hashlib
import importlib.util
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


def test_python_declarations_match_pre_move_baseline_with_approved_deltas() -> None:
    checkout = Path(__file__).resolve().parents[3]
    package = checkout / "src/python/fathomdb"
    spec = importlib.util.spec_from_file_location(
        "slice130_surface_comparator", checkout / "dev/tools/surface_comparator.py"
    )
    assert spec is not None and spec.loader is not None
    comparator = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(comparator)

    sources = {
        path.relative_to(package).as_posix(): path.read_text() for path in package.rglob("*.py")
    }
    baseline = json.loads(BASELINE.read_text())
    declarations = comparator.parse_python_wrappers(sources)
    baseline_declarations = baseline["python_wrapper_declarations"]
    expected = {row["path"]: row for row in baseline_declarations}
    assert len(baseline_declarations) == len(expected) == 1131

    # The snapshot is historical; these later public-interface changes are explicit.
    for path in ("fathomdb.Engine.search_frozen", "fathomdb.engine.Engine.search_frozen"):
        old = expected[path]
        assert "pool_n: int=0" in old["signature"]
        expected[path] = {
            **old,
            "signature": old["signature"].replace("pool_n: int=0", "pool_n: int | None=None"),
        }
    expected["fathomdb.errors.DependencyTraceError"] = {
        "kind": "python-value",
        "path": "fathomdb.errors.DependencyTraceError",
        "signature": "DependencyTraceError = _DependencyTraceError",
    }
    observed = {row["path"]: row for row in declarations}
    assert len(declarations) == len(observed) == len(expected) == 1132
    assert observed == expected
    stub = (package / "_fathomdb.pyi").read_text()
    prefix, marker, tail = stub.partition("    def search_frozen(\n")
    assert marker and "    def search_frozen(\n" not in tail
    frozen, suffix = tail.split("    ) -> SearchResult: ...\n", 1)
    approved = "pool_n: int | None = ..."
    assert frozen.count(approved) == 1
    historical_stub = (
        prefix
        + marker
        + frozen.replace(approved, "pool_n: int = ...", 1)
        + "    ) -> SearchResult: ...\n"
        + suffix
    )
    assert hashlib.sha256(historical_stub.encode()).hexdigest() == baseline["native_stub_sha256"]
