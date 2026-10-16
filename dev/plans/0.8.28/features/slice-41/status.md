---
title: FathomDB 0.8.28 Slice 41 — status
status: R0 APPROVE-WITH-FIXES applied (F1–F15); Q1, Q4, Q5 pending owner ruling
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
| Owner | owner | Rule Q1 (LanceDB renderer in M3) and Q4/Q5 (tagged body format, decision-index row) | pending; docs carry the defaults |
| W2 | implementer | RED tests (test-plan § 2, § 3) | not started |
| W4 | implementer | Core module, bindings, parity map, docs; no test edits | not started |
| W4 gate | owner | Sign `_slice41_context_pack_signature` (design § 5); the allowlist is edited only after that. AC41-22 stays open until then | not started |
| W5 | implementer | Scoped checks, then `./scripts/agent-verify.sh` once on the final tree | not started |
| M | orchestrator | AC41-27 differential and M1 preview now; after the Unit H head: M1, § 4.2, M3, § 4.5 | waiting on Slice 40 Unit H |
| R2 | codex | Code review | not started |

## Rules

- Planning docs only at W1. No test or code yet.
- Every expected value in the in-repo fixtures is hand-derived from
  REQ41-P2 to P9 and checked by hand. Record that here when W2 lands them.
- This slice does not edit `dev/plans/release-state-0.8.28.json`,
  `dev/plans/runs/STATUS-0.8.28.md` or any generated block.
