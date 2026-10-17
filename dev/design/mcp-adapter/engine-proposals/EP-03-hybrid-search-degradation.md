---
title: EP-03 — Typed degradation on hybrid search
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. Report on every hybrid SearchResult which arms ran and why any arm did not, including an absent embedder and a failed or timed-out query embedding, instead of silently returning lexical-only results.
---

# EP-03 — Typed degradation on hybrid search

Status: Draft. Priority P1. Not an accepted decision.

## Problem

A hybrid `search` whose query embedding is unavailable returns lexical results
and `soft_fallback: None`:

- The query embedding is computed with
  `dispatch_embed_vector(...)`. Every error except a panic becomes `None`
  (`fathomdb-engine/src/search_api.rs:542-548`).
- `DispatchError` already names the cause: `NotConfigured`, `Saturated`,
  `QueuedExpired`, `StartedTimeout`, `Provider`, `InvalidOutput`, `Closing`,
  `Cancelled` (`embed_dispatch/core.rs:20-30`). All of it is discarded.
- In the reader, `soft_fallback` is set only when vector eligibility degraded,
  or when a query vector existed but matched unprojected rows
  (`search.rs:1698-1718`). With no query vector it is `None`.
- `SoftFallback` carries only `branch`, with no reason
  (`search_types.rs:10-12`).

Two more verified facts the adapter got wrong:

- With `dense_disabled`, a non-frozen hybrid search fails with
  `VectorEquivalenceMismatch` before any work (`search_api.rs:490-500`). It does
  not degrade. The adapter's `{arm: vector, reason: dense_disabled}` entry
  (`src/rust/crates/fathomdb-mcp/src/tools.rs:676-681`) is reachable only
  through a race, and the error itself maps to `internal`
  (`error.rs:244`).
- The adapter's `embedder_unavailable` entry comes from `embedding_readiness`,
  sampled once per open epoch (`registry.rs:696-698`, `tools.rs:682-687`). A
  configured embedder whose call times out or is saturated produces no entry at
  all. That breaks MR-032.1 silently.

## Proposed surface

Add a `degradations` field to `SearchResult`, changed in place under
`ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md`.

```rust
#[non_exhaustive]
pub struct ArmDegradation {
    pub arm: SoftFallbackBranch,   // Vector, Text, TextEdge, GraphArm
    pub reason: DegradationReason,
}
#[non_exhaustive]
pub enum DegradationReason {
    EmbedderAbsent,         // DispatchError::NotConfigured
    EmbedderSaturated,      // Saturated, QueuedExpired
    EmbedderTimeout,        // StartedTimeout
    EmbedderFailed,         // Provider, InvalidOutput
    EligibilityDegraded,    // today's vector_eligibility_degraded
    ProjectionPending,      // today's "matching rows not yet projected" probe
}
```

- `degradations` is empty when every requested arm ran. Its order is fixed:
  arm, then reason.
- `soft_fallback` stays as it is, so ranking bytes and existing callers do not
  change. It is derivable from `degradations`.
- `Closing` and `Cancelled` keep failing the call as today. They are not
  degradations.
- `search_text_only` returns an empty `degradations`. Its only arm is text.

## Invariants and consistency

- Results and ranking do not change. Only the report is added.
- The report is computed in the same call that produced the hits, so it is never
  stale. The adapter's per-epoch sampling goes away.
- Each degradation reason is stable across bindings and goes in the parity map.

## Interaction with ADRs

- `ADR-0.8.25-eligibility-and-frozen-reads.md` requires a typed soft fallback
  when an arm declines. This adds the reason it lacks. No successor is needed.
- `ADR-0.8.18-vector-equivalence-self-check.md` keeps the dense-disabled refusal
  as an error. Unchanged.
- `ADR-0.8.27-rust-sdk-parity.md`: the field lands in Python and TypeScript in
  the same change.
- `dev/design/0.8.8-explain-and-telemetry-adr.md` makes the result bytes for
  `explain=false` a zero-cost contract (R-OBS-2, quoted at
  `search_types.rs:143-150`). An always-present field changes the serialized
  shape. The owner must accept that, or make it present only when non-empty.

## Cost and performance

There is no extra query. The reasons are already in hand where the engine
discards them.

## Test plan sketch

- Real engine opened with `EmbedderChoice::None`: hybrid search returns
  lexical hits and `[{Vector, EmbedderAbsent}]`.
- Embedder with a forced call timeout (`embedder_call_timeout_ms`): the result
  carries `EmbedderTimeout`, which is the case the adapter misses today.
- A healthy embedder with unprojected matching rows gives `ProjectionPending`,
  the same condition as today's soft fallback.
- Byte test: `results` and `soft_fallback` are identical with and without the
  change, for a fixed fixture.

## Adapter change once it lands

Map `degradations` straight to `{arm, reason}`. Remove the per-epoch
`embedder_usable` sample and the `dense_disabled` entry. Separately, as an
adapter-only fix that needs no engine change, map `VectorEquivalenceMismatch`
to `not_ready` with remedy "use mode text", instead of `internal`.

## Open questions for the owner

1. Always-present field, or present only when non-empty, given R-OBS-2?
2. Is `EmbedderSaturated` a degradation, or should a saturated embed fail the
   call as `Overloaded` so callers retry?
