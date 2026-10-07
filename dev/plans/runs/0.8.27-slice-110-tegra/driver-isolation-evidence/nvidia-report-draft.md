# Jetson AGX Orin 64GB (L4T R36.5.2): `cuInit` and the default memory pool return CUDA_ERROR_OUT_OF_MEMORY when small host mappings fragment [8 GiB, 128 GiB)

> **Note, 2026-10-07: superseded wording, retained as historical evidence.**
> The corrected finding is that *any* CUDA memory pool, default or explicit,
> maps one contiguous address range of ceil32(maxSize/3) at creation. The
> default pool behaves as an explicit pool with maxSize equal to device memory,
> which is about 20,960 MiB on the 64 GB Orin. A pool's usable capacity equals
> that range, and new ranges have a 1 GiB minimum. This draft's statements
> about the pool's address range predate that correction. The corrected draft
> is kept outside the repository.

## Summary

On a Jetson AGX Orin 64GB, the CUDA driver places all of its GPU virtual-address (VA) reservations inside the
process address range [8 GiB, 128 GiB). A few small, unrelated host mappings in that range make two calls fail with
`CUDA_ERROR_OUT_OF_MEMORY`, although about 53 GB of memory is free:

1. **`cuInit` fails** when no unmapped hole of at least 4 GiB is left in the range. 4 KiB `PROT_NONE` pages
   every 3.99 GiB across the range make `cuInit` fail. Pages every 4.0 GiB, which leave one hole of exactly 4 GiB,
   do not. CUDA is then unusable in the process. The failure is not sticky: after the pages are unmapped, the next
   `cuInit` succeeds.
2. **The default memory pool fails** when no contiguous 20960 MiB range of GPU VA is available in that range.
   `cuDeviceGetDefaultMemPool`, `cuMemAllocAsync` and `cudaMallocAsync` then fail even for 4 bytes, while
   `cuMemAlloc` works. Three 4 KiB pages at 38, 68 and 98 GiB, mapped before `cuInit`, cause this in 20 of 20 runs.

The two reproducers below are short C programs against `libcuda` only. Any process whose address space fills
with many small mappings in this range is affected. One example is a Node.js process with a growing JavaScript
heap. As its heap grows, the process first loses stream-ordered allocation and then cannot initialize CUDA at all.

## Environment

| Item | Value |
|---|---|
| Board | NVIDIA Jetson AGX Orin Developer Kit (`/proc/device-tree/model`), 64 GB |
| L4T | `# R36 (release), REVISION: 5.2, GCID: 46426093, BOARD: generic, EABI: aarch64, DATE: Thu Jul 16 18:56:22 UTC 2026` (`nvidia-l4t-core` / `nvidia-l4t-cuda` 36.5.2-20260716114719) |
| Kernel | 5.15.199-tegra (OOT kernel variant), Ubuntu 22.04.5 LTS, glibc 2.35, 4 KiB pages |
| Kernel module | `NVRM version: NVIDIA UNIX Open Kernel Module for aarch64 540.5.0 Release Build` |
| CUDA | toolkit 12.6 (`nvcc` V12.6.68), `cuDriverGetVersion` = 12060 |
| Memory | `cuMemGetInfo` total = 65879896064 B, free ≈ 53.4 GB; `MemTotal` 64335836 kB |
| Power mode | MAXN |
| Other | `randomize_va_space` = 2, `overcommit_memory` = 0, `ulimit -v` unlimited, no other GPU process running during the runs |

## Reproducer 1: `cuInit`

