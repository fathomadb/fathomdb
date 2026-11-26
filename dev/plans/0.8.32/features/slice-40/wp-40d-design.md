---
title: WP-40D Design — Pooled-Reader Projection-Generation Status Cache & PerHitExplain.vector_cosine
status: APPROVED
target_release: 0.8.32
slice: 40
date: 2026-11-25
---

# WP-40D Design — `Gate1-D10` (`R32-08`)

- **Governing decision & requirements:** `Gate1-D10` Option 2 (`dev/plans/release-state-0.8.32.json`), `R32-08` (`dev/plans/0.8.32/plan.md`), `AC32-PERF-STATUS-01..03` and `AC32-EXPLAIN-COSINE-01..04` (`dev/plans/0.8.32/features/slice-40/plan.md`).
- **Architecture & ADR alignment:** `dev/design/fathomdb-data-plane-architecture-v2.md`, `dev/design/engine.md`, `dev/design/retrieval.md`, `dev/design/0.8.8-explain-and-telemetry-adr.md` (§A.4 Q1 extended specifically for `PerHitExplain.vector_cosine` while keeping `EP-13-raw-arm-scores.md` `vector_distance`/`text_bm25` deferred), `ADR-0.6.0-vector-identity-embedder-owned.md`, and `ADR-0.8.28-vector-compatibility-open-policy` (no custom production provider admission restored).
- **Non-breaking invariants:**
  - `struct Engine` retains exactly **49** fields and **6** root `impl Engine` methods (`lib_rs_blob:` updated in `dev/plans/0.8.27/features/slice-90/phase3-root-reconciliation.md:4` in the same commit).
  - Zero module-boundary violations (`dev/tools/module-boundary-policy.txt`: `search`, `graph_expand`, and `reader_pool` depend on `projection_generation`; `projection_generation` has zero dependency on `reader_pool`).
  - Zero Schema 39 DDL change, zero `SearchHit` change, zero `EvidenceContributionV1` (`evidence/codec.rs`) wire change, zero `fathomdb-mcp` `PerHitOut` change, and zero `scripts/governed-surface-pin.json` change (`58` ops / `97` allowlist / `core = 5` / `recovery_denylist = 5`; confirmed `check-sdk-surface-parity.py:553-611` and `check-governed-surface-pin.sh` do not inspect `PerHitExplain` fields).

---

## 1. Part A — Unified Snapshot Boundary & Shared `ProjectionGenerationStatusCache` (`AC32-PERF-STATUS-01..03`)

### 1.1 Problem Statement

Currently:

- `Engine::read_projection_generation_status_at` (`projection_generation.rs:1789`) and `Engine::read_mutation_projection_status` (`projection_generation.rs:1964`) call `Engine::cached_status_in_snapshot` (`projection_generation.rs:1705-1770`) on `Engine.connection`, storing a single `CachedProjectionGenerationStatus` keyed on `PRAGMA data_version`.
- `read_search_work_in_tx` (`search/fusion_stage.rs:626-637` — invoked by `search(..., explain=true)`, `search_frozen(..., explain=true)`, and `search_with_evidence`) and `read_graph_expand_with_stop_words_in_tx` (`graph_expand/execution.rs:1346, 1402-1408` — invoked by query-seeded `graph.expand` at `reader_pool.rs:1001`) execute inside `ReaderWorkerPool` threads and call **uncached** `projection_generation::status_in_snapshot` on every query.
- When a vector projection or `_fathomdb_vector_kinds` entry is declared, `physical_completion` performs an `O(N)` completion scan over `canonical_nodes` / `canonical_edges` / `vector_default` / `_fathomdb_vector_spans` on every query (~200 ms at 10k nodes, ~3.3 s at 100k nodes), even when opened with `use_default_embedder = false`.
- Additionally, `read_projection_generation_status_at` and `read_mutation_projection_status` pass `authoritative_boundary(&tx)?` (`projection_generation.rs:330-370`, which omits `physical_erasure::cursor_floor`), while `fusion_stage.rs:633` and `graph_expand/execution.rs:1407` pass `load_next_cursor(&tx)` (which includes `_fathomdb_projection_terminal` and `physical_erasure::cursor_floor`).

### 1.2 Unified Snapshot Boundary Helper

Define one shared helper in `fathomdb-engine/src/projection_generation.rs`:

```rust
pub(crate) fn snapshot_observed_boundary(connection: &Connection) -> Result<u64, EngineError> {
    let base = authoritative_boundary(connection)?;
    let floor = crate::physical_erasure::cursor_floor(connection)?;
    Ok(base.max(floor))
}
```

