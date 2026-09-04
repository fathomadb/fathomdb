---
title: 0.8.25 Slice 25 independent design review — cycle 3
status: COMPLETE_FAIL
review_cycle: 3
reviewed_commit: 15923b7b
reviewed_on: 2026-09-04
verdict: FAIL
---

# Slice 25 independent design review — cycle 3

## Verdict

**FAIL.** D25-01 through D25-09, D25-12, and D25-13 are closed. Two prior
areas remain partially open and one telemetry conflict is new.

| ID | Priority | Finding | Required correction |
| --- | --- | --- | --- |
| D25-14 | P1 | Rust constructors and exceptional/refusal mapping remain incomplete or contradictory. | Give exact signatures; preserve storage-corruption ownership; distinguish dependency-generation exhaustion; define every whole-batch path/index and RED case. |
| D25-15 | P1 | Corrupt/orphan/over-bound source-reference rows can evade replay and erasure validation. | Define an exact per-request cap and relevant-chain validator used by replay, purge, and erasure; reject malformed/orphan/erased-owner rows. |
| D25-16 | P2 | Receipt-array maxima and intra-operation affected-ID ordering remain unspecified. | Pin formulas/maxima, current outcome/closure consistency, and property proofs. |
| D25-17 | P2 | A returned refusal receipt was described as an operation failure, contradicting the accepted Finished/Failed lifecycle model. | Treat refusal consistently as a completed typed outcome or amend the lifecycle contract; cover conflict and erased-ID telemetry. |

No product implementation began from design v5.
