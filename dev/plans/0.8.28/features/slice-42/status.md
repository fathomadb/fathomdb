---
title: FathomDB 0.8.28 Slice 42 — status
status: PLANNED (W1 done 2026-10-16; R1 next)
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 42 status

Step log for [plan.md](plan.md). Commits are on
`slice/0.8.28-42-test-hygiene-and-published-cache` (base `d10ad65d5`).

| Step | State (2026-10-16) |
| --- | --- |
| W1 docs | done: this directory, the master-plan section, ladder entries for Slices 40 and 42 |
| R1 plan review | not started |
| O1 rulings | open: Q-A1 (the test lines), Q-B1 (the C3 threshold), Q-B2 (install the 0.8.27 CLI into scratch) |
| A-W2 to A-W5 | not started |
| B-W1, B-W2 | not started |
| R-INV, R2, O2 | not started |

## Notes

- Ledger seq 312 says "six sibling tests". The binary has four other
  tests that are not ignored, and two ignored ones (the negative control
  and the crash victim). The requirements use four.
