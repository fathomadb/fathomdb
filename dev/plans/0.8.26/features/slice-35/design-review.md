---
title: FathomDB 0.8.26 Slice 35 independent design review
status: PASS
reviewed_on: 2026-09-15
---

# Slice 35 independent design review

The read-only design reviewer returned FAIL with five findings. No reviewer
edits were made.

1. **P1 — schema-33 contradiction.** A schema-33 candidate could not refuse a
   representative schema-33 earlier database. Resolved by parameterizing the
   read-only classifier with prototype schema 34 and proving fresh bootstrap
   with a test-only no-op step 34. The production step/cutover remains Slice 40.
2. **P1 — affected-revision bound.** The first resolution assumed one active
   G11 revision and proposed 384. Code-grounded review then proved G11 may retire
   many coexisting body-null regular edges and that schema 33 enforces 256. The
   final design retains 256, removes the false per-operation formula, and
   detects the exact one-over case during rollback-only semantic simulation.
3. **P1 — false binding precedence.** The draft did not match the current PyO3
   and N-API parse order. Resolved by recording the exact current sequence and
   requiring collision fixtures for every earlier-precedence family.
4. **P2 — underspecified measurements.** Resolved by defining logical/API call
   counts, timing boundaries, response-byte accounting, queue/lock labels,
   lifecycle slow events, projection settling, and checkpointed byte capture.
5. **P2 — inherited variants implicit.** Resolved by naming positive domain-
   effect controls for all four existing operations before the fifth is added.

The same read-only reviewer rereviewed the first reconciliation and returned
PASS. After the G11 correction it temporarily required a read-only capacity
preflight and an executable source-reference bound. Code review then showed
that the preflight masked earlier semantic failures and that a fixed reserve
weakened receipt integrity. The final design instead validates and applies in
request order inside the rollback-only savepoint, refuses the exact 257th
affected revision with no committed effect, and validates source references at
`min(1024, 8 * operation_count + 2 * affected_revision_count)`. The final
rereview verdict is recorded below.

The final read-only rereview returned PASS with no material findings. It
confirmed that the plan, design, capacity implementation, receipt-dependent
source-reference formula and tests, fixed-order accumulated-state benchmark,
completion-spread calculation, and performance report are mutually aligned.

Canonical verification later exposed that the original Slice 25 fixture was a
sealed 0.8.25 release-evidence input. The amended design restores that fixture
byte-identically and assigns current edge conformance to a new Slice 35 fixture
and separate TypeScript test, leaving the retained installed-smoke inventory
unchanged. A final read-only integration rereview returned PASS and confirmed
the historical/current boundary across Rust, Python, TypeScript, and the
Python fixture override.
