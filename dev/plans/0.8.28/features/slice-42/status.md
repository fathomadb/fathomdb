---
title: FathomDB 0.8.28 Slice 42 — status
status: IN_PROGRESS (R-DES APPROVE-WITH-FIXES applied 2026-10-31; phase 2 started)
target_release: 0.8.28
observed_on: 2026-10-31
---

# Slice 42 status

Step log for [plan.md](plan.md). Commits are on
`slice/0.8.28-42-test-hygiene-and-published-cache` (base `59079d5c5`,
`dev/0.8.28`).

| Step | State (2026-10-31) |
| --- | --- |
| P1 phase 1 docs | done: this directory rewritten for the Unit B re-scope; master-plan Slice 42 section updated |
| R-DES design review | APPROVE-WITH-FIXES on `d6cfab127` (no P1); ten findings applied, see [review.md](review.md) |
| A Unit A | not started (Q-A1 ruled 2026-10-16; line numbers re-checked at `59079d5c5`, unchanged) |
| B-RED/GREEN | not started |
| R-CODE | not started |
| V verification | not started |
| L land | not started |

## Phase 1 findings (2026-10-31)

- **Compile probe.** In a scratch export of `59079d5c5` with `rusqlite`
  0.41.1, `libsqlite3-sys` 0.39.1 and the root patch removed, five
  `trace_v2` calls and three `busy_handler` test calls needed edits
  (design § 2.2). After them, workspace check and clippy were clean.
- **Test probe.** `cargo test -p fathomdb-schema -p fathomdb-engine
  --features test-hooks,operator` on SQLite 3.54.1: 1,637 passed, 3
  failed, 38 ignored. The three failures are parallel-run flakes present
  at the base (design § 2.5).
- **Pre-existing flake, outside this slice.** `slice50_evidence.rs`:
  `evidence_search_races_are_snapshot_atomic_or_wholly_refused` and
  `evidence_linearizes_at_sidecar_and_resolver_return_seams` fail with
  `EvidenceUnavailable` in 6 of 15 parallel runs at `59079d5c5`; 5 of 5
  serial runs pass. For the coordinator to place.
- **Windows VM.** `gh-runner-wonl-win11` exists and is shut off; it is
  started for verification (test-plan § 4.3).
- **macOS.** No local host; the CI matrix covers it in 0.8.28 final
  verification.

## Superseded

The 2026-10-16 Unit B investigation (routes R1–R12, evidence E1–E3,
rulings Q-B1 and Q-B2, steps B-W1, B-W2, R-INV and O2) was superseded by
the owner's re-scope on 2026-10-31 before any of it ran. The 2026-10-16
plan review (R1, APPROVE-WITH-FIXES on `4e651fdfb`) covered Unit A as
it stands and the old Unit B.
