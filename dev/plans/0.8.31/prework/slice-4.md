---
title: 0.8.31 Prework Slice 4 — Architecture Review and Code Alignment
status: COMPLETE
target_release: 0.8.31
slice: 4
date: 2026-11-22
---

# 0.8.31 Prework Slice 4 — Architecture Review & Code Alignment

- **Target worktree audited (read-only):** `/home/coreyt/projects/fathomdb-worktrees/release-0.8.31`
- **Primary authorities reviewed:**
  - `dev/design/fathomdb-data-plane-architecture-v2.md` (`v2.2`)
  - `dev/adr/ADR-0.8.31-property-graph-inline-and-declared-indexes.md`
  - `dev/adr/ADR-0.8.31-strict-endpoint-integrity.md`
  - `dev/adr/ADR-0.8.31-historical-graph-and-fused-seeding.md`
  - `dev/adr/ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md`
  - `dev/plans/0.8.31/{plan.md,scope.md}`, `dev/plans/0.8.31/features/slice-10/prompt.md`, `dev/interfaces/*.md`
  - `src/rust/crates/{fathomdb-schema,fathomdb-engine,fathomdb-cli,fathomdb-sdk}/`, `src/inspector/` (`0.2.1`)

---

## Executive Summary

The `0.8.31` ADRs are cohesive, but direct code inspection exposes **14 concrete seams (`P4-01`–`P4-14`)** where ADR prose diverges from the shipped `0.8.30` / `Inspector 0.2.1` codebase or creates cross-slice traps. The highest-urgency schema-freeze blocker before Slice 10 lands is **`P4-09`**: `canonical_nodes` and `canonical_edges` carry **no commit epoch timestamp**, and `superseded_at` stores a `write_cursor` integer, so `as_of_epoch_ms` in `ADR-0.8.31-historical-graph-and-fused-seeding.md` is unimplementable without either scoping `0.8.31` to `as_of_write_cursor` only or adding commit-timestamp storage in Schema 38 now. Immediately adjacent traps include `Inspector 0.2.1` compile/admission locks in the Cargo workspace (`P4-13`), `vec0`'s 16-column non-NULL metadata constraint (`P4-01`), duplicate-column risk in `read_visibility_coalescing_upgrade_sql!` (`P4-04`), and `actuate`'s per-operation `apply_batch_in_transaction` loop breaking if strict endpoint enforcement lands in the wrong layer (`P4-07`).

---

## 1. Inline `properties` + `canonical_property_values` + `vec0` + `canonical_attributes` + Custom Vector Kinds

### `P4-01` — `vec0` 16-column budget, non-NULL metadata rejection, and typed vs. `TEXT` pre-KNN lowering (`Slice 10`)

- **Evidence:**
  - `sqlite-vec 0.1.9` (`sqlite-vec.c`): `VEC0_MAX_METADATA_COLUMNS = 16` (`L3486`) and all four metadata types (`BOOLEAN`, `INTEGER`, `FLOAT`, `TEXT`) strictly reject SQL `NULL` (`L8193-8217`).
  - `src/rust/crates/fathomdb-engine/src/vector_storage.rs:74-96` (`vector_partition_create_sql`): defines 7 built-in non-vector columns (`source_type TEXT partition key`, `kind TEXT`, `created_at INTEGER`, `status TEXT`, `lifecycle INTEGER`, `valid_from INTEGER`, `valid_until INTEGER`), leaving **at most 9–10 metadata slots** (note the stale doc comment at `L75` still says *"six built-in metadata columns"*). Dynamic attribute columns are hardcoded `{col} TEXT` (`L85`, `L331`).
  - `src/rust/crates/fathomdb-engine/src/projection_registry.rs:1097-1108`: encodes present `attr_<hex>` values as `\u{1} || val` to distinguish present-empty from absent `''`.
  - `src/rust/crates/fathomdb-engine/src/vector_storage.rs:146-155` (`desired_vector_attr_columns`): selects every `_fathomdb_projection_registry` entry with `ProjectionRole::Filterable`, with no `entity_class` filter and no slot-budget enforcement.
  - `src/rust/crates/fathomdb-engine/src/filter.rs:517-524` (`vector_filter_clause`): only emits `attr_<hex> = ?N`.
- **Risk / Drift:**
  1. Lexicographical `\u{1}`-prefixed `TEXT` breaks numeric `<`/`<=`/`>`/`>=` ranges (`"\x0110" < "\x012"`). Conversely, native `vec0` `INTEGER`/`FLOAT`/`BOOLEAN` columns still cannot hold `NULL` for absent values, so any non-NULL sentinel (e.g. `0` or `i64::MIN`) corrupts `!=`, `= false`, or `<` ranges unless matched against an explicit presence mask/post-filter.
  2. Declaring an `edge` `Filterable` property would consume scarce `vec0` node metadata slots unless filtered out of `desired_vector_attr_columns`.
