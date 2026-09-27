---
title: FathomDB 0.8.27 Slice 80 - read, search, graph, and evidence design
status: COMPLETE
target_release: 0.8.27
baseline_sha: bb077cfa
---

# Slice 80 design

## Boundary

This slice is a behavior-preserving move of root-owned read-side items in
`src/rust/crates/fathomdb-engine/src/lib.rs` (19,001 lines at `bb077cfa`)
into private modules. It also splits `graph_expand.rs` into a directory
module. Three design-review cycles plus an external code-grounded review
(`design-review.md`) closed every finding before commission. The following do
not change:

- SQL text and statement order;
- snapshot, transaction, and reader-dispatch behavior;
- error mapping and `cfg` gates;
- public and doc-hidden paths;
- schema (34), wire, bindings, and packages.

Only these changes are permitted:

- module declarations and `use` lines;
- the minimum `pub(crate)` or `pub(super)` visibility;
- root `pub use` re-exports that keep every public and doc-hidden path;
- path-only `super::` and `crate::` edits;
- link-only doc repairs;
- the gate retargets listed below.

Moved code is copied verbatim. Rustfmt re-wraps caused by a visibility
prefix are allowed.

### Authority

In order of precedence:

1. **Accepted ADRs:**
   - `ADR-0.6.0-retrieval-pipeline-shape`;
   - `ADR-0.6.0-database-lock-mechanism-reader-pool-revision`;
   - `ADR-0.6.0-single-writer-thread`;
   - `ADR-0.8.0-filter-grammar`;
   - `ADR-0.8.0-graph-traversal-scope`;
   - `ADR-0.8.25-eligibility-and-frozen-reads`;
   - `ADR-0.8.25-frozen-pagination-and-operational-state`;
   - `ADR-0.8.25-compact-source-evidence`;
   - `ADR-0.8.25-governed-dependency-trace-and-integrity`;
   - `ADR-0.8.25-absolute-read-performance-successor`;
   - `ADR-0.8.26-exact-graph-artifact-evidence`.
2. **Active and locked designs:** `dev/design/retrieval.md`,
   `retrieval-result-limits.md`, `engine.md`, `bindings.md`, and
   `errors.md`.
3. **Slice designs:** the 0.8.25 Slice 35/45/50/55/60 designs and the 0.8.26
   Slice 10/20/60 designs.
4. **The tests.**

## Destination modules

Every module is private. Every item keeps its `cfg` and `doc(hidden)`.

