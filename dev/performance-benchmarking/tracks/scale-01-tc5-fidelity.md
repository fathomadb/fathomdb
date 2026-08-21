# SCALE-01 — TC-5 GPU-primary vector-stage fidelity characterization

**Status:** active; the CPU-only v1 manifest runner, release-gated executor,
and external driver are historical controls. FathomDB 0.8.23 provides the
private GPU selector, but v2 integration and review are required before release.
Execution remains blocked until an eligible all-real 17,272/7,667 source,
manifest, output root, GPU/model/ground-truth/runtime inventory, pinned RTX
3090, and coordinator release are qualified. See the
[0.8.23 GPU-primary amendment](../2026-08-21-fathomdb-0.8.23-gpu-primary-amendment.md).

## Decision

What all-real, GPU-primary vector-stage fidelity is observed at the
manifest-qualified 17,272-document envelope, with an optional small CPU bridge
only when a release-version equivalence decision needs it?

## Preparation and contract

1. Implement the test-only manifest-backed runner described by the dated TC-5
   plan; do not alter the historical eu7 gate or its output file.
2. Freeze the external corpus manifest, 7,667-document GPU bridge,
   17,272-document GPU arm, selected RTX 3090 UUID, model asset hash, seeds,
   and bootstrap procedure. A CPU bridge, if separately released, is limited to
   7,667 documents and twelve effective workers.
3. Test count mismatch, duplicate, missing, malformed, synthetic-padding, and
   incomplete-provenance failure paths before the smoke invocation.

## Exit evidence

One complete external result contains both frozen arms and a safe receipt. It
reports fidelity and uncertainty only; it does not assert a latency or release
claim.