- **Proposal:** Exclude `entity_class = 'edge'` from `desired_vector_attr_columns` (`vector_storage.rs:146`) and update the stale "six built-in" comment (`L75`). Enforce an explicit `vec0` metadata slot cap or allowlist only node scalar `Eq` predicates (and bounded positive ranges if typed) on `vec0` pre-KNN `attr_<hex>`, while routing `Ne` and numeric/float `PropertyPredicate` range queries (or overflows past the 16-column cap) through `canonical_property_values` (`_num_idx` / `_text_idx` / `_bool_idx`) with matching post-KNN candidate overfetch parity.

### `P4-02` — Pervasive `body`-only projection, hydration, and filter paths vs. `properties` precedence (`Slice 10`, `Slice 20`)

- **Evidence:**
  - `projection_commit.rs:411-420`: async vector commit reads `SELECT body FROM canonical_nodes WHERE write_cursor = ?1 LIMIT 1` and passes only `&body` into `vector_attr_insert_fragments`.
  - `vector_storage.rs:264-269` (`refresh_vector_attr_values_for_row`) and `projection_registry.rs:1151-1215` (`vector_attr_insert_fragments`, `project_node_attributes`, `backfill_attribute`): inspect `body` only (`SELECT write_cursor, body FROM canonical_nodes`).
  - `filter.rs:661-684` (`append_node_eligibility_sql_view`): when `view.include_superseded || view.include_inactive` is true, bypasses attribute tables and runs `scalar_attribute_sql(&format!("{alias}.body"), ...)`, ignoring `properties` completely.
  - `read.rs:25-30, 102, 231, 406` (`NodeRecord`), `search_types.rs:130-160` (`SearchHit`), `graph_expand/types.rs:123-144` (`GraphOriginV1`, `GraphTargetV1`), `graph_expand/execution.rs:473, 495`, and `evidence.rs:1069, 1098, 1455` omit `properties`.
  - Key validation discrepancy: `projection_registry.rs:717-722` (`is_valid_attribute_name`) rejects `""`, `'"'`, `'\\'`, and `.is_control()`, whereas `ADR-0.8.31-property-graph-inline-and-declared-indexes.md:35-36` specifies rejecting empty keys and `\x1e` / NUL for inline `properties`.
- **Risk:** Vector metadata backfill/async inserts, relaxed-`ReadView` searches, and re-activated nodes (`transition`) would ignore `properties` or mis-rank `properties` vs. `json_extract(body, ...)` precedence. A declared `Filterable` property containing `"` or `\` in `properties` could be rejected on projection registration despite passing write validation.
- **Proposal:** Thread `(body, properties: Option<&str>)` into all attribute/property projectors, `vector_attr_insert_fragments`, `refresh_vector_attr_values_for_row`, `backfill_attribute`, and `append_node_eligibility_sql_view`, consistently preferring `properties` over `json_extract(body, ...)`. Keep `read.list` `Predicate` (`read.rs:221`) explicitly specified (either `body`-only JSON predicates as today or `COALESCE`/fallback) and ensure `configure_projections` key rules align cleanly with `PropertyMap` validation.

### `P4-03` — Hardcoded 7-kind vector allowlist extends far beyond `vector_storage.rs` (`Slice 10`)

- **Evidence:**
  - `vector_storage.rs:464-518`: `VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES` (`[(&str, &str); 7]`), `resolve_source_type(kind: &str) -> Result<&'static str, EngineError>`, `kind_is_vector_committable`.
  - Hardcoded callers/SQL mirrors: `filter.rs:618-621` (`CASE {alias}.kind WHEN 'email' ... WHEN 'doc' THEN 'article' END = ?N` returns `NULL` for any custom kind!), `filter.rs:360`, `projection_generation.rs:1001-1015, 1163-1176` (`WITH committable(...) AS (VALUES ...)`), `search.rs:562` (`!crate::kind_is_vector_committable(&kind)` after querying `_fathomdb_vector_kinds` at `L552`), `index_projector.rs:93, 120`, `projection_commit.rs:360`, `write.rs:378`, `projection_registry.rs:2129-2155`, `data_plane_integrity.rs:1122, 1248`.
  - `src/inspector/src/vector.rs:252`: classifies vector rows by `if source_type == "edge_fact" { Class::Edge } else { Class::Node }`.
