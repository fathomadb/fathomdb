---
title: FathomDB 0.8.28 Slice 30 — cudarc upstream package (prepared, not posted)
status: PROPOSED (prepared 2026-10-08 in S30-T9; owner sign-off required; nothing posted)
target_release: 0.8.28
observed_on: 2026-10-08
---

# cudarc upstream package for the private memory pool

> **Owner sign-off required. Nothing has been posted.** No issue, comment
> or pull request has been opened on
> [chelsea0x3b/cudarc](https://github.com/chelsea0x3b/cudarc) or on Candle.
> Everything below is a draft (ruling 38; todo `TC-b8e5fe4d`). 0.8.28 does
> not wait for upstream.

This is the S30-T9 upstream package of the
[Slice 30 plan](plan.md) § 7 (AC30-13). It builds on:

- the shape notes in
  `dev/plans/0.8.28/prework/cudarc-upstream-patch-notes.md`;
- the vendored backport, `third_party/cudarc-0.19.7/fathomdb-private-pool.patch`
  (FATHOMDB-PATCH.md items 5 and 6);
- the study results, `dev/plans/runs/0.8.28-pool-study/results.md`.

The issue threads were **not** re-read for this package (§ 5). Quotations
come from the notes and the study plan's survey of 2026-10-05 and
2026-10-06.

## 1. Issue draft for chelsea0x3b/cudarc

Intended as a comment on #536 (safe pool support) or a new issue that
links it, whichever the owner prefers. It asks before any pull request is
written, as the notes recommend.

````markdown
### Proposal: an opt-in, per-context private memory pool

**Problem.** A library that uses cudarc allocates from the device's
current memory pool (`cuMemAllocAsync`) or synchronously. It has no safe
way to allocate from a pool of its own. NVIDIA's guidance for the
stream-ordered allocator is that libraries should not change a device's
pool, because that affects the whole application, and should instead
create their own pool and allocate from it with `cuMemAllocFromPoolAsync`.
#544 added the result-layer calls; there is no safe layer yet (#536).

**Where we hit it.** On a Jetson AGX Orin 64 GB (L4T R36, CUDA 12.6) the
default pool needs one contiguous range of about 20960 MiB of process
address space. In a Node.js process the V8 heap fragments that range, and
`cuDeviceGetDefaultMemPool` / `cuMemAllocAsync` return
`CUDA_ERROR_OUT_OF_MEMORY` with more than 50 GB free; only synchronous
allocation works, 2-5x slower. A pool created by the library maps a much
smaller range (about a third of its `maxSize` on that device), so a
capped private pool keeps stream-ordered allocation. On that device:

- every allocation that fit succeeded in 751 of 751 processes using a
  private pool, including 8 concurrent processes and a 20-minute soak;
- creating the pool lazily, after the heap had grown, succeeded in 720 of
  720 processes;
- steady-state speed matched the default pool (ratio 0.986, 95 % CI
  [0.907, 1.114]) and was 1.9-4.9x faster than synchronous allocation;
- the device's current pool was never the private pool, and another user
  of the device kept allocating from the default pool.

The device numbers are motivation only; nothing device-specific is
proposed.

**Proposed API (opt-in; nothing changes for existing callers).**

- `MemPoolProps { max_size, release_threshold }` and
  `CudaMemPool::create(ordinal, &props) -> Result<CudaMemPool, DriverError>`,
  a safe owner of a `cuMemPoolCreate` pool with `attribute`, `trim_to`,
  `raw`, and `Drop` calling `cuMemPoolDestroy`. The pool is never made the
  device's current pool.
- `CudaContext::new_with_mem_pool(ordinal, Arc<CudaMemPool>)`: retains the
  primary context like `new`, rejects a pool of another device with
  `CUDA_ERROR_INVALID_VALUE`, and allocates every buffer with
  `cuMemAllocFromPoolAsync` from that pool.
- `AllocMode { Default, Synchronous, Private }` and
  `CudaContext::alloc_mode()`, fixed at construction (as suggested in
  #174, "so we only have to run it once"); `CudaContext::mem_pool()`
  returns the pool.
- Frees stay the ordinary `cuMemFreeAsync`, which returns memory to the
  pool it came from. `CudaSlice` and its `Drop` are unchanged: no new
  slice kind (#594).
- Lifetime: the context holds the pool, streams hold the context, slices
  hold their stream, so the pool is destroyed after the last of them.

**What is deliberately not proposed.** No environment variables, no
global state, no `target_arch` cfgs (#194), and no automatic policy: when
to use a private pool, how large, and what to do when it is exhausted are
the caller's decision (#106: opt-in). The driver's own
`CUDA_ERROR_OUT_OF_MEMORY` is returned at the cap.

**Zero-length allocations.** No special case. A zero-byte request goes to
the driver; `cuMemAllocFromPoolAsync(0)` succeeds with a null pointer that
`cuMemFreeAsync` accepts (60 of 60 runs). That keeps the null pointer the
driver's, not one synthesized by cudarc (#194).

**Open questions for you.**

1. A context constructor, as above, or a stream-level setting?
2. `CUmemPoolProps.maxSize` exists only from `cuda-12020`. Should
   `max_size` exist only under those features, so that a size request is
   never silently dropped on older bindings?
3. One pull request or two (the pool, then the context constructor)?

We have a C reproducer and device tests in cudarc's `mod tests` style, and
can run the tests on a Jetson AGX Orin and an x86_64 host.
````

## 2. Commit series

The upstream series is a re-port onto cudarc `main`, not a diff of the
vendored 0.19.7. The vendored patch also carries FathomDB's 0.8.27
allocator fallback (items 1–4), which is not proposed, so some hunks
simplify upstream. Two pull requests, each standing alone (notes § 3).

**Pull request 1: the pool.**

| # | Proposed commit | Vendored source (`fathomdb-private-pool.patch`) |
| --- | --- | --- |
| 1 | `driver: add MemPoolProps and a safe CudaMemPool` — `create`, `attribute` (64-bit attributes only), `trim_to`, `raw`, `Drop` (`cuMemPoolDestroy`); `maxSize` gated per open question 2 | new `src/driver/safe/mem_pool.rs`; the `mem_pool` module and `CudaMemPool`, `MemPoolProps` re-exports in `src/driver/safe/mod.rs` |
| 2 | `driver: test CudaMemPool properties and release threshold` | tests `pool_props_are_pinned_device_memory_on_the_ordinal`, `a_created_pool_carries_its_release_threshold` (module `fathomdb_private_pool` in `core.rs`), moved into the new module's `mod tests` |

**Pull request 2: the context.**

| # | Proposed commit | Vendored source |
| --- | --- | --- |
| 3 | `driver: add AllocMode and CudaContext::alloc_mode()` | `AllocMode`, `alloc_mode_of` and `alloc_mode()` in `core.rs`; the `AllocMode` re-export in `mod.rs`. Upstream's `Default` / `Synchronous` come from `has_async_alloc` alone |
| 4 | `driver: CudaContext::new_with_mem_pool and mem_pool()` | the `mem_pool` field on `CudaContext` (`None` in every other constructor), `new_with_mem_pool`, `mem_pool()`, and the pool branches of `CudaStream::null` and `CudaStream::alloc`. The doc sentence about the aarch64 process-wide decision is dropped |
| 5 | `driver: test private-pool contexts` | tests `a_context_with_its_own_pool_is_private_whatever_the_device_decided` (simplified to the upstream states), `a_private_context_allocates_from_its_pool_and_leaves_the_current_pool_alone`, `zero_length_private_allocations_are_null_and_free_cleanly`, `the_pool_is_destroyed_last_and_cleanly`, `a_pool_of_another_device_is_rejected`, `ordinary_contexts_report_default_or_synchronous` |
| 6 | `examples: allocate from a private pool` | new; no vendored counterpart (notes § 3) |

**Kept out of the series:**

- the `Cargo.toml` feature `fathomdb-private-pool` (item 6, local only);
- the test helper `AllocModeByDevice::decided` and the tests
  `a_private_context_never_consults_or_records_the_process_decision` /
  `private_context_in_a_fresh_process`, which test the interaction with
  the local item 2 table;
- every `FATHOMDB PATCH` marker and FathomDB name.

## 3. Upstream status per item

This matches `third_party/cudarc-0.19.7/FATHOMDB-PATCH.md` ("Item
status") and the `patch_items` of `scripts/pinned-override-rot.json`, as
checked on 2026-10-08.

| Item | Upstream status | Removal path |
| --- | --- | --- |
| 1. Allocator fallback | local only | when Candle's cudarc has an equivalent fallback, or no artifact needs it |
| 2. Per-device decision | local only | with item 1 |
| 3. Zero-length synchronous allocation | local only | with item 1 |
| 4. Tests (`fathomdb_alloc_fallback`) | local only | with items 1–3 |
| 5. Private memory pool primitive | proposed upstream, not posted (this package) | when Candle's cudarc has `CudaMemPool` and `CudaContext::new_with_mem_pool` |
| 6. Feature marker `fathomdb-private-pool` | local only | with item 5 |

When the owner posts, item 5 becomes "proposed (posted)". The statuses in
`FATHOMDB-PATCH.md` and the pinned-override record change together, in
the same commit.

## 4. Candle note (optional)

Under the same banner: owner sign-off required; nothing posted.

The Fathom Candle fork's branch `fathomdb/0.8.28-cuda-from-context` adds
`CudaDevice::from_context(Arc<CudaContext>)` and
`Device::new_cuda_from_context` (test `73e267d0`, implementation
`5b74532e`). They wrap a caller-built cudarc context, such as a
private-pool context, so that every Candle allocation goes through that
context's allocator on its default stream. A short note to Candle would
propose the same two constructors. It only becomes useful upstream once
cudarc has a way to build such a context, so it follows the cudarc
outcome.

## 5. Re-read before posting

None of these threads was re-read for this package. Each must be
re-read on the day of posting, and the notes' § 2 updated.

| Thread | Why it matters | State |
| --- | --- | --- |
| #536 safe pool support | where the issue goes; maintainer reply? | NOT YET RE-READ |
| #544 result-layer pool calls | the layer the primitive sits on | NOT YET RE-READ |
| #553 `has_async_alloc()` getter | existing allocator-state accessor | NOT YET RE-READ |
| #558 safe `CudaMemPool` (closed by author) | prior attempt; why it closed | NOT YET RE-READ |
| #594 capture-scoped graph pool | "needs redesign"; `CudaSlice` enum objection; the `Arc<dyn Any>` owner proposal (UNVERIFIED) | NOT YET RE-READ |
| #519 slice ownership | the ownership question #594 raises | NOT YET RE-READ |
| #174 decide once | per-context state at construction | NOT YET RE-READ |
| #106 opt-in | non-standard behaviour opt-in | NOT YET RE-READ |
| #194 no device-specific code; null pointers | no cfgs; zero-length behaviour | NOT YET RE-READ |
| #555, #556, #595 | claims need failing tests or benchmarks | NOT YET RE-READ |

Also before posting: rebase onto cudarc `main` (0.19.10 on 2026-10-05),
and run the notes' § 6 checklist (CI feature matrix, `no-std` clippy,
device tests, no FathomDB names).

## 6. NVIDIA report addendum (2026-10-08)

The only NVIDIA report draft in the repository,
`dev/plans/runs/0.8.27-slice-110-tegra/driver-isolation-evidence/nvidia-report-draft.md`,
is marked superseded (2026-10-07). The corrected draft is kept outside the
repository, so no current draft exists here to amend, and this addendum is
recorded in its place:

- **Adoption outcome.** FathomDB 0.8.28 adopts a private, capped CUDA
  memory pool (`cuMemPoolCreate` + `cuMemAllocFromPoolAsync`, release
  threshold 0, 3 GiB on the AGX Orin 64 GB) as its allocator on that
  device, behind the `tegra-pool` build feature, instead of the default
  pool or synchronous allocation (study results § 14). It works around the
  default-pool failure; it does not fix it.
- **The `cuInit` finding stands.** `cuInit` still fails with
  `CUDA_ERROR_OUT_OF_MEMORY` once no 4 GiB hole is left in
  [8 GiB, 128 GiB) (study R4: every late-import failure was `cuInit`, with
  or without the pool). FathomDB's answer is to run `cuInit` at module
  load and fall back to the 0.8.27 path when it did not run; the driver
  behaviour itself is unchanged and remains NVIDIA's to address.
- Whoever posts the corrected report should add these two points to it.
