---
title: FathomDB 0.8.28 Slice 30 — Tegra private CUDA memory pool, design
status: PROPOSED (revision 1, 2026-10-08; for design review)
target_release: 0.8.28
observed_on: 2026-10-08
---

# Slice 30 design: Tegra private CUDA memory pool

This design implements D28-08 under the [Slice 30 plan](plan.md). It is
written from the code at `cb9594f44`. Citations are `path:line` on that
commit. Owner rulings are cited by number from
`dev/plans/0.8.28/prework/tegra-cuda-memory-pool-study.md`, and plan
decisions as SD-n.

## 1. Rule

1. **Who gets the pool.** A process with these properties creates one
   private CUDA memory pool at its first GPU use, and builds every FathomDB
   CUDA context on it (P-first-use; rulings 7, 12, 38):
   - it was built with the `tegra-pool` feature;
   - it runs on aarch64 Linux;
   - it has an integrated GPU of a measured class (§ 2.1);
   - its module-load early `cuInit` ran.

   The pool's release threshold is 0. The device's current pool is never
   read or changed.
2. **Never refuse at the first decision (SD-3).** In every other case the
   process takes the 0.8.27 path: cudarc's own default-pool-or-synchronous
   decision. That covers:
   - an early `cuInit` that was opted out, failed, or did not run at load;
   - a gate that is off;
   - an invalid setting;
   - pool creation that fails.

   The reason is recorded (§ 2.5).
3. **Fail closed afterwards.** Once a private-pool context exists, the
   process never builds a non-private context. A later private build failure
   is an error, not a fallback, because a buffer must never reach a free API
   other than the one that allocated it.
4. **Without `tegra-pool`, nothing changes (SD-5).** The device-creation
   function is `Device::new_cuda`, as in 0.8.27, and no pool symbol is
   compiled. The allocator report is `None`.

## 2. Parameters

### 2.1 Gates and sizing (ruling 31, SD-2)

The device facts are read once, with the device's primary context retained:

- `CU_DEVICE_ATTRIBUTE_INTEGRATED`;
- `CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED`;
- `cuDeviceTotalMem`;
- the Tegra identity: `/etc/nv_tegra_release` exists, the same check
  `tegra_fragmented_va_cuda.rs` uses.

The gates are evaluated in this order. The first that is off gives the
reason.

| Gate | Off when | Reason | Lifted by `on` |
| --- | --- | --- | --- |
| Mode | `FATHOMDB_POOL_MODE=off` | `mode_off` | — |
| Setting | Any pool setting malformed | `invalid_setting` | no |
| Early `cuInit` | `ModuleLoad` record is not `Initialized` | `cuinit_opted_out`, `cuinit_failed`, `cuinit_not_at_load` | no |
| Integrated | attribute ≠ 1 | `discrete` | no |
| Pool support | attribute ≠ 1 | `no_pools` | no |
| Size floor | 2 GiB > total / 4 | `too_small` | no (SD-2) |
| Tegra identity | no `/etc/nv_tegra_release` | `not_tegra` | yes |
| Measured class | not (total ≥ 48 GiB and compute capability 8.7): only the AGX Orin 64 GB class is measured | `unmeasured_class` | yes |

Size = clamp(total / 20, 2 GiB, 3 GiB), unless `FATHOMDB_POOL_MAXSIZE` is
set. The override applies only after every gate passes. On the AGX Orin
64 GB (61.36 GiB reported, sm_87) the size is 3 GiB. Thor (about 122 GiB,
sm_110) and the 32 and 16 GB Orin fail the measured-class gate, so they
need `on` (DQ-1).

### 2.2 Settings (SD-7)

| Variable | Default | Values |
| --- | --- | --- |
| `FATHOMDB_POOL_MODE` | `auto` | `auto`, `on`, `off` |
| `FATHOMDB_POOL_MAXSIZE` | derived (§ 2.1) | bytes, `<n>G` or `<n>M`; > 0 |
| `FATHOMDB_POOL_RELEASE_THRESHOLD` | `0` (ruling 30) | `0`, `max` |

