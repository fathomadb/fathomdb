---
title: ADR-0.8.32 — Partitioned frozen-read additive stability and artifact-scoped evidence property predicates
date: 2026-11-26
target_release: 0.8.32
desc: Accepted ADR (#103) amending ADR-0.8.25 (#46, #47, #48) and extending ADR-0.8.28 (#70) and ADR-0.8.32 (#102): Part 1 defines Schema 39 scope-and-partition visibility watermarks, ReadSurface-aware snapshot validation (universal serving_digest, 0x53 0x39 surface tags, SearchNoGraph edge-FTS, search_expand_frozen GraphOrEdge routing, TraceDependency, pre/post-decode evidence), full 20-table trigger classification, non-deadlocking AppendOnlyLog/mixed-batch coalescing, and fathomdb-mcp pre-gate removal with MR-070.2 drift probing; Part 2 (#269 Option A) makes SearchFilter.property_predicates artifact-scoped on source_filter while retaining source-enforced attributes and Schema 39 tenant scope, clarifies trace_dependency, and ratifies GraphArm SearchHit.source_id vs. ResolvedEvidenceV1.source_id semantics.
blast_radius: src/rust/crates/fathomdb-schema/src/lib.rs; src/rust/crates/fathomdb-engine/src/{frozen_read.rs,write_commit.rs,actuation.rs,record_lifecycle.rs,projection_commit.rs,projection_generation.rs,erasure.rs,physical_erasure.rs,read.rs,search_api.rs,dependency_trace.rs,export_manifest.rs,search/fusion_stage.rs,evidence/ranked.rs,evidence/graph.rs,graph_expand/}; src/inspector/src/admission.rs; src/rust/crates/fathomdb-mcp/src/tools.rs; dev/plans/0.8.32/features/slice-41/**; dev/adr/ADR-0.6.0-decision-index.md; dev/interfaces/{rust,python,typescript,wire}.md
status: accepted
---

# ADR-0.8.32: Partitioned frozen-read additive stability and artifact-scoped evidence `property_predicates`

> **Status: `accepted` (`#103`, Gate 1 Decision 11, 2026-11-26) — authored for `0.8.32` Slice 41 (`#267` Part 3 & `#269` Option A), grounded by [frozen-read-stability-experiment.md](../plans/0.8.32/features/slice-41/frozen-read-stability-experiment.md).**
> **Amends:** `ADR-0.8.25-eligibility-and-frozen-reads.md` (`#46`), `ADR-0.8.25-frozen-pagination-and-operational-state.md` (`#47`), and `ADR-0.8.25-compact-source-evidence.md` (`#48`) on Schema `39+` databases.
> **Extends:** `ADR-0.8.28-transactional-visibility-coalescing.md` (`#70`) and `ADR-0.8.32-caller-scope-isolation.md` (`#102`).

## Context

Two distinct defects discovered during `0.8.32` Slice 41 hardening stem from over-broad state coupling across FathomDB's frozen-read, MCP session, and evidence paths:

1. **Global frozen-read invalidation on unrelated additive writes (`#267` Part 3):** `ADR-0.8.25-eligibility-and-frozen-reads.md` validates stateless `FrozenReadContextV1` tokens against global store-wide counters (`load_next_cursor` in `src/rust/crates/fathomdb-engine/src/write_commit.rs`, `_fathomdb_read_visibility_state`, `load_projection_cursor`, and `projection_serving_encoding` scanning all `_fathomdb_projection_state` kinds plus global `state_nonce` in `src/rust/crates/fathomdb-engine/src/frozen_read.rs`). Furthermore, `AppendOnlyLog` inserts into `operational_mutations` run outside `VisibilityCoalescingGuard`, firing `_fathomdb_read_visibility_pt_ai` on `_fathomdb_projection_terminal` and advancing global visibility; and `fathomdb-mcp` caches global `VisibilityStateV1` (`minted_state`) as a pre-gate before calling the engine (`src/rust/crates/fathomdb-mcp/src/tools.rs`). Empirically on live `0.8.32` (`8` concurrent readers running `freeze_read_context(SearchFilter(kind="Doc")) -> search_with_evidence -> resolve_evidence_many` while a writer commits every `5 ms`), **all five** unrelated additive workloads suffer severe to total collapse:
   - Disjoint `kind="AppTurn"` writes (`Engine.write` / `Engine.actuate`): `19.58%` failure at `60` rows / `0 ms` pause, **`100.00%` failure** (`240/240`) with `15 ms` workflow think time or at `5,000` rows.
   - `operational_mutations` (`append_only_log`) appends: `14.58%` failure at `60` rows / `0 ms`, **`100.00%` failure** (`240/240`) at `15 ms` or `5,000` rows.
   - Background `projection_worker` indexing `AppTurn` vectors (`use_default_embedder=True`): **`100.00%` failure** (`25/25`) even with zero concurrent writers.
   - Isolated tenant writes (`ScopePolicy.single("tenant-b")`) while `"tenant-a"` holds a frozen context: **`100.00%` failure** (`40/40`).
2. **Over-broad `property_predicates` source enforcement (`#269` Option A):** In `authorize_source_access` (`src/rust/crates/fathomdb-engine/src/evidence/ranked.rs`), `query_graph_evidence_batch` (`src/rust/crates/fathomdb-engine/src/evidence/graph.rs`), and `resolve_graph_evidence` (`src/rust/crates/fathomdb-engine/src/evidence/graph.rs`), `source_filter` clones `ReadContextV1.eligibility` and clears only `kind` and `source_type`, inadvertently forcing derived artifacts' `Filterable` `property_predicates` (which describe extracted claims/metrics rather than raw sources) to also match the canonical source node.

## Experimental Evidence Summary

Full measurements are recorded in [frozen-read-stability-experiment.md](../plans/0.8.32/features/slice-41/frozen-read-stability-experiment.md):

- **WAL pinning rejected (Exp 2A):** Holding a SQLite `BEGIN DEFERRED` snapshot across `200` writer batches blocked `8 / 8` (`100%`) `PRAGMA wal_checkpoint(TRUNCATE)` calls (`busy=1`) and caused `-wal` to grow linearly to `5,310,712 B` (`1,289` frames), versus `0 / 8` blocked checkpoints and `0 B` post-truncate WAL under stateless tokens (`424 B` baseline `fdbfr1` wire tokens).
- **Same-`(scope, kind)` global cursor clamping rejected as default (Exp 3B & 3C):**
  - Filtering `graph_expand` recursive joins with `AND ce.write_cursor <= ?` caused SQLite's planner to abandon `canonical_edges_scope_from_active_idx (scope=? AND from_id=?)` and scan `canonical_edges_scope_cursor_idx (scope=? AND write_cursor<?)`, regressing 2-hop traversal latency by **`8.3x` at `5k` rows (`0.0137 ms -> 0.1135 ms`)** and **`7.7x` at `50k` rows (`0.0142 ms -> 0.1099 ms`)**.
  - `vector_default` (`sqlite-vec v0.1.9` `vec0`) carries `scope` and `kind` metadata columns pushed natively before `TOP_K_BIT_CANDIDATES = 192` (`5 / 5` `Doc` recall under `250` competing `AppTurn` rows), whereas it carries **no `write_cursor` metadata column** (`0 / 5` `Doc` vector hits survived `250` closer post-boundary same-kind `Doc` rows).
- **Partitioned Schema 39 visibility watermarks win on cost and plan safety (Exp 3A & 3B):**
  - Coalesced per-`(scope, partition_kind)` watermark upserts add `+4.4 µs` per single-row write (`0.0260 ms` vs. `0.0216 ms` `p50`) and `+14.9 µs` (`+8.3%`) across 4 buckets in a 50-row batch.
  - `validate_snapshot` latency drops by **`36.0%`** (`0.0114 ms -> 0.0073 ms` `p50` at `50k` rows) by replacing the 6-table `UNION ALL MAX(write_cursor)` + `_fathomdb_projection_state` scan with indexed `WITHOUT ROWID` PK point lookups.
- **BM25 corpus sensitivity (Exp 2B):** Even with explicit `AND cn.kind = 'Doc' AND cn.write_cursor <= 5000`, appending `+100` to `+2,000` disjoint-vocab `AppTurn` rows shifted SQLite `bm25(search_index)` weights (`0.80%` to `18.38%` mean relative delta on rare-DF queries; `11/20` to `12/20` exact top-10 preservation under `+2,000` / common-DF zero-crossings) because FTS5 computes corpus-wide `N`, `avgdl`, and `df`.

---

## Decision — Part 1: Scope-and-Partition Frozen Read Stability (`#267` Part 3)

### 1.1 Normative Contract: Unrelated Additive Writes vs. Drift-Triggering Mutations

On Schema `39+` databases, a `FrozenReadContextV1` binds **only** the visibility partitions intersected by its opener's `scope.read_scopes()` (`ScopePredicate`), its `ReadContextV1.eligibility` (`SearchFilter`), and its active `ReadSurface`. Under `ADR-0.8.32-caller-scope-isolation.md` (`#102`), `ScopePolicy::Unscoped` yields `read_scopes() == [""]` (the single-agent empty-string scope, never a cross-tenant wildcard), so **all** callers — scoped and unscoped alike — resolve watermarks uniformly over `scope.read_scopes()`.

#### Mutation-Class Drift Matrix (Schema `39+`)

| Concurrent Mutation | `read_scopes()=S`, `kind=Some("Doc")`, `SearchNoGraph` | `read_scopes()=S`, `kind=Some("Doc")`, `GraphOrEdge` / `EvidenceGraph` | `read_scopes()=S`, `kind=None` (`SearchFilter::default()`), Canonical / Search | `read_scopes()=S`, `kind=None`, `OperationalState` | `read_scopes()=S`, `TraceDependency` | Different Scope `tenant-b ∉ S` (Even Erasure / Delete!) |
| --- | --- | --- | --- | --- | --- | --- |
| Additive `canonical_nodes INSERT` (`kind="AppTurn"`, fresh `logical_id`, in `S`) | **NO DRIFT** | **NO DRIFT** | `StateDrifted` | **NO DRIFT** | `StateDrifted` | **NO DRIFT** |
| Additive `canonical_nodes INSERT` (`kind="Doc"`, in `S`) | `StateDrifted` | `StateDrifted` | `StateDrifted` | **NO DRIFT** | `StateDrifted` | **NO DRIFT** |
| `operational_mutations` (`AppendOnlyLog`) `INSERT` (in `S`) | **NO DRIFT** | **NO DRIFT** | **NO DRIFT** | **NO DRIFT** | **NO DRIFT** | **NO DRIFT** |
| `operational_state` (`LatestState`) `INSERT` / replace (in `S`) | **NO DRIFT** | **NO DRIFT** | **NO DRIFT** | **`StateDrifted`** | **NO DRIFT** | **NO DRIFT** |
| Additive `canonical_edges INSERT` (`kind="supports"`, `kind != "Doc"`, in `S`) | **NO DRIFT** | **`StateDrifted`** | **`StateDrifted`** | **NO DRIFT** | **`StateDrifted`** | **NO DRIFT** |
| Additive `canonical_edges INSERT` (`kind="Doc"`, matching `eligibility.kind`, in `S`) | **`StateDrifted`** (`edge:Doc` hits edge-FTS) | **`StateDrifted`** | **`StateDrifted`** | **NO DRIFT** | **`StateDrifted`** | **NO DRIFT** |
| Destructive `UPDATE`/`DELETE`/supersede/`transition`/`purge`/`erase_source` on **ANY kind** in `S` (or physical sanitation in `S`) | **`StateDrifted`** | **`StateDrifted`** | **`StateDrifted`** | **`StateDrifted`** | **`StateDrifted`** | **NO DRIFT** (isolated in `tenant-b`) |
| Async `projection_worker` commit for `AppTurn` vectors (in `S` or other scope) | **NO DRIFT** | **NO DRIFT** | `StateDrifted` (vector state in `S`) | **NO DRIFT** | **NO DRIFT** | **NO DRIFT** |
| Async `projection_worker` commit for `Doc` (or `edge_fact` when `kind=Some("edge_fact")`) vectors in `S` | **`StateDrifted`** | **`StateDrifted`** | `StateDrifted` | **NO DRIFT** | **NO DRIFT** | **NO DRIFT** |
| `source_dependencies` / `_fathomdb_dependency_closures` write in `S` | **`StateDrifted`** | **`StateDrifted`** | **`StateDrifted`** | **NO DRIFT** | **`StateDrifted`** | **NO DRIFT** |
| Global structural mutations (`_fathomdb_embedder_profiles.mean_vec` pin, `_fathomdb_projection_registry`, `_fathomdb_vector_kinds`, `_fathomdb_projection_generations` rebuild/switch, uncoalesced raw SQL on row-owned/global tables) | **`StateDrifted`** | **`StateDrifted`** | **`StateDrifted`** | **`StateDrifted`** | **`StateDrifted`** | **`StateDrifted`** (DB-wide) |

Note on `StateDrifted` surfacing: per `ADR-0.8.25-compact-source-evidence.md`, `search_frozen`, `search_expand_frozen`, `graph_expand`, `read_canonical_page`, `read_operational_state*`, and `trace_dependency` return `FrozenReadErrorReason::StateDrifted`, whereas `search_with_evidence`, `resolve_evidence`, `resolve_evidence_many`, and `resolve_graph_evidence` map drift to non-disclosing `EvidenceErrorReasonV1::EvidenceUnavailable` at `/evidenceRef`.

### 1.2 Unreleased Schema 39 In-Place Extension, Trigger Classification & Coalescing

Because Schema `39` in `dev/0.8.32` is unreleased (`STEP_39_SQL` in `src/rust/crates/fathomdb-schema/src/lib.rs`), this design extends `STEP_39_SQL` **in place** (no Schema `40` bump prior to `0.8.32` release; preserves existing `-- MIGRATION-ACCRETION-EXEMPTION:` without canonical node/edge table rebuilds).

1. **Global structural epoch on `_fathomdb_read_visibility_state`:**
   Retain `(singleton, generation, state_nonce, coalescing)` (`generation` remains the monotonic regression guard for `Engine::open`, `read_visibility_state`, and `mint` `fetch_max`) and add:

   ```sql
   ALTER TABLE _fathomdb_read_visibility_state
       ADD COLUMN global_struct_gen INTEGER NOT NULL DEFAULT 0;
   ALTER TABLE _fathomdb_read_visibility_state
       ADD COLUMN global_struct_nonce TEXT NOT NULL DEFAULT '';
   ```

   Backfill `global_struct_nonce = lower(hex(randomblob(32)))` in `STEP_39_SQL`.

2. **Single partitioned watermark table (`WITHOUT ROWID`, used uniformly by `Scoped` and `Unscoped`):**

   ```sql
   CREATE TABLE _fathomdb_scope_visibility_watermarks(
       scope TEXT NOT NULL,
       partition_kind TEXT NOT NULL,
       add_generation INTEGER NOT NULL DEFAULT 0,
       max_cursor INTEGER NOT NULL DEFAULT 0,
       destructive_generation INTEGER NOT NULL DEFAULT 0,
       state_nonce TEXT NOT NULL,
       PRIMARY KEY(scope, partition_kind)
   ) WITHOUT ROWID;
   ```

   *(No separate global kind watermark table exists; `Unscoped` handles query `scope = ""` via `scope.read_scopes()` directly.)*

   Reserved `partition_kind` keys within a `scope`:
   - `'node:' || kind` — canonical node inserts for `kind` in `scope`.
   - `'node_all'` — bumped on **any** canonical node insert in `scope` (bound when `eligibility.kind.is_none()` or `TraceDependency`).
   - `'edge:' || kind` — bumped on canonical edge writes of `kind` in `scope` (bound by `SearchNoGraph` when `eligibility.kind == Some(K)` so `TextEdge` / `edge_fact` matches drift cleanly while disjoint edge kinds remain `NO DRIFT`).
   - `'edge_any'` — bumped on **any** canonical edge insert/update/delete in `scope`.
   - `'op_state'` — bumped on `operational_state` (`LatestState`) writes/updates/deletes in `scope` (bound **exclusively** on `ReadSurface::OperationalState`, never on canonical/search/evidence/graph/trace surfaces).
   - `'op_log:' || collection_name` — bumped on `operational_mutations` (`AppendOnlyLog`) appends in `scope` (never checked by canonical, evidence, graph, trace, or `latest_state` frozen reads).
   - `'vector_ready:' || kind` — bumped on coalesced async `projection_worker` commits into `vector_default` for rows of `(scope, kind)` (`'vector_ready_all'` bumped concurrently for `eligibility.kind.is_none()` readers in `scope`). Because `_fathomdb_projection_state` has no `scope` column, Schema `39+` `serving_digest` **drops** the global `_fathomdb_projection_state` scan and routes `(scope, 'vector_ready:' || kind)` / `'vector_ready_all'` into the surface-selective `projection_readiness_tag` (see §1.3(2)) so `"tenant-b"` vector activity never drifts `"tenant-a"`, and vector readiness never drifts `CanonicalPage`, `OperationalState`, or `TraceDependency`. `projection_commit.rs` resolves affected scopes from the committed cursor batch before upserting watermarks.
   - `'scope_destructive'` — bumped on **any** destructive mutation (`UPDATE`, `DELETE`, supersession, lifecycle `transition`, `purge`, `erase_source`, or `physical_erasure` row sanitation) in `scope`, regardless of `kind`.
   - `'scope_dependency'` — bumped on `register_source_dependency` or `_fathomdb_dependency_closures` mutations in `scope`.

3. **Explicit Classification of All 20 Trigger-Bearing Tables (`19` Step-37 Tables + `cpv`):**
   - **8 `scope`-bearing tables (`cn, ce, ar, sv, sl, sd, dc, os`):**
     - `cn` (`canonical_nodes`): coalesced path updates `node:<kind>`, `node_all`, and (`UPDATE`/`DELETE`/supersession) `scope_destructive`. Raw `coalescing = 0` triggers check `NEW.scope`/`OLD.scope` and `NEW.kind`/`OLD.kind`.
     - `ce` (`canonical_edges`): updates `edge:<kind>` and `edge_any` (plus `scope_destructive` on `UPDATE`/`DELETE`).
     - `ar, sv, sl` (`_fathomdb_artifact_revisions`, `_fathomdb_source_versions`, `_fathomdb_source_links`): coalesced with owning node/edge write. Raw `coalescing = 0` `AFTER INSERT` bumps `node_all`, `edge_any`, **and `(NEW.scope, 'scope_destructive')`** (failing closed across all read surfaces in `NEW.scope` on out-of-band provenance mutations while preserving cross-tenant isolation), while `AFTER UPDATE/DELETE` bumps `(OLD.scope, 'scope_destructive')`, `node_all`, and `edge_any`.
     - `sd, dc` (`_fathomdb_source_dependencies`, `_fathomdb_dependency_closures`): updates `(scope, 'scope_dependency')` (plus `scope_destructive` on `UPDATE`/`DELETE`).
     - `os` (`operational_state`): `INSERT`, `ON CONFLICT DO UPDATE`, and raw `UPDATE`/`DELETE` bump **only** `(scope, 'op_state')` — **never** `scope_destructive` — so `LatestState` replacements drift `OperationalState` without drifting canonical `Doc` or `SearchFilter::default()` searches.
   - **5 row-owned projection/property tables (`ca, vr, vs, pt, cpv`):**
     - All internal engine writes, `projection_commit`, `actuate`, `record_lifecycle::transition` (`Engine::transition`, gated on `canonical_batch_has_no_custom_triggers`, explicitly bumping `(write_scope, 'scope_destructive')`, plus `(write_scope, 'scope_dependency')` / `(write_scope, 'vector_ready:' || kind)` + `'vector_ready_all'` when touched, and global `_fathomdb_read_visibility_state.(generation, state_nonce)`), `purge_inner`, `excise_source`, and `physical_erasure::sanitize_unit` run under `VisibilityCoalescingGuard` (`coalescing = 1`), attributing watermarks explicitly to the owning row's `scope` (via `ScopePredicate` or cursor join before row removal), **so `"tenant-b"` `transition`, `purge`, `erase_source`, or physical sanitation NEVER bumps `global_struct_gen`**.
     - Raw uncoalesced SQL (`coalescing = 0`) triggers on `ca, vr, vs, pt, cpv` fail closed by incrementing `(global_struct_gen, global_struct_nonce)` and global `_fathomdb_read_visibility_state.(generation, state_nonce)`.
   - **7 global metadata tables (`oc, pr, vk, ep, pg, pc, ps`):**
     - Raw `coalescing = 0` triggers on all 7 bump `(global_struct_gen, global_struct_nonce)` and global `_fathomdb_read_visibility_state.(generation, state_nonce)`.
     - Under coalesced engine paths (`coalescing = 1`): `projection_commit.rs` explicitly bumps `(global_struct_gen, global_struct_nonce)` whenever pinning `_fathomdb_embedder_profiles.mean_vec`; `configure_projections` / rebuilds bump `(global_struct_gen, global_struct_nonce)` when modifying `_fathomdb_projection_registry`, `_fathomdb_vector_kinds`, `_fathomdb_projection_generations`, or `_fathomdb_projection_generation_current`; while coalesced `_fathomdb_projection_state` (`ps`) enqueue/advance updates bump **only** `(scope, 'vector_ready:' || kind)` + `(scope, 'vector_ready_all')` without bumping `global_struct_gen`.

4. **Non-Deadlocking `AppendOnlyLog` & Mixed-Batch Handling (`operational_mutations`, `write_commit.rs`):**
   `commit_batch_transaction` MUST **retain** inserting `('up_to_date', 'non_canonical')` into `_fathomdb_projection_terminal` and calling `advance_projection_cursor` (`write_commit.rs`), preserving `scheduler_scan_sql`, `advance_projection_cursor`, `EP-01` `projection_cursor`, and `data_plane_integrity.rs::non_canonical_is_legitimate_terminal_only` intact with zero deadlocks. Additionally, `commit_batch_transaction` generalizes `canonical_batch` so **any non-`AdminSchema` batch** — canonical batches, `OpStore` (`AppendOnlyLog` / `LatestState`) batches, **and mixed `[Node/Edge, OpStore]` batches** in `Engine.write` — executes under `VisibilityCoalescingGuard` (`coalescing = 1`, gated on `canonical_batch_has_no_custom_triggers`), upserting `(scope, 'op_log:' || collection)` (for `AppendOnlyLog`), `(scope, 'op_state')` (for `LatestState`), and matching `node:*`/`edge:*`/`scope_destructive` partitions for any canonical writes in the batch plus global `_fathomdb_read_visibility_state.(generation, state_nonce)`, while **leaving uninvolved `node:*`, `edge:*`, `edge_any`, `vector_ready:*`, `scope_destructive`, and `global_struct_gen` untouched**.

5. **Schema `39` Layout, Admission & Integrity Checklist:**
   Extending `STEP_39_SQL` in place requires synchronized updates in:
   - `TABLE_MANIFEST` (`src/rust/crates/fathomdb-schema/src/lib.rs`) adding `_fathomdb_scope_visibility_watermarks` (`TableExportClass::InternalRuntime`, `is_canonical = false`, `visibility_trigger_prefix = None`) and Schema 39 trigger SQL macro;
   - `src/rust/crates/fathomdb-engine/src/export_manifest.rs` (inherits via `TABLE_MANIFEST`);
   - `src/inspector/src/admission.rs` (`manifest_columns` for `_fathomdb_read_visibility_state` adding `global_struct_gen,global_struct_nonce`, plus `_fathomdb_scope_visibility_watermarks`);
   - `src/rust/crates/fathomdb-engine/src/projection_generation.rs` (`bootstrap_is_fresh` allowing fresh empty/migration watermark tables);
   - `src/rust/crates/fathomdb-engine/src/frozen_read.rs` (`validate_on_open` validating `global_struct_gen`/`global_struct_nonce` columns and Schema 39 trigger SQL matching `TABLE_MANIFEST`);
   - Schema `38 -> 39` migration and reopen tests (`slice20_migrate_38_to_39.rs`, `slice40_reopen_cursor_and_projection_ns.rs`, `migrate_cli.rs`).

### 1.3 `ReadSurface`, Refactored `validate_snapshot` API, and `fdbfr1` Wire Codec

1. **Explicit `ReadSurface` & Refactored Signature:**
   Because `FrozenReadBinding` carries only digests (`context_digest`, `scope_digest`) rather than unhashed scope strings or `SearchFilter` fields, every validation caller supplies the already-authenticated `&ReadContextV1` (`frozen.context`), `&ScopePredicate`, and explicit `ReadSurface`:

   ```rust
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

   pub(crate) fn validate_snapshot(
       connection: &Connection,
       context: &ReadContextV1,
       expected: &FrozenReadBinding,
       scope: &ScopePredicate,
       surface: ReadSurface<'_>,
   ) -> Result<u64, FrozenReadError>
   ```

   **Authoritative call-site inventory (`validate_snapshot` & pre/post evidence):**
   - `search_api.rs` (`validate_frozen_read_context_for_binding`): `ReadSurface::Preflight`. Validates `node`, `vector_ready`, matching-kind `edge:<K>`, `scope_dependency`, universal `serving_digest`, and `registry_digest` matching `context.eligibility` (also enforcing `edge_any` and `node_all` when `context.eligibility.kind.is_none()`).
   - `search/fusion_stage.rs` (`read_search_in_tx`, single snapshot-start validation): `if use_graph_arm || expand_depth.is_some() { ReadSurface::GraphOrEdge } else { ReadSurface::SearchNoGraph }` — ensuring `Engine::search_expand_frozen` (`use_graph_arm = false`, `expand_depth = Some(depth)`) validates `ReadSurface::GraphOrEdge` (`edge_any`) cleanly while plain `search_frozen` / `search_with_evidence` uses `ReadSurface::SearchNoGraph`. *(Post-resolve TOCTOU re-validation lives in `search_api.rs` and `graph_expand/execution.rs`, not in `search/fusion_stage.rs`.)*
   - `search_api.rs` (`resolve_evidence` & `resolve_evidence_many_slice`):
     - **Pre-decode transaction check:** `ReadSurface::EvidenceNode` (identical partitions to `SearchNoGraph`).
     - **Post-decode per-item check in `resolve_single_ref` / `resolve_many_raw` (`evidence/ranked.rs`):** once `decode_token` reveals `payload.artifact_class == EvidenceArtifactClassV1::Edge` or `payload.arm == EvidenceArmV1::GraphArm`, require `actual.projection_cursor == expected.projection_cursor` (`edge_any` intact!). In `resolve_evidence`, mismatch fails with `EvidenceUnavailable`. In `resolve_evidence_many` (aligned with `ADR-0.8.31 #76` per-slot non-disclosure), if `actual.projection_cursor != expected.projection_cursor`, affected `Edge`/`GraphArm` slots return **per-slot `EvidenceResolveManyItemV1::Refused(EvidenceUnavailable, "/evidenceRefs/{i}")`**, while unaffected node-only slots in the same mixed batch still resolve!
     - **Post-resolve TOCTOU re-validation (`search_api.rs`):** `resolve_evidence` re-validates `EvidenceGraph` if the resolved token was `Edge`/`GraphArm` (else `EvidenceNode`); `resolve_evidence_many_slice` re-validates `EvidenceNode`, plus `EvidenceGraph` if any slot resolved to `Resolved` with `Edge`/`GraphArm` (collapsing post-resolve race drift to whole-call `EvidenceUnavailable` as documented in `resolve_evidence_many_slice`).
   - `search_api.rs` (`resolve_graph_evidence`): `ReadSurface::EvidenceGraph`.
   - `graph_expand/execution.rs` (`read_graph_expand_with_stop_words_in_tx`, pre + post validation): `ReadSurface::GraphOrEdge`.
   - `read.rs` (`read_canonical_page_in_tx`): `ReadSurface::CanonicalPage { path_kind: kind }`. Enforces universal `serving_digest`, `registry_digest`/auth, `canonical_write_boundary`, and `read_visibility_generation` (binding `(scope, 'node:' || K)` when `eligibility.kind == Some(K)` and `(scope, 'node_all')` when `eligibility.kind.is_none()`, skipping `projection_readiness_tag`, `edge_kind_tag`, `projection_cursor`, `dependency_generation`, `op_state_tag`, and `node_all_tag`).
   - `read.rs` (`read_operational_state_in_tx` & `read_operational_state_page_in_tx`): `ReadSurface::OperationalState`. Enforces **only** `op_state_tag`, universal `serving_digest`, `registry_digest`, and auth (skipping `canonical_write_boundary`, `read_visibility_generation`, `dependency_generation`, `projection_cursor`, `projection_readiness_tag`, `edge_kind_tag`, and `node_all_tag`).
   - `dependency_trace.rs` (`execute`): `ReadSurface::TraceDependency`. Enforces `node_all_tag`, `projection_cursor` (`edge_any`), `dependency_generation` (`scope_dependency`), universal `serving_digest`, `registry_digest`, and auth (skipping `canonical_write_boundary`, `read_visibility_generation`, `projection_readiness_tag`, `edge_kind_tag`, and `op_state_tag`).

2. **Compact `fdbfr1` Wire Codec (`FROZEN_READ_SCHEMA_VERSION = 1`, `492`–`556 B` vs. `1,024 B` cap) & `validate_snapshot` Return Contract:**
   - **Monotonic Return Clock Contract:** `validate_snapshot()` MUST return **`Ok(load_visibility_generation(connection)?)`** — the global `_fathomdb_read_visibility_state.generation` monotonic clock also returned by `mint()` — and MUST NEVER return partitioned `actual.read_visibility_generation`. That preserves `Engine::read_visibility_generation.fetch_max(generation, Ordering::AcqRel)` in `search/fusion_stage.rs` and `slice35_frozen_read_races.rs::consume_rejects_visibility_generation_regression_observed_in_process`.
   - When `PRAGMA user_version < 39`: `binding_for_snapshot` runs the unchanged legacy global encoding (`0` or `32` trailing bytes after `context_digest`), preserving Schema `31`/`32` normative fixtures bit-for-byte.
   - When `PRAGMA user_version >= 39`: `FrozenReadBinding` carries a `2`-byte Schema 39 extension marker (`[0x53, 0x39]`, `"S9"`) plus four `u64` surface-selective tags (`4 * 8 = 32 B`) appended deterministically after optional `scope_digest` — giving trailing byte lengths after `context_digest` of **`34 B`** (`Unscoped`, **`492 B`** total token) or **`66 B`** (`Scoped`, **`556 B`** total token), cleanly disjoint from legacy `< 39` trailing lengths (`0` / `32 B`) and well below `TOKEN_MAX_BYTES = 1024`. All fields are computed once at `mint()` over `scope.read_scopes()`:
     - **`serving_digest` (`[u8; 32]`, strictly universal across all surfaces):** SHA-256 over `SERVING_DOMAIN_V4`, `(global_struct_gen, global_struct_nonce)`, sorted `current_generation` metadata, and sorted `(scope, 'scope_destructive')` watermark tuples `(scope, 'scope_destructive', add_generation, max_cursor, destructive_generation, state_nonce)` — **excluding** global `_fathomdb_projection_state`, global `load_visibility_nonce`, and all surface-selective partitions (`node:*`, `edge:*`, `vector_ready:*`, `op_state`, `op_log:*`, `scope_dependency`).
     - **`canonical_write_boundary` (`u64`) & `read_visibility_generation` (`u64`):** pure `node` partition — `MAX(max_cursor)` and domain-folded generation/nonce/cursor watermark strictly over `(scope, 'node:' || K)` when `kind == Some(K)` (or `(scope, 'node_all')` when `kind.is_none()`), without folding `vector_ready:*` or `edge:*` so `CanonicalPage` remains isolated from async vector readiness and edge writes.
     - **`dependency_generation` (`u64`):** `64`-bit domain-separated hash of `(scope, 'scope_dependency')` across `scope.read_scopes()` (no `is_unscoped()` global fallback).
     - **`projection_cursor` (`u64`, `edge_any_tag`):** `64`-bit domain-separated hash of `(scope, 'edge_any')` across `scope.read_scopes()`.
     - **`op_state_tag` (`u64`):** `64`-bit domain-separated hash of `(scope, 'op_state')` across `scope.read_scopes()`.
     - **`node_all_tag` (`u64`):** `64`-bit domain-separated hash of `(scope, 'node_all')` across `scope.read_scopes()`.
     - **`projection_readiness_tag` (`u64`):** `64`-bit domain-separated hash of `(scope, 'vector_ready:' || K)` when `kind == Some(K)` (or `(scope, 'vector_ready_all')` when `kind.is_none()`).
     - **`edge_kind_tag` (`u64`):** `64`-bit domain-separated hash of `(scope, 'edge:' || K)` when `kind == Some(K)` (or `(scope, 'edge_any')` when `kind.is_none()`).

   **Zero-Contradiction Checked-Field Matrix (`validate_snapshot` enforces `actual.<col> == expected.<col>`):**
   Every cell below compares fields emitted by a single `mint()`, so at zero writes `actual == expected` holds `100%` identically on every surface:

   | `ReadSurface` | `auth` + `registry` + `serving_digest` (universal) | `canonical_write_boundary` + `read_visibility_generation` (`node:<K>`/`node_all`) | `projection_readiness_tag` (`vector_ready`) | `edge_kind_tag` (`edge:<K>`/`edge_any`) | `dependency_generation` (`scope_dependency`) | `projection_cursor` (`edge_any`) | `node_all_tag` (`node_all`) | `op_state_tag` (`op_state`) |
   | --- | --- | --- | --- | --- | --- | --- | --- | --- |
   | `OperationalState` | **Check** | Skip | Skip | Skip | Skip | Skip | Skip | **Check** |
   | `CanonicalPage` | **Check** | **Check** | Skip | Skip | Skip | Skip | Skip | Skip |
   | `Preflight` | **Check** | **Check** | **Check** | **Check** | **Check** | If `kind.is_none()` | If `kind.is_none()` | Skip |
   | `SearchNoGraph` | **Check** | **Check** | **Check** | **Check** | **Check** | If `kind.is_none()` | If `kind.is_none()` | Skip |
   | `EvidenceNode` (pre) | **Check** | **Check** | **Check** | **Check** | **Check** | If `kind.is_none()` | If `kind.is_none()` | Skip |
   | `GraphOrEdge` | **Check** | **Check** | **Check** | **Check** | **Check** | **Check** | If `kind.is_none()` | Skip |
   | `EvidenceGraph` | **Check** | **Check** | **Check** | **Check** | **Check** | **Check** | If `kind.is_none()` | Skip |
   | `TraceDependency` | **Check** | Skip | Skip | Skip | **Check** | **Check** | **Check** | Skip |

   *(Post-decode in `resolve_single_ref`/`resolve_many_raw`: if `payload` is `Edge` or `GraphArm`, additionally enforce `actual.projection_cursor == expected.projection_cursor`, upgrading `EvidenceNode` to `Edge`/`GraphArm` isolation.)*

### 1.4 MCP Session Pre-Gate Removal & `MR-070.2`-Compliant Attribution (`src/rust/crates/fathomdb-mcp/src/tools.rs`)

`fathomdb-mcp` currently maintains a session cache storing `entry.minted_state: VisibilityStateV1` and rejects cached `FrozenReadContextV1` sessions via global `visibility != entry.minted_state` before invoking `engine` across three tool handlers (`src/rust/crates/fathomdb-mcp/src/tools.rs`), plus global `after != visibility` / `after != minted_state` post-error checks:

- `search` pre-check and `SearchExecOutcome::Drifted` post-error fallback,
- `fetch_evidence` pre-check and post-error fallback,
- `graph_expand` pre-check and post-error fallback.

Under ADR `#103`, all three MCP handlers MUST remove:

1. the global `visibility != entry.minted_state` pre-gate,
2. the premature `entry.epoch != guard.epoch` `state_drift()` rejection on reused `frozen_context` handles across MCP close/reopen (`expire_idle` or LRU eviction — `FrozenReadContextV1` tokens are durable across close/reopen per `dev/interfaces/mcp.md`, whereas epoch staleness remains scoped to unfrozen continuations), and
3. all global `after != visibility` / `after != minted_state` post-error checks across **both** reused `existing_frozen` and freshly minted (`freeze: true` / `include_evidence: true`) `frozen_ctx` calls, passing `frozen_ctx` directly to `engine.search_with_evidence`, `engine.resolve_evidence_many`, `engine.resolve_graph_evidence`, and `graph::expand`.

`MR-070.2` (`fathomdb-sdk` is the sole FathomDB crate allowed in `src/rust/crates/fathomdb-mcp/Cargo.toml`) and `scripts/governed-surface-pin.json` (`58` ops / `97` allowlist, zero new governed verbs) remain strictly unchanged:

- **Direct `FrozenRead` handling:** If the engine returns `fathomdb_sdk::Error::Engine(EngineError::FrozenRead(fe))` with `fe.reason == FrozenReadErrorReason::StateDrifted`, MCP immediately maps it to `SearchExecOutcome::Drifted` / `FetchEvidenceExec::Drifted` / `ToolError::state_drift()`.
- **`MR-070.2`-compliant cold-path probe on `ErrorKind::Evidence` (`EvidenceUnavailable`):** When an operation returns `ErrorKind::Evidence` while a `frozen_ctx` (either reused `existing_frozen` or freshly minted) is active, MCP determines context drift strictly through existing `fathomdb-sdk` frozen consumers (never comparing global `VisibilityStateV1`, constructing `#[non_exhaustive]` `FrozenSearchOptions` via `let mut opts = FrozenSearchOptions::default(); opts.use_graph_arm = ...; opts.limit = 1;`):
  - **`search` (`engine.search_with_evidence(&ev_req)`):** probe `engine.search_frozen(&ev_req.query, &frozen_ctx, opts)` with `opts.use_graph_arm = ev_req.use_graph_arm` and `opts.limit = 1` (exercising exact matching `SearchNoGraph` or `GraphOrEdge` surface); map `FrozenRead(StateDrifted)` to `SearchExecOutcome::Drifted`, else propagate the original error.
  - **`fetch_evidence` (`engine.resolve_evidence_many(&req)` & `graph_refs` `engine.resolve_graph_evidence(&req)`):**
    - For `resolve_evidence_many`: whole-call `Err(EvidenceUnavailable)` occurs only on `Preflight`, post-resolve `EvidenceNode`, or post-resolve `EvidenceGraph` TOCTOU (`resolve_many_raw` puts per-item failures inside `Ok(items)`). Probe `engine.search_frozen("probe", &frozen_ctx, opts)` with `opts.use_graph_arm = false, opts.limit = 1` (`SearchNoGraph`/`EvidenceNode`); if `Ok(_)` (node/vector/dependency/scope watermarks clean), probe with `opts.use_graph_arm = true, opts.limit = 1` (`GraphOrEdge`/`EvidenceGraph` TOCTOU `edge_any`). Map `FrozenRead(StateDrifted)` from either probe to `FetchEvidenceExec::Drifted` (`ToolError::state_drift()`), else propagate the original error.
    - For `graph_refs` (`engine.resolve_graph_evidence(&req)`): if `ErrorKind::FrozenRead` (`StateDrifted`) occurs, return whole-call `FetchEvidenceExec::Drifted` immediately; if `ErrorKind::Evidence` occurs while `frozen_ctx` is active, probe `engine.search_frozen("probe", &frozen_ctx, opts)` with `opts.use_graph_arm = true, opts.limit = 1` (`GraphOrEdge` — covering both node/vector/dependency and `edge_any` partitions checked by `EvidenceGraph`), mapping `FrozenRead(StateDrifted)` to whole-call `FetchEvidenceExec::Drifted` (`ToolError::state_drift()`), and only converting non-drift `ErrorKind::Evidence` into a per-item `ResolvedFetchSlot` error.
  - **`graph_expand` (`graph::expand(engine, &request)`):** `graph_expand/execution.rs` propagates `FrozenRead(StateDrifted)` directly without collapsing it to `EvidenceUnavailable`; if `ErrorKind::Evidence` still occurs while `frozen_ctx` is active (reused or freshly minted), probe `engine.search_expand_frozen("probe", &frozen_ctx, 0, SearchExpandOptions { search_limit: 1, view: None })` (`GraphOrEdge`), mapping `FrozenRead(StateDrifted)` to `ToolError::state_drift()`.
- Live `read::visibility_state(engine)` calls remain **exclusively** for populating `Envelope.state` telemetry (`EP-01`).

### 1.5 Default vs. Opt-In Decision & Compatibility with Existing Tests

**Decision: Default behavior on `ReadContextV1` (`Schema >= 39`), with zero new flags on `ReadContextV1` and zero changes to `scripts/governed-surface-pin.json` (`58` ops / `97` allowlist).**

- Existing callers, SDKs, and MCP tools gain `0%` false drift across `C1`–`C5` immediately.
- Every normative contract test passes unmodified:
  - `slice35_frozen_read.rs` (`kind="note"` -> writes `"note"` -> `StateDrifted`),
  - `slice35_frozen_read_races.rs` (`SearchFilter::default()` -> writes node -> `StateDrifted`; `_fathomdb_embedder_profiles.mean_vec` SQL mutation -> `StateDrifted`),
  - `slice45_pagination.rs` (`SearchFilter::default()` canonical & `operational_state` -> `StateDrifted`; `SearchFilter(kind="doc")` on `read_operational_state_page` -> `ContextNotApplicable`),
  - `slice50_evidence.rs` & `slice50_resolve_evidence_many.rs` (`SearchFilter::default()` -> `EvidenceUnavailable`),
  - `slice20_graph_evidence.rs` & `slice38_historical_graph.rs` (`transition`, raw SQL `UPDATE`/`DELETE` -> `StateDrifted`),
  - `slice75_feature_interactions.rs` & `slice75_schema26_upgrade.rs` (`erase_source` -> `StateDrifted`),
  - `slice35` / `slice40` normative fixtures (Schema `31`/`32`, gated on `user_version < 39`).
- Same-`(scope, kind)` additive writes (`C6`) intentionally remain drift-triggering until an opt-in `Snapshot` mode (`0.8.33+`) addresses `vec0` `write_cursor` metadata columns and recursive edge index plans.

### 1.6 Security & Erasure Non-Disclosure Proof

1. **Cross-kind provenance protection:** `scope_destructive` is keyed on `(scope, 'scope_destructive')`, **not** `kind`; any supersession, `transition`, `purge`, `erase_source`, or `physical_erasure` in `scope` on any kind (`"SourceDoc"` or `"Doc"`) invalidates all open frozen reads in `scope`.
2. **Same-ID kind-switching protection:** Replacing a `Doc` `logical_id` with `AppTurn` updates `superseded_at` on the `Doc` row, bumping both `scope_destructive` and `node:Doc`.
3. **Live snapshot authorization:** `resolve_evidence*` re-verifies `authorize_source_access`, `authorize_intrinsic_lifecycle`, SHA-256, dependency closure, and `ScopePredicate` in the live transaction.
4. **Raw SQL tamper protection:** All 20 trigger tables fail closed under `coalescing = 0` (`UPDATE`/`DELETE`/`INSERT`).
5. **Cross-scope / cross-DB token binding:** HMAC-SHA256, `database_id`, `scope_digest`, and `context_digest` are verified prior to watermark validation.

### 1.7 BM25 / Lexical Ranking Honesty Contract

As shown in Experiment 2B, SQLite's contentless `search_index` FTS5 table maintains global `(N, avgdl, df)` statistics. Across unrelated additive writes:

- **Guaranteed exact:** Eligibility-set membership, provenance/evidence resolution, `ReadView` lifecycle/validity filtering, keyset pagination order (`write_cursor`), `operational_state` values, and graph topology.
- **Explicit non-guarantee:** FTS5 `bm25(search_index)` raw scores and tie-adjacent lexical ordering reflect live `search_index` `(N, avgdl, df)` statistics rather than frozen IDF snapshots.

### 1.8 Acceptance Criteria (`AC32-FROZEN-STAB-01..06`)

- **`AC32-FROZEN-STAB-01` (Disjoint kind write & actuate stability):** `8` concurrent readers running `freeze_read_context(SearchFilter(kind="Doc")) -> search_with_evidence -> resolve_evidence_many` (and `search_frozen` / `read_canonical_page("Doc")` / `graph_expand`) while a writer commits `kind="AppTurn"` nodes via `Engine.write` and `Engine.actuate` achieve **`0%` unintended drift**.
- **`AC32-FROZEN-STAB-02` (`AppendOnlyLog` & `LatestState` isolation):** High-frequency `operational_mutations` appends produce **`0%` drift** on canonical and `operational_state` contexts; `operational_state` (`LatestState`) upserts produce **`0%` drift** on canonical/search/evidence/graph/trace contexts (even `SearchFilter::default()`) while drifting `ReadSurface::OperationalState`.
- **`AC32-FROZEN-STAB-03` (Cross-scope isolation including destructive ops):** Additive writes, `transition`, `purge`, `erase_source`, and `physical_erasure` in `"tenant-b"` produce **`0%` drift** on `"tenant-a"` `FrozenReadContextV1` tokens across close/reopen and within `fathomdb-mcp` cached sessions.
- **`AC32-FROZEN-STAB-04` (Async `projection_worker` kind/scope isolation):** Background `projection_worker` embedding `"AppTurn"` (or `"tenant-b"`) vectors produces **`0%` drift** on frozen `"Doc"` contexts in `"tenant-a"`, whereas embedding `"Doc"` vectors in `"tenant-a"` or pinning `_fathomdb_embedder_profiles.mean_vec` produces deterministic drift.
- **`AC32-FROZEN-STAB-05` (Fail-closed security, edge/graph & mixed-batch behavior):** Same-`(scope, kind)` canonical write, matching-`kind` `TextEdge` / `edge_fact` insert, same-scope edge write on `GraphOrEdge` / `EvidenceGraph` / `TraceDependency` / post-decode `Edge`/`GraphArm` slot (`resolve_evidence_many` per-slot `Refused(EvidenceUnavailable)` while node-only slots resolve), destructive mutation on any kind in `scope`, `source_dependencies` mutation in `scope`, `global_struct` update, or raw `coalescing=0` SQL mutation ALL fail closed (`StateDrifted` / `EvidenceUnavailable`).
- **`AC32-FROZEN-STAB-06` (Zero regression, wire budget & boot/inspector parity):** All existing `slice35`, `slice38`, `slice40`, `slice45`, `slice50`, `slice20`, `slice75`, MCP, and `38 -> 39` migration/admission suites pass; `fdbfr1` token size stays `<= 1,024` bytes (`492`–`556 B` across unscoped/scoped `Schema 39+`); Schema `31`/`32` normative fixtures stay bit-identical; `validate_snapshot` `p50` does not exceed baseline (`<= 0.012 ms` at `50k` rows).

---

## Decision — Part 2: Artifact-Scoped `property_predicates` on `ReadContextV1.eligibility` (`#269` Option A)

### 2.1 Amendment to `ADR-0.8.25-compact-source-evidence.md` (`#48`)

`ADR-0.8.25-compact-source-evidence.md` is amended to classify `SearchFilter` eligibility constraints explicitly across `authorize_source_access` (`src/rust/crates/fathomdb-engine/src/evidence/ranked.rs`), `query_graph_evidence_batch` (`src/rust/crates/fathomdb-engine/src/evidence/graph.rs`), and `resolve_graph_evidence` (`src/rust/crates/fathomdb-engine/src/evidence/graph.rs`):

| Constraint Tier | `SearchFilter` / Policy Fields | Applied to Target Artifact? | Applied to Canonical Source? | Rationale |
| --- | --- | --- | --- | --- |
| **Tenant Boundary** | Schema 39 `ScopePredicate` (`ScopePolicy`) | Yes | Yes | Opener-enforced multi-tenant isolation (`ADR-0.8.32` `#102`) |
| **Access-Bearing & Lifecycle Guards** | `attributes`, `created_after`, `status`, `ReadView` validity/lifecycle, `source_dependencies` closures | Yes | Yes | Access-control attributes (`tenant`, ACL labels) and revocation/erasure must block disclosure of underlying canonical source bytes (`slice50_evidence.rs` `source_bytes_must_match_access_bearing_attribute_terms`; `slice38_historical_graph.rs`) |
| **Artifact-Scoped Domain Selectors** | `kind`, `source_type`, **`property_predicates`** (`#269` Option A) | Yes | **No** (cleared on `source_filter`) | Declared `Filterable` JSON/entity properties characterize extracted/derived artifact payloads (e.g. `confidence >= 0.8`), which canonical raw sources generally do not carry |

All three source-authorization paths clear `source_filter.property_predicates.clear();` alongside `source_filter.kind = None;` and `source_filter.source_type = None;`. `SearchFilter.property_predicates` remains part of `ReadContextV1` canonical encoding and `context_digest` HMAC authentication (`frozen_read.rs`).

**Clarification on `trace_dependency` (`src/rust/crates/fathomdb-engine/src/dependency_trace.rs`):** In `trace_dependency`, `visible_artifact` validates the caller-named `root_revision_id` against the full `context.eligibility` (including `property_predicates`), whereas `TO_SOURCE_CANDIDATE_QUERY` (`ToSource` direction) and `TO_DEPENDENTS_CANDIDATE_QUERY` (`ToDependents`) filter candidate source/derived chains using only `kind`, `source_type`, `created_after`, `status`, and `attributes` (never `property_predicates`). This matches `#269` Option A cleanly: `property_predicates` stays excluded from `ToSource` candidate source filtering while continuing to gate root artifact visibility.

### 2.2 Clarification: Intentional `GraphArm` `SearchHit.source_id` vs. `ResolvedEvidenceV1.source_id`

Per `dev/plans/0.8.25/features/slice-50/design.md`, on a `GraphArm`-origin hit:

- `SearchHit.source_id` preserves the **traversed edge's legacy `source_id`** for backwards compatibility with existing `search` / `GraphArm` callers.
- `ResolvedEvidenceV1.source_id` (and `EvidenceRefV1` resolution) resolves the **returned node artifact's own canonical provenance source** via `EvidenceOriginCapture`, while `ResolvedGraphEvidenceV1.terminal_edge_evidence_ref` separately resolves the exact terminal edge's provenance (`GraphEvidenceSidecarEntryV1`).

Divergence between `SearchHit.source_id` and `ResolvedEvidenceV1.source_id` on `GraphArm` hits is intentional by contract and MUST NOT be enforced as a cross-field equality check.

### 2.3 Acceptance Criterion (`AC32-EVIDENCE-PROP-01`)

- **`AC32-EVIDENCE-PROP-01` (`property_predicates` artifact-only vs. `attributes` source-enforced):** Under `ReadContextV1(eligibility = SearchFilter(kind="Doc", attributes=[("acl", "team-a")], property_predicates=[confidence >= 0.8]))`, resolving a derived `"Doc"` artifact (`acl="team-a"`, `confidence=0.95`) backed by a canonical `"SourceDoc"` (`acl="team-a"`, **no `confidence` property**) SUCCEEDS across `search_with_evidence`, `resolve_evidence`, `resolve_evidence_many`, and `graph_expand(include_evidence=true)` + `resolve_graph_evidence`; changing the canonical source's `acl` attribute to `"team-b"` still FAILS closed with non-disclosing `EvidenceErrorReasonV1::EvidenceUnavailable` at `/evidenceRef`.

## Consequences & Rollout

- **Positive:** Completely eliminates `14.58%`–`100.00%` false-positive frozen-read drift under concurrent disjoint-`kind`, `operational_mutations`, `operational_state`, background `projection_worker`, cross-scope, and MCP session workloads; improves `validate_snapshot` `p50` by `~36%` at `50k` rows; unblocks artifact-scoped `property_predicates` on derived evidence queries; preserves `0` broken existing tests and `governed-surface-pin.json` (`58` ops / `97` allowlist) invariance.
- **Negative / Deferred:** Concurrent additive inserts matching the **same `(scope, kind)`** (or any canonical insert when `eligibility.kind = None`) continue to trigger `StateDrifted` intentionally until a future opt-in `Snapshot` mode addresses `vec0` `write_cursor` metadata columns and recursive edge index plans (`0.8.33+`). `STEP_39_SQL` adds `_fathomdb_scope_visibility_watermarks` (`WITHOUT ROWID`) and two columns on `_fathomdb_read_visibility_state` prior to `0.8.32` Schema `39` freeze.
- **Documentation & Index Sync (on Ratification / Slice 41 Implementation):** Registered ADR `#103` in `dev/adr/ADR-0.6.0-decision-index.md` (noting amendments to `#46`, `#47`, `#48` and extensions to `#70`, `#102`); synchronize `dev/interfaces/{rust,python,typescript,wire}.md` with partitioned additive stability, `BM25` non-freeze honesty, per-slot `resolve_evidence_many` edge/graph drift behavior, and artifact-scoped `property_predicates`.
