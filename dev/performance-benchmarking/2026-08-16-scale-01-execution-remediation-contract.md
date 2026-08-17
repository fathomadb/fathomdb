# SCALE-01 TC-5 execution and ground-truth remediation contract

**Track:** `SCALE-01`

**Date:** 2026-08-16

**Status:** implementation preparation only. This contract does not acquire a
corpus, inspect corpus payloads, load a model, invoke EU7, run a smoke or long
characterization, write a campaign artifact, or make an external request.

## Execution-release contract

`experiments/configs/scale-01/tc5-execution.v1.json` is the frozen,
deliberately disabled configuration for the authorized TC-5 smoke and long CPU
characterization. `experiments.tc5_characterization` composes the unchanged
`tc5-manifest.v1` validator and produces a content-free
`tc5-execution-receipt.v1` only.

Before a live invocation, an independent reviewer and the Track Runner
coordinator must jointly release a reviewed configuration revision. Until then,
the runner refuses execution. A release must retain all of these pins:

| Control | Frozen value |
| --- | --- |
| Arms | Canonical 7,667-document bridge and 18,472-document primary, both required |
| Documents | Manifest-qualified real documents only; no padding or substitutions |
| Backend | CPU same-backend fidelity characterization |
| Model | `fathomdb-bge-small-en-v1.5`, with manifest asset hash |
| Vector stage | Pre-fusion 1-bit `K=192` plus f32 rerank / exact-f32 same-model top-10 ground truth |
| Query/bootstrap | 100 queries; seeds `0x0E77C0125E1EC7` and `0x0E77B007574A9`; 1,000 resamples |

The invocation accepts only an existing external `tc5-manifest.v1`, external
corpus root, and external output root. Repository paths and the historical
`dev/plans/runs/eu7-latest-measurements.json` are rejected. Any later receipt
may contain only hashes and logical references, never document IDs, corpus
payloads, raw paths, prediction payloads, or raw metrics. It becomes eligible
for the normal `experiments.index-row.v1` append only when both arms have
complete safe results.

TC-5 reports fidelity and uncertainty. It makes no SCALE-02 capacity,
latency, SLO, release, or product-support claim.

## Current-configuration ground-truth remediation

The remediation is a diagnosis of what the current configuration measures; it
is not an assertion that the 0.90 goal passes and it does not change that goal.
It starts only after the primary TC-5 receipt is complete and an independent
review confirms its provenance and completeness.

### Controlled comparisons

Run these comparisons under the same qualified manifest, CPU host class,
model asset hash, query set, seeds, `K=192`, and 1,000-resample procedure:

1. **Ground-truth replay:** recompute exact-f32 same-model top-10 from the
   manifest-qualified primary arm and compare its stable IDs to the recorded
   ground-truth artifact digest. A mismatch is a ground-truth defect, not a
   fidelity result.
2. **SUT replay:** rerun the pre-fusion 1-bit candidate plus f32-rerank vector
   stage against that exact ground truth. Confirm no fused-search path,
   synthetic row, changed candidate breadth, or different embed device entered
   either arm.
3. **Bridge diagnostic:** repeat the same comparison on the 7,667-document
   canonical bridge. It establishes snapshot behavior only and is not a
   regression claim against historical EU7 output.

No arm may change two variables at once. Any code, model, corpus-manifest,
feature-set, host, seed, or ground-truth change creates a separately labeled
diagnostic cell rather than a substitute result.

### Uncertainty and outcome handling

Each qualified cell records recall@10 point estimate, 95% bootstrap interval,
bootstrap sigma, query completion count, exact-ground-truth digest, and SUT
result digest. The historical 0.90 predicate is reported as context only.

| Outcome | Meaning | Next action |
| --- | --- | --- |
| `ground_truth_defect` | Exact-f32 replay differs from pinned ground truth | Correct the artifact contract; do not quote a fidelity value. |
| `sut_configuration_defect` | SUT replay has fused, CPU/model, `K`, corpus, or completeness drift | Correct the configuration; rerun the affected qualified cell. |
| `qualified_below_goal` | Complete primary cell reports the goal predicate unmet | Keep 0.90 as the goal; return evidence and remediation options to HITL. |
| `qualified_goal_observed` | Complete primary cell reports the predicate met | Record bounded fidelity evidence only; no SCALE-02 claim follows. |
| `inconclusive` | Missing arm, provenance, queries, bootstrap, or repeatability | Repair the defect before any interpretation. |

The remediation may produce a diagnosis and bounded alternatives, but no
automatic floor relaxation, release conclusion, or capacity claim.
