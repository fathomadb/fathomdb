---
title: FathomDB 0.8.28 Tegra CUDA memory-pool study — results, Phases 0-4
status: PARTIAL (Phases 0, 1, 1b, 2, 3 and 4; Phase 5 not started)
target_release: 0.8.28
observed_on: 2026-10-06
---

# Tegra CUDA memory-pool study: results of Phases 0–4

This records Phases 0 and 1 of
`dev/plans/0.8.28/prework/tegra-cuda-memory-pool-experiment-protocol.md`
(the protocol) on one Jetson AGX Orin 64 GB, Phase 1b of protocol
revision 3 (§ 10), Phase 2 of revision 4 (§ 11), and Phases 3 and 4 of
revision 5 (§ 12). It does not rule. Phase 5 (analysis and upstream
package) was not started. Everything not measured here is marked UNMEASURED.
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

## 12. Revision 5: rulings 18–25, Phase 3 and Phase 4

Revision 5 applies the owner's rulings on the Phase 2 decisions (protocol
"Owner rulings (2026-10-06, after Phase 2) and revision 5"). It then runs
Phase 3 (robustness) and Phase 4 (performance). Ruling 25, given during
Phase 4, ranks allocation correctness and release above latency, so
§ 12.7 reports them first.

Every Phase 3 and Phase 4 series ran on the revision-5 build:

- **Node addon:** `consumer-p4` (sha `ee544bbb…`).
- **Python wheel:** `wheel-p3` in `venv-p` (sha `ba87181d…`).

Both builds contain:

- the owner's close fix;
- runtime gating and sizing;
- the Python and reranker exhaustion kinds.

Pool sizing was derived from the device, with no `FATHOMDB_POOL_MAXSIZE`:
3 GiB on this host (`install` event `integrated=1 pools_supported=1
total_mem=65879896064 gate=on size_source=derived`). The witness was off,
and the interleave was the fixed one (§ 11.10). Summaries:
`summaries/phase3*.txt`, `summaries/phase4*.txt`,
`summaries/rev5-*.txt`.

### 12.1 Code (revision 5)

| Commit | Content | Tests |
| --- | --- | --- |
| `7b9b18146` | Runtime gating and sizing (ruling 24); the Python `CudaPoolExhaustedError` and the reranker's forced-CUDA classification (ruling 23); the C7 investigation mode. Also a fix: `new_cuda_device` was gated on the CUDA features, so a CPU-only `default-embedder` build of `fathomdb-embedder` did not compile on the study branch. | Pure sizing and gate tests: Orin 64/32/16/8 GB, Thor, discrete, pool-less, override. Reranker classification. napi and Python binding tests. Each went red, then green. |
| `f0b6b4c7e` | Cherry-pick of the owner's close fix `c816b8653` (ruling 18). It applied without conflicts. Study only; not pushed to any release branch. | `fathomdb-engine` `tests/projection_runtime.rs`: 13 passed, 1 ignored, including `close_releases_embedder_while_engine_handle_is_retained` |
| `10dbcd7ce` | Protocol and plan revision 5, plus harness: `cb1check.py` with pure tests (ruling 21), `concurrent-runner.sh` (R7), soak mode (R6), Python perf mode (P7), and `MemAvailable` at every measurement point. | 8 pure `cb1check` tests |

Gates:

- `cargo clippy -D warnings` is clean for:
  - the workspace;
  - `fathomdb-embedder`, `fathomdb-engine` and `fathomdb-napi` with
    `embed-cuda`, `rerank-cuda` and `tegra-pool-experiment`;
  - `fathomdb-py` with `tegra-pool-experiment`, `default-embedder` and
    `default-reranker`.
- `cargo fmt --check` is clean.
- The policy has 16 pure tests.
- The experiment feature is in no release feature set.

**Runtime gate and sizing (`cuda_pool_policy.rs`).** At the first device,
the policy reads three facts: `CU_DEVICE_ATTRIBUTE_INTEGRATED`,
`CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED` and `cuDeviceTotalMem`.

1. A discrete device, or one without pool support, gets no pool. The
   process keeps the shipped path, and the `install` event says `gate=off`
   with the reason.
2. On an integrated device:
   - `FATHOMDB_POOL_MAXSIZE`, when set, wins.
   - Otherwise `maxSize` = device memory / 20, clamped to
     [2 GiB, 3 GiB].
   - No pool when the 2 GiB floor exceeds a quarter of device memory.

The constants are named and documented: `POOL_SIZE_DEVICE_DIVISOR`,
`POOL_SIZE_FLOOR`, `POOL_SIZE_CEILING` and `POOL_FLOOR_MAX_SHARE_DIVISOR`.
The `cfg(target_os = "linux", target_arch = "aarch64")` gate now only
limits compilation.

**Error kind.**

- **Python.** `CudaPoolExhaustedError` is created with `create_exception!`
  under `EmbedderError`, the same pattern as `EmbedDevicePolicyError` and
  `RerankerDevicePolicyError`, and TypeScript's
  `CudaPoolExhaustedError extends EmbedderError`. It carries `kind`,
  `ordinal`, `max_size_bytes` and `code` (`FDB_CUDA_POOL_EXHAUSTED`).
  `fathomdb.errors` exports it only in experiment wheels.
- **Reranker.** Under forced CUDA, a cross-encoder forward that exhausts
  the private pool now returns `RerankerDevicePolicyError::CudaPoolExhausted`
  (kind `cuda_pool_exhausted`) instead of the generic `CudaProbeFailed`
  refusal. napi maps it to `FDB_CUDA_POOL_EXHAUSTED`, and Python to
  `CudaPoolExhaustedError`.
- **Auto policy.** The batch-to-per-pair fallback is unchanged.
- **Remaining gap for adoption.** The module-level `rerank()` function
  turns every error into a string, and the bindings surface it as
  `WriteValidationError`. This is pre-existing and also seen in C7, so its
  pool exhaustion is not typed.

