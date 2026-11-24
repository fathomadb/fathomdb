---
title: ADR-0.8.32 — Opt-in bulk write mode with strict default and doctor integrity verification
date: 2026-11-24
target_release: 0.8.32
desc: Keep default Engine::write and Engine::actuate strictly endpoint-validated (dangling_edge_endpoints == 0) while adding explicit opt-in WriteOptions / write_with_options (EndpointValidationMode::BulkNonStrict) with truthful WriteReceipt.dangling_edge_endpoints counts, non-bypassable scope/batch invariants, and fathomdb doctor dangling plus cross-scope edge verification.
blast_radius: src/rust/crates/fathomdb-engine/src/{write,write_commit,lib}.rs; src/rust/crates/fathomdb-sdk/**; src/rust/crates/fathomdb-py/**; src/rust/crates/fathomdb-napi/**; src/python/**; src/ts/**; src/rust/crates/fathomdb-cli/**; src/conformance/governed-*.json; scripts/governed-surface-pin.json; dev/interfaces/{rust,rust-sdk,python,typescript,cli}.md; docs/reference/**
status: accepted
---

# ADR-0.8.32: Opt-in bulk write mode with strict default and `doctor` integrity verification

## Context

`ADR-0.8.31-strict-endpoint-integrity.md` (`#74`) established strict atomic endpoint integrity across every `0.8.31` write path: `Engine::write` (`EndpointValidationMode::Strict` in `src/rust/crates/fathomdb-engine/src/write_commit.rs`) and `Engine::actuate` both require every edge's `from_id` and `to_id` to resolve to an active, non-superseded canonical node (`superseded_at IS NULL AND state = 'active'`) in the batch's final transaction state, rolling back atomically with `EngineError::WriteValidation` otherwise. `WriteReceipt::dangling_edge_endpoints` (`src/rust/crates/fathomdb-engine/src/write.rs`) was preserved on the receipt struct (`0` on every `0.8.31` commit) specifically so `0.8.32` could restore controlled out-of-order ingestion via an explicit opt-in bulk mode (`R32-03`).

Large graph imports and sharded batch pipelines frequently ingest edges before all target nodes arrive across separate batches. Requiring callers to topologically sort multi-batch graph exports into node-first phases adds significant ingestion complexity, whereas weakening `Engine::write` globally would silently reintroduce dangling edges into ordinary agent and application writes.

## Decision (HITL Gate 1 Decision 4 Pin B, 2026-11-24)

### 1. Default `Engine::write` and `Engine::actuate` Remain Strictly Validated

- **`Engine::write` stays strict by default:** Calling `Engine::write(batch)` (and Python `engine.write(batch)` / TypeScript `engine.write(batch)`) executes `EndpointValidationMode::Strict`. Any missing, inactive (`pending` / `deleted` / `purged`), superseded, or cross-scope edge endpoint rolls back the entire transaction atomically with `WriteValidation`, guaranteeing `WriteReceipt.dangling_edge_endpoints == 0` on every default commit.
- **`Engine::actuate` remains strict-only:** `Engine::actuate` (and every `fathomdb-mcp` write/checkpoint path per `ADR-0.8.32-mcp-read-write-and-distribution.md` `AC32-MCP-SAFETY-05`/`09`) never accepts bulk or non-strict endpoint flags and always rejects missing, inactive, superseded, or cross-scope endpoints atomically.

### 2. Explicit Opt-In `WriteOptions` / `Engine::write_with_options` (`Pin B`)

Expose an explicit opt-in bulk write surface across Rust (`fathomdb-engine`, `fathomdb-sdk`), Python (`fathomdb-py`, `src/python/`), and TypeScript (`fathomdb-napi`, `src/ts/`) with pre-authorized `scripts/governed-surface-pin.json` updates (`core = 5` and `recovery_denylist = 5` unchanged):

- **API surface:**
  - `EndpointValidationMode` (public opt-in enum: `Strict` [default] and `BulkNonStrict`) inside `WriteOptions` (defaulting to `EndpointValidationMode::Strict`).
  - `Engine::write_with_options(&self, batch: &[PreparedWrite], options: &WriteOptions) -> Result<WriteReceipt, ...>` (plus Python/TypeScript option parameter / parity method tracked in `src/conformance/governed-operation-parity.json`).
  - `Engine::write(batch)` delegates to `write_with_options(batch, &WriteOptions::default())`.
- **Admitted out-of-order edges & truthful `WriteReceipt.dangling_edge_endpoints`:**
  - Under `EndpointValidationMode::BulkNonStrict`, edges whose `from_id` or `to_id` does not yet exist as an active same-scope node in the batch's final transaction state are admitted instead of rolling back.
  - Before commit, the engine probes `from_id` and `to_id` independently against active canonical nodes (`superseded_at IS NULL AND state = 'active'`) within the caller's scope and reports the exact count in `WriteReceipt.dangling_edge_endpoints` (`0`, `1`, or `2` per active edge written in the batch).
  - Node-only batches continue taking the zero-SQL endpoint-check fast path and return `dangling_edge_endpoints == 0`.
  - Subsequent batches in the same caller `scope` can write the missing `logical_id` nodes so previously admitted edges resolve during traversal without rewriting the edges.
- **Non-bypassable invariants under `BulkNonStrict`:**
  - **Caller `scope` & anti-spoof enforcement:** `BulkNonStrict` still stamps opener-bound `ScopePolicy` on every inserted node, edge, and operational row, still enforces same-scope ownership on `logical_id` / `(collection_name, record_key)`, and **still atomically rejects cross-scope endpoints** (an edge whose `from_id` or `to_id` matches an active node belonging to a different caller `scope` is a `ScopeViolation` / hard validation failure, never admitted as "dangling").
  - **All other batch invariants remain strict:** Half-open validity windows (`valid_from < valid_until`), typed scalar `properties` validation, mandatory `source_id`, `InitialState` constraints, provenance retention, read-visibility trigger updates, and erasure invariants are unchanged and roll back atomically on any violation.

### 3. `fathomdb doctor` Dangling & Cross-Scope Edge Verification

Bulk ingestion requires an operator-grade read-only audit after out-of-order loading completes:

- Extend `fathomdb doctor` (`data-plane-integrity` / `check-integrity` in `src/rust/crates/fathomdb-cli/src/lib.rs`) to detect and report:
  1. **Dangling active edge endpoints:** active edges (`superseded_at IS NULL AND state = 'active'`) whose `from_id` or `to_id` does not resolve to an active node (`superseded_at IS NULL AND state = 'active'`).
  2. **Cross-scope edge anomalies:** edges whose active endpoints belong to a distinct `scope` from the edge's own `scope`.
- Emit deterministic structured findings under `--json` (distinguishing clean stores with exit `0` from dangling/cross-scope findings with `exit_code::DOCTOR_FOUND_ISSUES` `65`), preserving `ADR-0.6.0-cli-scope` and `docs/positions/recovery-surface.md` (diagnostic verification only; zero silent repair or drop).

### 4. Documentation and Docstring Alignment

Update stale `"informational (default FLAG-AND-COUNT)"` and `"unconditionally 0"` wording in `src/rust/crates/fathomdb-engine/src/write.rs`, `src/python/fathomdb/types.py`, `src/ts/src/write.ts`, `dev/interfaces/{rust,rust-sdk,python,typescript,cli}.md`, and `docs/reference/{rust-sdk,rust-api,python-api,typescript-api,errors,cli}.md` in Slice 30 alongside implementation.

## Consequences

- Ordinary callers (`Engine::write`, `Engine::actuate`, `fathomdb-mcp`) continue receiving strict endpoint referential integrity with zero behavior change.
- Bulk ingestion pipelines opt in explicitly via `WriteOptions` / `write_with_options`, inspect `WriteReceipt.dangling_edge_endpoints`, and verify post-import graph completeness with `fathomdb doctor`.
- Amends and extends `ADR-0.8.31-strict-endpoint-integrity.md` (`#74`) without weakening strict defaults.

## Citations

- `ADR-0.8.31-strict-endpoint-integrity.md` (`#74`)
- HITL Gate 1 Decision 4 Pin B (`dev/plans/0.8.32/prework/slice-9-triage.md`)
- `dev/plans/0.8.32/scope.md` § 3 and `dev/plans/0.8.32/plan.md` (`R32-03`)
