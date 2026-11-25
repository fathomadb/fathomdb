---
title: Slice 40 Plan — 0.8.32 Production Read+Write fathomdb-mcp, Pin C Seams, Bug 1/2 Fixes, and 3-Channel Distribution
status: APPROVED
target_release: 0.8.32
slice: 40
date: 2026-11-25
---

# Slice 40 Plan — `MCP-RW`

- **Target branch / worktree:** `dev/0.8.32` at `/mnt/data/worktrees/release-0.8.32` (never mutate `/home/coreyt/projects/fathomdb`).
- **Requirements covered:** `R32-02` (production read+write `fathomdb-mcp` with `allow_writes: false` default, per-store opt-in + bound caller `scope`, `SC-01`–`SC-10` fail-fast startup conflicts, `AC32-MCP-SAFETY-01`–`09`, SDK-vs-MCP exact frozen-evidence equivalence, `PutLatestState` + `EP-07` + `EP-01` `Pin C` seams, and 3-channel release packaging) and `R32-06` (`Gate1-D9` `0.8.31` reliability & schema fixes: empty-DB reopen cursor bug + per-class projection namespace Option B2-strict in unreleased Schema 39).
- **Authoritative ADRs & rulings:**
  - `dev/adr/ADR-0.8.32-mcp-read-write-and-distribution.md` (`#99`, accepted 2026-11-24 — Owner Rulings 1 & 3, `Gate1-D4` Pin C, `Gate1-D5`)
  - `dev/adr/ADR-0.8.32-caller-scope-isolation.md` (`#102`, accepted 2026-11-24)
  - `dev/design/mcp-adapter/engine-proposals/EP-01-store-state-read.md` & `EP-07-open-without-create.md`
  - `dev/plans/release-state-0.8.32.json` (`Gate1-D4` Pin C pre-authorization: `core = 5` and `recovery_denylist = 5` strictly invariant; `Gate1-D5` `--features default-embedder` for GH Release binaries & PyPI wheels while `default = []` on `crates.io`; **`Gate1-D9` Option B2-strict** owner approval 2026-11-25).
- **Hard invariants:**
  - **`MR-070.2` & `rmcp` confinement (`fathomdb-mcp/tests/manifest.rs`):** `fathomdb-mcp` links ONLY `fathomdb-sdk` in runtime dependencies (zero `rusqlite` or other `fathomdb*` crate deps), features forward ONLY to `fathomdb-sdk`, `rmcp = "=3.5.1"` stays exact-pinned, and ONLY `src/edge.rs` and `src/main.rs` may reference `rmcp::`.
  - **Default read-only MCP posture (`allow_writes: false`) & exact `7`-tool read-only surface (`ADR-0.8.32 #99:27`):** Read-only profiles expose the exact 7 read-only tools (`list_stores`, `use_store`, `orient`, `search`, `fetch`, `expand`, `feedback`), with bounded exact frozen-evidence support integrated via optional frozen-context/evidence parameters on `search`, `expand`, and `fetch`. Mutating tools (`actuate`, `put_latest_state`) are omitted from `tools/list` on read-only profiles and refused deterministically (`writes_disabled`, zero DB mutation) if invoked directly.
  - **Per-store write opt-in requires non-empty bound writable caller `scope`:** `roots` auto-discovered stores are always strictly read-only.
  - **Strict-only MCP writes & zero operator escape hatches:** MCP never exposes bulk/non-strict writes, raw SQL, or recovery/erasure/migration commands (`AC32-MCP-SAFETY-05`, `AC32-MCP-SAFETY-09`).
  - **Base Python wheel `AC-041`:** `src/python/pyproject.toml` `[project.scripts]` stays strictly empty; `fathomdb-mcp` ships as lockstep binary wheel `src/python-mcp/` exposed via optional extra `fathomdb[mcp]`.
  - **`struct Engine` (`49` fields) & root `impl Engine` (`6` methods):** Any edit to `fathomdb-engine/src/lib.rs` updates `lib_rs_blob:` in `dev/plans/0.8.27/features/slice-90/phase3-root-reconciliation.md:4` in the same commit.
  - **Strictly typed `ProjectionTarget` (`Gate1-D9` Option B2-strict, `REQ-046b`):** Zero bare-string shims on `configure_projections` `drop` or `ProjectionDelta.{built, dropped, deferred}`.
  - **Deferred out of Slice 40:** `EP-05` (`read::kinds`), `EP-10`'s `origin_class`, `EP-16`, and npm MCP wrapper package.
  - TDD first (`failing test -> red -> green -> refactor`); zero external consumer-project specifics; no `--no-verify` / `--no-gpg-sign`; no `prettier` or unguarded `markdownlint-cli2 --fix` on `.md` (`npm run format:md` only).

---

## 1. Work Package Decomposition (Strictly Sequenced)

The 4 work packages execute sequentially under strict one-writer-per-checkout discipline:

1. **`WP-40A1`** — `Gate1-D9` Bug 1 (empty-DB reopen cursor fix + `data_plane_integrity` terminal-only handling) & Bug 2 Option B2-strict (unreleased Schema 39 `_fathomdb_projection_registry PRIMARY KEY(entity_class, name)` + strictly typed `ProjectionTarget` across Rust/Python/TS + additive `load_next_cursor` indexes).
2. **`WP-40A2`** — `Pin C` Engine & SDK/Python/TS Seams (`PutLatestState` in `ActuationBatchV1`, `EP-07` lock-before-embedder `create_if_missing` + `DatabaseNotFound`, `EP-01` `read::visibility_state`, governed pin `57 -> 58` / `95 -> 97`, docs/interfaces).
3. **`WP-40B`** — Production Read+Write `fathomdb-mcp` (`SC-01`–`SC-10`, `AC32-MCP-SAFETY-01`–`09`, live `EP-01` `state`, `EP-07` open, scoped frozen evidence, `actuate` & `put_latest_state`, blocking drain fence, detach `locomo_*` examples).
4. **`WP-40C`** — 3-Channel `fathomdb-mcp` Distribution, All Release/Contract Gates & MCP Docs (`publish = true`, `src/python-mcp/`, `release.yml`, `promote-npm-release.yml`, `docs-pages.yml`, all 12+ contract tests, `AGENTS.md`, `dev/design/*` lifecycle, `docs/`).

---

### WP-40A1 — `Gate1-D9`: Empty-DB Reopen Cursor Fix (Bug 1) & Per-Class Projection Namespace Option B2-Strict (Bug 2)

