---
title: FathomDB 0.8.28 Tegra CUDA memory-pool study — results, Phases 0, 1, 1b and 2
status: PARTIAL (Phases 0, 1, 1b and 2; Phases 3-5 not started)
target_release: 0.8.28
observed_on: 2026-10-06
---

# Tegra CUDA memory-pool study: results of Phases 0, 1, 1b and 2

This records Phases 0 and 1 of
`dev/plans/0.8.28/prework/tegra-cuda-memory-pool-experiment-protocol.md`
(the protocol) on one Jetson AGX Orin 64 GB, and Phase 1b of protocol
revision 3 (§ 10) and Phase 2 of revision 4 (§ 11). It does not rule.
Phases 3–5 (robustness, soaks, performance, analysis and upstream package)
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
- **A-first-use and B: kept**, but not cleared. *Superseded by ruling 7
  (2026-10-06):* they remain only as comparison arms, and P-first-use (not
  measured in Phases 0–1) is the primary candidate. R5, the key rule-out for
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
| Which arm can be the aarch64 default (owner ruling 3) | all phases | UNMEASURED; after Phase 1 the A-load forms are out. By ruling 7 (2026-10-06) P-first-use is the only shipping candidate; A-first-use and B are comparison arms | § 4 |
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

## 10. Phase 1b (protocol revision 3)

Phase 1b built the primary design, P-first-use, test-first and ran the
revision-3 R5 matrix. Commits: `d6460b16f` (revision 3), `dbd08e313` (code),
`70b0410c4` (C5, equivalence, pilot), and the commit that adds this section.

### 10.1 Build and tests

- **cudarc (vendored).** `CudaContext::new_with_mem_pool(ordinal,
  Arc<CudaMemPool>)` allocates with `cuMemAllocFromPoolAsync` from the
  context's own pool (`AllocMode::Private`). It runs no device decision and
  never touches the process-wide table. No `CudaSlice` variant was added.
  Tests were written first: 8 compile errors (red), then 28 / 28 green on
  the Orin. The new tests are a pure test (a pool context is private without
  a device decision) and two device tests: allocations, zero-length
  included, come from the pool, return at a synchronization, and leave the
  current pool unchanged; a full pool returns a typed
  `CUDA_ERROR_OUT_OF_MEMORY` and recovers.
- **Candle.** The pinned fork's candle-core gains
  `CudaDevice::from_context` and `Device::new_cuda_from_context`: 2 files,
  +45 / −16 (`patches/candle-from-context.patch`). On the study branch it is
  a path override in `third_party/`. The fork's own unit test for it
  (`from_context_tests`) was not run: the vendored copy is outside the
  workspace, and building the fork separately would have competed with R5
  for the host. The product path exercises the constructor in every
  P-first-use process (`allocMode=private`, with the pool counters moving).
  `scripts/check-pinned-override-rot.py` fails on this branch by design (4
  findings, all about the Candle path override), because the branch is never
  merged.
- **Policy.** Variants `S | P-first-use | A-first-use | B`; the A-load names
  are rejected. The fail-closed rule is pure functions with a test (red,
  then 6 / 6 green): the first device fixes the process mode, and once a
  private device exists, a failed private build is refused. The load-time
  hook is removed. clippy `-D warnings` and fmt are clean with and without
  the feature.

### 10.2 C5 and the unit halves of CB1–CB3 (`pool_c5.c`, table 7)

60 runs: control and 3pages layouts × 3 GiB (threshold 0 and `max`) and
192 MiB (threshold 0), 10 per cell. Summary: `summaries/phase1b-c5-results.txt`.

| Observation | Result |
| --- | --- |
| `cuMemAllocFromPoolAsync(0)` | `CUDA_SUCCESS` with a null pointer, 60 / 60; freeing it succeeds, 60 / 60 |
| `cuMemPoolCreate` with no current context (M5 hypothesis) | `CUDA_SUCCESS`, 60 / 60 |
| Exhaustion | typed `CUDA_ERROR_OUT_OF_MEMORY` at exactly ceil32(`maxSize`/3) (1024 MiB at 3 GiB, 64 MiB at 192 MiB), 60 / 60 |
| `cuCtxSynchronize` after the error | `CUDA_SUCCESS`, 60 / 60 |
| VmRSS at the cap minus before the pool | `reserved_high` + 8.8 to 10.2 MiB (CB1 unit half: ≤ + 32 MiB) |
| After freeing and synchronizing | threshold 0: reserved 0, VmRSS back to + 9–10 MiB; `max`: reserved stays 1024 MiB until `cuMemPoolTrimTo(0)`, then 0 |
| Recovery | a 64 MiB allocation succeeds after the frees, 60 / 60 |
| Current pool before and after | unchanged, 60 / 60 |
| Another user's `cuMemAllocAsync(16 MiB)` | control: from the current (default) pool, whose used memory rose by 16 MiB while the private pool's did not (30 / 30); 3pages: `CUDA_ERROR_OUT_OF_MEMORY`, as it would be without FathomDB (30 / 30), private pool unaffected |
| `cuDeviceGetMemPool` when the current pool is the default | same `CUresult` as `cuDeviceGetDefaultMemPool` (success in control, `CUDA_ERROR_OUT_OF_MEMORY` in 3pages); first read maps 132–144 KiB, never a 20 GiB range |

Consequences:

- **H2 settled.** No zero-length special case is needed in the P path; the
  upstream shape passes 0 bytes to the driver unchanged.
- **M5.** The context-free create is supported in C (60 / 60). The policy
  still retains the primary context, because its probe needs one.
- **M4.** In 3pages, reading the current pool returns the same
  `CUDA_ERROR_OUT_OF_MEMORY` as the default-pool query. *Inferred:* the read
  runs the driver's lazy default-pool creation, so the environment gate on
  the Node C9 check stays. The maps diff cannot settle this alone: in the
  control layout the default pool fits inside the `cuInit` reservation and
  maps nothing new.

### 10.3 Product smoke (Node 25, 1 process each)

All passed: S `sync`, P-first-use `private`, A-first-use `explicit`, B
`explicit` (default pool OOM). Two findings for Phase 2 (n = 1 each, not
results):

- **C9 criterion (deviation).** In a P-first-use process with
  `FATHOMDB_POOL_COEXIST_CHECK=1`, the current-pool read before the private
  pool was created returned `CUDA_ERROR_OUT_OF_MEMORY`. At exit it returned
  the default pool's handle (equal to `cuDeviceGetDefaultMemPool`'s; the
  private pool was never current). The default pool had become obtainable
  between the two reads. P-first-use did not change it, but the revision-3
  rule "pairs equal before and after" would report a failure. Phase 2
  should read C9 as: the current-pool handle never equals the private pool,
  and equals the default pool whenever both reads succeed. An
  OOM-to-success transition is recorded as the default pool's own lazy
  behaviour. This needs the reviewer's or the owner's agreement.
- **CB2 smoke.** After `engine.close()` and 10 s idle at threshold 0, the
  exit line showed `reserved_cur` 64 MiB and `used_cur` 17.5 MiB, not 0.
  *Inferred:* the module-level CLS-embedder and reranker singletons outlive
  `engine.close()`, and no synchronization follows the last free. If Phase 2
  confirms it, CB2 fails as written. The remedy (trim on close, or a
  synchronize after release) is a design question.
