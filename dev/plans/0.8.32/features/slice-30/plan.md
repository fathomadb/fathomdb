---
title: Slice 30 Plan — 0.8.32 Opt-In Bulk Write Mode (Pin B) and Inspector Active-Scope UI
status: APPROVED
target_release: 0.8.32
slice: 30
date: 2026-11-25
---

# Slice 30 Plan — `BULK-AND-INSPECTOR`

- **Target branch / worktree:** `dev/0.8.32` at `/mnt/data/worktrees/release-0.8.32` (never mutate `/home/coreyt/projects/fathomdb`).
- **Requirements covered:** `R32-03` (opt-in bulk / non-strict write mode preserving strict defaults on `Engine::write` & `Engine::actuate`, truthful `WriteReceipt.dangling_edge_endpoints`, and `fathomdb doctor` dangling + cross-scope edge verification) and `R32-06` (Inspector `TABLE_MANIFEST`-backed admission, active-`scope` UI subordinate to `D-01` nondisclosure, and Inspector JS unit + Playwright smoke CI wiring).
- **Authoritative ADRs & rulings:**
  - `dev/adr/ADR-0.8.32-opt-in-bulk-write-mode.md` (`#100`, accepted 2026-11-24)
  - `dev/adr/ADR-0.8.32-caller-scope-isolation.md` (`#102`, accepted 2026-11-24)
  - `dev/plans/release-state-0.8.32.json` (`Gate1-D4` Pin B pre-authorization: `core = 5` and `recovery_denylist = 5` strictly invariant)
  - `dev/design/cross-cutting-change-checklist.md`
  - Slice 20 `FIX-3` review carry-forwards (`P2 #1` multi-scope SQL in `src/inspector/src/{storage,graph}.rs` and `P2 #3` Inspector READMEs).
- **Hard invariants:**
  - Default `Engine::write` (Rust, Python, TypeScript) remains strictly endpoint-validated (`EndpointValidationMode::Strict`, `dangling_edge_endpoints == 0` on every commit).
  - `Engine::actuate` (and every future MCP write path) remains strict-only and never accepts bulk/non-strict flags.
  - Under `BulkNonStrict`, caller `scope` isolation, anti-spoof stamping, cross-scope active endpoint rejection (`EngineError::ScopeViolation`), half-open validity windows, typed scalar properties, mandatory `source_id`, `InitialState`, provenance retention, triggers, and erasure invariants are **non-bypassable** and roll back atomically on violation.
  - `struct Engine` stays at exact `49` fields and root `impl Engine` stays at exact `6` methods (`test_slice90_root_reconciliation.py`); any edit to `fathomdb-engine/src/lib.rs` updates `lib_rs_blob:` in `dev/plans/0.8.27/features/slice-90/phase3-root-reconciliation.md:4` in the same commit.
  - `docs/positions/recovery-surface.md` & `ADR-0.6.0-cli-scope`: `fathomdb doctor` performs read-only diagnostic verification only (zero silent repair or drop); `AC-043a` exact `{physical, logical, semantic}` 3-key shape on `IntegrityReport` is preserved.
  - Inspector active-`scope` views remain strictly subordinate to `D-01` nondisclosure (`--no-show-inactive` and default erased-row hiding must never leak scopes whose only rows are hidden).
  - TDD first (`failing test -> red -> green -> refactor`); zero external consumer-project specifics; no `--no-verify` / `--no-gpg-sign`; no `prettier` or unguarded `markdownlint-cli2 --fix` on `.md` (`npm run format:md` only).

---

## 1. Work Package Decomposition (Sequenced Isolated Worktrees)

The 3 work packages execute sequentially in fresh isolated worktrees cut off `dev/0.8.32` under `/mnt/data/worktrees/` (one writer per checkout; merge & verify each package cleanly before cutting the next).

---

### WP-30A — Engine Opt-In Bulk Write (`WriteOptions` / `write_with_options`) & `fathomdb doctor` Edge Verification (`R32-03`, `P4-08`, `P5-08`)

