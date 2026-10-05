# Default memory pool across context teardown

Question from the second review of the cudarc allocator fallback. The patch
records one allocator decision per device per process. Can a cached
"stream-ordered" decision outlive the context that justified it? That would
happen if the default pool's address range were released when its context
goes away, and the process heap then grew into the freed range.

Host: Jetson AGX Orin 64 GB, L4T R36.5.2, driver 540.5.0, CUDA 12.6. Plain C
against `libcuda` ([pool_teardown.c](pool_teardown.c)). Fresh processes, run
one at a time under the shared GPU lock ([run-series.sh](run-series.sh)).

## Method

1. After `cuInit`, take a snapshot of the [8 GiB, 128 GiB) window from
   `/proc/self/maps`.
2. Retain the primary context, or create a non-primary one.
   - Obtain the default pool and run `cuMemAllocAsync`.
   - Create an explicit pool with `maxSize` 1 GiB and allocate from it.
   - Take a snapshot.
3. Tear the context down.
   - Primary: release it, and check refcount 0 with
     `cuDevicePrimaryCtxGetState`.
   - Non-primary: `cuCtxDestroy`.
   - Take a snapshot. Every range that was mapped before teardown and is
     unmapped after it counts as freed.
4. Map blockers. All are 4 KiB `PROT_NONE` pages placed with
   `MAP_FIXED_NOREPLACE`.
   - One every 4 GiB through each freed range.
   - The three standard blockers at 38, 68 and 98 GiB.
   - One every 4 GiB through every remaining unmapped hole.

   After this no hole of 4 GiB or more is left in the window, so a destroyed
   pool could not be re-created outside existing mappings.
5. Retain or create the context again. Query the default pool and compare its
   handle with the earlier one. Then try `cuMemAllocAsync`, `cuMemAlloc`, and
   allocation from the explicit pool.

Layouts:

- **Open.** No blockers before `cuInit`. The driver makes one 61.36 GiB
  reservation, and the default pool lives inside it.
- **Free gap.** Five pages before `cuInit`, at 32, 56, 80, 104 and 125 GiB + 4
  KiB. The driver's reservations are forced into 15.34 GiB quarters, so the
  default pool `mmap`s a new 20.47 GiB range into the one free 21 GiB hole.
  This is the case where teardown could plausibly unmap the pool.
- **Control.** The three standard blockers before `cuInit`, so the pool is
  unavailable from the start.

## Results ([results-summary.txt](results-summary.txt))

| Series | Runs | Added by pool | Freed by teardown | Pool after re-retain | Same handle | `cuMemAllocAsync` after | Explicit pool after |
| --- | --- | --- | --- | --- | --- | --- | --- |
| primary, open | 10 | 0 | 0 | success | 10 / 10 | success | success |
| primary, free gap | 10 | 20.47 GiB | 0 | success | 10 / 10 | success | success |
| non-primary, open | 10 | 0 | 0 | success | 10 / 10 | success | success |
| non-primary, free gap | 10 | 20.47 GiB | 0 | success | 10 / 10 | success | success |
| control (pool unavailable from the start) | 5 | 0 | 0 | `CUDA_ERROR_OUT_OF_MEMORY` | — | `CUDA_ERROR_OUT_OF_MEMORY` | success |

An earlier run of the same probe without the hole-filling step gave the same
answer in 40 / 40 runs.

## Conclusions

- **The default pool survives context teardown.** This holds whether its
  context is a primary context released to refcount 0 or a destroyed
  non-primary context. The driver unmapped nothing in the window. With every
  remaining hole blocked, re-retaining returned the same pool handle, and
  stream-ordered allocation worked in 40 / 40 runs. A cached "stream-ordered"
  decision therefore stays valid for the process, and the patch needs no
  re-check.
- **Refuted:** "the driver makes all window reservations inside `cuInit`".
  In the free-gap layout the default pool added a new 20.47 GiB mapping when
  first obtained, in 20 / 20 runs. That mapping was then kept across
  teardown.
- **Confirmed:** an explicit pool survives primary-context release and
  re-retain, and non-primary destroy and re-create, in 40 / 40 runs. It also
  works in the control layout.
- **Not measured:** `cuDevicePrimaryCtxReset`, which neither cudarc nor Candle
  calls, and other Jetson models.

Sample logs: [sample-logs/](sample-logs/). The remaining per-run logs are
omitted; each repeats its series' `RESULT` line, and those lines are counted
in the summary.
