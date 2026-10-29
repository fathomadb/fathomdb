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

The runs behind `results.json`, from the repository root, with
`S=<scratch>/ac013` and `MM=<restored target>` (deleted afterwards):

```bash
D=dev/experiments/ac013-seed-connections
R="--cells $D/cells.json --out $S/seed/sweep --lock $S/measure.lock --restored-target $MM"
python3 $D/seed_sweep.py run $R --arms seed --repeats 2
python3 $D/seed_sweep.py run $R --arms gate --repeats 2 --only A768-o-c2000,A768-o-c4000
AC013_CORPUS_N=100000 python3 $D/seed_sweep.py run $R --arms seed --repeats 2 \
  --only A768-h-c2000,A768-h-c4000,A768-o-c2000,A768-o-c4000
python3 $D/seed_sweep.py summarize --out $S/seed/sweep > $D/results.json
```

## Results (2026-10-29)

Cell ids: build (A absent, R restored), dimension, mode (h held, o
overlap), runtime `cache_size` (c2000 is the default), `-t1` one worker,
`-sdoff` `secure_delete=OFF`. Two repeats each, shown as r0/r1. No seed run
exceeded load 2.5 except R768-o-c4000 r1 (peak 2.8, seed 1,916 ms), which
decides nothing.

### Connection map

| Connection | Writes during seed | Misses per vector, A768 default | At -4000 |
| --- | --- | --- | --- |
| Writer | nodes, FTS, cursors | 1.0–1.7 per row (held) | unchanged |
| Projection dispatcher | cursor rewind only; reads pending rows | 12.4 | 12.3 |
| Projection workers (2) | every vector (`vector_default`, `_fathomdb_vector_rows`, terminals, cursor) | 278.8–278.9 | 7.4–7.6 |

Each worker vector costs 459 page requests at 768-d and 264 at 384-d in
every cell; the cache decides only how many miss.

### Seed time, n = 10,000 (ms)

| Cell | Write (held) | Drain (held) | Seed (overlap, gate shape) | Worker misses / vector | Worker misses / batch |
| --- | --- | --- | --- | --- | --- |
| A768 default | 479 / 507 | 3,264 / 3,280 | 3,702 / 3,744 | 278.8 / 278.9 | 17,755 / 17,763 |
| A768 -4000 | 476 / 494 | 828 / 827 | 1,269 / 1,261 | 7.5 / 7.5 | 481 / 477 |
| A768 -8000 | 475 / 504 | 841 / 846 | 1,321 / 1,349 | 7.6 / 7.5 | 481 / 481 |
| A768 -32000 | 474 / 469 | 828 / 846 | 1,341 / 1,365 | 7.5 / 7.5 | 476 / 476 |
| A768 -64000 | 467 / 474 | 848 / 813 | 1,310 / 1,283 | 7.4 / 7.6 | 471 / 481 |
| A384 default | 472 / 471 | 654 / 653 | 1,119 / 1,158 | 4.0 / 3.8 | 258 / 245 |
| R768 default | 472 / 479 | 1,195 / 1,229 | 1,509 / 1,535 | 7.6 / 7.5 | 481 / 480 |
| R384 default | 473 / 474 | 927 / 960 | 1,305 / 1,203 | 4.2 / 4.1 | 265 / 264 |
| A768 default, 1 worker | 497 / 481 | 3,742 / 3,721 | 4,162 / 4,133 | 276.5 / 276.5 | 17,613 / 17,613 |
| A768 -4000, 1 worker | 500 / 473 | 1,223 / 1,244 | 1,678 / 1,673 | 0.3 / 0.3 | 16 / 16 |
| A768 default, `secure_delete=OFF` | 471 / 471 | 3,263 / 3,299 | — | 278.7 / 278.7 | 17,754 / 17,755 |
| A768 -8000, `secure_delete=OFF` | 467 / 469 | 835 / 818 | — | 7.5 / 7.5 | 478 / 480 |

A384 and R cells do not move with the runtime `cache_size` (every level in
`results.json`). In overlap mode the write/drain split still swings
(A768 default write 803–3,528 ms): the writer waits for the write lock that
slow worker commits hold.

### Gate, A768 (warm, 1,000 searches)

| Cell | p50 (ms) | p99 (ms) | Gate seed (ms) |
| --- | --- | --- | --- |
| default | 125\*, 122 | 143\*, 140 | 3,967\*, 3,716 |
| runtime -4000 | 123, 122 | 133, 135 | 1,331, 1,330 |

\* Load peaked at 9.3 during the run.

### n = 100,000, A768 (ms)

| Cell | Write | Drain | Seed | Worker misses / vector | Dispatcher misses / vector | Writer drain misses |
| --- | --- | --- | --- | --- | --- | --- |
| held, default | 6,710 / 6,779 | 48,588 / 46,607 | 55,298 / 53,385 | 293.0 / 292.8 | 115.7 / 115.7 | 6.4 M / 6.6 M |
| held, -4000 | 7,941 / 6,730 | 34,003 / 33,968 | 41,944 / 40,699 | 2.8 / 2.8 | 115.3 / 115.3 | 10.8 M / 10.7 M |
| overlap, default | 42,209 / 27,541 | 1,185 / 14,866 | 43,394 / 42,407 | 292.9 / 292.6 | 11.3 / 36.0 | 0.02 M / 0.8 M |
| overlap, -4000 | 7,745 / 7,321 | 24,818 / 27,410 | 32,563 / 34,731 | 4.0 / 4.0 | 88.8 / 97.8 | 7.6 M / 8.3 M |

Worker misses per vector stay flat with N. A second cost that grows with the
pending backlog appears at 100k: dispatcher scans and the writer-side
pending-work checks during `drain`. The runtime `cache_size` does not touch
it.

### Against the pre-registered rule

- **Cause: confirmed.** A768 default worker misses per vector are 70× A384's
  (prediction 1). -4000 cuts them by 97% and the held drain from 3.27 s to
  0.83 s, more than the whole gap to R768 (prediction 2). Both repeats.
- **Best cell: runtime `cache_size=-4000`** (overlap seed median 1,265 ms;
  -8000 to -64000 are 1,296–1,353 ms). It beats the restored flag (1,509 /
  1,535 ms).
- **No leak into readers.** Gate p50 differs from the default by 0–2 ms.
- **WAL reset: supported.** With two workers a large cache still misses
  about 480 pages per batch, one chain walk; with one worker it misses 16
  (prediction 3). One worker is slower overall (embedding runs on fewer
  threads).
- Predictions 4 (writer unaffected), 5 (`secure_delete` irrelevant) and 6
  (misses per vector flat at 100k) hold.
- **Not explained here:** the backlog-growing dispatcher and writer drain
  misses at 100k, and the one-worker cells' writer drain misses (12 per row,
  identical across cache levels), likely the `drain` idle check scanning the
  pending set each time no job is active.
