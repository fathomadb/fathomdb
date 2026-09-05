---
title: 0.8.25 Slice 40 verification matrix
status: COMPLETE
candidate_product_commit: 2313fd34
review_candidate_commit: 2313fd34
implementation_review: PASS_CYCLE_8_FIX_1
fixture_reconciliation_commit: ea7a553f
fixture_reconciliation_review: PASS
closeout_evidence_commit: ed9776ad
closeout_oracle_fix_commit: ccf7c695
closeout_review: PASS
---

# Slice 40 verification matrix

This record maps every Slice 40 claim to an executable proof or retained
receipt. A passing unit suite alone does not close the slice. The exact hashes,
locators, commands, and platform outcomes are retained in
[`evidence-manifest.json`](evidence-manifest.json).

## Acceptance traceability

| Criterion | Required claim | Executable evidence | State |
| --- | --- | --- | --- |
| S40-AC1 | Database-local generation identity has closed grammar, survives restart/copy, is never reused, and changes only for real configuration/rebuild transitions. | `step32_projection_generation`; `slice40_projection_generation`; `projection_generation::tests::generation_mint_retries_collisions_without_reusing_history` | PASS focused |
| S40-AC2 | Readiness is boundary-qualified and never treats a terminal without its exact physical member as ready. | `slice40_projection_completion`; `projection_generation::tests::completion_classifier_is_closed_over_every_persisted_shape`; `slice40_status_performance` | PASS |
| S40-AC3 | A Slice 25 pending cursor is resolved only through its receipt, operation, generation, and physical-member state. | `slice40_mutation_projection_status`; Python and TypeScript `receipt-keyed mutation status` tests; PyO3/N-API request codec tests and N-API canonical-decimal property test; fresh Linux and Windows package smokes | PASS |
| S40-AC4 | Configuration/rebuild retires and mints metadata honestly while the physical stores remain in place and expose non-ready state until complete. | `non_noop_configuration_mints_but_exact_replay_reuses_generation`; `each_operator_rebuild_mints_a_distinct_generation`; transition/status cases in completion and mutation suites; retained status detail | PASS |
| S40-AC5 | Frozen reads drift on generation/readiness changes; stale work cannot publish across transition, lifecycle closure, or erasure. | `slice35_frozen_read`; `slice40_projection_generation_races`; selected Slice 30 closure and erasure tests below; full serial workspace | PASS |
| S40-AC6 | Public/wire parity, indexed bounds, bounded cost, CPU/CUDA package behavior, and Windows build coverage are proven without overstating GPU computation. | Binding unit/package tests; query-plan fixture; common and status campaigns; CUDA preflight; Windows native build and package smoke | PASS |

## Race and lifecycle seams

| Seam | Exact proof | State |
| --- | --- | --- |
| Worker queued across generation transition | `queued_worker_across_generation_transition_is_rediscovered` (exactly two embedding calls) | PASS |
| Worker computing during generation transition | `worker_computing_across_generation_transition_discards_stale_result` | PASS |
| Worker waiting on the publication lock during transition | `worker_at_write_lock_boundary_discards_after_transition` (exactly two embedding calls) | PASS |
| Publication transaction linearizes before transition | `publication_holding_write_lock_linearizes_before_transition` (exactly one embedding call) | PASS |
| Closure wins before worker publication | `projection_worker_before_admission_cannot_publish_dependency_residue` | PASS predecessor |
| Worker terminalizes after admission | `admission_before_projection_worker_terminalizes_without_publication` | PASS predecessor |
| Restart recovers nonterminal closure without reusing authority | `next_writer_recovers_a_proving_soft_closure_after_reopen` | PASS predecessor |
| Concurrent writer is fenced during closure | `unrelated_writer_is_fenced_while_physical_closure_is_nonterminal` | PASS predecessor |
| Erasure drains/fences publication | `erase_source_drains_before_freezing`; `excise_source_drains_before_freezing`; `erase_source_removes_no_embedder_pending_edge_work` | PASS predecessor |
| Old/new reader linearization and token drift | `frozen_context_rejects_tamper_database_mismatch_context_change_and_state_drift`; generation/readiness additions in `slice35_frozen_read` | PASS focused |
| Direct old-generation or duplicate physical publication | `worker_publication_never_repairs_a_partial_projection_tuple`; `stale_worker_result_cannot_publish_into_a_new_generation` | PASS |
| Temporal membership changes while dispatcher is idle | `dispatcher_rechecks_pending_member_at_source_valid_from_without_external_notification`; boundary and clock-rollback cache tests | PASS |

The Slice 30 tests remain authoritative for dependency closure and erasure;
Slice 40 does not duplicate them under new names. The Slice 40 tests add only
the generation-specific captured-work seams.

## Cost and query-plan proof

