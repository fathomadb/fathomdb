# SCALE-02 — Local-first scale envelope

**Status:** complete; advisory boundary observed at the first registered point.

## Decision

What measured local-first operating envelope is supportable for the selected
retrieval profile without implying an unmeasured capacity guarantee?

## Inputs and scope

- The selected profile is A0: turn-level FTS, `top_k=10`, with no embedder or
  reranker. The [ANSWER-01 result](answer-01-shortlist-scoring.md) retains A0.
- The
  [SCALE-01 primary](../../../experiments/runs/tc5-gpu-primary-20260822T1605Z-2d574205/record.json)
  qualifies pre-fusion vector fidelity at 17,272 documents only. It does not
  select vector, hybrid, or reranking for A0 or establish their efficiency.
- SCALE-02 measures FathomDB 0.8.23 on one declared host. Its result is an
  advisory operating envelope for the registered workload, not a product
  capacity limit.
- The executable
  [configuration](../../../experiments/configs/scale-02/a0-envelope.v1.json)
  binds the workload and advisory policy approved by the HITL on 2026-08-22.

## Draft plan

1. Pre-register a workload matrix at 10k, 17,272, 25k, 40k, and 50k canonical
   records. Make A0's open, ingest/drain/readiness, and FTS cells required.
   Register pre-fusion exact-vector, hybrid, or bounded CE reranker cells only
   as separately named components; an unmeasured component is out of scope.
2. Bind the corpus manifest, query set, operation mix, concurrency, seeds,
   FathomDB build and configuration, projection enrollment, model assets, and
   explicit CPU/GPU routes before execution. Report canonical and derived rows
   separately. GPU cells must pin the approved GPU; never use `auto` or a
   silent CPU fallback.
3. Give every independent repetition a fresh database and output directory
   through the shared
   [`prepare_test_database`](../../../experiments/fathomdb_test_setup.py)
   setup. Complete integrity checks and `fathomdb doctor` before timing. Keep
   model acquisition outside the measured interval while recording its bytes
   and elapsed time separately. In each repetition, record cold observations
   before the declared warm-up and steady-state observations after it; never
   pool them.
4. Pre-register repetitions, warm-up, sampling, ordering, timeout/retry rules,
   uncertainty construction, and advisory pass criteria. Run the size ladder
   in increasing order and advance only after the current point produces a
   complete, valid receipt.
5. For every registered operation, report p50/p95/p99 latency, throughput,
   error and timeout counts, and sample completeness. Also report peak host
   CPU/RAM, GPU memory/utilization where used, database bytes, projection
   amplification, ingest acknowledgement-to-ready time, and
   mutation-to-ready time.

## Decision rule

Name the largest directly measured point at which every required A0 cell meets
the pre-registered advisory criteria. Report optional component envelopes
separately: their success does not enlarge A0, and their failure does not
shrink A0 unless it affects a required A0 cell. Keep cold and steady-state,
CPU and GPU, and canonical and derived-row results distinct. Link SCALE-01 as
fidelity evidence without pooling it into SCALE-02 performance results.

## Result

The [10k receipt](../../../experiments/runs/scale-02-a0-10000-20260822T1715Z-77c37c77/record.json)
records five complete fresh-database repetitions with zero errors and zero
timeouts. Steady FTS p50 was 29.94 ms against the approved 20 ms limit; p99 was
58.93 ms against 150 ms. The first point therefore failed advisory eligibility,
and the registered stop rule prohibited the 17,272-to-50k points. No passing
SCALE-02 operating point was established under this policy.

## Stop

Stop at the first unsafe point, breached resource limit, persistent error under
the registered retry rule, incomplete repetition, provenance/device mismatch,
or reused database. Do not advance through a failed size, extrapolate between
points or past 50k, substitute synthetic rows for the registered corpus, or
convert the advisory result into a capacity guarantee.
