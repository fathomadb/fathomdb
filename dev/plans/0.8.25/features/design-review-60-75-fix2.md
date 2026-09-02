---
title: 0.8.25 Slice 70 design review FIX-2 resolution
date: 2026-09-01
cycle: FIX-2
status: RESOLVED_PENDING_REREVIEW
review_source: dev/plans/0.8.25/features/design-review-60-75-cycle1.md
---

# Slice 70 FIX-2 resolution

## Scope

This cycle resolves only C1-70-01 and C1-70-02. The other seven cycle-0
findings remain closed by FIX-1. Slice 70 remains `DRAFT_REVIEW`, cannot become
READY before Slice 7 activates architecture v2, and still depends on Slice 65.

## Resolution

| Finding | Resolution | Added proof |
| --- | --- | --- |
| C1-70-01 | Slice 70 now uses only Slice 65 states: retrieval-gated treatments may install as `qualified_retrieval_only`; context-changing treatments require correctness, groundedness, and attribution no-regression before `qualified_answer`. Answer use rejects retrieval-only profiles. Release 0.8.25 has no default promotion path, and omitted profile remains compiled A0. | Dependent-contract fixtures reject removed `qualified_opt_in`, default entries, A0 aliases/shadowing, and answer use of retrieval-only temporal or associative profiles. |
| C1-70-02 | PPR is now an exact mass-conserving integer algorithm. The design defines ranked/unranked seed weights, one canonical checked-`u128` apportionment function, exact restart/follow budgets, global follow allocation across nodes, complete per-node outgoing allocation, exact dangling redistribution through the seed vector, canonical remainder domains, and convergence only after the vector sums to `M`. | Hand-computable fixtures cover uneven out-degree, multiple dangling nodes, seed weights that do not divide `M`, exact restart/follow accounting, per-iteration mass conservation, canonical remainder ties, convergence, reopen, and result digests. |

## Replacement record

- `fix2-60-75/slice-70-design.md`

Re-review should confirm only these two corrections and their mapped
RED/GREEN cases. Any unresolved P1/P2 finding continues to block READY.
