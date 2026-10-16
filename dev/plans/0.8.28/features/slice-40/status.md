---
title: FathomDB 0.8.28 Slice 40 — status
status: ACTIVE (W1, W2 done; awaiting R1)
target_release: 0.8.28
observed_on: 2026-10-15
---

# Slice 40 status

Step log for [plan.md](plan.md). Commits are on
`slice/0.8.28-40-fts-ranking-defects` (base `574b09e78`). The output record
for phase 1 is `output.json` in this directory.

| Step | State (2026-10-15) |
| --- | --- |
| W1 docs | done; see output.json for the commit |
| W2 RED tests | done; 6 RED in `fathomdb-query`, 12 RED in `slice40_fts_ranking.rs`, guards green (test-plan § 5) |
| R1 | not started |
| W3 | not started |
| W4 | not started |
| W5 | not started |
| R2 | not started |

## Notes

- The worktree's background `.venv` bootstrap ran `npm install` in
  `src/ts`, which rewrote `src/ts/package-lock.json`. That change is not
  part of the slice and is not committed.
- No production code changed in phase 1.
