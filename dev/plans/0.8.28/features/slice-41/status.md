---
title: FathomDB 0.8.28 Slice 41 — status
status: R0 APPROVE-WITH-FIXES applied (F1–F15); owner rulings recorded 2026-10-16; W2 next
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
| W2 | implementer | RED tests (test-plan § 2, § 3) | not started |
| W4 | implementer | Core module, bindings, parity map, docs; no test edits | not started |
| W4 surface | implementer | Write the allowlist entry `_slice41_context_pack_signature` (design § 5) and the decision-index row, both citing the 2026-10-16 owner rulings | not started |
| W5 | implementer | Scoped checks, then `./scripts/agent-verify.sh` once on the final tree | not started |
| M | orchestrator | AC41-27 differential and M1 preview now; after the Unit H head: M1, § 4.2, M3, § 4.5 | waiting on Slice 40 Unit H |
| R2 | codex | Code review | not started |

## Rules

- Planning docs only at W1. No test or code yet.
- Every expected value in the in-repo fixtures is hand-derived from
  REQ41-P2 to P9 and checked by hand. The AC41-12 fixture's expected text
  is written by hand from the rules and checked against a reading of
  `render_evidence`, not captured from either implementation. W2 records
  the derivation below.
- This slice does not edit `dev/plans/release-state-0.8.28.json`,
  `dev/plans/runs/STATUS-0.8.28.md` or any generated block.
