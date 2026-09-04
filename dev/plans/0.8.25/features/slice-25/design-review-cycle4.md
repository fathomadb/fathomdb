---
title: 0.8.25 Slice 25 independent design review — cycle 4
status: COMPLETE_FAIL
review_cycle: 4
reviewed_commit: 34ee0bde
reviewed_on: 2026-09-04
verdict: FAIL
---

# Slice 25 independent design review — cycle 4

## Verdict

**FAIL.** D25-16 and D25-17 are closed. Constructor ownership and source-ID
compatibility require one final correction, as do persisted refusal scalars.

| ID | Priority | Finding | Required correction |
| --- | --- | --- | --- |
| D25-18 | P1 | Rust constructor, binding decoder, Engine, and admitted-refusal errors are conflated; several lifecycle/nested paths are absent. | Define the four layers separately, exact reason/path mappings, and generation exhaustion attribution to the first true insert. |
| D25-19 | P1 | The source-ref SQL NUL check rejects a `SourceId` currently accepted by `SourceId::new`. | Preserve the landed grammar and test all boundary-valid classes, including embedded NUL. |
| D25-20 | P2 | Keyed replay does not validate a refused row's closed reason and required index/path relationship. | Validate every refusal scalar against the mapping table and add a corruption RED per rule. |

No product implementation began from design v6.