1. **Bug 1 — Empty-DB Reopen Cursor Fix & Boundary Consistency (Zero Schema Change):**
   - `src/rust/crates/fathomdb-engine/src/write_commit.rs:5-33` (`load_next_cursor`): fold in `max_cursor(connection, "_fathomdb_commit_timestamps").unwrap_or(0)` and `max_cursor(connection, "_fathomdb_projection_terminal").unwrap_or(0)` so registering `AdminSchema` collections on an empty DB (or as the trailing write on a non-empty DB, or after purging high-cursor rows once `cursor_floor` clears) never regresses `next_cursor` on reopen, never hits `SQLITE_CONSTRAINT_PRIMARYKEY` -> `EngineError::Storage` on `_fathomdb_commit_timestamps`, and never silently skips projection on multi-row post-reopen batches.
   - `src/rust/crates/fathomdb-engine/src/projection_generation.rs:330-363` (`authoritative_boundary`) & `src/rust/crates/fathomdb-engine/src/physical_erasure.rs:302`: fold in `max_cursor(..., "_fathomdb_commit_timestamps").unwrap_or(0)` so all boundary readers agree.
   - **Additive O(log N) indexes for `load_next_cursor` / `EP-01` in unreleased `STEP_39_SQL` (`fathomdb-schema/src/lib.rs`):**
     - Add `CREATE INDEX IF NOT EXISTS operational_mutations_write_cursor_idx ON operational_mutations(write_cursor);` and `CREATE INDEX IF NOT EXISTS _fathomdb_dependency_closures_boundary_idx ON _fathomdb_dependency_closures(admitted_write_boundary);` so `load_next_cursor()` (and `EP-01` `read::visibility_state()`) runs in O(log N) with zero full table scans!
   - `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:1976-2083, 2418-2530`: teach `active_projection_findings` and `projection_generation_findings` to recognize terminal-only `'up_to_date'` cursors owned by `operational_mutations`, `operational_state`, or `_fathomdb_commit_timestamps` (`AdminSchema`) when vector/sidecar/span rows are all absent, avoiding false-positive `DenseProjectionOwnerMissing` / `ProjectionMemberCorrupt` findings.

2. **Bug 2 — Option B2-strict: Per-Class Projection Namespace in Unreleased Schema 39 (`PRIMARY KEY(entity_class, name)`) & Strictly Typed `ProjectionTarget`:**
   - **Schema 39 (`src/rust/crates/fathomdb-schema/src/lib.rs`):**
     - In `STEP_39_SQL` (still unreleased!), recreate `_fathomdb_projection_registry` preserving the exact Schema-38 columns and constraints (`name TEXT NOT NULL`, `roles TEXT NOT NULL`, `fts_tokenizer TEXT`, `vector_embedder TEXT`, `vector_declared INTEGER NOT NULL DEFAULT 0`, `source TEXT`, `entity_class TEXT NOT NULL DEFAULT 'node' CHECK(entity_class IN ('node', 'edge'))`, `vector_kinds TEXT`, `PRIMARY KEY(entity_class, name)`), migrating rows losslessly via `INSERT...SELECT` (Schema-38 names are already unique) and restoring all 3 `_fathomdb_projection_registry` visibility triggers (`pr_ai`/`pr_au`/`pr_ad`).
     - Update `dev/fixtures/golden-0.8.31-schema38.sqlite` migration tests (`slice20_migrate_38_to_39.rs`, `migrate_cli.rs`) to verify `_fathomdb_projection_registry` upgrades cleanly to `PRIMARY KEY(entity_class, name)`.
   - **Engine (`properties.rs`, `projection_registry.rs`, `projection_runtime.rs`, `projection_generation.rs`, `filter.rs`, `search/projected_text.rs`, `vector_storage.rs`, `open.rs`, `frozen_read.rs`, `data_plane_integrity.rs`, `lib.rs`):**
     - Derive `Ord, PartialOrd` on `PropertyEntityClass` (`src/rust/crates/fathomdb-engine/src/properties.rs:26-33`) so `(PropertyEntityClass, String)` is a valid `BTreeMap` / `BTreeSet` key.
     - Introduce strictly typed value struct:

       ```rust
       #[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
       pub struct ProjectionTarget {
           pub name: String,
           pub entity_class: PropertyEntityClass,
       }
       ```

       with `ProjectionTarget::node(name: impl Into<String>) -> Self`, `ProjectionTarget::edge(name: impl Into<String>) -> Self`, and `ProjectionTarget::new(name: impl Into<String>, entity_class: PropertyEntityClass) -> Self`. (Do NOT add bare-string `From<&str>` shims per `Gate1-D9` strictly typed ruling!)
     - Switch `Engine::configure_projections(&self, specs: &[ProjectionSpec], drop: &[ProjectionTarget]) -> Result<ProjectionDelta, EngineError>` and `ProjectionDelta { pub built: Vec<ProjectionTarget>, pub dropped: Vec<ProjectionTarget>, pub deferred: Vec<ProjectionTarget>, pub unchanged: bool }`.
     - Add `pub entity_class: PropertyEntityClass` to `ProjectionRuntimeStatusEntry` (`projection_runtime.rs:80-86`, populated in `projection_generation.rs:2113-2123`, mirrored in SDK/Python/TS + `python_surface_additions.json` / `test_slice130_package_boundary.py`) so `read_projection_status` never emits ambiguous duplicate entries when a `Node` and an `Edge` projection share a name.
     - Update **every** single-key `(PropertyEntityClass, String)` registry site:
       - `projection_registry.rs`: `load_projection_registry` (`BTreeMap<(PropertyEntityClass, String), StoredProjection>`), `validate_nested_projection_sources_for_body_in_registry` (`906-912`), `load_projection_registry_row` (`1046-1071`, `WHERE entity_class = ?1 AND name = ?2`), `persist_projection_row` (`ON CONFLICT(entity_class, name) DO UPDATE`), `remove_projection_row` (`WHERE entity_class = ?1 AND name = ?2`), `backfill_canonical_property_values_for_schema_38` (`1146, 1166`), `clear_attribute_projection` (delete `canonical_property_values WHERE entity_class = ?1 AND attr_name = ?2`; touch node-only `canonical_attributes`/`property_search_index` ONLY when `entity_class == Node`), `vector_attr_insert_fragments` (`1456`), `project_node_attributes_with_registry` / `project_edge_properties_with_registry` (`1493-1533`), `configure_projections` dedupe/drop/upsert/clear loop (`1845-1956, 2123-2124`), `rederive_projections_on_boot` (`2775-2778`), and test mutators `set_legacy_projection_vector_declared_for_test` / `set_legacy_projection_search_subobjects_for_test` (`3202, 3234-3238`).
       - `filter.rs:523-572`, `search/projected_text.rs:42-50`, `vector_storage.rs:190-198, 413, 425-427` (`reshape_vector_partition_nondestructive` `registry.get(&(PropertyEntityClass::Node, name))`), and `open.rs:937`.
       - `projection_generation.rs:538` and `frozen_read.rs:617`: add `, entity_class` tie-breaker to `ORDER BY` (keeping digests 100% identical for Schema-38 migrated stores).
       - `data_plane_integrity.rs:1636-1652, 2095-2124, 2230-2245`: scan & look up by `(entity_class, name)`.
     - Re-export `ProjectionTarget` from `fathomdb-engine/src/lib.rs` and update `dev/plans/0.8.27/features/slice-90/phase3-root-reconciliation.md:4` `lib_rs_blob:` in the same commit!
   - **Rust Facade/SDK, Python & TypeScript Parity (Strictly Typed, Zero Bare-String Shims):**
     - **Rust (`fathomdb`, `fathomdb-sdk`):** re-export `ProjectionTarget`, update `Engine::configure_projections(&self, specs: &[ProjectionSpec], drop: &[ProjectionTarget]) -> Result<ProjectionDelta>` (adding `guard::projection_target` for `drop`), update `ProjectionRuntimeStatusEntry` mirrors, `reexports.rs`, & `surface.rs`.
     - **Python (`fathomdb-py`, `src/python/`):** expose `@dataclass(frozen=True) class ProjectionTarget: name: str; entity_class: PropertyEntityClass = "node"` from `fathomdb` and `fathomdb.types`; `Engine.configure_projections(self, specs: list[ProjectionSpec], drop: list[ProjectionTarget] | None = None) -> ProjectionDelta` (strictly rejecting bare `str` in `drop` with `TypeError`!); `ProjectionDelta.{built, dropped, deferred}: list[ProjectionTarget]`; add `entity_class` to `ProjectionRuntimeStatusEntry`; update `_fathomdb.pyi`, `python_surface_additions.json`, `test_slice130_package_boundary.py`, and rebuild `.venv` wheel via `bash scripts/setup-python-owned.sh --build`.
     - **TypeScript (`fathomdb-napi`, `src/ts/`):** export `ProjectionTarget { readonly name: string; readonly entityClass?: PropertyEntityClass }` from `src/ts/src/index.ts`; `Engine.configureProjections(specs: ProjectionSpec[], drop?: ProjectionTarget[]): Promise<ProjectionDelta>` (strictly rejecting bare `string` in `drop` with `TypeError`!); `ProjectionDelta.{built, dropped, deferred}: ProjectionTarget[]`; add `entityClass` to `ProjectionRuntimeStatusEntry`; rebuild addon & `dist/`.
     - **Interfaces & Docs:** update `dev/interfaces/{rust,rust-sdk,python,typescript,wire}.md` and `docs/reference/{rust-sdk,rust-api,python-api,typescript-api}.md` (zero governed-surface verb count change: `engine.configure_projections` stays `57/57` & `95/95` in `WP-40A1`).
   - **TDD Tests (`WP-40A1`):**
     - Rust default-feature suite (`src/rust/crates/fathomdb-engine/tests/slice40_reopen_cursor_and_projection_ns.rs`):
       - Empty-DB `admin.configure` -> close -> reopen -> 1-row write & multi-row write (confirming no `StorageError` and projection runs on row 0); non-empty trailing `admin.configure` -> reopen -> write.
       - Registering `"status"` on BOTH `node` and `edge` in one call and across sequential calls succeeds; dropping `ProjectionTarget::edge("status")` deletes ONLY edge CPV/registry rows while preserving node `"status"` EAV/FTS/CPV/`vec0` intact; `ProjectionDelta.{built, dropped, deferred}` carries exact `ProjectionTarget` entries; `read_projection_status` carries distinct `entity_class` entries; `SearchFilter` & `search_projected_text` resolve `(Node, "status")` cleanly.
     - Operator/CLI test (`src/rust/crates/fathomdb-cli/tests/doctor_reopen_cursor.rs`): confirms `fathomdb doctor data-plane-integrity --json` exits `0` (`0` findings) on an empty DB with `AdminSchema` / `OpStore` terminal cursors.
     - Update existing Rust/Python/TS projection tests (`slice38_properties_and_indexes.rs`, `test_slice38_properties.py`, TS property/projection tests, `scripts/slice135_python_capabilities.py` if it passes `drop`) and add Python & TS tests asserting simultaneous `(node, "status")` + `(edge, "status")`, isolated drop, and `TypeError` on bare-string `drop`.
   - **WP-40A1 Gate:**
     - Default features: `cargo test -p fathomdb-engine --test slice40_reopen_cursor_and_projection_ns`
     - Operator / test-hook features: `cargo test -p fathomdb-engine --features operator,migration-test-hooks,test-hooks --test slice38_properties_and_indexes --test slice20_migrate_38_to_39`
     - CLI, SDK, bindings & lint: `cargo test -p fathomdb-cli --test doctor_reopen_cursor --test migrate_cli`, `cargo test -p fathomdb-sdk --test surface`, `cargo clippy --workspace --all-targets -- -D warnings`, `python3 scripts/tests/test_slice90_root_reconciliation.py`, `cd src/ts && npm test`, Python projection + boundary + `test_slice135_*.py`, `./scripts/agent-typecheck.sh`, `./scripts/agent-lint.sh`.