- **Why `_fathomdb_projection_terminal` is excluded here:** `authoritative_boundary` covers committed canonical/operational rows, `_fathomdb_commit_timestamps`, Step-23 reserved cursor, and `_fathomdb_dependency_closures`, and folding `physical_erasure::cursor_floor` preserves monotonicity across physical erasure. Excluding `_fathomdb_projection_terminal` ensures corrupt orphan terminal rows above the authoritative boundary cannot artificially inflate `observed_boundary` and hide corruption. (`load_next_cursor` in `write_commit.rs:5-37` remains unchanged for cursor allocation.)
- **Call sites unified:** `read_projection_generation_status_at`, `read_mutation_projection_status`, `search/fusion_stage.rs:628`, and `graph_expand/execution.rs:1402` all call `projection_generation::snapshot_observed_boundary(&tx)?`.

### 1.3 Slot-Verified, Epoch-Guarded Shared `ProjectionGenerationStatusCache`

Replace `Mutex<Option<CachedProjectionGenerationStatus>>` in `struct Engine` (`fathomdb-engine/src/lib.rs:562-567`) with:

```rust
projection_generation_status_cache: Arc<projection_generation::ProjectionGenerationStatusCache>,
#[cfg(feature = "test-hooks")]
projection_generation_status_full_owner_scan_count: Arc<AtomicU64>,
```

(Exact same field names in `struct Engine` — `49` fields preserved, `module-boundary-policy.txt` `field projection_generation_status_cache root` preserved, `lib_rs_blob:` updated in `phase3-root-reconciliation.md:4`.)

Inside `fathomdb-engine/src/projection_generation.rs` (zero import of `reader_pool`!):

```rust
pub(crate) const STATUS_CACHE_MAX_ENTRIES: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StatusCacheConnectionSlot {
    ReaderWorker(usize), // 0..reader_worker_count
    Primary,             // reader_worker_count
}

impl ProjectionGenerationStatusCache {
    pub(crate) fn new(
        reader_worker_count: usize,
        #[cfg(feature = "test-hooks")] full_owner_scan_count: Arc<AtomicU64>,
    ) -> Self { ... }
}
```

#### Internal State (`Mutex<StatusCacheState>`)

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
struct SlotObservation {
    data_version: u64,
    total_changes: u64,
    visibility_generation: u64,
    state_nonce: String,
    epoch: u64,
}

#[derive(Clone)]
struct CachedStatusEntry {
    cache_epoch: u64,
    visibility_generation: u64,
    state_nonce: String,
    schema_version: u32,
    authoritative_boundary: u64,
    effective_from_epoch_s: i64,
    valid_before_epoch_s: Option<i64>,
    runtime_state: ProjectionRuntimeStateV1,
    is_unscoped: bool,
    scope_read_scopes: Vec<String>,
    status: ProjectionGenerationStatusV1,
    coverage_counts: Option<crate::StoredHeadCoverageCounts>,
}

