---
title: FathomDB 0.8.27 Slice 85 - recovery status
status: IN_PROGRESS
target_release: 0.8.27
updated_on: 2026-09-29
release_bound_candidate: 7a2f9bf90783f545603516502bac0016d4b93a14
release_closeout_commit: 8cd3389dadba89925d51440b83d05044e87d2090
recovery_branch: slice-85-fix
recovery_source_candidate: 294af94b5b0e9076ccb35956a3863ef12c5599af
implementation_status: COMPLETE
qualification_status: BLOCKED
---

# Slice 85 recovery status

The owner authorized bounded recovery under the repository-root
`plan-slice-85-recovery.md`. The recovery commits are ancestors of the current
`release/0.8.27` branch, while release state still binds the earlier reviewed
candidate. The recovery work preserves engine
ownership, narrow errors and removal of the four inherited cycles, while
retiring the gate's inventories, general resolver and excessive mutation work.

Current work and qualification live once in [recovery-receipt.md](recovery-receipt.md).
Phase A and Phase B are integrated. The bounded independent review is closed;
source checks are recorded against `294af94b5`. Official public/hidden capture
and GPU evidence remain outstanding. The public capture was blocked by disk
capacity at its recorded attempt; current free space clears that preflight.
The hidden capture and GPU routes remain blocked by the NVML mismatch.
No release-state binding or generated release view was changed by recovery.

## Retained engine deliverable

- `read_api` and `graph_api` own Engine facades;
- `reader_pool` owns the private request protocol and typed factories;
- `structural_state`, `reader_transaction` and `wal_attribution` own their
  leaf responsibilities;
- read, filter, graph expansion, search and telemetry own handler carriers
  and results; and
- graph error impl colocation, private counters, real error-route tests and
  the feature-gated source-scraper repair remain.

The maintained gate contract is AC27-85C/D/E in the master plan. Normal lint
checks production boundaries; the registered fast suite covers cheap units,
wrapper freshness and ownership guards. Architectural qualification is explicit,
separate from fast/heavy/default-all. There is no frozen whole-crate edge,
receiver, inherent or module-SCC census.

## Historical evidence

The original accepted candidate and closeout above remain the release binding.
`baseline-verification.md`, `code-review.md`, `review-verification.md` and
`runtime-closure-review.md` retain their candidate-specific evidence.
`tdd-chronology.md` preserves compact historical RED/GREEN references;
the full pre-recovery status and chronology remain at `df1ffd000` in Git.
The eight review cycles and interrupted tail did not advance release authority.
Their counts and old surface/native receipts do not qualify the recovery head.

Implementation completion and evidence limitations must be read from the
recovery receipt. Rebinding requires a separate owner instruction and missing
qualification; ancestry alone does not qualify the recovery for Slice 90.