1. **Public Bulk-Write Types & Engine Entrypoint (`fathomdb-engine`):**
   - In `src/rust/crates/fathomdb-engine/src/write.rs`:
     - Add `#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)] pub enum EndpointValidationMode { #[default] Strict, BulkNonStrict }`.
     - Add `#[non_exhaustive] #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)] pub struct WriteOptions { pub endpoint_validation: EndpointValidationMode }` (matching `fathomdb-sdk/src/options.rs:73-75` `#[non_exhaustive]` convention) with constructors `WriteOptions::strict()`, `WriteOptions::bulk_non_strict()`, and `WriteOptions::with_endpoint_validation(self, mode: EndpointValidationMode) -> Self`.
     - Add `pub fn write_with_options(&self, batch: &[PreparedWrite], options: &WriteOptions) -> Result<WriteReceipt, EngineError>` on `impl Engine` in **`write.rs`** (NEVER in `lib.rs`!), preserving the full existing `Engine::write` instrumentation envelope (`ensure_open()`, `EventCategory::{Writer, Admin}`, `emit_event` `Started`/`Finished`/`Failed`/`Error`, `counters.record_write`/`record_error`, `detect_slow`, and atomic rollback BEFORE `self.next_cursor.store(last_cursor, Ordering::SeqCst)` at `write.rs:519`).
     - Delegate `Engine::write(&self, batch)` directly to `self.write_with_options(batch, &WriteOptions::default())`.
     - Update stale `WriteReceipt::dangling_edge_endpoints` docstring in `write.rs:13-19` to document `Strict` (`0` on every commit) vs. `BulkNonStrict` (truthful `0`/`1`/`2` per surviving active edge written in the batch).
   - In `src/rust/crates/fathomdb-engine/src/lib.rs`:
     - Re-export `EndpointValidationMode` and `WriteOptions` from `write`.
     - Recompute and update `lib_rs_blob:` in `dev/plans/0.8.27/features/slice-90/phase3-root-reconciliation.md:4` in the same commit (`test_slice90_root_reconciliation.py` green; `struct Engine` stays at `49` fields).
   - In `src/rust/crates/fathomdb-engine/src/write_commit.rs`:
     - Rename internal `pub(crate) enum EndpointValidationMode` (`write_commit.rs:57`) to `CommitEndpointValidationMode { Strict, BulkNonStrict, DeferredToActuationSimulation }` (also updating `lib.rs:468` `pub(crate) use write_commit::{...}`) so `DeferredToActuationSimulation` never escapes to the public API and `actuation.rs` (`Engine::actuate`) stays strictly validated (`DeferredToActuationSimulation` -> `validate_prospective_edge_endpoints`).
     - Thread `CommitEndpointValidationMode` from `write.rs` through `commit_batch_transaction` into **both** `apply_batch_in_transaction` call sites (`write_commit.rs:602, 636`).
     - **Exact `BulkNonStrict` endpoint accounting & index-backed cross-scope refusal:**
       - **Zero-SQL fast path for node-only / op-store batches:** Check in memory first whether `batch` contains any `PreparedWrite::Edge` or `PreparedWrite::ProvenancedEdge`. If none exist, execute **zero** endpoint SQL and return `dangling_edge_endpoints = 0`.
       - Only when edges exist in `batch`, query surviving active edges written by this batch (`SELECT from_id, to_id FROM canonical_edges WHERE scope = ?1 AND write_cursor BETWEEN ?2 AND ?3 AND superseded_at IS NULL`) so same-batch edge supersession counts only surviving active edges. If none survive, return `dangling_edge_endpoints = 0`.
       - Collect distinct endpoint `logical_id`s across surviving active edges and query active same-scope nodes (`SELECT logical_id FROM canonical_nodes WHERE scope = ?1 AND logical_id IN (...) AND superseded_at IS NULL AND state = 'active'`, chunked at `64`, hitting `canonical_nodes_scope_logical_active_idx`).
       - Compute the subset `missing` of distinct endpoints absent from `present_in_scope`.
       - **Index-backed atomic cross-scope endpoint check (`ScopeViolation`, zero full-table scan):** ONLY when `!missing.is_empty()`:
         1. Run O(log N) index check `SELECT EXISTS(SELECT 1 FROM canonical_nodes WHERE scope < ?1 AND superseded_at IS NULL AND state = 'active' LIMIT 1) OR EXISTS(SELECT 1 FROM canonical_nodes WHERE scope > ?1 AND superseded_at IS NULL AND state = 'active' LIMIT 1)` (uses leading `scope` key on `canonical_nodes_scope_state_active_idx` / `canonical_nodes_scope_logical_active_idx`). On single-scope stores (the default), this returns `0` in O(log N) and skips cross-scope probing cleanly!
         2. When other scopes exist, iterate distinct non-caller active scopes via O(log N) index jumps (`SELECT scope FROM canonical_nodes WHERE scope > ?1 AND superseded_at IS NULL AND state = 'active' ORDER BY scope LIMIT 1`, starting from `None` -> `ORDER BY scope LIMIT 1` and skipping `scope`) and probe each other scope with `SELECT 1 FROM canonical_nodes WHERE scope = ?1 AND logical_id IN (...) AND superseded_at IS NULL AND state = 'active' LIMIT 1` (chunked at `64`, hitting `canonical_nodes_scope_logical_active_idx`!). If any match exists, abort the transaction atomically with `CommitBatchError::Engine(EngineError::ScopeViolation)` (never admitting a cross-scope endpoint as "dangling"!).
       - Note on `Strict` mode: retain `validate_strict_batch_edge_endpoints` atomic rejection (`WriteValidation` on missing/inactive/superseded/cross-scope endpoints, preserving `0.8.31` error compatibility).
       - Sum `dangling_edge_endpoints` across surviving active edges: for each `(from_id, to_id)`, add `(!present_in_scope.contains(from_id)) as u64 + (!present_in_scope.contains(to_id)) as u64` (`0`, `1`, or `2` per edge).
       - All other invariants (writable `ScopePolicy`, opener-bound `scope` stamping, same-scope ownership, valid windows, typed properties, mandatory `source_id`, `InitialState`, provenance, triggers, erasure) remain 100% strict and roll back atomically on violation.
