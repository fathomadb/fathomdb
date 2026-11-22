---
title: Slice 20 Plan — 0.8.31 Property Graph Schema, Inline Properties, Declared Indexes & W12 Fixes
status: ACTIVE
target_release: 0.8.31
slice: 20
date: 2026-11-22
---

# Slice 20 — `0.8.31` Plan (`PROPERTY-SCHEMA`)

- **Branch / Worktree:** `release/0.8.31` at `/home/coreyt/projects/fathomdb-worktrees/release-0.8.31`
- **Depends on:** Slice 10 (`PREWORK-PREP`, landed at `3c4b84439`)
- **Primary Authorities:**
  - `dev/plans/0.8.31/plan.md` (`R31-01`, `R31-06`, `X1`–`X4`)
  - `dev/plans/0.8.31/scope.md`
  - `dev/plans/0.8.31/features/slice-20/prompt.md`
  - `dev/adr/ADR-0.8.31-property-graph-inline-and-declared-indexes.md`
  - `dev/adr/ADR-0.8.28-resumable-physical-cleanup.md`
  - `dev/plans/0.8.31/features/slice-0/w12-fence-review.md`
  - `dev/plans/0.8.31/prework/slice-{3,4,5,9-triage}.md`
  - `dev/plans/prompts/RELEASE-AND-SLICE-PLANNING-TEMPLATE.md`

---

## 1. Goals & Non-Goals

### Goals

1. **Advance on-disk schema to Schema `38` (`SCHEMA_VERSION = 38`):**
   - Land migration step `38` in `src/rust/crates/fathomdb-schema/src/lib.rs` with `-- MIGRATION-ACCRETION-EXEMPTION:` marker, zero `logical_id` writes/defaults (`check_migration_logical_id_pin`), `visibility_coalescing_trigger_sql!` for `("cpv", "canonical_property_values")`, `VISIBILITY_TRIGGER_TABLES` expanded from `19` to `20`, `canonical_batch_has_no_custom_triggers` / `bootstrap_is_fresh` / `projection_registry_digest` / `stored_coverage` updated, and `docs/compatibility/index.md` updated to schema `38` in lockstep (`scripts/check-public-doc-truth.py`).
2. **Deliver inline canonical typed properties outside `body` on both nodes and edges:**
   - Add `properties TEXT` to `canonical_nodes` and `canonical_edges`, typed Rust/Python/TypeScript `PropertyValue` (`String | Int | Float | Bool`) and `PropertyMap`, write/actuate input support (`PreparedWrite::{Node,Edge}`, `ProvenancedNodeV1`, `ProvenancedEdgeV1`, canonical `request_digest` hashing), immutable revisioned updates (`_fathomdb_artifact_revisions`), and O(1) read hydration on `NodeRecord`, `SearchHit`, `GraphTargetV1`, `GraphOriginV1`, `ResolvedEvidenceV1`, and `ResolvedGraphEvidenceV1` without polling `canonical_property_values`.
3. **Deliver declared typed property indexes (`canonical_property_values`) and `vec0` budget enforcement:**
   - Extend `_fathomdb_projection_registry` with `entity_class` (`'node' | 'edge'`) and `vector_kinds`, create `canonical_property_values` with 4 composite B-tree indexes (`_cursor_idx`, `_text_idx`, `_num_idx`, `_bool_idx`), add `SearchFilter.property_predicates` / `FilterTerm::Property` with typed rejection of undeclared filter keys, enforce active-only cleanup on node supersession + edge G0/G11 supersession + `record_lifecycle` transitions, thread `(body, properties)` through all projectors/backfills/relaxed-`ReadView` paths with `properties` > `body` precedence, and cap `vec0` node `Filterable` `Eq` slots at `8` (excluding `entity_class = 'edge'` and reserving `1` slot for `0.8.32` `scope`), routing `Ne`, numeric ranges, and slot overflow cleanly via `canonical_property_values`.
4. **Add Schema 38 commit timestamps (`_fathomdb_commit_timestamps`) and unified edge incident indexes:**
   - Create `_fathomdb_commit_timestamps(write_cursor INTEGER PRIMARY KEY, committed_at_ms INTEGER NOT NULL)` + `_fathomdb_commit_timestamps_ms_idx(committed_at_ms, write_cursor)`, populated atomically once per committed batch in both `commit_batch` and `actuate_new` strictly when `next_cursor > base_cursor`, with monotonic reverse-PK clamping.
   - Create `canonical_edges_{from,to}_active_idx` and `canonical_edges_{from,to}_sup_kind_idx(from_id|to_id, superseded_at, kind)` in Schema 38 so Slices 30–40 require zero further schema bumps.
5. **Reserve inert `scope` column for `0.8.32`:**
   - Add `scope TEXT NOT NULL DEFAULT ''` to `canonical_nodes`, `canonical_edges`, `operational_mutations`, and `operational_state`, verified inert in `0.8.31`.
6. **Lift hardcoded 7-kind vector allowlist via explicit opt-in enrollment:**
   - Allow custom node kinds (`source_type = kind`) to vector-index only when explicitly enrolled into `_fathomdb_vector_kinds` via `configure_projections`, strictly rejecting reserved `"edge_fact"` and invalid tokens while preserving `tc67_unsupported_vector_kind_report` for unenrolled kinds; replace `&'static str` `resolve_source_type` and update all SQL 7-kind mirrors across `filter.rs`, `projection_generation.rs`, `search.rs`, `index_projector.rs`, `projection_commit.rs`, and `data_plane_integrity.rs`.
7. **Add governed `read.operational_collections` / `read.operational_collection(name)` + `AdminSchema` kind-conflict refusal:**
   - Implement across `fathomdb-engine`, `fathomdb-sdk`, `fathomdb-py`, `src/python/`, `fathomdb-napi`, and `src/ts/`, updating `governed-operation-parity.json`, `governed-surface-allowlist.json`, `scripts/governed-surface-pin.json` (HITL Gate 1 pre-authorized), `python_surface_additions.json`, `test_slice130_package_boundary.py`, and `sdk-surface-parity.test.ts` (`53/53 -> 55/55`).
8. **Deliver W12 erasure ergonomics fixes, inline-property physical erasure, and `sqlite_master` + `ExportClass` completeness audit:**
   - Autonomous two-truncate ordering (first `wal_checkpoint(TRUNCATE)` before `prepare_redacted`), `ExciseReport.completed_pending_erasure` / `EraseReport.completedPendingErasure` with cross-source `excise_source_audit` retention guard, and always-on lock-free atomic WAL attribution counters driving release-mode `ErasureIncomplete.detail`.
   - Register `canonical_property_values` in `ROW_OWNED_PROJECTIONS`, charge `properties` bytes in `physical_erasure::admission::envelope`, extend `data_plane_integrity.rs`, and add automated `sqlite_master` + `ExportClass` completeness audit (positive + negative tests).
9. **Preserve workspace compilation and Inspector fixture hygiene (`P4-13`):**
   - Refactor literal constructions in `src/inspector/tests/{fixtures/mod.rs,integration/*.rs}` into local helpers and keep `cargo clippy --workspace --all-targets` and `cargo test --workspace` green on Schema `38`.

### Non-Goals (Explicitly Excluded to Subsequent Slices & `0.8.32+`)

- **Slice 30 (`STRICT-GRAPH`):** Strict-only write endpoint integrity enforcement (`ADR-0.8.31-strict-endpoint-integrity.md`, legacy dangling-edge test rewrites), ad-hoc and indexed `edge_predicates`/`target_predicates` execution on `graph.expand`/`use_graph_arm`, opt-in `graph_seed` (`"fused_top_k" | "graph_fts" | "both"`) + seed provenance, and mixed-edge `SEED_FTS_N` slot fix.
- **Slice 40 (`HISTORY-GRAPH`):** Historical `as_of_write_cursor` / `as_of_epoch_ms` graph traversal execution in `graph_expand/execution.rs` and historical `evidence.rs`.
- **Slice 50 (`SDK-CLI-UPGRADE`):** `resolve_evidence_many`, `search(hints=)`, `OpenReport` capability booleans, minimal schema-37->38 `fathomdb migrate` CLI helper, and `fathomdb-cli` PyPI binary wheels.
- **Slice 60 (`INSPECTOR`):** Inspector schema-38 UI/worker/browser upgrades for properties and `as_of`, Inspector dangling-edge fixture conversions, `local-ci.sh` scoped default, and harness timeouts.
- **Slice 70 (`QUALIFICATION`):** Full paired `0.8.30` vs. `0.8.31` 10k/50k/100k bootstrap latency gate, `0.8.31` release notes, generated docs regeneration, golden schema-38 archive, and `AGENT_LONG=1` full verify.
- **Slice 80 (`PUBLICATION`):** Registry publication (`crates.io`, PyPI, npm, Tegra, Inspector) and `RELEASE_SCHEMAS["0.8.31"] = 38` flip in `scripts/check-public-doc-truth.py`.
- **`0.8.32+`:** Caller `scope` enforcement/indexing/MCP, opt-in bulk write mode, and CLI `export`/`import` implementation (`ExportClass` is audited as a schema classification contract in Slice 20 only).

---

## 2. Draft Needs, Requirements & Acceptance Criteria

> **Invariant:** `dev/acceptance.md` remains **LOCKED** (zero edits to canonical `AC-NNN` IDs or thresholds). All Slice 20 verification binds release requirements `R31-01` and `R31-06` via slice-local `AC31-20-*` criteria and named TDD tests.

