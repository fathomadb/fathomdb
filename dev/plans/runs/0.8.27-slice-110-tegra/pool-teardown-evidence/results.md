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

### Without a live explicit pool, and a control after `cuInit`

The series above keep the explicit pool alive across teardown. That pool
lives in the window too, so it could have held the driver's pool machinery in
place and confounded "the default pool survives". A later round added two
`explicit=` options to the probe and a control
([run-variants.sh](run-variants.sh),
[results-variants-summary.txt](results-variants-summary.txt)):

- `explicit=none` never creates the explicit pool.
- `explicit=destroy` creates and uses it, then calls `cuMemPoolDestroy`
  before teardown.
- `fill-before-pool` is a control. In the free-gap layout, right after the
  context exists and before the first `cuDeviceGetDefaultMemPool`, it maps a
  blocker every 4 GiB through every unmapped hole of the window.

| Series | Runs | Added by pool | Freed by teardown | Pool after re-retain | Same handle | `cuMemAllocAsync` after |
| --- | --- | --- | --- | --- | --- | --- |
| primary, open, no explicit pool | 10 | 0 | 0 | success | 10 / 10 | success |
| primary, free gap, no explicit pool | 10 | 20.47 GiB | 0 | success | 10 / 10 | success |
| non-primary, open, no explicit pool | 10 | 0 | 0 | success | 10 / 10 | success |
| non-primary, free gap, no explicit pool | 10 | 20.47 GiB | 0 | success | 10 / 10 | success |
| primary, free gap, explicit pool destroyed | 10 | 20.47 GiB | 0 | success | 10 / 10 | success |
| non-primary, free gap, explicit pool destroyed | 10 | 20.47 GiB | 0 | success | 10 / 10 | success |
| control: holes filled after `cuInit`, before the first pool query | 10 | 0 | 0 | `CUDA_ERROR_OUT_OF_MEMORY` | — | `CUDA_ERROR_OUT_OF_MEMORY` |

`cuMemAlloc` succeeded in every run, the control included.

## Conclusions

- **On this Jetson, the default pool survives context teardown.** This holds
  whether its context is a primary context released to refcount 0 or a
  destroyed non-primary context, and whether an explicit pool is kept alive
  (40 / 40), never created (40 / 40) or destroyed before teardown (20 / 20).
  The driver unmapped nothing in the window. With every remaining hole
  blocked, re-retaining returned the same pool handle, and stream-ordered
  allocation worked in all 100 runs. A cached "stream-ordered" decision
  therefore stays valid for the process on the measured device, and the
  patch needs no re-check there.
- **The control detects an unavailable pool after `cuInit`.** Blocking every
  hole after `cuInit` but before the first pool query left the pool at
  `CUDA_ERROR_OUT_OF_MEMORY` in 10 / 10 runs. The probe's hole filling is
  therefore able to deny the pool a range, so the survival above is not an
  artefact of a layout that could never fail. It also confirms that the
  free-gap pool range is mapped lazily, at the first pool query.
- **Refuted:** "the driver makes all window reservations inside `cuInit`".
  In the free-gap layout the default pool added a new 20.47 GiB mapping when
  first obtained, in 20 / 20 runs. That mapping was then kept across
  teardown.
- **Confirmed:** an explicit pool survives primary-context release and
  re-retain, and non-primary destroy and re-create, in 40 / 40 runs. It also
  works in the control layout.
- **Not measured:** a co-resident library calling `cuDevicePrimaryCtxReset`
  or `cudaDeviceReset` (neither cudarc nor Candle does), other Jetson models,
  and non-Tegra aarch64 Linux CUDA hosts.

Sample logs: [sample-logs/](sample-logs/), one per series (`*-none-01`,
`*-destroy-01` and `*-fillbeforepool-01` are from the later round). The
remaining per-run logs are omitted; each repeats its series' `RESULT` line,
and those lines are counted in the summaries.
