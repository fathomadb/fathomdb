# REASON-01 held-out eligibility result

## Decision

Reject `protected_multiquery_v1` for promotion and stop REASON-01 before the
native HippoRAG-2 comparison. The profile materially improved supporting-session
retrieval and met its latency/context limits, but failed the frozen answer,
groundedness, and attribution boundaries.

## Registered measurement

- Cohort: all 109 untouched LongMemEval-S multi-session cases.
- Runtime: exact FathomDB 0.8.23 build with CUDA embedding and reranking.
- Arms: A0 FTS-10 and `protected_multiquery_v1` context-20.
- Answerer: DeepSeek V4 Pro, thinking disabled, temperature zero.
- Judge: Claude Haiku, temperature zero.
- Spend: $2.6626 against a $5 hard cap.
- Uncertainty: 10,000 paired case-level bootstrap draws, seed 20260830.

## Results

| Measure | A0 | Protected | Paired result |
| --- | ---: | ---: | ---: |
| Fractional gold-session recall | 0.8610 | 0.9391 | +0.0781; one-sided 95% lower +0.0526 |
| Any-gold session rate | 0.9817 | 0.9908 | descriptive |
| All-gold session rate | 0.6881 | 0.8440 | descriptive |
| Answer accuracy | 0.2569 | 0.2294 | −0.0275; one-sided 95% lower −0.0826 |
| Grounded rate | 0.2661 | 0.2294 | −0.0367 |
| Attribution rate | 0.2569 | 0.2018 | −0.0550 |
| Citation-contract validity | 1.0000 | 1.0000 | pass |
| Cold retrieval p95 | 1.69 ms | 81.02 ms | protected limit 100 ms: pass |
| Steady retrieval p95 | 1.49 ms | 66.81 ms | protected limit 75 ms: pass |
| Maximum context items | 10 | 20 | protected limit 20: pass |

The retrieval hypothesis is supported, but the treatment is not eligible: its
answer-correctness lower bound is below zero and both groundedness and
attribution regress. No held-out tuning is permitted.

## Execution correction

The first paid attempt stopped after Claude Haiku returned three prose/truncated
objects for one judgment. All responses and charges were already checkpointed.
The v2 execution configuration strengthened only the JSON transport instruction,
raised the judge output allowance from 180 to 400 tokens, and allowed two more
semantic-shape attempts. A typed one-time checkpoint rebind preserved the 218
retrieval cells, three answer cells, two completed judgments, and all prior
charges. It did not change retrieval, inputs, models, scoring definitions, or
outcomes. The final checkpoint contains 218 answer results, 218 judgment results,
218 answer attempts, and 221 judgment attempts.

## Evidence

- External receipt:
  `data/performance-benchmarking/locomo-multihop/runs/reason01-heldout-20260830-01/reason01-heldout-receipt.v1.json`
  (`sha256:ebaa82efd2636f0f9e8cda82ecddb6f4fcd7ed1d87d5a13f1284a8a927f09e64`).
- Safe summary:
  `data/performance-benchmarking/locomo-multihop/runs/reason01-heldout-20260830-01/reason01-heldout-summary.v1.json`
  (`sha256:3f3d696b93af584f5aff7372c7939605b540baea8f5f3c2a4e64a5bf511033bf`).
- External checkpoint:
  `data/performance-benchmarking/locomo-multihop/runs/reason01-heldout-20260830-01/reason01-heldout-checkpoint.v1.json`
  (`sha256:0caeb0c357fb97398a9e3a70edca214254d158dac27753346bcdbfa02c6b5170`).

The unrelated npm-recovery assertions and expired CUDA release-candidate
manifest remain known repository debt. They did not affect this measurement.
