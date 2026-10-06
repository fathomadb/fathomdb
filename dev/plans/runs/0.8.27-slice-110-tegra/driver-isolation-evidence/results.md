# Driver isolation: default memory pool fails with CUDA_ERROR_OUT_OF_MEMORY on fragmented VA (Jetson AGX Orin 64GB)

Host: Jetson AGX Orin Developer Kit 64GB, L4T R36.5.2, kernel 5.15.199-tegra, CUDA 12.6.68 toolkit,
driver 540.5.0 (`cuDriverGetVersion` = 12060), `cuMemGetInfo` total = 65879896064 B. Full list: `env.txt`.
All runs: fresh processes, plain C against `libcuda` (no Node.js, Candle, cudarc or Python).
Each GPU series ran under `flock` on a shared GPU lock (another agent was measuring concurrently).

## Files

| File | What |
|---|---|
| `minimal_repro.c` | ~60-line Driver API reproducer: three 4 KiB `PROT_NONE` pages at 38/68/98 GiB, then `cuMemAllocAsync`. `control` arg skips the pages. |
| `minimal_repro_runtime.c` | Same via the Runtime API (`cudaMallocAsync`). |
| `cuinit_repro.c` | `cuInit` reproducer for the report draft's second finding: 4 KiB pages every 3.99 GiB make `cuInit` fail (`4.0` and `control` pass); may be attached with `minimal_repro.c`. |
| `pool_va_repro.c` | Full instrumented reproducer used for every series (options documented in its header). |
| `run-series.sh`, `sweep.sh`, `run-configs.sh` | Series drivers (one fresh process per run). |
| `cfg-*.txt` | Exact argument lists for the configuration-driven series. |
| `analyze.py` → `results-summary.txt` | Outcome counts and per-call result patterns per series. |
| `gaps.py` → `gaps-summary.txt` | Largest reservation VMA / largest free hole per run and the rule check. |
| `node_layout_check.py` → `node-layout-check.txt` | The same rule applied to the 63 earlier Node.js runs (`../alloc-split-evidence/`). |
| `mapdiff.py` | Diff two `/proc/self/maps` snapshots. |
| `logs/<series>/` | Raw per-run logs, `results.txt` (one `RESULT` line per run), `series-header.txt`. |
| `logs/strace/` | `strace` of control, n=3, n=4, s2 seed 2 and seed 101, explicit-maxSize pool. |
| `logs/minimal/` | 20+20 Driver API and 10+10 Runtime API minimal-repro runs. |
| `logs/kernel-log-check.txt` | Kernel journal check (no messages after boot; `dmesg` itself not permitted). |
| `maps-all.tar.xz` | All `/proc/self/maps` snapshots (4 stages per run), compressed. Extract in this directory (`tar -xJf maps-all.tar.xz`) to recreate `logs/<series>/maps/` before rerunning `gaps.py`. |

Raw logs and strace output contain local paths/usernames; attach only `minimal_repro.c` and its output to any external report.

**Repository copy.** The committed copy omits `logs/` (raw per-run logs, `strace`
output, maps snapshots) and `maps-all.tar.xz`; they stayed on the Jetson host.
The series drivers are archived as `run-series.sh.txt`, `sweep.sh.txt` and
`run-configs.sh.txt` (they are records, not maintained scripts). The summaries
above were produced from the omitted logs.

## Build

```sh
gcc -O2 -Wall -Wextra -std=gnu11 -I/usr/local/cuda-12.6/include pool_va_repro.c -o pool_va_repro -lcuda -lpthread
gcc -O2 -Wall -I/usr/local/cuda/include minimal_repro.c -o minimal_repro -lcuda
gcc -O2 -Wall -I/usr/local/cuda/include minimal_repro_runtime.c -o minimal_repro_runtime -L/usr/local/cuda/lib64 -lcudart
```

## Results by series

"FAIL" = `cuDeviceGetDefaultMemPool` and `cuMemAllocAsync(4)` both return 2 (`CUDA_ERROR_OUT_OF_MEMORY`).
In every run of every series `cuInit`, context creation, `cuMemGetInfo` (~53.4 GB free of 65.88 GB),
`CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED` (=1), `cuMemAlloc_v2(4)` + free, and `cuMemAddressReserve` of 2 MiB / 64 MiB / 1 GiB succeeded.
Blockers are anonymous private `MAP_FIXED_NOREPLACE` mappings created before `cuInit`.

