---
title: Slice 90 Phase 3 root reconciliation
target_release: 0.8.27
lib_rs_blob: 65a34b56a1fa99e297bb6dc4a4ce377bcd411ed5
---

# Slice 90 Phase 3 root reconciliation

This inventory is derived from the engine `lib.rs` blob named above after the
approved structural moves and the query-helper doc correction. Slice 140
refreshes this inventory after gating the four retained test fixtures behind
`any(test, feature = "test-hooks")`; the prior Slice 90 blob was
`9afaec4a6a42eed8415f531b3e22047a9bf7e242`. The accepted
[design](design.md) and its reviewed test-seam owner amendment govern the
owner decisions.
The fast-tier `test-slice90-root-reconciliation` guard checks the root item
closure, the retained method gates, the test-only worker fixture, and selected
public re-exports. This is a source inventory, not a final qualification or
performance receipt.

Slice 103 added operator-only owed-erasure recovery re-exports in the root
composition, changing its blob from `b2f01b3f95bab83a263bd1f2a1df742e07a72a69`
to `e395a9b0e75928bcdea6528c71bba65c091050de`. Later Windows-only tests
and a Slice 114 stale-comment removal changed the whole-file blob to
`4417d3ad27122c38a1668ca746968af344418905`. The embedder-close fix then made
`runtime_embedder` a weak reference, changing the blob to
`9afaec4a6a42eed8415f531b3e22047a9bf7e242`. Slice 140 then gated the
retained fixtures, producing `0226b8836c929a30793b8913461f5abf341123c2`.
0.8.28 Slice 30 added the public `RerankPassagesError` re-export beside
`rerank_passages`, producing `0875d2b8486941649d87e3895a0f29d5d00f3993`.
0.8.28 Slice 31 added the public GPU pool-setting re-exports from
`fathomdb_embedder` (`CudaPoolSettingConflict`, `CudaPoolSettingValue`,
`CudaPoolMode`, `PoolSettingSource`, `ReleaseThreshold`,
`CUDA_POOL_MAX_SIZE_MIN`), producing `ea1062616be0af07b35ca4089e5c606eeb368470`.
0.8.28 Slice 40 H2 added the `test-hooks` re-export
`vector_fusion_pool_for_test`, producing `35b302ee926d5bb3e1bcda6895e130f73a022162`.
0.8.28 Slice 41 added the `context_pack` module and its re-exports, the
`fathomdb_query` stop-word re-exports (`QueryStopWords`, `StopWordBase`,
`StopWordRejection`, `DEFAULT_STOP_WORDS_SHA256`) and the root storage field
`query_stop_words`, producing `d7065f6f041f547a1d124182983f17268a379145`.
0.8.28 Slice 42 changed only the root `tests` module (the Windows
verbatim-path guard tests moved to the version-only predicate, and its
portable test joined them), producing `12e4a6448f727d4afd940a9386d4c4e7d1e05f09`.
0.8.28 Slice 50 implements owner `seq-359`'s narrow provider-admission policy:
actual verification refusal fails open without returning an attached engine,
removing the `dense_disabled`, `dense_disabled_reason`, and
`vector_equivalence_refusals` storage fields. It also adds the
`provider_admission_tests` unit-test module and the corresponding root
composition changes, producing the blob above. The root storage fields are
then 41; the approved root method inventory was unchanged.

0.8.28 Slice 50 scan qualification adds the `test-hooks` re-exports
`ProjectionScanPageForTest` and `ProjectionScanForTest`, producing
`c182b0b20f9c96582aa5fe84773beb5a270f294a`. Production root storage is unchanged.

0.8.28 Slice 85 adds persisted physical policy, published explicit-debt state, and
maintenance admission ownership. These three named fields bring root storage to
46 fields; their semantic consumers remain outside root. The current-format
lock-admission test expects current format 37. The blob above includes these changes.

## Remaining root declarations

| Declaration | Disposition |
| --- | --- |
| `Engine` and its 46 fields | Physical engine storage and lifetime identity remain at root. Field bodies and state consumers are owned by the modules in the design map. |
| `impl Debug for Engine` | Facade rendering of path, close state, and embedder identity. |
| `Engine::path`, `Engine::ensure_open` | The two approved root facade controls. |
| `Engine::execute_for_test` | Approved Slice 140 arbitrary writer-SQL/lifecycle fixture; `any(test, test-hooks)`, public and doc-hidden. |
| `Engine::maintenance_transaction_for_test<T>` | Approved 0.8.28 Slice 81 cross-domain maintenance fixture; public, doc-hidden and `any(test, feature = "test-hooks")`. Holds the actual Engine writer through an IMMEDIATE transaction and commit/RAII rollback; returns successful-call mutex wait and occupancy. Callback must not control transactions or reenter Engine. |
| `Engine::run_one_thread_poison_for_test` | Approved Slice 140 composite write/search/lifecycle fixture; `any(test, test-hooks)`, public and doc-hidden. |
| `Engine::query_i64_col_for_test`, `Engine::query_text_col_for_test` | Approved public, doc-hidden arbitrary writer-SQL column fixtures under `any(test, test-hooks)`. `DML ... RETURNING` may mutate; neither method enforces a statement kind. |
| `PROJECTION_WORKERS` | `#[cfg(test)]` root unit-test fixture with value 2, consumed only by the root `tests` module. It is not a runtime worker-count default or an unallocated production constant. |

