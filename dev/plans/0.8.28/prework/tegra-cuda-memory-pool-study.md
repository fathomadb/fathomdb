---
title: FathomDB 0.8.28 prework — Tegra CUDA memory-pool study plan
status: PROPOSED
target_release: 0.8.28
observed_on: 2026-10-05
---

# Tegra CUDA memory-pool study plan (0.8.28)

This is a study plan, not results. It defines what must be measured, and what
would count as a pass, before FathomDB decides whether 0.8.28 ships a CUDA
memory pool on aarch64 Linux (Jetson/Tegra). Nothing here authorizes a product
change. Shipping any option needs its own 0.8.28 ruling.

## Authority and scope

- **Owner rulings, 2026-10-05** (`dev/plans/release-state-0.8.27.json`):
  `slice-110-early-cuinit-with-allocator-fallback` and
  `tegra-allocator-0.8.27-sync-fallback-pool-study-0.8.28`. 0.8.27 ships the
  aarch64-Linux synchronous allocation fallback together with early `cuInit`
  at Node addon load. No memory pool ships in 0.8.27. 0.8.28 evaluates the
  pool options below.
- **Tracking:** todos ledger `TC-1c70e523-38a0-4bb7-b238-0307d2b0489f`
  (seq 272, area `release/0.8.28`, p1). The revisit obligation for the
  existing workaround is `TC-9fef1b7c-4442-4c77-b925-992f338c9aac` (seq 270).
- **Evidence base.** Slice 110's committed evidence on
  `llm/slice110-tegra-allocator-fix`, under
  `dev/plans/runs/0.8.27-slice-110-tegra/`: `explicit-pool-experiment/`
  (`analysis.txt`, `cudarc-0.19.7-explicit-pool.patch`, `pool_lifecycle.c`),
  `early-cuinit-verification/` (including `experiment/E-cost-side-effects.txt`),
  `pool-teardown-evidence/results.md`, `driver-isolation-evidence/results.md`
  and `receipt.md`; plus `third_party/cudarc-0.19.7/FATHOMDB-PATCH.md` and
  `dev/tegra-platform-reference.md` §§ 7.5–7.8 on that branch. Until Slice
  110 integrates, read them with `git show <branch>:<path>`. Every number
  below comes from those files and was measured only on one Jetson AGX Orin
  64 GB (L4T R36.5.2, driver 540.5.0, CUDA 12.6).

## Owner rulings (2026-10-05)

The owner ruled on the protocol's open questions (protocol § 11) on
2026-10-05. They amend this plan as follows; the affected text below is
updated to match.

1. **Python import hook: yes.** The Tegra Python wheel gains an import-time
   `cuInit` and pool hook, behind the study's experiment feature, so Python
   runs the same variant set as Node (S, A-load-hold, A-load-release,
   A-first-use, B). It is built and measured from Phase 2 on; Phases 0 and 1
   do not wait for it.
