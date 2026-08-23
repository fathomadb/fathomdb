# TEMPORAL-01 — Time-scoped retrieval

**Status:** input acquisition complete; requires a factual preflight and fixed
measurement contract.

## Decision

Do temporal filters and version-aware projections retrieve the correct state
for time-scoped and changed-fact questions without returning stale superseded
evidence?

## Draft plan

1. Bind the registered local TimeQA dev/test files and LongMemEval-S temporal
   slice to a content-free factual preflight. Keep TimeQA and LongMemEval
   results separate: they have different source and query shapes.
2. Fix a licensed local manifest of time-scoped, ordering, and changed-fact
   cases with canonical timestamps, source identity, and declared exclusions.
3. Compare the selected baseline with one temporal-filter/projection treatment.
   Report evidence recall, as-of and current-state accuracy, stale-hit rate, and
   latency separately by corpus and query class.
4. Accept only if time-scoped or changed-state quality improves without a
   provenance, supersession, or source-erasure violation.

## Stop

Stop after the fixed comparison or on inadequate gold. Do not broaden the
corpus or infer temporal correctness from aggregate LOCOMO accuracy.