struct StatusCacheState {
    cache_epoch: u64,
    max_visibility_generation: u64,
    slots: Vec<Option<SlotObservation>>, // len = reader_worker_count + 1
    entries: Vec<CachedStatusEntry>,     // bounded <= STATUS_CACHE_MAX_ENTRIES (8)
}
```

Also add `total_changes: u64` to `CachedMutationProjectionStatus` (`projection_generation.rs:183-193, 1865-1878, 1998-2009`) so `read_mutation_projection_status` never misses same-connection non-visibility-bumping mutations on `Engine.connection`.

#### Startup Seeding Barriers (No Unseeded Slot When `Engine::open` Returns)

- **`Primary` slot (`open.rs`):** `status_cache.observe_connection(&connection, StatusCacheConnectionSlot::Primary)?` executes **after** all boot mutations (`run_vector_equivalence_probe`, `reconcile_edge_history_on_boot`, `boot_graft_declared_vector_backfill` at `open.rs:1136-1190`) and visibility load, and **before** `projection_runtime.notify_new_work()` (`open.rs:1349`) and `physical_maintenance.start(...)` (`open.rs:1352`).
- **`ReaderWorker` slots (`reader_pool.rs`):** Each reader worker thread calls `let _ = status_cache.observe_connection(&connection, StatusCacheConnectionSlot::ReaderWorker(worker_idx));` (non-panicking before `ready.send(())` since `reader_worker_loop` returns `()`; if seeding ever failed, leaving `slots[worker_idx] = None` still fails safe via Case B forced verification) right after opening its dedicated read-only `Connection` and strictly **before** `ready.send(())` (`reader_pool.rs:682`, awaited by `ReaderWorkerPool::new` at `reader_pool.rs:1349-1351`), guaranteeing `Engine::open` returns with all healthy workers seeded.
- `observe_connection` reads `(visibility_generation, state_nonce, coalescing)`, `PRAGMA data_version`, and `connection.total_changes()` (zero completion scan) and stores `SlotObservation { ..., epoch: state.cache_epoch }` while setting `state.max_visibility_generation = state.max_visibility_generation.max(visibility_generation)`.

#### Lookup & Insertion Algorithm (`get_or_compute`)

Called inside an already-open `DEFERRED` read transaction `&tx` (no mutex held across SQLite scans!):

1. **Snapshot metadata read (unlocked, O(1) indexed/pragma queries):**
   - Read `(visibility_generation, state_nonce, coalescing)` from `_fathomdb_read_visibility_state WHERE singleton = 1` (fail closed with `corruption()` if missing, malformed hex, `coalescing != 0`, or `generation > i64::MAX as u64`).
   - Read `schema_version: u32` (`PRAGMA schema_version`), `data_version: u64` (`PRAGMA data_version`), `total_changes: u64` (`tx.total_changes() as u64`).
   - Compute `boundary = snapshot_observed_boundary(tx)?`.

2. **Slot-verified cache probe (lock acquired briefly):**
   - Examine `state.slots[slot_idx]`:
     - **Case A — Slot previously observed the exact same `(visibility_generation, state_nonce)`:**
       - **Subcase A1 (`prev.data_version != data_version || prev.total_changes != total_changes`):** Out-of-band external tamper (e.g., direct edit to `vector_default`, which has `visibility_trigger_prefix: None`) or same-connection non-visibility-bumping mutation detected!
         - `state.cache_epoch = state.cache_epoch.wrapping_add(1)`.
         - `state.entries.clear()`.
         - `state.slots[slot_idx] = Some(SlotObservation { data_version, total_changes, visibility_generation, state_nonce, epoch: state.cache_epoch })`.
         - Forced MISS with `miss_epoch = state.cache_epoch`, `allow_insert = true`.
       - **Subcase A2 (`prev.epoch != state.cache_epoch`):** `tx` is pinned to a pre-tamper snapshot (`data_version` and `total_changes` have not advanced past the `cache_epoch` bump caused by another slot detecting a tamper).
         - Uncached pass-through: do **NOT** hit `state.entries`, do **NOT** update `state.slots[slot_idx]`, and set `allow_insert = false` so `tx`'s pre-tamper scan cannot poison the post-tamper epoch.
       - **Subcase A3 (`prev.data_version == data_version && prev.total_changes == total_changes && prev.epoch == state.cache_epoch`):** Slot is verified clean for `(visibility_generation, state_nonce, state.cache_epoch)`!
         - Search `state.entries` (MRU back-to-front) for an entry matching:
           - `entry.cache_epoch == state.cache_epoch`
           - `entry.visibility_generation == visibility_generation`
           - `entry.state_nonce == state_nonce`
           - `entry.schema_version == schema_version`
           - `entry.authoritative_boundary == boundary`
           - `effective_at_epoch_s >= entry.effective_from_epoch_s`
           - `entry.valid_before_epoch_s.is_none_or(|upper| effective_at_epoch_s < upper)`
           - `entry.runtime_state == runtime_state`
           - `entry.is_unscoped == scope.is_unscoped()`
           - `entry.scope_read_scopes.as_slice() == scope.read_scopes()`
         - **HIT (provided `!capture_coverage || entry.coverage_counts.is_some()`):**
           - Move matched entry to MRU back (bounded `<= 8`).
           - Clone `entry.status` (setting `status.effective_at_epoch_s = effective_at_epoch_s`) and `counts = if capture_coverage { entry.coverage_counts.clone() } else { None }`.
           - Release lock.
           - If `capture_coverage`: build `coverage = match counts { Some(counts) => crate::stored_coverage::token(tx)?.map(|token| CachedStoredHeadCoverage { token, counts }), None => None }`.
           - Return `Ok((status, coverage))` in **O(1)** (zero completion scan)!
         - **MISS (key not yet cached or `capture_coverage` needs counts):** capture `miss_epoch = state.cache_epoch`, `allow_insert = true`, release lock.
     - **Case B — `slots[slot_idx]` is `None` or `prev.visibility_generation != visibility_generation || prev.state_nonce != state_nonce`:**
       - Because SQLite's `PRAGMA data_version` collapses `1` vs `>= 2` commits between a connection's observations, a slot's first visit to a new `(visibility_generation, state_nonce)` MUST verify `&tx` rather than blindly trusting another slot's entry (closing the cross-slot `vector_default` tamper blind spot completely!).
       - Forced slot-verification MISS with `miss_epoch = state.cache_epoch`, `allow_insert = true`, release lock.

3. **Full snapshot computation (unlocked — never hold cache mutex across SQLite scan!):**
   - Increment `full_owner_scan_count` (`#[cfg(feature = "test-hooks")]`, relaxed atomic).
   - Call `status_with_capture_in_snapshot(tx, scope, runtime_state, effective_at_epoch_s, boundary, true)?` — always capturing `coverage_counts` (`vector_spans::capture` executes the exact same 4-table `LEFT JOIN` + `UNION ALL` as `vector_spans::validate`, `vector_spans.rs:3-41, 61-90`, so storing `coverage_counts` on every cache fill has zero extra query complexity and guarantees `Primary` `read_projection_generation_status` and `read_mutation_projection_status` can ALWAYS publish `Available` `stored_head_coverage` even after a pooled reader warmed the cache!).
   - If `capture_coverage`: build `CachedStoredHeadCoverage` via `crate::stored_coverage::token(tx)?` (`Primary` callers: BOTH `read_projection_generation_status_at` at `projection_generation.rs:1789` AND `read_mutation_projection_status` at `projection_generation.rs:1964`). Pooled-reader callers (`search/fusion_stage.rs`, `graph_expand/execution.rs`) pass `capture_coverage = false` and ignore `coverage`.
   - Compute `valid_before_epoch_s = next_status_membership_boundary_in_scope(tx, effective_at_epoch_s, scope)?`.