---

### WP-40A2 — `Pin C` Engine & SDK/Python/TS Seams (`PutLatestState`, `EP-07`, `EP-01`) (`R32-02`, `X1`, `X4`, `P4-10`, `P4-11`)

1. **Atomic `PutLatestState` inside `ActuationBatchV1` (`actuation.rs` + bindings):**
   - In `src/rust/crates/fathomdb-engine/src/actuation.rs`:
     - Add `LatestStateActuationV1` (`collection: String`, `record_key: String`, `payload_json: String` with constructor `LatestStateActuationV1::new(collection, record_key, payload_json)` validating non-empty collection/record_key and valid JSON payload; **re-validate all fields inside `validate_request` (`actuation.rs:420-442`)** even if fields are public/mutated, mirroring `LifecycleActuationV1`).
     - Add `ActuationOperationV1::PutLatestState(LatestStateActuationV1)`.
     - Update ALL exhaustive match & commit-sidecar sites in `actuation.rs`:
       - `encode_operation` (`444-483`): unique canonical tag + `collection`, `record_key`, `payload_json` in the `operation_id` digest so reusing `operation_id` with any differing checkpoint field fails atomically with `ActuationErrorReason::OperationIdConflict` and zero mutation;
       - `collect_source_refs` (`1153-1197`): non-artifact op (zero source refs);
       - `expected_affected_revisions` (`1424-1519`): non-artifact op (zero affected artifact revisions);
       - `simulate_request` / `apply_request` (`1537, 2028, 2133, 2172-2191`): **unconditionally** validate that `collection` is registered in `operational_collections` with kind `latest_state` AND validate `payload_json` against `operational_collections.schema_json`; increment `put_count`, `rows`, `simulation_cursor_base`, and `cursor_available` overflow guard; upsert `(scope, collection_name, record_key)` in `operational_state` under opener-bound caller `scope`; call `record_projection_terminal(tx, cursor, "up_to_date")`, `advance_projection_cursor(tx)`, and `temporal::record_batch_commit_timestamp` (`_fathomdb_commit_timestamps` when `next_cursor > base_cursor`); emit empty `pending_projection_write_cursors` for this op;
       - **`map_domain_error` (`1763-1824`), `!cursor_available` (`2172-2193`), & `validate_refusal_shape` (`998-1046`):** update `map_domain_error` to take operation context so `PutLatestState` maps `EngineError::OpStore` (unregistered collection / non-`latest_state` kind) -> `/collection` and `SchemaValidation` / `WriteValidation` -> `/payloadJson` or `/recordKey` (never hardcoding `"/record"` for `PutLatestState`!); include `PutLatestState` in the `!cursor_available` overflow scan (`2172-2193`, emitting `/operations/{index}/collection`); and update `validate_refusal_shape` (`1018-1026`) to permit `/collection`, `/recordKey`, and `/payloadJson` on `WriteRefused` and `/collection` on `WriteCursorExhausted`.
       - verify `_fathomdb_actuation_receipts` lookup/persistence is strictly scoped to caller `scope` (`PRIMARY KEY(scope, operation_id)`).
   - Expose `LatestStateActuationV1` / `PutLatestState` across Rust (`fathomdb`, `fathomdb-sdk`), Python (`fathomdb-py`, `src/python/`), and TypeScript (`fathomdb-napi`, `src/ts/`) `actuate` codecs + tests.

