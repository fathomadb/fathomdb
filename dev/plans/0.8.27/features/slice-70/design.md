---
title: FathomDB 0.8.27 Slice 70 - projection, embedding, vector, and reranking design
status: PROPOSED
target_release: 0.8.27
baseline_sha: a95b5b0f
---

# Slice 70 design

## Boundary

This is a behavior-preserving move of root-owned items in
`src/rust/crates/fathomdb-engine/src/lib.rs` (25,899 lines at `86c66379`)
into private sibling modules. Design review cycle 1 (`design-review.md`)
returned FAIL on text-only findings; this revision closes them. The
following do not change:

- SQL text;
- statement order;
- transaction scope;
- lock acquisition and `commit_gate` ordering;
- startup and close order;
- error mapping;
- feature and `cfg` gates;
- public paths;
- schema (34), wire, bindings, and packages.

The only permitted changes are:

- module declarations;
- `use` lines;
- the minimum `pub(crate)` or `pub(super)` visibility needed across sibling
  modules;
- root `pub use` re-exports for moved public items;
- link-only repairs of moved intra-doc links; and
- the source-scraper and gate retargets listed under "Source scrapers and
  gates".

Moved code is copied verbatim.

### Authority

In order of precedence:

1. accepted ADRs and locked `dev/design/*` (including
   `ADR-0.6.0-embedder-protocol.md`, `dev/design/embedder.md`, and
   `dev/design/bindings.md`);
2. `dev/plans/0.8.25/features/slice-40/design.md` for projection-generation
   and member-completion semantics;
3. the tests;
4. `dev/design/embedder-decision.md` and
   `dev/design/0.8.1-slice-10-reranker-design.md`, which are informative
   history only (`UNREVIEWED`).

Where the accepted configurable-pool and timeout contract disagrees with the
code, the code's current behavior is preserved. The gap is ledger
`TC-b602d87a…` (seq 258), owned by Slice 90.

## Destination modules

Every module is private (`mod x;` in `lib.rs`), and every item keeps its
existing `cfg`. A private helper used only by moved items moves with them.
A helper shared with items that stay at root stays at root, or moves with its
dominant domain, and gets `pub(crate)`. The seam table in `tdd-chronology.md`
records every visibility change.