The exact 46 storage fields are grouped here to prevent a generic
"remaining state" exemption:

| Storage role | Root field names |
| --- | --- |
| Identity and admission state | `path`, `requested_config`, `query_stop_words`, `resolved_config`, `runtime_embedder_identity`, `explanation_open_nonce`, `explanation_sequence` |
| Connections and lifecycle | `closed`, `close_lock`, `lock`, `connection`, `reader_pool`, `profile_contexts`, `reader_lookaside_rcs`, `managed_connections`, `writer_connection_registration`, `actual_checkpoint_observations`, `binding_native_state_observations`, `wal_attribution` |
| Physical cleanup | `physical_maintenance`, `physical_service_gate`, `physical_policy`, `explicit_debt`, `maintenance_admission` |
| Projection and write state | `next_cursor`, `read_visibility_generation`, `projection_generation_status_cache`, `mutation_projection_status_cache`, `projection_generation_status_full_owner_scan_count`, `projection_runtime`, `provenance_row_cap` |
| Embed dispatch state | `runtime_embedder`, `embed_dispatch` |
| Events and telemetry | `counters`, `subscribers`, `profiling_enabled`, `slow_threshold_ms`, `telemetry`, `telemetry_enabled` |
| Test controls | `graph_expand_rss_baseline_bytes`, `graph_expand_rss_delta_bytes`, `graph_evidence_before_resolve_return_hook`, `erasure_before_primary_lock_hook`, `force_next_commit_failure`, `actuation_after_initial_lookup_delay_ms`, `actuation_failure_after_operation` |

Root also composes module declarations, imports, and public re-exports. The
remaining `#[cfg(test)]` modules are `gpu_allocation_witness_opt_in_tests`,
`provider_admission_tests`,
`slice20_fix1_tests`, `slice90_close_tests`,
`slice90_concurrent_close_tests`, `slice90_close_review_tests`,
`slice90_post_probe_real_error_tests`, `visibility_admission_tests`,
`visibility_coalescing_tests`, and `tests`. They are test containers,
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

0.8.28 Slice 60 adds the private `vector_spans` module declaration and updates
the current schema-version fixture to 35; root field and method ownership
remain unchanged. The inventory blob above tracks those changes.

0.8.28 Slice 80 adds the feature-gated private property corpus rendezvous
re-exports (`arm_property_corpus_hook_for_test`, `PropertyCorpusPauseForTest`).
Root field and method ownership remain unchanged; the inventory blob above
tracks this composition-only update.

0.8.28 Slice 81 adds the explicitly inventoried generic
`maintenance_transaction_for_test<T>` fixture, changing the whole-file blob from
`53f4187db2a9d2025fffdc78310d0edc2799dcf9` to the value above. The seam is owned
at root because it supplies an atomic actual-writer capability for cross-domain
experimental maintenance, not a production domain operation. Production root
storage, methods and runtime behavior remain unchanged; no pending-closure fence
is bypassed. Its reviewed contract is in the [Rust interface](../../../../interfaces/rust.md)
and [Slice 81 design](../../../../plans/0.8.28/features/slice-81/design.md).

0.8.28 Slice 82 adds the private `physical_erasure` module and the feature-gated
`arm_physical_erasure_hook_for_test` re-export. Root fields and method ownership
remain unchanged. Physical debt and sanitation live in the new internal module;
the rendezvous implementation is owned by `test_hooks`. The current blob above
tracks this composition-only update.

0.8.28 Slice 83 adds the owned physical-maintenance worker and singleton service
guard, represented by the two physical-cleanup fields above. It retains the root
facade methods and exact storage inventory; the current blob includes those
changes and the new private cleanup modules.

0.8.28 Slice 95 replaces the private native trigger guard import with the
transaction-owned visibility guard, adds the two private visibility test
containers above, and advances the current-format composition fixture to 37.
The whole-file blob above tracks that composition; the 46 storage fields,
root item closure and production ownership assertions remain unchanged.
