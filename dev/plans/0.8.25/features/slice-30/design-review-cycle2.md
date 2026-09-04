---
title: 0.8.25 Slice 30 independent design review — cycle 2
status: COMPLETE
review_cycle: 2
reviewed_commit: d5e17e04
verdict: CHANGES_REQUIRED
---

# Slice 30 independent design review — cycle 2

## Verdict

**CHANGES_REQUIRED.** D30-01, D30-02, D30-04, D30-05, D30-06, and
D30-07 are closed. D30-03 is improved but incomplete, and three additional
implementation-shaping gaps remain.

| ID | Priority | Finding | Required correction |
| --- | --- | --- | --- |
| D30-03 | P1 | Post-crash physical proof cannot recover the erased dependent identity set from a count alone. | Commit the complete structural zero proof atomically with destructive mutation and define the limited at-rest rechecks. |
| D30-08 | P1 | Open-time finalization cannot discharge telemetry without its caller-attached sink and has no `EngineOpenError` mapping. | Open validates and preserves barriers; allow recovery configuration and exact root retry while blocking unsafe writes. |
| D30-09 | P2 | The plan's derived-write guard is absent from ordinary and actuation provenance-write semantics. | Add shared provenance admission, typed reasons, actuation mapping, precedence, and RED coverage. |
| D30-10 | P2 | The closure-sequence singleton lacks canonical parsing, exhaustion, and `>= MAX` open validation. | Reuse Slice 20's generation-singleton invariants and tests. |

The review requests no deferred multi-source, recursive, or public recovery
surface.
