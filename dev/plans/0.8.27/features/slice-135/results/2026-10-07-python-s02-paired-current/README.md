---
title: Slice 135 installed Python S02 paired whole-product timing
status: AUDITED_PYTHON_SUBSET
target_release: 0.8.27
---

# Installed Python S02 paired timing — 2026-10-07

The [frozen Python S02 subset](../../s02-python-comparison-protocol.json)
compared installed 0.8.26 and repaired 0.8.27 candidate wheels in five
alternating baseline/candidate pairs. Every block used one warm-up and 20
measured fresh-process, real-database sequences. The
[independent audit](audit.json) accepted all **100 measured samples per
version**, checked each retained raw attempt, installed wheel and native
identity, expected graph/evidence and erasure state, host snapshots, GNU Time
resources and reported statistics. A surviving-source mutation was rejected.
The [run manifest](run-manifest.json), per-block commands, manifests, raw
samples, output and resource reports are retained. [SHA256SUMS](SHA256SUMS)
seals this receipt.

Baseline source: `f99e002f0d2e4002f3694c9f8d4986b56089edaa`; wheel
SHA-256: `7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282`.
Candidate product source: `8587b0571f4e6477045e60ddd341b0cd28f5a3cb`;
wheel SHA-256: `813255e452a5a001fca5a0943412309a277fff67d6128544cfb8850d87dffe38`.
The candidate's `Cargo.toml`, `Cargo.lock`, `src/rust` and `src/python` bytes
still match that product source at receipt time. This compares the installed
Python boundary from `Engine.open` through materialized post-erasure/reopened
state and reopened `close`; independent SQLite checks follow the timer.
Default-model startup and two fresh opens are included.

| Whole-product nearest-rank statistic | 0.8.26 | Candidate | Candidate delta |
| --- | ---: | ---: | ---: |
| p50, 100 samples | 5,413.236 ms | 5,421.778 ms | +0.158% |
| p95, 100 samples | 5,527.307 ms | 5,531.506 ms | +0.076% |
| Observed min–max | 5,302.297–5,626.041 ms | 5,334.743–5,707.823 ms | Descriptive |

| Pair | Order | p50 delta | p95 delta | Host swap-counter change, baseline/candidate |
| --- | --- | ---: | ---: | --- |
| 1 | Baseline, candidate | +1.845% | +3.485% | +1/+4 pages |
| 2 | Candidate, baseline | +0.038% | +0.264% | +3/+33 pages |
| 3 | Baseline, candidate | +0.187% | −0.574% | +1/+3 pages |
| 4 | Candidate, baseline | +0.205% | −0.405% | +22/+6 pages |
| 5 | Baseline, candidate | −0.425% | −0.766% | +81/+19 pages |

Pair delta medians were +0.187% at p50 and −0.405% at p95. All ten blocks
had host swap-counter movement, so there is **no warning-free pair** for a
clean-pair sensitivity estimate. The measured children had zero swap events
and zero major faults. No block met an invalidation condition. Host paging
movement remains a measurement limitation; it was not attributed to the
measured process or silently excluded. Five pairs do not support a
significance or equivalence claim.

GNU Time measured child peak RSS of 447,144–449,092 KiB for 0.8.26 and
312,980–315,812 KiB for the candidate. Median child user/system CPU seconds
were 29.00/11.34 versus 28.99/10.385. Both versions totaled 8 filesystem
input blocks and 1,160,800 output blocks across their 100 measured children.
These are whole-process resources, not stage costs.

To recompute the receipt from the Slice 135 checkout root:

```sh
python3 scripts/slice135_python_s02_pair_audit.py \
  --root dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-s02-paired-current \
  --protocol dev/plans/0.8.27/features/slice-135/s02-python-comparison-protocol.json \
  --baseline-wheel dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-wheel-baseline/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl \
  --candidate-wheel dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-frozen-error-fix/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl \
  --pairs 5 --output /tmp/slice135-python-s02-recomputed-audit.json
```

This is a controlled **Python S02 subset**, not the full system-latency
checkpoint. p99 is unsupported by the frozen protocol. TypeScript/Rust SDK
latency, S01/S03/C01, contention, Pareto coverage, robustness and
logic/exception matrices remain open. This result does not qualify the final
0.8.27 candidate or its release artifacts.
