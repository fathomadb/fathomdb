# SCALE-02 stream-default implementation note

## Decision

HITL `seq-267` approves `stream_default` as the production FTS path. It uses
streamed BM25 boundary-tie completion with shipped reader defaults. mmap and an
enlarged cache remain excluded. No confirming benchmark is required.

## Production implementation

Candidate `9e507553f954c56d5c6177eabf1750faddf3acfd` introduced the measured path. The
production landing now:

- scans eligible direct-text FTS5 searches in native `ORDER BY rank` order;
- retains the first 100 candidates and finishes only the BM25 score group crossing
  that boundary;
- restores the production `(BM25 score, write_cursor)` order in memory and
  truncates to 100;
- falls back to the shipped full stable sort if statement execution or row
  conversion fails;
- keeps filters, edge-bearing databases, and other ineligible searches on their
  existing exact paths;
- leaves reader cache, mmap, and temp-store settings at shipped defaults.

Content-free experiment witnesses remain available for route selection,
rows consumed, boundary-group size, query plan, and writer/reader SQLite
settings. Writer connections explicitly apply WAL plus
`synchronous=NORMAL`, matching the accepted durability profile.

Tests cover strict boundaries, ties beyond row 101, all-equal scores, stable
cursor order, forced-full-sort equivalence, row-conversion failure fallback,
query-plan behavior, public limit-prefix stability, and generated tie-group
shapes. The private stream-selection gate and the superseded strict-boundary
helper are removed from production code. The forced-full-sort control remains
private to the experiment/test gate.

## Evidence

The
[rank-boundary result](2026-08-22-scale-02-rank-boundary-result.md) records 60
fresh-database repetitions at 25k, 40k, and 50k. Both streamed cells had zero
top-100 or top-10 mismatches, zero full-sort fallbacks, zero errors, and zero
timeouts. `stream_default` passed every registered decision criterion and was
selected over mmap128 on footprint.

## Landing verification

The production landing is code-complete. Verification covers:

- `scale02_fts_rank_fast`: production routing, exact-path equivalence,
  boundary-tie completion, and failure fallback;
- the generated boundary-group property test;
- `slice23_text_limit_prefix_stability`: public direct-text limit-prefix
  behavior.

The existing SCALE-02 receipt is the decision basis. Landing verification is a
code-correctness gate, not authorization for another performance run.
