# GLOBAL-01 — Native GraphRAG comparison

**Status:** complete, limited; global-sensemaking calibration, not a
personal-memory default gate.

## Decision

Under a fair native reproduction, how does FathomDB compare with GraphRAG on
global coverage, diversity, empowerment, and cost?

## Draft plan

1. Start from a named global-synthesis failure. Use a TRACE-01-safe graph
   treatment if GRAPH-01 earns one; otherwise use source-linked summaries with
   bounded coverage-oriented retrieval and map-reduce.
2. Pin the official GraphRAG build and run one small smoke, then one matched
   comparison on the same corpus, read budget, answerer, judge, and cost cap.
3. Report coverage, diversity, empowerment, cost, and uncertainty for both
   exhaustive and product-viable FathomDB modes.

## Stop

Stop on corpus mismatch, incomplete arms, or the cost cap. A split result is a
valid result; do not tune indefinitely.

## Outcome

The [first-run result](../2026-08-29-global-01-first-run-result.md) is split.
Native GraphRAG led directionally on comprehensiveness, diversity, and
empowerment; source-linked map-reduce led on directness. Reopen only for a
preregistered hierarchical-summary or graph treatment prompted by the measured
coverage gap.
