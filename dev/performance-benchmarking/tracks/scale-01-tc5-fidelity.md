# SCALE-01 — TC-5 all-real fidelity characterization

**Status:** complete; GPU primary registered.

## Decision

What all-real, GPU-primary vector-stage fidelity is observed at the
manifest-qualified 17,272-document envelope, with an optional, smaller
12-core CPU equivalence bridge only when separately justified?

## Inputs

- The registered
  [7,667-document GPU smoke](../../../experiments/runs/tc5-gpu-smoke-20260822T1446Z-2d574205/record.json).
- The reviewed
  [0.8.23 GPU-primary amendment](../2026-08-21-fathomdb-0.8.23-gpu-primary-amendment.md)
  and frozen
  [TC-5 v2 configuration](../../../experiments/configs/scale-01/tc5-gpu-v2.json),
  which bind the qualified all-real corpus, model, query, measurement, and GPU
  inputs. This current plan replaces the amendment's provisional duplicate
  long-run sequence: the qualified primary is the closing characterization.

## Run and decision rule

1. Accept the smoke as preparation evidence only after its receipt confirms
   the frozen inputs, zero synthetic rows, complete vector projection, all 100
   queries, and the pinned GPU route.
2. From a clean checkout, repeat the GPU preflight and dry-run, create a fresh
   database, and run the 17,272-document primary under the same frozen fidelity
   rule and query-source exclusions.
3. Register the safe receipt and report recall@10, its 95% bootstrap interval,
   and bootstrap sigma. The primary answers this descriptive track only when
   all inputs and 100 queries qualify; the historical 0.90 value is context,
   not an acceptance threshold or an automatically changed floor.
4. Run a CPU bridge only for a separately stated release-equivalence question:
   7,667 documents, no GPU, no more than twelve observed effective workers,
   pre-registered tolerances, and a separate receipt.

## Stop

Stop on corpus or selection drift, synthetic rows, incomplete projection or
provenance, model/GPU mismatch, a reused database, or partial queries/bootstrap.
A qualified primary closes SCALE-01 and permits SCALE-02 workload registration;
it makes no latency, efficiency, retrieval-relevance, capacity, or product
claim. CPU is not required for closure.

## Result

- [Registered GPU primary](../../../experiments/runs/tc5-gpu-primary-20260822T1605Z-2d574205/record.json):
  17,272 real documents, 100 complete queries, recall@10 `0.958`, 95%
  bootstrap interval `[0.939, 0.975]`, and bootstrap sigma `0.00876`.
- The historical `0.90` reference was observed. This remains bounded
  vector-stage fidelity evidence, not a latency, relevance, or capacity claim.
- No CPU bridge was needed.
