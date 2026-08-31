# GLOBAL-01 first-run result

## Result

The registered result is **split**. Native Microsoft GraphRAG led directionally
on global coverage-oriented measures; the FathomDB source-linked map-reduce
treatment was substantially more direct.

| Metric | FathomDB win rate | Clustered 95% interval |
| --- | ---: | ---: |
| Comprehensiveness | 0.3750 | 0.0000–0.7500 |
| Diversity | 0.3750 | 0.0000–0.7500 |
| Empowerment | 0.3875 | 0.0375–0.7500 |
| Directness | 0.9688 | 0.9062–1.0000 |

The run used eight global questions, five order-swapped judge repetitions, and
80 judgments per metric. The small question count produces wide
question-clustered intervals. It does not satisfy the registered near-parity
boundary for the three headline metrics and does not support a general win
claim.

## Validity and cost

- The native GraphRAG witness passed with complete reports for every generated
  community and two non-empty witness answers.
- Both comparison arms used `deepseek-v4-pro` with thinking disabled. The judge
  remained the independently registered `claude-haiku` alias.
- Three malformed judge objects were cost-checkpointed and retried. One earlier
  malformed object was charged at a conservative upper bound.
- Registered run cost was $1.5335 against the $6.00 cap. Provider setup probes
  and interrupted compatibility diagnosis are outside this measurement cost.
- The receipt is
  [`global-01-native-comparison-20260829T1613Z-40685e82`](../../experiments/runs/global-01-native-comparison-20260829T1613Z-40685e82/record.json).

## Decision

Close this first run as complete and limited. Preserve native GraphRAG as the
global-coverage reference and the source-linked map-reduce treatment as the
directness reference. Do not tune this sample post hoc. Reopen GLOBAL-01 only
with a preregistered hierarchical-summary or graph treatment prompted by this
named coverage gap.
