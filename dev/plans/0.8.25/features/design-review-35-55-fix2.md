---
title: 0.8.25 Slices 35–55 design review FIX-2 resolution
status: COMPLETE
review_cycle: FIX-2
reviewed_on: 2026-09-01
design_status: DRAFT_REVIEW
source_review: dev/plans/0.8.25/features/design-review-35-55-cycle1.md
---

# Slices 35–55 FIX-2 resolution

FIX-2 changes only the Slice 50 and 55 successor designs. Historical designs
remain unchanged; both designs remain `DRAFT_REVIEW`, blocked on Slice 7 and
independent cycle-2 review.

| Finding | Changed section | Resolution | Status |
| --- | --- | --- | --- |
| C1-50-01 | Slice 50 Evidence receipt/reference persistence | Receipt identity fields are exhaustively limited to opaque artifact/source/set revisions and Engine event/generation IDs. Free-form source, logical, owner, path/session, public hit, projection-name, and payload IDs are prohibited; provenance resolves source identity only after authorization. | RESOLVED |
| C1-55-01 | Slice 55 Deterministic replayable trace continuation | Immutable page response and next cursor commit with frontier advancement under `(lease,input_ordinal)`; identical cursor retries/concurrent duplicates replay exact bytes; pages share lease retention/erasure. | RESOLVED |
| C1-55-02 | Slice 55 Frozen integrity job contract | Engine-minted boundary fixes database/write/time/liveness/generations on every job/status/finding/page; drift ends incomplete typed; repairs revalidate boundary and authority. | RESOLVED |
| C1-55-03 | Slice 55 Reverse dependency index generation/cutover | Repair builds a generation-scoped shadow from forward authority, applies boundary-ordered dual writes, verifies reciprocal equivalence, atomically activates, preserves/fails closed old reads appropriately, and defines crash/restart/cleanup/receipt states. | RESOLVED |

This is the author FIX-2 record, not an independent PASS verdict.
