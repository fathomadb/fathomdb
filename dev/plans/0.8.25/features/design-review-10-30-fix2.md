---
title: 0.8.25 Slice 10–30 design review — FIX-2 resolution
status: COMPLETE
review_cycle: FIX-2
resolved_on: 2026-09-01
source_review: dev/plans/0.8.25/features/design-review-10-30-cycle1.md
---

# Slice 10–30 FIX-2 resolution

Replacement designs remain `DRAFT_REVIEW`, Slice 7 gated, and subject to cycle
2 independent review. Slice 10 is unchanged. No historical record was edited.

| Finding | Changed design/section | Resolution | Status |
| --- | --- | --- | --- |
| C1-15-01 revision validator contradiction | Slice 15, Closed revision-ID validators | Separate caller/runtime/migration validators with one stored union and parity fixtures. | RESOLVED |
| C1-20-01 activation/loss boundary conflation | Slice 20, Structural liveness and validity boundaries | Only live→non-live boundaries queue closure; `valid_from` never closes or auto-reactivates. | RESOLVED |
| C1-30-01 undiscovered-dependent visibility | Slice 30, Barrier admission and conservative read guard | Indexed ancestry guard runs pre-truncation on every governed read and fails closed. | RESOLVED |
| C1-30-02 semantic-operation journal erasure | Slices 25/30, journal index and erasure | Reference index, terminal payload stripping, deterministic recovery-before-erasure, and raw proof added. | RESOLVED |