4. **Epoch- & generation-guarded insertion (lock acquired briefly):**
   - If `!allow_insert || state.cache_epoch != miss_epoch || visibility_generation < state.max_visibility_generation`:
     - Do **NOT** insert into `state.entries` and do **NOT** overwrite `state.slots[slot_idx]` (prevents both pre-tamper `S_pre` poisoning and older concurrent snapshot `S1` overwriting `S2`).
     - Release lock and return `Ok((status, coverage))` to the caller.
   - If `visibility_generation > state.max_visibility_generation`:
     - Advance `state.max_visibility_generation = visibility_generation` and retain only entries with `entry.visibility_generation >= visibility_generation`.
   - Deduplicate any matching key in `state.entries`, evict LRU front if `state.entries.len() >= STATUS_CACHE_MAX_ENTRIES`, push `CachedStatusEntry { cache_epoch: miss_epoch, ..., coverage_counts }`, and update `state.slots[slot_idx] = Some(SlotObservation { data_version, total_changes, visibility_generation, state_nonce, epoch: miss_epoch })`.
   - Release lock and return `Ok((status, coverage))`.

#### Why This Satisfies Every Correctness & Performance Invariant

- **1 scan total on seeded startup across all 8 pooled readers:** Because every worker slot `W0..W7` seeds `(open_vis_gen, open_nonce, dv_w, 0, epoch=0)` before `Engine::open` returns, on an opened DB without new writes, Query 1 (`W0`) performs **1** scan and Queries `2..N` across **all 8 workers** (`W0..W7`) hit Case A3 in **O(1)** immediately!
- **At most 1 verification scan per worker on first entry after a visibility-advancing commit, then 0 steady-state scans forever:** Closes the cross-slot `vector_default` external-tamper blind spot 100% faithfully while matching `AC32-PERF-STATUS-01`.
- **Zero concurrent pre-tamper or stale-snapshot poisoning:** `SlotObservation.epoch` + `CachedStatusEntry.cache_epoch` + `allow_insert` + `max_visibility_generation` mathematically prevent pre-tamper `S_pre` or older `S1` from poisoning `S_post` / `S2`.
- **Zero `stored_head_coverage` regression:** Storing `coverage_counts` on every cache fill ensures `read_projection_generation_status` and `read_mutation_projection_status` always publish `Available` `stored_head_coverage` even when a pooled reader warmed the cache first.
- **Zero `Engine.connection` self-write blindness:** `SlotObservation.total_changes` (and `CachedMutationProjectionStatus.total_changes`) catches same-connection mutations when `PRAGMA data_version` stays constant.
- **Zero thrash on interleaved `valid_as_of` / `runtime_state`:** Bounded `STATUS_CACHE_MAX_ENTRIES = 8` LRU retains multiple effective-time windows and runtime states simultaneously.

