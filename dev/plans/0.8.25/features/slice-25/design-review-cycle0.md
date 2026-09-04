---
title: 0.8.25 Slice 25 independent design review — cycle 0
status: COMPLETE_FAIL
review_cycle: 0
reviewed_commit: 6aab307e
reviewed_on: 2026-09-04
verdict: FAIL
---

# Slice 25 independent design review — cycle 0

## Verdict

**FAIL.** The initial scope-reconciled draft did not define an executable
contract against the landed Slice 15/20 engine. Five implementation-shaping
findings required correction before RED.

## Findings

| ID | Priority | Finding | Required correction |
| --- | --- | --- | --- |
| D25-01 | P1 | The public/wire contract omitted exact methods, types, parsing, refusal/error ownership, and compatibility. A second Engine method also conflicted with the accepted `PreparedWrite` one-entry-point statement. | Define every public type and SDK behavior; record a successor ADR or design a compatible extension. |
| D25-02 | P1 | Operations named nonexistent fact/derived variants and lifecycle states. The draft targeted revisions although landed `transition` targets logical IDs/current rows, and it both permitted and rejected forward references. | Narrow to landed provenance-complete node and dependency types, actual lifecycle states, revision-pinned logical transitions, existing same-logical-ID supersession, and one explicit ordering rule. |
| D25-03 | P1 | The transaction boundary and Slice 30 relationship were undefined. Existing write and transition methods own transactions while Slice 20 alone exposes a composition seam; the draft also referenced future projection-generation types. | Specify validator/apply seams, exact transaction/cursor publication flow, dependency-stage closure behavior, and only current projection substrate. |
| D25-04 | P2 | Receipt DDL, request digest, replay ordering, malformed-vs-refused behavior, corruption handling, and erasure privacy were unspecified. | Define schema step, canonical encoding, keyed terminal replay, crash boundary, lazy validation, and source-erasure behavior without an unbounded open scan. |
| D25-05 | P2 | Limits, error precedence, field paths, and RED coverage were not executable. | Pin operation/reference bounds, typed errors and refusal vocabulary/precedence, and tests for parsing, roles, ordering, lifecycle, supersession, races, corruption, notifications, projections, and privacy. |

The draft status also incorrectly remained blocked on completed Slice 7/20
work; this was a P3 metadata correction.

## Disposition

All findings are addressed by design version 3 at commit `f57d85f2`, which is
submitted for independent cycle-1 review. No product implementation began from
the failed draft.
