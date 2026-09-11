---
title: ADR-0.8.25-absolute-read-performance-successor
date: 2026-09-11
target_release: 0.8.25
desc: Retire AC-020's relative ratio and adopt absolute read-batch budgets plus an independent reader-progress witness.
blast_radius: acceptance; perf_gates; reader_pool tests; release verification
status: accepted (HITL seq-277)
---

# ADR-0.8.25 — Absolute read-performance successor

**Status:** accepted by owner ruling seq-277.

## Decision

Retire AC-020 without rewriting its historical failures. Its relative formula
penalized large sequential improvements by shrinking the concurrent limit even
when concurrent time also improved materially.

Register three successor assertions on the unchanged fixture:

- AC-081a: exactly 1,600 sequential searches complete in <=500 ms, with a
  non-blocking warning at >=200 ms.
- AC-081b: eight readers complete exactly 1,600 total searches in <=100 ms,
  with a non-blocking warning at >=80 ms.
- AC-081c: one real reader connection completes a search while a distinct
  reader on the same Engine holds a live SQLite snapshot.

Hard limits are inclusive and evaluated at full precision. Exactly 500 ms or
100 ms passes with a warning; one nanosecond above fails. The ratio remains
descriptive and cannot pass or fail acceptance.

## Verification

Slice 80 uses one hashed release test executable for seven fresh processes.
Every valid observation must pass both numeric limits. The deterministic
reader witness is correctness coverage, not a timing ratio. Slice 85 may reuse
the receipt when candidate and relevant-input identities match.

## Consequences

The shipping reader pool, SQLite runtime configuration, statement reuse,
query mix, operation counts, timing boundaries and result checks do not change.
Warnings are visible but nonblocking. The retired AC-020 helper and tests remain
compiled as ignored history and are absent from default `AGENT_LONG` execution.

AC-072/073/075/076 remain independently binding; these successors do not claim
production QPS, real-corpus fidelity or large-corpus latency.
