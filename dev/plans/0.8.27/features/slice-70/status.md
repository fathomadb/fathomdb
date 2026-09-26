---
title: FathomDB 0.8.27 Slice 70 - implementation status
status: COMPLETE
implemented_on: 2026-09-26
planning_commit: 4da6a8c5
implementation_candidate: 36fc2352cf243e022315ea302368d9424096aebd
closeout_commit: e3ab55f683cf797ea020fcd8dea2eb927d6ec86c
---

# Slice 70 implementation status

## Completed scope

Slice 70 is complete on `release/0.8.27`. It moved the root-owned
projection, vector, mean, embedding, registry, runtime, worker, commit,
rebuild, and standalone reranking implementation out of
`fathomdb-engine/src/lib.rs` into private modules. `lib.rs` shrank from 25,899
to 19,001 lines. No behavior changed. The new modules are:

- `vector_storage.rs` and `vector_equivalence.rs`;
- `mean.rs` and `embedding.rs`;
- `projection_registry.rs`;
- `projection_runtime.rs`, `projection_worker.rs`, and
  `projection_commit.rs`;
- `projection_rebuild.rs`, gated on `operator` like all of its items; and
- `rerank.rs`.

`projection_generation.rs` gained the projection and embedding-readiness
status reads.

`Engine` stays at the crate root. The three public reranker functions keep
their root paths through `pub use`. `ProjectionRuntimeShared` keeps its
shape, including four search-owned fields handed off to Slice 80. The search
paths, the open path, the operator diagnostics, the runtime constants, and
every `*_for_test` seam remain for Slices 80, 90, and 140. Schema (34), SQL,
wire, bindings, and packages are unchanged.

## Plan reconciliation and requirements

A pre-commission review raised seven findings. All seven were verified
against the code and accepted, some with corrections (`plan.md`):

- **Recovery:** the recovery wording now separates pending rediscovery from
  terminal failures, which only a governed rebuild retries.
- **Readiness:** readiness is member-scoped.
- **Slice boundaries:** the Slice 80/90 boundaries were fixed.
- **Configuration gap:** the configurable-pool and timeout gap is assigned to
  Slice 90 (ledger seq 258).
- **CUDA executor:** the named executor is this host's RTX 3090s.
- **Stale metadata:** AC-079 was corrected in `dev/interfaces/rust.md`. The
  dual-runtime ADR status goes to Slice 140 (seq 259).

The release-local requirements are R27-70A-H.

Two review cycles produced the design. The first was FAIL on text-only
findings; the second was PASS-WITH-FIXES (`design-review.md`).

## Tests

- **Residue test:** `tests/slice70_projection_commit_residue.rs` proves that
  a failed projection commit leaves no terminal, sidecar, vec0, or
  failure-audit row. It covers both a success arm and a failed-outcome arm.
  It was shown non-vacuous by a mutant.
- **Classifier:** a drafted classifier table duplicated the existing in-crate
  `completion_classifier_is_closed_over_every_persisted_shape`, so it was
  discarded.
- **Audit:** the slice35 virtual-mutation audit was red before this slice.
  It was repaired (seq 257, now done) and re-keyed through the move.
- **Manifest:** the slice35 manifest extractor was hardened so a body cannot
  cross a file boundary or a `pub(crate) fn`.

## Review and verification

- **Code review** (Opus 5.5, high): PASS-WITH-FIXES, with no P1 and no
  semantic change. All findings were closed (`code-review.md`).
- **Independent verification** (Sonnet): PASS at `36fc2352`
  (`review-verification.md`). The AC-037 live layer it could not run was run
  afterwards and passed.
  - **Canonical gate:** 126/127 at first. The failing Windows WAL guard was a
    move side effect, fixed by a path-only retarget.
  - **Workspace:** Clippy and check pass.
  - **Python receipt:** passes.
  - **Surfaces:** the public surface equals the Slice 30 baseline. The hidden
    surface is additive only (the residue tests).
  - **Feature-complete on the 3090s:** 349/357 passed, 0 failed,
    8 documented ignored. It includes CUDA-selection and tolerance evidence.
  - **Strict security with live AC-037:** 0/0/0, after a temporary per-binary
    AppArmor `userns` profile for `/usr/bin/unshare`.

## Open items

- **AC-037 live layer:** passed for Slice 70 through a temporary AppArmor
  profile. Slice 150 must still run it on its qualification executor.
- **Slice 80:** the owner of `ProjectionRuntimeShared`'s four search fields.
- **Slice 90:** the configurable embedder pool and timeout forwarding (seq 258).
- **Slice 140:** the dual-runtime ADR status (seq 259).
- **Future Slice 40 design edit:** the table's precedence for a failed,
  unenrolled edge.

## Cleanup

No worktree or branch was created; all work ran in the release worktree.
Scratch captures, the feature-complete logs, and review build directories
live in the session scratchpad. The regenerable
`target/debug/incremental` cache was removed twice to meet the comparator's
100 GB free-space floor. Tags, registries, and publication were not touched.
