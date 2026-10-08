---
title: ADR-0.8.28-tegra-private-cuda-pool
date: 2026-10-08
target_release: 0.8.28
desc: On the Jetson AGX Orin 64 GB, every FathomDB CUDA context allocates from one private, capped CUDA memory pool with release threshold 0, behind the non-default tegra-pool feature
blast_radius: vendored cudarc 0.19.7; Fathom Candle fork 0.10.3; fathomdb-embedder (policy module, device creation, report, early cuInit helper); fathomdb-embedder-api 0.7.0 (EmbedderError); fathomdb-engine, napi, py, sdk and cli errors; doctor cuda-allocator; Tegra release scripts; interface docs
status: accepted (owner rulings 25–38 and the 2026-10-08 HITL)
---

# ADR-0.8.28 — Tegra private CUDA memory pool

Accepted under the owner rulings 25–38 in
`dev/plans/0.8.28/prework/tegra-cuda-memory-pool-study.md` and the
2026-10-08 HITL (push the Candle branch; the Python hook in the Tegra wheel;
the SD-9 `fathomdb-embedder-api` break). It implements D28-08. The full
design, with its traces, is
`dev/plans/0.8.28/features/slice-30/design.md`; the plan is
`dev/plans/0.8.28/features/slice-30/plan.md`.

## Context

On a shared Jetson host, every allocation that fits must succeed and its
memory must be given back, without the synchronous allocator's 2–5×
latency. Allocation correctness and release rank above latency (ruling 25).
The 0.8.27 allocator decision (cudarc's default pool, or synchronous
allocation when that pool is unavailable) cannot cap or return what
FathomDB holds. The study measured a private pool with release threshold 0
as the arm that meets the need on the AGX Orin 64 GB.

## Decision

### Rule (design § 1)

1. **Who gets the pool.** A process built with `tegra-pool`, on aarch64
   Linux, on an integrated GPU of the measured class, whose module-load early
   `cuInit` ran, decides once at its first CUDA device build. It creates one
   private pool (release threshold 0), allocates and frees a 4-byte probe on
   it, and builds the first private context. The device's current pool is
   never read or changed.
2. **Never refuse at the decision (SD-3, ruling 37).** Any off gate, invalid
   setting, opted-out, failed or missing early `cuInit`, pool-creation,
   probe or first-build failure, or a panic, takes the 0.8.27 path with a
   recorded reason.
3. **Fail closed afterwards.** Once the decision is private, every later
   context for that ordinal is built on the pool, or the build returns a
   typed error. It never builds a context on another allocator, because a
   buffer must be freed by the API that allocated it. Another ordinal takes
   the 0.8.27 path.
4. **Without `tegra-pool`** the decision is 0.8.27's and no pool symbol is
   compiled. The typed errors, the `cuda_allocator` report (`not_built`) and
   `doctor cuda-allocator` ship in every build.

### Gates, sizing and settings (design §§ 2.1–2.3; ruling 31, SD-2, SD-7)

- Gates in order, the first off one giving the reason: mode (`mode_off`),
  setting (`invalid_setting`), early `cuInit` (`cuinit_*`), integrated
  (`discrete`), pool support (`no_pools`), size floor (`too_small`), Tegra
  identity (`not_tegra`) and measured class (`unmeasured_class`). `on` lifts
  only the last two; the 8 GB Orin stays off in every mode (SD-2).
- Tegra identity is `/etc/nv_tegra_release` or a device-tree `nvidia,tegra`
  entry. The measured class is compute capability 8.7 with at least 48 GiB
  (DQ-1): only the AGX Orin 64 GB.
- Size is clamp(total / 20, 2 GiB, 3 GiB): 3 GiB on the AGX Orin 64 GB.
- Settings are environment variables in FathomDB, never in cudarc:
  `FATHOMDB_POOL_MODE` (`auto`/`on`/`off`), `FATHOMDB_POOL_MAXSIZE` (bytes,
  `<n>G` or `<n>M`, 32 MiB to device total) and
  `FATHOMDB_POOL_RELEASE_THRESHOLD` (`0`/`max`; default `0`, ruling 30). They
  are read once, inside the decision. A malformed value turns the pool off
  with `invalid_setting`; it never aborts.

### Errors (design § 2.4; ruling 33)

Three kinds, typed on every path in every SDK (engine embed, batch embed,
search rerank, open, module-level `embed_batch_cls` / `embedBatchCls` and
`rerank()`), CLI exit 70:

- `cuda_pool_exhausted` (`ordinal`, `max_size_bytes`, `message`): the next
  request runs normally;
- `cuda_context_lost` (`recorded_context_id`, `current_context_id`,
  `driver_error`, `operation`);
- `cuda_private_build_refused` (`ordinal`, `message`).

The reranker under `auto` raises pool exhaustion and context loss instead
of producing neutral scores. The reranker singleton no longer memoizes a
load failure of these kinds. `rerank_passages` returns
`RerankPassagesError` instead of `String`, and module-level `rerank()`
raises device-policy errors as their typed class.

### Report (design § 2.6; SD-8)

`CudaDeviceInfo` gains `cuda_allocator: Option<CudaAllocatorReport>` (path,
reason, pool size, release threshold, module-load `cuInit` outcome):
`not_built` on aarch64 Linux CUDA builds without `tegra-pool`, the built
context's real allocator with it, `None` on every other target. The new
sibling verb `fathomdb doctor cuda-allocator`
(`fathomdb.doctor.cuda-allocator.v1`) reports the decision this process
makes; `doctor gpu` v1 is unchanged.

