---
title: Slice 90 prospective root test-seam disposition amendment
status: PROPOSED
source_sha: e05de8be99d4b66a29cafe02e6b0682ebd1993ec
---

# Slice 90 root test-seam disposition — prospective amendment

This draft supplements `dev/plans/0.8.27/features/slice-90/design.md` §§78–160. It is an item-specific proposal, not authority to move code until reviewed. Line numbers refer to `fathomdb-engine/src/lib.rs` at the fixed source SHA above. `current-source-inventory.md` is the entry census, not a final disposition map. A later HEAD requires a fresh symbol comparison before execution.

The later implementation commits through `e0f062c7` move only production
constants outside this table; no listed test-seam declaration changed.

## Path and gate invariants

- All listed `Engine` methods are `pub`; moving an implementation to an `impl Engine` in a private owner preserves the `Engine::method` path. Keep signatures, attributes, behavior, and `#[doc(hidden)]` exactly as they are. An ungated test-named method remains ungated; adding a gate changes the Rust surface.
- Gate codes: **U** ungated; **D** `#[cfg(debug_assertions)]`; **T** `#[cfg(feature = "test-hooks")]`; **DT** `#[cfg(any(debug_assertions, feature = "test-hooks"))]`; **X** `#[cfg(any(test, debug_assertions, feature = "test-hooks"))]`.
- Documentation codes: **H** has `#[doc(hidden)]`; **V** does not. Preserve `#[must_use]` and `#[allow(dead_code)]` where present. Existing free public functions need root `pub use` to keep `fathomdb_engine::function`; private helpers need no public re-export. The two public timing structs need root re-exports after relocation.
- Physical `Engine` storage stays in root. The four search control atomics stay on `ProjectionRuntimeShared` in `projection_runtime`; moving setters changes method-body ownership only.

## Proposed per-item disposition

