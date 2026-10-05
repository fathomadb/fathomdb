---
title: Slice 110 Tegra CUDA allocator handoff
status: OPEN
target_release: 0.8.27
---

# Slice 110 Tegra CUDA allocator handoff

## Outcome (allocator fix, pending review)

The intermittent refusal is root-caused and fixed on this branch; the
sections after this one are the pre-fix handoff, kept as history.

- **Root cause.** The Orin driver reports memory-pool support, but the
  device's default pool needs one contiguous 20960 MiB range of process
  address space inside [8 GiB, 128 GiB). Node heaps fragment that window, so
  `cuDeviceGetDefaultMemPool` and `cuMemAllocAsync` return
  `CUDA_ERROR_OUT_OF_MEMORY` with about 53 GB free; `cuMemAlloc` works. cudarc
  0.19.7 chose stream-ordered allocation from the pool attribute alone.
- **Fix.** `third_party/cudarc-0.19.7` (vendored published crate, governed
  `[patch.crates-io]`) changes behaviour only in aarch64 Linux builds. Every
  other target compiles upstream's allocator logic. On aarch64 Linux it uses
  stream-ordered allocation only when the default pool can be obtained. Only pool-unavailable errors select the synchronous
  allocator, and others fail context creation. The decision is made once per
  device per process, so every context wrapper agrees. On the measured Orin a
  cached decision stays valid. The default pool survived context teardown
  with an explicit pool kept alive (40 / 40), never created (40 / 40) or
  destroyed first (20 / 20). A post-`cuInit` control detected a missing pool
  (10 / 10). Other Jetsons, non-Tegra aarch64 Linux CUDA hosts (Grace Hopper,
  GB10, SBSA) and a co-resident `cuDevicePrimaryCtxReset` or
  `cudaDeviceReset` are unmeasured. Zero-byte synchronous requests return
  null.
  See its `FATHOMDB-PATCH.md` and `fathomdb-alloc-fallback.patch`.
- **Tests.** `fathomdb-embedder` test `tegra_fragmented_va_cuda` reproduces
  the layout in-process and drives the real Candle probe (red 6 / 6, green
  11 / 11). After review fix 1 it also asserts, on integrated GPUs only, that
  the default pool is unavailable and that the context allocates
  synchronously (12 / 12). Since review fix 2 it asserts only on an integrated
  GPU with the AGX Orin 64 GB's memory and skips elsewhere (12 / 12 on the
  final code). `scripts/tests/test_vendored_cudarc.sh` runs the vendored unit
  tests (fast tier, 14 tests covering both the on-target and the off-target
  rule, after the tautological scope test was removed). Since the early-cuInit
  round, the regression test skips on any driver failure before `cuda:0` is
  identified.
