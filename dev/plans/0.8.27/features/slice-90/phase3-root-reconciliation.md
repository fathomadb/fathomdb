---
title: Slice 90 Phase 3 root reconciliation
target_release: 0.8.27
lib_rs_blob: fdf4bcbb9dc75501ba4d197bb019e3182b3eed27
---

# Slice 90 Phase 3 root reconciliation

This inventory is derived from the engine `lib.rs` blob named above after the
approved structural moves and the query-helper doc correction. The accepted
[design](design.md) and its reviewed test-seam owner amendment govern the
owner decisions.
The fast-tier `test-slice90-root-reconciliation` guard checks the root item
closure, the retained method gates, the test-only worker fixture, and selected
public re-exports. This is a source inventory, not a final qualification or
performance receipt.

Slice 103 added operator-only owed-erasure recovery re-exports in the root
composition, changing its blob from `b2f01b3f95bab83a263bd1f2a1df742e07a72a69`
to the value above. The 43 root storage fields and approved root method
inventory remain unchanged.

## Remaining root declarations

| Declaration | Disposition |
| --- | --- |
| `Engine` and its 43 fields | Physical engine storage and lifetime identity remain at root. Field bodies and state consumers are owned by the modules in the design map. |
| `impl Debug for Engine` | Facade rendering of path, close state, and embedder identity. |
| `Engine::path`, `Engine::ensure_open` | The two approved root facade controls. |
| `Engine::execute_for_test` | Approved Slice 140 arbitrary writer-SQL/lifecycle fixture; `any(test, debug_assertions, test-hooks)`, public and doc-hidden. |
| `Engine::run_one_thread_poison_for_test` | Approved Slice 140 composite write/search/lifecycle fixture; `debug_assertions`, public and doc-hidden. |
| `Engine::query_i64_col_for_test`, `Engine::query_text_col_for_test` | Approved ungated, public, doc-hidden arbitrary writer-SQL column fixtures. `DML ... RETURNING` may mutate; neither method enforces a statement kind. |
| `PROJECTION_WORKERS` | `#[cfg(test)]` root unit-test fixture with value 2, consumed only by the root `tests` module. It is not a runtime worker-count default or an unallocated production constant. |

The exact 43 storage fields are grouped here to prevent a generic
"remaining state" exemption:

| Storage role | Root field names |
| --- | --- |
| Identity and admission state | `path`, `requested_config`, `resolved_config`, `runtime_embedder_identity`, `explanation_open_nonce`, `explanation_sequence`, `dense_disabled`, `dense_disabled_reason`, `vector_equivalence_refusals` |
| Connections and lifecycle | `closed`, `close_lock`, `lock`, `connection`, `reader_pool`, `profile_contexts`, `reader_lookaside_rcs`, `managed_connections`, `writer_connection_registration`, `actual_checkpoint_observations`, `binding_native_state_observations`, `wal_attribution` |
| Projection and write state | `next_cursor`, `read_visibility_generation`, `projection_generation_status_cache`, `mutation_projection_status_cache`, `projection_generation_status_full_owner_scan_count`, `projection_runtime`, `provenance_row_cap` |
| Embed dispatch state | `runtime_embedder`, `embed_dispatch` |
| Events and telemetry | `counters`, `subscribers`, `profiling_enabled`, `slow_threshold_ms`, `telemetry`, `telemetry_enabled` |
| Test controls | `graph_expand_rss_baseline_bytes`, `graph_expand_rss_delta_bytes`, `graph_evidence_before_resolve_return_hook`, `erasure_before_primary_lock_hook`, `force_next_commit_failure`, `actuation_after_initial_lookup_delay_ms`, `actuation_failure_after_operation` |

Root also composes module declarations, imports, and public re-exports. The
remaining `#[cfg(test)]` modules are `gpu_allocation_witness_opt_in_tests`,
`slice20_fix1_tests`, `slice90_close_tests`,
`slice90_concurrent_close_tests`, `slice90_close_review_tests`,
`slice90_post_probe_real_error_tests`, and `tests`. They are test containers,
not production owner methods. The public `D27Observation`,
`MEAN_VEC_PIN_THRESHOLD`, `RowKind`, mean test internals, and the other moved
public types/functions retain their root re-export paths. The three D27
observation methods now have one ordinary implementation in `embed_dispatch`;
none remains in root.

The source scan finds no other root production struct, enum, constant, static,
free function, trait, or implementation body. There are zero unresolved
root-owned production items against the accepted map. Feature-qualified
public/hidden surface comparison and the full release gate remain separate
candidate checks.
