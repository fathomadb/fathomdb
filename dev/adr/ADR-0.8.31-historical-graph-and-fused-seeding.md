---
title: ADR-0.8.31 — Historical graph reconstruction, typed traversal predicates and opt-in fused-hit seeding
date: 2026-11-22
target_release: 0.8.31
desc: Bitemporal/revision-aware graph traversal (as_of_write_cursor and as_of_epoch_ms) subordinate to physical erasure, typed edge/target predicates on graph.expand and use_graph_arm, opt-in fused-hit seeding with seed provenance, and mixed-edge seed slot fix.
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

## Decision (amended HITL Gate 1, 2026-11-22)

1. **Historical graph reconstruction (`as_of` / revision-aware traversal)**:
   - Add an opt-in historical edge view mode to `GraphExpandRequestV1` (and
     `GraphReadContextV1`; leaving `ReadView` world-time-only) supporting:
     - **Validity time (`valid_as_of`)**: edge valid when
       `(t_valid IS NULL OR t_valid <= valid_as_of) AND (t_invalid IS NULL OR t_invalid > valid_as_of)`
       (selecting and checking `t_valid` explicitly in SQL).
     - **Revision/knowledge time (`as_of_write_cursor` or `as_of_epoch_ms`)**:
       - `as_of_epoch_ms` resolves once per `graph.expand` call via Schema 38's
         `_fathomdb_commit_timestamps` index:
         `SELECT MAX(write_cursor) FROM _fathomdb_commit_timestamps WHERE committed_at_ms <= ?1`.
         Because `_fathomdb_commit_timestamps.write_cursor` records each
         committed batch's high-water `WriteReceipt.cursor` (`max(row_cursors)`),
         comparing `row.write_cursor <= as_of_cursor` and
         `(row.superseded_at IS NULL OR row.superseded_at > as_of_cursor)`
         correctly admits every row committed in or before that batch.
       - On stores upgraded from Schema 37 via `fathomdb migrate`, pre-upgrade
         commits have no historical wall-clock timestamps; `as_of_epoch_ms`
         earlier than the first Schema-38 commit resolves to cursor `0` (empty),
         while all pre- and post-upgrade history remains directly addressable
         via `as_of_write_cursor`.
     - **Immutable-revision scope & `Engine::transition` semantics**:
       `as_of` reconstructs **immutable write revisions** (`write_cursor <=
       as_of_cursor`, `superseded_at IS NULL OR superseded_at > as_of_cursor`,
       historical node `body` and `properties` as of `as_of`). Because
       `Engine::transition` (`record_lifecycle.rs`) mutates `canonical_nodes.state`
       in place without allocating a new `write_cursor`, current-state lifecycle
       semantics apply to historical node eligibility.
   - **Strict subordination to erasure**: purged (`purge`) and source-erased
     (`erase_source`) records are physically deleted and are NEVER replayed;
     historical traversal replays only retained superseded/expired revisions
     (and `evidence.rs` keeps erased or barrier-blocked sources `Unavailable`).
   - **Zero penalty on default current-time traversal**: default current-time
     `graph.expand` pushes `superseded_at IS NULL`, `kind`, `t_valid`, and
     `t_invalid` into SQL (`UNION ALL` for `Both` direction) backed by Schema 38
     partial active incident indexes
     (`canonical_edges_from_active_idx(from_id, kind) WHERE superseded_at IS NULL`
     and `canonical_edges_to_active_idx(to_id, kind) WHERE superseded_at IS NULL`)
     so superseded rows never consume incident-edge `LIMIT` or `work_units`,
     even when `edge_kinds` is empty. Historical traversal uses partial
     composite indexes `canonical_edges_from_sup_kind_idx(from_id, superseded_at,
     kind) WHERE superseded_at IS NOT NULL` and
     `canonical_edges_to_sup_kind_idx(to_id, superseded_at, kind) WHERE
     superseded_at IS NOT NULL` plus
     `canonical_nodes_logical_sup_idx(logical_id, superseded_at, write_cursor)
     WHERE superseded_at IS NOT NULL`
     (verified via `EXPLAIN QUERY PLAN` `SEARCH` for both empty and non-empty
     `edge_kinds` and historical node resolution).
2. **Closed typed predicates on edges and target nodes**:
   - Extend `GraphExpandRequestV1` (marked `#[non_exhaustive]` with constructor
     ergonomics) with `edge_predicates: Vec<PropertyPredicate>` and
     `target_predicates: Vec<PropertyPredicate>` using a closed typed grammar
     (`Eq`, `Ne`, `Lt`, `Lte`, `Gt`, `Gte` over `PropertyValue`
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
     behalf of the ineligible endpoint (require endpoint eligibility cleanly
     before consuming a slot), and rewrite Phase 2 edge lookups as `UNION ALL`
     with `edge_kinds`/`properties` filtering.
   - Expose per-hit `graph_seed_source` (`fused_top_k` / `edge_fts` /
     `entity_fts`) and `graph_hop_depth` in search explanation and evidence
     sidecars across Rust, Python, and TypeScript.

## Consequences

- Closes historical verification-chain reconstruction, typed traversal filtering,
  fused-hit seeding with auditable provenance, and the mixed-edge seed-slot fix.
- Default current-time graph queries and `use_graph_arm = false` searches incur
  zero extra work.
