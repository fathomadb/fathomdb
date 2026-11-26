---
title: Slice 41 Detailed Design — 0.8.32 Engine Query Scaling, Purge Lineage, Evidence Eligibility, and Partitioned Frozen-Read Stability
status: APPROVED
target_release: 0.8.32
slice: 41
date: 2026-11-26
---

# Slice 41 Detailed Design — `ENGINE-QUERY-AND-EVIDENCE-HARDENING`

## 0. Scope, Traceability, and Hard Invariants

- **Requirements & rulings covered:** `R32-09` (`#268` purge-lineage & shared-bucket receipt hardening, `#267` Parts 1 & 2 Option 2B FTS/evidence & `read.get` `O(N)` elimination, `#269` Option A artifact-scoped `property_predicates` on evidence `source_filter`, and `#267` Part 3 Schema 39 partitioned frozen-read additive stability) and `R32-06` (`TABLE_MANIFEST`, `sqlite_master` completeness, erasure integrity, and zero latency regression).
- **Authoritative references:**
  - `dev/adr/ADR-0.8.32-frozen-read-additive-stability-and-evidence-eligibility.md` (`#103`, accepted 2026-11-26 — amends `#46`, `#47`, `#48`; extends `#70`, `#102`)
  - `dev/plans/0.8.32/features/slice-41/plan.md` (`APPROVED`)
  - `dev/plans/0.8.32/features/slice-41/frozen-read-stability-experiment.md`
  - `dev/design/fathomdb-data-plane-architecture-v2.md`, `dev/design/engine.md`, `dev/design/retrieval.md`, `dev/design/actuation.md`, `dev/design/mcp-adapter/design.md`, and `dev/design/mcp-adapter/requirements.md`
- **Hard invariants:**
  - **Governed surface invariance:** `scripts/governed-surface-pin.json` (`58` ops / `97` allowlist, `core = 5`, `recovery_denylist = 5`) and `src/conformance/governed-{operation-parity,surface-allowlist}.json` remain strictly **unchanged** (`0` new governed verbs; `0` new flags on `ReadContextV1`).
  - **`MR-070.2` & `rmcp` confinement:** `src/rust/crates/fathomdb-mcp/Cargo.toml` links ONLY `fathomdb-sdk` among workspace FathomDB crates (zero direct `fathomdb-engine` or `rusqlite` dependencies).
  - **Unreleased Schema 39 in-place extension:** `SCHEMA_VERSION` in `src/rust/crates/fathomdb-schema/src/lib.rs` remains `39`. `STEP_39_SQL` is extended in place (no Schema `40` bump prior to `0.8.32` release, no canonical node/edge table rebuilds). Golden `0.8.31` Schema `38 -> 39` migration tests and Schema `31`/`32` normative frozen-read fixtures remain bit-identical and green.
  - **`TABLE_MANIFEST` single source of truth:** `TABLE_MANIFEST` grows from `35` to `36` named tables (`31` regular + `5` virtual) with `_fathomdb_scope_visibility_watermarks`, synchronized across `fathomdb-schema`, `fathomdb-engine/src/export_manifest.rs`, `src/inspector/src/admission.rs`, `projection_generation::bootstrap_is_fresh`, and `frozen_read::validate_on_open`.
  - **`lib_rs_blob` sync:** If `src/rust/crates/fathomdb-engine/src/lib.rs` is modified (e.g., module wiring for visibility watermark helpers), update `lib_rs_blob:` in `dev/plans/0.8.27/features/slice-90/phase3-root-reconciliation.md` in the same commit.
  - **Product-generic discipline:** All code, tests, SQL, comments, and documentation remain strictly generic to FathomDB with zero external consumer-project specifics.

---

## 1. `WP-41A`: Purge-Lineage & Shared-Bucket Receipt Hardening (`#268`, `AC32-PURGE-LINEAGE-01..02`)

### 1.1 Root-Cause & Existing Behavior Analysis

Two interrelated defects occur when callers run `actuate -> transition("deleted") + purge() -> actuate` on entities sharing a caller `source_id` bucket:

1. **Post-purge cursor monotonicity (`AC32-PURGE-LINEAGE-01`):**
   - `actuate_new` in `src/rust/crates/fathomdb-engine/src/actuation.rs` allocates `base_cursor = load_next_cursor(&tx)` and inserts the batch commit timestamp via `record_batch_commit_timestamp(&tx, next_cursor, ...)` into `_fathomdb_commit_timestamps(write_cursor INTEGER PRIMARY KEY, committed_at_ms INTEGER NOT NULL)`.
   - `purge_inner` in `src/rust/crates/fathomdb-engine/src/erasure.rs` deletes the purged node's rows from `canonical_nodes`, `canonical_edges`, `_fathomdb_artifact_revisions`, and row-owned projection shadows (`erase_row_projections`, including `_fathomdb_projection_terminal`), while intentionally retaining `_fathomdb_commit_timestamps`.
   - `load_next_cursor` in `src/rust/crates/fathomdb-engine/src/write_commit.rs` already folds `max_cursor(connection, "_fathomdb_commit_timestamps")`, `_fathomdb_projection_terminal`, `reserved_write_cursor`, `_fathomdb_dependency_closures`, and `physical_erasure::cursor_floor`.
   - However, `purge_inner` and `excise_source_inner_with_prior` in `src/rust/crates/fathomdb-engine/src/erasure.rs` compute `pending_cursor` / `proof_boundary` from `self.next_cursor.load(Ordering::SeqCst)` rather than `self.next_cursor.load(Ordering::SeqCst).max(load_next_cursor(&tx))`, and no dedicated regression test covers `actuate -> transition("deleted") + purge() -> actuate` across both sole-node purge (empty `canonical_nodes`) and trailing-cursor purge (surviving lower-cursor nodes), both in-process and across `close()`/`open()`, followed by `doctor data-plane-integrity`.
2. **Over-broad shared-bucket receipt redaction in `purge_inner` (`AC32-PURGE-LINEAGE-02`):**
   - Every committed or refused actuation receipt stores reference rows in `_fathomdb_actuation_receipt_source_refs(operation_id, schema_version, ref_kind, ref_value, scope)` where `ref_kind IN ('source_id', 'source_revision_id', 'artifact_revision_id')`:
     - `collect_source_refs` in `src/rust/crates/fathomdb-engine/src/actuation.rs` records:
       - `PutCanonicalNode` / `PutDerivedNode` / `PutDerivedEdge`: `("source_id", source_id)`, `("artifact_revision_id", artifact_revision_id)`, and (when `Some`) `("source_revision_id", source_revision_id)`.
       - `RegisterSourceDependency`: `("source_revision_id", source_revision_id)` and `("artifact_revision_id", derived_revision_id)`.
       - `TransitionLifecycle`: `("artifact_revision_id", expected_current_revision_id)`.
       - `PutLatestState`: no canonical source refs.
     - `enrich_affected_refs` in `src/rust/crates/fathomdb-engine/src/actuation.rs` additionally inserts `("artifact_revision_id", rev)` for every affected revision (including superseded predecessors), `("source_revision_id", rev)` when `role == "canonical_source"`, and `("source_id", source_id)`.
   - In `purge_inner` in `src/rust/crates/fathomdb-engine/src/erasure.rs`, the current code queries `SELECT source_id FROM canonical_nodes WHERE scope=?1 AND logical_id=?2` and `canonical_edges WHERE scope=?1 AND (from_id=?2 OR to_id=?2)` into `erased_source_ids` and passes `erased_source_ids` into `actuation_erasure_refs_for_cursors(&tx, scope, &affected_cursors, erased_source_ids)`.
   - `redact_actuation_receipts_for_refs` then redacts **every** receipt matching `("source_id", shared_bucket)` in `scope` — destroying cached receipts for surviving sibling entities that merely share the same `source_id` bucket, causing legitimate sibling `operation_id` replays to fail with `ActuationErrorReason::OperationIdErased`.

### 1.2 Exact Code Changes (`src/rust/crates/fathomdb-engine/src/erasure.rs`)

1. **Narrow `purge_inner` receipt redaction to purged revision refs only:**
   - Remove the `erased_source_ids` SQL queries in `purge_inner` (`SELECT source_id FROM canonical_nodes ...` and `SELECT source_id FROM canonical_edges ...`).
   - Invoke:

   ```rust
   let receipt_refs = actuation_erasure_refs_for_cursors(
       &tx,
       scope,
       &affected_cursors,
       std::iter::empty::<String>(),
   )?;
   ```

   - `actuation_erasure_refs_for_cursors` continues collecting `("artifact_revision_id", revision)` and `("source_revision_id", revision)` from `_fathomdb_artifact_revisions` for every `write_cursor` in `affected_cursors` (all versions of `lid` in `canonical_nodes`, all touching edges in `canonical_edges`, and all physical-closure registered dependents from `physical_plans`).
   - `proof_scope` passed to `dependency_closure::physical_proof_scope(&tx, scope, &affected_cursors, &receipt_refs, None)` and `dependency_closure::measure_physical_closures` receives the exact same narrowed `receipt_refs`, keeping `remaining_receipt_reference_rows == 0` exact.
   - `excise_source_inner_with_prior` (`erase_source` / `excise_source`) remains unchanged on `receipt_refs`: it continues passing `[source_id.to_string()]` into `actuation_erasure_refs_for_cursors`, redacting all receipts bound to `("source_id", source_id)` including refused receipts whose revisions never landed in `_fathomdb_artifact_revisions`.
2. **Defense-in-depth monotonic cursor floor across all erasure paths (`src/rust/crates/fathomdb-engine/src/erasure.rs`):**
   - Compute `let current_floor = self.next_cursor.load(Ordering::SeqCst).max(crate::write_commit::load_next_cursor(&tx));` consistently across:
     - `purge_inner` early-return absent-node path (`physical_erasure::capture(&tx, &[], current_floor)`) and main purge path (`pending_cursor = current_floor.saturating_add(1)` and `proof_boundary`),
     - `excise_source_inner_with_prior` (`proof_boundary = current_floor.saturating_add(1)` and `audit_cursor` base), and
     - `excise_collection_record_inner` (`audit_cursor = current_floor.saturating_add(1)`).
   - This guarantees in-process `self.next_cursor` can never trail persisted `_fathomdb_commit_timestamps`, `_fathomdb_projection_terminal`, `_fathomdb_dependency_closures`, or `physical_erasure::cursor_floor` on any erasure path.

### 1.3 Security & Non-Disclosure Proof Across All Shared-Bucket Topologies

| Scenario | Purged Target & Cursors in `affected_cursors` | Redacted Receipts | Surviving Sibling Receipts in Same `source_id` Bucket | Erased Content Leakage Check |
| --- | --- | --- | --- | --- |
| 1. Single derived node `D1` purged; sibling `D2` and source `S1` survive in `"bucket-a"` | `D1` (all versions) + touching edges (`rev-D1`) | `op-D1` (`PutDerivedNode`), any `TransitionLifecycle` / `RegisterSourceDependency` referencing `rev-D1` | `op-S1`, `op-D2` untouched; replaying `op-D2` returns its cached receipt | Zero `D1` body or `rev-D1` bytes remain in DB or `-wal` (`source_id` is non-PII by contract) |
| 2. `canonical_source` node `S1` purged with registered dependent `D1` (in `physical_plans`), while unrelated `S2` / `D2` share `"bucket-a"` | `S1` + `D1` + touching edges (`rev-S1`, `rev-D1`) | `op-S1` (`artifact_revision_id=rev-S1`), `op-D1` (`source_revision_id=rev-S1` and `artifact_revision_id=rev-D1`), `op-dep`, `op-life-S1` | `op-S2`, `op-D2` untouched (distinct `rev-S2` / `rev-D2`) | Zero `S1`/`D1` bodies or `rev-S1`/`rev-D1` bytes remain |
| 3. `canonical_source` node `S1` purged while an unregistered derived node `D1` still links to `rev-S1` | Refused before deletion by `erase_artifact_identity_for_cursors(..., refuse_surviving_dependents = true, None)` with `ProvenanceInUse` | None (transaction rolls back atomically) | Untouched | No partial purge or dangling provenance |
| 4. Refused `TransitionLifecycle` referencing `expected_current_revision_id = rev-D1` | `D1` (`rev-D1`) | Redacted via `("artifact_revision_id", rev-D1)` recorded by `collect_source_refs` | Unrelated siblings untouched | Zero `rev-D1` references remain |
| 5. `erase_source("bucket-a")` | All nodes/edges in `"bucket-a"` plus `[source_id]` | **Every** receipt in `scope` with `ref_kind = 'source_id' AND ref_value = 'bucket-a'` (including refused multi-source receipts) | None (entire source bucket erased) | Preserves `slice25_actuation_verification.rs` `erasing_one_source_redacts_a_refused_multi_source_receipt` verbatim |

---

## 2. `WP-41B`: Zero-Schema-Change FTS/Evidence & `read.get` `O(N)` Elimination (`#267` Parts 1 & 2, Option 2B, `AC32-QUERY-SCALE-01..06`)

