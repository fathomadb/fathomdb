# SCALE-01 TC-5 factual input qualification contract

**Track:** `SCALE-01`  
**Date:** 2026-08-16  
**Status:** read-only factual qualification; not a live-execution release.

## Purpose

This contract turns factual readiness into a small, content-free proof before
the authorized TC-5 smoke. It complements the manifest, live-executor, and
external-driver contracts. It neither supersedes them nor permits a model
load, driver invocation, external measurement, index append, or release.

`experiments.tc5_input_qualification` consumes only a future
`tc5-input-inventory.v1`, the safe `tc5-manifest.v1`, the committed
CORPUS-01 matrix, and the committed
`tc5-input-qualification-policy.v1`. The external manifest may contain stable
document IDs and content hashes, but the report it writes contains no document
IDs, corpus text, paths, query/prediction data, or host identity.

## Required factual bindings

The inventory is eligible for a factual-qualified report only when it binds:

- a named CORPUS-01 matrix corpus that supports `retrieval_fidelity_only`, its
  exact matrix SHA-256, a license-copy SHA-256, source revision, and source
  artifact SHA-256;
- the exact validated all-real 18,472-document manifest SHA-256 and its
  canonical first-7,667-document bridge;
- CPU-host, cached pinned-model, exact-f32 ground-truth, sealed vector-stage
  runtime, and external-output-root attestations; and
- the manifest's source artifact and model-asset hashes.

The report is `factual_inputs_qualified`, not `released`. It still requires a
coordinator-bound execution release, an accepted integrated SHA, an external
runner hash, and the live executor's release validation before either arm may
run. It supports only bounded fidelity evidence; it cannot support a SCALE-02,
latency, SLO, product, or release claim.

## Failure and artifact rules

An absent source, license, exact manifest, external root, CPU/model fact,
ground-truth digest, or vector-stage runtime produces
`blocked_prerequisite`. The blocker report names only fixed prerequisite codes
and safe logical inventory IDs. It must not synthesize/pad a manifest, reuse
historical EU7 output, or convert LOCOMO material into a TC-5 source.

All reports are new files below the declared external preflight root. The
writer rejects a repository destination, an escaped path, symlink destination,
or existing file. A blocked report is evidence that no live action was taken;
it is not a failed measurement.
