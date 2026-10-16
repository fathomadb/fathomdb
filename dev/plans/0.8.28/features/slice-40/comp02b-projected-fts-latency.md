---
title: "COMP-02b projected-FTS latency diagnosis"
slice: 40
release: 0.8.28
date: 2026-10-16
status: diagnosed, not fixed
---

# COMP-02b projected-FTS latency diagnosis

COMP-02b (`dev/performance-benchmarking/2026-10-15-comp-02b-filtered-fts-result.md`
in the comp-01 worktree) found `search_projected_text` at 45–52 ms p50 on
the 10k TC-5 corpus against SurrealDB's 4–74 ms, and showed the attribute
filter is not the cost: the unfiltered projected search is slower than
the filtered one, and `search_text_only` on the same database is about
6× faster. This note pins the mechanism.

## Reproduction

One fresh 10k projected database built with the Slice 40 wheel, the 100
TC-5 queries, warm pass then a seeded shuffle, engine calls from Python:

| Engine call | p50 (ms) | p95 (ms) |
| --- | ---: | ---: |
| `search_projected_text`, no filter | 82.0 | 105.1 |
| `search_projected_text`, filter `c01` | 65.6 | 86.6 |
| `search_projected_text`, filter `c33` | 76.7 | 100.9 |
| `search_text_only`, same database | 13.9 | 20.5 |
| `search_projected_text`, single term `meeting` | 26.7 | 27.8 |

The median query compiles to 8 OR terms and matches 3,806 of 10,000
property rows (min 1,245, max 6,176).

## Decomposition

The statement `search_projected_text` runs is `property_fts_rank_sql`
(`src/rust/crates/fathomdb-engine/src/filter.rs`): FTS5 match joined to
`canonical_nodes`, the full `FrozenView::node_sql` predicate (existence,
validity, and both dependency-closure `NOT EXISTS` subqueries), selecting
`n.body`, `ORDER BY bm25(property_search_index), write_cursor`, with no
`LIMIT`. The Rust loop stops at `limit`, but SQLite has already sorted
every joined row. Raw sqlite3 timings of that statement and variants on
the same database, same queries:

| Variant | p50 (ms) |
| --- | ---: |
| V0 engine statement as shipped | 52.0 |
| V0 + attribute filter `c01` | 25.6 |
| V0 + `LIMIT 10` | 23.3 |
| V0 without the two `NOT EXISTS` subqueries | 46.6 |
| V0 without `n.body` in the select list | 25.4 |
| Full predicate, body deferred to an outer join, `ORDER BY bm25, wc LIMIT 10` | 23.5 |
| Bare FTS, no join, `ORDER BY bm25` | 14.2 |
| **V8: full predicate, `rank MATCH 'bm25()' ORDER BY rank LIMIT 10`** | **5.2** |
| V8 on the single term `meeting` (833 matches) | 0.8 |

Reading the table:

- The dependency-closure subqueries cost about 6 ms. They are evaluated
  per matched row, but the tables they probe are empty here and every
  probe is an indexed miss. Not the cause.
- About 27 ms is carrying 3,800 node bodies (1.6 KB JSON each) through
  the temporary ORDER BY b-tree. The attribute filter is faster only
  because it shrinks the rows the sorter holds.
- About 14 ms is the `bm25()` auxiliary function and the external sort
  on the bare match set.
- The FTS5 rank stream (`rank MATCH 'bm25()' ORDER BY rank`) removes
  both: FTS5 orders internally and the join, predicate and body read
  happen only for rows until the limit fills. `EXPLAIN QUERY PLAN` shows
  no temporary b-tree for it.

The single-term engine call (26.7 ms) and the single-term raw statement
(25.6 ms) agree, so there is no fixed per-call overhead to chase; the
whole gap is the unbounded sort.

## Cause

This is the same defect the body path closed in SCALE-02
(`2026-08-22-scale-02-rank-boundary-result.md`, HITL `seq-268`,
"stream_default"): the shipped fallback sorted every candidate with its
body; the streamed boundary-tie completion reads in rank order and stops
after the limit's tie group is complete. `read_search_in_tx` carries that
path (`rank_sql` plus `collect_complete_rank_boundary`) and keeps the
full sort only as a fallback. `read_projected_text_in_tx`
(`src/rust/crates/fathomdb-engine/src/search.rs`) was written against the
pre-SCALE-02 shape and never received the streamed path.

Secondary: Slice 40 now keeps 1–2 character tokens, so `"what's"` yields
the content token `"s"`, which matches 3,912 of 10,000 documents. Half the
TC-5 queries carry such a fragment, and they inflate the median candidate
set from 2,744 to 3,806 rows. That is the STOPWORDS fragment follow-up in
`status.md`; it is a multiplier on this defect, not its cause.

## Proposed fix shape (not implemented)

Mirror the body path in `read_projected_text_in_tx`: run
`property_fts_rank_sql` with `AND rank MATCH 'bm25()' ORDER BY rank`,
collect through `collect_complete_rank_boundary(limit)` so the documented
`bm25 ascending then write_cursor ascending` order survives ties, and
fall back to the current full-sort statement when the streamed statement
fails to prepare. Expected on this corpus: about 5 ms p50 unfiltered,
below SurrealDB at every cohort selectivity measured in COMP-02b. The
`LIMIT` must come from the validated `limit` argument, never a fixed
window, so the limit-near-100 shortfall recorded for the body path is not
copied.

Tests to write first: a rank-stream witness for the projected statement
(no temporary ORDER BY b-tree), ordered-result equality against the
forced full sort on a corpus with boundary ties, and the existing
projected-search contract tests unchanged.

Scripts: `build_and_time.py` and `decompose.py` in the session scratchpad
under `comp02b-diag/`; inputs `~/.cache/fathomdb-comp/tc5-10000`.
