---
title: Slice 20 Status — 0.8.31 Property Graph Schema, Inline Properties, Declared Indexes & W12 Fixes
status: CLOSED
target_release: 0.8.31
slice: 20
date: 2026-11-22
---

# Slice 20 Status — `0.8.31` Property Schema (`PROPERTY-SCHEMA`)

- **Branch / Worktree:** `release/0.8.31` in `/home/coreyt/projects/fathomdb-worktrees/release-0.8.31`
- **Plan:** [`dev/plans/0.8.31/features/slice-20/plan.md`](./plan.md)
- **Commits:** `022b62b49` (Layer 1), `3da57de2e` (Layer 2), `644333afb` (Layer 3), `947398398` (Layer 4), `36802ba4e` / `9ee29cd3e` (Layer 5 & governed pin), `1c61aea20` (`FIX-1`), `975637f3c` (`FIX-2`)
- **Verdict:** **CLOSED / LANDED** — all 5 implementation layers + `FIX-1` + `FIX-2` verified green across Rust, Python, TypeScript, and Markdown gates; independent adversarial code review returned **`APPROVE`** at HEAD `975637f3c`.

## 1. Delivered Scope

1. **Layer 1 — Schema 38 Foundation (`022b62b49`):**
   - Advanced `SCHEMA_VERSION = 38` and exported `STEP_38_SQL` as the single source of truth for migration step 38 and offline upgrade tooling: `canonical_{nodes,edges}.properties`, inert `scope` columns (`canonical_nodes`, `canonical_edges`, `operational_mutations`, `operational_state`), `_fathomdb_projection_registry.{entity_class, vector_kinds}`, `canonical_property_values` with 4 B-tree indexes (`_cursor_idx`, `_text_idx`, `_num_idx`, `_bool_idx`), `_fathomdb_commit_timestamps` + `_ms_idx`, `canonical_nodes_logical_sup_idx`, `canonical_edges_{from,to}_{active,sup_kind}_idx`, and `visibility_coalescing_trigger_sql!` for `canonical_property_values` (`VISIBILITY_TRIGGER_TABLES` `19 -> 20`).
   - Updated `frozen_read.rs`, `projection_generation.rs`, `stored_coverage.rs`, `ROW_OWNED_PROJECTIONS`, `NON_PROJECTION_CURSOR_TABLES`, and `docs/compatibility/index.md` in lockstep.
2. **Layer 2 — Property Types, Codec & Workspace Lockstep (`3da57de2e`):**
   - Added `fathomdb-engine/src/properties.rs` (`PropertyValue`, `PropertyMap`, `PropertyEntityClass`, `PropertyComparisonOp`, `PropertyPredicate`, `OperationalCollectionInfo`, canonical sorted-key JSON codec, `-0.0 -> 0.0` canonicalization, exact `i64`/`f64` boundary comparison with `proptest` suites) and extended all 13 workspace crates in lockstep.
   - Extracted Inspector `P4-13` fixture helpers in `src/inspector/tests/fixtures/mod.rs`.
3. **Layer 3 — Storage, Declared Indexes, Commit Timestamps, `vec0` 8-Slot Budget & `R1` Vector Kinds (`644333afb`):**
   - Implemented inline property persistence on `PreparedWrite::{Node,Edge}` and `actuate` (including canonical `request_digest` hashing), `_fathomdb_commit_timestamps` monotonic batch recording strictly when `next_cursor > base_cursor`, O(1) read hydration across `read`, `search`, `rerank`, `fusion`, `graph_expand`, and `evidence`, and `canonical_property_values` projection/supersession/lifecycle/rebuild/boot maintenance.
   - Enforced `MAX_VEC0_NODE_PROPERTY_SLOTS = 8` with indexed `INNER JOIN canonical_property_values` exact pre-selection for non-pushable predicates and slot 9+ overflow, plus explicit opt-in `_fathomdb_vector_kinds` (rejecting `"edge_fact"` and preserving `tc67`).
