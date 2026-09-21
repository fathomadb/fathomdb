# GRAPH-EXPAND-01 — Bounded traversal characterization

**Status:** implemented; native smoke verified; larger scale cells not run

## Decision

What correctness, latency, throughput, work, and resource envelope does native
bounded graph expansion provide under the frozen generated workload?

## Plan

Implement the independent BFS/work oracle, exact generated manifest, synthetic
correctness tier, optional scale cells, safe receipts, and gauntlet adapter.

## Stop

Stop on oracle disagreement, workload drift, invalid work accounting, or a
resource blocker. Never resize a registered scale silently.
