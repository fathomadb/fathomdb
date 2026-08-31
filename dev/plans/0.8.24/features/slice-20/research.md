---
title: 0.8.24 Slice 20 — focused implementation research
status: COMPLETE
target_release: 0.8.24
---

# Slice 20 — focused implementation research

## Questions and primary sources

| Question | Primary source | Conclusion applied to the design |
| --- | --- | --- |
| Is `ORDER BY rank` equivalent to default `bm25()` ordering, and why can it stop early? | [SQLite FTS5 — sorting by auxiliary-function results](https://www.sqlite.org/fts5.html#sorting_by_auxiliary_function_results) | The hidden `rank` column maps to no-argument `bm25()` by default. SQLite explicitly states that `ORDER BY rank` is faster than `ORDER BY bm25(ft)`, especially when the caller abandons the query early or uses a limit. Because the persistent mapping is configurable, the statement pins `rank MATCH 'bm25()'` per query. |
| Which direction is better BM25? | [SQLite FTS5 — the bm25 function](https://www.sqlite.org/fts5.html#the_bm25_function) | FTS5 negates BM25 so numerically smaller values are better. Ascending rank/score is correct. |
| May equal-rank rows be assumed to have stable cursor order? | [SQLite SELECT — ORDER BY](https://www.sqlite.org/lang_select.html#orderby) and the FTS5 rank documentation | No secondary order is promised by `ORDER BY rank`. The implementation must consume the complete score group at the fixed boundary and restore the product's `write_cursor` tiebreak in Rust. |
| How do statement and row failures surface while streaming? | [rusqlite 0.40 `Rows`](https://docs.rs/rusqlite/0.40.0/rusqlite/struct.Rows.html) and [`Row`](https://docs.rs/rusqlite/0.40.0/rusqlite/struct.Row.html) | `Rows::next()` is a fallible streaming step and `Row::get()` is fallible conversion. Both errors can be propagated from the collector, causing the caller to discard partial streamed candidates and run the existing full-sort path. |

## Repository and version fit

- The workspace pins `rusqlite = 0.40` with bundled SQLite and
  `fallible_uint`; the researched API matches the selected dependency.
- The retained receipt observed bundled SQLite 3.53.2 and no temporary ORDER
  BY B-tree for the measured statement. Slice 20 verifies the plan shape as a
  correctness/algorithm check, not a timing run.
- Current schema does not customize FTS rank mapping, but a database can do so
  independently. The design therefore does not trust ambient state and pins
  the mapping in the eligible statement.
- No online competitive or benchmark research is relevant: owner selection is
  closed and will not be reopened.

## Challenging aspects resolved

1. **Native rank does not own tie determinism.** Complete the score group that
   crosses row 100, then stable-sort only the bounded candidates by the product
   key.
2. **A failed stream may already have partial rows.** Keep streamed candidates
   local to a fallible operation and publish them only on complete success;
   otherwise discard and execute the untouched exact query.
3. **An optimization must not change fusion inputs.** Restrict eligibility to
   the existing direct-text fixed-100 node window and keep filters/edges on the
   existing path.
4. **The test oracle must not become a runtime API.** Compile force-full-sort
   and route evidence only under the private `test-hooks` feature.
