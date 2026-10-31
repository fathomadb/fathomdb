---
title: FathomDB 0.8.28 Slice 42 — plan
status: PLANNED (phase 1 docs rewritten 2026-10-31 for the Unit B re-scope; awaiting design review)
target_release: 0.8.28
observed_on: 2026-10-31
---

# Slice 42 plan

Branch `slice/0.8.28-42-test-hygiene-and-published-cache`, fast-forwarded
to `dev/0.8.28` at `59079d5c5`; worktree
`../fathomdb-worktrees/slice-0.8.28-42`. It lands on `dev/0.8.28` before
Slices 50 to 90, so they measure on the SQLite that ships.

- **Unit A** edits one test file (owner ruling Q-A1).
- **Unit B** moves to `rusqlite` 0.41.1 and `libsqlite3-sys` 0.39.1,
  deletes the vendored `libsqlite3-sys` 0.38.1 and its patch, reworks the
  Windows verbatim-path guard into a version check, and updates the
  records ([design.md](design.md) § 2).

## Steps

| Step | Who | What | Done when |
| --- | --- | --- | --- |
| P1 | implementer | Phase 1: [requirements.md](requirements.md), [design.md](design.md), [test-plan.md](test-plan.md), this plan, [status.md](status.md), [GOAL.md](GOAL.md); the master-plan Slice 42 section | Committed; `./scripts/agent-lint-md.sh` and `./scripts/check-release-state-views.sh` pass |
| R-DES | Fable reviewer (orchestrator runs it) | Design review of requirements, AC, design and test plan | Verdict recorded in `review.md`; findings applied |
| A | implementer | Unit A: RED baseline loop at the base, the ruled edit, the GREEN loops and the leak checks (test-plan § 2) | AC42-01 to AC42-06 recorded |
| B-RED/GREEN | implementer | B-RED-1 → B-GREEN-1, B-RED-2 → B-GREEN-2, B-RED-3 → B-GREEN-3 (design § 2.6), then the docs commit (design § 2.7) | Each RED fails at its own sha, evidence in its body; each GREEN passes; RED shas audited in a detached checkout |
| R-CODE | codex (orchestrator runs it) | Code review of the slice branch | APPROVE, or findings fixed and re-reviewed |
| V | implementer | Final-tree verification: full gate, scratch consumer, Windows VM, perf baseline (test-plan § 4) | AC42-11, AC42-13, AC42-14, AC42-16 recorded; open platform legs named |
| L | orchestrator | Land on `dev/0.8.28`; update the release state | Landed sha recorded |

## Rules

- TDD: one RED commit per behaviour, failing at its own sha for the
  stated reason, with "RED evidence:" in its body; then its GREEN. No
  coverage commits that pass on arrival. Docs-only commits are exempt.
- Unit A edits only the ruled lines. A test that fails for another
  reason is reported, not edited. Unit B edits tests only where the
  `rusqlite` 0.41 API or a deleted function forces it (design § 2.2,
  § 2.4).
- BGE embedding runs on the RTX 3090 `cuda:0` only (test-plan § 1).
- No upstream issue, PR or release request. No push, no tag.
