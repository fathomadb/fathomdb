# GLOBAL-01 — Native GraphRAG comparison

**Status:** complete; treatment rejected.

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

The first paid A/A response then exposed a local type error: the client mixed
string `finish_reason` with numeric usage values before checkpointing. No A/A
cell, witness answer, or held-out work completed. The fixed client validates
only numeric token fields and persists finish reason separately. Use a fresh
preflight and artifact root.

The [post-fix preflight](../../../experiments/runs/global-01-lazy-preflight-20260829T2117Z-b0f3c328/record.json)
passed at zero spend.

The [v3 witness](../../../experiments/runs/global-01-lazy-witness-20260829T2118Z-b0f3c328/record.json)
then reached 4,096 tokens with `finish_reason=length` on all three control
reductions. Do not raise the task ceiling again before removing avoidable
serialization overhead. Use short local source, mapped-claim, and final-claim
references in reduction output; restore and validate canonical identities in
the caller before persistence.

V4 implements that adapter while retaining the 4,096-token matched ceiling.
Unknown references, duplicate references, missing coverage rows, invalid
dispositions, and unlinked final claims fail closed. Canonical source IDs and
hashes are restored before the existing structured-answer validator and
checkpoint persistence.

The [v4 preflight](../../../experiments/runs/global-01-lazy-preflight-20260829T2129Z-62bb47c3/record.json)
passed at zero spend.

The [v4 witness](../../../experiments/runs/global-01-lazy-witness-20260829T2130Z-62bb47c3/record.json)
confirmed that compact reduction works, producing two complete witness answers.
The run then stopped before held-out work because all three assertion-scorer
attempts reached its hardcoded 700-token ceiling. The routed Claude Haiku 4.5
model supports 64,000 output tokens. Register a 2,048-token scorer ceiling,
which raises the conservative full-run projection to $11.72 under the existing
$12 cap. The [v5 preflight](../../../experiments/runs/global-01-lazy-preflight-20260829T2142Z-da326e0b/record.json)
passed at zero spend. The [v5 witness](../../../experiments/runs/global-01-lazy-witness-20260829T2144Z-da326e0b/record.json)
confirmed normal scorer completion below 2,048 tokens, but exact-shape
validation rejected extra explanatory top-level fields. Register a deterministic
required-field projection. The [v6 preflight](../../../experiments/runs/global-01-lazy-preflight-20260829T2150Z-52b3aafe/record.json)
passed at zero spend. The [v6 witness](../../../experiments/runs/global-01-lazy-witness-20260829T2151Z-52b3aafe/record.json)
validated scorer projection and completed one question's scoring and judging.
It then exhausted generic retries on a map batch that alternated between an
uncited claim and an overlong claim. Bind targeted, content-free correction
instructions for those existing validators, then preflight one fresh witness.
The [v7 preflight](../../../experiments/runs/global-01-lazy-preflight-20260829T2158Z-60b3642c/record.json)
passed at zero spend. Its three-question witness passed for $0.703. The
[39-question held-out comparison](../../../experiments/runs/global-01-lazy-coverage-20260829T2159Z-60b3642c/record.json)
then completed within the authorized cap and rejected the treatment.

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
provides that basis. The registered
[held-out result](../2026-08-29-global-01-lazy-coverage-result.md) rejects
`global_lazy_coverage_v1`: grounding, directness, attribution, lifecycle,
cost, and latency passed, but headline pairwise quality and assertion recall
failed. Retain `source_mapreduce_c_v1_fts50` and do not tune the treatment
against the held-out outcomes.
