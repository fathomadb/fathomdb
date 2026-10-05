# Jetson AGX Orin 64GB (L4T R36.5.2): `cuMemAllocAsync` / default memory pool returns CUDA_ERROR_OUT_OF_MEMORY when three 4 KiB mappings exist below 128 GiB

## Summary

On a Jetson AGX Orin 64GB, `cuDeviceGetDefaultMemPool` and `cuMemAllocAsync` (and `cudaMallocAsync`) return
`CUDA_ERROR_OUT_OF_MEMORY` for a 4-byte allocation with about 53 GB free. This happens whenever the process
already has a few small, unrelated mappings in the address range [8 GiB, 128 GiB) before `cuInit`.
Synchronous `cuMemAlloc` works in the same process.

The reproducer below is a 60-line C program against `libcuda` only. It places three 4 KiB `PROT_NONE` pages at
38, 68 and 98 GiB, and fails in 20 of 20 runs; without the pages it passes in 20 of 20.

From `strace` and `/proc/self/maps`:
- The driver places its GPU VA reservation (the size of device memory) inside [8 GiB, 128 GiB). When that range is fragmented, it splits the reservation into smaller pieces.
- The default pool needs one contiguous range of exactly 20960 MiB of GPU VA. It gives up with `CUDA_ERROR_OUT_OF_MEMORY` when no piece or unmapped hole that large exists, even though physical memory is plentiful.

Any host process that maps memory in that window can therefore lose stream-ordered allocation, intermittently and depending on its own address-space layout.

## Environment

| Item | Value |
|---|---|
| Board | NVIDIA Jetson AGX Orin Developer Kit, 64GB (`/proc/device-tree/model`) |
| L4T | `# R36 (release), REVISION: 5.2, GCID: 46426093, BOARD: generic, EABI: aarch64, DATE: Thu Jul 16 18:56:22 UTC 2026` (`nvidia-l4t-core` / `nvidia-l4t-cuda` 36.5.2-20260716114719) |
| Kernel | 5.15.199-tegra (OOT kernel variant), Ubuntu 22.04.5 LTS, glibc 2.35, 4 KiB pages |
| Kernel module | `NVRM version: NVIDIA UNIX Open Kernel Module for aarch64 540.5.0 Release Build` |
| CUDA | toolkit 12.6.68 (`nvcc` V12.6.68), `cuDriverGetVersion` = 12060 |
| Memory | `cuMemGetInfo` total = 65879896064 B, free ≈ 53.4 GB; `MemTotal` 64335836 kB |
| Power mode | MAXN |
| Other | `randomize_va_space` = 2, `overcommit_memory` = 0, `ulimit -v` unlimited, no other GPU process running during the runs |

## Minimal reproduction

```c
/* minimal_repro.c
 * Build: gcc -O2 -Wall -I/usr/local/cuda/include minimal_repro.c -o minimal_repro -lcuda
 * Run:   ./minimal_repro          # with the three pages -> async alloc fails
 *        ./minimal_repro control  # without them         -> async alloc succeeds
 */
#define _GNU_SOURCE
#include <cuda.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>

#ifndef MAP_FIXED_NOREPLACE
#define MAP_FIXED_NOREPLACE 0x100000
#endif

static void check(const char *what, CUresult r) {
    const char *name = "?";
    cuGetErrorName(r, &name);
    printf("%-28s -> %d (%s)\n", what, (int)r, name);
}

int main(int argc, char **argv) {
    if (!(argc > 1 && strcmp(argv[1], "control") == 0)) {
        const unsigned long long gib = 1ULL << 30;
        const unsigned long long at[3] = {38 * gib, 68 * gib, 98 * gib};
        for (int i = 0; i < 3; i++) {
            void *p = mmap((void *)at[i], 4096, PROT_NONE,
                           MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
            printf("mmap 4 KiB PROT_NONE at %#llx -> %p\n", at[i], p);
        }
    }

    CUdevice dev;
    CUcontext ctx;
    CUmemoryPool pool;
    CUdeviceptr p;
    check("cuInit", cuInit(0));
    check("cuDeviceGet", cuDeviceGet(&dev, 0));
    check("cuDevicePrimaryCtxRetain", cuDevicePrimaryCtxRetain(&ctx, dev));
    check("cuCtxSetCurrent", cuCtxSetCurrent(ctx));

    size_t free_b, total_b;
    check("cuMemGetInfo", cuMemGetInfo(&free_b, &total_b));
    printf("  free=%zu total=%zu\n", free_b, total_b);

    check("cuMemAlloc(4)", cuMemAlloc(&p, 4));
    check("cuMemFree", cuMemFree(p));
    check("cuDeviceGetDefaultMemPool", cuDeviceGetDefaultMemPool(&pool, dev));
    CUresult r = cuMemAllocAsync(&p, 4, NULL);
    check("cuMemAllocAsync(4)", r);
    if (r == CUDA_SUCCESS) {
        check("cuMemFreeAsync", cuMemFreeAsync(p, NULL));
        check("cuCtxSynchronize", cuCtxSynchronize());
    }
    cuDevicePrimaryCtxRelease(dev);
    return r == CUDA_SUCCESS ? 0 : 1;
}
```