- **CB3/CB4 smoke.** `OVERSIZE_BATCH=128` at 3 GiB: `embedBatchCls` failed
  after 0.6 s with an `EmbedderError` whose message carries
  `DriverError(CUDA_ERROR_OUT_OF_MEMORY)` (`kind` and `code` are null). The
  next `engine.embed` ran on CUDA with the pre-error hash, and a short batch
  succeeded. The pool's `reserved_high` was 1024 MiB, and VmRSS rose by
  946 MiB during the batch. Whether a null `kind` counts as "typed" for CB3
  is open: the error class is typed, the kind is not.

### 10.4 Phase 0 equivalence on the P build (`NO_SWAP=1`)

| Attempt | Outcome |
| --- | --- |
| 1 | stopped after 5 runs: swap use rose from 768 to 1024 KiB (no-swap rule); discarded |
| 2 (10 + 10) | 20 / 20 passed, identical hash `d9dafb8c410005f3`, path agreement 10 / 10, sync ratio 0.990 [0.911, 1.026], stream ratio 1.150 [1.064, 1.524] on n = 3 / 1 |
| 3 (20 + 20) | 40 / 40 passed, same hash, agreement 20 / 20, sync ratio 0.995 [0.972, 1.028], stream ratio 0.947 [0.874, 1.406] on n = 3 / 2 |

Verdict: **PASS** on the sync path (n = 24 / 27 pooled). The stream path is
UNDERPOWERED: the production build took it in 3 of 30 processes. The two
estimates (1.150 and 0.947) bracket 1, and neither interval excludes it. The
S path makes the same driver calls as production; the only change on it is
Candle's constructor refactor.

### 10.5 Heap pilot (H3a; table 5a)

Node 25, installed P build, `import` mode, 10 runs per size, largest
unmapped hole in [8, 128) GiB before open (`summaries/phase1b-heap-pilot.txt`):

| Heap objects | heapUsed MiB | VmRSS MiB | Largest hole GiB, median (min–max) |
| --- | --- | --- | --- |
| 0 | 5 | 55 | 8.24 (7.05–15.02) |
| 100k | 21 | 89 | 6.81 (4.48–8.98) |
| 400k | 73 | 150 | 3.86 (2.50–6.10) |
| 1M | 161 | 253 | 1.98 (1.39–3.29) |
| 4M | 638 | 817 | **0.59** (0.53–0.81) |
| 8M | 1315 | 1490 | 0.43 (0.32–0.61) |

The boundary (the first size with a median below 1 GiB) is **4M**, and the
size before it is 1M. Both were already R5 cells, so the boundary cells add
nothing new. The saved runs went to an **8M** cell (all variants) and to 60
rather than 30 P-first-use runs at 4M and 8M on every Node version
(deviation).

### 10.6 R5 lazy creation after heap growth (revision 3; table 5b)

Installed P build (`.node` sha256 `ac58381e…`), `full` mode, witness on, 3 GiB,
threshold 0, unpaired, run 13:30–15:31 UTC. On Node 25 the three variants
ran in interleaved randomised blocks: 3 chunks × 10 blocks, seeds
20261001–3, one process per variant and heap cell per block, with two
P-first-use copies at 4M and 8M. P-first-use on Node 24 and 26 ran per cell.
Warm runs are excluded. Full table: `summaries/phase1b-r5.txt`.

| Variant | Cells | Pass (Wilson 95 %) | Pool created and probed | Stream-ordered among passes | Zero-failure bound |
| --- | --- | --- | --- | --- | --- |
| **P-first-use** | Node 24/25/26 × 0/100k/400k/1M (30 each) and 4M/8M (60 each) | **719 / 720** [99.2, 100] % | **720 / 720** [99.5, 100] % | 719 / 719 `private` | 0.42 % |
| A-first-use | Node 25 × 6 heaps × 30 | 180 / 180 [97.9, 100] % | 180 / 180 | 180 / 180 `explicit` | 1.67 % |
| B | Node 25 × 6 heaps × 30 | 180 / 180 [97.9, 100] % | 139 / 139 (41 had a default pool) | 139 `explicit`, 41 `default` | 1.67 % |

- **Every pool was created, in every cell, including the boundary cells.**
  At 4M and 8M the largest unmapped hole before creation was below the
  1 GiB contiguous need in 455 of the 463 boundary-cell processes that
  created a pool (cell medians 0.37–0.71 GiB, minimum 0.28 GiB). The pool
  was still created in all of them. The one-sided rule (a hole of
  at least ceil32(`maxSize`/3) suffices) had 0 misses. Its converse does
  not hold, for the reason in § 10.7.
- **The one failure** (P-first-use, Node 26, 4M, run 021) is the GPU
  allocation witness, not the allocator. The pool was created and probed,
  the decision was `private`, and the model loaded: `used_high` 127 MiB at
  exit. The open failed with `insufficient_delta`: the system-wide
  `cuMemGetInfo` delta was −126 MB. During that run the host's page cache
  shrank by 1.16 GiB, and `MemAvailable` rose by 498 MiB. *Inferred:*
  concurrent page-cache reclaim inflated "free" memory, which is the known
  limitation of the shared counter (protocol § 6.3). It did not repeat in
  1079 other processes. Under the protocol a failure counts, so the cell
  reads 59 / 60.
- **CB1 (pool part):** `reserved_high` at exit was 160 MiB in every R5
  process of every variant, below the 1024 MiB cap. The VmRSS + VmSwap
  comparison against S belongs to Phase 4.
- Power per cell: a 30-run cell detects a 5 % failure rate 79 % of the
  time (2 %: 45 %); a 60-run cell 95 % (70 %).

### 10.7 Where the pool lands: the early-`cuInit` reservation (table 5c)

Coordinator question: why does P-first-use pass at 4M and 8M when the pilot
put the largest hole below 1 GiB there? The policy's `pre-pool` and
`post-pool` maps snapshots answer it (`analyze.py poolloc`,
`summaries/phase1b-r5-pool-location.txt`; the counts include the warm runs).

- In **1054 of 1054** P-first-use, A-first-use and B processes that created
  a pool, the only mappings the creation and probe added were two 64 KiB
  `/dmabuf:` pages. They sit inside a pre-existing large anonymous
  `PROT_NONE` range in [8, 128) GiB, never in an unmapped hole and never
  outside the window. For example, a P-first-use 8M process split
  `14e2494000-18b573e000 ---p` around `1525387000-15253a7000 /dmabuf:`.
- Those large `PROT_NONE` ranges total **61.32 GiB in every cell**, from
  heap 0 to 8M. That is the driver's `cuInit` reservation (61.36 GiB; Slice
  110), split into units, the largest a median 15–23 GiB. Early `cuInit`
  makes it at module registration, before the consumer grows its heap, so
  V8's later pages fall outside it.
- **So the private pool's address range lies inside the early-`cuInit`
  driver reservation.** *Inferred* (the pool's own virtual range is not
  visible in maps; its first pages are): the 1 GiB contiguous need is met
  from free space inside the reservation, which no heap growth can
  fragment.
