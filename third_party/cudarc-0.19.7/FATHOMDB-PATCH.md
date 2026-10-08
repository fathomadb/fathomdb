# Vendored cudarc 0.19.7 with a CUDA allocator fallback and a private memory pool

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
and `Cargo.toml`, and `src/driver/safe/mem_pool.rs` is new. Every change is
marked `FATHOMDB PATCH`. The complete diff against the published crate is
two patches in this directory, applied in order with `patch -p1` to the
published 0.19.7 package:

1. `fathomdb-alloc-fallback.patch`: items 1 to 4 (0.8.27);
2. `fathomdb-private-pool.patch`: items 5 and 6 and their tests (0.8.28).

Together they reproduce this tree.

### Item status

| Item | Upstream status | Removal path |
| --- | --- | --- |
| 1. Allocator fallback | Local only | Drop when Candle's cudarc resolves to a release with an equivalent fallback, or when no FathomDB artifact needs the default pool to be optional on aarch64 Linux. |
| 2. Per-device decision | Local only | Removed with item 1, which it serves. |
| 3. Zero-length synchronous allocation | Local only | Removed with item 1; only the synchronous fallback path needs it. |
| 4. Tests (`fathomdb_alloc_fallback`) | Local only | Removed with items 1 to 3. |
| 5. Private memory pool primitive | Proposed upstream, not posted | Drop when Candle's cudarc resolves to a release with `CudaMemPool` and `CudaContext::new_with_mem_pool`; switch FathomDB to the upstream names. The tests (`fathomdb_private_pool`) go with it. |
| 6. Feature marker `fathomdb-private-pool` | Local only | Removed with item 5; FathomDB stops enabling it in the same change. |

**Scope of items 1 to 4: aarch64 Linux only.** Items 1 to 3 apply only when the crate is
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
differences from items 1 to 4 are structural: helper functions and doc comments.
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

   Run the module with `scripts/tests/test_vendored_cudarc.sh`.
5. **Private memory pool primitive.** Opt-in and on every target, so it
   matches the upstream proposal; nothing changes for a caller that does not
   use it.
   - `CudaMemPool` (`mem_pool.rs`) wraps the result-layer `cuMemPool*` calls
     that upstream already has: `create(ordinal, &MemPoolProps { max_size,
     release_threshold })` makes a pinned device-memory pool, `attribute`
     reads a 64-bit pool attribute, `trim_to` and `raw` expose the driver's,
     and `Drop` calls `cuMemPoolDestroy`. The pool is never made the device's
     current pool.
   - `CudaContext::new_with_mem_pool(ordinal, Arc<CudaMemPool>)` retains the
     primary context as `new` does, after checking that the pool belongs to
     that device (`CUDA_ERROR_INVALID_VALUE` otherwise, with nothing
     retained). Its allocations use `cuMemAllocFromPoolAsync` on that pool;
     frees use the ordinary `cuMemFreeAsync`, which returns memory to the
     pool it came from, so `CudaSlice::drop` is unchanged.
   - It does not run the item 1 decision and does not read or record the
     item 2 table, so every other context of the device, including a
     co-resident cudarc user's, keeps its own allocator. The device's current
     pool is neither read nor changed.
   - `pub enum AllocMode { Default, Synchronous, Private }` and
     `CudaContext::alloc_mode()` report the allocator a context uses;
     `CudaContext::mem_pool()` returns its pool.
   - Zero-byte requests go to the driver like any other.
     `cuMemAllocFromPoolAsync(0)` succeeds with a null pointer that
     `cuMemFreeAsync` accepts, so item 3 is not needed here.
   - Lifetime: the context holds the pool, every stream holds the context
     and every slice holds its stream, so the pool is destroyed after the
     last of them.
   - Tests, in the `fathomdb_private_pool` module, run through the same
     script. A pure test covers the three allocator states. On a CUDA host
     the tests check:
     - allocation from the pool, with the device's current pool unchanged;
     - null zero-length buffers that free without error;
     - the pool released only after the context, then destroyed cleanly;
     - rejection of another device's pool;
     - on aarch64 Linux, in a fresh child process, that a private context
       leaves the item 2 table empty.
6. **Feature marker.** `Cargo.toml` declares the no-op feature
   `fathomdb-private-pool`. A dependent that enables it fails to resolve
   against an unpatched cudarc with "package `cudarc` does not have feature
   `fathomdb-private-pool`", not with a missing symbol.

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

Item 5 lets FathomDB keep stream-ordered allocation on such a device
without the default pool: a capped pool of its own needs a far smaller
contiguous range, and it is used only by FathomDB's contexts. The design is
in the FathomDB repository under
`dev/plans/0.8.28/features/slice-30/`.

Items 1 to 4 are not proposed upstream. Item 5 is intended to be; nothing
is posted without the FathomDB owner's sign-off. Drop this vendor copy once
the Candle cudarc resolves to a release that makes every item removable (see
the item status table).
