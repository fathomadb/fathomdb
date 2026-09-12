---
title: FathomDB 0.8.26 Slice 6 — proposal review and HITL decisions
status: DRAFT
---

# Slice 6 plan — proposal review and HITL decisions

## Purpose

Collect Slices 0–5 into a concise decision package, obtain interactive owner
rulings, and replace the provisional Slice 7 plan with an approved
implementation plan. This document describes the future session; it does not
record decisions that have not happened.

## Proposal scoring

Score every item on four independent axes:

- **understood:** high, medium, or low;
- **stability risk:** critical, high, medium, or low;
- **effort:** XS, S, M, L, or XL; and
- **disposition:** include, postpone, reject, or needs more information.

Stability risk dominates effort. A low-effort change is not recommended when
it weakens write, replay, lifecycle, privacy, or compatibility invariants.

## Interactive workflow

1. Present the proposal register, leading with the highest-value lowest-risk
   items and separating mandatory preparation from optional cleanup.
2. Ask the HITL to rule each item and each unresolved architecture decision.
3. Write the rulings, rationale, owner, and affected slices durably.
4. Replace the provisional Slice 7 plan/design with only approved work.
5. Ask one read-only subagent to review that exact Slice 7 plan against the
   rulings, repository invariants, and acceptance coverage.
6. Perform no more than two `FIX-n` cycles. Preserve each finding and its
   resolution in a review record.
7. Present the reviewed changes interactively to the HITL, capture the final
   decision, and update the overall release plan.

## Required outputs

- `proposal-register.md`;
- `hitl-decisions.md`;
- the replaced Slice 7 `plan.md` and `design.md`;
- `slice-7-independent-review.md`; and
- updated scope, ladder state, and immediate-next-action in the overall plan.

## Exit criteria

Every proposal and decision has an explicit ruling; Slice 7 contains no
unapproved work; the independent review is closed in at most two fix cycles;
and the HITL has approved or rejected execution. Feature implementation cannot
start from a draft or inferred decision.
