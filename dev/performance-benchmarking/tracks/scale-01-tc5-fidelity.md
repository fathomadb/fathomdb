# SCALE-01 — TC-5 all-real fidelity characterization

**Status:** active; the CPU-only v1 manifest runner, release-gated executor,
and external driver are retained as historical controls. The
[0.8.23 GPU-primary amendment](../2026-08-21-fathomdb-0.8.23-gpu-primary-amendment.md)
supersedes them for any new TC-5 release after its integration and independent
review. Execution remains blocked before that release until the all-real
17,272/7,667 source, manifest, output root, model/ground-truth/runtime
inventory, and coordinator release are qualified.

## Decision

What all-real, GPU-primary vector-stage fidelity is observed at the
manifest-qualified 17,272-document envelope, with an optional, smaller
12-core CPU equivalence bridge only when separately justified?

## Preparation and contract

1. Implement the test-only manifest-backed runner described by the dated TC-5
   plan; do not alter the historical eu7 gate or its output file.
2. Freeze the external corpus manifest, 7,667-document bridge, 17,272-document
   GPU primary arm, pinned RTX 3090 UUID, model asset hash, seeds, and bootstrap
   procedure. A separately released CPU bridge is limited to the 7,667-document
   arm and twelve effective workers.
3. Test count mismatch, duplicate, missing, malformed, synthetic-padding, and
   incomplete-provenance failure paths before the smoke invocation.

## Exit evidence

One complete external result contains both frozen arms and a safe receipt. It
reports fidelity and uncertainty only; it does not assert a latency or release
claim.