- **Risk:** Any custom kind enrolled in `_fathomdb_vector_kinds` will silently vanish from `SearchFilter.source_type` (`filter.rs:618`), fail `dense_physical_shape` / `unregistered_node_completion_sql` / `historical_dense_incomplete`, or panic/error on `&'static str` return types. Enrolling `"edge_fact"` as a custom node kind would corrupt node-vs-edge vector partition classification in both `Engine` (`vector_storage.rs:476`) and `Inspector` (`vector.rs:252`).
- **Proposal:** Reject `"edge_fact"` as a reserved kind when enrolling custom node vector kinds. Replace `resolve_source_type -> &'static str` with a `Cow<'_, str>` / `String` helper (connection-backed or built-in + enrolled `_fathomdb_vector_kinds`), and update all SQL mirrors (`filter.rs:618`, `projection_generation.rs:1000-1030, 1163-1176`, `data_plane_integrity.rs:1122, 1248`, `search.rs:562`) to admit enrolled `_fathomdb_vector_kinds`.

### `P4-04` — Visibility coalescing macro & custom-trigger allowlist traps for `canonical_property_values` (`Slice 10`)

- **Evidence:**
  - `ADR-0.8.31-property-graph-inline-and-declared-indexes.md:122` cites `read_visibility_coalescing_upgrade_sql!`. In `fathomdb-schema/src/lib.rs:124-138`, that macro executes `ALTER TABLE _fathomdb_read_visibility_state ADD COLUMN coalescing ...` (already executed in step 37!) before creating triggers.
  - `frozen_read.rs:34-54, 418-468`: slices `VISIBILITY_TRIGGER_TABLES` by `schema_version` (`[..19]` for `>= 35`) and asserts `COUNT(*) == trigger_tables.len() * 3`.
  - `write_commit.rs:427-466` (`canonical_batch_has_no_custom_triggers`): disables the coalescing fast path whenever any trigger exists on a hardcoded 16-table `tbl_name IN (...)` allowlist outside `_fathomdb_read_visibility_%` (and mirrors in `projection_commit.rs:18-51` for projection tables).
  - `frozen_read.rs:561-591` (`projection_registry_digest`) hashes `(name, roles, fts_tokenizer, vector_embedder, vector_declared, source)` without any new `entity_class` / property-type columns.
  - `projection_generation.rs:361-380` (`bootstrap_is_fresh`) enumerates table names explicitly.
- **Proposal:** In step 38 (`fathomdb-schema/src/lib.rs`), invoke **`visibility_coalescing_trigger_sql!`** directly for `("cpv", "canonical_property_values")` instead of `read_visibility_coalescing_upgrade_sql!` (avoiding duplicate-column error). Update `VISIBILITY_TRIGGER_TABLES` (19→20, `if schema_version >= 38`), `canonical_batch_has_no_custom_triggers` (`write_commit.rs:438, 452` to include `'canonical_property_values'`), `bootstrap_is_fresh` (`projection_generation.rs:361`), and `projection_registry_digest` (`frozen_read.rs:561`) if schema columns change on `_fathomdb_projection_registry`.

### `P4-05` — Operational-collection `kind` conflict bypass (`Slice 10`)

- **Evidence:** `write_commit.rs:863-871` runs `INSERT INTO operational_collections(...) ... ON CONFLICT(name) DO UPDATE SET schema_json = excluded.schema_json, retention_json = excluded.retention_json`, leaving `kind` silently unchanged on conflict; `write_validation.rs:220-231` only inspects `operational_collections` for `OpStore`, not `AdminSchema`.
- **Proposal:** Validate existing `kind` vs. requested `kind` in `write_validation.rs` (and re-check inside the writer transaction before `ON CONFLICT DO UPDATE`), failing with `EngineError::WriteValidation` on mismatch while allowing idempotent schema/retention updates when `kind` matches.

---

## 2. Reserved `scope` Column in Schema 38 vs. `0.8.32` Activation

### `P4-06` — `scope TEXT NOT NULL DEFAULT ''` neutrality in `0.8.31` vs. `0.8.32` indexing and `vec0` slots (`Slice 10`, `0.8.32`)

- **Evidence:**
  - All current canonical/operational inserts use explicit column lists (`write_commit.rs:749, 879, 891`, `projection_commit.rs:523`) — safe with `NOT NULL DEFAULT ''`.
  - `physical_erasure/admission.rs:32-67` (`envelope`) uses `PRAGMA table_info({table})` on `operational_mutations` and `operational_state` (automatically includes `scope` bytes — 0 bytes when `''`), and `Inspector` `validate_layout` (`src/inspector/src/admission.rs:132-136`) selects explicit column lists (tolerant of additive columns).
  - No `SELECT *` exists on `canonical_nodes`, `canonical_edges`, `operational_mutations`, or `operational_state` across `fathomdb-engine` / `fathomdb-schema`.