- **The pilot's "largest hole" metric does not cover the space the pool
  uses.** It measures unmapped gaps only, and the reservation is mapped
  (`PROT_NONE`), so the metric goes to 0.3 GiB while the reservation keeps
  15–23 GiB units. The metric predicts `cuInit` (which needs an unmapped
  4 GiB hole) and the default pool's new 20 GiB mapping. For a pool created
  after early `cuInit` it is a lower bound that does not bind.
- **Early `cuInit` is therefore a load-bearing part of the P design**, not
  only of the 0.8.27 fix. The A-first-use and B pools land the same way.

**Falsifying cell (late `cuInit`).** P-first-use, Node 25, 4M and 8M,
`FATHOMDB_CUDA_EARLY_INIT=off`, 10 processes each under the same lock and
rules: **0 / 20 passed**. Every process failed at open with
`EmbedDevicePolicyError`: `cuInit returned CUDA_ERROR_OUT_OF_MEMORY` (the
driver could not reserve its range). No pool was attempted, because the
policy runs only after a successful device probe. With `cuInit` deferred
past heap growth there is no reservation and no CUDA at all. The cell
falsifies "P works without early `cuInit`". It cannot show whether a pool
alone would fit after a late `cuInit`, because `cuInit` fails first.
Summaries: `summaries/phase1b-late-cuinit-*.txt`; sample under
`samples/phase1b-late-cuinit-h4M/`.

### 10.8 Decision-table rows added by Phase 1b

| Clause or criterion | Depends on | Status | Evidence |
| --- | --- | --- | --- |
| P build reproduces the product (precondition) | Phase 0 equivalence | PASS on the sync path (24 / 27, ratios 0.990 and 0.995); stream path UNDERPOWERED (n = 6 / 3) | § 10.4 |
| C5 zero-length from a private pool | `pool_c5.c`, cudarc device test | PASS (60 / 60 null and freeable; device test green); product suites in Phase 2 | § 10.2 |
| R5: lazy private-pool creation at 3 GiB in every heap cell, boundary cells included | R5 | PASS for creation (720 / 720); pass rate 719 / 720, the failure attributed to the witness (inferred) | § 10.6 |
| Early `cuInit` required by P | pool location, late-`cuInit` cell | ESTABLISHED: pool inside the reservation 1054 / 1054; late `cuInit` 0 / 20 | § 10.7 |
| CB1 cap | C5 unit half; R5 `reserved_high`; Phase 4 | unit half PASS (+8.8 to +10.2 MiB at the cap); R5 `reserved_high` 160 MiB ≤ 1024 MiB; product VmRSS + VmSwap comparison UNMEASURED | § 10.2, § 10.6 |
| CB2 trim after close | C5 unit half; Phase 2 | unit half PASS; product UNMEASURED; smoke (n = 1) showed 64 MiB still reserved | § 10.2, § 10.3 |
| CB3 typed cap error / CB4 no CPU move | C5, cudarc test; Phase 2 | unit halves PASS; product UNMEASURED; smoke (n = 1) passed CB4 but its error `kind` is null | § 10.2, § 10.3 |
| C9 coexistence | C5, cudarc test; Phase 2 | C-level PASS (current pool unchanged 60 / 60; another user's allocation drew from the default pool); Node series UNMEASURED; criterion needs amending (§ 10.3) | § 10.2, § 10.3 |

### 10.9 Deviations (Phase 1b)

1. The pilot boundary (4M) and the size before it (1M) were already R5
   cells. The saved runs went to an 8M cell for all variants and to 60
   P-first-use runs at 4M and 8M on every Node version: 1080 processes,
   against the 1050 planned.
2. The first equivalence attempt was stopped by a 256 KiB swap rise (the
   revision-3 no-swap rule) and discarded. It was rerun twice (10 + 10, then
   20 + 20), so the stream path, which production takes in only 10 % of
   processes, had more than one reference process.
3. The Candle `from_context` unit test in the vendored copy was not run;
   the product path covers the constructor (§ 10.1).
4. Two Python test-data generators in the packaged candle-core
   (`tests/pth.py`, `tests/npy.py`) are not committed: they fail the
   repository's ruff hook and are not built.
5. `pool_c5.c` reads `cuDeviceGetMemPool` before creating the private pool
   in the same process, so in the control layout the default pool may
   already exist when the "other user" allocates. This mirrors a process in
   which another library ran first.

### 10.10 GPU time (Phase 1b)

Lock-held time: C5 smoke and series about 4 min; vendored cudarc test runs
(compile inside the lock) about 3 min; product smoke about 2 min;
equivalence about 8.5 min (three attempts); heap pilot 3.6 min; R5 2 h
0 min; late-`cuInit` cell 1.5 min. **About 2 h 22 min.** Builds (P addon
twice, 2 min and 1 min) ran outside the lock.

### 10.11 What needed a ruling (ruled 2026-10-06 as rulings 12–17)

Ruled on 2026-10-06 (protocol revision 4, "Owner rulings (2026-10-06,
after Phase 1b)"): items 1, 2, 4, 5 and 6 agreed (6 with a 1 MiB swap
tolerance); item 3 conditional (trim is an experimental arm only). The
items are kept as asked.

1. **Early `cuInit` is load-bearing for P** (§ 10.7). The decision rule
   should state it: P-first-use is a candidate only together with early
   `cuInit`. That covers Node, and Python once Phase 2 adds its import hook.
   A late import, or an application that opts out of early `cuInit`, gets
   no CUDA at all at large heaps, pool or not.
2. **The C9 criterion** should be amended as in § 10.3: compare against
   the private pool and the default pool, not before against after.
3. **CB2 as written may fail** (smoke): with threshold 0, memory stays
   reserved after `engine.close()` while module-level singletons keep
   buffers, and nothing synchronizes after the last free. Phase 2 measures
   it; a trim-on-close or a synchronize in the policy is a design choice.
4. **CB3 "typed":** the cap error reaches JavaScript as `EmbedderError`
   with a null `kind`. Decide whether the class suffices or a kind is
   required.
5. **The witness under page-cache reclaim** (§ 10.6) can fail a healthy
   process. R5-style pass/fail rows may want the witness off, or its
   failures classified separately.
6. The pinned-override-rot gate fails on the study branch because of the
   Candle path override (expected; the branch is never merged). A fork
   commit carrying `patches/candle-from-context.patch` would remove the
   override.

## 11. Phase 2 (protocol revision 4)

Phase 2 ran the correctness rows under owner rulings 12–17, on the commits
listed below and on the same host as Phases 0–1b:

- C1, C2, C5, C7, C8, C9 and C9b;
- CB1–CB4 on the product path;
- the experimental trim arm, the pool-exhaustion error kind, and the Python
  import hook.

All series ran Node 25.9.0 or Python 3.12 with the witness unset (ruling 16):

- **Node addon:** `consumer-p3`, a P build with the trim arm and the error kind.
- **Python wheel:** `venv-p`, the experiment wheel with the import-time hook.

Summaries are in `summaries/phase2.txt` and `summaries/phase2-*.txt`;
samples are under `samples/phase2-*/`.
The analysis is `harness/analyze.py phase2 <scratch>/logs/phase2`.

### 11.1 Code and tests