| Series | Blockers | Runs | FAIL | Rate |
|---|---|---|---|---|
| s1-control | none | 20 | 0 | 0% |
| s2-random18-8G-128G-none | 18 × 256 KiB `PROT_NONE`, uniform in [8, 128) GiB, seeds 1–200 | 200 | 178 | 89% |
| s2-random18-8G-128G-rw | same addresses, `PROT_READ\|PROT_WRITE` (touched) | 200 | 178 | 89% (same seeds as above) |
| s2-rerun-seeds1-50-none | rerun of seeds 1–50 | 50 | 46 | 92%; per-seed outcome identical to first run 50/50 |
| s2b-random{1,2,3,4} | 1/2/3/4 × 256 KiB in [8, 128) GiB | 100 each | 0 each | 0% |
| s2b-random6 | 6 × 256 KiB in [8, 128) GiB | 100 | 3 | 3% |
| s2b-random9 | 9 × 256 KiB in [8, 128) GiB | 100 | 34 | 34% |
| s2b-random12 | 12 × 256 KiB in [8, 128) GiB | 100 | 67 | 67% |
| s2c-random18-0G-256G | 18 × 256 KiB in [256 MiB, 256 GiB) | 100 | 34 | 34% |
| s3-random18-0G-8G | 18 × 256 KiB in [4, 8) GiB | 100 | 0 | 0% |
| s3-random18-128G-256G | 18 × 256 KiB in [128, 256) GiB | 100 | 0 | 0% |
| s3-random18-256G-512G | 18 × 256 KiB in [256, 512) GiB | 100 | 0 | 0% |
| s4a-single-4K-sweep | ONE 4 KiB page at 8 GiB + k·0.5 GiB, k = 0..127, ×3 | 384 | 0 | 0% |
| s4b n=1,2,4 | n equally spaced 4 KiB pages at 8 + 120·i/(n+1) GiB | 5 each | 0 | 0% |
| s4b n=3,5,6,7 | same | 5 each | 5 each | 100% |
| minimal (Driver API) | 3 × 4 KiB at 38/68/98 GiB | 20 | 20 | 100% |
| minimal control | none | 20 | 0 | 0% |
| minimal (Runtime API, `cudaMallocAsync`) | 3 × 4 KiB at 38/68/98 GiB | 10 | 10 | 100% (`cudaErrorMemoryAllocation`) |
| minimal Runtime control | none | 10 | 0 | 0% |
| s5 n3-pthread / control-pthread | CUDA calls on a spawned pthread | 3 / 3 | 3 / 0 | thread makes no difference |

### Threshold series (deterministic, 3/3 runs per point)

| Series | Probe | Last FAIL | First pass |
|---|---|---|---|
| s4c-freegap-threshold | All driver reservations forced to 15.34 GiB quarters; one unmapped hole of exactly G bytes inside [8, 128) GiB | G = 20960 MiB − 4 KiB | G = 20960 MiB (21978152960 B) |
| s4c top-108 / top-112 | Holes inside [8,128) < 20 GiB, unlimited free space above 128 GiB | FAIL | — |
| s4d / s4e in-unit | Full 61.36 GiB reservation, outside holes < 20 GiB, 2 MiB `cuMemAddressReserve` fences leaving one hole of Y MiB inside the reservation | Y = 20958 MiB | Y = 20960 MiB |

20960 MiB = 655 × 32 MiB = (cuMemGetInfo total / 3) rounded up to a 32 MiB multiple (total/3 = 20942.65 MiB). The rounding relation is inferred; only one memory size was available.

### Explicit pools (failing n=3 layout, default pool fails in all of these)

