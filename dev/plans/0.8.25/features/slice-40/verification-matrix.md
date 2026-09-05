---
title: 0.8.25 Slice 40 verification matrix
status: IN_PROGRESS
candidate_commit: 56350c2f
---

# Slice 40 verification matrix

This record maps every Slice 40 claim to an executable proof or retained
receipt. A passing unit suite alone does not close the slice. `PENDING` rows
remain release blockers until replaced by exact evidence or an explicit
platform-unavailable record permitted by the design.

## Acceptance traceability

| Criterion | Required claim | Executable evidence | State |
| --- | --- | --- | --- |
| S40-AC1 | Database-local generation identity has closed grammar, survives restart/copy, is never reused, and changes only for real configuration/rebuild transitions. | `step32_projection_generation`; `slice40_projection_generation`; `projection_generation::tests::generation_mint_retries_collisions_without_reusing_history` | PASS focused |
| S40-AC2 | Readiness is boundary-qualified and never treats a terminal without its exact physical member as ready. | `slice40_projection_completion`; `projection_generation::tests::completion_classifier_is_closed_over_every_persisted_shape`; `slice40_status_performance` | PASS functional; measurement pending |
| S40-AC3 | A Slice 25 pending cursor is resolved only through its receipt, operation, generation, and physical-member state. | `slice40_mutation_projection_status`; Python and TypeScript `receipt-keyed mutation status` tests; PyO3/N-API request codec tests and N-API canonical-decimal property test | PASS focused; fresh-package proof pending |
| S40-AC4 | Configuration/rebuild retires and mints metadata honestly while the physical stores remain in place and expose non-ready state until complete. | `non_noop_configuration_mints_but_exact_replay_reuses_generation`; `each_operator_rebuild_mints_a_distinct_generation`; transition/status cases in completion and mutation suites | PASS focused; retained transition observation pending |
| S40-AC5 | Frozen reads drift on generation/readiness changes; stale work cannot publish across transition, lifecycle closure, or erasure. | `slice35_frozen_read`; `slice40_projection_generation_races`; selected Slice 30 closure and erasure tests below | PASS focused; enclosing gate pending |
| S40-AC6 | Public/wire parity, indexed bounds, bounded cost, CPU/CUDA package behavior, and Windows build coverage are proven without overstating GPU computation. | Binding unit/package tests; query-plan fixture; common and status campaigns; CUDA preflight; Windows cross-build/native record | PENDING final artifacts |

## Race and lifecycle seams

| Seam | Exact proof | State |
| --- | --- | --- |
| Captured generation before queued publication | `stale_worker_result_cannot_publish_into_a_new_generation` | PASS |
| Worker computing during generation transition | `worker_computing_across_generation_transition_discards_stale_result` | PASS |
| Closure wins before worker publication | `projection_worker_before_admission_cannot_publish_dependency_residue` | PASS predecessor |
| Worker terminalizes after admission | `admission_before_projection_worker_terminalizes_without_publication` | PASS predecessor |
| Restart recovers nonterminal closure without reusing authority | `next_writer_recovers_a_proving_soft_closure_after_reopen` | PASS predecessor |
| Concurrent writer is fenced during closure | `unrelated_writer_is_fenced_while_physical_closure_is_nonterminal` | PASS predecessor |
| Erasure drains/fences publication | `erase_source_drains_before_freezing`; `excise_source_drains_before_freezing`; `erase_source_removes_no_embedder_pending_edge_work` | PASS predecessor |
| Old/new reader linearization and token drift | `frozen_context_rejects_tamper_database_mismatch_context_change_and_state_drift`; generation/readiness additions in `slice35_frozen_read` | PASS focused |
| Duplicate or stale physical publication | `worker_publication_never_repairs_a_partial_projection_tuple`; `stale_worker_result_cannot_publish_into_a_new_generation` | PASS |

The Slice 30 tests remain authoritative for dependency closure and erasure;
Slice 40 does not duplicate them under new names. The Slice 40 tests add only
the generation-specific captured-work seams.

