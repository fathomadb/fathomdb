# AC-013 drain backlog cost (TC-68169770)

At n = 100,000 (768-d), a cost appeared in
[../ac013-seed-connections/](../ac013-seed-connections/README.md) that grows
with the pending projection backlog and ignores cache size. This directory
traces where it comes from, measures how it scales, and prototypes a fix
under CS64 (`FATHOMDB_PERF_VEC0_CHUNK_SIZE=64`, the lead AC-013 fix).

- Pre-registration: [PREREG.md](PREREG.md)
- Cell table: [cells.json](cells.json)
- Every run: [results.json](results.json), written by `drain_sweep.py
  summarize`

Raw logs are not committed; the driver keeps them under its `--out`
directory.

## Tools

Run every command from the repository root. All builds are release builds
with `--features test-hooks`.

### Engine hooks (`fathomdb-engine`, experiment only)

- **Page tracer by role** (`src/page_trace.rs`, test-hooks).
  `FATHOMDB_PERF_PAGE_TRACE` takes a role list: `reader`, `writer`,
  `dispatcher`, `worker`, `probe`, or `all`. `1` keeps the old reader-only
  scope. Each traced connection records:
  - per statement: page-cache hits and misses, inclusive and exclusive of
    nested statements;
  - SQLite `PROFILE` wall time (`ns`, `exclusive_ns`), keyed by role and SQL
    text.

  It is installed on the writer at open and on every `open_runtime_connection`
  (dispatcher, workers, runtime probes).
- **Drain phase counters** (`src/drain_trace.rs`, test-hooks).
  `drain_phase_trace_for_test()` and `reset_drain_phase_trace_for_test()`
  expose process-global counters:
  - dispatcher scans: count, ns, jobs, candidate pages and rows, in-flight
    skips;
  - worker batches: embed ns, `commit_gate` wait ns, commit ns, jobs;
  - `Engine::drain`'s pending-work probe: count, ns, and how many found work.
- **Query plans** (test-hooks). `Engine::projection_scan_query_plans_for_test()`
  returns `EXPLAIN QUERY PLAN` for the dispatcher's scheduler scan
  (`scheduler_scan_sql`) and the candidate-page query (`candidate_page_sql`)
  on the engine connection, exactly as the current knobs build them.
- **Drain-scan candidate** (not gated on test-hooks). With
  `FATHOMDB_PERF_EXPERIMENTS=1`, `FATHOMDB_PERF_DRAIN_SCAN=armlimit` gives
  each `UNION ALL` arm of both queries its own
  `ORDER BY write_cursor LIMIT 256`. The knob is read on every call.
- Tests: `tests/exp_drain_trace_hooks.rs`.

```bash
cargo test --release -p fathomdb-engine --features test-hooks --test exp_drain_trace_hooks
```

### Drain probe: `src/rust/crates/fathomdb-engine/tests/exp_ac013_seed_drain.rs`

Seeds the AC-013 corpus with the same generator, seed, embedder, batches and
kind as `perf_gates.rs`, then drains. It prints one `AC013_DRAIN {json}` line
with:

- write, drain and seed ms;
- the phase counters, with per-vector values;
- a drain timeline (`AC013_SAMPLE_MS`);
- writer and runtime cache deltas;
- the per-role and per-statement trace when `FATHOMDB_PERF_PAGE_TRACE` is
  set;
- the scan query plans;
- a digest of the projected store: `_fathomdb_vector_rows`, `vector_default`
  without `created_at`, `_fathomdb_projection_terminal`, and
  `_fathomdb_open_state`.

It is `#[ignore]`. Its header lists every knob: `AC013_CORPUS_N`,
`AC013_VECTOR_DIM`, `AC013_SEED_MODE=held|overlap`, `AC013_RUNTIME_THREADS`,
`AC013_DRAIN_TIMEOUT_MS`, `AC013_SAMPLE_MS`, `AC013_DIGEST`,
`AC013_STORE_OUT`, `AC013_TRACE_TOP`.

```bash
AC013_CORPUS_N=30000 FATHOMDB_PERF_PAGE_TRACE=all FATHOMDB_PERF_EXPERIMENTS=1 \
  FATHOMDB_PERF_VEC0_CHUNK_SIZE=64 cargo test --release -p fathomdb-engine \
  --features test-hooks --test exp_ac013_seed_drain -- --ignored --nocapture
```

### Sweep driver: `dev/experiments/ac013-seed-drain/drain_sweep.py`

