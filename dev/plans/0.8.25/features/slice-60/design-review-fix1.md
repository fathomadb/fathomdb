---
title: 0.8.25 Slice 60 design review — FIX-1 response
status: FIX1_AWAITING_REVIEW
review_cycle: 1
responds_to: design-review-cycle1.md
baseline: e5057256c548b181a6396aa8b1df4d24faf33248
---

# Slice 60 design review — FIX-1 response

## Disposition

Design version 3 resolves all Cycle 1 findings without beginning source or test
implementation. The scope remains the owner-approved one-page constrained graph
primitive. A second independent review is still required; this response does
not mark the design READY.

## Finding closure matrix

| Cycle 1 finding | FIX-1 disposition |
| --- | --- |
| P1-1 public/wire contract | Defines exact Rust types and method, Python dataclasses and `graph.expand`, TypeScript interfaces and `graph.expand`, native method names, target/origin cardinality, casing, integer/score encoding, recursive closed-request/open-response decoding, response coherence, and exact error families/FFI mappings. |
| P1-2 current/frozen contradiction | Adds the closed versioned `GraphReadContextV1` union. Current mode resolves one instant in one transaction; frozen mode authenticates and validates before/after the same transaction. It pins existence-relaxation and error precedence. |
| P1-3 seed/result identity | Explicit seeds are logical `IdSpace` values with ordered all-or-nothing visibility. Query seeds use a native logical-node-only hybrid mode before ranking limits with fixed search controls, deterministic deduplication and ordinals, truthful fallback, and no edge/anonymous seeds. Seeds are returned separately, excluded from targets, and depth zero is exact. |
| P1-4 work/top-N/index | Defines one unit per raw directional incident edge row, exact W success/W+1 no-result failure, exhaustive traversal before target truncation, complexity/storage bounds, and endpoint-index plans. Raw-row accounting makes schema 33 and its endpoint indexes sufficient; there is no migration. |
| P1-5 kind/direction/cycle/origin | Kinds are open case-sensitive strings, empty list means any, duplicates reject, edge kinds constrain traversal, and target kinds are return-only. Node/edge liveness, per-seed visited scope, seed/self-loop behavior, parallel-edge accounting, actual terminal direction, compact origin, and a total origin/result tie-break are normative. |
| P1-6 explanation/projection | Adds exact versioned result degradation and optional explanation sidecars, positional target association, content-free correlation, resolved seed contents, compact origin, lifecycle/dependency state, projection generation/origin/readiness, and an explicit hard/soft matrix. |
| P2 verification gaps | Maps codec/property and malformed-input matrices, deterministic cancellation-safe races, high-degree W/W+1, endpoint query plans, schema-33 no-migration proof, coherent device applicability, full workspace gates, and fresh Linux/Windows source-independent Rust/Python/Node consumers. |
| P3 stale readiness | Replaces the stale Slice 7 block with `FIX1_AWAITING_REVIEW` in both design and plan. Slice 7 and Slice 55 are acknowledged complete, but READY remains gated on independent Cycle 2. |

## Authority choices

- Existing ontology-neutral arbitrary `kind` strings rule out an invented kind
  registry or “unknown kind” error.
- Existing `ReadContextV1` and `FrozenReadContextV1` are wrapped, not reshaped.
- Existing endpoint indexes plus raw-row work charging bound traversal without
  a compound kind migration.
- Existing search fallback, frozen-read, vector-equivalence, projection, and
  storage errors retain their families. Only graph-specific validation,
  seed-visibility, work-bound, and graph-invariant failures use
  `FDB_GRAPH_EXPANSION`.
- Target kind remains output-only, preserving traversal through a differently
  typed intermediate node as required by the earlier independent graph review.

## Remaining gate

There is no unresolved owner/HITL choice in FIX-1. Independent Cycle 2 must
confirm that the contract is executable and has no implementation-shaping P1/P2
gap before any RED implementation work begins.
