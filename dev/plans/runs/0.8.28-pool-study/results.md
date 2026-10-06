---
title: FathomDB 0.8.28 Tegra CUDA memory-pool study — results, Phases 0 and 1
status: PARTIAL (Phases 0-1 only; Phases 2-5 not started)
target_release: 0.8.28
observed_on: 2026-10-06
---

# Tegra CUDA memory-pool study: results of Phases 0 and 1

This records Phases 0 and 1 of
`dev/plans/0.8.28/prework/tegra-cuda-memory-pool-experiment-protocol.md`
(the protocol) on one Jetson AGX Orin 64 GB. It does not rule. Phases 2–5
(correctness, robustness, soaks, performance, analysis and upstream package)
were not started. Everything not measured here is marked UNMEASURED.
Statements marked *inferred* are readings of the data, not measurements.

Raw logs stay on the host under `<scratch>/pool-study/logs/<phase>/<series>/`
(protocol § 9.2). This directory holds the harness, matrices, summaries
(`summaries/*.txt`, produced by `harness/analyze.py`), samples and this file.

## 1. Environment and artifacts (table 1)

| Item | Value |
| --- | --- |
| Board | Jetson AGX Orin Developer Kit 64 GB |
| L4T / kernel | R36.5.2 / 5.15.199-tegra |
| Driver | 540.5.0, `cuDriverGetVersion` 12060; CUDA toolkit 12.6.68 |
| `cuMemGetInfo` total | 65 879 896 064 B |
| Power mode | MAXN (`nvpmodel -q`); unchanged in every series header. `jetson_clocks --show` needs root and was not recorded |
| Node | 24.15.0, 25.9.0, 26.10.0 (nvm) |
| Builds | Node CUDA addon, `embed-cuda,rerank-cuda` (production) and `embed-cuda,rerank-cuda,tegra-pool-experiment` (experiment), release profile, from this branch's tree at `642347d3a` (the binaries were built from the same source before `cargo fmt` reformatted two files) |
| Build time | experiment 3 min 30 s; production 59 s after it (Candle CUDA kernels already cached in the worktree target) |
| Artifact hashes | `artifacts.sha256` (experiment `.node` `cee63050…`, production `.node` `99ace6e2…`) |
| Form | installed form for every Node series (`harness/stage-installed.sh`: npm-packed main and platform packages installed offline, lifecycle scripts off) |
| Host record | `env.txt`; per series `series-header.txt`, per run `run-*.host.json` (raw, on host) |

## 2. Phase 0

### 2.1 Revisit check (protocol § 10 item 5)

The Slice 110 failure still reproduces on this L4T and driver, so the study
continues.

| Probe | Result |
| --- | --- |
| `minimal_repro` (three 4 KiB pages at 38/68/98 GiB) | `cuMemAllocAsync` `CUDA_ERROR_OUT_OF_MEMORY` 20 / 20 |
| `minimal_repro control` | `CUDA_SUCCESS` 20 / 20 |
| `pool_teardown primary` free-gap, `explicit=keep` | pool before and after teardown, async after, explicit after: success 10 / 10 |
| same, `fill-before-pool` control | default pool and async `CUDA_ERROR_OUT_OF_MEMORY` 10 / 10 |

### 2.2 Production equivalence (table 2)

Ten `full` runs of each build, Node 25, unobstructed, witness on,
interleaved in randomised blocks (seed `20261006`, block orders in
`summaries/interleave-block-orders.txt`), after one throwaway warm run.

| Build | Passed | Embed hash | Rerank score sets | Path (inferred from latency) | `allocMode` read | Steady embed median ms by path (n) |
| --- | --- | --- | --- | --- | --- | --- |
| production | 10 / 10 | `d9dafb8c410005f3` | 1 | stream 2, sync 8 | not available | stream 12.9 (2); sync 25.2 (8) |
| experiment, `S` | 10 / 10 | `d9dafb8c410005f3` | 1 | stream 5, sync 5 | default 5, sync 5 | stream 12.0 (5); sync 24.9 (5) |

- Same steps passed (all), identical embedding hash and rerank scores (the
  hash Slice 110 recorded).