- `3acc4704a`: rulings 12–17 in the protocol and plan (revision 4).
- `74a6b4d62`: the trim arm, the pool-exhaustion kind, the Python import hook,
  named constants, and `harness/study-config.sh`. Pure Rust tests and the
  TypeScript test went red then green.
- The commit that adds this section also brings:
  - the named trim-tick constants, with a test (red, then green);
  - the private pool's ordinal in the exit event;
  - non-P exit events that carry the coexist fields (C9b);
  - the harness modes `trimcycle`, `trimstress`, `reset` and `coresident`;
  - `pool_reset.c`;
  - the interleave fix (§ 11.10);
  - the Phase 2 matrices.

Gates:

- `cargo clippy -D warnings` is clean for the workspace and for the
  experiment feature set.
- `cargo fmt --check` is clean.
- The policy has 11 pure tests, all passing.
- The experiment feature is in no release feature set:
  `scripts/release/cuda-artifact-contract.sh` is unchanged.

### 11.2 Correctness rows (table 8)

Pass rates are Wilson 95 % intervals. "Hash" is the 16-hex prefix of the
SHA-256 of the f64 embedding. Every passing process in every Phase 2 cell,
Node and Python, gave:

- embed hash `d9dafb8c410005f3`;
- rerank scores `[1e-05, 0.997981]`;
- CLS hash `a0cdfdfecb115b33` (Python and Node `trimcycle`).

| Row | Cells | n | Result | Verdict |
| --- | --- | --- | --- | --- |
| C1 provenance | Node 25, heap 400k, interleaved; S, P-first-use, A-first-use, B | 50 each | 200/200 pass [98.1, 100] %. One `decide` per process in every process. P private 50/50; A explicit 50/50; B explicit 41, default 9; S sync 40, default 10. No mismatch. | PASS |
| C2 lifetime (Node `cycles`, 50 open/close cycles) | P 10, A 3, B 3 | 16 | **0/16**. Every process fails at the 8th open, cycle index 8, with `model deserialize: CUDA_ERROR_OUT_OF_MEMORY`. See § 11.3. | **FAIL as specified**; passes with GC after close |
| C2 with GC after each close (`--expose-gc`, `GC_AFTER_CLOSE=1`) | P 5, A 3, B 3; S 3 (contrast) | 14 | 14/14 pass, 50/50 cycles each (P+A+B 11/11 [74.1, 100] %); VmRSS flat | PASS with GC; see § 11.3 |
| C5 zero-length, product suites | `candle_bge` integration tests (`loader-test-hooks`) + reranker `score_batch_empty`, forced CUDA, `FATHOMDB_POOL_VARIANT=P-first-use` (`decide` = private in both) | suite | 9/9 + 1/1 pass, including the empty string and the empty batch | PASS |
| C6 multi-device | pure unit tests | — | The fail-closed tests pass. No multi-device host exists, so this is UNMEASURED on hardware. | code path only |
| C7 reset, C probe (`pool_reset.c`) | private pool, 16 MiB live, `cuDevicePrimaryCtxReset` | 20 | 20/20 every call `CUDA_SUCCESS` [83.9, 100] %. After the reset: the pool handle still allocates; the pre-reset pointer frees; `used` = 16 MiB and `reserved` = 32 MiB; destroy succeeds. | PASS (pool survives a reset) |
| C7 reset, product (Python `reset` mode: embed, rerank and CLS; `cuDevicePrimaryCtxReset` through `ctypes`; then the three again) | S 20, P 20, A 10, B 10 | 60 | **0/60 for every variant, the shipped S included.** The first call after the reset raises `EmbedderError` (no kind) and the process then crashes at exit, SIGSEGV, exit 139. Crashes: S 13/20, P 19/20, A 10/10, B 10/10. | FAIL, but not caused by P: a co-resident reset already breaks the shipped path (§ 11.4) |
| C8 parity, Python vs Node | Python S, P, A, B (20 each) vs Node C1 (50 each) | 80 + 200 | 80/80 Python pass [95.4, 100] %. Every process in both bindings has the same embed hash, rerank scores and CLS hash. Python allocation modes: P private 20, A explicit 20, S **default 20**, B **default 20**. Python's address space leaves the default pool obtainable; Node S takes the synchronous path in 40 of 50. | PASS |
| C9, Python co-resident user | P 20, S 10 | 30 | The co-resident user's default pool succeeds, the current pool is the default pool, and a 16 MiB `cuMemAllocAsync` succeeds: 30/30. | PASS |
| C9 amended rule (ruling 13), Node install and exit events | every P process with coexist fields (C2 `cycles` 10, C9b 30) | 80 reads | The current pool **never** equals the private pool (0 of 80 reads). Where both reads succeed it equals the default pool every time (44/44). The other 36 reads were `OOM` current-pool reads, recorded and not failed. A and B install their pool as current by design, so the rule does not apply to them. | PASS |
| C9b, default-pool success at exit | P vs S × heaps 0 / 1M / 4M, interleaved | 10 each | P 8, 8, 7 of 10; S 7, 6, 7 of 10. Pooled P 23/30 [59.1, 88.2] % vs S 20/30 [48.8, 80.8] %. | no harm detected (power: § 11.5) |

### 11.3 C2: a closed engine keeps its model until garbage collection

The C2 row fails at the same point in every process. In P, A and B the 8th
`Engine.open` in a process (cycle index 8) fails with
`model deserialize: CUDA_ERROR_OUT_OF_MEMORY`, and the private pool's
`used_high` is 1072255488 B, its 1 GiB capacity. This is not a pool defect.

Diagnosis:

- `Engine::close` (`fathomdb-engine/src/runtime_lifecycle.rs`) stops the
  scheduler, readers and writer. It does not drop `runtime_embedder`.
- The model weights (about 133 MiB for bge-small in f32) are freed only when
  the `Engine` itself drops. In Node that happens when the JS wrapper is
  garbage-collected.
- A loop of `open` / `embed` / `close` with a small JS heap never collects
  it.

Measured (Node 25, `cycles` mode, per-cycle VmRSS):

| Arm | n | Result | VmRSS after each close |
| --- | --- | --- | --- |
| S (the shipped 0.8.27 path), 12 cycles, no GC | 3 | 3/3 pass | grows by about 159 MiB per cycle: 545 → 2291 MiB after 12 cycles |
| P, 50 cycles, `gc()` after each close | 5 | 5/5 pass, 50/50 cycles | flat: 552–709 MiB |
| S, 50 cycles, `gc()` after each close | 3 | 3/3 pass | flat: 545–669 MiB |
| A, B, 50 cycles, `gc()` after each close | 3 + 3 | 6/6 pass | flat: 554–724 MiB |

So:

- **On the shipped path** every un-collected closed engine holds its model
  on the device. That is about 159 MiB of shared DRAM per engine.
- **Under any capped pool** (P, A, B) the same retention becomes a hard
  failure after seven closed but un-collected engines. Capacity is about
  1 GiB at 3 GiB `maxSize`.

The protocol's C2 criterion (50 cycles pass) fails for every pool variant as
the product stands. The fix belongs to the engine, not the pool: drop the
embedder, or release its device memory, in `Engine::close`. That is a
product change outside this study (§ 11.14, decision 1).

