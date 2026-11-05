---
title: FathomDB 0.8.28 Slice 81 — status
status: COMPLETE
target_release: 0.8.28
---

# Slice 81 status

The bounded investigation is complete, reviewed by Astra and landed on
`dev/0.8.28` and `release/0.8.28`; no production erasure/SLA pass is claimed.
See [results](../../../../experiments/erasure-availability/RESULTS.md) for all
72 dispositions, controls, hashes, requirements and hypotheses.

| Work | State |
| --- | --- |
| Requirements, acceptance and plan reconciliation | Updated; original work, related implementation and scoped successor limits enumerated |
| Design and transaction seam | Reviewed by Astra; test-only real-writer callback implemented and interface documented |
| TDD implementation | Committed RED/GREEN groups for atomicity, A/B, recovery/deadlines, fixture/controller and monitor correction |
| Focused verification | 18 Rust tests and 14 Python controller tests pass; independent RED replay, GREEN verification and evidence reconciliation |
| Original 45-minute campaign | 10 complete / 13 incomplete / 49 never started; preserved unchanged |
| Reviewed continuation | 13 complete / 25 incomplete / 11 never started; stopped on monitor exit race, preserved unchanged |
| Remaining-allowance recovery | 11 complete; no retries, threshold changes or extra allowance |
| Final 72 unique replicas | 34 complete / 38 incomplete / 0 never started |
| Comparative screening | Four three-pair legacy cells pass; node loaded and property idle unqualified due to missing complete pair; all large fresh incomplete |
| Production physical-erasure finding | OPEN; known shipping ghost retained |
| Native availability, broad routing and numerical SLA | UNQUALIFIED; production successors required |
| Final repository gate / landing | Full collector: 205 ran, 204 passed, one stale root-inventory metadata failure; reviewed correction landed, targeted rerun passed all 8 tests |

Implementation baseline was `a81ec9928` on `dev/0.8.28`; original planning baseline
was `3fc2df15e4dfadd67b9fb57e5e7cea3d62613ead`. The isolated worktree
`/tmp/fathomdb-slice81`, branch `slice/0.8.28-81`, was used to keep one writer per
checkout. Source froze at `e632d5e9a65379622dbde32cf2591f81a34917a1` before
recovery; subsequent changes are documentation. Reviewed implementation and results
landed at `7ca1d182bef03c69199f538904ce8332842d80a2`. No release tag or publication
is part of this slice.

The full gate ran once at `e632d5e9a65379622dbde32cf2591f81a34917a1`
and exited 1 after 1,729 seconds. All lint, typecheck and strict security checks
passed; the test collector ran all 205 registered suites with no skips. Its only
failure was the exact engine-root inventory blob, corrected by reviewed metadata
in the landing commit. The targeted rerun passed all 8 tests in 10.223 seconds. No full gate was rerun.

The owner-requested separate native benchmark completed in 264.781348412 seconds
for 1,000 of 10,000 fresh node owners with five revisions each. Native logical
erasure succeeded; FTS physical residue remained. This single warm-cache result
is separate from the 72 campaign attempts and does not qualify a deletion SLA.

The temporary implementation worktree and branch were removed after fast-forward
landing in both release checkouts. Raw campaign and benchmark evidence remains in
`/tmp/fathomdb-slice81-campaign`, `/tmp/fathomdb-slice81-native-benchmark` and
`/tmp/fathomdb-slice81-evidence`; the compact benchmark receipt is committed.

## Follow-up and project reconciliation

Slice 81 is complete for now; no further POC execution is scheduled. The status,
results, native benchmark, interface inventory and temporary-worktree cleanup are
recorded above. Documentation and release-state checks passed after closeout.

HITL approved CH1 on 2026-11-03: [Slice 82](../slice-82/plan.md) owns physical
correctness/native cost and [Slice 83](../slice-83/plan.md) owns resumability/
availability. The release plan/ladder, current/history sequence and candidate
design, project roadmap and program sequencing index reflect this follow-up.
Neither the POC result nor this allocation closes the production erasure finding.
The physical-erasure guarantee/API decision is reserved for HITL at the end of 83.