- Steady-embed median ratio experiment / production: stream path 0.928
  (bootstrap 95 % CI 0.672–1.053, n = 5 / 2), synchronous path 0.988
  (0.927–1.131, n = 5 / 8). Both point estimates are within 10 %; the stream
  path rests on two production processes.
- The read `allocMode` agrees with the latency inference in 10 / 10
  experiment processes.
- Open time medians: production 1725 ms, experiment 1718 ms.
- **Verdict: PASS.** The experiment build with `S` reproduces the product.
  It is also structurally identical: `S` makes no driver call in the policy,
  and the vendored cudarc probes an installed pool only after one was
  installed through `CudaMemPool::install`, so `S` makes exactly the 0.8.27
  driver calls (§ 6, deviation 1).
- The default pool was available in 7 of 20 unobstructed processes here
  (2 / 10 and 5 / 10), in line with Slice 110's mixtures.

### 2.3 V8 `--random-seed` layout check

*Hypothesis tested:* Node's `--random-seed` seeds V8's mmap address hints, so
equal seeds give equal layouts. 20 import-only runs, heap-400k, seeds 1–10
twice, maps after the heap grew. **Refuted:** 1 of 10 pairs had identical
window mapping starts (the other pairs shared 69–98 of 80–102 starts;
`summaries/phase0-seeds.txt`). R5 therefore ran **unpaired**, with no seed
flag, and later phases must also run unpaired.

### 2.4 Warm cache

Every Node series began with one throwaway `full` (or same-mode) run
(`run-000`, excluded). Each run also waited for 3 s of GPU idle, so every
process starts after the GPU has idled; that applies to every variant alike.
*Inferred:* that is why import of the installed addon here (S median 140 ms)
is much slower than Slice 110's 31.3 ms import-with-`cuInit` arm. Slice 110
recorded about +110 ms on the first `cuInit` after idle, and its arms did not
force idle. The page-cache `Cached` delta is recorded per run in the host
records but was not analysed.

## 3. Phase 1

### 3.1 C3 capacity (table 3)

`harness/pool_capacity.c`, 560 probes: 7 `maxSize` values (192 MiB to
8 GiB) × chunk {1, 16, 64, 256} MiB × threshold {0, max} × layout
{control, 3pages} × 5 runs. Full table: `summaries/phase1-c3-capacity.txt`.

| `maxSize` MiB | Capacity, chunk 1 MiB (every layout and threshold, 20 / 20 runs) | Single largest allocation | ceil32(`maxSize`/3) |
| --- | --- | --- | --- |
| 192 | 64 | 64 | 64 |
| 384 | 128 | 128 | 128 |
| 1024 | 352 | 352 | 352 |
| 2048 | 704 | 704 | 704 |
| 3072 | 1024 | 1024 | 1024 |
| 4096 | 1376 | 1376 | 1376 |
| 8192 | 2752 | 2752 | 2752 |

- **Capacity = ceil32(`maxSize`/3)** (`maxSize`/3 rounded up to 32 MiB),
  exactly, in all 560 runs (all 5 runs of every cell identical). Residual 0
  at every size, so the C3 model criterion (within 5 %) holds. A straight-line
  fit gives `0.3354·maxSize + 3.4 MiB` with residuals up to 5.6 % at 192 MiB,
  only because of the 32 MiB rounding.
- **The granularity hypothesis is refuted.** Capacity does not depend on the
  chunk size: larger chunks only round the total down to a whole number of
  chunks (for example 1024 MiB with 256 MiB chunks: 256 of 352). The single
  largest allocation equals the capacity. The "about a third" reading of the
  two Slice 110 product failures is confirmed and made exact.
- This matches the research-agent probe (scratchpad `pooltest/t3.c`, `t7.c`;
  independent, not part of this protocol): 384→128, 768→256, 2048→704,
  4096→1376, 8192→2752 MiB with private-pool allocation.
- Layout (control or 3pages) and threshold (0 or max) make no difference to
  capacity.
- Exhaustion behaviour: the first refused request returned
  `CUDA_ERROR_OUT_OF_MEMORY` in 560 / 560 runs and was never sticky
  (`cuCtxSynchronize` succeeded after it in 560 / 560). Synchronous
  `cuMemAlloc(64 MiB)` still succeeded at the cap in 560 / 560. A 4-byte async
  request at the cap succeeded where a chunk's rounding left room (160) and
  was refused otherwise (400). After freeing everything, the same amount was
  allocated again in 560 / 560 (recovery). No safety abort fired.
