# Performance PROGRAM completion execution hand-off

**Date:** 2026-08-17
**Role:** Track Runner coordinator
**Goal:** complete the already-authorized `LOCOMO-01`, `PARENT-01`,
`SCALE-01`, and `CORPUS-01` work with qualified external evidence and safe
receipts.
**Authority:** HITL authorizations `seq-249` and `seq-250`, plus the stated
goal. This hand-off does not authorize `ANSWER-01`, `MEMORY-01`, `SCALE-02`, a
product claim, a paid service, a substitute corpus, or a floor relaxation.

**2026-08-21 amendment:** the released 0.8.23 baseline and its
[GPU-primary amendment](2026-08-21-fathomdb-0.8.23-gpu-primary-amendment.md)
supersede this historical hand-off's incompatible CPU-first and unavailable
NVIDIA-driver instructions. Preserve this document as the prior execution
packet; use the amendment for new LOCOMO/PARENT GPU cells and the future TC-5
v2 release path.

## Start here

Work only in `/tmp/fathomdb-performance-experiments-20260815` on branch
`experiments/performance-experiments-20260815`. Before any mutation or external
action, verify the actual branch and HEAD, then run:

```bash
./scripts/track-runner.sh check
./scripts/track-runner.sh status
./scripts/track-runner.sh brief LOCOMO-01
./scripts/track-runner.sh brief PARENT-01
./scripts/track-runner.sh brief SCALE-01
./scripts/track-runner.sh brief CORPUS-01
```

Treat Git, the current `TRACK-RUNNER-STATUS.md`, the dated contracts, and safe
external artifacts as authoritative. Do not rely on this document's starting
SHA or on prior narration. The coordinator alone updates `PROGRAM.md`,
`TRACK-RUNNER-STATUS.md`, shared helpers, shared conventions, receipts, and the
experiment index. Use one writer per isolated worktree for any code or
track-specific contract correction; require independent read-only review before
integrating it.

## Non-negotiable evidence boundary

- Keep corpus payloads, questions, answers, retrieved text, predictions,
  credentials, databases, model output, and raw paths out of Git, receipts,
  Track Runner status, and handoffs. Use only allowed hashes, safe identifiers,
  counts, fixed diagnostics, and logical artifact names.
- Validate every external input against the frozen contract before any model,
  GPU, adapter, driver, or benchmark action. Do not replace a missing or
  mismatched input with a similarly shaped dataset, synthetic padding, a
  historical EU7 output, or a new pin.
- A coordinator release is action-specific and self-hashed. Bind it to the
  then-current integrated SHA, exact configuration/runner/adapter/manifest
  digests, qualified external roots, accepted review evidence, and the named
  HITL authorizations. Validate it before creating a directory or loading an
  input.
- Preserve red-first tests, clean commits, worker handoffs, independent review,
  and coordinator-only integration. Use Track Runner's check/status/brief
  sequence at each lane start and transition.
- If a prerequisite cannot be established from authorized real material, write
  only a content-free blocked report, update Track Runner, and stop that lane.
  Never convert a blocker into a result or claim.

## 1. Resolve and qualify LOCOMO/PARENT inputs

The historical raw LOCOMO input is available and its canonical normalized hash
matches the frozen Phase-B pin when derived with `eval.locomo_loader`. The
integrated qualification correction is the authority for that distinction.

Resolve the remaining inputs outside Git:

1. Locate or recover the frozen 32-question
   `locomo-fixed-subset.v1` whose SHA-256 is pinned by
   `phase-b-execution.v1.json`. Verify its IDs and count against the qualified
   raw corpus. Do not generate a convenience subset or alter the pin.
2. Locate the byte-matching canonical turn and session provenance manifests.
   Validate their SHA-256 values and structural membership.
3. Establish an unambiguous, canonical parent-relation proof compatible with
   `parent_child_turn_session_v1`: exact enclosing session, contiguous ordered
   members, child top-10 rank preservation, at most five session bundles, and
   at most one neighbor per side. Do not namespace, rewrite, or otherwise
   repair the old ambiguous child IDs invisibly.
4. Run `experiments.locomo_input_qualification` into a new empty external
   preflight root. Continue only if its report is `qualified`, its TRACE sidecar
   is valid, and its canonical parent proof exists. A blocked report is a stop,
   not a release candidate.

If the pinned subset or manifests cannot be recovered, prepare a factual
amendment package naming the exact incompatibility, candidate source, hashes,
and required representation change. Do not change the frozen Phase-B config or
execute a cell until the amendment receives the required independent review and
coordinator/HITL ruling.

