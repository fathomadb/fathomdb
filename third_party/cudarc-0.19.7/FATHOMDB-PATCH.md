# Vendored cudarc 0.19.7 with a CUDA allocator fallback

This is the published `cudarc` 0.19.7 crate, extracted from the crates.io
package whose `Cargo.lock` checksum is
`1cea5f10a99e025c1b44ae2354c2d8326b25ddbd0baf76bde8e55cfd4018a2cc`
(upstream commit `3e5d38b5fe5ec81c934bdc2c7207f181772e307d`, recorded in
`.cargo_vcs_info.json`). Only the package's git-ignored `Cargo.lock` was
omitted. The root `[patch.crates-io]` entry routes Candle's cudarc 0.19.7 to
this copy; `ug-cuda`'s cudarc 0.17.8 still resolves from crates.io. The
licenses are the upstream `LICENSE-MIT` and `LICENSE-APACHE`.

## Delta from upstream

The changed files are `src/driver/safe/core.rs`, `src/driver/safe/mod.rs`
and the new `src/driver/safe/mem_pool.rs`. Every change is marked
`FATHOMDB PATCH`. The complete diff against the published crate is
`fathomdb-alloc-fallback.patch` in this directory; applying it with
`patch -p1` to the published 0.19.7 package reproduces this tree.

**Scope: aarch64 Linux only.** Items 1 to 3 apply only when the crate is
built for `cfg(all(target_os = "linux", target_arch = "aarch64"))`, the
platform of the measured Jetson failure. That predicate also covers non-Tegra
aarch64 Linux CUDA hosts, such as Grace Hopper, GB10 and other SBSA servers.
They are in scope of the cfg, but nothing here was measured on them. The
FathomDB artifacts that compile this crate for aarch64 Linux are Tegra
(Jetson) builds. On every other target the crate behaves as published 0.19.7:

- each context chooses stream-ordered allocation from
  `CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED` alone;
- the default pool is never queried, and there is no process-wide decision
  table;
- constructors return only upstream's errors, with upstream's (absent)
  cleanup;
- zero-byte synchronous requests still reach `cuMemAlloc`, and every
  synchronous pointer is still passed to `cuMemFree`.

The zero-length handling is gated as well. Off target the synchronous path
runs only on devices without memory pools, and there it would turn
upstream's `CUDA_ERROR_INVALID_VALUE` into success. That is a behaviour
change, and nothing has measured it there. Off target the only remaining
differences are structural: helper functions and doc comments.
`select_async_alloc` has an upstream-equivalent variant, and the
`SYNC_FALLBACK` constant is `false`. The off-target branch has not been
compiled for a real non-aarch64 target on the Jetson used for this work. It
was compiled, clippy-checked and unit-tested there from a copy with the cfg
predicate flipped. A real x86_64 build is left to CI. The off-target
constructor paths (`CudaContext::new` and the other constructors calling the
upstream rule on a live device) are exercised only by a real off-target build
running the GPU tests on a CUDA host; the flipped copy and the pure tests do
not cover them.

1. **Allocator selection.** Upstream decides once per context to use
   stream-ordered allocation (`cuMemAllocAsync`) whenever
   `CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED` is positive. The patch also
   requires that `cuDeviceGetDefaultMemPool` succeeds, with the new context
   current. Only two results of that query select `cuMemAlloc` instead:
   `CUDA_ERROR_OUT_OF_MEMORY` (the measured Tegra failure) and
   `CUDA_ERROR_NOT_SUPPORTED` (the documented answer of a device without
   pools; it is included because stream-ordered allocation cannot work on
   such a device). Any other error is returned from the constructor rather
   than downgrading a healthy device. That includes a sticky error left by
   earlier asynchronous work. On that error path `new` releases the primary
   context it retained, and `new_non_primary` and `new_cig` destroy the
   context they created. `from_raw_context` returns that error before its
   wrapper exists, so ownership stays with the caller. A later
   `bind_to_thread` failure in any constructor still drops the new wrapper,
   which releases or destroys the context. That is upstream's behaviour, which
   the patch does not change. There is no environment switch.
