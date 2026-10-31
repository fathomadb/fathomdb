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

Runs 2026-10-30 on windchill3, release builds, binaries `eaf9752c1`
(stage A) and `49bf0a69d` (stages B to E; same default code, plus the
knobs). Every number below is in [results.json](results.json); `analyze.py
results.json latency` and `... components` print them. Latency is untraced
p50 per search, median of the repeats (stage B repeats agreed within 1.6%).
Traced times are inflated by the tracer (about a microsecond per nested
statement, charged to the parent) and are for attribution only.

### 1. Is the workload realistic? Partly.

The text arm compiles a query as an OR of every non-stop-word token
(`fathomdb-query` `compile_text_query`, IR-C content-OR, `eff146522`), so a
document matches if it shares any one token with the query.

| Corpus | Terms per query | Matches at 10k | at 30k | at 100k |
| --- | --- | --- | --- | --- |
| SYN (AC-013 fixture) | 43.5 | 100% of n, every query | 100% | 100% |
| LME (LongMemEval-S turns, 500 questions) | 8.5 | median 29%, mean 33% | 29% / 34% | 30% / 34% |

`match_counts.py` gives the same counts (and 30% / 34% at all 189,514
turns). The synthetic queries are 55-74-token documents drawn from a
1,024-word Zipf vocabulary, so every query contains the head words every
document contains. Real questions are short, but content-OR still matches a
third of the store, because ordinary words survive the stop list (an
exploratory pass at 30k, not in results.json, found the most frequent term
of most questions to be "have", "many", "can", "new" or "time").

Verdict (PREREG rule 4): the text cost is **real** in kind, O(matches) with
matches about 0.3 n on natural-language questions, so it is O(n) on real
stores too. It is **overstated by about 3.4x** in size by the fixture
(median 100% vs 29% of n, and 5x more doclists per query). Neither
pre-registered bound holds exactly (LME's median is above 5%, and 3.4x is
just outside "within 3x"). The eligibility scans do not depend on the
query and are fully real; on LME, whose turns are longer, they cost more
than on the fixture.

A second realism finding (stage D): in a store with **any** superseded
node the vector arm is declined on every default search. LME 10k with 10
supersessions: 500 of 500 hybrid searches returned `SoftFallback(Vector)`
and the vector-only pass returned no hits. Node supersession keeps the old
version's vector row (`write_commit.rs`, by design since the fix-30 era;
edges prune theirs), and Slice 35's fail-closed rule declines the whole arm
whenever any vector row is ineligible. An agent-memory store that re-ingests
by `logical_id` is therefore effectively text-only today, which matters
more than any latency here.

### 2. Per-statement attribution, CS64, hybrid (traced, per search)

| Component | SYN 10k req / miss / ms | SYN 100k | LME 10k | LME 100k | b (req, ms) |
| --- | --- | --- | --- | --- | --- |
| FTS content lookups (`search_index_content` by rowid) | 30,814 / 1,565 / 43.3 | 304,997 / 16,496 / 431.8 | 10,160 / 1,986 / 10.9 | 102,666 / 20,496 / 110.8 | 1.00, 1.00 |
| FTS top-level statement (doclists, bm25, row decoding) | 361 / 139 / 11.3 | 1,122 / 249 / 109.9 | 69 / 37 / 3.8 | 159 / 83 / 37.6 | 0.4, 0.99 |
| FTS docsize lookups (bm25 lengths) | 20,000 / 28 / 3.1 | 200,000 / 292 / 31.7 | 6,695 / 29 / 1.2 | 67,980 / 301 / 11.7 | 1.00, 1.01 |
| Vector-row eligibility `EXISTS` | 11,144 / 1,494 / 5.5 | 214,382 / 15,056 / 56.0 | 12,920 / 3,273 / 7.9 | 233,152 / 33,827 / 83.0 | 1.26-1.28, 1.01 |
| All-nodes eligibility `EXISTS` | 1,437 / 1,432 / 2.5 | 14,356 / 14,354 / 25.4 | 3,213 / 3,209 / 4.9 | 33,129 / 33,128 / 50.1 | 1.00, 1.01 |
| Vector stage (bit-KNN + rerank + vec0 internals) | 6,599 / 4,969 / 9.7 | 11,372 / 7,057 / 31.5 | 6,622 / 4,988 / 9.5 | 11,387 / 7,071 / 31.2 | 0.24, 0.51 |
| Hydration | 100 / 47 / 0.1 | 120 / 65 / 0.2 | 100 / 48 / 0.1 | 120 / 62 / 0.1 | ~0 |
| Total traced (untraced p50) | 75.6 (63.9) | 687.1 (610.8) | 38.3 (32.6) | 324.7 (282.9) | |