- **Risk / `0.8.32` Coupling:** Schema 38 reserves `scope` with no index and no `vec0` metadata column (`vector_default` only has ~9 free metadata slots total, per `P4-01`). In `0.8.32`, activating scope isolation on `vector_default` pre-KNN filters will consume one of those remaining metadata slots, and `0.8.32` will likely need composite indexes (additive in step 39+) plus `scope` carried in frozen-read / pagination codecs (`frozen_read.rs`, `graph_expand/codec.rs`).
- **Proposal:** Keep `scope TEXT NOT NULL DEFAULT ''` strictly inert in Slice 10, verify via test that default rows and `fathomdb migrate` (37→38) preserve `''` with zero write/read penalty, and explicitly account for 1 reserved `vec0` metadata slot for `0.8.32` scope filtering when enforcing Slice 10's property-index slot budget (`P4-01`).

---

## 3. Strict-Only Endpoint Integrity vs. `engine.write`, `actuate`, and `ingest_with_extractor`

### `P4-07` — `actuate` per-op `apply_batch_in_transaction` seam breaks if strictness is enforced inside `apply_batch_in_transaction` (`Slice 20`)

- **Evidence:**
  - `write_commit.rs:921, 932-986` (`count_dangling_edge_endpoints`): runs at the end of `apply_batch_in_transaction`. It (a) only checks `superseded_at IS NULL` without `AND state = 'active'` (`L973`), and (b) skips edges superseded later in the same batch (`last_index > i` by `logical_id`, `L964-970`, also missing G11 `(from_id, to_id, kind)` supersession).
  - `actuation.rs:1567` (`simulate_request`) and `actuation.rs:2172, 2238` (`actuate_new`): apply each `ActuationOperationV1` by calling `apply_batch_in_transaction(&tx, &[write], ...)` **one item at a time**, and run `validate_prospective_edge_endpoints(connection, request)` (`L1640, 1650`, checking `superseded_at IS NULL AND state = 'active'`) only **after** the entire loop finishes so an `[PutDerivedEdge, PutNode]` batch succeeds and an invalid batch records `ActuationOutcomeV1::Refused`.
- **Risk:** Moving strict rollback inside `apply_batch_in_transaction` directly would break same-batch `[PutDerivedEdge, PutNode]` in `actuate`, returning `Err(EngineError::Storage)` (`actuation.rs:1594, 2220`) instead of `ActuationOutcomeV1::Refused` (for dangling) or `Ok(Committed)` (for edge-before-node). Conversely, leaving `last_index` skipping inside strict `write` lets a dangling earlier edge sneak into history if superseded within the same batch.
- **Proposal:** Pass an explicit endpoint-validation mode into `apply_batch_in_transaction` (or validate endpoints in `commit_batch` right after `apply_batch_in_transaction` for `Engine::write`, while keeping `validate_prospective_edge_endpoints` post-loop for `actuate`). In `Engine::write`, check **all** edges in the batch (not just non-in-batch-superseded ones) against `superseded_at IS NULL AND state = 'active'` via `canonical_nodes_logical_active_idx`, rolling back atomically to `EngineError::WriteValidation`, while preserving the zero-query fast path for node-only batches.

### `P4-08` — `ingest_with_extractor` two-transaction split & legacy test fallout (`Slice 20`)

- **Evidence:**
  - `ingest.rs:204-226, 366-412`: `ingest_with_extractor` runs `read_get_many(&ids, &ReadView::default())`, writes `self.write(&new_nodes)` in transaction 1 (`L224`), then maps `from_entity`/`to_entity` (falling back to `(name, "entity")` at `L372-379` if unlisted) and calls `self.write(&edge_batch)` in transaction 2 (`L410`).
- **Risk:** If a pre-existing entity becomes inactive/superseded between tx1 and tx2, or an extractor emits an edge referencing an unlisted entity omitted from synthesized `entities[]`, tx2 will fail atomically with `WriteValidation` leaving tx1's `new_nodes` committed (consistent with Slice 0 E3 deferral, but worth verifying in tests). Additionally, existing tests across Rust (`pr_g8_dangling_edges.rs`, `slice20_graph_traversal.rs`, `slice85_physical.rs`) and Python/TS that construct dangling/inactive endpoints via `write` will break unless updated per `ADR-0.8.31-strict-endpoint-integrity.md:45-50`.
- **Proposal:** Audit all `write`/edge-creation fixtures in Slice 20: update inactive-intermediate traversal tests to write active nodes + edge first and then `transition` the node inactive, and ensure `ingest_with_extractor` tests cover valid synthesized endpoints.

---

## 4. Historical `as_of` Graph Traversal vs. `graph_expand`, `use_graph_arm`, and Erasure

### `P4-09` — CRITICAL BEFORE SLICE 10 FREEZE: No commit epoch timestamps exist for `as_of_epoch_ms` (`Slice 10` / `Slice 30`)

