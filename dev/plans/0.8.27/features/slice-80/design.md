---
title: FathomDB 0.8.27 Slice 80 - read, search, graph, and evidence design
status: PROPOSED
target_release: 0.8.27
baseline_sha: bb077cfa
---

# Slice 80 design

## Boundary

This slice is a behavior-preserving move of root-owned read-side items in
`src/rust/crates/fathomdb-engine/src/lib.rs` (19,001 lines at `bb077cfa`)
into private modules. It also splits `graph_expand.rs` into a directory
module. The following do not change:

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

- **Placement rule:** a helper used only by moved items moves with them. A
  helper shared with root items stays at root, or moves with its dominant
  owner as `pub(crate)`.
- **Public items:** each moved public item gets a root `pub use` with the
  same `cfg`.
- **Seam table:** `tdd-chronology.md` records every visibility change.

| Module | Items at `bb077cfa` (by name) |
| --- | --- |
| `fusion.rs` | `RRF_K`, `RRF_WEIGHT_VECTOR`, `RRF_WEIGHT_TEXT`, `RRF_WEIGHT_GRAPH`, `RECENCY_WEIGHT`, `branch_str`, `fuse_rrf`, `fuse_three_arms`, `apply_recency_reweight`, `apply_importance_reweight`, `build_importance_confidence_maps` |
| `filter.rs` | `ScalarValue`, `ComparisonOp`, `PREDICATE_PATH_ALLOWLIST`, `Predicate` (and impl), `SearchFilter` (and impl), `FilterTerm`, `Filter` (with `TryFrom` and impl), `validate_filter_attributes_on_snapshot`, `vector_filter_clause`, `vector_filter_values`, `build_vector_phase1_sql`, `append_node_eligibility_sql`, `append_edge_eligibility_sql`, `body_fts_rank_sql`, `edge_fts_rank_sql`, `property_fts_rank_sql`, `text_hit_passes_filter`, `hit_attributes_pass_filter`, `edge_fts_hit_passes_filter`, `edge_fts_hit_passes_non_attribute_filter` |
| `search_types.rs` | `SoftFallback`, `SoftFallbackBranch`, `SearchHit`, `GraphFrontierStats` (and impl), `SearchResult`, `Explanation`, `StructuralInclusionStateV1`, `StructuralProjectionOriginV1`, `StructuralDependencyStateV1`, `StructuralLifecycleStateV1`, `StructuralDegradationCodeV1`, `StructuralInclusionV1`, `QueryTrace`, `PerHitExplain`, `Bm25fFieldWeights`, `Bm25fQueryPlan` (with `Default` impls), `TOP_K_BIT_CANDIDATES`, `SEARCH_RERANK_LIMIT`, `DEFAULT_SEARCH_RESULT_LIMIT`, `MAX_SEARCH_RESULT_LIMIT`, `validate_search_result_limit` |
| `search.rs` | `structural_dependency_state`, `structural_lifecycle_state`, `read_projected_text_in_tx`, `CapturedGraphOrigin`, `SearchOriginCapture`, `NoEvidenceCapture` (with its size assert), `EvidenceCapture`, `read_search_work_in_tx`, `SearchStatement` (and impls), `prepare_search_statement`, `load_projection_cursor_for_search`, `read_search_in_tx`, `rank_search_hit_from_row`, `collect_complete_rank_boundary`, `retain_complete_rank_boundary_candidates`, the `test-hooks` search witnesses (`append_json_witness_for_test`, `record_fts_route_for_test`, `slice71_search_statement_trace`, `record_slice71_profile_statement_for_test`, `record_fts_query_plan_for_test`), `fts5_tokenize`, `bm25f_match_expression`, `bm25f_score_doc`, `bm25f_search_inner` |
| `search_api.rs` | `impl Engine`: `freeze_read_context`, `validate_frozen_read_context_for_binding`, `search_frozen`, `search_with_evidence`, `resolve_evidence`, `resolve_graph_evidence`, `search_expand_frozen`, `tc5_vector_stage`, `usable_dense_runtime`, the `search*` variants (`search` through `search_projected_text_with_limit`), `dense_disabled`, `dense_disabled_reason`, `vector_equivalence_refusal_count`, `search_reranked_with_explain`, `search_inner`, `search_inner_with_stats`, `search_inner_with_frozen_binding_and_stats`, `search_inner_with_frozen_binding_and_expansion`, `bm25f_search` |
| `telemetry.rs` | `TelemetrySink`, `append_jsonl`; `impl Engine`: `enable_telemetry`, `last_telemetry_query_id`, `capture_telemetry`, `finalize_search_observability`, `mint_explanation_correlation_id`, `capture_telemetry_with_sink`, `record_feedback` |
| `read.rs` | `NodeRecord`, `OpStoreRow`, `OperationalStateRecordV1`, `READ_COLLECTION_MAX_LIMIT`, `read_get_by_id_in_tx`, `read_collection_in_tx`, `read_list_in_tx`, `read_canonical_page_in_tx`, `read_canonical_page_baseline_in_tx`, `query_canonical_page_rows`, `canonical_page_query`, `OPERATIONAL_STATE_POINT_SQL`, `OPERATIONAL_STATE_PAGE_SQL`, `read_operational_state_in_tx`, `read_operational_state_page_in_tx`, `validate_operational_context`, `validate_operational_collection`, `page_search_error`; `impl Engine`: `read_get`, `read_get_many`, `read_collection`, `read_mutations`, `read_collection_dispatch`, `read_list`, `read_list_filter`, `read_canonical_page`, `read_operational_state`, `read_operational_state_page`, `receive_page_result` |
| `reader_pool.rs` | `READER_POOL_SIZE`, `READER_LOOKASIDE_SLOT_SIZE`, `READER_LOOKASIDE_SLOT_COUNT`, `READER_WORKER_CHANNEL_CAPACITY`, the reader pause aliases, `ReaderWorkerPool` (with its impl, `Debug`, and `Drop`), `SearchReaderWork`, every `*ReaderRequest` struct, `ReaderRequest` (whole, including the WAL and diagnostic variants), `FrozenQueryRuntime`, the reader response aliases, `SearchReaderError`, `PageReaderError` (and their `From` impls), `CacheStatusReply`, `reader_worker_loop`, `finish_reader_request`, `begin_attributed_reader_tx`, `apply_perf_experiment_reader_pragmas`, `configure_reader_lookaside`, `read_lookaside_used_hiwtr`, `read_cache_status` |
| `graph_expand/` | Replaces `graph_expand.rs`. `mod.rs` holds declarations and re-exports that keep every `crate::graph_expand::X` path, including `read_graph_expand_in_tx` and `GraphExpandReaderControlsForTest`. The submodules are listed in the next four rows. |
| `graph_expand/types.rs` | Current lines 24-29 (`SCHEMA_VERSION`) and 90-378. `SCHEMA_VERSION` becomes `pub(super)`. |
| `graph_expand/codec.rs` | Current lines 1922-3470, plus `is_false`. |
| `graph_expand/execution.rs` | Current lines 30-89, 379-1920 (validation included), and the in-file tests at 3472-3525. |
| `graph_expand/traversal.rs` | `TraversalDirection`, `SearchExpandResult`, `bfs_graph_arm_candidates`, `GRAPH_NEIGHBORS_HARD_CAP`, `build_bfs_sql`, `build_bfs_with_depth_sql`, `crossed_boundary_since_in_tx`, `graph_neighbors_in_tx`, `search_expand_in_tx`, `search_expand_on_snapshot`, `explain_graph_neighbors_in_tx`; `impl Engine`: `graph_neighbors`, `search_expand`, `search_expand_with_limit`, `crossed_boundary_since` |