### 2.1 Decoupling `EvidenceContributionV1` from `explain = true` (`AC32-QUERY-SCALE-01`)

Currently `SearchReaderWork::evidence` in `src/rust/crates/fathomdb-engine/src/search.rs` hardcodes `explain = true` even when `request.include_explanation == false`, forcing `read_search_in_tx` in `src/rust/crates/fathomdb-engine/src/search/fusion_stage.rs` to compute `vec_distance_cosine`, `count_dropped_edge_hits`, `status_cache.get_or_compute`, `structural_lifecycle_state`, `structural_dependency_state`, and `PerHitExplain` only for `build_search_result` in `src/rust/crates/fathomdb-engine/src/evidence/ranked.rs` to drop `search_result.explanation = None`.

**Exact structural changes:**

1. **`src/rust/crates/fathomdb-engine/src/search.rs`:**
   - Update `SearchReaderWork::evidence` to accept `include_explanation: bool` and forward `include_explanation` as the `explain` argument to `Self::hybrid(...)` instead of hardcoded `true`.
   - Extend `SearchOriginCapture` with a zero-overhead method and pass optional lightweight contributions into `finish`:

   ```rust
   pub(crate) trait SearchOriginCapture: Sized {
       type Output;

       fn requires_edge_origin(&self) -> bool {
           false
       }

       fn requires_contributions(&self) -> bool {
           false
       }

       fn record_graph_origin(&mut self, _node_cursor: u64, _origin: CapturedGraphOrigin) {}

       #[allow(clippy::too_many_arguments)]
       fn finish(
           self,
           connection: &Connection,
           scope: &crate::scope::ScopePredicate,
           cursor: u64,
           soft_fallback: Option<SoftFallback>,
           results: Vec<FusionHit>,
           graph_stats: GraphFrontierStats,
           explanation: Option<Explanation>,
           contributions: Option<Vec<crate::evidence::EvidenceContributionV1>>,
           expanded: Option<SearchExpandResult>,
           degradations: Vec<ArmDegradation>,
           snippets: Option<Vec<SnippetV1>>,
       ) -> Result<Self::Output, SearchReaderError>;
   }
   ```

   - `NoEvidenceCapture` remains zero-sized (`const _: () = assert!(std::mem::size_of::<NoEvidenceCapture>() == 0);`), returns `false` for `requires_contributions()`, and ignores `contributions`.
   - `EvidenceCapture` returns `true` for `requires_contributions()` and forwards `contributions` to `evidence::build_search_result`.
2. **`src/rust/crates/fathomdb-engine/src/search_api.rs`:**
   - Pass `request.include_explanation` into `SearchReaderWork::evidence(...)` (the `search_with_evidence` path).
3. **`src/rust/crates/fathomdb-engine/src/search/fusion_stage.rs`:**
   - Define `let capture_contributions = capture.requires_contributions();` and `let track_contributions = explain || capture_contributions;`.
   - Keep `execute_vector_arm(..., explain)`, `count_dropped_edge_hits(..., explain)`, `status_cache.get_or_compute(...)`, and `PerHitExplain` / `StructuralInclusionV1` gated strictly on `explain` (skipped completely when `include_explanation == false`).
   - Gate `exp_vector_ranks`, `exp_text_ranks`, `exp_graph_ranks`, `exp_fused_scores`, and post-truncation `exp_importance` / `exp_confidence` on `track_contributions`.
   - Pass `track_contributions` to `collect_node_fts_candidates` so `can_defer_text_identity` checks `!track_contributions` (preserving rank-map population parity with `explain = true`).
   - After `results.truncate(final_limit)` and optional post-truncation importance/confidence hydration (§2.2), if `capture_contributions` is `true`, construct `Vec<EvidenceContributionV1>` directly from `(results, exp_vector_ranks, exp_text_ranks, exp_graph_ranks, exp_fused_scores, exp_importance, exp_confidence)` using the exact same finiteness check as `contribution` in `src/rust/crates/fathomdb-engine/src/evidence/ranked.rs` (`EvidenceCorrupt` at `"/explanation/perHit"` on non-finite values).
4. **`src/rust/crates/fathomdb-engine/src/evidence/ranked.rs`:**
   - Update `build_search_result` to accept `contributions: Option<Vec<EvidenceContributionV1>>` and consume `contributions` directly instead of requiring `search_result.explanation.as_ref()`.
   - When `include_explanation == false`, `search_result.explanation` is already `None`. When `include_explanation == true`, verify `explanation.per_hit.len() == search_result.results.len()` and `contributions.len() == search_result.results.len()`. Every `EvidenceContributionV1` field (`schema_version`, `vector_rank`, `text_rank`, `graph_rank`, `fused_score`, `ce_score`, `blended_score`, `importance`, `confidence`) is bit-identical between `include_explanation = false` and `include_explanation = true`.

### 2.2 FTS-Only `search_index.kind = ?` Pushdown, Streaming `ORDER BY rank`, Capping Guard, and Post-Truncation Importance/Confidence (`AC32-QUERY-SCALE-02`)

1. **`search_index.kind = ?` pushdown (`src/rust/crates/fathomdb-engine/src/search/fts_fast_path.rs`):**
   - In `collect_node_fts_candidates`, when `filter.and_then(|f| f.kind.as_deref())` is `Some(kind)`, push `rusqlite::types::Value::Text(kind.to_string())` onto `text_params` at index `kind_idx` and build `fts_kind_clause = format!(" AND search_index.kind = ?{kind_idx}")`.
   - Inject `{fts_kind_clause}` immediately after `WHERE search_index MATCH ?1` in BOTH `rank_sql` and `body_fts_rank_sql_view` (passed via `eligibility` / dedicated clause before joining `canonical_nodes`). Because `search_index` stores `kind UNINDEXED`, SQLite evaluates `search_index.kind = ?` on the FTS5 row before probing `canonical_nodes` by PK (`write_cursor`), while retaining `cn.kind = ?` and `search_index.kind IS cn.kind` integrity checks verbatim.
   - In `rank_sql`, emit `CROSS JOIN canonical_nodes cn` whenever an owning `canonical_nodes` row is required (`!scope.is_unscoped() || filter.is_some()`, matching `property_fts_rank_stream_sql` in `src/rust/crates/fathomdb-engine/src/filter.rs`) and retain `LEFT JOIN canonical_nodes cn` only for unscoped, unfiltered queries that may surface legacy ownerless FTS rows. Using `CROSS JOIN` prevents SQLite's planner from reordering the join to drive from `canonical_nodes_scope_kind_idx` after `ANALYZE` on skewed `kind` distributions, keeping `search_index` as the outer loop so FTS5 `ORDER BY rank` streaming never degrades to `USE TEMP B-TREE FOR ORDER BY`.
2. **Banning `ORDER BY`-less `{limit_clause}` on deferred queries (`src/rust/crates/fathomdb-engine/src/search/fts_fast_path.rs`):**
   - Remove `{limit_clause}` unconditionally from both `defer_full_hydration` and `defer_text_identity` SQL statements (neither query has an `ORDER BY`, so applying a SQL `LIMIT` would silently truncate arbitrary FTS5 rowid order).
   - Add `observe_slice30_guard("all_nodes_directly_eligible", "fts_only_limit", false, fts_only_limit.is_none())` to `can_defer_text_identity` so any bounded FTS-only query routes through FTS5 `ORDER BY rank` streaming or `join_sql`.
3. **Safety-gated `fts_only_limit` when `_fathomdb_vector_kinds` is empty & bounded initial capacity (`src/rust/crates/fathomdb-engine/src/search/fts_fast_path.rs` & `fusion_stage.rs`):**
   - Pass `pool_n: usize` and `has_vector_kinds: bool` (computed once per query via `SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds LIMIT 1)` in `fusion_stage.rs`, shared with `detect_vector_soft_fallback`) and `vector_rows_visible: bool` into `collect_node_fts_candidates`.
   - Compute:

   ```rust
   let fts_only_safe = !use_graph_arm && !recency_enabled && !importance_enabled;
   let effective_fts_cap = if rerank_depth > 0 {
       final_limit.max(rerank_depth).max(pool_n)
   } else {
       final_limit
   };
   let vector_arm_structurally_empty =
       query_vector.is_none() || (!vector_rows_visible && !has_vector_kinds);
   let fts_only_limit = direct_text_candidate_limit
       .or_else(|| (fts_only_safe && vector_arm_structurally_empty).then_some(effective_fts_cap));
   ```

   - **OOM guard for large `rerank_depth` / `pool_n` (up to `u32::MAX`):** Because `rerank_depth` and `pool_n` are validated up to `u32::MAX` (`4,294,967,295`), `collect_complete_fusion_rank_boundary` (and `collect_complete_rank_boundary`) MUST cap initial buffer preallocation to `Vec::with_capacity(limit.saturating_add(1).min(4096))`. Large `rerank_depth` / `pool_n` values then stream and grow strictly proportionally to the actual number of matching FTS rows without attempting upfront multi-GiB allocation.
   - Exact safety properties:
     - Direct text-only API (`direct_text_candidate_limit == Some(MAX_SEARCH_RESULT_LIMIT)`) retains its fixed candidate window.
     - Whenever `use_graph_arm || recency_enabled || importance_enabled`, `fts_only_limit` is `None` (never truncates before graph seeding or score reweighting).
     - Whenever `rerank_depth > 0`, the cap expands to `final_limit.max(rerank_depth).max(pool_n)` so the cross-encoder receives its full candidate pool without upfront OOM.
     - When `_fathomdb_vector_kinds` is empty and no vector rows are visible, FTS-only `search_frozen` and `search_with_evidence` cap cleanly even when `use_default_embedder = true` produced a `query_vector`.
4. **Streaming `ORDER BY rank` for filtered FTS-only queries (`src/rust/crates/fathomdb-engine/src/search/fts_fast_path.rs`):**
   - Widen `rank_stream_requested` from `direct_text_candidate_limit.is_some() && filter.is_none() && !force_full_sort` to:

   ```rust
   let rank_stream_requested = fts_only_limit.is_some() && !force_full_sort && !vector_stage_only;
   ```

   - `rank_stream_eligible` retains the `O(1)` `NOT EXISTS(SELECT 1 FROM search_index_edges LIMIT 1)` check so mixed node+edge text queries remain on the reference `join_sql` path.
   - `rank_sql` joins `canonical_nodes cn` via `CROSS JOIN` when `!scope.is_unscoped() || filter.is_some()` (else `LEFT JOIN`), evaluates `WHERE search_index MATCH ?1{fts_kind_clause}{text_existence}{text_scope}{text_validity}{text_eligibility}{text_filter} AND rank MATCH '{rank_mapping}' ORDER BY rank`, and streams through `collect_complete_fusion_rank_boundary(&mut rows, rank_candidate_limit, ...)`, guaranteeing exact `(bm25 score, write_cursor)` tie completion at the `rank_candidate_limit` boundary with zero `USE TEMP B-TREE FOR ORDER BY` even post-`ANALYZE` on skewed `kind` distributions (verified on SQLite 3.45.1: `SCAN search_index VIRTUAL TABLE INDEX 32:rM3` -> `SEARCH cn USING INTEGER PRIMARY KEY (rowid=?)`).
5. **Post-truncation `build_importance_confidence_maps` when `!importance_enabled` (`src/rust/crates/fathomdb-engine/src/search/fusion_stage.rs`):**
   - Inside both the `use_graph_arm` and non-graph branches:
     - When `importance_enabled == true`: run `build_importance_confidence_maps(&tx, &fused)?` **before** `apply_record_importance_reweight` (unchanged ranking and error semantics).
     - When `importance_enabled == false`: skip `build_importance_confidence_maps` before `results.truncate(final_limit)` and pass empty maps (or skip `apply_record_importance_reweight` as a no-op).
     - **Immediately after `results.truncate(final_limit)`**, when `!importance_enabled && track_contributions`: run `let (imp_map, mut conf_map) = build_importance_confidence_maps(&tx, &results)?;`, merge any `graph_edge_confidence` entries for surviving `results`, and store into `exp_importance` / `exp_confidence`.
   - This reduces `SELECT importance`/`SELECT confidence` lookups from `O(total_fused_candidates)` to `O(final_limit)` while propagating `rusqlite::Error` -> `EngineError::Storage` on any row decode failure for all surviving post-truncation hits (`<= final_limit`).

### 2.3 `empty_physical_fast_path` Internal `node_sql` Skip (`AC32-QUERY-SCALE-03`)

In `empty_physical_fast_path` in `src/rust/crates/fathomdb-engine/src/projection_generation.rs`:

- Before preparing/running `node_sql`, check in `O(1)`:

```rust
let has_explicit_node_enrolment: bool = connection
    .prepare_cached(
        "SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds WHERE kind <> 'edge_fact' LIMIT 1)",
    )
    .and_then(|mut stmt| stmt.query_row([], |row| row.get(0)))
    .map_err(|_| EngineError::Storage)?;
let (node_count, first_node, unenrolled_nodes): (u64, Option<u64>, u64) =
    if node_arm_declared || has_explicit_node_enrolment {
        // existing node_sql query unchanged
    } else {
        (0, None, 0)
    };
```

