---
title: ADR-0.8.31 — Strict-only write endpoint integrity
date: 2026-11-22
target_release: 0.8.31
desc: Enforce atomic endpoint existence and active-state referential integrity on all 0.8.31 write paths; defer opt-in bulk/non-strict import mode to 0.8.32.
blast_radius: src/rust/crates/fathomdb-engine/src/write.rs; src/rust/crates/fathomdb-engine/src/write_commit.rs; src/rust/crates/fathomdb-engine/src/write_validation.rs; src/rust/crates/fathomdb-sdk/**; src/python/**; src/ts/**
status: accepted
---

# ADR-0.8.31: Strict-only write endpoint integrity

## Context

In FathomDB 0.8.30, `Engine::actuate` (`validate_prospective_edge_endpoints`
in `src/rust/crates/fathomdb-engine/src/actuation.rs`) strictly rejects derived
edges whose `from_id` or `to_id` is not an active, non-superseded canonical
node in the batch's final state, whereas ordinary `Engine::write`
(`count_dangling_edge_endpoints` in
`src/rust/crates/fathomdb-engine/src/write_commit.rs`) counts missing or
superseded endpoints in `WriteReceipt::dangling_edge_endpoints` and still
commits the batch (and does not check `state = 'active'`). External review
identified inconsistent write referential integrity across write paths as a
production-readiness gap.

## Decision (Owner A3 ruling, 2026-11-21)

1. **Strict-only endpoint integrity in `0.8.31`**:
   - Every edge written through `Engine::write` (and `Engine::actuate`) must
     reference `from_id` and `to_id` logical nodes that exist in the batch's
     final transaction state with `superseded_at IS NULL AND state = 'active'`.
   - Same-batch node creation is valid regardless of ordering within the batch
     (validation runs inside the write transaction after canonical inserts and
     before commit, using the active partial index
     `canonical_nodes_logical_active_idx` via `SEARCH`, never `SCAN`).
   - If any edge endpoint is missing, superseded, or inactive (e.g.
     `tombstoned` / `deleted`), the entire batch rolls back atomically and
     returns a typed `EngineError::WriteValidation` (mapped to
     `WriteValidationError` in Python/TypeScript and `ErrorKind::InvalidInput` /
     write validation in `fathomdb-sdk`) leaving zero partial state after
     reopen.
   - `WriteReceipt::dangling_edge_endpoints` remains on the receipt struct for
     wire/SDK compatibility (always `0` on committed `0.8.31` writes, ready to
     report non-zero counts when opt-in bulk mode lands in `0.8.32`).
2. **Pre-1.0 test and doc updates**:
   - Existing tests asserting that `Engine::write` commits dangling edges
     (`pr_g8_dangling_edges.rs`, `slice20_graph_traversal.rs` inactive-node
     setup, `slice85_physical.rs` orphan-edge setup, and functional Python/TS
     dangling-edge tests) are updated under this ADR to verify strict atomic
     refusal (or construct valid active endpoints first and then transition/
     tombstone them when testing traversal over inactive intermediates).
   - `docs/reference/{python-api,typescript-api,rust-sdk,rust-api}.md` and
     `dev/interfaces/` are updated in the same slice.
3. **Bulk/non-strict import deferred to `0.8.32`**:
   - An explicit opt-in bulk import mode (restoring controlled out-of-order
     edge ingestion with `dangling_edge_endpoints` reporting and post-import
     `doctor` checks) is scheduled for `0.8.32` (`R32-03`).

## Consequences

- `0.8.31` stores guarantee zero dangling edges on creation across all write
  paths.
- Node-only batches perform zero endpoint lookups (`node_only_dangling_check`
  fast path preserved).
