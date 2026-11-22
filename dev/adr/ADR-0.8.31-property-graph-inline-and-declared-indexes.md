---
title: ADR-0.8.31 — Property graph inline typed properties and declared indexes
date: 2026-11-22
target_release: 0.8.31
desc: Schema 38+ storage and indexing contract for outside-body typed properties on nodes and edges, reserved scope column, app-declared vector kinds, op-collection kind readback, and erasure/visibility completeness invariants.
blast_radius: src/rust/crates/fathomdb-schema/**; src/rust/crates/fathomdb-engine/**; src/rust/crates/fathomdb-sdk/**; src/rust/crates/fathomdb-py/**; src/rust/crates/fathomdb-napi/**; src/python/**; src/ts/**
status: accepted
---

# ADR-0.8.31: Property graph inline typed properties and declared indexes

## Context

FathomDB 0.8.30 (schema 37) stores structured metadata only inside
`canonical_nodes.body` JSON and projects node attributes into
`canonical_attributes(write_cursor, attr_name, attr_value)` (text-only) and
`vec0` metadata columns. `canonical_edges` has fixed columns only and no general
property store. Consumers such as Memex 0.6.2 therefore embed JSON envelopes
(UUIDs, SHA-256 digests, filter tags) inside `canonical_nodes.body`, polluting
FTS5 and the 512-wordpiece embedder, and cannot attach arbitrary typed
properties to edges or declare custom vector-indexed node kinds beyond the
hardcoded 7-kind allowlist in `VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES`
(`src/rust/crates/fathomdb-engine/src/vector_storage.rs`).

## Decision (Owner Option 1, 2026-11-21)

Advance `SCHEMA_VERSION` from `37` to `38` (migration step 38) with the
following storage, indexing, identity, erasure, and visibility contracts:

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
     - `num_val REAL`
     - `bool_val INTEGER CHECK(bool_val IS NULL OR bool_val IN (0, 1))`
   - Composite B-tree indexes:
     - `canonical_property_values_cursor_idx(write_cursor)`
     - `canonical_property_values_text_idx(entity_class, attr_name, text_val, write_cursor)`
     - `canonical_property_values_num_idx(entity_class, attr_name, num_val, write_cursor)`
     - `canonical_property_values_bool_idx(entity_class, attr_name, bool_val, write_cursor)`
   - Only properties declared `Filterable` via `configure_projections` (extended
     additively with optional `entity_class` defaulting to `node`) are written
     to `canonical_property_values` (synchronous, same-transaction) and, for
     node string/scalar filters, to `vec0` pre-KNN metadata columns.
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
3. **Reserved `scope` column for `0.8.32`**:
   - Add `scope TEXT NOT NULL DEFAULT ''` to `canonical_nodes`,
     `canonical_edges`, `operational_mutations`, and `operational_state` in
     schema 38 (inert in `0.8.31`; activated by `0.8.32` engine-enforced caller
     scope without requiring another breaking schema bump for `0.8.31` / Memex
     stores).
4. **App-declared vector-committable kinds (Memex R1)**:
   - Lift the fixed 7-kind node allowlist (`VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES`
     in `vector_storage.rs`). Built-in kinds keep their existing `source_type`
     mapping (`doc` -> `article`, others identity); any non-empty custom kind
     (without NUL or `\x1e`) explicitly enrolled via `admin.configure` /
     `configure_projections` (or persisted in `_fathomdb_vector_kinds`) is
     vector-committable with `source_type = kind`.
5. **Operational-collection kind readback (Memex R12)**:
   - Add governed readback `read.operational_collections` /
     `read.operational_collection(name)` across Rust (`fathomdb-sdk`), Python,
     and TypeScript (registered in `governed-operation-parity.json`).
   - Re-registering an existing operational collection with a conflicting `kind`
     (`append_only_log` vs. `latest_state`) fails with a typed validation error
     instead of silently retaining the old `kind`.
6. **Erasure, read-visibility, and completeness audit invariants**:
   - Register `canonical_property_values` in `ROW_OWNED_PROJECTIONS`
     (`projection_registry.rs`) and add it to the schema-38 read-visibility
     trigger manifest (`read_visibility_coalescing_upgrade_sql!` in
     `fathomdb-schema` and `VISIBILITY_TRIGGER_TABLES` in `frozen_read.rs`).
   - Extend `physical_erasure/admission.rs` (`envelope`) to include
     `length(CAST(properties AS BLOB))` alongside `body` on `canonical_nodes`
     and `canonical_edges`, and include `canonical_property_values` in derived
     cleanup and `doctor` integrity verification.
   - Add an automated `sqlite_master` completeness audit test in
     `fathomdb-engine` asserting every non-SQLite-internal table in a
     bootstrapped schema-38 database is explicitly classified across:
     - `CANONICAL_TABLES` / `ROW_OWNED_PROJECTIONS` / internal `_fathomdb_*`,
     - read-visibility triggers,
     - erasure coverage,
     - `doctor` integrity checks, and
     - `ExportClass` (`DurableAuthority | OptInVector | OptInAudit |
       DerivedProjection | InternalRuntime`) so schema-38+ additions stay
       classified by construction for `0.8.31`–`0.8.34` upgrade/export/backup.

## Consequences

- Eliminates Memex ask R2 (projected-text hybrid indexing) because Memex can
  store plain text in `body` and move envelope metadata into inline `properties`.
- Preserves `0.8.30`'s `hybrid_deferred_identity` and streamed FTS fast paths
  with zero extra JOINs when no property predicate is supplied.
- Schema 37 stores are refused on direct `Engine::open`; live `0.8.30` upgrades
  use the `0.8.31` minimal 37→38 CLI helper
  ([ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md](ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md)).
