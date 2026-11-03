---
title: FathomDB 0.8.28 Slice 83 — CH1 Phase 2
status: ACTIVE
target_release: 0.8.28
---

# Slice 83 — CH1 Phase 2: resumable work and availability

HITL approved and allocated this phase on 2026-11-03 after Astra approved the
[shared CH1 plan](../../../ch1-production-erasure-plan.md). This entry point
indexes that plan rather than duplicating requirements, design tasks or tests.
Implementation is NOT_STARTED. Dependency: Slice 82; this phase precedes
Slice 90.

- Scope, requirements and acceptance: [Phase 2](../../../ch1-production-erasure-plan.md#phase-2--resumable-work-and-availability-slice-83), CH1-2A–2F.
- Design and contract constraints: [shared contract](../../../ch1-production-erasure-plan.md#shared-contract-and-scope).
- RED/GREEN, measurements, Astra design/code reviews and independent verification:
  [verification plan](../../../ch1-production-erasure-plan.md#measurement-review-and-verification-plan).
- Next action: reconcile the baseline and write the exact phase production design,
  persisted-state/contract changes and test mapping; obtain Astra review before code.
- Status is owned by [release state](../../../release-state-0.8.28.json) and the
  [release board](../../../runs/STATUS-0.8.28.md).

Phase 2 qualifies native resumability and availability after Phase 1; it preserves existing physical-success meanings.
The [physical-erasure guarantee/API decision](../../../ch1-production-erasure-plan.md#end-of-slice-83-hitl-decision-physical-erasure-guarantee)
belongs to HITL at the end of Slice 83. A specific cleanup API remains under
consideration; no API or numerical SLA is selected by this allocation.
