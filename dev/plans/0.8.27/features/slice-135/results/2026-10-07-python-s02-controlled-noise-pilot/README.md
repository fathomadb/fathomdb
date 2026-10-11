---
title: Slice 135 controlled installed Python S02 baseline noise pilot
status: AUDITED_BASELINE_ONLY
target_release: 0.8.27
---

# Controlled installed Python S02 baseline noise pilot — 2026-10-07

Five separate serial blocks ran the exact 0.8.26 installed wheel. Each block
retained one warm-up and three measured real-database whole-product sequences.
The [independent audit](audit.json) recomputed all **15 valid measured**
durations, artifact and runner hashes, materialized state, GNU Time resources,
start/end host snapshots and nearest-rank block medians. Its surviving-source
and governor-drift negative controls were rejected. The
[pilot manifest](pilot-manifest.json), per-block manifests, commands, raw
attempts, environment snapshots, resources and stdout/stderr are retained;
[SHA256SUMS](SHA256SUMS) seals the receipt.

The measured boundary is installed SDK `Engine.open` through materialized
post-erasure and reopened state, ending after reopened `close`. Independent
SQLite canonical-row checks and semantic assertions follow the product timer.
Every attempt starts a fresh process and database; default-model startup is
included in both open calls. The wheel is bound to baseline source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa` and SHA-256
`7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282`.
The source checkout was clean at each block start.

| Block | Three product durations (ms) | Median (ms) | Host paging |
| --- | --- | ---: | --- |
| 1 | 5402.418, 5368.636, 5359.243 | 5368.636 | None |
| 2 | 5346.738, 5431.125, 5407.093 | 5407.093 | +5 pages; measured children zero swaps |
| 3 | 5417.748, 5319.930, 5401.977 | 5401.977 | None |
| 4 | 5423.790, 5392.054, 5328.147 | 5392.054 | None |
| 5 | 5395.525, 5342.825, 5384.530 | 5384.530 | None |

The block-median spread is **38.457 ms (0.71%)**. Removing block 2 only for
sensitivity analysis gives **33.341 ms** across four warning-free blocks;
block 2 remains in the receipt and the primary spread. The median of all 15
durations is 5392.054 ms, and the median reopened-model stage is 4409.055 ms
(about 82% of the sequence). Measured child peak RSS ranged from 447,380 to
449,412 KiB. All block snapshots retained the performance governor, matching
host/kernel/CPU/storage identity, no named competing jobs and more than 1 GiB
free. One host swap warning is not attributed to the measured children.

To rerun the independent audit from the Slice 135 checkout root:

```sh
python3 dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-s02-controlled-noise-pilot/audit.py \
  dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-wheel-baseline/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl \
  dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-s02-controlled-noise-pilot/block-runner.snapshot.py \
  dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-s02-controlled-noise-pilot/block-01/bundle/slice135_python_s02_timing.py \
  dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-s02-controlled-noise-pilot/block-01/bundle/slice135_python_s01.py \
  dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-s02-controlled-noise-pilot/block-01/bundle/slice135_python_s02.py
```

This is a baseline noise result, not a 0.8.26/0.8.27 comparison. Three
samples per block cannot support p95 or p99, and fresh-process cold/model
startup must not be described as persistent-session warm latency. No candidate
or contention cell ran. The bounded spread supports the frozen
[Python S02 subset](../../s02-python-comparison-protocol.json) with five
alternating pairs and 20 valid measured sequences per block: 100 per version
for nearest-rank p50/p95; p99 remains unsupported.
The broader Phase 1 protocol, TypeScript/Rust boundaries and contention cell
remain separate. Focused block-runner tests passed; the full repository gate
remains open.