4. **Layer 4 — Operational Collections, W12 Erasure Fixes 3a/3b/3c & Integrity (`947398398`):**
   - Added `read_operational_collections` / `read_operational_collection` across Engine, Facade, and SDK, plus `AdminSchema` kind-conflict refusal (`WriteValidation`).
   - Landed W12 Fix 3a (Autonomous two-truncate ordering in `erasure.rs` & `physical_maintenance.rs`), Fix 3b (`completed_pending_erasure` + cross-source `excise_source_audit` guard), Fix 3c (always-on lock-free idempotent `AtomicBool::swap` WAL attribution counters), `export_manifest.rs` `ExportClass` completeness audit, `physical_erasure/admission.rs` byte accounting, and `data_plane_integrity.rs` entity-class-aware `cpv` + custom-kind integrity checks.
5. **Layer 5 — Python & TypeScript Bindings, Governed Surface `55/55`, Pin Provenance & Interface Docs (`36802ba4e`, `9ee29cd3e`):**
   - Delivered full Python (`fathomdb-py` + `src/python/`, owned noneditable wheel, `hypothesis` tests, `_fathomdb.pyi` SHA-256 reverse-patch ledger) and TypeScript (`fathomdb-napi` + `src/ts/src/properties.ts` lossless `bigint` codec) bindings.
   - Updated governed surface parity (`53 -> 55` live ops, `87 -> 91` allowlist, preserving 5 `core` verbs and 5 `recovery_denylist` names) and pinned `36802ba4e` in `scripts/governed-surface-pin.json`; regenerated `docs/generated/sdk-operation-map.md` and updated `dev/interfaces/{rust,rust-sdk,python,typescript,wire}.md`.
6. **Review & Regression Remediations (`1c61aea20` `FIX-1`, `975637f3c` `FIX-2`):**
   - Replaced correlated `EXISTS` with indexed `INNER JOIN canonical_property_values` in `build_vector_phase1_plan` with `EXPLAIN QUERY PLAN` pins; canonicalized `-0.0 -> 0.0` in `property_value_to_eav_string`; gated custom `vector_kinds` enrolment on `dense_arm_live`; pruned dense rows on custom `vector_kinds` shrink/drop with clean `doctor data-plane-integrity`; validated `propertyPredicates` across TS evidence/trace/graph and Python graph FFI boundaries with aligned 64-predicate parity; aligned `actuate` invalid-properties `WriteValidation` across SDKs; corrected interface docs to real `STEP_38_SQL` names; shrunk `EdgeRow` in `graph_expand/execution.rs` so `slice60_fix4_rss` passes unmodified; fixed explicit self-loop direction classification; updated Schema 38 / ADR `#73` assertions in `ep03_degradation_bytes`, `pr_g8_dangling_edges`, `slice19_join_indexes`, and `slice70_native_eligibility`.

## 2. Verification Summary

- **Rust:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` (0 warnings), `slice38_properties_and_indexes` (7/7), `slice38_query_plans` (2/2), `slice38_vector_kinds` (4/4), `slice38_erasure_and_audit` (5/5), `slice38_op_collections` (engine + sdk), `slice55_data_plane_integrity` (66/66), `slice60_graph_expand` (19/19), `slice60_fix4_rss` (1/1), `slice60_fix5_rss` (1/1), `slice70_native_eligibility` (10/10), `ep03_degradation_bytes` (1/1), `pr_g8_dangling_edges` (8/8), `slice19_join_indexes` (3/3), `stored_coverage_tests` (17/17), `fathomdb-inspector` (66/66).
- **Python:** Owned wheel built via `scripts/setup-python-owned.sh --build`; `test_slice38_properties.py` (6/6), `test_slice130_package_boundary.py`, `test_sdk_surface_parity_oracle.py`, full `pytest` (1,784+ passed), `ruff check`, `pyright` (0 errors, 0 warnings).
- **TypeScript:** `npm run build` + full `npm test` (**650/650 passed**) + `slice38-properties.test.js` (6/6).
- **Governance & Docs:** `./scripts/check-governed-surface-pin.sh` PASS, `python3 scripts/check-public-doc-truth.py` PASS, `./scripts/agent-lint-md.sh` PASS, `./scripts/agent-lint-docs.sh` PASS, `dev/acceptance.md` untouched, zero external consumer-project leaks.
- **Adversarial Code Review:** Independent `slice-reviewer` returned **`APPROVE`** at HEAD `975637f3c` (0 open P0/P1/P2/P3 findings).
