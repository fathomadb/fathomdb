---
title: 0.8.25 Slice 75 — integrated closure
status: DRAFT
depends_on: 60
design: design.md
design_status: SCOPE_RECONCILED_FORMAL_REVIEW_REQUIRED
---

# Slice 75 plan

## Outcome and carried obligations

Implement the trimmed subset of R25/AC25-75; the measurement half of Memex need
21, needs 22/24, and the integrated audit portion of need 23 under the approved
[scope adjustment](../../scope-adjustment-2026-09-02.md). Audit—do not
backfill—the retained Slice 10–60 contracts: installed SDK/wire parity,
representative concurrency/lifecycle/evidence paths, selected regression
performance, and retrieval-only evaluation.

## AC-013 vector-latency investigation

Before running the integrated closure matrix, investigate the pre-existing
`ac_013_vector_retrieval_latency` failure exposed by the Slice 10 diagnostic.
This belongs to Slice 75 because it is a release-wide canonical performance
gate; Slice 40 owns projection-generation identity and readiness, not vector
query latency.

Run the AC-072 binding 10k fixture in isolation with the repository's canonical
release-mode AC-013 runner. Compare the release candidate with the pinned
0.8.25 branch-point baseline
`4fc1b890a11ebfaa8f11b15823656e856002807a` under the same host, compiler,
SQLite, vector dimension, corpus seed, and process-isolation conditions. Run
three alternating isolated repetitions per commit. Retain raw logs under
`dev/plans/runs/0.8.25-slice-75-ac013/` and write the comparison to
`ac-013-vector-latency-investigation.md` in this slice directory. The record
must contain every repetition's p50/p99, seed and drain time, host/build
identity, and exact commands and commits.

Classify the result before continuing:

- if both isolated release-mode runs pass, record the earlier failure as a
  debug-build or shared-runner diagnostic artifact;
- if only the release candidate fails, stop Slice 75 and route the regression
  to the slice or change that introduced it;
- if both runs fail, stop release closure and present the pre-existing binding-
  gate failure for explicit disposition; and
- do not relax AC-072, change its fixture, or introduce an optimization
  treatment as part of this investigation.

## Generic release-preflight correction

Slice 20 commissioning found that `scripts/preflight.sh` hard-codes the 0.8.23
completion file and tests dependency closure only through prose in the master
plan. It therefore rejects a correctly based 0.8.25 feature worktree even when
the live 0.8.25 release-state writer records the prerequisite complete.

- **S75-R-PREFLIGHT.** Preflight must discover the one active release-state
  file, validate its exact release/board/plan/ref contract, and decide worktree
  freshness and dependency closure from that state instead of a hard-coded
  release or narration grep.
- **S75-AC-PREFLIGHT.** RED fixtures reproduce the false 0.8.25 stale-base and
  missing-Slice-15 outcomes. GREEN accepts the exact release tip and completed
  prerequisite, preserves 0.8.23 completed-release behavior, and rejects an
  absent/malformed state, wrong release/ref, non-descendant worktree, open
  prerequisite, and primary-checkout landing.

This is release-infrastructure work, not a dependency-product change. Design
review, TDD RED/GREEN, implementation review, and independent verification are
required with the rest of Slice 75.

## Verification routes

Selected: fast, heavy, all, applicable all-feature/operator, Windows CPU/native
Rust/Python/Node, packaged Python/npm/native/CLI, final packaged-candidate
native `Engine.search` witness, the isolated release-mode AC-013 comparison,
and focused CUDA only where a retained dense/graph contract changed.
Live-model, Windows CUDA, pre-publication
registry-installed, and exhaustive scale-by-feature-by-CUDA matrices are N/A.
Actual registry-installed smokes remain a separately authorized
post-publication close gate.

## Draft-to-ready and delivery

Define receipt-presence and agreement criteria, installed wire fixtures,
representative concurrency, evidence/page/dependency overhead,
mutation-to-ready, erasure propagation, selected rebuild/resource regressions,
classification gates, and generic release-state preflight; design a
proportionate workload matrix without semantic answer claims; review; implement
RED/GREEN harness checks; review; execute all selected routes; and record final
release evidence. Stop when an owning slice lacks proof or a data-plane claim
mixes answer-system metrics.