| Local AC | Traces To | Falsifiable Criterion | Primary Verification Tests |
| --- | --- | --- | --- |
| `AC31-20-1` | `R31-01` | Fresh migrate reaches `SCHEMA_VERSION == 38`; `canonical_nodes` and `canonical_edges` carry `properties TEXT` and `scope TEXT NOT NULL DEFAULT ''`; `operational_mutations` and `operational_state` carry `scope TEXT NOT NULL DEFAULT ''`; `_fathomdb_projection_registry` carries `entity_class` and `vector_kinds`; `canonical_property_values` + 4 indexes, `_fathomdb_commit_timestamps` + `_ms_idx`, `canonical_edges_{from,to}_active_idx`, `canonical_edges_{from,to}_sup_kind_idx`, and `60` coalescing visibility triggers (`20 * 3`, including `cpv_{ai,au,ad}`) exist cleanly; `logical_id` pin and accretion guard hold; `Engine::open` refuses schema `37`; `docs/compatibility/index.md` documents schema `38`. | `fathomdb-schema/tests/step38_property_schema.rs`, `fathomdb-schema/tests/slice25_logical_id_pin.rs`, `fathomdb-engine/tests/slice95_visibility_coalescing.rs`, `scripts/check-public-doc-truth.py` |
| `AC31-20-2` | `R31-01` | Nodes and edges store `String`, `Int` (`i64::MIN..=i64::MAX`, exact `abs(x) > 2^53`), `Float` (canonicalized finite IEEE-754, `-0.0` handled cleanly), and `Bool` properties outside `body`; empty `Some(map)` normalizes to SQL `NULL`; `NaN`/`±Inf`, non-finite, empty keys, `\0`/`\x1e`, nested JSON/arrays/nulls, and Py/TS out-of-range integers fail atomically with `WriteValidation` (`WriteValidationError`); `body`-FTS and embedding vectors exclude property keys/values; O(1) hydration populates `NodeRecord`, `SearchHit`, `GraphTargetV1`, `GraphOriginV1`, `ResolvedEvidenceV1`, and `ResolvedGraphEvidenceV1` without querying `canonical_property_values`; `actuate` `request_digest` binds `properties` canonically. | `fathomdb-engine/tests/slice38_properties_and_indexes.rs`, `fathomdb-sdk/tests/slice38_properties.rs`, `src/python/tests/test_slice38_properties.py`, `src/ts/tests/slice38-properties.test.ts` |
| `AC31-20-3` | `R31-01` | Declared `Filterable` node and edge properties (`configure_projections` with `entity_class`) synchronize into `canonical_property_values` on write/actuate/backfill/transition; `properties` takes precedence over `json_extract(body, ...)` on matching declared keys; node supersession, edge G0 (`logical_id`) & G11 (`(from_id, to_id, kind)`) supersession, and `transition` to `Deleted`/`Pending` purge stale `canonical_property_values` rows while transitioning back to `Active` re-projects from `(body, properties)`; property updates create new `write_cursor` and `_fathomdb_artifact_revisions` rows; `SearchFilter.property_predicates` / `FilterTerm::Property` execute indexed `SEARCH` via `canonical_property_values_{text,num,bool}_idx` and `vec0` node `Eq`, preserve exact 64-bit `INTEGER` vs `REAL` comparison (`2^53 + 1 != 2^53`), and reject undeclared or edge-only filter keys with `InvalidFilter`. | `fathomdb-engine/tests/slice38_properties_and_indexes.rs`, `fathomdb-engine/tests/slice38_query_plans.rs`, `src/python/tests/test_slice38_properties.py`, `src/ts/tests/slice38-properties.test.ts` |
| `AC31-20-4` | `R31-01` | `desired_vector_attr_columns` excludes `entity_class = 'edge'`, reserves `1` dynamic `vec0` metadata slot for `0.8.32` `scope`, caps `vec0` `attr_<hex>` metadata columns at `8` node `Filterable` properties (`MAX_VEC0_NODE_PROPERTY_SLOTS = 8`), fixes stale built-in column count comments, ensures String vs Int/Float/Bool values never collide on `vec0` `TEXT` columns, and routes `Ne`, numeric range predicates, numeric `Eq`, and overflow (`9+`) declared node properties cleanly via exact `canonical_property_values` pre-selection (zero `LIMIT 192` starvation). | `fathomdb-engine/tests/slice38_properties_and_indexes.rs`, `fathomdb-engine/src/vector_storage.rs` unit tests |
| `AC31-20-5` | `R31-01` | `_fathomdb_commit_timestamps` records exactly one monotonically non-decreasing `committed_at_ms` row per committed canonical/op-store batch keyed by high-water `WriteReceipt.cursor` in both `commit_batch` and `actuate_new` strictly when `next_cursor > base_cursor`, and performs zero inserts (never colliding on PK) for dependency-only or lifecycle-only actuations; `scope` remains `''` and zero-cost across all canonical and operational writes. | `fathomdb-engine/tests/slice38_properties_and_indexes.rs`, `fathomdb-engine/src/actuation.rs` unit tests |
| `AC31-20-6` | `R31-01` | Explicit opt-in custom node vector-kind enrollment (`ProjectionVector.kinds` / `vector_kinds`) enrolls valid non-empty node kinds into `_fathomdb_vector_kinds` (`source_type = kind`), vector-indexes + searches + drains + reopens cleanly, strictly refuses `""`, whitespace-only, `\0`/`\x1e`, and reserved `"edge_fact"` with `WriteValidation`, replaces `&'static str` `resolve_source_type` and SQL 7-kind mirrors, and leaves unenrolled kinds (`"entity"`, `"invoice"`) reported in `vector_unsupported_kinds` with `tc67_unsupported_vector_kind_report` and `slice22_projection_status` green across Rust/Py/TS. | `fathomdb-engine/tests/slice38_vector_kinds.rs`, `tc67_unsupported_vector_kind_report`, `slice22_projection_status`, `test_slice38_properties.py`, `slice38-properties.test.ts` |
| `AC31-20-7` | `R31-01` | `read.operational_collections` and `read.operational_collection(name)` return deterministic metadata parity (`name`, `kind`, `format_version`, `schema_json`, `retention_json`) across Rust (`fathomdb-sdk`), Python, and TypeScript (`None`/`null` for missing); `AdminSchema` conflicting `kind` re-registration is atomically refused with `WriteValidation` in `validate_write` and transactionally before `ON CONFLICT DO UPDATE`; `governed-operation-parity.json` (`55/55`), `governed-surface-allowlist.json`, `scripts/governed-surface-pin.json`, `python_surface_additions.json`, `test_slice130_package_boundary.py`, and `sdk-surface-parity.test.ts` pass in lockstep. | `fathomdb-engine/tests/slice38_op_collections.rs`, `fathomdb-sdk/tests/surface.rs`, `src/python/tests/test_slice130_package_boundary.py`, `src/python/tests/test_surface.py`, `src/ts/tests/sdk-surface-parity.test.ts` |
| `AC31-20-8` | `R31-06` | `canonical_property_values` is registered in `ROW_OWNED_PROJECTIONS` (`guard_row_owned_registry` fails if omitted); automated `sqlite_master` + `ExportClass` audit classifies every non-internal schema-38 table (including dynamic `vector_default_*` / FTS5 shadows via deterministic rules and `_fathomdb_vector_kinds` + `_fathomdb_commit_timestamps` as `DurableAuthority`) and fails on any unclassified dummy table. | `fathomdb-engine/src/lib.rs` (`guard_row_owned_registry`), `fathomdb-engine/tests/slice38_erasure_and_audit.rs` |
| `AC31-20-9` | `R31-06` | `purge` and `erase_source` (both Autonomous and ExplicitMaintenance) leave zero `canonical_property_values` rows, zero custom-kind `vector_default` rows, zero raw byte substrings of inline node or nullable-`body` edge `properties` across `main`, `-wal`, `-shm`, and telemetry sinks, valid closure proofs, and `physical_erasure::admission::envelope` accurately charges `body` + `properties` bytes. | `fathomdb-engine/tests/slice38_erasure_and_audit.rs` |
| `AC31-20-10` | `R31-06` | W12 erasure fixes hold: (a) Autonomous first `wal_checkpoint(TRUNCATE)` runs before `prepare_redacted`, so on checkpoint BUSY `debt.redacted` remains `false`, unrelated plain writes succeed + advance `safe_boundary`, denied sources/identities and dependency/actuation writes stay fenced, retry finishes cleanly, and ExplicitMaintenance preserves existing order; (b) `ExciseReport.completed_pending_erasure` / `EraseReport.completedPendingErasure` distinguishes completing retries (`true`) from fresh/already-erased no-ops (`false`), suppresses duplicate 0-row `excise_source_audit` rows, and preserves `excise_source_audit` whenever `nodes_excised + edges_excised > 0` across sources; (c) always-on lock-free atomic counters in `WalAttributionCollector` classify BUSY causes truthfully in release builds without `FATHOMDB_WAL_ATTRIBUTION`. | `fathomdb-engine/tests/slice38_erasure_and_audit.rs`, `fathomdb-sdk/tests/behavior.rs`, `src/python/tests/test_erase_source.py`, `src/ts/tests/erase-source.test.ts` |
| `AC31-20-11` | `R31-01`, `R31-06` | `EXPLAIN QUERY PLAN` pins in `fathomdb-engine/tests/slice38_query_plans.rs` assert indexed `SEARCH` (`canonical_property_values_{text,num,bool}_idx`, `canonical_nodes_write_cursor_idx`, `_fathomdb_commit_timestamps`, `canonical_edges_{from,to}_active_idx`, `canonical_edges_{from,to}_sup_kind_idx` for both empty and non-empty `edge_kinds`) and assert unpredicated `search` / `search_text_only` / current `graph.expand` emit zero joins onto `canonical_property_values` or `superseded_at` historical scans. | `fathomdb-engine/tests/slice38_query_plans.rs` |
| `AC31-20-12` | `R31-01`, `R31-06` | `src/inspector/tests/{fixtures/mod.rs,integration/*.rs}` exhaustive `PreparedWrite` / `Provenanced*` / `ProjectionSpec` literals are wrapped into local helper constructors and `cargo clippy --workspace --all-targets` + `cargo test --workspace` compile and pass cleanly on Schema `38`. | `cargo clippy --workspace --all-targets`, `cargo test --workspace` |

---

## 3. Architecture & Draft Design

### 3.1 Schema Step 38 Exact SQL DDL (`src/rust/crates/fathomdb-schema/src/lib.rs`)

Advance `pub const SCHEMA_VERSION: u32 = 38;` and append `Migration { step_id: 38, sql: concat!(...) }` using the `-- MIGRATION-ACCRETION-EXEMPTION:` marker (required by `check_migration_accretion`) and referencing zero `logical_id` mutations or defaults (`check_migration_logical_id_pin`):

```sql
-- MIGRATION-ACCRETION-EXEMPTION: 0.8.31 Schema 38 inline typed properties, declared property B-tree indexes, batch commit timestamps, reserved 0.8.32 scope columns, schema-38 edge incident indexes, and cpv visibility coalescing triggers; additive shape only, no canonical or projection content migration.
ALTER TABLE canonical_nodes ADD COLUMN properties TEXT;
ALTER TABLE canonical_nodes ADD COLUMN scope TEXT NOT NULL DEFAULT '';
ALTER TABLE canonical_edges ADD COLUMN properties TEXT;
ALTER TABLE canonical_edges ADD COLUMN scope TEXT NOT NULL DEFAULT '';
ALTER TABLE operational_mutations ADD COLUMN scope TEXT NOT NULL DEFAULT '';
ALTER TABLE operational_state ADD COLUMN scope TEXT NOT NULL DEFAULT '';
ALTER TABLE _fathomdb_projection_registry ADD COLUMN entity_class TEXT NOT NULL DEFAULT 'node' CHECK(entity_class IN ('node', 'edge'));
ALTER TABLE _fathomdb_projection_registry ADD COLUMN vector_kinds TEXT;

CREATE TABLE canonical_property_values(
    entity_class TEXT NOT NULL CHECK(entity_class IN ('node', 'edge')),
    write_cursor INTEGER NOT NULL CHECK(write_cursor > 0),
    attr_name TEXT NOT NULL CHECK(length(attr_name) > 0),
    value_type TEXT NOT NULL CHECK(value_type IN ('string', 'integer', 'float', 'boolean')),
    text_val TEXT,
    num_val NUMERIC,
    bool_val INTEGER CHECK(bool_val IS NULL OR bool_val IN (0, 1)),
    PRIMARY KEY(entity_class, write_cursor, attr_name),
    CHECK(
        (value_type = 'string' AND text_val IS NOT NULL AND num_val IS NULL AND bool_val IS NULL) OR
        (value_type IN ('integer', 'float') AND text_val IS NULL AND num_val IS NOT NULL AND bool_val IS NULL) OR
        (value_type = 'boolean' AND text_val IS NULL AND num_val IS NULL AND bool_val IS NOT NULL)
    )
);
CREATE INDEX canonical_property_values_cursor_idx
    ON canonical_property_values(write_cursor);
CREATE INDEX canonical_property_values_text_idx
    ON canonical_property_values(entity_class, attr_name, text_val, write_cursor);
CREATE INDEX canonical_property_values_num_idx
    ON canonical_property_values(entity_class, attr_name, num_val, write_cursor);
CREATE INDEX canonical_property_values_bool_idx
    ON canonical_property_values(entity_class, attr_name, bool_val, write_cursor);

CREATE TABLE _fathomdb_commit_timestamps(
    write_cursor INTEGER PRIMARY KEY CHECK(write_cursor > 0),
    committed_at_ms INTEGER NOT NULL
);
CREATE INDEX _fathomdb_commit_timestamps_ms_idx
    ON _fathomdb_commit_timestamps(committed_at_ms, write_cursor);

CREATE INDEX canonical_nodes_logical_sup_idx
    ON canonical_nodes(logical_id, superseded_at, write_cursor);
CREATE INDEX canonical_edges_from_active_idx
    ON canonical_edges(from_id, kind) WHERE superseded_at IS NULL;
CREATE INDEX canonical_edges_to_active_idx
    ON canonical_edges(to_id, kind) WHERE superseded_at IS NULL;
CREATE INDEX canonical_edges_from_sup_kind_idx
    ON canonical_edges(from_id, superseded_at, kind);
CREATE INDEX canonical_edges_to_sup_kind_idx
    ON canonical_edges(to_id, superseded_at, kind);
```

