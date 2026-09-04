---
title: 0.8.25 Slice 25 status
status: COMPLETE_ON_RELEASE_BRANCH
slice: 25
updated: 2026-09-04
---

# Slice 25 status

## Current state

Slice 25 is complete on `release/0.8.25`. The bounded caller-decided actuation
batch, compact terminal receipts, exact replay, typed refusal behavior,
source-linked erasure closure, and additive Rust/Python/TypeScript contracts
are implemented on additive schema step 29. Independent implementation review
passed at cycle 7 with no unresolved P1/P2 or material P3 finding.

## Evidence

- RED/FIX-6 commits: `8db99b12` / `f9a65511`.
- The binding RED tests were preserved; the rollback proof's invalid initial
  vector fixture and test-only GREEN repair are recorded in
  [`implementation-tdd-chronology.md`](implementation-tdd-chronology.md).
- Final independently reviewed implementation: `51f152a6`.
- Release-branch completion point: `131053da`.
- TypeScript SDK: 387/387 tests pass.
- Fresh release wheel: 7/7 Slice 25 Python tests pass.
- Focused Rust/schema Slice 25 suites: 76/76 tests pass.
- The release-state schema version and its authoritative Rust constant both
  report schema step 29.
- The corrected governed-surface pin suite passes against the exact approved
  42-member surface.
- Final independent review:
  [`implementation-review-cycle7.md`](implementation-review-cycle7.md).

## Next

Begin Slice 30 only after the between-slice memory checkpoint and compaction.
