---
title: FathomDB 0.8.27 Slice 85 - execution plan
status: IN_PROGRESS
target_release: 0.8.27
baseline: 8b2a9edaf
recovery_baseline: c64fad7b653d04bcc838d7441a04efc3da809bd2
---

# Slice 85 execution plan

## Current authority

The owner authorized the [recovery plan](../../../../../plan-slice-85-recovery.md)
on 2026-09-29. It supersedes the accumulated gate expansion and the former
closeout workflow. The [master plan](../../../plan-0.8.27.md) retains engine
ownership and AC27-85A/B/F/G while replacing C/D/E with the bounded contract.
Recover forward on `slice-85-fix`; do not reset to a historical review cycle.

The original accepted design, baseline and RED/GREEN receipts remain historical
in `design-review.md`, `baseline-verification.md` and `tdd-chronology.md`.
The original implementation baseline is `8b2a9edaf`; the accepted release state
still binds `7a2f9bf9`. Neither historical receipt qualifies the recovered head.
The similarly named 0.8.25 Slice 85 manifest checker/tests are outside scope.

## Recovery sequence

1. Reconcile design consumers; preserve module classifications, field ownership,
   named type admission, forbid floor, transitive root reach and cfg coverage.
2. Retire passive freezes, then receiver/resolver machinery and dependent tests
   in coherent checkpoints. Trim redundant mutants and keep cheap tests in fast;
   production qualification is explicit, outside fast/heavy/all.
3. Complete the two specified privacy/import cleanups, preserving all engine
   ownership moves, narrow errors and public mappings.
4. Perform the bounded remaining-engine phase: ownership/error/visibility and
   ADR compatibility review; deterministic teardown witness; benchmark request
   envelope fix; isolate the affected global hook; exact feature qualification.
5. Verify the exact candidate, review surviving interrupted changes once, and
   record one recovery receipt with evidence and explicit limits.

New behavior-bearing fixes require a staged or committed failing test first.
Retained tests are not weakened; removed tests specify retired machinery.
There is no new resolver, broad import/style sweep, feature cross-product or
Slice 90 runtime implementation. Only work rated at least 4/5 is admitted.

## Qualification and handoff

Follow the recovery plan's integrated qualification, including required workspace
checks, the standalone gate, explicit mutation qualification, named engine
feature routes, official public/hidden comparisons and candidate-bound native
receipt. Preserve acceptance baselines; review intentional internal deltas.
An unavailable executor or asset is unverified evidence, never a pass.

Independent review is bounded by the reduced contract and protected behavior.
The final candidate remains awaiting landing: do not update release bindings,
mark Slice 85 settled for Slice 90, push, tag or publish under this instruction.
Slice 150 alone retains exact-final-candidate live AC-037 qualification.