### 1.4 Reader Pool & Direct Caller Threading

- `ReaderWorkerPool::new` takes `status_cache: Arc<projection_generation::ProjectionGenerationStatusCache>` (constructed in `open.rs` and shared with `Engine.projection_generation_status_cache`).
- Each worker thread seeds `StatusCacheConnectionSlot::ReaderWorker(worker_idx)` before `ready.send(())` (`reader_pool.rs:682`), and passes `(&status_cache, StatusCacheConnectionSlot::ReaderWorker(worker_idx))` into:
  - `read_search_work_in_tx` (`ReaderRequest::Search` and `ReaderRequest::SearchEvidence`) -> `search/fusion_stage.rs:626-637`
  - `read_graph_expand_with_stop_words_in_tx` (`graph_expand/execution.rs:1346`, called at `reader_pool.rs:1001`) -> `graph_expand/execution.rs:1402-1408`
- Any in-crate `#[cfg(test)]` direct callers of `read_graph_expand_in_tx` (`graph_expand/execution.rs:1318`) or `read_search_work_in_tx` pass a local `ProjectionGenerationStatusCache::new(1, ...)` + `StatusCacheConnectionSlot::ReaderWorker(0)`.

---

## 2. Part B — Additive `PerHitExplain.vector_cosine: Option<f64>` (`AC32-EXPLAIN-COSINE-01..04`)

### 2.1 `sqlite-vec = "=0.1.9"` `vec_distance_cosine` Verification & Numeric Contract

- At the start of `WP-40D` TDD, verify `vec_distance_cosine(v.embedding, vec_f32(?2))` directly on bundled `sqlite-vec = "=0.1.9"` for:
  1. identical unit vectors -> `cos_dist ≈ 0.0` -> `vector_cosine ≈ 1.0`,
  2. orthogonal vectors -> `cos_dist ≈ 1.0` -> `vector_cosine ≈ 0.0` (normalized `+0.0`, never `-0.0`),
  3. opposite vectors -> `cos_dist ≈ 2.0` -> `vector_cosine ≈ -1.0`,
  4. non-unit-norm scaled vectors (e.g., `[3.0, 4.0, ...]` vs `[6.0, 8.0, ...]`, via test embedder) -> `cos_dist ≈ 0.0` -> `vector_cosine ≈ 1.0`,
  5. zero-norm stored or query vector (`[0.0, ...]`) -> SQL returns `NULL` or `NaN` -> helper returns `None` (never `NaN`, never `0.0`).

Conversion helper in `fathomdb-engine/src/search/vector_arm.rs`:

```rust
pub(crate) fn cosine_similarity_from_distance(distance: Option<f64>) -> Option<f64> {
    let d = distance?;
    if !d.is_finite() {
        return None;
    }
    let sim = 1.0 - d;
    if !sim.is_finite() {
        return None;
    }
    let clamped = sim.clamp(-1.0, 1.0);
    Some(if clamped == 0.0 { 0.0 } else { clamped })
}
```

### 2.2 SQL Generation (`filter.rs`) — Zero Cost When `explain == false`, Unchanged L2 Ordering

- Extend `build_vector_phase1_plan` and `build_vec0_fast_phase1_sql` with `include_cosine: bool`:
  - `include_cosine == false` (all `explain == false` searches, `filter.rs:1178-1187` `build_vector_phase1_sql`, and `search_api.rs:2004-2011` `explain_vector_phase1_for_test`): emits the **exact byte-identical SQL** as today (`SELECT c.rowid, vec_distance_l2(v.embedding, vec_f32(?2)) AS l2 ... ORDER BY l2 LIMIT {final_limit}`). Zero `vec_distance_cosine` evaluation when `explain == false`!
  - `include_cosine == true` (both fast-path `build_vec0_fast_phase1_sql` at `filter.rs:1169` — including `view.include_out_of_window = true` at `:1158-1163` — AND preselection path at `filter.rs:1129`): emits:

```sql
SELECT c.rowid,
       vec_distance_l2(v.embedding, vec_f32(?2)) AS l2,
       vec_distance_cosine(v.embedding, vec_f32(?2)) AS cos_dist
FROM candidates c JOIN vector_default v ON v.rowid=c.rowid
ORDER BY l2 LIMIT {final_limit}
```