Followed immediately inside the same `concat!(...)` by **direct invocation of `visibility_coalescing_trigger_sql!`** (`P4-04` — never `read_visibility_coalescing_upgrade_sql!`, which would fail trying to re-add `_fathomdb_read_visibility_state.coalescing` added in step 37):

```rust
visibility_coalescing_trigger_sql!("cpv", "ai", "INSERT", "canonical_property_values"),
visibility_coalescing_trigger_sql!("cpv", "au", "UPDATE", "canonical_property_values"),
visibility_coalescing_trigger_sql!("cpv", "ad", "DELETE", "canonical_property_values"),
```

Coalescing and open-validation lockstep updates:

- `src/rust/crates/fathomdb-engine/src/frozen_read.rs`:
  - Expand `VISIBILITY_TRIGGER_TABLES` from `19` to `20`, appending `("cpv", "canonical_property_values")` at index `19`.
  - In `validate_on_open`, select `&VISIBILITY_TRIGGER_TABLES[..20]` when `schema_version >= 38` (retaining `[..19]` for `35..=37`).
  - Gate `projection_registry_digest` (`frozen_read.rs:561`) explicitly on `schema_version >= 38`: for `< 38`, keep the legacy 6-column SELECT unchanged (so `normative_v1_fixture_remains_pinned_at_schema_31` and `normative_v2_fixture_pins_projection_generation_binding` stay 100% intact); for `>= 38`, select `name,roles,fts_tokenizer,vector_embedder,vector_declared,source,entity_class,vector_kinds` and encode `entity_class` + `vector_kinds` into the digest.
  - Gate `encode_context` (`frozen_read.rs:656`) so an empty `filter.property_predicates` emits **zero** extra bytes (keeping normative schema-31/32 `context_encoding_hex` / `context_digest` byte-identical) while non-empty `property_predicates` encodes a versioned canonical tag + sorted predicate vector.
- `src/rust/crates/fathomdb-engine/src/projection_generation.rs`:
  - Gate `declaration_digest` (`projection_generation.rs:468-495`) identically on `schema_version >= 38` so serving-generation digests incorporate `entity_class` and `vector_kinds` on Schema 38+ without breaking `< 38` fixtures.
  - Extend `bootstrap_is_fresh` to include `"canonical_property_values"` and `"_fathomdb_commit_timestamps"` when `schema_version >= 38`.
- `src/rust/crates/fathomdb-engine/src/write_commit.rs`:
  - Add `'canonical_property_values'` to both `sqlite_master` and `sqlite_temp_master` `tbl_name IN (...)` lists in `canonical_batch_has_no_custom_triggers`.
- `src/rust/crates/fathomdb-engine/src/stored_coverage.rs`:
  - Replace hardcoded `token.user_version != 37` with `token.user_version != fathomdb_schema::SCHEMA_VERSION`.
- `dev/plans/release-state-0.8.31.json:8` & `docs/compatibility/index.md:144`:
  - Bump `"schema_version": 38` in `dev/plans/release-state-0.8.31.json` in lockstep with `SCHEMA_VERSION = 38` and `docs/compatibility/index.md` (`scripts/check-public-doc-truth.py` fails closed on drift).

### 3.2 Rust / Python / TypeScript Types & Canonical Serialization

1. **Core property types (`fathomdb-engine`, re-exported from `fathomdb` and `fathomdb-sdk`):**

   ```rust
   #[derive(Clone, Debug, PartialEq)]
   pub enum PropertyValue {
       String(String),
       Int(i64),
       Float(f64),
       Bool(bool),
   }

   pub type PropertyMap = std::collections::BTreeMap<String, PropertyValue>;

   #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
   pub enum PropertyEntityClass {
       #[default]
       Node,
       Edge,
   }

   #[derive(Clone, Copy, Debug, Eq, PartialEq)]
   pub enum PropertyComparisonOp {
       Eq,
       Ne,
       Lt,
       Lte,
       Gt,
       Gte,
   }

   #[derive(Clone, Debug, PartialEq)]
   pub struct PropertyPredicate {
       pub key: String,
       pub op: PropertyComparisonOp,
       pub value: PropertyValue,
   }
   ```

   - **Trait boundary (`Eq` -> `PartialEq`):** Because `PropertyValue::Float(f64)` implements `PartialEq` but not `Eq`, drop `Eq` from structs/enums embedding `PropertyMap` or `PropertyPredicate` (`SearchFilter`, `ReadContextV1`, `FrozenReadContextV1`, `NodeRecord`, `GraphOriginV1`, `GraphTargetV1`, `BoundaryCrossing`, `EvidenceResolveRequestV1`, `GraphEvidenceResolveRequestV1`, `ResolvedGraphEvidenceV1`, `fathomdb_sdk::TextSearchOptions`, `fathomdb_sdk::ProjectedTextSearchOptions`), keeping `PartialEq` cleanly (matching `SearchHit`, `SearchResult`, `ProvenancedNodeV1`, `ProvenancedEdgeV1`, `ResolvedEvidenceV1`, `ActuationBatchV1`).
   - **Canonical JSON & codec rules (`validate_and_encode_properties` / `decode_properties`):**
     - Reject `key.is_empty()`, `key.contains(['\0', '\x1e'])`, `PropertyValue::String` containing `'\0'` or `'\x1e'`, and `!f.is_finite()` (`NaN`, `+inf`, `-inf`) with `EngineError::WriteValidation`. Normalize `-0.0` to `0.0` before JSON encoding so canonical encoding never emits `"-0.0"`.
     - If `properties` is `None` or `Some(map)` where `map.is_empty()`, persist SQL `NULL` and read back `None`. Otherwise serialize as canonical sorted compact JSON object (`BTreeMap` key order, exact `i64` decimal, finite `f64` JSON number, `true`/`false`, string).
     - On raw JSON decoding (`canonical_nodes.properties` / `canonical_edges.properties`), parse JSON object members distinguishing exact JSON integer (`v.as_i64()`) vs finite float (`v.as_f64()`) vs bool (`v.as_bool()`) vs string (`v.as_str()`). Reject corrupt/non-object stored `properties` with `EngineError::Storage`.
     - **Python (`fathomdb-py` + `fathomdb`):** Translate `dict[str, str | int | float | bool] | None`; check `PyBool` **before** `PyInt` (`bool` subclasses `int` in Python), verify signed 64-bit `i64` bounds (rejecting oversized Python `int` with `WriteValidationError`), verify finite `float` and canonicalize `-0.0` to `0.0` at the boundary, reject `None` values inside the map and nested `dict`/`list`.
     - **TypeScript (`fathomdb-napi` + `src/ts`):** Support `number | boolean | string | bigint` in TS write inputs (`Record<string, PropertyValue>`) and serialize exact property maps canonically across N-API (or tagged N-API representation) so exact 64-bit integers (`abs(x) > 2^53`) pass losslessly via `bigint` (`Int`) alongside IEEE-754 `number` (`Object.is(v, -0)` normalized to `0.0`, `Number.isSafeInteger` -> `Int`, non-integer finite `number` -> `Float`), with safe runtime decoders returning `bigint` only when outside `Number.MIN_SAFE_INTEGER..=Number.MAX_SAFE_INTEGER` or a canonical `PropertyValue` parity helper documented in TypeScript interfaces. Reject `null`, nested objects/arrays, symbols, `NaN`/`Infinity`, lone surrogates (`validateFfiString`), and out-of-range `bigint` (`WriteValidationError`).

2. **Additive public struct & read-model extensions (`O(1)` inline hydration):**
   - **Writes:** Add `properties: Option<PropertyMap>` to `PreparedWrite::Node`, `PreparedWrite::Edge`, `ProvenancedNodeV1`, and `ProvenancedEdgeV1`. Thread through `storage_write_shape`, `fathomdb-sdk/src/guard.rs` (`guard::prepared_write` validating property keys/strings/finite floats and `guard::projection_spec` validating `spec.vector.kinds`), `write_commit.rs` (`runtime_revision_id` determinism stays keyed by cursor/source/content cleanly alongside `properties`), and `src/rust/crates/fathomdb-engine/src/actuation.rs` `encode_node` / `encode_edge` (`request_digest` encodes optional `PropertyMap` deterministically with length, sorted key, type tag, and canonical value bytes so different `properties` always yield different SHA-256 digests).
   - **Reads (`O(1)` directly from `canonical_nodes.properties` / `canonical_edges.properties`, zero `canonical_property_values` joins):**
     - `NodeRecord` (`read.rs`): add `pub properties: Option<PropertyMap>` (hydrated in `read_get_by_id_in_tx`, `read_list_in_tx`, `query_canonical_page_rows`, `graph_expand/traversal.rs`).
     - `SearchHit` (`search_types.rs`): add `pub properties: Option<PropertyMap>` (hydrated in every `SearchHit` path in `search.rs`, `rerank.rs`, `fusion.rs`; `None` when absent).
     - `GraphTargetV1` (`graph_expand/types.rs`): add `pub properties: Option<PropertyMap>` (serialized with `#[serde(skip_serializing_if = "Option::is_none")]` in `graph_expand/codec.rs` `TargetWire` so `slice60_graph_expand.rs::ordinary_graph_response_bytes_remain_literal_with_parallel_edges` remains byte-identical when `properties` is `None`).
     - `GraphOriginV1` (`graph_expand/types.rs`): add `pub terminal_edge_properties: Option<PropertyMap>` (serialized with `#[serde(skip_serializing_if = "Option::is_none")]` in `graph_expand/codec.rs` `OriginWire`).
     - `ResolvedEvidenceV1`, `PendingGraphEvidence`, and `GraphEvidenceArtifactV1::{Node, Edge}` (`evidence.rs`): add `pub properties: Option<PropertyMap>`, hydrate directly from `(body, properties)` in `resolve_evidence` and `resolve_graph_evidence`, and serialize `"properties"` with `skip_serializing_if = "Option::is_none"` in `encode_resolved_graph_evidence_v1` / evidence wire codecs.
     - Extend `SearchFilter` (`#[non_exhaustive]`) with `pub property_predicates: Vec<PropertyPredicate>` and `FilterTerm` with `Property(PropertyPredicate)`; update `Filter::lower_for_read_list` (`filter.rs:362-397`) to lower `FilterTerm::Property` and take `&Connection` so `FilterTerm::SourceType` calls `resolve_source_type_on`; update `graph_expand/codec.rs` `FilterWire` (`L62-68`) and `decode_graph_expand_request_v1` `check_closed` allowlist (`L360-364`) to accept optional `propertyPredicates` while omitting when empty.
     - Extend `ProjectionSpec` with `pub entity_class: PropertyEntityClass` (defaulting to `PropertyEntityClass::Node` in Py/TS/JSON inputs when omitted) and `ProjectionVector` with `pub kinds: Option<Vec<String>>` (exposed on Py/TS `ProjectionSpec` as optional `entity_class` / `entityClass` and `vector_kinds` / `vectorKinds`).

