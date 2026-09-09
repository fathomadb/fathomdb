---
title: 0.8.25 Slice 71 — status
status: COMPLETE_ON_RELEASE_BRANCH
slice: 71
updated: 2026-09-09
---

# Slice 71 status

Slice 71 is complete. The performance limits were not changed.

71B completed at `eda95b07`, recovering the write regression through
transaction-level visibility invalidation, prepared write/projection checks,
bounded projection batching, and a nonblocking drain deadline. Its original
results are in [71b-performance-recovery.md](71b-performance-recovery.md).

AC-072 completed at product commit `84c056c6` after RED `de8da0f9`. Bounded
diagnostics attributed the read failure to the canonical join and stable-ID
work across every FTS match plus quadratic body fusion. The correction keeps
the complete BM25-ranked text set, uses a collision-safe linear fusion index,
and defers identity/source hydration until final results only when the same
snapshot proves the default hybrid path has no dependencies or unsafe nodes.
All other modes retain the existing joined path. Candidate reduction, arm
removal, and threshold relaxation were rejected.

The exact 10k/384d/1,000-query `B,C,C,B,B,C` campaign is retained under
`dev/plans/runs/0.8.25-slice-71/ac072-final/`. Baseline repetitions measured
p50 164/161/162 ms and p99 173/169/171 ms. Candidate repetitions measured p50
69/69/69 ms and p99 76/75/77 ms. All environments were valid, both arms met
the 20% spread rule, and the final classification is `candidate_recovery`.
The execution binding records the runner and executable hashes and the actual
candidate checkout `4d8500d9`; its product source tree is identical to
`84c056c6`, with only the committed campaign manifest added between them.

The two candidate-only 71B guards also pass. Scale-02 10k acknowledgement was
1,361.080/1,368.250/1,378.400 ms and total was
1,365.443/1,372.627/1,382.779 ms, below the 1,543.539/1,548.545 ms guards.
Projection-active 10k total was 1,274.333/1,274.926/1,292.692 ms, below the
1,442.198 ms guard. Gating spreads were 1.28% or less for Scale-02 and 1.45%
for projection-active total. AC-013 acknowledgement remained non-gating.

Focused verification passed 49 direct blast-radius tests covering exact hybrid
ranking, deep overlap, duplicates, ownerless rows, fusion, lifecycle,
dependency closure, eligibility, and result shape. The changed library and
Slice 71 test pass clippy with warnings denied. Independent design/code review
passed on the exact correction, and the separate retained-evidence audit
passed. Broad verification rounds completed: zero. Slice 75 retains full
release verification; Slice 72 is now unblocked.
