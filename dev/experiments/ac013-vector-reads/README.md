# AC-013 vector reads (option 5, dev/0.8.29)

The page-cache sweep (`../ac013-page-cache/`) found that one 768-d hybrid
search asks the page cache for about 141,000 pages from a 14,400-page store,
and that misses decide AC-013 latency. This experiment traces where those
requests come from, prototypes ways to stop the vector stage re-reading
pages, and measures them at the default 2 MB reader cache.

- Pre-registration: [PREREG.md](PREREG.md)
- Results, every repeat: [results.json](results.json), written by
  `vsweep.py summarize`

The prototypes live on the experiment branch only. A winner becomes its own
TDD slice; nothing here lands as-is.

## Tools

Run every command from the repository root. All builds are release builds.

### Page-request tracer: `src/rust/crates/fathomdb-engine/src/page_trace.rs`

`test-hooks` only. With `FATHOMDB_PERF_PAGE_TRACE` set when the engine
opens, each reader connection gets an `sqlite3_trace_v2` (`STMT` |
`PROFILE`) callback that records `SQLITE_DBSTATUS_CACHE_HIT` /
`_CACHE_MISS` at each statement's start and end. Statements that start
while another is open (vec0's and FTS5's internal statements) are children;
`exclusive` subtracts them. `take_page_trace_for_test()` returns the
aggregate by SQL text. Blob reads inside a vec0 callback count against the
innermost open statement. The callback's own cost inflates latency, so take
latency from untraced runs.

### Vector-read probe: `src/rust/crates/fathomdb-engine/tests/exp_ac013_vector_reads.rs`

The AC-013 fixture (same generator, seeds, embedder and batches as the gate
and `exp_ac013_page_cache.rs`). `#[ignore]`, `test-hooks`.

```bash
FATHOMDB_PERF_PAGE_TRACE=1 cargo test --release -p fathomdb-engine \
  --features test-hooks --test exp_ac013_vector_reads -- --ignored --nocapture
```

- Prints one `AC013_VREADS {json}` line: hybrid and text-only p50/p99/mean
  and page requests and misses per search, `results_digest` (every hit's
  cursor, branch and score bits over the timed pass) and `ids_digest`, seed
  write and drain time, store size before and after a checkpoint, and with
  the tracer the per-statement table (`trace`, `trace_text_only`).
- Knobs: `AC013_CORPUS_N`, `AC013_VECTOR_DIM`, `AC013_SAMPLES`,
  `AC013_RESULTS_OUT` (per-query hit lines, for diffing),
  `AC013_TRACE_TOP`, `FATHOMDB_PERF_PAGE_TRACE`, and the candidate knobs.

### Candidate knobs: `src/rust/crates/fathomdb-engine/src/vector_read_experiment.rs`

Inactive unless `FATHOMDB_PERF_EXPERIMENTS` is set. Not gated on
`test-hooks`, so the MCP server honours them too.

| Knob | Effect |
| --- | --- |
| `FATHOMDB_PERF_VECTOR_RERANK=chunked` | Bit-KNN alone, then each candidate's f32 read from its vec0 chunk blob through one blob handle per chunk, in offset order; same `vec_distance_l2` |
| `FATHOMDB_PERF_VECTOR_RERANK=f32table` | Rerank from `_fathomdb_vector_f32` by rowid; falls back to the shipped join when the table or a row is missing |
| `FATHOMDB_PERF_VECTOR_F32_TABLE=1` | Create `_fathomdb_vector_f32` with the partition and write each projected vector to it (insert path only) |
| `FATHOMDB_PERF_VEC0_CHUNK_SIZE=N` | Create new vec0 partitions with `chunk_size=N` |

### Store relayout: `src/rust/crates/fathomdb-engine/examples/ac013_relayout.rs`

Copies a seeded store into a candidate's layout without re-embedding
(stage, drop, recreate with `chunk_size`, refill; or fill
`_fathomdb_vector_f32`), then `VACUUM`. Tooling only: this is the data
migration `AGENTS.md` § 10 forbids in a product release. Used for the MCP
arm, whose bge store takes about 30 minutes to seed.

```bash
cargo build --release -p fathomdb-engine --example ac013_relayout
B=target/release/examples/ac013_relayout
$B /tmp/ac013-bge.sqlite /tmp/stores/base-vacuumed.sqlite
$B /tmp/ac013-bge.sqlite /tmp/stores/cs64.sqlite --chunk-size 64
$B /tmp/ac013-bge.sqlite /tmp/stores/cs256.sqlite --chunk-size 256
$B /tmp/ac013-bge.sqlite /tmp/stores/f32t.sqlite --f32-table
```

### Sweep driver: `dev/experiments/ac013-vector-reads/vsweep.py`

Imports `../ac013-page-cache/sweep.py` and adds a shared `flock`
(`--lock`; the load wait happens inside the lock), the arms `vreads`,
`vtrace` and `recall` (`ac_013b_recall_at_10_floor` with `AGENT_LONG=1`),
per-cell MCP stores (`mcp_store` under `--mcp-store-dir`), and a
`load_flag` when the load after a run exceeds `--flag-load` (2.5). All
sweep.py arms (`gate`, `mcp`, `i6`, `iso`, ...) work unchanged. Cells:
[cells.json](cells.json).

### Test runner: `dev/experiments/ac013-vector-reads/run-tests.sh`

Runs the engine's vector, vec0, projection and slice40 test targets (plus
`pr_g10_filtered_knn` and `perf_gates`) under one candidate's knobs.

```bash
OUT=/tmp/tests dev/experiments/ac013-vector-reads/run-tests.sh CHUNKED \
  FATHOMDB_PERF_EXPERIMENTS=1 FATHOMDB_PERF_VECTOR_RERANK=chunked
```

## Commands behind results.json

With `OUT`, `LOCK` (the shared measurement lock), `STORES` (the relayout
stores) and `TC5=~/.cache/fathomdb-comp/tc5-10000`:

```bash
D=dev/experiments/ac013-vector-reads
R="--cells $D/cells.json --out $OUT --lock $LOCK --mcp-store-dir $STORES --tc5-inputs $TC5"
python3 $D/vsweep.py run $R --arms vtrace --repeats 1
python3 $D/vsweep.py run $R --arms gate,vreads --repeats 2
python3 $D/vsweep.py run $R --arms recall,iso --repeats 1
python3 $D/vsweep.py run $R --arms i6 --repeats 2
AC013_SAMPLES=500 python3 $D/vsweep.py run $R --arms mcp --repeats 2
AC013_CORPUS_N=100000 python3 $D/vsweep.py run $R --arms gate --repeats 1 --only BASE,F32T,CHUNKED,CS64
AC013_CORPUS_N=100000 AC013_SAMPLES=300 python3 $D/vsweep.py run $R --arms vtrace --repeats 1 --only BASE,F32T,CHUNKED,CS64
python3 $D/vsweep.py summarize --out $OUT > $D/results.json
```

Before a sweep, build so no run compiles: `cargo test --release -p
fathomdb-engine --features test-hooks --no-run` for the `perf_gates`,
`exp_ac013_vector_reads` and `slice40_reader_page_cache` targets, and the
MCP build in `../ac013-page-cache/README.md`.