### Actual output (with the three pages)

```text
mmap 4 KiB PROT_NONE at 0x980000000 -> 0x980000000
mmap 4 KiB PROT_NONE at 0x1100000000 -> 0x1100000000
mmap 4 KiB PROT_NONE at 0x1880000000 -> 0x1880000000
cuInit                       -> 0 (CUDA_SUCCESS)
cuDeviceGet                  -> 0 (CUDA_SUCCESS)
cuDevicePrimaryCtxRetain     -> 0 (CUDA_SUCCESS)
cuCtxSetCurrent              -> 0 (CUDA_SUCCESS)
cuMemGetInfo                 -> 0 (CUDA_SUCCESS)
  free=53278990336 total=65879896064
cuMemAlloc(4)                -> 0 (CUDA_SUCCESS)
cuMemFree                    -> 0 (CUDA_SUCCESS)
cuDeviceGetDefaultMemPool    -> 2 (CUDA_ERROR_OUT_OF_MEMORY)
cuMemAllocAsync(4)           -> 2 (CUDA_ERROR_OUT_OF_MEMORY)
```

### Control output (`./minimal_repro control`)

```text
cuInit                       -> 0 (CUDA_SUCCESS)
...
cuMemAlloc(4)                -> 0 (CUDA_SUCCESS)
cuMemFree                    -> 0 (CUDA_SUCCESS)
cuDeviceGetDefaultMemPool    -> 0 (CUDA_SUCCESS)
cuMemAllocAsync(4)           -> 0 (CUDA_SUCCESS)
cuMemFreeAsync               -> 0 (CUDA_SUCCESS)
cuCtxSynchronize             -> 0 (CUDA_SUCCESS)
```

The same happens through the Runtime API: `cudaMallocAsync(&p, 4, 0)` returns `cudaErrorMemoryAllocation` in 10 of 10 runs with the
three pages, and succeeds in 10 of 10 without them. `cudaMalloc(4)` succeeds in both cases.

## Expected vs actual

- **Expected:** A 4-byte `cuMemAllocAsync` from the default pool succeeds whenever the same allocation succeeds through `cuMemAlloc` and tens of GB are free. Failing that, the default pool should not depend on the placement of unrelated small host mappings.
- **Actual:** `cuDeviceGetDefaultMemPool` and `cuMemAllocAsync` return `CUDA_ERROR_OUT_OF_MEMORY`, deterministically for a given address-space layout, whenever the [8 GiB, 128 GiB) range is fragmented as described below. No kernel log messages are produced.

## Measurements

All runs used fresh processes and plain C against `libcuda`. Blockers are anonymous private mappings created with
`MAP_FIXED_NOREPLACE` before `cuInit`. "Fail" means `cuDeviceGetDefaultMemPool` and `cuMemAllocAsync(4)` both return 2.