- **Ranking invariance:** `ORDER BY l2` is completely unchanged, and multi-scope merge (`vector_arm.rs:124-127`) continues sorting strictly by `(left.1.total_cmp(&right.1), left.0.cmp(&right.0))` on `l2` distance.

### 2.3 Threading Through `execute_vector_arm` & `fusion_stage.rs`

- Change `execute_vector_arm` to accept `explain: bool` and return `VectorArmOutput`:

```rust
pub(super) struct VectorArmOutput {
    pub(super) hits: Vec<FusionHit>,
    pub(super) cosines: HashMap<RecordKey, Option<f64>>,
}
```

- Widen candidate rows in `vector_arm.rs:91, 97, 134, 179` to `(i64, f64, Option<f64>)` (updating `#[cfg(test)] rank_probe` tuple destructuring at `vector_arm.rs:134` to keep emitting `(rowid, distance.to_bits())` unchanged).
- When `explain == false`: `cosines` is empty (`HashMap::new()`); `rowids` reads 2 columns `(i64, f64)` with `cos_dist = None`.
- When `explain == true`: `rowids` reads 3 columns `(i64, f64, Option<f64>)`; for every hydrated hit pushed to `results` — BOTH `FusionOwnership::CanonicalNode` (`branch = Vector`) AND `FusionOwnership::CanonicalEdge` (`branch = TextEdge`, `edge_fact`) — insert `cosines.entry(hit.key()).or_insert_with(|| cosine_similarity_from_distance(cos_dist))`.
- In `search/fusion_stage.rs`:
  - Capture `exp_vector_cosines = explain.then_some(vector_output.cosines)`.
  - When constructing `PerHitExplain`:

```rust
vector_cosine: exp_vector_cosines
    .as_ref()
    .and_then(|m| m.get(&h.key()).copied().flatten()),
```

- **Exact invariant proof:**
  - `exp_vector_cosines` is populated at the exact same hydration loop and keyed by the exact same `RecordKey` (`(FusionOwnership, write_cursor)`) as `exp_vector_ranks`, and survives all fusion branches (`vector_stage_only`, `deferred_node_candidates`, `use_graph_arm`, standard `fuse_rrf`), CE reranking, and `search_with_evidence`.
  - Any hit with `vector_rank == None` (single-arm `Text`, text-only `TextEdge`, `GraphArm`, multi-arm `Text + GraphArm` or `TextEdge(text-only) + GraphArm`, text-only query, `use_default_embedder = false`, or vector soft-fallback) is absent from `exp_vector_cosines` -> `vector_cosine == None`.
  - Any hit scored by `execute_vector_arm` (`vector_rank.is_some()`, including vector-scored `TextEdge` `edge_fact` hits) resolves its exact pre-RRF `Some(cosine)` in `[-1.0, 1.0]` (or `None` if zero-norm / non-finite).

### 2.4 Public Type & Cross-Language Parity Surface (`AC32-EXPLAIN-COSINE-03..04`)