- 16 GiB was not exhausted (owner ruling 5). By the rule it would hand out
  5472 MiB; *predicted, not measured*.

**Host memory: what bounds a pool's real use (owner's "circuit breaker"
question).** Medians, chunk 1 MiB, per layout × size × threshold, are in
`summaries/phase1-c3-capacity.txt`. Pool reserved and process VmRSS:

| Point | threshold 0 | threshold max |
| --- | --- | --- |
| At the cap | reserved = capacity; VmRSS up by reserved + 9–12 MiB | same |
| After freeing everything and synchronizing | reserved 0; VmRSS back to baseline (+9–12 MiB) | reserved stays at the cap; VmRSS stays up by the same amount |
| After `cuMemPoolTrimTo(0)` | reserved 0; VmRSS at baseline | reserved 0; VmRSS back to baseline |

- Real memory a pool can hold is bounded by **ceil32(`maxSize`/3)**, not by
  `maxSize`: at 8 GiB it is 2752 MiB, at 3 GiB 1024 MiB. Measured as VmRSS,
  which tracked the pool's `RESERVED_MEM_CURRENT` to within about 10 MiB in
  every cell.
- The release threshold decides whether freed memory goes back at a
  synchronization point. With 0 it did, every time. With `max` the pool kept
  its high-water reservation until trimmed, and `cuMemPoolTrimTo(0)` returned
  it all.
- `MemAvailable` is a poor observable for this. At the cap it fell by only
  9–23 MiB in every control cell with threshold 0, while VmRSS rose by up to
  2.7 GiB. In the 3pages cells of 2 GiB and more, and in control cells with
  threshold `max` of 3 GiB and more, it fell by 0.5–2.4 GiB (55–86 % of the
  reserved memory). *Inferred:* the nvmap page pool absorbs device allocations before
  they are charged to `MemAvailable` (platform reference § 7.2, § 7.4). The
  research-agent probe saw the same. VmRSS and the pool counters are the
  primary signals.
- Consequence for the protocol's safety floor: a `MemAvailable` floor can
  under-read real use. `pool_capacity.c` therefore also stops if its own
  VmRSS exceeds `maxSize` + 2 GiB, and `maxSize` ≤ 8 GiB bounds a probe at
  2.7 GiB of real memory.

### 3.2 Contiguous address-space need (table 4)

`harness/pool_gap.c`: after the context exists, 2 MiB `cuMemAddressReserve`
fences every 128 MiB through every driver reservation and 4 KiB blockers
every 128 MiB through every unmapped hole of [8 GiB, 128 GiB) leave exactly
one free extent of G bytes. It is either inside the driver's reservation
(in-unit) or an unmapped hole between reservations (between). `maxSize` is
swept in 1 GiB steps, 3 runs per point, until the first failure plus two
confirmations. Summary: `summaries/phase1-gap-sweep.txt`.

| Gap G | Largest passing `maxSize` (between / in-unit) | First failing (between / in-unit) | New mapping at the largest pass, between (in-unit) |
| --- | --- | --- | --- |
| 0 (all fenced) | — / — | 1 GiB / 1 GiB | — |
| 1 GiB | 3 / 3 GiB | 4 / 4 GiB | 1033 MiB (9 MiB) |
| 2 GiB | 6 / 6 GiB | 7 / 7 GiB | 2057 MiB (9 MiB) |
| 4 GiB | 12 / 12 GiB | 13 / 13 GiB | 4105 MiB (9 MiB) |
| 8 GiB | 24 / 24 GiB | 25 / 25 GiB | 8201 MiB (9 MiB) |
| 12 GiB | 36 / 36 GiB | 37 / 37 GiB | 12297 MiB (9 MiB) |
| 16 GiB | 48 / 48 GiB (sweep end) | none ≤ 48 GiB | 16393 MiB (9 MiB) |

- **need(`maxSize`) = `maxSize`/3 of contiguous free address space**,
  measured at six gaps in both placements, every point 3 / 3 (a 9 MiB
  baseline mapping appears in every run, pool or not). This replaces the
  Slice 110 inference from one layout.
