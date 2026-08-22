# SCALE-01 — TC-5 all-real fidelity characterization

**Status:** active; verify TC-5 v2, then run GPU fidelity.

## Decision

What all-real, GPU-primary vector-stage fidelity is observed at the
manifest-qualified 17,272-document envelope, with an optional, smaller
12-core CPU equivalence bridge only when separately justified?

## Draft plan

1. Verify and commit the v2 runner, query-source exclusion, aggregate recall,
   fresh-database setup, and explicit `cuda:0` controls. Do not run TC-5 from a
   dirty worktree.
2. Run a 7,667-document GPU smoke, then the 17,272-document, 100-query GPU
   primary with the frozen TC-5 fidelity rule.
3. Run CPU only if a release-equivalence bridge is still needed: 7,667
   documents, up to twelve observed workers, in a separate receipt.

## Stop

Stop on incomplete provenance, model mismatch, GPU mismatch, or partial queries.
The result is fidelity evidence only; CPU is not required for closure.
