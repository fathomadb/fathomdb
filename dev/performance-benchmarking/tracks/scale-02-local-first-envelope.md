# SCALE-02 — Local-first scale envelope

**Status:** blocked on SCALE-01 and a selected LOCOMO/PARENT projection profile.

## Decision

What measured local-first operating envelope is supportable for the selected
retrieval profile without implying an unmeasured capacity guarantee?

## Preparation and contract

1. Pre-register the workload matrix: open, ingest/drain, FTS, vector, hybrid,
   rerank, and every enabled projection, with CPU/GPU/cold/steady cells separate.
2. Freeze the scale ladder, host/device, corpus or fixture provenance, criteria,
   error policy, and repetitions before measurement.
3. Count canonical records separately from vector children, summaries, entities,
   edges, and all derived projection rows.

## Exit evidence

The report names the largest observed passing configuration and every excluded
operation. It contains p50/p95/p99, throughput, errors, and projection
amplification; it is advisory only.
