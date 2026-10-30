"""Tests for drain_sweep.py: cell order, env isolation, record parsing, grouping."""

import importlib.util
from pathlib import Path

spec = importlib.util.spec_from_file_location("drain_sweep", Path(__file__).with_name("drain_sweep.py"))
drain_sweep = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drain_sweep)


def test_cell_order_alternates():
    cells = [{"id": "a"}, {"id": "b"}, {"id": "c"}]
    assert [c["id"] for c in drain_sweep.cell_order(cells, 0)] == ["a", "b", "c"]
    assert [c["id"] for c in drain_sweep.cell_order(cells, 1)] == ["c", "b", "a"]


def test_cell_env_clears_perf_knobs_and_sets_n():
    base = {"PATH": "/bin", "FATHOMDB_PERF_DRAIN_SCAN": "armlimit", "AC013_CORPUS_N": "5"}
    env = drain_sweep.cell_env({"env": {"FATHOMDB_PERF_EXPERIMENTS": "1"}}, base, 30000, False)
    assert env["PATH"] == "/bin"
    assert "FATHOMDB_PERF_DRAIN_SCAN" not in env
    assert env["FATHOMDB_PERF_EXPERIMENTS"] == "1"
    assert env["AC013_CORPUS_N"] == "30000"
    assert "FATHOMDB_PERF_PAGE_TRACE" not in env
    assert drain_sweep.cell_env({}, base, 1, True)["FATHOMDB_PERF_PAGE_TRACE"] == "all"


def test_parse_drain_reads_the_record_line():
    log = 'noise\nAC013_DRAIN {"n": 10, "drain_ms": 2.5}\nmore'
    assert drain_sweep.parse_drain(log) == {"n": 10, "drain_ms": 2.5}
    assert drain_sweep.parse_drain("no record") == {}


def test_summarize_groups_by_cell_and_n_and_derives_per_vector():
    def rec(cell, n, repeat, drain_ms):
        return {"cell": cell, "env": {}, "n": n, "traced": False, "repeat": repeat, "exit": 0,
                "load_before": 1.0, "load_peak": 1.5, "load_after": 1.2, "load_flag": False,
                "result": {"n": n, "drain_ms": drain_ms, "write_ms": 1.0, "seed_ms": drain_ms + 1}}
    out = drain_sweep.summarize([rec("A", 10000, 0, 100.0), rec("A", 10000, 1, 120.0),
                                 rec("A", 30000, 0, 600.0)])
    assert sorted(out) == ["A@n10000", "A@n30000"]
    assert [r["repeat"] for r in out["A@n10000"]["runs"]] == [0, 1]
    assert out["A@n10000"]["runs"][0]["drain_us_per_vector"] == 10.0
    assert out["A@n30000"]["runs"][0]["drain_us_per_vector"] == 20.0