### 3.3 Declared `canonical_property_values`, `vec0` 8-Slot Budget & `(body, properties)` Threading (`P4-01`, `P4-02`, `P4-06`)

1. **`ROW_OWNED_PROJECTIONS` & active-only maintenance:**
   - Register `RowOwnedProjection { table: "canonical_property_values", cursor_column: "write_cursor", class: ProjectionClass::PropertyValue }` in `ROW_OWNED_PROJECTIONS` (`projection_registry.rs`).
   - Validation in `configure_projections`:
     - Enforce `is_valid_attribute_name` rules cleanly (aligned with `PropertyMap` key validation while preserving safe tokenizer/SQL handling).
     - If `entity_class == PropertyEntityClass::Edge`: allow `ProjectionRole::Filterable` only; reject `Rankable` or `Searchable` (`fts` / `vector`) with `EngineError::WriteValidation` (`property_search_index` and `vector_default` `attr_<hex>` metadata columns are node-only).
     - Changing `entity_class` or dropping `Filterable` on an existing projection name without listing `name` in `drop` triggers `ProjectionDestructive`.
   - **Projecting `(body, properties)` (`P4-02`):**
     - Precedence rule for a declared `Filterable` key `name`:
       1. For `entity_class == 'node'`: check `properties.get(name)` first; if present, use its typed `PropertyValue`. Otherwise, fall back to `extract_json_typed_property(body, &projection_path)` (preserving existing `canonical_attributes` text writes intact for `attributes` queries while indexing JSON scalars into `canonical_property_values`).
       2. For `entity_class == 'edge'`: extract strictly from `canonical_edges.properties` (`Option<&PropertyMap>`).
     - Numeric `NUMERIC` affinity nuance: bind `i64` for `PropertyValue::Int(i)` (`value_type = 'integer'`), canonical finite `f64` for `PropertyValue::Float(f)` (`value_type = 'float'`), UTF-8 `text` for `PropertyValue::String(s)` (`value_type = 'string'`), and `0 | 1` integer for `PropertyValue::Bool(b)` (`value_type = 'boolean'`). Note: SQLite converts integral floats like `1.0` inside `num_val NUMERIC` to `INTEGER` storage internally while `value_type = 'float'` preserves declared type and `properties` JSON preserves exact type on hydration; `i64` with `abs(x) > 2^53` keeps exact 64-bit integer precision in `num_val`.
   - **Active-only lifecycle hooks (`ADR-0.8.31 #73`):**
     - Node supersession (`write_commit.rs` + `actuation.rs` via `apply_batch_in_transaction`): pass `&[ProjectionClass::Attribute, ProjectionClass::PropertyFts, ProjectionClass::PropertyValue]` to `purge_row_projections_for_cursor_in`.
     - Edge supersession (`write_commit.rs`): both **G0 (`logical_id`)** and **G11 (`(from_id, to_id, kind)`)** loops call `purge_row_projections_for_cursor_in(tx, *sc, &[ProjectionClass::PropertyValue])`.
     - Lifecycle transitions (`record_lifecycle.rs::transition` and `actuation.rs::apply_lifecycle`): purge `ProjectionClass::PropertyValue` on transition out of `Active`; select `SELECT ..., properties FROM canonical_nodes` and pass `(body, properties)` into `project_node_attributes`, `project_node_property_values`, and `refresh_vector_attr_values_for_row` when transitioning back to `Active`.
     - Also pass `(body, properties)` and handle `ProjectionClass::PropertyValue` across all remaining rebuild/boot/clear/read paths:
       - `projection_commit.rs` (`SELECT body, properties FROM canonical_nodes WHERE write_cursor = ?1 LIMIT 1`),
       - `projection_registry.rs`: `truncate_all_row_projections` (`L496`, add `ProjectionClass::PropertyValue`), `load_projection_registry_row` (`L920`, `operator` feature), `clear_attribute_projection` (`L1010`, purge `canonical_property_values` for `(entity_class, name)`), `backfill_attribute`, `apply_projection_config`, `rederive_projections_on_boot` (`L2268-2320`, handle node and edge `(body, properties)` into `canonical_property_values`), and `is_destructive_projection_change` / `apply_projection_config` (`L1233, 1553-1592`, reconcile `_fathomdb_vector_kinds` to the exact union of remaining declared `vector_kinds` across all specs whenever any spec shrinks/drops `vector_kinds`, not only when `vector_declared_after == false`!),
       - `projection_rebuild.rs` (`L98-171`): include `properties` in `canonical_node_rows` and `edge_rows`, and restrict `canonical_property_values` edge rebuild strictly to `superseded_at IS NULL`,
       - `index_projector.rs` (`L23-96, 264-288, 353`): include `properties` in `restore_registered_derived_projections` and replace static `kind_is_vector_committable` with `kind_is_vector_committable_on`,
       - `vector_storage.rs::reshape_vector_partition_nondestructive` (`L374-393`): replace raw `scalar_attribute_sql("cn.body", ...)` with `(cn.body, cn.properties)` precedence encoding,
       - `filter.rs` & `read.rs`: in `append_node_eligibility_sql_view`, `text_hit_passes_filter_view`, `build_vector_phase1_sql`, and `read.rs::canonical_page_query` (`L398`), whenever `view.include_superseded || view.include_inactive` is set, evaluate `property_predicates` inline against `(cn.body, cn.properties)` JSON instead of active-only `canonical_property_values`.

2. **`vec0` 16-column metadata budget & typed `SearchFilter` lowering (`P4-01`, `P4-06`):**
   - Update `vector_storage.rs` doc comment (*"seven built-in metadata columns..."*) and enforce `const MAX_VEC0_NODE_PROPERTY_SLOTS: usize = 8;` (`16` total minus `7` built-in minus `1` reserved for `0.8.32` `scope`).
   - `desired_vector_attr_columns` filters `_fathomdb_projection_registry` to `stored.entity_class == PropertyEntityClass::Node && stored.roles.contains(&ProjectionRole::Filterable)`, sorts deterministically by attribute name, and truncates via `.take(MAX_VEC0_NODE_PROPERTY_SLOTS)`. Properties 9+ live exclusively in `canonical_property_values`.
   - `vec0` `attr_<hex>` column encoding for declared node `Filterable` properties (`vector_attr_insert_fragments` / `encode_property_vec0_metadata` — since `vec0` rejects SQL `NULL` and requires every column bound on `INSERT`):
     - `String(s)` -> `"\u{1}" + s` (100% compatible with existing `attributes` tests).
     - `Bool(b)` -> `"\u{2}b:1"` / `"\u{2}b:0"` (distinct prefix `\u{2}` prevents `"1"` string collision).
     - `Int(i)` / `Float(f)` / absent -> `""` sentinel in `vec0` `attr_<hex>` (all numeric predicates route to `canonical_property_values_num_idx` so exact `i64` precision for `abs(x) > 2^53` and cross-type `Int(5) == Float(5.0)` never rely on string-encoded `vec0` metadata; legacy `SearchFilter.attributes` string equality continues matching `"\u{1}" + s` identically).
   - **Pre-KNN vs exact-fallback vector filtering (`build_vector_phase1_sql`, `filter.rs:564-598`):**
     - Node `Eq` (`PropertyComparisonOp::Eq`) on `String` or `Bool` occupying one of the `<= 8` live `vec0` columns emits pre-KNN `attr_<hex> = ?` directly inside `vector_filter_clause` (`LIMIT TOP_K_BIT_CANDIDATES` scans only matching candidates).
     - When a query carries any non-`vec0`-pushable property predicate (`Ne`, `<`, `<=`, `>`, `>=`, numeric `Eq`, slot `9+`, or relaxed `ReadView` `include_superseded || include_inactive`): do **not** apply `canonical_property_values` `EXISTS(...)` only after the hardcoded `LIMIT 192` global Hamming subquery (which would drop recall whenever matching rows sit outside the top-192 global unfiltered vectors!). Instead, pre-select eligible `write_cursor`s via `canonical_property_values` B-tree indexes (`_text_idx`/`_num_idx`/`_bool_idx`, or inline JSON under relaxed `ReadView`) joined against `_fathomdb_vector_rows` / `vector_default` (exact L2 rerank over matching `write_cursor`s — or `sqlite-vec` `rowid IN (...)` constraint if supported and verified via unit test — guaranteeing **100% recall** with zero top-192 starvation, while unpredicated and `<= 8`-slot `String`/`Bool` `Eq` queries remain 100% on the unchanged `vec0` bit-KNN fast path!).
   - **Missing-key & type-domain semantics (`SearchFilter.property_predicates`, consistent with Slice 30 inline evaluator):**
     - Validation (`validate_filter_attributes_on_snapshot`): every predicate key must be registered in `_fathomdb_projection_registry` with `entity_class == 'node'` and `Filterable`; otherwise return `InvalidFilter`.
     - Ordering (`Lt`, `Lte`, `Gt`, `Gte`) on `Bool` returns `InvalidFilter` cleanly.
     - Absent key evaluates `false` for all operators (`Eq`, `Ne`, `Lt`, `Lte`, `Gt`, `Gte`) — `EXISTS(SELECT 1 FROM canonical_property_values cpv WHERE cpv.entity_class='node' AND cpv.write_cursor = {alias}.write_cursor AND cpv.attr_name = ? AND ...)`.
     - Cross-type comparisons: `String` compares only against `value_type = 'string'`, `Bool` against `'boolean'`, and `Int`/`Float` against `value_type IN ('integer', 'float')` on `num_val NUMERIC` (preserving exact `i64` precision for `abs(x) > 2^53` while allowing exact `Int(5) == Float(5.0)` / `Int(5) < Float(5.5)` numeric comparisons).
     - Note `SearchFilter.attributes` remains supported identically for legacy JSON-body attribute equality.

### 3.4 Batch Commit Timestamps (`_fathomdb_commit_timestamps`, `P4-09`)

Add helper `record_batch_commit_timestamp(tx: &Connection, high_water_cursor: u64, now_epoch_ms: i64) -> rusqlite::Result<()>`:

