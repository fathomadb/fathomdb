---
title: 0.8.25 Slice 25 independent design review — cycle 5
status: COMPLETE_FAIL_FIX_CAP_REACHED
review_cycle: 5
reviewed_commit: 0bf1575c
reviewed_on: 2026-09-04
verdict: FAIL
---

# Slice 25 independent design review — cycle 5

## Verdict

**FAIL; five-FIX cap reached.** D25-01 through D25-20 are substantially
closed. Two implementation-shaping P2 ambiguities remain, so RED cannot start
without an explicit owner exception authorizing one final documentation-only
correction and review.

| ID | Priority | Finding | Recommended correction |
| --- | --- | --- | --- |
| D25-21 | P2 | Lifecycle missing/stale revision has overlapping `reference_unavailable` and `lifecycle_refused` mappings; `NotLifecycleAddressable` appears after constructor rejection; cursor/generation exhaustion is unordered. | Make invalid/non-logical address constructor-only; map all missing/stale lifecycle revisions to `lifecycle_refused`; reserve cursor before dependency generation; add the combined-exhaustion RED. |
| D25-22 | P2 | Bare `foo` and `l:foo` resolve to one target today, but normalization before digest is unspecified. | Strip `l:`, store/digest the bare value, reject empty/record-separator/non-logical prefixes, and add constructor/digest/replay equivalence RED cases. |

P3: keyed receipt validation should require affected revision IDs to be unique
members of the landed stored-revision union; Slice 30 should apply the same
grammar/uniqueness rule to closure IDs when it activates them.

## Gate

The release owner must either authorize one exceptional final design FIX/review
cycle with the recommended deterministic resolutions or postpone Slice 25.
Implementation and Slice 30 remain blocked meanwhile.
