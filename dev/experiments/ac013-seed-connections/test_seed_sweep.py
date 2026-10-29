"""Measurement-correctness tests for seed_sweep.py."""

import importlib.util
from pathlib import Path

_SPEC = importlib.util.spec_from_file_location("seed_sweep", Path(__file__).with_name("seed_sweep.py"))
seed_sweep = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(seed_sweep)


def test_cell_order_reverses_on_odd_repeats():
    cells = [{"id": "a"}, {"id": "b"}, {"id": "c"}]
    assert [c["id"] for c in seed_sweep.cell_order(cells, 0)] == ["a", "b", "c"]
    assert [c["id"] for c in seed_sweep.cell_order(cells, 1)] == ["c", "b", "a"]


def test_cell_env_sets_runtime_pragmas_and_clears_inherited_levers():
    base = {"FATHOMDB_PERF_READER_PRAGMAS": "cache_size=-64000", "AC013_CORPUS_N": "100000"}
    env = seed_sweep.cell_env(
        {"id": "x", "mode": "absent", "runtime": "cache_size=-8000",
         "env": {"AC013_SEED_MODE": "held"}}, base, None)
    assert env["FATHOMDB_PERF_EXPERIMENTS"] == "1"
    assert env["FATHOMDB_PERF_RUNTIME_PRAGMAS"] == "cache_size=-8000"
    assert "FATHOMDB_PERF_READER_PRAGMAS" not in env
    assert env["AC013_CORPUS_N"] == "100000" and env["AC013_SEED_MODE"] == "held"
    plain = seed_sweep.cell_env({"id": "y", "mode": "absent"}, base, None)
    assert "FATHOMDB_PERF_EXPERIMENTS" not in plain


def test_cell_env_restored_uses_flag_and_separate_target():
    env = seed_sweep.cell_env({"id": "r", "mode": "restored"}, {}, "/tmp/mm")
    assert env["LIBSQLITE3_FLAGS"] == "-DSQLITE_ENABLE_MEMORY_MANAGEMENT"
    assert env["CARGO_TARGET_DIR"] == "/tmp/mm"


def test_parse_gate_uses_ceil_percentile():
    samples = ",".join(str(v) for v in range(1, 101))
    log = ("AC013_NUMBERS n=10000 samples=100 seed_ms=3900 p50_ms=122 p99_ms=135\n"
           f"AC013_TREATMENT_RECORD treatment=warm n=10000 seed_write_ms=2000 "
           f"embedding_ms=x projection_drain_ms=1900 accepted_writes=1 samples_us={samples} r=x\n")
    out = seed_sweep.parse_gate(log)
    assert (out["seed_ms"], out["p50_ms"], out["seed_write_ms"], out["drain_ms"]) == (3900, 122, 2000, 1900)
    assert (out["p50_us"], out["p99_us"]) == (50, 99)


def test_seed_keep_derives_per_vector_figures():
    res = {"n": 100, "write_ms": 1.0, "drain_ms": 2.0, "seed_ms": 3.0,
           "worker_totals": {"miss_per_job": 4.0, "miss_per_round": 256.0, "requests_per_job": 9.0,
                             "rounds": 2},
           "runtime": [{"role": "projection_dispatcher", "cache_miss": 50, "cache_used_bytes": 1},
                       {"role": "projection_worker", "cache_miss": 400, "cache_used_bytes": 7}],
           "writer": {"miss_per_row": 0.5}}
    keep = seed_sweep.seed_keep(res)
    assert keep["dispatcher_miss_per_vector"] == 0.5
    assert keep["worker_miss_per_vector"] == 4.0 and keep["worker_used_bytes"] == [7]
