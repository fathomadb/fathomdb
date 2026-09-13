---
title: FathomDB 0.8.26 Slice 8 — HITL decision record
status: PARTIAL
observed_on: 2026-09-12
last_updated: 2026-09-13
---

# Slice 8 HITL decision record

## Ruled decisions

| Decision | Ruling | Durable ledger | Effect |
| --- | --- | --- | --- |
| D26-02 | Option A: qualify the existing CLI first; if a prebuilt fallback is needed, stop and warn loudly before expanding packaging scope. | `seq-280` | Slice 30 remains narrow; prebuilt packaging is conditional and requires an explicit visible gate. |
| D26-08 | Option A: keep 0.8.26 as narrow as possible around Memex needs; postpone unrelated maintenance and preserve historical evidence. | `seq-281` | Broad dependency, cleanup, and developer-tool churn stays out absent a hard build or shipped-security blocker. |
| D26-03 | Make 0.8.26 a breaking V2-only actuation release. V1-shaped calls may only receive a loud V2 direction; carry no V1 request, receipt, replay, integrity, operation-ID, data, or database-migration compatibility. Accept fresh databases only. | `seq-282` | Slice 35/40 must replace V1 rather than coexist with it. D26-05 must be reconsidered as a V2-only receipt decision. |

The 0.8.26 machine-readable release state does not yet exist; its creation is
part of the unresolved Slice 9 preparation decision. These ledger-backed
rulings must be copied into that state by the state-authoring tool when P26-02
is implemented. They must not be inferred or rewritten.

## Draft HITL positions — not rulings

The HITL is generally accepting of the following recommendations but has
explicitly directed that they remain draft, not final decisions:

| Decision | Draft position | Effect now |
| --- | --- | --- |
| D26-01 | Generally accepts option A with Slice 15. | Continue design discussion; do not approve or execute Slice 15 or Slice 20. |
| D26-04 | Generally accepts strict complete-state endpoint refusal. | Re-evaluate against the V2-only fresh-database contract; do not finalize yet. |
| D26-05 | Earlier conditional acceptance of current receipt storage is displaced by `seq-282`. | Reframe as the exact V2-only receipt shape; no V1 compatibility question remains. |
| D26-06 | Generally accepted the prior narrow Slice 9 bundle; re-evaluation now moves P26-09 to Slice 50. | Confirm or reject the narrower replacement before Slice 9 is replaced. |
| D26-07 | Generally accepted the prior narrow Slice 50 placement; re-evaluation adds P26-09 and makes P26-11 conditional on real evidence. | Confirm or reject the adjusted placement; publication remains separately gated. |

These positions are deliberately absent from the append-only decision ledger
and future release-state `decisions.ruled` array. They become rulings only after
an explicit final HITL decision.

## Clarified decisions awaiting explicit ruling

### D26-01

Option A has low ordinary-path risk only when paired with named Slice 15 before
Slice 20. Slice 15 prototypes and measures post-selection revision hydration,
primary-connection point resolution, ordinary-search sentinels, mixed writes,
and erasure linearization. See
[`spike-d26-01-graph-evidence-impact.md`](spike-d26-01-graph-evidence-impact.md).
The `seq-282` actuation/database break does not authorize a graph API break and
does not reduce the graph reader/erasure risk, so the recommendation remains
an opt-in successor that preserves graph V1.

### D26-03 — ruled

HITL `seq-282` replaces the earlier coexistence proposal. V2 is the sole
functional actuation grammar and 0.8.26 is fresh-database-only. See
[`ADR-0.8.26-breaking-v2-actuation-and-fresh-database-boundary.md`](../../../../adr/ADR-0.8.26-breaking-v2-actuation-and-fresh-database-boundary.md).

### D26-04 and D26-05

The scope spike initially recommended strict endpoint existence with current
receipt storage. `seq-282` removes V1 receipt and cross-version compatibility,
so D26-04 remains provisionally strict while D26-05 must now select the exact
V2-only receipt fields and integrity contract for a fresh database. See
[`spike-d26-03-05-actuation-shape.md`](spike-d26-03-05-actuation-shape.md).

The revised D26-05 recommendation is `ActuationReceiptV2` with the compact
operation ID, V2 request digest, terminal outcome/refusal location and reasons,
affected revisions, resulting write/dependency boundaries, pending projection
cursors, projection generation, and closure-operation concepts that V2
actually produces. Edge revisions use `affected_revision_ids`. Under strict
D26-04 there is no dangling count. V2 source references remain internal for
V2 erasure/integrity; none is interpreted as historical V1 data.

### D26-06

The proposal IDs mean:

- P26-01: repair contradictory 0.8.25 publication state and validate its
  lifecycle truth;
- P26-02: create the 0.8.26 release state and board after that repair;
- P26-03: make preflight select an explicit authoritative main ref;
- P26-04: use checkout-owned developer tools and fresh artifact environments,
  with disk budgeting at the full matrix;
- P26-05: correct only the maintained authority, platform, and CLI inventory
  documents found stale;
- P26-08: correct comments that name download-artifact v8 while the pinned SHA
  is v4.3.0; do not upgrade the runtime in this release; and
- P26-09: derive duplicated Windows release-wheel probe inventories from one
  machine-readable contract.

The revised narrow recommendation is: implement P26-01, P26-02, P26-03, exact
P26-05 corrections, and the P26-08 comment correction in Slice 9; treat P26-04
as execution conditions rather than product work; move P26-09 to Slice 50.
Drop P26-06's markdown dependency remediation unless it becomes a hard build
blocker. An ultra-narrow alternative implements only P26-01 and P26-02 and
defers all other corrections.

This is not speculative: the 2026-09-12 Slice 8 verification run stopped at
`public-doc-truth` because root `README.md` lacks the validator's current
published-0.8.23 statement. Slice 8 records the failure but does not implement
the P26-01 correction.

### D26-07

Slice 50 is the final integrated, non-publishing candidate/package/platform
verification slice. It is not the tag or publication step; those remain a
separate explicit gate. The revised narrow option A moves P26-09 Windows probe
inventory consolidation there, adds registry visibility polling logic
(P26-13), and performs exact generated Gitleaks digest registration (P26-11)
only if the actual benign evidence requires it. It postpones unrelated
full-history Gitleaks triage (P26-12) and speculative runner/dispatch work
(P26-10 and P26-14).

## Still required

Explicit final HITL rulings remain required for D26-01, D26-04, reframed
D26-05, D26-06, and clarified D26-07. After final rulings are recorded, Slice 8
will replace the provisional Slice 9 plan, complete the approved Slice 15 and
Slice 35 plan/design documents, obtain the required independent review, and
return the reviewed plan for execution approval.
