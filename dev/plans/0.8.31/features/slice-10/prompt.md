---
title: Slice 10 — Schema 38+, outside-body inline properties + declared typed indexes, app-declared vector kinds, op-collection readback, W12 fixes and sqlite_master audit
status: ACTIVE
target_release: 0.8.31
slice: 10
date: 2026-11-22
---

# Slice 10 — Schema 38+ Property Graph Core, Vector Kinds, Op-Collection Readback, W12 Fixes & Completeness Audit · `[implementation]` · depends-on: `0 (CLOSED on release/0.8.31)`

> **You are the Slice 10 implementer agent.** Instantiated from
> `dev/plans/prompts/0.8.0-SLICE-TEMPLATE.md`. This prompt is self-contained.
> Work ONLY inside your assigned worktree `$WT`. Never touch `/home/coreyt/projects/fathomdb`
> or `/home/coreyt/projects/fathomdb-worktrees/release-0.8.31`. Never use
> `--no-verify` or push to `origin`.

## 0. Worktree setup

- Your assigned worktree `$WT` is pre-created off `release/0.8.31` on branch `impl/0.8.31-slice-10`.
- Verify `git -C "$WT" rev-parse --show-toplevel` equals `$WT` before any edit.
- Stay strictly inside `$WT`.

## 1. Required reading (before any edit)

1. `AGENTS.md` — TDD mandatory, real databases only (no mocks), never `--no-verify`, never hardcode `c_char`.
2. `dev/adr/ADR-0.8.31-property-graph-inline-and-declared-indexes.md` — authoritative contract for schema 38, Option 1 inline+declared properties, reserved `scope`, app-declared vector kinds, op-collection readback, and `sqlite_master` + `ExportClass` audit.
3. `dev/plans/0.8.31/features/slice-0/w12-fence-review.md` — authoritative contract for W12 erasure ergonomics fixes (Ask 3a/3b/3c).
4. `dev/plans/0.8.31/plan.md` (`R31-01`, `R31-06`, X1–X4 DoD).

## 2. Mandate (what this slice delivers — and does not)

Deliver:

1. **Schema 38 (`src/rust/crates/fathomdb-schema/src/lib.rs`)**:
   - Bump `SCHEMA_VERSION` from `37` to `38`.
   - Add `Migration { step_id: 38, sql: ... }` (with `-- MIGRATION-ACCRETION-EXEMPTION:` additive-only comment):
     - `ALTER TABLE canonical_nodes ADD COLUMN properties TEXT;`
     - `ALTER TABLE canonical_nodes ADD COLUMN scope TEXT NOT NULL DEFAULT '';`
     - `ALTER TABLE canonical_edges ADD COLUMN properties TEXT;`
     - `ALTER TABLE canonical_edges ADD COLUMN scope TEXT NOT NULL DEFAULT '';`
     - `ALTER TABLE operational_mutations ADD COLUMN scope TEXT NOT NULL DEFAULT '';`
     - `ALTER TABLE operational_state ADD COLUMN scope TEXT NOT NULL DEFAULT '';`
     - Create `canonical_property_values(entity_class, write_cursor, attr_name, value_type, text_val, num_val NUMERIC, bool_val)` with CHECK constraints (using `NUMERIC` affinity so 64-bit `i64` values with `|x| > 2^53` never lose precision) and 4 composite B-tree indexes (`_cursor_idx`, `_text_idx`, `_num_idx`, `_bool_idx`).
     - Create both partial active incident edge indexes (`canonical_edges_from_active_idx(from_id, kind) WHERE superseded_at IS NULL`, `canonical_edges_to_active_idx(to_id, kind) WHERE superseded_at IS NULL`) and historical edge composite indexes (`canonical_edges_from_kind_sup_idx(from_id, kind, superseded_at)`, `canonical_edges_to_kind_sup_idx(to_id, kind, superseded_at)`).
     - Attach read-visibility coalescing triggers to `canonical_property_values` (`cpv`) matching step 37 and register `("cpv", "canonical_property_values")` in `VISIBILITY_TRIGGER_TABLES` (`frozen_read.rs`).
