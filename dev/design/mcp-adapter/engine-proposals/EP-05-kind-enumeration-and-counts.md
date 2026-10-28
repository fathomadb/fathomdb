---
title: EP-05 — Kind enumeration and per-kind counts
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. A read that lists the canonical node kinds present in a store with their visible counts, cached by read-visibility generation, for orientation and for "did you mean" candidates.
---

# EP-05 — Kind enumeration and per-kind counts

Status: Draft. Priority P2. Not an accepted decision.

## Problem

- No SDK read lists kinds. `read::list` and `read::canonical_page` both need a
  kind as input (`fathomdb-sdk/src/read.rs:62`, `read.rs:82-91`). The projection
  reads return projection names, `vector_unsupported_kinds` and
  `affected_kinds` (`read.rs:127-153`), not the kinds stored.
- `orient` reports `kinds` and `counts` as `unavailable`
  (`src/rust/crates/fathomdb-mcp/src/tools.rs:426-427`). MR-054.1 ("did you mean
  `note`") cannot name kind candidates (interface §6).
- The data is there: `canonical_nodes.kind` is indexed
  (`fathomdb-schema/src/lib.rs:374-375`).

There is no adapter workaround. The signal is reported `unavailable`.

## Proposed surface

`fathomdb_sdk::read::kinds(engine, options: KindsOptions) -> Result<KindsV1>`.

```rust
#[derive(Default)]
pub struct KindsOptions {
    pub view: Option<ReadView>,   // default view: active, current, valid now
    pub limit: usize,             // 1..=1000, default 200
}
#[non_exhaustive]
pub struct KindsV1 {
    pub kinds: Vec<KindCountV1>,  // sorted by kind, UTF-8 byte order
    pub omitted: u64,             // kinds beyond `limit`
    pub generation: u64,          // visibility generation the counts are for
}
pub struct KindCountV1 { pub kind: String, pub nodes: u64 }
```

- Counts are logical nodes visible under `view`, with the same predicates as
  `read.list` (`ReadView::node_sql`).
- `row_kind` and edge kinds are out of v1. Edge kinds can follow in the same
  shape.
- Errors: `InvalidArgument` for a `limit` out of range. Otherwise `Closing` and
  `Storage`, as for other reads.

## Invariants and consistency

- One reader snapshot. `generation` is read in the same transaction, so the
  caller can tie counts to the EP-01 state.
- Deterministic order and omission count for a given state and view.
- Under the default view, the kind set includes only kinds with at least one
  visible node, so a kind whose last node was retracted disappears.

## Interaction with ADRs

- A new governed read in all three SDKs, under
  `ADR-0.8.27-rust-sdk-parity.md`, with the allowlist sign-off of
  `ADR-0.8.0-supersede-five-verb-surface-cap.md`.
- Kinds are writer-set strings. MR-012.2 relies on that to echo them in errors.
  No ADR states that kinds are trusted names, so the adopting MCP ADR should say
  so.

## Cost and performance

- A `GROUP BY kind` with the view predicates reads every candidate row. The
  `kind` index helps the grouping but not the `superseded_at` and `state`
  predicates, so the cost is O(rows).
- Precedent for keeping that off the hot path: projection status is cached by
  visibility generation and `PRAGMA data_version`
  (`fathomdb-engine/src/projection_generation.rs:1451-1467`). The same cache key
  makes repeated orientation calls O(1) until a commit.
- Validity filtering (`valid_as_of`) defeats a generation-only cache, because
  results change with time. The cache would be keyed by the resolved instant
  too, or used only for the default view's `valid_as_of = now` rounded to the
  boundary. Not designed here.

## Test plan sketch

- Real database: three kinds with 2, 1 and 0 visible nodes (the 0 one
  superseded). Default view lists two kinds with exact counts.
  `include_superseded` lists three.
- `limit=1` returns the first kind by byte order and `omitted=1`.
- Cache test: two calls with no commit run one scan, and a commit invalidates
  it.
- Same-snapshot test with a concurrent writer: `generation` matches the counts.

## Adapter change once it lands

`orient` fills `kinds` and `counts` within its token cap (MR-022.2), and reports
`omitted`. `invalid_request` errors for unknown kinds list the nearest kinds by
edit distance, drawn only from this enumeration (MR-054.1, MR-054.3).

## Open questions for the owner

1. Are per-origin-class counts (MR-022) wanted in v1? They need EP-10's origin
   field first.
2. Time bounds (MR-022): `canonical_nodes` has `valid_from` and `valid_until`
   (`lib.rs:704-705`) but I found no creation-time column. Which instant should
   "time range" mean?
3. Is O(rows) on first call acceptable at the 1–2M scale, or should counts be
   maintained incrementally? Incremental counts are a write-path change.