## Cost and query-plan proof

| Claim | Receipt or test | Required interpretation | State |
| --- | --- | --- | --- |
| Ordinary write and storage regression | `scale-02-slice40-common` five-pair campaign | Candidate versus pinned Slice 35 source; 10,000 operations, 79 receipts, frozen dispatcher | PENDING final candidate rerun |
| Per-receipt amplification | Candidate-only one-operation observation in the common campaign | Exact logical, provenance, receipt, generation, terminal, sidecar, and vec0 row counts; descriptive, not a threshold | PENDING final candidate rerun |
| Reopen regression | Same common campaign | Five paired databases and registered relative-or-absolute policy | PENDING final candidate rerun |
| Cold status cost | `scale-02-slice40-status` | First generation and mutation calls use separate reopens and are excluded from steady samples | PENDING |
| Steady status bounds | Same status campaign | Five CPU-build and five CUDA-linked-build runs at 50,000 owners; p95/p99 policy; epoch rollovers excluded and counted | PENDING |
| Transition cost | Same status campaign | Metadata transition, first post-transition generation status, and rejected old-generation mutation status are separate observations | PENDING |
| No steady O(N) scan | `steady_full_owner_scans == 0` plus cache-invalidation tests | Counter increments only on completion-cache misses; epoch-rollover scans are reported separately | PASS smoke; formal pending |
| Durable indexed correlations | `status_query_plans_bound_correlations_with_durable_indexes` and retained plan strings in each status result | Canonical owner and correlated provenance/terminal/sidecar/kind lookups use durable indexes; vec0 virtual-table lookup is reported, not relabeled | PASS focused; formal retention pending |
| Page, WAL, and generation retention | `database_observations` in the status receipt; common worker storage fields | Page count/size, DB/WAL bytes, one serving generation, and retained generation-history rows | PENDING |
| Embedding throughput | Separate CUDA preflight/package receipt | Never attributed to SQLite status calls or the fixed CPU measurement embedder | PENDING |

The status campaign compares the SQLite status path in a CPU build with the
same path in a CUDA-linked build. Its fixed test embedder executes on CPU. It
is not evidence of GPU computation. Actual GPU allocation and embedding are
proved separately by the CUDA preflight and fresh-package smoke.

## SDK, package, and platform proof

| Surface | Required route | State |
| --- | --- | --- |
| Rust Engine/schema | Focused Slice 40 suites and full-workspace Clippy/check | Focused PASS; full gate pending |
| Python source binding | `fathomdb-py` unit tests | PASS |
| Python installed artifact | Fresh CPU wheel and fresh CUDA wheel; import-path witness; generation and receipt-keyed status smoke | PENDING |
| TypeScript source binding | Fresh native debug build, TypeScript compile, exact Slice 40 test file | PASS |
| TypeScript installed artifact | Fresh offline npm/native artifact; generation and receipt-keyed status smoke | PENDING |
| Canonical wire/property | Shared request/error fixtures, `u64::MAX`, N-API canonical decimal property test | PASS focused |
| CUDA runtime | `nvidia-smi`, exact CUDA features, `cuda-preflight.sh`, reopen/status smoke, allocation witness | PENDING |
| Windows cross-build | Release-workflow-equivalent PyO3 and N-API cross-build plus TypeScript build | PENDING |
| Windows native | Documented VM smoke when reachable; otherwise explicit unavailable record | PENDING |
| Repository | Fast agent verify, applicable operator/heavy routes, full workspace, docs/state checks, `git diff --check` | PENDING |

## Closure rule

Slice 40 may close only when:

1. independent review passes the exact candidate commit;
2. the measurement config pins that reviewed commit;
3. final common and status receipts pass or a registered bound produces an
   explicit advisory result and owner decision;
4. package, CUDA, Windows cross-build, and repository rows above carry exact
   evidence; and
5. `status.md` records hashes and commands without claiming that a CUDA-linked
   status build performed GPU computation.