```c
/* cuinit_repro.c
 * Build: gcc -O2 -Wall -I/usr/local/cuda/include cuinit_repro.c -o cuinit_repro -lcuda
 * Run:   ./cuinit_repro          # 4 KiB pages every 3.99 GiB in [8, 128) GiB -> cuInit fails
 *        ./cuinit_repro 4.0      # every 4.0 GiB (one hole of exactly 4 GiB) -> cuInit succeeds
 *        ./cuinit_repro control  # no pages                                  -> cuInit succeeds
 * After a failed cuInit the pages are unmapped and cuInit is called again.
 */
#define _GNU_SOURCE
#include <cuda.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>

#ifndef MAP_FIXED_NOREPLACE
#define MAP_FIXED_NOREPLACE 0x100000
#endif
#define GIB (1ULL << 30)

static void check(const char *what, CUresult r) {
    const char *name = "?";
    cuGetErrorName(r, &name);
    printf("%-28s -> %d (%s)\n", what, (int)r, name);
}

int main(int argc, char **argv) {
    int control = argc > 1 && strcmp(argv[1], "control") == 0;
    double spacing_gib = (argc > 1 && !control) ? strtod(argv[1], NULL) : 3.99;
    unsigned long long step = (unsigned long long)(spacing_gib * GIB) & ~0xfffULL;
    void *page[64];
    int n = 0;
    if (!control && step > 0 && step < 120 * GIB) {
        for (unsigned long long a = 8 * GIB + step; a < 128 * GIB && n < 64; a += step) {
            page[n] = mmap((void *)a, 4096, PROT_NONE,
                           MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
            if (page[n] == MAP_FAILED) { perror("mmap"); return 2; }
            n++;
        }
        printf("mapped %d x 4 KiB PROT_NONE pages, every %.2f GiB from %#llx to %p\n",
               n, spacing_gib, 8 * GIB + step, page[n - 1]);
    }

    CUresult r = cuInit(0);
    check("cuInit", r);
    if (r != CUDA_SUCCESS && n > 0) {
        for (int i = 0; i < n; i++) munmap(page[i], 4096);
        printf("unmapped the %d pages\n", n);
        r = cuInit(0);
        check("cuInit (retry)", r);
    }
    if (r != CUDA_SUCCESS) return 1;

    CUdevice dev;
    CUcontext ctx;
    CUmemoryPool pool;
    CUdeviceptr p;
    check("cuDeviceGet", cuDeviceGet(&dev, 0));
    check("cuDevicePrimaryCtxRetain", cuDevicePrimaryCtxRetain(&ctx, dev));
    check("cuCtxSetCurrent", cuCtxSetCurrent(ctx));
    check("cuMemAlloc(4)", cuMemAlloc(&p, 4));
    check("cuMemFree", cuMemFree(p));
    check("cuDeviceGetDefaultMemPool", cuDeviceGetDefaultMemPool(&pool, dev));
    cuDevicePrimaryCtxRelease(dev);
    return 0;
}
```

Each variant gave identical output in every run: 5 of 5 runs for the default variant, and 3 of 3 each for `4.0`
and `control`.

`./cuinit_repro`:

```text
mapped 30 x 4 KiB PROT_NONE pages, every 3.99 GiB from 0x2ff5c2000 to 0x1feccbc000
cuInit                       -> 2 (CUDA_ERROR_OUT_OF_MEMORY)
unmapped the 30 pages
cuInit (retry)               -> 0 (CUDA_SUCCESS)
cuDeviceGet                  -> 0 (CUDA_SUCCESS)
cuDevicePrimaryCtxRetain     -> 0 (CUDA_SUCCESS)
cuCtxSetCurrent              -> 0 (CUDA_SUCCESS)
cuMemAlloc(4)                -> 0 (CUDA_SUCCESS)
cuMemFree                    -> 0 (CUDA_SUCCESS)
cuDeviceGetDefaultMemPool    -> 0 (CUDA_SUCCESS)
```

`./cuinit_repro 4.0`. In this run `cuInit` succeeds, but the default pool is unavailable (finding 2):

```text
mapped 29 x 4 KiB PROT_NONE pages, every 4.00 GiB from 0x300000000 to 0x1f00000000
cuInit                       -> 0 (CUDA_SUCCESS)
cuDeviceGet                  -> 0 (CUDA_SUCCESS)
cuDevicePrimaryCtxRetain     -> 0 (CUDA_SUCCESS)
cuCtxSetCurrent              -> 0 (CUDA_SUCCESS)
cuMemAlloc(4)                -> 0 (CUDA_SUCCESS)
cuMemFree                    -> 0 (CUDA_SUCCESS)
cuDeviceGetDefaultMemPool    -> 2 (CUDA_ERROR_OUT_OF_MEMORY)
```

`./cuinit_repro control`: every call returns 0 (`CUDA_SUCCESS`).

## Reproducer 2: default memory pool

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

Output with the three pages (20 of 20 runs):

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

Control output (`./minimal_repro control`, 20 of 20 runs):

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

The Runtime API behaves the same way. With the three pages, `cudaMallocAsync(&p, 4, 0)` returns
`cudaErrorMemoryAllocation` in 10 of 10 runs. Without them it succeeds in 10 of 10. `cudaMalloc(4)` succeeds in both
cases.

