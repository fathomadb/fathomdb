---
title: 0.8.25 Slice 72 — installed CE profile and generic preflight
status: READY
depends_on: 71
design: design.md
design_version: 2
---

# Slice 72 plan

## Outcome

Deliver two focused release-confidence improvements:

1. make `scripts/preflight.sh` use the active release-state record instead of
   hard-coded 0.8.23 data and plan prose; and
2. retain source-independent CPU and CUDA cross-encoder correctness and
   performance profiles for the branch-point and the 0.8.25 candidate.

Slice 71 is complete and supplies the dependency. Slice 72 does not repeat its
read/write investigations, change CE product semantics, alter shipping feature
defaults, or run Slice 75's full regression and packaging matrix.

## Reconciliation of the draft

The draft was directionally correct. Review of the repository since it was
written produces these changes:

- Slice 71 is now complete at `84c056c6`, AC-072 passes, and the release-state
  writer records Slice 72 as next. No Slice 71 performance work remains here.
- `scripts/preflight.sh` is unchanged since the draft and demonstrably rejects
  the valid 0.8.25 worktree because it selects the completed 0.8.23 lifecycle
  and `origin/main` as its baseline.
- `scripts/release-current.py` already implements tracked-only discovery of the
  one live release. Reuse it rather than add another release selector.
- `--expect-closed` still greps plan prose. The 0.8.25 state already has unique
  ladder entries, exact dependency status, and Git-verifiable SHAs; these are
  the correct authority.
- An active release needs an explicit local branch ref. Add `active_ref` to the
  live state; do not misuse the existing `completion` object, because the local
  0.8.25 branch is intentionally ahead of its remote.
- CPU/CUDA CE execution, device policy, pinned weights, semantic fixture, and
  numerical tolerance already exist. Slice 72 adds an installed Python profile
  and validator, not another reranker implementation or the historical full
  CUDA package rehearsal.
- The CE core is byte-identical to the branch point, but `Engine.search` is not;
  retain the planned branch-point comparison for both standalone and end-to-end
  paths.
- The handwritten master-plan rows and board narrative lag the generated state
  after Slice 40. Correct those narrow stale statements during close.

## Requirements and acceptance criteria

### Generic preflight

| Requirement | Acceptance criterion |
| --- | --- |
| S72-R1 — select release state generically | S72-AC1: preflight reuses `release-current.py`, selects exactly one tracked live state, and validates its release, board, plan, and `active_ref`; missing, multiple, malformed, or mismatched state fails closed. |
| S72-R2 — use the correct Git baseline | S72-AC2: an active state uses `refs/heads/release/<release>`; a declared PENDING completion uses `origin/release/<release>`; COMPLETE uses `origin/main` only after the completion ref is reachable there. The target worktree must equal or descend from the selected baseline. |
| S72-R3 — decide dependency closure from state | S72-AC3: `--expect-closed N` requires one exact ladder entry, status `COMPLETE_ON_RELEASE_BRANCH` or `LANDED`, a resolving SHA, and ancestry of that SHA to both the baseline and target worktree. `--plan` may confirm state-plan identity but is not closure evidence. |
| S72-R4 — preserve unrelated gates | S72-AC4: general health use, primary-checkout landing refusal, and downstream landing checks retain their behavior; no release number is embedded in the new path. |

### Installed CE profile

