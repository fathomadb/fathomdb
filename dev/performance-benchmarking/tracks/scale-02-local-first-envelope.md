# SCALE-02 — Local-first scale envelope

**Status:** blocked on SCALE-01 and a selected LOCOMO/PARENT projection profile.

## Decision

What measured local-first operating envelope is supportable for the selected
retrieval profile without implying an unmeasured capacity guarantee?

## Draft plan

1. Use the ANSWER-01 profile and SCALE-01 receipt. Measure open, ingest/drain,
   FTS, exact vector, hybrid, and the selected reranker; omit unused surfaces.
2. Run the fixed ladder at 10k, 17,272, 25k, 40k, and 50k rows. Keep GPU and
   CPU results separate; pre-register repetitions and uncertainty, and do not
   pool cold and steady-state cells.
3. Report p50/p95/p99, throughput, errors, peak host/device memory and
   utilization, database bytes and projection amplification, ingest
   acknowledgement/ready time, and mutation-to-ready time.
4. Name the largest observed passing point and report canonical records and
   derived projection rows separately.

## Stop

Stop at the first unsafe or persistently failing point; do not extrapolate past
50k or convert the advisory result into a capacity guarantee.
