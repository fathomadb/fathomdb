---
title: FathomDB 0.8.28 Slice 42 — status
status: PLANNED (W1 done 2026-10-16; R1 APPROVE-WITH-FIXES applied; Q-A1 ruled 2026-10-16; implementation waits for the coordinator's go, after Slices 40 and 41)
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 42 status

Step log for [plan.md](plan.md). Commits are on
`slice/0.8.28-42-test-hygiene-and-published-cache` (base `d10ad65d5`).

| Step | State (2026-10-16) |
| --- | --- |
| W1 docs | done: this directory, the master-plan section, ladder entries for Slices 40 and 42 |
| R1 plan review | APPROVE-WITH-FIXES (F1–F13) on `4e651fdfb`; applied: F1 todos ledger and TC ids named; F2 Slice 40 depends on 31; F3 Slice 41 entry (NOT_STARTED, lands before 42; its docs live on its own branch, so the state cites the master plan); F4 board Slice 31 row; F5 no CI job runs the binary; F6 macOS coverage note; F9–F11 E1 control, E2/E3 caveats, route additions; F12 C3 precision |
| O1 rulings | Q-B1 ruled 2026-10-16 (10%, with the § 2.4 precision); Q-B2 ruled 2026-10-16 (allowed, scratch directory only); Q-A1 ruled 2026-10-16: option 2 exactly as design § 1.4 (helper after line 56; lines 366 and 429 call `fd_count_under(dir.path())`; line 437 unchanged; the macOS and Windows vacuous-check gap is accepted and recorded) |
| Implementation | waiting: Slice 42 lands after Slices 40 and 41; Unit A RED/GREEN and leak checks, and the Unit B runs (idle host), start on the coordinator's go |
| A-W2 to A-W5 | not started |
| B-W1, B-W2 | not started |
| R-INV, R2, O2 | not started |

## Notes

- Todos ledger seq 343 says "six sibling tests". The binary has four other
  tests that are not ignored, and two ignored ones (the negative control
  and the crash victim). The requirements use four.
