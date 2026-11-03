---
title: Slice 84 — cleanup performance regression draft
status: PROPOSED
target_release: 0.8.28
---

# Slice 84 — cleanup performance regression

Prepared at HITL's request for separate execution after Slice 83. No production
implementation or benchmark execution is performed by this planning task.

| Document | Purpose |
| --- | --- |
| [Concrete plan](plan.md) | Baseline reconciliation, S84 requirements/acceptance, RED/GREEN sequence, fixed paired protocol and verification |
| [High-level design draft](design.md) | Source-grounded candidate changes, invalidation/proof boundaries and pre-code design obligations |
| [Slice 83 findings](../slice-83/status.md) | Safety/progress evidence, loaded regression and unresolved gate status |

The live release state remains authoritative. Slice 84 is a draft follow-up,
proposed before Slice 90. Confirm Slice 83 completion and its final code before
running. Its physical-erasure guarantee/API decision stays at the end of Slice 83;
Slice 84 results will be brought back separately.

Astra approved this plan/high-level draft on 2026-11-03 with one protocol
clarification, now applied; see the [review record](plan.md#review-and-handoff).
The concrete production design still requires its specified pre-code review.
