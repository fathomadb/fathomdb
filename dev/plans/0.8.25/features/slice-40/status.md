---
title: 0.8.25 Slice 40 status
status: IMPLEMENTED-AWAITING-VERIFICATION
slice: 40
updated: 2026-09-05
---

# Slice 40 status

## Current state

Slice 40 is implemented and awaiting final verification on `release/0.8.25`.
Schema step 32, one durable
database-local serving-generation identity, physical-member-qualified
readiness, compact Slice-25 receipt correlation, frozen-read invalidation, and
additive Rust/Python/TypeScript status contracts are implemented. Projection
stores remain in place; no side-by-side store, public work manifest, semantic
policy, or application-owned cleanup was added.

Independent design review passed at cycle 4. Independent implementation review
passed at cycle 8 after closing the temporal-dispatcher progress gap, with no
remaining P0/P1/P2 finding. A separate review passed the full-gate fixture
reconciliation at `ea7a553f` without identifying a product regression.

## Evidence

- Reviewed product candidate: `2313fd34`.
- Full-gate fixture reconciliation: `ea7a553f`.
- Final implementation review:
  [`implementation-review-cycle8.md`](implementation-review-cycle8.md).
- Fixture-reconciliation review:
  [`implementation-review-fixture-reconciliation.md`](implementation-review-fixture-reconciliation.md).
- Verification map: [`verification-matrix.md`](verification-matrix.md).
- Common-path receipt:
  `scale-02-slice40-common-20260905T1101Z-0dde4803`.
  The 95-percent upper write regression was 0.319% at p50 and 0.282% at p95;
  reopen regression was 7.931% / 3.826 ms; maximum checkpointed storage growth
  was 28,672 bytes. All registered bounds passed.
- Status receipt:
  `scale-02-slice40-status-20260905T1120Z-bacc3304`.
  Across five CPU and five CUDA-linked 50k runs, the maximum p95 difference was
  0.006181 ms; generation p95/p99 were at most 0.024426/0.042481 ms and
  mutation p95/p99 at most 0.026680/0.048111 ms. No steady full-owner scan,
  error, or timeout occurred.
- CUDA preflight witness SHA-256 `30d32467…`; Python allocation witness
  `edd276d1…`; N-API allocation witness `0214e71d…`. Both consumers selected
  `cuda:0` on RTX 3090 UUID
  `GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b` and observed a 128 MiB allocation.
- Fresh Linux CPU wheel SHA-256 `db7f4b5f…`, npm package `84189b38…`, and N-API
  platform package `e8866d2f…` passed installed status/reopen smokes. Fresh
  CUDA packages passed driverless CPU, forced-unavailable, and real-GPU smokes.
- Windows native Python 3.11.16 and Node 24.18 builds passed. The wheel SHA-256
  is `bb6d4d92…`; the N-API binary is `83c822a3…`. Fresh installed/native
  status and reopen smokes both passed and reported `blocked` / `absent` after
  reopening without a projection runtime.
- Full schema tests and the focused Engine reconciliation suite passed. The
  serial Rust workspace otherwise passed; its only sandbox failure was the
  expected ptrace denial, and the unchanged exact test passed unconfined, 1/1.
- A fresh test wheel with the declared test hooks, default embedder, and default
  reranker passed 1,403 Python tests with 28 explicit skips. The ordinary
  shipped wheel intentionally omits the optional reranker and was tested only
  against its actual release surface.

The status campaign compares the same SQLite status path in CPU and
CUDA-linked builds using a fixed CPU embedder. It is build-parity evidence, not
GPU-compute or embedding-throughput evidence. Actual CUDA selection and
allocation are separately witnessed. Vector latency remains assigned to Slice
75.

## Remaining closure work

Retain the preregistered descriptive CUDA embedding-throughput observation,
write the exact evidence manifest, and pass the final documentation/state
gate. Only then may release state advance to Slice 45.
