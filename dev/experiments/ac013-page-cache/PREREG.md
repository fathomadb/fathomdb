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
  and after each run.

## Factors and levels

| Factor | Levels | Why |
| --- | --- | --- |
| Page-cache mode | absent (today), restored | The Unit I change itself |
| Reader `cache_size` | -2000 (default), -8000, -32000, -64000 | Size of a per-connection cache that holds the per-search working set |
| Writer `cache_size` | default, -32000 | Seed-time effect; crossed with mode at the default reader size only |
| Reader `mmap_size` | 268435456, flag absent, default reader size | A memory-mapped read path that bypasses most page-cache misses |

Cells (`cells.json`): the 2 × 4 mode × reader grid, two writer cells, one
mmap cell. Reader-pool size has no knob (`READER_POOL_SIZE = 8`), so it is
not a factor.

The writer PRAGMA hook covers the writer connection only. The projection
connections that drain the vector rows keep the default `cache_size`, so the
writer cells test the write path only.

## Arms and repeats

- **gate:** every cell, two repeats, cell order forward in repeat 0 and
  reversed in repeat 1. Flag-absent and flag-restored cells alternate in
  the table, so A and B arms interleave.
- **probe:** the eight grid cells and the mmap cell, one repeat (it supplies
  the cache counters; the gate supplies the latency verdict).
- **mcp c1:** cells A-r2000, R-r2000, and every flag-absent cell that
  passes the gate, two repeats.
- **REQ40-I6** for every fix candidate (see the constraint), two repeats.
  Clause (i), `reader_page_cache_timing_report` on the tc5-10000 pack, is
  the closest runnable form of the heavy-read measurement. The AC40-96 and
  AC40-97 counter tests (`iso`) run once. AC40-96 asserts the 2,000 KiB
  budget by construction, so it fails for any larger `cache_size`. That is
  not an isolation failure. AC40-97 is the isolation property.

## Noise and effect size

- Earlier repeat spread at p50 was 1–2 ms. A difference between cells is
  real when it is at least 5 ms and larger than three times the larger of
  the two cells' repeat ranges.
- Seed time: real when at least 500 ms and larger than three times the
  repeat range.

## Decision rule

H1 is **confirmed** when all three hold:

1. Flag absent: some reader `cache_size` level up to -64000 brings gate
   p50 to 85 ms or less (within 6 ms of `main`'s 79 ms), with probe misses
   per search at least 80% below A-r2000.
2. Flag restored at the default size: gate p50 at 85 ms or less, with
   misses per search at least 80% below A-r2000.
3. Across the flag-absent reader levels, p50 does not rise by a real
   amount as the cache grows.

H1 is **refuted** when no flag-absent level brings p50 within 20 ms of
79 ms, or misses do not fall by at least 50% at -64000. Anything between
is **partial**: the report then names what part of the cost the cache does
not explain.

## The constraint on fix candidates

A candidate is a cell whose gate p50 ≤ 80 ms and p99 ≤ 300 ms in both
repeats. It is acceptable only if:

- REQ40-I6 (i): each after-c01 body pass p50 ≤ 1.15 × the second before
  pass, on both routes, in both repeats; and
- AC40-97 passes.

Each candidate's memory ceiling is reported as per-connection limit ×
connections (8 readers plus the writer, plus the mmap size where it
applies).

## EP-18

Ruled in or out from git and the code, not by a sweep cell. EP-18 part A
(idle-first reader dispatch, merged at `54f762a53`) is not an ancestor of
Unit I `65e6eff7e`, where the regression already shows (122 ms). With one
caller, idle-first dispatch offers each request first to the cursor's
worker, which is idle, so the worker sequence equals round-robin's. EP-18
is ruled out if `54f762a53` is not an ancestor of `65e6eff7e` and the code
reading holds. If the gate at A-r2000 differs from the 122 ms Unit I value
by a real amount, EP-18 stays open and is reported as such.