2. **`fathomdb doctor` Dangling & Cross-Scope Edge Verification (`fathomdb-engine` + `fathomdb-cli`):**
   - **Data-plane integrity (`src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs`):**
     - Add `DataPlaneIntegrityCheckV1::EdgeEndpointIntegrity` (`"edge_endpoint_integrity"`) as the 5th closed check; update `DataPlaneIntegrityRequestV1::all` (`data_plane_integrity.rs:732-743`) to include it in canonical order.
     - Add finding codes `DataPlaneIntegrityFindingCodeV1::DanglingEdgeEndpoint` (`"dangling_edge_endpoint"`, `Severity::Error`) and `DataPlaneIntegrityFindingCodeV1::CrossScopeEdgeEndpoint` (`"cross_scope_edge_endpoint"`, `Severity::Critical`).
     - Keep `DataPlaneIntegrityFindingV1` (`data_plane_integrity.rs:645-654`) at its exact 8-field content-free schema (`write_cursor: Some(edge_cursor)`, `artifact_revision_ids: vec![]`, `code`, `severity` — zero payload/body/identity leakage!).
     - Implement read-only SQL check over active edges (`e.superseded_at IS NULL` — `canonical_edges` has no `state` column), charging `take_work(aggregate_checked, max_work_units)?` per scanned edge / probe and reusing the O(log N) distinct-scope index-jump probe for cross-scope classification:
       1. Active edge `from_id` or `to_id` missing an active same-scope node AND matching an active node in a distinct scope -> emit `CrossScopeEdgeEndpoint` (`Critical`).
       2. Active edge `from_id` or `to_id` missing an active same-scope node AND NOT matching a cross-scope active node -> emit `DanglingEdgeEndpoint` (`Error`).
     - Update `candidate_queries_for_test` / `data_plane_integrity_candidate_queries_for_test` (`data_plane_integrity.rs:163, 2931, 2943`) array length & 2-`i64` parameter arity cleanly so `data_plane_integrity_query_plans_for_test` stays green.
   - **Operator `check_integrity` (`src/rust/crates/fathomdb-engine/src/operator.rs` + `dev/design/recovery.md`):**
     - Extend `semantic_section(connection)` (preserving `AC-043a` exact `{physical, logical, semantic}` 3-key shape on `IntegrityReport`!) to run the same read-only dangling and cross-scope active-edge checks, appending `operator::Finding` (`operator.rs:193-199`: `code`, `stage: "semantic"`, content-free `locator`, `doc_anchor`, `detail`) with codes `"dangling_edge_endpoint"` and `"cross_scope_edge_endpoint"` anchored to `design/recovery.md#dangling-edge-endpoint` and `design/recovery.md#cross-scope-edge-endpoint`.
     - Register both anchors and entries in `dev/design/recovery.md` § `Code-to-operator-action cross-reference` (`recovery.md:181-241`) so `scripts/agent-lint-md.sh` anchor verification passes cleanly.
   - **CLI (`src/rust/crates/fathomdb-cli/src/lib.rs`):**
     - Update the `--check` parser (`src/rust/crates/fathomdb-cli/src/lib.rs:832-846`) to accept `"edge_endpoint_integrity"` -> `DataPlaneIntegrityCheckV1::EdgeEndpointIntegrity`.
     - Verify `fathomdb doctor data-plane-integrity --json <db>` (default all 5 checks and `--check edge_endpoint_integrity`) and `fathomdb doctor check-integrity --json <db>` exit `65` (`DOCTOR_FOUND_ISSUES`) on dangling/cross-scope findings and `0` on clean stores with zero DB mutation.
