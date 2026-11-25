---
title: Slice 20 Plan — 0.8.32 Scope Isolation, Schema 39, TABLE_MANIFEST, and Engine Refactor
status: APPROVED
target_release: 0.8.32
slice: 20
date: 2026-11-24
---

# Slice 20 Plan — `SCOPE-AND-REFACTOR`

- **Target branch / worktree:** `dev/0.8.32` at `/mnt/data/worktrees/release-0.8.32` (never mutate `/home/coreyt/projects/fathomdb`).
- **Requirements covered:** `R32-01` (engine-enforced caller scope & write authority, Schema 39 `S39-COMPOSITE` + 8-col `vec0` + verified `fathomdb migrate 38 -> 39`, single-scope/single-agent default), `R32-06` (declarative `TABLE_MANIFEST`, `sqlite_master` + `ExportClass` audit, scope-aware erasure), `R32-07` (zero-behavior-change test relocation + `search.rs`/`evidence.rs` splits).
- **Authoritative ADRs & rulings:**
  - `dev/adr/ADR-0.8.32-caller-scope-isolation.md` (`#102`, accepted 2026-11-24 — `Gate1-D1` Option 1)
  - `dev/plans/0.8.32/features/slice-10/scope-schema-experiment.md` (`WP-10D` empirical baseline)
  - `dev/plans/release-state-0.8.32.json` (`Gate1-D1`, `Gate1-D4` Pin A pre-authorization, `core = 5` and `recovery_denylist = 5` fixed)
  - `dev/design/cross-cutting-change-checklist.md`
- **Hard invariants:**
  - Single-scope (one agent per database, `scope = ""`) is the default posture and MUST be explicitly stated in user-facing `docs/` (`0.8.33` tracks revisiting/relaxing that direction).
  - `TC-11` (`check_migration_logical_id_pin`): `STEP_39_SQL` MUST NOT execute any SQL writing into `logical_id` on `canonical_nodes` or `canonical_edges` (no table recreate on those two tables — `DROP INDEX` + `CREATE [UNIQUE] INDEX` only).
  - `T2` tripwire constants (`RRF_K = 30.0`, `RRF_WEIGHT_TEXT = 3.0`, `RRF_WEIGHT_VECTOR = 1.0`, `DEFAULT_EMBEDDER_NAME = "fathomdb-bge-small-en-v1.5"`, `MAX_SEQUENCE_TOKENS = 512`) MUST remain untouched.
  - Zero external consumer-project specifics in any tracked file.
  - TDD first (`failing test -> red -> green -> refactor`) across all work packages; no `--no-verify` / `--no-gpg-sign`; no `prettier` or unguarded `markdownlint-cli2 --fix` on `.md`.

---

## 1. Work Package Decomposition (Strictly Sequenced Isolated Worktrees)

Because `WP-20A` restructures `fathomdb-engine/src/`, `WP-20B` changes `fathomdb-schema` + `TABLE_MANIFEST` + `vector_storage.rs`, `WP-20C` threads `ScopePolicy`/`ScopePredicate` across every engine module, `WP-20D` wires Pin A through Rust/Python/TS SDKs + CLI/MCP, and `WP-20E` lands docs/interfaces, the 5 packages execute **sequentially in fresh isolated worktrees** cut off `dev/0.8.32` under `/mnt/data/worktrees/` (one writer per checkout; merge & verify each package before cutting the next).

---

### WP-20A — Zero-Behavior-Change Test Relocation & `search.rs` / `evidence.rs` Splits (`R32-07`, `P2-01`, `P2-02`)

**Strict zero-behavior-change rule:** zero schema, API, or behavioral changes in `WP-20A`.

1. **Relocate all 10 top-level `*_tests.rs` / `slice*_tests.rs` files from `src/rust/crates/fathomdb-engine/src/` into `src/rust/crates/fathomdb-engine/src/unit_tests/`:**
   - Files: `stored_coverage_tests.rs`, `provider_admission_tests.rs`, `slice20_fix1_tests.rs`, `slice20_open_hook_tests.rs`, `slice90_close_tests.rs`, `slice90_concurrent_close_tests.rs`, `slice90_close_review_tests.rs`, `slice90_post_probe_real_error_tests.rs`, `visibility_coalescing_tests.rs`, `visibility_admission_tests.rs` (keeping `slice30_observer.rs` in place as runtime feature-gated code).
   - **Preserve `lib.rs:912` `mod tests {` untouched:** host the 10 relocated modules under `#[cfg(test)] mod unit_tests;` (`src/unit_tests/mod.rs` + `src/unit_tests/*.rs`) so `test_slice90_root_reconciliation.py:101` (`source.index("mod tests {")`), `test_windows_wal_attribution_ci_job.sh:493,989-990` (`sed '/^mod tests {$/,$d'`), `.github/workflows/ci.yml`, and `lib.rs:2171,2909` (`--exact "tests::wal_attribution_*"`) remain 100% intact.
   - Update `src/unit_tests/slice20_open_hook_tests.rs` hardcoded `--exact` subprocess test selectors (`"slice20_open_hook_tests::..."` -> `"unit_tests::slice20_open_hook_tests::..."`) and adjust `use super::` imports to `use crate::` where required.
   - Update gate scripts, policy, & root-reconciliation blob in lockstep:
     - `scripts/tests/slice65_managed_connection_owners.py:77-85`: exclude `src/unit_tests/*.rs` **ONLY when `"unit_tests" in test_modules`** (preserving the load-bearing `#[cfg(test)]` gate so stripping `#[cfg(test)]` from `mod unit_tests;` immediately flags `unit_tests/slice20_fix1_tests.rs` as production code!).
     - `scripts/tests/test_windows_wal_attribution_ci_job.sh:11,784,1132,1134,1145`: update `--override "unit_tests/slice20_fix1_tests.rs=$SLICE20_FIX1_SOURCE"` and mutate `original = "#[cfg(test)]\nmod unit_tests;"` (preserving the ungated-module mutation check).
     - `scripts/tests/test_slice90_reader_pool_size_owner.py:17`: update `slice90_close_review_tests.rs` path/import assertion to `unit_tests/slice90_close_review_tests.rs`.
     - `dev/plans/0.8.27/features/slice-90/phase3-root-reconciliation.md`: recompute and update `lib_rs_blob:` SHA-1 after modifying `lib.rs` (keeping `Engine` at `49` fields — `test_slice90_root_reconciliation.py:125-139` enforces exact `lib_rs_blob` match!).
     - `dev/tools/module-boundary-policy.txt`: replace removed top-level test module entries with `unit_tests` and `unit_tests::*`, and update `unparsed-macro` entry for `unit_tests::visibility_coalescing_tests`.