2. **`EP-07` (`OpenOptions.create_if_missing: bool` + `EngineOpenError::DatabaseNotFound`):**
   - In `src/rust/crates/fathomdb-engine/src/open.rs`:
     - Update `PendingDatabaseLock` (`open.rs:447-481`) to retain both `lock_path: PathBuf` and `created: bool` (true iff `create_new` created `.lock`).
     - **Correct lock-before-embedder ordering (`open.rs:522-545, 626-675, 853-861`):** Refactor open entrypoints so `PendingDatabaseLock` is acquired and pre-creation admissibility (`dangling_symlink` -> `Io` precedence, plus `create_if_missing == false` missing/zero-length refusal -> unlink `.lock` ONLY when `created == true` and return `EngineOpenError::DatabaseNotFound`, touching zero existing zero-length file bytes!) runs **BEFORE** `open_default_embedder` claims CUDA pool or loads BGE weights, holding that exact same lock into `open_with_migrations` (zero lock-drop-reacquire TOCTOU gap!).
     - Thread `create_if_missing: bool` (default `true`) via a clean open entrypoint on `fathomdb-engine`, updating `scripts/tests/test_slice90_open_methods_owner.py:11-25, 40` in the same commit!
     - Add unit race test using `run_admission_locked_hook_for_test` (`open.rs:799-800`) deleting the file under the lock and asserting `DatabaseNotFound` with zero created `.sqlite`/`.lock` files.
   - In `src/rust/crates/fathomdb-sdk/src/options.rs`:
     - Add `pub create_if_missing: bool` to `OpenOptions` with manual `impl Default` (`create_if_missing: true`) and `with_create_if_missing(mut self, create_if_missing: bool) -> Self`; audit all workspace `OpenOptions { ... }` struct literals (`options.rs:14-19` is not `#[non_exhaustive]`). Keep `EngineConfig` (and EARP `test_config_resolver.py`) unchanged!
   - **Error taxonomy (`49` shared error classes: base + `48` leaf):**
     - Add `EngineOpenError::DatabaseNotFound` (`fathomdb-engine`), `fathomdb_sdk::ErrorKind::DatabaseNotFound` (`fathomdb-sdk`), `DatabaseNotFoundError` in Python (`fathomdb-py`, `src/python/fathomdb/errors.py`, `_fathomdb.pyi`, `python_surface_additions.json`), and `DatabaseNotFoundError` in TypeScript (`fathomdb-napi`, `src/ts/src/errors.ts`, `src/ts/src/index.ts`), plus `create_if_missing: bool = True` on Python `Engine.open(...)` / `createIfMissing?: boolean` on TS `Engine.open(...)`.
     - Update all error-taxonomy parity tests (`fathomdb-sdk/tests/surface.rs`, `src/python/tests/test_errors.py`, `src/ts/tests/errors.test.ts`, `dev/interfaces/*`, `docs/reference/errors.md`).

3. **`EP-01` (`read::visibility_state` -> `VisibilityStateV1`):**
   - Respect `dev/tools/module-boundary-policy.txt:236, 262` (forbids `read -> reader_pool`):
     - Define `#[non_exhaustive] pub struct VisibilityStateV1 { pub schema_version: u32, pub database_id: String, pub generation: u64, pub nonce: String, pub canonical_write_boundary: u64, pub projection_cursor: u64 }` + read-only SQL helper `read_visibility_state_in_tx(conn: &Connection) -> Result<VisibilityStateV1, EngineError>` in `src/rust/crates/fathomdb-engine/src/read.rs` (one deferred read transaction; zero writer lock; zero payload content; uses the new O(log N) `load_next_cursor()` indexes!).
     - Add `ReaderRequest::VisibilityState` in `src/rust/crates/fathomdb-engine/src/reader_pool.rs`.
     - Add `pub fn read_visibility_state(&self) -> Result<VisibilityStateV1, EngineError>` in `src/rust/crates/fathomdb-engine/src/read_api.rs` (NOT `lib.rs`!), applying the in-process `fetch_max` generation-regression guard (`EngineError::FrozenRead(StateUnavailable)` on regression, `EngineError::Closing` after close, `EngineError::Storage` if singleton missing). Note `EP-01:85-89` (`operational_mutations` has no visibility trigger — document that `canonical_write_boundary` can advance without advancing `generation` on append-only mutation writes, and pin `Engine::open` generation-stability behavior in tests!).
     - Re-export `VisibilityStateV1` and `LatestStateActuationV1` from `fathomdb-engine/src/lib.rs`, updating `dev/plans/0.8.27/features/slice-90/phase3-root-reconciliation.md:4` `lib_rs_blob:` in the same commit (`struct Engine` stays `49` fields, root `impl Engine` stays `6` methods).
   - Expose across all 3 SDKs:
     - Rust: `fathomdb_sdk::read::visibility_state(engine: &Engine) -> Result<VisibilityStateV1>` in `fathomdb-sdk/src/read.rs` (+ re-export `VisibilityStateV1`, `LatestStateActuationV1` in `fathomdb` & `fathomdb-sdk`).
     - Python: `fathomdb.read.visibility_state(engine) -> VisibilityStateV1` (`fathomdb-py`, `src/python/fathomdb/read.py`, `types.py`, `_fathomdb.pyi`, `python_surface_additions.json`, `test_slice130_package_boundary.py`), rebuilt via `bash scripts/setup-python-owned.sh --build`.
     - TypeScript: `read.visibilityState(engine) -> VisibilityStateV1` (`fathomdb-napi`, `src/ts/src/read.ts`, `binding.ts`, `index.ts`), rebuilt via `cd src/ts && npm run build`.

