---
title: 0.8.25 Slice 25 implementation FIX-2 response
status: FIX_IMPLEMENTED_AWAITING_REVIEW
review_cycle: 2
reviewed_commit: 570a9e2ce895fde4820173532d23dd0b6da33542
---

# Slice 25 implementation FIX-2 response

Preserved telemetry/fault RED commit:
`ec44cfe2`.

Preserved per-operation rollback RED commit:
`eb4b0fe0`.

## Corrections

- Emit Writer `Started` after the race-closing receipt lookup and before
  admitted transaction work. Slow classification occurs only for the admitted
  attempt and precedes its terminal event; inner and outer exact replays remain
  event- and counter-silent.
- Apply the same pre-commit failure seam to cursor-exhaustion refusal as every
  other terminal outcome. The injected failure leaves no receipt and retry
  deterministically returns the refusal.
- Add a debug-only operation-position failure seam and prove rollback plus
  retry after each of the four operation variants.
- Add one shared Rust/Python/TypeScript all-variant fixture covering Unicode,
  present/absent optionals, negative validity, a byte locator, dependency
  registration, lifecycle transition, canonical digest, and receipt parity.
- Add table-driven receipt and source-reference corruption checks plus the
  first-over-bound 1,025-reference case.

The shared fixture and corruption tables extend acceptance coverage after the
behavioral fixes. They are not represented as pre-implementation RED history.