3. **TDD Tests (`WP-30A`):**
   - `src/rust/crates/fathomdb-engine/tests/bulk_write_opt_in.rs`:
     - Default `Engine::write` rejects missing/inactive/superseded/cross-scope endpoints atomically with `WriteValidation` and `dangling_edge_endpoints == 0`.
     - `Engine::actuate` stays strict-only.
     - `Engine::write_with_options(..., &WriteOptions::bulk_non_strict())`:
       - Admits out-of-order edges (0, 1, or 2 missing endpoints), returning exact `WriteReceipt.dangling_edge_endpoints`.
       - Node-only batch returns `0` on the zero-SQL fast path.
       - Same-batch node + edge resolves (`0`); same-batch edge supersession counts only surviving active edges.
       - Subsequent batch writing missing same-scope nodes resolves edges during `graph_expand` / `search` without rewriting edges.
       - Cross-scope active endpoint is atomically refused with `EngineError::ScopeViolation` (entire batch rolled back, DB untouched).
       - Inactive (`pending`/`deleted`/`purged`) or superseded same-scope node endpoint is counted as dangling under `BulkNonStrict`.
       - Invalid validity window / invalid property / bad `InitialState` still roll back atomically under `BulkNonStrict` without advancing `next_cursor`.
       - `ScopePolicy::read_only(...)` refuses `write_with_options` with `ScopeViolation`.
   - `src/rust/crates/fathomdb-cli/tests/doctor_dangling_edges.rs`:
     - Clean store -> `fathomdb doctor data-plane-integrity --json` (and `--check edge_endpoint_integrity`) and `check-integrity --json` exit `0` with `0` findings.
     - Bulk-ingested store with unresolved dangling edges -> both commands exit `65` with deterministic `dangling_edge_endpoint` JSON findings and zero DB mutation; after missing nodes arrive in a second batch, both return exit `0`.
     - Store with raw-SQL-injected cross-scope edge anomaly -> both commands exit `65` with `cross_scope_edge_endpoint` JSON finding and zero payload leakage.
4. **WP-30A Gate:** `cargo test -p fathomdb-engine --test bulk_write_opt_in`, `cargo test -p fathomdb-cli --test doctor_dangling_edges`, `cargo test -p fathomdb-engine data_plane_integrity`, `cargo clippy --workspace --all-targets -- -D warnings`, `python3 scripts/tests/test_slice90_root_reconciliation.py`, `python3 scripts/tests/test_slice90_data_plane_integrity_owner.py`, `python3 scripts/tests/test_slice90_operator_integrity_owner.py`, `bash scripts/check-module-boundaries.sh`, `bash scripts/agent-lint-md.sh`.

---

### WP-30B — `Pin B` Multi-Language SDK Parity, Governed Surface Pin & Docs/Interfaces (`R32-03`, `X1`, `X4`, `P3-03`, `P3-10`, `P6-07`)

1. **Rust Facade & SDK (`fathomdb`, `fathomdb-sdk`):**
   - `src/rust/crates/fathomdb/src/lib.rs` & `tests/reexports.rs`: re-export `EndpointValidationMode`, `WriteOptions`.
   - `src/rust/crates/fathomdb-sdk/src/lib.rs`, `src/rust/crates/fathomdb-sdk/src/engine.rs`, `tests/surface.rs`:
     - Re-export `EndpointValidationMode`, `WriteOptions` from `fathomdb_sdk`.
     - Keep `fathomdb_sdk`'s exact 3 namespaces (`pub mod admin; pub mod graph; pub mod read;` — NO `pub mod write`!).
     - Add `pub fn write_with_options(&self, batch: &[PreparedWrite], options: &WriteOptions) -> Result<WriteReceipt>` on `fathomdb_sdk::Engine` in `src/rust/crates/fathomdb-sdk/src/engine.rs` (running `guard::prepared_write` on every item in `batch`, returning `fathomdb_sdk::Result<WriteReceipt>`, and keeping `Engine::write` strictly validated).
     - Add function-pointer surface pin for `Engine::write_with_options` plus `EndpointValidationMode`/`WriteOptions` assertions in `src/rust/crates/fathomdb-sdk/tests/surface.rs`.
