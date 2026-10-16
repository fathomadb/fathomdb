---
title: FathomDB 0.8.28 Slice 40 — status
status: ACTIVE (W4 GREEN; W5 blocked on two release-state suites outside the slice)
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
| R1 | APPROVE-WITH-FIXES on `6360cd61a` (findings F1–F6; Q1 keep positional rank, Q2 follow-up, Q3 add short-token AC-038 rows) |
| W3 | done: F1 AC-038 short-token rows (RED), F2 possessive tests, F6 design wording; F3 and F4 apply at W4/W5; F5 lockfile restored |
| W4 | done at `13879d4af`: every Slice 40 test GREEN, no test edited; CHANGELOG `### Fixed` entries (F3) |
| W5 | engine suite (default features) exit 0, 474 passed; `fathomdb-query` exit 0; lint and lint-md exit 0; `agent-verify.sh` exit 1: Python suites failed on an unclean tree and passed on rerun; `test-check-board-currency` and `test-commission-manifest` fail on the 0.8.28 release-state files from the baseline (Slice 30 and 31 not LANDED, Slice 31 SHA not on the board); needs an orchestrator ruling (test-plan § 6) |
| R2 | not started |

## Follow-ups (not in this slice)

- `search_text_only` can return fewer than `limit` results when `limit` is
  near 100 and the fixed 100-row FTS window holds duplicate rows
  (requirements § 4). Pre-existing; the orchestrator files the ledger
  entry at landing.
- Consider adding possessive/contraction fragments (s, t, ll, re, ve) to
  STOPWORDS; needs its own measurement (R1 F2).
- The public docs give `Σ 1/(60 + rank)`; the code uses `RRF_K = 30` and
  3:1 text weighting (design § 5).
- Re-run COMP-01 NFCorpus after the fix to measure the recovered nDCG@10.

## Notes

- The worktree's `.venv` bootstrap ran `npm install` in `src/ts`, which
  rewrote `src/ts/package-lock.json`; restored with `git restore` (R1 F5)
  and never committed.
