---
title: Slice 35 implementation review — FIX-1
status: RESOLVED_PENDING_REVIEW
reviewed_commit: e8f9e5a6
date: 2026-09-04
---

# Slice 35 implementation review — FIX-1

The independent review returned `CHANGES_REQUIRED` with one P1 and seven P2
findings. FIX-1 resolves the implementation defects and adds the missing named
verification surfaces; the exact correction commit is recorded after commit.

## Disposition

- **Graph dependency eligibility (P1): not reproduced.** Both
  `ReadView::node_sql`/`FrozenView::node_sql` and
  `edge_validity_sql_for_view` already append the Slice-30 dependency predicate.
  Adding another predicate would create a second authority and duplicate SQL.
  Existing dependency-closure graph tests remain the executable proof.
- **Mean recomputation/snapshot mixing (P2): fixed.** Embedder-profile changes
  now advance the visibility generation, and frozen vector compilation,
  embedding, and pinned-mean reads occur after the reader snapshot is validated.
- **Frozen expansion validity relaxation (P2): fixed.** The expansion builder
  consumes the authenticated `FrozenView`; a real-database regression covers
  an out-of-window neighbor.
- **Failure precedence (P2): fixed.** Token authentication and bound-state
  validation precede ordinary query/range/backend work in Rust and both
  bindings. Negative binding ranges remain representable until native
  validation.
- **Generation regression (P2): fixed.** Consume publishes and compares the
  reader-snapshot generation through the Engine's monotonic process witness.
- **TypeScript response version (P2): fixed.** The wrapper rejects unknown
  outer or echoed response schema versions instead of rewriting them to v1.
- **Wire schema (P2): fixed.** The public wire interface records schema 31 and
  its additive state.
- **Verification omissions (P2): partially fixed here.** Named eligibility,
  frozen-race, trigger-manifest, cross-binding precedence, and installed-package
  smoke coverage were added. Package, platform, CUDA, performance, and enclosing
  gate results remain closeout work and are not claimed by this record.

No accepted historical design or receipt was rewritten. The design's
serving-authority table list was corrected from 13 to 14 to include the existing
embedder-profile centering state identified by review.