2. **Python Bindings (`fathomdb-py`, `src/python/`):**
   - `src/rust/crates/fathomdb-py/src/engine.rs`: expose `write_with_options(batch, endpoint_validation="strict")` on the native engine (validating type -> `TypeError`, unknown string literal -> `ValueError` matching `scope_policy`/`query_stop_words` at `engine.rs:151-189, 215-219`, mapping `ScopeViolation` -> `ScopeViolationError` and `WriteValidation` -> `WriteValidationError`).
   - `src/python/fathomdb/`:
     - Keep `_sdk_write.py` private (do NOT add a public `fathomdb.write` module; `__init__.py` keeps `admin, errors, graph, read`).
     - Add `EndpointValidationMode = Literal["strict", "bulk_non_strict"]` and `@dataclass(frozen=True) class WriteOptions: endpoint_validation: EndpointValidationMode = "strict"` exported from `fathomdb` and `fathomdb.types`.
     - Add `Engine.write_with_options(self, batch, options=None) -> WriteReceipt` in `src/python/fathomdb/engine.py` (delegating to private `_sdk_write.py`; accepting `None` -> strict default or `WriteOptions`, raising `TypeError` for non-`WriteOptions` and `ValueError` for unknown `endpoint_validation` literal).
     - Update stale `WriteReceipt.dangling_edge_endpoints` docstring in `src/python/fathomdb/types.py`.
     - Update `src/python/fathomdb/_fathomdb.pyi`, `src/python/tests/python_surface_additions.json` (new `Engine.write_with_options`, `WriteOptions`, `EndpointValidationMode` entries + updated `native_stub_sha256` if verified by `test_slice130_package_boundary.py`), and `src/python/tests/test_slice130_package_boundary.py`.
   - Rebuild owned `.venv` wheel via `bash scripts/setup-python-owned.sh --build`.
   - Add Python TDD parity tests (`src/python/tests/test_bulk_write_opt_in.py`): strict default refusal, `BulkNonStrict` dangling counts + subsequent resolution, cross-scope `ScopeViolationError`, invalid type (`TypeError`) & unknown literal (`ValueError`).
3. **TypeScript Bindings (`fathomdb-napi`, `src/ts/`):**
   - `src/rust/crates/fathomdb-napi/src/`: expose `writeWithOptions(batch, options)` native binding validating `"strict"` / `"bulk_non_strict"` (mapping `ScopeViolation` -> `ScopeViolationError`).
   - `src/ts/src/`:
     - Keep `src/ts/src/write.ts` private (do NOT export a public `write` namespace from `index.ts`).
     - Export `EndpointValidationMode` (`"strict" | "bulk_non_strict"`) and `WriteOptions` (`{ readonly endpointValidation?: EndpointValidationMode }`) from `src/ts/src/index.ts`.
     - Add `Engine.writeWithOptions(batch, options?)` on `Engine` in `src/ts/src/core.ts` (delegating to `src/ts/src/write.ts`, throwing `TypeError` on invalid `endpointValidation` literal matching `src/ts/src/core.ts:209, 321`).
     - Update stale `danglingEdgeEndpoints` docstring in `src/ts/src/write.ts`.
     - Rebuild native addon & `dist/`.
   - Add TS TDD parity tests (`src/ts/tests/bulk-write-opt-in.test.ts`): strict default refusal, `BulkNonStrict` dangling counts + subsequent resolution, cross-scope `ScopeViolationError`, invalid option `TypeError`.
4. **Governed Surface Pin (`Gate1-D4` Pin B Pre-Authorized; `core = 5`, `recovery_denylist = 5` Strictly Invariant):**
   - Update in lockstep:
     - `src/conformance/governed-operation-parity.json`: `56 -> 57` operations — add `"id": "engine.write_with_options"`, `"locator": "engine_instance"`, Rust `"write_with_options"`, Python `"write_with_options"`, TypeScript `"writeWithOptions"`.
     - `src/conformance/governed-surface-allowlist.json`: `93 -> 95` members (add `"write_with_options"` and `"writeWithOptions"`, keeping `core = 5` and `recovery_denylist = 5` untouched!).
     - `scripts/governed-surface-pin.json`: recompute exact `sha256`, `git_blob_sha1`, counts, and member lists with `Gate1-D4 Pin B (0.8.32 Slice 30)` provenance note.
     - Update hardcoded count assertions in ALL parity/pin/doc tests:
       - `scripts/tests/test_check_governed_surface_pin.sh`
       - `scripts/tests/test_check_sdk_surface_parity.py`
       - `src/ts/tests/sdk-surface-parity.test.ts:98` (`56/56 -> 57/57`)
       - `scripts/tests/test_generated_public_docs.py`
       - `dev/interfaces/rust-sdk.md:40` (`56/56 -> 57/57`)
     - Regenerate `docs/generated/sdk-operation-map.md` via `.venv/bin/python scripts/generate-public-docs.py --write` and verify `--check` passes.
5. **Interfaces & Reference Documentation (`X4`):**
   - Update `dev/interfaces/{rust,rust-sdk,python,typescript,cli}.md` (including `dev/interfaces/cli.md:516` "five checks in canonical order") and `docs/reference/{rust-sdk,rust-api,python-api,typescript-api,errors,cli}.md` with `WriteOptions`, `EndpointValidationMode`, `Engine::write_with_options` / `Engine.write_with_options` / `Engine.writeWithOptions`, truthful `dangling_edge_endpoints` semantics, and `fathomdb doctor` dangling/cross-scope edge findings (removing stale `"unconditionally 0"` / `"informational FLAG-AND-COUNT"` wording).
