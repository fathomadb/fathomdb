---
title: 0.8.25 Slice 30 design review — FIX-1 resolution
status: COMPLETE
review_cycle: FIX-1
source_review: design-review-cycle1.md
---

# Slice 30 design review — FIX-1 resolution

| Finding | Resolution | Status |
| --- | --- | --- |
| D30-01 | Removed public list/resume APIs. Recovery is internal; the only additive read is keyed status for Slice 25 receipt IDs. | RESOLVED |
| D30-02 | Soft effects and standalone proof now commit with the root mutation. Actuation uses its existing idempotent receipt; no request journal is added. | RESOLVED |
| D30-03 | Removed persisted work identifiers. Physical effects occur in the root transaction, prior completed source-revision closure rows are erased, and raw closure canaries are explicit. | RESOLVED |
| D30-04 | Added persisted `effective_at_epoch_s` and one-instant semantics. | RESOLVED |
| D30-05 | Made active barriers unconditional while source lifecycle/validity uses the same existing `ReadView` as the derived result. | RESOLVED |
| D30-06 | Closed schema phase/count/proof/blocker invariants plus exact constructors, errors, validation order, and binding shapes. | RESOLVED |
| D30-07 | Projection publication now rechecks owner/source/barrier state inside its write transaction. | RESOLVED |
