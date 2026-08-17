# SCALE-01 — TC-5 all-real fidelity characterization

**Status:** active; the manifest runner, release-gated executor, and external
driver are integrated and independent of LOCOMO work. Execution remains blocked
before release until an eligible all-real 18,472/7,667 source, manifest, output
root, CPU/model/ground-truth/runtime inventory, and coordinator release are
qualified.

## Decision

What all-real, CPU same-backend vector-stage fidelity is observed at the
manifest-qualified 18,472-document envelope?

## Preparation and contract

1. Implement the test-only manifest-backed runner described by the dated TC-5
   plan; do not alter the historical eu7 gate or its output file.
2. Freeze the external corpus manifest, 7,667-document bridge, 18,472-document
   arm, CPU device, model asset hash, seeds, and bootstrap procedure.
3. Test count mismatch, duplicate, missing, malformed, synthetic-padding, and
   incomplete-provenance failure paths before the smoke invocation.

## Exit evidence

One complete external result contains both frozen arms and a safe receipt. It
reports fidelity and uncertainty only; it does not assert a latency or release
claim.
