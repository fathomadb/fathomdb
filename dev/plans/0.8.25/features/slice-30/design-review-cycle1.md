---
title: 0.8.25 Slice 30 independent design review — cycle 1
status: COMPLETE
review_cycle: 1
reviewed_commit: 9a5fa6e7
verdict: CHANGES_REQUIRED
---

# Slice 30 independent design review — cycle 1

## Verdict

**CHANGES_REQUIRED.** The retained direct-dependency boundary is appropriate,
but seven implementation-shaping findings prevent READY status.

## Findings

| ID | Priority | Finding | Required correction |
| --- | --- | --- | --- |
| D30-01 | P1 | Public list/resume administration exceeds the release scope that parks public crash-journal administration. | Keep recovery internal and automatic; expose at most a keyed status lookup for closure IDs already returned by Slice 25. |
| D30-02 | P1 | Ordinary write has no operation ID/digest with which to recognize an exact post-commit retry or reproduce `WriteReceipt`. | Preserve legacy verb retry/return behavior by applying soft consequences atomically; do not invent a public or private prepared-request journal. |
| D30-03 | P1 | Source erasure already removes same-bucket derived rows, so pending physical work would appear corrupt; retained work identifiers also conflict with erasure. | Classify root-transaction effects before commit and define erasure of historical closure/work identifiers, with raw canaries. |
| D30-04 | P1 | The fixed validity instant is not persisted. | Persist exact epoch-second effective time and use it for admission, recovery, proof, and codecs. |
| D30-05 | P2 | Unconditional strict source eligibility would override historical `ReadView` relaxations. | Separate unconditional active-barrier fencing from source eligibility evaluated under the caller's existing view. |
| D30-06 | P2 | Persisted/public type invariants, blocker vocabulary, constructors, binding names, and serialization precedence are incomplete. | Close every schema/type/error invariant and exact cross-SDK wire shape. |
| D30-07 | P2 | A projection worker can check eligibility before admission and publish after proof. | Recheck owner eligibility and barriers after acquiring the worker write transaction, atomically with publication; test both interleavings. |

No recursive, multi-source, semantic-policy, or later-slice expansion is
required to resolve these findings.