- They are read once, at the first device decision. A later change has no
  effect, and the interface docs say so.
- A malformed value turns the pool off with `invalid_setting` and the
  offending variable's name. It never aborts.
- The settings live in FathomDB, never in cudarc.

### 2.3 Named constants (each has a pure test)

`POOL_SIZE_DEVICE_DIVISOR` = 20 · `POOL_SIZE_FLOOR` = 2 GiB ·
`POOL_SIZE_CEILING` = 3 GiB · `POOL_FLOOR_MAX_SHARE_DIVISOR` = 4 ·
`MEASURED_CLASS_MIN_TOTAL` = 48 GiB · `MEASURED_CLASS_COMPUTE_CAPABILITY` = 8.7 · `PROBE_BYTES` = 4 ·
`POOL_EXHAUSTED_KIND` = `"cuda_pool_exhausted"` ·
`CONTEXT_LOST_KIND` = `"cuda_context_lost"`.

The floor and ceiling rest on two measurements from one AGX Orin: a pool
holds ceil32(`maxSize`/3), and it grows in 32 MiB chunks. They are cited as
evidence in the docs, not used as logic.

### 2.4 Errors

**`fathomdb-embedder-api::EmbedderError`** gains two variants and becomes
`#[non_exhaustive]` (SD-9 below):

```rust
CudaPoolExhausted { ordinal: usize, max_size_bytes: u64, message: String },
CudaContextLost { recorded_context_id: u64, current_context_id: Option<u64>,
                  driver_error: String, operation: String },
```

`fathomdb-embedder::RerankerDevicePolicyError` gains the same two shapes as
variants, for the reranker. Mapping by surface:

| Surface | `cuda_pool_exhausted` | `cuda_context_lost` |
| --- | --- | --- |
| Engine | `EngineError::CudaPoolExhausted {..}`, stable code `cuda_pool_exhausted` | `EngineError::CudaContextLost {..}`, stable code `cuda_context_lost` |
| napi / TS | code `CUDA_POOL_EXHAUSTED`, class `CudaPoolExhaustedError extends EmbedderError` | `CUDA_CONTEXT_LOST`, `CudaContextLostError extends EmbedderError` |
| py / Python | `CudaPoolExhaustedError(EmbedderError)` | `CudaContextLostError(EmbedderError)` |
| `fathomdb-sdk` | `ErrorKind::CudaPoolExhausted`, parent `Embedder` | `ErrorKind::CudaContextLost`, parent `Embedder` |
| CLI | its own exit code in `engine_error_code`, beside `Embedder` | same |

**Payloads.**

- Exhausted: `ordinal`, `max_size_bytes`, `message`.
- Context lost: `recorded_context_id`, `current_context_id` (absent if the
  query itself failed), `driver_error`, `operation`.
- They appear as typed attributes in Python and TS and as fields in Rust.

**Paths.** Every path raises the typed kind, with no `WriteValidation` or
Debug string:

- engine `embed`, batch embed and search rerank;
- module-level `embed_batch_cls` / `embedBatchCls`;
- module-level `rerank()`.

For `rerank()`, `rerank_passages` (`fathomdb-engine/src/rerank.rs:98`)
returns a typed error enum in place of `String`. Non-finite scores stay
`WriteValidation`. Device-policy errors keep their existing typed classes
(`RerankerDevicePolicyError` in TS and Python), which today are flattened
to strings (plan C-6).

**Recovery.**

- After `cuda_pool_exhausted` the device stays CUDA and the next request
  runs normally. Nothing moves to the CPU (CB4).
- A load-time pool exhaustion is not memoized by the reranker singleton
  (`rerank.rs:279-298` memoizes only `DevicePolicy` errors today). The
  next call retries.

### 2.5 Report (SD-8)

