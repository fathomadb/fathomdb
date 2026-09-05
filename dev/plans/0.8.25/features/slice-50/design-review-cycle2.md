---
title: 0.8.25 Slice 50 design review cycle 2
status: CHANGES_REQUIRED
reviewed_commit: ac57ded1
---

# Slice 50 design review cycle 2

The independent FIX-1 review closed the source-authorization,
keyed-commitment, same-snapshot, and general nested-wire findings. Two P1
findings remained:

1. A graph-origin edge revision could be returned without reauthorizing the
   edge in the resolver's current snapshot.
2. The proposed node lifecycle spellings did not match the existing
   `LifecycleState` authority.

Design v7 resolves the first by treating graph origin as a third authorization
subject: it must still be the same live edge, connect the returned node, obey
the original graph admission constraints, and have no closure barrier before
its revision ID is disclosed. All failures collapse to
`evidence_unavailable`. It resolves the second by reusing the exact existing
`pending`, `active`, `deleted`, and `purged` wire spellings; successful strict
resolution is `active`.

Both corrections require design-review cycle 3.
