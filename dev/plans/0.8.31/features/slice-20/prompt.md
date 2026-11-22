---
title: Slice 20 — Schema 38+, outside-body inline properties + declared typed indexes, commit timestamps, app-declared vector kinds, op-collection readback, W12 fixes and sqlite_master audit
status: PLANNED
target_release: 0.8.31
slice: 20
date: 2026-11-22
---

# Slice 20 — Schema 38+ Property Graph Core, Commit Timestamps, Vector Kinds, Op-Collection Readback, W12 Fixes & Completeness Audit · `[implementation]` · depends-on: `10 (PREWORK-PREP)`

> **Relocated from pre-triage Slice 10 to Slice 20 per HITL Gate 1 (`2026-11-22`).**
> A full Phase C executable plan will be authored at `dev/plans/0.8.31/features/slice-20/plan.md`
> incorporating every Slice 4 (`P4-01`–`P4-06`, `P4-09`, `P4-10`, `P4-12`, `P4-13`), Slice 5
> (`P5-01`–`P5-03`, `P5-07`), and Phase B review finding before Slice 20 execution begins.

## Key deltas added by HITL Gate 1 & Phase B Review (`2026-11-22`)

1. **`P4-09` (Schema 38 commit timestamps):** Add `_fathomdb_commit_timestamps(write_cursor INTEGER PRIMARY KEY, committed_at_ms INTEGER NOT NULL)` + `_fathomdb_commit_timestamps_ms_idx(committed_at_ms, write_cursor)`, keyed by the batch high-water `WriteReceipt.cursor` (`max(row_cursors)`), populated once per committed batch in both `commit_batch` AND `actuate_new` with monotonic clamping (`ORDER BY write_cursor DESC LIMIT 1` on the PK — zero scan), classified as `DurableAuthority` in `sqlite_master` / `bootstrap_is_fresh` / `data_plane_integrity.rs`.
2. **`P4-01` & `P4-06` (`vec0` 16-column budget):** Exclude `entity_class = 'edge'` from `desired_vector_attr_columns`, reserve 1 metadata slot for `0.8.32` `scope`, use `vec0` `attr_<hex>` strictly for node `Eq` (no non-NULL sentinel overbuilding), fix stale "six built-in" comment, and route `Ne` / numeric ranges cleanly via `canonical_property_values` with post-KNN parity.
3. **`P4-02` (Re-projection on lifecycle transition):** Thread `(body, properties)` through all attribute/vector projectors, backfills, relaxed-`ReadView` filter paths, and `record_lifecycle.rs:232, 311-312` on re-projection back to `Active`.
4. **`P4-03` & `P5-01` (`R1` custom vector kinds):** Use explicit opt-in enrollment (keeping `tc67_unsupported_vector_kind_report` green), reject reserved `"edge_fact"`, replace `&'static str` `resolve_source_type`, and update all SQL 7-kind mirrors (`filter.rs`, `projection_generation.rs`, `search.rs`, `data_plane_integrity.rs`).
5. **`P4-04` (Visibility coalescing macro):** Invoke `visibility_coalescing_trigger_sql!` (NOT `read_visibility_coalescing_upgrade_sql!`) for `("cpv", "canonical_property_values")` in step 38; update `VISIBILITY_TRIGGER_TABLES` (`19 -> 20`), `canonical_batch_has_no_custom_triggers`, `bootstrap_is_fresh`, and `projection_registry_digest`.
6. **`P4-10` (Unified historical index ordering):** Create partial active incident indexes `canonical_edges_{from,to}_active_idx(from_id|to_id, kind) WHERE superseded_at IS NULL` and unified historical composite indexes `canonical_edges_{from,to}_sup_kind_idx(from_id|to_id, superseded_at, kind)` (verified via `EXPLAIN QUERY PLAN` for both empty and non-empty `edge_kinds`).
7. **`P4-12` (`sqlite_master` + `ExportClass`):** Classify dynamic `vector_default_*` / FTS5 shadow tables via deterministic prefix/suffix matchers, pin `_fathomdb_vector_kinds` + `_fathomdb_commit_timestamps` as `DurableAuthority`, and extend `data_plane_integrity.rs` + `physical_erasure/admission.rs` for `properties`.
8. **`P4-13` (Inspector workspace compile & fixture hygiene in Slice 20):** Update the 15 exhaustive `PreparedWrite` / `Provenanced*` literals in `src/inspector/tests/{fixtures/mod.rs,integration/graph.rs}` (wrapping them in local helpers so Slices 30–40 stay clean) and verify `cargo clippy --workspace --all-targets` + `cargo test --workspace` stay green on Schema 38.
9. **`P3-03` / `P5-08` & `check-public-doc-truth.py` lockstep:** Update `docs/compatibility/index.md` (`SCHEMA_VERSION` `37 -> 38`) alongside `fathomdb-schema/src/lib.rs`, and update `governed-{operation-parity,surface-allowlist}.json`, `scripts/governed-surface-pin.json` (pre-authorized at HITL Gate 1), `python_surface_additions.json`, and `test_slice130_package_boundary.py` for `read.operational_collections` / `read.operational_collection` and `EraseReport.completed_pending_erasure`.
