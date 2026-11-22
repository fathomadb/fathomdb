---
title: Inspector v0.1 Slice 15 — high-level draft design
date: 2026-10-29
product: FathomDB Inspector
target_release: 0.1
status: ACTIVE
desc: Organization of the release-wide requirements and verification baseline
---

# Slice 15 — Requirements and verification design guidance

Baseline **S15-2026-10-30**, reconciled from `d05912c99` against `9486c5ba9`.
[The plan](plan.md) and [status](status.md) record policy/review state. Canonical
criteria and the [verification matrix](../verification-plan.md) are prepared;
D-01 is adopted with exact flags/defaults and narrow erasure-pending export. Slice 16 owns architecture synthesis;
this document does not select product architecture or claim runtime behavior.

## Information flow and ownership

```text
User outcomes + owner decisions + source/contracts + reviewed findings
  → canonical needs / requirements / acceptance
  → shared verification matrix + fixture/oracle specifications
  → Slice 16 architectural obligations
  → Slice 20–60 implementation and verification assignments
```

Existing `needs.md`, `requirements.md` and `acceptance.md` remain authoritative
within the Inspector area. Keep stable INS/V01 IDs; update ambiguous meanings
in place with recorded rationale. Optional outcomes belong in ROADMAP.

The shared `plans/0.1/verification-plan.md` holds test strategy and one
traceability matrix referencing canonical criteria. Store each case's oracle,
fixture, layer, owner/stage, evidence and state there rather than duplicate AC.
Small related cases may share a row if no required boundary is hidden.
Executable test paths/commands become concrete in the owning code slices;
this planning document must not cite nonexistent tests as if available.

## Proof boundaries

- Distinguish declared configuration, stored materialization, session prerequisites
  and actual request evidence. The same distinction governs test expectations.
- Independent intended facts establish correctness; source-grounded Engine
  conformance establishes fidelity. Neither alone proves the entire Inspector.
- Qualify Engine startup/worker effects without claiming immutable bytes or
  permitting Inspector edit/rebuild/repair controls. Separate early refusals,
  failures after startup and successful changing observations.
- Real database fixtures establish contents and relationships. Browser tests
  establish actual structure-specific interaction. Human transfer tasks check
  useful interpretation after ordinary documentation; none substitutes for another.
- Required positive cases and expected unsupported/missing-runtime cases are
  separate. An absent dependency cannot make a mandatory feature test pass.

## Baseline maintenance and handoff

Slice 15 records policy decisions and remaining blockers with consequences.
Apply adopted D-01 independently of ordinary retrieval eligibility. Slice 16
may expose an untestable requirement or infeasible dependency: route that fact
back to the canonical baseline and matrix, record the change and affected
slices, and re-review the affected contract. Do not weaken required scope.

Later slices refine their tests/design while preserving the release-wide
criteria and shared fixtures. Re-review shared changes proportionately and
update affected owners. A final coverage audit belongs to Slice 60, but no
feature may defer its own correctness to that audit.

Direct Sol-high review and Terra planning verification are required by the
plan. The lean status record records what was decided/verified, not invented
application test passes. No separate approval packet or new test framework
is needed merely to finish requirements planning.

Owner clarification (2026-10-31): the canonical D-01 export requirement now
applies only to legitimately existing readable retained-erasure content.
Physical deletion stays correct. This supersedes any unconditional positive
Engine fixture assumption in this historical Slice15 baseline; see current
requirements and M20/M39. Other positive requirements are unchanged.
