---
title: FathomDB 0.8.27 Slice 70 - engine projection, embedding, vector, and reranking domains
status: PROPOSED
target_release: 0.8.27
baseline_sha: a95b5b0f
---

# Slice 70 plan

This plan supersedes the six-paragraph master-plan draft
(`dev/plans/plan-0.8.27.md`, "Slice 70 — projection, embedding, and
reranking") as the execution authority for this slice once design review
passes. `design.md` is the authority for the exact inventory, batches,
characterization, and evidence. Where this plan and `design.md` differ,
`design.md` wins. The review record is `design-review.md`.

## Outcome

Move the root-owned projection runtime, projection registry, vector physical
storage, mean and equivalence mechanics, projection-time embedding, and
standalone reranking out of `fathomdb-engine/src/lib.rs` into private
semantic modules. The move changes no behavior, schema, public path, feature
gate, transaction scope, lock acquisition, `commit_gate` ordering, or
startup/close order. Before moving, add only the missing projection-state
characterization named below.

This is a behavior-preserving structural slice. It fixes no known defect. If a
characterization test exposes a semantic defect, the move stops for a separate
RED/GREEN correction.

Execution is direct, in the release worktree, with one writer. No Steward or
Orchestrator role is used. Independent reviews are read-only. Codex is the
reviewer of record; an adversarial review subagent is the fallback when
codex is unavailable.

## Review of the pre-commission findings

A pre-commission review (2026-09-26) recommended not commissioning from the
draft and raised seven findings. Each finding was verified read-only against
`6f3b625e`:

| # | Finding | Verdict | Evidence and adjustment |
| --- | --- | --- | --- |
| 1 | No slice-level plan or design | ACCEPT | Only the master paragraph existed. This plan follows the Slice 50/60 artifact set: `plan`, `design`, `design-review`, `tdd-chronology`, `code-review`, `review-verification`, and `status`. |
| 2 | "Recovery after interruption" is ambiguous | ACCEPT, citation corrected | `tests/projection_runtime.rs` AC-063a/b (around line 480/513) covers only the terminal half. Pending-after-reopen is covered by `tc91_projection_commit_failure_survives_stop_and_reopen` and `slice21_projection_runtime_state::approved_open_boot_grafts_once_and_never_reopens_failed_terminals`. Rebuild retry is covered by `rebuild_projections::ac_063c_rebuild_projections_materializes_failed_terminal_rows`. In source, `next_pending_projection_jobs` treats a missing terminal as pending, and `reenqueue_stranded_vector_rows` skips `failed`. The replacement wording is adopted in R27-70C. |
| 3 | Physical-readiness language is overbroad | ACCEPT | The member-scoped triple (terminal, `_fathomdb_vector_rows` sidecar, vec0 row) is specified in `dev/plans/0.8.25/features/slice-40/design.md` and enforced by `projection_generation.rs` `member_completion` / `classify_completion`, with eligibility checked separately. `member_completion` issues three reads without its own transaction, so the single-snapshot guarantee rests on callers. The claimed test gap (`failed` plus a physical row; vec0 without a sidecar) was wrong: the in-crate unit test `completion_classifier_is_closed_over_every_persisted_shape` already covers the whole domain. |
| 4 | Ownership overlaps Slices 80 and 90 | ACCEPT, refined | `ProjectionRuntimeShared` holds four search fields: `search_limit_override`, `recency_reweight_enabled`, `importance_reweight_enabled`, and `vector_stage_only_for_test`. Slice 70 relocates the struct unchanged and records those fields as a Slice 80 hand-off; the struct's shape does not change. `fuse_rrf`, the `RRF_*` constants, and `read_search_in_tx` stay for Slice 80. `rerank_fused` and `try_rerank_fused` move as rerank implementation, and search keeps calling them. |
| 5 | Embedder configuration contract and code disagree | ACCEPT | `PROJECTION_WORKERS = 2` and `DEFAULT_EMBED_TIMEOUT_MS = 30_000` are fixed in `lib.rs`. The accepted `ADR-0.6.0-embedder-protocol.md` and the locked `dev/design/bindings.md` say pool size and timeout are configurable. Python `Engine.open` stores `EngineConfig` without forwarding it to native open. NAPI `open` ignores `engine_config`, and TypeScript only stores it. This is recorded as a known gap assigned to Slice 90. Slice 70 preserves current behavior and must not describe the fixed worker count as meeting the configurable-pool contract. |
| 6 | Feature and GPU evidence is under-specified | ACCEPT, premise corrected | `scripts/test-feature-complete.sh` (via `scripts/lib/feature_complete.py`) requires both RTX 3090s and pinned weights and fails closed when they are missing (`FATHOMDB_REQUIRE_LIVE=1`). It never uses `--all-features`, because the Metal features are macOS-gated. The claim that no usable CUDA host exists is stale: `prework/slice-0.md` says so, but the hidden-surface unit later ran the gate on the 3090 executor (`features/hidden-surface/status.md`). This plan names that executor. Metal remains unavailable and is recorded as such. |
| 7 | Stale authority metadata | ACCEPT, rescoped | AC-079: `dev/interfaces/rust.md` says it remains unminted, while `dev/acceptance.md` records it as HITL-signed and minted at Slice 40. This is corrected in Step 0. `ADR-0.8.23-dual-runtime-device-policy.md` is still `proposed` (decision-index row 43). Ledger `seq-250`/`seq-252` concern 0.8.23's Slice 70, not this slice, and an ADR status change needs a HITL ruling, so it is proposed to Slice 140 and not edited here. `dev/design/embedder-decision.md` and `dev/design/0.8.1-slice-10-reranker-design.md` remain `UNREVIEWED`. They are treated as informative history in the design's authority hierarchy and are not edited. |

## Design-time reconciliation (at `86c66379`)

`design.md` is the authority for the exact inventory. It settles these
points:

- **Maintenance:** `rebuild_projections`, `rebuild_vec0`, `run_rebuild`, and
  `rebuild_shadow_state` move to `projection_rebuild.rs`, because the master
  plan assigns projection maintenance to this slice.
- **Operator methods:** `verify_embedder` and the other operator diagnostics
  stay for Slice 90.
- **Test seams:** every `*_for_test` Engine seam stays at root for the
  Slice 140 gating.
- **Public reranking:** `rerank_fused`, `try_rerank_fused`, and
  `rerank_passages` are public at the crate root. They move with root
  `pub use` re-exports. `MEAN_VEC_PIN_THRESHOLD` and
  `mean_centering_internals_for_test` stay at root.
- **Open-path helpers:** `default_embedder_identity`,
  `edge_vector_prune_complete`, and `prune_orphaned_edge_vectors` are called
  only from the open path, so they stay for Slice 90. `projection_status`
  stays with its only caller, a Slice 140 test seam.
- **Gates that name moved owners:** `scripts/check-c1-conformance.sh` and
  its self-test get a Slice 40-style path retarget. AC-050c runs against the
  pre-move base.
- **Generation-race coverage:** R27-70B is already covered by
  `slice40_projection_generation` and
  `slice40_projection_generation_races`. No test is added for it.
- **Classifier coverage:** R27-70D is already covered by the in-crate
  exhaustive `completion_classifier_is_closed_over_every_persisted_shape`,
  so no test is added (`design.md`).
- **Mutation audit:**
  - **Status:** `experiments/slice35_virtual_mutation_audit.py` is red at the
    baseline (ledger seq 257).
  - **Cause:** its inventory predates the deliberate `INSERT OR IGNORE` to
    `INSERT` change in 0.8.25 Slice 40.
  - **Fix:** the audit keys the functions this slice moves, so the slice
    corrects the stale entry first and then re-keys each batch.
- **Surface captures:** the heavy public and hidden captures run at the
  pre-move receipt, after the batches that add root re-exports, and at the
  final candidate. Cheap checks run per batch.

## Assigned inventory and disposition

Line numbers are approximate at `a95b5b0f`. The by-name inventory in
`design.md` is authoritative.

| Destination (private) | Inventory from `lib.rs` |
| --- | --- |
| `projection_runtime.rs` | `ProjectionJob`, `ProjectionRuntimeState`, `ProjectionRuntimeShared` (unchanged, including the four search fields), `ProjectionRuntime`, and the startup types (around 685-957); `impl ProjectionRuntime` (around 2720-3300). |
| `projection_worker.rs` | Startup reporting, `projection_dispatcher_loop`, `projection_worker_loop`, `ProjectionOutcome`, `run_projection_jobs`, `embed_projection_batch`, `run_projection_job`, and the pending-work scans (around 16146-17597). |
| `projection_commit.rs` | Cursor helpers, `record_projection_terminal`, and `commit_projection_outcomes` with its `commit_gate` usage (around 17880-18316). |
| `projection_registry.rs` | Row-owned projections, `ProjectionPass`, `StoredProjection`, registry load and persist, `apply_projection_config`, vector enrolment and backfill, `rederive_projections_on_boot`, and the registry cache (around 20405-22330); `Engine::configure_projections` and `Engine::read_projections`. |
| `vector_storage.rs` | Profile and partition helpers, vec0 attribute columns, reshape and pack migrations, the blob codec, `quantize_binary_via_sql`, and committable kinds (around 18887-19470 and 20143-20227). |
| `vector_equivalence.rs` | The equivalence probes, `VectorEquivalenceOutcome`, `usable_dense_runtime`, `probe_embed`, and `run_vector_equivalence_probe` (around 19471-20143). |
| `mean.rs` | `MeanAccumulator`, `run_requantize_pass`, `recover_mean_vec_pin`, `recompute_mean_in_tx`, the mean helpers, and `Engine::recompute_mean`. |
| `embedding.rs` | `embed_with_watchdog`, `embed_batch_with_watchdog`, the circuit-breaker helpers, and `map_runtime_embedder_error`. |
| `rerank.rs` | `rerank_fused`, `try_rerank_fused`, `rerank_passages`, `ce_rerank`, `CandleCrossEncoder`, and `reranker_singleton`, with every `default-reranker` gate preserved. |
| `projection_generation.rs` (extended) | `derive_dense_readiness`, `Engine::read_projection_status`, and `Engine::read_embedding_readiness`. No duplicate status module is created. |
| Stays at root | Public projection, embedding-readiness, and verify types keep their root paths (moved only with root re-exports, if at all). `check_embedder_profile`, the open-time embedder and reranker gates, and the `open_with_migrations` startup order stay for Slice 90. `fuse_rrf`, `RRF_*`, `SEARCH_RERANK_LIMIT`, and every `search_*` path stay for Slice 80. `PROJECTION_WORKERS` and the timeout default keep their values. |

The call sites in `write.rs`, `write_commit.rs`, `erasure.rs`,
`actuation.rs`, `record_lifecycle.rs`, `consolidation.rs`, `frozen_read.rs`,
`dependency_closure.rs`, `data_plane_integrity.rs`, and
`projection_generation.rs` are rewired as import-only changes.

## Requirements and acceptance

These are release-local. The locked global acceptance register is not
changed.

| ID | Requirement | Acceptance |
| --- | --- | --- |
| R27-70A | Exact structural scope. | AC27-70A: an approved inventory and seam table; batches of 300-1,200 moved lines; `projection_generation.rs` is extended, not duplicated; no public, schema, or wire change. |
| R27-70B | Generation integrity. | AC27-70B: a no-op configuration reuses the generation; effective-configuration and rebuild transitions mint exactly as currently specified; a captured stale job cannot publish into a newer generation. |
| R27-70C | Truthful recovery. Pending work without a durable terminal is rediscovered after worker interruption, failed commit, shutdown, and reopen. Terminally failed work remains terminal and is retried only through the governed rebuild workflow. | AC27-70C: immediately after a failed projection commit, with cleanup paused, no terminal, sidecar, vec0 row, or failure audit exists. Pending work redispatches and survives reopen. Terminal failures do not retry implicitly. |
| R27-70D | Physical completeness is member-scoped. | AC27-70D: eligible dense completion requires a matching terminal, sidecar, and vec0 row in one snapshot. Every partial or mismatched tuple, including `failed` with a physical row and vec0 without a sidecar, fails typed. Lifecycle eligibility remains an independent axis. |
| R27-70E | Embedding safety is unchanged. | AC27-70E: serialization, the watchdog timeout, the bounded abandoned-thread circuit breaker, dimension validation, feature-off behavior, and caller/default identity rules are unchanged. |
| R27-70F | Vector and mean atomicity. | AC27-70F: projection publication, first mean pin, and manual mean recomputation keep their `commit_gate` ordering. Rollback publishes neither mean state nor events. Encoding and equivalence invariants remain exact. |
| R27-70G | The reranker boundary is standalone. | AC27-70G: only standalone reranking and the cross-encoder move. Feature-off identity, depth-zero and empty short circuits, deterministic ordering, non-finite handling, typed forced-device errors, and soft fallback are preserved. Search orchestration and RRF fusion remain in Slice 80. |
| R27-70H | Structural and feature evidence. | AC27-70H: the public and hidden surfaces compare equal and the release probe is equal. The test inventory is additive-only. The exact affected feature routes pass. `scripts/test-feature-complete.sh` passes on the named 3090 CUDA executor with a candidate-bound summary and artifact digest. The canonical gate, workspace Clippy with warnings denied, and `cargo check --workspace --all-targets` pass. |

## TDD implementation sequence

Use one writer in the release worktree. Read-only reviewers share it.

0. **Commission and reconcile (docs commit).**
   - Record the `slice-70-execution` ruled decision (repository owner,
     2026-09-26) in `dev/plans/release-state-0.8.27.json`. Mark Slice 70 in
     progress and update `release_kind`. Edit the JSON only, then run
     `scripts/check-release-state-views.sh`.
   - Correct the AC-079 status line in `dev/interfaces/rust.md`.
   - Through `ledgerwrite`, after a full unfiltered tail read, append two
     todos: the configuration-forwarding gap is assigned to Slice 90, and the
     dual-runtime ADR status reconciliation is proposed to Slice 140.
1. **Design.** Write `design.md`: authority hierarchy, grep-derived seam
   table, destination modules, characterization design, structural-move
   evidence, and risks. Resolve how the four search fields on
   `ProjectionRuntimeShared` are handed off.
2. **Independent design review.** Run a read-only design review and iterate
   to PASS before any code. Record it in `design-review.md`.
3. **Map existing tests to R27-70B-G** in `tdd-chronology.md` before adding
   tests.
4. **Characterize (tests only).** Add only these missing cases:
   - zero residue immediately after a failed projection commit, with a
     success arm and a failed-outcome arm, using
     `pause_projection_commit_failure_cleanup_for_test`;
   - the stale-inventory correction of the slice35 virtual-mutation audit
     (seq 257), as a separate test-infrastructure commit that fixes both the
     `INSERT` entries and the stale helper-caller entry.

   Prove each case non-vacuous with a recorded temporary mutant. If a case
   fails against unmodified production, stop for a separate RED/GREEN
   correction with review.
5. **Pre-move receipt.** The Slice 30 public comparison and the
   hidden-surface capture were taken at `a95b5b0f`. The public capture
   compared equal. The hidden capture showed only prior reviewed additions.
   Run the focused owner suites at the characterization commit.
6. **Move in the ten bounded batches** listed in `design.md`, one commit
   each, copied verbatim. Each batch re-keys the slice35 audit, the
   manifest, the C1 gate, and the plan-anchor citations for the functions it
   moved. Per-batch evidence follows `design.md` "Structural-move
   evidence". The heavy public and hidden captures run at batch 10 and at
   the final candidate.
7. **Refactor within the boundaries.** Imports and visibility only. A
   behavioral change requires its own RED test.
8. **Code review.** Independent review of the actual diff. Resolve
   behavioral findings with RED/GREEN. Record it in `code-review.md`.
9. **Verify.**
   - Run `./scripts/agent-verify.sh` on a ptrace-capable, unconfined
     executor, so AC-037 runs live.
   - Run `scripts/test-feature-complete.sh` on the named 3090 executor.
     Record the summary SHA-256, the matrix, the run and test counts, the
     ignored tests, and the exclusions.
   - Record CPU/CUDA numerical equivalence with a justified tolerance, not
     byte identity, separately from evidence that CUDA was actually selected
     and computed.
   - Record Metal as unavailable.
   - Produce the candidate-bound Python receipt.
   - Obtain an independent read-only verification verdict in
     `review-verification.md`.

   Unavailable evidence is never converted into a pass.
10. **Close.** Write `status.md`. Set Slice 70 to
    `COMPLETE_ON_RELEASE_BRANCH` with `sha`, `closeout_sha`, and evidence, and
    advance `next_slice` to 80. Regenerate the release-state views. No push,
    tag, or publication.

## Non-goals

- no search, query, vector-candidate, or RRF moves (Slice 80);
- no open, close, configuration-forwarding, device-resolution, or operator
  facade moves (Slice 90);
- no binding or SDK change (Slices 100-130);
- no change to `ProjectionRuntimeShared`'s shape, `PROJECTION_WORKERS`, or the
  embed timeout default;
- no schema, SQL, wire, error, or public-interface change;
- no ADR status edits and no edits to the two `UNREVIEWED` design memos; and
- no duplicate of existing projection owner tests.
