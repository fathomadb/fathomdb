---
title: EP-12 — Bounded match count and ranked results beyond the cap
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. Report a bounded lexical match count with a saturation flag on search, and decide whether ranked search may page past its 100-result cap, so callers can tell "these are all" from "there are more".
---

# EP-12 — Bounded match count and ranked results beyond the cap

Status: Draft. Priority P2. Not an accepted decision.

## Problem

- `SearchResult` is `{projection_cursor, soft_fallback, results, explanation}`
  (`fathomdb-engine/src/search_types.rs:138-152`). It has no total or lower
  bound. A full page cannot be told apart from a page that is everything.
- The ranked cap is `MAX_SEARCH_RESULT_LIMIT = 100`
  (`search_types.rs:360-371`), and the SDK repeats it (`fathomdb-sdk/src/guard.rs:11`).
- `ADR-0.8.25-frozen-pagination-and-operational-state.md` states "Ranked top-K
  search remains distinct from ordered pagination" and "No graph/search cursor
  ... is introduced". Keyset pages exist for canonical records by
  `write_cursor`, not for ranked results.

Adapter workaround removed: completeness is `limit_reached` with "total unknown"
whenever a page is full (interface §4 `search`). Continuations re-run the search
at `limit = min(100, offset + page)` and slice it
(`src/rust/crates/fathomdb-mcp/src/tools.rs:637`; design §7). Past 100 the
response says the cap was reached (MR-041.2).

## Proposed surface

**Part A, bounded count.** `SearchOptions.count_bound: Option<u32>` (1..=1000)
and `SearchResult.lexical_matches: Option<BoundedCount>`:

```rust
pub struct BoundedCount { pub at_least: u32, pub saturated: bool }
```

The count is lexical only. It counts FTS matches that pass the same eligibility
and view predicates as the text arm, with `LIMIT bound + 1`. A vector arm has
no match set (KNN always returns k), so no vector count is offered. That is the
honest form of MR-030.2.

**Part B, beyond the cap.** Not proposed as an engine change in this draft.
Ranked pages past 100 need a ranked continuation token bound to the frozen
context and the fused order. That contradicts the pagination ADR above and the
reproduce-or-fail rule. The owner decides whether MR-041.2 keeps its "cap
reached" answer.

## Invariants and consistency

- `lexical_matches` is computed in the same reader snapshot as the hits.
- `saturated: true` means at least `bound + 1` matches exist. `at_least` is then
  `bound`.
- The count ignores ranking, the graph arm and the vector arm. Its documentation
  must say so.

## Interaction with ADRs

- Part A is additive under `ADR-0.8.25-eligibility-and-frozen-reads.md`. The
  count must apply eligibility before the bound, as that ADR requires for every
  cap.
- Part B would need a successor to
  `ADR-0.8.25-frozen-pagination-and-operational-state.md`.

## Cost and performance

One extra bounded FTS query when `count_bound` is set, and none otherwise.
Worst case `bound + 1` index matches plus the eligibility joins. Measure it
against the text-query latency gates.

## Test plan sketch

Real database with 7 matching records: `count_bound=5` gives `{5, saturated}`,
and `count_bound=10` gives `{7, false}`. Retracted and out-of-window records are
not counted.

## Adapter change once it lands

`limit_reached` adds `matches: {at_least, saturated}` (MR-030.2). Text mode and
hybrid mode both use the lexical count, labelled as lexical.

## Open questions for the owner

1. Is a lexical-only count acceptable for MR-030.2 in hybrid mode?
2. Keep the 100-result cap as the contract for ranked results?
