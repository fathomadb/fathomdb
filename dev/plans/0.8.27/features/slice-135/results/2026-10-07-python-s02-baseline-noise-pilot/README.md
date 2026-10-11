---
title: Slice 135 installed Python S02 baseline-only timing pilot
status: DIAGNOSTIC_BASELINE_ONLY
target_release: 0.8.27
---

# Installed Python S02 baseline-only timing pilot — 2026-10-07

Five separate serial blocks exercised the verified 0.8.26 installed wheel.
Each block retained one warm-up and three measured fresh-database sequences.
All **15 measured sequences** passed the runner's state oracle, and the
[independent audit](audit.json) recomputed their identities, durations and
materialized state. A surviving erased source was rejected as a negative
control. The [raw manifest](manifest.json), per-attempt JSON, commands,
stdout/stderr and GNU Time reports are retained in the five block directories
and sealed by [SHA256SUMS](SHA256SUMS).

## Boundary and result

The new [timed runner](../../../../../../../scripts/slice135_python_s02_timing.py)
starts before installed SDK `Engine.open` and ends after the reopened engine
closes. The sequence includes 36 governed writes, vector projection drain,
text/vector/hybrid search, graph/evidence resolution, erasure and retry,
SDK reads of post-erasure and reopened state, and both closes. Direct SQLite
canonical-row checks and semantic assertions run **after** the product timer;
their duration is recorded separately. Each attempt uses a new process and
real temporary SQLite database. The baseline source is
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`; the wheel SHA-256 is
`7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282`.
The manifest and raw observations bind the runner, both helper files, corpus,
graph records and installed native module.

| Block | Product sequence, three samples (ms) | Median (ms) |
| --- | --- | ---: |
| 1 | 5367.936, 5339.991, 5387.194 | 5367.936 |
| 2 | 5422.961, 5357.333, 5344.365 | 5357.333 |
| 3 | 5387.704, 5433.172, 5421.597 | 5421.597 |
| 4 | 5370.917, 5405.054, 5365.041 | 5370.917 |
| 5 | 5387.962, 5420.380, 5371.860 | 5387.962 |

Across measured samples the median is **5387.194 ms**, minimum 5339.991 ms
and maximum 5433.172 ms. The block-median spread is **64.264 ms**, or 1.20%
of the median block median. Reopen with the default model takes a median
4398.351 ms, about 82% of the sequence. Independent verification takes a
median 1.031 ms and is excluded from the product duration. These are
descriptive pilot observations, not a 0.8.26/0.8.27 comparison.

The independent audit can be rerun from the Slice 135 checkout root:

```sh
python3 dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-s02-baseline-noise-pilot/audit.py \
  dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-wheel-baseline/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl \
  dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-s02-baseline-noise-pilot/timed-runner.snapshot.py \
  dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-s02-baseline-noise-pilot/s01-helper.snapshot.py \
  dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-s02-baseline-noise-pilot/s02-helper.snapshot.py
```

## Limits and next action

This pilot retained host, kernel and machine identity, process resource
reports and zero child swaps, but did not capture full start/end host
environment snapshots for every block. Each block has only three measured
sequences, so p95/p99 and a paired uncertainty claim are unsupported. The
fresh process per attempt includes default-model initialization twice and
does not represent a warmed persistent process. No candidate or contention
condition ran. Do not freeze the full S02 comparison protocol from this
pilot alone.

The next pilot must add complete per-block environment snapshots and a
declared warm/cold process condition. Then freeze counts and the paired
reporting rule before alternating the baseline and repaired candidate.
Run the same timed boundary through TypeScript where the API permits and
report the new Rust SDK as candidate-only. Focused Python tests passed 5/5;
Ruff check and format passed for the new runner and test. The full repository
gate remains separately open.