`CudaDeviceInfo` (`fathomdb-embedder/src/device_policy.rs:95`) gains
`cuda_allocator: Option<CudaAllocatorReport>`. `fathomdb-sdk` re-exports
the type (`fathomdb-sdk/src/lib.rs:60`); napi, py, TS and Python map it.

```rust
pub struct CudaAllocatorReport {
    pub path: CudaAllocatorPath,        // Private | DefaultPool | Synchronous
    pub reason: CudaAllocatorReason,    // § 2.1 reasons, plus `private_pool`, `pool_create_failed`
    pub pool_max_size_bytes: Option<u64>,
    pub release_threshold: Option<u64>, // Some(0) or Some(u64::MAX) on Private
    pub early_cuinit: EarlyCuInit,      // Ran | OptedOut | Failed(u32) | NotAtLoad
}
```

- It is `None` when the build lacks `tegra-pool`, or the target is not
  aarch64 Linux with `embed-cuda`/`rerank-cuda`.
- `path` comes from the context that was actually built: cudarc's
  `alloc_mode()`, added with the primitive in § 3.1.
- In a fallback process, `path` is `DefaultPool` or `Synchronous`.

`fathomdb doctor gpu` adds these JSON keys, in this order after the existing
ones, and the same lines in text: `allocator_path`, `allocator_reason`,
`pool_max_size_bytes`, `pool_release_threshold`, `early_cuinit`,
`cuda_context_state`. The schema version moves to
`fathomdb.doctor.gpu.v2`.

### 2.6 SD-9 (new, from the code): `fathomdb-embedder-api` takes one breaking change

`EmbedderError` (`fathomdb-embedder-api/src/lib.rs:42`) is a public enum
without `#[non_exhaustive]`, in the separately versioned axis-E crate
(0.6.1). Adding a variant breaks exhaustive matches downstream. The
study's note says adoption "is a breaking change to this crate and needs an
ADR".

**Decision:**

- Add both variants unconditionally. A feature-gated variant would change a
  public enum through Cargo feature unification.
- Mark the enum `#[non_exhaustive]`, so later kinds do not break again.
- Bump axis-E to 0.7.0 in this slice.
- The ADR amends `ADR-0.6.0-embedder-protocol.md` (decision index row 7)
  for the error-set change only.
- `tests/eu5b_lockflip.rs`, which matches exhaustively, gains the new arms.

## 3. Components

### 3.1 cudarc primitive (vendored, upstream-first; ruling 38)

`third_party/cudarc-0.19.7/src/driver/safe/mem_pool.rs` (new) adds
`CudaMemPool`:

- `new(ordinal, MemPoolProps { max_size, release_threshold })`;
- `attribute`;
- `trim_to`;
- `raw`;
- `Drop`, which calls `cuMemPoolDestroy`.

The constructor uses the result-layer calls upstream merged in #544.

`core.rs` adds three things:

- `CudaContext::new_with_mem_pool(ordinal, Arc<CudaMemPool>)`. It retains
  the primary context, as `new` does, and stores
  `alloc_mode = AllocMode::Private`.
- `alloc_mode()` and `mem_pool()` accessors.
- Allocation and free branches for `Private`: `cuMemAllocFromPoolAsync`,
  freed with the ordinary `cuMemFreeAsync`.

The private constructor does **not** call `select_async_alloc`
(`core.rs:155`). It never touches `PROCESS_ALLOC_MODE` (:141) or the
default pool, so a co-resident cudarc user's decision is unaffected (C9).

**Lifetime.** The pool outlives everything allocated from it, through this
chain: slice → stream → context → `Arc<CudaMemPool>`. No product path
destroys a pool that a live context references.

**Zero-length allocations.** A private context returns a null pointer for a
zero-byte request without a driver call, matching patch item 3. No null
pointer reaches a free.

The study's install path is not ported: `CudaMemPool::install`,
`AllocMode::Explicit`, and the A-first-use / B arms.