- `run` executes `cells.json` × `--n` × repeats. Each run:
  - takes the shared `flock` (`--lock`) and waits inside it for 1-minute
    load ≤ 2;
  - records the load before, at its peak and after, and flags runs above
    2.5;
  - interleaves cells, reversing their order on odd repeats.
- `--trace` turns on the tracer for attribution-only runs.
- `summarize` writes `results.json`; `table` prints the timing table.
- Tests: `python3 -m pytest dev/experiments/ac013-seed-drain/test_drain_sweep.py`.

### Steps: `dev/experiments/ac013-seed-drain/run-sweep.sh`

These are the exact runs behind `results.json`, in PREREG order. Set
`S=<scratch>/ac013` and `CARGO_TARGET_DIR` first.

```bash
D=dev/experiments/ac013-seed-drain
cargo test --release -p fathomdb-engine --features test-hooks \
  --test exp_ac013_seed_drain --test exp_ac013_vector_reads --no-run
$D/run-sweep.sh timed; $D/run-sweep.sh big; $D/run-sweep.sh trace; $D/run-sweep.sh vreads
python3 $D/drain_sweep.py summarize --out $S/drain/sweep > $D/results.json
python3 $D/drain_sweep.py table $D/results.json
```

### Other tools

- `show.py <run.log | results.jsonl>` prints one record: phases, roles, and
  the top statements.
- The engine tests under the knob use
  `dev/experiments/ac013-vector-reads/run-tests.sh` (Results § Equivalence).

## Results (2026-10-30)

All runs are 768-d. Except where marked, they are held-mode with two
workers. Timed runs are untraced and are listed r0 / r1 (/ r2). Every
timed run started at 1-minute load ≤ 2. Two runs peaked above 2.5: both
CS64 300k runs (peaks 2.50 and 2.56; drains 304 s and 303 s). They decide
nothing that 100k does not already decide, so neither was repeated.
`analyze.py results.json` reproduces every derived number below.

### Where the drain goes, by connection and statement (traced runs)

Requests are page-cache requests (hits plus misses), from the traced runs.
The tracer costs time, so the ms column is only for comparing rows within a
run.

| Connection / statement | CS64 10k: req per run × runs | CS64 100k: req per run × runs | CS64 100k: exclusive ms | CS64-AL 100k: req per run × runs |
| --- | --- | --- | --- | --- |
| Dispatcher: scheduler scan (`scheduler_scan_sql`) | 5,429 × 159 | 57,078 × 1,564 | 32,417 | 303 × 1,564 |
| Writer: `drain` probe (`candidate_page_sql`) | 6,374 × 7 | 63,037 × 1,380 | 29,383 | not in top 12 (2 probes) |
| Worker: `INSERT INTO vector_default` | 39 × 10,000 | 40 × 100,000 | 920 | 40 × 100,000 |
| Worker: `SELECT sql FROM sqlite_master` (`actual_vector_attr_columns`, per row) | 15 × 10,000 | 15 × 100,000 | 756 | 15 × 100,000 |
| Dispatcher and worker: `dense_member_kind_at` (each, per row) | 13 / 12 × 10,000 | 16 / 15 × 100,000 | 212 / 267 | 16 / 15 × 100,000 |

Per vector, by role:

| Role | CS64 10k req/vec (miss) | CS64 100k req/vec (miss) | CS64-AL 100k req/vec (miss) | CS64-AL 300k req/vec (miss) |
| --- | --- | --- | --- | --- |
| Projection dispatcher | 99.8 (12.4) | 909.0 (115.4) | 21.1 (1.2) | 23.8 (1.3) |
| Writer, during the drain | 4.5 (0.8) | 870.2 (112.2) | 0.2 (0.2) | 0.2 (0.2) |
| Projection workers | 90.7 (1.2) | 95.4 (0.6) | 95.4 (1.3) | 104.1 (1.4) |

Two things grow with the backlog. Everything else is flat per vector.

- **The dispatcher's scheduler scan.** Its `EXPLAIN QUERY PLAN` (the probe's
  `query_plans`) shows the `UNION ALL` subquery running as a
  `CO-ROUTINE pending` that feeds a `USE TEMP B-TREE FOR ORDER BY`.
  - Every scan reads, and sorts, every pending row past the projection
    cursor, with `body`, before `LIMIT 256` applies: about one page request
    per pending row.
  - The scan runs once per 64-row batch. Its total is
    Σ backlog ≈ N² / 128 rows.
  - At 100k it takes 32.6–32.9 s of dispatcher time against a 33.1–33.6 s
    drain. It is the critical path, because the workers wait for jobs.