2. **Split `src/rust/crates/fathomdb-engine/src/search.rs` (`3,847` lines) using Rust 2021 module style (`src/search.rs` + `src/search/*.rs`):**
   - Keep `src/search.rs` as the thin module root (`<= 500` lines) declaring `SearchReaderWork`, `FrozenQueryRuntime`, `EvidenceCapture`, `SearchReaderError` (preserving `module-boundary-policy.txt` `owner` rules), keeping `open_historical_property_corpus` (preserving `slice65_managed_connection_owners.py:41` and `test_windows_wal_attribution_ci_job.sh:843,850,853`), and keeping literal `use crate::projection_runtime::PROJECTION_CURSOR_KEY;` and `use crate::write_commit::load_next_cursor;` (preserving `test_slice90_projection_runtime_status_owner.py:90-96` and `test_slice90_write_cursor_owner.py:17-26`), while delegating logic to cohesive submodules:
     - `src/search/projected_text.rs` (projected text + historical projected text + cohort prefilter)
     - `src/search/bm25f.rs` (BM25F tokenization & scoring)
     - `src/search/graph_arm.rs` (`bfs_graph_arm_candidates` + phase-2 query)
     - `src/search/fts_fast_path.rs` (FTS5 rank stream, deferred hydration/identity, node/edge text paths)
     - `src/search/vector_arm.rs` (vector phase1/2, rowid hydration, soft fallback)
     - `src/search/fusion_stage.rs` (`read_search_in_tx` orchestration, fusion/rerank/explain/hydration)
     - Keep existing `src/search/{paired50_tests,ranking_tests}.rs`.
   - Ensure `src/rust/crates/fathomdb-engine/tests/slice35_virtual_mutation_manifest.rs` scans the split `search/*.rs` files if any virtual-table SQL moves out of `src/search.rs`.
3. **Split `src/rust/crates/fathomdb-engine/src/evidence.rs` (`3,628` lines) using Rust 2021 module style (`src/evidence.rs` + `src/evidence/*.rs`):**
   - Keep `src/evidence.rs` as the thin module root (`<= 700` lines) holding the public V1 DTOs (`lib.rs` re-exports unchanged) and keeping `pub fn explain_graph_evidence_preflights_for_test` (preserving `test_slice90_final_observation_test_seams_owner.py:15-20`), while delegating logic to:
     - `src/evidence/ranked.rs` (`build_search_result`, `resolve`, `resolve_many_raw`, `resolve_single_ref`)
     - `src/evidence/graph.rs` (`INTRINSIC_*_PREFLIGHT_SQL`, `GraphEvidenceAuthority`, `preflight_graph_evidence`, `mint_graph_evidence_reference`, `resolve_graph_evidence`)
     - `src/evidence/codec.rs` (binary token/payload codec & byte-span slicing)
   - **Rewrite `src/evidence.rs:5-10` root-reexport imports** (`ArtifactRevisionId`, `CanonicalHash`, `FrozenReadContextV1`, `LifecycleState`, etc.) directly to their semantic owner modules before marking `evidence` and `evidence::*` as `governed` in `dev/tools/module-boundary-policy.txt` (satisfying `module-boundary-gate/src/main.rs:663-674` no root-reexport indirection rule; zero `crate::*`/`super::*` globs; updated `unparsed-macro` hash if `evidence::tests` moves; zero stale entries).