The smoke on the revision-5 build passed:

- **Node:** `CudaPoolExhaustedError` / `FDB_CUDA_POOL_EXHAUSTED`.
- **Python:** `CudaPoolExhaustedError`, ordinal 0, `maxSizeBytes`
  3221225472, MRO `CudaPoolExhaustedError → EmbedderError → EngineError`.
- **Sizing:** derived 3 GiB; `FATHOMDB_POOL_MAXSIZE=1G` gives
  `size_source=env`.

### 12.2 C2 rerun on the build with the close fix (ruling 18)

Node 25, `cycles` mode: 50 `open` / `embed` / `close` cycles per process,
**no garbage collection**, `FATHOMDB_POOL_COEXIST_CHECK=1`. The four
variants ran interleaved in randomised blocks, 10 blocks.

| Variant | n | Pass (Wilson 95 %) | allocMode | VmRSS growth per cycle, cycles 2–50 (median, max) | Max VmRSS |
| --- | --- | --- | --- | --- | --- |
| S | 10 | 10/10 [72.2, 100] % | sync 7, default 3 | 0.33, 0.40 MiB | 445 MiB |
| P-first-use | 10 | 10/10 [72.2, 100] % | private 10 | 0.36, 0.42 MiB | 453 MiB |
| A-first-use | 10 | 10/10 [72.2, 100] % | explicit 10 | 0.40, 0.46 MiB | 453 MiB |
| B | 10 | 10/10 [72.2, 100] % | explicit 7, default 3 | 0.38, 0.46 MiB | 454 MiB |

**C2 passes for every variant: 40/40 [91.2, 100] %.** The per-engine
retention is gone:

- S grew about 159 MiB per closed engine before the fix (§ 11.3) and
  0.33 MiB per cycle after it.
- Every pool variant failed at the 8th open before the fix, and none fails
  now.

The remaining ~0.35 MiB per cycle is the same in every variant, S
included, so it is not pool memory. It comes to about 17 MiB over 50
cycles; its source was not investigated.

**Decision rule.** P is still not the default until the fix ships
(ruling 18).

### 12.3 C7 investigation (ruling 19)

**Method.** Python `reset` mode under `gdb -batch`: 5 S and 5 P processes
on the revision-4 wheel. Each process runs embed, rerank and CLS, then
calls `cuDevicePrimaryCtxReset(0)` through `ctypes`. Afterwards it tries
each call separately and records its first error and the driver's state.
Evidence: `summaries/rev5-c7-investigation.txt`.

**First error after the reset.** It was the same in 10 of 10 processes:

| Call | Error |
| --- | --- |
| `embed_batch_cls` | `DriverError(CUDA_ERROR_CONTEXT_IS_DESTROYED)`. Candle's cudarc context still names the destroyed primary context. |
| `engine.embed` | `EmbedderError: embedder error`, the engine's collapsed form |
| `rerank` | The re-probe refuses with `CudaProbeFailed`, surfaced as `WriteValidationError` (the string path of § 12.1) |
| driver state | `cuCtxGetCurrent` and `cuCtxSynchronize` both succeed: the reset context is usable, but FathomDB's handles point at the destroyed one |

**Exit crash.** SIGSEGV in 10 of 10 processes under gdb, S and P alike,
with the same stack:

```text
libcuda.so (no symbols)
cudarc::driver::safe::core::CudaStream::wait            <- cuStreamWaitEvent
<cudarc::driver::safe::core::CudaSlice<T> as Drop>::drop
drop_in_place<candle_transformers::models::bert::BertModel>
drop_in_place<fathomdb_embedder::candle_bge::CandleBgeEmbedder>
drop_in_place<fathomdb_engine::projection_runtime::ProjectionRuntimeShared>
drop_in_place<fathomdb_engine::projection_runtime::ProjectionRuntime>
drop_in_place<fathomdb_py::engine::PyEngine>             <- Python dealloc
```

**Component.** The crash is in cudarc's `CudaSlice` drop. Before freeing,
it waits on the slice's read and write events (`cuStreamWaitEvent`). The
stream and event handles were destroyed with the context, and libcuda
dereferences them and faults instead of returning an error.

- It is common to every variant, the shipped path included.
- It is not the study's pool teardown, attribute reads or exit handler. No
  crashing process reached the study's exit event, so its `teardown` line
  never printed.
- The same stack shows the C2 retention the owner fixed: the projection
  runtime held the embedder.

**P's excess crashes are not a separate mechanism.** Without gdb the rates
were S 13/20 and P 19/20 (§ 11.4). Under gdb both crash 5/5, with
identical frames. A guard on pool destroy or attribute reads would change
nothing, because the process crashes before the study's code runs. So no
study-gated guard was added (ruling 19).

**Possible fix (not made).** The drop paths in cudarc's `CudaSlice` and
`CudaEvent` could skip the wait and free when the context is no longer
valid, for example by checking `cuCtxGetApiVersion` on the slice's context
or by treating `CUDA_ERROR_CONTEXT_IS_DESTROYED` as fatal for the
context. That would be a cudarc-level, all-variant product change. It
would also leak the device memory, which the reset already freed. It is
for the owner to rule on.

A rerun on the revision-5 build is in § 12.7.4. It checks whether the close
fix moves the crash: the model is now dropped inside `close()`, not at
Python dealloc.

### 12.4 Pool sizing policy: analysis, not a decision (ruling 24)

**What the size does.** At release threshold 0, `maxSize` is a **cap, not
a reservation**:

- The pool holds only what is in use, plus the free part of the 32 MiB
  chunks that still hold live slices: 64 MiB idle in Phase 2 CB2.
- On this device the cap is reached at ceil32(`maxSize`/3) of allocations
  (C3).
- Creating the pool needs one contiguous `maxSize`/3 of address space
  inside the early-`cuInit` reservation (§ 3.2, § 10.7).

