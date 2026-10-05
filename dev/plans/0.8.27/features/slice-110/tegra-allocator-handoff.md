---
title: Slice 110 Tegra CUDA allocator handoff
status: OPEN
target_release: 0.8.27
---

# Slice 110 Tegra CUDA allocator handoff

This note is for the local agent continuing Slice 110. The work branch
`llm/slice110-tegra-allocator-fix` starts from `release/0.8.27` at
`1a6cd4938017f2cd3a5a05b8692bf40fd09fe86f`. Verify the actual branch and
HEAD before editing. The reviewed Slice 110 product code is `87670f61d`; later
release commits through this base contain qualification and documentation.
Slice 110 remains **IN_PROGRESS** only because the installed Jetson Node
forced-CUDA runtime row is intermittent. Do not advance release state or
represent one successful run as qualification.

## What is proven

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

## Evidence and next experiment

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
