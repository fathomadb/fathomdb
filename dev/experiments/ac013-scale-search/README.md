# AC-013 scale search (dev/0.8.29)

At n = 100k a hybrid search costs about 600 ms p50 under every vector fix
(`../ac013-vector-reads/`). This experiment explains that cost: how many
documents the text arm matches on the fixture and on a realistic corpus,
what each statement costs at 10k, 30k and 100k, why the code reads what it
reads, and env-gated prototypes for each component. Todo TC-b23a5ead
(seq 352).

- Pre-registration: [PREREG.md](PREREG.md)
- Results, every run: [results.json](results.json) (`ssweep.py summarize`)
- Findings: [Results](#results) below

The prototypes live on the experiment branch only. A winner becomes its own
TDD slice; nothing here lands as-is.

## Tools

Run every command from the repository root. All builds are release builds.

### Scale-search probe: `src/rust/crates/fathomdb-engine/tests/exp_ac013_scale_search.rs`

`#[ignore]`, `test-hooks`. The AC-013 fixture (same generator, seeds,
`VaryingEmbedder`, batches and digests as `exp_ac013_vector_reads.rs`), or a
JSONL corpus, seeded into a fresh store; then timed passes of `hybrid`
(`Engine::search`), `text` (`Engine::search_text_only`) and `vector`
(`Engine::search` on queries that match no text). Prints one
`AC013_SCALE {json}` line: per pass p50/p99/mean, cache hits and misses per
search, soft fallbacks, `digest`/`ids_digest`, and with the tracer the
per-statement table; once, the FTS match count per distinct query.

```bash
FATHOMDB_PERF_PAGE_TRACE=1 AC013_CORPUS_N=100000 AC013_SAMPLES=300 \
  AC013_PASSES=hybrid,text,vector cargo test --release -p fathomdb-engine \
  --features test-hooks --test exp_ac013_scale_search -- --ignored --nocapture
```

Knobs: `AC013_CORPUS_N`, `AC013_VECTOR_DIM`, `AC013_SAMPLES`,
`AC013_CORPUS_FILE` + `AC013_QUERY_FILE`, `AC013_SUPERSEDE_N`,
`AC013_PASSES`, `AC013_TRACE_TOP`, `AC013_RESULTS_OUT`,
`FATHOMDB_PERF_PAGE_TRACE` and the `FATHOMDB_PERF_*` candidate knobs.

### Page tracer time: `src/rust/crates/fathomdb-engine/src/page_trace.rs`

The vector-read experiment's per-statement page tracer now also records
`inclusive_ns` / `exclusive_ns` from `SQLITE_TRACE_PROFILE`. A statement's
time runs from its first step to its reset, so a top-level statement's time
includes the Rust row decoding between its steps. The tracer's callbacks
cost about a microsecond per statement and land on the parent, so statements
with many nested lookups read high; latency comes from untraced runs. Test:
`tests/exp_page_trace_time.rs`.

### Candidate knobs: `src/rust/crates/fathomdb-engine/src/scale_search_experiment.rs`

Inactive unless `FATHOMDB_PERF_EXPERIMENTS` is set; read once per process.
Not gated on `test-hooks`. Tests: `tests/exp_scale_search_knobs.rs`.

| Knob | Effect |
| --- | --- |
| `FATHOMDB_PERF_ELIGIBILITY=cache` | Answers the two direct-eligibility scans from a cache keyed by path, `_fathomdb_read_visibility_state` (generation, nonce) and view flags; only time-independent answers are cached |
| `FATHOMDB_PERF_FTS_ROWID_CURSOR=1` | Projector inserts body-FTS rows with `rowid = write_cursor` |
| `FATHOMDB_PERF_TEXT_ARM=deferred` | On the deferred-identity hybrid route, scores every match from `rowid, bm25()` and reads content only for rows that can reach the fused top `final_limit`; falls back to the complete path when a row's rowid is not its cursor |

### Corpus writer: `dev/experiments/ac013-scale-search/lme_corpus.py`

Writes LongMemEval-S turns (`corpus.jsonl`, 189,514 unique turns) and its
500 questions (`queries.jsonl`) into a scratch directory. The text never
enters the repository.

```bash
python3 -I dev/experiments/ac013-scale-search/lme_corpus.py \
  --lme ~/.cache/huggingface/hub/datasets--xiaowu0162--longmemeval-cleaned/snapshots/*/longmemeval_s_cleaned.json \
  --out $SCRATCH/lme
```

### Match counts: `dev/experiments/ac013-scale-search/match_counts.py`

The text arm's match count per query, from an in-memory FTS5 table with the
engine's tokenizer and a copy of `compile_text_query` (content-OR over the
non-stop-word tokens). `--synthetic` re-implements the fixture generator;
`--corpus/--queries` read the JSONL files. Cross-checks the probe's
`matches` and reaches sizes the probe does not seed.

### Sweep driver: `dev/experiments/ac013-scale-search/ssweep.py`

Runs a prebuilt probe binary per cell and repeat under the shared
measurement lock (load wait inside the lock), appends to
`<out>/results.jsonl`; `summarize` writes `results.json`; `run-gate` runs
the `perf_gates` AC-013 gate. Cells: [cells.json](cells.json).

### Stage runner: `dev/experiments/ac013-scale-search/run-stages.sh`

The pre-registered stages as commands. Build and copy the binary first so
later builds cannot change what a sweep measures:

```bash
B=$(cargo test --release -p fathomdb-engine --features test-hooks \
  --test exp_ac013_scale_search --no-run 2>&1 | grep -o '/[^ ]*exp_ac013_scale_search-[0-9a-f]*')
cp $B $SCRATCH/bin/scale-$(git rev-parse --short HEAD)
BIN=... SHA=... OUT=$SCRATCH/out LOCK=$LOCK LME=$SCRATCH/lme \
  dev/experiments/ac013-scale-search/run-stages.sh A B
```

### Analysis: `dev/experiments/ac013-scale-search/analyze.py`

`components` groups each traced run's statements into components (FTS
content, docsize, index, sorter, the two eligibility scans, vector stage,
hydration) with the 10k→n scaling exponent; `latency` lists untraced p50/p99
with digests, fallbacks and load flags, every repeat.

## Results

Pending.