| Module | Items (by name, at `86c66379`) |
| --- | --- |
| `vector_storage.rs` | `load_default_profile`, `default_profile_dimension`, `kind_is_vector_indexed`, `ensure_vector_partition`, `vector_partition_create_sql`, `create_vector_partition`, `attr_vec0_column`, `decode_attr_vec0_column`, `desired_vector_attr_columns`, `actual_vector_attr_columns`, `delete_vector_partition_row`, `reconcile_vector_attr_columns`, `refresh_vector_attr_values`, `refresh_vector_attr_values_for_row`, `reshape_vector_partition_nondestructive`, `migrate_vector_partition_pack1_to_pack2`, `KIND_TO_SOURCE_TYPE_CASE_SQL`, `migrate_vector_partition_to_pack1`, `encode_vector_blob`, `decode_vector_blob`, `quantize_binary_via_sql`, `hamming_bytes`, `VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES`, `resolve_source_type`, `kind_is_vector_committable` |
| `vector_equivalence.rs` | `vector_equivalence_probes`, `VectorEquivalenceOutcome`, `usable_dense_runtime`, `probe_embed`, `run_vector_equivalence_probe`, `probe_populate_or_check`, `collect_probe_baseline`, `persist_probe_baseline`, `StoredProbeRow`, `hash_fingerprint_field`, `probe_verification_fingerprint`, `probe_verification_is_cached`, `record_probe_verification`, `clear_probe_verification`, `probe_check_against_baseline`, `load_stored_probe_baseline`, `probe_check_stored_baseline`, `l2_distance` |
| `mean.rs` | `MeanAccumulator`, `cosine_similarity`, `run_pin_and_requantize_pass`, `run_requantize_pass`, `recover_mean_vec_pin`, `recompute_mean_in_tx`, `recompute_mean_in_tx_inner`, `identity_requires_mean_centering`, `read_pinned_mean_vec`, `subtract_mean`, and `Engine::recompute_mean` |
| `embedding.rs` | `embed_with_watchdog`, `embed_batch_with_watchdog`, `map_runtime_embedder_error`, `Engine::embed_text` |
| `projection_registry.rs` | `ProjectionClass`, `RowOwnedProjection`, `ROW_OWNED_PROJECTIONS`, `erase_row_projections`, `delete_row_owned_projection`, `saturating_add_u64`, `purge_row_projections_for_cursor_in`, `truncate_row_projections_in`, `truncate_all_row_projections`, `ProjectionPass`, `StoredProjection`, the attribute and nested-source helpers (`is_valid_attribute_name` through `validate_nested_projection_sources_for_body_in_registry`), `load_projection_registry`, `load_projection_registry_row`, `parse_roles_json`, `roles_to_storage`, `persist_projection_row`, `remove_projection_row`, `clear_attribute_projection`, `project_one_attribute`, `ATTR_VEC0_PRESENT_MARKER`, `encode_attr_vec0_present`, `extract_scalar_attribute`, `vector_attr_insert_fragments`, `project_node_attributes`, `backfill_attribute`, `is_destructive_projection_change`, `describe_projection_delta`, `apply_projection_config`, the vector enrolment and backfill functions (`unenrol_registry_vector_node_kinds` through `reenqueue_stranded_vector_rows`), `rederive_projections_on_boot`, the registry cache snapshot types and functions, `Engine::configure_projections`, `Engine::read_projections` |
| `projection_runtime.rs` | `ProjectionJob`, `ProjectionRuntimeState`, `ProjectionRuntimeShared` (shape unchanged), its `Debug` impl, `ProjectionRuntime`, the startup types and `missing_projection_runtime_roles`, `impl ProjectionRuntime` |
| `projection_worker.rs` | `report_projection_runtime_startup_failure`, `projection_runtime_injected_setup_failure`, `complete_projection_runtime_startup`, `projection_dispatcher_loop`, `projection_worker_loop`, `ProjectionOutcome`, `run_projection_jobs`, `projection_batch_enabled`, `embed_projection_batch`, `commit_projection_panic_failures`, `report_projection_commit_failure`, `run_projection_job`, the pending-work scans (`pending_edge_projection_from_where` through `pending_embedding_work`) |
| `projection_commit.rs` | `load_projection_cursor`, `store_projection_cursor`, `record_projection_terminal`, `terminal_state_for_cursor`, `projection_physical_tuple_is_empty`, `projection_tuple_corruption`, `advance_projection_cursor`, `commit_projection_outcomes` |
| `projection_rebuild.rs` | `Engine::rebuild_projections`, `Engine::rebuild_vec0`, `Engine::run_rebuild`, `Engine::rebuild_shadow_state` (every `operator` gate kept) |
| `rerank.rs` | `rerank_fused`, `try_rerank_fused`, `rerank_passages` (public; root `pub use` re-exports), `ce_rerank`, `CandleCrossEncoder`, `reranker_singleton` |
| `projection_generation.rs` (extended) | `derive_dense_readiness`, `Engine::read_projection_status`, `Engine::read_embedding_readiness` |

`ProjectionRuntimeShared` carries four search-owned fields:
`search_limit_override`, `recency_reweight_enabled`,
`importance_reweight_enabled`, and `vector_stage_only_for_test`. They move
with the struct unchanged, and search keeps reading them. Slice 80 decides
their final owner. Slice 70 does not change the struct's shape.

### Stays at root (explicit non-moves)