4. **WP-20A Verification Gate:**
   - `bash scripts/check-module-boundaries.sh`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --locked -p fathomdb-engine` (including `unit_tests::slice20_open_hook_tests`, `tests::wal_attribution`, `slice35_virtual_mutation_manifest`, `pr_g9_rrf_fusion`, `fusion` T2 tripwires)
   - `bash scripts/tests/test_module_boundary_gate.sh`, `python3 scripts/tests/slice65_managed_connection_owners.py`, `bash scripts/tests/test_windows_wal_attribution_ci_job.sh`, `python3 scripts/tests/test_slice90_root_reconciliation.py`, `python3 scripts/tests/test_slice90_reader_pool_size_owner.py`, `python3 scripts/tests/test_slice90_final_observation_test_seams_owner.py`, `python3 scripts/tests/test_slice90_search_api_test_seams_owner.py`, `python3 scripts/tests/test_slice90_projection_runtime_status_owner.py`, `python3 scripts/tests/test_slice90_write_cursor_owner.py`, `python3 scripts/tests/test_slice90_operator_integrity_owner.py`, `python3 scripts/tests/test_slice90_operator_diagnostics_owner.py`, `python3 scripts/tests/test_slice135_coverage_overlay.py`, `python3 scripts/tests/module_boundary_gate_qualification.py`.

---

### WP-20B — Single-Source `TABLE_MANIFEST`, Schema 39 (`S39-COMPOSITE`), 8-Col `vec0`, & `fathomdb migrate 38 -> 39`

1. **Single-source declarative `TABLE_MANIFEST` (`fathomdb-schema/src/lib.rs`):**
   - Define `pub enum TableExportClass { DurableAuthority, OptInAudit, OptInVector, DerivedProjection, InternalRuntime }` and `pub struct TableManifestEntry` (`name`, `is_canonical`, `visibility_trigger_prefix: Option<&'static str>`, `visibility_trigger_min_version: Option<u32>`, `row_owned_cursor_column: Option<&'static str>`, `has_non_projection_write_cursor: bool`, `export_class: TableExportClass`, `guard_canonical_custom_triggers: bool`, `guard_projection_custom_triggers: bool`, `physical_erasure_metadata: bool`).
   - Populate `pub const TABLE_MANIFEST: &[TableManifestEntry]` covering all 30 Schema-39 named tables (plus shadow-table suffix helpers for FTS5 and `vec0`).
   - Refactor all 7 fragmented inventories + sibling to derive directly from `TABLE_MANIFEST`:
     1. `CANONICAL_TABLES` (`fathomdb-schema/src/lib.rs`)
     2. Visibility trigger table lists (`fathomdb-schema/src/lib.rs` & `fathomdb-engine/src/frozen_read.rs`; expected trigger count derived from `TABLE_MANIFEST.iter().filter(|t| t.visibility_trigger_prefix.is_some()).count() * 3`)
     3. `ROW_OWNED_PROJECTIONS` (`fathomdb-engine/src/projection_registry.rs`)
     4. Non-projection `write_cursor` table guards (`fathomdb-engine/src/lib.rs`)
     5. `export_manifest.rs` (`classify_schema_table` / `audit_schema_sqlite_master`, keeping `audit_schema_38_sqlite_master` or adding `audit_schema_39_sqlite_master` while updating `slice38_erasure_and_audit.rs` to assert Schema 39 completeness + zero drift against `TABLE_MANIFEST`)
     6. `write_commit.rs` (`canonical_batch_has_no_custom_triggers`) & `projection_commit.rs` (`projection_batch_has_no_custom_triggers`) — closing the 7+ missing-table gap
     7. `physical_erasure/admission.rs` metadata table list
2. **Schema 39 (`S39-COMPOSITE`, `SCHEMA_VERSION = 39`) in `fathomdb-schema/src/lib.rs`:**
   - Bump `pub const SCHEMA_VERSION: u32 = 39;` and add step 39 (`STEP_39_SQL`, annotated with `-- MIGRATION-ACCRETION-EXEMPTION:` per `agent-lint-migrations.py` / `check_migration_accretion`) to `MIGRATIONS`:
     - **`canonical_nodes` (NO table recreate — `TC-11` safe):** `DROP INDEX` old Schema-38 single-column/partial indexes (`source_id`, `logical_active`, `kind`, `state_active`, `validity`, `kind_cursor_page`, `logical_sup`); keep `canonical_nodes_write_cursor_idx (write_cursor)`; create the exact `WP-10D` `S39-COMPOSITE` indexes (`_scope_source_id_idx`, `UNIQUE _scope_logical_active_idx (scope, logical_id) WHERE superseded_at IS NULL`, `_scope_kind_idx`, `_scope_state_active_idx (scope, write_cursor) WHERE state = 'active' AND superseded_at IS NULL`, `_scope_validity_idx`, `_scope_cursor_idx`, `UNIQUE _scope_kind_cursor_page_idx (scope, kind, write_cursor) WHERE logical_id IS NOT NULL`, `_scope_logical_sup_idx`).
     - **`canonical_edges` (NO table recreate — `TC-11` safe):** `DROP INDEX` old Schema-38 non-cursor indexes; keep `canonical_edges_write_cursor_idx (write_cursor)`; create the exact `WP-10D` `S39-COMPOSITE` indexes (`_scope_source_id_idx`, `UNIQUE _scope_logical_active_idx`, `_scope_from_id_idx`, `_scope_to_id_idx`, `_scope_cursor_idx`, `_scope_from_active_idx`, `_scope_to_active_idx`, `_scope_from_sup_kind_idx`, `_scope_to_sup_kind_idx`).
     - **`operational_state`:** Recreate with `PRIMARY KEY(scope, collection_name, record_key)`, `UNIQUE INDEX operational_state_scope_collection_cursor_page_idx (scope, collection_name, write_cursor)`, `operational_state_write_cursor_idx (write_cursor)`, and restore all 3 `read_visibility_coalescing` triggers (`os_*`).
     - **`operational_mutations` (NO table recreate):** `DROP INDEX operational_mutations_collection_id_idx` + `CREATE INDEX operational_mutations_scope_collection_id_idx ON operational_mutations(scope, collection_name, id)`.
     - **`_fathomdb_actuation_receipts` & `_fathomdb_actuation_receipt_source_refs`:** Add `scope TEXT NOT NULL DEFAULT ''` to both tables:
       - `_fathomdb_actuation_receipts`: composite `PRIMARY KEY(scope, operation_id)`.
       - `_fathomdb_actuation_receipt_source_refs`: composite `PRIMARY KEY(scope, operation_id, ref_kind, ref_value)`, composite `FOREIGN KEY(scope, operation_id) REFERENCES _fathomdb_actuation_receipts(scope, operation_id) ON DELETE CASCADE`, composite reverse index `_fathomdb_actuation_receipt_refs_reverse ON _fathomdb_actuation_receipt_source_refs(scope, ref_kind, ref_value, operation_id)`.
       - Update trigger `_fathomdb_actuation_ref_owner_before_insert` to check `WHERE scope = NEW.scope AND operation_id = NEW.operation_id AND outcome != 'erased'`.
     - **Provenance, dependency-registry, dependency-closure, & erasure-queue tables (closing all cross-scope collision/leak vectors on shared `source_id`, `source_version_id`, `artifact_revision_id`, `dependency_id`, & `idempotency_key`):**
       - `_fathomdb_artifact_revisions`: recreate with `scope TEXT NOT NULL DEFAULT ''` and composite `PRIMARY KEY(scope, revision_id)`, **explicitly restoring `_fathomdb_read_visibility_ar_{ai,au,ad}` triggers** and scoped indexes; update `data_plane_integrity.rs:475-496` to assert `scope` equality across revision, canonical node/edge, and source link.
       - `_fathomdb_source_versions`: recreate with `scope TEXT NOT NULL DEFAULT ''`, composite `PRIMARY KEY(scope, source_id, source_version_id)`, and composite **`UNIQUE(scope, source_revision_id)`** (removing global column-level `UNIQUE` on `source_revision_id`), **explicitly restoring `_fathomdb_read_visibility_sv_{ai,au,ad}` triggers**.
       - `_fathomdb_source_links`: recreate with `scope TEXT NOT NULL DEFAULT ''`, composite **`PRIMARY KEY(scope, artifact_revision_id)`**, and scoped lookup indexes (`(scope, source_revision_id, artifact_revision_id)`), **explicitly restoring `_fathomdb_read_visibility_sl_{ai,au,ad}` triggers**.
       - `_fathomdb_source_dependencies`: recreate with `scope TEXT NOT NULL DEFAULT ''`, composite **`PRIMARY KEY(scope, dependency_id)`**, composite **`UNIQUE(scope, derived_revision_id)`**, and scoped lookup indexes, **explicitly restoring `_fathomdb_read_visibility_sd_{ai,au,ad}` triggers**.
       - `_fathomdb_dependency_closures`: recreate with `scope TEXT NOT NULL DEFAULT ''` included in the composite PK/indexes, **explicitly restoring `_fathomdb_read_visibility_dc_{ai,au,ad}` triggers and all 3 closure indexes** (`_root`, `_recovery`, and `_active_retry` with `scope` in both `retry_fingerprint` and the `UNIQUE` index) so `root_kind = 'source_bucket'` closures never cross scopes or collide on shared `source_id`.
       - `_fathomdb_erasure_requests`: recreate with `scope TEXT NOT NULL DEFAULT ''` and composite `UNIQUE(scope, idempotency_key)` (instead of bare `ALTER TABLE`, eliminating cross-scope `IdempotencyConflict`).
   - **Branch `frozen_read.rs:485-510` index DDL checks on `schema_version`:**
     - `if schema_version >= 39`: validate `canonical_nodes_scope_kind_cursor_page_idx` and `operational_state_scope_collection_cursor_page_idx`.
     - `else if schema_version >= 33`: retain existing Schema `33..=38` checks untouched.
   - Update existing `SCHEMA_VERSION == 38` tests (`slice45_step33.rs`, `step38_property_schema.rs`, `unit_tests/stored_coverage_tests.rs:101-106`), update `phase3-root-reconciliation.md` `lib_rs_blob` if `lib.rs` changes, and add TDD `fathomdb-schema/tests/step39_scope_schema.rs` verifying `SCHEMA_VERSION == 39`, `MIGRATIONS.len() == 39`, `TC-11`, `check_migration_accretion`, `TABLE_MANIFEST`-derived trigger count, and `sqlite_master` completeness.
3. **Mandatory 8-column `vec0` (`scope TEXT`) across `vector_storage.rs`, `projection_commit.rs`, `mean.rs`, and `filter.rs`:**
   - Update `vector_partition_create_sql` (`vector_storage.rs:79-99`) to emit `scope TEXT` as the 8th built-in metadata column (`MAX_VEC0_NODE_PROPERTY_SLOTS = 8` preserved; 16 total metadata columns, matching `vector_storage.rs:93-96` having no `DEFAULT` clauses on `vec0` columns; verify DDL against `sqlite-vec v0.1.9` and reconcile `ADR-0.8.32` prose from `scope TEXT DEFAULT ''` to `scope TEXT` in `WP-20B`).
   - Update `ensure_vector_partition` to recognize 7-col legacy vs 8-col Schema-39 partitions cleanly during migration/reshape, and fix `reshape_vector_partition_nondestructive` (`vector_storage.rs:305-451`) to:
     - detect missing `scope` column and trigger 7-to-8-column non-destructive reshape even when `desired_attr_cols == actual_attr_cols`;
     - when upgrading `7 -> 8` columns, resolve `COALESCE(cn.scope, ce.scope, '')` by joining **both** `canonical_nodes cn` and `canonical_edges ce` on `vr.write_cursor` (closing the `canonical_edges` join gap);
     - when reshaping an already-8-column partition (`attr_*` change), copy `s.scope` directly without resetting to `''`.
   - Fix `projection_commit.rs:399-445` (`INSERT INTO vector_default` with `scope`), `mean.rs:151-219` (`run_pin_and_requantize_pass` reading and re-inserting `scope` so requantize never resets `scope` to `''`), all internal test fixtures writing to `vector_default` (`open.rs`, `physical_erasure.rs`, `filter.rs`, `consolidation.rs`, `vector_storage.rs`), and sync `fathomdb-engine/tests/slice35_virtual_mutation_manifest.rs:125`.
4. **Golden `0.8.31` Schema 38 fixture & verified offline `fathomdb migrate` (`38 -> 39` alongside `37 -> 38` per `ADR-0.8.32:88`):**
   - Copy `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.31-schema38.sqlite` into tracked `dev/fixtures/golden-0.8.31-schema38.sqlite` (verify `user_version = 38`, 7-col `vec0` with node + edge vectors, record SHA-256).
   - **Keep `migrate_37_to_38` intact** (preserving `fathomdb-engine/src/lib.rs:310-319` `arm_migrate_37_to_38_before_version_stamp_hook_for_test` and `fathomdb/src/lib.rs:288-293` `_no_migrate_37_to_38` `compile_fail` doctest — having `migrate_37_to_38` verify `validate_recovery_schema_invariants(&connection, 38)` cleanly without asserting Schema-39 `admit_current_database`). Explicitly update `fathomdb-engine/tests/slice50_migrate_37_to_38.rs` for `SCHEMA_VERSION = 39` (`supported: 39` at `:63`; use a true Schema-38 fixture for `AlreadyCurrent` at `:101-106`; chain `migrate_38_to_39` after `migrate_37_to_38` before calling `Engine::open` at `:335` and before Schema-39 `sqlite_master` structural parity at `:390-439`).
   - **Add `migrate_38_to_39` alongside `migrate_37_to_38`** in `fathomdb-engine/src/operator/migrate.rs` (backup `{stem}.schema38.{utc}-{pid}.sqlite`, preflight checks, single `BEGIN IMMEDIATE` `STEP_39_SQL` + 7-to-8-col `vec0` reshape preserving node & edge vectors, `validate_recovery_schema_invariants(&connection, 39)`, and `admit_current_database` passing Schema-39 `Engine::open` + `check_integrity(full = true)`), plus `arm_migrate_38_to_39_before_version_stamp_hook_for_test` and `_no_migrate_38_to_39` `compile_fail` doctest in `fathomdb/src/lib.rs`.
   - Wire `fathomdb-cli/src/lib.rs:run_migrate` to inspect `PRAGMA user_version`: dispatch `38 -> migrate_38_to_39`, dispatch `37 -> migrate_37_to_38` then `migrate_38_to_39` (reporting `from_version: 37, to_version: 39` with preserved original pre-migration backup!), and refuse any other version cleanly with `UnsupportedSchemaVersion`.
   - Update `scripts/tests/slice65_managed_connection_owners.py:52-55` `EXPECTED_RAW_OPENS` for `migrate_38_to_39`.
   - Add `fathomdb-engine/tests/slice20_migrate_38_to_39.rs` and update `fathomdb-cli/tests/migrate_cli.rs` to verify:
     - upgrading `dev/fixtures/golden-0.8.31-schema38.sqlite` (`38 -> 39`) preserves all 6 nodes, 3 edges, 7 node+edge vectors (in 8-col `vec0` with `scope = ''`), properties, and passes full `check_integrity`;
     - upgrading `dev/fixtures/golden-0.8.30-schema37.sqlite` via `fathomdb migrate` succeeds cleanly (`37 -> 39`) and passes full Schema-39 `Engine::open` + `check_integrity`;
     - already-at-39 idempotent refusal, unsupported version refusal (`seen_version: 36`), and backup SHA-256 verification.
5. **WP-20B Verification Gate:**
   - `cargo test --locked -p fathomdb-schema -p fathomdb-engine -p fathomdb-cli -p fathomdb --doc`
   - `bash scripts/agent-lint-migrations.sh`, `bash scripts/check-module-boundaries.sh`, `python3 scripts/tests/slice65_managed_connection_owners.py`, `python3 scripts/tests/test_slice90_root_reconciliation.py`, `cargo clippy --workspace --all-targets -- -D warnings`.

---

### WP-20C — Engine `ScopePolicy`, `ScopeViolation`, `search_expand` `ReadView`, & Full Route-by-Route `scope` Threading (`R32-01`)

1. **Domain types, validation, typed `ScopePredicate` helper, & error (`src/scope.rs`, `runtime_configuration.rs`, `errors.rs`, `open.rs`, `lib.rs`):**
   - Create `src/rust/crates/fathomdb-engine/src/scope.rs` (registered as `governed` in `dev/tools/module-boundary-policy.txt`) owning:
     - `pub const MAX_READ_SCOPES: usize = 32;` and `pub const MAX_SCOPE_TOKEN_BYTES: usize = 128;`.
     - `ScopePolicy`:
       - `ScopePolicy::Unscoped` (`Default`) — single-scope / single-agent mode binding `scope = ""` for both reads and writes.
       - `ScopePolicy::Scoped { read_scopes: Vec<String>, write_scope: Option<String> }` with ergonomic constructors (`ScopePolicy::single(scope)`, `ScopePolicy::read_only(read_scopes)`, `ScopePolicy::read_write(read_scopes, write_scope)`).
       - Strict fail-closed validation at `Engine::open` (`EngineOpenError::EngineConfiguration`, before DB/lock/worker side effects): `read_scopes` cardinality MUST be `1..=MAX_READ_SCOPES` (`0` or `> 32` rejected); every token MUST be non-empty, `<= 128` UTF-8 bytes, zero NUL/ASCII control bytes (`0x00..=0x1F`, `0x7F`), zero `\x1e`, no leading/trailing whitespace, and wildcard `"*"` **forbidden on all scope tokens (read and write)** (reconcile `ADR-0.8.32:37` prose to match); if `write_scope` is `Some(w)`, `w` MUST be present in `read_scopes`. Canonicalize `read_scopes` (dedup + sort) so digests/bindings are deterministic.
     - Shared typed **`ScopePredicate`** SQL builder (`append_sql(col, params)` / `is_unscoped()` / `read_scopes()` / `write_scope()`) used uniformly across `filter.rs`, `read.rs`, `search/*`, `graph_expand/*`, `evidence/*`, `write_commit.rs`, `actuation.rs`, `erasure.rs`, `erasure_queue.rs`, `projection_generation.rs`, and `dependency_trace.rs` — guaranteeing single-scope queries **always** emit `col = ?` bound to `""` (`ADR-0.8.32:88`) with zero ad-hoc parameter-offset drift.
   - **Keep `Engine` at `49` fields, `engine.config() == &requested` unmodified, and `lib.rs` `ROOT_METHODS` untouched (`test_slice90_root_reconciliation.py:86,133`, `slice90_engine_configuration.rs:39`):**
     - Add `pub scope_policy: ScopePolicy` to `EngineConfig` (keeping `EngineConfig` non-`#[non_exhaustive]` per `dev/interfaces/rust.md:93-95`, adding `EngineConfig::with_scope_policy` builder helper, and updating all workspace literal constructions in lockstep including `slice90_engine_configuration.rs:21-32`, `fathomdb-py/src/engine.rs:76-138`, `fathomdb-napi/src/admin.rs:155-244`, and `test_support.rs`).
     - Keep `requested_config: EngineConfig` completely unmutated so `assert_eq!(opened.engine.config(), &requested)` always holds; store the validated & canonicalized `ScopePolicy` inside existing `resolved_config: ResolvedRuntimeConfiguration` (`lib.rs:549`, zero new `Engine` struct fields!).
     - Place `impl Engine { pub fn scope_policy(&self) -> &ScopePolicy }` in `src/scope.rs` (outside `lib.rs`, matching `search_api.rs`/`read_api.rs` and keeping `lib.rs` `ROOT_METHODS` unchanged; update `phase3-root-reconciliation.md` `lib_rs_blob` only for module/re-export lines in `lib.rs`).
   - Add `EngineError::ScopeViolation { detail: String }` in `fathomdb-engine/src/errors.rs` (no `stable_code` method on `EngineError`; `"FDB_SCOPE_VIOLATION"` is defined in `fathomdb-napi/src/errors.rs` and `src/ts/src/errors.ts`), reserved strictly for explicit caller-authority violations (e.g., mutating on a read-only `ScopePolicy` handle, never as a cross-scope existence oracle).
   - Add `ReadView` parameter to `Engine::search_expand` / `search_expand_with_limit` (`search_api.rs:1752-1801`, defaulting to `ReadView::default()` when `None`).
2. **Thread `ScopePredicate` across EVERY data-plane route:**
   - **Read (`read.rs`, `graph_expand/traversal.rs`):** `read_get_by_id_in_tx`, `read_list_in_tx`, `read_collection_in_tx`, `read_operational_state_in_tx`, `read_operational_state_page_in_tx`, `read_canonical_page_in_tx` / `canonical_page_query_on`, `crossed_boundary_since_in_tx`. (`operational_collections` remains a DB-global collection-schema registry.)
   - **Search (`search/{fts_fast_path,vector_arm,projected_text,bm25f,graph_arm,fusion_stage}.rs`, `filter.rs`, `temporal.rs`):**
     - **8-col `vec0` (`filter.rs`, `search/vector_arm.rs`):** Single read scope emits `AND scope = ?` directly inside `build_vec0_fast_phase1_sql` (matching `WP-10D`); multi-read-scope (`2..=32` scopes) executes bounded per-scope `AND scope = ?` KNN queries and merges top-K by distance in Rust (avoiding any unverified `sqlite-vec` `IN (...)` behavior while preserving exact `WP-10D` plans!). Scope rowid hydration (`node_stmt`/`edge_stmt`) and the soft-fallback pending-vector probe (`canonical_nodes` scope gate).
     - **Body & Edge FTS (`search/fts_fast_path.rs`, `filter.rs`, `temporal.rs`):**
       - Add `CROSS JOIN` to `edge_fts_rank_sql_view` and `property_fts_rank*_sql` in `filter.rs:1355-1414` (`ADR-0.8.32:52`).
       - Scope `ownerless_existence_sql` (ownerless rows admitted ONLY for `Unscoped` `scope = ''`, never for non-empty scopes).
       - Scope `rank_stream_eligible`, scope `defer_full_hydration`, and **close the `defer_text_identity` leak** (`search/fts_fast_path.rs` — gate `can_defer_text_identity` off unless `Unscoped` on a single-scope-only store, or enforce explicit `cn.scope` predicate).
     - **BM25F (`search/bm25f.rs`):** bind `cn.scope` on `search_index_v2 JOIN canonical_nodes cn` and active cursor set.
     - **Projected text & cohort prefilter (`search/projected_text.rs`):**
       - Scope `smallest_cohort` (`canonical_attributes` joined with `canonical_nodes cn` filtered by `cn.scope`, or bypass cohort shortcut cleanly when non-unscoped so cross-scope attribute counts never distort/leak cohorts).
       - Scope `property_fts_rank*_sql` AND scope `read_historical_projected_text` (join `property_search_index` against `canonical_nodes n` filtered by `n.scope` so BM25 IDF/DF never leaks cross-scope token frequencies; assert `EXPLAIN QUERY PLAN` / bounded latency in tests).
     - **`use_graph_arm` & `graph_seed` (`search/graph_arm.rs`):** bind `scope` across `bfs_graph_arm_candidates` (`active_stmt`, `edge_seed_stmt`, `node_seed_stmt`) and `build_graph_arm_phase2_query`; **key BFS `visited` and `frontier` by `(scope, logical_id)` and enforce `target.scope = e.scope`** so multi-read-scope (`["a", "b"]`) traversal NEVER jumps across scopes on overlapping `logical_id`!
   - **Graph (`graph_expand/execution.rs`, `graph_expand/traversal.rs`, `dependency_closure.rs`):**
     - Key recursive CTE `visited` / `GROUP BY` / frontier state by **`(scope, logical_id)`** and enforce **`next_n.scope = e.scope`** across `graph.neighbors` (`build_bfs_sql`), `graph.search_expand` (`search_expand_on_snapshot`, `build_bfs_with_depth_sql`, threading `ReadView`), and `graph.expand` (`resolve_seeds`, `load_node`, all 6 `graph_expand_incident_sql` templates).
     - Enforce **`AND cur.scope = n.scope`** on `AsOf` tombstone `NOT EXISTS` subqueries in `graph_expand/execution.rs:729-734` and `resolve_seeds` (`1194-1208`) so deleting `"id"` in Scope B NEVER suppresses Scope A's historical view.
     - Thread `scope` through `dependency.rs` (`load_persisted_canonical_source`, `validate_dependency_chain`, `Engine::register_source_dependency` with `write_scope` `ScopeViolation` guard, `Engine::dependencies_for_source`, `Engine::dependency_for_derived`), `dependency_closure.rs` (`read_eligibility_sql*` and `Engine::read_dependency_closure`), and `dependency_trace.rs` (`TO_SOURCE_CANDIDATE_QUERY`, `TO_DEPENDENTS_CANDIDATE_QUERY`, `source_link_valid`, `_fathomdb_dependency_closures`).
   - **Frozen contexts, evidence refs, & cursors (`frozen_read.rs`, `pagination.rs`, `evidence/{ranked,graph,codec}.rs`):**
     - Bind canonical `ScopePolicy` fingerprint into `FrozenReadContextV1` canonical encoding (`encode_binding` / `authenticate` / `validate_snapshot`) — propagating automatically to `PageCursor` via `context_digest`. Mismatched scope handles fail authentication with `EngineError::FrozenRead(FrozenReadError::new(FrozenReadErrorReason::TokenAuthenticationFailed, ...))` (keeping `FrozenReadErrorReason` enum unchanged).
     - Verify caller scope binding + fetched `canonical_nodes`/`canonical_edges` row `scope` AND enforce `AND cur.scope = n.scope` on `AsOf` tombstone checks in `evidence/ranked.rs` (`1320-1325`) and `evidence/graph.rs` (`1709-1716, 1751-1754`); out-of-scope or revoked-grant evidence refs return nondisclosing `EvidenceErrorV1::unavailable()` (`EvidenceErrorReasonV1::EvidenceUnavailable`, keeping `EvidenceErrorReasonV1` enum unchanged and preserving `D-01` nondisclosure).
   - **Writes, provenance, dependency registry, actuation, lifecycle, erasure & status (`write_commit.rs`, `write_validation.rs`, `dependency.rs`, `actuation.rs`, `record_lifecycle.rs`, `erasure.rs`, `erasure_queue.rs`, `projection_generation.rs`, `dependency_closure.rs`):**
     - Enforce `write_scope`: read-only `ScopePolicy` (`write_scope: None`) refuses `write`, `actuate`, `register_source_dependency`, `transition`, `purge`, `erase_source`, `queue_physical_erasure`, and `withdraw_queued_erasure` atomically with `EngineError::ScopeViolation`, leaving DB state unchanged.
     - Stamp `scope` strictly from opener's bound `write_scope` (`""` for `Unscoped`, `<scope>` for `Scoped`) on `canonical_nodes`, `canonical_edges`, `operational_mutations`, `operational_state` (updating `write_commit.rs:974-981` column list + `ON CONFLICT(scope, collection_name, record_key)`!), `_fathomdb_artifact_revisions`, `_fathomdb_source_versions`, `_fathomdb_source_links`, `_fathomdb_source_dependencies`, `_fathomdb_dependency_closures`, `_fathomdb_actuation_receipts`, `_fathomdb_actuation_receipt_source_refs`, and `_fathomdb_erasure_requests`.
     - Scope `write_commit.rs:101-125` (`runtime_revision_id`) and `171-362` (`register_artifact_identity`, verifying `n.scope == write_scope` for `ProvenanceRole::Derived`!) so Scope B can NEVER link derived artifacts to Scope A's source node or probe Scope A's revisions via `SourceRevisionMissing` vs `SourceMismatch`.
     - Scope `write_commit.rs:1047-1054` (`validate_strict_batch_edge_endpoints`), `write_validation.rs` (`validate_prospective_edge_endpoints` & supersession lookups), and `actuation.rs:1705-1732` — **preserving `D-01` nondisclosure and existing contracts:** missing OR cross-scope endpoints return identical `EngineError::WriteValidation` (on `write`) or `ActuationOutcomeV1::Refused { reason: ActuationRefusalReasonV1::ReferenceUnavailable }` (on `actuate`), NEVER `ScopeViolation`!
     - Scope `actuation.rs:1266-1327` (`redact_actuation_receipts_for_refs`), `1425-1495` (`expected_affected_revisions`), and `projection_generation.rs:1775-1839` (`read_mutation_projection_status`) by `scope`.
     - Scope `record_lifecycle.rs` (`Engine::transition`), `erasure.rs:473-556` (`erase_artifact_identity_for_cursors` — delete `_fathomdb_source_links`/`_fathomdb_source_versions` strictly for `scope`), `erasure.rs:905-1088` (`Engine::purge` — scope `canonical_nodes` AND `canonical_edges`), governed `Engine::erase_source`, and `erasure_queue.rs:443-700` (`preview`, `Scope` key hashing caller `scope`, `guard_plain` fencing only matching `scope`, `physical_erasure_status` / `withdraw_queued_erasure` verifying caller `scope`). Operator `excise_source` continues operating at DB operator level.
3. **Comprehensive TDD Test Suite (`src/rust/crates/fathomdb-engine/tests/slice20_caller_scope_isolation.rs`):**
   - Overlapping `logical_id`, `record_key`, `source_id`, `source_version_id`, `artifact_revision_id`, `dependency_id`, `closure_operation_id`, and `operation_id` across `"agent-a"`, `"agent-b"`, `Unscoped` (`""`), and multi-read-scope (`read_scopes: ["agent-a", "agent-b"]`, `write_scope: None`) verifying:
     - Zero cross-scope leakage across every read, search (FTS, 8-col `vec0`, multi-read-scope `vec0` K-merge, hybrid, BM25F, projected text, historical projected text BM25 non-leak + `EXPLAIN QUERY PLAN`, `use_graph_arm`, `graph_seed`), graph (`neighbors`, `expand` Current/`AsOf`, `search_expand` `ReadView`), frozen context (`FrozenReadErrorReason::TokenAuthenticationFailed`), cursors, evidence & graph evidence (`EvidenceErrorReasonV1::EvidenceUnavailable`), dependency registry (`dependencies_for_source`, `dependency_for_derived`, `read_dependency_closure`), and `dependency_trace`.
     - Multi-read-scope BFS/CTE NEVER jumps from `"agent-a"` edge to `"agent-b"` node on shared `logical_id`, and `AsOf` tombstones in `"agent-b"` NEVER hide `"agent-a"` historical rows.
     - Grant revocation invalidates prior frozen contexts, cursors, and evidence refs with nondisclosing errors.
     - Write/provenance/erasure isolation: read-only scope refuses writes atomically (`ScopeViolation`); missing vs cross-scope edge endpoints return identical non-disclosing `WriteValidation` / `ActuationRefusalReasonV1::ReferenceUnavailable`; derived provenance cannot cross scopes; `purge`, `erase_source`, `queue_physical_erasure` (`guard_plain` does not fence Scope B), `redact_actuation_receipts_for_refs`, and `read_mutation_projection_status` stay strictly scoped; `mean.rs` pin/requantize preserves non-empty `scope`.
     - `EXPLAIN QUERY PLAN` `SEARCH`-only assertions on single-scope (`scope = ''`) and multi-scope queries.
     - `proptest` property tests for `ScopePolicy` token grammar, cardinality bounds (`1..=32`), and anti-spoof invariants.
4. **WP-20C Verification Gate:**
   - `cargo test --locked -p fathomdb-engine`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `bash scripts/check-module-boundaries.sh`, `python3 scripts/tests/test_slice90_root_reconciliation.py`.

---

### WP-20D — SDK, Python, TypeScript, CLI & MCP Surface Parity (`Pin A`, `X1`)

Update every registry enumerated in `dev/design/cross-cutting-change-checklist.md` in lockstep:

1. **Rust facade (`fathomdb`), `fathomdb-sdk`, `fathomdb-cli`, `fathomdb-mcp`:**
   - Re-export `ScopePolicy` in `fathomdb` and `fathomdb-sdk` (implementing `with_scope_policy` strictly inside `impl OpenOptions` and `with_view` strictly inside `impl SearchExpandOptions` — never inside `impl Engine` or as top-level `pub fn` in `fathomdb-sdk`, per `check-sdk-surface-parity.py:556-611`; verify `cargo test --doc`).
   - Add `ErrorKind::ScopeViolation` to `fathomdb-sdk/src/error.rs` (`ErrorKind::ALL`: `47 -> 48`, updating `fathomdb-sdk/tests/surface.rs:235` `SHARED_ERROR_CLASSES: [&str; 48]`).
   - Add `pub view: Option<ReadView>` to `fathomdb-sdk/src/options.rs::SearchExpandOptions` (keeping non-`#[non_exhaustive]` with `SearchExpandOptions::with_view` builder and updating all workspace literals) and thread through `fathomdb-sdk/src/graph.rs::search_expand`.
   - Map `EngineError::ScopeViolation` in `fathomdb-cli/src/lib.rs:539-565` (`engine_error_to_outcome` -> `CliOutcome::Unrecoverable`, exit `70`) and `ErrorKind::ScopeViolation` in `fathomdb-mcp/src/error.rs` (`ToolError::from_sdk`, leaving `fathomdb-mcp/src/config.rs` scope activation to Slice 40).
   - Verify `src/conformance/governed-surface-allowlist.json` / `scripts/governed-surface-pin.json` (update ONLY if governed command-verb members change, keeping `core = 5` and `recovery_denylist = 5` strictly fixed per `Gate1-D4`), `src/conformance/governed-operation-parity.json`, and `scripts/check-sdk-surface-parity.py`.
2. **PyO3 (`fathomdb-py`) & Python (`src/python/`):**
   - Add `ScopeViolationError` (`create_exception!`, `engine_error_to_py`, `lib.rs`, `src/python/fathomdb/errors.py`, `__init__.py`, `_fathomdb.pyi`, `test_errors.py`, `test_typed_errors.py`, `test_native_stub_surface.py`).
   - Expose `ScopePolicy` and `EngineConfig.scope_policy` / `Engine.open(..., scope_policy=...)` (`fathomdb-py/src/engine.rs`, `src/python/fathomdb/config.py`, `src/python/fathomdb/_sdk_open.py` `_KWARG_FIELDS`, `_fathomdb.pyi`, `test_surface.py`).
   - Add trailing flat kwarg `view: ReadView | None = None` to `graph.search_expand` (`fathomdb-py/src/graph_evidence.rs:111-125`, `src/python/fathomdb/graph.py`, `_fathomdb.pyi`).
   - Update `src/python/tests/python_surface_additions.json` (new exports, `search_expand` signature, updated `native_stub_sha256` / reverse-patch deltas) so `test_slice130_package_boundary.py` passes green.
   - Add `src/python/tests/test_slice20_scope_isolation.py` testing two-caller Python isolation, `ScopeViolationError`, and `graph.search_expand(view=...)`.
3. **N-API (`fathomdb-napi`) & TypeScript (`src/ts/`):**
   - Add `FDB_SCOPE_VIOLATION` / `ScopeViolationError` (`fathomdb-napi/src/errors.rs`, `src/ts/src/errors.ts`, `src/ts/src/index.ts`, `src/ts/index.d.ts`, `src/ts/tests/{errors,typed-errors}.test.ts`).
   - Expose `ScopePolicy` and `EngineConfig.scopePolicy` / `EngineOpenOptions.scopePolicy` (`fathomdb-napi/src/admin.rs`, `src/ts/src/core.ts`, `src/ts/src/binding.ts`, `src/ts/index.d.ts`, `src/ts/tests/surface.test.ts`).
   - Add optional `view?: ReadView` to `SearchExpandOptions` and `graph.searchExpand` (`fathomdb-napi/src/graph_evidence.rs`, `src/ts/src/search.ts`, `src/ts/src/graph.ts`, `src/ts/index.d.ts`).
   - Add `src/ts/tests/slice20-scope-isolation.test.ts` testing two-caller TS isolation, `ScopeViolationError`, and `graph.searchExpand({ view })`.
4. **Rebuild bindings & verify Pin A parity:**
   - Rebuild checkout-owned Python wheel (`bash scripts/setup-python-owned.sh --build`) and TS native addon (`cd src/ts && npm run build:native && npx tsc --noEmit`).
   - Run `python3 scripts/check-sdk-surface-parity.py`, `bash scripts/check-governed-surface-pin.sh`, `bash scripts/check-release-smoke-sdk-compile.sh`, `pytest` (`test_slice130_package_boundary.py`, `test_sdk_surface_parity_oracle.py`, `test_errors.py`, `test_typed_errors.py`, `test_surface.py`, `test_slice20_scope_isolation.py`), and TS test suite (`sdk-surface-parity.test.ts`, `surface.test.ts`, `errors.test.ts`, `typed-errors.test.ts`, `slice20-scope-isolation.test.ts`).

---

### WP-20E — Interface Contracts, Pre-Publish `0.8.32` Docs & Single-Scope Default Documentation (`X4`, `Gate1-D1`)

*Runs on `/mnt/data/worktrees/release-0.8.32` after `WP-20D` merges.*

1. **Internal interfaces (`dev/interfaces/`):**
   - Update `dev/interfaces/{rust,rust-sdk,python,typescript,cli,wire}.md` for Schema 39 (`S39-COMPOSITE` + 8-col `vec0`), `ScopePolicy`, `ScopeViolation`, `graph::search_expand` `ReadView`, `fathomdb migrate 38 -> 39`, non-`#[non_exhaustive]` struct contract updates, and single-scope default posture.
   - Clean up the 3 pre-existing `schema 37` / `.30` lines at `dev/design/fathomdb-data-plane-architecture-v2.md:157,182,471` noted in Slice 10's review.
2. **Pre-publish `0.8.32` public docs & owner-mandated single-scope default (`docs/`):**
   - Update `dev/plans/release-state-0.8.32.json` (`"schema_version": 39`, `"public_docs_target_release": "0.8.32"`).
   - Keep `scripts/generate-public-docs.py:18` `SOURCE_URL` pointing to `release/0.8.31/` (the newest published release branch pinned by `scripts/tests/test_generated_public_docs.py:62`) until pre-publication in Slice 60, and run `.venv/bin/python scripts/generate-public-docs.py --write` to ensure `docs/generated/{sdk-operation-map,contributor-map}.md` are fresh.
   - Leave `scripts/check-public-doc-truth.py:64` `RELEASE_SCHEMAS` untouched while `published` is `null`, and satisfy every `check-public-doc-truth.py:261-328` pre-publish assertion explicitly:
     - All 16 `PUBLIC_DOCS` (`CURRENT_RELEASE_DOCS`) carry `0.8.32` as the documentation target while retaining `"newest published release is 0.8.31"` truth (never claiming `0.8.32` is already published).
     - `docs/compatibility/index.md` carries `"SCHEMA_VERSION\` to **39**"` AND retains `"no public automatic migration"`(`line 317`).
     - `docs/install/typescript.md` carries `npm install --save-exact fathomdb@0.8.32` (`line 321`).
   - **Owner-mandated single-scope default framing (`Gate1-D1`):** Explicitly document in `docs/concepts/index.md`, `docs/operations/index.md`, `docs/use-cases.md`, `docs/reference/config.md`, and `docs/getting-started/index.md` that **by default a FathomDB database is single-scope / single-agent (one agent per database, `scope = ""`), and Schema 39 `scope` is the opt-in isolation layer for multi-agent / shared-store deployments**.
   - Document `ScopePolicy` (`docs/reference/config.md`, `docs/reference/{python-api,typescript-api,rust-sdk}.md`), `ScopeViolationError` (`47` leaf classes in `docs/reference/errors.md` and `docs/reference/typescript-api.md`), `graph.search_expand` `ReadView` (`docs/guides/graph-retrieval.md`, API refs), and `fathomdb migrate 38 -> 39` (`docs/reference/cli.md`).

---

## 2. Final Slice 20 Verification & Review Gate

Before closing Slice 20:

1. Run full unconfined `./scripts/agent-verify.sh` on `/mnt/data/worktrees/release-0.8.32` (must exit `0` with `0` failures and `0` security violations).
2. Run `mkdocs build --strict`, `bash scripts/agent-lint-docs.sh`, `python3 scripts/check-public-doc-truth.py`, `bash scripts/check-release-state-views.sh`, `bash scripts/check-board-currency.sh`, and `python3 -m unittest scripts/tests/test_generated_public_docs.py`.
3. Spawn independent read-only `pro` `slice-reviewer` on the full Slice 20 diff until **`VERDICT: APPROVE`** (zero open P1/P2 findings).
4. Write `dev/plans/0.8.32/features/slice-20/status.md`, commit as `merge(0.8.32): Slice 20 SCOPE-AND-REFACTOR — ...`, update `release-state-0.8.32.json` (`landed: [10, 20]`, `next_slice: 30`) + `STATUS-0.8.32.md`, verify `check-release-state-views.sh` + `check-board-currency.sh`, and push `HEAD:refs/heads/dev/0.8.32` to `origin`.
