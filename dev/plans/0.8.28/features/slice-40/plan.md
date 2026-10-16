---
title: FathomDB 0.8.28 Slice 40 — plan
status: ACTIVE (W1, W2 done; R1 next)
target_release: 0.8.28
observed_on: 2026-10-15
---

# Slice 40 plan

Branch `slice/0.8.28-40-fts-ranking-defects`, cut from `release/0.8.28` at
`574b09e78`, worktree `../fathomdb-worktrees/slice-0.8.28-40`. Rust only;
the Python and TypeScript bindings call the same engine functions and need
no change.

## Steps

| Step | Who | What | Done when |
| --- | --- | --- | --- |
| W1 | implementer | [requirements.md](requirements.md), [design.md](design.md), [test-plan.md](test-plan.md), this plan, [GOAL.md](GOAL.md), [status.md](status.md) | Docs committed; `./scripts/agent-lint-md.sh` passes |
| W2 | implementer | RED tests: `fathomdb-engine/tests/slice40_fts_ranking.rs`, the `fathomdb-query` test module, the AC-038 mirror | Tests committed; RED set recorded in test-plan § 5 |
| R1 | codex (orchestrator runs it) | Review of W1 and W2: requirements, design decisions, test adequacy, open questions Q1–Q3 | Verdict recorded in `review.md` |
| W3 | implementer | Apply R1 findings to docs and tests | Findings disposed; tests still RED for the right reasons |
| W4 | implementer | GREEN: `fuse_three_arms` per-arm credited set; `content_tokens` `is_empty()` filter; doc comments and `fathomdb-query/README.md` (design § 3, § 5). No test edits | Every Slice 40 test passes |
| W5 | implementer | Verify: `./scripts/agent-lint.sh`, `./scripts/agent-typecheck.sh`, then `./scripts/agent-verify.sh` once on the final clean tree | Gate green; result in status.md |
| R2 | codex | Code review of the slice branch | APPROVE; orchestrator cherry-picks |

## Rules

- TDD: W4 changes no test. A test that fails for a reason other than the
  slice's defects is reported, not edited.
- The orchestrator adds the ladder entry to the release state; this slice
  does not touch `dev/plans/release-state-0.8.28.json`,
  `dev/plans/runs/STATUS-0.8.28.md`, `dev/plans/plan-0.8.28.md` or any
  generated block.
- Follow-ups recorded, not done here: the candidate-window shortfall at
  `limit` near 100 (requirements § 4); the stale `Σ 1/(60 + rank)` formula
  in the public docs (design § 5); an NFCorpus re-run after W4 to measure
  the recovered nDCG@10.