4. **Governed Surface Pin (`Gate1-D4` Pin C Pre-Authorized; `core = 5`, `recovery_denylist = 5` Strictly Invariant):**
   - Update in lockstep:
     - `src/conformance/governed-operation-parity.json`: `57 -> 58` operations — add `"id": "read.visibility_state"`, `"locator": "read"`, Rust `"visibility_state"`, Python `"visibility_state"`, TypeScript `"visibilityState"`.
     - `src/conformance/governed-surface-allowlist.json`: `95 -> 97` members (`"visibility_state"`, `"visibilityState"`).
     - `scripts/governed-surface-pin.json`: updated `sha256`, `git_blob_sha1`, counts (`97 / 5 / 5 / 2`), member lists, `Gate1-D4 Pin C (0.8.32 Slice 40)` provenance.
     - Update ALL parity/pin/doc/slice135 tests: `scripts/tests/test_check_governed_surface_pin.sh`, `scripts/tests/test_check_sdk_surface_parity.py`, `src/ts/tests/sdk-surface-parity.test.ts` (`57/57 -> 58/58`), `src/python/tests/test_sdk_surface_parity_oracle.py`, `scripts/tests/test_generated_public_docs.py`, `dev/interfaces/rust-sdk.md` (`57 -> 58`), `scripts/slice135_python_capabilities.py` (register `"read.visibility_state"`).
     - Regenerate `docs/generated/sdk-operation-map.md` via `.venv/bin/python scripts/generate-public-docs.py --write`.
   - Update `dev/interfaces/{rust,rust-sdk,python,typescript}.md` and `docs/reference/{rust-sdk,rust-api,python-api,typescript-api,errors}.md`.

5. **WP-40A2 Gate:** `cargo test -p fathomdb-engine --test slice40_pin_c`, `cargo test -p fathomdb-sdk --test surface`, `cargo test -p fathomdb --test reexports`, `cargo clippy --workspace --all-targets -- -D warnings`, `python3 scripts/tests/test_slice90_root_reconciliation.py`, `python3 scripts/tests/test_slice90_open_methods_owner.py`, `bash scripts/check-module-boundaries.sh`, `bash scripts/check-governed-surface-pin.sh`, `python3 scripts/check-sdk-surface-parity.py`, `bash scripts/tests/test_check_governed_surface_pin.sh`, `python3 scripts/tests/test_check_sdk_surface_parity.py`, `python3 scripts/tests/test_generated_public_docs.py`, `.venv/bin/python scripts/generate-public-docs.py --check`, `cd src/ts && npm test`, Python parity + boundary + `test_sdk_surface_parity_oracle.py` + `test_slice135_*.py`, `./scripts/agent-typecheck.sh`, `./scripts/agent-lint.sh`, `mkdocs build --strict`.

---

### WP-40B — Production Read+Write `fathomdb-mcp` (`R32-02`, `SC-01`–`SC-10`, `AC32-MCP-SAFETY-01`–`09`, `P2-07`, `P4-12`, `P5-10`, `P5-11`)

1. **Config Schema, Embedded Policy & Fail-Fast Startup Conflict Matrix (`SC-01`–`SC-10`, `config.rs`, `registry.rs`, `lib.rs`, `main.rs`):**
   - Replace evaluation-era `"write"` / `"scope": null` keys with:
     - per-store `allow_writes: bool` (default `false`);
     - per-store `scope: Option<ScopePolicySpec>` (parsed into `fathomdb_sdk::ScopePolicy`: single-scope string shorthand `"alpha"` -> `read_write(["alpha"], "alpha")` when `allow_writes: true` or `read_only(["alpha"])` when `allow_writes: false`, plus structured `{ read: [...], write: Option<String> }`);
     - optional per-store `allowed_kinds: Option<Vec<String>>` (write-only constraint metadata).
   - **Embedded host opt-in & `SC-10` contract (`lib.rs`, `registry.rs`):**
     - Read host `ScopePolicy` via `engine.config().scope_policy` (existing `fathomdb_sdk::Engine::config(&self) -> &EngineConfig` — NO new `Engine::scope_policy()` method!).
     - Add `HostedStorePolicy { pub allow_writes: bool, pub scope: Option<ScopePolicy>, pub allowed_kinds: Option<Vec<String>> }` (`Default`: read-only), `Adapter::register_engine_with_policy(name, engine, policy)` (with `register_engine(name, engine)` delegating to read-only default and checking `engine.config().scope_policy`), and `AdapterError::PolicyConflict(String)` so `SC-10` deterministically rejects any mismatch between `stores[name]` / `HostedStorePolicy` and `engine.config().scope_policy` (or colliding filesystem `stores[name]`) before serving, leaving the hosted engine untouched.
   - Validate ALL 10 conflict rules `SC-01`–`SC-10` identically across `fathomdb-mcp --config <path.json> --check-config` (exit `2`, empty `stdout`, structured JSON error on `stderr`, ZERO engine opens/locks), `Config::load` / `Config::from_json_str` / `Config::validate`, `Adapter::new`, and `Adapter::register_engine[_with_policy]`:
     - `SC-01`: `allow_writes: true` with `scope: null`, omitted, or empty `""` -> reject.
     - `SC-02`: `allow_writes: true` paired with a read-only `scope` grant (`write: null` / omitted in structured policy) -> reject.
     - `SC-03`: `allow_writes: false` paired with write-only/write-grant keys (`write: "..."` in scope, `allowed_kinds`, or `create: true`) -> reject. (Audit and update ALL existing `"create": true` / `"write"` test fixtures across `fathomdb-mcp/tests/config.rs:52, 64-65, 99-100, 258, 286, 300, 314` and `fathomdb-mcp/tests/protocol.rs:105-124, 173-180` to align `MR-001.1` with `SC-03`!)
     - `SC-04`: Legacy evaluation key `"write"` present (alone or alongside `"allow_writes"`) -> reject naming offending key(s).
     - `SC-05`: `roots` auto-discovery paired with write enablement -> reject (`roots` entries stay strictly read-only).
     - `SC-06`: Two store names resolving via canonical path or symlink (checking parent canonicalization for not-yet-created `create: true` targets too) to the same database file with incompatible `allow_writes`/`scope`/`eager` settings **OR** any duplicate alias where either entry enables `allow_writes: true` -> reject at startup (preventing both cross-scope bypass and unserialized concurrent-write alias races!).
     - `SC-07`: Multiple `eager: true` stores when `max_open_stores == 1` -> reject.
     - `SC-08`: Contradictory or malformed `scope` policy grammar (empty/duplicate tokens, wildcard `"*"`, write scope not in read scopes, NUL/control bytes, exceeding bounds) -> reject fail-closed via `ScopePolicy` validation.
     - `SC-09`: Non-addressable `default_store`, `search_mode: "hybrid"` with `use_default_embedder: false`, or `run_limit_ms < call_deadline_ms` -> reject.
     - `SC-10`: Embedded `Adapter::register_engine[_with_policy]` conflicts with configured `stores[name]` or host `engine.config().scope_policy` -> reject with `AdapterError::PolicyConflict`, leaving hosted engine untouched.
   - Replace `path.try_exists()` TOCTOU check in `registry.rs` with `EP-07` `OpenOptions::default().with_create_if_missing(entry.create).with_scope_policy(...)`, mapping `ErrorKind::DatabaseNotFound` -> `unavailable/missing_file`.
   - Wire `EP-01` `fathomdb_sdk::read::visibility_state` into MCP tool response `Envelope.state` (`status: "available"`, `database_id`, `generation`, `nonce`, `canonical_write_boundary`, `projection_cursor`, drift labels `canonical`/`index`/`unchanged`), update `OrientOutput.not_in_this_release` (`[&'static str; 3] = ["verdict", "kinds", "counts"]`, removing `"state"`!), key & sort `orient` projection/status joining (`tools.rs:528-609`, `output.rs:121-133`) by `(entity_class, name)` (eliminating Option B2-strict cross-class name collisions), and update `dev/experiments/0.8.29-slice30-measurements/m_wire.py:173, 189-223, 248-272` + `field_census.rs:78-79` so `scripts/test-slice30-m-cli.sh` stays 100% green!