- **Correction to Slice 110:** an explicit pool is *not* limited to existing
  driver reservations. Between reservations it creates a new mapping of
  exactly `maxSize`/3 (for example 1024 MiB for 3 GiB), the same lazy
  mechanism as the default pool's 20 960 MiB (`cuMemGetInfo` total / 3). The
  default pool behaves as an explicit pool whose `maxSize` is the device
  memory (*inferred* from the equal ratio). Explicit pool creation is
  therefore exposed to address-space fragmentation like the default pool,
  only at a third of the explicit `maxSize`.
- **The "48 GiB control ceiling" is not reproduced.** Unfenced, pools were
  created up to the 64 GiB sweep end, also with 4 GiB of device memory held
  first. In the Slice 110 control (`pool_va_repro`, default pool created
  first) 48, 50, 52 and 56 GiB all succeeded (3 / 3 each). *Inferred:* 48 GiB
  was the end of the Slice 110 sweep, not a driver cap. The Slice 110 failure
  at 46 GiB in the n = 3 layout fits the rule: 46/3 = 15.33 GiB is more than
  that layout's 14.66 GiB largest gap.
- Safe `maxSize` for a gap G: at most 3·G.

### 3.3 R5 lazy creation after heap growth (table 5)

**Incomplete: stopped by a protocol stop condition after 35 of 1800
processes.** Matrix: `matrices/r5-lazy-creation.tsv` (A-first-use and B ×
Node 24/25/26 × heap 0/100k/400k/1M/4M × 3 and 16 GiB × threshold 0 × 30;
installed experiment build, `full` mode, witness on, unpaired per § 2.3).

| Cell | n | Pass (Wilson 95 %) | Pool created and probed | `allocMode` explicit | Smallest pre-creation largest unmapped hole in [8, 128) GiB | New mapping at creation |
| --- | --- | --- | --- | --- | --- | --- |
| A-first-use, Node 24, heap 0, 3 GiB | 30 | 30 / 30 [88.6, 100] % | 30 / 30 | 30 / 30 | 6.20 GiB | 6 MiB (median) |
| A-first-use, Node 24, heap 0, 16 GiB | 5 (stopped) | 5 / 5 [56.6, 100] % | 5 / 5 | 5 / 5 | 7.90 GiB | 6 MiB (median) |
| the other 58 cells | — | UNMEASURED | — | — | — | — |

- **The stop.** After run 005 of the 16 GiB cell, `SwapFree` was 256 KiB
  below `SwapTotal` (it was equal before that run;
  `run-005.host.json` / `host-after.json`). `MemAvailable` was 51 GiB,
  `CmaFree` 5 MiB. The 256 KiB sit in `/dev/zram0`. No process shows
  `VmSwap`, so they are unattributed pages (*inferred*: shmem or tmpfs
  reclaimed by kswapd). That run took 3.4 s against about 2.1 s for its
  neighbours. Protocol § 1.3 lists a `SwapFree` move as a stop condition
  ("stop the study, write up what was measured and consult the owner"), and
  § 1.1 refuses every later run until `SwapFree` equals `SwapTotal` again,
  which needs those pages to come back or root to reset the swap. R5 is
  therefore stopped, not finished.
- *Inferred:* the swap-out may be zone-local reclaim triggered by CUDA's
  low-zone (CMA/nvmap) allocations rather than by overall memory pressure.
  That is relevant to the host-memory question, but one event does not show
  it.
- What the 35 processes show: at heap 0 behind early `cuInit`, A-first-use
  created its pool in every process, at 3 and 16 GiB, and every process
  allocated on the explicit pool. The pool was carved inside the driver's
  `cuInit` reservation (6 MiB of new mappings, against 1–5.3 GiB for a new
  range), consistent with § 3.2.
- The predictive rule this phase was to confirm is the § 3.2 measurement:
  creation needs `maxSize`/3 contiguous, either inside the driver reservation
  (not visible in `/proc/self/maps`) or as an unmapped hole. From maps alone
  the rule is one-sided: a hole of at least ceil32(`maxSize`/3) suffices, and
  its absence does not imply failure. 0 of 35 processes contradicted it; the
  heap cells that would test it were not reached.
