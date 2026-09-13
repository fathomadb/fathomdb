---
title: FathomDB 0.8.26 Slice 8 — HITL decision record
status: PARTIAL
observed_on: 2026-09-12
---

# Slice 8 HITL decision record

## Ruled decisions

| Decision | Ruling | Durable ledger | Effect |
| --- | --- | --- | --- |
| D26-02 | Option A: qualify the existing CLI first; if a prebuilt fallback is needed, stop and warn loudly before expanding packaging scope. | `seq-280` | Slice 30 remains narrow; prebuilt packaging is conditional and requires an explicit visible gate. |
| D26-08 | Option A: keep 0.8.26 as narrow as possible around Memex needs; postpone unrelated maintenance and preserve historical evidence. | `seq-281` | Broad dependency, cleanup, and developer-tool churn stays out absent a hard build or shipped-security blocker. |

The 0.8.26 machine-readable release state does not yet exist; its creation is
part of the unresolved Slice 9 preparation decision. These ledger-backed
rulings must be copied into that state by the state-authoring tool when P26-02
is implemented. They must not be inferred or rewritten.

## Clarified decisions awaiting explicit ruling

### D26-01

Option A has low ordinary-path risk only when paired with named Slice 15 before
Slice 20. Slice 15 prototypes and measures post-selection revision hydration,
primary-connection point resolution, ordinary-search sentinels, mixed writes,
and erasure linearization. See
[`spike-d26-01-graph-evidence-impact.md`](spike-d26-01-graph-evidence-impact.md).

### D26-03

Option A means a full successor grammar containing all V1 operations plus
`PutDerivedEdge`, with separate V2 entry points and digest domain. It does not
mean an edge-only request. V1 remains unchanged. A named Slice 35 before Slice
40 will prototype and measure the contract.

### D26-04 and D26-05

The scope spike changed the recommendation. Ordinary flag/count semantics
would require durable receipt evolution because the current actuation receipt
cannot persist the dangling count. The lower-risk bundle is D26-04 option B,
strict endpoint existence over complete prospective batch state, together with
D26-05 option A, current receipt storage and explicit cross-version operation
ID behavior. See
[`spike-d26-03-05-actuation-shape.md`](spike-d26-03-05-actuation-shape.md).

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

The revised narrow recommendation is: implement P26-01, P26-02, P26-03, the
exact P26-05 corrections, the P26-08 comment correction, and P26-09 in Slice 9;
treat P26-04 as execution conditions rather than product work. Drop P26-06's
markdown dependency remediation from Slice 9 unless it becomes a hard build
blocker. An ultra-narrow alternative implements only P26-01 and P26-02 and
defers all other corrections.

This is not speculative: the 2026-09-12 Slice 8 verification run stopped at
`public-doc-truth` because root `README.md` lacks the validator's current
published-0.8.23 statement. Slice 8 records the failure but does not implement
the P26-01 correction.

### D26-07

Slice 50 is the final integrated, non-publishing candidate/package/platform
verification slice. It is not the tag or publication step; those remain a
separate explicit gate. The revised narrow option A places exact generated
Gitleaks digest registration (P26-11) and registry visibility polling logic
(P26-13) in Slice 50, while postponing the unrelated full-history Gitleaks
triage (P26-12) and speculative runner/dispatch work (P26-10 and P26-14).

## Still required

Explicit HITL rulings remain required for D26-01, D26-03, the combined
D26-04/D26-05 bundle, D26-06, and clarified D26-07. After those rulings are
recorded, Slice 8 will replace the provisional Slice 9 plan, add approved Slice
15 and Slice 35 plan/design documents, obtain the required independent review,
and return the reviewed plan for execution approval.
