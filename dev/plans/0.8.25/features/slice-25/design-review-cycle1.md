---
title: 0.8.25 Slice 25 independent design review — cycle 1
status: COMPLETE_FAIL
review_cycle: 1
reviewed_commit: f57d85f2
reviewed_on: 2026-09-04
verdict: FAIL
---

# Slice 25 independent design review — cycle 1

## Verdict

**FAIL.** Design v3 resolved the five cycle-0 findings at the architectural
level, but four narrower implementation-shaping gaps remained.

## Findings

| ID | Priority | Finding | Required correction |
| --- | --- | --- | --- |
| D25-06 | P1 | `ActuationError`, its `EngineError` relationship, binding mappings, exact wire shapes, optional rules, and decimal-string encoding were undefined. | Define the root error ownership, reason/code/path payload, binding classes, exact wire rules, and canonical decimal strings in both Python and TypeScript. |
| D25-07 | P1 | The digest could not encode signed validity timestamps and assigned no exact field/variant tags. | Specify the complete canonical binary encoding and require a comprehensive shared fixture with negative time, Unicode, both provenance roles, dependency, lifecycle, and optionals. |
| D25-08 | P1 | Receipt DDL excluded the reserved Slice 30 outcome, lacked conditional checks and a reverse erasure index, relied on disabled foreign keys, and did not cover resolved lifecycle/purge references. | Make the schema forward-compatible now; add constraints/reverse index/explicit deletion; collect named, created, resolved, and implicit references; cover source erasure and purge with bounded query-plan tests. |
| D25-09 | P2 | Ordered validation missed a dependency registered after the operation that made its source non-current/non-live. | Run a final prospective closure-safety pass over persisted and all prospective dependencies; test both orderings and replacement-target behavior. |

P3 corrections also require exact pending-state transition scope,
persisted-state error classification, lifecycle-only projection draining, and
the existing **global write cursor** terminology.

## Disposition

The next design revision must close D25-06 through D25-09 and the P3 items
before cycle-2 review. No product implementation began from design v3.
