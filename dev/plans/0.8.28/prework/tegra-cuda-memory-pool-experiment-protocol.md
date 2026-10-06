---
title: FathomDB 0.8.28 prework — Tegra CUDA memory-pool experiment protocol
status: PROPOSED
target_release: 0.8.28
observed_on: 2026-10-05
---

# Tegra CUDA memory-pool experiment protocol (0.8.28)

This document executes the study plan
`dev/plans/0.8.28/prework/tegra-cuda-memory-pool-study.md`. The plan says
what must be shown; this protocol says exactly what to build, run, record and
compute, so that an engineer can run the evaluation without further design
decisions. Where the two differ, this protocol's method governs and the plan
is corrected in place. Nothing here authorizes a product change. Nobody posts
anything upstream or to NVIDIA on the strength of this document; § 10 only
prepares material for the owner.

Every number quoted below comes from the Slice 110 evidence on this branch
under `dev/plans/runs/0.8.27-slice-110-tegra/` and was measured on one Jetson
AGX Orin 64 GB (L4T R36.5.2, driver 540.5.0, CUDA 12.6.68). Statements marked
*inference* or *hypothesis* are not measured and are tested by this protocol.

## 0. Conventions

- `<worktree>`: the study checkout. `<scratch>`: the session scratch
  directory. `<evidence>`: `dev/plans/runs/0.8.28-pool-study/` in the
  worktree (§ 9). Committed files never contain a real home path or user
  name; redact to `<worktree>` and `<scratch>` before `git add` (§ 9.3).
- **GPU lock.** Every GPU series runs under
  `flock <scratch>/gpu.lock <command>`, the same discipline as Slice 110
  (`fix-verification/run-series.sh`, `early-cuinit-verification/run-series.sh`
  both say so in their headers). The lock is held for a whole series, not per
  process. R7 (concurrent processes) holds the lock in its one launcher
  process. Nothing else uses the GPU while a series runs.
- **Fresh process per run.** One Node, Python or C process per data point.
  The study never reuses a process across runs, because every effect under
  study (address-space layout, pool creation, the once-per-device decision)
  is a process-lifetime effect.
- **Host and artifact record.** Each series directory holds
  `series-header.txt` with: UTC start, L4T release
  (`/etc/nv_tegra_release`), kernel, `cuDriverGetVersion`, Node version,
  Python version, `nvpmodel -q` output, `jetson_clocks --show` output,
  the sha256 of the `.node`, wheel and C binaries used, the variant, the
  pool parameters and the full environment filtered to `FATHOMDB_*`,
  `CUDA_*`, `NODE_OPTIONS`.
- **No sudo.** `nvpmodel` and `jetson_clocks` are read, never changed. If
  the power mode changes between the start and end of a series, the series is
  discarded and rerun.

## 1. Preconditions, safety and stop conditions

### 1.1 Host-quiet check (before every series, recorded per run)

A run starts only when all of the following hold; the runner records the
values and refuses otherwise:

1. No process named `cargo`, `rustc`, `nvcc`, `pytest`, `maturin`,
   `Runner.Worker`, or another `node`/`python` running a FathomDB consumer, is
   alive (`ps -eo args`). The Slice 110 runners counted these as `busy_procs`
   (`explicit-pool-experiment/run-series.sh.txt`); here they are a refusal.
2. 1-minute load average below 2.0 on this 12-core host.
3. `MemAvailable` at least 40 GiB and `SwapFree` equal to `SwapTotal`
   (`/proc/meminfo`). If `SwapFree` drops during a series, the series is
   invalid: swapping corrupts every timing.
4. GPU idle: `tegrastats` reports `GR3D_FREQ 0%` for 3 consecutive seconds.
5. Thermal zones (`/sys/devices/virtual/thermal/thermal_zone*/temp`) are
   recorded per run and reported; no threshold is enforced, but timing
   comparisons are made within interleaved blocks (§ 6.1) so that slow
   drift affects every variant alike.

### 1.2 Memory safety on the shared 64 GB unified memory

The host's desktop, this session and the GPU share one 64 GB DRAM
(`free -g`: 61 GiB total, about 51 GiB available at the time of writing).
Every probe that allocates real memory is bounded:

- **Pool exhaustion probes (C3)** allocate until the pool refuses, so they can
  reach `maxSize`. They run only for `maxSize` ≤ 8 GiB, only when
  `MemAvailable` ≥ 2 × `maxSize` + 16 GiB, one at a time, under
  `timeout 300`. The 16 GiB `maxSize` cell of C3 is run only if the measured
  capacity model (§ 6.5) from the sizes ≤ 8 GiB predicts exhaustion below
  8 GiB of real allocation; otherwise 16 GiB is characterised by pool
  creation and a 2 GiB allocation only, and the cell is marked *bounded, not
  exhausted*.
- **Concurrent processes (R7)** are launched by one runner that samples
  `MemAvailable` every second and sends `SIGTERM` to every child if it falls
  below 8 GiB, recording the trial as *aborted-for-safety* (a result, not a
  failure of the variant).
- **Soak (R6)** runs with `ulimit -v` unset (CUDA needs the 61 GiB virtual
  reservation) but with a watchdog that stops the process if RSS exceeds
  4 GiB or `MemAvailable` falls below 16 GiB.
- Every Node and Python run is wrapped in `timeout 600`; every C probe in
  `timeout 120`.
- Release threshold `max` keeps freed memory in the pool. In R7 that is
  bounded by the workload's reserved high (160 MiB per process at 3 GiB
  `maxSize` in `explicit-pool-experiment/analysis.txt`), so eight processes
  hold about 1.3 GiB; this is a measured expectation, and R7 records the
  actual sum.

### 1.3 Stop conditions

Stop the study, write up what was measured and consult the owner when:

- any variant crashes a process (signal, not a typed error) at any point:
  that is a correctness failure and a probable wrong-API free; collect the
  core pattern and stop that variant;
- the production-equivalence control (§ 5, Phase 0) fails: the experiment
  build does not reproduce S, so nothing measured with it is about the
  product;
- the same unexplained failure repeats twice in a cell (the repository's
  retry budget, `AGENTS.md` § 5);
- `SwapFree` moves, the power mode changes, or `MemAvailable` falls below
  the floors above.

## 2. Variants

### 2.1 Definitions

All variants keep the production synchronous path as the last resort and keep
Node's early `cuInit` at module registration
(`src/rust/crates/fathomdb-napi/src/cuda_early_init.rs`) as shipped. "Pool"
means one explicit pool per device per process created with
`cuMemPoolCreate` (pinned device allocation, `maxSize` as configured, release
threshold as configured), installed with `cuDeviceSetMemPool`, probed with one
4-byte `cuMemAllocAsync` / `cuMemFreeAsync` / `cuCtxSynchronize`, and never
destroyed. That is the lifecycle the Slice 110 experiment patch used
(`explicit-pool-experiment/cudarc-0.19.7-explicit-pool.patch`,
`slice110_explicit_pool`).