| Requirement | Acceptance criterion |
| --- | --- |
| S72-R5 — prove installed feature-present CE behavior | S72-AC5: isolated CPU (`pyo3/extension-module,default-reranker`) and CUDA (`pyo3/extension-module,rerank-cuda`) wheels import only from their install roots and run the pinned Berlin fixture through standalone `fathomdb.rerank` and a fixed-corpus `Engine.search`. Standalone results contain all six IDs and the known semantic reorder. Engine results match the manifest's retrieved-ID set and contain finite, non-degenerate CE scores in a stable order. |
| S72-R6 — bind device and CUDA allocation truthfully | S72-AC6: CPU reports effective CPU. Forced CUDA reports `cuda:0`, the selected RTX 3090 UUID, and matching-process GPU allocation while inference executes. Skip, fallback, absent allocation, source import, or model acquisition during a measured cell fails that cell. |
| S72-R7 — compare correctness and steady performance | S72-AC7: branch point `4fc1b890a11ebfaa8f11b15823656e856002807a` and the exact candidate run with the same fixture, model bytes, device, affinity, and thread policy. Standalone CPU/CUDA ranks are identical and per-ID CE scores differ by at most `1e-2`; Engine ranks are identical except for manifest-declared score ties within that tolerance. Each commit/device/path has three process-cold and five 20-call steady repetitions; candidate median-of-repetition p95 is no more than 10% above its matching baseline. |
| S72-R8 — retain strict, reusable evidence | S72-AC8: a checked-in manifest, fail-closed receipt validator, raw results, comparison receipt, and status bind commits, artifacts, imports, model/fixture hashes, commands, environment, timings, throughput, RSS/VRAM, and verdicts. Wheel/model/resource sizes are descriptive, not new product limits. |

## Implementation plan

### 1. Preflight RED/GREEN

RED extends the existing throwaway-repository fixtures to prove the present
0.8.25 false rejection and reject missing/multiple/malformed state, identity or
ref mismatch, open/duplicate/missing dependency, bad/non-ancestor dependency
SHA, and a non-descendant worktree. Preserve generic PENDING/COMPLETE and
primary-checkout cases, including dependency-only invocation against the
invoking checkout's HEAD.

GREEN adds `active_ref` to the 0.8.25 state and replaces the release-specific
selection and prose grep in `scripts/preflight.sh` with facts from the active
state. Keep the rest of the script narrow and unchanged.

### 2. CE profile RED/GREEN

RED adds validator fixtures for missing/unknown cells, wrong commit/artifact/
fixture/model identity, source imports, feature or device mismatch, fallback or
skip, missing IDs/reorder, null/non-finite/degenerate scores, CPU/CUDA tolerance
failure, absent CUDA PID allocation, insufficient repetitions, and a greater
than 10% steady-p95 regression.

GREEN adds:

- one compact checked-in manifest containing the fixed Berlin query/passages,
  pool/depth `6`, alpha `1.0`, 20 calls per steady repetition, exact feature
  sets, model hashes, thread/affinity policy, branch point, and thresholds;
- an installed Python worker that records import/open/first-call/steady timing,
  standalone and `Engine.search` results, RSS, effective device, and errors;
- a small runner that invokes isolated processes, binds the candidate and
  baseline artifacts, and captures CUDA UUID/PID/VRAM evidence; and
- a fail-closed validator/comparator that emits the final receipt.

Change product code only if a focused RED reveals a product defect. Do not
change the ordinary feature-off wheel, release workflow, model pin, candidate
pool, eligibility, ranking, or fallback policy to make the profile pass.

### 3. Review, execution, and close

Obtain an independent read-only design review before behavior changes and a
read-only code review after GREEN. Execute only:

- `bash scripts/tests/test_release_current.sh`;
- the focused preflight fixture;
- the new CE validator/runner fixtures;
- directly affected CPU and CUDA CE tests;
- scoped format/lint/type checks for changed files;
- the exact installed CPU/CUDA candidate/baseline campaign;
- release-state view checks and `git diff --check`.

A separate read-only verifier reviews retained results rather than launching a
duplicate campaign. No full `agent-verify`, broad regression, Windows,
cross-SDK, hosted CI, complete CUDA rehearsal, publication, tag, registry, or
`main` merge is part of Slice 72.

Close by writing `status.md`, changing Slice 72 to complete in the state writer,
advancing `next_slice` to 73, regenerating views, and removing temporary build,
venv, and baseline-worktree artifacts. If an isolated branch/worktree is used,
merge it to `release/0.8.25` and remove it.
