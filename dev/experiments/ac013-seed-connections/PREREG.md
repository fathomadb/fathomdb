# AC-013 seed time on the projection connections: pre-registration

Written 2026-10-29 on `exp/0.8.29-ac013-seed-connections` (cut from
`dev/0.8.29` `9db251dbf`) before any timed run. Tools: [README.md](README.md).

## Question

Flag-absent AC-013 seeding (n = 10,000, 768-d) takes 3.75–4.26 s, against
1.47–1.59 s with `SQLITE_ENABLE_MEMORY_MANAGEMENT` restored and 1.11–1.20 s
at 384-d. Does the 2 MB default page cache on the projection worker
connections, which insert the vectors, cause the difference?

## Connections that write during seed and drain (from the code)

| Connection | Opened by | Writes | PRAGMAs today |
| --- | --- | --- | --- |
| Writer (1) | `open.rs` `open_managed_connection` (`Writer`) | `Engine::write`: `canonical_nodes`, FTS, write and projection cursors | `secure_delete=ON`, `journal_mode=WAL`, `synchronous=NORMAL`, writer hook |
| Projection dispatcher (1) | `projection_worker.rs` `projection_dispatcher_loop` → `open_runtime_connection` | Reads pending rows; writes `_fathomdb_open_state` projection cursor only when it rewinds | `journal_mode=WAL`, `secure_delete=ON` |
| Projection workers (`scheduler_runtime_threads`, default 2) | `projection_worker_loop` → `open_runtime_connection` | `commit_projection_outcomes`: `_fathomdb_vector_rows`, `vector_default` (vec0), `_fathomdb_projection_terminal`, the projection cursor; batches of up to 64, serialized by `commit_gate` | `journal_mode=WAL`, `secure_delete=ON` |

Readers write nothing. Only the workers insert vectors.

## Mechanism under test

sqlite-vec 0.1.9 `vec0Update_InsertWriteFinalStep` opens a fresh
`sqlite3_blob_open` on the current chunk's f32 blob for every insert and
writes at `chunk_offset × dim × 4`. A fresh blob handle has no overflow-page
cache, so the write walks the overflow chain from its start. One 768-d chunk
is 3,145,728 B, about 769 overflow pages; a 384-d chunk is about 385. The
default `cache_size=-2000` holds about 470 pages. A sequential walk longer
than an LRU cache misses on every page; one shorter than it hits after the
first walk.

A second effect: in WAL mode a connection drops its page cache when it starts
a transaction after another connection has committed. With two workers
alternating commits (and the writer committing in overlap mode), a worker can
reuse its cache only within one 64-row batch.

## Factors and levels

| Factor | Levels |
| --- | --- |
| Runtime `cache_size` (`FATHOMDB_PERF_RUNTIME_PRAGMAS`) | default (-2000), -4000, -8000, -32000, -64000 |
| Build | flag absent (A); flag restored (R, `LIBSQLITE3_FLAGS=-DSQLITE_ENABLE_MEMORY_MANAGEMENT`, own `CARGO_TARGET_DIR`, deleted afterwards) |
| Dimension | 768, 384 |
| Seed mode | held (scheduler frozen while writing, then drain alone); overlap (the gate's shape) |
| Workers, A768 only | 2 (default) and 1 (`AC013_RUNTIME_THREADS=1`), all cache levels, both modes |
| `secure_delete=OFF` on runtime connections, A768 held only | default and -8000 |

52 cells (`cells.json`), two repeats each, interleaved (reverse order on the
second repeat). Every timed run holds the shared measurement lock and starts
at 1-minute load ≤ 2; runs whose load exceeds 2.5 are flagged and repeated
once if they decide a criterion.

Then:

- **Leak check (gate arm):** `run-ac013.sh` warm for A768 default and the best
  cell, two repeats each.
- **Larger corpus:** `AC013_CORPUS_N=100000` for A768 default and the best
  cell, held and overlap, two repeats each.

Reported per cell: write ms, drain ms, seed ms (overlap: the gate's
`seed_ms`), worker misses per inserted vector and per batch, dispatcher
misses per vector, writer misses per row.

## Predictions

1. A768 default: worker misses per vector in the hundreds. A384 default: at
   least 10× fewer.
2. At A768, -4000 or larger cuts worker misses per vector by ≥ 80% and the
   held drain time by at least half the gap between A768 default and R768
   default.
3. With 2 workers and a large cache, worker misses per batch stay near one
   chain walk (hundreds); with 1 worker in held mode they fall below 10% of
   that.
4. Writer misses per row do not change with the runtime `cache_size`.
5. `secure_delete=OFF` changes neither misses nor time by more than run-to-run
   spread (inserts free no pages).
6. At 100k, misses per vector stay at the 10k level in each cell (only the
   last chunk is written), so the lever's gain per vector holds.

## Decision rule

- **Cause confirmed** if predictions 1 and 2 hold in both repeats.
  **Partial** if misses fall as predicted but drain time does not, or the
  reverse. **Refuted** if A768 default misses per vector are within 3× of
  A384 default.
- **Best cell:** among A768 two-worker cells, the smallest `cache_size` whose
  overlap seed time (median of repeats) is within 5% of the fastest level.
- **No leak into readers:** the best cell's gate p50 is within ±5 ms of A768
  default in both repeats. The gate's seed time is also recorded.
- **WAL reset** is supported if prediction 3 holds; refuted if one worker
  shows the same misses per batch as two.
- **Fix framing:** if the best cell is -4000 or -8000 and the 100k run shows
  flat misses per vector, a fixed `cache_size` sized to one chunk is a fix
  whose size depends on dimension × `chunk_size`, not on corpus size. If
  misses per vector grow with N, it is not, and the fix is structural.