- **`Engine::drain`'s probe on the writer.**
  `wait_for_idle` runs `connection_has_pending_projection_work` on the
  engine's writer connection each time it wakes with
  `active_jobs + queued_jobs == 0`.
  - The probe's candidate-page query has the same co-routine-plus-sort
    shape, so it is also O(backlog).
  - It ran 1,282–1,394 times at 100k, roughly once for every batch the
    workers spent waiting on a slow scan. It accounts for 100% of the
    writer's drain page requests (rule 1) and 28.9–29.7 s of writer time.
  - **Why the writer works during a drain at all:** this probe is the only
    writer work in held mode.
  - **The one-worker anomaly:** with one worker the runtime goes idle after
    every batch.
    - CS64-t1 at 10k ran 158 probes at 5,437 requests each. That is
      11.9 misses per row: the "12 per row at every cache size". Cache size
      cannot change it, because each probe reads the whole backlog.
    - With `armlimit` the same 158 probes cost 297 requests each, and writer
      drain misses fell from 119,396 to 9,944.

`next_pending_projection_jobs` skips in-flight rows in Rust, which costs
little: 250–256 rows seen per scan at every N. The persisted cursor tracks
the drain (`advance_projection_cursor`), so the scan does not re-read
committed rows. It re-reads, and sorts, the whole un-projected tail.

### Scaling (rule 2)

| Cell | Drain µs/vector at 10k / 30k / 100k / 300k (median) | t(N) / t(10k) | Fit: N² share at 100k | Verdict |
| --- | --- | --- | --- | --- |
| BASE | 327 / 347 / 468 / — | 1.06, 1.43 | 36% | between (commit-bound; scan partly hidden) |
| BASE-c4000 | 84 / 120 / 336 / — | 1.44, 4.01 | 90% | super-linear |
| BASE-c4000-AL | 82 / 81 / 80 / — | 0.99, 0.97 | −3% | linear |
| **CS64** | 61 / 105 / 334 / 1,012 | 1.73, 5.49, 16.7 | ≈100% | **super-linear, O(N²)** |
| **CS64-AL** | 56 / 56 / 57 / 62 | 0.99, 1.01, 1.11 | 5% | **linear, O(N)** |

Held drain in ms, every timed run:

| Cell | 10k | 30k | 100k | 300k |
| --- | --- | --- | --- | --- |
| BASE | 3,311 / 3,235 | 10,491 / 10,327 | 47,247 / 46,312 | — |
| BASE-c4000 | 864 / 811 | 3,692 / 3,533 | 33,773 / 33,422 | — |
| BASE-c4000-AL | 840 / 798 | 2,484 / 2,402 | 8,006 / 7,899 | — |
| CS64 | 617 / 599 | 3,191 / 3,111 | 33,106 / 33,626 | 304,287 / 302,631 |
| CS64-AL | 558 / 566 | 1,686 / 1,661 | 5,667 / 5,650 | 28,870 / 16,928 / 18,723 |

- At 100k, BASE-c4000 and CS64 drain in the same time (33.4 s): the scan is
  the critical path, so it hides CS64's whole layout gain. With the fix, the
  layout gain shows up: 7.9 s against 5.66 s.
- At 300k the two CS64-AL repeats disagreed by 1.7×, so a third repeat was
  run (not pre-registered). r0 (28.9 s) did the same work, with the same
  scan count and scan time, but spent more time in commits from about the
  100k-th row on. Load stayed ≤ 1.9 and its digest was identical. Taken as
  environmental (shared disk at 96%). Median 18.7 s.
- The CS64-AL write phase at 300k is 22.8–23.1 s, against 6.8 s at 100k.
  That writer-side growth is a separate cost and was not investigated here.

### Gate shape (overlap) and the share of the backlog cost (rule 3)

| Cell, n = 100k | Write ms | Drain tail ms | Seed ms | Dispatcher scan ms |
| --- | --- | --- | --- | --- |
| CS64-o | 13,766 / 13,945 | 83 / 80 | 13,849 / 14,026 | 3,757 / 4,116 |
| CS64-AL-o | 13,286 / 13,675 | 74 / 78 | 13,360 / 13,753 | 2,616 / 3,561 |

- **Held:** (CS64 − CS64-AL drain) / CS64 seed = **69%** of the 100k held
  seed (40.1 s → 12.5 s).
