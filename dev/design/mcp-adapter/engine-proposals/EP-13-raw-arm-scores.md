---
title: EP-13 — Raw per-arm scores in the explanation
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. Carry the vector distance and BM25 score the reader already computes into PerHitExplain, so a confidence verdict can be built on similarity-calibrated values rather than rank-fusion scores.
---

# EP-13 — Raw per-arm scores in the explanation

Status: Draft. Priority P2. Not an accepted decision.

## Problem

- `PerHitExplain` carries per-arm ranks (`vector_rank`, `text_rank`,
  `graph_rank`), `fused_score` (the RRF value), `ce_score`, `blended`,
  `importance`, `confidence` and `structural`
  (`fathomdb-engine/src/search_types.rs:265-299`). It has no raw similarity.
- The reader computes raw values and drops them. The vector arm reranks with
  `vec_distance_l2` and carries the distance per hit
  (`search.rs:1552-1555`). The text arm selects `bm25(search_index)`
  (`search.rs:1854-1857`).
- `ce_score` is the only calibrated score, and it exists only inside the
  reranked pool with a cross-encoder loaded (`search_types.rs:99-105`).

Adapter status: `verdict` is fixed to `unavailable` (`src/rust/crates/fathomdb-mcp/src/tools.rs:780`;
MR-031.1). There is no workaround, by design. Rank-fusion scores must never
feed a verdict.

## Proposed surface

Add to `PerHitExplain`, in place (`#[non_exhaustive]` leaf):

- `vector_distance: Option<f64>`: the f32 L2 distance from phase 2, for hits the
  vector arm returned.
- `text_bm25: Option<f64>`: the raw SQLite `bm25()` value (more negative is
  better), for hits the text arm returned.
The graph arm is left out. I did not check what raw value it computes.

These are present only when `explain=true`. `SearchHit` does not change.

## Invariants and consistency

- The values are the same numbers ranking used, from the same snapshot. They
  are not recomputed.
- Mean-centered binary quantization affects phase 1 only. Phase-2 distance is on
  un-centered vectors (`search.rs:1491-1497`), so the reported distance is
  comparable across queries for one embedder.
- `bm25()` is not comparable across queries or corpora. The field
  documentation must say so.

## Interaction with ADRs

- `dev/design/0.8.8-explain-and-telemetry-adr.md` ratified the explanation
  field set. §A.4 Q1 kept "raw exposed; normalization deferred"
  (`search_types.rs:278-280`). This adds raw fields within that rule. Adding
  them needs owner ratification, as the original set did.
- `ADR-0.6.0-vector-identity-embedder-owned.md`: distance is meaningful only
  per embedder identity. The trace already carries `embedder_id`.

## Cost and performance

None when `explain=false`. With explain on, the cost is two values per hit that
are already in memory.

## Test plan sketch

On a real database with the default embedder: an exact-duplicate query has the
smallest `vector_distance`. A query term present in one document only gives
that document a `text_bm25` value and leaves the field `None` on vector-only
hits. Values are stable across two runs.

## Adapter change once it lands

The verdict rule (MR-031, ADR-owned) can use `vector_distance` and `ce_score`.
Calibration on the MR-031.2 corpus is still required before `verdict` leaves
`unavailable`.

## Open questions for the owner

1. Expose cosine similarity as well as L2 distance? The vectors' normalization
   was not checked for this proposal.
2. Include raw scores for edge-fact (`TextEdge`) hits?
