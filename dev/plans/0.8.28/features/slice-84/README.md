---
title: Slice 84 — cleanup performance regression
status: IN_PROGRESS
target_release: 0.8.28
---

# Slice 84 — cleanup performance regression

Owner-authorized execution after the completed Slice 83 baseline. The reviewed candidate completed the fixed bounded protocol and failed
qualification. Original scheduling is restored; final verification is pending.

| Document | Purpose |
| --- | --- |
| [Concrete plan](plan.md) | Baseline reconciliation, S84 requirements/acceptance, RED/GREEN sequence, fixed paired protocol and verification |
| [High-level design draft](design.md) | Source-grounded candidate changes, invalidation/proof boundaries and pre-code design obligations |
| [Status](status.md) | Negative qualification, rejection, verification and remaining regression |
| [Slice 83 findings](../slice-83/status.md) | Safety/progress evidence, loaded regression and unresolved gate status |

The live release state allocates Slice 84 before Slice 90. The broader
physical-erasure guarantee/API decision remains open; this slice retains the
accepted physical-success semantics. See the plan's execution reconciliation
and design's selected mechanism for changes from the original draft.