- B was not run (it comes after A-first-use in the matrix).

### 3.4 P1 import cost of the A-load variants (table 6)

Installed experiment build, Node 25, `CONSUMER_MODE=import`, 3 GiB,
threshold 0, 20 processes per cell in randomised interleaved blocks (seed
`20261007`), after one warm run. The production build ran as an extra cell.
Differences are medians against experiment `S` with bootstrap 95 % CIs.

| Variant | n | Import ms median (IQR) | RSS delta MiB median (IQR) | Import vs S | RSS vs S | Gate (≤ S + 5 ms and ≤ S + 32 MiB) |
| --- | --- | --- | --- | --- | --- | --- |
| S | 20 | 139.6 (138.7–142.4) | 10.9 (10.5–11.4) | — | — | — |
| production S | 20 | 141.4 (138.7–143.0) | 10.5 (10.3–12.5) | +1.7 [−1.2, 3.3] | −0.4 [−0.8, 0.8] | PASS (control) |
| A-load-hold | 20 | 231.6 (228.8–233.0) | 119.9 (117.8–120.2) | +92.0 [88.7, 93.5] | +109.0 [106.9, 109.4] | **FAIL** |
| A-load-release | 20 | 265.7 (249.6–268.9) | 29.8 (28.6–30.6) | +126.0 [110.5, 128.4] | +18.9 [17.6, 19.8] | **FAIL** (import time) |

- Every pool was created, installed and probed at registration (20 / 20 per
  A-load variant). A-load-release released its context successfully in
  20 / 20, and in a `full` smoke run its pool survived that release and was
  decided `explicit`.
- Releasing the context removes most of the memory cost (+19 MiB instead of
  +109 MiB). It costs more time than holding, because the primary context is
  created and then destroyed inside the import. *Inferred:* the next open
  creates it again.
- The import-cost gate fails for both A-load forms by a wide margin. Neither
  can meet it as built, so they are **ruled out as shipping candidates by
  P1**, unless Phase 2 finds a cheaper load-time form. Under the plan's rule,
  A-load-release was the fallback if B and A-first-use failed only R5.

### 3.5 Workload high-water mark (input to the chosen size)

Five `perf` runs (A-first-use, 3 GiB, Node 25, batch sizes 1/8/32/128 and
rerank): pool `used_high` 272.0 MiB, `reserved_high` 288.0 MiB (maximum
over runs). Smoke and `full` runs: used 143.7 MiB, reserved 160 MiB, as in
Slice 110.

## 4. What Phase 1 rules out or keeps

- **A-load-hold: ruled out** as a shipping candidate by P1 (+92 ms import,
  +109 MiB RSS against S; gate 5 ms / 32 MiB).
- **A-load-release: ruled out** as built, by P1 (+126 ms import; its RSS,
  +19 MiB, would pass). It stays only as a comparison arm if a cheaper
  load-time form is found.
- **A-first-use and B: kept**, but not cleared. R5, the key rule-out for
  lazy creation, was stopped after 35 processes (§ 3.3). C3 and the gap sweep
  give the model R5 was meant to test: capacity and contiguous need are both
  `maxSize`/3, exactly.
- **Capacity model (C3): established.** Capacity = ceil32(`maxSize`/3),
  exhaustion is a typed, non-sticky `CUDA_ERROR_OUT_OF_MEMORY` with full
  recovery, and a pool's real memory is bounded by ceil32(`maxSize`/3)
  (threshold 0 returns it at synchronization; `max` keeps it until trimmed).
- **Chosen size (provisional).** The workload high-water mark is 272 MiB
  used (288 MiB reserved). Twice that is 544 MiB, which the measured
  capacities first reach at 2 GiB (704 MiB). 2 GiB needs a 704 MiB
  contiguous extent, 3 GiB needs 1 GiB; the R5 heap cells that would give the
  smallest gap were not reached. Provisional choice: **3 GiB** (capacity
  1024 MiB, 3.8× the high-water mark, and the size R5 tests), with 2 GiB as
  the smallest admissible. Final after R5.
- **No 48 GiB driver ceiling** (§ 3.2); the Slice 110 statement is withdrawn.

## 5. Decision table (protocol § 8.1 item 12), Phases 0–1 rows only

