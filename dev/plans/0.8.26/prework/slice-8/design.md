---
title: FathomDB 0.8.26 Slice 8 — initial decision scaffold
status: DRAFT
---

# Slice 8 design — initial decision scaffold

The completed, scored decision package is
[`proposal-register.md`](proposal-register.md). This initial scaffold remains
the pre-review seed and is not a second authority.

## Initial proposal register

These are seed assessments for review, not HITL decisions. Slice 8 adds every
actionable root cause found by Slices 6–7 before the session.

| Proposal | Value | Understood | Risk | Effort | Initial recommendation |
| --- | --- | --- | --- | --- | --- |
| repair frozen explanation correlation | cutover observability | high | low | S | include |
| complete frozen/evidence guide | integration safety | high | low | S | include |
| installed Memex-shaped witness | release stability | high | low | M | include |
| exact artifact-revision graph evidence | enables evidence-gated graph use | high | medium | M | include after contract decision |
| qualify/harden existing operator integrity route | cutover assurance | high | medium | S–M | qualify crates.io CLI first; preserve SDK denylist |
| atomic derived-edge actuation | graph-authoring blocker | high | high | L | include only through a changed-in-place V1 contract and invariant gates |
| minimum receipt evolution | truthful edge audit | medium | medium | M | fold into edge actuation only |
| environment/tooling corrections from Slice 0 | enabling | pending | pending | pending | decide from evidence |
| dependency upgrades from Slice 1 | maintenance/security | pending | pending | pending | decide individually |
| cruft actions from Slice 2 | maintainability | pending | pending | pending | decide individually |
| local build/preflight/verification fixes from Slice 6 | release throughput | pending | pending | pending | prioritize recurring root causes |
| CI/CD/package/registry fixes from Slice 7 | delivery reliability | pending | pending | pending | allocate by artifact dependency |
| Priority 3+ Memex requests | future value | variable | high/critical | L–XL | postpone from 0.8.26 |

## Delivery-placement test

1. Does the correction need to land before any feature implementation? If yes,
   consider Slice 9.
2. Does it need a feature or candidate artifact introduced at or after Slice
   10? If yes, assign the first reserved slice after that dependency.
3. Is it integrated packaging, platform, signing/provenance, or release-route
   evidence? If yes, consider Slice 50.
4. Is the evidence weak, value low, or risk disproportionate? Postpone, reject,
   or request more information.

## HITL decision agenda

1. Confirm that P0.1–P0.3, P1.1–P1.2, and P2.1–P2.2 are the entire feature
   boundary for 0.8.26.
2. Choose an in-place graph V1 change or first-generation V1 sidecar; introduce
   no parallel V2 result or method.
3. Approve the exact frozen artifact-evidence contract and non-disclosure rule.
4. Confirm whether Memex can deploy the crates.io CLI; choose a prebuilt target
   only if needed, and approve exact tool identity plus schema compatibility.
5. Confirm that doctor/recovery authority remains absent from public SDKs.
6. Apply superseding ruled D26-03 (`seq-283`): no parallel functional V1/V2
   APIs, affected V1 contracts change in place, fresh databases only, and no
   historical compatibility or migration machinery.
7. Choose ordinary flag/count or stricter governed derived-edge endpoint
   semantics and approve the exact minimum changed-in-place V1
   receipt/storage behavior.
8. Rule every environment, dependency, cruft, documentation, architecture,
   test-infrastructure, build, preflight, verification, and CI/CD proposal from
   Slices 0–7.
9. Approve exact placement and dependencies for included Slice 6–7 fixes.

## Review contract

The Slice 9 reviewer is read-only and independent of the authoring pass. The
review checks exact ruling coverage, no unapproved scope, correct later-slice
allocation, TDD ordering, public contract handling, rollback/stop gates, and
proportionate verification. `FIX-1` and `FIX-2` may correct the plan. A third
unresolved cycle stops Slice 8 and returns the conflict to the HITL.
