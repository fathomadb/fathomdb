# SCALE-01 TC-5 external driver contract

**Track:** `SCALE-01`

**Date:** 2026-08-16

**Status:** implementation preparation only. This contract does not acquire or
inspect a real corpus, load a model, invoke the driver, run a smoke or long
characterization, write an external artifact, or append an experiment index.

## Purpose

`experiments/tc5_external_driver.py` is the hash-pinnable external executable
named by a later `tc5-execution-release.v1`. It is copied to the controlled
external runtime. It has no flags, no stdin protocol, and no repository output:
the accepted `experiments.tc5_live_executor` starts it only after it validates
the coordinator release, external roots, and exact arm order.

One invocation produces one strict, content-free `tc5-arm-result.v1` sidecar.
The live executor remains responsible for bridge-then-primary ordering,
resume-safe sidecar reuse, two-arm receipt closure, and index eligibility.

## Environment ABI

The driver accepts exactly the `TC5_*` variables supplied by
`tc5_live_executor`: action, arm, document count, manifest path and digest,
corpus/output roots, arm-result destination, CPU/model asset pins,
candidate/query/bootstrap pins, and ground-truth/SUT labels. Any missing or
unknown `TC5_*` name fails before the driver loads a manifest, corpus input, or
runtime.

It accepts only:

- actions `tc5-smoke` and `tc5-long-cpu-characterization`;
- `bridge: 7,667` or `primary: 18,472` documents;
- CPU, `fathomdb-bge-small-en-v1.5`, `K=192`, 100 queries, and 1,000
  bootstrap resamples under the frozen seeds; and
- the pre-fusion 1-bit candidate plus f32-rerank vector-stage SUT against
  same-model exact-f32 top-10 ground truth.

CUDA/NVIDIA visibility or a non-CPU FathomDB embed-device request fails
closed. The result path must be pre-created by the live executor, resolved
under the declared external output root, and outside the repository. An
existing non-symlink sidecar fails before the driver loads an external input or
constructs a runtime; it is never overwritten. Executor-level resume validates
an existing qualified result before choosing not to start the driver.

## External inputs and exact measurement

After ABI validation, the driver validates the release-qualified
`tc5-manifest.v1`, then reads only an external
`tc5-corpus-input.v1.json` at the corpus root. Its document rows bind the
manifest's canonical IDs/content hashes and name contained external payload
files. Its 100 canonical query rows bind their content hashes and the fixed
query-selection seed. Payload text, document IDs, query text, and paths stay
inside that runtime.

The current public FathomDB binding deliberately does not expose a
vector-stage selector. Therefore the driver has a sealed runtime protocol and
fails closed by default: it cannot call public `Engine.search()` as a fused
search substitute. A later, separately reviewed external CPU runtime must
implement that protocol against the exact pre-fusion vector-stage seam, ingest
only the selected all-real documents, compute same-model exact-f32 top-10
ranks, and compare them to the engine vector-stage top-10 ranks. It returns
only digests and per-query recall to this driver; the driver projects aggregate
recall/CI/sigma, completion counts, and manifest-derived provenance.

No GPU fallback, synthetic padding, partial query/bootstrap completion,
non-finite statistic, raw input field, SCALE-02 claim, receipt, or index append
is representable in the driver output.

## Release-time factual prerequisites

Before an authorized smoke, the coordinator must verify all of the following:

1. An accepted integrated SHA containing this driver; the external executable
   copy's SHA-256 is named in the release sidecar.
2. A CORPUS-01-qualified external manifest, corpus-input file, payload root,
   and 100-query input set whose hashes/identity match the manifest and
   license/provenance record.
3. A CPU host with no GPU selected, a cached pinned model asset, and a
   separately reviewed exact vector-stage runtime package. The release must
   bind the host/model/runtime facts and external output root through the
   existing live-executor contract.
4. An external output root with the executor-created arm directory/result
   destination. Historical EU7 outputs and repository paths remain forbidden.

The driver does not create authority for the TC-5 remediation or a SCALE-02
claim. Those remain separate, evidence-gated Track Runner actions.
