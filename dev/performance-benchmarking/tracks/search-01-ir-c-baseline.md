# SEARCH-01 — IR-C FTS baseline retention

**Status:** complete historical baseline; retain as a regression reference.

## Decision

What FathomDB-only lexical retrieval result anchors later retrieval experiments?

## Preparation and contract

1. Preserve the historical receipt, corpus digest, workload, and distinction
   between retrieval quality and answer-quality claims.
2. Re-run only through a new frozen regression plan that states why historical
   evidence is insufficient; never overwrite its historical receipt.
3. Use it as an FTS comparator for IR behavior, not as an agent-memory or
   latency/comparator verdict.

## Exit evidence

The existing evidence remains discoverable and immutable. Any new measurement is
a separately identified receipt with its own configuration and claim class.
