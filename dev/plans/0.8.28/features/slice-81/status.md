---
title: FathomDB 0.8.28 Slice 81 — status
status: IMPLEMENTED_PENDING_GATE
target_release: 0.8.28
---

# Slice 81 status

The bounded investigation is implemented and measured. Final repository gate,
results review and landing are pending; no production erasure/SLA pass is claimed.
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
| Final repository gate / landing | PENDING; owning thread records actual result |

Implementation baseline was `a81ec9928` on `dev/0.8.28`; original planning baseline
was `3fc2df15e4dfadd67b9fb57e5e7cea3d62613ead`. The isolated worktree
`/tmp/fathomdb-slice81`, branch `slice/0.8.28-81`, was used to keep one writer per
checkout. Source froze at `e632d5e9a65379622dbde32cf2591f81a34917a1` before
recovery; subsequent changes are documentation. The owning thread merges back
and removes the temporary worktree/branch after validation. No release tag or
publication is part of this slice.
