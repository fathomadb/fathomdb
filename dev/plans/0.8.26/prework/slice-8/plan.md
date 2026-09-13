---
title: FathomDB 0.8.26 Slice 8 — proposal review and HITL decisions
status: COMPLETE
---

# Slice 8 plan — proposal review and HITL decisions

## Slice-complete workflow

This plan adopts the [lean slice execution contract](../../slice-execution-contract.md).
It reconciles all draft and allocated changes, completes requirements and
design decisions, obtains the specified plan/design review, verifies ruling
coverage, writes status, and performs no product implementation.

## Purpose

Collect Slices 0–7 into a concise decision package, obtain interactive owner
rulings, allocate each approved item to the correct release stage, and replace
the provisional Slice 9 plan with an approved implementation plan. Completed
outputs record decisions only where backed by cited HITL ledger rulings.

## Proposal scoring

Score every item on four independent axes:

- **understood:** high, medium, or low;
- **stability risk:** critical, high, medium, or low;
- **effort:** XS, S, M, L, or XL; and
- **disposition:** include, postpone, reject, or needs more information.

For included items, record one delivery placement: Slice 9, an exact reserved
post-10 slice, an existing feature slice, or Slice 50. Stability risk dominates
effort. A cheap change is not recommended when it weakens write, replay,
lifecycle, privacy, compatibility, build, or supply-chain invariants.

## Interactive workflow

1. Present the proposal register, leading with the highest-value lowest-risk
   items and separating product scope from build and delivery reliability.
2. Ask the HITL to rule each item, unresolved architecture decision, and
   proposed delivery placement.
3. Write the rulings, rationale, owner, dependency, and affected slices
   durably.
4. Replace the provisional Slice 9 plan/design with only approved work assigned
   to Slice 9; allocate later work explicitly in the overall and owning plans.
5. Ask one read-only subagent to review the exact Slice 9 plan against the
   rulings, repository invariants, and acceptance coverage.
6. Perform no more than two `FIX-n` cycles. Preserve every finding and
   resolution in a review record.
7. Present the reviewed plan and later-slice allocations interactively to the
   HITL, capture the final decision, and update the overall release plan.

## Required outputs

- `proposal-register.md`, including Slice 6–7 failure-root-cause items;
- `hitl-decisions.md` with disposition and delivery placement;
- the replaced Slice 9 `plan.md` and `design.md`;
- `slice-9-independent-review.md`;
- any approved reserved post-10 slice plan/design stubs; and
- updated scope, ladder state, and immediate-next-action in the overall plan.

## Exit criteria

Every proposal and failure root cause has an explicit ruling and placement;
Slice 9 contains no unapproved or later-dependent work; the independent review
is closed in at most two fix cycles; and the HITL has approved or rejected
execution. Feature implementation cannot start from a draft or inferred
decision.
