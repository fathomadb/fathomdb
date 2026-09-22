---
title: FathomDB 0.8.27 prework design review
status: COMPLETE
target_release: 0.8.27
reviewed_on: 2026-09-21
---

# 0.8.27 prework design review

## Verdict

**PASS.** An independent design-review subagent reviewed the complete Slices
0-9 planning package against the shipped implementation, accepted interfaces,
tests, lifecycle registry, roadmap, and deferred scope. No P1-P4 finding
remains unresolved.

## Findings and resolution

1. **Erasure transaction phases (P2).** The initial draft incorrectly treated
   every incomplete erasure as an atomic rollback. The approved plan now
   distinguishes pre-commit rollback from the accepted post-commit
   `ErasureIncomplete` contract: canonical deletion is durable, the scrub
   obligation persists, retry is idempotent, and incomplete work never reports
   successful closure.
2. **Retained identity and report semantics (P2).** The initial physical-
   absence oracle conflicted with retained non-PII proof records. The approved
   plan preserves requested-bucket `ExciseReport` counts and permits raw
   `source_id` only in exact audit/closure fields with exact row/content counts;
   it forbids a broad identity allowlist.
3. **Memex acceptance strength (P2).** The existing consumer characterization
   contains only a failure oracle. Slice 20 must retain its scenario/setup,
   replace that oracle with exact receipt/count, restart/visibility, physical-
   absence, and survivor assertions, then freeze the revised consumer test
   through Slice 150.
4. **Allocation and lifecycle coherence (P3).** The 0.9 proposal and lifecycle
   registry now identify the 0.8.27 plan as successor authority. D28-03 retains
   both graph and `operational_state` continuation and both proof gates. Slice 0
   verifies the existing workspace; Slice 9 owns state/board activation.
5. **Scope and status wording (P4).** The plan names five selected monolithic
   facades rather than incorrectly calling them the five largest files, and all
   governed documents use canonical status values.

## Focused evidence

The review confirmed passing `git diff --check`, plan-status lint,
design-status lint, design-lifecycle validation, plan-anchor validation, and
design-reference validation. Independent closeout verification remains a
separate Slice 9 gate after release-state and board activation.
