# GRAPH-01 — Graph-projection self-characterization

**Status:** planned; native graph comparisons remain parked until this decision.

## Decision

Do bounded, high-confidence, provenance-backed graph projections improve
multi-hop evidence retrieval enough to justify extraction and maintenance?

## Preparation and contract

1. Close TRACE-01 for entity and edge provenance, invalidation, and erasure.
2. Freeze extractor, confidence threshold, entity-resolution policy, graph size
bound, lexical/hybrid seed depth, traversal bound, and parent evidence return.
3. Use MuSiQue-style supporting-evidence recall and answer F1; compare lexical
seeding plus bounded expansion against matched non-graph controls.
4. Record extraction failures, graph cardinality, stale/mixed-entity cases,
   storage multiplier, and query cost as first-class outcomes.

## Exit evidence

A complete self-characterization distinguishes graph benefit from retrieval or
measurement artifacts. It provides the evidence required to prioritize GLOBAL-01
or REASON-01; it does not assume graph-only retrieval.