2. **Canonical Tool Surface (Preserving Exact `7`-Tool Read-Only Posture per `ADR-0.8.32 #99:27`), Frozen Evidence, & Governed Writes (`tools.rs`, `edge.rs`, `session.rs`, `error.rs`, `output.rs`):**
   - **Read-only profile (`allow_writes: false` on all stores) — exact `7` tools in `tools/list`:**
     - `list_stores`, `use_store`, `orient`, `search`, `fetch`, `expand`, `feedback` (`m_native_tests.py`, `m_session.py`, and read-only `protocol.rs` `TOOL_NAMES: [&str; 7]` remain 100% satisfied!).
     - **Bounded exact frozen-evidence workflow integrated cleanly into `search`, `expand`, and `fetch`:**
       - `search`: optional `freeze: bool` (mints scoped `FrozenReadContextV1` and runs `search_with_evidence` / `search_frozen`, returning an authenticated `frozen_context` token + per-hit `evidence_ref`s) and optional `frozen_context: String` (reuses an existing frozen context token); extend `SearchArgs` continuation `only_paging` guard (`tools.rs:1028-1046`) to reject `freeze` / `frozen_context` on continuation-only calls.
       - `expand`: optional `frozen_context: String` / `include_evidence_refs: bool` (returns `graph_evidence_ref`s).
       - `fetch`: optional `frozen_context: String` + `evidence_refs: Vec<String>` / `graph_evidence_refs: Vec<String>` (calls `engine.resolve_evidence_many` / `engine.resolve_graph_evidence`, returning exact UTF-8 bytes, `artifact_revision_id`, caller-scope filtering, and typed `state_drift`, `erased`, `truncated`, and `unavailable` outcomes matching `fathomdb-sdk` identically!).
   - **Write-enabled profile (at least one store with `allow_writes: true` + bound writable `scope`) — `9` tools in `tools/list` (`7` read + `2` governed write):**
     - `actuate`: bounded governed `ActuationBatchV1` (`operation_id`, optional `expected_write_boundary`, strict-only operations `PutCanonicalNode`, `PutDerivedNode`, `PutDerivedEdge`, `RegisterSourceDependency`, `TransitionLifecycle`, `PutLatestState`).
     - `put_latest_state`: bounded ergonomic `latest_state` checkpoint tool (`operation_id`, `collection`, `record_key`, `payload_json`, optional `expected_write_boundary`, delegating atomically to `ActuationBatchV1` with one `PutLatestState`).
     - Both enforce `allowed_kinds` (when configured), strict-only endpoints, per-canonical-path write serialization (`AC32-MCP-SAFETY-01`), and reject bulk/non-strict flags or forbidden operator escape hatches (`AC32-MCP-SAFETY-05`, `AC32-MCP-SAFETY-09`).
     - Direct JSON-RPC invocation of `actuate` / `put_latest_state` on a read-only server or targeting a read-only store fails deterministically with `writes_disabled` and zero DB mutation.
   - **Closed `ErrorClass` & `ToolError` codes (`error.rs`, `field_census.rs`, `dev/interfaces/mcp.md` §6):**
     - Extend `ErrorClass` with `Conflict` (`"conflict"`) alongside existing 7 classes (`InvalidRequest`, `NotFound`, `NotReady`, `Unavailable`, `Timeout`, `Busy`, `Internal`).
     - Define exact template-only error mappings (zero record content, secret tokens, raw SQL, or cross-scope existence leakage — `AC32-MCP-SAFETY-08`):
       - `writes_disabled`: `ErrorClass::InvalidRequest`, `code: "writes_disabled"`
       - `kind_not_allowed`: `ErrorClass::InvalidRequest`, `code: "kind_not_allowed"`
       - `forbidden_operation`: `ErrorClass::InvalidRequest`, `code: "forbidden_operation"`
       - `stale_write_boundary`: `ErrorClass::Conflict`, `code: "stale_write_boundary"` (optional structured `observed_write_boundary` hint when content-free)
       - `operation_id_conflict`: `ErrorClass::Conflict`, `code: "operation_id_conflict"`
       - `scope_violation` / cross-scope reference: nondisclosing `ErrorClass::NotFound` (`code: "not_found"`) on reads/continuations, or `ErrorClass::InvalidRequest` (`code: "scope_violation"`) on write-authority refusal.
     - Keep `edge.rs` `INSTRUCTIONS` `<= 1200` bytes (dynamic or concise read-only vs read-write instructions naming every exposed tool!), update `unknown_argument_remedy`, `field_census.rs`, and `protocol.rs`.
   - **Session & continuation isolation (`AC32-MCP-SAFETY-04`):**
     - Key session/continuation handles by `(store, epoch, scope)` so cross-scope references and cursors return nondisclosing `not_found`, and caller payload fields never override opener-bound `scope`.
   - **Blocking cancellation & shutdown drain fence (`bridge.rs`, `registry.rs`, `AC32-MCP-SAFETY-06`, `AC32-LEAK-01`):**
     - Track active blocking SDK closures per store and enforce a bounded drain fence on `notifications/cancelled`, `run_limit_ms` timeouts, idle eviction, and `Adapter::shutdown()` so in-flight blocking writes/reads finish or drain deterministically before store close/reopen/shutdown completes, never wedging admission permits or racing reopen.

3. **Crate Hygiene (`P2-07`, `fathomdb-mcp/Cargo.toml`):**
   - Detach evaluation-only `[[example]]` targets `locomo_load` (`examples/locomo_load.rs`) and `locomo_eval` (`examples/locomo_eval/main.rs`) from `src/rust/crates/fathomdb-mcp/Cargo.toml` AND remove/move or exclude them via `package.exclude` so `cargo package --locked --allow-dirty -p fathomdb-mcp --list` contains zero `locomo_*` files (confirm zero remaining references in `scripts/` or tests), while **retaining `slice30_m_helper`** (`required-features = ["test-seams"]`, exercised by `scripts/test-slice30-m-{cli,control}.sh`) and syncing `scripts/test-feature-matrix.toml`.
   - Preserve `tests/manifest.rs` (`MR-070.2`, `rmcp = "=3.5.1"` confinement in `edge.rs` / `main.rs`).

