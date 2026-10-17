---
title: EP-06 — Lifecycle and validity fields on NodeRecord and SearchHit
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. Return each record's lifecycle state, supersession instant and validity window on point reads, list reads and search hits, so callers can tell a current record from a superseded, retracted or expired one without a second query.
---

# EP-06 — Lifecycle and validity fields on NodeRecord and SearchHit

Status: Draft. Priority P2. Not an accepted decision.

## Problem

- `NodeRecord` is `{logical_id, kind, body, write_cursor}`
  (`fathomdb-engine/src/read.rs:25-30`). `SearchHit` is
  `{id, write_cursor, kind, body, score, branch, source_id, ce_score}`
  (`search_types.rs:76-106`). Neither carries currency.
- The columns exist on `canonical_nodes`: `superseded_at`
  (`fathomdb-schema/src/lib.rs:367`), `state` and `reason` (`lib.rs:584-585`),
  `valid_from` and `valid_until` (`lib.rs:704-705`).
- Point reads honour the relaxed views. `read_get_by_id_in_tx` appends
  `view.node_sql(...)` and resolves a slot to the highest `write_cursor`
  (`read.rs:96-108`). So `fetch` with `include_superseded` or `include_inactive`
  can return a superseded or deleted row, and the caller cannot tell it is one.
- Search refuses both relaxations (`temporal.rs:146-160`), because projection
  indexes are not version-complete. Search hits are current by construction,
  but their validity window is still not reported.

Two documentation corrections found while verifying:

- The doc comment on `read_get_by_id_in_tx` says "superseded versions are never
  returned" (`read.rs:68-72`). The code below it honours `include_superseded`.
  The comment is stale.
- `dev/design/mcp-adapter/design.md` §1 says `read.get` "reads only active
  rows". That holds only under the default view.

There is no adapter workaround. MR-013 is reported `unavailable`. The adapter's
`fetch` passes the relaxed view through
(`src/rust/crates/fathomdb-mcp/src/tools.rs:878-883`), but returns no state.

## Proposed surface

Add one shared, additive struct, changed in place on existing types:

```rust
#[non_exhaustive]
pub struct RecordCurrencyV1 {
    pub state: LifecycleState,        // Active, Pending, Deleted, Purged
    pub superseded_at: Option<i64>,   // epoch seconds; None = current version
    pub valid_from: Option<i64>,
    pub valid_until: Option<i64>,
}
```

- `NodeRecord.currency: RecordCurrencyV1` on `read.get`, `read.get_many`,
  `read.list`, `read.canonical_page` and `graph.neighbors`.
- `SearchHit.currency: Option<RecordCurrencyV1>`: `Some` for `l:` hits, read in
  the same hydration query that already joins `canonical_nodes`
  (`search.rs:1854-1862`), and `None` for `h:` and edge-fact hits where no
  single node row applies.
- `GraphTargetV1` gains the same field. That is an in-place V1 change.
- No new errors.

## Invariants and consistency

- Values come from the same row and snapshot as `body`, so they cannot
  disagree with the content returned.
- `superseded_at.is_some()` implies the view had `include_superseded`. Under the
  default view, every record reports `Active`, `superseded_at: None` and a
  window that contains the effective instant. A test asserts that.
- The `state` column has no `CHECK` constraint (`lib.rs:583-586`). `ReadView`
  documents `pending`, `deleted` and `purged` (`temporal.rs:45-48`). I did not
  verify which values the writers set, so the enum needs either a closed list
  checked against every writer, or an `Other(String)` arm.

## Interaction with ADRs

- `ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md`: an in-place
  change of `NodeRecord`, `SearchHit` and `GraphTargetV1`, with no V2.
- `ADR-0.8.0-agent-memory-retrieval-and-identity.md` and the OPP-12
  record-lifecycle protocol own the lifecycle vocabulary. The enum must match
  it. I did not check the protocol's state names against the values the
  engine writes.
- The `SearchHit` change touches the R-OBS-2 byte-stability rule
  (`search_types.rs:143-150`). Default-path ranking bytes are unchanged, but the
  serialized hit grows.

## Cost and performance

Four extra columns on rows already being read. There is no extra query. The
search hydration selects grow by four columns.

## Test plan sketch

- Real database: write v1, supersede with v2, retract another record, write a
  record with `valid_until` in the past. `get_many` under each view reports the
  exact currency for each. Search returns only current rows, each with
  `Active`.
- Validity-boundary test: `valid_as_of` inside and outside the window, for both
  `get` and `search`.
- Binding parity: the Python and TypeScript `NodeRecord` and `SearchHit` carry
  identical values for the same fixture.

## Adapter change once it lands

`fetch` and `search` results add `{state, superseded_at, valid_from,
valid_until}` (MR-013.1), and MR-013 leaves the `unavailable` row. Fix the
stale comment and the design.md sentence in the same change.

## Open questions for the owner

1. Should `reason` (retraction reason) be included? It is writer-set text, so it
   is content for MR-012 purposes.
2. Should search ever report supersession? It would need version-complete
   projection indexes, which `temporal.rs:155-157` rules out today.