- **Slice 80:**
  - `TOP_K_BIT_CANDIDATES`, `SEARCH_RERANK_LIMIT`,
    `DEFAULT_SEARCH_RESULT_LIMIT`, `MAX_SEARCH_RESULT_LIMIT`,
    `validate_search_result_limit`;
  - `RRF_*`, `RECENCY_WEIGHT`, `fuse_rrf`, `fuse_three_arms`,
    `apply_recency_reweight`, `apply_importance_reweight`,
    `build_importance_confidence_maps`;
  - `vector_filter_*`, `read_search_in_tx`, every `search_*` method;
  - `bm25f_search`, `write_node_importance`, `node_importance`;
  - the telemetry helpers `branch_str` and `append_jsonl`, used only by
    search;
  - `reproject_search_index_after_tokenizer_upgrade` and its
    `CanonicalNodeRow` helpers;
  - `project_canonical_node_row` / `project_canonical_edge_row` and
    `index_targets_for_row_kind`, which are write and search projectors.
- **Slice 90:**
  - the open path (`open_with_migrations`, `check_embedder_profile`, and the
    embedder and reranker open gates);
  - `open_managed_connection` / `open_runtime_connection`;
  - the integrity sections;
  - `verify_embedder`, `check_integrity`, `safe_export`, the `dump_*`
    methods, and the WAL operations;
  - `default_embedder_identity`, `edge_vector_prune_complete`, and
    `prune_orphaned_edge_vectors`, called only from the open path;
  - `Engine::drain_embedder_events`;
  - `PROJECTION_WORKERS` and the other runtime constants at `lib.rs:463-488`
    (unchanged values);
  - `Engine::close`.
- **Slice 140:**
  - every `*_for_test` Engine seam, including the projection, vector, and
    embed seams at `lib.rs:9401-10483`;
  - `projection_status`, whose only caller is `projection_status_for_test`;
  - `pub mod mean_centering_internals_for_test` and its public constant
    `MEAN_VEC_PIN_THRESHOLD`. `mean.rs` reaches the constant through
    `super::`. In the other direction, the root module
    `mean_centering_internals_for_test` uses `MeanAccumulator` (with its
    `new`, `add`, `materialize`, and `count` methods) and
    `run_requantize_pass`. After batch 3, the root imports them from `mean`,
    and those items and methods become `pub(crate)`.
- **Why these stay:** Slice 140 gates the always-compiled doc-hidden test
  seams, and root placement keeps that gating one edit.
- **Unclassified items:** any item not listed above stays at root.

## Characterization design

### Existing owners mapped to requirements (reused, not duplicated)

| Requirement | Owners |
| --- | --- |
| R27-70B generation | `slice40_projection_generation::{non_noop_configuration_mints_but_exact_replay_reuses_generation, each_operator_rebuild_mints_a_distinct_generation, fresh_generation_is_stable_across_restart}`; `slice40_projection_generation_races::{stale_worker_result_cannot_publish_into_a_new_generation, worker_computing_across_generation_transition_discards_stale_result, queued_worker_across_generation_transition_is_rediscovered, worker_at_write_lock_boundary_discards_after_transition, publication_holding_write_lock_linearizes_before_transition}` |
| R27-70C recovery | `projection_runtime::{ac_063a_*, ac_063b_restart_does_not_retry_terminal_projection_failures}`; `tc91_projection_commit_hardening::*` (six tests); `slice21_projection_runtime_state::approved_open_boot_grafts_once_and_never_reopens_failed_terminals`; `slice40_projection_completion::missing_member_below_the_watermark_is_rediscovered_before_drain_returns`; `rebuild_projections::ac_063c_*` |
| R27-70D completeness | `slice40_projection_completion::{worker_publication_never_repairs_a_partial_projection_tuple, sidecar_row_identity_must_match_the_projection_owner, usable_runtime_cannot_leave_a_stranded_node_marker, complete_edge_without_required_enrolment_is_corrupt, global_status_rejects_a_cursor_owned_by_both_node_and_edge}`; `slice40_projection_generation::a_terminal_without_its_physical_vector_is_typed_corruption` |
| R27-70E embedding | `pr9_embed_watchdog`, `pr9_embed_serialization`, `pr9_concurrent_embed`, `slice30_embedding_readiness`, `slice70_runtime_policy` (`default-embedder`) |
| R27-70F vector and mean | `tc91_projection_commit_failure_at_mean_pin_is_rollback_safe`, `pr2b_mean_recompute`, `eu5a1`, `eu5a2`, `eu5b`, `eu5f_production_pin`, `vector_contracts`, `vector_quant_pack1`, `vector_equivalence_probe`, `tc68_probe_fingerprint_cache`, `tc76_vec0_long_metadata_delete`, `tc33_fix6_edge_vector_prune`, `rebuild_vec0` (`operator`) |
| R27-70G rerank | `pr_g10_reranker`, `pr_e2_rerank_nonfinite`; with `default-reranker`: `pr_g10_reranker_ce`, `pr_e2_rerank_passages`, `slice71_rerank_policy`, `slice71_open_report`; `slice72_*` (feature-complete) |

