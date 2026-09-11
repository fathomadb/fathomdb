# Slice 77 result

Slice 77 completed with no eligible second treatment. Statement reuse remains the optimization anchor, but it does not satisfy AC-020.

## Retained anchor timing

Seven fresh processes ran the unchanged registered AC-020 test against the uninstrumented statement-reuse anchor.

| Metric | Result |
| --- | ---: |
| Sequential median | 174 ms |
| Concurrent median | 86 ms |
| Scaling ratio | 2.0233x |
| Sequential IQR | 5 ms |
| Concurrent IQR | 3 ms |
| AC-020 passes | 0/7 |

The registered bound was approximately 31--33 ms in these runs. At unchanged sequential performance, concurrent time would need roughly another 62% reduction to reach the median bound of about 32.6 ms. Dispersion remained below 10%.

## Residual attribution

The first direct post-cache CPU profile met the 50-sample gate and showed active SQLite lock, allocation, and page-cache stacks. It did not provide off-CPU wait attribution or the lookaside miss-size/full census needed to select a bounded correction. Vector conversion and parser/preparation were below the sealed 10% threshold.

Consequently, no treatment entered RED/GREEN implementation. The existing registered AC-020 failure remains the performance RED. No product correction, matched A/B campaign, AC-072 guard, or Slice 71B write guard was applicable. No broad regression ran.

All raw logs, profiles, reports, hashes, and the machine-readable summary are retained beside this record.