| Claim | Receipt or test | Required interpretation | State |
| --- | --- | --- | --- |
| Ordinary write and storage regression | `scale-02-slice40-common-20260905T1101Z-0dde4803` | Candidate versus pinned Slice 35 source; 10,000 operations, 79 receipts, frozen dispatcher | PASS: p50 0.319%, p95 0.282% upper regression |
| Per-receipt amplification | Candidate-only one-operation observation in the common campaign | Exact logical, provenance, receipt, generation, terminal, sidecar, and vec0 row counts; descriptive, not a threshold | PASS retained |
| Reopen regression | Same common campaign | Five paired databases and registered relative-or-absolute policy | PASS: 7.931% / 3.826 ms |
| Cold status cost | `scale-02-slice40-status-20260905T1120Z-bacc3304` | First generation and mutation calls use separate reopens and are excluded from steady samples | PASS retained separately |
| Steady status bounds | Same status campaign | Five CPU-build and five CUDA-linked-build runs at 50,000 owners; p95/p99 policy; temporal boundaries excluded and counted, while boundary-free epoch changes remain steady | PASS: max generation p95/p99 0.024/0.042 ms; mutation 0.027/0.048 ms |
| Transition cost | Same status campaign | Metadata transition, first post-transition generation status, and rejected old-generation mutation status are separate observations | PASS retained separately |
| No steady O(N) scan | `steady_full_owner_scans == 0` plus cache-invalidation tests | Counter increments only on completion-cache misses; epoch-rollover scans are reported separately | PASS: zero in all ten runs |
| Durable indexed correlations | `status_query_plans_bound_correlations_with_durable_indexes` and retained plan strings in each status result | Canonical owner and correlated provenance/terminal/sidecar/kind lookups use durable indexes; vec0 virtual-table lookup is reported, not relabeled | PASS retained |
| Page, WAL, and generation retention | `database_observations` in the status receipt; common worker storage fields | Page count/size, DB/WAL bytes, one serving generation, and retained generation-history rows | PASS: +28,672 bytes maximum; WAL zero after checkpoint |
| CUDA selection and allocation | CUDA preflight witness `30d32467…`; Python `edd276d1…`; N-API `0214e71d…` | Proves `cuda:0` on RTX 3090 with 128 MiB observed allocation; makes no embedding-throughput claim | PASS |
| Embedding throughput | `scale-02-slice40-cuda-throughput-20260905T1421Z-fc632910` | Five 1,024-record repetitions on the exact RTX 3090; descriptive only and never attributed to SQLite status calls | PASS: 286.096 records/s median |
| Receipt classification | `slice40-common-v1` and `slice40-status-v1` plans; pre-index classification callback; retained-run sidecars | Decision metrics are compact and source-bound; full raw detail and all executing Python/Rust sources are hashed | PASS |

The status campaign compares the SQLite status path in a CPU build with the
same path in a CUDA-linked build. Its fixed test embedder executes on CPU. It
is not evidence of GPU computation. Actual GPU allocation and embedding are
proved separately by the CUDA preflight and fresh-package smoke.

## SDK, package, and platform proof

| Surface | Required route | State |
| --- | --- | --- |
| Rust Engine/schema | Focused Slice 40 suites and full serial workspace | PASS; the sole sandbox ptrace failure passed unchanged unconfined, 1/1 |
| Python source binding | `fathomdb-py` unit tests | PASS |
| Python installed artifact | Fresh CPU wheel and fresh CUDA wheel; import-path witness; generation and receipt-keyed status smoke | PASS Linux CPU/CUDA and Windows native |
| TypeScript source binding | Fresh native debug build, TypeScript compile, exact Slice 40 test file | PASS |
| TypeScript installed artifact | Fresh offline npm/native artifact; generation and receipt-keyed status smoke | PASS Linux CPU/CUDA and Windows native |
| Canonical wire/property | Shared request/error fixtures, `u64::MAX`, N-API canonical decimal property test | PASS focused |
| CUDA runtime | `nvidia-smi`, exact CUDA features, `cuda-preflight.sh`, reopen/status smoke, allocation witness | PASS; RTX 3090 UUID `GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b` |
| Windows build | Native Python 3.11.16 wheel and Node 24.18 N-API build; TypeScript output | PASS on `gh-runner-wonl-win11` |
| Windows native | Fresh installed wheel and native N-API status/reopen smokes | PASS; both report blocked/absent under a no-runtime reopen |
| Repository | Fast agent verify, applicable heavy routes, full workspace, docs/state checks, `git diff --check` | PASS: final unconfined fast gate 103/103, zero skipped/excluded |

## Closure rule

Slice 40 may close only when:

1. independent review passes the exact candidate commit;
2. the measurement config pins that reviewed commit;
3. final common and status receipts pass or a registered bound produces an
   explicit advisory result and owner decision;
4. package, CUDA, Windows cross-build, and repository rows above carry exact
   evidence; and
5. [`status.md`](status.md) records hashes and commands without claiming that a
   CUDA-linked status build performed GPU computation.
