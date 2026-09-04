---
title: 0.8.25 Slice 15 design review — cycle 1
status: COMPLETE
review_cycle: 1
reviewed_on: 2026-09-03
verdict: FAIL
---

# Slice 15 independent design review — cycle 1

## Verdict

**FAIL.** Legacy migration and revision ownership are corrected, but six
implementation-shaping details remain.

## Findings

| ID | Priority | Finding | Required correction |
| --- | --- | --- | --- |
| D15-07 | P1 | A required field on existing variants is breaking and conflicts with the accepted prepared-write shape. | Add versioned entity newtypes/variants; preserve existing variants as incomplete legacy writes. |
| D15-08 | P1 | Revision receipt cardinality is undefined for mixed canonical/operational batches. | Return one optional revision per input item and define SDK encoding. |
| D15-09 | P1 | Canonical/derived node/edge roles and null edge-body hashing are ambiguous. | Make canonical sources node-only; define derived edges and domain-separated null-body encoding. |
| D15-10 | P1 | Persisted versioning, integrity enforcement, and erasure order are unspecified. | Add schema versions/checks and choose explicit Engine-enforced integrity and transaction order. |
| D15-11 | P2 | `SourceLocator` and optional-ID wire encodings are incomplete. | Define exact tagged snake/camel objects and one omission rule. |
| D15-12 | P2 | The proposed top-level error struct contradicts the accepted error taxonomy. | Define a subsystem `ProvenanceError` wrapped by `EngineError::Provenance`. |

D15-02 through D15-04 are resolved. D15-01, D15-05, and D15-06 remain
partially open through the findings above.