- **Evidence:**
  - `ADR-0.8.31-historical-graph-and-fused-seeding.md:38-43` specifies `known_as_of` as **`as_of_write_cursor` or `as_of_epoch_ms`** (*"created at or before `known_as_of` ... `superseded_at IS NULL OR superseded_at > known_as_of_boundary`"*), and `plan.md:76` references *"revision timestamps"*.
  - Schema 37 (`fathomdb-schema/src/lib.rs`) and `write_commit.rs:749, 800, 817`: `canonical_nodes`, `canonical_edges`, and `_fathomdb_artifact_revisions` have **zero commit-timestamp columns**. `superseded_at` stores the **superseding `write_cursor`** integer (`params![cursor, ...]`), never epoch time; `vector_default.created_at` (`projection_commit.rs:366`) is async worker time for vector rows only.
  - Schema 38 (`ADR-0.8.31-property-graph-inline-and-declared-indexes.md` / `slice-10/prompt.md`) adds no commit-timestamp column or commit-log table.
  - `temporal.rs:31-34` (`ReadView`): documents *"Transaction-time / `history_as_of` is explicitly OUT OF SCOPE"*.
- **Risk:** Since Slice 10 freezes Schema 38 before Slice 30 starts, implementing `as_of_epoch_ms` in Slice 30 will be **impossible** against Schema 38 unless resolved before Slice 10 closes.
- **Proposal:** Decide immediately before Slice 10 closes either:
  1. **Scope `0.8.31` `known_as_of` strictly to `as_of_write_cursor`** (updating `ADR-0.8.31-historical-graph-and-fused-seeding.md:38-40` and `plan.md:76` to remove/defer `as_of_epoch_ms`), **or**
  2. **Add `committed_at_ms INTEGER` / `_fathomdb_commit_timestamps(write_cursor INTEGER PRIMARY KEY, committed_at_ms INTEGER NOT NULL)` in Schema 38 inside Slice 10** (populated atomically inside `commit_batch` / `actuate` with monotonic handling if needed for `known_as_of`). Also decide whether `known_as_of` lives strictly on `GraphExpandRequestV1` / `GraphReadContextV1` rather than mutating `ReadView` (`temporal.rs:34`) globally.

### `P4-10` — `graph_expand` pre-filter `LIMIT`/`work_units`, `OR` multi-index scan, `(from_id, kind, superseded_at)` column order, missing `t_valid`, and terminal-depth skip (`Slice 20`, `Slice 30`)

- **Evidence:**
  - `graph_expand/execution.rs:489-496, 525-550` (`graph_expand_incident_sql`, `load_incident_edges`): queries `WHERE {predicate} LIMIT ?2` with `{predicate} = (from_id=?1 OR to_id=?1)` for `Both`, **without** `kind`, `superseded_at`, `t_valid`, or `t_invalid` predicates, and **without** `ORDER BY`.
  - `graph_expand/execution.rs:825-843, 878`: fetches `remaining + 1` unfiltered rows, aborts with `GraphExpansionBoundExceeded` if `edges.len() > remaining`, and increments `work_units += 1` (`L878`) before filtering `edge_kinds` (`L879`), `superseded_at` (`L882`), or `t_invalid` (`L883`).
  - `graph_expand/execution.rs:495, 882-885`: never selects or checks `t_valid` (future `t_valid > valid_as_of` edges currently traverse!).
  - `graph_expand/execution.rs:895-899` vs `942-955`: at `depth + 1 == request.max_depth`, if `candidates.len() >= result_limit`, `L895` skips before `load_node` / `target_kinds` / `target_predicates` / `origin_cmp` replacement (`L942-955`).
  - `graph_expand/execution.rs:473` (`load_node`): `WHERE n.logical_id=?1 ... ORDER BY n.write_cursor DESC LIMIT 1` lacks `known_as_of` revision bounding (`write_cursor <= as_of` and `(superseded_at IS NULL OR superseded_at > as_of)`).
  - `graph_expand/execution.rs:1114-1160` (`encode_graph_evidence_request`): serializes request fields for frozen evidence authentication without `edge_predicates`, `target_predicates`, or `known_as_of`.
  - `ADR-0.8.31-property-graph-inline-and-declared-indexes.md:78-80` (`slice-10/prompt.md:44`): defines historical composite indexes `canonical_edges_from_kind_sup_idx(from_id, kind, superseded_at)` and `canonical_edges_to_kind_sup_idx(to_id, kind, superseded_at)`.
