# EARP S6 retrieval repricing result

## Decision question and claim class

Why did corpus-scale `search_text_only` take seconds per query after the
public result limit landed, and did the Slice 19 write-cursor indexes remove
that cost? This is a diagnostic result, not a performance or regression gate.

## Systems and configurations

- Before the fix: FathomDB 0.8.22 development code after Slice 18, with
  `Engine.search_text_only` at `limit=10` and `limit=100`.
- After the fix: FathomDB 0.8.22 development code with Slice 19 commit
  `e95afd29`, including `canonical_nodes_write_cursor_idx` and
  `canonical_edges_write_cursor_idx`.
- Source records: commits `0467dda1` and `9d05def4` on
  `feat/earp-eval-platform-20260806`. Those commits contain the historical
  diagnosis and post-fix measurement; no evaluator code is ported here.

## Corpus and protocol

The run ingested the frozen `0.8.x-B` corpus: 10,506 documents from ten
sources, corpus SHA-256
`fe973fcd49fbbda083158f69fe720f17858ab8528e171fa2188eec84131c7d4e`.
The corpus combines project-authored data with MIT, Apache-2.0, BSD-3-Clause,
CC-BY-4.0, research-use, and undeclared/upstream-chain source licenses. Its
payload remains local; this note redistributes none of it.

The timing treatment used a deterministic stride sample of 12 real-gold
queries, warm caches, and interleaved limit arms. The mechanism probe compared
raw FTS5 against the same query joined to `canonical_nodes` by `write_cursor`.
The post-fix run repeated that method on the same host after rebuilding the
binding. Each pre- and post-fix cell is one execution; there was no repeated-run
latency protocol or tail-latency treatment.

## Result and uncertainty

Before Slice 19, `limit=10` averaged 11.89 seconds and `limit=100` averaged
12.05 seconds. A single-keyword probe took 0.008 seconds as raw FTS5 but 7.69
seconds after the unindexed `write_cursor` join. Adding the index on a copy of
the database reduced that join probe to 0.0068 seconds.

After Slice 19, the warm means were 37.8 milliseconds at `limit=10` and 40.5
milliseconds at `limit=100`; the query plan used
`canonical_nodes_write_cursor_idx`. The observed before/after warm-mean ratio
was 315 times. Because the sample contained only 12 deterministic queries and
no repeated timing trials, no confidence interval or robust tail statistic is
available. The full-pass estimates of about 15.2 hours before and 2.9 minutes
after are extrapolations, not observed end-to-end pass durations.

## Artifact availability

The historical source note is available in Git at
`dev/notes/earp-s6-reprice-2026-08-07.md` in commits `0467dda1` and
`9d05def4`. A local copy was observed at
`experiments/runs/earp-s6-reprice-2026-08-07.md` while preparing this record.
The session scratch scripts and timing database named by the source note are
not retained, so the measurements are historical evidence rather than a fully
reproducible performance artifact.

## Nonclaims

This result does not establish a latency SLO, throughput, process-cold
performance, behavior on another corpus, or an eligible repeated-run summary.
It supports the narrower diagnosis that an unindexed write-cursor join caused
the measured S6 cost and that the indexed plan removed that mechanism in the
observed environment.