## Expected vs actual

- **Expected:** `cuInit` succeeds whenever the device is present and memory is free. A 4-byte `cuMemAllocAsync`
  from the default pool succeeds whenever the same `cuMemAlloc` succeeds. Neither call should depend on where
  unrelated small host mappings are placed.
- **Actual:** Both return `CUDA_ERROR_OUT_OF_MEMORY`. For a given address-space layout the outcome is
  deterministic. No kernel log messages are produced.

## Measurements

All runs used fresh processes and plain C against `libcuda`. The pages were placed one of two ways:
`MAP_FIXED_NOREPLACE` at exact addresses, or ordinary hinted `mmap` calls, described as such below.

### `cuInit`

| Layout before `cuInit` | Runs | `cuInit` result |
|---|---|---|
| 4 KiB `PROT_NONE` pages at equal spacing across [8, 128) GiB, spacing 3.0 / 3.5 / 3.9 / 3.99 GiB | 2 each | OOM in all 8 |
| Same, spacing 4.0 / 4.01 / 4.1 / 4.5 / 6.0 GiB | 2 each | success in all 10; default pool OOM in all 10 |
| 500, 1000 or 2000 × 256 KiB RW mappings at random *hinted* (non-fixed) addresses in [0, 256 GiB); 219–983 of them landed in [8, 128) GiB | 60 | OOM in 60 of 60 |
| Same with 100 hinted mappings (33–54 in the window) | 20 | success in 20; default pool OOM in 20 |
| `cuInit` first, then 100–2000 hinted mappings | 160 | success in 160 of 160. Afterwards, every later step succeeded in every run: the default pool, context create/destroy, a kernel launch, and `cuMemAlloc` of 1 and 4 GiB |
| Real layouts from 30 Node.js processes. Every mapping overlapping [8, 128) GiB was replayed as a 256 KiB `PROT_NONE` page at its start address (122–173 pages) | 30 | OOM in exactly the 13 layouts whose Node.js process had failed to create any CUDA context. The other 17 succeeded, each with the default pool OOM |
| Failed `cuInit` (500 or 2000 hinted mappings) → second `cuInit` in the same layout → unmap the mappings in the window → `cuInit` again | 10 | first OOM, second OOM, third success in 10 of 10. Context retain and `cuMemAlloc` then succeed |

Observed through `/proc/self/maps` after `cuInit` succeeds:

- **Unobstructed:** one `PROT_NONE` reservation of device memory + 4 KiB (65879900160 B) at exactly
  `0x200000000`.
- **Fragmented:** the reservation is split into smaller pieces. The smallest piece seen is 4 GiB.
  - 4 GiB holes everywhere (spacing 4.01–6.0 GiB): 16 × 4 GiB.
  - One 4 GiB hole (spacing 4.0 GiB): `cuInit` still succeeded, with a single 4 GiB reservation at `0x200000000`.
  - A replayed Node.js layout with three such holes: three 4 GiB pieces.
- **Inference:** `cuInit` fails only when it cannot place even one 4 GiB piece in [8, 128) GiB. Large unmapped
  ranges outside that window existed in every failing layout and were not used.

### Default memory pool

Blockers are anonymous private `MAP_FIXED_NOREPLACE` mappings created before `cuInit`. "Fail" means
`cuDeviceGetDefaultMemPool` and `cuMemAllocAsync(4)` both return 2.

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

In these layouts the following succeeded in every run:

- `cuInit`, `cuDevicePrimaryCtxRetain` and `cuMemGetInfo`;
- `CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED` (= 1);
- `cuMemAlloc_v2(4)`;
- `cuMemAddressReserve` of 2 MiB, 64 MiB and 1 GiB.

`cuMemAddressReserve` of 32 GiB fails in every failing run.

Observed via `strace` of `mmap` and `/proc/self/maps`:

1. **At `cuInit`**, the driver reads `/proc/self/maps` and reserves GPU VA with anonymous `PROT_NONE` `mmap`s, only
   inside [8 GiB, 128 GiB). It tries progressively smaller pieces:
   - one piece of device memory + 4 KiB;
   - 2 × 30.68 GiB;
   - 15.34 GiB quarters;
   - 7.67 GiB eighths;
   - 4 GiB pieces.

   The pieces are placed around existing mappings. A single small mapping anywhere in the range never caused a
   failure: the driver moved the whole reservation past it, or split it into two halves. Adjacent pieces merge into
   one VMA in `/proc/self/maps`.