- **Verification.** 100 / 100 fresh forced-CUDA open/embed/rerank/witness Node
  processes passed (Node 25, 24 and 26; in-tree and installed package);
  forced CPU passed 6 / 6. After review fix 1, a rebuilt artifact passed
  10 / 10 in-tree and 10 / 10 installed on Node 25, with forced CPU 3 / 3 in
  each form. After review fix 2 and the aarch64-only gating, the same
  10 + 10 / 3 + 3 check passed again. The rebuilt Tegra Python wheel passed
  10 / 10 (5 normal, 5 fragmented). Counts, hashes and the Python wheel result
  are in
  the [receipt](../../../runs/0.8.27-slice-110-tegra/receipt.md#allocator-root-cause-and-fix).
- **Early `cuInit` (Node, aarch64 Linux CUDA builds).** `cuInit` needs a
  4 GiB hole in the same window and failed once a JavaScript heap of roughly
  70 MiB or more had grown first. The addon now calls `cuInit` from a
  shared-library constructor when it is loaded
  (`src/rust/crates/fathomdb-napi/src/cuda_early_init.rs`). It is skipped
  only when both device policies are `cpu`. A forced-CUDA refusal after an
  out-of-memory `cuInit` names the cause and the remedy (import first, or
  `node --import fathomdb`). `scripts/tests/test_tegra_node_early_cuinit.sh`
  failed 3 / 3 before and passed 5 / 5 after. Behind an early import,
  50 / 50 heap-heavy processes passed. A late import was refused 5 / 5 with
  the new message, and `--import` rescued it 8 / 8. Cost: about +11 ms per
  import and about 61.5 GiB of reserved virtual address space. See the
  [receipt](../../../runs/0.8.27-slice-110-tegra/receipt.md#early-cuinit-in-the-node-addon)
  and the user section in `docs/embedder.md`.
- **Still open.** A late import into a large heap still fails. The
  synchronous path is about 1.8–2.4 times slower per steady embed (one
  definition, in the receipt). Only the AGX Orin 64 GB was measured. Slice 110
  stays IN_PROGRESS until independent review. Do not change release state
  before then.

> **Revisit obligation (user-directed).** The aarch64-Linux-only cudarc
> allocator workaround, and the early `cuInit` added in this slice,
> must be revisited at the **next micro release** and **loudly at the next
> minor release**. Check four things: whether NVIDIA has fixed the
> default-pool/`cuInit` address-space behaviour in a newer L4T/CUDA; whether
> upstream cudarc gained a fallback or pool API, so the vendored copy can be
> dropped; whether V8/Node changed the ARM64 mmap hint mask; and the planned
> explicit-pool work to recover stream-ordered speed. Tracked as todos-ledger
> `TC-9fef1b7c-4442-4c77-b925-992f338c9aac` (seq 270).

## Pre-fix handoff

This note is for the local agent continuing Slice 110. The work branch
`llm/slice110-tegra-allocator-fix` starts from `release/0.8.27` at
`1a6cd4938017f2cd3a5a05b8692bf40fd09fe86f`. Verify the actual branch and
HEAD before editing. The reviewed Slice 110 product code is `87670f61d`; later
release commits through this base contain qualification and documentation.
Slice 110 remains **IN_PROGRESS** only because the installed Jetson Node
forced-CUDA runtime row is intermittent. Do not advance release state or
represent one successful run as qualification.

### What is proven

- The NAPI decomposition and subscriber contract passed independent review,
  local `agent-verify` (182/182 suites), and exact installed package checks on
  Linux x64, Windows x64, Linux x64 CUDA/reranker, hosted Linux arm64 GNU, and
  both macOS targets. See [status](status.md) for their hashes and limits.
- On the Jetson AGX Orin, fresh CPU and CUDA-capable Node package pairs build,
  install, preserve the 17/44 production runtime surface and TypeScript
  declarations, and pass CPU embedding/reranking and subscriber checks. Some
  forced-CUDA runs complete a real 384-dimensional embed, rerank scores, and
  the in-process GPU allocation witness. The runtime is nevertheless
  intermittent: before reboot Node 24 passed 5/10 and Node 25 passed 3/10;
  after reboot Node 25 passed 1/5 full runs and 3/10 open-only runs.
- After Memex CI run
  [37235196745](https://github.com/coreyt/memex/actions/runs/37235196745)
  finished, a controlled repeat with no Memex worker or named NVIDIA compute
  process passed only 3/5 clean Node 25 forced-CUDA open/embed/rerank/witness
  runs. The exact installed `.node` SHA-256 was
  `929f7dbfa950c1ba8fbb3760b365dfa9b33e6887696e9ef9d984f9542df8ff80`.
- The public failure is typed `FDB_EMBED_DEVICE_POLICY`,
  `kind: cuda_probe_failed`. Temporary instrumentation found that
  `Device::new_cuda(0)` succeeds and the first
  `Tensor::zeros(1, DType::F32, &device)` returns
  `DriverError(CUDA_ERROR_OUT_OF_MEMORY, "out of memory")`. This happens before
  model loading, database open, reranking, or GPU witness work. Direct NAPI
  module loading passed 10/10.
- A valid context-retaining CUDA Driver API probe measured
  `53,818,724,352` then `53,831,561,216` free bytes out of
  `65,879,896,064` total across a failed open. System `MemAvailable` and
  `CmaFree` were steady, and no named CUDA process or accessible GPU-device
  file descriptor identified another memory owner. The ordinary 4-byte
  `malloc_sync`/`free_sync` path succeeded in the **same Node process and
  context** on five failures, while `CudaStream::alloc_zeros::<u8>(4)` returned
  the same OOM. This is evidence against total device capacity exhaustion or
  another visible workload; it narrows the fault to the stream allocation/zero
  path. It does **not** yet distinguish `malloc_async` from the following
  asynchronous memset.
- Standalone Candle minimal probes previously passed 10/10 on the main thread
  and 10/10 on a spawned Rust thread; a prior Python wheel passed 5/5. A V8
  heap limit comparison did not improve Node pass rate. No retry, CPU fallback,
  or production allocator change has been made.

### Evidence and next experiment

The [Tegra receipt](../../../runs/0.8.27-slice-110-tegra/receipt.md) binds
source, toolchain, artifacts and all attempt counts. The retained
[idle-host logs](../../../runs/0.8.27-slice-110-tegra/idle-host-evidence/)
contain the five-run consumer check, same-process allocator results, valid
CUDA free-memory probe source/output, and package hashes. Earlier diagnostic
logs and hashes are in the receipt. The temporary Jetson build directory was
removed after these logs were preserved; diagnostic source and installed
binary hashes were restored before cleanup.

The next narrow experiment is to separate `CudaStream::alloc::<u8>(4)` from
`memset_zeros` in the **same installed Node process** and record the exact
failing call. The previous temporary split did not run: it triggered a cold
CUDA rebuild, so only that agent-owned diagnostic process group was stopped.
The first probe is in
`src/rust/crates/fathomdb-embedder/src/candle_bge.rs` (`CandleCudaProvider`);
NAPI `Engine.open` hands it to Tokio's blocking thread in
`src/rust/crates/fathomdb-napi/src/engine.rs`. Pinned CUDARC 0.19.7 selects
stream-ordered allocation when the device reports memory-pool support. Compare
the last known working **Node** package and its Candle/CUDARC/Jetson stack if
one exists; the passing historical Python wheel does not qualify Node.
Use an isolated build on the Jetson after checking it is not running other CI
work. Its local toolkit needs `/usr/local/cuda-12.6/bin` on `PATH` in addition
to `CUDA_PATH=/usr/local/cuda-12.6` and `CUDA_COMPUTE_CAP=87`; the tested
feature set is `embed-cuda,rerank-cuda`, Node 25.9.0/npm 11.12.1, CUDA 12.6.68.
Disk was ample on the Jetson but only single-digit GiB remained free on
windchill3 after the branch's pre-push checks. Recheck capacity before builds
and avoid a second large local target directory.

If a repair is justified, write a failing test first, preserve the accepted
forced-CUDA refusal contract, and prove the real model and allocation-witness
paths on the installed Node package. Check the blast radius for Python and
non-Tegra CUDA because the suspected CUDARC/Candle path is shared. Obtain the
requested independent code review and Terra verification before changing
Slice 110 status to complete. Do not infer that a synchronous 4-byte probe
alone qualifies the model path.