| Clause or criterion | Depends on | Status | Evidence |
| --- | --- | --- | --- |
| Experiment build reproduces the product (precondition) | Phase 0 equivalence | PASS (n = 10 + 10; stream-path ratio rests on 2 production processes) | table 2 |
| Failure still reproduces on this L4T/driver (study continues) | revisit check | PASS (20 / 20 vs 0 / 20) | § 2.1 |
| Paired layouts via `--random-seed` | Phase 0 seeds | FAIL (1 / 10 pairs); later phases run unpaired | § 2.3 |
| C3 capacity model within 5 % at every size | C3 | PASS (0 % with ceil32(`maxSize`/3), 560 / 560) | table 3 |
| C3 exhaustion policy (typed refusal, no silent CPU move, every pointer freed by its API) | C3 + Phase 2 | UNMEASURED in the product; C probe shows typed non-sticky OOM and recovery 560 / 560 | § 3.1 |
| Contiguous need is predictable | gap sweep | PASS: need = `maxSize`/3 at 6 gaps × 2 placements, 3 / 3 per point | table 4 |
| R5: lazy creation succeeds at the chosen size in every heap cell (B, A-first-use) | R5 | UNMEASURED (2 of 60 cells, both heap 0, A-first-use only: 35 / 35 created) | § 3.3 |
| A-load-release passes the import-cost gate | P1 | FAIL (+126.0 ms [110.5, 128.4]) | table 6 |
| A-load-hold passes the import-cost gate | P1 | FAIL (+92.0 ms, +109.0 MiB) | table 6 |
| Which arm can be the aarch64 default (owner ruling 3) | all phases | UNMEASURED; after Phase 1 only A-first-use and B remain candidates | § 4 |
| C1, C2, C4–C8; R1–R4, R6–R8; P2–P7 | Phases 2–4 | UNMEASURED | — |

## 6. Deviations from the protocol, and why

1. **Decision order (§ 2.2).** The vendored cudarc checks an installed
   explicit pool *before* querying the default pool, and only if a pool was
   installed through `CudaMemPool::install` in this process. The protocol's
   order (default first) would label an A-variant process `default` while
   its allocations come from the explicit pool, and the query would lazily
   map the default pool's 20 GiB range in a variant that never uses it. For
   B (installs only after the default query failed) and S (never installs)
   the order changes nothing. With no installed pool there is no extra
   driver call, which keeps production identical.
2. **An extra site label, `embedder-witness`**, for the GPU-witness device
   (`attest_retained_cuda_device`). The protocol counts it among the three
   `candle_bge.rs` sites but its label list has no name for it.
3. **Installed form for every Node series**, not only C8 and the final
   confirmation. File copies (`cp`) were not permitted in this session, so
   the staging script packs with `npm pack` and has napi write each `.node`
   into its own platform-package directory. It does not inject
   `optionalDependencies`; Node resolves the platform package from the
   consumer's `node_modules` all the same.
4. **GPU idle** is read from the sysfs counter `tegrastats` reports as
   `GR3D_FREQ` (`/sys/devices/platform/bus@0/17000000.gpu/load`), 3 samples
   1 s apart.
5. **Host-quiet check extended:** it also refuses while any other
   same-user process holds a GPU device node open. That catches CUDA work
   started without the lock; no process holds one at idle on this host.
   `pool_capacity.c` also stops at VmRSS > `maxSize` + 2 GiB (§ 3.1).
6. **Contamination and reruns.** A separate research agent ran CUDA pool
   probes on this Orin without the GPU lock for roughly 20–30 minutes up to
   about 00:53 UTC on 2026-10-06. The first revisit check (00:31–00:33 UTC)
   and the first gap sweep (from 00:35 UTC) overlapped that window. Both were
   stopped and rerun after it; the tables use only the reruns. The
   superseded results agree with them (`summaries/*-CONTAMINATED-superseded.txt`).
   Every later series ran with the GPU-fd check of deviation 5.
7. **`pool_gap.c` method (§ 4.6).** It uses the unobstructed layout with
   fine fences and blockers (128 MiB spacing) instead of forcing quarters
   with five pre-`cuInit` blockers, so it can place the gap either inside a
   reservation or between reservations and measure both. It also adds `G = 0`
   negative controls, an unfenced ceiling sweep (48–64 GiB, with and without
   a 4 GiB device hold) and the Slice 110 `pool_va_repro` control at
   48–56 GiB.