BASE differs only in the vector stage: 76k requests / 67-75k misses /
76-110 ms at every n (the overflow-chain rerank of `../ac013-vector-reads/`).

Scaling verdicts (rule 3):

- **FTS content, docsize and row decoding: O(matches).** Requests per
  match are the same on both corpora (content 3.05 vs 3.02, docsize 2.00 vs
  2.00 per match); traced ms per match differ by 1.3x (longer LME rows,
  shorter doclists). Because matches are ~n on SYN and ~0.34 n on LME, both
  grow as O(n).
- **Both eligibility scans: O(n)**, independent of the query (identical in
  the hybrid and the no-match vector pass). The vector-row scan's requests
  grow as n^1.27 (an index probe per row whose depth grows); its time is
  linear. The nodes scan reads the whole `canonical_nodes` table including
  bodies, so its cost is proportional to corpus bytes (14k pages SYN, 33k
  pages LME at 100k).
- **Vector stage: CS64 sublinear to 100k, then linear in the bit scan**
  (section 5). BASE is flat to 100k because its fixed rerank cost dominates.
- Text-only (`search_text_only`, rank-stream route) is O(matches) too:
  FTS5's internal `ORDER BY rank` scores every match before the limit.
  p50 SYN 41 / 125 / 413 ms, LME 4.1 / 10.4 / 32.2 ms at 10k / 30k / 100k,
  unchanged by every knob.

### 3. Why the code does this

**The text arm reads every match's content.** Hybrid search with no
filter takes the Slice 71 deferred-identity route (`84c056c62`,
`dev/plans/0.8.25/features/slice-71/design.md` v8): `SELECT body, kind,
write_cursor, bm25(search_index), rowid FROM search_index WHERE MATCH`,
every row, sorted in Rust by (bm25, cursor). It reads `body` for all of
them because RRF fusion (`fusion.rs` `fuse_three_arms`) keys on body text
and credits each body at its first text rank, and a vector hit can sit at a
deep text rank. Slice 71 measured a fixed-prefix variant at p50 59 ms and
rejected it because exact RRF can need ranks beyond any prefix; it kept the
complete list and only deferred the identity join. bm25 itself needs every
match's `docsize` row, and FTS5 has no top-k pruning (no WAND or MaxScore),
so even `ORDER BY rank LIMIT k` scores every match. Nothing filters after
the FTS; there are no snippets on this route.

**The two eligibility scans** come from 0.8.25:

- `vector_arm_requires_fallback` (Slice 35, `c3de8c593`;
  ADR-0.8.25-eligibility-and-frozen-reads): vec0 cannot apply lifecycle,
  validity or dependency predicates inside its KNN, and post-cap filtering
  is forbidden, so before KNN the engine proves that no vector row maps to
  an ineligible or missing node or edge, and otherwise declines the whole
  arm. The proof is `EXISTS(... _fathomdb_vector_rows LEFT JOIN
  canonical_nodes ... LEFT JOIN canonical_edges ...)`: a scan of every
  vector row with an index probe and a table read per row.
- `all_nodes_directly_eligible` (Slice 71): the deferred-identity route may
  skip the per-row `canonical_nodes` join only if no node is superseded,
  inactive or outside its window, proven by `EXISTS(SELECT 1 FROM
  canonical_nodes WHERE ...)`, a full table scan.

Both ask "does any violating row exist?". On a clean store the answer is
no, which can only be proven by reading everything, on every search. The
predicates are parameterised ORs (`?2=0 AND superseded_at IS NOT NULL OR
...`), so no index, partial or not, can serve them. Their answers are pure
functions of three tables, the view flags and (only through validity
windows and `t_invalid`) the instant, so they can be cached per snapshot
or maintained incrementally.

### 4. Prototypes (CS64 primary), untraced hybrid p50, median of repeats