### 11.4 C7: a co-resident context reset breaks every variant

The C probe shows that a private pool, and memory allocated from it, survive
`cuDevicePrimaryCtxReset`. The product does not survive it, under any
variant. After a reset:

- Candle's context-level state is gone: its loaded kernels, its stream and
  cuBLAS handles.
- So the first embed fails, and teardown later crashes.

The shipped path (S) fails the same way, 20/20, with 13 of those crashing
at exit. P crashes at exit more often (19/20 against 13/20, Fisher's exact
two-sided p ≈ 0.04), and so do A and B (10/10 each). A likely cause is the
pool's own teardown after the reset: the exit event reads pool attributes,
and the pool is destroyed when the process ends. That is not established.

What it means:

- Before the reset P is as good as S, and after it P is no worse; neither
  survives.
- **C7 as a pass criterion cannot be met by any variant.** It needs a
  ruling: either record it as "a co-resident reset is unsupported", or make
  surviving a reset a product requirement.

### 11.5 C9b and the amended C9 rule

The private pool is never the current pool. In every P install and exit
event that carries the coexist fields, `current_pool` differs from
`private_pool`, 80 of 80. Where both the current-pool and default-pool
reads succeed, they name the same handle, 44 of 44.

The co-resident library's success with the default pool at exit is about
the same with and without P. P succeeded in 23 of 30 processes and S in 20
of 30. The failures are the default pool's own `CUDA_ERROR_OUT_OF_MEMORY`,
from its 20960 MiB contiguous need, and S shows them too: S takes the
synchronous path in 80 % of C1 processes for exactly this reason.

**Power.** At 10 processes per cell, only a large harm would show. Pooled
over heaps, 30 against 30, the two-sided 95 % difference in proportions is
about ±23 points. That rules out "P makes the default pool unavailable"; it
does not rule out a harm of 10–20 points. Python C9: 30 of 30 succeeded,
for P and S alike.

### 11.6 CB1–CB4 (table 9)

| Row | Cells | n | Result | Verdict |
| --- | --- | --- | --- | --- |
| CB1 cap | P at threshold 0 and `max`, heaps 0 and 400k, interleaved with S | 20 + 20 (h0); 20 + 10 (h400k) | `reserved_high` 160 MiB in every process (≤ 1024 MiB). VmRSS + VmSwap at `afterRerank` minus the S synchronous-path median (588 MiB at heap 0, n = 13; 683 MiB at 400k, n = 17) is at most +31 MiB in every cell; the rule allows `reserved_high` + 32 MiB = 192 MiB. | PASS as written; **the observable is weak** (below) |
| CB2 trim after close (threshold 0) | P, 10 s idle after close | 20 (h0) + 20 (h400k) | `reserved_cur` = 64 MiB in **every** process (median = max), `used_cur` 17 MiB, spare 47 MiB | **FAIL as written** (`reserved_cur` = 0 required) |
| CB2 at threshold `max` | same | 20 (h0) + 10 (h400k) | `reserved_cur` 160 MiB (= `reserved_high`), spare 143 MiB | recorded |
| CB2 with the trim arm (`FATHOMDB_POOL_TRIM=idle`, default 5000 ms) vs off | P threshold 0, heap 0, interleaved | 20 + 20 | `reserved_cur` 64 MiB in both arms in every process; the trims released nothing (§ 11.7) | FAIL as written; trim does not help |
| CB3 typed cap error | P Node 20, P Python 20; A 10, B 10 (contrast) | 60 | P Node 20/20: `CudaPoolExhaustedError`, code `FDB_CUDA_POOL_EXHAUSTED`, kind `cuda_pool_exhausted`. P Python 20/20: `EmbedderError`, kind `cuda_pool_exhausted`, ordinal 0, `max_size_bytes` 3221225472. One `exhausted` event per process. A and B: `EmbedderError` with no kind, 19/20 (by design: the kind is for the private pool only). In one B process the oversized batch succeeded on the default pool. | PASS (P) |
| CB4 no CPU move | same | 60 | The next embed ran on CUDA with the same hash in 60/60 [94.0, 100] % | PASS |

**CB1's observable is weak on Tegra.** The pool's pinned memory does not
show up fully in the process's VmRSS:

- P runs within +31 MiB of S while its pool reserves 160 MiB.
- In `trimcycle`, idle trims that return 47+ MiB move VmRSS by 0.0 MiB
  (median).

CB1 therefore passes, but would pass even if the pool took more than it
should. A system-level observable would decide it: `MemAvailable` deltas
in a quiet host, `tegrastats`, or nvmap accounting (nvmap needs root).
§ 11.14, decision 4.

**CB2.** At threshold 0, freed memory goes back only at a synchronization.
After `engine.close()` nothing synchronizes, so 47 MiB of spare memory stays
reserved. The other 17 MiB is held by the module-level reranker and CLS
singletons, which live until process exit. `reserved_cur` = 0 is therefore
not reachable while those singletons exist, with or without trim; the best
reachable value is `reserved_cur` = `used_cur`.

### 11.7 The trim arm (ruling 14; experimental, not the default)

**Safety.** No failure in any trim-arm process:

| Test (ruling 14) | Evidence | Result |
| --- | --- | --- |
| Trim while the embedder and reranker singletons hold slices | `trimcycle`: 20 processes × 5 cycles. Each cycle runs embed, rerank and CLS, idles 1 s (> 200 ms trim idle), then runs all three again. The singletons live throughout. | 20/20 pass; median 84 trim calls per process; one embed hash, one CLS hash and one score set across all 100 cycles |
| Embed concurrent with trim, worker thread and Node workers | `trimstress`: trim idle 0, so a trim at every 1 ms tick; 200 main-thread embeds at concurrency 4 on the libuv pool; 2 `worker_threads` workers, each with its own engine and 100 embeds | 10/10 pass, exit 0; 2000 + 2000 embeds, **0 hash mismatches**; median 23 206 trim calls per process |
| Reopen after trim; repeated open / close / trim cycles | `trimcycle` reopens every cycle | 100/100 reopen and match |
| CB2 with trim (`FATHOMDB_POOL_TRIM=idle`, default 5000 ms) vs off, threshold 0, 10 s idle after close | 20 + 20, interleaved | both arms: `reserved_cur` 64 MiB, `used_cur` 16 MiB in every process; the idle arm made 20–21 trim calls per process |
| No crash, no use-after-free | exit status 0 in all 50 trim-arm processes, hashes identical | No crash. A use-after-free was not instrumented: `compute-sanitizer` was not run. |

**Effect: none measurable at threshold 0.** A diagnosis run sampled the
pool once a second through `trimcycle` with a 3 s idle: 2 processes per
arm, idle 200 ms against off. The sampled `reserved_cur` / `used_cur` were
the same in both arms at every sample: 288 / 270, 416 / 396, then
544 / 523 MiB. The trims never lowered `reserved_cur`.

The reason is that threshold 0 already returns every fully free chunk to
the system at each synchronization. What stays reserved is free space
inside the 32 MiB chunks that still hold live slices: the singletons' and
the open engine's. `cuMemPoolTrimTo` cannot release those chunks.
(`trimstress`, where nothing stays live after close, ends with
`reserved_cur` = 0.)