So a size can fail three ways:

- **Too small.** A large batch fails with `cuda_pool_exhausted`. CB3: 128 ×
  400-token passages exhaust 1 GiB of capacity, and no CPU fallback is
  taken.
- **Too large.** It does not pin memory at threshold 0. It loosens the
  circuit breaker, because a runaway workload consumes more shared RAM
  before it fails. It also raises the contiguous need: R5 showed 3 GiB
  (1 GiB need) creatable at every heap up to 8M objects.
- **Ratio wrong on another device.** If the /3 capacity ratio does not hold
  there, the cap binds at up to `maxSize`, not `maxSize`/3.

The workload high-water mark is 160 MiB in `full` mode (R5, Phase 2) and
272 MiB at batch 128 with rerank (§ 3.5).

**Options for each class**, given the workload high-water mark and that
capacity is about `maxSize`/3 here:

- **fixed:** 3 GiB everywhere;
- **fraction:** 1/20 of device memory, clamped to [2, 3] GiB, as
  implemented;
- **workload:** capacity ≥ 2–4 × the 272 MiB high-water mark, so `maxSize`
  1.6–3.3 GiB at /3;
- **off:** the shipped path (S).

| Device class | Device memory | fixed 3 GiB | fraction (implemented) | workload-derived | off (S) | Failure modes | Recommendation |
| --- | --- | --- | --- | --- | --- | --- | --- |
| AGX Orin 64 GB (measured) | 61.36 GiB | 3 GiB; capacity 1 GiB = 3.8 × high-water | 3 GiB (ceiling) | 1.6–3.3 GiB | slow sync path in about 80 % of Node processes (C1) | none measured at 3 GiB; batch-128 long passages exhaust it (CB3, typed) | **3 GiB** (accepted ruling) |
| AGX Orin 32 GB | ~30 GiB | 3 GiB = 10 % of RAM as a cap | 2 GiB (floor) | 1.6–3.3 GiB | as above, if the default pool fails there too (UNMEASURED) | /3 and the reservation size unverified; if /3 fails, 2 GiB is a 7 % cap | fraction (2 GiB) **after** one C3 capacity run and an R5 cell on the device |
| Orin NX 16 GB | ~15 GiB | 20 % of RAM as a cap | 2 GiB (13 %) | 1.6 GiB | as above | as above; a cap that binds hurts other processes more on 16 GB | fraction, with the same on-device check; consider 1.6 GiB (the workload floor at 2 × high-water) |
| Orin Nano / NX 8 GB | ~7.4 GiB | 40 % of RAM: unsafe if /3 fails | **off** (the floor exceeds 1/4 of memory) | 1.6 GiB = 22 % of RAM | the shipped path | the pool's benefit (fast path) against a 22–40 % cap; the reservation for 8 GB is UNMEASURED | **off** (implemented). Revisit only with on-device data. |
| Thor 128 GB (CUDA 13, Blackwell iGPU) | ~122 GiB | 3 GiB | 3 GiB (ceiling) | 1.6–3.3 GiB | — | whether the default pool fails at all, the /3 ratio, the reservation and CUDA 13 pool behaviour are all UNMEASURED | ceiling 3 GiB, gated by an on-device R5 and C3 run before enabling; until then, **opt-in** (`FATHOMDB_POOL_VARIANT`) |
| Discrete GPU (x86_64 or aarch64 dGPU) | VRAM | the cap bounds VRAM, not host RAM: 75 % of a 4 GB card, an arbitrary throttle on 80 GB | **off** (implemented) | — | upstream default pool (stream-ordered and fast) | the study's problem (no contiguous 20960 MiB host VA for the default pool) does not arise. Whether /3 is a Tegra artefact is UNKNOWN: no amd64 host was available for a C3 probe. | **off** |
| Non-Tegra aarch64: GH200 | HBM + coherent LPDDR | as discrete | off if the device reports `integrated=0` (expected for Hopper; UNVERIFIED) | — | upstream path | whether its default pool fails; the `integrated` value | **off**; verify the attribute |
| Non-Tegra aarch64: GB10 (unified LPDDR5x) | ~128 GB shared | — | if it reports `integrated=1`, the policy would apply 3 GiB **unmeasured** | — | — | everything UNMEASURED | add a Tegra-identity condition (e.g. `/etc/nv_tegra_release`, as `tegra_fragmented_va_cuda.rs` uses) or require opt-in until measured |

**Recommendation.** Keep the implemented fraction rule (1/20 within
[2, 3] GiB, off below 8 GiB) as the shape. Ship it enabled only for the
measured class: integrated, Tegra, about 64 GB. Every other class should
stay off or opt-in until a C3 capacity run and an R5 heap run exist on
that device. Every class marked UNMEASURED needs hardware this study does
not have (R9).

### 12.5 CB1 (ruling 21)

`harness/cb1check.py` implements the check, with pure tests:

- **Primary:** `reserved_high` ≤ `maxSize`.
- **Sanity:** only when the `install` event says `integrated=1`. It
  compares the process's `MemAvailable` drop (before open → after rerank)
  with S's synchronous-path processes in the same series:
  - The noise band is 3 × MAD of the baseline, at least 16 MiB.
  - Above 256 MiB the result is inconclusive.
  - The process is consistent when its excess over the baseline median and
    `reserved_high` is ≤ 32 MiB + the band.
- **Skips, with a reason (never a failure):**
  - a discrete device;
  - no integrated flag;
  - no `MemAvailable` reading;
  - a baseline under 5 processes.

On this host:

- `/proc/meminfo` `MemAvailable` is readable.
- nvmap's debugfs is not: `/sys/kernel/debug` is root-only.
- `tegrastats` is readable but system-wide, and adds nothing to
  `MemAvailable`.
- No per-process GPU-memory source exists without root. `VmRSS` does not
  see the pool's pinned memory (§ 11.6), so the sanity check uses
  `MemAvailable` deltas.

