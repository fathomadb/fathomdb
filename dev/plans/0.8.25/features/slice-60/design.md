---
title: 0.8.25 Slice 60 — minimal constrained combined-expansion design
status: DRAFT_SCOPE_RECONCILED_BLOCKED_ON_SLICE_7
design_version: 2
target_release: 0.8.25
depends_on: 55
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 60 design

## Authority and boundary

Implements the retained subset of R25/AC25-60, Memex need 15, the compact
graph-origin portion of need 12, and A25-05/A25-06. It closes a genuine
combined-expansion gap: existing graph primitives have bounded traversal and
direction support, but the search-expansion path cannot honor caller direction,
edge label, target kind, and eligibility together.

Rich continuation, replayable full-path evidence, graph diffusion, ontology,
relation inference, and automatic routing are not part of 0.8.25.

## Contract

```text
GraphSeedV1 = Query { text, ranked_limit: 1..25 }
              | Explicit { logical_ids: [1..25] }
GraphExpandRequestV1 {
  schema_version: 1, seed, direction: incoming | outgoing | both,
  edge_kinds: [0..32], target_kinds: [0..32],
  context: ReadContextV1,
  max_depth: 0..3, result_limit: 1..50,
  max_work_units: 1..10000,
  include_explanation: false | true
}
GraphOriginV1 {
  schema_version: 1, seed_logical_id, target_logical_id,
  hop_count, terminal_edge_kind, direction
}
GraphExpandResultV1 {
  schema_version: 1, targets, graph_origins,
  complete: true, work_units, explanation?
}
```

Exactly one seed source is required. Empty or duplicate explicit seeds,
unknown kinds, unindexed eligibility, and out-of-range bounds fail before any
read. Memex concepts such as `slot` map to canonical `edge_kind` or declared
target attributes outside the Engine.

The result is one deterministic page. No cursor is emitted. Each returned
target has at least one compact origin; full ordered edge sequences and
replayable path evidence are deferred. Callers can resolve returned artifacts
through Slice 50 when source evidence is required.

## Execution

1. Validate seed, constraints, bounds, and one Slice 35 read context.
2. Query seeding calls `Engine.search` once under that same context; explicit
   seeds resolve under it without invoking search.
3. Apply eligibility before seed truncation. Compile direction and exact
   canonical `edge_kind` into the bounded BFS. Apply indexed eligibility before
   every frontier cap. A target is returned only when its artifact kind matches
   `target_kinds` and it is eligible/live.
4. Traverse breadth first with cycle guards to `max_depth` and
   `max_work_units`. Deduplicate targets by logical ID, retaining their
   deterministically first origin under order `(hop_count, seed_ordinal,
   terminal_edge_kind, target_logical_id)`.
5. Sort by that order and apply `result_limit`. Discovering more work than the
   hard bound returns `graph_expansion_bound_exceeded` and no result; reaching
   the caller result limit after bounded discovery is a complete top-N result,
   not a continuation claim.

All seeding, traversal, hydration, and optional explanation use the same read
context. A frozen context is reproduced or fails typed; an unfrozen context is
one Engine-call transaction. No constraint may be emulated after truncation or
in an SDK.

## Compatibility, failures, and verification

Existing `neighbors`, `search_expand`, default `Engine.search`, and
`use_graph_arm=false` behavior remain unchanged. The new request is additive.
Explanation uses Slice 55 compact inclusion/degradation codes. It does not
enumerate excluded paths. Failures include invalid seed/argument, unsupported
kind/predicate/version, seed invisible, read-context unavailable/drifted,
projection unavailable/degraded, and `graph_expansion_bound_exceeded`.

RED/GREEN real-database fixtures cover incoming/outgoing/both, edge-kind and
target-kind matrices, query/explicit seeds, eligibility below the unfiltered
frontier cap, cycles, every bound, deterministic insertion-order independence,
same-context mutation races, compact origins, and no ignored argument. Query
plans prove native pre-truncation constraints. Compatibility fixtures pin the
existing APIs and default result shape.

Run fast, heavy, all/all-feature, Windows Rust/Python/Node, and locally packed
graph/search routes. CUDA runs only if query seeding changes dense/rerank
dispatch; otherwise CUDA is N/A. Operator, live-model, and pre-publication
registry routes are N/A. A formal independent READY review remains required
after Slice 7 and Slice 55 complete.