Consequences:

- **The latency comparison measured no re-grow.** The after-idle embed
  ratio of 0.993 [0.984, 1.001] (rerank 0.979 [0.950, 1.023], CLS 0.998
  [0.992, 1.007]) shows that the arm's thread and trim calls cost nothing
  detectable. It does not price a trim followed by a re-grow, because the
  pool never shrank.
- **The arm keeps calling trim** at every tick while reserved exceeds used,
  even when a trim releases nothing. That is the 20–84 calls per process.
  If the arm were kept, it should stop after a trim that does not lower
  `reserved_cur`.
- **An untested case where trim could matter** is threshold `max`, which
  keeps whole free chunks cached. That combination was not run.

The diagnosis also shows the C2 retention directly: `used_cur` grows by
about 127 MiB per reopen, because each closed engine's model is still
held (§ 11.3).

Note that `trim_us` sums whole microseconds per call and is a lower bound.

### 11.8 Pool-exhaustion error kind (ruling 15)

**Survey (before implementing).** The existing taxonomy:

- `kind` values are lower_snake. They are `cuda_probe_failed`,
  `cuda_incompatible` and `cuda_not_compiled` for CUDA.
- Codes are `FDB_UPPER_SNAKE`.
- The napi envelope is `{code, message, payload}`. TS payload fields are
  camelCase and Python attributes are snake_case.
- `dev/design/errors.md` keeps the 41-row matrix. Each binding's interface
  doc lists its classes under "## Errors"; the TS worked CUDA example is at
  `dev/interfaces/typescript.md` § "Node.js on Jetson".

Before this change, a CUDA out-of-memory from the forward pass reached the
engine as the unit `EngineError::Embedder` and JavaScript as `EmbedderError`
with a null `kind`. That is the smoke finding of § 10.3.

**Design (implemented behind `tegra-pool-experiment` only).**

| Layer | Shape |
| --- | --- |
| Classification | `is_pool_exhaustion(mode, driver_oom)`: true only for `CUDA_ERROR_OUT_OF_MEMORY` in a process whose devices allocate from the private pool. Elsewhere the error is reported as before. `forward_error` finds the driver error through Candle's `Context` / `WithPath` / `WithBacktrace` wrappers and emits an `exhausted` event. |
| `fathomdb-embedder-api` | `EmbedderError::CudaPoolExhausted { ordinal, max_size_bytes, message }` |
| `fathomdb-engine` | `EngineError::CudaPoolExhausted { ordinal, max_size_bytes }`; stable code `CudaPoolExhaustedError`. Display: "CUDA memory pool on device {ordinal} is exhausted (maxSize {max_size_bytes} bytes)". |
| napi | code `FDB_CUDA_POOL_EXHAUSTED`, payload `{kind: "cuda_pool_exhausted", ordinal, maxSizeBytes}` |
| TypeScript | `CudaPoolExhaustedError extends EmbedderError`, so existing `instanceof EmbedderError` handlers still catch it. Adds the `code`, `kind`, `ordinal` and `maxSizeBytes` fields. |
| Python | the existing `EmbedderError`, with the attributes `kind`, `ordinal` and `max_size_bytes`. A dedicated subclass is left to adoption. |
| CLI | stable code `CudaPoolExhaustedError` |

It is a subclass, not a sibling. The device is not lost and the engine
stays usable, so CB4 holds: the next embed runs on CUDA with the same hash.
No CPU fallback is taken, which matches the existing no-CPU-fallback rule
for forced CUDA.

**Draft interface-doc wording (study evidence only; not in
`dev/interfaces/`).** Adoption needs this text in
`dev/interfaces/{typescript,python,rust}.md` "## Errors", a row in
`dev/design/errors.md`, and an ADR.