2. **The default pool needs one contiguous 20960 MiB (21978152960 B) range of GPU VA.** It takes the range in one of
   two ways:
   - inside a single existing driver reservation (only a full or half-size one is large enough);
   - by `mmap`ing a new 21978152960-byte `PROT_NONE` region into an unmapped hole in [8 GiB, 128 GiB), lazily, at
     the first `cuDeviceGetDefaultMemPool`. This case was seen in `strace`.

   Not every range the driver will need is reserved at `cuInit`. Two thresholds were measured, 3 of 3 runs each:
   - **Unmapped hole:** 20960 MiB passes and 20960 MiB − 4 KiB fails. Free space above 128 GiB is not used.
   - **Hole inside a full 61.36 GiB reservation**, fenced by 2 MiB `cuMemAddressReserve` calls: 20960 MiB passes
     and 20958 MiB fails.

   When the default pool fails, the driver rereads `/proc/self/maps`, makes no `mmap` attempt, and returns
   `CUDA_ERROR_OUT_OF_MEMORY`.

   One control blocked every hole after `cuInit` but before the first pool query. The pool then failed in 10 of 10
   runs, although `cuInit` had succeeded with an unobstructed window.
3. **Size relation (inference):** 20960 MiB = 655 × 32 MiB. That equals `cuMemGetInfo` total / 3 (20942.65 MiB),
   rounded up to a multiple of 32 MiB. Only one memory size was measured. This size agrees with a public report of
   `cudaMallocAsync` stopping at about 20 GB on this board (linked below).
4. **Prediction rule:** the pool is available if and only if one of these exists:
   - a driver reservation of full or half size;
   - an unmapped hole of at least 20960 MiB in [8, 128) GiB.

   The rule predicted all 2149 layout runs and all 63 runs of the original application (below).
5. **Once obtained, the default pool persisted.** It survived these teardowns, with every hole in the window
   blocked afterwards:
   - release of the primary context to refcount 0 (checked with `cuDevicePrimaryCtxGetState`);
   - `cuCtxDestroy` of a non-primary context.

   In 100 of 100 runs, re-retain returned the same pool handle and `cuMemAllocAsync` worked. The driver unmapped
   nothing. This covers both the in-reservation layout and the layout in which the pool `mmap`ed its own 20960 MiB
   range. An explicit pool was kept alive (40), never created (40) or destroyed first (20).
6. **Explicit pools behave differently.** In a layout where the default pool fails:
   - `cuMemPoolCreate` with `maxSize = 0` also fails with OOM.
   - With an explicit `maxSize` from 32 MiB to 44 GiB it succeeds, `cuMemAllocFromPoolAsync` works, and no new
     large `mmap` appears.
   - With `maxSize` of 46 GiB or more it fails in that layout. Without blockers it succeeds up to 48 GiB.

### Ruled out

- **Physical memory exhaustion:** about 53 GB free in every run. Where a context exists, a synchronous allocation
  of the same size succeeds in the same process.
- **Other GPU processes:** none were running, and runs were serialized.
- **Missing pool support:** `CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED` = 1, and the control passes 20/20.
- **Thread, page protection or nondeterminism:** a pthread behaves like the main thread. `PROT_NONE` and RW blockers
  give identical results, and outcomes are deterministic per layout (50/50 seeds repeat).
- **Kernel-side errors:** the kernel log shows no nvgpu or nvmap messages during any run.

## Impact

This was found in Node.js processes (Node 25.9.0, also 24.15.0 and 26.10.0) that load a native addon using the CUDA
Driver API.

- **What the mappings are.** At process start, before the addon was loaded, [8, 128) GiB already held 6–18
  mappings (median 12 over 200 processes). In an earlier series, `/proc/self/maps` before `cuInit` held about 18
  anonymous 256 KiB mappings spread across [0, ~251 GiB). The number of mappings in the window grew with the
  JavaScript heap: 140–220 at a 66–74 MiB heap, and 1222–1340 at 622–655 MiB.
  - **Inference:** these are V8 heap pages. V8 places them at random `mmap` hints. On ARM64 Linux, current V8
    source masks the hints with `0x3FFFFFF000` (`src/base/platform/platform-posix.cc`, `GetRandomMmapAddr`, comment:
    "We truncate to 38 bits"), which spreads them over [0, 256 GiB). We did not instrument V8 itself.
