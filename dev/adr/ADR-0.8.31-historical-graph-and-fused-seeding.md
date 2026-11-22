---
title: ADR-0.8.31 — Historical graph reconstruction, typed traversal predicates and opt-in fused-hit seeding
date: 2026-11-22
target_release: 0.8.31
desc: Bitemporal/revision-aware graph traversal (as_of) subordinate to physical erasure, typed edge/target predicates on graph.expand and use_graph_arm, opt-in fused-hit seeding with seed provenance, and mixed-edge seed slot fix.
blast_radius: src/rust/crates/fathomdb-engine/src/graph_expand/**; src/rust/crates/fathomdb-engine/src/search.rs; src/rust/crates/fathomdb-engine/src/filter.rs; src/rust/crates/fathomdb-sdk/**; src/python/**; src/ts/**
status: accepted
---

# ADR-0.8.31: Historical graph reconstruction, typed traversal predicates, and opt-in fused-hit seeding

## Context

In FathomDB 0.8.30:

1. `read_graph_expand_with_stop_words_in_tx`
   (`src/rust/crates/fathomdb-engine/src/graph_expand/execution.rs`)
   unconditionally skips any edge with `edge.superseded_at.is_some()` or
   `t_invalid <= view.edge_now()`, preventing reconstruction of historical
   verification chains as they existed before an edge was superseded.
2. `GraphExpandRequestV1` (`graph_expand/types.rs`) filters only on
   `edge_kinds` and `target_kinds`, with no typed property predicates and no
   returned edge properties.
3. `bfs_graph_arm_candidates` (`search.rs`) walks all edges without edge-kind
   or edge-property filters, seeds only from `search_index_edges` and entity
   `search_index` FTS (ignoring vector/hybrid relevance and body-less edges),
   exposes no per-hit seed provenance or hop depth, and Seed source A uses
   `ef eligible OR et eligible`, letting an edge touching an ineligible node
   consume a `SEED_FTS_N = 10` slot.

## Decision

1. **Historical graph reconstruction (`as_of` / revision-aware traversal)**:
   - Add an opt-in historical edge view mode to `GraphExpandRequestV1` (and
     `ReadContextV1` / `GraphReadContextV1`) supporting:
     - **Validity time (`valid_as_of`)**: edge valid when
       `(t_valid IS NULL OR t_valid <= valid_as_of) AND (t_invalid IS NULL OR t_invalid > valid_as_of)`.
     - **Revision/knowledge time (`known_as_of` / `as_of_write_cursor` or
       `as_of_epoch_ms`)**: edge existed at the chosen revision boundary
       (`write_cursor <= as_of_cursor` / created at or before `known_as_of`)
       and was not yet superseded at that instant
       (`superseded_at IS NULL OR superseded_at > known_as_of_boundary`), with
       corresponding node version resolution at the same boundary.
   - **Strict subordination to erasure**: purged (`purge`) and source-erased
     (`erase_source`) records are physically deleted and are NEVER replayed;
     historical traversal replays only retained superseded/expired revisions.
   - **Zero penalty on default current-time traversal**: default current-time
     `graph.expand` uses a partial active SQL predicate (`superseded_at IS NULL`)
     backed by partial active edge indexes so superseded rows never consume
     incident-edge `LIMIT` or `work_units`. Historical traversal uses dedicated
     composite indexes `(from_id, kind, superseded_at)` and
     `(to_id, kind, superseded_at)` (verified via `EXPLAIN QUERY PLAN`
     `SEARCH`).
2. **Closed typed predicates on edges and target nodes**:
   - Extend `GraphExpandRequestV1` with `edge_predicates: Vec<PropertyPredicate>`
     and `target_predicates: Vec<PropertyPredicate>` using a closed typed
     grammar (`Eq`, `Ne`, `Lt`, `Lte`, `Gt`, `Gte` over `PropertyValue`
     `String | Int | Float | Bool` — no SQL, no OR/arbitrary DSL).
   - Evaluate predicates before an edge can enter the traversal frontier or
     displace a candidate target, ensuring ineligible edges neither traverse
     nor displace eligible results.
   - Return inline `properties` on `GraphTargetV1` and `terminal_edge_properties`
     on `GraphOriginV1` (bound to the exact revision `write_cursor` when
     resolved via `resolve_graph_evidence`).
3. **Opt-in fused-hit seeding, seed provenance, and `use_graph_arm` fixes**:
   - Add opt-in `graph_seed` mode (`GraphFts` default for backward
     compatibility, `FusedTopK`, `Both`) and `graph_seed_k` (default `10`,
     bounded) plus optional `graph_edge_kinds` and `graph_edge_predicates` on
     search requests when `use_graph_arm = true`.
   - Default `use_graph_arm = false` remains byte-identical and zero-cost.
   - Fix Seed source A in `bfs_graph_arm_candidates` so an edge bridging an
     ineligible node and an eligible node cannot consume a `SEED_FTS_N` slot on
     behalf of the ineligible endpoint (require both endpoints eligible for
     Seed source A, or evaluate edge-endpoint eligibility cleanly before
     taking a slot).
   - Expose per-hit `graph_seed_source` (`fused_top_k` / `edge_fts` /
     `entity_fts`) and `graph_hop_depth` in search explanation and evidence
     sidecars across Rust, Python, and TypeScript.

## Consequences

- Closes the external reviewer's largest functional gap (historical
  verification chain reconstruction) and Memex Graph Asks 2, 3, and 5.
- Default current-time graph queries and `use_graph_arm = false` searches incur
  zero extra work.
