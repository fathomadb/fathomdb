---
title: Retrieval Subsystem Design
date: 2026-04-30
target_release: 0.6.0
desc: Fixed-stage retrieval pipeline, query planning, and branch-fallback behavior
blast_radius: search path; REQ-010, REQ-011, REQ-017, REQ-018, REQ-029, REQ-034
status: locked
---

# Retrieval Design

This file owns the fixed-stage retrieval pipeline, safe FTS grammar handling,
hybrid branch composition, and the graph-expansion configuration that survives
the ADR-level pipeline choice.

## 0.6.0 stage surface

0.6.0 supports graph `expand` on search results as carried-forward product
surface. `rerank` is deferred and is not part of the 0.6.0 search contract.

## Soft-fallback signal

REQ-029 / AC-031 make the hybrid fallback signal part of the public search
contract.

The typed branch enum in 0.6.0 is exactly:

- `Vector`
- `Text`

Semantics:

- the fallback record is present only when one non-essential branch could not
  contribute
- `Vector` means the vector branch could not contribute
- `Text` means the text branch could not contribute
- total request failure is not expressed as a soft-fallback record

This file owns the branch enum and its meaning. The per-binding field name on
the returned fallback record is owned by `interfaces/{python,typescript,rust}.md`.

## Direct-text FTS rank-boundary collection

The direct `search_text_only*` family fixes its node candidate input at 100 for
every accepted public result limit. For an unfiltered direct-text request on a
database with no edge-body FTS rows, the engine collects that input using the
FTS5 hidden-rank stream:

1. the query pins `rank MATCH 'bm25()'` so a persistent database rank setting
   cannot change product semantics;
2. `ORDER BY rank` is consumed through the complete BM25 score group crossing
   candidate 100;
3. the bounded candidates are restored to ascending
   `(bm25 score, write_cursor)` order and truncated to 100; and
4. normal body deduplication, RRF, and public-limit truncation continue
   unchanged.

The complete boundary score group is required because SQLite does not promise
secondary ordering among equal ranks. In the all-equal case this intentionally
scans every matching node to preserve the stable cursor prefix.

Filters, edge-bearing databases, hybrid/vector requests, and legacy-schema
fallbacks retain the existing full stable-sort collector. If streamed statement
preparation, stepping, or row conversion fails, partial streamed candidates are
discarded and that same full-sort path runs. The optimization changes no cache,
mmap, temp-store, reader-pool, hybrid/vector, schema, or public SDK behavior.

The writer connection explicitly applies `WAL + synchronous=NORMAL`, restoring
the accepted durability invariant; reader-pool and runtime connections are not
changed by this collector.
