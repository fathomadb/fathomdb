# Slice 45 pagination performance pilot findings

Date: 2026-09-05

The initial 10k pilot incorrectly compared the governed page query with a SQL
shape that omitted dependency/lifecycle eligibility. That comparison is
rejected. A matched-shape test hook now runs the identical query on the same
reader-worker path while omitting only frozen-token and cursor work.

Two causal implementation findings were real:

1. Frozen validation re-encoded every `_fathomdb_projection_terminal` row. At
   10k canonical plus 10k state rows, validation alone was about 2.45 ms p95.
   Schema 33 serving-binding v3 now relies on the separately authenticated
   monotonic visibility generation for terminal mutation detection. The
   schema-31/32 encodings remain pinned.
2. The inherited dependency eligibility predicate looked up
   `_fathomdb_artifact_revisions` by `write_cursor` without a leading index.
   Matched page p95 grew from about 28.8 ms at 10k to 298.7 ms at 50k. Adding
   `_fathomdb_artifact_revisions(write_cursor)` reduced the 50k matched query
   to about 0.35 ms p95. An additional `operational_state(write_cursor)` index
   bounds the frozen canonical-write-boundary lookup.

Post-correction 50k pilot (1,000 samples in one process): matched page p95
0.352 ms, frozen first-page p95 0.493 ms, continuation p95 0.382 ms, current
state p95 0.023 ms, frozen state p95 0.104 ms, and frozen validation p95 0.173
ms. No comparison crossed both the relative and absolute materiality
thresholds. These are pilot observations; the formal multi-process receipt is
authoritative.
