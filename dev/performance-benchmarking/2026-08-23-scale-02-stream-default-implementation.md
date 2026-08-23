# SCALE-02 stream-default implementation note

## Decision

HITL `seq-267` approves `stream_default` as the production FTS path. It uses
streamed BM25 boundary-tie completion with shipped reader defaults. mmap and an
enlarged cache remain excluded. No confirming benchmark is required.

## Implemented

Candidate `9e507553f954c56d5c6177eabf1750faddf3acfd` adds the measured path behind the
experiment gate:

- scan FTS5 in native `ORDER BY rank` order;
- retain the first 100 candidates and finish only the BM25 score group crossing
  that boundary;
- restore the production `(BM25 score, write_cursor)` order in memory and
  truncate to 100;
- fall back to the shipped full stable sort if statement execution or row
  conversion fails;
- leave reader cache, mmap, and temp-store settings at shipped defaults.

The candidate also adds content-free experiment witnesses for route selection,
rows consumed, boundary-group size, query plan, and writer/reader SQLite
settings. Writer connections explicitly apply WAL plus
`synchronous=NORMAL`, matching the accepted durability profile.

Tests cover strict boundaries, ties beyond row 101, all-equal scores, stable
cursor order, forced-full-sort equivalence, query-plan behavior, and generated
tie-group shapes. The frozen runner performs preflight before timing and writes
safe receipts plus an external artifact manifest.

## Evidence

The
[rank-boundary result](2026-08-22-scale-02-rank-boundary-result.md) records 60
fresh-database repetitions at 25k, 40k, and 50k. Both streamed cells had zero
top-100 or top-10 mismatches, zero full-sort fallbacks, zero errors, and zero
timeouts. `stream_default` passed every registered decision criterion and was
selected over mmap128 on footprint.

## Remaining production landing

The measured implementation is still selected by the private experiment gate.
The production change must:

1. make boundary-tie streaming the ordinary eligible direct-text path;
2. preserve forced-full-sort and content-free witness seams as test controls;
3. keep filters, edge-bearing databases, and ineligible searches on their
   existing exact paths;
4. update tests so production behavior, failure fallback, and limit-prefix
   stability are explicit contracts;
5. remove stale experiment-only framing after the production tests pass.

The existing SCALE-02 receipt is the decision basis. Landing verification is a
code-correctness gate, not authorization for another performance run.
