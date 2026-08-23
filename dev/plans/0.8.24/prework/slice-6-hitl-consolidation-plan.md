---
title: 0.8.24 Slice 6 — consolidation, independent review, and HITL plan
status: DRAFT
target_release: 0.8.24
---

# Slice 6 — consolidation, independent review, and HITL plan

## Purpose

Consolidate every proposal from Slices 0–5 into a concise decision package,
conduct an interactive owner session, and turn only the owner's accepted
prework into the final Slice 7 implementation plan. An independent read-only
review and at most two documented FIX-n cycles test that plan before the owner
gives its final disposition.

## Required inputs

- The 0.8.24 plan and every completed Slice 0–5 finding, design, CRUD, alignment,
  and verification record.
- The Slice 0 owner decision brief and all unresolved external-evidence rows.
- This draft Slice 7 plan, used only as a contingent shell until the owner has
  made the first-round decisions.
- Current git evidence for the release branch/base and any newer main-owned CI
  work that affects the proposed allocations.

## Scope

1. Enumerate every distinct proposal, including no-change, postpone, reject,
   and decision-required findings.
2. Score each actionable proposal for understanding, concrete risk, effort,
   recommendation, dependencies, and destination.
3. Present the register interactively to the owner and record an explicit
   decision for each row.
4. Write a detailed Slice 7 implementation plan containing only accepted
   prework items.
5. Obtain an independent read-only review of that plan, apply no more than two
   documented FIX-n cycles, and obtain the owner's final plan decision.

## Non-goals and no-implementation boundary

- Do not implement any accepted item, feature, test, workflow, dependency,
  runner, registry, or documentation change in Slice 6.
- Do not treat a recommendation, score, or silence as owner acceptance.
- Do not move feature work from Slices 10–70 into Slice 7 merely because the
  feature was discussed during prework.
- Do not create or push a release tag, publish an artifact, dispatch hosted CI,
  or approve an environment.
- Do not start the independent review before the owner has completed the
  first-round proposal decisions and the author has written the resulting
  Slice 7 plan.

## Work plan

### 6.1 Build the proposal register

Deduplicate overlapping Slice 0–5 findings without losing provenance. Every
proposal row contains:

| Field | Allowed content |
| --- | --- |
| ID and proposal | Stable local ID and exact requested change |
| Sources | All Slice 0–5 records that support or constrain it |
| Understanding | Clear, needs evidence, or unclear, with the unresolved question |
| Risk | Low, medium, or high plus a concrete failure mode |
| Effort | S, M, or L plus dependencies/external lead time |
| Recommendation | Include, postpone, reject, or needs clarification |
| Destination | Slice 7, 10, 20, 30, 40, 50, 60, 70, or later release |
| HITL decision | Accepted, postponed, rejected, or needs clarification |
| Verification | Minimum proof if accepted |

No-change findings may be grouped by coherent surface. Actionable findings are
never hidden inside a grouped summary. Conflicting recommendations are shown
to the owner rather than silently resolved by the author.

### 6.2 Conduct the first interactive HITL session

Present a compact overview followed by decision rows grouped into:

1. prework maintenance eligible for Slice 7;
2. feature-scope and architecture allocations for Slices 10–70;
3. external prerequisites and unresolved Slice 0 choices; and
4. postponed/rejected proposals that require confirmation.

Record the owner's exact decision and rationale. A row marked “needs
clarification” stays out of Slice 7. If a decision changes a draft requirement,
architecture boundary, or feature allocation, update the relevant proposal
record before constructing Slice 7.

### 6.3 Write the accepted-work Slice 7 plan

Replace the contingent candidate table in the Slice 7 draft with the accepted
rows only. For each accepted row, specify exact files/surfaces, sequence,
dependencies, TDD or other bounded proof, stop conditions, and the no-extra-CI
posture appropriate to its blast radius. Preserve rejected and postponed rows
in the decision record, not as optional implementation work.

### 6.4 Independent read-only review

Commission one fresh independent review agent after the first-round HITL
record and Slice 7 plan exist. The reviewer may inspect files and git state but
must not edit, commit, push, dispatch workflows, or contact registries. Its
review must answer:

- Does every Slice 7 item have recorded owner acceptance?
- Did feature work, an unresolved owner decision, or an external prerequisite
  leak into prework implementation?
- Are exact files, dependencies, TDD/proof, and stop conditions complete?
- Are public-contract, ADR, worktree/main, and release-state boundaries
  respected?
- Is verification proportional and sufficient without manufacturing hosted
  CI ceremony?
- Is any accepted Slice 0–5 proposal missing or any rejected item present?

The raw review is retained as
`slice-6-slice-7-independent-review.md` with an explicit pass/fix verdict.

### 6.5 Apply at most two FIX-n cycles

The author, not the reviewer, applies plan corrections. Record each cycle in
the review artifact or a linked `slice-6-slice-7-fix-N.md`:

1. finding and severity;
2. accepted correction or reason for rejection;
3. exact plan change; and
4. reviewer verification result.

After FIX-1, request a read-only re-review. If material findings remain, apply
FIX-2 and request one final re-review. After two cycles, unresolved material
findings are presented to the owner; they are not silently waived and do not
trigger a third automatic cycle.

### 6.6 Final interactive HITL disposition

Present the revised Slice 7 plan, review verdict, rejected findings, and any
remaining risks. Record the owner's final decision. If accepted, update the
Slice 7 plan status and exact accepted scope. If not accepted, keep Slice 7
blocked/draft and record what must change; do not implement.

## Deliverables

1. `slice-6-proposal-register.md` — complete scored register with source links.
2. `slice-6-hitl-decisions.md` — first-round and final owner decisions with
   rationale and allocations.
3. Finalized `slice-7-accepted-prework-implementation-plan.md` containing only
   accepted prework.
4. `slice-6-slice-7-independent-review.md` plus up to two linked FIX-n records.
5. Overall-plan update recording Slice 6 disposition and the exact next action.

## Completion and verification

Slice 6 is complete only when:

- every actionable Slice 0–5 proposal appears once in the register with all
  source records and an owner decision;
- every accepted row appears in exactly one destination, and Slice 7 contains
  only accepted prework;
- the independent read-only review and any FIX-1/FIX-2 records are retained;
- the owner has made and recorded the final Slice 7 plan decision;
- decision, review, Slice 7, and overall-plan records agree; and
- scoped Markdown lint and `git diff --check` pass.

No implementation verification or hosted workflow is part of this
decision-and-planning slice.
