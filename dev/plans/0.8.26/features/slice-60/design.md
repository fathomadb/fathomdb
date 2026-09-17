---
title: FathomDB 0.8.26 Slice 60 — current design-owner model
status: DRAFT
target_release: 0.8.26
---

# Slice 60 design — current profiles over historical baselines

## Authority order

Accepted ADRs decide policy, `dev/interfaces/` owns public contracts, the
active architecture owns system shape, and code plus tests witness the as-built
implementation. Maintained topic designs explain how those authorities compose;
they do not override them.

Each rewritten owner begins with an explicit current 0.8.26 profile. Historical
0.6.x material is removed when stale and valueless or retained beneath a clearly
non-current section when it contains unique rationale. A reader must not need
to infer which era a normative statement describes.

## Shared seam model

The three owners intentionally overlap only at named seams:

- retrieval owns candidate generation, ranking/fusion, view eligibility,
  evidence, and explanation behavior;
- recovery owns operator inspection/recovery boundaries and exact diagnostic
  surfaces; and
- engine owns storage/runtime topology, canonical identity, transactions,
  projection publication, and the storage seams used by retrieval/recovery.

Cross-links point to the owner instead of duplicating its detailed rules. The
final engine pass therefore triggers a mandatory retrieval/recovery cross-read,
which may amend earlier wording without treating it as frozen completed work.

## Expected change class

This is a documentation reconciliation. No code change is expected. Discovery
of an implementation defect is a stop condition with a separately reviewed
remediation proposal, not implicit authorization to change runtime behavior.