2. **One decision per device per process.** Upstream stores the decision per
   context wrapper. `CudaContext::new` may wrap the same primary context many
   times. Buffers also move between wrappers through `CudaSlice::leak` and
   `CudaStream::upgrade_device_ptr`, and they are freed by whichever wrapper
   owns them last. Wrappers that chose differently could therefore pass a
   `cuMemAlloc` buffer to `cuMemFreeAsync`. The patch records the first
   successful decision for each device in a process-wide table. All four
   constructors (`new`, `new_non_primary`, `new_cig`, `from_raw_context`) read
   it, including wrappers of foreign contexts, because the default pool
   belongs to the device rather than to a context. The table lock is held
   while deciding, so racing constructors agree. A failed decision is not
   recorded, so the next constructor probes again. `has_async_alloc` keeps the
   per-wrapper copy that `CudaStream::null`, `CudaStream::alloc` and
   `CudaSlice::drop` read, and all copies for one device are now equal.
   The decision lasts for the process lifetime. A cached "stream-ordered"
   decision relies on the default pool outliving the contexts that use it.
   That was measured only on the Jetson AGX Orin 64 GB (L4T R36, CUDA 12.6),
   in plain C and fresh processes. A primary context was released to
   refcount 0, or a non-primary context destroyed, in both an unobstructed
   layout and one where the pool had to `mmap` a new 20.47 GiB range into a
   hole. The pool survived in 40 / 40 runs with an explicit pool kept alive,
   40 / 40 with no explicit pool ever created and 20 / 20 with the explicit
   pool destroyed before teardown. The driver unmapped nothing in the window.
   After every remaining hole was blocked, re-retaining or re-creating the
   context returned the same pool handle, and `cuMemAllocAsync` succeeded. In
   a control, blocking every hole after `cuInit` but before the first pool
   query made the pool unavailable in 10 / 10 runs, so the probe does detect
   a missing pool. Not measured: a co-resident library calling
   `cuDevicePrimaryCtxReset` or `cudaDeviceReset` (neither cudarc nor Candle
   does), and other Jetson models. The probe and its results are in the
   FathomDB repository under
   `dev/plans/runs/0.8.27-slice-110-tegra/pool-teardown-evidence/`. A cached
   "synchronous" decision is conservative: if the address space later frees a
   20960 MiB hole, the device stays on the slower but correct synchronous
   path.

   The leaked handle does not carry allocator provenance. `leak` returns a
   bare `CUdeviceptr`, and recording provenance would mean changing that public
   type or keeping a side table keyed by pointer. With one decision per device,
   every pointer that cudarc allocated on a device came from that device's only
   allocator. Upgrading it on any wrapper of the same device therefore frees it
   correctly. Two cases remain outside this guarantee. Upgrading onto another
   device was already invalid upstream, because it uses the wrong context.
   Pointers from a foreign allocator were already the caller's responsibility
   upstream: a foreign `cuMemAlloc` pointer upgraded onto a pool-capable
   context reached `cuMemFreeAsync`. The safety section of
   `upgrade_device_ptr` now states that requirement explicitly.
3. **Zero-length synchronous allocation.** `cuMemAlloc(0)` returns
   `CUDA_ERROR_INVALID_VALUE`, whereas `cuMemAllocAsync(0)` succeeds with a
   null pointer. Synchronous zero-byte requests now return a null pointer
   without a driver call, and `CudaSlice::drop` does not pass a null pointer
   to `cuMemFree`. Without this, `CudaStream::null()` and zero-element tensors
   would fail whenever the fallback is active.
4. **Tests.** The `fathomdb_alloc_fallback` test module has two groups.
   - Pure tests run on every target. They cover both rules: off target, the
     upstream selection whatever the pool query would return, and upstream
     zero-length handling. On target, the pool-error classification and the
     null zero-length rule. They also cover the once-per-device table
     (repeat, failed and concurrent decisions).
   - Tests on a CUDA host check that the selection matches the build's rule.
     On aarch64 Linux only, they also check that two wrappers of device 0 share
     the decision, the zero-byte request, and a sync-allocating context's null
     zero-length buffers; a data round trip runs everywhere.

   No test compares `SYNC_FALLBACK` with its own defining `cfg!` expression;
   which rule a build applies is shown by the tests above running on that
   build's target.

   Run the modules with `scripts/tests/test_vendored_cudarc.sh`.
