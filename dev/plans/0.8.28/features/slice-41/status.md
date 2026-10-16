---
title: FathomDB 0.8.28 Slice 41 — status
status: W2 RED and W4 GREEN done; two test rulings and the Unit H measurements open; code review next
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 41 status

Branch `slice/0.8.28-41-sdk-evidence-helper`, cut from
`slice/0.8.28-40-fts-ranking-defects` at `aea8e0457`. Worktree
`../fathomdb-worktrees/slice-0.8.28-41`.

| Step | Who | What | State (2026-10-16) |
| --- | --- | --- | --- |
| W1 | implementer | GOAL, requirements, design, test plan, this file | done; see the commit log |
| R0 | reviewer subagent | Review of W1, including Q1–Q6 | APPROVE-WITH-FIXES on `8e4f9a543`; F1–F15 applied with the orchestrator's rulings (test-plan § 6) |
| Owner | owner | Rulings 2026-10-16: allowlist addition approved with the R0 adjustments; tagged body opt-in with hazards documented and a decision-index row (can be revisited); LanceDB on the Slice 40 renderer in M3, labelled | done (test-plan § 6) |
| W2 | implementer | RED tests (test-plan § 2, § 3) | done at `86e15e333` (+ `338d1da9f` Cargo.lock); RED record in test-plan § 7.1 |
| W4 | implementer | Core module, bindings, parity map, docs; no test edits | done: core `3e0903d91`, surface `2ccc6afc6`, pin `334497e4c`, docs in the following commit; GREEN record in test-plan § 7.2 |
| W4 surface | implementer | Write the allowlist entry `_slice41_context_pack_signature` (design § 5) and the decision-index row, both citing the 2026-10-16 owner rulings | done: allowlist and parity map `2ccc6afc6`, re-issued pin `334497e4c`, decision-index row 64 |
| W5 | implementer | Scoped checks (the full gate waits for the code review) | see the report; open: the pinned surface-count tests and the hybrid drain timeout need rulings |
| M | orchestrator | AC41-27 differential and M1 preview now; after the Unit H head: M1, § 4.2, M3, § 4.5 | AC41-27 180/180 and the M1 preview done (test-plan § 7.3, § 7.4); the rest waits on Slice 40 Unit H |
| R2 | codex | Code review | not started |

## Rules

- Every expected value in the in-repo fixtures is hand-derived from
  REQ41-P2 to P9 and checked by hand. The AC41-12 fixture's expected text
  is written by hand from the rules and checked against a reading of
  `render_evidence`, not captured from either implementation. W2 recorded
  the derivation in the fixture's `_derivation` field. The fixture scales the
  caps to `item_chars` 40 and `budget_chars` 95, so a round over the cap
  stays small enough to check by hand.
- This slice does not edit `dev/plans/release-state-0.8.28.json`,
  `dev/plans/runs/STATUS-0.8.28.md` or any generated block.