| Cell | SYN 10k | SYN 30k | SYN 100k | LME 10k | LME 30k | LME 100k |
| --- | --- | --- | --- | --- | --- | --- |
| BASE | 124.1 | 255.3 | 687.1 | 94.1 | 163.0 | 352.9 |
| CS64 | 63.9 | 185.6 | 610.8 | 32.6 | 85.8 | 282.9 |
| CS64+ELIG | 56.5 | 160.7 | 530.6 (-13%) | 20.1 | 48.9 | 149.3 (-47%) |
| CS64+FTS | 47.6 | 131.7 | 418.0 (-32%) | 24.2 | 60.6 | 185.3 (-35%) |
| CS64+ALL | 39.8 | 108.0 | 336.3 (-45%) | 11.2 | 21.2 | 54.4 (-81%) |

- **Identity (rule 1): exact.** Within each corpus and n, every CS64 cell
  has the same hybrid `digest` and `ids_digest` and the same text-only
  digests (for example SYN 100k `13fca8d1ecbb4b36` / `201a878a48fbc9c0`,
  LME 100k `9ffc88179abe1e4e` / `5b0f88eb7725f2d5`). BASE's hybrid digest
  matches CS64's at 30k and 100k and on LME at every n; on SYN 10k it
  differs (`9d58...` vs `4d4a...`, same `ids_digest`), the known
  chunk-layout score-bit difference from `../ac013-vector-reads/`, not a
  prototype effect.
- **Effect (rule 2): all three material at 100k on both corpora**, both
  repeats agreeing. On the realistic corpus the two fixes compound to 5.2x
  (283 to 54 ms); on the fixture to 1.8x (611 to 336 ms), where scoring
  100k matches remains.
- **ELIG** removes both scans after the first search on a snapshot (cache
  hits; `exp_scale_search_knobs`). What remains of the text arm under FTS
  (traced, SYN 100k): the `rowid, bm25()` statement 340 ms traced
  (doclists, 200k docsize lookups, the sort); content is read for 20 rows
  (the 10 fused vector hits, checked, and the first 10 text-only bodies)
  instead of 100,000.
- Text-only p50 is unchanged by every cell (table in section 2): the
  prototypes touch only the hybrid route. Hybrid under CS64+ALL (336 ms at
  SYN 100k) is now faster than text-only (413 ms), because sorting
  `(rowid, bm25)` pairs in Rust beats FTS5's rank sorter plus its
  `canonical_nodes` join; the text-only route could take the same path (not
  prototyped).
- Gate (stage E, `perf_gates` AC-013 at 10k, informational): CS64 63 / 64
  ms, CS64+ALL 38 / 38 ms p50 (p99 70 / 126 and 44 / 46). All four runs
  ended above load 2 (2.2-3.7, other projects); they decide nothing.
- Existing suites under all knobs (`../ac013-vector-reads/run-tests.sh`,
  26 search, lifecycle, eligibility, frozen-read, snippet and fusion
  targets): 24 pass; `slice71_search_statement_trace` and
  `slice75_feature_interactions` fail only on their route-witness asserts
  (they expect the `hybrid_deferred_identity` route name; their result
  equality asserts against the forced complete ranking pass).
- MCP 1-in-flight at 100k: not run (section 8).

### 5. CS64 at larger n: the vector stage (stage C)

`vector` pass (queries that match no text) with the eligibility cache, so
the search is the vector stage plus hydration. Traced split of phase 1:

| n | Cell | Untraced p50 | Bit-KNN chunk scan | f32 rerank (exclusive) | Phase 1 total (traced) |
| --- | --- | --- | --- | --- | --- |
| 100k | BASE+ELIG | 104.1 | 16.3 ms | 88.3 ms | 105.4 |
| 100k | CS64+ELIG | 31.3 | 23.4 ms | 7.3-7.5 ms | 31.3-31.5 |
| 300k | BASE+ELIG | 134.6 | 49.4 ms | 92.4 ms | 142.7 |
| 300k | CS64+ELIG | 75.7 | 69.6 ms | 7.5 ms | 77.6 |

Rule 5: no regression at the largest n measured; CS64's phase 1 is 46%
below BASE's at 300k. But the bit scan is linear and 1.4x dearer per row
under CS64 (0.23 vs 0.165 microseconds per row: 16x more chunks, one blob
open each), while CS64's rerank saving is a constant ~81 ms. Extrapolated,
the two meet near 1.2M rows (about 240-255 ms each at 1M). 1M was not
measured: the first BASE seed passed the 10 GB budget (4.0 GB store plus a
6.8 GB WAL after 18 minutes) and was stopped (PREREG amendment 1). Seeding
is also much faster under CS64 (300k: 36 s vs 133 s). The 100k stage C runs
all ended above load 2; their p50s agree within 3% with the traced runs.

