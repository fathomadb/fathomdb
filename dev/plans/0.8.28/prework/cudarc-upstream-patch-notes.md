---
title: FathomDB 0.8.28 prework — notes for a cudarc upstream pool patch
status: PROPOSED (standing note; preparation only, 2026-10-07)
target_release: 0.8.28
observed_on: 2026-10-07
---

# Notes for a cudarc upstream memory-pool patch

This is a standing note on how to shape a memory-pool contribution to
[chelsea0x3b/cudarc](https://github.com/chelsea0x3b/cudarc) so that it
fits that repository. It is preparation only. **Nothing is posted upstream
— no issue, comment or pull request — without the owner's sign-off.**

**Sources.**

- Statements about upstream come from three places:
  - the study plan's "Upstream cudarc compatibility" survey (2026-10-05);
  - the protocol's pool-shape assessment (§ 10 item 6, 2026-10-06), which
    quotes the issue threads;
  - the vendored crate at `third_party/cudarc-0.19.7/`, which is the
    published 0.19.7 plus the FathomDB patch described in
    `third_party/cudarc-0.19.7/FATHOMDB-PATCH.md`.
- A quotation here was verified when it was read for one of those
  documents. The issue threads were not re-read for this note.
- **UNVERIFIED** marks anything not checked against a primary source.
- Re-read every cited thread on the day an issue or pull request is drafted.

## 1. Scope: what goes upstream and what stays in FathomDB

**Upstream: the pool primitive only.** Each item is opt-in and policy-free.

| Item | Shape | Basis |
| --- | --- | --- |
| `CudaMemPool` | A safe wrapper with `Drop` (`cuMemPoolDestroy`) over the result-layer calls already merged in #544 (`result::mem_pool::{create, destroy, trim_to, get_attribute, set_attribute, alloc_async}`). Constructor from an ordinal and a small properties struct (`max_size`, `release_threshold`); attribute read; `trim_to`. | #544 merged the driver calls and left the safe layer for later; #536 asks for "a safe-level `CudaMemPool` type with Drop" |
| `CudaContext::new_with_mem_pool(ordinal, Arc<CudaMemPool>)` | A context on the primary context that allocates with `cuMemAllocFromPoolAsync` from its own pool and never reads or changes the device's current pool | NVIDIA's library guidance (§ 4); #106 and #174 (opt-in, decided once) |
| `AllocMode::Private` | The context's allocator state, fixed at construction | #174: "so we only have to run it once" |
| `CudaContext::mem_pool()` | Accessor for the context's pool | keeps the pool's lifetime visible to the caller |
| `CudaSlice::drop` | **Unchanged.** A private-pool slice is freed with the ordinary `cuMemFreeAsync`; there is no new slice kind | #594 (§ 2) |

**Stays in FathomDB:**

- the sizing policy (1/20 of device memory within [2, 3] GiB, and the
  quarter rule);
- Tegra and integrated-device gating, and the device-class table;
- every setting: pool mode, `FATHOMDB_POOL_MAXSIZE` and the release
  threshold (study plan, design note "Settings and constants (ruling 31)");
- early `cuInit` at module load (ruling 12; ruling 35 keeps it under
  discussion);
- the `cuda_pool_exhausted` error kinds and their SDK classes (ruling 33);
- the 0.8.27 synchronous fallback (`FATHOMDB-PATCH.md` items 1–3) and its
  process-wide decision table;
- every harness fact: the 1/3 capacity ratio, the 32 MiB chunks and the
  20960 MiB default-pool range.

**Not proposed at all:**

- the installed-pool rule, `CudaMemPool::install` and `AllocMode::Explicit`
  (`FATHOMDB-PATCH.md` item 5, the study's comparison arms A and B);
- the zero-length null-pointer synthesis (item 3).

## 2. Maintainer preferences

| Preference | Source | Status |
| --- | --- | --- |
| Per-context state, decided once at construction, preferably at runtime rather than through a build flag | #174: "so we only have to run it once"; "I like the runtime detection better than adding an additional compiler flag TBH" | verified (protocol § 10 item 6) |
| Non-standard behaviour is opt-in | #106 (2023-03-29): "Would prefer opt-in to the non-standard behavior, rather than opting out of it." | verified (same) |
| No device- or target-specific code (no `target_arch` cfgs) | #194: "We don't have any device specific code in cudarc" | verified (independent review, 2026-10-06) |
| No synthesized null pointers | #194 (2024-01-09): "I'm hesitant to use a null pointer" | verified (same) |
| No new `CudaSlice` kind; wants pools | #594 (maintainer review 2026-09-22): "CudaSlice is almost turning into an enum at this point … We should probaly discuss this a bit more" and "I definitely want to support memory pools"; labelled "needs redesign" | verified (plan survey) |
| An `Arc<dyn Any>` owner field on `CudaSlice` as an alternative | #594 thread | **UNVERIFIED** (reported by the 2026-10-06 review; not re-read). Whether that would be preferred to a per-context pool is open. |
| No globals | Non-sys source has no `Mutex`, `RwLock` or `thread_local!`; `OnceLock` only for library loading | survey of the source (plan, 2026-10-05) |
| No environment variables | No runtime environment switches in the crate; the owner relays that the maintainer accepts none | source survey verified; the maintainer statement is **UNVERIFIED** here (relayed, not quoted) |
| Claims need representative failing tests or benchmarks | #555, #556, #595 | verified (plan survey) |
| Safe pool support | #536 (open, 2026-02-28): no maintainer reply | verified as of 2026-10-06; re-check |

## 3. Shape: how the patch should look

**Layering.** Follow `sys` (bindgen) → `result` (thin unsafe calls
returning `Result<_, DriverError>`) → `safe` (owning types). #544 already
provides the `result` calls, so the patch touches `safe` only:

- a new `src/driver/safe/mem_pool.rs`;
- the constructor and the `AllocMode` state in `core.rs`;
- re-exports in `safe/mod.rs`.

No `sys` or `result` change should be needed. Recheck this against `main`.

**Errors.** Return `DriverError` (`result::DriverError(pub sys::CUresult)`)
as every safe constructor does:

- no new error type;
- no panics;
- a pool of another device is `CUDA_ERROR_INVALID_VALUE`;
- pool exhaustion is the driver's own `CUDA_ERROR_OUT_OF_MEMORY`.

Callers such as FathomDB classify it themselves.

**CUDA-version gating.**

- `CUmemPoolProps.maxSize` exists in the bindings only from `cuda-12020`.
- The study's prototype silently ignores `max_size` on earlier bindings.
  For upstream, prefer making the field, or the ability to set it, exist
  only under `cuda-12020` and later, so that a size request is never
  dropped. This is an open shape question for the maintainer.
- Everything must compile under every `cuda-*` feature, on Windows, and
  under the `no-std` clippy check.

**Style.**

- Short doc comments stating the contract (inputs, errors, safety). No
  measurement narrative: the FathomDB patch's long, measurement-heavy
  comments were flagged as off-style (plan survey).
- Naming follows the existing types (`CudaContext`, `CudaStream`,
  `CudaSlice`, `MemPoolProps`-style properties).

**Tests and examples.**

- Tests go in the module's `#[cfg(test)] mod tests`, as in `core.rs`. They
  use a real device and skip cleanly when none is present, as the existing
  GPU tests do. Upstream CI is compile-only, with no GPU and no aarch64 job
  (plan survey).
- Add one example in the `examples/NN-name.rs` series, for instance
  "allocate from a private pool".

**Keep it small and splittable.** Possible pull requests, each standing
alone:

1. `CudaMemPool` and its properties struct, with tests.
2. `CudaContext::new_with_mem_pool`, `AllocMode::Private` and
   `mem_pool()`, with tests and the example.

Ask on #536 before writing either, because the maintainer has not yet
chosen between a context constructor and a stream-level setting (plan,
pool-shape assessment).

## 4. Evidence to attach

1. **Why a private pool.** NVIDIA, "Using the NVIDIA CUDA Stream-Ordered
   Memory Allocator, Part 2": "In general, libraries should not change a
   device's pool, as doing so affects the entire top-level application."
   A library may instead "create its own pool and then allocate from that
   pool using `cudaMallocFromPoolAsync`" (quoted in protocol § 10 item 6).
   So the primitive never calls `cuDeviceSetMemPool`.
2. **The Tegra motivation, as motivation and not API.** On a Jetson AGX
   Orin 64 GB the default pool needs one contiguous 20960 MiB range of
   address space. Node/V8 fragment it, so `cuDeviceGetDefaultMemPool`
   fails with `CUDA_ERROR_OUT_OF_MEMORY` while synchronous allocation
   works. A private pool sized `maxSize` needs only about a third of that
   as contiguous range there (study results, C3 and R5).
   - That ratio is a measured fact of one device. It must not appear as
     logic or as a default in the patch.
3. **A small, self-contained reproducer.**
   - The C reproducer `minimal_repro.c`
     (`dev/plans/runs/0.8.27-slice-110-tegra/driver-isolation-evidence/`),
     plus a cudarc-only Rust test or example that allocates through the new
     constructor.
   - Not FathomDB's harnesses, engine or bindings.
   - A benchmark in the `examples/` style if speed is claimed (protocol
     § 10 item 3).

## 5. A separate candidate, kept out of the pool patch

**`CudaSlice` drop SIGSEGV after a co-resident context reset.** After
another library calls `cuDevicePrimaryCtxReset` or `cudaDeviceReset`,
`CudaSlice::drop` calls `CudaStream::wait` (`cuStreamWaitEvent`) on
destroyed stream and event handles, and libcuda faults.

- **The record.** It is recorded as todo seq 274 in
  `dev/todos-and-considerations-ledger.jsonl`. That entry is on branch
  `llm/slice117-jetson-node-cuda-plan` and was written when the crash was
  seen at exit.
- **What the study found** (results § 12.3, § 12.7.4, and the revision-6
  rerun):
  - it happens on every allocator path, the shipped synchronous one
    included;
  - with the engine close fix, it now fires inside `Engine::close`.
- **Why it is separate.** It is a general lifetime issue, not a pool
  issue. Folding it into the pool patch would tie two unrelated
  discussions together.
- **If proposed:** a separate issue with its own minimal reproducer (a
  cudarc context, one slice, a reset through the driver API, then a drop).
  No fix design is proposed here. Skipping the wait and the free when the
  context is gone would leak the device memory, which the reset already
  freed.

## 6. Pre-submission checklist

- [ ] Owner sign-off before any issue, comment or pull request is opened.
- [ ] Re-read #536, #544, #594, #106, #174 and #194 on the day, and update
      § 2.
- [ ] Rebase on cudarc `main`. On 2026-10-05 `main` was 0.19.10. Re-port
      the patch rather than diffing against the vendored 0.19.7.
- [ ] Run its CI locally where possible:
  - `cargo fmt`;
  - `cargo clippy` with the full feature list of `cargo-clippy.yaml`,
    including `no-std` (vendored 0.19.7 workflow; check `main`'s);
  - `cargo check --all-targets` across the `cuda-*` version matrix of
    `cargo-check.yaml`;
  - the device tests on a real GPU: this Orin, and an x86_64 host if one
    is available.
- [ ] No FathomDB names in code, comments, tests or examples. No
      `FATHOMDB PATCH` markers, no study terms (variants, rulings), no
      environment variables, no `target_arch` cfgs.
- [ ] No change to `CudaSlice::drop` or to the default allocator choice.
- [ ] The diff is split as in § 3, and each part builds and tests alone.
- [ ] Evidence per § 4: the reproducer and the NVIDIA guidance. Claims
      without a test or benchmark are removed.
