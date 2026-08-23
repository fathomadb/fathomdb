# TEMPORAL-01 — Time-scoped retrieval

**Status:** factual preflight complete; comparison blocked on a validity-window
manifest and temporal adapter.

## Decision

Do temporal filters and version-aware projections retrieve the correct state
for time-scoped and changed-fact questions without returning stale superseded
evidence?

## Execution plan

1. Run a content-free factual preflight over the registered inputs: TimeQA test
   easy/hard, LongMemEval-S `temporal-reasoning`, and TimelineQA's three
   generated density families. Pin the registry, source revision, payload
   digests where available, counts, and source-field capability. Keep all three
   results separate. TimelineQA remains evaluation-only, external, and never
   committed or shipped.
2. Create one external, deterministic query-and-evidence manifest after the
   preflight. It must define selected records, canonical source identities,
   timestamps, validity-window derivation, exclusions, and per-corpus
   denominators without copying corpus text into Git.
3. Compare the selected ANSWER-01 baseline (`a0_turn_fts`, with shipped
   `stream_default` runtime) against exactly one treatment: the same retrieval
   profile with source-derived validity windows and
   `ReadView(valid_as_of=...)`. Use a fresh database for every cell. No model,
   answerer, GPU, or paid call is part of this preparation.
4. Report retrieval evidence recall, validity-window accuracy, stale-hit rate,
   and latency by corpus and declared class. TimeQA can support time-sensitive
   answer reporting, but the held-out test files contain no unanswerable
   records and therefore cannot support an abstention metric. It also cannot
   support a `valid_as_of` claim unless the external manifest supplies a
   source-derived window. LongMemEval and TimelineQA may support a bounded
   window treatment only after that derivation is reviewed.
5. Stop after the one fixed comparison. Do not pool corpora, infer a
   supersession or erasure claim, or call this historical-state retrieval:
   FathomDB's shipped `ReadView` is world-time validity only and has no
   `history_as_of` search.

## Stop

Stop at the factual-preflight block if a source-derived validity-window mapping
cannot be declared. Do not broaden the corpus or infer temporal correctness
from aggregate LOCOMO accuracy.