2. **Inline typed properties outside `body` + declared typed indexes (`fathomdb-engine`)**:
   - Define `PropertyValue` (`String`, `Int(i64)`, `Float(f64)`, `Bool(bool)`), `PropertyMap` (`BTreeMap<String, PropertyValue>`), and `PropertyPredicate` (`Eq`, `Ne`, `Lt`, `Lte`, `Gt`, `Gte`) with strict validation (no nested objects/arrays, no `NaN`/`Inf`, no empty keys, no NUL/`\x1e`).
   - Accept optional `properties` on `PreparedWrite::Node`, `PreparedWrite::Edge`, `PreparedWrite::ProvenancedNode`, `PreparedWrite::ProvenancedEdge`, and `ActuationOperationV1` node/edge operations; persist canonical compact JSON in `properties` outside `body`.
   - Update property values ONLY via immutable new revisions (superseding prior version), never in-place `UPDATE`.
   - Same-transaction synchronous projection & active-only maintenance: project declared `Filterable` properties into `canonical_property_values` (precedence: explicit `properties` over `json_extract(body, ...)`) and populate `vec0` metadata columns for declared node properties. Purge prior cursors' `canonical_property_values` rows on node supersession, edge G0 (`logical_id`) and G11 (`(from_id, to_id, kind)`) supersession, and `transition` to `deleted`/`pending` (re-projecting on `transition` back to `active`).
   - Extend `SearchFilter` with `property_predicates: Vec<PropertyPredicate>` evaluated via indexed `SEARCH` on `canonical_property_values` (`EXPLAIN QUERY PLAN` verified); return a typed error if an undeclared key is passed to corpus-wide `SearchFilter`.
   - Hydrate `properties` in O(1) on `SearchHit`, `read.get` / `read.get_many` / `read.list`, and resolved evidence (bound to exact revision `write_cursor`).
   - Preserve `0.8.30` unpredicated fast paths (`hybrid_deferred_identity`, streamed FTS) with zero extra JOINs when `property_predicates` is empty.
3. **App-declared vector-committable kinds (Memex R1)**:
   - Update `vector_storage.rs` and all engine call sites (`projection_commit.rs`, `projection_registry.rs`, `projection_generation.rs`, `index_projector.rs`, `write.rs`, `search.rs`, `filter.rs`, `data_plane_integrity.rs`) so built-in kinds keep existing `source_type` mapping (`doc` -> `article`) and any valid custom kind explicitly enrolled in `_fathomdb_vector_kinds` / `configure_projections` is vector-committable with `source_type = kind`.
4. **Operational-collection kind readback (Memex R12)**:
   - Reject re-registering an existing operational collection with a conflicting `kind` (`EngineError::WriteValidation`) in `write_commit.rs` (`PreparedWrite::AdminSchema`).
   - Add governed readback `list_operational_collections` / `operational_collections` (and lookup by name) across Engine, `fathomdb-sdk`, Python, and TypeScript (`governed-operation-parity.json` + `governed-surface-allowlist.json`).
5. **W12 erasure ergonomics fixes (Memex Ask 3)**:
   - (3a) In Autonomous mode (`erasure.rs`), run the first `wal_checkpoint(TRUNCATE)` BEFORE `prepare_redacted` clears `denied_sources`/`denied_identities` and sets `debt.redacted = true`, so a checkpoint BUSY keeps `guard_plain_write_inner` + `certify_plain_write` open for unrelated plain writes; once first truncate succeeds, run `prepare_redacted` + final truncate + finalize (targeted amendment to `ADR-0.8.28-resumable-physical-cleanup.md`; audit `ActualCheckpointObserver` assertions). Preserve ExplicitMaintenance ordering untouched.
   - (3b) Add `completed_pending_erasure: bool` (`completedPendingErasure: boolean` in TS) to `ExciseReport` / `EraseReport` across Rust, Python, and TypeScript, setting `true` on both `finish_redacted_before_request` and `root_retry || !pending.is_empty()` completing-retry paths; skip `excise_source_audit` ONLY when `completed_pending_erasure && nodes_excised == 0 && edges_excised == 0`.
   - (3c) Decouple always-on lightweight atomic active-role/checkpoint counters (zero mutex/`eprintln!` on the hot path) from opt-in `FATHOMDB_WAL_ATTRIBUTION` stderr tracing so production release builds truthfully classify `Engine::checkpoint_erasure_at_rest`'s `ErasureIncomplete.detail` and distinguish initial erasure vs. retry wording.
