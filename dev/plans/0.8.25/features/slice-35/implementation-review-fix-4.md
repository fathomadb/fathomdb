---
title: Slice 35 implementation review — FIX-4
status: RESOLVED_PENDING_FINAL_REVIEW
date: 2026-09-05
---

# Slice 35 implementation review — FIX-4

The first two preregistered legacy-search campaigns did not satisfy the strict
three-percent p95 upper-bound policy. Both failed receipts remain immutable:

- v1 `scale-02-slice35-20260905T0019Z-3756fde2`: p50 upper 2.426%, p95 upper
  3.827%; and
- v2 `scale-02-slice35-20260905T0031Z-864f6c61`: p50 upper 2.448%, p95 upper
  30.545%, including one host-pressure-contaminated candidate repetition.

The correction preserved the failure evidence, added a bounded
`SearchReaderRequest` payload after a committed 224-byte RED guard, built exact
candidate artifacts, and preregistered a fresh comparison without reclassifying
either prior run. The v3 receipt
`scale-02-slice35-20260905T0047Z-a934bd34` passed: p50 upper 1.656%, p95 upper
1.983%, with zero errors and zero timeouts. Its candidate product commit is
`1dfe0a166d15168a3bdf31830967c034d3dcc477`.

The campaigns also exposed a separate bulk-ingest regression signal. It is not
inside the registered legacy-search claim and is now an explicit Slice 75
release investigation with a stop threshold. No visibility trigger or frozen-
read safety invariant was weakened during Slice 35.
