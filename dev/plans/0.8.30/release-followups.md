---
title: 0.8.30 receiving allocations — feedback and reset review
status: PROPOSED
target_release: 0.8.30
---

# 0.8.30 receiving allocations

These owner-selected follow-ups are allocations, not a new release plan, source
readiness or execution grant. Reconcile them into the actual .30 plan and current
source before implementation. Existing fork-maintenance scope remains separate.

| Allocation / authority | Requirement and acceptance | Sequence / limits |
| --- | --- | --- |
| Precise per-result feedback; TC-e70f4224-4d38-4932-9ef6-bf801d10c334; seq-375 | Review richer occurrence labels that identify the exact selected result within response/store/epoch, including repeated public refs. Select schema, text rendering, validation, paging and fetch distinction explicitly. Tests must prove exact attribution, label lifetime, both formats, unique/repeated refs and atomic validation without invented oracle snapshots. | The selected [.29 atomic ambiguity refusal](../0.8.29/features/slice-50/feedback-decision.md) must land and qualify in .29; this review cannot replace or postpone it. Genuine behavioral RED before GREEN and independent design/final review precede any selected new ABI. |
| Reset policy and live-fault review; TC-cab4c8d7-0ea1-426c-9166-22ebd4cc4fc9; seq-376/377 | Revisit quiescent completed-construction/frozen-native-live-set residue until restart, no post-loss growth, known accounting and opaque native host bytes without a hard cap. Review real Orin private-pool and selected3090 child reset tests for typed refusal, immediate close, last-clone drop, normal exit and fresh-process recovery. | First consume [.29 source/seam repairs](../0.8.29/features/slice-110/plan.md) and qualified pins. Actual faults remain HELD until concrete venue/session/resource authority. Partial-load real reset remains excluded pending separately reviewed handoff. |

Source/seam proof alone cannot claim real reset survival or close original
TC-281155a1. The original Orin defect requires actual Orin positive teardown proof;
3090 does not substitute. Proposed child caps remain120s after readiness,30min per
venue,8GiB GPU/16GiB owned host memory, subject to later concrete selection. Keep
healthy/OOM/CPU/driverless controls and preserve failed historical evidence.
No host-wide reset, driver/primary-toolkit change, K620, same-process recovery or
unowned resource cleanup follows from these allocations. New CUDA13 delivery and
quantitative evidence belong to [.31](../0.8.31/cuda-delivery-plan.md).
