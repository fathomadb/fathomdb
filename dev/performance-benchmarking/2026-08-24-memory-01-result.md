# MEMORY-01 result

## Decision

Pass. FathomDB A0 meets the registered near-parity rule against native Mem0
OSS on the official LOCOMO evidence-aware answer-scoring path.

| Class | Questions | Mem0 OSS | FathomDB A0 | FathomDB minus Mem0 |
| --- | ---: | ---: | ---: | ---: |
| Overall | 1,540 | 67.21% | 75.19% | +7.99 pp |
| Multi-hop | 282 | 82.62% | 74.47% | -8.16 pp |
| Temporal | 321 | 16.20% | 52.02% | +35.83 pp |
| Open-domain | 96 | 77.08% | 83.33% | +6.25 pp |
| Single-hop | 841 | 80.38% | 83.35% | +2.97 pp |

The one-sided 95% paired bootstrap lower bound for the overall FathomDB-minus-
Mem0 delta is +5.78 percentage points. The receipt uses 10,000 question-level
paired resamples with seed `20260814`.

## Read

- Retain A0 as the general memory profile. The overall comparison supports a
  better-than-near-parity claim under this fixed LOCOMO contract.
- Do not claim class-wide dominance. Mem0 leads the multi-hop class by 8.16
  points; that class is the concrete follow-up diagnosis.
- The aggregate multi-hop loss does not itself supply the bounded labelled
  failure set required to start GRAPH-01 or REASON-01.
- Total campaign spend recorded by the isolated Airlock state was $6.52,
  below the approved $20 ceiling. Both scoring services completed without a
  recorded retry, rate-limit, or server-error marker.

## Evidence

- [Paired decision receipt](../../experiments/runs/fathomdb-vs-mem0-locomo-comparison-20260824T2140Z-01e702be/record.json)
- [FathomDB retrieval arm](../../experiments/runs/fathomdb-locomo-official-seam-20260824T1309Z-3762b22a/record.json)
- [Native Mem0 retrieval arm](../../experiments/runs/mem0-oss-locomo-native-20260824T1325Z-9de95019/record.json)

Raw questions, evidence, generated answers, and judge explanations remain in
the access-controlled external output roots. The repository receipt retains
only aggregates and hashes.
