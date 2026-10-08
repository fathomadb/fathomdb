---
title: FathomDB 0.8.28 Slice 30 — Tegra private CUDA memory pool, design
status: PROPOSED (revision 2, 2026-10-08; design-review findings 1–14 resolved)
target_release: 0.8.28
observed_on: 2026-10-08
---

# Slice 30 design: Tegra private CUDA memory pool

This design implements D28-08 under the [Slice 30 plan](plan.md).

- **Code citations** are `path:line` on `b11e4523d`. The source does not
  differ from `cb9594f44`.
- **Owner rulings** are cited by number from
  `dev/plans/0.8.28/prework/tegra-cuda-memory-pool-study.md`.
- **Plan decisions** are cited as SD-n.
- **Review findings** are cited as F-n (`design-review.md`).

## 1. Rule

1. **Who gets the pool.** A process with all of these properties makes one
   allocator decision at its first CUDA device build:
   - built with the `tegra-pool` feature;
   - running on aarch64 Linux;
   - an integrated GPU of a measured class (§ 2.1);
   - its module-load early `cuInit` ran.

   If every gate passes, the decision does three things:
   1. creates one private CUDA memory pool, with release threshold 0 (P-first-use; rulings 7, 12, 30, 38);
   2. allocates and frees a 4-byte probe on it;
   3. builds the first private context.

   The device's current pool is never read or changed.
2. **Never refuse at the decision (SD-3).** The process takes the 0.8.27
   path, which is cudarc's own default-pool-or-synchronous decision, with a
   recorded reason (§ 2.6), in any of these cases (F-7):
   - early `cuInit` was opted out, failed, skipped or not run at load;
   - a gate is off;
   - a setting is invalid;
   - pool creation fails;
   - the probe fails;
   - the first private context build fails;
   - the decision panics.
3. **Fail closed afterwards.** Once the decision is `Private`, every later
   CUDA device for that ordinal is built on the pool. If such a build fails,
   it returns the typed error of § 2.4 (`cuda_pool_exhausted`,
   `cuda_context_lost`, or `cuda_private_build_refused`); it never builds a
   non-private context.
   - **Why:** a buffer must always be freed by the API that allocated it.
   - **Another ordinal** takes the 0.8.27 path. A buffer never crosses
     devices, so per-device allocators are safe. No Tegra has two GPUs, so
     this is tested with pure tests only (F-7).
4. **Without `tegra-pool`, the allocator decision is 0.8.27's (SD-5,
   F-5).** The device-creation function is `Device::new_cuda`, and no pool
   symbol is compiled. Other parts of this slice ship in every build:
   - the typed `rerank()` errors (§ 2.4);
   - the `cuda_allocator` report (`not_built`);
   - `doctor cuda-allocator`.

   The Python early-`cuInit` hook is compiled only with `tegra-pool`
   (§ 3.4).

## 2. Parameters

### 2.1 Gates and sizing (ruling 31, SD-2, DQ-1)

The decision reads these device facts with the primary context retained:

- `CU_DEVICE_ATTRIBUTE_INTEGRATED`;
- `CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED`;
- `CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR` and `_MINOR`;
- `cuDeviceTotalMem`.

The Tegra identity is `/etc/nv_tegra_release` **or** a
`/proc/device-tree/compatible` entry containing `nvidia,tegra`, the
two-source check of `tegra_fragmented_va_cuda.rs:74-83` (F-12).

Gates are evaluated in order. The first that is off gives the reason.

| Gate | Off when | Reason | Lifted by `on` |
| --- | --- | --- | --- |
| Mode | `FATHOMDB_POOL_MODE=off` | `mode_off` | — |
| Setting | any pool setting malformed | `invalid_setting` | no |
| Early `cuInit` | `ModuleLoadInit` is not `Ran` | `cuinit_opted_out`, `cuinit_skipped_cpu_only`, `cuinit_driver_absent`, `cuinit_failed`, `cuinit_not_at_load` | no |
| Integrated | attribute ≠ 1 | `discrete` | no |
| Pool support | attribute ≠ 1 | `no_pools` | no |
| Size floor | 2 GiB > total / 4 | `too_small` | no (SD-2) |
| Tegra identity | neither source present | `not_tegra` | yes |
| Measured class | not (compute capability 8.7 and total ≥ 48 GiB) | `unmeasured_class` | yes |