### Added characterization (only the gaps)

1. **Exhaustive completion classifier (R27-70D).**
   - **What:** a `#[cfg(test)]` unit test in `projection_generation.rs` over
     `classify_completion`.
   - **Input domain:**
     - terminal ∈ {none, `up_to_date`, `failed`};
     - sidecar ∈ {none, expected kind with matching identity, expected kind
       with mismatched identity, other kind};
     - physical ∈ {none, expected source type and kind, wrong source type,
       wrong kind};
     - `is_edge` ∈ {false, true};
     - `enrolled` ∈ {false, true};
     - runtime state ∈ every `ProjectionRuntimeStateV1` variant.
   - **Kinds:** `is_edge = true` uses `edge_fact`. `is_edge = false` uses a
     node kind that `resolve_source_type` accepts.
   - **Precedence:** the Slice 40 table lists both "`failed` + no sidecar +
     no vec0 → failed" and "edge member not enrolled → corrupt". For a
     failed, unenrolled edge, the specific failed row wins. It carries no
     enrolment qualifier. The edge row's rationale concerns enrolment that
     must be complete before publication. (The table's phrase "enrolment is
     scheduler state" is scoped to node-only completion and is not relied
     on here.) The ambiguity is recorded for a future Slice 40 design edit.
     This is also
     current production behavior (`projection_generation.rs` `Failed` arm),
     which a characterization test must pin.
   - **Oracle:** a hand-written expected function that transcribes the
     0.8.25 Slice 40 table with that precedence:
     - `Complete` iff `up_to_date` and matching sidecar and matching physical
       row and (node or enrolled);
     - `Failed` iff `failed` and neither sidecar nor physical row;
     - `Pending` iff no terminal, sidecar, or physical row and enrolled, or
       the stranded-node marker case (node, `up_to_date`, no sidecar, no
       physical row, not enrolled, runtime not `Usable`);
     - otherwise typed corruption, asserted as
       `ProjectionGenerationErrorReason::ProjectionGenerationCorrupt`.
   - **Named assertions:** `failed` with a physical row, and a physical row
     without a sidecar, are both corruption.
   - **Non-vacuity mutants:**
     - drop the `physical.is_none()` term of the `Failed` arm;
     - drop the `sidecar` term of the `Complete` arm.
2. **Zero residue immediately after a failed projection commit
   (R27-70C).**
   - **Where:** `tests/slice70_projection_commit_residue.rs`, gated
     `#![cfg(debug_assertions)]` like `tc91_projection_commit_hardening.rs`,
     because the hooks are `debug_assertions`-only. It runs in the default
     debug route.
   - **Hooks:** `force_next_projection_commit_failure_for_test` plus
     `pause_projection_commit_failure_cleanup_for_test`. The pause fires
     after the forced error dropped the uncommitted transaction and before
     redispatch. No rows are pre-seeded.
   - **Success arm:** a fixed embedder and a configured vector projection.
     Write one node and wait on `reported`. From a separate connection,
     assert zero terminal, sidecar, and `vector_default` rows for that
     cursor. Then release, `drain`, and assert that the vector is present.
   - **Failed-outcome arm:** an always-failing embedder with short retry
     delays (`set_projection_retry_delays_for_test`), so the rolled-back
     transaction held both a `failed` terminal and a `projection_failures`
     audit row. At the pause, assert zero terminal rows and zero
     `operational_mutations` rows in collection `projection_failures` for
     that cursor. Then release, `drain`, and assert that the failure count is
     exactly 1, so the redispatched failure is recorded once.
   - **Non-vacuity mutant:** temporarily move the forced-failure block in
     `commit_projection_outcomes` after `tx.commit()`. Both arms must
     fail.