- **Risk:**
  1. Pre-filter `LIMIT remaining + 1` lets superseded/wrong-kind edges trip `GraphExpansionBoundExceeded` and burn `work_units`, directly violating `ADR-0.8.31-historical-graph-and-fused-seeding.md:52`.
  2. In `Both` mode, `OR` + `LIMIT` makes truncation non-deterministic across `from_id` vs. `to_id` index order.
  3. On Schema 38's `(from_id, kind, superseded_at)`, when `edge_kinds` is empty (`[]`), `kind` is unconstrained, so SQLite **cannot range-seek on `superseded_at`** and must inspect every superseded edge version of the node! (Splitting `as_of` queries as `UNION ALL` between active rows `superseded_at IS NULL AND write_cursor <= ?as_of` on `canonical_edges_*_active_idx` and historical superseded rows `superseded_at > ?as_of AND write_cursor <= ?as_of` helps only if an index puts `superseded_at` before `kind` — e.g., `(from_id, superseded_at)` or covering `(from_id, superseded_at, kind, write_cursor)` — when `edge_kinds` is empty!).
  4. Missing `t_valid` check violates bitemporal `[t_valid, t_invalid)` correctness.
- **Proposal:** Push `superseded_at`, `kind IN (...)`, `t_valid`, and `t_invalid` (and `write_cursor <= as_of` for `known_as_of`) into SQL, rewrite `Both` as `UNION ALL` of separate `from_id = ?1` and `to_id = ?1` selects (selecting `properties` and `t_valid`), consider replacing or complementing `(from_id, kind, superseded_at)` with `(from_id, superseded_at, kind)` (verified via `EXPLAIN QUERY PLAN` for **both** empty and non-empty `edge_kinds`), fix `load_node` + `resolve_seeds` for `known_as_of`, audit `L895` terminal early-skip vs `origin_cmp` when predicates apply, and extend `encode_graph_evidence_request` + `codec.rs`.

### `P4-11` — Historical evidence provenance & `use_graph_arm` (`search.rs`) coupling (`Slice 20`, `Slice 30`)

- **Evidence:**
  - `evidence.rs:883-920` (`authorize_intrinsic_lifecycle`), `L1157-1187` (`load_graph_edge_revision`), `L1189-1213` (`source_revision_is_eligible`): check boolean `include_superseded` (`(?4=1 OR e.superseded_at IS NULL)`) and require `source_superseded == false` and active source (`n.superseded_at IS NULL AND n.state='active'`). `GraphEvidenceMaterial` (`L1455-1480`) lacks `properties`.
  - `search.rs:1625-1641` (`Seed source A`): uses `EXISTS(ef ...) OR EXISTS(et ...)` — confirmed source of the mixed-edge `SEED_FTS_N` slot bug.
  - `search.rs:1855-1865` (`Phase 2`): uses `(e.from_id = ?1 OR e.to_id = ?1) ... AND (e.temporal_fallback IS NULL OR e.temporal_fallback = 0) ... ORDER BY e.write_cursor LIMIT 64` with no `edge_kinds` or `edge_predicates` support.
- **Risk:** Historical `graph.expand` with `include_evidence: true` will refuse superseded edges/sources (or fail provenance snapshot rules) unless `evidence.rs` respects `known_as_of` while strictly honoring physical erasure (`DELETE`d rows + `_fathomdb_dependency_closures` active barriers remain unavailable). Also in `search.rs`, Phase 2's `OR` clause cannot cleanly leverage separate partial active `from`/`to` indexes without `UNION ALL`, and `temporal_fallback` semantics should stay documented between `graph_expand` (allows fallback) and `use_graph_arm` (skips fallback).
- **Proposal:** Thread `known_as_of` + `properties` through `preflight_graph_evidence` / `evidence.rs` (keeping erased/barrier-blocked sources `Unavailable`), fix `Seed source A` to require eligibility consistently for the seeded endpoint(s) before consuming `SEED_FTS_N`, rewrite `search.rs` Phase 2 edge lookup with `UNION ALL` + `edge_kinds`/`properties` filtering, and ensure `graph_seed="fused_top_k"|"both"` filters/hydrates fused candidates by `logical_id IS NOT NULL` while keeping `use_graph_arm = false` untouched.

---

## 5. `ExportClass` Table-Manifest Audit, `Inspector 0.2.1` Seams, Docs Drift & CLI Packaging

### `P4-12` — `sqlite_master` completeness & `ExportClass` manifest gaps (`Slice 10`)

- **Observed schema inventory (`golden-0.8.30-schema37.sqlite` = 70 non-`sqlite_%` tables + 1 new table `canonical_property_values` in schema 38):**