| Variant | Binding | `cuInit` | Pool created | Default pool used? | Allocator decision |
| --- | --- | --- | --- | --- | --- |
| **S** | Node, Python | Node: at registration. Python: at first use. | never | yes, if available | production rule: default pool available → stream-ordered, else synchronous (`third_party/cudarc-0.19.7/src/driver/safe/core.rs`, `async_alloc_decision`) |
| **A-load-hold** | Node only | at registration | at registration, right after `cuInit`, on a primary context the hook retains and **keeps retained** for the process life | no (the explicit pool is installed whether or not the default is available) | § 2.2 rule |
| **A-load-release** | Node only | at registration | at registration, then the hook **releases** the primary context it retained | no | § 2.2 rule |
| **A-first-use** | Node, Python | as S | immediately before the first `Device::new_cuda` of the process | no | § 2.2 rule |
| **B** | Node, Python | as S | before the first `Device::new_cuda`, **only if** `cuDeviceGetDefaultMemPool` fails with `CUDA_ERROR_OUT_OF_MEMORY` | yes, if available | § 2.2 rule |

Notes on the definitions:

- A-load has two sub-variants because the Slice 110 cost measurement
  (`early-cuinit-verification/experiment/E-cost-side-effects.txt`) shows that
  most of the load-time cost is the retained context, not the pool: import
  with `cuInit` only was 31.3 ms / +13 MiB RSS (arm 1); `cuInit` plus a
  retained primary context was 74.8 ms / +112 MiB (arm 2); `cuInit` plus
  retained context plus a 3 GiB pool was 92.4 ms / +121.5 MiB (arm 4). Whether
  a pool created at load survives releasing that context in the product is
  **not measured**; in plain C an explicit pool survived primary-context
  release to refcount 0 and re-retain in 40 / 40 runs
  (`pool-teardown-evidence/results.md`). A-load-release tests that in the
  product and is the only form of A-load that can pass the plan's import-cost
  gate.
- A-first-use and B create the pool at the same moment with the same call.
  They differ only in whether the default pool is preferred when available.
  Any fragmentation effect on lazy creation (R5) therefore applies to both
  equally; only the A-load variants are immune to it.
- Python has no registration-time hook (the wheel's bindings are unchanged by
  Slice 110; `receipt.md`, "Tegra Python wheel"). Python runs S, A-first-use
  and B only. Adding an import-time hook to the wheel is an owner decision
  (§ 11), not part of this study.
- **Deferred variant P (private pool).** Allocating with
  `cuMemAllocFromPoolAsync` while leaving the device's current pool alone
  needs a change in cudarc's `CudaStream::alloc` / `CudaSlice::drop` paths,
  not just a device-level install, and is not the shape the plan's upstream
  survey favours. It is built and run only if C7 or R7 shows that installing
  the pool device-wide harms a co-resident consumer; the protocol then adds a
  variant row with the same harnesses.

### 2.2 The allocator decision in the experiment build

The production rule (`async_alloc_decision`) returns stream-ordered only when
`cuDeviceGetDefaultMemPool` succeeds. With an explicit pool installed that
query still fails, so without a rule change every pool variant would be
downgraded to synchronous. The experiment build replaces the rule, on aarch64
Linux only and for all four constructors, with:

1. `CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED` ≤ 0 → synchronous (`sync`).
2. `cuDeviceGetDefaultMemPool` succeeds → stream-ordered (`default`).
3. Otherwise, if `cuDeviceGetMemPool` returns a pool and a 4-byte
   `cuMemAllocAsync` / `cuMemFreeAsync` / `cuCtxSynchronize` on the null
   stream succeeds → stream-ordered (`explicit`).
4. Otherwise → `sync`. Errors other than `CUDA_ERROR_OUT_OF_MEMORY` and
   `CUDA_ERROR_NOT_SUPPORTED` from step 2 keep the production behaviour of
   failing the constructor.

The result is a three-state decision (`default`, `explicit`, `sync`), recorded
once per device per process in the existing `AllocModeByDevice` table and
read by every wrapper, exactly as the production two-state decision is. The
pure unit tests in the `fathomdb_alloc_fallback` module are extended to the
three states (plan C1).

**The decision must be made after the pool is installed.** The table caches
the first successful decision for the process, so a `CudaContext` constructed
before the policy installs the pool would record `sync` for the life of the
process. The policy therefore uses result-level calls (`result::primary_ctx::
retain`, `result::mem_pool::create`, `result::device::set_mem_pool`,
`result::primary_ctx::release`, all present in the vendored crate's
`src/driver/result.rs`) and never constructs a safe `CudaContext` itself.

### 2.3 Where the code lives

The split prototypes the upstream shape the plan's survey recommends: a small
opt-in primitive in cudarc, policy in FathomDB.

**Vendored cudarc** (`third_party/cudarc-0.19.7`, experiment branch only):

- `src/driver/safe/mem_pool.rs` (new): `pub struct CudaMemPool` wrapping a
  `CUmemoryPool` for one device; `CudaMemPool::create(ordinal, &MemPoolProps)
  -> Result<Self, DriverError>` (`MemPoolProps { max_size: usize,
  release_threshold: u64 }`, pinned device allocation, no handle types),
  `install(&self) -> Result<(), DriverError>` (`cuDeviceSetMemPool`),
  `attribute(&self, CUmemPool_attribute) -> Result<u64, DriverError>`,
  `trim_to(&self, keep: usize)`, `raw(&self) -> CUmemoryPool`. `Drop`
  destroys the pool, which is upstream's RAII convention; FathomDB keeps the
  pool alive by storing it in a `static OnceLock` and never dropping it. The
  `Drop` doc states the hazard the plan's C2 names: destroying an installed
  pool reverts the device to its default pool. `max_size` is set only under
  `cuda-12020` or later, matching the survey's binding fact; the experiment
  build targets 12.6, so the field is present.