6. **WP-30B Gate:** `bash scripts/check-governed-surface-pin.sh`, `python3 scripts/check-sdk-surface-parity.py`, `bash scripts/tests/test_check_governed_surface_pin.sh`, `python3 scripts/tests/test_check_sdk_surface_parity.py`, `python3 scripts/tests/test_generated_public_docs.py`, `.venv/bin/python scripts/generate-public-docs.py --check`, `cargo test -p fathomdb-sdk --test surface`, `cargo test -p fathomdb --test reexports`, `cd src/ts && npm test`, Python pytest on `test_bulk_write_opt_in.py` + `test_slice130_package_boundary.py`, `bash scripts/agent-lint-md.sh`, `bash scripts/agent-lint-docs.sh`, `mkdocs build --strict`.

---

### WP-30C — Inspector Active-`scope` UI (`TABLE_MANIFEST`-Backed), Carry-Forward Multi-Scope SQL & Playwright CI Wiring (`R32-06`, `P4-09`, `P3-11`, `P5-09`, `P6-07`)

1. **`TABLE_MANIFEST`-Backed Schema-39 Admission (`src/inspector/src/admission.rs`):**
   - Replace the hardcoded 35-table inventory in `src/inspector/src/admission.rs` with `fathomdb_schema::TABLE_MANIFEST` (`fathomdb-schema` is already a dependency of `fathomdb-inspector`) so Inspector admission automatically validates all Schema-39 tables from the single source of truth while retaining column checks and the 8-col `vec0` (`scope TEXT`) `malformed_vector_shape` guard.
2. **Multi-Scope SQL, `Session` Engine Scope Switching, & D-01-Safe Scope Filtering (`src/inspector/src/`):**
   - **Resolve ALL Slice 20 `FIX-3` review P2 #1 & multi-scope join sites:**
     - `storage.rs:1367-1368`: qualify `Reader::associations` joins on `a.scope = l.scope` and bind the root row's `scope`.
     - `graph.rs:672-687`: update `Reader::revision_locator` to take `scope: &str` (`WHERE scope=?1 AND revision_id=?2`, and `WHERE scope=?1 AND write_cursor=?2` at `687`) across **all three** callers (`graph.rs:768`, `1022`, `1028`).
     - `graph.rs:832`: qualify `Reader::graph_admission` join with `a.scope = canonical_nodes.scope`.
     - `graph.rs:198, 335, 348, 408, 606-607, 728`: bind `scope` on `Reader::graph_scan` and `expansion_locations` so colliding `(scope, logical_id)` / `(scope, revision_id)` rows across scopes never cross-match.
     - `storage.rs:1031, 1104, 1139-1145` (`catalog`), `lexical.rs:252` (`lexical_origins`), and `vector.rs:271` (`neighborhood`): enforce `scope` equality on `_fathomdb_artifact_revisions` / `canonical_*` joins and honor optional `scope` filter.
   - **Add `#[serde(default)] scope: Option<String>` to `Filter` (`storage.rs:121`) and `worker::Request::Catalog` (`worker.rs:23`), plus optional `scope` on `lexical` / `serving` requests:**
     - **Index-backed `page` query strategy (honoring actual Schema-39 indexes):**
       - `canonical_nodes` and `canonical_edges`: use Schema-39 composite `(scope, ...)` indexes (`EXPLAIN QUERY PLAN` `SEARCH`, zero `SCAN`).
       - `operational_state` and `operational_mutations`: when `filter.collection` is set, use `operational_state_scope_collection_cursor_page_idx(scope, collection_name, write_cursor)` / `operational_mutations_scope_collection_id_idx(scope, collection_name, id)` (`SEARCH`, zero `SCAN`); when `filter.collection` is `None` and `filter.scope` is set, use the existing `write_cursor`/`rowid` index `SEARCH` (`WHERE {ordering} > ?1 AND scope = ?2 ORDER BY {ordering}`) with zero `TEMP B-TREE` and zero full table `SCAN` (matching `storage.rs:684-686`).
     - Ensure list/summary rows (`page` items) carry `scope` cleanly (`detail` already includes `record.scope` at `storage.rs:81,94,104` & `app.js:958-967`).
   - **Explicit `Session` Engine-Scope Switching for Native Serving (`session.rs`, `lexical.rs`, `vector.rs`, `graph.rs`):**
     - `Engine`'s `ScopePolicy` is immutable after `Engine::open` (`Unscoped` binds `scope = ""` ONLY; `ScopePolicy::read_only(vec![s])` requires non-empty `s`).
     - Store `engine: Option<Engine>` and `engine_scope: String` (initially `""` -> `ScopePolicy::Unscoped`) on `Session` (`src/inspector/src/session.rs`), plus helper `Session::ensure_engine_scope(&mut self, target_scope: &str) -> Result<(), Failure>`:
       - If `self.engine_scope == target_scope` and `self.engine.is_some()`, no-op (zero overhead on single-scope `""` stores and repeated queries in the same scope!).
       - Otherwise, safely `take()` and close/drop the prior `Engine` handle first (releasing its writer lock cleanly without leaving moved-out state on reopen failure), reopen with `ScopePolicy::Unscoped` if `target_scope.is_empty()` or `ScopePolicy::read_only(vec![target_scope.to_string()])` if non-empty (preserving `cached_cpu` & `EngineConfig` runtime limits), restore `self.engine = Some(opened.engine)` and `self.engine_scope`, and verify fingerprint/lock invariants.
     - Route native `Engine` operations cleanly (strictly `D-01`-oblivious):
       - `graph_serving`: always calls `ensure_engine_scope(&seed_record.scope)` so native graph expansion matches the selected record's exact scope.
       - `lexical` and `serving`: when an explicit `scope: Some(s)` is supplied, call `ensure_engine_scope(&s)`. When `scope` is `None`, handle uniformly via `ensure_engine_scope("")` (`Unscoped`, `"engine_scope": ""`, 100% backwards-compatible on single-scope stores and strictly `D-01`-oblivious to any hidden non-empty scopes).
   - **D-01-Safe Per-Scope `catalog()` Discovery:**
     - Add `scopes: Vec<ScopeSummary>` to `catalog()` response (reporting each visible scope name and its visible counts across admitted classes, strictly filtered by the active disclosure policy `policy.show_inactive` / `policy.show_erased`).
     - **Non-disclosure invariant (`D-01`):** A scope containing ONLY inactive/deleted/erased rows MUST NOT appear in `catalog().scopes`, counts, or UI dropdowns when `--no-show-inactive` (or default erased hiding) excludes those rows.
     - When `Request::Catalog { scope: Some(s) }` is supplied, scope the returned class counts to `s` while still returning the visible `scopes` list (so the UI switcher stays populated).