| Current `lib.rs` symbol and line | Gate/docs | Final implementation owner and source-grounded reason | Root path disposition |
| --- | --- | --- | --- |
| `Engine::explain_graph_evidence_preflights_for_test` 787 | T/H | `evidence`; calls its intrinsic preflight oracle. | Inherent method preserved. |
| `Engine::begin_d27_observation_for_test` 796; `Engine::with_d27_foreground_owner_for_test` 802; `Engine::d27_observation_for_test` 817 | T/V each | `embed_dispatch`; collector, foreground owner and report all belong to dispatch. | Inherent methods preserved; do not add `doc(hidden)`. |
| `Engine::pause_projection_worker_after_wal_transaction_for_test` 832 | D/H; also `allow(dead_code)` | `projection_runtime`; delegates to its pause control. | Inherent method and all attributes preserved. |
| `Engine::pause_projection_worker_while_queued_for_test` 841; `Engine::pause_projection_worker_before_write_lock_for_test` 848 | T/H each | `projection_runtime`; existing runtime owns the worker rendezvous. | Inherent methods preserved. |
| `Engine::_graph_frontier_stats_for_test` 861 | U/V | `search_api`; it invokes `search_inner_with_stats`, owned by `search_api.rs:408`, to expose one graph-arm result. `graph_expand` owns the meter type, not this search entry. | Preserve the leading underscore and ungated inherent method. |
| `Slice45FrozenStageTiming` 725; `Engine::measure_slice45_frozen_stages_for_test` 882 | T/H each | `frozen_read`; the page cursor is authenticated through `pagination`, then frozen-token and snapshot authority are checked there. The seam times the frozen setup while calling, not reimplementing, the pagination oracle. | Root `pub use` for the struct; inherent method preserved. |
| `Slice45MintStageTiming` 735; `Engine::measure_slice45_mint_stages_for_test` 909 | T/H each | `frozen_read`; invokes `frozen_read::mint_measured` and publishes its generation. | Root `pub use` for the struct; inherent method preserved. |
| `Engine::slice45_page_query_plans_for_test` 929 | T/H | `read_api`; this Engine-facing read diagnostic prepares the `read::canonical_page_query` shape and two operational read shapes. | Inherent method preserved. |
| `Engine::reader_worker_count_for_test` 981; `Engine::live_reader_worker_count_for_test` 987; `Engine::next_reader_worker_index_for_test` 995 | D/H each | `reader_pool`; all delegate to pool-owned worker state. | Inherent methods preserved. |
| `Engine::reader_lookaside_config_rcs_for_test` 1005 | D/H | `connection_runtime`; observes the per-reader connection setup result captured in the root `reader_lookaside_rcs` field. Storage stays root; setup authority is connection runtime. | Inherent method preserved. |
| `Engine::reader_lookaside_used_per_worker_for_test` 1016; `Engine::cache_status_per_worker_for_test` 1027 | D/H each | `reader_pool`; dispatches worker-owned connection observations on their existing threads. | Inherent methods preserved. |
| `Engine::force_next_commit_failure_for_test` 1033 | D/H | `write_commit`; arms the next writer commit via the root test field. | Inherent method preserved; field stays root. |
| `Engine::set_actuation_after_initial_lookup_delay_ms_for_test` 1039; `Engine::force_actuation_failure_after_operation_for_test` 1045 | D/H each | `actuation`; arms actuation-specific fault controls. | Inherent methods preserved; fields stay root. |
| `Engine::force_next_projection_commit_failure_for_test` 1054; `Engine::force_next_projection_storage_failure_for_test` 1062; `Engine::pause_projection_commit_failure_cleanup_for_test` 1070 | D/H each | `projection_runtime`; existing methods delegate to its worker fault/rendezvous controls. `projection_commit` still owns the transaction implementation. | Inherent methods preserved. |
| `Engine::acknowledge_projection_stop_for_test` 1082 | D/H | `projection_runtime`; acknowledges the stopping worker boundary. | Inherent method preserved. |
| `Engine::execute_for_test` 1097 | X/H | **Retain root**. Accepted design §122: arbitrary writer SQL plus lifecycle event dispatch is a cross-domain Slice 140 seam. | Keep exact root method and gate. |
| `Engine::run_one_thread_poison_for_test` 1121 | D/H | **Retain root**. Accepted design §123: composes write, search, projection status and lifecycle failure. | Keep exact root method and gate. |
| `Engine::set_projection_scheduler_frozen_for_test` 1201 | U/H | `projection_runtime`; delegates to runtime scheduler control. | Inherent method preserved, ungated. |
| `Engine::transition_projection_generation_for_test` 1211; `Engine::projection_generation_status_full_owner_scan_count_for_test` 1233; `Engine::projection_generation_status_query_plans_for_test` 1240 | T/H each | `projection_generation`; all measure or transition that generation authority; writer connection remains Engine storage. | Inherent methods preserved. |
| `Engine::publish_projection_success_for_test` 1252 | T/H | `projection_commit`; the decisive action is `commit_projection_outcomes`; captured generation is an input, not a second owner. | Inherent method preserved. |
| `Engine::projection_scheduler_pending_scan_for_test` 1284; `Engine::set_projection_retry_delays_for_test` 1289; `Engine::set_embed_timeout_ms_for_test` 1296 | U/H each | `projection_runtime`; existing implementations delegate to that runtime's scheduler and test policy controls. | Inherent methods preserved, ungated. |
| `Engine::projection_status_for_test` 1301; `Engine::projection_failure_count_for_test` 1322 | U/H each | `projection_runtime`; these report durable runtime terminal/failure state. | Inherent methods preserved, ungated. |
| `Engine::has_vector_for_cursor_for_test` 1312 | U/H | `projection_runtime`; reads `terminal_state_for_cursor` as a runtime readiness observation, not a physical vec0 row. `projection_commit` retains the terminal storage helper. | Inherent method preserved, ungated. |
| `Engine::set_provenance_row_cap_for_test` 1338; `Engine::provenance_row_count_for_test` 1343; `Engine::oldest_provenance_record_key_for_test` 1353 | U/H each | `provenance`; operational mutation retention controls and observations. | Inherent methods preserved; cap field stays root. |
| `Engine::configure_vector_kind_for_test` 1377 | U/H | `projection_registry`; directly writes `_fathomdb_vector_kinds`, whose production `register_vector_kind` is in `projection_registry.rs:1928`. | Inherent method preserved. |
| `Engine::set_legacy_projection_vector_declared_for_test` 1398 | D/H | `projection_registry`; mutates a registry declaration and transitions its generation in one transaction. | Inherent method preserved. |
| `Engine::set_legacy_projection_search_subobjects_for_test` 1429 | DT/H | `projection_registry`; same declaration-plus-generation authority. | Inherent method and exact `any(...)` gate preserved. |
| `Engine::secure_delete_enabled_for_test` 1460; `Engine::runtime_secure_delete_enabled_for_test` 1491 | U/H each | `connection_runtime`; inspect writer/runtime connection setup policy. | Inherent methods preserved, ungated. |
| `Engine::reader_secure_delete_enabled_for_test` 1477 | D/H | `reader_pool`; broadcasts the worker-owned secure-delete probe. | Inherent method preserved. |
| `Engine::write_canonical_row_with_kind_for_test` 1518 | U/H | `write`; it performs a canonical write and uses the same index projection and cursor publication path as production writes. | Inherent method preserved, ungated. |
| `Engine::canonical_rows_with_row_kind_for_test` 1600 | U/H | `read_api`; reads ordered canonical cursors by `RowKind`. | Inherent method preserved, ungated. |
| `Engine::write_vector_for_test` 1623 | U/H | `vector_storage`; the decisive output is a vec0 row and associated vector/profile/mean state. It can remain a test-only inherent method there while calling existing `mean` and projection helpers. Design §§178–179 and code-change plan §Outcome still exclude its direct provider call from dispatcher/deadline/performance proof and defer test-gate cleanup to Slice 140. | Preserve ungated inherent method; record the direct-provider proof exception separately from physical root ownership. |
| `Engine::drain_mean_centering_events_for_test` 1769 | U/H | `embedding`; pairs with already moved `Engine::drain_embedder_events` and reads the same pending event queue. | Inherent method preserved. |
| `Engine::set_search_limit_for_test` 1787; `Engine::set_recency_reweight_enabled_for_test` 1795; `Engine::set_importance_reweight_enabled_for_test` 1805; `Engine::set_vector_stage_only_for_test` 1880 | U/H each | `search_api`; that owner consumes all four controls (`search_api.rs:128–137`, `579–587`, `1039`). Physical atomics and defaults remain on `ProjectionRuntimeShared` under design §§135–149. | Inherent methods preserved; no duplicate state or gate changes. |
| `Engine::force_next_recompute_failure_for_test` 1889 | D/H | `mean`; arms mean recomputation's one-shot failure. | Inherent method preserved. |
| `Engine::vector_row_count_for_test` 1894; `Engine::has_vector_row_for_cursor_for_test` 1907; `Engine::read_vector_blob_for_test` 1921; `Engine::read_vector_bin_for_test` 1937 | U/H each | `vector_storage`; inspect physical vec0 rows/blobs, distinct from terminal state. | Inherent methods preserved. |
| `Engine::query_i64_col_for_test` 1956; `Engine::query_text_col_for_test` 1969 | U/H each | **Retain root** under the itemized writer-SQL fixture ruling in `design.md`. They prepare arbitrary SQL on the Engine writer connection with vec0 loaded and collect column zero. There is no read-only check; DML with `RETURNING` may mutate, so the current Rust docstring's read-only claim is stale and must be corrected during final inventory reconciliation. The Slice 15e filterable-vector and later interaction tests use these seams. | Preserve both ungated public doc-hidden Engine paths and exact behavior. |
| `Engine::default_embedder_profile_for_test` 1980 | U/H | `open`; reads the profile admitted and installed at open. | Inherent method preserved. |
| `Engine::dependency_trace_query_plans_for_test` 1989; `Engine::measure_dependency_trace_for_test` 2060 | T/V each | `dependency_trace`; tests the production trace SQL and measured execution. | Inherent methods preserved; do not add `doc(hidden)`. |
| `Engine::dependency_trace_candidate_queries_for_test` 2054; `Engine::seed_hidden_dependency_trace_fixture_for_test` 2114 | T/H each | `dependency_trace`; candidate SQL and governed fixture belong with that query family. | Inherent methods preserved. |
| `Engine::schema_objects_for_test` 2175 | T/V | `operator`; observes the same schema inventory as `dump_schema`. | Inherent method preserved; do not add `doc(hidden)`. |
| `vector_phase1_sql_for_test` 2250; `slice35_ranked_eligibility_sql_for_test` 2258 | U/H; `must_use` each | `search_api`; these are public test-facing witnesses for SQL used by search. Keep the SQL constructors and eligibility/ranking grammar in `filter.rs:565–718`; call them from this facade. | Root `pub use search_api::{...}`; preserve ungated names and `must_use`. |
| `take_slice71_search_statement_trace_for_test` 2283 | T/H | `search_api`; public test-facing search witness that drains the existing state owned by `search.rs:513`. | Root `pub use search_api::take_slice71_search_statement_trace_for_test`. |
| `record_writer_pragma_witness_for_test` 2291 | T/V, private | `connection_runtime`; witnesses writer pragma setup. Open calls it after connection construction. | Import privately at call site; no root public path. |

