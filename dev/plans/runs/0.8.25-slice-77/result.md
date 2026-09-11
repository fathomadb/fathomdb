# Slice 77 result

Slice 77 completed inconclusively with no authorized second treatment. Statement reuse remains the optimization anchor, but it does not satisfy AC-020.

## Retained anchor timing

Seven fresh processes ran the unchanged registered AC-020 test against the uninstrumented statement-reuse anchor.

| Metric | Result |
| --- | ---: |
| Sequential median | 174.216944 ms |
| Concurrent median | 86.531221 ms |
| Scaling ratio | 2.013342x |
| Sequential IQR | 4.196290 ms |
| Concurrent IQR | 3.125786 ms |
| AC-020 passes | 0/7 |

The full-precision median registered bound was 32.665677 ms. At unchanged sequential performance, concurrent time would need roughly another 62% reduction. Dispersion remained below 10%.

The original collector summary truncated the already-present failure diagnostics to integer millisecond markers. A focused RED/GREEN correction now prefers those full-precision durations, and the observations and summary were regenerated from retained raw logs without rerunning timing. The contemporaneous executor preflight and planned timeout were not retained; `environment-qualification.md` therefore limits this series to diagnostic confirmation rather than treatment selection or acceptance.

## Residual attribution

The first direct post-cache CPU profile captured 52 samples, but five are teardown inside the temporary profile boundary. Its 47 search samples miss the sealed minimum of 50, so the profile is inconclusive. It descriptively shows active SQLite lock, allocation, and page-cache stacks, but does not provide off-CPU wait attribution or the lookaside miss-size/full census needed to select a bounded correction. Its vector-conversion and parser/preparation percentages cannot be used against the 10% threshold.

Consequently, no treatment entered RED/GREEN implementation. The existing registered AC-020 failure remains the performance RED. No product correction, matched A/B campaign, AC-072 guard, or Slice 71B write guard was applicable. No broad regression ran.

All raw logs, profiles, reports, hashes, and the machine-readable summary are retained beside this record.
