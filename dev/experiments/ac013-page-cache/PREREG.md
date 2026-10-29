# AC-013 page-cache sweep: pre-registration

Written before the sweep runs. Tools and commands: [README.md](README.md).

## What was seen before this was written

Disclosed so the rule below is read against it.

- Gate at `dev/0.8.29` + W4 tooling, one run: p50 124 ms, p99 146 ms,
  seed 3,877 ms (write 3,695 ms, drain 181 ms).
- Probe, one run, same build: hybrid p50 133 ms. Per search, about 72,300
  reader page-cache misses and 68,700 hits. Text-only on the same queries:
  p50 41 ms, about 330 misses. Every reader's `cache_used_bytes` is at the
  2,000 KiB default (2,102,645). In the probe the seed split was write
  1,150 ms and drain 2,701 ms. The write and the background projection
  overlap, so only their sum is compared.

## Hypothesis

H1: under Unit I each reader's page cache is capped at 2,000 KiB. The vector
stage of one hybrid search touches more distinct pages than that, so they
miss and are re-read through the OS on every search. With
`SQLITE_ENABLE_MEMORY_MANAGEMENT` the busy reader could take pages from the
shared group (capacity: the sum of every connection's limit), and the same
pages stayed resident.

Mechanism, stated so it can fail: 72,300 misses × 4 KiB is about 283 MB per
search, several times the whole store (10,000 × 768 × 4 B is about 31 MB of
f32). So the misses are re-reads, not first reads. The likely source is the
phase-1 rerank join on `vector_default`: vec0 keeps vectors in chunk blobs
of 1,024 rows (about 3 MB of f32, some 770 overflow pages), and reading one
candidate's row walks the chunk's overflow chain from its start. About 192
candidates × about 385 pages per walk is about 74,000 page reads, which
matches. A 2,000 KiB cache (about 500 pages) cannot hold one chain, so every
walk misses. A larger cache masks the walk; it does not remove it.

Predicted knee: one chain is about 3 MB and all ten are about 31 MB, so
most of the gain should come by -8000 to -32000, with little more at
-64000. At 384-d, chains and store halve, so the knee should halve.

## Fixed conditions

- Code: branch `feat/0.8.29-w4-ac013` (`dev/0.8.29` `8aec60ece` plus
  measurement tooling only). Release profile. Two builds: the warm `target/`
  (flag absent) and one `CARGO_TARGET_DIR` with
  `LIBSQLITE3_FLAGS=-DSQLITE_ENABLE_MEMORY_MANAGEMENT` (flag restored).
- Workload: the gate as written: n = 10,000, 768-d `VaryingEmbedder`,
  1,000 warm-up and 1,000 timed single-threaded `Engine::search` calls,
  fresh temp store per run. The probe runs the same workload.
- MCP arm: the store seeded once with the default embedder (384-d, CPU) by
  `ac013_mcp seed`, read-only afterwards. The server runs with Block 1
  queue defaults, `FATHOMDB_MCP_LOG=warn`, phase `c1` only.
- Host: this machine (24 threads, 62 GB). No GPU. A run starts only when the
  1-minute load average is at most 2.0; the driver records the load before
  and after each run. The gate and probe stores live in `/tmp`, which is
  disk here, not tmpfs; misses are served from the OS page cache.

## Factors and levels

| Factor | Levels | Why |
| --- | --- | --- |
| Page-cache mode | absent (today), restored | The Unit I change itself |
| Reader `cache_size` | -2000 (default), -8000, -16000 (flag absent only), -32000, -64000 | Size of a per-connection cache that holds the per-search working set |
| Writer `cache_size` | default, -32000 | Seed-time effect; crossed with mode at the default reader size only |
| Reader `mmap_size` | 268435456, flag absent, default reader size | A memory-mapped read path that bypasses most page-cache misses |

| Vector dimension | 768 (gate), 384 (flag absent; default and -32000 only) | Bridges to the MCP arm's 384-d store |

Cells (`cells.json`, 14): the 2 × 4 mode × reader grid, A-r16000, two
writer cells, one mmap cell and two 384-d cells. The table orders the
writer, mmap and 384-d cells among the grid cells, so no group runs only at
one end of a repeat. Reader-pool size has no knob (`READER_POOL_SIZE = 8`),
so it is not a factor.

The writer PRAGMA hook covers the writer connection only. The projection
connections that drain the vector rows keep the default `cache_size`, so the
writer cells test the write path only.

## Arms and repeats

- **gate:** every cell, two repeats, cell order forward in repeat 0 and
  reversed in repeat 1. Flag-absent and flag-restored cells alternate in
  the table, so A and B arms interleave. A third repeat for A-r2000,
  R-r2000 and the best flag-absent level.
- **probe:** every cell except the two writer cells, one repeat. It supplies
  the cache counters, the store's `page_count` and WAL frames, and RSS; the
  gate supplies the latency verdict. The mmap cell is judged by latency
  only: mapped pages bypass the page cache, so its miss count falls whether
  or not it helps.
- **mcp c1:** cells A-r2000, R-r2000, and every flag-absent cell that
  passes the gate, two repeats.
- **REQ40-I6** for every fix candidate (see the constraint), two repeats.
  Clause (i), `reader_page_cache_timing_report` on the tc5-10000 pack, is
  the closest runnable form of the heavy-read measurement. The AC40-96 and
  AC40-97 counter tests (`iso`) run once. AC40-96 asserts the 2,000 KiB
  budget by construction, so it fails for any larger `cache_size`. That is
  not an isolation failure. AC40-97 is the isolation property.

## Noise and effect size

- Statistics use microseconds (`p50_us` from the gate's raw samples), not
  the gate's whole-millisecond print. The primary comparison between two
  cells is the paired one: for each of the 1,000 queries, the median over
  the cell's repeats, minus the same for A-r2000, then the median of those
  differences.
- Earlier repeat spread at p50 was 1–2 ms. A difference between cells is
  real when it is at least 5 ms and larger than three times the larger of
  the two cells' p50 repeat ranges (taken as at least 0.5 ms).
- Seed time: real when at least 500 ms and larger than three times the
  repeat range.

## Decision rule

The mechanism is judged against in-sweep controls, not against `main`'s
79 ms from another day and tree. The gate's 80 ms limit is the separate fix
verdict below.

H1 is **confirmed** when all three hold:

1. Flag restored at the default size (R-r2000): misses per search at least
   80% below A-r2000, and p50 lower than A-r2000 by a real amount.
2. Flag absent: some reader `cache_size` level up to -64000 has misses per
   search at least 80% below A-r2000 and a paired p50 difference to
   R-r2000 of at most 5 ms.
3. Across the flag-absent reader levels, p50 does not rise by a real
   amount as the cache grows.

H1 is **refuted** when R-r2000 is not faster than A-r2000 by a real amount,
or when no flag-absent level up to -64000 cuts misses by at least 50%
although the store (`db_bytes` from the probe) fits in that level. If the
store does not fit in -64000, the refutation by misses is void, and the
report says so. Anything between is **partial**: the report then names what
part of the cost the cache does not explain.

The knee prediction is checked separately: it holds if the flag-absent p50
at -32000 is within 5 ms of the best level.

## The constraint on fix candidates

A candidate is a cell whose gate p50 ≤ 80 ms and p99 ≤ 300 ms in both
repeats. It is acceptable only if:

- REQ40-I6 (i): each after-c01 body pass p50 ≤ 1.15 × the second before
  pass, on both routes, in both repeats;
- AC40-97 passes; and
- probeiso: after/before hybrid p50 ratio ≤ 1.15.

A flag-absent candidate passes the two slice40 checks almost by
construction (no connection can take another's pages), so its main risk is
memory; the probe's RSS is reported with each candidate. Under the flag
restored, the group limit grows with the lever (connections × per-connection
limit), so isolation must be measured, not assumed.

Each candidate's memory ceiling is reported as per-connection limit ×
connections (8 readers plus the writer, plus the mmap size where it
applies).

## Reviewer points not adopted

- A non-capacity fix arm (a rowid-keyed f32 table for the rerank, or a
  smaller vec0 `chunk_size`) is likely the best candidate, but it is a
  product change and this task forbids those. FINDINGS lists it as an
  unmeasured option.

## EP-18

Ruled in or out from git and the code, not by a sweep cell. EP-18 part A
(idle-first reader dispatch, merged at `54f762a53`) is not an ancestor of
Unit I `65e6eff7e`, where the regression already shows (122 ms). With one
caller, idle-first dispatch offers each request first to the cursor's
worker, which is idle, so the worker sequence equals round-robin's. EP-18
is ruled out if `54f762a53` is not an ancestor of `65e6eff7e` and the code
reading holds. If the gate at A-r2000 differs from the 122 ms Unit I value
by a real amount, EP-18 stays open and is reported as such.
