---
title: Slice 135 deferred text identity assertion-strength mutation
status: KILLED_MUTANT
target_release: 0.8.27
---

# Deferred text identity mutation probe

The exact candidate source `224e44c593c13d86ece648adabe445723db04070`
passed the named real-database
`slice135_deferred_text_identity_error` test. An isolated source copy then
reintroduced the historical swallowed-row-error branch shown in
[mutation.patch](mutation.patch): a failed canonical provenance row decode
again became an absent identity. The test compiled and **failed at its
intended assertion**; it did not fail from a build or setup error.

The [control output](control.stdout) reports one pass. The
[mutant output](mutant.stdout) reports one failed test, and
[mutant diagnostics](mutant.stderr) show the exact defect: malformed
provenance returned a successful text hit with `source_id: None` instead
of `Storage`. The isolated mutant was never applied to the candidate build
checkout or Slice 135 product tree. This is one targeted assertion-strength
check for a high-use search path, not a mutation score over the engine.
