# TEMPORAL-01 — Time-scoped retrieval

**Status:** planned; requires adequate temporal gold and a selected retrieval baseline.

## Decision

Do temporal filters and version-aware projections retrieve the correct state
for time-scoped and changed-fact questions without returning stale superseded
evidence?

## Draft plan

1. Fix a licensed, local manifest of time-scoped, ordering, and changed-fact
   cases from the CORPUS-01-qualified portfolio, with canonical timestamps,
   source identity, and explicit supersession.
2. Compare the selected baseline with one temporal-filter/projection treatment.
   Report evidence recall, as-of and current-state accuracy, stale-hit rate, and
   latency separately by query class.
3. Accept only if time-scoped or changed-state quality improves without a
   provenance, supersession, or source-erasure violation.

## Stop

Stop after the fixed comparison or on inadequate gold. Do not broaden the
corpus or infer temporal correctness from aggregate LOCOMO accuracy.