2. **Private-pool variant: deferred**, and built only if it is the shape the
   cudarc maintainer would prefer. The upstream-preparation track assesses,
   from cudarc issues and maintainer comments (#536, #594, #544, #106, #174,
   #194), whether a private pool (`cuMemAllocFromPoolAsync`) or a
   set-current-pool shape (`cuDeviceSetMemPool`) is the more
   upstream-compatible one, and records that assessment.
3. **Goal: a working pool that recovers stream-ordered speed is the default
   on aarch64 Linux,** with the synchronous fallback kept as the safety net.
   The performance gate is framed as recovering default-pool (fast-path)
   speed, with S's synchronous path as the floor to beat. The decision table
   identifies which arm, if any, can be the default.
4. **Soak: 15 to 30 minute runs only** until the arms are tidy and shown
   correct, robust and fast. The 8 h and 24 h soaks are withdrawn and will be
   re-planned later.
5. **16 GiB exhaustion cell: not run.** Exhaustion probes stay at or below
   8 GiB `maxSize` while the trade-off is explained to the owner.

## What is already known

### The two failures

1. **`cuInit` needs a 4 GiB hole.** Unobstructed, `cuInit` makes one
   61.36 GiB reservation at `0x200000000`. It returns
   `CUDA_ERROR_OUT_OF_MEMORY` when no unmapped hole of at least 4 GiB is left
   in [8 GiB, 128 GiB). V8 on ARM64 Linux scatters heap pages through that
   window. Early `cuInit` at addon registration (0.8.27) fixes this for an
   import-first application; a late import into a large heap still fails.
2. **The default memory pool needs one contiguous 20960 MiB range** in the
   same window, either inside a large enough driver reservation or as a new
   lazily created `mmap` of 21978152960 bytes. 20960 MiB is about a third of
   device memory (`cuMemGetInfo` total / 3 rounded up to 32 MiB; inferred from
   one memory size). If no such range exists, `cuDeviceGetDefaultMemPool` and
   every `cuMemAllocAsync` return `CUDA_ERROR_OUT_OF_MEMORY` while
   `cuMemAlloc` works. A placement rule matched 2149 / 2149 C runs and
   63 / 63 Node runs. Early `cuInit` does not secure this range: the default
   pool can still `mmap` a new 20.47 GiB range at its first query, and in
   most measured Node processes it was unavailable even behind an early
   import (51 of 78, then 65 of 83, passing CUDA runs took the synchronous
   path).

### What 0.8.27 ships

The vendored `third_party/cudarc-0.19.7`, on aarch64 Linux only, uses
stream-ordered allocation only if the default pool query succeeds, otherwise
`cuMemAlloc`. The decision is made once per device per process and read by
every context constructor, so a buffer moved between wrappers is freed by the
API that allocated it. Zero-byte synchronous requests return null.

### Explicit-pool experiment (experiment-only patch, not shipped)

The experiment created one explicit pool per device per process
(`cuMemPoolCreate`, pinned device allocation type, `maxSize` default 3 GiB,
release threshold 0 by default), installed it with `cuDeviceSetMemPool`,
probed one 4-byte `cuMemAllocAsync`, and never destroyed it. It used that pool
only when the default pool was unavailable.

| Measurement (Node 25, passing runs, all heap/size/threshold configs pooled) | Default pool | Explicit pool | Synchronous |
| --- | --- | --- | --- |
| Runs | 15 | 50 | 25 |
| Open, median ms | 1393.7 | 1398.5 | 1372.1 |
| First embed, median ms | 46.1 | 43.3 | 56.2 |
| Steady embed, median ms (IQR) | 11.2 (10.4–14.4) | 12.3 (10.4–14.6) | 25.4 (24.9–27.1) |
| First rerank, median ms | 142.1 | 152.7 | 147.1 |
| Second rerank, median ms | 7.8 | 9.0 | 11.8 |

- Output identity: the embedding hash and rerank scores were identical in all
  98 passing runs across the three paths.
- Pool footprint at 3 GiB `maxSize`: used high 143.7 MiB, reserved high
  160.0 MiB in every run. Release threshold `max` against 0 at 3 GiB: steady
  embed 9.7 ms (n = 5) against 12.1 ms (n = 24).
- **Usable capacity inferred at about `maxSize`/3.** A 192 MiB pool reached
  63.5 MiB used and failed at open (2 / 2 runs that used it); a 384 MiB pool
  reached 128.0 MiB and failed at the first rerank (3 / 3). Two sizes only;
  the mechanism is unknown. Both failures were single large allocations
  (model weights at open, cross-encoder weights at the first rerank), so an
  alternative reading is that the remaining reservable space could not hold
  one large block, not that the pool caps at a third. The study measures
  capacity directly with fixed-size chunks and a single-allocation bisection
  (protocol § 6.5) rather than from where the product fails.
- **Exhaustion fails hard today.** Forced CUDA refused with the typed
  witness or `CudaProbeFailed` errors. Nothing fell back.
- **Large `maxSize` is exposed to fragmentation.** With a 400k-object Node
  heap and no early `cuInit`, a 36 GiB pool could not be created and the
  process fell back to synchronous allocation in 6 / 6 passing runs, while a
  3 GiB pool was created in 13 / 13 runs that got a context.
- In plain C, in a fragmented layout whose largest free gap was 14.66 GiB and
  whose driver units were 15.34 GiB, `cuMemPoolCreate` succeeded for
  `maxSize` up to 44 GiB and failed from 46 GiB; the control layout succeeded
  up to 48 GiB. `strace` showed no new large `mmap` when an explicit pool was
  created. **Inferred, not established:** an explicit pool needs about
  `maxSize`/3 of contiguous address space, carved from existing driver
  reservations rather than a new mapping. The study must verify this.
- **Creating the pool at load is costly, and the cost is mostly the retained
  context.** Import with `cuInit` only: median 31.3 ms, +13 MiB RSS. Import
  that also retains a primary context and keeps it: 74.8 ms, +112 MiB. Import
  that also creates a 3 GiB pool on that retained context: 92.4 ms,
  +121.5 MiB RSS (10 runs each, `E-cost-side-effects.txt` arms 1, 2 and 4).
  Releasing the context after creating the pool was not measured in the
  product; in plain C an explicit pool survived primary-context release and
  re-retain in 40 / 40 runs. The platform reference records no reliability
  gain from doing any of that at load in those runs.

### Lifetime

- The default pool, once obtained, survived primary-context release to
  refcount 0 and non-primary destroy in 100 / 100 C runs. An explicit pool
  survived release and re-retain, and destroy and re-create, in 40 / 40.
- Destroying an explicit pool while it is a device's current pool reverts the
  device to its default pool, which is the unavailable one here. The
  experiment patch relies on this as documented CUDA behaviour; the study
  verifies it on the device.
- Not measured: a co-resident library calling `cuDevicePrimaryCtxReset` or
  `cudaDeviceReset`; other Jetson models; non-Tegra aarch64 Linux CUDA hosts
  (which the cfg also covers).

## Options under study

| Arm | Definition |
| --- | --- |
| **S — baseline** | 0.8.27 behaviour: early `cuInit` at Node load; default pool if available, else synchronous `cuMemAlloc`. |
| **A — explicit pool** | One process-wide pool per device, created with a non-zero `maxSize`, installed with `cuDeviceSetMemPool`, never destroyed, used whether or not the default pool is available. Sub-arms by creation time: **A-load-hold** (at registration, on a primary context the hook retains and keeps), **A-load-release** (at registration, then the hook releases that context) and **A-first-use** (immediately before the first CUDA context). The A-load forms need a load-time hook: the Node addon has one, and the Python wheel gains one in Phase 2 (owner ruling 1). |
| **B — early `cuInit` + lazy pool** | Early `cuInit` at load as shipped. Before the first CUDA context, use the default pool if available; otherwise create the explicit pool then; otherwise synchronous. This is the experiment's `pool` mode. |

Two facts constrain the implementation of every pool arm:

- The shipped decision rule selects stream-ordered allocation only when
  `cuDeviceGetDefaultMemPool` succeeds. With an explicit pool installed that
  query still fails, so the rule must gain a third state: the device's
  current pool, when it is not the default, is probed with one 4-byte
  stream-ordered allocation. The decision becomes `default` / `explicit` /
  `synchronous`, still once per device per process.
- That decision is cached at the first context construction, so the pool
  must be installed **before** any cudarc `CudaContext` exists in the
  process. Candle constructs one inside `Device::new_cuda`
  (`candle_bge.rs`, `candle_reranker.rs`), so the policy runs ahead of every
  such call using result-level driver calls, never a safe context.
- A-first-use and B create the pool at the same moment with the same call;
  they differ only in preferring the default pool when it is available. Any
  fragmentation exposure of lazy creation (R5) applies to both; only the
  A-load forms avoid it.

A design question the study should also settle (proposal, not measured):
`cuDeviceSetMemPool` changes device-wide state that any co-resident library
using `cuMemAllocAsync` or `cudaMallocAsync` would then draw from, bounded by
FathomDB's `maxSize`. An alternative is to keep the pool private and allocate
with `cuMemAllocFromPoolAsync`, leaving the device's current pool alone. That
variant changes cudarc's allocation and free paths, not only a device-level
install, and is not the shape the upstream survey favours; it is **deferred**
(owner ruling 2: built only if it is the shape the cudarc maintainer would
prefer, which the upstream-preparation track assesses)
and built only if C7 or R7 shows the device-wide install harming a
co-resident consumer.

Each arm keeps the synchronous path as the last resort when pool creation or
its probe fails. Sizes to cover: `maxSize` 1, 3, 8 and 16 GiB, plus the
largest size that passes the robustness gates; release threshold 0 and `max`.
The Tegra Python wheel has no registration hook today. By the owner's ruling
(ruling 1) the study adds an experiment-gated import-time hook to it, so from
Phase 2 on Python runs the same variant set as Node.

The executable protocol is
`dev/plans/0.8.28/prework/tegra-cuda-memory-pool-experiment-protocol.md`;
where its method is more specific than a row below, the protocol governs.

## Correctness — what must be shown

Pass means zero failures in every row. With zero failures in N runs, the
95 % upper bound on the failure rate is about 3/N; each row's sample size
therefore bounds, and does not prove, its claim (50 runs: 6 %; 20 runs:
15 %). Since the Slice 110 experiment build, which path a process took has
been inferred from bimodal steady-embed latency; the study build reports the
decision directly (protocol § 2.3, § 6.4), and every row below reads it
rather than inferring it.

| # | Property | Method | Sample | Pass |
| --- | --- | --- | --- | --- |
| C1 | **Allocator provenance across wrappers and contexts.** The once-per-device decision now has three states (default pool, explicit pool, synchronous). | Vendored unit tests extended to all three states; device tests that create wrappers through `new`, `new_non_primary`, `new_cig` and `from_raw_context`, move buffers with `leak` / `upgrade_device_ptr`, and free them on another wrapper, including racing constructors. | Unit tests on every target; device tests 50 fresh processes per arm and size. | Every wrapper reports the same decision; every free uses the allocating API; no driver error, leak or crash. Known limit: upstream's own `from_raw_context` and non-primary-context GPU tests crash with `SIGSEGV` on the published 0.19.7 crate (`receipt.md`, "Red and green"), so those constructor paths may be testable only through pure tests on a stub; the results record which paths ran on the device. |
| C2 | **Pool lifetime.** The pool is never destroyed while installed; destroying it would revert to the unavailable default pool. | Code audit plus a C probe that destroys an installed pool in the failing layout and checks the device's current pool and the next `cuMemAllocAsync`. Product test: engine open/close/reopen cycles in one process. | C probe 20 runs; 50 processes × 100 cycles per arm. | No product path calls `cuMemPoolDestroy` on an installed pool; the C probe confirms the revert; all cycles allocate on the decided path. |
| C3 | **Exhaustion behaviour.** Capacity was about `maxSize`/3, and exhaustion fails hard. | Characterize capacity against `maxSize` (at least 6 sizes from 192 MiB to 16 GiB). Evaluate candidate policies: (i) size with measured headroom and keep a typed refusal; (ii) overflow into a second explicit pool, which keeps every pointer freeable with `cuMemFreeAsync`; (iii) per-allocation synchronous overflow, which needs per-pointer provenance and breaks the once-per-device invariant. | 10 processes per size; exhaustion driven deterministically by a test allocator. | Capacity model predicts the exhaustion point within 5 % at every size. The chosen policy never silently moves forced CUDA to CPU, keeps the typed error contract, and frees every pointer with the allocating API. Policy (iii) is accepted only with a provenance design that passes C1. |
| C4 | **cuBLAS / cuRAND workspaces.** Whether their workspaces come from the installed pool, the default pool or `cudaMalloc`. | Compare pool used/reserved counters with `cuMemGetInfo` deltas around rerank (cuBLAS-heavy) and any cuRAND use; run reranking at the smallest passing `maxSize` and in the fragmented layout. | 20 processes per arm. | The pool's share of the rerank's memory is measured from the pool counters; the residual (workspaces outside the pool) is bounded, not attributed, because `cuMemGetInfo` is a shared system-wide counter on this iGPU and cuBLAS resolves driver entry points internally; rerank passes at the chosen size in the fragmented layout. |
| C5 | **Zero-length buffers.** `cuMemAllocAsync(0)` returns null; `cuMemAlloc(0)` is invalid. | Existing zero-element tensor and `CudaStream::null()` tests on each arm. | Every arm and both bindings. | All pass; no null pointer reaches a free call. |
| C6 | **Multi-device.** The decision table and pool are per device. | Pure unit tests with two simulated devices; no multi-GPU Tegra exists. | Unit tests. | Decisions and pools never cross devices. The real multi-device case is declared unmeasured. |
| C7 | **After primary-context teardown, and co-resident resets.** | Repeat the pool-teardown probe with the arm's pool; add `cuDevicePrimaryCtxReset` and `cudaDeviceReset` from a co-resident library. | 20 C runs per case; 20 product processes with a co-resident reset. | Allocation keeps working on the decided path, or the failure is detected and refused with a typed error; never a wrong-API free. |
| C8 | **Python and Node parity.** The Tegra wheel has no early `cuInit` hook; the Node addon does. | Run C1–C5 through both bindings, installed from packed artifacts. | 20 processes per binding per arm. | Same decisions, same outputs (embedding hash and rerank scores identical to S), same error kinds. |

## Robustness — what must be shown

Robustness rows report pass rates with Wilson 95 % confidence intervals.
"Robust" means zero failures in at least 300 runs **pooled over a row**
(failure rate below about 1 % at 95 % confidence); a 30-run cell inside a
row bounds that cell at about 10 %, and the results state the bound each N
gives. Pass rate alone cannot discriminate the pool arms from S: behind an
early import S passes by falling back to the slow synchronous path (100 / 100
in Slice 110). The **discriminating endpoint** for R1–R5 is therefore the
fraction of passing processes on each allocator path (`default`, `explicit`,
`synchronous`), reported with intervals beside the pass rate; an arm is
better than S only at an equal pass rate with a higher stream-ordered
fraction.

| # | Property | Method | Sample | Pass |
| --- | --- | --- | --- | --- |
| R1 | **Fragmented layouts (synthetic).** | C harness with the arm's pool: the standard three blockers, the 200-seed random-18 series, the n-sweep and the free-gap layout. | 300+ runs per arm. | Arm A/B obtains a pool wherever S would have fallen back to synchronous, or falls back cleanly; no crash. |
| R2 | **Real Node heaps at several sizes.** | Import first, then grow to 0, 100k, 400k, 1M and 4M objects; Node 24, 25 and 26. | 30 processes per cell (450 per arm). | Zero failures; record the fraction of processes on each path. |
| R3 | **Heap growth during use.** | Open and embed, grow the heap, embed and rerank again, repeat. | 30 processes × 3 Node versions. | Zero failures; no path change after the first decision. |
| R4 | **Late import.** | Grow the heap first, then import, with and without `node --import fathomdb`. | 30 processes per heap size. | Same or better than S. A `cuInit` refusal remains typed and names the remedy. |
| R5 | **Lazy creation after heap growth (the key question for B and A-first-use, which create the pool at the same moment).** The default pool can `mmap` a new range after `cuInit`, so lazy creation is exposed to fragmentation; explicit pools are inferred to need about `maxSize`/3 contiguous, from one C layout. | Behind early `cuInit`, grow the heap to each R2 size, then create the pool at each `maxSize`; capture `/proc/self/maps` around creation (`strace` on a 3-run subset per cell). The contiguous need is measured separately as need(`maxSize`) against a controlled gap (protocol § 4.6, § 6.5). | 30 processes per heap × size cell; the bound is about 10 % per cell and 1 % over the pooled row. | The inferred `maxSize`/3 rule is confirmed or replaced by a measured rule that predicts success in all runs; B and A-first-use pass at the chosen size in every cell. |
| R6 | **Long-running soak.** | Continuous embed, rerank and ingest with periodic heap churn. | 15–30 min per arm, at least two processes (owner ruling 4; longer soaks re-planned later). | No allocator error; pool reserved memory and RSS stay within 10 % of their first-hour high; steady latency drift under 10 %. |
| R7 | **Concurrent processes on unified memory.** | 2, 4 and 8 processes on one Orin, each with its own pool, under embed and ingest load; release threshold 0 and `max`. The allocation witness is **off**: it reads a shared system-wide `cuMemGetInfo` counter and needs a sole GPU consumer, so it would fail as a harness artefact. A launcher aborts the trial if `MemAvailable` falls below 8 GiB. | 10 trials per count and threshold. | No process fails or is OOM-killed; the sum of reserved pool memory and system free memory are recorded; the chosen threshold leaves other processes, including CPU-only ones, their memory. |
| R8 | **Release-threshold effects on shared DRAM.** | Measure memory returned to the system after idle under each threshold. | 20 processes per threshold. | Threshold choice is justified by measured latency against held memory. |
| R9 | **Unmeasured Jetson models.** | Repeat R1, R2 and the performance core on other boards. | Minimum 100 runs per board for R2. | Required hardware: at least one 8 GB board (Orin Nano or Orin NX) and an AGX Orin 32 GB. Non-Tegra aarch64 Linux CUDA hosts (GH200, GB10, SBSA) are in the cfg; without access, they stay declared unmeasured, or the cfg is narrowed to measured Tegra. |

## Performance — what must be shown

Each arm runs on the same host with the GPU lock held, one process at a time
except in R7. Each cell: at least 20 fresh processes per Node version, and in
each process 5 warm-up and 50 timed iterations. The unit of analysis is the
process (one median per process; iterations within a process are
correlated). Report the median and IQR of the per-process medians and a
percentile-bootstrap 95 % confidence interval, resampling processes, for the
ratio against the reference arm. Arms are interleaved in randomised blocks,
not run as one block per arm, so thermal drift, GPU rail-gating and
page-cache state affect every arm alike. S is a mixture: in Slice 110's
unobstructed small-consumer runs the default pool was available in 1 / 10,
6 / 30 and 5 / 10 processes, the rest synchronous. "S" as a reference
therefore means S's synchronous-path processes; the default-pool reference
is S's default-pool processes; every table is also reported by allocator
path.

| # | Measure | Reference |
| --- | --- | --- |
| P1 | Import time, open time, RSS and virtual-size deltas | S |
| P2 | First embed and steady embed (current medians: default pool 11.2 ms, explicit pool 12.3 ms, synchronous 25.4 ms) | Default pool in an unobstructed layout, and S |
| P3 | First and steady rerank | Same |
| P4 | Embed batch sizes 1, 8, 32 and 128 | Same |
| P5 | Bulk ingest throughput (documents per second over at least 10,000 documents) | Same |
| P6 | Memory overhead: pool reserved high, `cuMemGetInfo` delta, RSS | S |
| P7 | Python wheel steady embed and ingest | Same |

Pass, per arm (owner ruling 3: the target is recovering default-pool,
fast-path speed; S's synchronous path is the floor to beat):

- steady embed and steady rerank within 1.15× of the default pool, with the
  95 % CI of the speed-up over S's synchronous path above 1.5×;
- first embed and ingest throughput no worse than S;
- import time no more than 5 ms above S, and RSS overhead no more than
  32 MiB above S. On the existing measurement (+61 ms, +110 MiB), A-load
  fails this unless its cost changes.

## Decision rule for 0.8.28

The owner's goal (ruling 3) is that a working pool which recovers
stream-ordered speed becomes the default on aarch64 Linux, with the
synchronous fallback as the safety net. The rule below picks the arm that can
be that default; the decision table names it, or names none.

- **Ship B** if C1–C8 pass, R1–R8 pass on the AGX Orin 64 GB, the
  performance gates pass, and R5 shows lazy creation succeeds at the chosen
  `maxSize` in every heap cell.
- **Ship A-load-release instead** if B and A-first-use fail only R5 (they
  create the pool at the same moment, so a lazy-creation failure hits both)
  and A-load-release passes everything, including the import-cost gate.
  **A-load-hold** is not a shipping candidate on the existing cost
  measurement (+61 ms, +110 MiB at import) unless that measurement changes.
- In either case the synchronous path stays as the last resort, gated as
  today to aarch64 Linux, and boards outside R9's measured set are declared
  unmeasured.
- **Keep the 0.8.27 synchronous fallback** if any correctness row fails, if no
  exhaustion policy passes C3, if robustness depends on the fragmentation
  layout in a way R5 cannot predict, or if the speed-up does not clear the
  performance gate.
- A partial pass is not shippable. Record it in the revisit ledger entry and
  the NVIDIA report instead.

## Where the fix can live: upstream cudarc or a FathomDB patch

The owner's question is whether FathomDB's pool implementation, and the
default-pool fallback that 0.8.27 already ships, can be contributed upstream to
cudarc (github.com/chelsea0x3b/cudarc, the repository crates.io lists), so the
vendored `third_party/cudarc-0.19.7` can be dropped, or whether they stay a
FathomDB patch. The study treats this as a question to answer, not an
assumption.

### Background: why the fix sits above the driver

Verified against the evidence:

- The stack allocates through the CUDA Driver API via cudarc
  (`cuDeviceGetDefaultMemPool`, `cuMemAllocAsync`, `cuMemAlloc`); see
  `FATHOMDB-PATCH.md` and the experiment patch.
- The C reproducer showed the Runtime API's `cudaMallocAsync` failing the
  same way, with `cudaErrorMemoryAllocation`, in 10 / 10 runs (control
  0 / 10). The default-pool failure therefore sits below the runtime, in the
  driver, whose address reservation the default pool depends on.
- The `cuInit` failure is also in the driver (§ 7.8 of the platform
  reference).
- The workaround uses public pool APIs (`cuMemPoolCreate`,
  `cuDeviceSetMemPool`); the Runtime API has counterparts
  (`cudaMemPoolCreate`, `cudaDeviceSetMemPool`). Only the driver forms were
  exercised.
- The CUDA runtime is statically linked into these artifacts through the
  pinned Candle fork's `candle-kernels` (root `Cargo.toml`; `1a131e780`), but
  allocation does not go through it.

Making the default pool degrade gracefully would be a driver change in
`libcuda`, so it is NVIDIA's to make. The channel is the NVIDIA Developer
Forums report drafted in
`driver-isolation-evidence/nvidia-report-draft.md`, which had not been posted
at the time of the evidence. Below the driver nothing can change; the library
that chooses how to allocate is cudarc, which is why both the shipped fallback
and any pool live there.

### Known about cudarc

From the Slice 110 evidence (`FATHOMDB-PATCH.md` and the filing notes in
`driver-isolation-evidence/results.md`), as of that work:

- cudarc 0.19.7 decides stream-ordered against synchronous allocation once per
  context wrapper, from `CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED` alone,
  with no fallback; the decision field is crate-private, so Candle cannot set
  it.
- The Slice 110 filing notes recorded 0.19.10 as the latest release, an open
  request for memory-pool wrappers, and no existing issue about this failure.
  The current upstream state is in the survey section below.
- FathomDB's patch note states the change is intended to be proposed upstream,
  and that the vendor copy is dropped once Candle's cudarc resolves to a
  release with an equivalent fallback.
- Root `[patch.crates-io]` entries do not propagate. A downstream Rust build of
  the published FathomDB crates with `embed-cuda` resolves unpatched cudarc and
  keeps the failure. Only an upstream release fixes that case.
- Dropping the vendor copy also needs the pinned Candle fork to move to that
  cudarc release (Candle uses cudarc 0.19.7; `ug-cuda` separately resolves
  cudarc 0.17.8 from crates.io).

These points come from the Slice 110 evidence; the survey below updates the
upstream state.

### Questions the study must answer

1. Which parts are upstreamable on their own: (a) the default-pool fallback
   with the once-per-device decision and zero-length handling, already
   shipped in 0.8.27; (b) a pool policy (explicit or lazy) as an opt-in or
   automatic behaviour; (c) only enabling APIs (for example pool wrappers or a
   public allocator-mode setter) with FathomDB keeping the policy.
2. Whether the process-wide decision table, which exists because buffers move
   between wrappers through `leak` / `upgrade_device_ptr`, is acceptable
   upstream, or whether upstream would carry provenance differently.
3. Whether the aarch64-Linux-only gating is acceptable upstream, or must
   become a runtime decision that is correct on every target.
4. What FathomDB keeps if upstream accepts only part: the residual patch, its
   pin-rot governance, and the cost of carrying it across cudarc upgrades.
5. The decision rule: drop the vendor copy only when a Candle-compatible
   cudarc release contains behaviour that passes C1–C8 and R1–R5 unchanged;
   otherwise keep a bounded FathomDB patch and record why.

Experiments that settle it: rerun the C1, C3 and R1 suites against an upstream
cudarc build with the proposed change applied; run the same suites on a
non-aarch64 CUDA host to show no off-target behaviour change; and confirm
whether a newer L4T/CUDA driver removes the need (rerun `minimal_repro.c` and
`pool_teardown.c`, per the revisit obligation).

### Upstream cudarc compatibility (style survey, 2026-10-05)

Source: a survey of the upstream repository, issues and pull requests,
recorded on 2026-10-05. Statements marked *inference* are this plan's reading,
not the maintainer's.

**Upstream state.**

- The repository is
  [chelsea0x3b/cudarc](https://github.com/chelsea0x3b/cudarc), with one
  maintainer (@chelsea0x3b). `main` equals 0.19.10 (2026-09-24). The README
  describes the crate as "Safe and minimal"; `lib.rs` asks users who need
  missing safe functionality to open a ticket.
- Pools exist only at the result (unsafe, thin) layer:
  `result::mem_pool::{create, destroy, trim_to, get_attribute,
  set_attribute, alloc_async}` and
  `result::device::{get_default_mem_pool, get_mem_pool, set_mem_pool}`
  ([chelsea0x3b/cudarc#544](https://github.com/chelsea0x3b/cudarc/issues/544),
  merged 2026-03), plus a public `has_async_alloc()` getter
  ([chelsea0x3b/cudarc#553](https://github.com/chelsea0x3b/cudarc/issues/553)).
  There is no safe `CudaMemPool`. The vendored 0.19.7 already contains the
  result-level functions (`third_party/cudarc-0.19.7/src/driver/result.rs`,
  `pub mod mem_pool` and the `device` pool functions), so a safe primitive
  can be built on them without touching the sys layer.
- [chelsea0x3b/cudarc#536](https://github.com/chelsea0x3b/cudarc/issues/536)
  (safe pool support) is open with no maintainer reply.
  [chelsea0x3b/cudarc#558](https://github.com/chelsea0x3b/cudarc/issues/558), a
  safe `CudaMemPool` with `alloc_from_pool`, was closed by its author.
  [chelsea0x3b/cudarc#594](https://github.com/chelsea0x3b/cudarc/issues/594), a
  capture-scoped graph pool, is labelled "needs redesign"; the maintainer
  wrote that "CudaSlice is almost turning into an enum at this point", asked
  to discuss further, and said "I definitely want to support memory pools".
- `CUmemPoolProps.maxSize` exists in the bindings only from the `cuda-12020`
  feature. Any API must compile across all 19 `cuda-*` features, on Windows
  and under the `no-std` clippy check. CI checks compilation only: no GPU
  tests and no aarch64 job.
- Releases come every two to eight weeks. Small additive pull requests can
  merge within a day; design-heavy ones take weeks.

**Upstream style.**

- Per-context decisions are stored as fields at construction. The maintainer
  proposed exactly that for the async/sync choice, "so we only have to run it
  once" ([chelsea0x3b/cudarc#174](https://github.com/chelsea0x3b/cudarc/issues/174), 2023).
- Non-sys source has no `Mutex`, `RwLock` or `thread_local!`; it uses
  `OnceLock` only for library and symbol loading, and atomics on
  `CudaContext`. There are no runtime environment-variable switches.
- No `target_arch` cfgs: "We don't have any device specific code in cudarc"
  ([chelsea0x3b/cudarc#194](https://github.com/chelsea0x3b/cudarc/issues/194)).
- Non-standard behaviour is opt-in, and runtime detection is preferred to
  build flags
  ([chelsea0x3b/cudarc#106](https://github.com/chelsea0x3b/cudarc/issues/106),
  #174).
- The maintainer declined synthesizing null pointers for zero-length
  allocations: "I'm hesitant to use a null pointer" (#194, 2024-01-09).
- Claimed problems need representative failing tests or benchmarks
  ([chelsea0x3b/cudarc#555](https://github.com/chelsea0x3b/cudarc/issues/555),
  [chelsea0x3b/cudarc#556](https://github.com/chelsea0x3b/cudarc/issues/556),
  [chelsea0x3b/cudarc#595](https://github.com/chelsea0x3b/cudarc/issues/595)).
  Breaking changes are acceptable when they clarify, marked `[Breaking]`.

**Fit of FathomDB's current patch** (*inference*).

- Conflicts with upstream style: the aarch64 cfg gating; the process-wide
  `Mutex` decision table; the automatic, environment-dependent downgrade inside
  every constructor; the zero-length null-pointer synthesis; and long,
  measurement-heavy doc comments.
- Aligns with upstream: releasing or destroying the context on constructor
  error paths, and returning unexpected errors instead of downgrading.

**Upstream-compatible shape** (*inference*). A small, explicit, opt-in safe
wrapper: a `CudaMemPool` handle in its own module, and a context method that
creates one with given properties and installs it as the device's current
pool. Existing `alloc`, `alloc_zeros` and `free_async` would then draw from it
unchanged, with no new `CudaSlice` variant. Avoid an `alloc_from_pool` that
returns `CudaSlice` until the ownership question in #594 and
[chelsea0x3b/cudarc#519](https://github.com/chelsea0x3b/cudarc/issues/519) is
settled. This favours the `cuDeviceSetMemPool` form of arms A and B over the
private `cuMemAllocFromPoolAsync` variant for anything proposed upstream; the
private variant would stay FathomDB-side if C7 or R7 require it.

**Consequence for 0.8.28** (*inference*). The work likely splits in two:

1. an upstreamable opt-in pool primitive in cudarc, gated on `cuda-12020`+
   for `maxSize` and compiling across every feature and platform;
2. FathomDB-side policy (when to install the pool, Tegra detection, the
   synchronous fallback, and exhaustion handling) in FathomDB code rather than
   in the vendored crate.

Whether the 0.8.27 default-pool fallback itself can be expressed with
upstream's existing API (`has_async_alloc()`, result-level pool queries) or
still needs a residual patch is part of question 1 above. The study
**prepares** an evidence package for a comment on #536 (the C reproducer, a
test in cudarc's `mod tests` style, a benchmark, and a draft comment;
protocol § 10) before any pull request is written. Nobody posts it without
the owner; if the owner posts, the maintainer's response is recorded as an
input to the decision rule.

## Execution notes

- Study code is experiment-only: a FathomDB-side policy module behind a cargo
  feature that no release feature set enables, selecting the variant from an
  environment variable, plus a small opt-in pool primitive and the
  three-state decision in the vendored cudarc (protocol § 2.3). The
  experiment build with the variant set to `S` is checked against the
  production build before anything else is measured (protocol § 5, Phase 0).
  It does not land on a release branch.
- Every series runs one fresh process at a time under the shared GPU lock,
  records the host, L4T, driver, Node and artifact hashes, and commits only
  pruned, path-redacted evidence, following Slice 110's retention practice.
  Memory-safety floors and stop conditions are in protocol § 1.
- Results go to a sibling results document
  (`dev/plans/runs/0.8.28-pool-study/results.md`) and a release-state
  ruling; this plan is updated only to correct its method.
