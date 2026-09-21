# GRAPH-EVIDENCE-01 — Exact evidence fidelity

**Status:** implemented; native smoke verified; full lifecycle campaign pending

## Decision

Does frozen graph expansion disclose and resolve the exact target and terminal
edge evidence without stale, foreign, tampered, or erased disclosure?

## Plan

Implement the reviewed synthetic evidence matrix, semantic oracle, separate
fidelity/cost output, safe receipts, and optional gauntlet adapter defined in
the graph benchmark requirements and design.

## Stop

Stop on oracle disagreement, nondisclosure failure, input drift, or an
unqualified runtime. Do not convert fidelity failures into latency samples.