The measured class admits only the AGX Orin 64 GB, which reports
61.36 GiB at sm_87:

- the 32 GB Orin reports about 30 GiB;
- Thor is sm_110;
- GB10 is sm_121.

Size = clamp(total / 20, 2 GiB, 3 GiB), which is 3 GiB on the 64 GB Orin.
`FATHOMDB_POOL_MAXSIZE` replaces it only after every gate passes.

### 2.2 Settings (SD-7)

| Variable | Default | Values |
| --- | --- | --- |
| `FATHOMDB_POOL_MODE` | `auto` | `auto`, `on`, `off` |
| `FATHOMDB_POOL_MAXSIZE` | derived (§ 2.1) | bytes, `<n>G` (GiB) or `<n>M` (MiB); at least 32 MiB and at most device total; out of bounds is `invalid_setting` |
| `FATHOMDB_POOL_RELEASE_THRESHOLD` | `0` (ruling 30) | `0`, `max` |

- They are read once, inside the decision. A later change has no effect,
  and the docs say so.
- A malformed value turns the pool off with `invalid_setting`, naming the
  variable. It never aborts.
- They live in FathomDB, never in cudarc.

### 2.3 Named constants (each with a pure test)

`POOL_SIZE_DEVICE_DIVISOR` = 20 · `POOL_SIZE_FLOOR` = 2 GiB ·
`POOL_SIZE_CEILING` = 3 GiB · `POOL_FLOOR_MAX_SHARE_DIVISOR` = 4 ·
`MAXSIZE_MIN` = 32 MiB · `MEASURED_CLASS_MIN_TOTAL` = 48 GiB ·
`MEASURED_CLASS_COMPUTE_CAPABILITY` = (8, 7) · `PROBE_BYTES` = 4.

The floor and ceiling rest on two measurements from one AGX Orin: a pool
holds ceil32(`maxSize`/3), and it grows in 32 MiB chunks. The docs cite
them as evidence, not as logic.

### 2.4 Errors

**Three kinds.**

| Kind (stable code) | Raised when | Payload |
| --- | --- | --- |
| `cuda_pool_exhausted` | a `Private` process gets `CUDA_ERROR_OUT_OF_MEMORY` | `ordinal`, `max_size_bytes`, `message` |
| `cuda_context_lost` | § 3.5 detection | `recorded_context_id`, `current_context_id` (nullable), `driver_error`, `operation` |
| `cuda_private_build_refused` | rule 1.3: a later private build fails for another reason | `ordinal`, `message` |

**Where they are added.**

- **`fathomdb-embedder-api::EmbedderError`** gains `CudaPoolExhausted`,
  `CudaContextLost` and `CudaPrivateBuildRefused`, and becomes
  `#[non_exhaustive]` (§ 2.7).
- **`fathomdb-embedder`:**
  - `RerankerDevicePolicyError` gains the same three, and `kind()` covers
    them.
  - `EmbedderLoadError` and `RerankerLoadError` gain them for load-time
    failures (F-2).
  - `candle_reranker.rs:166` stops mapping a device-build failure to
    `ModelDeserialize`.
- **Engine.** `EngineError` and `EngineOpenError` gain matching variants,
  with stable codes equal to the kind names.

**Binding classes.** Each class extends the binding's `EmbedderError`.

| Kind | napi / TS (code; class) | py / Python | `fathomdb-sdk` | CLI exit |
| --- | --- | --- | --- | --- |
| `cuda_pool_exhausted` | `CUDA_POOL_EXHAUSTED`; `CudaPoolExhaustedError` | `CudaPoolExhaustedError` | `ErrorKind::CudaPoolExhausted` | 70 |
| `cuda_context_lost` | `CUDA_CONTEXT_LOST`; `CudaContextLostError` | `CudaContextLostError` | `ErrorKind::CudaContextLost` | 70 |
| `cuda_private_build_refused` | `CUDA_PRIVATE_BUILD_REFUSED`; `CudaPrivateBuildRefusedError` | `CudaPrivateBuildRefusedError` | `ErrorKind::CudaPrivateBuildRefused` | 70 |

