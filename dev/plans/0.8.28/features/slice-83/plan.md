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

## Slice 82 handoff

Reconcile [accepted Phase 1 results](../slice-82/status.md) before design. The
original 10k-node native bottleneck is now 1.229 seconds median rather than
271.090, with physical witnesses cleared; do not price availability from the old
scan cost. Detailed drain, writer wait/hold and checkpoint instrumentation was
explicitly deferred here from Phase 1 requirement 1. Integrated loaded work must
measure actual native admission/deletion, sanitation units, finalization, queued
wait and writer ownership separately. Phase 1's property baseline is incomplete
at a 600-second process cap, not a native-duration observation or speedup.
Ordinary policy controls compare candidate default/secure modes only; they do
not establish baseline-to-candidate ordinary-write regression or a service bound.
