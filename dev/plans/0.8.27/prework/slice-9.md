---
title: FathomDB 0.8.27 prework Slice 9 - proposal review and decisions
status: ACTIVE
target_release: 0.8.27
observed_on: 2026-09-21
---

# Slice 9 - proposal review and direct decisions

## Plan and delta reconciliation

Outcome: consolidate Slices 0-8, score every proposal, decide release scope and
placement, prepare Slice 10, obtain independent design review and verification,
then activate one release state/board pointing to Slice 10. This slice changes
planning authority only; it does not implement product, dependency, tooling,
CI, or public-document corrections.

The original draft plan is adjusted as follows:

- correct the Memex-cutover statement while retaining F27-01 as a 0.8.27
  publication blocker;
- preserve requested-bucket `ExciseReport` counts and intentional non-PII
  closure/audit proofs;
- treat the F27 root cause as unknown until RED localization;
- reuse existing engine/SDK modules and current design-owner authority;
- make the 0.9.0 -> 0.8.27 refactor move and D27 -> 0.8.28 moves explicit
  decisions rather than implied edits;
- require the full comparator/feature/move prerequisites before structural
  work; and
- keep stale generated-map deletion and unrelated dependency/CI cleanup out.

## Requirements and acceptance

| ID | Requirement | Acceptance signal |
| --- | --- | --- |
| PW27-9A | Every Slice 0-7 proposal has an explicit disposition and later owner. | `proposal-register.md` has no unallocated row; Slice 8 remains empty. |
| PW27-9B | Release requirements, acceptance, design, testing, and sequencing agree with shipped code. | Plan/test approach and slice records incorporate independent code-grounded corrections. |
| PW27-9C | The next implementation slice is bounded and independently reviewed. | Slice 10 plan/design has no product feature work and package review has no unresolved high-severity finding. |
| PW27-9D | One machine state and board represent the active release. | Release-state/view/current-release/preflight checks pass and point to Slice 10 after closeout. |

## Decisions and next-slice design

The recommended decisions are recorded in
[`proposal-register.md`](proposal-register.md). They approve F27-01 plus the
bounded five-file semantic refactor, postpone all unproven D27 continuity
candidates, and admit only four focused Slice 10 preparation families:

1. current public/release/roadmap truth;
2. the `smol-toml` advisory and two action-comment corrections;
3. exact benign-digest secret-scan authority; and
4. release-local tooling/build prerequisites for the comparator pilot.

The Slice 10 plan/design live under `../features/slice-10/`. They require
behavioral RED/GREEN for truth/security/tooling checks, mechanical-only comment
and index edits, independent design and code review, and focused verification
before the repository gate. Product erasure and structural code remain
prohibited until their assigned slices.

## Implementation, review, verification, and status

Implementation is the durable decision package, scope reconciliation, and
eventual state/board activation. No behavioral code exists in prework, so TDD
and code review are not applicable. Material design changes require an
independent read-only design review; a separate independent verifier must run
the focused planning/document/state checks. Their records are
`prework-design-review.md` and `prework-verification.md`.

Status remains `REVIEW_PENDING` until those findings are resolved and the
release state/board validates. No temporary worktree exists. Next: Slice 10,
not authorized by this prework commission.
