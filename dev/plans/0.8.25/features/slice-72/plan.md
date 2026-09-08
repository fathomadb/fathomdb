---
title: 0.8.25 Slice 72 — installed CE profile and generic preflight
status: DRAFT
depends_on: 71
design: design.md
---

# Slice 72 draft plan

## Outcome

Deliver two bounded release-confidence improvements: a generic release-state-
aware preflight gate and installed, source-independent cross-encoder CPU/CUDA
profiles. Consume Slice 71 closure; do not repeat its investigations or Slice
75's full regression matrix.

## Generic preflight correction

The current script embeds historical 0.8.23 assumptions and can reject a
correct 0.8.25 worktree. Design a version-neutral contract that discovers the
single active `dev/plans/release-state-*.json`, validates release, board, plan,
ladder, dependency, and Git-verifiable completion refs, and fails closed on
missing/multiple/malformed state, wrong identity, open dependency,
non-descendant source, or primary-checkout landing.

The active release-state format currently lacks a fully explicit active-ref
and completion-object contract. Resolve that representation honestly in the
design before coding; do not infer closure from board prose. RED fixtures must
reproduce the 0.8.25 false rejection and every fail-closed case. GREEN must
preserve valid completed-release behavior without embedding a release number
in product logic.

## Installed CE CPU/CUDA profile

Build exact-commit, source-independent CPU (`default-reranker`) and Linux CUDA
(`rerank-cuda`) artifacts using one pinned model and checked-in query/passage
fixture. Prove the CE is active through finite, non-degenerate scores and a
known semantic reorder; feature-off identity behavior is separate.

Pin query/passages/order, candidate pool, rerank depth, alpha, model/cache,
threads, affinity, and warmed bytes. Record requested/resolved device, CUDA
allocation, package/model size, import/open/cold load, standalone inference,
end-to-end `Engine.search`, throughput, RSS/VRAM, errors, and per-repetition
p50/p95/p99. Require at least three process-cold and five steady repetitions.
Compare candidate and branch-point baseline under identical hardware. Any
correctness divergence, silent fallback, absent allocation witness, or steady
p95 regression above 10% stops closure. Absolute metrics remain descriptive
unless an accepted policy supplies a stricter gate.

## Verification and handoff

Use TDD, independent design/code review, focused preflight and reranker tests,
and only the CPU/CUDA profile cells above. Do not run full repository, Windows,
cross-SDK, hosted-CI, or release publication routes. Write strict manifest,
receipt, and status records for Slice 75 consumption. Publication, registry
mutation, tagging, and `main` merge remain unauthorized.
