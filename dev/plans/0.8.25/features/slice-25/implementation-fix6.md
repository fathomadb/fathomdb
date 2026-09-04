---
title: 0.8.25 Slice 25 implementation FIX-6 response
status: FIX_IMPLEMENTED_AWAITING_REVIEW
review_cycle: 6
reviewed_commit: 79d296fa
red_commit: 8db99b12
green_commit: f9a65511
---

# Slice 25 implementation FIX-6 response

The release owner explicitly authorized this sixth FIX cycle. The failing
regressions were preserved in `8db99b12`; the correction is `f9a65511`.

## Corrections

- Rebuild sanitized TypeScript request objects with `Object.fromEntries`,
  preserving enumerable own `__proto__` data properties for strict native
  schema rejection. Top-level and nested regression tests assert exact paths
  and no write.
- Validate the complete top-level header, caller IDs, and raw operation count
  before translating any operation in both Python and N-API. The common Slice
  15 identity grammar remains sourced through `ArtifactRevisionId` validation
  rather than being copied into the bindings.
- Replace the rollback control's nonempty checks with exact pre-state and
  post-state comparisons. A known vector-committable kind now proves successful
  changes across canonical attributes, FTS v1/v2, provenance, dependencies,
  projection state/terminals, vector enrollment/rows, and receipt rows; every
  injected infrastructure failure restores the exact pre-state.

## Verification before review

- TypeScript SDK: 387/387 tests passed, including both `__proto__` and both
  precedence regressions.
- Fresh release wheel: 7/7 Slice 25 Python tests passed.
- Rust focused Slice 25 and schema suites: 76/76 tests passed.
- N-API and Python binding checks and TypeScript typecheck passed.