6. **Erasure coverage & `sqlite_master` completeness audit (with `ExportClass`)**:
   - Register `canonical_property_values` in `ROW_OWNED_PROJECTIONS` (`projection_registry.rs`).
   - Extend `physical_erasure/admission.rs` (`envelope`) to sum `COALESCE(length(CAST(body AS BLOB)), 0) + COALESCE(length(CAST(properties AS BLOB)), 0)` on `canonical_nodes` and `canonical_edges` (both `canonical_edges.body` and `properties` are nullable).
   - Add an automated `sqlite_master` completeness audit test classifying every table in a schema-38 DB across `CANONICAL_TABLES` / `ROW_OWNED_PROJECTIONS` / internal `_fathomdb_*`, `VISIBILITY_TRIGGER_TABLES`, erasure coverage, `doctor`, and `ExportClass` (`DurableAuthority | OptInVector | OptInAudit | DerivedProjection | InternalRuntime`).
   - Add RED→GREEN `purge` and `erase_source` (Autonomous + ExplicitMaintenance) tests verifying zero surviving `canonical_property_values` rows, zero WAL/file grep remnants of inline property values, and valid closure proofs.
7. **Full SDK parity (X1)**:
   - Update `fathomdb-sdk`, `fathomdb` facade, `fathomdb-py` + `src/python/` (including `_fathomdb.pyi`, 0 pyright errors), `fathomdb-napi` + `src/ts/`, `src/conformance/governed-operation-parity.json`, and `src/conformance/governed-surface-allowlist.json`.
   - Update affected tests referencing hardcoded `SCHEMA_VERSION == 37` to `38` cleanly.

**Deferred / out of scope (do NOT build here):**

- Strict-only write endpoint integrity, `graph.expand` predicates, opt-in fused-hit seeding, and mixed-edge slot fix (Slice 20).
- Historical graph `as_of` traversal (Slice 30).
- `resolve_evidence_many`, `search(hints=)`, `OpenReport` booleans, `fathomdb migrate` 37→38, `fathomdb-cli` PyPI wheel (Slice 40).
- Inspector UI changes (Slice 50).
- Activating caller `scope` enforcement (`0.8.32`).

## 3. Execution discipline (strict TDD)

1. Commit a concise design memo at `dev/design/slice-10-property-schema-design.md` (`status: ACTIVE`).
2. Commit **RED** failing tests first (Rust engine + schema + erasure + `EXPLAIN QUERY PLAN` + `sqlite_master` audit + Python/TS parity tests) and observe genuine failures before writing implementation code.
3. Implement **GREEN** cleanly across schema → engine → SDKs (`fathomdb-sdk`, `fathomdb-py`, `fathomdb-napi`, `src/python`, `src/ts`) → conformance manifests → targeted interface/doc updates.
4. Run self-checks in `$WT`:
   - `cargo test -p fathomdb-schema -p fathomdb-engine -p fathomdb-sdk -p fathomdb`
   - `cargo clippy --workspace --all-targets`
   - Rebuild owned Python wheel via `bash scripts/setup-python-owned.sh --build` (or `--provision` if `.venv` absent) and run affected `pytest` + `.venv/bin/pyright -p src/python` (0/0) + `.venv/bin/ruff check src/python`
   - Build and test TypeScript (`cd src/ts && npm run build && npm test` or affected suites)
   - `./scripts/agent-lint-migrations.sh`
   - `./scripts/agent-lint-md.sh`
5. Leave `$WT` clean (`git status` empty) on branch `impl/0.8.31-slice-10` — do NOT merge into `release/0.8.31` or push; report exact commit SHAs (design, RED, GREEN) and test counts to the orchestrator.