- **Re-import strategy (Slice 70):** after each move, `lib.rs` re-imports
  the moved items (`use x::{...}`, with the item's `cfg`). Sibling modules
  that name them as `super::X` or `crate::X` therefore resolve unchanged.
  These include `evidence.rs`, `frozen_read.rs`, `dependency_trace.rs`,
  `graph_expand`, `projection_registry.rs`, `projection_runtime.rs`, and
  `vector_equivalence.rs`. Their files are not edited.
- **No field widening (PW27-4A):** no struct field widens. Data carriers
  whose private fields are built at root stay at root (see
  `reader_pool.rs`). A child module of the crate root can read root-private
  fields. The same rule keeps `TelemetrySink` (its `path` is read by
  `erasure.rs`) and `EvidenceCapture` (constructed by `reader_pool.rs`) at
  root; the implementation first moved them and widened four fields, which the
  post-hoc design review (2026-09-27) found and reverted.

- **Placement rule:** a helper used only by moved items moves with them. A
  helper shared with root items stays at root, or moves with its dominant
  owner as `pub(crate)`.
- **Dependency direction:** the root re-import strategy makes every moved
  item reachable from every module, so a cycle would compile silently. The
  intended direction is recorded here, and the code review checks each new
  module's `use`/`super::` references against it. The rules apply at
  **facade-vs-handler granularity**: an `impl Engine` facade that lives in
  the same module as its handlers (for example the read facades in `read.rs`
  or the traversal facades in `graph_expand/traversal.rs`) may call
  `reader_pool` dispatch or `search_api` even though that module's handlers
  must not. At module granularity this produces the facade-induced cycles
  listed below.
  - `search_types` depends only on carrier and value types.
  - `filter` and `fusion` never depend on `search_api` or `telemetry`.
  - Among the new modules, `search` never depends on `search_api` or
    `telemetry`. It also uses `temporal`, `dependency_closure`, `evidence`,
    and `frozen_read`.
  - One module cycle exists today and is kept verbatim as an accepted Slice 90
    seam: `search` ↔ `graph_expand` (`read_search_in_tx` calls
    `search_expand_on_snapshot`; `graph_expand/execution` calls
    `structural_dependency_state`). No new handler-level cycle may be added.
  - Three further module-level cycles are facade-induced and accepted as
    Slice 90 seams (recorded by the post-hoc design review, 2026-09-27; the
    earlier claim that `search` ↔ `graph_expand` was the only cycle was
    false):
    - `read` ↔ `reader_pool`: the `read.rs` `impl Engine` facades call
      `self.reader_pool.dispatch`, and the pool calls the `read_*_in_tx`
      handlers.
    - `graph_expand` ↔ `reader_pool`: the traversal and execution facades
      (`traversal.rs`, `execution.rs`) call dispatch, and the pool calls the
      traversal and `read_graph_expand_in_tx` handlers.
    - `graph_expand` ↔ `search_api`: the `search_expand*` facade in
      `traversal.rs` calls `self.search_inner`, and `search_api.rs` names
      `traversal::SearchExpandResult`.
  - The four cycles above are complete **among Slice 80's modules**. Three
    further cycles with earlier-slice modules were inherited unchanged from
    root ↔ module cycles when code left the root; they are recorded here as
    Slice 90 context, not as new Slice 80 seams:
    - `search` ↔ `dependency_closure`: `dependency_closure.rs` calls
      `crate::prepare_search_statement`, and `search.rs` calls
      `dependency_closure::read_eligibility_sql`.
    - `search` ↔ `evidence`: `evidence.rs` uses `CapturedGraphOrigin`, and
      `search.rs` calls `evidence::build_search_result`.
    - `graph_expand` ↔ `evidence`: `evidence.rs` uses `TraversalDirection`,
      and `graph_expand/execution.rs` uses `GraphEvidenceAuthority`.
  - `reader_pool` calls the read, search, and graph handlers. Those handlers
    use the root-private `begin_attributed_reader_tx` primitive, so no
    handler depends back on `reader_pool`; only facades do.
  - `search_api` and `telemetry` may call domain logic; domain handlers never
    call them. The `graph_expand` traversal facade calling `search_inner` is
    the accepted facade exception above.
  - `graph_expand/codec` depends on `graph_expand/types` and on crate
    value and carrier types (`ReadContextV1`, `ReadView`, `SearchFilter`,
    the `Structural*StateV1` types, `ArtifactRevisionId`, and the evidence
    sidecar types), never on `execution` or `traversal` functions.
    `graph_expand/types` depends on nothing else in `graph_expand`.
  - The reader carriers kept at root are a temporary Slice 90 seam, not a
    destination.
- **Public items:** each moved public item gets a root `pub use` with the
  same `cfg`.
- **Seam table:** `tdd-chronology.md` records every visibility change. The
  reader-pool functions that must become `pub(crate)` are known now:
  - the pool methods `new`, `dispatch`, `shutdown`, `worker_count`,
    `live_count`, `next_worker_index`, `lookaside_used_per_worker`,
    `cache_status_per_worker`, `secure_delete_per_worker`,
    `wal_connection_inventory_for_test`, and
    `wal_native_state_inventory_for_test`.

  `reader_worker_loop`, `finish_reader_request`,
  `read_lookaside_used_hiwtr`, and `read_cache_status` stay private.
  `begin_attributed_reader_tx` stays root-private because read, search, and
  graph handlers share it and the pool dispatches to those handlers.

| Module | Items at `bb077cfa` (by name) |
| --- | --- |
| `fusion.rs` | `RRF_K`, `RRF_WEIGHT_VECTOR`, `RRF_WEIGHT_TEXT`, `RRF_WEIGHT_GRAPH`, `RECENCY_WEIGHT`, `fuse_rrf`, `fuse_three_arms`, `apply_recency_reweight`, `apply_importance_reweight`, `build_importance_confidence_maps` |
| `filter.rs` | `ScalarValue`, `ComparisonOp`, `PREDICATE_PATH_ALLOWLIST`, `Predicate` (and impl), `SearchFilter` (and impl), `FilterTerm`, `Filter` (with `TryFrom` and impl), `validate_filter_attributes_on_snapshot`, `vector_filter_clause`, `vector_filter_values`, `build_vector_phase1_sql`, `append_node_eligibility_sql`, `append_edge_eligibility_sql`, `body_fts_rank_sql`, `edge_fts_rank_sql`, `property_fts_rank_sql`, `text_hit_passes_filter`, `hit_attributes_pass_filter`, `edge_fts_hit_passes_filter`, `edge_fts_hit_passes_non_attribute_filter` |
| `search_types.rs` | `SoftFallback`, `SoftFallbackBranch`, `SearchHit`, `GraphFrontierStats` (and impl), `SearchResult`, `Explanation`, `StructuralInclusionStateV1`, `StructuralProjectionOriginV1`, `StructuralDependencyStateV1`, `StructuralLifecycleStateV1`, `StructuralDegradationCodeV1`, `StructuralInclusionV1`, `QueryTrace`, `PerHitExplain`, `Bm25fFieldWeights`, `Bm25fQueryPlan` (with `Default` impls), `TOP_K_BIT_CANDIDATES`, `SEARCH_RERANK_LIMIT`, `DEFAULT_SEARCH_RESULT_LIMIT`, `MAX_SEARCH_RESULT_LIMIT`, `validate_search_result_limit` |
| `search.rs` | `structural_dependency_state`, `structural_lifecycle_state`, `read_projected_text_in_tx`, `CapturedGraphOrigin`, `SearchOriginCapture`, `NoEvidenceCapture` (with its size assert), the `SearchOriginCapture` impl for the root `EvidenceCapture`, `read_search_work_in_tx`, `SearchStatement` (and impls), `prepare_search_statement`, `load_projection_cursor_for_search`, `read_search_in_tx`, `rank_search_hit_from_row`, `collect_complete_rank_boundary`, `retain_complete_rank_boundary_candidates`, `bfs_graph_arm_candidates` (generic over `SearchOriginCapture`; it uses `CapturedGraphOrigin`, `SearchHit`, `GraphFrontierStats`, and `append_node_eligibility_sql`, not the BFS builders), the `test-hooks` search witnesses (`append_json_witness_for_test`, `record_fts_route_for_test`, `slice71_search_statement_trace`, `record_slice71_profile_statement_for_test`, `record_fts_query_plan_for_test`), `fts5_tokenize`, `bm25f_match_expression`, `bm25f_score_doc`, `bm25f_search_inner` |
| `search_api.rs` | `impl Engine`: `freeze_read_context`, `validate_frozen_read_context_for_binding`, `search_frozen`, `search_with_evidence`, `resolve_evidence`, `resolve_graph_evidence`, `search_expand_frozen`, `tc5_vector_stage`, the `search*` variants (`search` through `search_projected_text_with_limit`), `dense_disabled`, `dense_disabled_reason`, `vector_equivalence_refusal_count`, `search_reranked_with_explain`, `search_inner`, `search_inner_with_stats`, `search_inner_with_frozen_binding_and_stats`, `search_inner_with_frozen_binding_and_expansion`, `bm25f_search` |
| `telemetry.rs` | `append_jsonl`, `branch_str` (both used only by telemetry); `impl Engine`: `enable_telemetry`, `last_telemetry_query_id`, `capture_telemetry`, `finalize_search_observability`, `mint_explanation_correlation_id`, `capture_telemetry_with_sink`, `record_feedback` |
| `read.rs` | `NodeRecord`, `OpStoreRow`, `OperationalStateRecordV1`, `READ_COLLECTION_MAX_LIMIT`, `read_get_by_id_in_tx`, `read_collection_in_tx`, `read_list_in_tx`, `read_canonical_page_in_tx`, `read_canonical_page_baseline_in_tx`, `query_canonical_page_rows`, `canonical_page_query`, `OPERATIONAL_STATE_POINT_SQL`, `OPERATIONAL_STATE_PAGE_SQL`, `read_operational_state_in_tx`, `read_operational_state_page_in_tx`, `validate_operational_context`, `validate_operational_collection`, `page_search_error`; `impl Engine`: `read_get`, `read_get_many`, `read_collection`, `read_mutations`, `read_collection_dispatch`, `read_list`, `read_list_filter`, `read_canonical_page`, `read_operational_state`, `read_operational_state_page`, `receive_page_result` |
| `reader_pool.rs` | **Functions only:** `impl ReaderWorkerPool` (including `Drop`), `reader_worker_loop` (moved whole, including its inline WAL and diagnostic match arms: `HoldWalSnapshot*`, `LookasideStatus`, `CacheStatus`, `SecureDeleteStatus`, and `Wal*Inventory`, with their snapshot-hold behavior verbatim), `finish_reader_request`, `read_lookaside_used_hiwtr`, `read_cache_status`. The pool's two `*_for_test` methods (`wal_connection_inventory_for_test` and `wal_native_state_inventory_for_test`) move with the impl, because they are pool methods, not `Engine` seams. Slice 140's inventory must include them. **Kept at root** as shared infrastructure and data carriers: the private `begin_attributed_reader_tx` primitive; `ReaderWorkerPool` (struct and `Debug`), `SearchReaderWork`, every `*ReaderRequest` struct, `ReaderRequest`, `FrozenQueryRuntime`, the response aliases, `SearchReaderError`, `PageReaderError` (and their `From` impls), `CacheStatusReply`, the `READER_*` constants, and the `Reader*Pause` aliases (used only by the root `WalAttributionCollector`). The carrier fields are built at root, in `graph_expand`, and in Slice 90's WAL seams (`self.reader_pool.senders[0]`). Slice 90 owns these carriers, the shared transaction primitive, and the WAL arms. |
| `graph_expand/` | Replaces `graph_expand.rs`. `mod.rs` holds declarations and re-exports that keep every `crate::graph_expand::X` path, including `read_graph_expand_in_tx` and `GraphExpandReaderControlsForTest`. The submodules are listed in the next four rows. |
| `graph_expand/types.rs` | Current lines 24-25 (`SCHEMA_VERSION`) and 90-378, plus `TraversalDirection` from `lib.rs` (a value type used by the request types and the codec; its root `pub use` keeps the public path). `SCHEMA_VERSION` becomes `pub(super)`. |
| `graph_expand/codec.rs` | Current lines 1922-3470, plus `is_false` (lines 26-28). |
| `graph_expand/execution.rs` | Current lines 30-89 and 379-1920 (validation included). The in-file `graph_evidence_request_tests` (3472-3525) do **not** move here: they go to `graph_expand/mod.rs`, so their qualified names stay `lib::graph_expand::graph_evidence_request_tests::*`, as the hidden baseline records. `mod.rs` brings the items those tests name into scope (`use super::*` inside the test module then resolves them); an item private to a submodule becomes `pub(super)`, which is item visibility, not field widening. The imports that only the tests need (`execution::encode_graph_evidence_request`, which becomes `pub(super)`; `ReadContextV1`, `ReadView`, `SearchFilter`, `IdSpace`, `FrozenReadContextV1`) go in `#[cfg(test)] use` lines in `mod.rs`, so the non-test build has no unused imports. Names already re-exported by `mod.rs`, such as `TraversalDirection`, are not imported again. |
| `graph_expand/traversal.rs` | `SearchExpandResult`, `GRAPH_NEIGHBORS_HARD_CAP`, `build_bfs_sql`, `build_bfs_with_depth_sql`, `crossed_boundary_since_in_tx`, `graph_neighbors_in_tx`, `search_expand_in_tx`, `search_expand_on_snapshot`, `explain_graph_neighbors_in_tx`; `impl Engine`: `graph_neighbors`, `search_expand`, `search_expand_with_limit`, `crossed_boundary_since` |

### Stays at root

- **Slice 90:**
  - the open, runtime, WAL, and operator facade, including
    `mint_explanation_open_nonce` and its nonce `static`;
  - `detect_slow`, `emit_event`, and `emit_sqlite_internal_error`;
  - the reader data carriers listed under `reader_pool.rs`;
  - the `TelemetrySink` and `EvidenceCapture` carriers, kept at root so their
    private fields do not widen (post-hoc design review, 2026-09-27);
  - the shared private `begin_attributed_reader_tx` primitive;
  - `configure_reader_lookaside` and `apply_perf_experiment_reader_pragmas`,
    called only from the open path;
  - `Engine::usable_dense_runtime`, a runtime helper called from projection
    code and `lib.rs`;
  - the write/search index projectors (`project_canonical_node_row`,
    `project_canonical_edge_row`, `IndexTargetSet`,
    `index_targets_for_row_kind`, `reproject_search_index_after_tokenizer_upgrade`,
    `search_index_tokenizer_reproject_complete`, `CanonicalNodeRow`,
    `canonical_node_rows`, `row_kind_from_column`).
- **Slice 140:**
  - every `*_for_test` `Engine` seam defined in `lib.rs`. The seams
    already in `graph_expand.rs` (`measure_graph_expand_for_test`, the
    `seed_graph_expand_*_for_test` and `graph_expand_with_*_for_test`
    families, `explain_graph_expand_for_test`, and
    `pub fn graph_expansion_degradation_codes_for_test`) move with the file
    into `graph_expand/execution.rs`;
  - every root `pub fn *_for_test` (`vector_phase1_sql_for_test`,
    `slice35_ranked_eligibility_sql_for_test`,
    `take_slice71_search_statement_trace_for_test`), now calling the moved
    `pub(crate)` items;
  - `record_writer_pragma_witness_for_test`, which is writer-side.
- **Shared, stays at root:**
  - `RowKind` (write and read);
  - `hex_encode` and `hex_nibble`;
  - `MEAN_VEC_PIN_THRESHOLD` and `mean_centering_internals_for_test`, which
    stay at root per Slice 70;
  - `write_node_importance` and `node_importance`, which are writes and
    simple reads of importance, not search;
  - `apply_perf_experiment_writer_pragmas`;
  - any item not listed above.

### Search-owned runtime fields

`search_limit_override`, `recency_reweight_enabled`,
`importance_reweight_enabled`, and `vector_stage_only_for_test` stay on
`ProjectionRuntimeShared`, with its shape unchanged. The rationale is in
`plan.md`. The moved search facades read them through the same
`self.projection_runtime.shared` path. Slice 90 may relocate them when it
finalizes `Engine` state.

## Batches (one commit each, about 300-1,200 moved lines)

1. **`fusion.rs`** (about 300 lines).
2. **Filter types** into `filter.rs` (about 445).
3. **Filter functions** into `filter.rs` (about 545). Retarget the
   `plan-0.8.20.md` citations of `text_hit_passes_filter` and
   `edge_fts_hit_passes_filter`.
4. **`search_types.rs`** (about 450), including the search-limit constants.
   Retarget the C1 `SearchHit` probe and the C1 self-test fixtures (arms 12p
   and 12w).
5. **`search.rs` helpers**: capture types, statement and rank-boundary
   helpers, witnesses, and BM25F (about 720).
6. **`search.rs` graph arm**: `bfs_graph_arm_candidates` (about 425).
7. **`search.rs` core**: `read_search_in_tx` alone (about 1,110).
8. **`search_api.rs` part A**: the frozen and evidence facades plus the
   `search_inner*` family (about 690).
9. **`search_api.rs` part B plus `telemetry.rs`**: the search variants, the
   dense-status methods, `search_reranked_with_explain`, and telemetry
   (about 800). Retarget the `plan-0.8.20.md` citation of
   `pub fn search_filtered`.
10. **`read.rs`** (about 960).
11. **`reader_pool.rs`**: the functions only (about 640). Retarget the
    Windows WAL guard (see gates).
12. **Graph split, first step:** `git mv graph_expand.rs
    graph_expand/execution.rs`, add `mod.rs`, move the in-file tests into
    `mod.rs`, and move the types into `types.rs` (about 330). Retarget
    `slice60_fix1_wire`. Confirm that the two graph unit-test names are
    unchanged in `cargo test --lib -- --list`.
13. **Request codec** into `graph_expand/codec.rs` (about 755).
14. **Result codec** into `graph_expand/codec.rs` (about 795).
15. **`graph_expand/traversal.rs`** (about 575).

A `concat!` gate can list only files that already exist, so each batch
extends the gates for the modules it creates:

- **Modules extracted from `lib.rs`:** added to the slice35 manifest
  `SOURCE`, with the boundary sentinel.
- **`graph_expand/*.rs` files:** added only to `slice60_fix1_wire`. The
  slice35 manifest never scanned `graph_expand.rs`, and its scope stays
  unchanged.

Heavy public and hidden captures run in three places: at the pre-move
receipt, after batch 9 (by then most root re-exports exist), and at the
final candidate. AC-050c runs every batch against the pre-move base as the
cheap public-removal check.

## Characterization

### Existing owners (reused, not duplicated)

**R27-80B — snapshot authority and transaction lifetime:**

- `slice35_frozen_read_races`
- `slice45_pagination::{page_reader_snapshot_linearizes_before_concurrent_write, operational_page_snapshot_linearizes_before_concurrent_replacement, operational_point_snapshot_linearizes_before_concurrent_replacement}`
- `slice50_evidence::{evidence_search_races_are_snapshot_atomic_or_wholly_refused, evidence_linearizes_at_sidecar_and_resolver_return_seams}`
- `slice60_graph_expand::{frozen_pre_pin_drift_and_post_pin_isolation_use_bounded_cancellation_safe_hooks, current_post_pin_write_linearizes_after_the_complete_operation}`
- `slice35_after_validation_races`
- `slice20_graph_evidence::graph_resolver_and_erasure_spellings_linearize_under_the_primary_mutex`
- `reader_pool` (7 tests)
- `cursors::concurrent_search_does_not_observe_speculative_failed_cursor`
- the lib `wal_attribution_reader_*` family

**R27-80C — eligibility before every bounded cap:**

- `slice35_eligibility_pretruncation` (including its ranked-SQL and plan
  tests)
- `graph_frontier_applies_target_eligibility_before_its_edge_cap`
- `every_eligibility_term_executes_before_page_truncation`
- `slice60_graph_expand::{lifecycle_supersession_and_indexed_eligibility_apply_before_admission, result_limit_selects_top_n_only_after_the_walk_is_complete}`
- `pr_g10_filtered_knn`, `slice15e_prekn_filterable`, and
  `tc33_fix2_edge_validity_on_search`

**R27-80D — ordering and fusion:**

- `pr_g9_rrf_fusion`, `f9_importance_ranking`, and `pr_g12_recency`
- `slice23_text_limit_prefix_stability`
- `slice60_graph_expand::{cycles_self_loops_parallel_edges_and_multiple_origins_are_deterministic, edge_only_insertion_permutations_have_identical_complete_response_bytes}`
- `evidence_pins_the_traversal_winner_among_parallel_edges`
- lib `completed_rank_group_matches_the_full_stable_prefix`

**R27-80E — codecs:**

- `slice60_wire` (proptest and canonical fixtures), `slice60_fix1_wire`,
  `slice60_fix2_wire`, and `slice60_fix3_wire`; these also own error
  precedence (typed refusal order for malformed carriers)
- `slice55_wire` (proptest)
- the in-file tests of `evidence.rs`, `frozen_read.rs`, and `pagination.rs`
- `slice50_evidence::authorized_*_corruption_is_typed*`
- `slice35_frozen_read`

**R27-80F — visibility and filters.** Search applies the validity axis and
refuses existence relaxation. Consideration TC-38 is covered for these
paths:

- `slice15b_search_validity::{read_view_on_search_selects_by_instant_and_can_relax_validity, text_only_search_also_hides_out_of_window_nodes, filtered_and_explained_search_hide_out_of_window_nodes, search_refuses_a_view_that_relaxes_the_existence_axis}`
- `opp12_existence_axis::{r_ex_2_pending_absent_from_default_search_and_read, r_ex_2_vector_search_excludes_superseded_node_version}`
- `slice35_filter_grammar`, `slice40_filter_unification`, and
  `slice10_read_view`
- **Not covered:** the graph arm and `search_projected_text` under a
  non-default view. Both get the mandatory characterization below.

**Plan and statement structure:**

- `slice60_graph_expand::endpoint_plans_use_shipped_indexes_without_full_edge_scan_and_schema_stays_34`
- `slice20_graph_traversal::explain_plan_uses_indexes`
- `slice45_pagination::page_query_plans_use_governed_indexes_without_mutation_log_or_temp_sort`
- `slice71_search_statement_trace`
- `slice20_graph_evidence::evidence_hydration_executes_zero_or_exactly_two_sql_statements`

### Added (proven gaps): reader transaction release after refusal (R27-80B)

No owner proves that a search refused *inside* a reader transaction releases
that worker's snapshot. The reader-pool tests cover success paths, and the
WAL-attribution tests cover erasure refusals. A leaked snapshot would pin
the WAL and silently degrade later erasure.

- **Where:** `tests/slice80_reader_transaction_release.rs`, default
  features.
- **Test:**
  1. Seed a database.
  2. Issue exactly one refused search per worker: eight sequential refusals.
     `READER_POOL_SIZE` is 8 and private, and dispatch is round-robin, so
     each worker takes exactly one. Debug-only guards, gated on
     `cfg(debug_assertions)` so the release `--tests` check compiles, assert
     two things: `reader_worker_count_for_test() == 8`, and the per-dispatch
     progression. `next_reader_worker_index_for_test()` returns the counter
     modulo the worker count, so its value after eight dispatches equals its
     value before; comparing only the endpoints would be vacuous. Instead,
     record `start` before the first refusal and assert that the index is
     `(start + i) % 8` before refusal `i` and `(start + i + 1) % 8` after
     it. Together these prove one dispatch per refusal onto 8 distinct
     workers. Each call asserts the typed `InvalidFilter`. That rejects any
     refusal made before dispatch, such as a closed engine, disabled dense
     search, or an empty query. The refusal must happen on the reader
     snapshot. The expected refusal is a filter naming an attribute that is
     not projected, which `validate_filter_attributes_on_snapshot` rejects.
     The TDD step confirms the refusal site and uses another in-transaction
     refusal if needed.
  3. Assert the typed error each time.
  4. Assert that `erase_source` on the seeded source completes. That
     requires the WAL checkpoint at rest to succeed, which is impossible
     while any reader holds a snapshot.
  5. Assert that a following search returns the expected hits.
- **Non-vacuity mutant:** on the refusal path, keep the reader transaction
  (or its connection's read snapshot) alive after returning the error.
  - **Where it fails:** with one refusal per worker, no poisoned worker is
    reused, so the test fails at the erasure assertion. The failure must be
    recorded.
  - **Hang risk:** there is none. Erasure retries a bounded 5 × 25 ms. The mutant is reverted, and a byte comparison confirms the
  production file is restored.
- **If the test fails on unmodified production:** stop for a RED/GREEN fix.

### Added (proven gaps): search view paths (R27-80F)

Search's shipped contract applies the validity axis and refuses existence
relaxation (`ReadView::reject_existence_relaxation_on_search`). The owners
cover the hybrid, text-only, filtered, and explained paths. No owner asserts
validity exclusion or inclusion on the graph arm or on
`search_projected_text` under a non-default view. (`slice50_evidence` runs
the graph arm with `valid_as_of: Some(1_000)` but asserts evidence, not
window exclusion; every `search_projected_text` call uses
`ReadView::default()`.)

- **Where:** `tests/slice80_search_view_paths.rs`, default features.
  Fixtures follow `slice15b_search_validity`: set `valid_from`/`valid_until`
  through the governed write path, with the far-future and past constants.
- **Graph arm** (`search_reranked_view`, `use_graph_arm = true`,
  `rerank_depth = 0`): a seed node matches the query and is in window; an
  edge reaches a neighbor that does not match the query and whose window
  has closed. The seed's window must also cover the assertion-3 instant
  (`valid_from` NULL), and the edge must have NULL or later `t_invalid`,
  because edge validity binds the same instant (`temporal.rs` `edge_now`).
  Otherwise the arm has no root and the test fails for the wrong reason.
  `slice30_graph_arm::graph_arm_temporal_fallback_excluded_or_downweighted`
  shows a non-matching neighbor surfacing as a hit.
  1. Under the default view the neighbor is absent.
  2. Under `include_out_of_window` it is present, which proves the arm
     reached it.
  3. Under a `valid_as_of` inside its window it is present.
  4. `include_superseded` and `include_inactive` each return
     `InvalidArgument`.
- **Projected text** (`search_projected_text` on a declared `searchable`
  projection): the same four assertions for one in-window and one
  out-of-window node that both match.
- **Non-vacuity mutants:** each must fail assertion 1 and be recorded;
  restore the code and compare bytes.
  - **Graph arm:** node validity on the neighbor path is applied twice, so
    one mutant removes both: the validity part of `target_node`
    (`view.node_sql("target", 3)`, the edge-query join) and the
    `body_validity` string (`view.validity_sql("canonical_nodes", 2)`, the
    hydration query), keeping every bound parameter referenced (`?2` and
    `?3` stay in use through `read_eligibility_sql`). The seed and resolve
    sites are out of scope, because the neighbor never seeds.
  - **Projected text:** remove the validity part of `frozen.node_sql("n",
    3)` in `read_projected_text_in_tx`, its only validity site.
- **If the TDD step finds that the fixture cannot reach a path** (for
  example, the graph arm does not surface a non-matching neighbor as a
  hit), it adjusts the fixture, not the assertions, and records why.
- **If a test fails on unmodified production:** stop for a RED/GREEN fix.

## Source-scraping gates and retargets

Each gate runs in **every** batch. Retargets are path-only and made in the
batch that moves the named item:

- **`scripts/check-c1-conformance.sh`.** Batch 4 moves `SearchHit`:
  - Show the gate RED, then add a `SEARCH_TYPES` path constant listed in
    `--list-sources` and retarget the `SearchHit` probe.
  - The `ProjectionVector`, `ProjectionFts`, `ProjectionRole`,
    `ProjectionSpec`, `ProjectionDelta`, `DenseReadiness`, and
    `DEFAULT_EMBEDDER_NAME` probes stay on `lib.rs`.
  - Run `scripts/tests/test_check_c1_conformance.sh`.
- **`tests/slice60_fix1_wire.rs`.** It reads `graph_expand.rs` through
  `include_str!`. Batch 12 changes it to a `concat!` of the
  `graph_expand/*.rs` files and keeps every needle, including the
  `#[cfg(feature = "test-hooks")]\nstruct GraphExpandPinRendezvous`
  adjacency.
- **`tests/slice35_virtual_mutation_manifest.rs`.**
  - Append each new module to `SOURCE` with the boundary sentinel.
  - Needles and counts are unchanged.
  - The `manifest_bodies.py` body check from Slice 70 stays empty.
- **`experiments/slice35_virtual_mutation_audit.py`.** This whole-crate
  scanner is keyed by file. Re-key any mutation site that moves; none is
  expected, because the index projectors stay at root.
- **`scripts/tests/test_windows_wal_attribution_ci_job.sh`.** Batch 11
  breaks it:
  - `wal_attribution.fire_reader_completion_pause(connection.is_autocommit())`
    lives in `finish_reader_request`, which moves.
  - Two of the three `WalConnectionInventory` occurrences (the pool impl and
    the worker loop) move. The one in the root `ReaderRequest` enum stays.
    The design does not rely on that: every needle whose owner moves is
    retargeted.
  - **Retarget:** follow the `RUNTIME_SOURCE` precedent (`36fc2352`). Add an
    injectable `READER_POOL_SOURCE` for `reader_pool.rs`:
    - add it to the marker concatenation;
    - point the reader-completion-pause assertion at it.

    No current fixture mutates these needles. The recursive fixture runs
    inherit the default path.
  - **Unchanged:** the `lib.rs` production `Connection::open(` count stays
    1, because `open_managed_connection` stays at root.
  - It runs every batch.
- **`scripts/lint-plan-anchors.sh`.** In the ACTIVE `plan-0.8.20.md`:
  - the two post-filter citations are updated in batch 3;
  - the `pub fn search_filtered` citation is updated in batch 9;
  - the `truncate_wal` and `wal_checkpoint_truncate_once` citations stay on
    `lib.rs`.
- **`scripts/tests/test_check_c1_conformance.sh`.** Fixture arms 12p and 12w
  assert on and edit `SearchHit`'s `pub id: IdSpace` line in `lib.rs`. Batch
  4 points both at `search_types.rs`, so the decoy still exercises the
  retargeted probe.
- **Comment anchors.** Two comments in `tests/perf_gates.rs` cite `lib.rs`
  lines for moved SQL. They receive link-only path repairs.
- **Hidden-surface rows.**
  - Moved doc-hidden items keep effective `cfg` and hidden values through
    `pub use`: `fuse_rrf`, `fuse_three_arms`, `apply_recency_reweight`, and
    the `graph_expand` `*ForTest` re-exports. `CacheStatusReply` stays at
    root.
  - `arm_reader_search_hook_for_test` and
    `take_slice71_search_statement_trace_for_test` stay at root.
- **Other readers.** `lib.rs` `mod tests` and `evidence.rs` tests that reach
  moved private items get path-only imports. The `crate::graph_expand::`
  paths hold through `mod.rs`.

## Evidence

- **Pre-move:**
  - public and hidden captures;
  - baseline route counts from the extended
    `batch-check.sh` (scratchpad). It is extended with the read-side owner
    routes above:
    - default;
    - `test-hooks,migration-test-hooks`;
    - `operator,test-hooks`;
    - `default-reranker`;
    - `default-embedder`;
    - `tc5-benchmark`, for `tc5_vector_stage` and the `VectorStage` path;
  - the WAL guard, `slice60_fix1_wire`, and the C1 self-test.
- **Per batch:** the ten feature-route `cargo check`s, the release
  `--tests` check, crate Clippy with warnings denied on two routes,
  `cargo fmt --check`, the owner routes with counts equal to the baseline
  (zero tests counts as a failure), and the gates above. Also AC-050c
  against the pre-move base, and `wc -l src/lib.rs`.
- **Final:**
  - the `agent-verify.sh` canonical gate;
  - workspace Clippy and check;
  - the Python receipt;
  - strict security with live AC-037 through the HITL runbook;
  - `scripts/test-feature-complete.sh` on the RTX 3090 host;
  - the public capture, equal to the Slice 30 baseline;
  - the hidden capture, additive only (the two new test files; no
    existing test path renamed).

## Risks

- **Reader pool seam.**
  - The worker loop moves whole, including its inline WAL and diagnostic
    arms, so WAL snapshot-hold behavior now lives in `reader_pool.rs`.
  - The data carriers stay at root, and no field widens.
  - This seam is handed to Slice 90 explicitly: moving the carriers and
    extracting the WAL arms.
- **`read_search_in_tx` size.** At about 1,110 lines in one function, the
  batch is a single contiguous cut. It must not be refactored.
- **The graph file split.** `git mv` preserves history for `execution.rs`.
  The codec batches must keep item order within `codec.rs`. The in-file
  `graph_evidence_request_tests` go to `graph_expand/mod.rs` so their
  qualified names do not change; the hidden capture checks this.
- **Doc-hidden re-exports.** `rerank.rs` showed that a redundant
  `#[doc(hidden)]` on the re-export is harmless. The hidden capture is the
  oracle.
- **Scale.** Fifteen batches touch the hottest read paths. The owner-route
  count equality and the gates in every batch are the safety net. A
  compile-driven logic change stops the batch.
