---
title: ADR-0.8.28-tegra-private-cuda-pool
date: 2026-10-08
target_release: 0.8.28
desc: On the Jetson AGX Orin 64 GB, every FathomDB CUDA context allocates from one private, capped CUDA memory pool with release threshold 0 by default, behind the tegra-pool feature (implied by embed-cuda and rerank-cuda since Slice 31)
blast_radius: cudarc-fathomdb 0.19.10 (crates.io, from coreyt/cudarc); the Candle fork crates 0.11.0 and candle-kernels by git patch at 182ceab0; fathomdb-embedder (policy module, device creation, report, early cuInit helper); fathomdb-embedder-api 0.7.0 (EmbedderError); fathomdb-engine, napi, py, sdk and cli errors; doctor cuda-allocator; Tegra release scripts; interface docs
status: accepted (owner rulings 25–38 and the 2026-10-08 HITL); amended 2026-10-13 (Slice 31)
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
   (`scripts/check-tegra-pool-registry-build.sh`). Both the Tegra Python
   wheel's feature set (`CUDA_PYTHON_FEATURES_TEGRA`) and the governed Tegra
   Node addon feature set (`CUDA_TEGRA_NAPI_FEATURES`) carry it. The 0.8.27
   Tegra Node artifact did not include this 0.8.28 feature; the 0.8.28
   artifact build does. `scripts/check-cuda-release-contract.py` refuses it
   in every x86_64, generic arm64 and CPU set.

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

## Amendment 2026-10-13 — consumption through the published fork crates (0.8.28 Slice 31)

**Context.** The decision above stands except where decisions 6 and 9-11
below say otherwise: who gets the pool, the gates, sizing, errors and the
Node and Python early-`cuInit` contract are unchanged. Decisions 1-5 change
where the cudarc primitive and its evidence live; decision 6 adds the close
release; decisions 9-11 move the settings onto every SDK's open, give plain
Rust processes the pool, and make the CUDA features imply `tegra-pool`
(owner rulings Q29, Q30, Q33-Q37). Slice 31
moved the vendored cudarc delta to FathomDB's cudarc fork
(`coreyt/cudarc`, branch `fathomdb/0.19.10`, rebased on upstream
v0.19.10) and published it as `cudarc-fathomdb` 0.19.10 from commit
`b777f8d9`; the Candle fork depends on it and is published as
`candle-core-fathomdb`, `candle-nn-fathomdb` and
`candle-transformers-fathomdb` 0.11.0 from `182ceab0` (owner rulings
Q9, Q17, Q18, Q21).

**Decision.**

1. **Consumption.** `fathomdb-embedder` depends on the Candle fork crates
   `=0.11.0` and reaches `cudarc-fathomdb` 0.19.10 only through them
   (`candle_core::cuda::cudarc`); it has no direct `cudarc` dependency, and
   `tegra-pool` adds none. `Cargo.lock` holds exactly `cudarc-fathomdb`
   0.19.10 from crates.io, checked by `scripts/check-pinned-override-rot.py`.
   The root `[patch.crates-io]` keeps only `candle-kernels` by git at
   `182ceab0`, which carries the static CUDA runtime into FathomDB-built
   artifacts (ADR-0.8.25-driverless-cuda-runtime-linkage), and
   `libsqlite3-sys`. Exactly one `cudarc` 0.19 is in the graph,
   `cudarc-fathomdb`, enforced by `scripts/check-single-cudarc.sh`.
2. **crates.io consumers get `tegra-pool`.** Because the fork crates are
   published, a downstream `embed-cuda` build (which implies `tegra-pool`,
   decision 11) with no `[patch]` table resolves and builds.
   `scripts/check-tegra-pool-registry-build.sh` proves it with a path copy
   of the FathomDB embedder from the tree under test, depending on the
   published fork crates (`ENABLE=embed-cuda-only` names only
   `embed-cuda`); it does not build published FathomDB crates, which do
   not exist until the release. Any registry `embed-cuda` build, with or
   without `tegra-pool`, links the CUDA runtime dynamically through
   upstream `candle-kernels` 0.10.2 and needs `LIBRARY_PATH` set to the
   CUDA library directory to link; `tegra-pool` adds no link requirement
   of its own. The CPU-loadable guarantee of REQ-068 applies only to
   FathomDB-built artifacts.
3. **The marker feature is retired.** `tegra-pool` no longer enables
   `fathomdb-private-pool`; the package name `cudarc-fathomdb` already
   rules out upstream cudarc.
4. **Evidence for the primitive lives in the fork.** The fork's
   requirements, design, tests, reviews and qualification receipts
   (`fathomdb/` on `fathomdb/0.19.10`; G1-G6, G8 and the CI receipt at
   `b777f8d9`) replace `scripts/tests/test_vendored_cudarc.sh`. The item
   table with upstream status and removal paths is the published crate's
   `FATHOMDB.md` (items 1-9). FathomDB guards lock drift between its
   `Cargo.lock` and the fork's last recorded run
   (`scripts/check-cudarc-fathomdb-lock-set.py`, owner ruling Q8), and
   records the exact fork versions it relies on in
   `scripts/pinned-override-rot.json`.