3. **Inspector UI (`src/inspector/assets/app.js`, HTML/CSS):**
   - Add an active-`scope` switcher/selector in the Inventory view and persistent scope filter control across list, lexical, serving, and graph views:
     - Shows discovered scopes from `catalog().scopes` (single-scope `""` stores stay uncluttered; multi-scope stores display per-scope counts and allow switching active scope across views).
     - Surface `scope` badges on multi-scope list/summary rows (`detail` already renders `record.scope` at `app.js:958-967`).
     - Preserve all existing accessible labels, keyboard navigation, refresh/back state, and pin/comparison workflows (zero regression on `unit/{comparison,ux}.mjs` and existing browser tests).
4. **Inspector Tests, Multi-Scope Fixture & Docs (`P3-11`, `P5-09`, `P6-07`, Slice 20 `FIX-3` P2 #3):**
   - Extend `src/inspector/examples/fixture.rs` with a `"scoped"` multi-scope fixture mode (creating default `""`, `"alpha"`, `"beta"` with colliding `logical_id`s across `"alpha"` & `"beta"`, plus a `"hidden-inactive"` scope containing ONLY inactive/pending/deleted rows for `D-01` proof!).
   - Add Rust integration tests in `src/inspector/tests/` verifying:
     - `TABLE_MANIFEST`-backed admission;
     - multi-scope `catalog` per-scope counts and `D-01` nondisclosure (`"hidden-inactive"` absent under `--no-show-inactive`, visible only when inactive rows are shown);
     - scoped `page` (+ `EXPLAIN QUERY PLAN` checks matching the exact Schema-39 index strategy above), `detail`, `associations`, `lexical`, `vector`, `graph`, and `serving` with colliding `logical_id` across `"alpha"` vs. `"beta"` (zero cross-scope leakage!).
   - Add Playwright browser test `src/inspector/tests/browser/scope.mjs` verifying the active-`scope` UI switcher, scope filtering across inventory/list/detail/graph, and `D-01` hiding of `"hidden-inactive"` under `--no-show-inactive`. Register `scope.mjs` in `src/inspector/tools/local-ci.sh:254-259` and run `api.mjs`, `ux.mjs`, and `scope.mjs` live locally against host Chromium (`chromium-1248`) + Playwright!
   - Update `src/inspector/{README.md,package/README.md,tools/README.md}`, `dev/inspector/*`, and `docs/inspector/*` for Schema `39` (refusing `38` and older) and active-`scope` UI subordinate to `D-01` (note: updating `src/inspector/package/README.md` is part of the Inspector source commit; sealed release candidates in Slice 60/70 are built fresh from the updated HEAD).
5. **CI Wiring (`.github/workflows/ci.yml` + `scripts/tests/test_ci_proportional_routing.py`, `P5-09`):**
   - Add an `inspector-browser-smoke` job in `.github/workflows/ci.yml` using the exact `expected_scoped_condition` format asserted by `scripts/tests/test_ci_proportional_routing.py:267-278, 450-452`:
     - `!cancelled() && needs.changes.result == 'success' && ( needs.changes.outputs.release_qualification == 'true' || needs.changes.outputs.ci_workflow == 'true' || ( needs.changes.outputs.docs_only != 'true' && ( needs.changes.outputs.inspector == 'true' ) && needs.changes.outputs.ci_mode != 'lite' ) )`
   - Runs on `ubuntu-latest` (`timeout-minutes: 25`), executing:
     1. `python3 -m unittest discover -s src/inspector/tests/tools -p 'test_*.py'`
     2. `node --test src/inspector/tests/unit/comparison.mjs src/inspector/tests/unit/ux.mjs`
     3. `cargo build --locked -p fathomdb-inspector --bins --examples`
     4. Installs exact pinned Playwright (`playwright@<exact-version-matching-host>`) into `"$RUNNER_TEMP/inspector-pw"` (`npm --prefix "$RUNNER_TEMP/inspector-pw" install --no-save playwright@<exact-version>` + `npx --prefix "$RUNNER_TEMP/inspector-pw" playwright install --with-deps chromium`, zero new `package.json`/`package-lock.json` inside `src/inspector/`, zero `src/ts` or `sbom-survey/tiers.toml` drift, zero `setup-node` `cache-dependency-path` conflict!), sets `INSPECTOR_PLAYWRIGHT="$RUNNER_TEMP/inspector-pw/node_modules/playwright/index.mjs"` and derives `INSPECTOR_CHROMIUM` dynamically via `node -e "console.log(require('$RUNNER_TEMP/inspector-pw/node_modules/playwright').chromium.executablePath())"`, and runs `node --test --test-concurrency=1 src/inspector/tests/browser/api.mjs src/inspector/tests/browser/ux.mjs src/inspector/tests/browser/scope.mjs`.
   - Update `scripts/tests/test_ci_proportional_routing.py` (`SCOPED_DRIVERS` + `inspector-*` expected job sets) in the same commit!
   - Verify ALL CI & Inspector tooling contract tests stay 100% green: `actionlint`, `scripts/tests/test_ci_proportional_routing.py`, `scripts/tests/test_verify_ci_timeout_budget.sh`, `scripts/tests/test_ci_long_job_efficiency.sh`, `scripts/tests/test_ci_skip_message_warning.sh`, `scripts/tests/test_paths_filter_oracle.sh`, `src/inspector/tests/tools/test_local_release.py`, `src/inspector/tests/tools/test_local_ci.py` (including D4-B static guard).
6. **WP-30C Gate:** `cargo test -p fathomdb-inspector -- --test-threads=1`, `python3 -m unittest discover -s src/inspector/tests/tools -p 'test_*.py'`, `node --test src/inspector/tests/unit/comparison.mjs src/inspector/tests/unit/ux.mjs`, live local Playwright run (`api.mjs`, `ux.mjs`, `scope.mjs`), `actionlint`, all `scripts/tests/test_ci_*` + `test_paths_filter_oracle.sh`, `bash scripts/agent-lint-md.sh`, `bash scripts/agent-lint-docs.sh`, `mkdocs build --strict`.

---

## 2. Slice 30 Closure Gate

Before closing Slice 30 (`status.md` + `merge(0.8.32): Slice 30 ...` + `release-state-0.8.32.json` / `STATUS-0.8.32.md` + push to `origin/dev/0.8.32`):

1. Full unconfined `./scripts/agent-verify.sh` on `/mnt/data/worktrees/release-0.8.32`: **EXIT `0`** (`0` failed suites, `0` security violations, `test-python` + native receipt PASS, `test-ts` PASS).
2. Live local Inspector Playwright run (`api.mjs`, `ux.mjs`, `scope.mjs`) + Inspector JS unit + Inspector tooling tests: **EXIT `0`**.
3. `mkdocs build --strict`: **EXIT `0`** (`0` warnings).
4. Independent `pro` adversarial code review (`slice-reviewer`): **`VERDICT: APPROVE`** (zero open P1 blockers).