- **Equivalence proof:** Look at `node_sql`'s `WHERE` predicate: `AND (?1!=0 OR vk.kind IS NOT NULL)` where `?1 = node_arm_declared` and `vk` is `LEFT JOIN _fathomdb_vector_kinds vk ON vk.kind = n.kind AND vk.kind <> 'edge_fact'`. When `!node_arm_declared && !has_explicit_node_enrolment`, `?1 == 0` AND `vk.kind` is `NULL` for every row, so `node_sql` is algebraically guaranteed to return `(0, None, 0)`.
- The subsequent `edge_count` / `first_edge` query, `edge_enrolled` lookup, and `if unenrolled_nodes != 0 || (edge_count != 0 && !edge_enrolled) { return Err(corruption()); }` check remain **100% verbatim**.

### 2.4 `store_single_unscoped` Range Probe & `detect_vector_soft_fallback` `O(1)` Short-Circuit (`AC32-QUERY-SCALE-04`)

1. **`store_single_unscoped` (`src/rust/crates/fathomdb-engine/src/search/fts_fast_path.rs`):**
   - Compute the cheap boolean guards (`can_defer_text_identity_cheap`, i.e., `scope.is_unscoped() && query_vector.is_some() && filter.is_none() && perf_limit.is_none() && fts_only_limit.is_none() && !has_source_dependencies && !view.view.include_superseded && !view.view.include_inactive && !view.view.include_out_of_window && !recency_enabled && !importance_enabled && rerank_depth == 0 && !use_graph_arm && !track_contributions && !force_full_sort`) **before** querying SQLite.
   - Only when `can_defer_text_identity_cheap` is `true`, evaluate `store_single_unscoped` via `prepare_cached`:

   ```sql
   SELECT NOT (
       EXISTS(SELECT 1 FROM canonical_nodes WHERE scope < '' LIMIT 1)
       OR EXISTS(SELECT 1 FROM canonical_nodes WHERE scope > '' LIMIT 1)
   )
   ```

   - Verified on Schema 39 SQLite 3.45.1 (`EXPLAIN QUERY PLAN`): both subqueries execute `SEARCH canonical_nodes USING COVERING INDEX canonical_nodes_scope_cursor_idx (scope<?)` and `(scope>?)` — strictly `O(log N)` `SEARCH`-only with zero `SCAN`.
2. **`detect_vector_soft_fallback` (`src/rust/crates/fathomdb-engine/src/search/vector_arm.rs`):**
   - Pass `has_vector_kinds: bool` (from `SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds LIMIT 1)`) into `detect_vector_soft_fallback`.
   - When `!vector_eligibility_degraded && query_vector.is_some() && !vector_rows_visible && !has_vector_kinds`, immediately short-circuit to `Ok(None)` without executing the `search_index JOIN canonical_nodes JOIN _fathomdb_vector_kinds LEFT JOIN _fathomdb_projection_terminal` query (when `_fathomdb_vector_kinds` is empty, that inner join is algebraically empty).

### 2.5 Snapshot Pinning, `historical_dense_incomplete` Short-Circuit, and Single-Pass SHA-256 (`AC32-QUERY-SCALE-05`)

1. **Snapshot pinning in `O(1)` (`frozen_read.rs` & `graph_expand/execution.rs`):**
   - In `mint_inner` in `src/rust/crates/fathomdb-engine/src/frozen_read.rs`, replace `tx.query_row("SELECT COUNT(*) FROM canonical_nodes", ...)` with:

   ```rust
   tx.prepare_cached(
       "SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton = 1",
   )
   .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
   .map_err(|_| EngineError::Storage)?;
   ```

   - In `read_graph_expand_with_stop_words_in_tx` in `src/rust/crates/fathomdb-engine/src/graph_expand/execution.rs` (`else` branch for `GraphReadContextV1::Current`), replace `tx.query_row("SELECT COUNT(*) FROM canonical_nodes", ...)` with the identical `O(1)` singleton lookup on `_fathomdb_read_visibility_state WHERE singleton = 1`. Both pin SQLite's `BEGIN DEFERRED` read snapshot in `O(1)` (1 B-tree PK lookup instead of an `O(N)` table/index count).
2. **`historical_dense_incomplete` short-circuit (`src/rust/crates/fathomdb-engine/src/search/fusion_stage.rs`):**
   - Replace:

   ```rust
   let history_incomplete = if (view.view.include_superseded || view.view.include_inactive)
       && direct_text_candidate_limit.is_none()
   {
       let incomplete = historical_dense_incomplete(&tx, scope, view)?;
       query_vector.is_none() || incomplete
   } else {
       false
   };
   ```

   with:

   ```rust
   let history_incomplete = if (view.view.include_superseded || view.view.include_inactive)
       && direct_text_candidate_limit.is_none()
   {
       query_vector.is_none() || historical_dense_incomplete(&tx, scope, view)?
   } else {
       false
   };
   ```

   When `query_vector.is_none()`, the `||` short-circuits immediately (`true`) without scanning `canonical_nodes` / `canonical_edges` in `historical_dense_incomplete`.
3. **Single-pass `source_body` SHA-256 (`evidence/ranked.rs`, `evidence/graph.rs`, `dependency.rs`):**
   - Currently `build_search_result` (`validate_source_bytes` -> `validate_full_provenance`), `resolve_single_ref` (`canonical_body_hash` -> `validate_full_provenance`), and `resolve_graph_evidence` (`validate_source_bytes` -> `validate_full_provenance`) hash `stored.source_body` twice: once directly and a second time inside `load_persisted_canonical_source` or `validate_dependency_chain` in `src/rust/crates/fathomdb-engine/src/dependency.rs`.
   - Introduce an internal helper struct in `src/rust/crates/fathomdb-engine/src/dependency.rs`:

   ```rust
   pub(crate) struct VerifiedSourceBody<'a> {
       pub(crate) body: &'a str,
       pub(crate) sha256_hex: &'a str,
   }
   ```

   - Add `pub(crate) fn load_persisted_canonical_source_with_verified_hash(connection: &Connection, scope: &str, source_revision: &str, verified: Option<VerifiedSourceBody<'_>>) -> Result<Option<ProspectiveCanonicalSource>, EngineError>` and `pub(crate) fn validate_dependency_chain_with_verified_hash(..., verified: Option<VerifiedSourceBody<'_>>) -> Result<(), EngineError>`.
   - Inside both functions: if `let Some(v) = verified` AND `v.body == canonical_body`, compare `hash == v.sha256_hex` directly (`0` extra SHA-256 calls); if `verified` is `None` or `v.body != canonical_body` (defensive mismatch check), compute `canonical_body_hash(&canonical_body)` and compare.
   - Keep existing `load_persisted_canonical_source` and `validate_dependency_chain` delegating with `verified = None` (zero behavioral change for all non-evidence callers).
   - Update `validate_full_provenance` in `src/rust/crates/fathomdb-engine/src/evidence/ranked.rs` to accept `verified_sha256: &str` and pass `Some(VerifiedSourceBody { body: &stored.source_body, sha256_hex: verified_sha256 })`, computing `canonical_body_hash(&stored.source_body)` exactly **once** in `build_search_result`, `resolve_single_ref`, and `resolve_graph_evidence`. Widen `CANONICAL_BODY_HASH_CALLS` in `src/rust/crates/fathomdb-engine/src/write_commit.rs` from `#[cfg(test)]` to `#[cfg(any(test, feature = "test-hooks"))]` with a `#[doc(hidden)] pub fn canonical_body_hash_calls_for_test() -> usize` accessor so integration tests under `required-features = ["test-hooks"]` (as well as in-crate unit tests) can assert exact single-pass hashing directly.

### 2.6 `read.get` & `read.get_many` Zero-Schema-Change Option 2B `UNION ALL` (`AC32-QUERY-SCALE-06`)

In `read_get_by_id_in_tx` in `src/rust/crates/fathomdb-engine/src/read.rs`:

1. **Why the current query scans `canonical_nodes`:**
   - Schema 39 provides two partial indexes:
     - `canonical_nodes_scope_logical_active_idx ON canonical_nodes(scope, logical_id) WHERE superseded_at IS NULL`
     - `canonical_nodes_scope_logical_sup_idx ON canonical_nodes(scope, logical_id, superseded_at, write_cursor) WHERE superseded_at IS NOT NULL`
   - When `view.include_superseded == true`, `view.node_sql(...)` omits `superseded_at IS NULL`, so neither partial index can satisfy the query; SQLite falls back to `SCAN canonical_nodes` plus `USE TEMP B-TREE FOR ORDER BY` for `ORDER BY write_cursor`.
   - Furthermore, `view.node_sql(...)` unconditionally injects `dependency_closure::read_eligibility_sql`, executing two correlated `NOT EXISTS` subqueries per candidate even when `_fathomdb_source_dependencies` is empty in `scope`.
2. **Exact Option 2B SQL & Rust implementation (`src/rust/crates/fathomdb-engine/src/read.rs`):**
   - Check `let has_source_dependencies = crate::dependency_closure::has_source_dependencies(&tx, scope)?;`.
   - Build common clauses using numbered placeholders (`?1..`):
     - `scope_sql` via `scope.append_sql_values("canonical_nodes.scope", &mut scope_sql, &mut binds)`
     - `placeholders` for deduplicated `logical_ids` (`?{lid_start}..`)
     - `state_sql = if view.include_inactive { "" } else { " AND canonical_nodes.state = 'active'" }`
     - `now_idx = binds.len() + 1; let validity_sql = view.validity_sql("canonical_nodes", now_idx);` (pushing `now` to `binds` iff `view.now_param()` is `Some(now)`)
     - `dep_sql`: if `has_source_dependencies`, call `crate::dependency_closure::read_eligibility_sql("canonical_nodes", view.include_superseded, view.include_inactive, view.include_out_of_window, now_idx)`; else `String::new()`.
   - Construct SQL with **zero `ORDER BY write_cursor`**:
     - If `!view.include_superseded`:

     ```sql
     SELECT logical_id, kind, body, write_cursor, properties
     FROM canonical_nodes
     WHERE {scope_sql} AND logical_id IN ({placeholders})
       AND canonical_nodes.superseded_at IS NULL{state_sql}{validity_sql}{dep_sql}
     ```

     - If `view.include_superseded`:

     ```sql
     SELECT logical_id, kind, body, write_cursor, properties
     FROM canonical_nodes
     WHERE {scope_sql} AND logical_id IN ({placeholders})
       AND canonical_nodes.superseded_at IS NULL{state_sql}{validity_sql}{dep_sql}
     UNION ALL
     SELECT logical_id, kind, body, write_cursor, properties
     FROM canonical_nodes
     WHERE {scope_sql} AND logical_id IN ({placeholders})
       AND canonical_nodes.superseded_at IS NOT NULL{state_sql}{validity_sql}{dep_sql}
     ```

     *(Numbered `?N` placeholders allow both `UNION ALL` legs to share the single `binds` slice without duplicating bound values.)*
   - Perform deterministic max-`write_cursor` tiebreaking in Rust:

   ```rust
   for row in rows {
       let record = row?;
       match found.entry(record.logical_id.clone()) {
           std::collections::hash_map::Entry::Occupied(mut existing) => {
               if record.write_cursor > existing.get().write_cursor {
                   existing.insert(record);
               }
           }
           std::collections::hash_map::Entry::Vacant(vacant) => {
               vacant.insert(record);
           }
       }
   }
   ```

   - **Verified `EXPLAIN QUERY PLAN` (both single-scope and multi-scope `IN (?, ?)`):**
     - `COMPOUND QUERY`
     - `LEFT-MOST SUBQUERY`: `SEARCH canonical_nodes USING INDEX canonical_nodes_scope_logical_active_idx (scope=? AND logical_id=?)`
     - `UNION ALL`: `SEARCH canonical_nodes USING INDEX canonical_nodes_scope_logical_sup_idx (scope=? AND logical_id=? AND superseded_at>?)`
     - Zero `SCAN canonical_nodes` and zero `USE TEMP B-TREE FOR ORDER BY`.
   - Expose a `#[cfg(feature = "test-hooks")]` helper (or query-builder helper `read_get_by_id_sql_for_test`) so integration tests can execute `EXPLAIN QUERY PLAN` directly on the exact production SQL.

---

## 3. `WP-41C`: Artifact-Scoped `property_predicates` on Evidence `source_filter` (`#269` Option A, `AC32-EVIDENCE-PROP-01..03`)

### 3.1 Exact 3 Code Sites

Under `ADR #103` §2.1, `SearchFilter.property_predicates` is an artifact-scoped domain selector (like `kind` and `source_type`), while Schema 39 `ScopePredicate`, access-bearing `attributes`, `created_after`, `status`, `ReadView` validity/lifecycle, and `source_dependencies` closures remain strictly source-enforced.

Add `source_filter.property_predicates.clear();` right alongside `source_filter.kind = None; source_filter.source_type = None;` at all three source-authorization sites:

1. `authorize_source_access` in `src/rust/crates/fathomdb-engine/src/evidence/ranked.rs`:

   ```rust
   let mut source_filter = eligibility.clone();
   source_filter.kind = None;
   source_filter.source_type = None;
   source_filter.property_predicates.clear();
   ```

2. `query_graph_evidence_batch` in `src/rust/crates/fathomdb-engine/src/evidence/graph.rs`:

   ```rust
   let mut source_filter = frozen.context.eligibility.clone();
   source_filter.kind = None;
   source_filter.source_type = None;
   source_filter.property_predicates.clear();
   ```

3. `resolve_graph_evidence` in `src/rust/crates/fathomdb-engine/src/evidence/graph.rs`:

   ```rust
   let mut source_filter = frozen.context.eligibility.clone();
   source_filter.kind = None;
   source_filter.source_type = None;
   source_filter.property_predicates.clear();
   ```

**Invariants preserved:**

- `encode_context` in `src/rust/crates/fathomdb-engine/src/frozen_read.rs` is unchanged: `property_predicates` remains part of `context_digest` HMAC authentication.
- `trace_dependency` in `src/rust/crates/fathomdb-engine/src/dependency_trace.rs` is **intentionally unchanged** (`AC32-EVIDENCE-PROP-03`): its SQL candidate pre-queries omit `property_predicates`, while Rust `authorize_candidate_chain` and `include_candidate` invoke `visible_artifact` (delegating to `text_hit_passes_filter_view` / `edge_fts_hit_passes_filter`) with the full `context.eligibility` on both `source_revision_id` and `derived_revision_id`, preserving `ADR-0.8.25 #49` symmetric endpoint visibility.

### 3.2 Test Updates & Documentation Touchpoints

- **Existing test extension (`src/rust/crates/fathomdb-engine/tests/slice38_historical_graph.rs`):**
  - Currently `src-good` carries both `tenant = "acme"` and `score = 50`, coupling attribute and property enforcement.
  - Decouple them: give `src-good` `tenant = "acme"` **without** `score` (or `score = 5`, below `score >= 40`), keep `target-good` carrying `tenant = "acme"` and `score = 50`, keep `src-bad` failing access-bearing `tenant = "other"` (`EvidenceUnavailable`), and add a target failing `score >= 40` backed by `src-good` (excluded / `EvidenceUnavailable`).
- **New cross-binding tests (`AC32-EVIDENCE-PROP-01..03`):**
  - Rust: `src/rust/crates/fathomdb-engine/tests/slice41_evidence_property_predicates.rs` covering `search_with_evidence`, `resolve_evidence`, `resolve_evidence_many`, `graph_expand(include_evidence = true)` + `resolve_graph_evidence`, attribute-mismatch nondisclosure (`EvidenceUnavailable` at `/evidenceRef`), cross-scope isolation, and `trace_dependency` symmetric endpoint preservation.
  - Python: `src/python/tests/test_slice41_evidence_property_predicates.py`.
  - TypeScript: `src/ts/tests/slice41-evidence-property-predicates.test.ts` (matching `src/ts/tsconfig.json` and `src/ts/package.json`).
- **Documentation updates:**
  - `dev/interfaces/rust.md`, `dev/interfaces/rust-sdk.md`, `dev/interfaces/python.md`, `dev/interfaces/typescript.md`, `dev/interfaces/wire.md`, and `docs/` pages for evidence/frozen-reads: document `#269` Option A (artifact-scoped `property_predicates` vs. source-enforced `attributes`/`ScopePredicate`), `trace_dependency` intentional symmetry, and `GraphArm` `SearchHit.source_id` (traversed edge) vs. `ResolvedEvidenceV1.source_id` (returned node artifact canonical provenance, `ADR #103` §2.2).

---

## 4. `WP-41D`: Schema 39 Partitioned Frozen-Read Additive Stability (`#267` Part 3, `ADR #103`, `AC32-FROZEN-STAB-01..06`)

### 4.1 Exact `STEP_39_SQL` In-Place Extension & All 20 Trigger-Bearing Tables

Extend `STEP_39_SQL` in `src/rust/crates/fathomdb-schema/src/lib.rs` **in place** (`SCHEMA_VERSION` stays `39`):

#### 4.1.1 Pre-Section-14 Trigger Cleanup & Section 14 DDL

1. **Remove duplicate pre-Section-14 trigger creation in `STEP_39_SQL` Sections 3, 6, 7, 8, 9, 10, 12:**
   - Sections 3 (`os`), 6 (`ar`), 7 (`sv`), 8 (`sl`), 9 (`sd`), 10 (`dc`), and 12 (`pr`) recreate those tables; omit their `visibility_coalescing_trigger_sql!` lines so Section 14 creates all 60 Schema 39 triggers once cleanly after every table carries its Schema 39 `scope` column.
2. **Append Section 14 to `STEP_39_SQL`:**

```sql
-- 14. Partitioned frozen-read visibility watermarks (ADR-0.8.32 #103).
ALTER TABLE _fathomdb_read_visibility_state
    ADD COLUMN global_struct_gen INTEGER NOT NULL DEFAULT 0
    CHECK(global_struct_gen BETWEEN 0 AND 9223372036854775807);
ALTER TABLE _fathomdb_read_visibility_state
    ADD COLUMN global_struct_nonce TEXT NOT NULL DEFAULT '';
UPDATE _fathomdb_read_visibility_state
   SET global_struct_nonce = lower(hex(randomblob(32)))
 WHERE singleton = 1;

CREATE TABLE _fathomdb_scope_visibility_watermarks(
    scope TEXT NOT NULL,
    partition_kind TEXT NOT NULL,
    add_generation INTEGER NOT NULL DEFAULT 0 CHECK(add_generation BETWEEN 0 AND 9223372036854775807),
    max_cursor INTEGER NOT NULL DEFAULT 0 CHECK(max_cursor >= 0),
    destructive_generation INTEGER NOT NULL DEFAULT 0 CHECK(destructive_generation BETWEEN 0 AND 9223372036854775807),
    state_nonce TEXT NOT NULL CHECK(length(state_nonce) = 64),
    PRIMARY KEY(scope, partition_kind)
) WITHOUT ROWID;
```

#### 4.1.2 Schema 39 Trigger SQL Macros & Single-Source Manifest Generator (`src/rust/crates/fathomdb-schema/src/lib.rs`)

- To guarantee `STEP_39_SQL` and `frozen_read::validate_on_open` can **never** drift, add a `pub enum VisibilityTriggerClass` on `TableManifestEntry` and export `pub fn expected_visibility_trigger_sql(entry: &TableManifestEntry, suffix: &str, operation: &str, schema_version: u32) -> String` from `fathomdb-schema` (matching the exact `concat!` SQL emitted in `STEP_39_SQL`, verified by a unit test in `fathomdb-schema`!).
- Every Schema 39 trigger shares the identical Step-37 coalescing gate and admission guard:
  - `WHEN NOT EXISTS(SELECT 1 FROM _fathomdb_read_visibility_state WHERE singleton=1 AND typeof(coalescing)='integer' AND coalescing=1)`
  - `BEGIN SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM _fathomdb_read_visibility_state WHERE singleton=1 AND typeof(coalescing)='integer' AND coalescing=0) THEN RAISE(ABORT,'read visibility coalescing state unavailable') END;`
- Every Schema 39 trigger updates global `_fathomdb_read_visibility_state.(generation, state_nonce)` (plus `global_struct_gen, global_struct_nonce` for global/row-owned classes) and executes unambiguous `INSERT INTO _fathomdb_scope_visibility_watermarks(...) VALUES(...) ON CONFLICT(scope, partition_kind) DO UPDATE SET ...` upserts (verified syntax-valid on SQLite 3.45.1!).

**Exact SQLite UPSERT fragment for additive watermark bump (`upsert_add(scope_expr, pk_expr, cursor_expr)`):**

```sql
INSERT INTO _fathomdb_scope_visibility_watermarks(scope,partition_kind,add_generation,max_cursor,destructive_generation,state_nonce)
VALUES(<scope_expr>,<pk_expr>,1,<cursor_expr>,0,lower(hex(randomblob(32))))
ON CONFLICT(scope,partition_kind) DO UPDATE SET
  add_generation=CASE WHEN add_generation=9223372036854775807 THEN RAISE(ABORT,'read visibility generation exhausted') ELSE add_generation+1 END,
  max_cursor=MAX(max_cursor,excluded.max_cursor),
  state_nonce=lower(hex(randomblob(32)));
```

**Exact SQLite UPSERT fragment for destructive watermark bump (`upsert_dest(scope_expr, pk_expr)`):**

```sql
INSERT INTO _fathomdb_scope_visibility_watermarks(scope,partition_kind,add_generation,max_cursor,destructive_generation,state_nonce)
VALUES(<scope_expr>,<pk_expr>,0,0,1,lower(hex(randomblob(32))))
ON CONFLICT(scope,partition_kind) DO UPDATE SET
  destructive_generation=CASE WHEN destructive_generation=9223372036854775807 THEN RAISE(ABORT,'read visibility generation exhausted') ELSE destructive_generation+1 END,
  state_nonce=lower(hex(randomblob(32)));
```

**Exact SQLite UPDATE fragment for global structural bump (`update_global_struct`):**

```sql
UPDATE _fathomdb_read_visibility_state
SET generation=CASE WHEN generation=9223372036854775807 THEN RAISE(ABORT,'read visibility generation exhausted') ELSE generation+1 END,
    state_nonce=lower(hex(randomblob(32))),
    global_struct_gen=CASE WHEN global_struct_gen=9223372036854775807 THEN RAISE(ABORT,'read visibility generation exhausted') ELSE global_struct_gen+1 END,
    global_struct_nonce=lower(hex(randomblob(32)))
WHERE singleton=1;
```

**Complete 20-Table Trigger Classification Matrix (`60` triggers total in Section 14, preceded by `DROP TRIGGER IF EXISTS` for all `60`):**

| Class | Tables (`prefix`) | `AFTER INSERT` (`_ai`, `coalescing = 0`) | `AFTER UPDATE` (`_au`, `coalescing = 0`) | `AFTER DELETE` (`_ad`, `coalescing = 0`) |
| --- | --- | --- | --- | --- |
| **1A. Scoped Canonical Nodes** | `canonical_nodes` (`cn`) | Global `(gen, nonce)`; `upsert_add(NEW.scope, 'node:' \|\| NEW.kind, NEW.write_cursor)`; `upsert_add(NEW.scope, 'node_all', NEW.write_cursor)` | Global `(gen, nonce)`; `upsert_dest(OLD.scope, 'scope_destructive')`; `upsert_dest(NEW.scope, 'scope_destructive')`; `upsert_add(OLD.scope, 'node:' \|\| OLD.kind, OLD.write_cursor)`; `upsert_add(NEW.scope, 'node:' \|\| NEW.kind, NEW.write_cursor)`; `upsert_add(OLD.scope, 'node_all', OLD.write_cursor)`; `upsert_add(NEW.scope, 'node_all', NEW.write_cursor)` | Global `(gen, nonce)`; `upsert_dest(OLD.scope, 'scope_destructive')`; `upsert_add(OLD.scope, 'node:' \|\| OLD.kind, 0)`; `upsert_add(OLD.scope, 'node_all', 0)` |
| **1B. Scoped Canonical Edges** | `canonical_edges` (`ce`) | Global `(gen, nonce)`; `upsert_add(NEW.scope, 'edge:' \|\| NEW.kind, NEW.write_cursor)`; `upsert_add(NEW.scope, 'edge_any', NEW.write_cursor)` | Global `(gen, nonce)`; `upsert_dest(OLD.scope, 'scope_destructive')`; `upsert_dest(NEW.scope, 'scope_destructive')`; `upsert_add(OLD.scope, 'edge:' \|\| OLD.kind, OLD.write_cursor)`; `upsert_add(NEW.scope, 'edge:' \|\| NEW.kind, NEW.write_cursor)`; `upsert_add(OLD.scope, 'edge_any', OLD.write_cursor)`; `upsert_add(NEW.scope, 'edge_any', NEW.write_cursor)` | Global `(gen, nonce)`; `upsert_dest(OLD.scope, 'scope_destructive')`; `upsert_add(OLD.scope, 'edge:' \|\| OLD.kind, 0)`; `upsert_add(OLD.scope, 'edge_any', 0)` |
| **1C. Scoped Provenance** | `_fathomdb_artifact_revisions` (`ar`), `_fathomdb_source_versions` (`sv`), `_fathomdb_source_links` (`sl`) | Global `(gen, nonce)`; `upsert_dest(NEW.scope, 'scope_destructive')`; `upsert_add(NEW.scope, 'node_all', 0)`; `upsert_add(NEW.scope, 'edge_any', 0)` *(fails closed across all read surfaces in `NEW.scope` on raw provenance inserts while isolating other scopes)* | Global `(gen, nonce)`; `upsert_dest(OLD.scope, 'scope_destructive')`; `upsert_dest(NEW.scope, 'scope_destructive')`; `upsert_add(OLD.scope, 'node_all', 0)`; `upsert_add(NEW.scope, 'node_all', 0)`; `upsert_add(OLD.scope, 'edge_any', 0)`; `upsert_add(NEW.scope, 'edge_any', 0)` | Global `(gen, nonce)`; `upsert_dest(OLD.scope, 'scope_destructive')`; `upsert_add(OLD.scope, 'node_all', 0)`; `upsert_add(OLD.scope, 'edge_any', 0)` |
| **1D. Scoped Dependencies** | `_fathomdb_source_dependencies` (`sd`), `_fathomdb_dependency_closures` (`dc`) | Global `(gen, nonce)`; `upsert_add(NEW.scope, 'scope_dependency', 0)` | Global `(gen, nonce)`; `upsert_add(OLD.scope, 'scope_dependency', 0)`; `upsert_add(NEW.scope, 'scope_dependency', 0)`; `upsert_dest(OLD.scope, 'scope_destructive')`; `upsert_dest(NEW.scope, 'scope_destructive')` | Global `(gen, nonce)`; `upsert_add(OLD.scope, 'scope_dependency', 0)`; `upsert_dest(OLD.scope, 'scope_destructive')` |
| **1E. Scoped Operational State** | `operational_state` (`os`) | Global `(gen, nonce)`; `upsert_add(NEW.scope, 'op_state', NEW.write_cursor)` *(never bumps `scope_destructive`)* | Global `(gen, nonce)`; `upsert_add(OLD.scope, 'op_state', OLD.write_cursor)`; `upsert_add(NEW.scope, 'op_state', NEW.write_cursor)` *(never bumps `scope_destructive`)* | Global `(gen, nonce)`; `upsert_add(OLD.scope, 'op_state', 0)` *(never bumps `scope_destructive`)* |
| **2. Row-Owned Projection / Property (5 tables)** | `canonical_attributes` (`ca`), `canonical_property_values` (`cpv`), `_fathomdb_vector_rows` (`vr`), `_fathomdb_vector_spans` (`vs`), `_fathomdb_projection_terminal` (`pt`) | `update_global_struct` *(fail closed on raw uncoalesced SQL; engine paths run under `VisibilityCoalescingGuard` and attribute watermarks to owning scope)* | `update_global_struct` | `update_global_struct` |
| **3. Global Metadata (7 tables)** | `operational_collections` (`oc`), `_fathomdb_projection_registry` (`pr`), `_fathomdb_vector_kinds` (`vk`), `_fathomdb_embedder_profiles` (`ep`), `_fathomdb_projection_generations` (`pg`), `_fathomdb_projection_generation_current` (`pc`), `_fathomdb_projection_state` (`ps`) | `update_global_struct` *(under coalesced paths, `ps` updates are attributed to `(scope, 'vector_ready:*')`, whereas `ep.mean_vec` pin / `vk` enrolment / `pr` / `pg` / `pc` bump `global_struct`)* | `update_global_struct` | `update_global_struct` |

