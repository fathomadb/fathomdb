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
| D26-03 | Introduce no parallel functional V1/V2 public API pairs. Change affected V1 contracts in place, make 0.8.26 breaking and fresh-database-only, retain no historical compatibility or migration machinery, and defer parallel versions until post-1.0. | `seq-283`, superseding `seq-282` as to V2 naming and redirects | Slice 35/40 extend V1 in place. Graph evidence also remains V1. D26-05 selects the changed-in-place V1 receipt. |
| D26-04 | Option B: require derived-edge endpoints in the complete prospective batch state, including endpoints later in the batch. | `seq-284` | Slice 35/40 use strict endpoint refusal; no dangling-edge admission or operation-order requirement. |
| D26-05 | Option A: change the compact `ActuationReceiptV1` contract in place and add only proven edge fields. | `seq-285` | Compatible with D26-04 B; no dangling count, consequence manifest, or historical compatibility. |
| D26-06 | Revised narrow option A for Slice 9. | `seq-286` | Slice 9 implements P26-01/02/03, exact P26-05 corrections, and P26-08 comment truth; P26-09 moves to Slice 50. |
| D26-07 | Revised narrow option A for Slice 50. | `seq-287` | Include P26-09/P26-13 and conditional P26-11; postpone P26-10/P26-12/P26-14. Publication remains separately gated. |

The 0.8.26 machine-readable release state does not yet exist; its creation is
part of the now-ruled Slice 9 preparation decision. These ledger-backed
rulings must be copied into that state by the state-authoring tool when P26-02
is implemented. They must not be inferred or rewritten.

## Open HITL position

The following decision remains open:

| Decision | Position | Effect now |
| --- | --- | --- |
| D26-01 | Defer the public-shape decision until Slice 15 produces performance and erasure-linearization evidence. | Slice 15 may run as a decision-support spike; Slice 20 remains blocked. |

This position remains absent from the append-only decision ledger and future
release-state `decisions.ruled` array until the HITL selects the public shape.

## Decision detail

### D26-01

No public shape is selected before Slice 15. Slice 15 prototypes and measures
post-selection revision hydration,
primary-connection point resolution, ordinary-search sentinels, mixed writes,
and erasure linearization. See
[`spike-d26-01-graph-evidence-impact.md`](spike-d26-01-graph-evidence-impact.md).
The `seq-283` release-wide rule requires graph evidence to retain V1 naming. It
does not reduce graph reader/erasure risk, so the shape decision is now between
an in-place graph V1 change and an opt-in first-generation V1 evidence sidecar
on the existing V1 graph operation. A parallel graph V2 result or method is
excluded.

### D26-03 — ruled

HITL `seq-283` supersedes `seq-282` as to V2 naming and redirects. V1 remains
the sole functional actuation grammar and changes in place; no parallel V2 API
or router exists. The breaking, fresh-database-only, no-migration, and no
historical-compatibility decisions remain. See
[`ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md`](../../../../adr/ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md).

### D26-04 and D26-05 — ruled

The scope spike initially recommended strict endpoint existence with current
receipt storage. `seq-282` removes historical receipt and cross-release
compatibility; `seq-283` keeps the current contract named V1. HITL `seq-284`
selects strict complete-prospective-state endpoint existence, and `seq-285`
selects the compatible compact changed-in-place V1 receipt. See
[`spike-d26-03-05-actuation-shape.md`](spike-d26-03-05-actuation-shape.md).

The ruled D26-05 contract is a changed-in-place `ActuationReceiptV1`
with the compact operation ID, current V1 request digest, terminal
outcome/refusal location and reasons,
affected revisions, resulting write/dependency boundaries, pending projection
cursors, projection generation, and closure-operation concepts that the
current contract actually produces. Edge revisions use
`affected_revision_ids`. Under strict D26-04 there is no dangling count.
Current V1 source references remain internal for erasure/integrity; none is
interpreted as data from an earlier release.

### D26-06 — ruled

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

HITL `seq-286` selects the revised narrow plan: implement P26-01, P26-02, P26-03, exact
P26-05 corrections, and the P26-08 comment correction in Slice 9; treat P26-04
as execution conditions rather than product work; move P26-09 to Slice 50.
Drop P26-06's markdown dependency remediation unless it becomes a hard build
blocker.

This is not speculative: the 2026-09-12 Slice 8 verification run stopped at
`public-doc-truth` because root `README.md` lacks the validator's current
published-0.8.23 statement. Slice 8 records the failure but does not implement
the P26-01 correction.

### D26-07 — ruled

HITL `seq-287` rules that Slice 50 is the final integrated, non-publishing candidate/package/platform
verification slice. It is not the tag or publication step; those remain a
separate explicit gate. The revised narrow option A moves P26-09 Windows probe
inventory consolidation there, adds registry visibility polling logic
(P26-13), and performs exact generated Gitleaks digest registration (P26-11)
only if the actual benign evidence requires it. It postpones unrelated
full-history Gitleaks triage (P26-12) and speculative runner/dispatch work
(P26-10 and P26-14).

## Still required

Only D26-01 remains open, explicitly deferred until Slice 15 reports. Slice 8
has replaced the provisional Slice 9 plan and finalized the Slice 15 and Slice
35 draft plan/design documents. Required independent review and any focused
corrections remain before Slice 8 can prepare Slice 9 for execution. Slice 20
remains blocked until the post-Slice-15 D26-01 ruling.