| Category / Table(s) | Schema 38 Role | Recommended `ExportClass` | Notes / Audit Trap |
| --- | --- | --- | --- |
| `canonical_nodes`, `canonical_edges`, `operational_collections`, `operational_mutations`, `operational_state` | Canonical & Op Authority | `DurableAuthority` | Explicit columns on `INSERT` protect `properties`/`scope`. |
| `_fathomdb_artifact_revisions`, `_fathomdb_source_versions`, `_fathomdb_source_links`, `_fathomdb_source_dependencies` | Provenance / Dependency Authority | `DurableAuthority` | Required for integrity & evidence after logical restore. |
| `_fathomdb_projection_registry`, `_fathomdb_vector_kinds` | Schema / Enrolment Authority | `DurableAuthority` | Required so declared indexes & custom kinds survive vector-free export (`OptInVector` risks losing kind/property definitions!). |
| `_fathomdb_actuation_receipts`, `_fathomdb_actuation_receipt_source_refs`, `_fathomdb_dependency_closures` | Idempotency / Audit / Closure | `OptInAudit` | Audit/idempotency records; must decide explicitly in Slice 10 whether closures/receipts export under `OptInAudit` vs `DurableAuthority`. |
| `vector_default`, `_fathomdb_vector_rows`, `_fathomdb_vector_spans`, `_fathomdb_embedder_profiles`, `_fathomdb_embed_probe` | Dense Embeddings & Profiles | `OptInVector` | Per `ADR-0.8.31-cli...md:77-81`: **never** export `embedding_bin` or `mean_vec`; export uncentered f32 vectors + spans + `EmbedderIdentity`. |
| `canonical_attributes`, `canonical_property_values`, `property_search_index`, `search_index`, `search_index_v2`, `search_index_edges` | Row-owned / Derived Indexes | `DerivedProjection` | Re-derived on rebuild/import; `canonical_attributes` & `canonical_property_values` belong in `ROW_OWNED_PROJECTIONS`. |
| `*_config`, `*_content`, `*_data`, `*_docsize`, `*_idx` (20 FTS5 shadow tables across 4 vtables) | SQLite FTS5 Internals | `DerivedProjection` (or `InternalRuntime`) | Managed by SQLite FTS5 vtables; classify via prefix/suffix rule. |
| `vector_default_chunks`, `vector_default_info`, `vector_default_rowids`, `vector_default_vector_chunksNN`, `vector_default_metadatachunksNN`, `vector_default_metadatatextNN` | `sqlite-vec` Internals | `InternalRuntime` | **Dynamic table names!** (`metadatachunks08..15` / `metadatatext*` appear/vanish as `attr_<hex>` columns reshape). Classify by `vector_default_*` prefix! |
| `_fathomdb_migrations`, `_fathomdb_schema_meta`, `_fathomdb_open_state`, `_fathomdb_read_visibility_state`, `_fathomdb_projection_state`, `_fathomdb_projection_terminal`, `_fathomdb_projection_generations`, `_fathomdb_projection_generation_current`, `_fathomdb_erasure_requests` | DB-Local Runtime / Lifecycle State | `InternalRuntime` | DB-local cursors/secrets/nonces/locks; export must refuse if `_fathomdb_erasure_requests` or closures are non-complete (`ADR-0.8.31-cli...md:50-54`). |

- **Additional `doctor` (`data_plane_integrity.rs`) seams:**
  - `data_plane_integrity.rs:157-173` (`candidate_queries_for_test` 13 queries) and `ATTRIBUTE_OWNER_QUERY` (`L59-64`, `L226-260`, `L1811-1914`) check ONLY `canonical_attributes` and `property_search_index` from `n.body`, ignoring `canonical_property_values`, edge properties, and dynamic `attr_<hex>` columns on `vector_default`. Note `physical_erasure/admission.rs:22` checks `body` only on canonical tables — must sum `COALESCE(length(CAST(body AS BLOB)), 0) + COALESCE(length(CAST(properties AS BLOB)), 0)`.
- **Proposal:** Make Slice 10's `sqlite_master` audit classify dynamic FTS5/vec0 shadows via deterministic prefix/suffix matchers (`search_index*`, `property_search_index_*`, `vector_default_*`), pin the exact `ExportClass` split for `_fathomdb_vector_kinds` (`DurableAuthority`) and audit/closure tables (`OptInAudit`), and extend `data_plane_integrity.rs` to validate `canonical_property_values` completeness/orphan status without breaking `candidate_queries_for_test`.

### `P4-13` — `Inspector 0.2.1` workspace build & schema-38 admission seams (`Slice 10`, `Slice 20`, `Slice 30`, `Slice 50`)

- **Evidence:**
  - `Cargo.toml:3`: `"src/inspector"` is now a workspace member, so `cargo clippy --workspace --all-targets` compiles it in **every** slice.
  - `src/inspector/src/admission.rs:118` & `session.rs:178`: refuse whenever `PRAGMA user_version != fathomdb_schema::SCHEMA_VERSION`.
  - `src/inspector/src/graph.rs:572-591`: constructs `GraphExpandRequestV1 { schema_version: 1, ..., include_evidence: true }` **without `..Default::default()`** (and `GraphExpandRequestV1` does not derive `Default` or `#[non_exhaustive]`).
  - `src/inspector/src/lexical.rs:229-234`: constructs `ReadView { include_superseded: ..., include_inactive: ..., include_out_of_window: ..., valid_as_of: ... }` **without `..Default::default()`**.
  - `src/inspector/src/storage.rs:75-103`: `Class::fields` omits `properties` and `scope`; `L215, 551, 825, 1036, 1232, 1278` read `canonical_attributes` only; `src/inspector/assets/app.js:1148-1160, 1266-1291` only render materialized attributes / property FTS and `valid_as_of`.
  - `src/inspector/src/graph.rs:614`: hardcodes `INDEXED BY canonical_nodes_logical_active_idx` (`WHERE logical_id=?1 AND superseded_at IS NULL`), which remains valid for current-time origin lookups but cannot inspect historical superseded nodes (`as_of`) in Slice 50 without relaxing the `INDEXED BY` / `superseded_at` filter.