> `CudaPoolExhaustedError` (code `FDB_CUDA_POOL_EXHAUSTED`, kind
> `cuda_pool_exhausted`) extends `EmbedderError`. It is raised when an embed
> or `embedBatchCls` call needs more device memory than FathomDB's
> private CUDA memory pool may hold. [Adoption: add "rerank" here once
> the reranker's forward is classified; see the gap below.] The pool's cap is `maxSizeBytes`
> (`FATHOMDB_POOL_MAXSIZE`, default 3 GiB) on device `ordinal`. The call
> fails and nothing is moved to the CPU. The engine stays open and later
> calls of the usual size succeed on the same device. Remedies: smaller
> batches, or a larger `FATHOMDB_POOL_MAXSIZE`. It is raised only on builds
> and devices that use the private pool. Elsewhere a CUDA out-of-memory stays
> an `EmbedderError` with no kind. Python raises `EmbedderError` with
> `kind == "cuda_pool_exhausted"`, `ordinal` and `max_size_bytes`. Rust
> returns `EngineError::CudaPoolExhausted { ordinal, max_size_bytes }`. As
> with other errors, match on `code` and `kind`, not on the message.

Open points for adoption:

- whether Python gets a subclass;
- **Gap: the reranker is not classified.** Only the embedder's three
  forward sites (`candle_bge.rs`: embed, batch forward, CLS) go through
  `forward_error`. The cross-encoder's forward (`candle_reranker.rs`) uses
  the same private pool, but its out-of-memory still surfaces as before,
  with no kind. Adoption must route it through the same classification, or
  the draft wording must narrow "rerank" out. CB3 was measured on
  `embedBatchCls` only.

### 11.9 Numeric-literal inventory ("no magic numbers")

Scope:

- `cuda_pool_policy.rs` (PP);
- the vendored cudarc patch (CORE = `safe/core.rs`, MP = `safe/mem_pool.rs`);
- the napi and embedder `cuInit` hooks (EI = `fathomdb-napi/src/cuda_early_init.rs`,
  DI = `fathomdb-embedder/src/cuda_driver_init.rs`);
- the tests;
- the harness (H/).

Classes:

- **a**: a measured platform fact;
- **b**: a product default;
- **c**: a test or harness tolerance;
- **d**: incidental (unit shifts, ABI enum values, sentinels).

**(a) Measured platform facts.** None drives product logic. The policy and
the cudarc fallback decide from behaviour: `MEMORY_POOLS_SUPPORTED`, then a
default-pool query, then falling back only on `OUT_OF_MEMORY` or
`NOT_SUPPORTED`.

| Fact | Where | Used for | Off this device |
| --- | --- | --- | --- |
| capacity = ceil32(`maxSize`/3); contiguous need `maxSize`/3 | CORE:64-65, PP doc of `DEFAULT_MAX_SIZE`, H/analyze.py | comments, analysis | never measured elsewhere |
| [8, 128) GiB window, 4 GiB `cuInit` hole, 61.36 GiB reservation | EI:4-6, DI:3-6, H/pool_gap.c, H/analyze.py | docs, harness hole maths | probably scales with RAM; harness analysis wrong elsewhere |
| fixed blocker addresses (0x980000000, ...) | H/pool_c5.c, H/pool_capacity.c, H/revisit-check.sh | probes | Orin layout only |
| GPU-load sysfs path `17000000.gpu/load` | H/lib-host.sh | quiet check | missing path: the host is never quiet and every run is refused after `STUDY_QUIET_WAIT_S` |
| 18.0 ms stream/sync latency split | H/analyze.py | allocator-path classification | device and model specific |
| 272 MiB workload high-water | PP doc | rationale for 3 GiB | model specific |
| 60..64 GiB device memory + `CU_DEVICE_ATTRIBUTE_INTEGRATED` | `fathomdb-embedder/tests/tegra_fragmented_va_cuda.rs` | test gate | the only device-identity-gated test; it reads the attribute at runtime and skips elsewhere with a reason |

**(b) Product defaults** (all in PP, experiment feature only):

| Constant | Value | Documented | Override | Risk elsewhere |
| --- | --- | --- | --- | --- |
| `DEFAULT_MAX_SIZE` | 3 GiB | yes (rationale: 1 GiB capacity on this device = 3.8 × high-water) | `FATHOMDB_POOL_MAXSIZE` | see below; the largest risk |
| `DEFAULT_RELEASE_THRESHOLD` | 0 | yes | `FATHOMDB_POOL_RELEASE_THRESHOLD` (`0` / `max`) | low |
| `DEFAULT_TRIM_IDLE_MS` | 5000 | yes, as an unmeasured choice | `FATHOMDB_POOL_TRIM_IDLE_MS` | low (the arm is opt-in) |
| `PROBE_BYTES` | 4 | yes | no (not needed) | none; but CORE:230 `installed_pool_is_usable` repeats a bare `4` |
| `TRIM_TICKS_PER_IDLE`, `TRIM_TICK_MIN_MS`, `TRIM_TICK_MAX_MS` | 4, 1, 250 ms | yes; were the bare literal `(idle_ms / 4).clamp(1, 250)`, named during this audit with a test | no | low |
| exit-event ordinal | was `0` for the P teardown | — | — | fixed during this audit to use the private pool's ordinal; non-P teardown still reads device 0, which is a diagnostic only |

**(c) Test and harness tolerances.**

- Held in `harness/study-config.sh` and env-overridable:
  - quiet: 40 GiB free, load 2.0, 3 GPU-idle samples, 7200 s wait;
  - 8 GiB memory floor;
  - 1 MiB swap tolerance;
  - 600 s / 120 s timeouts;
  - 8 GiB pool cap.
- Not yet wired (follow-ups found by the audit):
  - The C probes repeat the 8 GiB cap and the memory floors as literals:
    `pool_c5.c`, `pool_capacity.c`, `pool_reset.c`, `pool_gap.c --hold`.
    `pool_gap` has no maxSize cap, but it is driven only by `gap-sweep.sh`,
    which was not re-run.
  - `STUDY_IDLE_AFTER_CLOSE_S` is an orphan. The CB matrices pass
    `IDLE_AFTER_CLOSE_S=10` explicitly.
  - The gap-sweep ranges and the sysfs path stay in their scripts.
  - The consumer workload defaults (`WARMUP`, `TIMED`, `CYCLES`, `STRESS_*`,
    batch sizes) and the statistics constants are not in the config:
    bootstrap 10 000 resamples with seed 1, z 1.96, the +5 ms and +32 MiB
    gates. They are env- or argument-overridable per script.
- The cudarc unit tests use a 192 MiB pool with 8 MiB chunks. They rely on
  the cap, not on the /3 ratio, so they are generic.

**(d) Incidental.** No action:

- unit shifts and the `G` / `M` suffixes;
- `u64::MAX` for `max`;
- `max_size: 0` = the driver default;
- `trim_to(0)`;
- hand-copied CUresult values. EI:50 compares against `2`
  (`CUDA_ERROR_OUT_OF_MEMORY`) instead of `sys::CUresult`. EI is the
  production hook as of 0.8.27 and is untouched by the study; a follow-up
  for the main line;
- `CU_MEMPOOL_ATTR_USED_MEM_CURRENT = 7` in `pool_smoke.py`;
- `MAP_FIXED_NOREPLACE`;
- fill bytes and fake handles;
- device 0 in probes.

**What would break or become unsafe on other configurations.**

- **Jetson Orin NX / Nano (8–16 GB).**
  - If /3 holds, a 3 GiB maxSize pins at most 1 GiB, 6–12 % of shared RAM.
  - If it does not hold, up to 3 GiB, 37 % of 8 GB.
  - The reservation and hole sizes in the docs would be wrong.
  - The harness cannot run there: the 40 GiB quiet floor and the
    2·maxSize + 16 GiB C-probe precheck are unreachable, and the fixed
    addresses and 64 GiB sweeps assume this layout.
- **Jetson Thor (CUDA 13).** Unverified: the /3 ratio, how the default pool
  fails, the window, and the sysfs path. A missing path means permanent
  "not quiet".
- **Non-Tegra aarch64 (GH200, GB10).** This is the main product exposure.
  These hosts pass the `cfg(target_os = "linux", target_arch = "aarch64")`
  gate, so they get the cudarc fallback and the registration-time `cuInit`.
  With the experiment feature they also get the pool policy. On GH200 a
  3 GiB cap bounds HBM: a large batch that the default pool would serve
  fails as `cuda_pool_exhausted`, a new failure mode. Nothing in the
  embedder refuses SBSA at runtime; only the CLI reports
  `Arm64SbsaUnsupported`. 64 KiB-page kernels also break the harness's
  `4096` alignment.
- **x86_64 discrete.** Nothing here is compiled. If it were ported, 3 GiB
  is 75 % of a 4 GB card and an arbitrary throttle on an 80 GB card.
- **The cudarc sync fallback itself** uses no platform constant. It is
  safe on any device.

**Gating recommendation.**

1. Keep `cfg(target_os = "linux", target_arch = "aarch64")` only to limit
   what is compiled.
2. At the first device, decide at runtime from `CU_DEVICE_ATTRIBUTE_INTEGRATED`,
   `MEMORY_POOLS_SUPPORTED` and `cuDeviceTotalMem`:
   - apply the private pool only on an integrated device;
   - elsewhere behave as `S` (the shipped behaviour) and log why.
3. Do not treat `DEFAULT_MAX_SIZE` as a constant beyond the AGX Orin 64 GB.
   Either derive it from the workload high-water mark with a stated
   headroom factor, capped at a stated fraction of total memory, or require
   `FATHOMDB_POOL_MAXSIZE` on unmeasured devices.
4. Add the device name, total memory and integrated flag to the
   `fdb-pool-exp` install event, so results from different devices cannot
   be mixed.

### 11.10 Harness defect: blocks were not randomised

`interleave.sh` shuffled each block with `shuf --random-source=<(yes
"$seed-$b")`. `shuf` reads only the first bytes of its random source, and
those are the seed, which is the same for every block. **Every interleaved
series in Phases 0–2 ran one fixed order in every block**: the Phase 0 and
1b equivalence series, P1, R5 (Node 25 chunks), C9b and C1.

Interleaving still protected those series against slow drift, because
every block ran every cell. Position effects, such as always following a
given cell, were not randomised. For the correctness rows this does not
matter. The timing results most exposed are:

- the Phase 0 and 1b equivalence ratios (§ 2.2, § 10.4);
- P1 (§ 3.4).

They should be read with that caveat.

The fix (Python's seeded `random.Random("<seed>-<block>")`) was applied
during Phase 2. `cb12-h0`, `cb12-h400k`, `trimcycle` and the Phase 2b top-ups
ran randomised orders (verified in each `blocks.txt`).

### 11.11 Decision-table rows (Phase 2)

| Clause or criterion | Status | Evidence |
| --- | --- | --- |
| C1 provenance | PASS (200/200; one decide per process) | § 11.2 |
| C2 lifetime (50 cycles) | FAIL as specified for P, A and B (0/16, at the 8th open); PASS with GC after close (11/11) | § 11.3 |
| C5 zero-length, product suites under P | PASS (9/9 `candle_bge` + 1/1 reranker empty batch under P, forced CUDA) | § 11.2 |
| C6 multi-device | code path only (pure tests) | § 11.2 |
| C7 co-resident reset | C probe PASS (20/20); product FAIL for every variant including S (0/60) | § 11.4 |
| C8 parity | PASS (80/80 Python; hashes and scores identical to Node) | § 11.2 |
| C9 (ruling 13) | PASS (0/80 current = private; 44/44 current = default when both read) | § 11.5 |
| C9b | no harm detected (P 23/30 vs S 20/30); powered only for large harms | § 11.5 |
| CB1 cap | PASS as written; observable weak (pinned pool memory is not in VmRSS) | § 11.6 |
| CB2 trim after close, threshold 0 | FAIL as written (64 MiB reserved, 17 MiB in use by singletons) | § 11.6 |
| CB2 with the trim arm | FAIL as written; trim releases nothing at threshold 0 (64 MiB in both arms) | § 11.6, § 11.7 |
| CB3 typed cap error | PASS (P: kind `cuda_pool_exhausted`, 40/40; Node `CudaPoolExhaustedError`) | § 11.6, § 11.8 |
| CB4 no CPU move | PASS (60/60) | § 11.6 |
| Trim arm safety (ruling 14) | no failure in 50 processes (4000 stress embeds, 100 trimmed cycles, 20 CB2); UAF not instrumented | § 11.7 |
| Trim effect and re-grow latency | no effect at threshold 0 (the trims never lowered `reserved_cur`), so no re-grow was priced; the arm's own overhead is not detectable (0.993 [0.984, 1.001]) | § 11.7 |
| Python import hook (ruling 1 / 12) | Implemented and present in the experiment wheel that every Python row used; Python C8 and C9 pass. Not shown separately: no Python heap-growth (R5-style) cell was run, so the hook's effect is not yet measured. | § 11.1 |

### 11.12 Deviations (Phase 2)

1. **C2 cell sizes.** A and B ran 3 processes, not 10. They failed at the
   same cycle as P, deterministically; the GC re-runs were 3 each. The C
   half of C2 (`pool_lifecycle.c` section D, 20 runs) was not run. It
   probes destroy-while-current, which P never does, since its pool is
   never current.
2. **C7 C half.** `pool_reset.c` (new) replaced `pool_teardown.c --reset`
   for the private pool. S, A and B's C-level reset cases were not re-run.
   The product reset row covers all four variants.
3. **CB2 at heap 400k** came from two series:
   - `cb12-h400k`: 10 blocks, threshold 0 and S, with the old fixed order;
   - `cb12-h400k-b`: 10 blocks of threshold 0, `max` and S, randomised.

   Threshold `max` at 400k has 10 processes, not 20.
4. **CB1 baseline.** The pooled median of S's synchronous-path processes in
   the same interleaved series, not a per-block median. Only 13 of 20 S
   processes at heap 0 took the synchronous path.
5. **Witness.** Off in every Phase 2 row (ruling 16). No witness row was
   run in Phase 2. The R5 witness failure (§ 10.6) stands as the witness
   evidence.
6. **The interleave defect** (§ 11.10) was found and fixed mid-phase.
   `c9b` and `c1` ran fixed orders.
7. **Code after measurement.** The trim-tick constants were named and the
   P exit event's ordinal changed after the Phase 2 addon was built. Both
   preserve behaviour on this single-device host: the tick formula is
   identical. The C5 product suites ran on the current code.
8. **CB3 contrast cells** A and B ran 10 processes each, as the matrix
   says, not 20 per variant.

### 11.13 GPU time (Phase 2)

Lock-held time:

| Part | Time |
| --- | --- |
| Product smoke | about 2 min |
| First driver, 16:08:54–17:21:48 UTC (including the C2 diagnosis runs that queued on the lock in between) | 72.9 min |
| Top-ups (C7 product, C8 A and B, C2 with GC, CB at 400k, CB2 trim), 17:22:01–17:52:51 | 30.8 min |
| C5 product suites and the trim diagnosis | about 2.5 min |
| **Total** | **about 1 h 48 min** |

Builds and test compiles ran outside the lock.

### 11.14 What needs a ruling

1. **C2: close does not free the model.** `Engine::close` keeps the
   embedder, and so its device memory, until the `Engine` drops. In Node
   that is garbage collection. On the shipped path this costs about
   159 MiB per closed, un-collected engine. Under any capped pool it fails
   at the 8th open. Options:
   - (a) a product fix: drop or release the embedder in `close`. It is
     needed for P, and arguably a 0.8.27 memory bug in its own right;
   - (b) treat P as blocked until (a) lands;
   - (c) accept, and document that cycles need GC.

   Recommendation: (a), test-first, as its own slice, outside this study.
2. **C7: a co-resident primary-context reset is fatal for every variant,
   the shipped one included.** Rule it "unsupported, recorded" rather than
   a pass criterion, or make surviving a reset a product requirement,
   which would be a separate investigation of Candle's context state. P
   adds more crashes at exit after a reset (19/20 against 13/20).
3. **CB2 as written cannot pass while the module-level singletons hold
   slices.** 16–17 MiB is held until exit, spread over two 32 MiB chunks,
   so 64 MiB stays reserved with or without trim. Options:
   - amend CB2 to "no wholly free chunk stays reserved after close +
     idle", which threshold 0 already meets;
   - make the trim arm the default. The evidence in § 11.7 says this
     would not help: at threshold 0 the trims release nothing;
   - accept the 47 MiB spare at threshold 0, and drop the trim arm, or
     test it only at threshold `max`.
4. **CB1's observable.** VmRSS does not see the pool's pinned memory on
   Tegra. Choose a system-level observable for Phase 4 (`MemAvailable`
   delta, `tegrastats` or nvmap), or accept CB1 as satisfied by
   `reserved_high` ≤ 1024 MiB alone.
5. **Timing results from Phases 0–1b** ran fixed block orders (§ 11.10).
   Re-run the equivalence and P1 comparisons in Phase 3 with the fixed
   harness, or accept them with the caveat.
6. **Error kind (ruling 15) open points.** These are for adoption:
   - the reranker's forward is not classified yet;
   - Python's dedicated subclass.

   The draft wording is in § 11.8.
7. **Runtime gating (constants audit).** Gate the pool policy on
   `CU_DEVICE_ATTRIBUTE_INTEGRATED` and total memory at the first device,
   and derive or require `maxSize` off the measured device. Keep
   `cfg(aarch64 linux)` only to limit what is compiled (§ 11.9).
