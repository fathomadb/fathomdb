# SCALE-01 — TC-5 all-real fidelity characterization

**Status:** active; implement TC-5 v2, then run GPU fidelity.

## Decision

What all-real, GPU-primary vector-stage fidelity is observed at the
manifest-qualified 17,272-document envelope, with an optional, smaller
12-core CPU equivalence bridge only when separately justified?

## Draft plan

1. Add only the v2 controls needed by FathomDB 0.8.23's exact pre-fusion
   selector and explicit `cuda:0` setup.
2. Run a 7,667-document GPU smoke, then the 17,272-document, 100-query GPU
   primary with the frozen TC-5 fidelity rule.
3. Run CPU only if a release-equivalence bridge is still needed: 7,667
   documents, up to twelve observed workers, in a separate receipt.

## Stop

Stop on incomplete provenance, model mismatch, GPU mismatch, or partial queries.
The result is fidelity evidence only; CPU is not required for closure.