### 6. Blast radius

| Candidate | Code | Schema / storage | ADRs and docs | Tests | Invariants it relies on |
| --- | --- | --- | --- | --- | --- |
| Eligibility answers cached by read-visibility generation (ELIG) | `dependency_closure.rs` (two functions), one cache module | None: reuses `_fathomdb_read_visibility_state` and its triggers | ADR-0.8.25-eligibility-and-frozen-reads (its generation becomes a cache key for ordinary reads; its "ordinary reads keep the legacy cost" consequence changes); Slice 71 design | Cache hit/miss and staleness after supersession, state change, window insert, vector-row and edge mutation; concurrent writer + reader snapshots | Every insert/update/delete of `canonical_nodes`, `canonical_edges`, `_fathomdb_vector_rows` bumps the generation (Slice 35's mutation-manifest audit); any table rebuild must keep those triggers. Answers cached only when no validity window or `t_invalid` exists |
| Same, maintained counters instead of a cache | Triggers maintain counts of superseded / inactive / windowed nodes, invalid edges and orphan vector rows; reads become O(1) even right after a write | New table + triggers, SCHEMA_VERSION bump; fresh stores only, or a one-time count | Same ADRs | Trigger correctness under every write path, erasure, rebuild | Counter = predicate count, transactionally |
| FTS `rowid = write_cursor` | `index_projector.rs` insert; every other `search_index` insert path (FTS rebuild, restore, re-projection) must keep it | Storage invariant, no DDL; fresh stores only (owner: no compatibility needed); a re-insert of an existing cursor becomes a constraint error | FTS layout note (successor to the Slice 71 design) | Rowid identity after every write/rebuild path | One body-FTS row per cursor |
| Deferred-content text arm (FTS) | `search.rs` deferred route and a ranked-fusion variant that must stay identical to `fuse_three_arms` (a product version should share its scoring code) | Needs the rowid identity | Slice 71 design amendment: the complete ranking is computed, not materialised | Slice 71/75 equivalence fixtures (deep overlap, duplicates, ownerless rows); route witnesses renamed | Equal bodies tokenise identically, so they share one bm25 value; rowid = cursor (checked per fetched row, else fallback) |
| CS64 | (from `../ac013-vector-reads/`) | vec0 `chunk_size=64`, fresh stores | ADR-0.7.0-vector-binary-quant successor | as there | Bit scan cost grows 1.4x per row; meets BASE near 1.2M |

The public API, bindings, MCP surface and results stay unchanged under all
of them.

### 7. Recommendation

1. Keep CS64 as the vector fix for AC-013 and stores up to ~1M rows; above
   that, plan the rerank fix that does not multiply chunks (F32T or a
   separate f32 store), since CS64's bit scan then costs more than it saves.
2. Take the eligibility fix next: it is pure engine code, results are
   exact, and on realistic text it halves 100k search time (283 to 149 ms).
   Ship it as maintained counters if write-then-search is the normal agent
   pattern (the cache misses after every write and then pays an extra O(n)
   window check); the generation cache is enough for read-mostly stores.
3. Take the deferred-content text arm with `rowid = write_cursor` for new
   stores: exact, -32 to -35% at 100k; with (2), 54 ms at LME 100k.
4. Put to the owner, separately: the vector arm is off in any store that
   has superseded a node (stage D). Pruning a superseded node's vector row,
   as edges already do, or making vector eligibility native, would restore
   hybrid retrieval for re-ingesting stores; it is a Slice 35 decision, not
   a latency one.
5. What remains is scoring every content-OR match (SYN 100k: ~300 ms;
   LME 100k: tens of ms). Removing it needs a top-k bm25 bound (WAND-style,
   outside FTS5) or a query-side term policy, both of which change
   behaviour and need their own experiment.

### 8. Not done

- MCP 1-in-flight at 100k: the server opens only stores embedded with the
  pinned default embedder, and a 100k bge store takes about 5 hours to seed
  on CPU (30 minutes at 10k); a CUDA build was out of scope here. The
  probe's sequential loop is the engine half of that measurement.
- The vector stage at 1M (over the disk budget, above).
- A write-interleaved workload (write, then search) for the eligibility
  cache; and product-grade versions of the prototypes (the route witnesses,
  the shared scoring code, every FTS insert path).
