---
title: FathomDB 0.8.28 Slice 70 — native vector eligibility
status: approved by Astra; implementation verification pending
target_release: 0.8.28
---

# Slice 70 plan

Baseline: `1c65e66250c0e8f3bd919a2c3f6879369921eb89`, existing
`dev/0.8.28` checkout, one writer. No new branch or worktree.

## Reconciliation since the master-plan draft

1. Slice 60 landed format 35, 64-row chunks, independent vector IDs and
   coverage spans. Extend that unpublished layout; preserve spans and f32
   bytes through attribute reshapes and recentering. Refuse old physical
   layouts rather than add a migration.
2. Slice 50 bounded pending work; Slice 60 qualified clean-store latency and
   recall. Reuse their production paths; do not import experimental cache,
   deferred hydration or drain patches. D28-12 remains shared latency work.
3. The current fallback combines a store-wide direct-state scan with source
   dependency safety. Remove only the direct-state scan: source dependencies
   and incomplete closure barriers must remain fail-closed.
4. The named lifecycle files omit `actuation.rs`, which independently changes
   node state. Include it. Edge validity uses `t_invalid`, not `t_valid`;
   match the current read contract rather than invent a start constraint.
5. The draft's upper sentinel needs an exact exclusive-end translation at
   i64 endpoints. Store inclusive upper bounds (finite end minus one), with
   a reversed empty interval for finite MIN. Property-test against nullable
   canonical predicates.
6. Attribute capacity becomes ten plain filterable columns (six built-in
   metadata columns consume the sixteen-column vec0 limit). Existing direct
   SQL fixtures must supply the new mandatory columns. Reconcile fixtures
   without weakening their result assertions.
7. Slice 80 owns historical membership and public view flags; Slice 90 owns
   direct filtered body FTS. No chunk production, historical backfill, new
   public options, ANN index or schema migration is allocated here.

## Execution and acceptance

[Design](design.md) is reviewed by Astra before code. Requirements:

- R70-1: all vec0 creation/insertion paths carry lifecycle and validity;
  fresh/reopen/rebuild/reshape/recenter preserve the three fields and spans.
- R70-2: lifecycle and validity changes update every vector ID mapped to the
  affected record in the canonical transaction, including actuation,
  supersession, soft closures, reactivation and consolidation. Erasure
  removes every mapped vector. An unmapped physical head refuses erasure;
  failures roll back canonical, FTS and vector changes.
- R70-3: default dense KNN filters direct eligibility before its 192-candidate
  cap. Superseded, inactive, future and expired distractors cannot disable
  dense retrieval or consume its candidate slots. Preserve dependency fallback.
- R70-4: native selection matches the reference nullable predicates at start,
  exclusive end and i64 extremes; clean-store results and f32 scores remain
  unchanged. Frozen visibility and filter bind ordering remain correct.
- R70-5: ten filterable attributes work; exceeding vec0 capacity fails
  atomically. No public search-signature or view-flag change.

Commit failing tests with observed RED evidence, then a separate GREEN per
behavior group: physical metadata and preservation; transactional upkeep;
native candidate filtering and fallback removal; record-keyed erasure.
Property tests exercise validity translation. Existing guards that pass on
arrival are coverage, not RED. Astra reviews implementation; a separate
subagent runs affected tests independently. Run the full agent gate once on
the final code tree, with scoped reruns for specific repairs. Reuse AC-013
for the latency blast radius; no new 2M layout or GPU recall sweep is needed
for unchanged embeddings/rerank. Record actual outcomes and limits in status,
update release JSON/derived views and handwritten board, fast-forward the
existing release branch and push explicit refs. No tag/publication.