### Stays at root

- **Slice 90:**
  - the open, runtime, WAL, and operator facade, including
    `mint_explanation_open_nonce` and its nonce `static`;
  - `detect_slow`, `emit_event`, and `emit_sqlite_internal_error`;
  - the WAL and diagnostic executors behind the `ReaderRequest` variants;
  - the write/search index projectors (`project_canonical_node_row`,
    `project_canonical_edge_row`, `IndexTargetSet`,
    `index_targets_for_row_kind`, `reproject_search_index_after_tokenizer_upgrade`,
    `search_index_tokenizer_reproject_complete`, `CanonicalNodeRow`,
    `canonical_node_rows`, `row_kind_from_column`).
- **Slice 140:**
  - every `*_for_test` `Engine` seam;
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

1. **`fusion.rs`** (about 305 lines).
2. **Filter types** into `filter.rs` (about 445).
3. **Filter functions** into `filter.rs` (about 545). Retarget the
   `plan-0.8.20.md` citations of `text_hit_passes_filter` and
   `edge_fts_hit_passes_filter`.
4. **`search_types.rs`** (about 450). Retarget the C1 `SearchHit` probe.
5. **`search.rs` helpers**: capture types, statement and rank-boundary
   helpers, witnesses, and BM25F (about 720).
6. **`search.rs` core**: `read_search_in_tx` alone (about 1,110).
7. **`search_api.rs` part A**: the frozen and evidence facades plus the
   `search_inner*` family (about 700).
8. **`search_api.rs` part B plus `telemetry.rs`**: the search variants, the
   dense-status methods, `search_reranked_with_explain`, and telemetry
   (about 800).
9. **`read.rs`** (about 960).
10. **`reader_pool.rs` part A**: the constants, types, pool impl, and
    `Drop` (about 700).
