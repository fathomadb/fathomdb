---
title: ADR-0.8.31 — Property graph inline typed properties and declared indexes
date: 2026-11-22
target_release: 0.8.31
desc: Schema 38 storage and indexing contract for outside-body typed properties on nodes and edges, batch commit timestamps, reserved scope column, explicit opt-in app-declared vector kinds, op-collection kind readback, and erasure/visibility completeness invariants.
blast_radius: src/rust/crates/fathomdb-schema/**; src/rust/crates/fathomdb-engine/**; src/rust/crates/fathomdb-sdk/**; src/rust/crates/fathomdb-py/**; src/rust/crates/fathomdb-napi/**; src/python/**; src/ts/**
status: accepted
---

# ADR-0.8.31: Property graph inline typed properties and declared indexes

## Context

FathomDB 0.8.30 (schema 37) stores structured metadata only inside
`canonical_nodes.body` JSON and projects node attributes into
`canonical_attributes(write_cursor, attr_name, attr_value)` (text-only) and
`vec0` metadata columns. `canonical_edges` has fixed columns only and no general
property store. Embedded agent applications therefore embed JSON envelopes
(UUIDs, SHA-256 digests, filter tags) inside `canonical_nodes.body`, polluting
FTS5 and the 512-wordpiece embedder, and cannot attach arbitrary typed
properties to edges or declare custom vector-indexed node kinds beyond the
hardcoded 7-kind allowlist in `VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES`
(`src/rust/crates/fathomdb-engine/src/vector_storage.rs`).

## Decision (Owner Option 1, 2026-11-21; amended HITL Gate 1, 2026-11-22)

Advance `SCHEMA_VERSION` from `37` to `38` (migration step 38) with the
following storage, indexing, commit-timestamp, identity, erasure, and
visibility contracts:

1. **Inline canonical typed properties outside `body`**:
   - Add `properties TEXT` (canonical compact JSON object of typed scalar
     values: string, integer, float, boolean — or `NULL` when absent) to both
     `canonical_nodes` and `canonical_edges`, strictly outside `body`.
   - Validation rejects non-object JSON, nested objects/arrays in scalar
     properties, non-finite floats (`NaN`/`Inf`), empty keys, and control-byte
     delimiters (`\x1e` / NUL) at write time.
   - `body` remains clean plain text (or caller text) for FTS5 and embedding.
     Inline `properties` hydrate in O(1) on `SearchHit`, `GraphTargetV1`,
     `GraphOriginV1`, `read.get` / `read.get_many` / `read.list`, and resolved
     evidence without N+1 EAV joins.
   - Property changes are strictly **immutable/revisioned**: updating a node or
     edge's properties creates a new `write_cursor` and `_fathomdb_artifact_revisions`
     entry superseding the prior version — never an in-place `UPDATE`.
2. **Declared typed property indexes for corpus-wide `SearchFilter`**:
   - Add a row-owned derived index table `canonical_property_values` keyed by
     `(entity_class, write_cursor, attr_name)` with typed columns:
     - `entity_class TEXT NOT NULL CHECK(entity_class IN ('node', 'edge'))`
     - `write_cursor INTEGER NOT NULL`
     - `attr_name TEXT NOT NULL`
     - `value_type TEXT NOT NULL CHECK(value_type IN ('string', 'integer', 'float', 'boolean'))`
     - `text_val TEXT`
     - `num_val NUMERIC` (SQLite `NUMERIC` affinity preserves exact 64-bit
       `INTEGER` storage class for `abs(x) > 2^53` while ordering `INTEGER` and
       `REAL` values accurately in one B-tree index; cross-type `Int` vs
       `Float` comparisons use exact numeric ordering without lossy `i64 -> f64`
       truncation for `Int` vs `Int` predicates)
     - `bool_val INTEGER CHECK(bool_val IS NULL OR bool_val IN (0, 1))`
   - Composite B-tree indexes:
     - `canonical_property_values_cursor_idx(write_cursor)`
     - `canonical_property_values_text_idx(entity_class, attr_name, text_val, write_cursor)`
     - `canonical_property_values_num_idx(entity_class, attr_name, num_val, write_cursor)`
     - `canonical_property_values_bool_idx(entity_class, attr_name, bool_val, write_cursor)`
   - **Active-only maintenance contract**: `canonical_property_values` is
     strictly active-only (mirroring `canonical_attributes`):
     - On node supersession (`logical_id`), purge prior cursor's rows via
       `purge_row_projections_for_cursor_in`.
     - On edge supersession — both G0 (`logical_id`) and G11
       (`(from_id, to_id, kind)`) — purge prior cursor's rows via
       `purge_row_projections_for_cursor_in`.
     - On node lifecycle transitions (`transition` to `deleted` / `pending`),
       purge the node's `canonical_property_values` rows; on transition back to
       `active`, re-project declared filterable properties from the active row's
       `properties` (with `json_extract(body, ...)` fallback, threading
       `properties` through `record_lifecycle.rs`).
   - **Schema 38 incident edge indexes**: include both partial active incident
     indexes (`canonical_edges_from_active_idx ON canonical_edges(from_id, kind)
     WHERE superseded_at IS NULL` and `canonical_edges_to_active_idx ON
     canonical_edges(to_id, kind) WHERE superseded_at IS NULL`) for zero-waste
     current-time `graph.expand`, unified historical composite indexes
     (`canonical_edges_from_sup_kind_idx(from_id, superseded_at, kind)` and
     `canonical_edges_to_sup_kind_idx(to_id, superseded_at, kind)`), and a
     non-partial historical node lookup index
     (`canonical_nodes_logical_sup_idx(logical_id, superseded_at, write_cursor)`)
     so SQLite can range-seek on `superseded_at` for both empty and non-empty
     `edge_kinds` and resolve historical nodes without full `canonical_nodes`
     table scans during bitemporal `as_of` traversal — avoiding any second
     schema bump in later `0.8.31` slices.
   - **`vec0` 16-column metadata budget (`sqlite-vec 0.1.9`)**:
     - `sqlite-vec` caps metadata columns at 16 (`VEC0_MAX_METADATA_COLUMNS = 16`,
       7 built-in columns leaving 9 dynamic slots) and rejects SQL `NULL`.
     - Exclude `entity_class = 'edge'` from `desired_vector_attr_columns`
       (`vector_storage.rs`), reserve **1 dynamic metadata slot** for `0.8.32`
       caller `scope` pre-KNN filtering, and enforce an explicit cap of at most
       8 declared node `Filterable` `vec0` `attr_<hex>` columns.
     - Use `vec0` pre-KNN `attr_<hex>` strictly for node scalar `Eq` predicates
       (no speculative non-NULL sentinel encodings); route `Ne`, numeric/float
       range predicates (`<`, `<=`, `>`, `>=`), and any declared node properties
       beyond the 8-slot `vec0` cap cleanly through `canonical_property_values`
       with matching post-KNN candidate overfetch parity.
   - Only properties declared `Filterable` via `configure_projections` (extended
     additively with optional `entity_class` defaulting to `node`) are written
     to `canonical_property_values` (synchronous, same-transaction) and, within
     the `vec0` budget above, to `vec0` pre-KNN metadata columns.
   - Corpus-wide `SearchFilter` / `Filter` supports typed equality and range
     predicates on declared filterable node properties via indexed `SEARCH`
     (never full-corpus `SCAN`), and returns a typed error on undeclared keys.
   - Local `graph.expand` and `use_graph_arm` evaluate closed typed predicates
     on **any** edge or target property (declared or undeclared) inline against
     the fetched `properties` JSON after indexed `(from_id, kind)` /
     `(to_id, kind)` lookup, with zero write-side index bloat for non-filtered
     envelope keys (UUIDs, SHA-256 hashes).
   - Existing `canonical_attributes` and `property_search_index` remain
     supported for backward-compatible JSON-body attribute projections; when an
     explicit `properties` map contains a declared key, `properties` takes
     precedence over `json_extract(body, ...)`.
3. **Batch commit timestamps (`_fathomdb_commit_timestamps`, HITL Gate 1 `P4-09`)**:
   - Create `_fathomdb_commit_timestamps(write_cursor INTEGER PRIMARY KEY,
     committed_at_ms INTEGER NOT NULL)` and index
     `_fathomdb_commit_timestamps_ms_idx(committed_at_ms, write_cursor)` in
     Schema 38.
   - Key each row by the committed batch's high-water `WriteReceipt.cursor`
     (`max(row_cursors)`), inserting **one row per committed batch** inside both
     `commit_batch` and `actuate_new` — strictly **only when
     `next_cursor > base_cursor`** (so dependency-only or lifecycle-only
     actuations that allocate zero canonical cursors never collide on
     `PRIMARY KEY(write_cursor)`).
   - Compute `committed_at_ms = max(now_epoch_ms, prev_committed_at_ms)` where
     `prev_committed_at_ms` is read via `SELECT committed_at_ms FROM
     _fathomdb_commit_timestamps ORDER BY write_cursor DESC LIMIT 1` (PK reverse
     lookup, zero scan), preserving monotonicity across clock steps.
   - Classify `_fathomdb_commit_timestamps` as `ExportClass::DurableAuthority`
     in the `sqlite_master` audit, `bootstrap_is_fresh`, and
     `data_plane_integrity.rs` (zero user content/PII; not a user-mutated
     `VISIBILITY_TRIGGER_TABLES` table).
4. **Reserved `scope` column for `0.8.32`**:
   - Add `scope TEXT NOT NULL DEFAULT ''` to `canonical_nodes`,
     `canonical_edges`, `operational_mutations`, and `operational_state` in
     schema 38 (inert in `0.8.31`; activated by `0.8.32` engine-enforced caller
     scope without requiring another breaking schema bump for `0.8.31` stores).
5. **Explicit opt-in app-declared vector-committable kinds**:
   - Lift the fixed 7-kind node allowlist (`VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES`
     in `vector_storage.rs`). Built-in kinds keep their existing `source_type`
     mapping (`doc` -> `article`, others identity).
   - Any non-empty custom kind (without NUL or `\x1e`, and strictly rejecting
     reserved `"edge_fact"` to preserve node-vs-edge vector partition
     classification across `fathomdb-engine` and `fathomdb-inspector`) becomes
     vector-committable (`source_type = kind`) **only when explicitly enrolled**
     into `_fathomdb_vector_kinds` (never auto-enrolling arbitrary corpus kinds,
     so unenrolled kinds continue to report in `vector_unsupported_kinds`).
   - Replace `resolve_source_type -> &'static str` with owned/`Cow` resolution
     and update all SQL 7-kind mirrors (`filter.rs`, `projection_generation.rs`,
     `search.rs`, `data_plane_integrity.rs`) to admit enrolled
     `_fathomdb_vector_kinds`.
6. **Operational-collection kind readback**:
   - Add governed readback `read.operational_collections` /
     `read.operational_collection(name)` across Rust (`fathomdb-sdk`), Python,
     and TypeScript (registered in `governed-operation-parity.json`).
   - Validate existing `kind` vs. requested `kind` in `write_validation.rs` and
     re-check inside the writer transaction before `ON CONFLICT DO UPDATE` so
     re-registering an existing operational collection with a conflicting `kind`
     (`append_only_log` vs. `latest_state`) fails with `EngineError::WriteValidation`.
7. **Erasure, read-visibility, and completeness audit invariants**:
   - Register `canonical_property_values` in `ROW_OWNED_PROJECTIONS`
     (`projection_registry.rs`), attach read-visibility coalescing triggers via
     **`visibility_coalescing_trigger_sql!`** directly for
     `("cpv", "canonical_property_values")` in step 38 (not
     `read_visibility_coalescing_upgrade_sql!`, which would re-add the step-37
     `coalescing` column), and update `VISIBILITY_TRIGGER_TABLES` (`19 -> 20`
     for `schema_version >= 38`), `canonical_batch_has_no_custom_triggers`
     (`write_commit.rs`), `bootstrap_is_fresh` (`projection_generation.rs`), and
     `projection_registry_digest` (`frozen_read.rs`).
   - Extend `physical_erasure/admission.rs` (`envelope`) to sum
     `COALESCE(length(CAST(body AS BLOB)), 0) + COALESCE(length(CAST(properties AS BLOB)), 0)`
     on `canonical_nodes` and `canonical_edges` (both `canonical_edges.body`
     and `properties` are nullable), and include `canonical_property_values` in
     derived cleanup and `doctor` integrity verification.
   - Add an automated `sqlite_master` completeness audit test in
     `fathomdb-engine` asserting every non-SQLite-internal table in a
     bootstrapped schema-38 database (classifying dynamic `vector_default_*` and
     FTS5 shadow tables via deterministic prefix/suffix matchers, with
     `_fathomdb_vector_kinds` and `_fathomdb_commit_timestamps` pinned as
     `DurableAuthority`) is explicitly classified across:
     - `CANONICAL_TABLES` / `ROW_OWNED_PROJECTIONS` / internal `_fathomdb_*`,
     - read-visibility triggers,
     - erasure coverage,
     - `doctor` integrity checks, and
     - `ExportClass` (`DurableAuthority | OptInVector | OptInAudit |
       DerivedProjection | InternalRuntime`) so schema-38+ additions stay
       classified by construction for `0.8.31`–`0.8.34` upgrade/export/backup.

## Consequences

- Eliminates the need for projected-text indexing on the hybrid route because
  applications store clean plain text in `body` and move structured envelope
  metadata into inline `properties`.
- Preserves `0.8.30`'s `hybrid_deferred_identity` and streamed FTS fast paths
  with zero extra JOINs when no property predicate is supplied.
- Schema 37 stores are refused on direct `Engine::open`; live `0.8.30` upgrades
  use the `0.8.31` minimal 37→38 CLI helper
  ([ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md](ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md)).
