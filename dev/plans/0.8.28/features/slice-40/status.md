---
title: FathomDB 0.8.28 Slice 40 — status
status: ACTIVE (Defects A, B, C GREEN on the branch; gate red only on the two orchestrator-owned release-state suites)
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
| C-W1 Defect C docs | done 2026-10-16: requirements § 1.3, REQ40-C1..C6, AC40-27..32; design § 2b; test-plan § 3b |
| C-W2 Defect C RED tests | done: `slice40_projected_fts_stream.rs`, 2 RED (route and plan witness, fallback witness), 3 GREEN regression net (test-plan § 5.3) |
| C-R1 | APPROVE-WITH-FIXES on `04bf5e5e6` (Q4 keep `projected_` prefix, Q5 no CROSS JOIN, Q6 seam unchanged; F1–F6 applied) |
| C-W4 Defect C fix | done at `a3830681d`: stream statement, fallback, witnesses, CHANGELOG; 5/5 Defect C tests GREEN, no test edited |
| C-W5 | default engine suite 474 passed; F2 test list all green; lint and lint-md exit 0; `agent-verify.sh` exit 1 on `test-check-board-currency` and `test-commission-manifest` only (release-state files, unchanged from W5); test-plan § 5.4, § 6.2 |

## Follow-ups (not in this slice)

- `search_text_only` can return fewer than `limit` results when `limit` is
  near 100 and the fixed 100-row FTS window holds duplicate rows
  (requirements § 4). Pre-existing; the orchestrator files the ledger
  entry at landing.
- Consider adding possessive/contraction fragments (s, t, ll, re, ve) to
  STOPWORDS; needs its own measurement (R1 F2). First data point on the
  10k TC-5 corpus: 50 of 100 queries carry a fragment, and `"s"` alone
  matches 3,912 of 10,000 documents; the median candidate set per query
  is 3,806 rows with fragments and 2,744 without.
- The public docs give `Σ 1/(60 + rank)`; the code uses `RRF_K = 30` and
  3:1 text weighting (design § 5).
- COMP-01 NFCorpus re-run: done, see `nfcorpus-recovery.md` (nDCG@10
  0.3087 → 0.3260, forecast 0.3260).
- Potential addition to this slice, not added, deferred by owner ruling
  2026-10-16: fielded
  title boost through declared `searchable` projections. On NFCorpus it
  measured +0.002 nDCG@10 with a confidence interval that includes zero
  (`nfcorpus-recovery.md` § hypothesis tests), so it waits for a corpus
  where titles are a real signal.
- COMP-02b projected-FTS latency: added to this slice as Defect C by owner
  ruling 2026-10-16 (requirements § 1.3, design § 2b). Evidence in
  `comp02b-projected-fts-latency.md`.

## Notes

- The worktree's `.venv` bootstrap ran `npm install` in `src/ts`, which
  rewrote `src/ts/package-lock.json`; restored with `git restore` (R1 F5)
  and never committed.
