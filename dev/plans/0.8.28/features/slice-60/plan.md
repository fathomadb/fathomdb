---
title: FathomDB 0.8.28 Slice 60 — plan
status: approved; implementation awaiting final review and qualification
target_release: 0.8.28
observed_on: 2026-11-01
---

# Slice 60 plan (`STORE-FORMAT-35`)

Baseline: `13d718ec8`, completed Slice 50 on `dev/0.8.28`. Work directly in
that existing checkout, with one writer. No Steward or Orchestrator.

## Changes since the draft and disposition

1. The draft is the Slice 60 outline in `dev/plans/plan-0.8.28.md`; no
   separate slice plan existed. Retain format 35, 64-row chunks, FTS rowid,
   refusal, rebuild/recentring and the successor ADR.
2. Slice 42 adopted SQLite 3.54.1 with per-connection caches. Slice 50
   bounded both pending-work scans and closed provider admission changes.
   Measure on those shipped changes; do not transplant experimental cache,
   eligibility or drain patches from the AC-013 research branches.
3. The 2026-10-30 recommendation on `dev/0.8.29` predates the owner ruling
   assigning FTS rowid to this slice and native eligibility to Slice 70.
   Retain FTS rowid here; reject eligibility caching and deferred hydration
   here. D28-12 is the allocated latency item, shared across Slices 50–70.
   D28-14 is Slice 90; other draft candidates are assigned elsewhere.
4. The 2026-10-31 B2-8 ruling adds head-window coverage storage and an
   embedder truncation method. The overflow re-evaluation on `dev/0.8.29`
   requires readiness without enabling chunking. The current vector mapping
   has `write_cursor UNIQUE`: remove that storage restriction for fresh
   format-35 stores and provide a separate vector ID plus record/window
   identity. Production still writes one head vector per record.
5. Existing open admission already refuses every non-current version before
   normal open. Reuse it and prove format-34 refusal without mutation; do
   not add a second compatibility path. The vec0 sentinel currently accepts
   any SQL containing `status` and contains old upgrade branches: replace
   that permissive policy with a current-layout check.
6. CS64 evidence stops at 300k rows and has a Hamming-cutoff tie difference.
   Qualify 64/128/256 to 2M vector rows before freezing; report tie-aware
   equivalence and eu7 recall, not a false byte-identical ranking promise.
   Keep one short status record, no generated benchmark data in git.

## Work sequence

1. Astra reviews [requirements](requirements.md), [design](design.md) and
   the successor ADR before implementation.
2. Add genuine failing tests, commit RED, then GREEN for each behavior:
   format/refusal/shape; explicit FTS rowid; span schema and transactional
   upkeep; truncation reporting and dispatch propagation; physical completeness and
   custom-trigger visibility. Bump the independent API axis to 0.8.0. Passing existing
   guards are not presented as RED evidence. Add property tests for spans
   and projection round trips.
3. Implement from the reviewed design, update Rust interface and relevant
   format documentation. Reuse production rebuild and recenter paths.
4. Keep a reproducible vector-only 2M benchmark in the repo, adapted from
   the AC-013 prototypes; test measurement correctness. Run AC-013,
   report-only ac_013b, and real BGE eu7 on RTX 3090. Record actual results
   and decide the layout using the rule below before accepting the ADR.
5. Astra code review and a separate test agent verify the affected suites.
   Run the full agent gate once on the final code tree; fix individual
   findings with scoped reruns, without repeated full regressions.
6. Record status, review outcomes, benchmark limits and changes in the
   release JSON/derived views and CHANGELOG; commit and push dev/0.8.28.
   No new branch or worktree is planned, so none needs cleanup.

## Measurement decision

Use the same deterministic vectors and queries, SQLite cache settings,
192-candidate binary KNN and f32 rerank across layouts. Include 10k, 100k,
300k, 1M and 2M vector rows, recording seed time, disk bytes, bit-KNN,
rerank and total vector-stage p50/p95. Map counts to 1 and 5.1 vectors per
record (2M vectors is about 392k long records); do not label this an actual
chunked retrieval evaluation. Build/run cells serially to bound disk use.
CS64 is retained if it passes AC-013, preserves the recall contract, and
no other measured layout dominates its vector-stage p50 at every count.
Report measured crossings and high-scale cost explicitly. If CS64 fails
that rule, present the measured tradeoff before changing the owner-selected
layout; do not silently introduce R8/F32T or an ANN index.