The SDK kinds have parent `Embedder`, and each CLI exit is its own row in
`cli.md`.

**Payload access.**

- Python and TS expose the payload as typed attributes.
- `fathomdb-sdk` gains `Error::cuda_details() -> Option<&CudaErrorDetails>`
  (F-3). `Error::sdk(kind, message)` cannot carry fields, so the SDK
  constructs these three kinds with details.

**Explicit arms (F-3, F-10).** Every match that maps these enums gets an
explicit arm for each new variant, ahead of any `_` or generic arm. The
places:

- py `errors.rs:81-82,429`;
- napi `errors.rs:157,469`;
- SDK `error.rs:58,89`;
- engine `embedding.rs:107` and `errors.rs:144-145`;
- `tests/eu5b_lockflip.rs:151`;
- CLI `engine_error_code`.

A mapping test per variant per surface proves that no catch-all swallows
the variant.

**Every path is typed (ruling 33).** There is no `WriteValidation`, Debug
string or neutral score on any of these:

- engine `embed`, batch embed and search rerank;
- module-level CLS `embed_batch_cls` / `embedBatchCls`, load and forward;
- module-level `rerank()`, load and forward;
- engine open (`open.rs:582ff`), which today wraps everything in
  `Failed`.

`rerank_passages` (`rerank.rs:98`) returns `RerankPassagesError`
(`WriteValidation { message }` | `Reranker(RerankerDevicePolicyError)`)
in place of `String`. Non-finite scores stay `WriteValidation`, and
device-policy errors keep their existing typed classes, no longer
stringified (plan C-6).

**The reranker under `auto` (F-1).**
`CandleCrossEncoder::score_batch` (`rerank.rs:328-339`) first classifies
the batch error through `forward_error`. Pool exhaustion and context loss
then propagate as errors whatever the policy. Only other errors keep the
per-pair neutral-score fallback, and `score` (:315) does the same per pair.

**Recovery.**

- After `cuda_pool_exhausted` the device stays CUDA, and the next request
  runs normally (CB4).
- The reranker singleton (`rerank.rs:279-298`) changes from `OnceLock` to
  `Mutex<Option<Singleton>>`, held only across the load (F-2). It caches
  `Loaded`, `DevicePolicy` and genuine `Unavailable` (no weights or
  network).
  - **What it does not cache:** load-time pool exhaustion, context loss
    and private-build refusals. They are returned, and the next call
    retries.
  - **CLS singletons** (napi `embedding.rs:13`, py `:154`, SDK
    `standalone.rs:80`) already cache only success.

### 2.5 Fork (hazard H-1, F-13)

The CUDA driver is not fork-safe once initialised. With the Python hook,
a `multiprocessing` child forked after `import fathomdb` cannot use CUDA,
and it will see a driver error. In 0.8.27 such a child worked if the
parent had not used the GPU. The fix is documentation plus the existing
opt-out:

- set `FATHOMDB_CUDA_EARLY_INIT=off`; or
- use the `spawn` or `forkserver` start methods (the default on Linux from
  Python 3.14).

A fork while another thread is inside FathomDB's CUDA initialisation can
leave the child blocked on a copied lock; the docs say not to fork during
initialisation. G2 records the forked child's outcome on device, with and
without the hook. The hook ships only in `tegra-pool` builds, and the
owner rules on it before S30-T5 (DQ-2).

### 2.6 Report (SD-8, F-5, F-6, F-12)

`CudaDeviceInfo` (`device_policy.rs:95`) becomes `#[non_exhaustive]` and
gains `cuda_allocator: Option<CudaAllocatorReport>`.

```rust
#[non_exhaustive]
pub struct CudaAllocatorReport {
    pub path: Option<CudaAllocatorPath>,   // Private | DefaultPool | Synchronous; None = unknown
    pub reason: CudaAllocatorReason,       // § 2.1 reasons + private_pool, pool_create_failed,
                                           // probe_failed, private_build_failed, decision_panicked,
                                           // other_ordinal, not_built
    pub pool_max_size_bytes: Option<u64>,
    pub release_threshold: Option<ReleaseThreshold>, // Zero | Max ("0" / "max" in JSON and napi)
    pub module_load_init: ModuleLoadInit,  // Ran | OptedOut | SkippedCpuOnly | DriverAbsent | Failed(u32) | NotAtLoad
}
```