`FATHOMDB-PATCH.md` lists every item with an upstream status and a removal
path:

| Item | Upstream status |
| --- | --- |
| 1. Allocator fallback | local-only |
| 2. Per-device decision | local-only |
| 3. Zero-length | local-only |
| 4. Tests | local-only |
| 5. Private pool primitive | proposed (not posted) |

`scripts/pinned-override-rot.json` lists exactly this set, and the vendored
tests run through `scripts/tests/test_vendored_cudarc.sh`.

### 3.2 Candle (SD-1)

`CudaDevice::from_context(Arc<CudaContext>)` and
`Device::new_cuda_from_context` come from local commit `bd68a7ca`, rebased
onto the fork's `1aefdd008`. The device uses the context's default stream,
and every Candle allocation goes through the context's allocator. The three
crates move to 0.10.3.

### 3.3 Policy module, `fathomdb-embedder/src/cuda_pool_policy.rs`

The study's module, cut down (plan § 5), in two parts.

**The pure part** compiles on every host:

- mode and setting parsers;
- `DeviceFacts`;
- `gate_and_size(mode, facts, early, settings) -> Decision`;
- the process-mode functions: `next_build`, `on_private_failure`,
  `mode_after`;
- `is_pool_exhaustion`;
- the report types.

**The driver part** compiles only with
`cfg(all(feature = "tegra-pool", target_os = "linux", target_arch = "aarch64", any(feature = "embed-cuda", feature = "rerank-cuda")))`.
Its process state and functions:

```text
static SETTINGS:   OnceLock<Result<Settings, InvalidSetting>>
static FIRST_USE:  Once                       // runs the decision once
static PRIVATE:    OnceLock<(usize, Arc<CudaMemPool>)>
static MODE:       Mutex<ProcessMode>         // Undecided | Production | Private
static REPORT:     OnceLock<CudaAllocatorReport>
static CTX_ID:     OnceLock<u64>              // § 3.5
pub(crate) fn new_cuda_device(ordinal) -> candle_core::Result<Device>
pub(crate) fn forward_error(candle_core::Error, what) -> EmbedderError
pub(crate) fn allocator_report() -> Option<CudaAllocatorReport>
```

Without the driver part, `new_cuda_device` is `Device::new_cuda`,
`forward_error` maps to `Failed` as today, and `allocator_report` returns
`None`.

**Call sites.** Every Candle CUDA device FathomDB creates goes through
`new_cuda_device`:

- `candle_bge.rs:260` (probe), `:382` (witness) and `:432` (load);
- `candle_reranker.rs:121` (probe) and `:166` (load).

Every forward error goes through `forward_error`. The reranker maps it into
`RerankerDevicePolicyError`.

### 3.4 Early-`cuInit` contract (ruling 37)

