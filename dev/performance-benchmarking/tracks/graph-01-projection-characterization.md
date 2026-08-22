# GRAPH-01 — Graph-projection self-characterization

**Status:** planned; native graph comparisons remain parked until this decision.

## Decision

Do bounded, high-confidence, provenance-backed graph projections improve
multi-hop evidence retrieval enough to justify extraction and maintenance?

## Draft plan

1. Start with a labelled multi-hop failure set and TRACE-01-safe entities and
   edges.
2. Compare the selected non-graph baseline with one lexical-seeded, bounded
   graph-expansion treatment.
3. Accept only if supporting-evidence recall or answer F1 improves without
   lifecycle violations and within the declared extraction/query cost.

## Stop

Stop after the bounded comparison. Do not build a graph-only retriever or start
native comparator tracks unless this treatment shows a useful gain.
