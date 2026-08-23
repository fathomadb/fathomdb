# TEMPORAL-01 — Time-scoped retrieval

**Status:** factual preflight complete; first executable cell is synthetic
TRACE validity. External-corpus comparison remains blocked.

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
2. The official LongMemEval and TimelineQA releases do not provide the needed
   external record-to-evidence validity-window manifest. Record that finding
   in [the source review](../2026-08-23-temporal-01-source-manifest-review.md).
   Do not infer a corpus mapping from timestamps, session ordering, generator
   output, or the paper's gold-evidence description.
3. Run one synthetic TRACE validity cell first. It uses a fixed, payload-free
   lifecycle fixture with canonical source IDs and three known half-open
   validity windows. In one fresh FathomDB database, index the same FTS query
   text for every fixture record and query at the lower boundary, interior,
   upper boundary, and an out-of-window instant with
   `ReadView(valid_as_of=...)`. The expected hit IDs are declared in the
   checked-in configuration. Use the selected FTS profile (`a0_turn_fts`,
   shipped `stream_default`), no embedder, no reranker, no GPU, and no model or
   paid call. Record boundary exactness, unexpected-hit count, and query
   latency. This is a FathomDB validity contract check, not an external-corpus
   retrieval or answer-quality result.
4. Only if an upstream or otherwise reviewed source-derived external manifest
   later exists, compare the selected baseline against the same profile with
   source-derived validity windows and `ReadView(valid_as_of=...)`. The
   manifest must define selected records, canonical source identities,
   timestamps, derivation, exclusions, and denominators without copying corpus
   text into Git. Keep each corpus separate.
5. Stop after the synthetic cell unless that manifest and a temporal adapter
   are bound. Do not pool corpora, infer supersession or erasure, or call the
   result historical-state retrieval: shipped `ReadView` is world-time
   validity only and has no `history_as_of` search.

## Stop

Stop external-corpus execution if a source-derived validity-window mapping
cannot be declared. The synthetic TRACE cell cannot establish corpus fidelity,
supersession, erasure, or answer quality. Do not broaden the corpus or infer
temporal correctness from aggregate LOCOMO accuracy.