| Probe | Result |
|---|---|
| `cuMemPoolCreate` with `maxSize = 0` (created after the default pool) | 2 (OOM) in all 676 failing runs and in all 22 s2 passing runs; across all series it succeeded only where a second 20960 MiB range was still available (950/950 full-reservation runs; 257/489 half-reservation runs) — consistent with a second 20960 MiB contiguous reservation (inference) |
| `cuMemPoolCreate` with `maxSize` = 32 MiB, 1, 8, 14, 15, 16, 20, 20959M, 20960M, 20961M, 22, 30, 32, 40, 42, 44 GiB | 0 (success); `cuMemAllocFromPoolAsync(4)` succeeds; strace shows no new large mmap |
| same, `maxSize` ≥ 46 GiB (n=3) / up to 48 GiB in control | 2 in n=3 at 46, 48, 52, 56, 60, 61, 62, 64 GiB; control succeeds up to 48 GiB |
| explicit pool (maxSize 1–40 GiB) + `cuDeviceSetMemPool` + `cuMemAllocAsync` 4 B and 256 MiB + memset + free | all succeed (n=3 layout and s2 failing seeds 2, 3, 4, 5, 101) |

## Mechanism (observed via strace and /proc/self/maps)

1. During `cuInit` the driver reads `/proc/self/maps` and mmaps `PROT_NONE` reservations for the GPU VA space,
   total = device memory + 4 KiB (65879900160 B), only inside [8 GiB, 128 GiB).
   Unobstructed: one 61.36 GiB mapping at exactly 0x200000000. If no hole that large exists, it falls back to
   2 × 30.68 GiB, then 15.34 GiB quarters, 7.67 GiB eighths, and 4 GiB pieces, placed first-fit around existing mappings.
   A single small mapping anywhere in the window never causes failure: the driver moves the whole reservation past it,
   or splits it into halves (s4a).
2. Adjacent driver reservations merge into one VMA in `/proc/self/maps` (for example a 15.34 + 7.67 pair shows as "23.01 GiB"),
   so VMA sizes overstate the driver's contiguous units.
3. `cuDeviceGetDefaultMemPool` (or the first `cuMemAllocAsync`) needs one contiguous 20960 MiB range of GPU VA,
   either inside one existing driver reservation (only a full or half reservation is big enough) or as a new
   `mmap` of exactly 21978152960 B into an unmapped hole inside [8, 128) GiB (strace, n=4). If neither exists it
   rereads `/proc/self/maps`, makes no mmap attempt, and returns `CUDA_ERROR_OUT_OF_MEMORY`.
4. Rule "pass ⇔ largest reservation VMA ≥ 30.6 GiB or largest unmapped hole in [8,128) GiB ≥ 20960 MiB"
   matches 2149 / 2149 runs here (excluding the `--pre-reserve` series, which the maps cannot show) and 63 / 63 earlier Node.js runs.
5. No kernel log messages were emitted during any run.

## Where to file (recommendation; nothing has been posted)

1. **Primary:** NVIDIA Developer Forums → Robotics & Edge Computing → Jetson Systems → Jetson AGX Orin. This is where L4T driver and libcuda issues on this board get moderator triage; for example, the node-llama-cpp VMM OOM thread there drew a moderator reply asking for the L4T version. Post `nvidia-report-draft.md` with `minimal_repro.c` inline.
2. **Secondary, separate:** a cudarc issue (github.com/chelsea0x3b/cudarc, the crate's repository per crates.io; latest 0.19.10).
   - Searches found no existing issue about this failure.
   - #536 (open) requests memory-pool wrappers; #543 (closed) exposed the `has_async_alloc` field.
   - Proposal: an opt-out or automatic fallback to `cuMemAlloc` when `cuMemAllocAsync` / `cuDeviceGetDefaultMemPool` returns `CUDA_ERROR_OUT_OF_MEMORY`. Alternatively, an explicit-`maxSize` pool set with `cuDeviceSetMemPool`, which tested as a working workaround.
   - Cross-reference the NVIDIA forum thread once it exists.
3. **Cross-references:** in the forum post, not as comments on others' issues.
   - NVIDIA/TensorRT-LLM#10894 (closed, no NVIDIA explanation visible) shows the same ~20 GB default-pool size on this board.
   - Kwaai-AI-Lab/KwaaiNet#204 (merged) carries a cudarc sync-allocation opt-out for the 1/3-memory cap on an Orin Nano.
   - ggml-org/llama.cpp#29142 (open) is a 32 GB `cuMemAddressReserve` failure on the same board and driver.
   - None of the three reports VA fragmentation as the trigger; this report is new information, not a duplicate.