3. **Stale-job generation.** Already covered, as the owners above show. No
   test is added.

If a characterization test fails against unmodified production, the move
stops for a separate RED/GREEN correction.

## Source scrapers and gates

- `tests/slice35_virtual_mutation_manifest.rs` counts literal mutation
  needles over a `concat!` of `include_str!` files. Each new module that
  receives a mutation site or a coupled function is appended to `SOURCE` in
  the batch that creates it. Every needle, count, and coupling assertion stays
  byte-identical.
- `experiments/slice35_virtual_mutation_audit.py` keys `MutationSite` and
  helper-caller entries by file.
  - **Pre-existing red:** the audit is already red at the baseline (ledger
    seq 257). Its inventory still expects
    `commit_projection_outcomes … INSERT OR IGNORE INTO vector_default` ×2,
    but 0.8.25 Slice 40 (`2a65a38a`) deliberately made publication
    `INSERT INTO`, so a partial tuple fails rather than being repaired.
  - **First fix:** a standalone test-infrastructure commit corrects those
    two entries to `INSERT INTO`. It also removes the stale
    `PRODUCTION_HELPER_CALLERS["delete_vector_partition_row"]` entry for
    `commit_projection_outcomes`, which no longer calls that helper. After
    that commit,
    `tests/experiments/test_slice35_virtual_mutation_audit.py` goes green.
  - **During the move:** each batch re-keys its moved functions from
    `lib.rs` to the new file, path-only.
  - **Closeout:** seq 257 is closed as done.
- **Manifest body identity.** `function_body` ends a body at `\nfn`,
  `\n    fn`, `\n    pub fn`, or `\n    pub async fn`, not at
  `pub(crate) fn`. A visibility change could therefore widen an extracted
  body and make a `contains_all` check vacuously green. Each batch records
  that every `contains_all` target's extracted body is byte-identical before
  and after the move.
- **C1 conformance gate.**
  - **What breaks:** `scripts/check-c1-conformance.sh` reads `ENG = …/lib.rs`
    for `configure_projections`, `apply_projection_config`,
    `commit_projection_outcomes`, `notify_new_work`, and
    `load_projection_registry`. Its self-test fixtures
    (`scripts/tests/test_check_c1_conformance.sh`) edit
    `configure_projections` and `apply_projection_config` text in `lib.rs`.
  - **How it is fixed:** following Slice 40 (`11962b37`, `19e4028f`), the
    batch that moves each owner first shows the gate RED. It then adds
    per-module path constants to the gate, listed in its source manifest,
    and retargets the matching probes and fixture paths.
  - **What stays the same:** probe semantics and assertions.
    `scripts/c1-conformance-pin.json` is unchanged; its `lib.rs:NNNN`
    evidence strings are prose.
- **Removal detector.** AC-050c (`scripts/security/check-removal-changelog.sh`)
  runs with its base at the pre-move SHA and must report zero removals. That
  includes the three reranker functions moved behind root `pub use`.
- **Other checks:**
  - **Plan-anchor citations.** `scripts/lint-plan-anchors.sh` checks
    citations of functions by file. The ACTIVE `dev/plans/plan-0.8.20.md`
    cites `migrate_vector_partition_pack1_to_pack2` and
    `run_pin_and_requantize_pass` in `lib.rs` (around lines 730 and 749).
    Those citations are corrected in the batches that move the functions,
    as Slice 60 did.
  - Line-number comments in test prose (`perf_gates.rs` and similar) are
    not gates and are not edited.

## Structural-move evidence