**Note on `OLD.scope != NEW.scope` / `OLD.kind != NEW.kind`:** Every Class 1 `AFTER UPDATE` (`_au`) trigger in rows `1A`–`1E` above emits paired `OLD.*` and `NEW.*` `VALUES(...) ON CONFLICT(...) DO UPDATE` statements for every touched partition (`scope_destructive`, `node:`/`edge:`, `node_all`, `edge_any`, `scope_dependency`, `op_state`), handling raw SQL `UPDATE ... SET scope = 'other'` or `UPDATE ... SET kind = 'other'` atomically without SQLite `SELECT ... WHERE ... ON CONFLICT` grammar ambiguity and drifting both source and destination partitions.

### 4.2 Manifest, Admission, Bootstrap, and Open-Validation Parity

1. **`src/rust/crates/fathomdb-schema/src/lib.rs`:**
   - Append `_fathomdb_scope_visibility_watermarks` to `TABLE_MANIFEST` (`is_canonical: false`, `visibility_trigger_prefix: None`, `visibility_trigger_min_version: None`, `row_owned_cursor_column: None`, `has_non_projection_write_cursor: false`, `export_class: TableExportClass::InternalRuntime`, `guard_canonical_custom_triggers: true`, `guard_projection_custom_triggers: true`, `physical_erasure_metadata: false`).
   - Update doc comment count (`36 named tables: 31 regular + 5 virtual`).
2. **`src/rust/crates/fathomdb-engine/src/export_manifest.rs`:**
   - Automatically inherits `_fathomdb_scope_visibility_watermarks -> ExportClass::InternalRuntime` via `TABLE_MANIFEST` (verify existing audit tests pass).
3. **`src/inspector/src/admission.rs`:**
   - Update `manifest_columns` for `"_fathomdb_read_visibility_state"` to `"singleton,generation,state_nonce,coalescing,global_struct_gen,global_struct_nonce"`.
   - Add `"_fathomdb_scope_visibility_watermarks" => "scope,partition_kind,add_generation,max_cursor,destructive_generation,state_nonce"`.
   - Update `validate_layout`'s visibility row query to additionally verify `AND global_struct_gen>=0 AND length(global_struct_nonce)=64 AND global_struct_nonce NOT GLOB '*[^0-9a-f]*'`.
4. **`src/rust/crates/fathomdb-engine/src/projection_generation.rs`:**
   - In `bootstrap_is_fresh`: add `if schema_version >= 39 && count(connection, "_fathomdb_scope_visibility_watermarks")? != 0 { return Ok(false); }` (fresh Schema 39 bootstrap leaves `_fathomdb_scope_visibility_watermarks` empty before `bootstrap` creates `_fathomdb_projection_generations`).
5. **`src/rust/crates/fathomdb-engine/src/frozen_read.rs` (`validate_on_open`):**
   - For `schema_version >= 39`:
     - Verify `_fathomdb_read_visibility_state` has `global_struct_gen` (`>= 0`) and `global_struct_nonce` (`is_lower_hex(&nonce, 64)`).
     - Verify `_fathomdb_scope_visibility_watermarks` exists in `sqlite_master` (`type = 'table'`) and passes `SELECT scope,partition_kind,add_generation,max_cursor,destructive_generation,state_nonce FROM _fathomdb_scope_visibility_watermarks LIMIT 0`.
     - Compare all `60` triggers (`trigger_table_count * 3`) against `fathomdb_schema::expected_visibility_trigger_sql(entry, suffix, operation, schema_version)` (preserving exact Step 31/33/37 trigger expectations for `schema_version < 39`).

### 4.3 Watermark Upsert Helpers & Complete Engine Write-Path Audit

Add a focused internal helper set (in `src/rust/crates/fathomdb-engine/src/write_commit.rs` or a new internal helper module `visibility_watermarks.rs` wired via `lib.rs` with synchronized `lib_rs_blob:`):

```rust
#[derive(Default)]
pub(crate) struct VisibilityDelta {
    pub(crate) additive: std::collections::BTreeMap<(String, String), u64>, // (scope, partition_kind) -> max_cursor
    pub(crate) destructive_scopes: std::collections::BTreeSet<String>,
    pub(crate) global_struct: bool,
    pub(crate) any_visibility_change: bool,
}

impl VisibilityDelta {
    pub(crate) fn record_add(&mut self, scope: &str, partition_kind: impl Into<String>, cursor: u64) { ... }
    pub(crate) fn record_destructive(&mut self, scope: &str) { ... }
    pub(crate) fn record_global_struct(&mut self) { ... }
    pub(crate) fn apply(&self, tx: &Connection) -> rusqlite::Result<()> { ... }
}
```

- `VisibilityDelta::apply(tx)` executes (only inside an active writer transaction while `coalescing = 1` or at commit end):
  1. For each `((scope, partition_kind), max_cursor)` in `self.additive`: runs `prepare_cached` `INSERT INTO _fathomdb_scope_visibility_watermarks(...) VALUES(?1,?2,1,?3,0,lower(hex(randomblob(32)))) ON CONFLICT(scope,partition_kind) DO UPDATE SET add_generation = ..., max_cursor = MAX(max_cursor, excluded.max_cursor), state_nonce = lower(hex(randomblob(32)))`.
  2. For each `scope` in `self.destructive_scopes`: runs `prepare_cached` destructive upsert on `(scope, "scope_destructive")`.
  3. If `self.global_struct`: updates `_fathomdb_read_visibility_state` incrementing both `(generation, state_nonce)` and `(global_struct_gen, global_struct_nonce)` and resetting `coalescing = 0`.
  4. Else if `self.any_visibility_change || !self.additive.is_empty() || !self.destructive_scopes.is_empty()`: calls `advance_read_visibility(tx)?` (increments `(generation, state_nonce)` and resets `coalescing = 0`, leaving `(global_struct_gen, global_struct_nonce)` untouched).

**Exhaustive Audit of Every Engine Write Path Mutating Any of the 20 Trigger Tables:**

