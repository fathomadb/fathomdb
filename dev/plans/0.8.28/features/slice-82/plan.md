---
title: FathomDB 0.8.28 Slice 82 — CH1 Phase 1
status: COMPLETE
target_release: 0.8.28
---

# Slice 82 — CH1 Phase 1: physical correctness and native cost

HITL approved and allocated this phase on 2026-11-03 after Astra approved the
[shared CH1 plan](../../../ch1-production-erasure-plan.md). This entry point
indexes that plan rather than duplicating requirements, design tasks or tests.
Implementation and qualification are complete; see [status](status.md) and the
reviewed [production design](design.md). Dependency: Slice 81; this phase precedes
Slice 83.

- Scope, requirements and acceptance: [Phase 1](../../../ch1-production-erasure-plan.md#phase-1--physical-correctness-and-native-cost-slice-82), CH1-1A–1F.
- Design and contract constraints: [shared contract](../../../ch1-production-erasure-plan.md#shared-contract-and-scope).
- RED/GREEN, measurements, Astra design/code reviews and independent verification:
  [verification plan](../../../ch1-production-erasure-plan.md#measurement-review-and-verification-plan).
- Exact production design and test mapping received Astra approval on 2026-11-03.
  Implementation, Astra reviews and independent qualification are complete;
  [Slice 83](../slice-83/plan.md) is next.
- Status is owned by [release state](../../../release-state-0.8.28.json) and the
  [release board](../../../runs/STATUS-0.8.28.md).

Phase 1 retains conservative admission while repairing physical correctness and native cost.
The [physical-erasure guarantee/API decision](../../../ch1-production-erasure-plan.md#end-of-slice-83-hitl-decision-physical-erasure-guarantee)
belongs to HITL at the end of Slice 83. A specific cleanup API remains under
consideration; no API or numerical SLA is selected by this allocation.

## Baseline reconciliation and execution

Baseline is `bdadf19f6` (2026-11-03). The shared plan was written at
`aab42d333`; the intervening commit allocated 82/83 and reconciled Slice 81,
without production changes. Review of assigned CH1 and draft allocations finds:

1. Slice 81 is experimental code only. Its post-native POC job cannot provide
   crash-safe production admission; retain its fixtures, merge algorithm and
   immutable results, replace its queue with native erasure bookkeeping.
2. Slices 60/70/80 shipped format-35 cursor-owned body FTS, vector spans/native
   eligibility and historical reads. Erasure must preserve authorized history;
   sanitation must merge existing FTS, not rebuild an active-only projection.
3. Native debt currently exists only for nonempty registered-dependent closures.
   Purge also admits it only inside that dependent loop. Extend native debt to
   every root operation without changing closure roots or affected-count semantics.
4. Native proof performs per-cursor scans of unindexed FTS cursor columns.
   Profile DELETE and zero-proof statements separately; optimize both if measured.
5. FTS default deletion leaves retained terms in all four registered families;
   `secure_delete=ON` alone does not establish FTS physical cleanup. The POC's
   negative-first/positive-continuation merge is the selected starting mechanism.
6. Telemetry queue lacks original-path authentication and tolerates malformed
   payloads; late materialized search emission can recreate erased identifiers.
   These are in-scope physical-completion defects, not Slice 83 availability work.
7. Offline recovery currently returns a no-op with no closure IDs. Extend it and
   doctor to root/retained-term debt. Existing format refusal and sink/WAL errors
   remain; no open-time automatic cleanup, new public API, scheduler or expiry.

Approve the allocated scope with the above concrete adjustments. CH1-1A–1F
remain the acceptance groups; the design maps each to executable tests and
measurements. No new entries in locked `dev/acceptance.md`.

Execution order: Astra design review; RED/GREEN durable admission and sanitation;
RED/GREEN original-sink/late-emission safety; profile baseline; RED/GREEN measured
native scan correction; focused existing regressions; paired qualification;
Astra code review and independent Astra verification; status update. Each
behavioral RED is committed before implementation. Tests remain read-only during
fix-to-spec. Use this existing development checkout serially; reviewers are
read-only. Scoped checks while iterating, one final full code-phase gate.

## Approved qualification adjustments

Astra reviewed two measured scope adjustments at closeout. Original shared Phase
1 requirement 1's detailed drain/mutex/checkpoint timers move to Slice 83; the
actual dominant SQL stage, merge-command cost and total native durations establish
the repair's purpose without claiming availability. The supplemental property
baseline exceeded its 600-second whole-process budget and remains incomplete,
not a passing or censored-native measurement. No retry or budget increase was
used. Three completed original-node pairs satisfy CH1-1F; completed edge/scale and
policy controls provide additional evidence. Full facts and limits are in status.