The `mean_centering_internals_for_test` module and its `AccumulatorHandle`, `new_mean_accumulator`, `accumulator_add`, `accumulator_materialize`, `accumulator_count`, and `run_requantize_pass` (2215–2241) belong to `mean`: each body wraps `MeanAccumulator` or `run_requantize_pass`, and `eu5a2_machinery` is its caller. Move the module implementation to `mean` and keep the exact ungated public doc-hidden `fathomdb_engine::mean_centering_internals_for_test` path through a root re-export. Preserve all signatures and `must_use` attributes. This is a named owner move, not a root implementation exception.

## Review decisions and execution order

The proposed choices above resolve the source-grounded overlaps without duplicating owner state. `write_vector_for_test` moves to `vector_storage` but remains the accepted direct-provider exception to runtime proof. The two arbitrary-SQL column fixtures remain as newly itemized root exceptions beside `execute_for_test` and `run_one_thread_poison_for_test`. The mean test module implementation moves to `mean` with its public root path re-exported. Independent review must approve this complete item map before implementation; a reviewer may reject any proposed owner, in which case that item needs a prospective ruling rather than an ad hoc move.

After review: (1) add these rows to the accepted design and final item inventory; (2) move one semantic owner group at a time, with public re-exports and identical cfg/docs; (3) verify qualified test identities, source scrapers, feature variants and module boundary checks per batch; (4) reconcile root to storage/composition/approved controls and explicit test exceptions. Do not infer a full-green claim from this draft.