5. **Library loading.** The fork loads only CUDA libraries of the major it
   was built for (item 9). FathomDB's check of the loaded libraries, its
   typed refusal and `doctor cuda-libraries`, and the probe-time
   compute-capability floor, are CUDA-version-policy decisions, recorded
   in the amendment of 2026-10-13 to
   ADR-0.8.28-cuda-version-support-policy.
6. **Close.** With threshold `0`, freed memory returns to the system
   automatically. With `max`, freed memory stays reserved while any
   FathomDB model is open in the process; when the last one closes,
   FathomDB synchronizes the decision's context and trims unused pool
   memory, unless the context is lost. Module-level models and the
   engine reranker stay open for the process's life, so a process
   holding one keeps the memory until exit (owner rulings Q29, Q34).
7. **Tegra facts (fork measurements, JetPack 6).** A capped pool serves
   about a third of its `maxSize`, so the 3 GiB default serves about
   1 GiB; a process has room for about four uncapped pools; `cuMemGetInfo`
   is not a release signal on Tegra. None changes the sizing rule.
8. **Why CUDA 12.x** is recorded in ADR-0.8.28-cuda-version-support-policy
   (Slice 31 plan § 15 Q6).
9. **The pool settings are open settings.** Every SDK's open takes
   `cuda_pool` (`auto`, `on`, `off`), `cuda_pool_max_size` (bytes) and
   `cuda_pool_release_threshold` (`0`, `max`); each `FATHOMDB_POOL_*`
   environment variable applies only when the open does not set its
   setting. The first configuration in force holds for the life of the
   process; an open that sets one of them to another value fails with an
   invalid-argument error naming the setting, the value in force and its
   source (owner rulings Q33, Q35; Slice 31 design § 15). This amends the
   "Settings" component, which named only environment variables.
10. **Plain Rust processes reach the pool through the probe's `cuInit`.**
    The load-time gate guards against `cuInit` running after the V8 heap
    has fragmented the Tegra address space, a Node hazard. In a Rust
    process the embedder probe's `cuInit` is the first call, made before
    the first device build. The decision therefore admits a process whose
    most recent `cuInit` succeeded even if no load hook ran. FathomDB makes
    no new `cuInit` call. `FATHOMDB_CUDA_EARLY_INIT` governs only the Node
    and Python load hooks, and the pool opt-out is `cuda_pool = off` (owner
    ruling Q36; Slice 31 design § 14.2). This amends component 4, "Rust has
    no load hook".
11. **The CUDA features imply `tegra-pool`.** In every FathomDB crate that
    has the feature, `embed-cuda` and `rerank-cuda` enable `tegra-pool`.
    The pool code is compiled only on aarch64 Linux, so x86 builds are
    unchanged, and the run-time opt-out `cuda_pool = off` replaces the
    compile-time one. The feature name is kept for 0.8.28, and a later
    release may retire it (owner ruling Q37; Slice 31 design § 14.3). On an
    aarch64 Linux host that is not a Jetson, a source-built Python
    extension therefore runs `cuInit` at import, with hazard H-1, as Node
    already does; this is documented as-is (owner ruling Q38, 0.8.29 todo
    `TC-4755a9d9`).

**Superseded statements in this ADR.**

- Front matter `blast_radius`, "vendored cudarc 0.19.7; Fathom Candle
  fork 0.10.3": superseded by `cudarc-fathomdb` 0.19.10 and the Candle
  fork crates 0.11.0, with `candle-kernels` by git patch.
- Components, item 1, "The vendored cudarc 0.19.7 gains `CudaMemPool`
  (create, attribute, trim, raw, destroy on drop) …": superseded. The
  primitive is in `cudarc-fathomdb` 0.19.10; `attribute` is replaced by
  typed reads (`reserved_current`, `reserved_high`, `used_current`,
  `used_high`, `release_threshold`, `max_size`).
- Components, item 1, "The no-op feature `fathomdb-private-pool` marks
  the vendored crate. `FATHOMDB-PATCH.md` gives each item an upstream
  status and removal path, and `scripts/pinned-override-rot.json` lists
  exactly that set": superseded by decisions 3 and 4 above.
- Components, item 2, "core, nn and transformers move to 0.10.3 together
  (candle-kernels stays 0.10.2)": superseded by 0.11.0 from the registry;
  `candle-kernels` stays 0.10.2, by git patch at `182ceab0`.
- Components, item 6, "It is supported only in repository and artifact
  builds, which carry the vendored cudarc; against registry cudarc it
  fails at resolution with the marker message
  (`scripts/check-tegra-pool-registry-build.sh`)": superseded by decision
  2.
- Components, item 4, "Rust has no load hook: a Rust process reports
  `cuinit_not_at_load` and takes the 0.8.27 path": superseded by decision
  10.
- Decision, "Settings are environment variables in FathomDB, never in
  cudarc": superseded by decision 9 (they are open settings, with the
  environment variables as fallbacks; still never in cudarc).
- Components, item 6, `tegra-pool` "non-default": superseded by decision
  11.
- Components, item 5, the co-resident reset: unchanged; still
  unsupported in 0.8.28, fix planned for 0.8.29.

**Consequences.** Upgrading cudarc or Candle is a new fork version,
qualified in the fork, then an exact-version change in FathomDB under
R-PIN. A fork item that FathomDB relies on cannot change without a
reviewed change to the pinned-override checker. The new public surface
of the library check is listed in the CUDA version support policy ADR's
amendment.