| Layer | Exact File(s) & Symbols | Exact Change |
| --- | --- | --- |
| Engine (`#[non_exhaustive]`) | `src/rust/crates/fathomdb-engine/src/search_types.rs` | Add `pub vector_cosine: Option<f64>` on `PerHitExplain` |
| Rust facade & SDK | `src/rust/crates/fathomdb/src/lib.rs`, `src/rust/crates/fathomdb-sdk/src/lib.rs:120` | Both re-export `fathomdb_engine::PerHitExplain` directly; add assertion in `fathomdb-sdk/tests/surface.rs` |
| Python native | `src/rust/crates/fathomdb-py/src/graph_evidence.rs:488-611` | Add `#[pyo3(get)] vector_cosine: Option<f64>` on `PyPerHitExplain` and copy `hit.vector_cosine` |
| Python package & boundary | `src/python/fathomdb/types.py`, `src/python/fathomdb/_sdk_search.py:356-429` (`_map_per_hit_explain`), `src/python/fathomdb/_fathomdb.pyi`, `src/python/tests/python_surface_additions.json`, `src/python/tests/test_slice130_package_boundary.py:646-701` | Add `vector_cosine: float \| None = None` on `PerHitExplain`; validate `None` or finite `float`/`int` (reject `bool`, `NaN`, `inf`, out of `[-1.0, 1.0]`) in `_map_per_hit_explain` and forward `vector_cosine`; add `_fathomdb.pyi` entry + `python_surface_additions.json` + `test_slice130_package_boundary.py` stub reversal stanza; rebuild `.venv` wheel via `bash scripts/setup-python-owned.sh --build` |
| TypeScript native | `src/rust/crates/fathomdb-napi/src/graph_evidence.rs:420-528` (+ unit test at `:543-613`) | Add `pub vector_cosine: Option<f64>` on N-API `PerHitExplain` (`vectorCosine` in JS) and populate from Rust |
| TypeScript package | `src/ts/src/binding.ts:376-393` (`NativePerHitExplain`), `src/ts/src/search.ts:739-753, 787-839` (`PerHitExplain`, `mapPerHitExplain`) | Add `vectorCosine?: number \| null` on `NativePerHitExplain` and `readonly vectorCosine: number \| null` on `PerHitExplain`; in `mapPerHitExplain` normalize `p.vectorCosine ?? null` and assert `null` or `typeof === "number" && Number.isFinite(v) && v >= -1.0 && v <= 1.0`; rebuild native addon & `dist/` |
| Interfaces & public docs | `dev/interfaces/{rust,rust-sdk,python,typescript}.md`, `docs/reference/{rust-sdk,rust-api,python-api,typescript-api}.md` | Document `PerHitExplain.vector_cosine` / `vectorCosine` (introducing/extending `PerHitExplain` field sections where absent) |
| ADR & proposal cross-refs | `dev/design/0.8.8-explain-and-telemetry-adr.md` §A.4 Q1, `dev/design/mcp-adapter/engine-proposals/EP-13-raw-arm-scores.md` | Brief amendment note recording `Gate1-D10` ratified `PerHitExplain.vector_cosine` in `0.8.32` while keeping `vector_distance`/`text_bm25` deferred |
| Unchanged surfaces | `SearchHit`, `EvidenceContributionV1` (`evidence/codec.rs`), `fathomdb-mcp` `PerHitOut`, Schema 39 DDL, `scripts/governed-surface-pin.json` (`58`/`97`) | **Strictly untouched** |

### 2.5 Bundled Slice 40 Housekeeping

1. `scripts/tests/test_runtime_release_alignment.sh:262`: update the non-recovery `post-publish-smoke` matrix assertion to match `WP-40C`'s `["crates-cli","crates-mcp","crates-sdk","pypi-wheel","pypi-mcp","npm-package"]`.
2. `dev/interfaces/mcp.md:177`: add `entity_class` to the `orient` field census row (`projections[].entity_class/name/roles[]/fts/vector; projection_status.projections[].entity_class/name/dense_readiness`).

---

## 3. TDD Verification Plan (Strict RED → GREEN)

All tests are written **failing first** before production implementation, with feature gating cleanly separated so default `cargo test` and `scripts/check-test-target-coverage.py` both pass:

1. **Rust `AC32-PERF-STATUS-01..03` — `src/rust/crates/fathomdb-engine/tests/slice40_status_cache.rs` (`required-features = ["test-hooks"]` in `fathomdb-engine/Cargo.toml`, registered in `scripts/test-feature-matrix.toml` via `scripts/lib/test_targets.py --write-matrix`) + unit tests in `projection_generation.rs`:**
   - **Seeded startup O(1) pooled-reader cache (`use_default_embedder = false` AND `= true`):** Open a store with a declared vector projection and nodes; run `24` sequential & concurrent calls across `search(..., explain=true)`, `search_frozen(..., explain=true)`, `search_with_evidence`, and query-seeded `graph.expand`. Assert `projection_generation_status_full_owner_scan_count` increases by **1** total across all 8 seeded reader workers on cold first query and stays flat (`delta == 0`) on all subsequent calls.
   - **Internal write, projection publication & `Engine.connection` self-write invalidation:** Write a new node -> assert next pooled read observes updated `observed_boundary` / `pending_count` immediately (at most 1 verification scan per worker entering the new `visibility_generation`, then `delta == 0` at steady state). Mutate on `Engine.connection` directly -> assert `Primary` slot detects `total_changes` immediately (covering both `read_projection_generation_status` and `read_mutation_projection_status`).
   - **External raw-SQLite `vector_default` tamper detection (warmed, cold-after-open, AND cross-slot after visibility advance!):**
     - Mutate `vector_default` directly via external `rusqlite::Connection` (`visibility_trigger_prefix: None`, leaving `_fathomdb_read_visibility_state` unchanged!) on: (a) a freshly opened cold store before any query, (b) a warmed store, and (c) the cross-slot scenario (`open` -> write advancing `vis_gen` -> warm `W0` -> external `vector_default` tamper -> query across `W1..W7` and `Primary`). Assert corruption/re-scan is detected on **every** slot with zero permanent poisoning!
   - **Real concurrent multi-thread snapshot race test (`projection_generation.rs` unit test):** Hold two live read transactions (`S_pre` and `S_post`) across threads exercising step-1 -> step-2 pre-tamper `epoch` mismatch (`allow_insert = false`) and step-4 `vis_gen < max_visibility_generation` rejection; assert `S_pre` / `S1` never overwrites `S_post` / `S2` or poisons any slot.
   - **`stored_head_coverage` roundtrip after pooled-reader warm:** Warm `ProjectionGenerationStatusCache` via `search(..., explain=true)` before calling `read_projection_generation_status()` and `read_mutation_projection_status()`; assert `read_projection_status().vector_coverage` transitions to `Available` cleanly and `snapshot_observed_boundary` matches across all callers (including after physical erasure raising `cursor_floor`).
   - **Interleaved `valid_as_of` & multi-scope isolation:** Interleave two `valid_as_of` instants across an edge expiry boundary -> assert `delta == 0` (no thrash in `<= 8` LRU); verify distinct `ScopePolicy` predicates never cross-contaminate cached counts.