**When it is `None`, and when it reports `not_built`:**

| Build | `cuda_allocator` |
| --- | --- |
| aarch64 Linux, `embed-cuda`/`rerank-cuda`, no `tegra-pool` | `Some { path: None, reason: not_built, .. }` (stock cudarc has no `alloc_mode()`) |
| aarch64 Linux, CUDA features, `tegra-pool` | `path` from the context actually built, through `alloc_mode()` |
| Every other target | `None` |

`fathomdb-sdk` re-exports the types (`lib.rs:60`); napi, py, TS and Python
map them.

**`fathomdb doctor cuda-allocator [--json]` (F-6).** This is a new sibling
record, `fathomdb.doctor.cuda-allocator.v1`, following the
`reranker-gpu.v1` and `platform.v1` precedent. `doctor gpu` v1 is
unchanged.

- **JSON keys, in order:** `schema_version`, `built`,
  `module_load_init`, `mode`, `path`, `reason`, `pool_max_size_bytes`,
  `release_threshold`, `cuda_context_state`.
- **Exit:** 0, or 70 on an internal failure.
- **Order of work:**
  1. Return a frozen `not_applicable` result for an explicit `cpu` policy,
     an affirmed `arm64_sbsa` platform, or a build without a CUDA feature,
     before any CUDA call.
  2. Otherwise run the early-`cuInit` helper, so the process decides as an
     SDK process would.
  3. Run the device probe and report the decision.

  `doctor gpu` itself does not change.

### 2.7 SD-9: `fathomdb-embedder-api` takes one breaking change (DQ-3, F-10)

`EmbedderError` (`fathomdb-embedder-api/src/lib.rs:42`) is a public enum
without `#[non_exhaustive]` in the axis-E crate, version 0.6.1.

- **Decision:**
  - Add the three variants unconditionally. A feature-gated variant would
    change a public enum through Cargo feature unification.
  - Mark the enum `#[non_exhaustive]`.
  - Bump axis-E to 0.7.0.
- **ADR.** It amends `ADR-0.6.0-embedder-protocol.md` (index row 7) for
  the error set. It also states that caller-supplied embedders may return
  the new variants.
- **Other public breaks**, recorded in the ADR:
  - `CudaDeviceInfo` (new field; now `#[non_exhaustive]`);
  - `RerankerDevicePolicyError`, `EmbedderLoadError` and
    `RerankerLoadError` (new variants);
  - `fathomdb_engine::rerank_passages` (error type changes from `String`).

## 3. Components

### 3.1 cudarc primitive (vendored, upstream-first; ruling 38)

**`mem_pool.rs` (new).** `CudaMemPool` with these members:

- `create(ordinal, &MemPoolProps { max_size, release_threshold })`;
- `attribute`;
- `trim_to`;
- `raw`;
- `Drop`, which calls `cuMemPoolDestroy`.

It sits on the result-layer calls merged upstream in #544. It does not
port the study's `install`, `InstalledPools`, `AllocMode::Explicit` or the
A / B arms.

**`core.rs`.**

- `pub enum AllocMode { Default, Synchronous, Private }`.
- `CudaContext::new_with_mem_pool(ordinal, Arc<CudaMemPool>)`. It retains
  the primary context as `new` does, and checks that the pool's device
  matches.
- `alloc_mode()` and `mem_pool()` accessors.
- `Private` branches: allocation uses `cuMemAllocFromPoolAsync`, and free
  uses the ordinary `cuMemFreeAsync`.

The private constructor does not call `select_async_alloc` (`core.rs:155`)
or touch `PROCESS_ALLOC_MODE` (:141). The 0.8.27 decision of every other
context on the device, including a co-resident cudarc user's, is
unaffected (C9).

