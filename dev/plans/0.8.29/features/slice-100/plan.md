---
title: Slice 100 — bounded interrupted-erasure qualification
status: PROPOSED
target_release: 0.8.29
slice: 100
---

# Slice 100 — bounded interrupted-erasure qualification

Database boundary, owner `seq-368`: .29 requires no database migration. Use the
existing format37 substrate and stored span/visibility data. See the
[release baseline](../../../plan-0.8.29.md#baseline-and-workspace); a design
requiring a storage-format change returns for explicit scope review.

## Current evidence and boundary

D28-09 qualifies interruption inside a transaction or WAL checkpoint and repairs
confirmed violations; see the [original scope](../../../0.8.28-draft-scope.md).
The [brief](../../planning-brief.md) and [reconciliation](../../reconciliation-plan.md)
place this on current format 37. Physical erasure is complete except defects,
per the owner's November 4 ruling in `physical-erasure-done-defects-only.md`
(read in the external memory store); this is no optimization or availability program.
No product test, fault campaign, environment change or contract amendment occurred
while drafting. Historical passes retain their source/platform limits.

## D28-09 coverage and gap matrix — first execution deliverable

| Obligation | Inspected existing evidence | Proposed disposition |
| --- | --- | --- |
| Atomic deletion and sanitation rollback | [Slice 82 recovery](../../../../../src/rust/crates/fathomdb-engine/tests/slice82_recovery.rs) exits before/after deletion and sanitation commits; physical transaction unit test injects actual deferred-COMMIT failure | Reuse; inspect exact transaction interior and reopened-state assertions before claiming a remaining gap |
| Autonomous continuation after reopen | [Slice 83 availability](../../../../../src/rust/crates/fathomdb-engine/tests/slice83_availability.rs) exits after deletion/unit/redaction and observes idle completion without target/follow-up calls | Substantial discharge; keep the worker active in autonomous controls |
| Explicit queued/executing/finalizing/complete replay | [Slice 85 crash](../../../../../src/rust/crates/fathomdb-engine/tests/slice85_crash.rs) exits at eight before/after queue/delete/redaction/receipt commits | Reuse; it proves logical state and retry but does not alone prove every physical/history family after each exit |
| Checkpoint refusal and durable fences | [Slice 85 physical](../../../../../src/rust/crates/fathomdb-engine/tests/slice85_physical.rs) pins an external reader and verifies finalizing/fence/retry; completeness tests retain WAL blockers | BUSY is covered; determine whether an interruption during an executing checkpoint remains uncovered |
| Canonical/projection/native/physical survivor and history | [Slice 82 physical](../../../../../src/rust/crates/fathomdb-engine/tests/slice82_physical_erasure.rs), [Slice 83 routes](../../../../../src/rust/crates/fathomdb-engine/tests/slice83_routes.rs), Slice 85 physical witnesses | Reuse positive all-family/non-head-vector/survivor oracles; add interrupted composition only if unmapped |
| Current format-37 visibility and vector spans | [Visibility ADR](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md) and [current tests](../../../../../src/rust/crates/fathomdb-engine/tests/slice95_visibility_coalescing.rs) cover clean state, rollback and active span cleanup | Map current erasure ingress and interrupted post-state; no assumed format-36-only gap |

Produce `coverage-matrix.md` with named tests, fault position, mode, assertions,
source identity and covered/partial/missing verdict. Coverage closure alone is
valid: if every D28-09 obligation is discharged, stop with a reviewed closure
record rather than commissioning a redundant campaign. Inspect test bodies and
inline `#[cfg(test)]` modules before classifying any absence.

## Slice preparation: draft need, requirements and acceptance

Release-local labels below are draft acceptance statements, not canonical AC IDs.

- N100: applications need truthful erasure/recovery outcomes after interruption,
  preserving authorized survivors/history and the registered physical boundary.
- R100-1 / A100-1: each selected fault has a precise reached-boundary witness and
  an expected committed/rolled-back state; no partial canonical/projection cut,
  committed armed visibility marker, or speculative terminal success is accepted.
- R100-2 / A100-2: explicit Queued remains unchanged until activation; interrupted
  removal retains immutable debt and all post-delete mutation fences across
  handle drop/reopen. Explicit open performs no autonomous physical work.
- R100-3 / A100-3: Autonomous reopen resumes its accepted worker without an exact
  caller argument; supported unrelated writes retain the certified envelope,
  while affected/admin/projection routes remain fenced as specified.
- R100-4 / A100-4: Complete requires exact structural zero, original-sink redaction,
  retained-term sanitation, target-bearing retry/queue removal and actual TRUNCATE.
  Independent witnesses inspect main/WAL/registered sinks before close can mask WAL.
- R100-5 / A100-5: canonical nodes/edges/properties, current/historical projections,
  mapped non-head vectors and vector spans retain exact scope; survivor-only
  comparisons preserve authorized history, scores/order and no resurrection.
- R100-6 / A100-6: after blocker resolution the permitted retry/offline path
  converges to the same receipt and debt-free state; queued intent is not activated
  offline. Corrupt or unverifiable authority refuses safely with evidence intact.

## Architecture reading and design draft

Read the accepted [resumable cleanup ADR](../../../../adr/ADR-0.8.28-resumable-physical-cleanup.md),
[explicit policy ADR](../../../../adr/ADR-0.8.28-explicit-erasure-maintenance.md),
[maintenance contract](../../../../design/explicit-maintenance-erasure/contract.md),
[production design](../../../../design/explicit-maintenance-erasure/production-design.md)
and [wire layout](../../../../interfaces/wire.md) alongside actual transaction,
worker, queue and checkpoint callsites. Format 37 supersedes earlier boundaries.
Draft `design.md` as a state/fault/oracle table using existing hooks and real files;
add a test-only seam only for a demonstrated missing owned boundary.
A pre/post-checkpoint hook does not prove interruption inside its active call.
Challenge: autonomous progress can race observation; use deterministic rendezvous
and independent observers without silently disabling the behavior under test.
Challenge: close can clean WAL; capture physical evidence while ownership persists.

Planned primary-source research is narrowly conditional: inspect pinned SQLite
checkpoint/interrupt/transaction documentation and vendored or registry source
for the proposed control before choosing it. Document whether it can actually
interrupt that checkpoint and its connection/thread constraints. Do not invent
power-loss proof from process exit/kill, SQLite interruption, or COMMIT refusal.
The [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
favors owned anomaly boundaries; general VFS/power-loss campaigns remain deferred.
Fit review compares new coverage to existing suites and accepted contracts; revise
or delete redundant cases before implementation. Broader reliability redesign,
new cleanup API, SLA, deadlines and availability measurements are outside scope.

## Tasks, dependencies and integration

1. Complete the matrix and draft `requirements.md`, `design.md`, `test-plan.md`;
   independent review first decides existing closure versus concrete missing cases.
2. For missing cases only, specify deterministic real-database faults, watchdogs,
   independent positive target/survivor witnesses and exact pre/post-reopen state.
3. Stage failing behavior tests before any repair; preserve RED evidence per defect,
   then implement the smallest fix and freeze its oracle through GREEN/refactor.
   Extend existing property retry/reopen sequences only for a missing invariant;
   retain seeds and exercise first/persistent faults plus fault-free controls.
4. Review the exact delta independently; produce `status.md` with source/test/tool
   identities, fault reachability, outcomes, residual limitations and disposition.

Hard inputs: reviewed Slice 9 scope and required Slice 10 preparation, current
format-37 native artifacts and checkout-owned tooling before execution. Existing
.28 [Slice 83 status](../../../0.8.28/features/slice-83/status.md) and
[Slice 85 status](../../../0.8.28/features/slice-85/status.md) are provenance,
not current-tip passes. No semantic dependency on 80/90 or CUDA hardware is known.
Serialize engine/shared-hook landings with 40/50/70/110; numerical order is preferred
integration order, not a reason to block read-only design or coverage closure.
New prerequisites: determine a supported active-checkpoint control if actually
missing; prove native span fixtures are populated; identify a venue only for a
platform-specific fault selected by the reviewed matrix. No host access assumed.

Use scoped Markdown validation for planning-only closure. Source/test/executable
changes require the full agent gate on the final code phase, with owned environment
preflight; rerun affected composed seams after later engine landings. Preserve
failed attempts and historical assembled gates without relabeling them green.
Stop for owner scope decision if a missing control requires a new VFS campaign,
changed public semantics or broader reliability work; triage concrete defects
normally. Completion is reviewed D28-09 closure or qualified bounded gap repairs,
with no new physical-erasure feature program.