- The decision rule of § 2.2, plus `pub fn alloc_mode(&self) -> AllocMode`
  on `CudaContext` returning `Default | Explicit | Sync`. This is the single
  most important measurement change: Slice 110 **inferred** which path a
  process took from bimodal steady-embed latency (`receipt.md`, "Full-path
  Node verification"; platform reference § 7.7). This study reads it.
- No environment variables, no `target_arch` cfg beyond the existing
  `SYNC_FALLBACK` gate, no printing. The crate does not know the study
  exists.

**FathomDB** (`src/rust/crates/fathomdb-embedder/src/cuda_pool_policy.rs`,
new, compiled only with the cargo feature `tegra-pool-experiment` on aarch64
Linux with `embed-cuda` or `rerank-cuda`):

- Reads `FATHOMDB_POOL_VARIANT` (`S` | `A-load-hold` | `A-load-release` |
  `A-first-use` | `B`), `FATHOMDB_POOL_MAXSIZE` (bytes, or `1G`/`3G`/`8G`/
  `16G`), `FATHOMDB_POOL_RELEASE_THRESHOLD` (`0` | `max`). Unset variant means
  `S`. An unknown value aborts the process with a message, as the experiment
  patch did, so a typo can never be measured as a variant.
- `ensure_pool_before_first_context()`: called at the top of every product
  site that constructs a Candle CUDA device: `candle_bge.rs` (`probe_cuda`,
  the witness probe and the embedder load; three `Device::new_cuda` calls) and
  `candle_reranker.rs` (the reranker probe and load; two calls). It is
  idempotent behind a `Once`. For `A-first-use` and `B` it retains the primary
  context, runs the B default-pool query if the variant is `B`, creates and
  installs the pool through the primitive, probes it, releases the context,
  and records the outcome. For `S` and the A-load variants it does nothing.
- `install_pool_at_load()`: called from the Node registration hook after a
  successful `cuInit`, only for the A-load variants; same steps, and for
  `A-load-hold` it does not release the context.
- Reporting: one stderr line per decision event,
  `fdb-pool-exp event=<install|decide|teardown> pid=<pid> variant=<v>
  site=<registration|embedder-probe|embedder-load|reranker-probe|reranker-load>
  default_pool=<CUresult> create=<CUresult> set=<CUresult> probe=<CUresult>
  release=<CUresult> alloc_mode=<default|explicit|sync> max_size=<bytes>
  threshold=<n> reserved_cur=<n> reserved_high=<n> used_cur=<n> used_high=<n>
  elapsed_us=<n>`. Harnesses parse these lines; the product's public open
  report is not changed. A `teardown` line with the pool attributes is printed
  from the policy module at process exit (`libc::atexit`), so the pool's
  high-water marks are available without changing cudarc's `Drop`.
- With `FATHOMDB_POOL_STATS_EVERY_S=<n>` set, a thread prints a `stats` line
  every n seconds (used by the soak, § 4.7).

**Keeping it out of production.** The feature `tegra-pool-experiment` is not
in any feature set of `scripts/release/cuda-artifact-contract.sh`
(`CUDA_NAPI_FEATURES`, `CUDA_RERANK_NAPI_FEATURES`, `CUDA_PYTHON_FEATURES`,
`CUDA_RERANK_PYTHON_FEATURES`) and is added to the build commands of § 3 only.
The study branch is never merged to a release branch; a product
implementation, if ruled, is a separate 0.8.28 slice written to the ruling.
Phase 0 (§ 5) includes an equivalence control: the experiment build with
`FATHOMDB_POOL_VARIANT=S` against the production build on the same series.

## 3. Build recipe (this host)

All builds run from `<worktree>`. Never `pip install -e` or
`maturin develop` from a worktree (`AGENTS.md` § 10). Record every artifact
hash in `<evidence>/artifacts.sha256`.

**Environment** for every CUDA build:

```sh
export PATH=/usr/local/cuda-12.6/bin:$PATH
export CUDA_PATH=/usr/local/cuda-12.6
export CUDA_COMPUTE_CAP=87
export LIBRARY_PATH=/usr/local/cuda-12.6/targets/aarch64-linux/lib
```

**Node CUDA addon** (Node 25.9.0 via nvm; `receipt.md` records the same
recipe and the `PATH` requirement for `nvcc`):

```sh
source ~/.nvm/nvm.sh && nvm use 25.9.0
cd <worktree>/src/ts && npm ci
npm exec -- napi build --platform --release \
  --cargo-cwd ../rust/crates/fathomdb-napi \
  --features embed-cuda,rerank-cuda,tegra-pool-experiment --js false
npm run build   # dist/ for the in-tree consumer
sha256sum fathomdb.linux-arm64-gnu.node
```

The **production build** for the Phase 0 equivalence control is the same
command without `tegra-pool-experiment`; keep both `.node` files, named by
hash. The **installed form** is staged with
`dev/plans/runs/0.8.27-slice-110-tegra/fix-verification/stage-installed.sh
<worktree> <scratch>/staging-<hash> <scratch>/consumer-<hash>`, which packs
the thin main and the `linux-arm64-gnu` platform package and installs them
offline with lifecycle scripts disabled.

**Tegra Python wheel:** `scripts/release/build-python-cuda-tegra.sh --out
<scratch>/wheels` builds with the contract's `CUDA_PYTHON_FEATURES`. For the
experiment wheel, the engineer appends `,tegra-pool-experiment` to that
exported value with an uncommitted local edit of the contract script, records
`git diff scripts/release/cuda-artifact-contract.sh` into
`<evidence>/harness/wheel-features.diff`, and reverts it before any commit.
Install into a scratch venv: `python3 -m venv <scratch>/venv-<hash> &&
<scratch>/venv-<hash>/bin/pip install --no-index <wheel>`. Slice 110 used the
same pattern (`receipt.md`, "Tegra Python wheel").

**C probes** (from `driver-isolation-evidence/results.md`, "Build"):

```sh
gcc -O2 -Wall -Wextra -std=gnu11 -I/usr/local/cuda-12.6/include pool_va_repro.c -o pool_va_repro -lcuda -lpthread
gcc -O2 -Wall -I/usr/local/cuda/include pool_lifecycle.c -o pool_lifecycle -lcuda
gcc -O2 -Wall -I/usr/local/cuda/include pool_teardown.c -o pool_teardown -lcuda
gcc -O2 -Wall -I/usr/local/cuda/include pool_capacity.c -o pool_capacity -lcuda
gcc -O2 -Wall -I/usr/local/cuda/include pool_gap.c -o pool_gap -lcuda
```

Build time is part of the Phase 0 budget (§ 12); the Slice 110 receipt does
not record it, so the engineer records it.

## 4. Harnesses

### 4.1 Reused as-is

| File (under `dev/plans/runs/0.8.27-slice-110-tegra/`) | Use |
| --- | --- |
| `driver-isolation-evidence/pool_va_repro.c` | Layout driver: `--at` (replayed blockers), `--random N --seed S`, `--pre-reserve`, `--pool-maxsize`, `--maps-dir`. R1 and the gap sweep. |
| `driver-isolation-evidence/minimal_repro.c`, `minimal_repro_runtime.c`, `cuinit_repro.c` | Reproducers for the upstream and NVIDIA packages (§ 10) and the revisit check of the plan's upstream section. |
| `pool-teardown-evidence/pool_teardown.c`, `run-variants.sh` | C7 baseline, extended with `--reset` (§ 4.6). |
| `explicit-pool-experiment/pool_lifecycle.c` | C2's destroy-while-current probe (its section D) and the churn benchmark (section E). |
| `explicit-pool-experiment/replay_layout.py`, `window_layout.py` | Turn a Node `/proc/self/maps` snapshot into `--at` blockers; classify a snapshot's window layout. |
| `driver-isolation-evidence/mapdiff.py`, `gaps.py` | Diff maps snapshots; largest reservation / largest hole per run. |
| `fix-verification/stage-installed.sh` | Installed-package staging (§ 3). |

### 4.2 `pool-consumer.mjs` (new; merges the Slice 110 consumers)

Derived from `explicit-pool-experiment/consumer.mjs` (timings, embed hash,
rerank scores, `HEAP_OBJECTS`, `MAPS_DIR`) and
`early-cuinit-verification/consumer.mjs` (`FATHOMDB_MODULE`, `IMPORT_ORDER`,
`CONSUMER_MODE`, RSS/VmSize from `/proc/self/status`). One process, one JSON
line on stdout; stderr is captured whole by the runner and the
`fdb-pool-exp` lines are parsed from it.

Inputs (environment): `FATHOMDB_MODULE` (in-tree `dist/index.js` path or
`fathomdb` when installed), `FATHOMDB_DB_SCRATCH`, `IMPORT_ORDER`
(`early`|`late`), `HEAP_OBJECTS` (live objects before open),
`HEAP_GROW_AFTER_OPEN` (objects to add between the first and second
measurement block, for R3), `CONSUMER_MODE` (`import` | `open` | `full` |
`perf` | `ingest` | `cycles`), `WARMUP_ITERS` (default 5), `TIMED_ITERS`
(default 50), `BATCH_SIZES` (default `1,8,32,128`), `INGEST_DOCS` (default
10000), `CYCLES` (open/close repetitions, default 50), `MAPS_DIR`, plus the
device policy variables, `FATHOMDB_GPU_ALLOCATION_WITNESS`, and the
`FATHOMDB_POOL_*` variables of § 2.3, which the runner sets.

Steps by mode: `import` imports and reports; `open` opens and reports; `full`
is the Slice 110 sequence (open, report, embed ×11, rerank ×2, close);
`perf` adds the warm-up and timed blocks for single embed, each batch size
(`embedBatchCls`), and rerank; `ingest` writes `INGEST_DOCS` documents in
batches of 100 through `engine.write` with the default embedder on; `cycles`
opens and closes the engine `CYCLES` times, embedding once per cycle.

JSON fields recorded per process (all modes write every field, `null` when
not applicable):

```text
outcome, failedStep, error{name,code,kind,message}
node, variant, maxSize, releaseThreshold, order, preloaded, consumerMode
pid, heapObjects, heapUsedMiB, heapGrowAfterOpen
mem{startVszMiB, importRssDeltaMiB, importVszDeltaMiB, afterOpenRssMiB,
    afterOpenVszMiB, endRssMiB, endVszMiB}
timingsMs{import, open, embedFirst, embedSteady[], embedBatch{1:[],8:[],32:[],128:[]},
          rerankFirst, rerankSteady[], ingestTotal, cyclesOpen[], cyclesClose[]}
embedderDevice, embedderReason, rerankerDevice
witness{deltaBytes, floorBytes, outcome}       # only when the witness is on
embedSha, rerankScores[]
ingest{docs, docsPerSecond}
allocMode                                      # parsed by the runner from stderr: default|explicit|sync|none
poolEvents[]                                   # parsed fdb-pool-exp lines, as objects
host{memAvailableKiB, swapFreeKiB, loadAvg1, thermalMilliC[], gpuIdleBefore}
```

The runner (§ 4.4) merges `allocMode`, `poolEvents` and `host` into the JSON
after the process exits, so the consumer stays a pure client.

### 4.3 `pool_smoke.py` (new; extends `fix-verification/python_smoke.py`)

Same `fragment` option (three blockers at 38, 68 and 98 GiB before import),
plus `--heap N` (allocate N Python objects before import, for a Python
analogue of R2), `--mode full|perf|cycles|reset`, and `--reset` which, after
the first embed, calls `cuDevicePrimaryCtxReset(0)` (or `cudaDeviceReset`
with `--reset runtime`) through `ctypes` on `libcuda.so` / `libcudart` and
then embeds and writes again (C7's co-resident reset). Output fields mirror
§ 4.2 in snake_case.

### 4.4 `run-series.sh` and `run-matrix.sh` (new)

`run-series.sh <variant> <binding:node|python> <runtime-version> <count>
<outdir> [consumer args]` runs `count` fresh processes one at a time, each
after the host-quiet check of § 1.1, writing `run-NNN.out/.err/.log`,
`run-NNN.host.json`, and a one-line summary. It is derived from
`early-cuinit-verification/run-series.sh` and adds the host record, the
`timeout`, and the `fdb-pool-exp` parsing.

`run-matrix.sh <matrix.tsv> <outdir>` executes a matrix of cells. A cell is
one line: criterion, variant, binding, runtime version, layout, `maxSize`,
threshold, consumer mode, count. For timing criteria (P1–P7) the matrix is
run in **randomised blocks**: each block runs one process of every cell in a
random order, blocks repeat `count` times, and the block order seed is
recorded (`shuf --random-source=<(yes <seed>)`). For pass/fail criteria the
cells run as series. Interleaving is what protects timing comparisons from
thermal drift, GPU rail-gating and page-cache state; the Slice 110 series
were run as blocks per variant and are therefore exposed to drift between
blocks.

### 4.5 `pool_capacity.c` (new; from `pool_lifecycle.c` section C)

Measures capacity directly instead of inferring it from where the product
fails.

Arguments: `--layout control|3pages|at LIST`, `--maxsize MiB`, `--threshold
0|max`, `--chunk MiB` (allocation granularity: 1, 16, 64, 256), `--single`
(bisect the largest single `cuMemAllocAsync` that succeeds from an empty
pool). Steps: blockers; `cuInit`; retain primary context; create pool;
install; (a) allocate `--chunk` MiB chunks until the first non-success,
touching each with `cuMemsetD8Async`; record count, bytes, `CUresult`, and
the pool's `RESERVED_MEM_CURRENT/HIGH`, `USED_MEM_CURRENT/HIGH`; (b) free
everything, synchronize, repeat (a) once to show recovery; (c) with
`--single`, bisect the largest single allocation that succeeds; (d) record
`cuMemGetInfo` before and after (sole consumer; informational). One
`RESULT` line: `layout maxsize_mib chunk_mib threshold allocated_mib
first_error reserved_high used_high single_max_mib recovered_mib
mem_free_before mem_free_after`.

### 4.6 `pool_gap.c` (new; from `pool_va_repro.c --pre-reserve` and the s4c method)

Measures the contiguous address-space need of an explicit pool as a function
of `maxSize`, instead of inferring `maxSize`/3 from one layout.

Method, from `driver-isolation-evidence/results.md` ("Threshold series"):
five blockers before `cuInit` force the driver's reservation into 15.34 GiB
quarters; then, after context creation, `cuMemAddressReserve` fences leave
exactly one unmapped hole of `G` bytes inside the window and fill every other
hole with 4 GiB-spaced blockers (the `pool_teardown.c` hole-filling step).
Arguments: `--gap MiB`, `--maxsize MiB`, `--in-unit` (place the hole inside a
driver unit with fences, as s4d/s4e did, instead of between units), `--maps-dir`.
Output: `RESULT gap_mib maxsize_mib in_unit create_rc set_rc probe_rc
new_mmap_bytes` where `new_mmap_bytes` is the size of any mapping added
between the pre-create and post-create maps snapshots.

`pool_teardown.c` gains `--reset primary|runtime`, which calls
`cuDevicePrimaryCtxReset` or `cudaDeviceReset` after the pool is created
and used, then retries `cuDeviceGetMemPool`, `cuMemAllocAsync` and allocation
from the explicit handle. (C7.)

### 4.7 `soak-consumer.mjs` and `soak-watch.sh` (new)

Loop until `SOAK_SECONDS`: embed (single and batch 32), rerank, write a batch
of 50 documents, every 60 s allocate and drop 200k JS objects and force a GC
(`--expose-gc`), every 60 s append a line to `soak.jsonl` with wall time, RSS,
VmSize, heapUsed, the last 60 s median embed and rerank latency, and the
`stats` line fields from `FATHOMDB_POOL_STATS_EVERY_S=60`. `soak-watch.sh`
samples `tegrastats` and `/proc/meminfo` every 10 s and enforces § 1.2.

### 4.8 `concurrent-runner.sh` (new)

`concurrent-runner.sh <variant> <k> <threshold> <outdir>` starts `k`
`pool-consumer.mjs` processes in `perf` mode within 100 ms of each other with
the witness **off** (`FATHOMDB_GPU_ALLOCATION_WITNESS` unset), records each
exit, and runs `tegrastats --interval 1000` for the trial's duration. The
witness is off because it reads a system-wide `cuMemGetInfo` counter and
requires a sole GPU consumer
(`src/rust/crates/fathomdb-embedder/src/gpu_witness.rs`,
`SOLE_GPU_CONSUMER_PRECONDITION`; `receipt.md` records two `insufficient_delta`
witness failures caused by a concurrent cargo test). A witness failure in R7
would be a harness artefact, not a variant failure.

### 4.9 `analyze.py` (new; from `explicit-pool-experiment/analyze_pool.py`)

Reads every `run-*.log` and `*.host.json` under a results tree and emits the
tables of § 8 as Markdown and TSV, with the statistics of § 6.

## 5. Phase order and the Phase 0 controls

Phases are ordered to front-load the measurements most likely to rule a
variant out. A variant that fails a Phase 1 gate is dropped from later phases
except where the plan needs its number for a comparison.

| Phase | Content | Rule-out it can produce |
| --- | --- | --- |
| 0 | Builds; harness smoke; production-equivalence control; `--random-seed` layout check; warm-cache check (§ 6.6) | the experiment build is not the product |
| 1 | C3 capacity and the gap sweep (§ 4.5, § 4.6); R5 lazy creation after heap growth; P1 import cost of A-load-hold and A-load-release | a `maxSize` has no safe capacity model; lazy creation fails in a heap cell; A-load cannot meet the import gate |
| 2 | C1, C2, C5, C6, C7, C8 | any correctness failure |
| 3 | R1, R2, R3, R4, R7, R8, then R6 soak on the surviving candidate(s) | robustness failure; memory-sharing failure |
| 4 | P1–P7 on the survivors against S and the default-pool reference | performance gate failure |
| 5 | Analysis, results document, decision mapping, upstream package (§ 10) | — |

**Phase 0 controls:**

1. **Production equivalence.** 10 `full` runs each of the production `.node`
   and the experiment `.node` with `FATHOMDB_POOL_VARIANT=S`, Node 25, small
   consumer, unobstructed. Pass: identical embedding hash and rerank scores,
   the same set of steps passed, and steady-embed medians per path within
   10 %. The experiment build must also show `allocMode` consistent with the
   inferred path (bimodal latency) in every run; this is the one place the
   old inference is used, to validate the new direct reading.
2. **V8 layout reproducibility** (*hypothesis*: Node's `--random-seed` seeds
   the generator V8 uses for its mmap address hints, so two processes with
   the same seed and the same program produce the same in-window layout).
   20 import-only runs, seeds 1–10 twice, `MAPS_DIR` set; compare the
   `[8 GiB, 128 GiB)` mapping start addresses of the pairs with
   `replay_layout.py`. If ≥ 9 / 10 pairs are identical, the R2–R5 cells run
   with `--random-seed=<n>` for `n` = 1..count, the same seeds in every
   variant, giving **paired** layouts across variants. If not, R2–R5 run
   unpaired and the report says so. Either way the seeds used are recorded.
3. **Warm cache.** One throwaway `full` run before every series so that the
   model files are in the page cache; it is recorded and excluded. The open
   times in `explicit-pool-experiment/analysis.txt` cluster at about 1.1,
   1.4 and 2.0 s across otherwise identical runs; this protocol records the
   GPU idle state (`gpuIdleBefore`, from `tegrastats` in the 3 s before
   launch) and the page-cache state (`fincore`-style `vmtouch` is not
   installed; use `/proc/meminfo` `Cached` delta) to attribute that, and
   interleaves variants so it cannot bias a comparison.

## 6. Measurement methods

### 6.1 Timing

- Timing points are `performance.now()` around each awaited call in the
  consumer (Node) or `time.perf_counter()` (Python), as in Slice 110.
- Per process: `WARMUP_ITERS` discarded, `TIMED_ITERS` kept, for single
  embed, each batch size and rerank. The **unit of analysis is the process**:
  iterations within a process are correlated, so each process contributes
  one median per measure.
- Summary per cell: median and IQR of the per-process medians, and the count.
- Comparison: the ratio of cell medians, with a 95 % percentile bootstrap
  confidence interval from 10 000 resamples of processes within each cell.
  When Phase 0 establishes paired layouts, R2–R5 pass rates are compared
  paired (McNemar-style discordant counts) and reported alongside.
- Path-conditional reporting: every timing table is reported **by
  `allocMode`** (`default`, `explicit`, `sync`) as well as by variant,
  because S is a mixture (in Slice 110's unobstructed small-consumer runs the
  default pool was available in 1 / 10, 6 / 30 and 5 / 10 processes;
  `explicit-pool-experiment/analysis.txt`, "default-pool availability"). The
  plan's performance gate is evaluated against S's synchronous-path runs
  (the path S takes in most Node processes) and, separately, against the
  default-pool reference.

### 6.2 Proportions

- Pass rates, path fractions and pool-creation success rates are reported
  with Wilson 95 % intervals.
- Zero failures in N runs gives a 95 % upper bound of about 3/N on the
  failure rate (the rule of three): 30 runs → 10 %, 100 → 3 %, 300 → 1 %.
  Each table states the bound its N gives. A per-cell N of 30 bounds that
  cell at 10 %; the plan's 1 % bound applies only to the pooled row.
- **The discriminating endpoint for the pool variants is the stream-ordered
  fraction, not the pass rate.** S passes behind an early import (100 / 100
  in `receipt.md`) by falling back to the slow path; a pool variant is
  better only if its `allocMode` is `default` or `explicit` in more processes
  at equal pass rate. R2–R5 therefore report, per cell: pass rate, and the
  fraction of passing processes on each `allocMode`, both with intervals.

### 6.3 Memory

- Process memory: `VmRSS` and `VmSize` from `/proc/self/status` at the
  consumer's measurement points (§ 4.2). Import deltas use the same method
  as `early-cuinit-verification/consumer.mjs`.
- Pool memory: `CU_MEMPOOL_ATTR_RESERVED_MEM_{CURRENT,HIGH}` and
  `USED_MEM_{CURRENT,HIGH}` from the `fdb-pool-exp` lines (install, each
  decide, teardown, and `stats`).
- System memory: `MemAvailable`, `Cached`, `CmaFree` from `/proc/meminfo`
  and `tegrastats` RAM, before and after each process.
- `cuMemGetInfo`: used only through the product's witness and only in
  single-process series. It is a shared, system-wide counter on this
  integrated GPU and the nvmap page pool absorbs allocations before they are
  charged (`dev/tegra-platform-reference.md` § 7.2, § 7.4;
  `gpu_witness.rs`). It is never used as a per-variant memory measure.

### 6.4 Which path a process took

Read from `allocMode` (§ 2.3), never inferred from latency after Phase 0.
Every passing process must have exactly one `decide` event per device; two
events with different `alloc_mode` in one process is a C1 failure.

### 6.5 Pool capacity (C3) and contiguous need (R5)

- Capacity: `pool_capacity.c` at `maxSize` ∈ {192 MiB, 384 MiB, 1 GiB,
  2 GiB, 3 GiB, 4 GiB, 8 GiB} (16 GiB per § 1.2), chunk ∈ {1, 16, 64, 256}
  MiB, layouts {control, 3pages}, threshold {0, max}, 5 runs per cell. The
  Slice 110 inference "usable capacity ≈ `maxSize`/3" rests on two product
  failures (192 MiB pool: 63.5 MiB used high; 384 MiB: 128.0 MiB;
  `explicit-pool-experiment/analysis.txt`). *Alternative hypothesis*: the
  product's failures were single large allocations (model weights at open,
  cross-encoder weights at the first rerank) that did not fit the remaining
  reservable space, and the apparent 1/3 is a granularity effect. The chunk
  sweep separates these: a cap independent of chunk size supports the 1/3
  rule; a cap that moves with chunk size or with `--single` does not. Fit
  `capacity = a·maxSize + b` on the measured points and report the residuals;
  the plan's C3 pass is "predicts within 5 % at every size".
- Contiguous need: `pool_gap.c` with gaps G ∈ {1, 2, 4, 8, 12, 16} GiB and
  `maxSize` swept in 1 GiB steps up to 48 GiB (3 runs per point, stop at the
  first failure plus two confirmations). The Slice 110 inference
  "need ≈ `maxSize`/3" comes from one layout (largest gap 14.66 GiB:
  creation succeeded to 44 GiB and failed from 46; control to 48 GiB;
  `driver-isolation-evidence/results.md`, "Explicit pools"). The control's
  own 48 GiB ceiling is unexplained by a gap rule (its reservation is
  61.36 GiB) and is reported as a separate finding (*hypothesis*: a driver
  cap related to device memory). The output is a table need(G) and the
  largest `maxSize` that is safe for the smallest gap the R5 heap cells
  produce.

### 6.6 Confounds and how each is handled

| Confound | Handling |
| --- | --- |
| Host load, other GPU users | § 1.1 refusal; GPU lock; one process at a time outside R7 |
| Thermal and clock drift, GPU rail-gating | Randomised blocks for timing (§ 4.4); `gpuIdleBefore` recorded and the first-after-idle effect reported separately (Slice 110: +110 ms on the first `cuInit` after idle, `receipt.md`) |
| Page cache / model files | Warm-cache run per series (§ 5) |
| Witness's system-wide counter | Witness on only in single-process series; off in R7 and R6 |
| V8 heap randomness | Paired seeds if Phase 0 confirms them; otherwise larger N and Wilson intervals; layouts replayed into C with `replay_layout.py` for deterministic R1 cells |
| Node version | 24.15.0, 25.9.0, 26.10.0 as separate cells; never pooled across versions in a gate |
| In-tree vs installed | In-tree for the bulk; installed form for C8 and the final confirmation of the candidate, as Slice 110 did |
| Mixture in S | Path-conditional tables (§ 6.1) |
| Experiment build vs product | Phase 0 equivalence control |

## 7. Run matrix

`maxSize` set, unless a row says otherwise: {1, 3, 8, 16} GiB; threshold
{0, max}. "Pool variants" means A-load-hold, A-load-release, A-first-use, B
(Node) and A-first-use, B (Python). Layouts: *unobstructed* (small
consumer), *3pages* (the three 4 KiB blockers; in Node via a tiny preload
that maps them before import is not possible from JS, so 3pages is a C and
Python layout only), *heap-N* (import first, then N live objects: 0, 100k,
400k, 1M, 4M), *late-N* (N objects, then import), *replayed* (C blockers at
a captured Node layout's mapping starts).

| Row | Variants | Binding / runtime | Layouts | `maxSize` × threshold | Runs | Phase |
| --- | --- | --- | --- | --- | --- | --- |
| Phase 0 equivalence | S (prod), S (exp) | Node 25 | unobstructed | — | 10 + 10 | 0 |
| Phase 0 seeds | S (exp), import-only | Node 25 | heap-400k | — | 20 | 0 |
| C3 capacity | C probe | — | control, 3pages | 7 sizes × 2 thr × 4 chunks | 5 per cell = 560 | 1 |
| Gap sweep | C probe | — | forced quarters + one gap | 6 gaps × ≤48 sizes | ≤ 3 per point, early stop | 1 |
| R5 lazy creation | A-first-use, B | Node 24/25/26 | heap-0/100k/400k/1M/4M | {3, 16} GiB × thr 0 first; {1, 8} if the rule is unclear | 30 per cell: 2 × 3 × 5 × 2 × 30 = 1800 | 1 |
| P1 import cost | S, A-load-hold, A-load-release | Node 25, import-only, installed | unobstructed | 3 GiB × 0 | 20 each, interleaved | 1 |
| C1 provenance | all | Node 25; vendored unit tests on every target | unobstructed, heap-400k | {3, 16} × 0 | 50 per variant × size | 2 |
| C2 lifetime | all pool variants | C probe (`pool_lifecycle.c` D) + Node `cycles` mode | 3pages (C); unobstructed (Node) | 3 GiB × 0 | C: 20; Node: 10 processes × 50 cycles per variant | 2 |
| C5 zero-length | all | vendored tests + embedder zero-element tests, both bindings | — | 3 GiB | test suites | 2 |
| C6 multi-device | all | pure unit tests | — | — | tests | 2 |
| C7 resets | S, A-first-use, B | C (`pool_teardown.c --reset`) 20 per case; Python `--reset` 20 per variant | 3pages, free-gap | 3 GiB × 0 | 20 × cases | 2 |
| C8 parity | S, A-first-use, B | Python wheel and installed Node 25 | unobstructed, fragment/3pages (Python) | 3 GiB × 0 | 20 per binding × variant | 2 |
| R1 synthetic | A (any), B as `--pool-maxsize` in C | C probe | 3pages; random-18 seeds 1–100; n-sweep n = 3, 5, 6, 7; free-gap | {3, 16} GiB | ≥ 300 per size | 3 |
| R2 real heaps | survivors + S | Node 24/25/26 | heap-0/100k/400k/1M/4M | chosen size × chosen thr | 30 per cell; ≤ 3 variants × 450 | 3 |
| R3 growth in use | survivors + S | Node 24/25/26 | heap-0 then `HEAP_GROW_AFTER_OPEN`=1M | chosen | 30 × 3 per variant | 3 |
| R4 late import | survivors + S | Node 25; with and without `node --import` | late-400k, late-1M | chosen | 30 per cell | 3 |
| R7 concurrency | survivors | Node 25, `perf` mode, witness off | unobstructed | chosen size × {0, max} | 10 trials × k ∈ {2, 4, 8} × 2 thr | 3 |
| R8 threshold | survivors | Node 25, `perf` then 60 s idle, then `teardown` stats | unobstructed | chosen × {0, max} | 20 per thr | 3 |
| R6 soak | final candidate (8 h each for ≤ 2 survivors first) | Node 25, 2 processes sequentially or k = 2 concurrently with witness off | unobstructed | chosen | 24 h | 3 |
| P1–P4, P6 | survivors, S, default-pool reference (S runs with `allocMode=default`) | Node 24/25/26, `perf`, randomised blocks | unobstructed | chosen | 20 per variant × version | 4 |
| P5 ingest | survivors, S | Node 25, `ingest` 10 000 docs | unobstructed | chosen | 5 per variant | 4 |
| P7 Python | A-first-use, B, S | wheel, `perf` and ingest | unobstructed | chosen | 20 per variant | 4 |
| R9 | — | other boards | — | — | not available on this host; declared unmeasured | — |

"Chosen size" is fixed after Phase 1 as the smallest `maxSize` whose measured
capacity (§ 6.5) is at least twice the largest workload high-water mark
measured in Phase 1 `perf` runs at batch 128 with rerank, and whose
contiguous need is below the smallest gap seen in the R5 heap cells; the
value and its derivation go in the results document. "Chosen threshold" is
fixed after R8.

## 8. Analysis and reporting

### 8.1 Tables

1. Environment and artifacts (one table, from the series headers).
2. Phase 0 equivalence: per build, pass count, hash identity, steady medians
   by path; the inferred-vs-read path agreement count.
3. Capacity: per `maxSize` × chunk × threshold × layout, allocated MiB at
   first error, `used_high`, `reserved_high`, single-max; the fitted model
   and residuals; the verdict on the 1/3 rule.
4. Contiguous need: need(G) and the safe `maxSize` per gap; the control
   ceiling.
5. R5: per variant × Node × heap × size, pool-creation success (Wilson),
   path fractions, and the maps-derived largest gap per run; the predictive
   rule and its misses.
6. P1: import ms and RSS/VmSize deltas per variant with intervals; the
   import-cost gate verdict per A-load sub-variant.
7. Correctness: one row per C-row with sample sizes, failures, and the typed
   error kinds seen.
8. Robustness R1–R4, R7, R8: pass rate, path fractions, intervals, and the
   3/N bound stated; R7 memory sums; R8 held memory versus latency.
9. R6: hourly RSS, pool reserved, median latencies; drift against the
   first-hour high.
10. Performance P2–P7: by variant and by `allocMode`, medians, IQR, ratio
    against S-sync and against the default-pool reference with bootstrap
    intervals; the gate verdicts.
11. Output identity: embedding hash and rerank score sets by variant.
12. **Decision table:** one row per plan decision-rule clause, the criteria it
    depends on, and PASS / FAIL / UNMEASURED with the table that supports it.

### 8.2 Mapping onto the decision rule

The plan's rule (as corrected in this review) reads: ship B if C1–C8, R1–R8
on this host and the performance gates pass and R5 shows lazy creation at the
chosen size in every heap cell; ship **A-load-release** instead if B and
A-first-use fail only R5 and A-load-release passes everything including the
import-cost gate; otherwise keep the 0.8.27 synchronous fallback. The decision
table (§ 8.1 item 12) is the only input to that rule; narrative in the
results document may explain a row but never override it. A row whose N gives
a bound weaker than the claim it supports is marked PASS (bounded at x %),
never plain PASS.

### 8.3 Results document

`<evidence>/results.md`, with the frontmatter of this file's kind, states
the host, the artifacts, each table, the decision table, what was not
measured, and the open questions. It is the input to a 0.8.28 release-state
ruling; it does not itself rule.

## 9. Evidence directory layout and retention

### 9.1 Committed (curated)

```text
dev/plans/runs/0.8.28-pool-study/
  results.md                      # § 8.3
  env.txt                         # host, driver, toolchain, Node, Python
  artifacts.sha256                # every .node, wheel, C binary used
  harness/                        # pool-consumer.mjs, pool_smoke.py, run-series.sh,
                                  # run-matrix.sh, concurrent-runner.sh, soak-*.{mjs,sh},
                                  # pool_capacity.c, pool_gap.c, analyze.py,
                                  # pool_teardown.c (with --reset), wheel-features.diff
  patches/                        # cudarc mem_pool + decision rule diff; FathomDB policy diff
  matrices/*.tsv                  # the exact cells run, with seeds
  summaries/*.txt                 # analyze.py output per phase
  samples/<series>/               # at most 2 full run logs per series (one pass, one fail if any)
  seeds-and-layouts/              # Phase 0 seed check summary; one maps snapshot per R5 cell
```

Slice 110's retention practice applies (`receipt.md`, "Evidence retained and
omitted"): reproducer sources, analysis scripts, patches, summaries and
samples are committed; raw per-run logs, maps archives and `strace` output
are not.

### 9.2 Untracked (left on the host)

`<scratch>/pool-study/logs/<phase>/<series>/run-*.{out,err,log,host.json}`,
`maps/`, `strace/`, build logs, databases. Keep until the 0.8.28 ruling is
recorded; `results.md` names the host path pattern with `<scratch>`.

### 9.3 Redaction gate

Before `git add`, grep `<evidence>` recursively for the host's home-directory
prefix and for `$USER`; both must print nothing. Scripts refer to `<worktree>` and `<scratch>` as placeholders
and are archived as run records, as Slice 110 did with `*.sh.txt`.

## 10. Upstream track (preparation only)

Nothing in this section is posted by anyone but the owner. The study
assembles the following under `<evidence>/upstream/` so that a comment on
cudarc issue #536 (open, no maintainer reply; plan § "Upstream state") can be
made from facts the maintainer can run:

1. **Minimal reproducer:** `minimal_repro.c` with its 20 / 20 and 0 / 20
   outputs (`driver-isolation-evidence/results.md`), one paragraph on the
   host, no FathomDB internals. The maintainer requires representative
   failing tests or benchmarks (plan § "Upstream style", from issues #555,
   #556, #595).
2. **A test in cudarc's `mod tests` style** in the `safe` layer: create a
   context, create a `CudaMemPool` with `maxSize` 1 GiB, install it, allocate
   and free through the ordinary `CudaStream::alloc`, read `used_high`, and
   assert the slice round-trips. It must compile across the `cuda-*`
   features (the `max_size` field under `cuda-12020`+), on Windows and under
   the `no-std` clippy check (plan § "Upstream state"); it must skip cleanly
   when no device is present, as the existing GPU tests do.
3. **A benchmark:** a small program in the style of the vendored crate's
   `examples/` (`third_party/cudarc-0.19.7/examples/01-allocate.rs` and
   siblings) that times alloc/free churn with the default pool, an installed
   explicit pool and synchronous allocation, derived from
   `pool_lifecycle.c` section E; run on this Orin and reported with medians
   and intervals.
4. **A draft comment** for #536, written in the maintainer's terms:
   observed failure, the reproducer, the proposed minimal primitive
   (`CudaMemPool` + install; no `alloc_from_pool`, no new `CudaSlice`
   variant, per the maintainer's reservation on #594), what FathomDB keeps on
   its side, and a question on preferred shape. No claims about merge
   timing.
5. **The revisit check** the plan's upstream section asks for: rerun
   `minimal_repro.c` and `pool_teardown.c` against the current L4T/driver
   at the start of the study (Phase 0) and record whether the failure still
   reproduces; if it does not, the study stops and reports that instead.

The question whether the 0.8.27 default-pool fallback itself is expressible
with upstream's current API is answered by inspection in Phase 5: list the
calls the fallback needs (`cuDeviceGetDefaultMemPool`, a per-device decision
before the first allocation, the zero-length rule) against `has_async_alloc()`
and the result-level pool functions, and record what residual patch remains.

## 11. Open questions for the owner

1. Should the Tegra Python wheel gain an import-time `cuInit` (and, for the
   A-load variants, pool creation) so that Python can run the same variant
   set as Node? Without it, Python's best case is A-first-use or B.
2. Is the deferred private-pool variant (P) wanted at all, given the
   upstream shape? If C7/R7 force it, it adds a cudarc allocation-path change.
3. The performance gate compares against "S": confirm that the S reference
   is the synchronous path (what most Node processes get) and that the
   default-pool reference is a secondary comparison.
4. The 24 h soak costs a GPU-exclusive day; is an 8 h soak per surviving
   candidate plus 24 h on the final candidate acceptable?
5. Whether to run the 16 GiB exhaustion cell at all on this shared host
   (§ 1.2).
6. The todos-ledger id the plan cites for tracking
   (`TC-1c70e523-38a0-4bb7-b238-0307d2b0489f`) is not present in this
   worktree's `dev/todos-and-considerations-ledger.jsonl`; the revisit entry
   `TC-9fef1b7c-4442-4c77-b925-992f338c9aac` is. Confirm the tracking id.

## 12. Time and resource estimate

Per-process wall time, from Slice 110: a `full` Node run is about 2–3 s
(`sync-repair-experiment/analysis.txt`, `wall` column: 2.0–2.7 s); a `perf`
run with 5 + 50 timed iterations at four batch sizes and rerank is estimated
at 15–25 s; `ingest` of 10 000 documents at 10–25 ms per embed is 2–5 min; a C
probe is 1–5 s; an exhaustion probe up to 8 GiB is under 60 s. Builds: the
Node CUDA addon and the wheel each compile Candle's CUDA kernels; budget
30–90 min each on this host (unrecorded in Slice 110; Phase 0 records it).

| Phase | GPU-exclusive time | Notes |
| --- | --- | --- |
| 0 | 3–5 h | two addon builds, one wheel, harness smoke, 40 runs, revisit check |
| 1 | 6–8 h | C3 560 probes (~1 h); gap sweep (~1 h); R5 1800 runs × ~8 s (~4 h); P1 60 runs |
| 2 | 4–6 h | C1 500 runs (~1 h); C2 C probes + 40 cycle processes (~1 h); C7 and C8 (~1.5 h); test suites |
| 3 | 10–14 h + soak | R1 600 C runs (~1 h); R2 ≤ 1350 runs (~3.5 h); R3 270 (~1 h); R4 360 (~1 h); R7 60 trials (~1 h); R8 (~0.5 h); R6 8 h × ≤ 2 then 24 h |
| 4 | 3–4 h | P1–P4/P6 ≤ 240 `perf` runs (~1.5 h); P5 ≤ 15 ingests (~1 h); P7 60 Python runs |
| 5 | engineer time | analysis, results document, upstream package |

Total: about 30–40 h of GPU-exclusive time plus 24–40 h of soak, so roughly
one to two calendar weeks with the host shared, if no phase rules a variant
out early. A Phase 1 rule-out (for example, lazy creation failing in a heap
cell for both A-first-use and B) removes most of Phases 3–4 for those
variants and shortens the study to about half.
