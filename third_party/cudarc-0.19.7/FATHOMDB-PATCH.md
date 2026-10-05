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

The only changed file is `src/driver/safe/core.rs`. Every change is marked
`FATHOMDB PATCH`. The complete diff against the published crate is
`fathomdb-alloc-fallback.patch` in this directory; applying it with
`patch -p1` to the published 0.19.7 package reproduces this tree.

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
   context they created. `from_raw_context` leaves ownership with the caller.
   There is no environment switch.
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
   The decision lasts for the process lifetime. A pool that has been obtained
   belongs to the device. A device that was refused could succeed later only if
   the address space has since freed a 20960 MiB hole; it then stays on the
   slower but correct synchronous path.

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
4. **Tests.** The `fathomdb_alloc_fallback` test module has two groups. Pure
   tests cover the pool-error classification, the once-per-device table
   (repeat, failed and concurrent decisions) and the zero-byte request. Tests
   on a CUDA host cover two wrappers of device 0 sharing the decision, a
   sync-allocating context's null zero-length buffers, and a data round trip.
   Run it with `scripts/tests/test_vendored_cudarc.sh`.

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

The cost is speed on the fallback path: with synchronous allocation a steady
embedding took about 25 ms against about 10 ms with stream-ordered
allocation on the same Orin. Processes whose default pool is available keep
the upstream behaviour.

The change is intended to be proposed upstream. Drop this vendor copy once
the Candle cudarc resolves to a release containing an equivalent fallback.