- **Risk:** Even though Inspector UI work is deferred to Slice 50, bumping `SCHEMA_VERSION` in Slice 10 immediately locks Inspector out of schema-37 stores until Inspector's fixtures/UI align, and **adding fields to `GraphExpandRequestV1` (Slice 20/30) or `ReadView` (if changed) will break `cargo clippy --workspace` in Slices 20/30!**
- **Proposal:** Either use `#[derive(Default)]` + `..Default::default()` (or builder constructors) when extending `GraphExpandRequestV1` / `ReadView` in Slices 20/30 — OR explicitly update `src/inspector/src/{graph,lexical}.rs` struct literals inside the same slice that adds fields, keeping `cargo clippy --workspace --all-targets` green prior to Slice 50's full UI/history/property integration (which must also replace `INDEXED BY canonical_nodes_logical_active_idx` when historical inspection is enabled).

### `P4-14` — Architecture/interface doc drift & `fathomdb-cli` dual-`[[bin]]` wheel packaging seam (`Slice 10`, `Slice 20`, `Slice 40`)

- **Evidence:**
  - `dev/design/fathomdb-data-plane-architecture-v2.md`: still declares `target_release: 0.8.30` (`L6`), **twelve** workspace members (`L43` vs. 13 in `Cargo.toml:2-16`), and *"bootstraps directly at schema 37"* (`L156`).
  - `dev/interfaces/{wire,rust,python,typescript,cli}.md`: cite `schema 37` (`wire.md:31,75`, `rust.md:22`, `python.md:20`, `typescript.md:20`, `cli.md:365`), retain `dangling_edge_endpoints` default flag-and-count write behavior (`rust.md:805`, `python.md:217`), and lack the `fathomdb migrate` root (`cli.md`).
  - `src/rust/crates/fathomdb-cli/Cargo.toml`: defines **two** `[[bin]]` targets — `name = "fathomdb"` (`src/main.rs`) and `name = "perf-regression-check"` (`src/bin/perf-regression-check.rs`).
- **Risk:** With Maturin `bindings = "bin"` (`ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md:30`), multiple `[[bin]]` targets in `fathomdb-cli/Cargo.toml` will either cause Maturin ambiguity/error or ship the internal `perf-regression-check` helper on PyPI `PATH` alongside `fathomdb` unless explicitly scoped.
- **Proposal:** Update `dev/design/fathomdb-data-plane-architecture-v2.md` (13 crates including `fathomdb-inspector`, schema 38+, `0.8.31` profile) and `dev/interfaces/*.md` in lockstep with Slices 10–40 (`X4`). In Slice 40, either scope Maturin `bindings = "bin"` explicitly to `--bin fathomdb` (or `pyproject.toml` bin config) or move/feature-gate `perf-regression-check` so PyPI `fathomdb-cli` ships solely the `fathomdb` executable.

---

## Summary Matrix by Target Slice

| Proposals | Target Slice(s) | Blocking Risk if Missed |
| --- | --- | --- |
| `P4-09` | **Before Slice 10 close** (`Slice 30`) | `as_of_epoch_ms` impossible on Schema 38 without commit-timestamp storage or scope narrowing |
| `P4-01`..`P4-05`, `P4-12` | `Slice 10` | `vec0` runtime errors/slot exhaustion, `body` vs `properties` drift, custom-kind search bugs, duplicate `coalescing` DDL fail, `sqlite_master` false failures on dynamic `vec0` shadows |
| `P4-06` | `Slice 10` / `0.8.32` | Exhausted `vec0` metadata slots before `0.8.32` scope activation |
| `P4-07`, `P4-08`, `P4-10`, `P4-11` | `Slice 20` / `Slice 30` | Broken `actuate` same-batch ordering, premature `GraphExpansionBoundExceeded`, non-deterministic `Both` `OR` limits, empty-`edge_kinds` historical index scans, future `t_valid` leaks, evidence refusals on historical paths |
| `P4-13`, `P4-14` | `Slice 10`–`50` | `cargo clippy --workspace` compilation failures on `src/inspector`, shipping `perf-regression-check` in PyPI `fathomdb-cli` wheel, stale contracts |
