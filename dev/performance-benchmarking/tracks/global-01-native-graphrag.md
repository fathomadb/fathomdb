# GLOBAL-01 — Native GraphRAG comparison

**Status:** active; A/A passed and the corrected witness is next.

## Decision

Does explicit `global_lazy_coverage_v1` improve global comprehensiveness,
diversity, and empowerment over `source_mapreduce_c_v1` without weakening
directness, attribution, lifecycle correctness, latency, or cost beyond its
registered limits?

## Plan

1. Write a dated measurement contract that freezes the qualified-question
   manifest, development and held-out split, treatment parameters, matched
   answer budget, scorer, A/A procedure and pass boundary, repetitions,
   uncertainty method, cost cap, and acceptance boundary.
2. Perform a zero-spend input and environment preflight of corpus hashes,
   split integrity, model routes, dependencies, and cost estimation.
3. Implement `global_lazy_coverage_v1` as an explicit caller-selected
   experimental profile. Keep A0 unchanged and create no persistent extracted
   facts or graph projection.
4. Complete the zero-spend runner preflight: receipt writing, incremental
   checkpoints, resume-only-missing behavior, bounded retry and backoff,
   completeness guards, `ReadView`, canonical attribution, and supersession
   and erasure canaries.
5. Return for cost and execution authorization with the preflight receipt and
   estimated witness and matched-run cost.
6. If authorized, execute the A/A validation, then run one development-set
   witness if it passes. Validate execution, attribution, lifecycle behavior,
   and budgets; do not tune against held-out outcomes.
7. If the witness is valid, run the matched held-out control and treatment.
   Use native GraphRAG only as an optional reference, not a tuning oracle.
8. Record the receipt and decision. Accept the profile only if every registered
   quality, attribution, lifecycle, latency, and cost boundary passes.

The dated
[lazy-coverage measurement contract](../2026-08-29-global-01-lazy-coverage-contract.md)
freezes step 1. Steps 2–4 passed in the
[zero-spend preflight](../2026-08-29-global-01-lazy-preflight-result.md), and the
[safe receipt](../../../experiments/runs/global-01-lazy-preflight-20260829T1841Z-6da51962/record.json)
is registered. Coreyt authorized execution on 2026-08-29. The projected spend
is $9.50 and the hard cap is $12.00.

The A/A gate passed. The first witness map stopped on deterministic output
truncation before held-out execution. The
[execution note](../2026-08-29-global-01-witness-execution-note.md) records the
bounded prompt correction and same-checkpoint resume.

## Stop

Stop on corpus or split drift, invalid A/A behavior, failed lifecycle canaries,
incomplete arms, a non-resumable runner, or the cost cap. A split or rejected
result is valid; do not tune indefinitely.

## Outcome

The [first-run result](../2026-08-29-global-01-first-run-result.md) is split.
Native GraphRAG led directionally on comprehensiveness, diversity, and
empowerment; source-linked map-reduce led on directness. That measured coverage
gap supports only a preregistered treatment. The
[performance-gap analysis and falsifiable improvement hypothesis](../2026-08-29-global-01-improvement-hypothesis.md)
provides that basis. The treatment is contracted and implemented as an
experimental caller-side profile, but it is not yet executed or accepted.
