# REASON-01 compact-ledger v2 result

## Outcome

Reject `protected_evidence_ledger_v2` and close the offshoot. Retain A0. The
complete diagnostic does not make an untouched confirmation, native
HippoRAG-2 comparison, or MEMORY-01 refresh eligible.

V2 corrected the v1 execution flaw: the evidence-empty case produced the
registered deterministic abstention without a reader call, and the run
continued through all 109 cases. The measured treatment still failed its
descriptive no-regression boundary on groundedness and attribution.

## Result

The frozen consumed cohort produced these paired compact-versus-A0 outcomes:

| Metric | A0 raw | Compact ledger v2 | Delta | One-sided 95% lower bound |
| --- | ---: | ---: | ---: | ---: |
| Answer accuracy | 54.13% | 56.88% | +2.75 points | -4.59 points |
| Groundedness | 66.06% | 56.88% | -9.17 points | -19.27 points |
| Attribution | 61.47% | 55.96% | -5.50 points | -15.60 points |

Compaction materially improved context efficiency: mean answer input fell from
13,611 to 1,407 characters, context precision increased from 14.68% to 52.72%,
and evidence utilization increased from 18.35% to 67.52%. Those gains did not
preserve the registered quality metrics.

Eight cases used valid deterministic abstentions. Six cases exhausted ledger
semantic repair and were recorded as terminal quality failures rather than
aborting the run. Their downstream correctness, grounding, attribution, and
citation results were false as preregistered. The six failures are propagated
stage outcomes from six cases, not separate failures at every downstream
stage.

The protected raw reference remained stronger on this cohort: 60.55% answer
accuracy, 66.97% groundedness, and 63.30% attribution. Its mean answer input
was 29,654 characters, so it does not supply the compact efficiency result.

## Execution

- Cohort: all 109 cases from the consumed LongMemEval-S REASON-01 cohort.
- Retrieval: unchanged frozen A0 and protected candidate sets.
- Route probe: the exact dynamic citation schema passed through isolated
  Airlock, OpenRouter, and DeepSeek V4 Pro before the benchmark began.
- Completion: 109 of 109 cases with atomic response-first checkpoints.
- Runner-accounted cost: $3.151915 under the frozen price sheet.
- Authoritative Airlock spend: $3.029448, including the route probe, under the
  $10 provider cap.

The difference between modeled and Airlock spend reflects their distinct
pricing inputs; both are retained rather than forcing them to agree.

## Interpretation

The compact ledger is a caller-side answer-context treatment. It did not
change FathomDB retrieval, storage, Engine, or SDK behavior. Its evidence
selection was much more efficient, but ledger generation was fragile (87
semantic retries and six exhausted cases), and the selected evidence did not
support equally grounded or attributable answers.

The v2 execution therefore converts v1's harness stop into a valid negative
treatment result. Do not promote this profile, tune it against the consumed
cohort, run native HippoRAG-2 from it, or refresh MEMORY-01 from it.

## Evidence

- [Content-free experiment receipt](../../experiments/runs/reason-01-compact-ledger-v2-20260830T2156Z-572f51ea/record.json)
- External checkpoint:
  `data/performance-benchmarking/locomo-multihop/runs/reason01-compact-ledger-v2-20260830-01/reason01-compact-ledger-checkpoint.v2.json`
  (`sha256:df68d4a417506e396135fb8e76d90251e53c5e312d7525072d21cd4b3c23f62a`)
- External receipt:
  `data/performance-benchmarking/locomo-multihop/runs/reason01-compact-ledger-v2-20260830-01/reason01-compact-ledger-receipt.v2.json`
  (`sha256:a92fbf33e7453c61149ce6d11ed198687a0521f648064a7a80324ea06fa077e4`)
- External summary:
  `data/performance-benchmarking/locomo-multihop/runs/reason01-compact-ledger-v2-20260830-01/reason01-compact-ledger-summary.v2.json`
  (`sha256:d8135e0d1c5d3f84d3a814eb7b81a6561a59c541c4522c4f705a6bfe30d19faa`)
- External schema-probe receipt:
  `data/performance-benchmarking/locomo-multihop/runs/reason01-compact-ledger-v2-schema-probe-20260830-01/reason01-dynamic-citation-schema-probe.v1.json`
  (`sha256:767871a92217d3ad24d5852111c22adde18fde44dcb4c79183a0f77c51c48ad1`)
- External Airlock spend checkpoint:
  `data/performance-benchmarking/locomo-multihop/runs/reason01-compact-ledger-v2-20260830-01/airlock-spend-state.v1.json`
  (`sha256:65d6cd40d38110ee51546967e8f86b5aa1a79303ec2c1e527226cbab3b908206`)
