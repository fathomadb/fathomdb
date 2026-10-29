"""Measurement-correctness tests for sweep.py: parsers, percentile, order, env.

  python3 -m pytest dev/experiments/ac013-page-cache/test_sweep.py
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import sweep  # noqa: E402

GATE_LOG = """\
test ac_013_vector_retrieval_latency ... AC013_NUMBERS n=10000 samples=4 seed_ms=3877 p50_ms=124 p99_ms=146
AC013_TREATMENT_RECORD treatment=warm n=10000 seed_write_ms=3695 embedding_ms=not_separately_observable \
projection_drain_ms=181 accepted_writes=10000 vector_rows_after_drain=10000 drain_outcome=ok \
samples_us=400,100,300,200 result_counts=not_retained_per_query
"""


def test_gate_numbers_and_seed_split():
    got = sweep.parse_gate(GATE_LOG)
    assert (got["p50_ms"], got["p99_ms"], got["seed_ms"]) == (124, 146, 3877)
    assert (got["seed_write_ms"], got["drain_ms"]) == (3695, 181)
    # Sorted 100,200,300,400: p50 = ceil(4*0.5)=2nd = 200; p99 = ceil(3.96)=4th = 400.
    assert (got["p50_us"], got["p99_us"], got["mean_us"]) == (200, 400, 250)


def test_percentile_matches_the_gate():
    samples = list(range(1, 1001))
    assert sweep.percentile(samples, 50) == 500
    assert sweep.percentile(samples, 99) == 990
    assert sweep.percentile([7], 99) == 7


def test_i6_ratio_is_max_after_over_second_before():
    log = "\n".join([
        'unit-i route=defect-c pass="before 1" p50_ms=9.000 p95_ms=1.0 x',
        'unit-i route=defect-c pass="before 2" p50_ms=2.000 p95_ms=1.0 x',
        'unit-i route=defect-c pass="after c01, 1" p50_ms=2.200 p95_ms=1.0 x',
        'unit-i route=defect-c pass="after c01, 2" p50_ms=2.500 p95_ms=1.0 x',
        'unit-i route=defect-c pass="after c01, 3" p50_ms=2.100 p95_ms=1.0 x',
    ])
    got = sweep.parse_i6(log)["defect-c"]
    assert got["before2_p50_ms"] == 2.0
    assert abs(got["max_ratio"] - 1.25) < 1e-12


def test_iso_and_probe_and_mcp_parsers():
    iso = "test a_heavy ... ok\ntest other_readers ... FAILED\n"
    assert sweep.parse_iso(iso) == {"a_heavy": "ok", "other_readers": "FAILED"}
    assert sweep.parse_probe('x AC013_PROBE {"n": 3}\n') == {"n": 3}
    mcp = json.dumps({"command": "mcp", "phases": [{"phase": "c1_warmup"},
                                                   {"phase": "c1", "ok": {"p50_ms": 5}}]})
    assert sweep.parse_mcp("noise\n" + mcp + "\n")["ok"]["p50_ms"] == 5


def test_cells_alternate_direction_across_repeats():
    cells = [{"id": "a"}, {"id": "b"}, {"id": "c"}]
    assert [c["id"] for c in sweep.cell_order(cells, 0)] == ["a", "b", "c"]
    assert [c["id"] for c in sweep.cell_order(cells, 1)] == ["c", "b", "a"]


def test_cell_env_sets_levers_and_clears_inherited_ones():
    base = {"FATHOMDB_PERF_READER_PRAGMAS": "stale", "LIBSQLITE3_FLAGS": "stale", "PATH": "/bin"}
    absent = sweep.cell_env({"id": "x", "mode": "absent"}, base, None)
    assert "FATHOMDB_PERF_READER_PRAGMAS" not in absent and "LIBSQLITE3_FLAGS" not in absent
    restored = sweep.cell_env(
        {"id": "y", "mode": "restored", "reader": "cache_size=-8000"}, base, "/t")
    assert restored["LIBSQLITE3_FLAGS"] == sweep.RESTORE_FLAG
    assert restored["CARGO_TARGET_DIR"] == "/t"
    assert restored["FATHOMDB_PERF_EXPERIMENTS"] == "1"
    assert restored["FATHOMDB_PERF_READER_PRAGMAS"] == "cache_size=-8000"
    assert restored["FATHOMDB_PERF_WRITER_PRAGMAS"] == ""