| Write Path & File | Coalescing Strategy | Exact `VisibilityDelta` / Watermarks Emitted | Preserves `global_struct` Unchanged? |
| --- | --- | --- | --- |
| `commit_batch_transaction` (`write_commit.rs`) | Generalize `canonical_batch` to `coalesceable_batch = batch.iter().all(\|w\| !matches!(w, PreparedWrite::AdminSchema { .. }))` (covers canonical, `OpStore` `AppendOnlyLog`/`LatestState`, AND mixed `[Node/Edge, OpStore]` batches!). Gate on `canonical_batch_has_no_custom_triggers(&tx)?`. **Retain** inserting `('up_to_date', 'non_canonical')` into `_fathomdb_projection_terminal` and calling `advance_projection_cursor(&tx)` (zero deadlocks, preserves `EP-01` & `data_plane_integrity`). | Per write in `batch`: (1) `Node`/`ProvenancedNode`: `record_add(scope, format!("node:{kind}"), cursor)`, `record_add(scope, "node_all", cursor)`; if superseding any `prior_g0` row (`!prior_g0.is_empty()`), `record_destructive(scope)` and (if superseded row had a different `prior_kind`!) `record_add(scope, format!("node:{prior_kind}"), cursor)` + (if `admit_soft_closure` runs `apply_soft_effects` inline) `record_add(scope, "scope_dependency", 0)`, `record_add(scope, "edge_any", 0)`, `record_add(scope, "vector_ready_all", 0)`, and erased dependent kinds; (2) `Edge`/`ProvenancedEdge`: `record_add(scope, format!("edge:{kind}"), cursor)`, `record_add(scope, "edge_any", cursor)`; if superseding any `prior_g0`/`prior_g11` row, `record_destructive(scope)` + `record_add(scope, format!("edge:{prior_kind}"), cursor)`; (3) `OpStore` + `AppendOnlyLog`: `record_add(scope, format!("op_log:{collection}"), cursor)`; (4) `OpStore` + `LatestState`: `record_add(scope, "op_state", cursor)` (never `scope_destructive`!); (5) if `!vector_kinds_to_enrol.is_empty()` or `unstranded`, OR if `project_canonical_edge_row` (`index_projector.rs`, `INSERT OR IGNORE INTO _fathomdb_vector_kinds` for `'edge_fact'`) returns `changes > 0` (new vector kind row inserted): `record_global_struct()` (since `_fathomdb_vector_kinds` is global metadata!). | **Yes** for all canonical, `AppendOnlyLog`, `LatestState`, and mixed batches unless a new `_fathomdb_vector_kinds` row is inserted (`vk` enrolment). `AdminSchema` batches run uncoalesced so `oc` trigger bumps `global_struct`. |
| `actuate_new` (`actuation.rs`) | Gate on `canonical_batch_has_no_custom_triggers(&tx)?`. Arm `VisibilityCoalescingGuard::arm(&tx)?` BEFORE `SAVEPOINT fathomdb_actuation_domain`. On refusal (`ROLLBACK TO fathomdb_actuation_domain`): discard `delta`, call `trigger_guard.restore()?` (receipt tables have no visibility triggers!), commit refusal receipt. On commit: apply `delta` + `trigger_guard.restore()?` before `tx.commit()`. | Accumulate `VisibilityDelta` per operation: `PutCanonicalNode`/`PutDerivedNode` (additive `node:<kind>`, `node_all`, plus `scope_destructive` + prior kind if superseding, plus `scope_dependency`/`edge_any`/`vector_ready_all` if `admit_soft_closure` runs `apply_soft_effects` inline); `PutDerivedEdge` (`edge:<kind>`, `edge_any`, plus `scope_destructive` if superseding); `RegisterSourceDependency` (`scope_dependency`); `TransitionLifecycle` (`scope_destructive`, `node:<kind>`, `node_all`, plus `scope_dependency`/`edge_any`/`vector_ready_all` if `admit_soft_closure` runs `apply_soft_effects` inline, plus `vector_ready:<kind>`/`vector_ready_all` if `reopened`); `PutLatestState` (`op_state`, never `scope_destructive`); plus `record_global_struct()` if `!vector_kinds_to_enrol.is_empty()`, `unstranded`, `project_canonical_edge_row` inserts a new `'edge_fact'` row into `_fathomdb_vector_kinds` (`changes > 0`), or `restore_registered_derived_projections` (`index_projector.rs`) inserts a new row into `_fathomdb_vector_kinds` (`changes > 0`). | **Yes** (`AppTurn` actuation produces `0%` drift on `Doc` and isolated tenants!). |
| `Engine::transition` (`record_lifecycle.rs`) | Arm `VisibilityCoalescingGuard` gated on `canonical_batch_has_no_custom_triggers(&tx)?`. | `record_destructive(write_scope)`, `record_add(write_scope, format!("node:{kind}"), cursor)`, `record_add(write_scope, "node_all", cursor)`; if `admit_soft_closure` runs `apply_soft_effects` inline: `record_add(write_scope, "scope_dependency", 0)`, `record_add(write_scope, "edge_any", 0)`, `record_add(write_scope, "vector_ready_all", 0)`, and erased dependent kinds; if `reopened` (or vector rows refreshed): `record_add(write_scope, format!("vector_ready:{kind}"), cursor)` + `record_add(write_scope, "vector_ready_all", cursor)`; if `restore_registered_derived_projections` (`index_projector.rs`) inserts a new row into `_fathomdb_vector_kinds` (`changes > 0`): `record_global_struct()`. | **Yes** (`"tenant-b"` `transition` causes `0%` drift on `"tenant-a"`, unless a brand-new `_fathomdb_vector_kinds` row is enrolled!). |
| `commit_projection_outcomes` (`projection_commit.rs`) | Already armed with `VisibilityCoalescingGuard` gated on `projection_batch_has_no_custom_triggers(&tx)?`. | For each committed `Success` (`scope` already fetched via `vector_eligibility_and_scope_for_cursor`) and each committed `Failure` (`row_scope` already fetched, `kind` returned by `dense_member_kind_at`): `record_add(&scope, format!("vector_ready:{kind}"), *cursor)` and `record_add(&scope, "vector_ready_all", *cursor)`. When `pin_mean.is_some()` updates `_fathomdb_embedder_profiles.mean_vec`: `record_global_struct()`. Apply `delta` before `trigger_guard.restore()`. | **Yes** (unrelated `(scope, kind)` vector commits produce `0%` drift; `mean_vec` pin bumps `global_struct`!). |
| `purge_inner` (`erasure.rs`) | Arm `VisibilityCoalescingGuard` gated on `canonical_batch_has_no_custom_triggers(&tx)?`. Collect distinct erased node kinds and edge kinds from `affected_cursors` BEFORE deletes. | `record_destructive(scope)`, `record_add(scope, "scope_dependency", 0)`, `record_add(scope, "node_all", 0)`, `record_add(scope, "edge_any", 0)`, `record_add(scope, "vector_ready_all", 0)`, plus `node:<k>`/`vector_ready:<k>` for each erased node kind and `edge:<k>`/`vector_ready:edge_fact` for erased edges. | **Yes** (`"tenant-b"` `purge` causes `0%` drift on `"tenant-a"`!). |
| `excise_source_inner_with_prior` (`erasure.rs`) | Arm `VisibilityCoalescingGuard` gated on `canonical_batch_has_no_custom_triggers(&tx)?`. Collect erased node/edge kinds per `plan.scope` BEFORE deletes. | Same per-scope partition bumps as `purge_inner` for every `plan.scope` in `scope_plans`. | **Yes** (`"tenant-b"` `erase_source` causes `0%` drift on `"tenant-a"`!). |
| `finalize_soft_closure` (`dependency_closure.rs`) | Verified: `finalize_soft_closure` only executes `UPDATE _fathomdb_dependency_closures` (Class 1D, already scope-isolated under `coalescing = 0`) and never calls `apply_soft_effects`. `admit_soft_closure`'s `apply_soft_effects` (which mutates `canonical_nodes`/`canonical_edges` and calls `erase_row_projections`) executes inline inside the already-coalesced `commit_batch_transaction`, `actuate_new`, and `Engine::transition` transactions above. | Automatic via `dc` `_au` trigger (`scope_dependency` + `scope_destructive` for `OLD.scope`/`NEW.scope`, zero `global_struct` bump). | **Yes** (isolated to `scope`). |
| `physical_erasure::sanitize_unit` / `prepare_redacted` / `finalize` (`physical_erasure.rs`) | Verified uncoalesced: `sanitize_unit` merges FTS5 shadow tables + `_fathomdb_open_state`/`_fathomdb_erasure_requests` (0 visibility-trigger tables), `prepare_redacted` reads `_fathomdb_dependency_closures` and updates only `_fathomdb_erasure_requests` and `_fathomdb_open_state` (0 visibility-trigger tables), and `finalize` updates `_fathomdb_dependency_closures` (`dc`), whose `dc_au` trigger bumps `(OLD.scope, 'scope_dependency')`, `(NEW.scope, 'scope_dependency')`, `(OLD.scope, 'scope_destructive')`, and `(NEW.scope, 'scope_destructive')`. | Automatic via `dc_au` trigger on `finalize` (zero `global_struct` bump). | **Yes**. |
| `configure_projections`, `projection_rebuild`, `recompute_mean`, `boot_graft_declared_vector_backfill` (`projection_registry.rs`, `projection_rebuild.rs`, `mean.rs`) | Uncoalesced (`coalescing = 0`): triggers on `pr`, `vk`, `pg`, `pc`, `ep` fire and bump `global_struct`. In `projection_registry.rs` lexical-edge repair (where `advance_read_visibility` is called directly), call `bump_global_struct(connection)` instead. | `global_struct` bumped deterministically across all scopes. | Intentionally bumps `global_struct`. |
| `register_source_dependency` (`dependency.rs`), `write_node_importance` (`write.rs`), `consolidation.rs`, `excise_collection_record_inner` (`erasure.rs`) | Uncoalesced (`coalescing = 0`): touch only `scope`-bearing tables (`sd`, `cn`, `ce`, `os`) — triggers automatically bump only the owning `scope`'s partition (`scope_dependency`, `scope_destructive`+`node:*`, `scope_destructive`+`edge:*`, `op_state`). | Automatic via Class 1 triggers (zero `global_struct` bump). | **Yes**. |

### 4.4 `ReadSurface`, `FrozenReadBinding` (`0x53 0x39` Wire Codec), and `validate_snapshot` (`src/rust/crates/fathomdb-engine/src/frozen_read.rs`)