8. **`pool_capacity.c`** runs the single-allocation bisection in every cell
   (the protocol makes it optional) and samples `MemAvailable` and VmRSS at
   four points (owner's host-memory question).
9. **R5 consumer mode** is `full` (open, embed ×11, rerank ×2, close), so a
   created pool is also shown to serve the workload. `/proc/self/maps` is
   captured immediately before and after pool creation by the policy
   (`FATHOMDB_POOL_MAPS_DIR`). The plan's `strace` subset was not run.
10. **P1** adds a production-S cell, a free control of the interleaving.
11. **Python wheel not built in Phase 0.** No Phase 0–1 item uses it, and
    owner ruling 1 adds an import-time hook to it in Phase 2, which needs a
    new wheel anyway. The contract-script edit and `wheel-features.diff` are
    therefore deferred.
12. **Harnesses for Phases 2–5** (`pool_smoke.py`, `pool_teardown.c
    --reset`, `concurrent-runner.sh`, soak scripts) were not built; this run
    covered Phases 0–1 only.
13. **`jetson_clocks --show`** needs root; it is not recorded.

14. **R5 stopped** on the § 1.3 swap stop condition (§ 3.3); 58 of 60 cells
    were not run, and the 16 GiB heap-0 cell has 5 of 30 processes.

## 7. GPU time

Lock-held series time, summed from the series headers: about 1 h 55 min.
That includes 11 min of runs discarded for contamination (deviation 6) and
about 2 min of probe smoke runs. A first revisit attempt also held the lock
for about 10 min while waiting on a false "pytest busy" match in the quiet
check (fixed). Builds (4.5 min) ran outside the lock. The protocol budget for
Phases 0–1 was 9–13 h; most of the difference is the unrun R5 matrix
(about 3 h at the measured 4.7 s per process) and the skipped wheel build.

## 8. Not measured (Phases 0–1 scope)

- 16 GiB exhaustion (owner ruling 5).
- R5 sizes 1 and 8 GiB (the protocol runs them only if the rule is unclear).
- Python anything (Phase 2 on).
- A-load variants under heap growth (R5 covers the lazy variants only).
- Page-cache attribution of open-time clusters (data recorded, not analysed).
- Everything in Phases 2–5.

## 9. What Phases 2+ need from the owner

**Ruled 2026-10-06** (protocol revision 2, "Owner rulings (2026-10-06)"):
swap is monitored and stops only the no-swap timing series (item 1); the
working size is 3 GiB, 2 GiB the minimum, 8 GiB the exhaustion cap (item 2);
the A-load forms are dropped (item 3); the tracking id is resolved (item 5).
The private pool (P-first-use) became the primary design. The questions are
kept below as asked.

1. **The swap stop condition (blocks R5).** 256 KiB of zram swap, owned by
   no process, appeared during an R5 run with 51 GiB available. Should the
   rule become "swap use must not increase during a series" (baseline taken
   at the series start), or must swap be reset (needs root: `swapoff -a &&
   swapon -a`) before R5 resumes? R5 resumes from the first unfinished cell
   (`phase1.sh r5`, finished cells are skipped) once ruled.
2. **Chosen size.** Confirm 3 GiB as the working size (2 GiB is the smallest
   admissible by capacity), or name another. Ruling 5 says "8 GiB for main
   testing": is that the exhaustion cap only, or also the main `maxSize`?
   8 GiB needs a 2.7 GiB contiguous extent and can hold 2.75 GiB of real
   memory.
3. **A-load forms.** Both fail P1. Is a cheaper load-time form worth
   designing in Phase 2 (for example creating the pool without retaining a
   primary context, if the driver allows it), or are the A-load arms dropped?
4. **Host-memory bound ("circuit breaker").** The data say a pool's real
   memory is bounded by ceil32(`maxSize`/3), returned at each
   synchronization with threshold 0 and on `cuMemPoolTrimTo` otherwise.
   Whether the product needs more than `maxSize` plus threshold 0 is for the
   research agent's survey and the owner.
5. Still open from protocol § 11: the tracking-ledger id (question 6).
