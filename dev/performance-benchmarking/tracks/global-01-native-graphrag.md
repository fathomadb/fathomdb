# GLOBAL-01 — Native GraphRAG comparison

**Status:** v3 preflight passed; fresh A/A and witness next.

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

The A/A gate passed. The witness then exhausted its semantic retry boundary
without a complete answer. The
[execution note](../2026-08-29-global-01-witness-execution-note.md) and
[receipt](../../../experiments/runs/global-01-lazy-witness-20260829T1924Z-aa159044/record.json)
record the stopped, decision-ineligible run.

The [v2 recovery contract](../2026-08-29-global-01-v2-recovery-contract.md)
removes the contradictory attribution encoding, preserves the measurement
design, and uses a fresh checkpoint. Its
[zero-spend preflight](../2026-08-29-global-01-v2-preflight-result.md) passed.
Coreyt authorized fresh A/A and witness execution on 2026-08-29 with a $12
hard cap. Held-out execution remains conditional on a valid witness.

Fresh A/A passed, but the v2 witness stopped after deterministic retries
repeated over-limit claims without validation feedback. The
[v2 execution note](../2026-08-29-global-01-v2-witness-execution-note.md)
records the invalid receipt and content-free retry correction. The repeat
preflight passed in the [fresh receipt](../../../experiments/runs/global-01-lazy-preflight-20260829T2053Z-483e11ad/record.json),
and the next attempt used a fresh witness root.

That fresh run completed all control maps after one validation-aware retry, but
its reduction attempts each reached the exact output ceiling and produced
truncated JSON. Output-limit-aware feedback is now implemented under a new
semantic revision. After zero-spend binding, resume the bound checkpoint and
retain its completed A/A and maps.

The [output-limit correction preflight](../../../experiments/runs/global-01-lazy-preflight-20260829T2059Z-483e11ad/record.json)
passed at zero spend.

The resumed reduction again reached exactly 1,500 tokens after explicit
shortening feedback. The [model-limit review](../2026-08-29-global-01-output-limit-research.md)
confirms that 1,500 was an undersized experiment ceiling. V3 raises both matched
reduction arms to 4,096, records the 393,216-token routed-model maximum, keeps
the projected run below $12, and requires a fresh preflight and checkpoint.

The [v3 preflight](../../../experiments/runs/global-01-lazy-preflight-20260829T2113Z-b0f3c328/record.json)
passed at zero spend under configuration `b0f3c328`.

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
provides that basis. The treatment remains unaccepted. V2 is reopened only
under its preregistered recovery contract; do not reuse the v1 witness as a
quality result.