5. **Explicit memory pool (0.8.28 pool-study branch only).** This item exists
   on the `llm/0.8.28-tegra-pool-study` experiment branch, which is never
   merged to a release branch; it prototypes the opt-in primitive the 0.8.28
   study plan proposes for upstream
   (`dev/plans/0.8.28/prework/tegra-cuda-memory-pool-experiment-protocol.md`,
   section 2.3). How to shape that primitive for upstream, and
   what stays in FathomDB, is in
   `dev/plans/0.8.28/prework/cudarc-upstream-patch-notes.md` (preparation
   only; nothing is posted without the owner).
   - `mem_pool.rs` adds `CudaMemPool` (`create(ordinal, &MemPoolProps)`,
     `install`, `attribute`, `trim_to`, `raw`) and `MemPoolProps
     { max_size, release_threshold }`: pinned device memory, no handle types,
     `maxSize` set only where the bindings have it (`cuda-12020` and later).
     `Drop` destroys the pool; destroying an installed pool reverts the device
     to its default pool, so a user keeps an installed pool alive.
   - The allocator decision gains a third state. `CudaContext::alloc_mode()`
     returns `AllocMode::{Default, Explicit, Sync}`; `has_async_alloc()` is
     `alloc_mode() != Sync`. On aarch64 Linux, if a pool was installed on the
     device through `CudaMemPool::install` in this process, the decision first
     checks that it is still the device's current pool and that a 4-byte
     `cuMemAllocAsync` / `cuMemFreeAsync` / `cuCtxSynchronize` on the null
     stream succeeds; then it is `Explicit` and the default pool is never
     queried. Otherwise the rule of item 1 applies unchanged. The record of
     installed pools is a process-wide table read under its lock, so a
     process that never installs a pool makes exactly the driver calls of
     item 1. Off aarch64 Linux the rule is upstream's: `Default` for a
     pool-capable device, else `Sync`.
   - The process-wide table of item 2 stores the three-state decision.
   - No environment variable, cfg or output is added; the crate does not know
     about the study. Pure tests cover the rule and the installed-pool table;
     device tests (aarch64 Linux) check that an installed pool is decided
     `Explicit`, serves allocations, and that dropping it reverts the device's
     current pool.
6. **Private-pool context (0.8.28 pool-study branch only; the shape the study
   proposes upstream).** `CudaContext::new_with_mem_pool(ordinal,
   Arc<CudaMemPool>)` builds a context on the device's primary context that
   allocates from its own pool with `cuMemAllocFromPoolAsync` and never reads
   or changes the device's current pool. Its `alloc_mode()` is
   `AllocMode::Private` and `has_async_alloc()` is true, so `CudaSlice::drop`
   frees with `cuMemFreeAsync` as for any pool; no `CudaSlice` variant is
   added. The pool is held by the context (`mem_pool()`), which every slice
   keeps alive through its stream. The constructor runs neither the device
   decision nor the process-wide table of item 2; a pool of another device
   is rejected with `CUDA_ERROR_INVALID_VALUE`. Zero-length requests go to
   the driver unchanged: `cuMemAllocFromPoolAsync(0)` returns a null pointer
   on the measured Orin (pool-study C5, 60 / 60). The `upgrade_device_ptr`
   safety note says a private-pool pointer may move only into a
   stream-ordered context. Pure test: a pool context is `Private` without a
   device decision. Device tests: allocations (including zero-length) come
   from the pool, memory returns at synchronization with threshold 0, the
   current pool is unchanged, and a full pool returns a typed
   `CUDA_ERROR_OUT_OF_MEMORY` and recovers.

## Why

On a Jetson AGX Orin 64 GB (L4T R36, CUDA 12.6, driver 540.5.0) the driver
reports memory-pool support, but the default pool needs one contiguous range
of 20960 MiB of the process address space inside [8 GiB, 128 GiB). Node/V8
scatter heap pages through that window, so `cuDeviceGetDefaultMemPool` and
every `cuMemAllocAsync` return `CUDA_ERROR_OUT_OF_MEMORY` with more than
50 GB free. Synchronous `cuMemAlloc` works in every layout tested. Upstream
cudarc has no fallback, and the field that holds the decision is
crate-private, so Candle cannot select it. A C reproducer and measurements
are in the FathomDB repository under
`dev/plans/runs/0.8.27-slice-110-tegra/driver-isolation-evidence/`.

The cost is speed on the fallback path: a steady FathomDB embedding is about
1.8–2.8 times slower with synchronous allocation on the same Orin. For each
Node series with runs on both paths, that is the median steady embedding of
its synchronous runs over that of its stream-ordered runs (synchronous
24.7–28.2 ms, stream-ordered 9.2–14.6 ms). On aarch64 Linux, processes whose
default pool is available keep the upstream allocator. Other targets are
unaffected.

FathomDB must revisit this workaround at its next micro release and loudly at
its next minor release. The obligation is recorded in
`dev/todos-and-considerations-ledger.jsonl`.

The change is intended to be proposed upstream. Drop this vendor copy once
the Candle cudarc resolves to a release containing an equivalent fallback.