11. **`reader_pool.rs` part B**: the worker loop, `finish_reader_request`,
    reader connection setup, and `begin_attributed_reader_tx` (about 550).
12. **Graph split, first step:** `git mv graph_expand.rs
    graph_expand/execution.rs`, add `mod.rs`, and move the types into
    `types.rs` (about 330). Retarget `slice60_fix1_wire`.
13. **Request codec** into `graph_expand/codec.rs` (about 755).
14. **Result codec** into `graph_expand/codec.rs` (about 795).
15. **`graph_expand/traversal.rs`** (about 1,000).

Heavy public and hidden captures run in three places: at the pre-move
receipt, after batch 8 (by then most root re-exports exist), and at the
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
  `slice60_fix2_wire`, and `slice60_fix3_wire`
- `slice55_wire` (proptest)
- the in-file tests of `evidence.rs`, `frozen_read.rs`, and `pagination.rs`
- `slice50_evidence::authorized_*_corruption_is_typed*`
- `slice35_frozen_read`

**R27-80F — visibility and filters.** Consideration TC-38 is already covered:

- `slice15b_search_validity::{read_view_on_search_selects_by_instant_and_can_relax_validity, text_only_search_also_hides_out_of_window_nodes, filtered_and_explained_search_hide_out_of_window_nodes, search_refuses_a_view_that_relaxes_the_existence_axis}`
- `opp12_existence_axis::{r_ex_2_pending_absent_from_default_search_and_read, r_ex_2_vector_search_excludes_superseded_node_version}`
- `slice35_filter_grammar`, `slice40_filter_unification`, and
  `slice10_read_view`

**Plan and statement structure:**

- `slice60_graph_expand::endpoint_plans_use_shipped_indexes_without_full_edge_scan_and_schema_stays_34`
- `slice20_graph_traversal::explain_plan_uses_indexes`
- `slice45_pagination::page_query_plans_use_governed_indexes_without_mutation_log_or_temp_sort`
- `slice71_search_statement_trace`
- `slice20_graph_evidence::evidence_hydration_executes_zero_or_exactly_two_sql_statements`

### Added (the only gap): reader transaction release after refusal (R27-80B)

No owner proves that a search refused *inside* a reader transaction releases
that worker's snapshot. The reader-pool tests cover success paths, and the
WAL-attribution tests cover erasure refusals. A leaked snapshot would pin
the WAL and silently degrade later erasure.

- **Where:** `tests/slice80_reader_transaction_release.rs`, default
  features.
- **Test:**
  1. Seed a database.
  2. Issue refused searches at least three times `READER_POOL_SIZE` times,
     so every worker takes a refusal. The refusal must happen on the reader
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
  (or its connection's read snapshot) alive after returning the error. The
  test must then fail on the erasure assertion, and the failure must be
  recorded. The mutant is reverted, and a byte comparison confirms the
  production file is restored.
- **If the test fails on unmodified production:** stop for a RED/GREEN fix.

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
- **`scripts/tests/test_windows_wal_attribution_ci_job.sh`.**
  - Its probes read root tests and open-path items, which stay at root.
  - Its `lib.rs` production `Connection::open(` count stays 1, because
    `open_managed_connection` stays.
  - It runs every batch. Any break gets a path-only retarget like
    `36fc2352`.
- **`scripts/lint-plan-anchors.sh`.** The ACTIVE `plan-0.8.20.md` citations
  of the two post-filters are updated in batch 3. The `truncate_wal` and
  `wal_checkpoint_truncate_once` citations stay on `lib.rs`.
- **Hidden-surface rows.**
  - Moved doc-hidden items keep effective `cfg` and hidden values through
    `pub use`: `fuse_rrf`, `fuse_three_arms`, `apply_recency_reweight`,
    `CacheStatusReply`, and the `graph_expand` `*ForTest` re-exports.
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
  - the hidden capture, additive only (the new test).

## Risks

- **`ReaderRequest` breadth.** The WAL and diagnostic variants now live in
  `reader_pool.rs` while their executors stay at root. The worker loop calls
  those root executors through `super::`, and no field widens beyond
  `pub(crate)`. Slice 90 reviews this seam when it moves the executors.
- **`read_search_in_tx` size.** At about 1,110 lines in one function, the
  batch is a single contiguous cut. It must not be refactored.
- **The graph file split.** `git mv` preserves history for `execution.rs`.
  The codec batches must keep item order within `codec.rs`. The in-file
  `graph_evidence_request_tests` stay with `encode_graph_evidence_request`
  in `execution.rs`.
- **Doc-hidden re-exports.** `rerank.rs` showed that a redundant
  `#[doc(hidden)]` on the re-export is harmless. The hidden capture is the
  oracle.
- **Scale.** Fifteen batches touch the hottest read paths. The owner-route
  count equality and the gates in every batch are the safety net. A
  compile-driven logic change stops the batch.