## 2. Run the LOCOMO/PARENT executor in order

After qualification, copy or install the reviewed external adapter as required
by its contract and create one release record per authorized action. Use
`experiments.locomo_live_executor` only through its validated release path.

1. Release and run `fixed_subset_dry_run` first. It must execute the five
   frozen CPU cells in listed order, including the PARENT cell, over exactly 32
   questions. Review the complete safe dry-run projection before any full grid.
2. Release and run `cpu_grid` next: all 26 CPU cells, including the two PARENT
   cells, over 1,536 evidence-backed questions. Preserve per-class M1, M2,
   M4-proxy, M6, and M7 evidence.
3. Only after a real CUDA device is available and attested, release and run
   `gpu_ce_grid`: all 26 GPU cells. CPU fallback is forbidden. A declarative
   CUDA setting, `nvcc`, or an unavailable `nvidia-smi` device is insufficient.
4. Run `finalize-full-grid` only after the exact same-release/config CPU and
   GPU projections provide all 52 unique complete cells. Only this closure may
   make a safe Phase-B receipt and index projection eligible.

Do not report a CPU-only grid as a completed Phase-B program. Keep PARENT
metrics separate and include the approved child evidence, parent/session
recall, duplicate-rate, context-expansion, and class-latency evidence.

## 3. Resolve and qualify TC-5 inputs

Build a real, external `tc5-input-inventory.v1` and `tc5-manifest.v1`; never
reuse historical EU7 output or LOCOMO material. The inventory must bind an
eligible CORPUS-01 source and license copy, exact all-real primary 18,472
documents, canonical first-7,667 bridge, source revision/artifact, pinned
model asset, CPU-host, exact-f32 ground truth, vector-stage runtime, and
external-output attestations.

Run the TC-5 factual qualifier into a new external preflight root. It must
produce `factual_inputs_qualified`; a `blocked_prerequisite` report must remain
a stop. Provide a reviewed external driver implementing the strict arm-result
ABI, with a digest bound into the release. Reserve the qualifying CPU host and
verify no GPU selection, no synthetic documents, and no missing cached model
asset.

## 4. Run SCALE-01 in order

Create a strict `tc5-execution-release.v1` bound to the then-current integrated
SHA, live/execution configuration hashes, qualified manifest, external driver,
and one action. Use the release-gated `experiments.tc5_live_executor`.

1. Run `tc5-smoke`, always bridge before primary. Validate both safe arm
   sidecars and the two-arm receipt; a one-arm smoke is not a TC-5 result.
2. Review the smoke receipt. If it is qualified, issue a separate release for
   `tc5-long-cpu-characterization` and run bridge then primary again as frozen.
3. Review the complete long-run receipt. If the primary result is below the
   0.90 goal, run the approved ground-truth remediation under the identical
   qualified manifest, host, model, query set, seeds, `K=192`, and bootstrap
   procedure. Report `ground_truth_defect`, `sut_configuration_defect`,
   `qualified_below_goal`, `qualified_goal_observed`, or `inconclusive` exactly
   as the remediation contract defines. Never relax the floor automatically or
   make a SCALE-02/product claim.

## 5. Complete CORPUS-01 with real human-gold evidence

Select actual authorized corpus/category pairs and create content-free factual
preflight bindings outside Git: source payload and license-copy hashes,
revision, class counts, exclusions, supported metric, paired-power artifact,
and claim binding. For unsupported native pairs, obtain the versioned qualified
human-gold amendment and matching coordinator-approved registry entry before
the category can count.

For `knowledge_update`, `supersession`, `source_erasure`, and
`time_scoped_validity`, conduct two independent blinded human reviews of the
external source material. An agent is not a human reviewer and must not
manufacture labels, answer oracles, or adjudication. Retain worksheets and
source text only externally. Record only the validated, content-free manifest,
agreement/exclusion/count summaries, paired-power evidence, and claim scope.

## Closeout and acceptance

For every completed action, confirm the exact external result set and safe
receipt before appending the existing experiment index. Independently review
each executor's result and then perform one cross-track review of provenance,
configuration, metric definitions, CPU/GPU boundaries, and claims. Update
Track Runner after every qualification, release, run, review, blocker, and
integration.

The goal is complete only when all four authorized tracks have their required
qualified external evidence and reviewed safe receipts: LOCOMO/PARENT has the
full 52-cell closure, SCALE-01 has reviewed smoke and long CPU evidence,
CORPUS-01 covers all four lifecycle categories with valid human-gold evidence,
and no claimed result exceeds its receipt and contract scope.