- **Batches (each about 300-1,200 moved lines, one commit each):**
  1. vector storage (about 620 lines);
  2. vector equivalence (about 670);
  3. mean and embedding (about 500);
  4. registry A: row-owned projections, `ProjectionPass`,
     `StoredProjection`, nested-source validation, and registry load and
     persist (about 700);
  5. registry B: attribute projection, `apply_projection_config`, and
     `Engine::configure_projections` / `read_projections` (about 750);
  6. registry C: vector enrolment and backfill, boot rederive, and the cache
     snapshot (about 600);
  7. runtime types and `impl ProjectionRuntime` (about 880);
  8. dispatcher and worker loops through `run_projection_job` (about 955);
  9. pending scans and projection commit (about 810);
  10. rerank, rebuild, and generation status (about 600).
- **Per batch (cheap):**
  - `cargo check -p fathomdb-engine --all-targets` with each of: default,
    `operator`, `test-hooks`, `default-embedder`, `default-reranker`,
    `default-reranker,tc5-benchmark`, `default-reranker,slice72-test-hooks`,
    `tc5-benchmark`, `slice72-test-hooks`, and `migration-test-hooks`;
  - `cargo clippy -p fathomdb-engine --all-targets -- -D warnings`;
  - that batch's focused owners from the table above, run serially, each
    on its required route: `rebuild_*` and `pr2b_mean_recompute` operator
    arms on `operator`; `slice40_*` on `test-hooks` in debug, plus
    `migration-test-hooks` where required; `slice71_*` and
    `pr_g10_reranker_ce` on `default-reranker`. A route that executes zero
    tests counts as a failure, not a pass.
  - the slice35 manifest test, the audit test, the manifest body-identity
    check, and the C1 gate when the batch moved a C1 owner.
- **At the pre-move receipt and at the final Rust candidate (heavy):** the
  captures at the docs-only commit `a95b5b0f` serve as the pre-move receipt,
  because the characterization tests change no public item.
  - a Slice 30 public capture and compare against the immutable
    `dev/plans/0.8.27/features/slice-30/baseline.json` (Node `v25.9.0`, as
    pinned);
  - a hidden capture and compare against
    `dev/plans/0.8.27/features/hidden-surface/baseline-8e2afb29.json`;
  - the release probe;
  - test inventory.
- **Why heavy captures are not run per batch:** moved items are private.
  Batch 10, which adds the root `pub use` of the reranker functions, runs the
  public and hidden captures before its commit.
- **Expected hidden diff:** additive only, meaning the Slice 50/60 reviewed
  additions plus the Slice 70 tests.
- **Doc warnings:** the post-move `rustdoc::broken_intra_doc_links` warning
  set, with private items documented, adds no warning.
- **Final gates:**
  - `./scripts/agent-verify.sh`;
  - `cargo clippy --workspace --all-targets -- -D warnings`;
  - `cargo check --workspace --all-targets`;
  - the release typecheck;
  - the candidate-bound Python receipt;
  - `scripts/test-feature-complete.sh` on the 3090 executor (summary SHA-256,
    matrix, counts, ignored tests, exclusions), with CPU/CUDA tolerance and
    CUDA-selection evidence recorded separately. Metal is recorded as
    unavailable.

## Risks

- **Intra-module privacy:**
  - **Where it bites:** `impl ProjectionRuntime` and `ProjectionRuntimeShared`
    fields are read by the worker, commit, erasure, write, and search
    modules.
  - **Mitigation:** fields get `pub(crate)` only as needed. There is no
    accessor churn and no logic change.
- **`cfg` drift:**
  - **Where it bites:** `debug_assertions`, `test-hooks`, `operator`, and
    `default-reranker` gates must travel with their items. A gate on a
    `use` line must match its item's.
  - **Mitigation:** the release `--tests` typecheck catches dead imports.
- **Public re-exports:**
  - **Where it bites:** `rerank_fused`, `try_rerank_fused`, and
    `rerank_passages` must keep their exact root paths, docs, and `cfg`.
    `MEAN_VEC_PIN_THRESHOLD` and `mean_centering_internals_for_test` stay at
    root.
  - **Mitigation:** the public and hidden captures after those batches are
    the oracle.
- **Unit tests in `lib.rs` `mod tests`:** they reach moved private items
  through `super::`. Their imports get path-only edits. Assertions are
  unchanged.
