---
title: 0.8.24 Slice 0 — benchmark evidence index
status: COMPLETE
target_release: 0.8.24
---

# Benchmark evidence index

## Candidate sources

| Source | Nature | Slice 20 use |
| --- | --- | --- |
| `experiments/performance-0.8.23-plan-20260821` at `b3528b26` | Performance branch with retained SCALE-02 results and a production-engine delta headed by `c7e83bfe`. | Primary candidate for the requested engine-level adjustment. |
| `plan/0.8.24-performance-findings` at `7ade42de` | Evidence-conformance design/tests for performance collection, not an engine-performance result. | Candidate supporting infrastructure, not proof of an engine change. |
| `/home/coreyt/projects/fathomdb/dev/performance-benchmarking/` | Untracked historical competitor register, last observed 2026-08-15. | Context only; it cannot by itself provide a branch-anchored release change. |

## Measured engine candidate

The retained SCALE-02 rank-boundary result on the performance branch records
60 fresh-database repetitions at 25k/40k/50k records. It recommends the
streamed BM25 boundary-tie completion with shipped reader defaults:

- reported steady p50: 8.60/10.54/12.95 ms at 25k/40k/50k with default readers;
- reported exact top-100 and public top-10 equivalence to a forced full-sort
  control; and
- reported zero errors, timeouts, and full-sort fallbacks for streamed cells.

The branch’s implementation note records owner approval `seq-267` for the
`stream_default` production path and says no confirming benchmark is required.
Its engine changes are not on `origin/main`, so Slice 20 must treat them as a
separate integration decision—not copy individual commits opportunistically.

## Slice 0 disposition

The performance branch above is the concrete evidence source requested for
Slice 20. The proposed decision rule is: accept only the already-retained
result and its documented fidelity/resource constraints, then verify code
integration with targeted correctness tests; do not trigger a speculative
confirming benchmark. The owner must decide whether this approved-but-unmerged
branch belongs in 0.8.24.

## Evidence

- `experiments/performance-0.8.23-plan-20260821`:
  `dev/performance-benchmarking/2026-08-22-scale-02-rank-boundary-result.md`
  and `2026-08-23-scale-02-stream-default-implementation.md`.
- `git log origin/main..experiments/performance-0.8.23-plan-20260821 -- src/rust/crates/fathomdb-engine`.
- `plan/0.8.24-performance-findings:dev/plans/0.8.24-retrieval-performance-conformance-findings.md`.