| Layout before `cuInit` | Runs | Fail |
|---|---|---|
| No extra mappings (control) | 20 | 0 |
| 18 × 256 KiB `PROT_NONE` at random addresses in [8, 128) GiB (200 seeds) | 200 | 178 (89%) |
| Same addresses, `PROT_READ\|PROT_WRITE` instead of `PROT_NONE` | 200 | 178 (same seeds) |
| Rerun of seeds 1–50 | 50 | 46; outcome identical per seed |
| 1, 2, 3 or 4 random 256 KiB mappings in [8, 128) GiB | 100 each | 0 |
| 6 / 9 / 12 random 256 KiB mappings in [8, 128) GiB | 100 each | 3 / 34 / 67 |
| 18 random 256 KiB mappings in [256 MiB, 256 GiB) | 100 | 34 |
| 18 random 256 KiB mappings in [4, 8), [128, 256) or [256, 512) GiB | 100 each | 0 |
| ONE 4 KiB page at 8 GiB + k × 0.5 GiB, k = 0…127 (3 runs each) | 384 | 0 |
| n equally spaced 4 KiB pages at 8 + 120·i/(n+1) GiB, n = 1, 2, 4 | 5 each | 0 |
| n = 3, 5, 6, 7 | 5 each | 5 each |
| CUDA calls on a spawned pthread instead of the main thread | 3 + 3 | same as main thread |

Unaffected in every run: `cuInit`, `cuDevicePrimaryCtxRetain`, `cuMemGetInfo`,
`CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED` (= 1), `cuMemAlloc_v2(4)`, and `cuMemAddressReserve` of 2 MiB, 64 MiB and 1 GiB.
`cuMemAddressReserve` of 32 GiB fails in every failing run.

### Observed driver behaviour (strace of `mmap` and `/proc/self/maps` snapshots)

1. **At `cuInit`** the driver reads `/proc/self/maps` and reserves GPU VA with anonymous `PROT_NONE` `mmap`s. The total is 65879900160 B (device memory + 4 KiB), and every piece lies within [8 GiB, 128 GiB).
   - Unobstructed, it is one mapping at exactly `0x200000000`.
   - If no hole that large exists, it uses 2 × 30.68 GiB, then 15.34 GiB quarters, then 7.67 GiB eighths, then 4 GiB pieces, placed around existing mappings.
   - One small mapping anywhere in the window never caused a failure: the driver moved the whole reservation past it, or split it into two halves.
2. **The default pool needs one contiguous 20960 MiB range of GPU VA** (21978152960 B). It takes this either inside a single existing driver reservation or by `mmap`ing a new 21978152960-byte `PROT_NONE` region into an unmapped hole inside [8 GiB, 128 GiB). The second case was seen in strace.
   - **Unmapped hole:** a hole of exactly 20960 MiB passes and 20960 MiB − 4 KiB fails (3/3 each). Free space above 128 GiB is not used.
   - **Inside a reservation:** with a full 61.36 GiB reservation fenced by 2 MiB `cuMemAddressReserve` calls, a 20960 MiB hole passes and a 20958 MiB hole fails (3/3 each).
   - **Failure path:** the driver rereads `/proc/self/maps`, makes no `mmap` attempt, and returns `CUDA_ERROR_OUT_OF_MEMORY`.
3. **Size relation (inference):** 20960 MiB = 655 × 32 MiB, which is (`cuMemGetInfo` total / 3 = 20942.65 MiB) rounded up to 32 MiB. We measured only one memory size, so the scaling with memory is inferred, not measured. It agrees with the public report of `cudaMallocAsync` stopping at about 20 GB on this board (linked below).
4. **Prediction rule:** "pass if and only if a full or half-size driver reservation exists, or an unmapped hole of at least 20960 MiB exists in [8, 128) GiB". It predicted the outcome of all 2149 layout runs, and all 63 runs of the original application (below).
5. **Explicit pools behave differently.** In the failing layout, `cuMemPoolCreate` with `maxSize = 0` fails with OOM, but with an explicit `maxSize` from 32 MiB to 44 GiB it succeeds. `cuMemAllocFromPoolAsync` from that pool works, and no new large `mmap` appears. With `maxSize` of 46 GiB or more it fails in that layout (it succeeds up to 48 GiB without blockers).

