---
title: 0.8.25 Slice 60 implementation review — cycle 5
status: FAIL
review_cycle: 5
candidate: 46837b55a805ee979347fd0e2e3f28ece36a8ed2
---

# Slice 60 implementation review — cycle 5

## Verdict

**FAIL.** No P0 or P3 finding exists. Two P1 blockers and one P2 evidence defect
remain. Projection pre-mapping, persisted `Proving` exclusion, separate
erase/excise controls, candidate bounding, and prior focused behavior pass.

## Findings

1. **P1 — Persisted provenance validation was weakened for the fixture.**
   Canonical/link source-ID agreement and four dependency-chain equality checks
   were removed, contradicting Slice 20's fail-closed contract. FIX-5 must
   restore every check and use a contract-valid dependency chain; the injected
   nonterminal `Proving` barrier already preserves the row physically.
2. **P1 — Relevant-feature strict Clippy is red.** The graph read helper has nine
   arguments, and the corrected dependency fixture retains unused imports and a
   dead digest. FIX-5 must group test-only controls into a cfg-gated carrier and
   remove mechanical test debt through a separately audited test correction.
3. **P2 — The current-RSS witness measures only retained SQLite allocator
   bytes.** It misses Rust frontier/candidate peak memory and does not assert
   exact work for both 10,000-work arms. FIX-5 must use isolated-process current
   RSS around the live request, or accurately separate allocator-retention proof
   from an executable candidate-retention/peak-RSS witness, with exact work
   assertions for both arms.

## Required FIX-5 discipline

Commit executable RED additions before product fixes and preserve prior frozen
oracles except independently audited mechanical migrations. Then implement
GREEN and obtain independent review cycle 6. No packaging, registry work, tags,
or publication.