- **Overlap** (the gate's shape): **2.7%** (13.9 s → 13.6 s).
  - When the projection runs while the corpus is written, the backlog stays
    short, so the scan stays cheap and is off the critical path.
  - The 100k overlap seed is then about held write plus held drain
    (6.8 + 5.7 s). The one SQLite write lock serializes the writer's commits
    and the worker commits.
  - That serialization, not the backlog, is what remains under CS64 in the
    gate shape.
- At 10k the backlog cost is small in either mode: CS64-AL held drain is
  −7.6% against CS64.

### Equivalence (rule 4): identical

- **Store digests.** Every CS64-AL, BASE-c4000-AL, CS64-AL-o and CS64-AL-t1
  run matches its shipped-scan cell at the same N, and every repeat matches
  every other. This covers 25 runs from 10k to 300k: `_fathomdb_vector_rows`,
  `vector_default` (embedding, bit column, `source_type`, kind, status),
  terminals, and `projection_cursor` = N.
- **AC-013 searches.** `exp_ac013_vector_reads` at 10k gives
  `results_digest` 4d4a989d2c4858ea and `ids_digest` 81d55824f18d31f4 under
  both CS64 and CS64-AL. The per-query hit files are byte-identical.
- **Engine tests under `armlimit`: 0 failures.** Run with
  `FATHOMDB_PERF_EXPERIMENTS=1 FATHOMDB_PERF_DRAIN_SCAN=armlimit`:
  - `run-tests.sh` (40 vector, vec0, projection and slice40 targets);
  - `slice20c_flush_barrier`, `slice20_dense_readiness`,
    `slice30_embedding_readiness`, `slice21_projection_runtime_state`,
    `eu5f_production_pin`, `erasure_drain_ordering`, `rebuild_projections`
    (operator), `slice15_byo_llm_ingest`, `projection_runtime`;
  - the `--lib` unit tests (119);
  - in a debug build, the `#![cfg(debug_assertions)]` targets
    `slice40_projection_completion` (17, including the below-watermark
    rewind), `slice40_projection_generation_races` (5) and
    `tc91_projection_commit_hardening` (6).

### Against the pre-registered rules

1. **Cause: confirmed.**
   - Scheduler-scan requests per run rise 10.5× from 10k to 100k in both
     BASE-c4000 and CS64 (5,429 → 57,078).
   - CS64-AL stays at 290 → 303 (1.6× at 300k: 474).
   - The candidate-page probe is 100% of the writer's drain requests at
     100k.
2. **Scaling:** CS64 is super-linear (t(100k) / t(10k) = 5.5, and 16.7 at
   300k). CS64-AL is linear (1.01, and 1.11 at 300k).
3. **Backlog share at 100k under CS64:** 69% of the held seed, 2.7% of the
   overlap seed.
4. **Equivalence: holds.**
5. **Fix worth a TDD slice: yes.** CS64-AL / CS64 held drain at 100k is
   0.171 and 0.168 in the two repeats; at 10k it is 7.6% faster.

### Why the code is built this way

- **Order of the shape's history:**
  1. 1980bf65d (2026-05-03), the Phase 9 vector runtime: a nodes-only
     `ORDER BY write_cursor LIMIT 32`, which can stream from an index.
  2. fe412aea1 (2026-05-27): the scan fills the whole in-flight budget per
     cycle. AC-013 seed at 10k fell from 28.1 s to 2.5 s.
  3. 0da572c7f (2026-06-13, Slice 15 G11): edge bodies, via a `UNION ALL`
     in a subquery under an outer `ORDER BY write_cursor LIMIT`. This is
     the first version that sorts the whole backlog. The point was to
     project nodes and edges in insertion order (slice-15 design §4.2).
  4. 2a65a38ac (2026-09-05, 0.8.25 Slice 40):
     - added the fixed 256-row page loop, the generation `CROSS JOIN` and
       the per-row `dense_member_kind_at`;
     - put readiness and `drain`'s probe on the same paged query, the
       below-watermark check (`lowest_pending_projection_member_at_or_below`)
       and the cursor rewind.
  5. Slice 40 required `EXPLAIN` fixtures only for its status and readiness
     queries, and treated drain as a setup timeout, not a metric. No
     scheduler or probe query plan was examined until this experiment.
     ADR-0.7.0-text-query-latency-gates-revised already records a 1M seed
     that "did not drain within 3 h (super-linear seed cost)".
- **Invariants the shape protects.** The fix keeps all of them.
  - **The queue is derived from durable state, not held in memory.**
    ADR-0.6.0-projection-model: "No durable projection queue table"; the
    cursor advances only on terminal states. ADR-0.6.0-scheduler-shape:
    memory does not grow with backlog. ADR-0.8.27 topology: terminal-state
    cursor authority. TC-91 design: "the pending canonical rows are the
    source of truth for redispatch".
    - That is why every scan restarts from the persisted cursor and skips
      in-flight rows in Rust.
    - The cursor lags because two workers commit out of order, and
      `advance_projection_cursor` steps only over consecutive terminals.
  - **Readiness and `drain` share one predicate.** R-20-DR (3ae399fdb).
    G11 fix-1 (6471a951d) added the probe's deliberate below-watermark
    check.
    - `drain` is a barrier that consults durable work (record-lifecycle
      api-surface C4), so readiness never says ready while embeds are
      pending.
    - 75e7230da / eda95b07a (2026-09-09) moved the probe onto the engine's
      own connection, with `try_lock` so that it never blocks a writer.
    - That move is why the cost lands on the writer connection.
  - **Eligibility at one instant.** Slice 40 / Slice 30:
    `dense_member_kind_at` runs per candidate in the dispatcher and again
    under the worker's IMMEDIATE transaction.
  - **Generation rediscovery.** Slice 40 design §§ 713–718.

### Blast radius of `armlimit` as a product change

- **Code:** `projection_worker.rs` only. Each `UNION ALL` arm of
  `scheduler_scan_sql` and `candidate_page_sql` is wrapped in
  `SELECT * FROM (<arm> ORDER BY <arm>.write_cursor LIMIT 256)`. Without
  the knob, the prototype keeps the shipped text.
  - Predicates, joins, the generation join, the page loop, in-flight
    skipping, `dense_member_kind_at`, the below-watermark check and the
    cursor rewind are unchanged.
  - The outer `ORDER BY ... LIMIT 256` over at most 512 rows returns the
    same page, because write cursors are unique across nodes and edges
    (`dense_member_kind_at` treats a shared cursor as corruption).
- **Schema:** none. No new index; the arms use
  `canonical_nodes_write_cursor_idx` and `canonical_edges_write_cursor_idx`,
  which already exist.
- **ADRs:** none amended. No ADR or design names the query's shape. A
  short design note beside the Slice 40 scan description would record the
  plan requirement.
- **Tests:**
  - The existing suites listed under Equivalence pass unchanged.
  - A product slice should add a plan or cost test: no temp B-tree inside a
    `UNION ALL` arm, or the tracer-based bound in
    `drain_scan_armlimit_bounds_scheduler_scan_reads_by_page_not_backlog`,
    with a node-plus-edge ordering case.
  - The investigation found no test of in-flight skipping or of strict
    node/edge interleaved dispatch order.
- **Memory and disk:** none added; the sort shrinks from the whole backlog
  to ≤ 512 rows, so the temp B-tree no longer grows with the backlog.
- **Readiness / `read_projections` / `pending_embedding_work`:** these use
  the same candidate-page query, so they get faster. `pending_embedding_work`
  still pages over every pending member (it counts them), but each page now
  costs a page, not the backlog.

### Recommendation

- Land `armlimit` as its own TDD slice, next to CS64: RED with a cost or
  plan test, GREEN with the two-function change.
- What it fixes:
  - the 100k held drain under CS64, from 33.4 s to 5.7 s;
  - drain scaling, from O(N²) to O(N): 300k from 303 s to 18.7 s;
  - the writer's drain misses, from 11 M to 15 k.
- It does little for the AC-013 gate itself at 10k (−7.6% held drain;
  overlap unchanged). It matters for bulk loads, for reopening with a
  backlog, and for any workload where writes outpace projection.
- **Next lever in the gate shape (not prototyped):** single-writer
  serialization of the writer and worker commits.
  - Per-row work that could be hoisted per batch:
    `actual_vector_attr_columns` (a `sqlite_master` read per row, 15
    requests and about 7.5 µs per vector traced) and the commit-side
    `dense_member_kind_at`.
  - The CS64-AL write phase also grows faster than N at 300k.

### Not done

- No overlap run at 300k.
- The writer-phase growth at 300k was not traced.
- No prototype for per-batch hoisting.
- No 384-d runs.
- No restored-flag build.
- The disputed CS64-AL 300k r0 was not re-traced.
