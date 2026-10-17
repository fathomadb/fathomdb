"""COMP-02b latency cells are timed on separate engine opens.

One heavy read can change the latency of every later query on the same open
engine (Slice 40 Unit I), so each latency cell and each COMP-02b cohort pass
opens the store afresh and closes it before the next one. These checks spy on
the real engine's open and close calls; the database is real.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from experiments import comp02b_latency, comp_fts
from experiments import comp03_classification as cc

COHORTS = ["c01", "c10", "c33"]


def _inputs(tmp_path: Path) -> Path:
    inputs = tmp_path / "in"
    inputs.mkdir()
    docs = [
        {"id": "d1", "body": "zebra migration across the savanna meeting", "cohort": "c01"},
        {"id": "d2", "body": "penguin colonies on antarctic ice", "cohort": "c10"},
        {"id": "d3", "body": "volcanic soil and coffee farming meeting", "cohort": "c33"},
        {"id": "d4", "body": "penguin chicks huddle on ice", "cohort": "c33"},
    ]
    (inputs / "corpus.json").write_text(json.dumps(docs))
    (inputs / "queries.json").write_text(json.dumps([
        {"id": "q1", "text": "penguin colonies"}, {"id": "q2", "text": "volcanic coffee"},
    ]))
    return inputs


class _Spy:
    """Records every open and close of a real engine, and which engine served each query."""

    def __init__(self) -> None:
        self.events: list[tuple[str, int]] = []


def _spy_engine(monkeypatch: pytest.MonkeyPatch) -> _Spy:
    fathomdb = pytest.importorskip("fathomdb")
    spy = _Spy()
    real = fathomdb.Engine

    class Proxy:
        def __init__(self, engine: Any, serial: int) -> None:
            self._engine = engine
            self.serial = serial

        def __getattr__(self, name: str) -> Any:
            attr = getattr(self._engine, name)
            if name.startswith("search"):
                def call(*args: Any, **kwargs: Any) -> Any:
                    spy.events.append(("query", self.serial))
                    return attr(*args, **kwargs)
                return call
            return attr

        def close(self) -> None:
            spy.events.append(("close", self.serial))
            self._engine.close()

    class Engine:
        @staticmethod
        def open(*args: Any, **kwargs: Any) -> Proxy:
            proxy = Proxy(real.open(*args, **kwargs), sum(kind == "open" for kind, _ in spy.events))
            spy.events.append(("open", proxy.serial))
            return proxy

    monkeypatch.setattr(fathomdb, "Engine", Engine)
    return spy


def _sessions(events: list[tuple[str, int]]) -> list[dict[str, Any]]:
    """Each open, in order, with its query count and whether it closed before the next open."""
    sessions: list[dict[str, Any]] = []
    for kind, engine in events:
        if kind == "open":
            assert all(s["closed"] for s in sessions), "an engine was still open at the next open"
            sessions.append({"engine": engine, "queries": 0, "closed": False})
        else:
            assert sessions and sessions[-1]["engine"] == engine, f"{kind} on an engine that is not the open one"
            if kind == "query":
                sessions[-1]["queries"] += 1
            else:
                sessions[-1]["closed"] = True
    return sessions


def test_latency_harness_times_each_cell_on_its_own_open(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    spy = _spy_engine(monkeypatch)
    timed: list[tuple[str, int]] = []
    real_time_cell = comp02b_latency.time_cell

    def time_cell(engine: Any, cell: dict[str, Any], queries: list[str]) -> dict[str, Any]:
        timed.append((cell["id"], engine.serial))
        return real_time_cell(engine, cell, queries)

    monkeypatch.setattr(comp02b_latency, "time_cell", time_cell)
    summary = comp02b_latency.run(_inputs(tmp_path), tmp_path / "attempt")

    sessions = _sessions(spy.events)
    build, timing = sessions[0], sessions[1:]
    assert build["queries"] == 0 and build["closed"]
    ids = [cell["id"] for cell in comp02b_latency.CELLS]
    assert [cell_id for cell_id, _ in timed] == ids
    assert [s["engine"] for s in timing] == [engine for _, engine in timed], "one open per cell, in cell order"
    assert all(s["closed"] for s in timing)
    assert all(s["queries"] == 4 for s in timing), "two queries: one warm pass and one timed pass per cell"

    assert summary["schema_version"] == "comp-02b.latency.summary.v2"
    assert summary["measurement"] == {"method": "fresh_open_per_cell", "opens": len(ids), "cell_order": ids}
    assert set(summary["cells"]) == set(ids)
    on_disk = json.loads((tmp_path / "attempt" / "summary.json").read_text())
    assert on_disk["measurement"] == summary["measurement"]


def test_latency_receipt_config_names_the_measurement_method():
    config = comp02b_latency.receipt_config({"inputs": {}})
    assert config["measurement"] == "fresh_open_per_cell"
    assert config["version"] == 2


def test_filtered_cell_times_each_cohort_on_its_own_open(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    spy = _spy_engine(monkeypatch)
    cell = comp_fts.run_filtered_cell("fathomdb", 1, _inputs(tmp_path), tmp_path / "work")

    sessions = _sessions(spy.events)
    assert sessions[0]["queries"] == 0, "the build open runs no timed query"
    timing = sessions[1:]
    assert len(timing) == len(COHORTS), "one open per cohort pass"
    assert all(s["closed"] and s["queries"] == 4 for s in timing), timing
    assert cell["measurement"] == {"method": "fresh_open_per_cohort", "opens": 3, "cohort_order": COHORTS}
    assert list(cell["passes"]) == COHORTS
    assert [cell["passes"][c]["open_index"] for c in COHORTS] == [0, 1, 2]
    assert cell["passes"]["c33"]["violations"] == 0


def test_filtered_cohort_orders_match_the_single_open_method(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    """The per-cohort shuffles keep the seeded sequence of the earlier single-open method."""
    pytest.importorskip("fathomdb")
    import random

    cell = comp_fts.run_filtered_cell("fathomdb", 3, _inputs(tmp_path), tmp_path / "work")
    rng = random.Random(3000 + 3)
    for cohort in COHORTS:
        order = [0, 1]
        rng.shuffle(order)
        assert [r["query_id"] for r in cell["passes"][cohort]["results"]] == [f"q{i + 1}" for i in order]


def test_comp02b_v2_config_names_the_fresh_open_per_cohort_method():
    from experiments import comp_record

    v1 = comp_record.load_config(Path("experiments/configs/comp-02/filtered-10000.v1.json"))
    v2 = comp_record.load_config(Path("experiments/configs/comp-02/filtered-10000.v2.json"))
    assert v2["harness_campaign"] == "comp02b" and v2["inputs"] == v1["inputs"] and v2["arms"] == v1["arms"]
    assert "fresh open per cohort" in v2["roster"]["cell"]
    assert "filtered-10000.v2.json" in v2["run"]


def test_classification_reads_a_v2_latency_summary_under_its_own_plan():
    cells = {cell["id"]: {"n": 2, "p50_ms": 1.0, "p95_ms": 2.0, "short_results": 0}
             for cell in comp02b_latency.CELLS}
    metrics = {"schema_version": "comp-02b.latency.summary.v2", "documents": 4, "queries": 2, "build_s": 1.0,
               "inputs": {}, "cells": cells,
               "measurement": {"method": "fresh_open_per_cell", "opens": 6, "cell_order": list(cells)}}
    shape = cc._shape({"experiment": "comp-02b-latency", "metrics": metrics})
    assert shape["plan_id"] == "comp02b-latency-v2"
    assert shape["rule"](["measurement", "opens"], 6, metrics) == ("exclude", "run_control")
    assert shape["rule"](["measurement", "cell_order", "0"], "x", metrics) == ("exclude", "run_control")
    v1 = {**metrics, "schema_version": "comp-02b.latency.summary.v1"}
    del v1["measurement"]
    assert cc._shape({"experiment": "comp-02b-latency", "metrics": v1})["plan_id"] == "comp02b-latency-v1"
