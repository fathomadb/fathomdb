# AC-013 vector reads (option 5): pre-registration

Written before the candidate measurements run. Tools and commands:
[README.md](README.md). Prior sweep: `../ac013-page-cache/`.

## What was seen before this was written

Disclosed so the rule below is read against it.

- **Trace, baseline, one run** (`vtrace`, n = 10,000, 768-d, 2 MB reader
  cache). Per hybrid search: 141,036 page requests and 72,340 misses, all
  attributed to statements. By exclusive cost:
  - phase-1 vector statement (bit-KNN plus the `JOIN vector_default`
    rerank): 75,670 requests, 67,310 misses;
  - FTS arm (`search_index` MATCH and its FTS5 internals): about 51,900
    requests, about 1,700 misses (it reads all 10,000 matching rows'
    content and docsize);
  - the two eligibility `EXISTS` scans (`_fathomdb_vector_rows` join and
    `canonical_nodes`): 12,600 requests, 2,900 misses;
  - vec0 rowid lookups: 768 requests (384 runs, two per candidate);
  - vec0 KNN chunk iterator: 43 requests.
- **Sanity runs** at n = 3,000 and 200 queries, one each, untimed: every
  candidate below gave the same results digest (hit cursors, branches and
  score bits) as the baseline. Page requests per search: baseline 92,700,
  CHUNKED 18,900, F32T 16,800, CS64 22,600, CS256 36,700.

## Hypothesis

H1: the phase-1 rerank walks each candidate's vec0 chunk overflow chain from
its start. `vec0_get_vector_data` (sqlite-vec 0.1.9) opens a fresh blob
handle per candidate and vector column, and SQLite's overflow-page cache
(`BtCursor.aOverflow`) lives on that handle, so every read starts at the
first overflow page. A 768-d f32 chunk is about 770 pages, so a read costs
about 385 page requests on average. A 2 MB cache cannot hold a chain, so
most requests miss. Shortening the walk removes the misses whatever the
cache size.

## Candidates (cells, `cells.json`)

| Cell | Knob | Storage change |
| --- | --- | --- |
| BASE | none | none |
| CHUNKED | `FATHOMDB_PERF_VECTOR_RERANK=chunked`: bit-KNN alone, then read each candidate's f32 from the chunk blob, grouped by chunk, in offset order, through one blob handle per chunk | none |
| CS64 | `FATHOMDB_PERF_VEC0_CHUNK_SIZE=64`: vec0 `chunk_size=64` | vec0 table option |
| CS256 | `FATHOMDB_PERF_VEC0_CHUNK_SIZE=256` | vec0 table option |
| F32T | `FATHOMDB_PERF_VECTOR_F32_TABLE=1` and `FATHOMDB_PERF_VECTOR_RERANK=f32table`: a rowid-keyed `_fathomdb_vector_f32` table written next to vec0; the rerank reads it | new table |

Predictions per search at n = 10,000, phase-1 page requests: CHUNKED about
7,000 (ten chains walked once each); CS64 about 5,000; CS256 about 19,000;
F32T about 1,000. At n = 100,000, CHUNKED grows towards 192 chain walks
again (about 98 chunks, most holding a candidate), while CS64 and F32T
stay near their 10,000 figures for the rerank; the bit-KNN scan grows with
n in every cell.

## Fixed conditions

- Code: `exp/0.8.29-ac013-vector-reads` (`dev/0.8.29` `9db251dbf` plus the
  tracer, the probe and the candidate knobs). Release builds. Flag absent
  (no `SQLITE_ENABLE_MEMORY_MANAGEMENT`), default 2 MB reader cache, no
  reader or writer PRAGMA levers.
- Engine workload: the gate as written (n = 10,000, 768-d
  `VaryingEmbedder`, 1,000 warm-up and 1,000 timed searches, fresh store per
  run).
- MCP: `ac013_mcp mcp` at 1 in flight, 500 + 500 calls. BASE and CHUNKED
  use a copy of the existing bge store passed through the same tool with no
  layout change (VACUUM only); CS64, CS256 and F32T use a copy rewritten by
  `examples/ac013_relayout.rs` (same vectors, new layout), with the
  cell's knobs in the server's environment.
- Every timed run holds the shared lock and starts at 1-minute load ≤ 2.0;
  a run whose load after exceeds 2.5 is flagged and repeated once.

## Arms and repeats

- `gate`: 2 repeats per cell, interleaved (forward then reverse order).
- `vreads`: 2 repeats per cell: misses, page requests, results digest, seed
  time, store size.
- `vtrace`: 1 run per cell: attribution.
- `recall` (`ac_013b`, report-only): 1 run per cell.
- `i6` (REQ40-I6 clause i): 2 repeats per cell; `iso` (AC40-96/97): 1 run.
- `mcp`: 2 repeats per cell.
- n = 100,000 (`AC013_CORPUS_N=100000`, gate report-only): `gate` and
  `vreads` (`AC013_SAMPLES=300` for `vreads`), 1 run each, for BASE and the
  best candidate by the rule below; CHUNKED too if it is not the best,
  since its headroom is the open question.

## Decision rule

A candidate **qualifies** when all hold:

1. Its results digest equals BASE's in every `vreads` run (identical
   retrieval, so `ac_013b` recall is unchanged; the `recall` arm must also
   print the same recall as BASE).
2. Gate p50 ≤ 80 ms and p99 ≤ 300 ms in both repeats.
3. Phase-1 misses per search (trace) fall by at least 80 % against BASE.
4. REQ40-I6 ratio ≤ 1.15 on both routes in both repeats, and AC40-97
   passes.
5. MCP p50 at 1 in flight is no more than 5 ms above BASE's (mean of
   repeats).

Among qualifying candidates, **recommend** in this order:

1. The best headroom: the lowest n = 100,000 gate p50, provided its phase-1
   page requests per search at 100,000 are under 2 × its 10,000 figure.
2. Within 5 ms gate p50 of that, prefer no storage change over a vec0
   option, and a vec0 option over a new table.
3. Disk growth over BASE above 25 % counts against a candidate; it is
   reported, not disqualifying.

If no candidate qualifies, report which criterion each fails.