#### 4.4.1 Exact Types & Signatures

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReadSurface<'a> {
    Preflight,
    SearchNoGraph,
    GraphOrEdge,
    EvidenceNode,
    EvidenceGraph,
    CanonicalPage { path_kind: &'a str },
    OperationalState,
    TraceDependency,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SurfaceTags {
    pub(crate) op_state_tag: u64,
    pub(crate) node_all_tag: u64,
    pub(crate) projection_readiness_tag: u64,
    pub(crate) edge_kind_tag: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrozenReadBinding {
    database_id: String,
    effective_valid_at: i64,
    canonical_write_boundary: u64,
    read_visibility_generation: u64,
    dependency_generation: u64,
    projection_cursor: u64,
    registry_digest: [u8; 32],
    serving_digest: [u8; 32],
    context_digest: [u8; 32],
    scope_digest: Option<[u8; 32]>,
    surface_tags: Option<SurfaceTags>, // None on Schema < 39; Some(...) on Schema >= 39
}

pub(crate) fn validate_snapshot(
    connection: &Connection,
    context: &ReadContextV1,
    expected: &FrozenReadBinding,
    scope: &crate::scope::ScopePredicate,
    surface: ReadSurface<'_>,
) -> Result<u64, FrozenReadError>
```

#### 4.4.2 Exact Domain Separators & Watermark Folding (`Schema >= 39`)

- `const SERVING_DOMAIN_V4: &[u8] = b"fathomdb.projection-serving-binding.v4\0";`
- `const WATERMARK_TAG_DOMAIN_V1: &[u8] = b"fathomdb.visibility-watermark-tag.v1\0";`
- `const SURFACE_TAGS_MAGIC: [u8; 2] = [0x53, 0x39];` (`"S9"`)
- For any set of `(scope, partition_kind)` lookups over sorted `scope.read_scopes()`:
  - Query `_fathomdb_scope_visibility_watermarks WHERE scope = ?1 AND partition_kind = ?2` (cached PK point lookup on `WITHOUT ROWID` table!). Absent row encodes deterministically as `(add_generation = 0, max_cursor = 0, destructive_generation = 0, state_nonce = "")`.
  - Helper `fold_partition_tag(domain_label: &str, rows: &[(&str, &str, u64, u64, u64, &str)]) -> u64`: SHA-256 over `WATERMARK_TAG_DOMAIN_V1`, length-prefixed `domain_label`, and for each sorted `(scope, pk, add_gen, max_cursor, dest_gen, nonce)` tuple encode all 6 fields -> take `u64::from_be_bytes(digest[0..8])`.
- Exact Schema `39+` field computation inside `binding_for_snapshot(connection, context, effective_valid_at, context_digest, scope)`:
  - **`serving_digest` (`[u8; 32]`, universal across all 8 surfaces):** SHA-256 over `SERVING_DOMAIN_V4`, `(global_struct_gen, global_struct_nonce)` from `_fathomdb_read_visibility_state WHERE singleton = 1`, `current_generation` (`id`, `digest`, `boundary`, `"serving"`, `origin`), and sorted `(scope, "scope_destructive", add_gen, max_cursor, dest_gen, state_nonce)` for every `scope` in `scope.read_scopes()`. **Excludes** global `_fathomdb_projection_state`, global `state_nonce`, and all surface-selective partitions.
  - **`canonical_write_boundary` (`u64`) & `read_visibility_generation` (`u64`):** let `node_pk = match &context.eligibility.kind { Some(k) => format!("node:{k}"), None => "node_all".to_string() }`. Over `scope.read_scopes()`, `canonical_write_boundary = MAX(max_cursor)` of `(s, node_pk)` and `read_visibility_generation = fold_partition_tag("node", &node_rows)`.
  - **`dependency_generation` (`u64`):** `fold_partition_tag("scope_dependency", &dep_rows)` over `(s, "scope_dependency")`.
  - **`projection_cursor` (`u64`, carrying `edge_any_tag`):** `fold_partition_tag("edge_any", &edge_any_rows)` over `(s, "edge_any")`.
  - **`op_state_tag` (`u64`):** `fold_partition_tag("op_state", &op_state_rows)` over `(s, "op_state")`.
  - **`node_all_tag` (`u64`):** `fold_partition_tag("node_all", &node_all_rows)` over `(s, "node_all")` (reuses `node_rows` when `eligibility.kind.is_none()`!).
  - **`projection_readiness_tag` (`u64`):** let `vec_pk = match &context.eligibility.kind { Some(k) => format!("vector_ready:{k}"), None => "vector_ready_all".to_string() }`; `fold_partition_tag("vector_ready", &vec_rows)`.
  - **`edge_kind_tag` (`u64`):** let `edge_pk = match &context.eligibility.kind { Some(k) => format!("edge:{k}"), None => "edge_any".to_string() }`; `fold_partition_tag("edge_kind", &edge_kind_rows)` (reuses `edge_any_rows` when `eligibility.kind.is_none()`!).
- **Optimization on `validate_snapshot`:** Only query the partition rows actually required by `surface` (e.g. `OperationalState` queries only `_fathomdb_read_visibility_state`, `current_generation`, `scope_destructive`, and `op_state` — 2–3 PK point lookups -> `<= 0.008 ms` `p50` at `50k` rows!).
- **Monotonic Return Clock Contract:** BOTH `mint_inner` and `validate_snapshot` MUST return **`load_visibility_generation(connection)?`** (the global `_fathomdb_read_visibility_state.generation` monotonic clock), NEVER the partitioned `binding.read_visibility_generation` hash! This preserves `Engine::read_visibility_generation.fetch_max(generation, Ordering::AcqRel)` and `slice35_frozen_read_races.rs::consume_rejects_visibility_generation_regression_observed_in_process`.

#### 4.4.3 Exact `0x53 0x39` Wire Codec & Byte Offsets

| Segment | Offset (`Unscoped`) | Offset (`Scoped`) | Size (`B`) | Notes |
| --- | --- | --- | ---: | --- |
| `schema_version` (`u32` BE = `1`) | `0..4` | `0..4` | `4` | Unchanged |
| `database_id` (`u32` len + `32` hex ASCII) | `4..40` | `4..40` | `36` | Unchanged |
| `effective_valid_at` (`i64` BE) | `40..48` | `40..48` | `8` | Unchanged |
| `canonical_write_boundary` (`u64` BE) | `48..56` | `48..56` | `8` | Partitioned `node` max cursor on `Schema >= 39` |
| `read_visibility_generation` (`u64` BE) | `56..64` | `56..64` | `8` | Partitioned `node` tag on `Schema >= 39` |
| `dependency_generation` (`u64` BE) | `64..72` | `64..72` | `8` | `scope_dependency` tag on `Schema >= 39` |
| `projection_cursor` (`u64` BE) | `72..80` | `72..80` | `8` | `edge_any` tag on `Schema >= 39` |
| `registry_digest` (`[u8; 32]`) | `80..112` | `80..112` | `32` | Unchanged |
| `serving_digest` (`[u8; 32]`) | `112..144` | `112..144` | `32` | `SERVING_DOMAIN_V4` universal digest on `Schema >= 39` |
| `context_digest` (`[u8; 32]`) | `144..176` | `144..176` | `32` | Unchanged |
| Optional `scope_digest` (`[u8; 32]`) | omitted | `176..208` | `0` / `32` | Present iff `Scoped` |
| Extension magic `[0x53, 0x39]` (`"S9"`) | `176..178` | `208..210` | `2` | Present iff `Schema >= 39` |
| `op_state_tag` (`u64` BE) | `178..186` | `210..218` | `8` | Present iff `Schema >= 39` |
| `node_all_tag` (`u64` BE) | `186..194` | `218..226` | `8` | Present iff `Schema >= 39` |
| `projection_readiness_tag` (`u64` BE) | `194..202` | `226..234` | `8` | Present iff `Schema >= 39` |
| `edge_kind_tag` (`u64` BE) | `202..210` | `234..242` | `8` | Present iff `Schema >= 39` |
| **Raw payload / Hex `fdbfr1` token total** | **`210 B` / `492 B`** | **`242 B` / `556 B`** | — | Strictly `<= TOKEN_MAX_BYTES` (`1,024 B`); disjoint from legacy `< 39` trailing lengths (`0 B` / `32 B` -> `424 B` / `488 B`) |

- **Decoder dispatch on trailing bytes `rem = bytes.len() - cursor.offset` after `context_digest`:**
  - `0` -> `(scope_digest: None, surface_tags: None)` (legacy `< 39` unscoped)
  - `32` -> `(scope_digest: Some(array32), surface_tags: None)` (legacy `< 39` scoped)
  - `34` -> require `take(2) == [0x53, 0x39]`, read 4 `u64`s -> `(scope_digest: None, surface_tags: Some(...))`
  - `66` -> read `scope_digest`, require `take(2) == [0x53, 0x39]`, read 4 `u64`s -> `(scope_digest: Some(array32), surface_tags: Some(...))`
  - Any other `rem` or wrong magic bytes -> `FrozenReadErrorReason::TokenMalformed`.
- **Property-based codec round-trip (`frozen_read::tests::binding_codec_round_trips_canonically`):** Extend the existing `proptest!` strategy to generate arbitrary `surface_tags: Option<SurfaceTags>` (`None` and `Some(SurfaceTags { op_state_tag, node_all_tag, projection_readiness_tag, edge_kind_tag })`) alongside `scope_digest: Option<[u8; 32]>`, verifying canonical `encode_binding` / `decode_binding` round-trip and malformed-magic rejection across all four trailing-byte lengths (`0`, `32`, `34`, `66`).
- **Legacy vs. Schema 39 cross-check in `validate_snapshot`:**
  - When `PRAGMA user_version < 39`: run unchanged legacy global `binding_for_snapshot` and require `actual == *expected` (preserving Schema `31`/`32` normative fixtures bit-for-byte).
  - When `PRAGMA user_version >= 39`: if `expected.surface_tags.is_none()` (e.g., pre-upgrade token presented to upgraded Schema 39 DB), fail closed with `StateDrifted`. Otherwise check `database_id`, `effective_valid_at`, `context_digest`, `scope_digest`, `registry_digest`, `serving_digest` unconditionally, plus the exact surface-selective fields from the ADR `#103` Checked-Field Matrix (`OperationalState`: `op_state_tag`; `CanonicalPage`: `canonical_write_boundary` + `read_visibility_generation`; `Preflight`/`SearchNoGraph`/`EvidenceNode`: `canonical_write_boundary` + `read_visibility_generation` + `projection_readiness_tag` + `edge_kind_tag` + `dependency_generation`, plus `projection_cursor` and `node_all_tag` when `context.eligibility.kind.is_none()`; `GraphOrEdge`/`EvidenceGraph`: same as `SearchNoGraph` plus unconditional `projection_cursor`; `TraceDependency`: `node_all_tag` + `projection_cursor` + `dependency_generation`).

#### 4.4.4 Authoritative 13-Site `validate_snapshot` Call-Site Table

| # | Call Site | `ReadSurface` Supplied | Notes |
| --- | --- | --- | --- |
| 1 | `validate_frozen_read_context_for_binding` (`search_api.rs`) | `ReadSurface::Preflight` | Preflight before SDK/binding argument validation |
| 2 | `read_search_in_tx` (`search/fusion_stage.rs`) | `if use_graph_arm \|\| expand_depth.is_some() { ReadSurface::GraphOrEdge } else { ReadSurface::SearchNoGraph }` | Carry `context: ReadContextV1` inside boxed `FrozenQueryRuntime` (zero `SearchReaderWork` inflation) |
| 3 | `resolve_evidence` pre-check (`search_api.rs`) | `ReadSurface::EvidenceNode` | Maps drift to `EvidenceUnavailable` |
| 4 | `resolve_single_ref` post-decode (`evidence/ranked.rs`) | `ReadSurface::EvidenceGraph` (iff `payload.artifact_class == Edge \|\| payload.arm == GraphArm`) | Pass `expected_binding: &FrozenReadBinding`; in `resolve_evidence` fails call with `EvidenceUnavailable`; in `resolve_many_raw` converts cleanly into per-slot `Refused(EvidenceUnavailable, "/evidenceRefs/{i}")` while node-only slots resolve |
| 5 | `resolve_evidence` post-TOCTOU (`search_api.rs`) | `if Edge \|\| GraphArm { ReadSurface::EvidenceGraph } else { ReadSurface::EvidenceNode }` | Whole-call `EvidenceUnavailable` on post-resolve race drift |
| 6 | `resolve_evidence_many_slice` pre & post (`search_api.rs`) | Pre: `ReadSurface::EvidenceNode`; Post: `EvidenceGraph` if any `Resolved` item is `Edge` or `GraphArm`, else `EvidenceNode` | Collapses post-resolve TOCTOU drift to whole-call `EvidenceUnavailable` |
| 7 | `resolve_graph_evidence` pre & post (`search_api.rs`) | `ReadSurface::EvidenceGraph` | Maps drift to `EvidenceUnavailable` |
| 8 | `read_graph_expand_with_stop_words_in_tx` pre & post (`graph_expand/execution.rs`) | `ReadSurface::GraphOrEdge` | Extract `context` from `request.context` before snapshot validation |
| 9 | `search_expand_in_tx` (`graph_expand/traversal.rs` via `reader_pool.rs`) | `ReadSurface::GraphOrEdge` | Carry `frozen_context: Option<Box<ReadContextV1>>` on `ReaderRequest::SearchExpand` |
| 10 | `read_canonical_page_in_tx` (`read.rs`) | `ReadSurface::CanonicalPage { path_kind: kind }` | Isolated from vector readiness, edges, dependencies, and op state |
| 11 | `read_operational_state_in_tx` & `read_operational_state_page_in_tx` (`read.rs`) | `ReadSurface::OperationalState` | Isolated from canonical/search/vector/edge/dependency writes |
| 12 | `execute` (`dependency_trace.rs`) | `ReadSurface::TraceDependency` | Enforces `node_all_tag`, `edge_any`, `scope_dependency`, and universal `serving_digest` |
| 13 | `measure_slice45_frozen_stages_for_test` (`frozen_read.rs`, `test-hooks` only) | `ReadSurface::Preflight` | Keeps Slice 45 stage-timing benchmark working |

### 4.5 `fathomdb-mcp` Session Pre-Gate Removal & `MR-070.2`-Compliant Drift Probes (`src/rust/crates/fathomdb-mcp/src/{session.rs,tools.rs}`)

1. **`src/rust/crates/fathomdb-mcp/src/session.rs`:**
   - Remove `pub(crate) minted_state: VisibilityStateV1` from `FrozenEntry` (retaining `epoch` ONLY if needed for diagnostics or removing `epoch` from `FrozenEntry` altogether since `FrozenReadContextV1` is durable across store close/reopen per `dev/interfaces/mcp.md`; keep `Continuation.epoch` untouched so unfrozen continuation handles still reject stale epochs!).
   - Clean up unused `VisibilityStateV1` import if no longer referenced in `session.rs` (note: `last_visibility` for `Envelope.state` drift labeling remains intact).
2. **`src/rust/crates/fathomdb-mcp/src/tools.rs` — Removals across all three handlers (`search`, `fetch_evidence`, `graph_expand`):**
   - Remove the `if entry.epoch != guard.epoch { return Err(ToolError::state_drift()); }` checks on `reused_frozen` in `search`, `fetch_evidence`, and `graph_expand` (preserving `continued_epoch.is_some_and(|epoch| epoch != guard.epoch)` for unfrozen continuations!).
   - Remove all three `if visibility != entry.minted_state` / `if visibility != minted_state` pre-gates before calling the engine.
   - Remove all `if after != visibility` / `if after != minted_state` post-error checks across **both** reused `existing_frozen` and freshly minted (`freeze: true` / `include_evidence: true`) `frozen_ctx` calls.
   - Retain `read::visibility_state(engine)?` exclusively to populate `Envelope.state` telemetry (`EP-01`).
3. **`src/rust/crates/fathomdb-mcp/src/tools.rs` — Exact `MR-070.2`-Compliant Cold-Path Probe Helpers:**
   - Use ONLY `fathomdb_sdk` types (`Engine`, `Error`, `EngineError`, `ErrorKind`, `FrozenReadContextV1`, `FrozenReadErrorReason`, `FrozenSearchOptions`, `SearchExpandOptions`):

   ```rust
   fn is_state_drifted(err: &fathomdb_sdk::Error) -> bool {
       matches!(
           err,
           fathomdb_sdk::Error::Engine(fathomdb_sdk::EngineError::FrozenRead(fe))
               if fe.reason == fathomdb_sdk::FrozenReadErrorReason::StateDrifted
       )
   }

   fn probe_frozen_search_drift(
       engine: &fathomdb_sdk::Engine,
       query: &str,
       frozen_ctx: &fathomdb_sdk::FrozenReadContextV1,
       use_graph_arm: bool,
   ) -> bool {
       let mut opts = fathomdb_sdk::FrozenSearchOptions::default();
       opts.use_graph_arm = use_graph_arm;
       opts.limit = 1;
       matches!(engine.search_frozen(query, frozen_ctx, opts), Err(ref err) if is_state_drifted(err))
   }

   fn probe_frozen_expand_drift(
       engine: &fathomdb_sdk::Engine,
       frozen_ctx: &fathomdb_sdk::FrozenReadContextV1,
   ) -> bool {
       let opts = fathomdb_sdk::SearchExpandOptions { search_limit: 1, view: None };
       matches!(
           engine.search_expand_frozen("probe", frozen_ctx, 0, opts),
           Err(ref err) if is_state_drifted(err)
       )
   }
   ```

   - **Handler error mapping:**
     - **`search` (`engine.search_with_evidence(&ev_req)`):**
       - If `is_state_drifted(&err)` -> `return Ok(SearchExecOutcome::Drifted)`.
       - If `err.kind() == ErrorKind::Evidence` -> if `probe_frozen_search_drift(engine, &ev_req.query, &frozen_ctx, ev_req.use_graph_arm)` -> `return Ok(SearchExecOutcome::Drifted)`, else `return Err(err)`.
     - **`fetch_evidence` — `resolve_evidence_many(&req)`:**
       - If `is_state_drifted(&err)` -> `return Ok(FetchEvidenceExec::Drifted)`.
       - If `err.kind() == ErrorKind::Evidence` -> two-step cold probe: if `probe_frozen_search_drift(engine, "probe", &frozen_ctx, false) || probe_frozen_search_drift(engine, "probe", &frozen_ctx, true)` -> `return Ok(FetchEvidenceExec::Drifted)`, else `return Err(err)`.
     - **`fetch_evidence` — `graph_refs` (`engine.resolve_graph_evidence(&req)`):**
       - If `is_state_drifted(&err)` -> `return Ok(FetchEvidenceExec::Drifted)`.
       - If `err.kind() == ErrorKind::Evidence` -> if `probe_frozen_search_drift(engine, "probe", &frozen_ctx, true)` -> `return Ok(FetchEvidenceExec::Drifted)` (whole-call `ToolError::state_drift()` escalation!), else convert non-drift `ErrorKind::Evidence` into a per-item `ResolvedFetchSlot` error.
     - **`graph_expand` (`graph::expand(engine, &request)`):**
       - Retain an `active_frozen_ctx: Option<FrozenReadContextV1>` covering both reused `existing_frozen` and freshly minted `include_evidence` contexts.
       - If `is_state_drifted(&err)` -> `return Ok(ExpandExec::Drifted)`.
       - If `err.kind() == ErrorKind::Evidence`, when `let Some(ref frozen_ctx) = active_frozen_ctx` and `probe_frozen_expand_drift(engine, frozen_ctx)` -> `return Ok(ExpandExec::Drifted)`, else `return Err(err)`.

---

## 5. Comprehensive TDD Matrix (`RED -> GREEN`) & Verification Commands

Every work package follows strict `RED -> GREEN -> Refactor` ordering. Test files are authored first and observed failing against the unpatched code before implementing the fix.

### 5.1 Acceptance Criteria -> Test Specification Matrix

| Work Package | AC ID | Test File & Case Names | Falsifiable Assertions |
| --- | --- | --- | --- |
| `WP-41A` | `AC32-PURGE-LINEAGE-01` | `src/rust/crates/fathomdb-engine/tests/slice41_purge_lineage.rs`: `actuate_delete_purge_actuate_sole_node_in_process_and_reopen`, `actuate_delete_purge_actuate_trailing_cursor_and_dpi` | Zero `StorageError` (`SQLITE_CONSTRAINT_PRIMARYKEY`), strictly monotonic `resulting_write_boundary` cursors, `doctor data-plane-integrity` reports `0` findings |
| `WP-41A` | `AC32-PURGE-LINEAGE-02` | `src/rust/crates/fathomdb-engine/tests/slice41_purge_lineage.rs`: `purge_single_node_preserves_shared_bucket_sibling_receipt_replay`, `purge_canonical_source_redacts_source_and_dependents_but_not_unrelated_sibling`, `erase_source_still_redacts_entire_shared_bucket_including_refused` | Surviving sibling `operation_id` replay returns exact cached receipt (not `OperationIdErased`); purged node + dependent receipts return `OperationIdErased`; `assert_database_and_wal_do_not_contain` erased body/revision; `erase_source` redacts whole bucket |
| `WP-41B` | `AC32-QUERY-SCALE-01` | `src/rust/crates/fathomdb-engine/tests/slice41_query_scale.rs`: `search_with_evidence_explain_false_skips_status_structural_cosine_and_matches_contributions` | SQLite authorizer/profile hook proves zero `vec_distance_cosine`, zero `status_in_snapshot` / `status_cache` computation, and zero `structural_dependency_state` work when `include_explanation = false`, while resolved `EvidenceContributionV1` is bit-identical to `include_explanation = true` |
| `WP-41B` | `AC32-QUERY-SCALE-02` | `src/rust/crates/fathomdb-engine/tests/slice41_query_scale.rs` (`required-features = ["test-hooks"]`): `filtered_fts_only_pushes_search_index_kind_and_streams_order_by_rank`, `filtered_fts_only_post_analyze_skewed_kind_preserves_stream_plan`, `empty_vector_kinds_cap_respects_graph_recency_importance_rerank_guards_and_large_pool_no_oom`, `importance_confidence_maps_run_post_truncation_and_preserve_storage_errors` | `EXPLAIN QUERY PLAN` (including post-`ANALYZE` on a skewed `kind` corpus) shows zero `SCAN canonical_nodes`, zero `USE TEMP B-TREE FOR ORDER BY`, and `search_index.kind = ?` filter; `defer_full_hydration` never applies `ORDER BY`-less `LIMIT`; `rerank_depth > 0` (including large `rerank_depth` / `pool_n` up to `u32::MAX` without upfront OOM), `use_graph_arm`, `recency_enabled`, `importance_enabled` preserve exact top-K; corrupt `importance` row at rank `<= final_limit` returns `EngineError::Storage`, while corrupt row beyond `final_limit` with `!importance_enabled` is not touched |
| `WP-41B` | `AC32-QUERY-SCALE-03` | `src/rust/crates/fathomdb-engine/tests/slice41_query_scale.rs`: `empty_physical_fast_path_skips_node_sql_when_unenrolled_and_preserves_edge_corruption_check` | SQLite authorizer denies `canonical_nodes` reads during `empty_physical_fast_path` when `!node_arm_declared && !has_explicit_node_enrolment` -> succeeds; unenrolled edge row still fails closed with corruption error |
| `WP-41B` | `AC32-QUERY-SCALE-04` | `src/rust/crates/fathomdb-engine/tests/slice41_query_scale.rs`: `store_single_unscoped_uses_covering_range_search_and_soft_fallback_short_circuits` | `EXPLAIN QUERY PLAN` confirms `SEARCH canonical_nodes USING COVERING INDEX canonical_nodes_scope_cursor_idx (scope<?)` / `(scope>?)`; SQLite authorizer confirms `detect_vector_soft_fallback` never queries `search_index` when `_fathomdb_vector_kinds` is empty |
| `WP-41B` | `AC32-QUERY-SCALE-05` | `src/rust/crates/fathomdb-engine/tests/slice41_query_scale.rs`: `mint_and_current_graph_expand_pin_without_count_star`, `historical_dense_incomplete_short_circuits_without_query_vector`, `evidence_resolution_hashes_source_body_once` | Authorizer confirms zero `SCAN canonical_nodes` on `mint` / `Current` `graph_expand` pin and zero `historical_dense_incomplete` scan when `query_vector.is_none()`; `canonical_body_hash_calls_for_test()` (`#[cfg(any(test, feature = "test-hooks"))]`) delta is exactly `1` per unique `source_body` across `build_search_result`, `resolve_evidence`, and `resolve_graph_evidence` |
| `WP-41B` | `AC32-QUERY-SCALE-06` | `src/rust/crates/fathomdb-engine/tests/slice41_query_scale.rs`: `read_get_and_get_many_include_superseded_use_search_only_union_all_and_match_semantics` | `EXPLAIN QUERY PLAN` for single-ID, multi-ID, single-scope, and multi-scope `include_superseded = true` contains zero `SCAN canonical_nodes` and zero `USE TEMP B-TREE FOR ORDER BY` (uses `canonical_nodes_scope_logical_active_idx` + `canonical_nodes_scope_logical_sup_idx`); exact parity on active, superseded, multi-version, missing, and dependency-revoked rows |
| `WP-41C` | `AC32-EVIDENCE-PROP-01`, `AC32-EVIDENCE-PROP-02`, `AC32-EVIDENCE-PROP-03` | Rust: `slice38_historical_graph.rs` (extended decoupled test) + `slice41_evidence_property_predicates.rs`; Python: `src/python/tests/test_slice41_evidence_property_predicates.py`; TypeScript: `src/ts/tests/slice41-evidence-property-predicates.test.ts` | Derived `"Doc"` (`acl="team-a"`, `confidence=0.95`) backed by `"SourceDoc"` (`acl="team-a"`, no `confidence`) resolves across `search_with_evidence`, `resolve_evidence`, `resolve_evidence_many`, `graph_expand` + `resolve_graph_evidence`; source `acl="team-b"` or cross-scope fails with `EvidenceUnavailable` at `/evidenceRef`; `trace_dependency` still rejects source lacking `property_predicates` |
| `WP-41D` | `AC32-FROZEN-STAB-01` | `src/rust/crates/fathomdb-engine/tests/slice41_frozen_read_stability.rs` (`required-features = ["operator", "test-hooks"]`): `concurrent_disjoint_kind_write_and_actuate_zero_drift`; Python: `src/python/tests/test_slice41_frozen_read_stability.py`; TypeScript: `src/ts/tests/slice41-frozen-read-stability.test.ts` | `8` concurrent readers running `freeze_read_context(kind="Doc") -> search_with_evidence -> resolve_evidence_many` (and `search_frozen`, `read_canonical_page("Doc")`, `graph_expand`) under concurrent `AppTurn` `write` + `actuate`: `0` unintended drift failures |
| `WP-41D` | `AC32-FROZEN-STAB-02` | `slice41_frozen_read_stability.rs`: `append_only_log_and_latest_state_isolation_no_deadlock` | High-frequency `operational_mutations` (`AppendOnlyLog`) appends produce `0%` drift on canonical/search/evidence/graph/trace/`operational_state` with `('up_to_date', 'non_canonical')` preserved and `drain` succeeding; `operational_state` (`LatestState`) upserts produce `0%` drift on canonical/search/evidence/graph/trace (even `SearchFilter::default()`) while drifting `OperationalState` |
| `WP-41D` | `AC32-FROZEN-STAB-03` | `slice41_frozen_read_stability.rs`: `cross_scope_isolation_including_destructive_ops_and_reopen`; MCP: `src/rust/crates/fathomdb-mcp/tests/mcp_frozen_evidence.rs` (`ac32_frozen_stab_03_mcp_disjoint_kind_op_log_cross_scope_reopen_and_graph_drift`) | `"tenant-b"` writes, `transition`, `purge`, `erase_source`, and `physical_erasure` produce `0%` drift on `"tenant-a"` tokens across close/reopen AND in `fathomdb-mcp` cached sessions (`search`, `fetch_evidence`, `graph_expand`, including after store idle-expire/reopen epoch change and `graph_refs` whole-call `state_drift`) |
| `WP-41D` | `AC32-FROZEN-STAB-04` | `slice41_frozen_read_stability.rs`: `async_projection_worker_kind_and_scope_isolation_and_mean_vec_pin` | Background `projection_worker` embedding `"AppTurn"` or `"tenant-b"` vectors produces `0%` drift on `"tenant-a"` `"Doc"`; embedding `"Doc"` in `"tenant-a"` or pinning `_fathomdb_embedder_profiles.mean_vec` drifts deterministically |
| `WP-41D` | `AC32-FROZEN-STAB-05` | `slice41_frozen_read_stability.rs`: `fail_closed_matrix_same_kind_edges_post_decode_destructive_deps_and_raw_sql_20_tables` | Same-`(scope, kind)` insert, matching-kind `TextEdge`/`edge_fact`, same-scope edge on `GraphOrEdge`/`search_expand_frozen`/`EvidenceGraph`/`TraceDependency`, post-decode `Edge`/`GraphArm` slot in `resolve_evidence_many` (per-slot `Refused(EvidenceUnavailable)` while node-only slot resolves!), destructive mutation on any kind in `scope`, `source_dependencies` mutation in `scope`, `global_struct` update, and raw `coalescing = 0` SQL across ALL 20 trigger tables fail closed; `validate_snapshot` preserves global monotonic `read_visibility_generation` clock |
| `WP-41D` | `AC32-FROZEN-STAB-06` | Existing `slice20`, `slice35`, `slice38`, `slice40`, `slice45`, `slice50`, `slice75`, `migrate_cli`, Inspector admission, MCP, Python, TS suites + `frozen_read::tests::binding_codec_round_trips_canonically` (`Option<SurfaceTags>`) + `slice41_frozen_read_stability.rs::wire_token_size_legacy_compat_and_validate_latency_budget` | Token bytes `492 B` (`Unscoped`) / `556 B` (`Scoped`) `<= 1,024 B`; Schema `31`/`32` normative fixtures bit-identical; `validate_snapshot` `p50 <= 0.012 ms` at `50k` rows; `governed-surface-pin.json` (`58`/`97`) unchanged |

### 5.2 `Cargo.toml` Test Declarations & Verification Commands (Stage 5 & Stage 6)

Register `[[test]]` entries in `src/rust/crates/fathomdb-engine/Cargo.toml` for the new feature-gated Slice 41 integration suites:

- `slice41_purge_lineage`: `required-features = ["operator", "test-hooks"]`
- `slice41_query_scale`: `required-features = ["test-hooks"]`
- `slice41_evidence_property_predicates`: no extra features required
- `slice41_frozen_read_stability`: `required-features = ["operator", "test-hooks"]`

```bash
# Scoped Rust tests per work package (run from repo root /mnt/data/worktrees/release-0.8.32)
cargo test -p fathomdb-schema
cargo test -p fathomdb-engine --features operator,test-hooks --test slice41_purge_lineage
cargo test -p fathomdb-engine --features test-hooks --test slice41_query_scale
cargo test -p fathomdb-engine --test slice41_evidence_property_predicates --test slice38_historical_graph
cargo test -p fathomdb-engine --features operator,migration-test-hooks,test-hooks --lib frozen_read::tests::binding_codec_round_trips_canonically
cargo test -p fathomdb-engine --features operator,migration-test-hooks,test-hooks \
  --test slice41_frozen_read_stability --test slice35_frozen_read --test slice35_frozen_read_races \
  --test slice45_pagination --test slice50_evidence --test slice50_resolve_evidence_many \
  --test slice20_graph_evidence --test slice20_migrate_38_to_39 --test slice75_feature_interactions
cargo test -p fathomdb-mcp
cargo test -p fathomdb-inspector

# Rebuild owned Python wheel & TS native addon before cross-binding tests
bash scripts/setup-python-owned.sh --build
.venv/bin/pytest src/python/tests/test_slice41_evidence_property_predicates.py src/python/tests/test_slice41_frozen_read_stability.py
(cd src/ts && npm run build && npm test)

# Governed surface invariance, markdown, docs, and full unconfined workspace gate
bash scripts/check-governed-surface-pin.sh
bash scripts/agent-lint-md.sh
mkdocs build --strict -d /tmp/fathomdb-slice41-mkdocs
./scripts/agent-verify.sh
```
