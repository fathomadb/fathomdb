# SCALE-02 rank-boundary off-shoot design

## Hypothesis

The growing full-sort share is caused by BM25 ties at the rank-fast candidate
boundary. For direct text searches without filters or edges, consuming FTS5's
native rank stream only until the score after the boundary changes, then
applying the existing `(score, write_cursor)` order to that bounded prefix,
will preserve exact retrieval while avoiding the full-match sort.

The experiment has two factors:

1. boundary handling: shipped full-sort fallback or streamed boundary-tie
   completion;
2. reader profile: shipped defaults or the previously qualified 128 MiB mmap
   profile.

The reader factor is included because it is the only prior treatment that
passed the scale-adjusted policy at 40k. It tests interaction with the new
mechanism; it is not another reader sweep.

## Local basis

Read-only analysis used the stored first-repetition databases and fixed 100
TC-5 queries behind the 25k, 40k, and 50k receipts. A query was predicted to
fall back exactly when rows 100 and 101 from `ORDER BY rank` had equal scores.
The prediction matched the recorded route for all 300 query-point pairs.

| Records | Rank-fast | Full-sort | Rows needed for a complete boundary tie, median / p95 / max | Full matches on fallback queries, median / p95 / max |
| ---: | ---: | ---: | ---: | ---: |
| 25,000 | 80 | 20 | 106 / 110 / 112 | 3,265 / 6,738 / 7,261 |
| 40,000 | 59 | 41 | 103 / 108 / 109 | 7,644 / 12,347 / 21,921 |
| 50,000 | 42 | 58 | 102 / 114 / 120 | 10,697 / 19,861 / 27,322 |

The derived scale fixture duplicates source bodies while adding unique row
markers. It adds 7,728, 22,728, and 32,728 rows at the three points, so more
equal BM25 scores at the top-100 boundary are expected. This is part of the
registered efficiency workload and must not be removed to make the result
pass.

On the stored 50k database, `EXPLAIN QUERY PLAN` for `ORDER BY rank` used the
FTS5 virtual-table scan without a temporary sort. Adding `write_cursor` as a
second SQL sort key, or using `ORDER BY bm25(...), write_cursor`, emitted
`USE TEMP B-TREE FOR ORDER BY`. The third off-shoot therefore keeps the native
rank order and resolves only its boundary tie in Rust.

This analysis is a design input, not a registered performance result. The
executable run must reproduce its route, tie-size, and query-plan witnesses.

## Research basis