### Components (design § 3)

1. **cudarc primitive, upstream-first (ruling 38).** The vendored cudarc
   0.19.7 gains `CudaMemPool` (create, attribute, trim, raw, destroy on
   drop), `AllocMode { Default, Synchronous, Private }`,
   `CudaContext::new_with_mem_pool`, `alloc_mode()` and `mem_pool()`.
   Private contexts allocate with `cuMemAllocFromPoolAsync` and bypass the
   process-wide fallback decision, so every other context on the device,
   including a co-resident cudarc user's, keeps the 0.8.27 decision. A
   zero-byte request goes to the driver. The no-op feature
   `fathomdb-private-pool` marks the vendored crate. `FATHOMDB-PATCH.md`
   gives each item an upstream status and removal path, and
   `scripts/pinned-override-rot.json` lists exactly that set.
2. **Candle (SD-1).** `CudaDevice::from_context` and
   `Device::new_cuda_from_context` are on the Fathom Candle fork; core, nn
   and transformers move to 0.10.3 together (candle-kernels stays 0.10.2).
3. **Policy module.** `fathomdb-embedder/src/cuda_pool_policy.rs`: a pure
   part on every host (parsers, gates, sizing, reasons, report types) and a
   driver part under `tegra-pool` + aarch64 Linux + a CUDA feature. One
   `OnceLock` decision under `catch_unwind`; every Candle CUDA device
   FathomDB builds goes through `new_cuda_device`, and every forward error
   through `forward_error`.
4. **Early `cuInit` (rulings 35, 37).** The decision moves from the Node
   addon into the shared helper `run_module_load_early_init`, which keeps the
   `FATHOMDB_CUDA_EARLY_INIT=off` opt-out and the all-`cpu` skip and records
   a `ModuleLoadInit`. Node calls it at registration; the Tegra Python wheel
   calls it at import (only `tegra-pool` builds); `doctor cuda-allocator`
   calls it after its early returns. Rust has no load hook: a Rust process
   reports `cuinit_not_at_load` and takes the 0.8.27 path. This also settles
   Slice 117's import-time `cuInit` item: the Tegra Node addon gets the same
   check-and-fall-back contract.
5. **Context loss, C7 (rulings 27, 36).** The decision records the private
   context's id. On a non-OOM CUDA error in a private process, an inactive
   primary context, a changed id, or a destroyed or invalid context means
   lost: the operation returns `cuda_context_lost` and one
   `fathomdb-cuda-context-lost` JSON line goes to stderr per process. A
   co-resident reset of the primary context stays **unsupported** in
   0.8.28: teardown afterwards still crashes (on the private-pool path in
   `cublasDestroy_v2`, from the Candle device's cuBLAS handle drop), and a
   heavy-tier characterization test pins that until the 0.8.29 fix.
6. **Feature wiring (SD-5).** `tegra-pool` is non-default in
   `fathomdb-embedder` and forwarded by the engine, napi, py, sdk, facade and
   CLI crates. It is supported only in repository and artifact builds, which
   carry the vendored cudarc; against registry cudarc it fails at
   resolution with the marker message
   (`scripts/check-tegra-pool-registry-build.sh`). Only the Tegra Python
   wheel's feature set (`CUDA_PYTHON_FEATURES_TEGRA`) and the manual Tegra
   Node addon recipe carry it, and `scripts/check-cuda-release-contract.py`
   refuses it in every x86_64 and CPU set.

### Amendment of ADR-0.6.0-embedder-protocol (SD-9, DQ-3)

`fathomdb-embedder-api::EmbedderError` gains `CudaPoolExhausted`,
`CudaContextLost` and `CudaPrivateBuildRefused` unconditionally (a
feature-gated variant would change a public enum through feature
unification) and becomes `#[non_exhaustive]`; axis E moves to 0.7.0. A
caller-supplied embedder may return the new variants. The rest of the
embedder protocol is unchanged.

## Consequences

- **Public breaks.** `EmbedderError` (new variants, `#[non_exhaustive]`);
  `CudaDeviceInfo` (new field, `#[non_exhaustive]`);
  `RerankerDevicePolicyError`, `EmbedderLoadError` and `RerankerLoadError`
  (new variants); `fathomdb_engine::rerank_passages` (error type).
- **Module-level models hold pool memory until exit (ruling 32).** The CLS
  embedder and reranker singletons are never released; engine-owned models
  return their memory when the engine closes.
- **Fork hazard H-1.** With the Tegra wheel's import hook, a `fork`-based
  `multiprocessing` child created after `import fathomdb` cannot use CUDA. Set
  `FATHOMDB_CUDA_EARLY_INIT=off`, or use the `spawn` or `forkserver` start
  methods; never fork while another thread is inside FathomDB's CUDA
  initialisation. This is potentially breaking for 0.8.27 users of that
  pattern.
- **Design questions.** DQ-1 (gate) as above; DQ-2 (hook only in
  `tegra-pool` builds) ruled by the owner before S30-T5; DQ-3 (SD-9)
  accepted.
- The 0.8.27 path is unchanged for every build without `tegra-pool` and for
  every target other than aarch64 Linux.
