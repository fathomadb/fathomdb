# GRAPH-01 — Graph-projection self-characterization

**Status:** complete; registered treatment rejected.

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
comparison. The [result](../2026-08-30-graph-01-result.md) and
[receipt](../../../experiments/runs/graph-01-protected-bridge-20260830T0035Z-d6e7c4b2/record.json)
close it.

## Stop

The bounded comparison is complete. Retain the fused control; do not build a
graph-only retriever, tune this treatment on the reused cohort, or start
REASON-01 from this rejected result.
