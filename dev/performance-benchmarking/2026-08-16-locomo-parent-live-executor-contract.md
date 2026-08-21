# LOCOMO-01/PARENT-01 external live-executor contract

**Tracks:** `LOCOMO-01`, `PARENT-01`  
**Date:** 2026-08-16  
**Status:** implementation preparation only; no release or benchmark execution

## Purpose

`experiments.locomo_live_executor` is the distinct, external-process boundary
for the already-frozen Phase-B plan. It does not replace
`experiments.locomo_phase_b`: that module continues to own grid expansion,
metric completeness, and the common receipt/index compatibility contract.

The local runner accepts only a coordinator-issued,
`locomo-live-executor.release.v1` record. It does not create a release record,
acquire a corpus, select a device, invoke a model, or append a repository
receipt/index row. `validate` and `preview` are safe. `execute` is deliberately
unusable until a later coordinator release supplies all real external paths and
digests.

## Frozen configuration and actions

The typed local configuration is
`experiments/configs/locomo-01/live-executor.v1.json`. It hash-pins both the
current runner source and the canonical Phase-B configuration digest. It
resolves exactly three separately released actions:

| Action | Mode | Cells | Question count | Release rule |
| --- | --- | ---: | ---: | --- |
| `fixed_subset_dry_run` | `dry_run` | the existing five listed cells, in order | 32 | CPU-only; produces a complete `dry_run_proof` projection |
| `cpu_grid` | `full_grid` | all 26 CPU cells, including the two PARENT cells | 1,536 | CPU-only; not index-eligible alone |
| `gpu_ce_grid` | `full_grid` | all 26 GPU cells, including every GPU cross-encoder and the two PARENT cells | 1,536 | CUDA required; CPU fallback forbidden; not index-eligible alone |

The CPU and GPU/CE cell sets are disjoint and exactly cover the frozen 52-cell
Phase-B grid. The GPU/CE action is named for the authorized GPU and
cross-encoder lane, but retains the non-CE GPU controls so the full grid is not
silently incomplete.

## Release record

Every release record is strict JSON with duplicate keys rejected and has a
`release_sha256` over canonical JSON excluding that field. It must bind:

- the current integrated Git SHA, the frozen Phase-B config digest, the local
  executor config digest, and the runner source digest;
- one accepted independent-review Git SHA and exactly the `seq-249` and
  `seq-250` authorizations;
- exactly one dry-run action or the explicit pair of separate `cpu_grid` and
  `gpu_ce_grid` gates. The pair shares one release identity/configuration so
  its closure cannot collate evidence from different releases;
- a matching GPU policy: the paired CPU/GPU release freezes `cuda:<index>` and
  forbids CPU fallback; CUDA availability is enforced when `gpu_ce_grid` is
  invoked (the CPU action does not probe or use that device);
- absolute, pre-existing external-only artifact, corpus, turn-provenance,
  session-provenance, fixed-subset, TRACE-sidecar, and parent-relation-proof
  roots; and
- an absolute executable cell adapter with an exact SHA-256; and
- an external, hash-pinned accepted review-evidence record. Its review SHA,
  release binding, integrated SHA, and Phase-B/executor/runner digests must all
  agree with the release.

The runner rejects a release before launching the adapter when its self-hash,
integration SHA, configuration, runner, authorization, action, GPU policy,
path, input digest, full TRACE lifecycle proof, canonical-manifest-bound parent
membership proof, review evidence, or adapter digest differs. Repository paths
and any `experiments/runs` historical output path are rejected. A GPU action
also calls the local CUDA enumerator before the adapter starts and requires an
adapter attestation of that exact selected CUDA device.

## External adapter protocol

The released executable receives one safe JSON request per frozen cell on
stdin. It receives only the released action/cell metadata and external input
locations. It must return exactly one
`locomo-live-executor.cell-result.v1` JSON object on stdout. Unknown fields,
raw strings in metric mappings, non-scalar metrics, a wrong cell/mode, or an
unsafe external metrics reference fail closed.

The runner rejects duplicate JSON keys before inspecting an adapter result. It
validates M1/M2/M4-proxy/M6/M7 and class metrics through the
Phase-B contract before retaining a result. A PARENT result must additionally
provide the existing required parent metrics and `parent_hits` relation proof.
Those hits are checked against both:

1. a hash-pinned, complete `trace-projection.v1` sidecar. Its complete source,
   projection/searchability, supersession, lifecycle outcome, and diagnostics
   semantics are validated before referenced sources may be active; and
2. a hash-pinned `locomo-parent-relation-proof.v1` external inventory. Each
   entry records only safe child/session/turn identifiers, ordinal, TRACE
   source identifier, canonical turn/session provenance fingerprints, and
   same-session member ordinals. Its own pinned turn/session manifest digests,
   every child membership, parent session, and ordered members must match those
   canonical manifests exactly before the bounded Phase-B parent bundle is
   formed.

This proves the approved exact-enclosing-session, hybrid-top-10,
rank-preserving, at-most-five-bundle, one-neighbor-per-side treatment without
copying corpus text, questions, hits, answer output, or credentials into the
projection.

## Output and closure boundary

The runner accumulates all required action results before writing exactly one
external `locomo-live-execution-projection.v1`. It creates no partial result
projection and refuses a duplicate action projection. The projection contains
only safe IDs, hashes, aggregate scalar summaries, and the bounded safe parent
context. It contains no external path or raw payload.

Only a completed five-cell dry projection is eligible for a Phase-B
`dry_run_proof` receipt. Each CPU or GPU/CE projection explicitly remains
ineligible until `finalize-full-grid` reads both same-release/config action
projections and verifies exactly 52 unique, complete cells. It rejects mixed
release IDs, self-hashes, config/runner digests, partial coverage, and
duplicates. Only that full-grid closure is receipt/index eligible; the
coordinator then—not this worker—may create the ordinary safe receipt and
append its index row.

## Future command and factual prerequisites

After independent acceptance, integration, and a new coordinator release, the
first permitted invocation is structurally:

```bash
python -m experiments.locomo_live_executor execute \
  experiments/configs/locomo-01/live-executor.v1.json \
  /absolute/external/locomo-parent-release.json \
  --action fixed_subset_dry_run
```

Before that command, the coordinator must provide the exact current integrated
SHA, release self-hash, reviewed adapter digest, pre-existing access-controlled
external root, byte-matching corpus/provenance/subset files, an accepted TRACE
sidecar, complete parent-relation proof, and a device policy matching the
released action. This worker has supplied none of those external artifacts and
has not run this command.