```rust
let prev_ms: Option<i64> = tx
    .query_row(
        "SELECT committed_at_ms FROM _fathomdb_commit_timestamps ORDER BY write_cursor DESC LIMIT 1",
        [],
        |r| r.get(0),
    )
    .optional()?;
let committed_at_ms = prev_ms.map_or(now_epoch_ms, |prev| prev.max(now_epoch_ms));
tx.prepare_cached(
    "INSERT INTO _fathomdb_commit_timestamps(write_cursor, committed_at_ms) VALUES(?1, ?2)",
)?
.execute(rusqlite::params![high_water_cursor as i64, committed_at_ms])?;
```

- **Exact placement (strictly `next_cursor > base_cursor`, once per committed batch):**
  - In `write_commit.rs` (`commit_batch_transaction`, inside the same transaction as `apply_batch_in_transaction` before `advance_read_visibility` / `tx.commit()`, NOT inside `apply_batch_in_transaction` itself so `actuate_new`'s per-op loop does not insert intermediate rows): `if !batch.is_empty() { record_batch_commit_timestamp(&tx, base_cursor.saturating_add(batch.len() as u64), current_epoch_ms())?; }`.
  - In `actuation.rs` (`actuate_new`, after the operation loop succeeds and before `store_receipt` / `tx.commit()`): `if next_cursor > base_cursor { record_batch_commit_timestamp(&tx, next_cursor, current_epoch_ms())?; }`. Dependency-only (`RegisterSourceDependency`) and lifecycle-only (`TransitionLifecycle`) actuations leave `next_cursor == base_cursor` and emit zero rows, preventing `PRIMARY KEY(write_cursor)` collisions.
- `as_of_epoch_ms` execution consumers land in Slice 40, reading `SELECT MAX(write_cursor) FROM _fathomdb_commit_timestamps WHERE committed_at_ms <= ?1` (indexed on `_fathomdb_commit_timestamps_ms_idx`).

### 3.5 Explicit Opt-In App-Declared Vector-Committable Kinds (`P4-03`, `P5-01`)

1. **Preserve `tc67` by explicit opt-in (`no auto-enrolment`):**
   - Built-in static vocabulary (`resolve_builtin_source_type`) preserves `email|article|paper|meeting|note|todo` -> identity, `doc` -> `article`, `edge_fact` -> `edge_fact`.
   - Custom node kinds (`source_type = kind`) become vector-committable **only** when explicitly enrolled via `ProjectionVector.kinds` (`vector_kinds` / `vectorKinds` in Py/TS `ProjectionSpec`) during `configure_projections`:
     - Validate every requested kind: reject `""`, whitespace-only, `'\0'`, `'\x1e'`, and reserved `"edge_fact"` (preserving node-vs-edge `vector_default.source_type` invariant across `Engine` and `Inspector`) with `EngineError::WriteValidation`.
     - Reject non-empty `vector_kinds` when `Searchable` role or `vector` sub-object is absent (`R-20-SV`).
     - Persist canonical sorted unique enrolled kinds in `_fathomdb_projection_registry.vector_kinds` and enroll each into `_fathomdb_vector_kinds` inside the `configure_projections` transaction (alongside un-stranding eligible existing rows when `dense_arm_live`).
     - `read.projections` decodes `vector_kinds` back into `ProjectionVector.kinds` so `read.projections -> configure_projections` round-trips cleanly.
     - On `unenrol_registry_vector_node_kinds` (when dropping all `searchable->vector` specs), clear non-`edge_fact` `_fathomdb_vector_kinds` rows symmetrically.
   - Any corpus node kind that is neither built-in nor explicitly enrolled stays excluded from `_fathomdb_vector_kinds` and reports in `ProjectionDelta.vector_unsupported_kinds` (`tc67_unsupported_vector_kind_report` and `slice22_projection_status` remain 100% green).
2. **Replace `&'static str` `resolve_source_type` and every SQL 7-kind mirror:**
   - Replace `resolve_source_type(kind: &str) -> Result<&'static str, EngineError>` in `vector_storage.rs` with `resolve_source_type_on<'a>(conn: &Connection, kind: &'a str) -> Result<Cow<'a, str>, EngineError>` (checking `resolve_builtin_source_type(kind)` first, then `_fathomdb_vector_kinds WHERE kind = ?1 AND kind <> 'edge_fact'`), plus `kind_is_vector_committable_on(conn, kind)`. Retain `resolve_builtin_source_type` for pure helper tests.
   - Update ALL callers and SQL mirrors:
     - `filter.rs`: `append_node_eligibility_sql` (`source_type` `CASE ... ELSE CASE WHEN EXISTS(SELECT 1 FROM _fathomdb_vector_kinds vk WHERE vk.kind = {alias}.kind AND vk.kind <> 'edge_fact') THEN {alias}.kind ELSE NULL END END = ?`) and `text_hit_passes_filter` (`resolve_source_type_on(tx, kind)`).
     - `projection_generation.rs`: `unregistered_node_completion_sql` (`L1000-1045`) and `empty_physical_fast_path` (`L1163-1204`) replace `WITH committable AS (VALUES ...)` inner-join exclusion with `LEFT JOIN _fathomdb_vector_kinds vk ON vk.kind = n.kind AND vk.kind <> 'edge_fact'` **plus explicit `WHERE` filter `AND (n.kind IN ('email','article','doc','paper','meeting','note','todo') OR vk.kind IS NOT NULL)`** so only built-in 7 kinds and explicitly enrolled custom node kinds enter completion (never unenrolled kinds like `"invoice"`, preserving `tc67` cleanly!); also update `classify_completion` (`L817`) and `dense_member_kind_at` (`L942`) to call connection-aware `resolve_source_type_on` / `kind_is_vector_committable_on`.
     - `search.rs`: `historical_dense_incomplete` calls `kind_is_vector_committable_on(tx, &kind)`.
     - `index_projector.rs` (`restore_registered_derived_projections`) and `projection_commit.rs` (`commit_projection_outcomes`): call `kind_is_vector_committable_on` / `resolve_source_type_on`.
     - `data_plane_integrity.rs` (`OwnerRetention` classification and `dense_physical_shape`): call `kind_is_vector_committable_on` and `resolve_source_type_on`.
     - `write.rs` (`vector_kind_needs_enrolment`): pass `connection` to `kind_is_vector_committable_on(connection, kind)`.

### 3.6 Governed `read.operational_collections` & Kind Conflict Refusal (`P4-05`, `P3-03`)

1. **`OperationalCollectionInfo` & governed readback (`fathomdb-engine`, `fathomdb-sdk`, `fathomdb-py`, `fathomdb-napi`, Python, TS):**

   ```rust
   #[derive(Clone, Debug, Eq, PartialEq)]
   pub struct OperationalCollectionInfo {
       pub name: String,
       pub kind: String,
       pub schema_json: String,
       pub retention_json: String,
       pub format_version: i64,
       pub created_at: i64,
   }
   ```

   - `Engine::read_operational_collections(&self) -> Result<Vec<OperationalCollectionInfo>, EngineError>`: queries `operational_collections` ordered by `name ASC` via `ReaderWorkerPool` (no write lock), returning public metadata parity across Rust (`read::operational_collections(&engine)`), Python (`read.operational_collections(engine)`), and TS (`read.operationalCollections(engine)`).
   - `Engine::read_operational_collection(&self, name: &str) -> Result<Option<OperationalCollectionInfo>, EngineError>`: returns `Some(...)` if registered or `None` (`None` / `null`) when absent (`read::operational_collection(&engine, name)`, `read.operational_collection(engine, name)`, `read.operationalCollection(engine, name)`). Guard empty/control/NUL/lone-surrogate names with SDK `guard` / `validateFfiString`.
2. **`AdminSchema` conflicting-`kind` refusal (`P4-05`):**
   - Keep `EngineError::SchemaValidation` for malformed `AdminSchema` payloads (`write_validation.rs:186-196`); check existing `SELECT kind FROM operational_collections WHERE name = ?1` inside `write_validation.rs::validate_write` for `PreparedWrite::AdminSchema` and return `EngineError::WriteValidation` strictly on existing/same-batch `kind` conflict (`append_only_log <-> latest_state`), allowing idempotent `schema_json` / `retention_json` updates on matching `kind`.
   - Re-verify the existing `kind` inside `write_commit.rs::apply_batch_in_transaction` (`L860`) immediately before executing `INSERT INTO operational_collections ... ON CONFLICT(name) DO UPDATE` so same-batch or concurrent conflicting `AdminSchema` declarations atomically fail with `EngineError::WriteValidation`.
3. **Conformance & boundary pin lockstep (pre-authorized HITL Gate 1):**
   - Register `read.operational_collections` and `read.operational_collection` in `src/conformance/governed-operation-parity.json`, `src/conformance/governed-surface-allowlist.json`, and `scripts/governed-surface-pin.json` (bumping `53 -> 55` live canonical operations in `src/ts/tests/sdk-surface-parity.test.ts` and `fathomdb-sdk/tests/surface.rs`), preserving `core` (`5`) and `recovery_denylist` (`5`, `AC-041`).
   - Update `src/python/fathomdb/_fathomdb.pyi`, `src/python/tests/python_surface_additions.json`, and `src/python/tests/test_slice130_package_boundary.py` reverse-patch assertions cleanly in lockstep (`OperationalCollectionInfo`, `read_operational_collections`, `read_operational_collection`, `EraseReport.completed_pending_erasure`, property types/fields).

### 3.7 W12 Erasure Ergonomics Fixes (`w12-fence-review.md`, `ADR-0.8.28`)

1. **Autonomous two-truncate ordering (`w12-fence-review.md` §1, `erasure.rs` & `physical_maintenance.rs`):**
   - Update **both** `Engine::finish_physical_dependency_closures_inner` (`erasure.rs:628-720`) **and** the background worker `PhysicalMaintenance::start` (`physical_maintenance.rs:140-145`):
     - Under `PhysicalErasurePolicy::Autonomous` when `!debt.redacted`: run the **first** `wal_checkpoint(TRUNCATE)` (`checkpoint_erasure_at_rest` / `checkpoint(c)`) **before** `physical_erasure::prepare_redacted(connection, closure_ids)`. If that checkpoint fails BUSY, `mark_physical_incomplete` records `"wal_checkpoint"` while `debt.redacted` remains `false`, preserving `guard_plain_write_inner` + `certify_plain_write` for unrelated plain `Node`/`Edge` writes while keeping denied sources/identities, dependency ops, actuations, and admin ops fenced. Once the first truncate succeeds, run `prepare_redacted`, then run the **final** `wal_checkpoint(TRUNCATE)` and `physical_erasure::finalize` so zero WAL debt metadata remains at rest.
     - Under `PhysicalErasurePolicy::ExplicitMaintenance`: retain the existing `prepare_redacted` -> `complete_erasure_at_rest` -> `finalize` sequence unchanged.
   - Audit existing `ActualCheckpointObserver` assertions (`fathomdb-engine/src/lib.rs`, `test_slice65_wal_attribution_installed.py`) so Autonomous tests account for the bounded two-truncate sequence without weakening checkpoint invariants.
2. **Completing-retry `ExciseReport.completed_pending_erasure` & cross-source audit guard (`w12-fence-review.md` §2):**
   - Add `pub completed_pending_erasure: bool` to `ExciseReport` (`erasure.rs`, re-exported via `fathomdb` and `fathomdb-sdk`), `completed_pending_erasure: bool` to `PyEraseReport` / `_fathomdb.pyi` / `python_surface_additions.json`, and `completedPendingErasure: boolean` to TS `EraseReport` (`fathomdb-napi/src/projection.rs`, `src/ts/src/write.ts`, `src/ts/src/binding.ts`).
   - Change `finish_redacted_before_request(&self)` to return `Result<bool, EngineError>` (`true` iff a prior redacted closure was completed during this call).
   - Track `let completed_prior = self.finish_redacted_before_request()?;` inside `erase_source_shared` (`erasure.rs`):
     - On `if root_retry || !pending.is_empty()` retry path: return `completed_pending_erasure: true`.
     - If `finish_redacted_before_request` finished a prior closure (`completed_prior == true`) and `excise_source_inner` excises `0` nodes and `0` edges for the requested `source_id`, return `completed_pending_erasure: true` and **suppress** writing a duplicate 0-count `excise_source_audit` row (by gating `excise_source_audit` insertion when `nodes_excised == 0 && edges_excised == 0 && completed_prior`, or running a lightweight read check before `excise_source_audit`!).
     - Whenever `nodes_excised > 0 || edges_excised > 0` for the current `source_id`, **always** write the current call's `excise_source_audit` row (never dropping real cross-source audit evidence even if `completed_prior == true`!), and propagate `completed_pending_erasure: completed_prior` (`false` on fresh normal erasures and true already-erased no-ops).
     - Update `fathomdb-cli/src/lib.rs::excise_report_json` (`L613`) and audit `fathomdb-cli/tests/` for any `excise-source` JSON snapshot assertions.
3. **Release-mode lock-free BUSY attribution (`w12-fence-review.md` §3, `wal_attribution.rs`, `reader_transaction.rs`, `projection_commit.rs`, `projection_worker.rs`, `erasure.rs`):**
   - Remove outer `.enabled` gates that currently skip attribution entirely in release builds: `reader_transaction.rs:14`, `projection_commit.rs:235`, `projection_worker.rs:1176`, and `wal_attribution.rs:450,457` (`checkpoint_begin`/`checkpoint_end`).
   - In `WalAttributionCollector`: maintain always-on lock-free per-slot `AtomicBool` active flags alongside per-`WalAttributionRole` (`Writer`, `ReaderWorker`, `ProjectionDispatcher`, `ProjectionWorker` — exact existing variants in `wal_attribution.rs:4-9`) `AtomicU64` active counters plus `AtomicBool` checkpoint overlap. Increment/decrement role counters **strictly on idempotent `false <-> true` state flips** (`swap(true)`/`swap(false)`) so repeated `set(..., true, ...)` in `begin_attributed_reader_tx` (`reader_transaction.rs:15,17`) and repeated `set(..., false, ...)` in `WalAttributionActivity::drop` (`wal_attribution.rs:291-292`) + `finish_reader_request` (`reader_pool.rs:1100-1101`) can NEVER double-count or underflow! Keep zero `Mutex` acquisition and zero `eprintln!` when `!self.enabled`, retaining full `Vec<WalCheckpointRecord>` + stderr trace gated on `self.enabled`.
   - Update `Engine::checkpoint_erasure_at_rest` (`erasure.rs`) to format `ErasureIncomplete { stage: "wal_checkpoint", detail }` from those always-on atomic counters so release builds truthfully report `"local_checkpoint_overlap"`, `"owned_reader_snapshot"` (`ReaderWorker`), `"owned_runtime_transaction"` (`ProjectionDispatcher | ProjectionWorker`), or `"unclassified_external"` alongside active role counts, distinguish initial erasure vs completing retry phrasing, and avoid falsely blaming a concurrent caller reader when the blocker is internal or external — while preserving substrings `"wal_checkpoint(TRUNCATE)"`, `"BUSY"`, and `"frames still in the log"` required by existing tests (`_expected_wal_baseline` in `test_slice65_wal_attribution_installed.py`).

### 3.8 `sqlite_master` + `ExportClass` Completeness Audit & `physical_erasure` Byte Accounting (`P4-12`)

1. **Formalize `ExportClass` manifest, `guard_row_owned_registry`, & automated `sqlite_master` audit (`fathomdb-engine`):**
   - Add `"_fathomdb_commit_timestamps"` to `NON_PROJECTION_CURSOR_TABLES` in **both** `guard_row_owned_registry` (`fathomdb-engine/src/lib.rs:3340`) and `slice15d_attribute_projections_registered_and_guard_bites` (`lib.rs:3441`), since `_fathomdb_commit_timestamps` carries a `write_cursor` PK as `DurableAuthority` (not a `ROW_OWNED_PROJECTIONS` derived table).
   - Define `pub(crate) enum ExportClass { DurableAuthority, OptInVector, OptInAudit, DerivedProjection, InternalRuntime }` covering every schema-38 non-`sqlite_%` table in `sqlite_master`:
     - **`DurableAuthority`**: `canonical_nodes`, `canonical_edges`, `operational_collections`, `operational_mutations`, `operational_state`, `_fathomdb_artifact_revisions`, `_fathomdb_source_versions`, `_fathomdb_source_links`, `_fathomdb_source_dependencies`, `_fathomdb_projection_registry`, `_fathomdb_vector_kinds` (survivability of custom kind declarations on vector-free export), `_fathomdb_commit_timestamps`.
     - **`OptInAudit`**: `_fathomdb_actuation_receipts`, `_fathomdb_actuation_receipt_source_refs`, `_fathomdb_dependency_closures`.
     - **`OptInVector`**: `vector_default`, `_fathomdb_vector_rows`, `_fathomdb_vector_spans`, `_fathomdb_embedder_profiles`, `_fathomdb_embed_probe` (never exporting centered binary/mean internals).
     - **`DerivedProjection`**: `canonical_attributes`, `canonical_property_values`, `property_search_index`, `search_index`, `search_index_v2`, `search_index_edges`, plus FTS5 shadow tables (`*_config`, `*_content`, `*_data`, `*_docsize`, `*_idx` for `search_index*` and `property_search_index_*`).
     - **`InternalRuntime`**: `_fathomdb_migrations`, `_fathomdb_schema_meta`, `_fathomdb_open_state`, `_fathomdb_read_visibility_state`, `_fathomdb_projection_state`, `_fathomdb_projection_terminal`, `_fathomdb_projection_generations`, `_fathomdb_projection_generation_current`, `_fathomdb_erasure_requests`, plus dynamic `sqlite-vec` shadow tables matched by `vector_default_*` prefix (`chunks`, `info`, `rowids`, `vector_chunksNN`, `metadatachunksNN`, `metadatatextNN`).
   - Add an automated completeness test asserting:
     1. Every non-`sqlite_%` table in a bootstrapped schema-38 database (with vector partition + declared `Filterable` attributes materialized so `metadatachunks*`/`metadatatext*` dynamic tables exist) matches exactly one manifest entry or dynamic matcher (`ExportClass`, canonical vs row-owned vs internal, visibility-trigger status, erasure reachability, `doctor` check status, and Inspector classification).
     2. Negative test: injecting an unclassified dummy table into a test DB causes the completeness audit predicate to fail loudly.
2. **Physical erasure admission & data-plane integrity (`physical_erasure/admission.rs`, `data_plane_integrity.rs`):**
   - Update `physical_erasure::admission::envelope` query on `canonical_nodes` and `canonical_edges` to account for both nullable `body` and nullable `properties`:

     ```sql
     SELECT
       COUNT(*),
       COALESCE(SUM(COALESCE(length(CAST(body AS BLOB)),0) + COALESCE(length(CAST(properties AS BLOB)),0)),0),
       COALESCE(MAX(COALESCE(length(CAST(body AS BLOB)),0) + COALESCE(length(CAST(properties AS BLOB)),0)),0)
     FROM (
       SELECT body, properties FROM canonical_nodes
       UNION ALL
       SELECT body, properties FROM canonical_edges
     )
     ```

   - In `physical_erasure/admission.rs:32-42` (`envelope_metadata`), leave `_fathomdb_commit_timestamps` excluded (contains only integer cursor + monotonic timestamp, zero user content/strings; `operational_collections` likewise holds only schema metadata, not row payload).
   - Extend `data_plane_integrity.rs`:
     - Update `ATTRIBUTE_OWNER_QUERY` (`L59-64`) and `expected_attribute_members` (`L226-281`) to read `(body, properties)` with `properties` precedence.
     - Add `PROPERTY_VALUE_MEMBER_QUERY`, extending `candidate_queries_for_test` cleanly while keeping `slice55_data_plane_integrity.rs` in lockstep, with entity-class-aware owner verification (`entity_class = 'node'` against `canonical_nodes`, `entity_class = 'edge'` against `canonical_edges` — never unconditionally checking `canonical_nodes` for edge rows in residue checks at `L1881-1903`!).
     - Update `dense_physical_shape` (`L1729`) to call connection-aware `resolve_source_type_on`.

---

## 4. Prerequisites Discovered (Flagged for Reviewer)

Audit of `/home/coreyt/projects/fathomdb-worktrees/release-0.8.31` surfaced these concrete prerequisites and cross-cutting blast-radius items:

1. **Slice 10 (`PREWORK-PREP`) completion prerequisite:** Slice 20 assumes Slice 10 (`WP-10A` ADR amendments + `WP-10C` `scripts/check-public-doc-truth.py` target flip) has landed. Once Slice 20 bumps `SCHEMA_VERSION` to `38`, `docs/compatibility/index.md` and `dev/plans/release-state-0.8.31.json` (`"schema_version": 38`) must move from `37` to `38` in the same commit so `check-public-doc-truth.py` remains green.
2. **`f64` inside `PropertyValue` breaks existing `#[derive(Eq)]` bounds:** Adding `PropertyMap` (`BTreeMap<String, PropertyValue>`) and `PropertyPredicate` breaks `#[derive(..., Eq)]` on `NodeRecord`, `SearchFilter`, `ReadContextV1`, `FrozenReadContextV1`, `GraphOriginV1`, `GraphTargetV1`, `BoundaryCrossing`, `EvidenceResolveRequestV1`, `GraphEvidenceResolveRequestV1`, `ResolvedGraphEvidenceV1`, `fathomdb_sdk::TextSearchOptions`, and `fathomdb_sdk::ProjectedTextSearchOptions` (`SearchHit` and `ProvenancedEdgeV1` already derive `PartialEq` without `Eq`). Keep `PartialEq` (`assert_eq!` requires only `PartialEq`) and verify zero callers rely on `Eq`.
3. **Workspace-wide struct-literal blast radius (`PreparedWrite`, `Provenanced*`, `ProjectionSpec`, `ExciseReport`):**
   - `PreparedWrite::{Node,Edge}` and `Provenanced{Node,Edge}V1` are constructed exhaustively across `~291` sites workspace-wide (across all 12 crates under `src/rust/crates/`, including `fathomdb-mcp`, plus `36` literal constructions across `src/inspector/tests/{fixtures/mod.rs,integration/graph.rs,integration/lexical.rs,integration/vector.rs,integration/eligibility.rs,integration/records.rs}`). Per `P4-13`, Slice 20 extracts repeated Inspector literals in `src/inspector/tests/{fixtures/mod.rs,integration/graph.rs}` into local helper constructors so Slices 30–60 remain insulated.
   - `ProjectionSpec` has ~58 exhaustive literal constructions across `fathomdb-engine`, `fathomdb-sdk`, `fathomdb-mcp`, and `src/inspector/tests/`; every literal must initialize `entity_class: PropertyEntityClass::Node` (or `..Default::default()` if `Default` is derived).
   - `ExciseReport` gains `completed_pending_erasure: bool`, affecting literal assertions, `fathomdb-cli/src/lib.rs::excise_report_json` (`L613`), and any `fathomdb-cli/tests/` `excise-source` JSON snapshot assertions.
   - `Engine::data_plane_integrity_candidate_queries_for_test` changes array length from `13` when adding `canonical_property_values` checks; update `src/rust/crates/fathomdb-engine/tests/slice55_data_plane_integrity.rs` in lockstep.
4. **Hardcoded schema-`37` and `57`-trigger tests that must advance with Step 38 (`prework/slice-5.md`):**
   - `src/rust/crates/fathomdb-schema/tests/slice45_step33.rs` (`assert_eq!(SCHEMA_VERSION, 37)`)
   - `src/rust/crates/fathomdb-engine/src/lib.rs` (unit test checking `schema_version_after == 37` and `step_id == 37`)
   - `src/rust/crates/fathomdb-engine/tests/slice40_fresh_database_cutover.rs` (`37 -> 38`, shifting prior rejected version to `37` and future probe to `39`)
   - `src/rust/crates/fathomdb-engine/tests/slice60_store_format.rs`
   - `src/rust/crates/fathomdb-engine/tests/truncate_wal.rs`
   - `src/rust/crates/fathomdb-engine/tests/slice95_visibility_coalescing.rs` (`37 -> 38`, `57 -> 60` triggers)
   - `src/rust/crates/fathomdb-engine/tests/slice85_policy.rs` (`PRAGMA user_version`)
   - `src/rust/crates/fathomdb-engine/tests/slice60_graph_expand.rs` (`assert_eq!(SCHEMA_VERSION, 37)`)
   - `src/rust/crates/fathomdb-engine/src/stored_coverage.rs` (`user_version != 37`) and `stored_coverage_tests.rs`
   - `src/rust/crates/fathomdb-cli/tests/slice55_composed_recovery.rs` (`schema["user_version"] == 37`)
   - *(Note: `src/inspector/tools/local-release.py` and `test_local_release.py` `'compiled_schema': 37` and Inspector UI/admission fixtures stay tracked for Slice 60, while `src/inspector/build.rs` already reads `SCHEMA_VERSION` dynamically from `fathomdb-schema/src/lib.rs`.)*
5. **Hardcoded 7-kind vector allowlist mirrors across multiple Rust modules (`P4-03`):** `vector_storage.rs`, `filter.rs`, `projection_generation.rs`, `search.rs`, `index_projector.rs`, `projection_commit.rs`, and `data_plane_integrity.rs` must all move to connection-aware `resolve_source_type_on` / `kind_is_vector_committable_on` / `_fathomdb_vector_kinds` SQL before custom vector kinds work end-to-end.
6. **`WalAttributionCollector::enabled` and `ActualCheckpointObserver` (`w12-fence-review.md`):** Release builds currently skip `register`/`set`/`checkpoint_begin` (`enabled: cfg!(test) || env(...)`), and Autonomous tests observing checkpoint attempt counts (`ActualCheckpointObserver` in `fathomdb-engine/src/lib.rs`, `erasure_completeness.rs`, `slice82_*`, `slice83_*`, `test_slice65_wal_attribution_installed.py`) must be audited against the Autonomous two-truncate sequence.
7. **Python boundary SHA-256 reverse-patch ledger & conformance pins:** `src/python/tests/test_slice130_package_boundary.py` computes an exact SHA-256 checksum over `_fathomdb.pyi` after applying deterministic reverse edits (`python_surface_additions.json`), and `src/ts/tests/sdk-surface-parity.test.ts` asserts exact count `53/53 live canonical operations`. Both require lockstep updates alongside `governed-operation-parity.json`, `governed-surface-allowlist.json`, and `scripts/governed-surface-pin.json` (authorized at HITL Gate 1).
8. **Strict endpoint integrity deferred to Slice 30 (`ADR-0.8.31-strict-endpoint-integrity.md`):** Existing dangling-edge tests (`pr_g8_dangling_edges.rs`, etc.) stay green in Slice 20 (`WriteReceipt.dangling_edge_endpoints` unchanged here) and convert in Slice 30.

---

## 5. TDD Implementation & Verification Plan

### 5.1 RED Phase (Failing Tests Written / Updated First)

Write failing tests (compile/assert red before product implementation) in the following exact files:

1. **Schema 38 migration & triggers (`src/rust/crates/fathomdb-schema/tests/step38_property_schema.rs`, plus `slice45_step33.rs` / `slice95_visibility_coalescing.rs` / `slice40_fresh_database_cutover.rs` updates):**
   - Assert `SCHEMA_VERSION == 38`, step-38 columns (`properties`, inert `scope = ''` across all 4 tables, `_fathomdb_projection_registry` `entity_class` & `vector_kinds`), `canonical_property_values` + 4 indexes (`_cursor_idx`, `_text_idx`, `_num_idx`, `_bool_idx`), `_fathomdb_commit_timestamps` + `_ms_idx`, `canonical_nodes_logical_sup_idx`, `canonical_edges_{from,to}_active_idx`, `canonical_edges_{from,to}_sup_kind_idx`, and `60` visibility triggers (`_fathomdb_read_visibility_cpv_{ai,au,ad}` incrementing generation + rotating `state_nonce` on INSERT/UPDATE/DELETE).
   - Verify fresh open vs migrated step-37 open refusal (`IncompatibleSchemaVersion { seen: 37, supported: 38 }`) and verify `docs/compatibility/index.md` declares schema `38`.
2. **Inline properties, typed B-tree indexes, `vec0` budget, and commit timestamps (`src/rust/crates/fathomdb-engine/tests/slice38_properties_and_indexes.rs`, `src/rust/crates/fathomdb-sdk/tests/slice38_properties.rs`):**
   - `slice38_node_and_edge_property_round_trip_and_o1_hydration`: round-trip `String`, `Int` (`i64::MIN`, `i64::MAX`, `-(1 << 53) - 1`, `(1 << 53) + 1`), `Float` (finite + `-0.0` normalization), and `Bool` on `PreparedWrite::{Node,Edge}`, `Provenanced{Node,Edge}V1`, and `actuate`; verify O(1) hydration on `NodeRecord`, `SearchHit`, `GraphTargetV1`, `GraphOriginV1`, `ResolvedEvidenceV1`, `ResolvedGraphEvidenceV1`, and verify `body`-FTS & embeddings never contain property keys/values; verify `Some(empty_map)` persists SQL `NULL`.
   - `slice38_property_validation_rejects_malformed_atomically`: reject `NaN`/`±Inf`, empty keys, `\0`/`\x1e` keys/strings with zero-row atomic rollback; reject different `properties` reusing same `actuate` `operation_id` (`ActuationErrorReason::OperationIdConflict`).
   - `slice38_property_codec_proptest`: `proptest` fuzzing `BTreeMap<String, PropertyValue>` canonical encode/decode round-trip, exact `i64` vs `f64` ordering in `canonical_property_values_num_idx` (`2^53 + 1 != 2^53`), and cross-type `Int` vs `Float` comparisons.
   - `slice38_declared_filterable_indexes_vec0_budget_and_lifecycle`: test `entity_class = Edge` `Filterable` registration (and refusal of `Searchable`/`Rankable` on edge), `properties` precedence over `json_extract(body, ...)`, immutable revisions in `_fathomdb_artifact_revisions`, cleanup on node supersession + edge G0 (`logical_id`) & G11 (`(from_id, to_id, kind)`) supersession, purge on `Deleted`/`Pending` transition + re-projection from `properties` on return to `Active`, typed `InvalidFilter` on undeclared keys / edge keys / `Bool` range ops, **9+ declared node `Filterable` properties** respecting the 8-slot `vec0` cap (reserving `1` slot for `scope` without `sqlite-vec` column-limit error), `String` vs `Int`/`Bool` non-collision on `vec0`, and `Ne`/range pre-filtering via `canonical_property_values` + relaxed-`ReadView` filter parity.
   - `slice38_commit_timestamps_monotonic_and_actuate_cursor_guard`: test single row per committed batch keyed by `WriteReceipt.cursor` in `write` and cursor-allocating `actuate`, monotonic clamping across simulated clock regressions, and zero inserts / zero PK collisions on dependency-only and lifecycle-only `actuate`.
3. **Custom app-declared vector kinds (`src/rust/crates/fathomdb-engine/tests/slice38_vector_kinds.rs` + existing `tc67_unsupported_vector_kind_report` & `slice22_projection_status`):**
   - `slice38_explicit_vector_kind_enrolment_indexes_searches_and_reopens`: enroll `"custom:memory"`, ingest nodes, `drain`, hybrid/vector search (including `SearchFilter.source_type`), close & reopen, verify persistence + readiness.
   - `slice38_vector_kind_rejects_edge_fact_and_preserves_tc67`: verify enrolling `"edge_fact"`, `""`, whitespace-only, `\0`, `\x1e`, or `vector_kinds` without `Searchable`+`vector` fails with `WriteValidation`; verify unenrolled `"entity"` / `"invoice"` stay excluded and reported in `vector_unsupported_kinds`.
4. **Operational collections readback & mismatch refusal (`src/rust/crates/fathomdb-engine/tests/slice38_op_collections.rs`):**
   - `slice38_read_operational_collections_sorted_and_kind_conflict_refused`: verify sorted `read_operational_collections`, `read_operational_collection` `Some`/`None`, idempotent schema/retention update on matching `kind`, and `EngineError::WriteValidation` zero-mutation refusal on `append_only_log <-> latest_state` conflict.
5. **Erasure, `sqlite_master` + `ExportClass` audit, and W12 fixes (`src/rust/crates/fathomdb-engine/tests/slice38_erasure_and_audit.rs`):**
   - `slice38_sqlite_master_and_export_class_completeness_audit` (plus `slice38_sqlite_master_audit_fails_on_unclassified_table` negative test and `guard_row_owned_registry` verification with materialized dynamic `vector_default_*` / FTS5 shadows).
   - `slice38_properties_physical_erasure_autonomous_and_explicit`: erase nodes + nullable-`body` edges + custom-kind vectors (`purge` + `erase_source` in both modes); assert zero `canonical_property_values` rows, zero raw substrings of property keys/values across `main`, `-wal`, `-shm`, telemetry sinks, and verify `physical_erasure::admission::envelope` charges `properties` bytes on nullable-`body` edges.
   - `slice38_w12_3a_autonomous_checkpoint_busy_keeps_plain_writes_open`: hold WAL snapshot BUSY, assert `debt.redacted == false`, unrelated plain writes pass + advance `safe_boundary`, denied sources/identities & dependency/actuation writes fail closed, retry finishes cleanly with zero debt residue; verify ExplicitMaintenance order unchanged.
   - `slice38_w12_3b_completed_pending_erasure_and_cross_source_audit_guard`: assert `completed_pending_erasure == true` on `finish_redacted_before_request` and `root_retry`/`pending`, `false` on fresh/no-op, zero duplicate 0-row `excise_source_audit` rows, and **retained `excise_source_audit` row** when finishing prior Source A's closure while excising real rows in Source B.
   - `slice38_w12_3c_release_mode_wal_attribution_without_env`: without `FATHOMDB_WAL_ATTRIBUTION`, prove `owned_reader_snapshot` vs `unclassified_external` external connection produces accurate classification + counts and never falsely blames a caller reader on external blockers while keeping required checkpoint/frame substrings intact.
6. **`EXPLAIN QUERY PLAN` pins (`src/rust/crates/fathomdb-engine/tests/slice38_query_plans.rs`):**
   - Assert indexed `SEARCH` (`canonical_property_values_{text,num,bool}_idx`, `canonical_nodes_write_cursor_idx`, `_fathomdb_commit_timestamps_ms_idx`, `canonical_nodes_logical_sup_idx`, `canonical_edges_{from,to}_active_idx`, `canonical_edges_{from,to}_sup_kind_idx` for empty & non-empty `edge_kinds`), and assert unpredicated `search` / `search_text_only` / current `graph.expand` contain zero joins on `canonical_property_values` or `superseded_at` historical scans.
7. **Cross-SDK & conformance parity (`src/python/tests/test_slice38_properties.py` with `hypothesis`, `src/ts/tests/slice38-properties.test.ts`, `test_erase_source.py`, `erase-source.test.ts`, `fathomdb-sdk/tests/surface.rs`, `test_slice130_package_boundary.py`, `sdk-surface-parity.test.ts`):**
   - Verify Python (`hypothesis`) + TypeScript round-trips (including extreme `i64` / `bigint`, `NaN`/`\x1e`/null rejections), `SearchFilter.property_predicates`, custom vector kinds + `tc67` preservation, `read.operational_collections` / `read.operational_collection`, `AdminSchema` kind conflict `WriteValidationError`, `EraseReport.completed_pending_erasure` / `completedPendingErasure`, and live parity `55/55`.

### 5.2 GREEN Phase (Ordered Implementation)

Execute in clean dependency layers so each layer compiles and turns its targeted RED suite GREEN:

1. **Layer 1 — Schema & Coalescing (`fathomdb-schema`, `frozen_read`, `write_commit`, `projection_generation`, `stored_coverage`):**
   - Step 38 DDL (`properties`, `scope`, `entity_class`, `vector_kinds`, `canonical_property_values` + 4 indexes, `_fathomdb_commit_timestamps` + `_ms_idx`, `canonical_nodes_logical_sup_idx`, 4 edge incident indexes, `visibility_coalescing_trigger_sql!` for `cpv`), `SCHEMA_VERSION = 38`, `VISIBILITY_TRIGGER_TABLES` `19 -> 20`, `canonical_batch_has_no_custom_triggers`, `bootstrap_is_fresh`, `stored_coverage`, `projection_registry_digest`, `docs/compatibility/index.md` schema `38`.
2. **Layer 2 — Core Types & Validation (`fathomdb-engine`):**
   - `PropertyValue`, `PropertyMap`, `PropertyEntityClass`, `PropertyComparisonOp`, `PropertyPredicate`, canonical JSON validator/codec, update `Eq -> PartialEq` on carriers, extend `PreparedWrite`, `ProvenancedNodeV1`, `ProvenancedEdgeV1`, `NodeRecord`, `SearchHit`, `GraphTargetV1`, `GraphOriginV1`, `ResolvedEvidenceV1`, `ResolvedGraphEvidenceV1`, `SearchFilter`, `FilterTerm`, `ProjectionSpec`, `ProjectionVector`, `OperationalCollectionInfo`, `ExciseReport`.
   - Extract Inspector struct literals in `src/inspector/tests/{fixtures/mod.rs,integration/*.rs}` into helper constructors and update workspace struct literals so workspace compiles cleanly on Schema `38`.
3. **Layer 3 — Write, Actuate, Lifecycle, Index Projector & Vector Storage (`fathomdb-engine`):**
   - Persist `properties` on `canonical_nodes` / `canonical_edges`; encode `properties` in `actuation.rs::request_digest`; implement `record_batch_commit_timestamp` in `commit_batch_transaction` and `actuate_new` (strictly `next_cursor > base_cursor`).
   - Register `canonical_property_values` in `ROW_OWNED_PROJECTIONS`; implement `entity_class` + `vector_kinds` registry validation/persistence in `projection_registry.rs`; project typed values from `(body, properties)` with `properties` > `body` precedence; purge `ProjectionClass::PropertyValue` on node & edge G0/G11 supersession and lifecycle deactivation; re-project `(body, properties)` on reactivation (`record_lifecycle.rs`, `actuation.rs`, `projection_commit.rs`, `vector_storage.rs::reshape_vector_partition_nondestructive`).
   - Enforce `MAX_VEC0_NODE_PROPERTY_SLOTS = 8` (`entity_class = 'node'`, reserving `1` slot for `scope`), update comments, implement explicit opt-in `_fathomdb_vector_kinds` enrollment (rejecting `"edge_fact"`), replace `&'static str` `resolve_source_type`, and update all SQL 7-kind mirrors (`filter.rs`, `projection_generation.rs`, `search.rs`, `index_projector.rs`, `projection_commit.rs`, `data_plane_integrity.rs`).
   - Implement `SearchFilter.property_predicates` / `FilterTerm::Property` validation, `vec0` node-`Eq` + `canonical_property_values` lowering, `frozen_read::encode_context` parity, O(1) read hydration (`read.rs`, `search.rs`, `rerank.rs`, `fusion.rs`, `graph_expand/`, `evidence.rs`), and relaxed-`ReadView` filter parity.
4. **Layer 4 — Op-Collections, W12 Erasure Fixes, Completeness Audit & `physical_erasure` (`fathomdb-engine`, `fathomdb-cli`):**
   - Implement `read_operational_collections` / `read_operational_collection` + `AdminSchema` kind-conflict refusal (`write_validation.rs` + `write_commit.rs`).
   - Implement W12 fixes (a) Autonomous first-truncate before `prepare_redacted`, (b) `completed_pending_erasure` + cross-source audit guard, (c) always-on atomic counters in `WalAttributionCollector` + truthful `checkpoint_erasure_at_rest` diagnostics.
   - Update `physical_erasure::admission::envelope` for `properties`, extend `data_plane_integrity.rs`, add `ExportClass` manifest + `sqlite_master` audit, update `fathomdb-cli/src/lib.rs::excise_report_json` & schema-37 test expectations (`slice55_composed_recovery.rs`, etc.).
5. **Layer 5 — SDK, Bindings, Conformance & Docs Parity (`fathomdb`, `fathomdb-sdk`, `fathomdb-py`, `fathomdb-napi`, `src/python`, `src/ts`, `src/conformance`, `dev/interfaces`):**
   - Expose new types/fields, `read.operational_collections`, `read.operational_collection`, `ProjectionSpec.entity_class`/`vector_kinds`, `EraseReport.completed_pending_erasure`/`completedPendingErasure` across Rust SDK, PyO3, N-API, Python, TypeScript, and MCP/examples where structs require fields.
   - Update `_fathomdb.pyi`, `python_surface_additions.json`, `test_slice130_package_boundary.py`, `governed-operation-parity.json`, `governed-surface-allowlist.json`, `scripts/governed-surface-pin.json` (HITL Gate 1), `dev/interfaces/{rust,rust-sdk,python,typescript,wire}.md` (strictly product-generic).

### 5.3 Proportional Verification Commands

Execute inside `/home/coreyt/projects/fathomdb-worktrees/release-0.8.31` (using its checkout-owned `.venv` and `node_modules`):

1. **Rust lint & full workspace compile/test (including Inspector & all targets):**

   ```bash
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test -p fathomdb-schema
   cargo test -p fathomdb-engine
   cargo test -p fathomdb-sdk
   cargo test -p fathomdb-cli
   cargo test -p fathomdb-inspector
   cargo test --workspace
   ```

2. **Python native wheel rebuild, typecheck, and suite:**

   ```bash
   bash scripts/setup-python-owned.sh --provision
   .venv/bin/ruff check src/python scripts
   .venv/bin/pyright
   .venv/bin/pytest src/python/tests/
   ```

   *(Verify `pyright` reports `0 errors, 0 warnings`.)*

3. **TypeScript native rebuild, typecheck, build, and suite:**

   ```bash
   (cd src/ts && npm run build && npm test)
   ```

4. **Docs, schema, migration, surface-pin & parity gates:**

   ```bash
   python3 scripts/check-public-doc-truth.py
   ./scripts/agent-lint-migrations.sh
   ./scripts/agent-lint-md.sh
   ./scripts/agent-lint-docs.sh
   ./scripts/check-governed-surface-pin.sh
   python3 scripts/check-sdk-surface-parity.py --signed src/conformance/governed-surface-allowlist.json --companion src/conformance/governed-operation-parity.json --help >/dev/null
   ```

   *(Also run `./scripts/check-release-state-views.sh` and grep all modified files to confirm zero external consumer-project specifics.)*

### 5.4 Dependencies, Parallelism & Risks

- **Dependencies & Serialize Rule:** Slice 20 (`PROPERTY-SCHEMA`) is a **single-checkout / single-writer keystone** touching schema (`fathomdb-schema`), core engine structs (`PreparedWrite`, `SearchFilter`, `NodeRecord`, `SearchHit`, `ExciseReport`), Inspector fixtures, and all three SDKs. Upstream Slice 10 must be landed cleanly first, and downstream Slices 30–80 block cleanly on Slice 20 schema/type freeze.
- **Internal Implementation Parallelism:** Once Layer 1 (`fathomdb-schema` Step 38) and Layer 2 (core Rust types + Inspector helper wrappers) land on the branch, W12 erasure ergonomics + `sqlite_master` audit (Layer 4) and Python/TypeScript binding scaffolding (Layer 5) can fan out in separate worktrees or clean sequential steps, with a single writer merging onto `release/0.8.31` and re-running the lockstep conformance pins.
- **Key Technical Risks & Mitigations:**
  1. **`vec0` 16-column limit & `NULL` rejection:** Fixed by `MAX_VEC0_NODE_PROPERTY_SLOTS = 8` (1 reserved for `0.8.32` `scope`), excluding `entity_class = 'edge'`, and routing `Ne`, numeric ranges, and slot overflow via `canonical_property_values`.
  2. **Type confusion between `String` and `Int`/`Bool` or `abs(x) > 2^53` precision loss:** Fixed by `NUMERIC` affinity + explicit `value_type` column, `\u{1}` vs `\u{2}` `vec0` prefixes, TS `bigint` lossless codec, and exact `i64` property tests.
  3. **Custom vector kinds breaking `tc67`:** Fixed by strict explicit opt-in via `ProjectionVector.kinds` (`_fathomdb_vector_kinds`) with `"edge_fact"` rejected and unenrolled kinds untouched.
  4. **Silent omission of `prepare_redacted` or cross-source audit suppression in W12:** Mitigated by Autonomous-only first-truncate ordering and gating audit suppression strictly on `completed_pending_erasure && nodes_excised == 0 && edges_excised == 0`.
  5. **Python/TS parity & stub SHA-256 breakage:** Mitigated by lockstep updates to `_fathomdb.pyi`, `python_surface_additions.json`, `test_slice130_package_boundary.py`, `governed-operation-parity.json`, `governed-surface-allowlist.json`, `scripts/governed-surface-pin.json`, and `sdk-surface-parity.test.ts` before slice verification.