2. **Rust `AC32-EXPLAIN-COSINE-01..04` — `src/rust/crates/fathomdb-engine/tests/slice40_explain_cosine.rs` (default features!) + unit tests in `search/vector_arm.rs`, `filter.rs`, and `fathomdb-napi/src/graph_evidence.rs`:**
   - **`sqlite-vec = "=0.1.9"` `vec_distance_cosine` truth table:** Identical (`≈ 1.0`), orthogonal (`0.0`, verifying `+0.0` not `-0.0`), opposite (`≈ -1.0`), non-unit-norm scaled vectors (`[3.0, 4.0, ...]` vs `[6.0, 8.0, ...]` -> `≈ 1.0`), and zero-norm vectors (`None`, never `NaN`/`0.0`).
   - **Exact arm & SQL-branch semantics:** Vector-scored node hit (`Some`), vector-scored `TextEdge` `edge_fact` hit (`Some`), hybrid `Vector + Text` hit (`Some`), preselection-path (`filter.rs:1129`, e.g., `Ne` / numeric property predicate and relaxed `ReadView`), `view.include_out_of_window = true` (`filter.rs:1158-1163`), multi-scope `ScopePolicy::multi`, text-only `Text` / `TextEdge` / `GraphArm` / multi-arm `Text + GraphArm` hits (`None`), `use_default_embedder = false` / `search_text_only` (`None`), vector soft-fallback (`None`), plus universal `assert!(p.vector_rank.is_some() || p.vector_cosine.is_none())`.
   - **Zero `explain=false` SQL change & L2-order invariance:** Assert `build_vector_phase1_plan(..., false)` emits zero `vec_distance_cosine` references, and `search(..., explain=true)` returns identical hit ordering and `SearchHit` fields as `search(..., explain=false)` (single-scope and multi-scope).
   - **N-API mapping unit test (`fathomdb-napi/src/graph_evidence.rs`):** Assert `PerHitExplain::from_rust` preserves `Some(v)` and `None` for `vector_cosine`.

3. **Python & TypeScript Parity & Negative Mapper Tests (`src/python/tests/test_slice40_wp40d.py`, `src/ts/tests/slice40-wp40d.test.ts`):**
   - Verify `PerHitExplain.vector_cosine` (`float | None`) in Python and `PerHitExplain.vectorCosine` (`number | null`) in TypeScript across vector-scored hits (`search(..., explain=True)` and `search_with_evidence(..., include_explanation=True)`), text-only hits (`None`/`null`), and negative mapper cases (`bool`, `NaN`, `Infinity`, `1.5`, `-1.5` rejected; JS `undefined` normalized to `null`).
   - Run `test_slice130_package_boundary.py`, `pyright`, `ruff`, `npm --prefix src/ts test`, `check-sdk-surface-parity.py`, `check-governed-surface-pin.sh` (`58`/`97` unchanged), `check-module-boundaries.sh`, `check-test-target-coverage.py`, `test_slice90_*.py`, and `test_runtime_release_alignment.sh`.