- **`cuInit` after heap growth.** The addon's first `cuInit` returned `CUDA_ERROR_OUT_OF_MEMORY` in a share of
  processes that grew with the heap:

  | JavaScript heap before `cuInit` | `cuInit` OOM |
  |---|---|
  | about 74 MiB | 16 of 30 processes |
  | 163–164 MiB | 30 of 30 |
  | 622–655 MiB | 20 of 20 |

  Forcing a garbage collection first did not help: 8 of 15 still failed at 66 MiB.
- **Default pool.** In an earlier series of 63 fresh processes, the first `cuMemAllocAsync` failed in about 70%.
  Every failure had `cuDeviceGetDefaultMemPool` returning `CUDA_ERROR_OUT_OF_MEMORY`.
- **Not specific to the CUDA code.** The same CUDA code passed 10 of 10 runs as a standalone binary and 5 of 5 as a
  Python extension.

Any long-running process that accumulates scattered small mappings in [8, 128) GiB before `cuInit` can therefore
make CUDA unusable outright, not just stream-ordered allocation. This includes V8-based runtimes on ARM64 Linux.
Frameworks that select the stream-ordered allocator automatically when `MEMORY_POOLS_SUPPORTED` = 1 also inherit the
default-pool failure.

## Workarounds that worked on this system (observations, not recommendations)

1. **Call `cuInit` as early as possible,** before the process builds large heaps or many mappings.
   - In Node.js, `cuInit` at addon load, before heap growth, succeeded in 200 of 200 processes (heaps 37–658 MiB
     afterwards). Later heap growth did not break the initialized driver.
   - This protects `cuInit` and the driver's initial reservation only. Node's start-up mappings already split that
     reservation: right after the early `cuInit`, the largest reservation was at least 30.6 GiB in only 30 of 200
     processes. Even a default-pool query made at that point failed in 29 of 40 processes.
   - In plain C with 25 hinted mappings before `cuInit`, `cuInit` succeeded in 60 of 60 runs and the default pool
     failed in 48 of 60.
   - In a library, initializing early is not always possible.
2. **Use synchronous allocation when the default pool is unavailable.** `cuMemAlloc` / `cudaMalloc` succeeded in
   every run that had a context.
3. **Use an explicit pool.** Create it with `cuMemPoolCreate` with a non-zero `maxSize` (tested 1–40 GiB), then make
   it current with `cuDeviceSetMemPool(dev, pool)`. After that, `cuMemAllocAsync` of 4 B and 256 MiB (plus memset and
   free) succeeded in every failing layout tested.

## Questions / requests

1. Why are GPU VA reservations confined to [8 GiB, 128 GiB) of the process address space on this platform? Is this
   a hardware or driver limit?
2. Why does `cuInit` need an unmapped hole of at least 4 GiB there, and fail outright when none exists, rather than
   reserving in smaller pieces?
3. Why does the default memory pool need one contiguous range of about 1/3 of device memory, and fail rather than
   reserve less or grow in pieces, as `cuMemPoolCreate` with an explicit `maxSize` apparently does? Could the failure
   carry a distinguishable error instead of `CUDA_ERROR_OUT_OF_MEMORY`?
4. Is there a supported way to make the driver place these reservations elsewhere? Alternatively, can a process
   pre-reserve them before its own mappings exist, for example through an environment variable, a loader hook, or
   an early `cuInit` that also reserves the default pool's range?
5. Do newer L4T / JetPack releases change this behaviour?

## Related public reports

- NVIDIA/TensorRT-LLM#10894, "cudaMallocAsync only can malloc 20G on Jetson AGX Orin 64G" (JetPack 6.2, CUDA 12.6,
  driver 540.4.0). This is consistent with the 20960 MiB default-pool size measured here.
- Kwaai-AI-Lab/KwaaiNet#204: on an Orin Nano, the stream-ordered allocator was capped at roughly one third of
  memory. The project works around it with synchronous allocation.
- ggml-org/llama.cpp#29142: a 32 GB `cuMemAddressReserve` fails on Jetson AGX Orin 64GB (driver 540.5.0). This is
  the same GPU VA budget described above.