4. **TDD Test Suites (`src/rust/crates/fathomdb-mcp/tests/`):**
   - `startup_conflicts.rs`: all 10 cases `SC-01`–`SC-10` across `--check-config` (exit `2`, empty stdout, JSON stderr, zero `.sqlite`/`.lock` created), `Config::from_json_str`, `Adapter::new`, and `Adapter::register_engine[_with_policy]`.
   - `mcp_write_safety.rs`: full `AC32-MCP-SAFETY-01`–`09` suite (read-only 7-tool default & direct write refusal, concurrent per-store write serialization, `(store, scope, operation_id)` replay across session/reopen/restart + conflict refusal, concurrent CAS `expected_write_boundary` race, two-caller `scope` isolation & anti-spoof, strict-only atomic batch rollback & bulk refusal, blocking cancellation & shutdown drain fence + permit/FD leak check `AC32-LEAK-01`, real subprocess `SIGKILL` crash/reopen integrity + receipt replay `AC32-MCP-SAFETY-07`, adversarial secret-scrubbing `AC32-MCP-SAFETY-08`, and forbidden operator escape-hatch refusal `AC32-MCP-SAFETY-09`).
   - `mcp_frozen_evidence.rs`: identical tasks run through `fathomdb-sdk` and `fathomdb-mcp` asserting exact UTF-8 byte equality, `artifact_revision_id`, caller-scope filtering, and typed drift/erasure/truncation outcomes.
   - Update existing `fathomdb-mcp` tests (`protocol.rs`, `field_census.rs`, etc.) and `dev/experiments/0.8.29-slice30-measurements/m_wire.py`.

5. **WP-40B Gate:** `cargo test -p fathomdb-mcp -- --test-threads=1`, `cargo test -p fathomdb-mcp --features default-embedder --test manifest`, `bash scripts/test-slice30-m-cli.sh`, `cargo clippy -p fathomdb-mcp --all-targets -- -D warnings`.

---

### WP-40C — 3-Channel `fathomdb-mcp` Distribution, All Release Contract Gates & MCP Docs (`Gate1-D5`, `P1-04`, `P1-05`, `P3-12`–`P3-15`, `P6-08`)

1. **`crates.io` Crate Readiness (`src/rust/crates/fathomdb-mcp/`):**
   - Flip `publish = true` in `src/rust/crates/fathomdb-mcp/Cargo.toml`, update `description` for production read+write MCP server, **keep `default = []`** (`Gate1-D5`), add byte-identical `LICENSE` (matching repo-root MIT `LICENSE`) and `README.md` (documenting `--features default-embedder`, `allow_writes: false` default, per-store opt-in + bound `scope`, and 3-channel install).
   - Verify `cargo package --locked --allow-dirty -p fathomdb-mcp --list` includes `LICENSE` and `README.md` and excludes evaluation artifacts.

2. **Lockstep PyPI Binary Wheel (`src/python-mcp/` & `src/python/pyproject.toml`):**
   - Create `src/python-mcp/` mirroring `src/python-cli/`:
     - `pyproject.toml`: `maturin>=1.9,<2`, `name = "fathomdb-mcp"`, workspace version, `bindings = "bin"`, `manifest-path = "../rust/crates/fathomdb-mcp/Cargo.toml"`, `targets = [{ name = "fathomdb-mcp", kind = "bin" }]`, `features = ["default-embedder"]`, PEP-639 `license = "MIT"`, `license-files = ["LICENSE"]` (NO `License ::` Trove classifier!), `dependencies = []`.
     - Byte-identical `LICENSE` and `README.md`.
   - In `src/python/pyproject.toml`: add optional extra `mcp = ["fathomdb-mcp==<version>"]` alongside `cli = ["fathomdb-cli==<version>"]` while keeping `[project.scripts]` strictly empty (`AC-041`!).

