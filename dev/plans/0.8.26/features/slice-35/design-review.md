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
2. **P1 — affected-revision bound.** One derived edge can retire distinct G0
   and G11 revisions, requiring three affected revisions including the new
   edge. Resolved by specifying collection/deduplication, a three-per-operation
   formula, global bound 384, and exact-bound corruption tests.
3. **P1 — false binding precedence.** The draft did not match the current PyO3
   and N-API parse order. Resolved by recording the exact current sequence and
   requiring collision fixtures for every earlier-precedence family.
4. **P2 — underspecified measurements.** Resolved by defining logical/API call
   counts, timing boundaries, response-byte accounting, queue/lock labels,
   lifecycle slow events, projection settling, and checkpointed byte capture.
5. **P2 — inherited variants implicit.** Resolved by naming positive domain-
   effect controls for all four existing operations before the fifth is added.

The same read-only reviewer rereviewed the reconciled plan and design and
returned PASS. All five findings are materially resolved; implementation is
authorized.