### Ruled out

- **Physical memory exhaustion:** about 53 GB free in every run, and a synchronous allocation of the same size succeeds in the same process.
- **Other GPU processes:** none were running, and runs were serialized.
- **Missing pool support:** `CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED` = 1, and the control passes 20/20.
- **Thread, page protection or nondeterminism:** a pthread behaves like the main thread, `PROT_NONE` and RW blockers give identical results, and outcomes are deterministic per layout (50/50 seeds repeat).
- **Kernel-side errors:** the kernel log shows no nvgpu or nvmap messages during any run.

## Impact

This was found in a Node.js process (Node 25.9.0) that loads a native addon using the CUDA Driver API. Across 63 fresh
processes the first `cuMemAllocAsync` failed in about 70%. Every failure had `cuDeviceGetDefaultMemPool` returning
`CUDA_ERROR_OUT_OF_MEMORY` and the driver's reservation split into pieces of 23.01 GiB or smaller.

Before CUDA initializes, `/proc/self/maps` of those processes already contains about 18 anonymous 256 KiB mappings
spread across [0, ~251 GiB). Comparing a failing and a passing run with otherwise matching layouts, the failing run had
one extra 256 KiB mapping, which split the driver's 31 GiB piece into 23 + 8 GiB. We did not determine which component
creates those mappings.

The same CUDA code passed 10 of 10 runs as a standalone binary and 5 of 5 as a Python extension. The application-level failure is
therefore a function of the host process's address-space layout, not of the CUDA code. Any runtime or library that
creates mappings in [8 GiB, 128 GiB) before `cuInit` can make `cudaMallocAsync`, `cuMemAllocAsync` and default memory
pools unusable intermittently. Frameworks that select the stream-ordered allocator automatically when
`MEMORY_POOLS_SUPPORTED` = 1 inherit the failure.

## Workarounds that work on this system

1. **Use synchronous allocation:** `cuMemAlloc` / `cudaMalloc` succeeded in every run.
2. **Use an explicit pool:** create a pool with `cuMemPoolCreate` with an explicit, non-zero `maxSize` (tested 1–40 GiB), then make it current with `cuDeviceSetMemPool(dev, pool)`. After that, `cuMemAllocAsync` of 4 B and 256 MiB (plus memset and free) succeeded in every failing layout tested.
3. **Initialize CUDA early:** call `cuInit` before the process creates scattered mappings. This is not always possible from a library.

## Questions / requests

1. Is it intended that the default memory pool reserves one contiguous ~1/3-of-memory GPU VA range up front, and fails outright (rather than reserving less, or growing in pieces) when that range is not available?
2. Is [8 GiB, 128 GiB) a hard limit for GPU VA on this platform? Is there a supported way to make `cuInit` reserve its VA before, or independently of, the host process's existing mappings?
3. Could `cuDeviceGetDefaultMemPool` / `cuMemAllocAsync` fall back to a smaller reservation, as `cuMemPoolCreate` with an explicit `maxSize` apparently does? Alternatively, could the failure carry a distinguishable error instead of `CUDA_ERROR_OUT_OF_MEMORY`?

## Related public reports

- NVIDIA/TensorRT-LLM#10894, "cudaMallocAsync only can malloc 20G on Jetson AGX Orin 64G" (JetPack 6.2, CUDA 12.6, driver 540.4.0). This is consistent with the 20960 MiB default-pool size measured here.
- Kwaai-AI-Lab/KwaaiNet#204: the stream-ordered allocator on an Orin Nano capped at roughly one third of memory; they work around it with synchronous allocation.
- ggml-org/llama.cpp#29142: a 32 GB `cuMemAddressReserve` fails on Jetson AGX Orin 64GB (driver 540.5.0). This is the same GPU VA budget described above.
