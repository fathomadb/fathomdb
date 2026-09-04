---
title: 0.8.25 Slice 15 status
status: COMPLETE_ON_RELEASE_BRANCH
completed: 2026-09-03
implementation_review: PASS_CYCLE_4
verification: PASS
memory_checkpoint: COMPLETE
---

# Slice 15 status

## Outcome

Slice 15 is complete on `release/0.8.25`. Schema step 27 adds immutable
artifact-revision, source-version, and source-link registries without legacy
backfill. Additive Rust, Python, and TypeScript write contracts accept complete
canonical or derived provenance while the existing write, receipt, record,
provider, and search-hit shapes remain compatible.

The Engine validates source identity, UTF-8 byte locators, canonical hashes,
and closed versioned wire objects before mutation. Existing writes receive
deterministic internal incomplete identities. Purge and source erasure account
for both artifact-side and source-side provenance, including fail-closed raw-
corruption cases.

## TDD and review

- Initial RED `8042f046`; GREEN `cdf2c23d`.
- FIX-1 RED `9e6650ab` plus packaged-smoke coverage `2bacba40`; GREEN
  `8a7b7523`.
- FIX-2 RED `63094a78`; GREEN `507f8f0d`.
- FIX-3 RED `10b5a563`; GREEN `c6fbd0ad`.
- FIX-4 RED `38f8a7f5`; GREEN `9a53e26a`.
- Independent design review passed at cycle 3.
- Independent implementation review passed at cycle 4 with no P1/P2/P3
  finding.

The correction cycles closed pre-batch vector-enrollment mutation, raw
provenance-orphan erasure, cross-SDK encoding and discriminator precedence,
and deterministic multi-unknown-field reporting.

## Verification

- Schema step 27: 2/2 passed; Engine Slice 15: 11/11 default and 12/12
  operator; facade exports: 1/1.
- Rust lifecycle and erasure suites: 28/28 passed.
- Python Slice 15: 96/96; Python lifecycle/source-erasure: 15/15.
- TypeScript/N-API Slice 15 and FFI safety: 110/110; lifecycle/erasure suites
  passed with no skips.
- Migration lint, smoke-script structure (46 checks), native artifact runtime
  validation, workspace lint/typecheck, and strict security checks passed.
- The independent verifier passed at reviewed commit `0ae51705`, integrated as
  `9a53e26a`, with clean trees before and after.

The final fast aggregate reached successful lint, typecheck, migration, and
security phases but its silent test tail was interrupted without a verdict.
This is not claimed as a pass. FIX-1's earlier 103/103 aggregate and the exact
post-FIX focused matrices above provide the retained evidence. Windows runtime,
CUDA, live-model, and registry-publication routes were unavailable or N/A under
design v5; local package-smoke behavior and its structural gates passed.

## Between-slice memory checkpoint

Reusable Slice 15 lessons were compacted into the external FathomDB memory
store and its `MEMORY.md` index before Slice 20 was commissioned. Release facts
and complete review evidence remain in this repository.
