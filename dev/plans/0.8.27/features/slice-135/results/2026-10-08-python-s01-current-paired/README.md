---
title: Slice 135 current-wheel installed Python S01 paired diagnostic
status: AUDITED_PAIRED_DIAGNOSTIC
target_release: 0.8.27
---

# Current-wheel installed Python S01 paired diagnostic — 2026-10-08

The [frozen protocol](../../s01-python-current-comparison-protocol.json) binds
baseline source `f99e002f0d2e4002f3694c9f8d4986b56089edaa` and wheel
`7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282`
to candidate source `d465cd56d2e863f900ea9a9da8bc372ca6a077c5` and wheel
`33d37be7c80183b063788889954da698ea959997fc13b61e356006a3c26c8894`.
It fixes two corpus sizes, five alternating pairs at each size, 1,000 warm
samples per text/vector/hybrid cell, and at least 20 seconds between blocks.
Both installed packages run against real databases, materialize result IDs and
check the seeded results at the Python call boundary. The campaign command and
full block evidence are in the local raw archive beside this note.

The [independent audit](independent-audit.json) accepted all 20 blocks and
60,120 observations, checked archived wheel/runner/protocol bytes, recomputed
per-cell percentiles, checked block order and idle intervals, and compared
resource and environment reports. A second audit of the copied archive was
byte-identical. [Negative controls](negative-controls.json) rejected a
reordered block, a wrong materialized text ID and a measured-child swap event.

Median of five within-pair warm p50 deltas, candidate relative to baseline:

| Rows | Text | Vector-bearing | Hybrid |
| --- | ---: | ---: | ---: |
| 32 | +4.53% | -1.62% | -2.92% |
| 256 | +0.98% | -1.68% | -1.49% |

The audit retains a measured-child major-fault warning in `32-01-baseline` and
host-only swap-drift warnings in `256-01-baseline` and `256-02-baseline`; no
measured child swapped. Warning-free sensitivity has four 32-row pairs and
three 256-row pairs. The 32-row hybrid p50 decrease remains negative in all
four warning-free pairs; other p50 cells have mixed signs. The full audit
contains p50/p95/p99 deltas, ranges, clean-pair values and block resources.
These are diagnostic observations, not an equivalence or release-wide speed
claim. The candidate wheel is source-bound to `d465cd56d`; later changes to
tests and harness files require an explicit final-candidate identity check.

The first independent-audit attempt lacked retained wheel copies. After their
exact bytes were added, the auditor incorrectly invalidated a recorded major
fault even though the frozen rule invalidates measured-child swap, and then
failed to match the block runner's major-fault warning text. The auditor was
corrected with a failing test and re-ran on unchanged timed blocks. No slow
observation was discarded. The raw archive remains local and untracked until
the end-of-phase retention decision.
