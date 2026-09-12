---
title: FathomDB 0.8.26 Slice 6 — initial decision scaffold
status: DRAFT
---

# Slice 6 design — initial decision scaffold

## Initial proposal register

These are seed assessments for review, not HITL decisions.

| Proposal | Value | Understood | Risk | Effort | Initial recommendation |
| --- | --- | --- | --- | --- | --- |
| repair frozen explanation correlation | cutover observability | high | low | S | include |
| complete frozen/evidence guide | integration safety | high | high | S | include |
| installed Memex-shaped witness | release stability | high | high | M | include |
| exact artifact-revision graph evidence | enables evidence-gated graph use | high | medium | M | include after contract decision |
| distribute operator integrity route | cutover assurance | high | medium | M | include with SDK denylist preserved |
| atomic derived-edge actuation | graph-authoring blocker | high | high | L | include only with V2 and invariant gates |
| minimum receipt evolution | truthful edge audit | medium | medium | M | fold into edge actuation only |
| environment/tooling corrections from Slice 0 | enabling | pending | pending | pending | decide from evidence |
| dependency upgrades from Slice 1 | maintenance/security | pending | pending | pending | decide individually |
| cruft actions from Slice 2 | maintainability | pending | pending | pending | decide individually |
| Priority 3+ Memex requests | future value | variable | high/critical | L–XL | postpone from 0.8.26 |

Documentation's direct stability leverage is scored as risk avoided: incomplete
or incorrect public guidance can cause unsafe integration even though the edit
itself is mechanically small.

## HITL decision agenda

1. Confirm that P0.1–P0.3, P1.1–P1.2, and P2.1–P2.2 are the entire feature
   boundary for 0.8.26.
2. Choose additive graph fields or successor response types.
3. Approve the exact frozen artifact-evidence contract and non-disclosure rule.
4. Choose the CLI artifact target matrix and exact version-match policy.
5. Confirm that doctor/recovery authority remains absent from public SDKs.
6. Approve or reject `ActuationBatchV2` plus `PutDerivedEdge`.
7. Approve endpoint, replay/digest, and minimum receipt semantics.
8. Rule every environment, dependency, cruft, documentation, architecture,
   and test-infrastructure proposal from Slices 0–5.

## Review contract

The Slice 7 reviewer is read-only and independent of the authoring pass. The
review checks exact ruling coverage, no unapproved scope, TDD ordering, public
contract handling, rollback/stop gates, and proportionate verification.
`FIX-1` and `FIX-2` may correct the plan. A third unresolved cycle stops Slice
6 and returns the conflict to the HITL.
