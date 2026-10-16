---
title: FathomDB 0.8.28 Slice 41 — status
status: DRAFT (W1 planning docs committed; review next)
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
| R0 | reviewer subagent | Review of W1, including test-plan § 6 Q1–Q6 | not started |
| W2 | implementer | RED tests (test-plan § 2, § 3) | not started |
| W4 | implementer | Core module, bindings, conformance maps, docs; no test edits | not started |
| W5 | implementer | Scoped checks, then `./scripts/agent-verify.sh` once on the final tree | not started |
| M | orchestrator | M1 preview; after the Unit H head: M1, § 4.2, M3, § 4.5 | waiting on Slice 40 Unit H |
| R2 | codex | Code review | not started |

## Rules

- Planning docs only at W1. No test or code yet.
- This slice does not edit `dev/plans/release-state-0.8.28.json`,
  `dev/plans/runs/STATUS-0.8.28.md` or any generated block.