- SQLite documents that FTS5's hidden `rank` is BM25 by default and that
  `ORDER BY rank` is faster when a caller stops consuming rows early or uses a
  `LIMIT`. That directly supports early abandonment; applying it to exact
  boundary-tie completion is the FathomDB hypothesis.
  [SQLite FTS5](https://www.sqlite.org/fts5.html#sorting_by_auxiliary_function_results)
- SQLite's planner documentation describes the same general shape as block
  sorting: preserve an indexed leading order, sort only equal-key groups, and
  stop early under a limit. It also explains why a full sort grows with all
  matching rows.
  [SQLite query planner](https://www.sqlite.org/queryplanner.html#partial_sorting_using_an_index_a_k_a_block_sorting)
- SQLite defines `USE TEMP B-TREE FOR ORDER BY` as evidence of an explicit
  sort. The run records this semantic marker under the pinned SQLite version;
  it does not compare unstable display formatting.
  [SQLite EXPLAIN QUERY PLAN](https://www.sqlite.org/eqp.html#temporary_sorting_b_trees)
- An SQLite virtual-table report demonstrates that adding a secondary order
  can force a whole temporary sort instead of sorting only the tied suffix.
  That agrees with the local FTS5 plans and rules out compound SQL ordering as
  the primary treatment.
  [SQLite virtual-table discussion](https://sqlite.org/forum/info/4400a8d215ae8ae4)
- FTS5 permits custom ranking functions, but an SQLite maintainer notes that
  reading table data from one adds lookups and that efficient `ORDER BY rank`
  requires constant trailing arguments. A custom rank embedding
  `write_cursor` is therefore a larger, less direct alternative.
  [SQLite custom-ranking discussion](https://www.sqlite.org/forum/forumpost/d5cfe2565bf04801)
- FTS5 `optimize` merges index b-trees into their fastest query form. It may
  address fragmented long-lived indexes, but these are fresh databases and
  the observed route is predicted exactly by boundary ties, so it is not a
  factor in this run.
  [SQLite FTS5 optimize](https://www.sqlite.org/fts5.html#the_optimize_command)

## Alternatives excluded

- A fixed 128-row probe would cover the observed maximum of 120 rows, but that
  threshold is learned from these inputs and creates a new fallback cliff.
- `ORDER BY rank, write_cursor` performs a full temporary sort in the deployed
  query shape.
- Adding a small cursor-dependent epsilon to BM25 can reorder unequal nearby
  scores and cannot satisfy exact equivalence.
- Custom FTS5 ranking, query-term narrowing, tokenizer changes, corpus
  deduplication, and FTS index maintenance change more than the diagnosed
  mechanism. They require separate evidence if this hypothesis fails.

## Assessment of general FTS5 tuning advice

An online recommendation for 100k-row FTS5 deployments was checked against
the shipped schema and prior SCALE-02 evidence. Its latency claim is not
portable to this program: it does not bind corpus duplication, query
selectivity, cold versus steady state, tail latency, hardware, concurrency, or
the result-equivalence contract.

- **External content:** FathomDB's `search_index` is currently content-storing,
  so this is not already enabled. In the stored 50k rep-1 database, SQLite
  `dbstat` attributes 97,378,304 bytes to `search_index_content` out of a
  385,273,856-byte database. External content could therefore be a meaningful
  storage hypothesis. SQLite also makes the application responsible for
  index/content consistency and may query the canonical table for required
  column values. For FathomDB it would require a schema migration, changes to
  projection and erasure behavior, and independent read/write measurements.
  It is not a treatment for the diagnosed boundary sort.
  [SQLite external-content tables](https://www.sqlite.org/fts5.html#external_content_and_contentless_tables)
- **Porter stemming:** already shipped as
  `porter unicode61 remove_diacritics 2`; no change is available to test.
- **Weighted BM25:** the current primary index has one indexed `body` column;
  `kind` and `write_cursor` are unindexed. Field weighting therefore requires
  a new multi-column index and changes relevance. The existing
  [`ADR-0.8.1`](../adr/ADR-0.8.1-deferred-f5-fielded-fts-bm25f.md) places that
  behind retrieval-quality evidence, so it cannot enter an efficiency-only
  off-shoot.
- **Snippets:** presentation output neither reduces the fallback share nor
  belongs in the registered retrieval response.
- **WAL and synchronous mode:** WAL is already a FathomDB invariant and is
  present in the stored database. The durability ADR declares
  `synchronous=NORMAL`, but prior SCALE-02 receipts did not capture the
  connection's observed synchronous value. Preflight must record both values
  and stop on an ADR/runtime mismatch; synchronous mode is held constant, not
  made an experimental factor.
- **Very large mmap:** mmap is already the second factor, using the qualified
  128 MiB setting. SQLite treats `mmap_size` as a ceiling subject to build
  limits, not a promise that an arbitrary 30 GB value is suitable. Prior
  SCALE-02 evidence also showed roughly 1 GiB higher process RSS with mmap128,
  so a 30 GB request conflicts with the program's footprint objective and is
  not justified by a larger blind setting.
- **`temp_store=MEMORY`:** it may move temporary sorts into memory, but the
  candidate is intended to remove the temporary full sort. Adding it would
  confound the mechanism and spend RAM on the control's failure mode.
- **FTS `optimize`:** SQLite says it merges the component b-trees into the
  fastest query form. That is relevant to long-lived, mutation-fragmented
  indexes. SCALE-02 builds fresh databases and the control routes are exactly
  predicted by boundary ties, so optimize is reserved for a separate aging or
  maintenance experiment.

## Implementation qualification

Write failing tests before changing the engine. The experiment treatment must:

- use one `ORDER BY rank` statement and consume through the first row whose
  score differs from the row-100 score;
- retain rows above the boundary plus the complete boundary-score group, sort
  them by the existing `(score, write_cursor)` contract, and truncate to the
  existing 100-candidate window;
- leave filtered, edge-fusion, non-text, and shorter-result paths unchanged;
- expose an experiment-only selector so the same binary can run both boundary
  levels; no behavior is a production default merely because the run passes;
- fall back to the shipped exact path on statement or conversion failure;
- emit route, rows consumed, and boundary-group size without content.

Tests cover fewer than 100 matches, no boundary tie, ties extending beyond
row 101, all-equal scores, unequal adjacent floating-point scores, stable
cursor order, limit prefixes, validity, supersession, ownership, filters, and
edges. Property tests compare complete ordered candidate signatures against
the shipped full-sort path across generated tie-group shapes.

A dry-run must also show that the streamed statement has no temporary ORDER BY
b-tree and reproduce the 20/41/58 control fallback counts on the stored query
set. It records observed `journal_mode` and `synchronous` on writer and reader
connections and reconciles them with the accepted durability ADR before any
timed cell. Any mismatch stops the run for review.

## Run matrix

The proposed executable contract is
[`rank-boundary.v1.proposed.json`](../../experiments/configs/scale-02/rank-boundary.v1.proposed.json).
It is pending HITL authorization and implementation digests.

| Boundary handling | Reader profile | Purpose |
| --- | --- | --- |
| shipped fallback | shipped defaults | paired production-footprint control |
| streamed tie completion | shipped defaults | isolates the algorithm effect |
| shipped fallback | 128 MiB mmap | reproduces the prior reader effect |
| streamed tie completion | 128 MiB mmap | measures interaction and the best plausible 50k cell |

Run all four cells at 25k, 40k, and 50k with five fresh databases per cell:
60 databases total. Balance treatment order by the registered seed. Preserve
the existing corpus construction, 100-query order, 100 cold queries, 100
warm-up queries, 1,000 steady queries, 20 mutations, concurrency one, and
scale-adjusted 25/40/50 ms p50 plus 150 ms p99 policy. Do not combine cache64,
GPU, a reranker, or another query treatment.

For every point and reader level, compare the streamed and control paths on
all 100 queries. Require exact ordered top-100 internal candidate signatures
and exact public top-10 signatures, including score bits, identifiers, cursor
order, and content-free payload hashes. Require zero errors and timeouts,
complete repetitions, observed reader settings, and less than 80% host RAM.

Report cold and steady p50/p95/p99, throughput, peak RSS, effective CPU,
database and derived-index bytes, full-sort share, rows consumed to complete
each boundary, boundary-group sizes, and repetition-bootstrap 95% uncertainty.
The pass rule uses the existing pooled point estimates; uncertainty remains
reported rather than silently changing the policy.

## Decision rule

The mechanism is supported only if streamed tie completion has exact retrieval
and zero full-sort fallbacks for every eligible query at all three points. It
must also avoid more than 10% p50 or p99 regression against its paired reader
control at 25k.

Choose the lowest-footprint eligible result:

1. If streamed completion with shipped readers passes through 50k, recommend
   that engine change for production review and keep shipped reader defaults.
2. If only streamed completion plus mmap passes at 50k, report the 50k result
   as a combined, higher-memory envelope. Do not land mmap without a separate
   HITL decision.
3. If streaming removes the fallbacks but 50k still fails, conclude that
   fallback growth is not sufficient to explain the 50k breach. The next plan
   may inspect native-rank scan cost or broad OR-match volume, but may not
   narrow retrieval without answer-quality evidence.
4. If equivalence, query-plan, provenance, resource, or completeness checks
   fail, reject the treatment and make no scale claim.

This off-shoot does not revise the original fixed-20-ms SCALE-02 envelope. A
passing result may extend only the separately approved scale-adjusted advisory
view, and production landing requires a post-result HITL decision.

## Evidence and authorization boundary

Store configurations, resolved code and binary digests, query-plan witnesses,
query-level content-free diagnostics, timings, databases, and artifact
manifests under the existing external SCALE-02 artifact root. Commit only the
safe aggregate receipt and result note under `experiments/runs/` and
`dev/performance-benchmarking/`.

The current work authorizes analysis and run design, not implementation or
execution. Before either begins, replace the proposed configuration with an
approved immutable configuration that binds the candidate source commit,
binary digests, artifact root, and HITL decision reference.