- **Shared helper.** The decision now in napi `cuda_early_init.rs:56-100`
  (opt-out `FATHOMDB_CUDA_EARLY_INIT=off`; skipped when every compiled
  component's policy is exactly `cpu`) moves into `cuda_driver_init.rs` as
  `run_module_load_early_init()`. Napi keeps its `module_exports` hook
  (:207) and calls the helper. A test pins that napi's behaviour does not
  change.
- **New states.** `CudaDriverInit` gains `OptedOut` and `SkippedCpuOnly`.
  They are recorded in the `ModuleLoad` slot, so the policy can tell
  "opted out" from "never ran at load".
- **Python.** `#[pymodule] _fathomdb` (`fathomdb-py/src/lib.rs`) calls the
  helper first, under a `Once`, inside `catch_unwind`. Import never fails
  because of it.
- **CLI.** `run_doctor_gpu` (`fathomdb-cli/src/lib.rs:738`) calls the
  helper before `product_gpu_diagnostic`.
- **Gate.** The pool gate reads `cuda_driver_init_seen_by(ModuleLoad)`
  (`cuda_driver_init.rs:205`). Only `Initialized` passes.

### 3.5 C7 instrumentation (ruling 36)

- **Recording the id.** When the private context is created, its
  primary-context id (`cuCtxGetId`, available in the vendored sys layer,
  `sys/mod.rs:7878`) is stored in `CTX_ID`.
- **Detection.** `forward_error`, and the device-build path, handle every
  non-OOM CUDA driver error the same way:
  1. Retain the primary context and query its id.
  2. If the id differs from `CTX_ID`, or the query fails with
     `CUDA_ERROR_CONTEXT_IS_DESTROYED` / `INVALID_CONTEXT`, map the error to
     `CudaContextLost`.
  3. Otherwise keep the existing mapping.
- **Scope.** Detection runs only in a private-pool process. The 0.8.27 path
  is unchanged.
- **One snapshot per process.** A `static SNAPSHOT: Once` writes one JSON
  line to stderr, prefixed `fathomdb-cuda-context-lost`, containing:
  - the error fields;
  - `cuDevicePrimaryCtxGetState` (flags, active);
  - the CUDA libraries in `/proc/self/maps`;
  - the pool's reserved and used counters;
  - whether the module-level embedder and reranker are loaded;
  - the count of live engines.

  The study's `fdb-pool-exp` lines are not kept. This line exists only on
  the loss path.
- **Characterization.** A heavy-tier GPU test (`#[ignore]`, run in G10)
  spawns a child that resets the primary context through the study's probe
  logic and then embeds. It records whether the child crashes or returns
  `cuda_context_lost`. The test asserts that it does one of the two, and
  documents which. 0.8.29's fix flips it.
- **`doctor gpu`.** `cuda_context_state` reports `active` or `inactive`,
  with flags, from `cuDevicePrimaryCtxGetState`.

### 3.6 Feature wiring (SD-5)

- **Chain.** `tegra-pool = []` in `fathomdb-embedder`, forwarded by
  `fathomdb-engine`, `fathomdb-napi`, `fathomdb-py`, `fathomdb-sdk`,
  `fathomdb` and `fathomdb-cli`. It does not imply `embed-cuda`. Without a
  CUDA feature it compiles to nothing.
- **Tegra Python wheel.** `scripts/release/cuda-artifact-contract.sh` gains
  `CUDA_PYTHON_FEATURES_TEGRA` (`…,embed-cuda,tegra-pool`), which
  `build-python-cuda-tegra.sh:205` uses. The x86 sets are unchanged.
- **Contract guard.** A check, with its test, refuses `tegra-pool` in any
  x86 or CPU feature set.
- **Manual Tegra addon.** The recipe in
  `scripts/tests/test_tegra_node_early_cuinit.sh`'s header, and Slice 117's
  plan note, add `tegra-pool`.

## 4. Traces (plan § 5.2)

### 4.1 Configuration matrix

| Target | Features | Mode | `cuInit` at load | Result | Code |
| --- | --- | --- | --- | --- | --- |
| aarch64 Linux Tegra 64 GB | `tegra-pool` + `embed-cuda` | `auto` | ran | `Private`, 3 GiB, reason `private_pool` | § 3.3 |
| same | same | `auto` | opted out | 0.8.27 path, `cuinit_opted_out` | § 3.4 |
| same | same | `auto` | never ran (Rust direct use) | 0.8.27 path, `cuinit_not_at_load` | § 3.4 |
| same | same | `off` | ran | 0.8.27 path, `mode_off` | § 2.1 |
| same | same | `bogus` | ran | 0.8.27 path, `invalid_setting` | § 2.2 |
| same | `embed-cuda` only | any | any | 0.8.27 path, report `None` | § 1.4 |
| aarch64 Linux, not Tegra (GB10-class) | `tegra-pool` + `embed-cuda` | `auto` / `on` | ran | `not_tegra` / sized by rule | § 2.1 |
| aarch64 Linux + discrete GPU | `tegra-pool` + `embed-cuda` | `on` | ran | 0.8.27 path, `discrete` | § 2.1 |
| x86_64 Linux CUDA | `tegra-pool` + `embed-cuda` | any | — | compiles to the 0.8.27 path, report `None` | § 3.3 cfg |
| macOS | `embed-metal` (+ `tegra-pool`) | any | — | unaffected, report `None` | § 3.3 cfg |
| any | no CUDA feature | any | — | unaffected | § 3.6 |

### 4.2 Feature unification

- `cargo test --workspace` unifies `tegra-pool` only if a workspace crate
  enables it by default. None does, so workspace runs build the
  feature-off code.
- The feature-on code runs in:
  - `cargo test -p fathomdb-embedder --features tegra-pool`, the pure tests
    on every host;
  - the Tegra GPU tests.
- A `cargo check` matrix step covers both states: every crate per crate,
  with and without `tegra-pool`, on the host target.
- Without the feature, `fathomdb-embedder` uses no vendored-only cudarc
  symbol, so the crates.io build compiles against stock cudarc. This is
  asserted by building `fathomdb-embedder` with the `[patch]` table
  disabled (`--config` override) in the T8 check.

### 4.3 Interference with existing behaviour

| Existing behaviour | Interaction | Result |
| --- | --- | --- |
| 0.8.27 fallback (patch items 1–3, `core.rs:72-182`) | The private constructor bypasses it; non-private constructors use it unchanged. | No change for non-pool processes. |
| Slice 110 early `cuInit` and opt-out (`cuda_early_init.rs`) | Logic moves to a shared helper; napi's behaviour is pinned by its existing tests. | Unchanged. |
| Reranker device policy (`reranker_device_policy.rs`) | Policy resolution is unchanged; a CUDA reranker device is built through `new_cuda_device`. | Same private pool and context as the embedder. |
| `FATHOMDB_EMBED_DEVICE=cpu` / forced CUDA | No CUDA device, so no decision; forced CUDA probes through `new_cuda_device`. | Unchanged semantics. |
| Embedder-close fix (`close_lock`, drop order) | Contexts are `Arc`; the pool is process-lifetime in `PRIVATE`; closing an engine drops its contexts but not the pool. | Reserved memory returns to the pool and, at threshold 0, to the driver (G3). |
| Module-level singletons (ruling 32) | They hold pool memory until exit. | Documented. |
| `doctor gpu` | Runs early `cuInit`, then its probe makes the real decision. | It reports what an SDK process would get. |
| Existing `FATHOMDB_*` variables | New names only; no collision. | — |

### 4.4 Execution paths

| Path | Load | First GPU use | Decision | Teardown |
| --- | --- | --- | --- | --- |
| Node | `module_exports` hook runs the helper (`cuda_early_init.rs:207`) | `Engine.open` probe, `embedBatchCls` (`embedding.rs:122`), or `rerank()` | `FIRST_USE` in `new_cuda_device` | Contexts drop with engines and singletons; `PRIVATE` lives until exit. No exit hook. |
| Python | `#[pymodule]` init runs the helper | the same three entries, with the GIL released around the native call | same | Interpreter finalisation: static `OnceLock`s are not dropped, so the driver reclaims at exit. |
| Rust | none (no load hook) | `Engine::open` / embedder | same; `cuinit_not_at_load` unless the application calls the helper | drop |
| CLI | `run_doctor_gpu` calls the helper | probe | same | exit |

### 4.5 Shared state and races

| State | Writers | Readers | Primitive | Hazard and answer | Test |
| --- | --- | --- | --- | --- | --- |
| Decision (`FIRST_USE`, `REPORT`) | first `new_cuda_device` | all later calls | `Once` + `OnceLock` | Concurrent first use: `Once` blocks the others until the decision is stored; all see one result. | Pure test with 8 threads over an injected driver |
| `MODE` | every device build | every device build | `Mutex`, held across the build | A build racing a failure: the mode is read and written under one lock. | Pure process-mode tests |
| `SETTINGS` | the first decision | the decision, the report | `OnceLock` | An environment change afterwards is ignored and documented. | Unit test |
| `ModuleLoad` record | the load hook | the gate, `doctor` | `Mutex` (`cuda_driver_init.rs:119`) | First use on another thread before registration finishes: Node and Python run the hook before any export is callable. | Existing napi tests and the new py test |
| `PRIVATE` pool | the decision | contexts | `OnceLock` + `Arc` | Pool creation fails while others wait: they see `MODE = Production` and use the 0.8.27 path. | Fault-seam test |
| Embedder in an engine | open, close | embed, rerank | `close_lock` (unchanged) | Close racing an in-flight embed: the existing fix holds. | Existing `slice90_close_tests.rs` |
| Drop order | Candle, cudarc | — | `Arc` chain (§ 3.1) | Exit with live singletons: the statics are never dropped, the pool is never destroyed before its contexts, and the driver reclaims at exit. | Vendored drop-order test |
| `CTX_ID`, `SNAPSHOT` | the private build; the first loss | every error mapping | `OnceLock` + `Once` | Two threads detect a loss together: one snapshot, both get typed errors. | Fault-seam test with 2 threads |
| Pool reservation | allocations | the exhaustion path | the driver | Concurrent exhaustion: each request gets its own typed error; nothing is shared. | G4 QUAL |
| CUDA across `fork()` | — | the child | — | **Hazard H-1:** Python now runs `cuInit` at import. A `multiprocessing` child forked after import cannot use CUDA (the driver is not fork-safe). In 0.8.27 that worked if the parent had not used the GPU. Answer: the docs name the opt-out and the `spawn`/`forkserver` start methods; G2 records the child's outcome on device. | G2 QUAL (characterization) |

## 5. Design questions for review

- **DQ-1. Measured-class gate.** A memory threshold alone (≥ 48 GiB)
  would also admit a 128 GB Thor in `auto`, which ruling 31 says must be
  opt-in. This design therefore adds compute capability 8.7 (Orin) to the
  gate, as § 2.1 shows. Reviewers: confirm this is the narrowest gate that
  admits only the measured class.
- **DQ-2. H-1 and Python early `cuInit`.** Ruling 37 requires it. The fork
  hazard is new information. Is documentation plus the opt-out enough for
  0.8.28, or should the Python hook default off until measured? This design
  follows the ruling and surfaces it to the owner.
- **DQ-3. SD-9.** The axis-E breaking bump (0.6.1 → 0.7.0) is a consequence
  of rulings 33 and 36. It is listed for the owner.

## 6. ADR

`dev/adr/ADR-0.8.28-tegra-private-cuda-pool.md` records §§ 1–5 and amends
`ADR-0.6.0-embedder-protocol.md` (SD-9). It is indexed as row 62.

## 7. Behaviour changes (changelog)

- **Default allocator:** in `tegra-pool` builds on the AGX Orin 64 GB, it is
  the private pool.
- **Python:** runs early `cuInit` at import (opt-out
  `FATHOMDB_CUDA_EARLY_INIT=off`); hazard H-1.
- **New error kinds:** `cuda_pool_exhausted` and `cuda_context_lost`, in
  every SDK.
- **`rerank()` typing:** device-policy errors from module-level `rerank()`
  now raise their typed class instead of `WriteValidation`.
- **New settings:** `FATHOMDB_POOL_MODE`, `FATHOMDB_POOL_MAXSIZE` and
  `FATHOMDB_POOL_RELEASE_THRESHOLD`.
- **New report field:** `cuda_allocator`.
- **`doctor gpu`:** schema `v2`, with new fields.
- **Dependency versions:** `fathomdb-embedder-api` 0.7.0; the Candle crates
  0.10.3.
