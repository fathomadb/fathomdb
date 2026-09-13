---
title: FathomDB 0.8.26 Slice 35 — breaking V2 actuation spike
status: DRAFT
---

# Slice 35 plan — breaking V2 actuation spike

## Purpose

Resolve implementation uncertainty before Slice 40 by building and measuring a
bounded V2-only prototype under accepted
[`ADR-0.8.26-breaking-v2-actuation-and-fresh-database-boundary.md`](../../../../adr/ADR-0.8.26-breaking-v2-actuation-and-fresh-database-boundary.md).
This is an implementation spike with requirements, acceptance, design review,
RED/GREEN discipline, code review, independent verification, status, and
explicit prototype retain/discard disposition.

## Entry reconciliation

Before work, enumerate changes since this draft, the final D26-04 and D26-05
rulings, current actuation/schema code, allocated Slice 3–5 items, and completed
Slices 10–30. Adjust the plan without expanding beyond V2 actuation,
fresh-database opening, and directly affected verification.

## Requirements and acceptance

- Only V2 actuation executes. Static V1 request/receipt types and functional V1
  digest, replay, receipt, and integrity paths are absent.
- Dynamic/native V1-shaped ingress returns the approved loud V2 direction after
  discriminator-only recognition and performs no write.
- Fresh 0.8.26 bootstrap succeeds. One representative earlier database is
  refused before mutation; no historical migration matrix or converter exists.
- V2 implements the inherited operation capabilities plus derived edge in one
  atomic batch and one V2 receipt/replay contract.
- The final D26-04 endpoint policy and D26-05 receipt fields are exact,
  cross-binding, bounded, and model-free.
- V2 validates its own provenance, lifecycle, dependency, source-reference,
  erasure, projection, replay, and integrity invariants without interpreting
  earlier-version rows.

## TDD RED/GREEN plan

1. Obtain independent review of [`design.md`](design.md) and resolve findings.
2. Commit or stage visible RED tests for V2 digest, atomic graph unit,
   rollback, replay, concurrency, endpoint policy, receipt integrity, erasure,
   projection, V1-retired refusal, removed static surfaces, and old-database
   refusal.
3. Implement the smallest GREEN prototype. Do not retain V1 adapters or a
   historical migration branch to make tests pass.
4. Measure the three-operation Memex unit, a 128-operation edge-heavy batch,
   1,000 sequential calls, and eight concurrent callers. Compare with a fixed
   0.8.25 baseline only for characterization.
5. Record p50, p95, throughput, writer-lock and slow-event incidence, receipt
   size, projection cursors, and database growth.
6. Obtain independent code review and independent focused verification.

## Exit and handoff

Write `status.md` with exact RED/GREEN/review/evidence commits. State which
prototype changes Slice 40 may retain and which were discarded. Update the
release ladder with the completed position between Slices 30 and 40. Stop for
HITL if D26-04 or D26-05 is unresolved, V1 functionality remains, an earlier
database mutates, V2 integrity is weakened, or performance evidence indicates
an architectural change rather than a bounded implementation correction.
