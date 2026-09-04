---
title: 0.8.25 Slice 25 independent design review — cycle 2
status: COMPLETE_FAIL
review_cycle: 2
reviewed_commit: 59f51a19
reviewed_on: 2026-09-04
verdict: FAIL
---

# Slice 25 independent design review — cycle 2

## Verdict

**FAIL.** D25-07 is resolved and D25-09 is substantially resolved. Four
implementation-shaping gaps remain.

| ID | Priority | Finding | Required correction |
| --- | --- | --- | --- |
| D25-10 | P1 | Operation discriminants/shapes, lifecycle field types, exports, error conversion, and stored-corruption ownership remain incomplete. | Pin exact Rust and SDK shapes; map existing errors deterministically; keep corrupt persistence under `EngineError::Storage`. |
| D25-11 | P1 | Receipt/source-ref schema lacks complete version, outcome, privacy, bounds, and canonical-JSON invariants. | Version source refs; strengthen SQL/application checks; add corruption RED cases for each forbidden field. |
| D25-12 | P2 | The closure trigger omits legal `pending -> deleted`, and cross-operation failure attribution is unspecified. | Treat every live-to-non-live move alike and attribute the final guard to the earliest source-loss operation. |
| D25-13 | P2 | Reverse indexing makes erasure targeted but not bounded because one source can have unbounded receipts. | State indexed target-complete O(matches) behavior and test adversarial volume; defer chunking/fencing to Slice 30. |

P3 corrections require explicit Slice-25-only actuation coverage for the
temporary closure guard and exact mutation telemetry/replay behavior.

No product implementation began from design v4.
