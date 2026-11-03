---
title: FathomDB 0.8.28 Slice 81 — status
status: PLANNED
target_release: 0.8.28
---

# Slice 81 status

Placed by owner on 2026-11-03 after Slice 80 and before Slice 90. This is the
bounded erasure-availability maintenance POC, not production erasure remediation.
Baseline: `3fc2df15e4dfadd67b9fb57e5e7cea3d62613ead` on `dev/0.8.28`.

| Work | State |
| --- | --- |
| [Plan](plan.md), [requirements/AC](requirements.md) | Planning approved by Astra-medium |
| [Experiment protocol](experiment-design.md), [test plan](test-plan.md) | Planning approved by Astra-medium |
| [Implementation design](design.md) | Draft design approved for POC planning; exact code seam remains a code-phase gate |
| POC implementation / RED-GREEN / measurements | NOT STARTED |
| Mechanism / actual-Engine availability / SLA | UNQUALIFIED |
| Shipping physical-erasure finding | OPEN; existing known-failing witness retained |
| Production mechanism, API/fence successor and numerical SLA | Not selected or approved |

Next: identify/review a safe actual-Engine composition seam, then
implement the approved bounded A/B protocol with TDD and independent verification.
C is conditional; historical/temporal backend work remains outside this slice.
No new worktree/branch was needed for planning. No source, tests, production schema,
public APIs, runtime semantics, release tag or publication changed.

Planning review: Astra-medium APPROVE on 2026-11-03 after the explicit idle,
replica/cell clock, shutdown-overshoot and percentile-completion clarifications.
No required corrections remain. Independent verification passed 43 local links,
five anchors, release order/generated views, unchanged Slice 80 state and matrix/
budget arithmetic; the scoped accounting correction also passed. These are
planning checks, not POC code tests or performance results.
