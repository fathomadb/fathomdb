---
title: 0.8.25 Slices 60/65/70/75 final independent design re-review
date: 2026-09-01
cycle: 2
reviewer: independent Codex design review
verdict: PASS
scope: read-only re-review; no design or implementation edits
---

# Final independent design re-review: Slice 70 FIX-2

## Verdict

**PASS.** C1-70-01 and C1-70-02 are fully resolved. No new P1/P2 finding was
identified. Together with the seven findings closed in cycle 1, all nine original
Slices 60/65/70/75 findings are closed at the design-review gate.

## Closure

### C1-70-01 — CLOSED

Slice 70 now consumes the exact Slice 65 state model:

- a retrieval-gated treatment may install only as
  `qualified_retrieval_only`;
- a context-changing treatment requires registered correctness, groundedness,
  and attribution no-regression before `qualified_answer`;
- `intended_use=answer_context` rejects retrieval-only temporal and associative
  profiles; and
- 0.8.25 has no default-promotion route: omitted profile is compiled A0, while
  `qualified_opt_in`, default entries, A0 aliases, and shadowing reject.

The normative contract is at
[slice-70/design.md:169](slice-70/design.md), and its dependent-contract RED/GREEN proof is mapped at
[slice-70/design.md:220](slice-70/design.md). It no longer contradicts Slice 65 or reintroduces an implicit default change.

### C1-70-02 — CLOSED

The fixed-point PPR algorithm is now dimensionally complete and mass-conserving:

- the seed vector is produced by one canonical checked-`u128` apportionment
  function that returns exactly the requested total;
- each iteration partitions `M` into exact restart and follow budgets;
- the follow budget is globally apportioned by current node mass before each
  non-dangling node's allocation is apportioned over canonical outgoing edge
  revisions;
- complete dangling follow allocations are apportioned through the seed vector;
  and
- the next vector must sum to exactly `R + F = M` before convergence testing.

The normative arithmetic is at
[slice-70/design.md:105](slice-70/design.md) and
[slice-70/design.md:115](slice-70/design.md). The mapped proof now covers uneven out-degree, multiple dangling nodes, non-dividing seed weights, canonical remainder domains, exact per-iteration conservation, convergence, near ties, and stable digests
([slice-70/design.md:216](slice-70/design.md)).

## Boundary and scope confirmation

- Semantic intent, temporal interpretation, truth selection, answer reasoning,
  and model/provider choice remain external.
- TEMPORAL external quality remains blocked without source-derived gold.
- GRAPH-01 and REASON-01 rejected treatments remain historical and cannot be
  renamed or promoted.
- Omitted-profile A0 behavior and the Slice 65 registry authority remain intact.
- Slice 70 remains `DRAFT_REVIEW` and dependency-gated; this PASS closes design
  review only and does not assert implementation or verification completion.