3. **All Release Workflows & Contract Gates Updated in Lockstep:**
   1. **`.github/workflows/release.yml`:**
      - Add 5-target `fathomdb-mcp` GH Release binary archives (`fathomdb-mcp-<version>-<target>.{tar.gz,zip}` + `.sha256`, built with `--features default-embedder`, `manylinux_2_28` glibc-checked on Linux).
      - Add `build-python-mcp` 5-target maturin wheel job (`--features default-embedder`, validated via `check_python_wheel_contents.py --kind mcp`).
      - Add **`publish-rust-t7-mcp`** in parallel with existing **`publish-rust-t7-cli`** (both `needs: publish-rust-t6-facade`), using `scripts/release/cargo-publish-if-new.sh` with pinned OIDC trusted publishing PLUS a tag-scoped **`v0.8.32` `CARGO_REGISTRY_TOKEN` initial-crate bootstrap arm** (mirroring the `v0.8.27` `publish-rust-t6-facade` precedent at `release.yml:1312` so brand-new `fathomdb-mcp` publishes on `v0.8.32` and OIDC takes over automatically on all subsequent tags!).
      - Wire `build-python-mcp`, MCP binary builds, and `publish-rust-t7-mcp` into `all-builds-passed`, `publish-pypi`, `github-release`, `wait-for-package-registry-visibility`, `co-tagging-assert`, and `post-publish-smoke` (`crates-mcp`, PyPI `fathomdb[mcp]`, GH Release stdio smoke).
   2. **`.github/workflows/promote-npm-release.yml` & `.github/workflows/docs-pages.yml`:**
      - Mirror `python-mcp-dist-*` and `mcp-*` binary artifacts into `promote-npm-release.yml:146-189` (`github-release`, per `ADR-0.8.32:78`).
      - Add `src/python-mcp/pyproject.toml` to `.github/workflows/docs-pages.yml:14-33` path triggers.
   3. **`scripts/tests/test_release_workflow_scope.sh` & `scripts/tests/test_actionlint_fixture.sh`:**
      - Update `upload-artifact-retry` count (`test_release_workflow_scope.sh:296`), `publish-rust-t7-mcp` job/tier assertions (`test_release_workflow_scope.sh:185-192`, `test_actionlint_fixture.sh:133-201`), tag-scoped `v0.8.32` token bootstrap allowlist in BOTH single-crate and fallback blocks (`test_actionlint_fixture.sh:228-253`), and `EXPECTED_RELEASE_SETUP_NODE_COUNT` / `EXPECTED_SETUP_NODE_TOTAL` (`268-270`) if changed.
   4. **`scripts/check-release-contract-truth.py` & `scripts/tests/test_release_contract_truth.sh`:**
      - Update `RELEASE_JOB_NEEDS` keeping `"publish-npm"` last in `wait-for-package-registry-visibility`; update `expected_release_condition` (`check-release-contract-truth.py:301-315`) and `release.yml` so `publish-rust-t7-mcp` carries the `((RECOVERY_ROUTE) || ...)` bypass cleanly without adding it to `v0.8.20` `RECOVERY_ALLOWED_JOBS` (and handle `co-tagging-assert` consistently for `v0.8.20` `recovery_skip_npm`).
      - Extend `check-release-contract-truth.py:456-547` to validate `build-python-mcp`, `python-mcp-dist-*`, `mcp-*` GH Release binary artifacts, and `crates-mcp` / `fathomdb[mcp]` `post-publish-smoke` checks, and update the `runs-on: ${{ matrix.runner }}` count assertion in `test_release_contract_truth.sh:287`.
   5. **`scripts/release/cargo-publish-if-new.sh` & `scripts/tests/test_cargo_publish_if_new.sh`:**
      - Add `fathomdb-mcp` to `is_dependent_crate()` (`242`) AND `usage()` (`52-56`), plus a `fathomdb-mcp` test arm in `test_cargo_publish_if_new.sh`.
   6. **`scripts/verify-release-gates.sh` & `scripts/tests/test_verify_release_gates.sh`:**
      - Add `fathomdb-mcp` to `PUBLISHABLE_CRATES` and add `src/python-mcp/pyproject.toml` to `SNAP_VERSIONED` in `test_verify_release_gates.sh:40-68` (preventing real worktree mutation during `set-version.sh` test!).
   7. **`scripts/check-license-consistency.sh` & `scripts/tests/test_check_license_consistency.sh`:**
      - Add `src/python-mcp` to `python_pkg_dirs` (`445`) AND update `mkfix()` (`test_check_license_consistency.sh:80-157`) to provision `src/python-mcp`.
   8. **`scripts/check_python_wheel_contents.py` & `scripts/tests/test_verify_release_python_wheel.sh`:**
      - Add `"mcp"` kind validating `fathomdb-mcp` binary entry + `LICENSE` and forbidding `_fathomdb*.so/.pyd` (mirroring `"cli"` kind).
   9. **`scripts/set-version.sh`, `scripts/tests/test_set_version.sh`, `scripts/tests/test_release_version_surfaces.sh`:**
      - Sync `src/python-mcp/pyproject.toml`, `src/python/pyproject.toml` `mcp = ["fathomdb-mcp==<ver>"]`, and `test_release_version_surfaces.sh:78-106` (`axis_w_packages` + `src/python-mcp/pyproject.toml` + `mcp` extra).
   10. **`scripts/release/wait-for-registry-version.py` & `scripts/tests/test_wait_for_registry_visibility.py`:**
       - Add `fathomdb-mcp` PyPI check gated on `(root / "src/python-mcp/pyproject.toml").is_file()` (matching `fathomdb-cli` at lines `87-103` so historical `v0.8.20` checks pass; crates.io polling already runs in `cargo-publish-if-new.sh`) and update `test_wait_for_registry_visibility.py:290-330`.
   11. **`scripts/generate-public-docs.py`, `scripts/tests/test_generated_public_docs.py`, & `scripts/sbom-survey/tiers.toml`:**
       - Register `src/python-mcp/` in `tiers.toml`, `generate-public-docs.py:108-123`, and `test_generated_public_docs.py:22-24, 106-130, 173-175`; regenerate `docs/generated/*` via `--write`.
   12. **`scripts/test-feature-matrix.toml` & `scripts/release/smoke/` (`test_smoke_scripts.sh`):**
       - Sync `test-feature-matrix.toml` (detached `locomo_*`, retained `slice30_m_helper`, `default-embedder` MCP coverage).
       - Add `scripts/release/smoke/smoke-crates-mcp.sh` + PyPI `fathomdb[mcp]` & GH Release stdio smoke helpers tested offline by `test_smoke_scripts.sh`.

4. **Documentation, Design Lifecycle & Interfaces (`P3-12`, `P3-14`, `P3-15`, `P6-08`):**
   - Update `AGENTS.md:31-32` (remove `publish = false` from `fathomdb-mcp`; add `src/python-mcp/` alongside `src/python-cli/`; keep `<= 300` lines and pass `check-agents-md-paths.py`).
   - Update `dev/interfaces/mcp.md`, `dev/design/mcp-adapter/{design,requirements}.md`, and sync `dev/design/document-lifecycle.json`, `dev/design/current-owner-authority.json`, and `dev/design/README.md` (`check-design-lifecycle.py`, `lint-design-status.sh`, `check-architecture-authority.py` all green!).
   - Create `docs/guides/mcp-server.md` and `docs/reference/mcp.md`, wire into `mkdocs.yml`, and update `docs/{use-cases,install,reference,operations,positions}/*` (preserving single-scope default posture and `newest published 0.8.31` pre-publish rules for `check-public-doc-truth.py`).

5. **WP-40C Gate:** `actionlint`, `bash scripts/tests/test_release_workflow_scope.sh`, `bash scripts/tests/test_actionlint_fixture.sh`, `bash scripts/tests/test_release_contract_truth.sh`, `bash scripts/tests/test_cargo_publish_if_new.sh`, `bash scripts/tests/test_verify_release_gates.sh`, `bash scripts/tests/test_check_license_consistency.sh`, `bash scripts/tests/test_verify_release_python_wheel.sh`, `bash scripts/tests/test_set_version.sh`, `bash scripts/tests/test_release_version_surfaces.sh`, `python3 scripts/tests/test_wait_for_registry_visibility.py`, `python3 scripts/tests/test_generated_public_docs.py`, `bash scripts/tests/test_smoke_scripts.sh`, `python3 scripts/check-public-doc-truth.py`, `python3 scripts/check-design-lifecycle.py`, `bash scripts/lint-design-status.sh`, `python3 scripts/check-architecture-authority.py`, `python3 scripts/check-agents-md-paths.py`, `bash scripts/agent-lint.sh`, `mkdocs build --strict`.

---

## 2. Slice 40 Closure Gate

Before closing Slice 40 (`status.md` + `merge(0.8.32): Slice 40 ...` + `release-state-0.8.32.json` / `STATUS-0.8.32.md` + push to `origin/dev/0.8.32`):

1. Full unconfined `./scripts/agent-verify.sh` on `/mnt/data/worktrees/release-0.8.32`: **EXIT `0`** (`0` failed suites, `0` security violations, `test-python` + native receipt PASS, `test-ts` PASS, `test-slice135-harness` PASS, `test-slice30-m-cli` PASS).
2. `mkdocs build --strict`: **EXIT `0`** (`0` warnings).
3. Independent adversarial code review (codex or fallback `pro` adversarial reviewer per `AGENTS.md` § 7): **`VERDICT: APPROVE`** (zero open P1 blockers).
4. **Owner interactive prerequisites reminder (prior to Slice 70 tag):** confirm PyPI Pending Publisher for `fathomdb-mcp` (`fathomadb/fathomdb`, `release.yml`, env `pypi`) and crates.io `CARGO_REGISTRY_TOKEN` `publish-new` scope for the initial `v0.8.32` publish of `fathomdb-mcp`.
