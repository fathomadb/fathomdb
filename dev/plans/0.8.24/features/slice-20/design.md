---
title: 0.8.24 Slice 20 — streamed FTS rank-boundary design
status: ACCEPTED-AFTER-REVISION
target_release: 0.8.24
---

# Slice 20 — streamed FTS rank-boundary design

## Decision

Implement the selected `stream_default` behavior as a narrow alternative node
collector inside `read_search_in_tx`. It changes how eligible direct-text node
candidates are obtained, not which candidates, scores, identities, or public
results are returned.

## Exists today versus net-new

| Concern | Current implementation | Slice 20 change |
| --- | --- | --- |
| Query compilation | Safe FTS grammar produces `match_expression`. | None. |
| Direct-text window | Direct-text APIs pass a fixed node candidate limit of 100 for every public limit. | Reuse this as the stream boundary. |
| Exact order | SQL orders by `bm25(search_index), write_cursor` and limits to 100. | Eligible path pins `rank MATCH 'bm25()'`, orders by FTS5 `rank`, completes the score group crossing 100, then restores the same stable key in Rust. |
| Validity/existence | The joined query enforces active, non-superseded, valid nodes and preserves ownerless legacy rows. | Statement text is identical except for order/limit; no predicate change. |
| Failure behavior | Join preparation falls through to older-schema source/plain variants; row mapping skips malformed rows. | Failed stream discards partial candidates and re-enters the current joined full-sort path; existing legacy fallbacks remain after it. |
| Filters/edges | Metadata post-filter and separate edge-body collection/fusion. | Any filter or existing edge-body FTS row makes stream ineligible. |
| Fusion/public limit | Deterministic dedup/RRF and final caller-limit truncation. | None. |
| Connection settings | Current main sets writer WAL but leaves writer `synchronous` ambient despite the accepted NORMAL contract. | Explicitly apply `synchronous=NORMAL` to the writer only; leave readers, runtime connections, cache, mmap, temp-store, and pool behavior unchanged. |

## Eligibility

The streamed collector is requested only when
`direct_text_candidate_limit.is_some()` and no private forced-control is active.
It is eligible only when:

- `filter.is_none()`; and
- `search_index_edges` contains no row.

The existing direct-text API guarantees the candidate limit is 100. Hybrid and
vector-soft-fallback paths do not provide `direct_text_candidate_limit` and
therefore cannot select the optimization. A failed edge-presence probe is
ineligible/fail-closed.

## Stream algorithm

1. Prepare the same current joined node SELECT and predicates, replacing only
   `ORDER BY bm25(...), write_cursor LIMIT 100` with
   `AND rank MATCH 'bm25()' ORDER BY rank`. The per-query mapping prevents a
   persistent database rank configuration from changing semantics.
2. Read rows fallibly in native rank order.
3. Retain the first 100 rows and the score of row 100.
4. Continue while subsequent rows have exactly the same `f64::total_cmp`
   score. Reading the first different score proves the boundary group ended;
   that row is not retained.
5. Sort retained candidates by ascending score then ascending `write_cursor`.
6. Truncate to 100 and pass candidates to the unchanged filter/edge/fusion and
   public truncation stages.

This may consume 101 rows for a strict boundary, or more when ties cross it.
The all-equal worst case is intentionally a full scan because exact stable
top-100 results require seeing every equal-score cursor.

## Failure and fallback

Statement preparation, `Rows::next`, and every `Row::get` propagate through one
fallible streamed operation. Candidates are returned only if that operation
finishes successfully. On any error:

- discard all partial streamed candidates;
- execute the untouched current joined full-sort query;
- retain its existing `rows.flatten()` malformed-row treatment; and
- retain its existing old-schema source/plain query chain if join preparation
  fails.

There is no retry loop and no mixed streamed/full-sort output.

## Proof design and TDD

The RED contract must fail against current main because the current query
cannot emit a private `rank_stream_*` route witness. The test then compares the
production path with a private forced-full-sort oracle on the same immutable
database. Additional tests cover:

- strict boundary and exact complete-result equality;
- 101+ equal-score rows crossing the fixed boundary;
- malformed streamed-row conversion and stable fallback;
- filter/edge ineligibility;
- no temporary ORDER-BY B-tree for the stream statement;
- generated score/tie groups against a complete stable sort; and
- the existing direct-text small/large prefix, fixed-validity, hybrid-fallback,
  and legacy-schema suites.

The force and route seams compile only with `test-hooks`; normal artifacts do
not read their environment variables or expose route telemetry.

## Architectural fit

The change stays inside the retrieval match stage and reader transaction owned
by `dev/architecture.md`. It preserves the fixed pipeline, safe query compiler,
multi-reader model, same-file FTS projection, RRF fusion, and public SDK
interfaces. `dev/design/retrieval.md` receives the internal collector rule so
the design no longer leaves implementation behavior implicit. No ADR successor,
schema change, or public-interface update is needed.

## Durability invariant

Immediately after the writer connection selects WAL, it applies
`synchronous=NORMAL`. This is not performance tuning: it restores the accepted
ADR-0.6.0 writer durability profile which current-main documentation already
claims. A narrow private writer witness verifies `journal_mode = wal` and
`synchronous = 1`. Reader-pool and runtime connection code is untouched. Normal
builds compile neither the witness environment lookup nor file emission.

## Excluded adjacent experiment behavior

- no mmap/cache/temp-store tuning and no journal-mode change beyond the
  existing WAL selection;
- no writer/reader connection witness;
- no shipped route, boundary-size, or query-plan JSONL witness;
- no experiment matrix/config/runner import;
- no TC-5 or unrelated engine/test changes; and
- no performance number or capacity promise beyond the retained evidence.

## Reviewer disposition

The independent reviewer returned **NEEDS-REVISION** and approved the core
architecture subject to bounded mandatory corrections. This revision:

- reconstructs only final FTS behavior on current main and explicitly retains
  the newer Windows WAL-attribution work;
- excludes TC-5 and experiment infrastructure;
- restores and tests the accepted writer `WAL + synchronous=NORMAL` invariant
  without changing reader/runtime connections;
- pins `rank` to `bm25()` against ambient persistent configuration;
- keeps controls and witnesses private to `test-hooks`;
- updates AC-076's stale mechanism explanation without a new threshold; and
- expands proof across ties, failures, filters, edges, fixed validity, hybrid
  fallback, legacy schema, query plan, and durability.

Any mismatch requiring a schema/SDK/public-contract change halts for HITL. No
benchmark rerun is permitted. With those corrections incorporated, the design
is accepted for implementation.
