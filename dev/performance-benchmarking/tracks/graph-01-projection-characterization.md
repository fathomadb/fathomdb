# GRAPH-01 — Graph-projection self-characterization

**Status:** contract and design reviewed; implementation in progress.

## Decision

Do bounded, high-confidence, provenance-backed graph projections improve
multi-hop evidence retrieval enough to justify extraction and maintenance?

## Plan

1. Reuse the pinned, labelled 300-question MuSiQue cohort and its question-blind
   paragraph extractions. Rebuild them in a fresh FathomDB 0.8.23 database with
   exact paragraph provenance.
2. Before retrieval scoring, report claim/edge extraction precision, confidence
   eligibility, entity-resolution and co-mingling errors, duplicate entities,
   orphan or unattributed edges, stale edges, and graph cardinality/storage.
3. Compare the selected fused non-graph baseline with one exact-anchor,
   two-hop, protected bridge-completion treatment. Do not repeat raw BFS,
   graph-only retrieval, lexical-seeded PPR/RRF, or index-key enrichment.
4. Accept only if supporting-evidence recall or answer F1 improves without a
   graph-quality or lifecycle violation and within the declared
   extraction/query cost.

The dated [measurement contract](../2026-08-29-graph-01-contract.md),
[design](../2026-08-29-graph-01-design.md), and
[design review](../2026-08-29-graph-01-design-review.md) freeze the runnable
comparison.

## Stop

Stop after the bounded comparison. Do not build a graph-only retriever or start
native comparator tracks unless this treatment shows a useful gain.