Results are in § 12.6 (R2) and § 12.7.1 (Phase 4).

### 12.6 Phase 3: robustness (table 10)

**Arms.** The arms are P-first-use (the candidate) and S (the shipped
path). A-first-use and B are comparison arms that do not ship (§ 8.2).
They had no R-row of their own here, and they appear only in C2 (§ 12.2)
and Phase 4.

**Setup.** Node 25.9.0 unless a row says otherwise; full mode; witness
off; derived size 3 GiB; threshold 0 unless stated. Interleaved series
used randomised block orders (checked in each `blocks.txt`).

| Row | Cells | n | Result | Verdict |
| --- | --- | --- | --- | --- |
| R1 synthetic layouts (`pool_capacity`: `cuMemPoolCreate`, fill to the first error, 64 MiB chunks) | 3pages, free-gap, n-sweep (n = 3, 5, 6, 7 evenly spaced pages), random-18 (seeds 1–100); 2 and 3 GiB | 180 per size | Pool created and filled to its measured capacity in **360/360** [98.9, 100] %: 704 MiB at 2 GiB and 1024 MiB at 3 GiB in every run. First error `CUDA_ERROR_OUT_OF_MEMORY` (the cap) in every run. | PASS (bounded at 98.9 %) |
| R2 real heaps, Node 25 | P vs S × heaps 0, 400k, 1M, 4M, 8M | 30 per cell | P **150/150** [97.5, 100] %, private 150/150. S 150/150, with sync 118 and default 32. One embed hash. | PASS |
| R2 real heaps, Node 24 and 26 | P vs S × heaps 0, 1M, 4M | 10 per cell | P 60/60 [94.0, 100] %, private 60/60. S 60/60, sync 48 and default 12. | PASS |
| R3 heap growth in use (open at heap 0, then 1M live objects, then embed) | P vs S × Node 24, 25, 26 | 20 per cell | P 60/60 [94.0, 100] %, private 60/60. S 60/60. | PASS |
| R4 late import without `--import` | P vs S × late-400k, late-1M | 15 per cell | late-1M: **P 0/15, S 0/15**. late-400k: P 5/15, S 8/15 (Fisher two-sided p ≈ 0.46). Every failure is at open: `cuInit` `CUDA_ERROR_OUT_OF_MEMORY` (the driver's range), the documented 0.8.27 limit. | as expected: no pool difference; P needs early `cuInit` (ruling 12) |
| R4 late import with `node --import fathomdb` | same | 15 per cell | P 30/30 [88.6, 100] %, private 30/30. S 30/30. | PASS |
| R7 concurrency, P (threshold 0 and `max`) | k = 2, 4, 8 processes in `perf` mode started within 100 ms, 10 trials each | 140 + 140 | **280/280** [98.6, 100] %, private 280/280. Lowest `MemAvailable` 45.7 GiB, no swap rise. Steady embed (median of process medians) 10.8 / 16.7 / 17.6 ms at k = 2 / 4 / 8 (threshold `max`: 9.9 / 16.7 / 17.6). | PASS |
| R7 concurrency, S | k = 2, 4, 8, 5 trials each | 70 | 70/70 pass. Steady embed by path: sync **71.5 / 196.6 / 224.7 ms**; default (k = 4, 8) 12.4 / 10.1 ms. | recorded |
| R8 threshold (perf, close, 60 s idle) | P threshold 0 vs `max` | 10 + 10 | After the idle, `reserved_cur`: 192 MiB at threshold 0, 288 MiB at `max` (= `reserved_high`). Steady embed 12.92 vs 13.02 ms, ratio 1.008 [0.791, 1.194]; batch-128 84.3 vs 83.8 ms. | threshold 0 holds 96 MiB less, at no latency cost detectable at this n. Phase 4 has the powered latency comparison. |
| R6 soak, 20 min | P and S, one process each, `--expose-gc` | 1 + 1 | 0 hash mismatches, no allocator error, no floor abort in either. P: 12 506 iterations, pool `reserved_cur` flat, VmRSS at minute 20 5.4 % over its first-5-minute high, latency drift under 1 %. S: 5 615 iterations, VmRSS 12.6 % over its first-5-minute high. Detail below. | P PASS (n = 1, below the protocol's 2); S over the 10 % RSS bound, without a pool |
| CB1 (ruling 21) over R2 Node 25 | every private process | 150 | Primary `reserved_high` ≤ `maxSize`: **150/150**. Sanity check: 150/150 consistent, against 118 S-sync processes. | PASS (sanity check weak; below) |

**Every Phase 3 failure was `cuInit`.** All 47 failures are the late-import
`cuInit` refusal: the R4 rows without `--import`, P and S alike. No row
failed for a pool reason.

**R7.** Under concurrency the shipped synchronous path degrades sharply:
at 8 concurrent processes a steady embed takes about 225 ms, against
17.6 ms for P. P scales like the default pool.

The default-pool processes in S's mixed trials ran 10–12 ms. They are not
comparable with P's all-pool trials, because their sync neighbours keep
the GPU less busy.

**CB1 sanity check.** It is consistent in every process, but weak. Each
process's `MemAvailable` drop from before open to after rerank had these
medians:

| Path | n | Median drop | IQR |
| --- | --- | --- | --- |
| sync | 118 | 205 MiB | 201–210 |
| default | 32 | 218 MiB | 212–225 |
| private | 150 | 224 MiB | 218–231 |

So P costs about +19 MiB of `MemAvailable` over S-sync. Its pool reports
`reserved_high` 160 MiB, so `MemAvailable` sees less than the pool
reserves. Phase 1 found the same for `pool_capacity` on Tegra. The check
bounds the excess from above only.

**R6, per-minute samples** (`summaries/phase3-r6-soak.txt`):

| | P (private) | S (sync) |
| --- | --- | --- |
| iterations in 20 min (embed + batch-32 + rerank + write 50 docs) | 12 506 | 5 615 |
| steady embed, first 5 min max → last 5 min median | 22.2 → 22.0 ms | 46.0 → 43.8 ms |
| batch-32 | 50.0 → 47.6 ms | 138.3 → 136.8 ms |
| rerank | 8.6 → 8.5 ms | 13.2 → 12.4 ms |
| VmRSS, minute 1 → 20 | 809 → 901 MiB (steps at minutes 4 and 9, then flat for 11 min) | 787 → 938 MiB (steps through minute 16) |
| pool `reserved_cur` (stats event each minute) | 288 MiB flat, 20/20 samples | — |

No latency drifts in either arm. P's minute 18 completed 324 iterations
against about 630 in its neighbours, with unchanged per-call latency; the
cause is unknown, because the soak samples the host only before and after
the run. Both arms' VmRSS rises in steps, S's
more than P's, while P's pool stays flat. The growth is therefore not
pool memory; it is consistent with SQLite and allocator growth under 280k
to 625k written documents. The soak is one process per arm (ruling 4: 15–30
minutes), so it shows the absence of a fast leak, not a rate.

### 12.7 Phase 4: allocation correctness and release, then performance

Ruling 25 (owner, during Phase 4) ranks allocation correctness and release
above latency, so this section reports them first. Summary:
`summaries/phase4.txt`.

**Series.** All on the revision-5 build, which includes the close fix
(ruling 18). The witness was off and the no-swap condition held.

| Series | Cells | Blocks | Runs |
| --- | --- | --- | --- |
| perf (`perf` mode: 5 warm-up + 50 timed calls each of steady embed, `embedBatchCls` at 1, 8, 32 and 128 texts, and rerank) | S, P threshold 0, P threshold `max`, B × Node 24, 25, 26 | 20, randomised | 240 |
| extra S block (protocol § 7) | S × Node 24, 25, 26 | 20, randomised, run **after** the perf series | 60 |
| Python P7 (`perf` mode, `wheel-p3`) | S, P, B, Node-free | 20, randomised | 60 |
| ingest P5 (10 000 documents, 100 per write) | S, P | 5, randomised | 10 |
| equivalence rerun (ruling 22) | production 0.8.26 S, revision-5 S | 20, randomised | 40 |
| P1 rerun (ruling 22) | P, S | 20, randomised | 40 |

#### 12.7.1 Allocation correctness and release (ruling 25)

Every private-pool process on the revision-5 build, Phases 3 and 4. The
state is the pool's own counters at the exit `teardown` event, after
`engine.close()`:

| Series | Mode | Private-pool processes passing | Pool after close: `reserved_cur` / `used_cur` / `reserved_high` (MiB) |
| --- | --- | --- | --- |
| R2 (Node 24, 25, 26), R3, R4 (the processes past `cuInit`) | `full` | 305/305 | 64 / 16.7 / 160, every process |
| R7 concurrency, threshold 0 / `max` | `perf` | 280/280 | 192 / 143.4 / 288 (140) and 288 / 143.4 / 288 (140) |
| R8, 60 s idle after close | `perf` | 20/20 | as R7 |
| R6 soak, 20 min | soak | 1/1 | 192 / 143.4 / 288 |
| Phase 4 Node perf, threshold 0 / `max` | `perf` | 120/120 | as R7 (60 + 60) |
| Phase 4 Python perf | `perf` | 20/20 | 192 / 143.4 / 320 |
| Phase 4 ingest | `ingest` | 5/5 | 64 / 16.7 / 160 |
| **All** | | **751/751 [99.5, 100] %** | |

1. **Every requested allocation that fits succeeds.**
   - 751/751 private-pool processes passed, with no allocator error. That
     includes 8 concurrent processes (R7) and 12 506 soak iterations.
   - R1's synthetic pools filled to their measured capacity in 360/360.
   - The only failures on the revision-5 build were:
     - the late-import `cuInit` refusals (R4, P and S alike), which happen
       before any pool exists;
     - the deliberately oversized CB3 batch (Phase 2), which is meant to
       fail.
2. **Pool memory returns to its baseline after close.**
   - **The baseline is the module-level singletons.** These hold their
     models for the life of the process: the reranker behind `rerank()`
     (16.7 MiB in use) and, in `perf` mode, the CLS embedder behind
     `embedBatchCls` (143.4 MiB in total with the reranker). `used_cur` at
     exit equals exactly that set in every process.
   - **The engine's own embedder is released by `close()`.** While open,
     `used_high` reaches 272 MiB. If close had kept the engine's model,
     `used_cur` would stay about 127 MiB higher. (*Inferred* from the
     counters; no per-allocation trace was taken.)
   - **Threshold 0.** `reserved_cur` exceeds `used_cur` by 47–49 MiB, less
     than two 32 MiB chunks. That fits ruling 20 (no wholly free chunk
     stays reserved), but per-chunk occupancy was not instrumented.
   - **Threshold `max`.** The pool keeps its high-water mark after close:
     288 MiB, of which 145 MiB is free. This is one reason to prefer
     threshold 0 (§ 12.7.2).
   - **Over time.** C2 (§ 12.2) closed and reopened 50 times per process
     with no growth beyond S's. In the soak, `reserved_cur` stayed at
     288 MiB for 20 minutes.
3. **Caps are respected.**
   - `reserved_high` ≤ `maxSize` (CB1 primary) in every process checked:
     Node perf 120/120, Python perf 20/20 and R2 150/150.
   - The highest `reserved_high` anywhere was 320 MiB (Python perf),
     against a 3072 MiB `maxSize` (1024 MiB capacity on this device, C3).
4. **Exhaustion is typed.**
   - In Phase 2 CB3, the cap error reached Node as `CudaPoolExhaustedError`
     / `FDB_CUDA_POOL_EXHAUSTED` in 20/20 processes.
   - In Python, Phase 2 CB3 surfaced it as `EmbedderError` with kind
     `cuda_pool_exhausted` in 20/20. The revision-5 build adds the dedicated
     `CudaPoolExhaustedError` (§ 12.1), shown once in the smoke.
   - The next embed stayed on CUDA in 60/60 (CB4).
   - **Gap:** the module-level `rerank()` string path (§ 12.1) is untyped.
5. **No host-memory hogging.**
   - At 8 concurrent pool processes, `MemAvailable` never fell below
     45.7 GiB, and swap did not grow (R7).
   - The CB1 sanity check was consistent in 150/150 `full`-mode processes
     (R2).
   - In `perf` mode it is **inconclusive** for 120/120: S's own
     `MemAvailable` drops are bimodal (126–622 MiB), which gives a 442 MiB
     noise band against the 256 MiB limit.
   - With the extra S block in the baseline, all 120 are consistent (band
     99 MiB). That block ran outside the interleave, so it is not counted.
   - `MemAvailable` drop medians in the interleaved `perf` processes:

     | Path | n | Median drop |
     | --- | --- | --- |
     | sync | 50 | 408 MiB |
     | default pool | 21 | 440 MiB |
     | B explicit | 49 | 426 MiB |
     | P, threshold 0 | 60 | 440 MiB |
     | P, threshold `max` | 60 | 459 MiB |

     P is within the S spread.
   - The Python processes recorded no `MemAvailable` at both points, so the
     sanity check skipped them with that reason.

#### 12.7.2 Performance gate (plan "Performance": ruling 3)

**References.**

- **S-sync (the floor):** S's synchronous-path processes, n = 99.
- **Node default-pool reference (the target):** S's and B's processes that
  took the default pool.
  - Pooled n = 32. That is 21 from the interleaved series (S 10, B 11) plus
    11 from the extra S block.
  - By Node version: 15 (Node 24), 12 (Node 25) and 5 (Node 26).
- **How often S got the default pool:** 10/60 in the interleaved series and
  11/60 in the extra block; Node 26 only 3/40.
- **Ratios** are medians of per-process medians, with a 95 % bootstrap
  interval (the process is the unit). The reference's n follows every ratio.

**Node, all versions pooled:**

| Measure | P / default-pool reference | P / S-sync (speed-up) | P threshold `max` / threshold 0 | P / B explicit |
| --- | --- | --- | --- | --- |
| steady embed | 0.986 [0.907, 1.114] (n = 60/32) | 0.517 [0.479, 0.568] (1.93×) | 1.044 [0.938, 1.134] | 0.930 [0.865, 1.085] |
| batch 1 | 0.999 [0.987, 1.011] (n = 60/32) | 0.356 [0.354, 0.359] (2.8×) | 0.998 [0.989, 1.006] | 1.002 [0.995, 1.014] |
| batch 8 | 0.998 [0.972, 1.074] (n = 60/32) | 0.338 [0.332, 0.361] (3.0×) | 0.993 [0.912, 1.053] | 1.019 [0.930, 1.132] |
| batch 32 | 0.993 [0.976, 1.007] (n = 60/32) | 0.239 [0.238, 0.240] (4.2×) | 0.998 [0.976, 1.006] | 1.007 [0.993, 1.075] |
| batch 128 | 0.994 [0.944, 1.006] (n = 60/32) | 0.204 [0.204, 0.206] (4.9×) | 1.003 [0.995, 1.008] | 0.998 [0.993, 1.005] |
| steady rerank | 0.981 [0.959, 1.003] (n = 60/32) | 0.658 [0.649, 0.665] (1.52×) | 1.016 [1.002, 1.035] | 1.014 [0.968, 1.046] |

Medians: steady embed is 12.77 ms for P, 12.95 ms for the default pool and
24.69 ms for S-sync. Steady rerank is 3.74, 3.81 and 5.68 ms.

**Node, by version.** P / default-pool reference, steady embed:

| Node | Ratio | n | |
| --- | --- | --- | --- |
| 24 | 0.986 [0.896, 1.296] | 20/15 | |
| 25 | 0.989 [0.797, 1.122] | 20/12 | **underpowered** |
| 26 | 1.095 [0.873, 1.359] | 20/5 | **underpowered** |

The same per-version labels apply to every measure (`summaries/phase4.txt`).
With only the interleaved reference (n = 21, Node versions pooled), steady
embed is 0.990 [0.909, 1.174].

**Python: the second default-pool reference.** This is not pooled with
Node.

- Python S took the default pool in **20/20** processes
  ([83.9, 100] %). *Inferred:* a Python process has no V8 heap
  reservations fragmenting its address space, so the default pool's
  contiguous range is still available.
- The Python bindings' overhead differs from Node's: for example, batch 32
  takes 16.1 ms in Python against 30.6 ms in Node.

P / Python S default pool (n = 20/20):

| Measure | Ratio |
| --- | --- |
| steady embed | 0.921 [0.786, 1.006] |
| batches 1–128 | 0.993–1.001, every interval within [0.983, 1.004] |
| steady rerank | 1.011 [0.991, 1.033] |

So in Python P costs nothing measurable against the default pool, and buys
nothing either: Python S already gets the default pool.

**Gate criteria** (plan, "Performance"):

| Criterion | Result | Verdict |
| --- | --- | --- |
| Steady embed within 1.15× of the default pool | 0.986 [0.907, 1.114] (n = 60/32) | PASS (bounded at 1.114); per version underpowered for Node 25 and 26 |
| Steady rerank within 1.15× of the default pool | 0.981 [0.959, 1.003] (n = 60/32) | PASS |
| 95 % CI of the speed-up over S-sync above 1.5× | embed 1.93× [1.76, 2.09] | PASS |
| (same criterion, rerank) | rerank 1.52× [1.504, 1.541] (n = 60/99); on the interleaved S alone (n = 50) [1.499, 1.538] | **marginal**: PASS at 1.504 with the extra block, short by 0.001 without it. The default pool itself reaches only 1.49× over S-sync on rerank (3.81 vs 5.68 ms), so the 1.5× floor is at the limit of the target for this measure. |
| First embed no worse than S | P / S-sync 0.903 [0.865, 0.931]; P / default pool 0.959 [0.916, 1.006] | PASS |
| Ingest throughput no worse than S | P / S 1.012 [0.917, 1.451]: 9954 vs 9836 documents/s, n = 5/5 (S: sync 3, default 2) | PASS on the point estimate; bounded at 0.917 (n = 5) |
| Import time ≤ 5 ms above S | +0.6 ms [−3.3, 5.8] (n = 20/20; 148.9 vs 148.4 ms) | PASS (bounded at 5.8 ms) |
| RSS overhead ≤ 32 MiB above S | 10.4 vs 10.3 MiB import RSS delta | PASS |

**P meets the performance gate on the pooled Node data.**

- It runs at the default pool's speed: every measure is within 2 % at the
  point estimate, and the embed interval stays under 1.15.
- It is 1.9–4.9× faster than the synchronous path on embeds.
- Its first embed is faster than S's.

The weak points:

- the rerank speed-up floor is marginal;
- the per-version reference is underpowered for Node 25 and 26 (Node 26
  rarely gets the default pool at all);
- ingest has n = 5.

**Threshold.** `max` gains nothing over threshold 0:

- embed 1.044 [0.938, 1.134];
- batches 0.993–1.003;
- rerank 1.016 [1.002, 1.035], slightly slower.

Threshold `max` also keeps 96 MiB more after close (§ 12.7.1, R8). **Recommend
threshold 0.**

**P against B (installed explicit pool):** equal within every interval.

#### 12.7.3 Equivalence and P1 reruns (ruling 22)

- **Equivalence** (fixed interleave), 20 + 20 processes:
  - one embedding hash (`d9dafb8c410005f3`) and one rerank score set
    across production 0.8.26 S and revision-5 S;
  - sync-path steady embed, revision-5 / production: 0.998
    [0.989, 1.009] (n = 15/17);
  - stream path: 0.975 [0.693, 1.066] (n = 5/3, underpowered).

  The revision-5 build does not change S.
- **P1:** P − S import is +0.6 ms [−3.3, 5.8]. Import RSS delta: 10.4 vs
  10.3 MiB. P adds nothing at import, as in Phase 1, now with randomised
  blocks.

#### 12.7.4 C7 on the revision-5 build

Python `reset` mode, 10 S + 10 P without gdb and 3 + 3 under gdb.
Evidence: `summaries/rev5-c7-close-build.txt` and
`samples/rev5-c7-gdb-close-S-run-1.txt`.

- **Exit status.** 10/10 S and 10/10 P end in SIGSEGV (exit 139),
  against S 13/20 and P 19/20 on the revision-4 build (§ 11.4).
- **No JSON record.** The crash now comes before the harness prints its
  record. The harness now writes the reset record to stderr before
  `close()`.
- **First errors after the reset** are unchanged in 6/6:
  - `embed_batch_cls`: `CUDA_ERROR_CONTEXT_IS_DESTROYED`;
  - `engine.embed`: "embedder error";
  - `rerank`: `CudaProbeFailed` as `WriteValidationError`;
  - the driver context is usable.
- **The crash moved into `close()`.** All 6 gdb runs, S and P alike, have
  this stack:

  ```text
  libcuda.so
  cudarc::driver::safe::core::CudaStream::wait
  <cudarc::driver::safe::core::CudaSlice<T> as Drop>::drop
  drop_in_place<candle_transformers::models::bert::BertModel>
  drop_in_place<fathomdb_embedder::candle_bge::CandleBgeEmbedder>
  fathomdb_engine::embed_dispatch::core::EmbedDispatcher::join_after_quiescence
  fathomdb_engine::runtime_lifecycle::<impl Engine>::close
  fathomdb_py::engine::PyEngine::__pymethod_close__
  ```

- **What the close fix changed.** The fix drops the model inside
  `close()`, where it used to be dropped at Python dealloc. So the same
  cudarc drop fault (§ 12.3) now fires earlier and deterministically.
- **No study guard.** It is still not the study's pool code, and no
  `teardown` event printed. So no study guard applies (ruling 19).
- **Effect on S.** After a co-resident reset, the close fix turns S's
  "13/20 crash at exit" into "crash in `close()` every time".
- **Who decides.** The fault is pre-existing, in every variant, and
  belongs to the owner's C7 ruling and the cudarc drop path. It is not a
  pool result.

#### 12.7.5 The cherry-picked close fix is not the one to adopt

`f0b6b4c7e` (the study's cherry-pick of `c816b8653`) has a P1 defect found
by an external review:

- `join_until` in the engine's `core.rs` drops the provider `Arc` while
  holding three locks: the provider mutex, the handles guard and
  `Engine::close`'s `close_lock`.
- A caller-supplied embedder whose `Drop` re-enters `Engine::close`
  therefore deadlocks.
- FathomDB's built-in Candle embedders do not re-enter, so every study
  result here stands.

**`f0b6b4c7e` must be replaced by the corrected fix before any adoption.**
C2 and a Phase 4 spot check should be re-run on the corrected fix.

### 12.8 Decision-table rows (revision 5)

| Clause or criterion | Status | Evidence |
| --- | --- | --- |
| Allocation correctness (ruling 25) | PASS: 751/751 [99.5, 100] % private-pool processes; no allocator error | § 12.7.1 |
| Release after close (ruling 25) | PASS: the engine's model is released; the module singletons stay (16.7 / 143.4 MiB); threshold 0 spare < 2 chunks | § 12.7.1 |
| C2 lifetime (50 cycles, no GC; close fix) | PASS (40/40 [91.2, 100] %; S growth gone) | § 12.2 |
| C7 co-resident reset | FAIL for every variant, S included; the crash is now in `close()` (6/6 gdb); not the pool | § 12.3, § 12.7.4 |
| CB1 cap (ruling 21) | primary PASS (290/290 checked); sanity check consistent 150/150 (`full`), inconclusive 120/120 (`perf`, noise) | § 12.5, § 12.6, § 12.7.1 |
| CB2 (ruling 20) | PASS at threshold 0 (inferred per chunk) | § 12.7.1 |
| CB3 typed cap error | PASS (Phase 2; Python subclass added, smoke n = 1); rerank string gap | § 11.6, § 12.1 |
| R1 synthetic layouts | PASS (bounded at 98.9 %) | § 12.6 |
| R2, R3 real heaps | PASS | § 12.6 |
| R4 late import | needs `--import` (ruling 12); P = S without it | § 12.6 |
| R6 soak | P PASS (n = 1, protocol asks 2) | § 12.6 |
| R7 concurrency | PASS (280/280) | § 12.6 |
| R8 threshold | threshold 0 recommended | § 12.6, § 12.7.2 |
| Performance gate (Node) | PASS pooled; rerank 1.5× floor marginal; Node 25/26 reference underpowered | § 12.7.2 |
| Performance (Python) | P = default pool (n = 20/20); Python S is never synchronous here | § 12.7.2 |
| Equivalence, P1 reruns | PASS | § 12.7.3 |
| Owner's close fix shipped (ruling 18) | NOT YET; the study's cherry-pick has a P1 deadlock and must be replaced | § 12.7.5 |

**Against the decision rule** (protocol § 8.2): on this host P-first-use
passes the allocation, release, cap and performance rows. It cannot become
the default yet:

- the owner's close fix has not shipped (ruling 18), and the cherry-picked
  form must be replaced;
- C7 is unruled (ruling 19);
- R4 requires early `cuInit` at module load (ruling 12).

### 12.9 Deviations (revision 5)

1. **R6** ran one process per arm, not two.
2. **The extra S block ran after the perf series**, not interleaved with
   it. The protocol adds it as a separate block. Ratios are given with it
   and, for steady embed, without it. The CB1 sanity baseline excludes it.
3. **The extra block's trigger.** The pooled interleaved reference
   (n = 21) was already above the protocol's trigger of 15. The block was
   run anyway, because the per-version references (8, 8 and 5) were not.
4. **C7 on the revision-5 build.** The 20 non-gdb runs left no JSON
   record. The harness change that writes the reset record before
   `close()` was made after them, and the 6 gdb runs used it.
5. **Python** recorded no `MemAvailable` pair, so the CB1 sanity check
   skipped those 20 processes.
6. **R4** cells without `--import` are expected failures (`cuInit`). They
   are not pool failures.

### 12.10 GPU time (revision 5)

Lock-held time, UTC:

| Part | Time |
| --- | --- |
| C7 investigation, gdb (revision-4 wheel) | about 1.5 min |
| Revision-5 smoke and the soak smoke | about 6 min |
| C2 on the close fix, 22:33:34–22:49:21 | 15.8 min |
| Phase 3, 22:52:31–01:45:46 | 173.3 min |
| Phase 4, 01:45:59–03:25:11 | 99.2 min |
| Extra S block, 03:25:16–04:01:10 | 35.9 min |
| C7 gdb on the revision-5 build | about 0.5 min |
| **Total** | **about 5 h 32 min** |

Builds ran outside the lock.

### 12.11 What needs a ruling

1. **Close fix.** Adopt the owner's *corrected* fix in place of
   `c816b8653` / `f0b6b4c7e` (the P1 deadlock). Then re-run C2 and a
   Phase 4 spot check on it.
2. **C7.** The crash after a co-resident reset now fires inside `close()`
   for every variant, in 10/10 processes. Choose one:
   - rule it "unsupported, recorded";
   - fund a cudarc drop-path fix: skip the wait and the free when the
     context is destroyed (§ 12.3).
3. **Rerank speed-up floor.** The 1.5× floor over S-sync is at the edge
   of what the default pool itself reaches on rerank (1.49×). Choose one:
   - accept P's 1.52× [1.504, 1.541];
   - re-base the rerank floor on the default pool's own speed-up.
4. **Per-version power.** The Node 25 and 26 default-pool references are
   n = 12 and 5. Node 26 rarely gets the default pool (3/40 S).
   Choose one:
   - accept the pooled reference, plus Python's (n = 20, default 20/20);
   - fund more S blocks on Node 26. The default pool is rare there, so this
     would need about 130 more S runs per 10 default-pool processes
     (3/40 = 7.5 %).
5. **Threshold.** Make threshold 0 the default. `max` costs 96 MiB more
   after close and gains no speed.
6. **Sizing beyond this device** (§ 12.4). Choose one:
   - enable only for integrated Tegra devices of about 64 GB, with a
     Tegra-identity condition so that GB10-class unified-memory devices do
     not inherit 3 GiB unmeasured;
   - make it opt-in elsewhere until a C3 run and an R5 run exist on that
     device.
7. **Module-level singletons.** `rerank()` and `embedBatchCls` keep
   their models, and so 16.7–143.4 MiB of pool, for the life of the
   process, past every `engine.close()`. This is the shipped behaviour on
   every path, not a pool effect. Choose one:
   - accept and document it;
   - add a release API.
8. **Typed exhaustion gaps for adoption:**
   - the module-level `rerank()` string path;
   - shipping the Python `CudaPoolExhaustedError` subclass beyond the
     experiment.
9. **CB1's sanity observable** is noisy in `perf` mode (`MemAvailable`
   drops are bimodal). Choose one:
   - accept the primary check alone off `full` mode;
   - fund a root-readable nvmap source.
10. **R4 late import** remains a precondition: early `cuInit` at module
    load (ruling 12).
