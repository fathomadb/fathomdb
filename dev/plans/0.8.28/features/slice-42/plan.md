---
title: FathomDB 0.8.28 Slice 42 — plan
status: PLANNED (W1 done; R1 APPROVE-WITH-FIXES applied; Q-A1 ruled 2026-10-16; implementation waits for the coordinator's go, after Slices 40 and 41)
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 42 plan

Branch `slice/0.8.28-42-test-hygiene-and-published-cache`, cut from
`slice/0.8.28-40-fts-ranking-defects` at `d10ad65d5`, worktree
`../fathomdb-worktrees/slice-0.8.28-42`. It depends on Slice 40, which is
in progress and not landed; the branch rebases onto the release branch
once Slice 40 lands. Slice 41 (also cut from Slice 40) lands first, so
Slice 42 rebases onto it before R2.

Unit A edits one test file. Unit B writes one document. No product code,
public surface, schema, `Cargo.toml` or workflow changes.

## Steps

| Step | Who | What | Done when |
| --- | --- | --- | --- |
| W1 | implementer | [requirements.md](requirements.md), [design.md](design.md), [test-plan.md](test-plan.md), this plan, [GOAL.md](GOAL.md), [status.md](status.md); the Slice 42 section of the master plan; ladder entries for Slices 40 and 42 in the release state | Committed; `./scripts/agent-lint-md.sh` and `./scripts/check-release-state-views.sh` pass |
| R1 | reviewer (orchestrator runs it) | Plan review: Unit A option choice, the exact lines, the leak checks; Unit B routes, evidence and criteria | Verdict recorded in `review.md`; findings applied |
| O1 | owner | Rulings Q-A1, Q-B1 and Q-B2 (all ruled 2026-10-16) (design § 3) | Rulings in the todos ledger and in status.md |
| A-W2 | implementer | RED baseline: 30 parallel runs at the base (test-plan § 2.1) | Failure count in status.md |
| A-W4 | implementer | The ruled edit only (design § 1.4) | 0 of 30 parallel runs fail; alone 5 of 5; serial 3 of 3 |
| A-W5 | implementer | Leak checks (test-plan § 2.3), scope diff, lint, then `./scripts/agent-verify.sh` once on the final clean tree | Results in status.md |
| B-W1 | implementer | Evidence E1 to E3 and the route measurements (test-plan § 3) | Commands and outputs collected in the scratch directory |
| B-W2 | implementer | `investigation.md` (design § 2.6) | Committed; lint-md passes |
| R-INV | reviewer | Review of `investigation.md`: evidence supports each row, the recommendation follows from C1 to C3 | Verdict recorded; findings applied |
| R2 | codex | Code review of the Unit A change | APPROVE; orchestrator cherry-picks |
| O2 | owner | Ruling on the recommendation | Ruling in the ledger; any implementation goes to a new or extended slice |

Unit A and Unit B are independent after O1; they can run in either order.

## Rules

- TDD: the existing test, run in parallel, is the RED case. A-W4 edits
  only the ruled lines. A test that fails for another reason is reported,
  not edited.
- Unit B leaves the tree clean: probes and uncommitted edits stay in the
  scratch directory or are reverted, and their results are copied into
  `investigation.md`.
- No upstream issue, PR or release request.
- No push.