**Zero length (F-4).** A zero-byte request goes to the driver.
`cuMemAllocFromPoolAsync(0)` returns success with a null pointer that can
be freed (results § 10.2, 60/60), so there is no special case. That keeps
the primitive identical to what goes upstream (cudarc#194). The C5 test
asserts a null pointer that is freed without error.

**Lifetime.** The pool outlives every allocation through
slice → stream → context → `Arc<CudaMemPool>`.

**Feature marker (F-9).** The vendored crate declares a no-op feature,
`fathomdb-private-pool = []`. `fathomdb-embedder`'s `tegra-pool` enables
`cudarc/fathomdb-private-pool` through a direct optional dependency on
`cudarc = "=0.19.7"`, which unifies with Candle's. Built against stock
crates.io cudarc, Cargo then fails with "package `cudarc` does not have
feature `fathomdb-private-pool`", not an opaque missing-symbol error.

**`FATHOMDB-PATCH.md`** gives each item an upstream status and a removal
path:

| Item | Status |
| --- | --- |
| 1. Allocator fallback | local-only |
| 2. Per-device decision | local-only |
| 3. Zero-length sync | local-only |
| 4. Tests | local-only |
| 5. Private pool primitive | proposed (not posted) |
| 6. Feature marker | local-only, removed with item 5 |

`scripts/pinned-override-rot.json` lists exactly this set. The tests run
through `scripts/tests/test_vendored_cudarc.sh`.

### 3.2 Candle (SD-1)

`CudaDevice::from_context(Arc<CudaContext>)` and
`Device::new_cuda_from_context` are on
`coreyt/candle-fathomdb@fathomdb/0.8.28-cuda-from-context`, built from
`1aefdd008` in three commits:

1. `73e267d0`: the test, red;
2. `5b74532e`: the implementation, green on the Orin;
3. `25368139`: core, nn and transformers move to 0.10.3; `candle-kernels`
   stays at 0.10.2.

The device uses the context's default stream, so every Candle allocation
goes through the context's allocator.

### 3.3 Policy module, `fathomdb-embedder/src/cuda_pool_policy.rs` (F-7)

**The pure part** compiles on every host:

- the setting parsers;
- `DeviceFacts`;
- `decide(mode, facts, init, settings) -> Plan`;
- the reasons;
- `is_pool_exhaustion`;
- the report types.

**The driver part** compiles under
`cfg(all(feature = "tegra-pool", target_os = "linux", target_arch = "aarch64", any(feature = "embed-cuda", feature = "rerank-cuda")))`.

```text
enum Decision {
    Private { ordinal, pool: Arc<CudaMemPool>, first_ctx: Arc<CudaContext>, ctx_id: u64 },
    Fallback { reason },
}
static DECISION: OnceLock<Decision>   // one state, set once
pub(crate) fn new_cuda_device(ordinal) -> candle_core::Result<Device>
pub(crate) fn forward_error(candle_core::Error, what) -> EmbedderError
pub(crate) fn allocator_report() -> Option<CudaAllocatorReport>
```

**`new_cuda_device(ordinal)`:**

1. `DECISION.get_or_init(|| catch_unwind(decide_and_build).unwrap_or(Fallback { decision_panicked }))`.
   - The decision reads the settings and the module-load record.
   - It creates the pool, probes it, and builds the first private context
     on it.
   - Any failure is `Fallback { reason }`. It never panics out, and a
     panic cannot poison a `Once`.
2. **`Fallback`, or another ordinal:** `Device::new_cuda(ordinal)`, the
   0.8.27 path.
3. **`Private`:**
   - **The first build** wraps the stored `first_ctx` through
     `Device::new_cuda_from_context`. The first context counts as a live
     use of the pool.
   - **Later builds** call `CudaContext::new_with_mem_pool` and wrap the
     result.
   - **On failure:** `CUDA_ERROR_OUT_OF_MEMORY` gives
     `CudaPoolExhausted`; a § 3.5 loss gives `CudaContextLost`; anything
     else gives `CudaPrivateBuildRefused`.

There is no `MODE` mutex and no lock is held across a device build. After
the decision, the state is immutable, so there is no lock order to state
(F-7).

**Call sites.** Every Candle CUDA device FathomDB builds goes through
`new_cuda_device`:

- `candle_bge.rs:260` (probe), `:382` (witness) and `:432` (load);
- `candle_reranker.rs:121` (probe) and `:166` (load).

Every forward error goes through `forward_error`, and the reranker maps it
into `RerankerDevicePolicyError`.

**Excluded.** `tegra_fragmented_va_cuda.rs:194,208` deliberately calls
`Device::new_cuda`. It is a 0.8.27 fallback test and is not run with
`tegra-pool` (F-14).

Without the driver part:

- `new_cuda_device` is `Device::new_cuda`;
- `forward_error` maps to `Failed` as today;
- `allocator_report` gives `not_built` on aarch64 Linux CUDA builds and
  `None` elsewhere.

### 3.4 Early-`cuInit` contract (ruling 37, F-11)

- **Shared helper.** The decision now in napi `cuda_early_init.rs:56-100`
  moves into `cuda_driver_init.rs` as
  `run_module_load_early_init(embed_cuda_compiled: bool, rerank_cuda_compiled: bool)`.
  - Each caller passes its own `cfg!` flags, so feature unification inside
    `fathomdb-embedder` cannot change the meaning.
  - The helper keeps the opt-out (`FATHOMDB_CUDA_EARLY_INIT=off`) and the
    skip when every compiled component's policy is exactly `cpu`.
- **Napi** keeps `initialize_cuda_driver_at_registration` (:207) and calls
  the helper. Its existing tests pin that its behaviour is unchanged.
- **New state type.** A separate `ModuleLoadInit` records the load-time
  outcome: `Ran | OptedOut | SkippedCpuOnly | DriverAbsent | Failed(u32)`.
  `NotAtLoad` is what the gate sees when nothing was recorded.
  `CudaDriverInit` and `cu_result()` are unchanged.
- **Python.** `#[pymodule] _fathomdb` calls the helper first, under a
  `Once`, inside `catch_unwind`.
  - It is compiled only under `tegra-pool` + aarch64 Linux + CUDA, so the
    Tegra wheel is the only Python artifact that runs `cuInit` at import.
  - Import never fails because of it.
- **CLI.** `doctor cuda-allocator` calls the helper after its early returns
  (§ 2.6).
- **Rust users.** There is no load hook in Rust. `initialize_cuda_driver()`
  stays `#[doc(hidden)]`, and `fathomdb-sdk` does not re-export the
  helper. A Rust process therefore reports `cuinit_not_at_load` and takes
  the 0.8.27 path. `rust-sdk.md` says so. Whether a call counts as "at
  load" is the caller's contract and is not verified.

### 3.5 C7 instrumentation (ruling 36, F-8)

- **Recording the id.** The decision records the private context's
  primary-context id (`cuCtxGetId`; `sys/mod.rs:7878`) in
  `Decision::Private`.
- **Detection.** It runs only in a `Private` process, on a non-OOM CUDA
  error in `forward_error` or a private build:
  1. Call `cuDevicePrimaryCtxGetState`, which does not retain. If it is
     inactive, the context was lost.
  2. Only when it is active: retain, call `cuCtxGetId`, then release. A
     different id means it was lost.
  3. An error from either query, `CUDA_ERROR_CONTEXT_IS_DESTROYED` or
     `INVALID_CONTEXT`, also means lost.

  This never creates a primary context after a reset.
- **One snapshot per process.** `static SNAPSHOT: Once` writes one JSON
  line to stderr, prefixed `fathomdb-cuda-context-lost`, with:
  - the error fields;
  - the primary-context state (flags, active);
  - the CUDA libraries in `/proc/self/maps`;
  - the pool's reserved and used counters;
  - the live private contexts, `Arc::strong_count(&pool) - 1`.
- **Characterization.** A heavy-tier GPU test (`#[ignore]`, run in G10)
  spawns a child. The child builds an embedder, resets the primary context
  with the study's probe logic, then embeds and closes. The test asserts
  what results § 13.4 shows, which this slice keeps: the first embed after
  the reset returns `cuda_context_lost`, and `close()` then dies with
  SIGSEGV in `CudaSlice::drop`. 0.8.29's fix flips the second assertion.
- **Doctor.** `doctor cuda-allocator` reports `cuda_context_state` as
  `active` or `inactive`, with flags.

### 3.6 Feature wiring (SD-5, F-9)

- **Chain.** `tegra-pool = ["dep:cudarc", "cudarc/fathomdb-private-pool"]`
  in `fathomdb-embedder`. It is forwarded by `fathomdb-engine`,
  `fathomdb-napi`, `fathomdb-py`, `fathomdb-sdk`, `fathomdb` and
  `fathomdb-cli`.
- **Docs.** The feature is documented as supported only in repository and
  artifact builds, which carry the vendored cudarc.
- **Tegra Python wheel.** `cuda-artifact-contract.sh` gains
  `CUDA_PYTHON_FEATURES_TEGRA` (`pyo3/extension-module,embed-cuda,tegra-pool`),
  used by `build-python-cuda-tegra.sh:206`.
- **Contract guard.** A guard and its test refuse `tegra-pool` in any x86
  or CPU set.
- **Manual Tegra Node addon.** The recipe in
  `scripts/tests/test_tegra_node_early_cuinit.sh` and the Slice 117 note
  add `tegra-pool`.

## 4. Traces (plan § 5.2)

### 4.1 Configuration matrix

| Target | Features | Mode | Load init | Result |
| --- | --- | --- | --- | --- |
| aarch64 Linux Tegra 64 GB | `tegra-pool` + `embed-cuda` | `auto` | `Ran` | `Private`, 3 GiB, `private_pool` |
| same | same | `auto` | `OptedOut` / `NotAtLoad` | 0.8.27 path, `cuinit_opted_out` / `cuinit_not_at_load` |
| same | same | `off` / bogus | `Ran` | 0.8.27 path, `mode_off` / `invalid_setting` |
| same | `embed-cuda` only | any | any | 0.8.27 path, `not_built` |
| aarch64 Linux, not Tegra (GB10-class) | `tegra-pool` + `embed-cuda` | `auto` / `on` | `Ran` | `unmeasured_class` / sized by rule |
| aarch64 Linux + discrete GPU | `tegra-pool` + `embed-cuda` | `on` | `Ran` | 0.8.27 path, `discrete` |
| x86_64 Linux CUDA | `tegra-pool` + `embed-cuda` | any | — | 0.8.27 path, `cuda_allocator` `None` |
| macOS | `embed-metal` (+ `tegra-pool`) | any | — | unaffected, `None` |
| any | no CUDA feature | any | — | unaffected |

### 4.2 Feature unification and crates.io (F-9)

- No crate enables `tegra-pool` by default. `cargo test --workspace`
  therefore builds the feature-off code.
- The feature-on code runs in:
  - `cargo test -p fathomdb-embedder --features tegra-pool`, the pure part
    on every host;
  - the Tegra GPU tests.
- **The T8 matrix step:**
  - `cargo check` every crate with and without `tegra-pool`, per crate.
  - In a scratch copy of the workspace with `[patch.crates-io]` stripped,
    check that `fathomdb-embedder --features embed-cuda` compiles against
    registry cudarc.
  - In the same copy, check that `--features embed-cuda,tegra-pool` fails
    with the feature-marker message.

### 4.3 Interference with existing behaviour

| Existing behaviour | Interaction | Result |
| --- | --- | --- |
| 0.8.27 fallback (`core.rs:72-182`) | Bypassed only by `new_with_mem_pool`. | Unchanged for every other context. |
| Slice 110 early `cuInit` | Moved into the helper, with the flags passed in. | napi behaviour pinned by its existing tests. |
| Reranker device policy | Resolution unchanged; devices built through `new_cuda_device`. | One pool and decision for both models. |
| `cpu` / forced CUDA | No device means no decision; a forced probe goes through `new_cuda_device`. | Unchanged. |
| Close fix (`close_lock`) | Contexts are `Arc`; the decision holds one context and the pool for the process lifetime. | The engine's memory returns to the pool, and at threshold 0 to the driver, except the first context's footprint (G3 measures it). |
| Module-level singletons (ruling 32) | They hold pool memory until exit. | Documented. |
| `doctor gpu` v1 | Untouched. | — |
| `FATHOMDB_*` variables | New names only. | — |

### 4.4 Execution paths

| Path | Load | First GPU use | Teardown |
| --- | --- | --- | --- |
| Node | the hook (`cuda_early_init.rs:207`) runs the helper | `Engine.open` probe, `embedBatchCls` or `rerank()` | Contexts drop with engines; `DECISION` lives until exit; the driver reclaims |
| Python (Tegra wheel) | `#[pymodule]` runs the helper | the same, with the GIL released around native calls | statics are not dropped at finalisation |
| Rust | none | `Engine::open` / embedder | `cuinit_not_at_load` |
| CLI | `doctor cuda-allocator` runs the helper | probe | exit |

### 4.5 Shared state and races

| State | Primitive | Hazard and answer | Test |
| --- | --- | --- | --- |
| `DECISION` | `OnceLock` + `catch_unwind` | Concurrent first use: `get_or_init` runs one initialiser, and others block, then read the result. A panic becomes `Fallback`. | 8-thread test over an injected decider; panic test |
| Module-load record | `Mutex` (`cuda_driver_init.rs:119`) | Use before registration completes cannot happen: napi and py run the hook before any export is reachable. | Existing napi tests; new py test |
| Reranker singleton | `Mutex<Option<…>>`, held across the load | Concurrent `rerank()` calls during a load wait, then see the cached or retried result. No lock is taken inside the load. | Retry test with an injected loader |
| Engine embedder | `close_lock` (unchanged) | Close racing an embed: the existing fix holds. | `slice90_close_tests.rs` |
| Drop order | the `Arc` chain | Statics are never dropped, so the pool outlives every context. | Vendored drop-order test |
| `SNAPSHOT` | `Once` | Two concurrent detections write one snapshot, and both return typed errors. | 2-thread fault-seam test |
| Pool reservation | the driver | Concurrent exhaustion: each request gets its own typed error. | G4 QUAL |
| Fork (H-1) | — | § 2.5. | G2 QUAL characterization |

## 5. Design questions (resolved)

- **DQ-1.** The gate is integrated, compute capability 8.7 and at least
  48 GiB, with the two-source Tegra identity (§ 2.1). A unit test shows
  that a missing `/etc/nv_tegra_release` with a device-tree entry still
  passes on Orin.
- **DQ-2.** The ruling binds, so the hook ships only in `tegra-pool`
  builds (§ 3.4) and hazard H-1 is documented (§ 2.5). It goes to the
  owner as a HITL decision before S30-T5.
- **DQ-3.** SD-9 is accepted, with § 2.7's conditions.

## 6. ADR

`dev/adr/ADR-0.8.28-tegra-private-cuda-pool.md` records §§ 1–5 and amends
`ADR-0.6.0-embedder-protocol.md` (§ 2.7). It is index row 62.

## 7. Behaviour changes (changelog)

- **Default allocator.** In `tegra-pool` builds on the AGX Orin 64 GB, it
  is the private pool.
- **Python (Tegra wheel only).** It runs early `cuInit` at import; hazard
  H-1, **potentially breaking** for fork-based `multiprocessing`.
- **New error kinds** in every SDK: `cuda_pool_exhausted`,
  `cuda_context_lost` and `cuda_private_build_refused`.
- **Module-level `rerank()`.** Device-policy errors now raise their typed
  class instead of `WriteValidation`.
- **Reranker under `auto`.** Pool exhaustion and context loss now raise
  instead of becoming neutral scores.
- **Reranker load retries.** A load that fails with a transient CUDA error
  is retried, not memoized.
- **New settings:** `FATHOMDB_POOL_MODE`, `FATHOMDB_POOL_MAXSIZE` and
  `FATHOMDB_POOL_RELEASE_THRESHOLD`.
- **New report field** `cuda_allocator`, and the new verb `fathomdb doctor
  cuda-allocator`.
- **Version changes:** `fathomdb-embedder-api` 0.7.0
  (`EmbedderError` now `#[non_exhaustive]`); `CudaDeviceInfo` now
  `#[non_exhaustive]`; the Candle crates 0.10.3.
