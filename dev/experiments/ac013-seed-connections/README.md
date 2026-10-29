# AC-013 seed time on the projection connections (dev/0.8.29)

Flag-absent AC-013 seeding (n = 10,000, 768-d) takes about 4 s, against
about 1.5 s with `SQLITE_ENABLE_MEMORY_MANAGEMENT` restored
([../ac013-page-cache/](../ac013-page-cache/README.md)). This directory
tests whether the projection worker connections, which insert the vectors
and keep SQLite's 2 MB default page cache, cause it.

- Pre-registration: [PREREG.md](PREREG.md)
- Cell table: [cells.json](cells.json)
- Every repeat: [results.json](results.json), written by
  `seed_sweep.py summarize`

Raw logs are not committed; the driver keeps them under its `--out`
directory.

## Tools

Run every command from the repository root. All are release builds.

### Engine hooks (`fathomdb-engine`, experiment only)

- `FATHOMDB_PERF_EXPERIMENTS=1` with `FATHOMDB_PERF_RUNTIME_PRAGMAS`
  (comma-separated `name=value`): applied by `open_runtime_connection` in
  `connection_runtime.rs` to the projection dispatcher, the projection
  workers and runtime probe connections, after `journal_mode` and
  `secure_delete`. Does nothing unless both are set. The existing
  `FATHOMDB_PERF_READER_PRAGMAS` / `FATHOMDB_PERF_WRITER_PRAGMAS` are
  unchanged.
- Under `--features test-hooks` only:
  - `Engine::runtime_cache_status_for_test()` returns the dispatcher's and
    each worker's `CACHE_HIT` / `CACHE_MISS` / `CACHE_USED`, refreshed after
    every dispatcher scan and worker commit batch, with scan or batch counts
    and job counts;
  - `Engine::writer_cache_status_for_test()` reads the writer's;
  - `Engine::runtime_pragma_for_test(name)` reads a PRAGMA on a fresh runtime
    connection.
- Tests: `tests/exp_runtime_connection_hooks.rs` (the hook applies
  `cache_size`; the counters move on every writing connection).

```bash
cargo test --release -p fathomdb-engine --features test-hooks \
  --test exp_runtime_connection_hooks
```

### Seed probe: `src/rust/crates/fathomdb-engine/tests/exp_ac013_seed_connections.rs`

Seeds the gate's corpus (same generator, seed, embedder and batch size) and
prints one `AC013_SEED {json}` line: write, drain and seed time, and per
writing connection the cache hits and misses, with misses per inserted
vector and per commit batch. `#[ignore]` and `test-hooks`-gated. Its header
lists every field.

```bash
AC013_SEED_MODE=held cargo test --release -p fathomdb-engine --features test-hooks \
  --test exp_ac013_seed_connections -- --ignored --nocapture
```

- `AC013_SEED_MODE=held` freezes the projection scheduler while writing
  (`set_projection_scheduler_frozen_for_test`), so write and drain are timed
  apart; `overlap` is the gate's shape.
- Knobs: `AC013_CORPUS_N`, `AC013_VECTOR_DIM`, `AC013_SEED_MODE`,
  `AC013_RUNTIME_THREADS` (`scheduler_runtime_threads`),
  `AC013_DRAIN_TIMEOUT_MS`, and the PRAGMA levers above.

### Sweep driver: `dev/experiments/ac013-seed-connections/seed_sweep.py`

Runs `cells.json` against the arms `seed` (the probe) and `gate`
(`scripts/perf-experiments/run-ac013.sh`, warm). Each run takes an exclusive
lock on `--lock` (the same flock other measuring agents use), waits inside it
for 1-minute load ≤ `--max-load`, samples the load each second while it
runs, and flags it above `--flag-load`. Cells interleave across repeats. Its
docstring lists every option. Tests:
`python3 -m pytest dev/experiments/ac013-seed-connections/test_seed_sweep.py`.

Before a sweep, build both targets so no timed run compiles:

```bash
cargo test --release -p fathomdb-engine --features test-hooks \
  --test exp_ac013_seed_connections --test perf_gates --no-run
CARGO_TARGET_DIR=$MM LIBSQLITE3_FLAGS=-DSQLITE_ENABLE_MEMORY_MANAGEMENT \
  cargo test --release -p fathomdb-engine --features test-hooks \
  --test exp_ac013_seed_connections --test perf_gates --no-run
```
