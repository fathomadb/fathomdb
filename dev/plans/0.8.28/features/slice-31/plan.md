---
title: FathomDB 0.8.28 Slice 31 — the private CUDA memory pool on forked cudarc, CUDA 12.6 and 13.x, through crates.io (D28-13) plan
status: APPROVED SLICE SCOPE (owner, 2026-10-11); revision 7 — W0 merge-forward plan added, merge awaits owner approval; revision 6 owner rulings and review fixes retained
target_release: 0.8.28
observed_on: 2026-10-11
---

# 0.8.28 Slice 31: the private CUDA memory pool on forked cudarc

This slice makes FathomDB's CUDA memory pool excellent end to end: the pool
primitive and the Jetson allocator fallback live on the owner's cudarc fork,
rebased onto upstream v0.19.10 and correct for every cudarc CUDA feature
(11.4 through 13.4); Candle and FathomDB consume that fork; the fork is
published so crates.io consumers of `fathomdb` get it; and a tidy cudarc
memory-pool proposal is prepared (not posted). It is the working plan for
D28-13 in `dev/plans/0.8.28-draft-scope.md` until the 0.8.28 plan,
release-state file and board exist; those must copy it. Slice number 31
(owner, 2026-10-11). The session goal is [GOAL.md](GOAL.md). Nothing in
this slice writes to `release/0.8.27`.

## 0. Owner instructions and where this plan covers them

| # | Owner instruction (2026-10-11) | Covered by |
| --- | --- | --- |
| I-1 | Slice 31 is approved | front matter; § 12 R0, R0-rev4, R0-rev5 |
| I-2 | Slice 31 gets its own worktree; the new `release-0.8.28` worktree holds the merge-forward plan and later merge | § 9 W0 steps 1–3; [merge-forward plan](merge-forward-plan.md) |
| I-3 | A goal so the work can continue and complete | [GOAL.md](GOAL.md) |
| I-4 | Bring the cudarc fork to v0.19.10, then apply our changes | § 9 W4 steps 1–2 |
| I-5 | Compatible with the latest CUDA 13.x; read release notes and published docs | § 2.3; REQ31-03; D-8; T-MATRIX, T-CUDA |
| I-6 | Subagents as appropriate | § 11 |
| I-7 | Appropriate reviews | § 12 |
| I-8 | Requirements + acceptance criteria, design, test plans reviewed | § 12 R-REQ, R-DES, R-TEST |
| I-9 | Opus reviews, not codex | § 11 review spawn; § 12 |
| I-10 | No steward or orchestrator | § 11 |
| I-11 | TDD (RED/GREEN) | § 10 (all code packages W4–W8) |
| I-12 | Code is reviewed | § 12 R-CODE-1, -2, -2b, -3, -4, R-PROP |
| I-13a | Implement a memory pool | REQ31-02, REQ31-13; § 6; § 7 |
| I-13b | Vendor libraries and forks used with excellence | REQ31-06; D-10, D-14 |
| I-13c | A tidy cudarc memory-pool proposal | REQ31-09; § 9 W7; § 12 R-PROP |
| I-13d | FathomDB 0.8.28 with excellence and pool support, including Jetson Tegra | REQ31-04, REQ31-10, REQ31-13; § 9 W6; T-ORIN |
| I-14 | Orin `10.83.10.13`, ssh/scp approved; a Claude agent may run there | § 2.2; § 11 |
| I-15 | Check outside PATH for libraries | § 2.2; § 9 W0 step 4; GOAL.md |
| I-16 | Excellence; this is a memory pool | § 6 D-1…D-15; § 7 T-PROP, T-CONC, T-FAULT, T-CAPTURE, T-MULTI, T-TEARDOWN, T-PERF |
| Q4 | Never install the R615 driver; it would disable `windchill3` | § 1; § 2.2; § 9 W0 step 4 guard; § 15 |
| Q5 | Tegra on CUDA 13 is UNMEASURED in Slice 31; propose doing it before the 0.8.28 release | REQ31-04; § 15; § 16 follow-up |
| Q6 | Build against CUDA 13.0 without changing `windchill3`'s primary CUDA 12.6; find why 12.x was chosen | § 2.2; § 9 W0 step 4; T-CUDA; § 15 Q6 |

## 1. Owner direction (2026-10-11)

- **Fork-first.** cudarc work on `coreyt/cudarc`, Candle work on
  `coreyt/candle-fathomdb`; nothing posted upstream until in-product use and
  owner sign-off. Ruling 38's "upstream-first in upstream style" governs the
  commit shape.
- **Consumption is a git dependency on the fork (2b)**; all six vendored
  items move to the fork. **End state: crates.io carries the patch, from the
  fork.** **Base: upstream cudarc v0.19.10.** **No `candle-kernels-fathomdb`
  now.**
- **Process:** the main thread runs the slice — no steward, no orchestrator;
  every review is an independent Opus subagent, not codex; TDD for all code.
- **CUDA:** shipping target stays CUDA 12.6 (x86 and Jetson Orin JetPack 6).
  CUDA 13 compatibility is proven by the full compile matrix through 13.4 and
  by CUDA 13.0 runtime tests on `windchill3`.
- **Never change `windchill3`'s driver** (stays 580.178.04; no R615 — Q4)
  **or its primary CUDA** (`/usr/local/cuda` → 12.6 — Q6). CUDA 13.0 is
  installed toolkit-only, user-local, side by side.
- **Tegra on CUDA 13 is UNMEASURED in Slice 31** (Q5); a follow-up before the
  0.8.28 release is proposed (§ 16).
- **Push policy:** fork and FathomDB branch pushes need no prompt; never push
  Memex or upstream repositories; crates.io publication is the owner's.

## 2. Current state (verified 2026-10-11)

| Fact | Evidence |
| --- | --- |
| The workspace patches `cudarc` to the vendored 0.19.7 tree | root `Cargo.toml` `[patch.crates-io]` |
| Vendored delta: items 1–4 (598 lines), items 5–6 with tests (676 lines) | `third_party/cudarc-0.19.7/FATHOMDB-PATCH.md` and patch files |
| Candle (fork rev `859b8ea1`) depends on `cudarc = "0.19.4"` | fork `Cargo.toml` |
| `fathomdb-embedder` depends on `cudarc = "=0.19.7"` on aarch64 Linux with marker feature `fathomdb-private-pool` | embedder `Cargo.toml` |
| `ug-cuda` → `cudarc` 0.17.8 (max CUDA 13.0) is lock-only; `cargo tree -i ug-cuda --features embed-cuda` is empty | `Cargo.lock`; research |
| Published `candle-core-fathomdb` 0.10.3 → upstream `candle-kernels` 0.10.2; the fork's static-CUDART kernels reach workspace builds only by git patch | fork `Cargo.toml`; `cargo info` |
| `coreyt/cudarc`: only `main` = upstream tag `v0.19.10` (`ec1cac71`); no tags | `git ls-remote` |
| cudarc v0.19.10 declares 19 CUDA features: `cuda-11040`…`11080`, `12000`…`12060`, `12080`, `12090`, `13000`…`13040`; upstream CI checks each × {ubuntu, windows} × {default, `no-std`} | `Cargo.toml`, `.github/workflows/cargo-check.yaml` at `ec1cac71` |
| `release/0.8.27` has 363 commits not in `release/0.8.28`, touching files W6 edits (`Cargo.toml`, `scripts/check-pinned-override-rot.py` and test, `release.yml`) | `git rev-list --count` |
| **Latent defect in today's vendored code:** `upgrade_device_ptr`'s safety doc (`core.rs:2797-2799`) says a pointer leaked from any context of the device is valid because all contexts share one decision; a `Private` context does not | vendored `core.rs` |
| x86 and Jetson builds share one pinned toolkit, `/usr/local/cuda-12.6` / nvcc 12.6.68 | `scripts/release/cuda-artifact-contract.sh:16-17` (rationale 11-15); `release.yml` GPU jobs |
| Slice 30's design, ADR (`ADR-0.8.28-tegra-private-cuda-pool.md`, index row 62) and qualification are the baseline not to regress | `slice-30/` |

### 2.1 Evaluation of v0.19.7 → v0.19.10

Verified in a scratch clone unless marked [I].

- 15 upstream commits: fixes #591 (`CudaSlice` drop/leak event waits gated on
  managed multi-stream sync) and #602 (`alloc_pinned` leak); additions
  `alloc_pinned_with_flags`, `CudaEvent::try_is_complete`,
  `CudaContext::wrap_borrowed_stream`, `CudaFunction::cu_function`,
  `CudaViewMut::into_view`; one break (`CUgraphInstantiate_flags` bitmask,
  #600); CUDA 13.3/13.4 features. No upstream pool work; #536 open; PR #594
  (graph memory pool) would conflict with items 1–3 and 5 [I].
- The delta applies as text; the private-pool `Cargo.toml` hunk fails on any
  git checkout and the marker feature is added by hand; the `CudaSlice::drop`
  hunk sits under #591's independent gate.
- Existing defects on both bases: `mem_pool.rs` fails with `cuda-13020`+
  (`CUmemLocation.id` in a union); `set_max_size`'s cfg list stops at
  `cuda-13020`; below 12020 `max_size` is silently ignored; x86 dead-code
  warning (`in_fresh_process`).
- RTX 3090: 62/62 non-ignored `driver::` tests incl. 19 FathomDB tests with
  `cuda-12060`; Candle `859b8ea1` checks clean. aarch64 paths not yet run on
  the rebased tree.

### 2.2 Hosts and toolchains (searched outside PATH)

| Host | Hardware | Driver | CUDA |
| --- | --- | --- | --- |
| `windchill3` (x86_64, this host) | 2× RTX 3090 (sm_86), Quadro K620 (sm_50) | 580.178.04 — **never changed** (Q4) | Primary toolkit `/usr/local/cuda` → 12.6 — **never changed** (Q6). CUDA 13.0 runtime/NVRTC/cuBLAS libraries only in pip wheels (`.venv/.../nvidia/cu13/`); no CUDA 13 `nvcc` yet. |
| Jetson AGX Orin 64 GB, `10.83.10.13` (`coreyt`; ssh/scp approved) | Orin iGPU (sm_87) | 540.5.0 | L4T R36.5.2 (JetPack 6), `/usr/local/cuda-12.6`; JetPack 6 cannot run CUDA 13 |
| `win-vm` | — | — | Windows `cargo check` |

**CUDA 13.0 on `windchill3` without touching the driver or primary CUDA
(Q6).** The NVIDIA runfile installs only the toolkit with `--toolkit`, and a
non-root install into a user directory needs no sudo
([installation guide](https://docs.nvidia.com/cuda/cuda-installation-guide-linux/index.html)):

```bash
sh cuda_13.0.<u>_<driver>_linux.run --silent --toolkit \
   --toolkitpath="$HOME/opt/cuda-13.0" --no-man-page
```

Run as `coreyt` without sudo, it cannot write `/usr/local/cuda`, the driver
or the package database. Builds select it per command only:
`CUDA_PATH=$HOME/opt/cuda-13.0 PATH=$HOME/opt/cuda-13.0/bin:$PATH
CUDARC_CUDA_VERSION=13000`. Driver 580 is CUDA 13.0's own branch (R580), so
13.0 builds — including Candle's PTX — run on it. **Guard:** W0 and every
qualification run record the `nvidia-smi` driver version and
`readlink -f /usr/local/cuda` before and after; any change stops the slice.
Downloading the runfile is network access beyond the package registries, so
the owner approves the download (W0 step 4).

### 2.3 CUDA 13.x (primary NVIDIA documentation)

Research subagent, 2026-10-11; the driver table and the Jetson upgrade table
were re-checked by the main thread. [I] = inference.

- **Latest: CUDA 13.4.2 (September 2026)**
  ([archive](https://developer.nvidia.com/cuda-toolkit-archive);
  [release notes, "13.4 Update 1"](https://docs.nvidia.com/cuda/cuda-toolkit-release-notes/index.html)).
- **Drivers:** 13.0 → R580, 13.1 → R590, 13.2 → R595, 13.3 → R610,
  13.4 → R615; any 13.x application runs on ≥ 580 under minor-version
  compatibility, but "applications that compile device code to PTX will not
  work on older drivers"
  ([minor-version compatibility](https://docs.nvidia.com/deploy/cuda-compatibility/minor-version-compatibility.html)).
  Candle's kernels are PTX, so on driver 580 only CUDA 12.x and 13.0
  workspace builds can run; 13.1+ builds compile but are runtime-UNMEASURED
  on `windchill3` (Q4). Forward compatibility does not apply to GeForce
  ([forward compatibility](https://docs.nvidia.com/deploy/cuda-compatibility/forward-compatibility.html)).
- **Jetson:** JetPack 6 cannot run CUDA 13; JetPack 7.2 (L4T 39.2) adds Orin
  with CUDA 13.2, upgradable to 13.3/13.4 via `cuda-compat-orin-13-4`
  ([CUDA for Tegra § 8.2.1](https://docs.nvidia.com/cuda/cuda-for-tegra-appnote/index.html)).
  Known issue 5699079 (L4T 39.2): an excessively large allocation may reboot
  the device. NVML is Thor-only; Orin compute capability comes from
  `CUDA_COMPUTE_CAP=87`.
- **Architectures:** CUDA 13 supports Turing onward; sm_86, sm_87 remain;
  sm_50 (K620) is not a CUDA 13 target
  ([nvcc](https://docs.nvidia.com/cuda/cuda-compiler-driver-nvcc/index.html)).
- **Pool-relevant API changes:** `CUmemLocation.id` in a union from 13.2;
  13.4 adds `localized` and location type `DEVICE_LOCALITY_DOMAIN`
  ([CUmemLocation 13.2](https://docs.nvidia.com/cuda/archive/13.2.0/cuda-driver-api/structCUmemLocation__v1.html));
  `CUmemPoolProps`: no `maxSize` ≤ 12.1, `maxSize` 12.2+, `usage` 12.6+,
  `gpuDirectRDMACapable` and `reserved[53]` in 13.4
  ([CUmemPoolProps](https://docs.nvidia.com/cuda/cuda-driver-api/structCUmemPoolProps__v1.html));
  new pool attributes (13.2: 9–14 incl. `MAX_POOL_SIZE`; 13.4: 15); 13.0
  host and managed pools; `cuCtxCreate` takes `CUctxCreateParams` in 13.0
  (cudarc keeps `cuCtxCreate_v4`, 12.5+)
  ([13.0 context API](https://docs.nvidia.com/cuda/archive/13.0.0/cuda-driver-api/group__CUDA__CTX.html)).
  Pool function signatures are unchanged between 12.x and 13.x; `maxSize` 0
  means system-dependent
  ([stream-ordered allocator](https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__MALLOC__ASYNC.html)).
- **cudarc version selection:** `CUDARC_CUDA_VERSION`, then the highest
  feature, then `nvcc --version` (exact match); issue
  [#525](https://github.com/chelsea0x3b/cudarc/issues/525) (Jetson built
  against 13.x bindings until `CUDARC_CUDA_VERSION=12060`). `build.rs`
  emits the env-pinned cfg in addition to any feature cfg (two version cfgs
  if both differ).
- **Candle:** upstream CI builds kernels in `nvidia/cuda:13.0.2`
  (`CUDA_COMPUTE_CAP=86`), PTX, `-std=c++17`; CCCL 3 needs C++17 [I: fine].
- **Artifacts:** 12.x and 13.x builds are separate artifacts (sonames and
  binding layouts differ) [I]. 0.8.28 artifacts stay on 12.6 (§ 15 Q6).

## 3. Target state

```text
fathomdb-embedder ──(cudarc = { package = "cudarc-fathomdb" })──┐
candle-core-fathomdb 0.10.4 ──(cudarc = { package = "cudarc-fathomdb" })──┤
                                                  cudarc-fathomdb (crates.io,
                                                  from coreyt/cudarc fathomdb/0.19.10)
```

- `cudarc-fathomdb` keeps `[lib] name = "cudarc"`; exactly one cudarc 0.19 in
  the graph, enforced by a guard.
- After publication: no `[patch.crates-io]` entry for cudarc or Candle
  core/nn/transformers; `candle-kernels` keeps its fork git patch.
- `third_party/cudarc-0.19.7` and every reference to it are gone; the item
  table lives in the fork's `FATHOMDB.md`.
- `release.yml` stays on CUDA 12.6 with `CUDARC_CUDA_VERSION=12060` pinned.

## 4. Artifacts (`dev/plans/0.8.28/features/slice-31/`)

Before the slice merges these live on `slice/0.8.28-31-cudarc-fork`; after
it merges, on `slice/0.8.28-31-post` (§ 13), which carries W7's documents,
the later `status.md`/`review.md` entries and W8 step 3.

| Artifact | Written in | Reviewed by |
| --- | --- | --- |
| `requirements.md` — final REQ31/AC31 (from § 5) | W1 | R-REQ |
| `design.md` — D-1…D-15 with citations; ADR amendment draft | W2 | R-DES |
| `test-plan.md` — every test traced to an AC31, host, command, expected result | W3 | R-TEST |
| `review.md` — each review's model, findings, dispositions | each review | — |
| `status.md` — step log, red/green shas, merge-forward shas, host guards | W0 onward | R-VERIFY |
| `qualification.md` — Orin and CUDA 13.0 results | W6 | R-VERIFY |
| `upstream-cudarc-package.md` — revised proposal (new file here; `slice-30/upstream-cudarc-package.md` gains a superseded pointer) | W7 | R-PROP |

## 5. Requirements and acceptance criteria (draft; finalized in W1, reviewed by R-REQ)

| Requirement | Acceptance criteria |
| --- | --- |
| **REQ31-01 Fork base.** `coreyt/cudarc` carries tag `v0.19.10`; `fathomdb/0.19.10` is cut from it; the delta is small upstream-style commits. | **AC31-01:** tree A = both patch files applied to `v0.19.10` with the recorded manual resolutions (marker feature, the #591-adjacent hunk); the branch with the fix and packaging commits excluded equals tree A (`git diff` empty); the fix and packaging commits are listed with their purpose. |
| **REQ31-02 Pool correctness.** The private pool never reads or changes the device's default or current pool; frees are stream-ordered; for allocations owned by `CudaSlice`, the pool outlives each allocation by construction; leaked pointers (`leak`) are the caller's responsibility and are never valid input to another allocator's context (documented safety precondition, corrected `upgrade_device_ptr` doc); `maxSize` is enforced; release threshold and trim obey their bounds; zero-length requests work; `Send + Sync` only where justified; multi-stream use with event tracking is correct. | **AC31-02:** on RTX 3090 (12.6, 13.0) and Orin (12.6): default-pool handle identical before and after; after drop + sync `used_cur` = `reserved_cur` = 0; with threshold T, after sync `reserved_cur` ≤ max(T, `used_cur`) rounded up to the allocation granularity; after `trim_to(k)`, `reserved_cur` ≤ max(k, `used_cur`); a request beyond the cap returns `DriverError(CUDA_ERROR_OUT_OF_MEMORY)` at the cudarc layer under N concurrent streams; T-PROP passes its case budget. |
| **REQ31-03 CUDA compatibility.** Compiles for all 19 cudarc CUDA features (11.4–13.4) with default features and `no-std`, on Linux and Windows; `max_size` exists only where the bindings support it; runs on CUDA 12.6 and CUDA 13.0 (the newest runnable on driver 580); no removed/deprecated Driver API; layout differences handled per D-8. | **AC31-03a:** T-MATRIX green, mirroring upstream `cargo-check.yaml`, plus clippy `--all-targets -D warnings`. **AC31-03b:** T-DEV and T-CANDLE pass with 12.6 (RTX 3090, Orin) and, if the owner approves the CUDA 13.0 download (W0 step 4), with 13.0 (RTX 3090); if declined, the 13.0 toolkit cells are UNMEASURED by owner decision. **AC31-03c:** the cudarc driver-only tests (no toolkit, nothing installed) pass on driver 580 when built with `CUDARC_CUDA_VERSION` = 13000, 13020 and 13040 — the only runtime evidence for the 13.2+ `CUmemLocation` union path; workspace runtime cells for 13.1–13.4 are UNMEASURED by owner ruling Q4. |
| **REQ31-04 Jetson Tegra.** Every Slice 30 gate holds on the Orin (JetPack 6, CUDA 12.6) on the new base. Tegra on CUDA 13 is UNMEASURED in Slice 31 (Q5). | **AC31-04:** `qualification.md` re-runs the Slice 30 gate rows on the Orin; `doctor cuda-allocator` reports `private`, 3 GiB; the Tegra/CUDA 13 row reads UNMEASURED citing owner ruling Q5 and the follow-up todo. |
| **REQ31-05 Fallback.** Items 1–4 act only on aarch64 Linux; elsewhere behaviour equals upstream. | **AC31-05:** cfg tests: on x86_64 `SYNC_FALLBACK` is `false` and allocation follows upstream `select_async_alloc`; the fallback's fresh-process tests pass on the Orin. |
| **REQ31-06 Vendor excellence.** Upstream APIs used (result-layer `mem_pool`, #544) instead of new bindings; minimal fork delta; every item has upstream status and removal path; Candle delta minimal; fork revs never rewritten after a pin; library allocations outside the pool are known (D-14). | **AC31-06:** `FATHOMDB.md` table complete; R-DES finds no duplicated upstream functionality; pinned-override gate passes. |
| **REQ31-07 One cudarc.** | **AC31-07:** the guard passes; a test introducing registry `cudarc 0.19.*` turns it red. |
| **REQ31-08 crates.io.** | **AC31-08 (post-publication):** `scripts/check-tegra-pool-registry-build.sh` with no patch table builds `tegra-pool` and passes the Orin GPU smoke. |
| **REQ31-09 Proposal.** Pool primitive only, on `proposal/mem-pool` from upstream `main`; upstream CI matrix and clippy clean; tests skip without a device; docs, example/benchmark, issue draft; coexists with #594. Not posted. | **AC31-09 (post-merge):** R-PROP approves; T-MATRIX green on the proposal branch. |
| **REQ31-10 FathomDB product.** `tegra-pool`, typed errors (`cuda_pool_exhausted`, `cuda_context_lost`), `doctor cuda-allocator`, early-`cuInit` contract unchanged; ADR amended for 2b; docs and changelog; no reference to the vendored tree remains. | **AC31-10:** full `./scripts/agent-verify.sh` green in the slice worktree with its own `.venv` (§ 9 W0 step 6); Slice 30 product-behaviour tests unchanged; `git grep -l third_party/cudarc -- . ':!dev/plans' ':!dev/todos-and-considerations-ledger.jsonl'` is empty; ADR amendment and changelog present. |
| **REQ31-11 Process.** | **AC31-11:** `status.md` lists red and green shas per step, each red re-run by the reviewer; `review.md` records every § 12 review with model `opus`. |
| **REQ31-12 Scope safety.** | **AC31-12:** no upstream issue/comment/PR; no commit from this slice on `release/0.8.27`; `windchill3` driver and `/usr/local/cuda` unchanged (guard log). |
| **REQ31-13 Performance.** No regression from the rebase. | **AC31-13:** Slice 30 gate G9 in full (`slice-30/plan.md` § 4.2) re-run on the Orin against the Slice 30 product: Node ratio ≤ 1.05 per measure with bootstrap 95 % CIs (CI upper > 1.10 → double N once), speed-up over synchronous > 1.5 (CI lower bound), import ≤ S + 5 ms, RSS ≤ S + 32 MiB; Python P / S ≤ 1.15; RTX 3090: median ratio new/0.19.7 ≤ 1.05 for embed steady state. |

**AC sets.** R-VERIFY before merge checks AC31-01…07, 10, 11, 12 (so far)
and 13. AC31-09 is checked at R-PROP after merge. AC31-08 and AC31-12 (final)
are checked at R-CODE-4 after publication.

## 6. Design topics (`design.md` answers each with citations)

Slice 30's design (`slice-30/design.md` § 3.1, § 4.5) is the baseline.

- **D-1 API shape for upstream.** `CudaMemPool` constructor and properties
  (device location, `maxSize`, release threshold; no IPC handles), naming in
  upstream conventions; `AllocMode` versus a narrower surface.
- **D-2 Lifetime and destruction.** The `Arc` chain slice → stream → context
  → pool; `cuMemPoolDestroy` with outstanding allocations (cite its deferred
  release); process exit and statics; **leak/upgrade hazard:** with a
  `Private` context on the device, a leaked pointer upgraded onto a
  `Synchronous` or default context would be freed with the wrong call —
  reject or make it an explicit safety precondition; fix the
  `upgrade_device_ptr` doc.
- **D-3 Stream ordering and reuse.** Cross-stream alloc/free,
  `CU_MEMPOOL_ATTR_REUSE_*` defaults, interaction with event tracking and
  #591.
- **D-4 Capacity and exhaustion.** `maxSize` availability by binding and by
  driver: a 12020+ build may load an older libcuda where `maxSize` lies in
  reserved bytes — check `cuDriverGetVersion` at creation and read back
  `MAX_POOL_SIZE` on 13.2+; `attribute()` accepts the 64-bit attributes;
  mapping from cudarc's `DriverError` to FathomDB's `cuda_pool_exhausted`;
  concurrent exhaustion.
- **D-5 Release threshold and trimming.** Threshold 0 versus higher,
  `trim_to` after close, latency effect.
- **D-6 Thread safety.** `Send`/`Sync` justification per type.
- **D-7 Observability.** Typed attribute reads (reserved/used current and
  high-water) and what FathomDB reports.
- **D-8 CUDA 11.4 → 13.4** (§ 2.3). One cfg-gated `CUmemLocation`
  constructor (plain `id` ≤ 13.1, union ≥ 13.2); `CUmemPoolProps` from
  `zeroed()` then field writes, never a literal; no derives 13.2+ bindings
  lack; no exhaustive matches on `CUmemLocationType`/`CUmemPool_attribute`;
  attributes read as raw integers; `cuCtxCreate_v4` only if a non-primary
  context is needed; `max_size` present only on 12020+ builds (or a nonzero
  value on older builds returns a typed error — never silently ignored); a
  cfg form that cannot silently drop new versions; the workspace pins
  `CUDARC_CUDA_VERSION` and forbids combining it with a different concrete
  `cuda-*` feature; whether embedding sm_86/sm_87 SASS in Candle's kernels is
  worth it (would let 13.x builds run on driver 580 — Candle-fork scope;
  decide, do not assume).
- **D-9 Graph capture.** Private-pool allocation during stream capture;
  coexistence with #594.
- **D-10 Vendor excellence.** Fork versus upstream APIs, Candle's minimal
  change, removal paths, `candle-kernels` git-pinned.
- **D-11 Packaging.** Naming, `[lib] name`, versioning, single-cudarc guard,
  marker feature's future.
- **D-12 Tegra.** Items 1–4 on the new base; `cfg(aarch64, linux)` also
  matches Thor/Grace/SBSA — decide whether to gate on
  `CU_DEVICE_ATTRIBUTE_INTEGRATED`; early-`cuInit` precondition.
- **D-13 Multi-device.** One pool per device, `cuMemPoolSetAccess` and peer
  access, ordinal checks (`windchill3` has two RTX 3090s).
- **D-14 Allocations outside the pool.** cuBLAS/cuBLASLt workspaces, static
  CUDART internals, module loads; whether to route cuBLAS workspace through
  the pool (`cublasSetWorkspace`); relevance to the Tegra default-pool
  failure.
- **D-15 Teardown and process lifecycle.** Lost context or device reset
  before drop (Slice 30 saw SIGSEGV in `cublasDestroy_v2`), pools in
  statics at exit, fork after init (H-1).

## 7. Test plan topics (`test-plan.md` expands each, traced to AC31)

- **T-UNIT** pure tests (props construction, cfg selection, error mapping).
- **T-DEV** device tests in cudarc's `mod tests` style, skipping cleanly
  without a device: create/drop, alloc/free, default pool unchanged, zero
  length, cap exhaustion, threshold and trim bounds (AC31-02 inequalities),
  attributes incl. 64-bit, leak/upgrade precondition.
- **T-PROP** proptest: random alloc, free, cross-stream free, sync, trim
  against a model; invariants `used_cur` ≤ `reserved_cur` ≤ cap, all zero
  after drop + sync; case budget stated in `test-plan.md`.
- **T-CONC** N threads × M streams; concurrent exhaustion; drop while other
  streams are live.
- **T-FAULT** `cfg(test)` seams in the result layer: `cuMemPoolCreate` OOM;
  `set_attribute` failure (create destroys the pool); OOM mid-sequence;
  destroy with allocations outstanding; sticky error or lost context before
  drop.
- **T-CAPTURE** private-pool allocation during stream capture: expected
  behaviour or error.
- **T-MULTI** two RTX 3090s: one pool per device, access and ordinal checks.
- **T-TEARDOWN** process exit with pools alive; fork after init (child
  behaviour documented); lost-context teardown.
- **T-PROC** fresh-process tests for the fallback's per-device table.
- **T-MATRIX** all 19 cudarc features × {Linux, Windows (`win-vm`)} ×
  {default, `no-std`}, mirroring upstream `cargo-check.yaml`; each cell sets
  `CUDARC_CUDA_VERSION` equal to that cell's feature; clippy `--all-targets -D warnings`; workspace
  with `embed-cuda`/`rerank-cuda` for 12060 (12.6 toolkit) and 13000 (13.0
  toolkit). Workspace cells needing a 13.1+ `nvcc` are out of scope (Q4/Q6).
- **T-CUDA** runtime cells:

  | Host / toolkit | Needs | Must pass |
  | --- | --- | --- |
  | RTX 3090, driver 580, cudarc driver tests, `CUDARC_CUDA_VERSION` = 13000, 13020, 13040 | driver only, nothing installed | T-DEV, T-CONC, T-FAULT (AC31-03c) |
  | RTX 3090, driver 580, 12.6 toolkit | present | everything |
  | RTX 3090, driver 580, user-local 13.0 toolkit | W0 step 4 | T-DEV, T-CANDLE, embed end to end |
  | RTX 3090, 13.1–13.4 builds | R615 driver (never — Q4) | UNMEASURED |
  | Orin, JetPack 6, 12.6 | present | everything incl. T-ORIN |
  | Orin, JetPack 7.2.x, 13.2/13.4 | reflash | UNMEASURED (Q5) |

  Every run pins `CUDA_VISIBLE_DEVICES` to a 3090 unless T-MULTI, and logs
  the driver/`/usr/local/cuda` guard.
- **T-CANDLE** Candle fork CUDA tests and the four FathomDB private-pool GPU
  tests against the fork revs.
- **T-PRODUCT** full `./scripts/agent-verify.sh`; Slice 30 product tests;
  single-cudarc guard red/green; `git grep third_party/cudarc` check.
- **T-ORIN** Slice 30 gate rows; `doctor cuda-allocator`; one
  Node/V8-fragmented end-to-end run (private pool selected, fallback works).
- **T-PERF** AC31-13 measurements; the proposal benchmark (default, private,
  synchronous) on RTX 3090 and Orin.
- **T-REGISTRY** registry-consumer check after publication (AC31-08).
- Exhaustion tests request at most cap + one granule, far below physical
  free memory; on JetPack 7 (future) the known reboot issue applies.

## 8. Order of the whole slice

R0, R0-rev4, R0-rev5 (done) → W0 → W1 → R-REQ → W2 → R-DES → W3 → R-TEST →
W4 → R-CODE-1 → W5 → R-CODE-2 → W6 (R-PIN per pin) → merge-forward →
qualification → R-CODE-3 + R-VERIFY → merge → W7 → R-PROP →
W8 (R-CODE-2b, R-PUB per crate) → R-CODE-4 → merge.
W7 may start once R-CODE-1 approves. Owner-gated steps (W0 step 4 download,
R-PIN, R-PUB, W8 publication) block only what depends on them.

## 9. Work packages (each ends at its review; findings fixed before moving on)

### W0. Prerequisites (main thread)

1. Review and commit this plan, `GOAL.md` and the
   [merge-forward plan](merge-forward-plan.md) on `release/0.8.28` before
   execution. Check local and remote branch heads. Use the existing clean
   `/home/coreyt/projects/fathomdb-worktrees/release-0.8.28` worktree for
   the merge; keep the branch and worktree until merge close. Record the
   exact plan and source SHAs in `status.md` once step 3 creates it (I-2).
2. **Merge forward, only after a fresh owner approval:** follow the
   [merge-forward plan](merge-forward-plan.md). It owns the source freeze,
   known conflict and semantic-overlap resolutions, merged document review,
   focused verification and stop boundary. The 0.8.27 checkout stays
   read-only. Resolve todo `TC-d71a9e36…` in the same merge: delete the
   superseded `features/cudarc-fork/plan.md` and keep one D28-13 row. Record
   both parent SHAs. Repeat before R-CODE-3/R-VERIFY if 0.8.27 advances;
   then, with no implementer running, merge updated `release/0.8.28` into
   the slice branch and rerun affected checks.
3. `scripts/preflight.sh`; create worktree
   `../fathomdb-worktrees/slice-0.8.28-31` on `slice/0.8.28-31-cudarc-fork`
   from `origin/release/0.8.28`; create fork clones
   `../fathomdb-forks/cudarc` and `../fathomdb-forks/candle-fathomdb`;
   start `status.md` on the slice branch.
4. Toolchains, searched outside PATH. Record the guard (driver 580.178.04,
   `readlink -f /usr/local/cuda` → 12.6). **Owner approval:** download the
   CUDA 13.0 runfile; install toolkit-only, user-local, no sudo (§ 2.2).
   Never install or update a driver on `windchill3`; never run the runfile
   with sudo or without `--toolkit`; never install apt `cuda`/driver
   packages. Re-check the guard.
5. Orin check over ssh (`10.83.10.13`): disk, GPU lock, toolchain.
6. Python gate environment: create a `.venv` owned by the slice worktree
   with `scripts/bootstrap.sh` run inside it (AGENTS.md § 3: the full gate
   preflights a checkout-owned `.venv`), never touching or borrowing the
   primary checkout's `.venv` (`agent-worktree-stale-base-trap.md`). The
   full gate must be green there; the Python parity oracle of
   `0.8.27-slice132-rust-sdk-lessons.md` only supplements it.

### W1. Requirements and acceptance criteria → R-REQ

`requirements.md` from § 5, traced to D28-13, Slice 30's R30/AC30 and
REQ-068. Research subagent supplies missing citations.

### W2. Design → R-DES

`design.md` answering D-1…D-15 with Driver API and release-note citations;
ADR amendment draft; records § 15 Q6's finding on 12.x versus 13.x.

### W3. Test plan → R-TEST

`test-plan.md`: each test, host, command, expected result, AC31, fault seam.

### W4. cudarc fork (`coreyt/cudarc`), TDD → R-CODE-1

1. Main thread: confirm fork `main` equals upstream `v0.19.10` (`ec1cac71`;
   do not rewrite `main`); cut `fathomdb/0.19.10` from `ec1cac71` and push
   the branch. No tag is pushed to the fork.
2. Implementer: port the existing tests first — they fail to compile on bare
   `v0.19.10` (RED) — then apply items 1–4 and 5–6 as small commits (GREEN);
   record tree A for AC31-01.
3. RED then GREEN fixes: `CUmemLocation` constructor, `CUmemPoolProps` from
   `zeroed()`, `max_size` cfg rule, `set_max_size` cfg form, 64-bit
   attributes, driver-version check, raw attribute reads, no exhaustive
   matches, `upgrade_device_ptr` precondition and doc, dead-code warning.
4. New tests from the test plan (T-DEV, T-PROP, T-CONC, T-FAULT, T-CAPTURE,
   T-MULTI, T-TEARDOWN, T-PROC), RED first.
5. Packaging commit (`cudarc-fathomdb`, `[lib] name = "cudarc"`,
   attribution, `FATHOMDB.md`).
6. **Minimum evidence for R-CODE-1:** AC31-01 procedure output; T-MATRIX;
   T-DEV/T-CONC/T-FAULT on RTX 3090 with 12.6 and with
   `CUDARC_CUDA_VERSION=13000` (driver only); T-DEV on the Orin. 13.0
   toolkit cells move to qualification if the download is pending.

### W5. Candle fork (`coreyt/candle-fathomdb`), TDD → R-CODE-2

Main thread cuts `fathomdb/0.8.28-cudarc-fork` from `859b8ea1`. Implementer:
depend on `cudarc-fathomdb` by git rev; any CUDA 13.0 build fix RED first;
D-8 SASS decision if taken; T-CANDLE on RTX 3090 (12.6, 13.0) and Orin.

### W6. FathomDB slice (`slice/0.8.28-31-cudarc-fork`), TDD → R-CODE-3 + R-VERIFY

1. RED: pinned-override checker and test, single-cudarc guard, CUDA release
   contract expect fork sources and no vendored tree.
2. GREEN (each pin commit gets R-PIN): pins; embedder dependency
   `package = "cudarc-fathomdb"`; delete `third_party/cudarc-0.19.7`; update
   every reference (`.gitattributes`, `.markdownlint-cli2.jsonc`, root
   `Cargo.toml`, `dev/tegra-platform-reference.md`, `scripts/agent-lint-md.sh`,
   `scripts/check-pinned-override-rot.py`, `scripts/pinned-override-rot.json`,
   `scripts/tests/test_check_pinned_override_rot.sh`,
   `scripts/check-tegra-pool-registry-build.sh` (path references only; its
   assertion changes in W8 step 3), and `dev/design/release.md` lines 131-142,
   "registry cudarc 0.19.7"); replace `scripts/tests/test_vendored_cudarc.sh` with a runner
   for the fork's tests (x86 and Orin); `release.yml` keeps
   `CUDARC_CUDA_VERSION=12060`; ADR amendment; changelog.
3. Full `./scripts/agent-verify.sh` (AC-036 needs ptrace; rerun unconfined if
   denied; W0 step 6 for Python receipts).
4. Merge-forward again, including into the slice branch (W0 step 2);
   qualification (T-ORIN, T-CUDA, T-PERF)
   into `qualification.md`, optionally run by a Claude agent on the Orin.
5. R-CODE-3 and R-VERIFY, then merge (§ 13 method).

### W7. Upstream proposal package → R-PROP

Main thread cuts `proposal/mem-pool` from upstream `main`. Implementer: item 5
only (no marker, no fallback), T-MATRIX and clippy clean, docs, example and
benchmark (T-PERF), tests RED first. Main thread writes
`dev/plans/0.8.28/features/slice-31/upstream-cudarc-package.md` on
`slice/0.8.28-31-post` (issue draft, #594 coexistence) and adds a superseded
pointer to the Slice 30 draft. Not posted.

### W8. Publication (owner) → R-CODE-2b, R-PUB, R-CODE-4

1. Implementer (TDD where code changes): on the Candle fork branch, switch
   to the crates.io `cudarc-fathomdb` version and bump to 0.10.4 → R-CODE-2b.
2. Owner publishes `cudarc-fathomdb` 0.19.10 after the dry-run (R-PUB), then
   the Candle crates 0.10.4 from the R-CODE-2b rev (R-PUB).
3. Implementer on `slice/0.8.28-31-post`: RED then GREEN
   `scripts/check-tegra-pool-registry-build.sh` — today it asserts that
   `tegra-pool` fails to resolve against registry cudarc 0.19.7; it must now
   require that `tegra-pool` builds from the registry with no patch table —
   and update `dev/design/release.md` § "Registry-cudarc check"; drop those
   patch entries; registry pins (R-PIN); T-REGISTRY; R-CODE-4; merge.

## 10. TDD rule (all code packages W4–W8, including W7's tests and example)

Failing test first (committed, or the red run recorded in `status.md`), then
the code, then refactor. Test files are read-only during fix-to-spec. Ported
tests count as RED only if they fail on the base (W4 step 2). Two failures of
the same kind stop the step for a re-think (AGENTS.md § 5). Reviewers re-run
each RED sha and confirm it fails.

## 11. Roles and subagents

- **Main thread** (session running [GOAL.md](GOAL.md)): writes requirements,
  design and test plan; creates every tag, branch, worktree and clone;
  briefs implementers; runs reviews; verifies results from git and logs;
  merges. Not the steward or orchestrator; neither is spawned.
- **Implementers** (`implementer` type, `.claude/agents/implementer.md`): one
  per checkout, in a pre-created worktree or clone, TDD, never pushing
  upstream. Briefs give the absolute path, the branch, the output path for
  the closure `output.json`, and override the definition's codex mention:
  the reviewer is an Opus subagent run by the main thread.
- **One writer per branch:** the main thread commits documents on the slice
  branch only when no implementer is running on it; runs are serialized.
- **Research subagents** (general-purpose) read NVIDIA, cudarc and Candle
  sources and return cited findings.
- **Review spawn:** Agent call with `model: "opus"`, effort high,
  `subagent_type` general-purpose (never steward or orchestrator), read-only
  instruction, the artifact plus REQ31/AC31, and a demand for findings with
  file:line and a failure scenario. Model recorded per review in `review.md`.
- **Orin agent:** a Claude agent may run on `10.83.10.13` for long
  qualification runs; results come back by `scp`; the main thread verifies.

## 12. Reviews (all Opus subagents unless marked owner)

| # | When | What | Blocks |
| --- | --- | --- | --- |
| R0 (owner) | done 2026-10-11 | plan revision 3 | — |
| R0-rev4 | done 2026-10-11 | plan revision 4 + GOAL (APPROVE-WITH-FIXES, 27 findings → revision 5) | — |
| R0-rev5 | done 2026-10-11 | plan revision 5 + GOAL (APPROVE-WITH-FIXES: H1, H2, M1–M5, L1–L4, partial 1/8/26 → revision 6) | — |
| R-REQ | end of W1 | `requirements.md`: coverage of § 0, testability of each AC31 | W2 |
| R-DES | end of W2 | `design.md` + ADR amendment draft, citations checked | W3 |
| R-TEST | end of W3 | `test-plan.md`: every AC31 covered, seams, hosts | W4 |
| R-CODE-1 | end of W4, before any pin | cudarc fork branch, RED reruns, W4 step 6 evidence | W5, W6 green, W7 |
| R-CODE-2 | end of W5, before the Candle pin | Candle fork diff from `859b8ea1` | W6 green |
| R-PIN (owner) | each pinned fork rev or registry version | the change (`scripts/pinned-override-rot.json` rule) | that pin |
| R-CODE-3 | end of W6, after merge-forward and qualification | the slice branch | merge |
| R-VERIFY | with R-CODE-3 | pre-merge AC set, `qualification.md`, `status.md` | merge |
| R-PROP | end of W7 | proposal branch and package | owner sign-off (later) |
| R-CODE-2b | W8 step 1 | Candle publication commit | R-PUB for Candle |
| R-PUB (owner) | before each `cargo publish` | dry-run output, packaged file list | each publish |
| R-CODE-4 | W8 step 3 | patch-drop follow-up, T-REGISTRY, AC31-08, AC31-12 final | merge |

Findings go into `review.md`; each is fixed RED then GREEN or given a
recorded disposition before the blocked step starts.

## 13. Branch management

| Repo | Branch | Cut from (by the main thread) | Writer | Lands by |
| --- | --- | --- | --- | --- |
| `coreyt/cudarc` | `fathomdb/0.19.10` | tag `v0.19.10` | implementer, `../fathomdb-forks/cudarc` | stays; pinned, then published |
| `coreyt/cudarc` | `proposal/mem-pool` | upstream `main` | implementer, separate clone or serialized | stays; posted later only with owner sign-off |
| `coreyt/candle-fathomdb` | `fathomdb/0.8.28-cudarc-fork` | `859b8ea1` | implementer, `../fathomdb-forks/candle-fathomdb` | stays; publishes 0.10.4 |
| `fathomadb/fathomdb` | `release/0.8.28` | existing branch | main thread, existing `../fathomdb-worktrees/release-0.8.28`; one writer at a time | holds W0 merge-forward and later slice merges |
| `fathomadb/fathomdb` | `slice/0.8.28-31-cudarc-fork` | `origin/release/0.8.28` | implementer and main thread, serialized | R-CODE-3 + R-VERIFY |
| `fathomadb/fathomdb` | `slice/0.8.28-31-post` | `origin/release/0.8.28` right after the slice merges | main thread (W7 docs) and implementer (W8 step 3), serialized; worktree `../fathomdb-worktrees/slice-0.8.28-31-post` | R-CODE-4, merge |

- **Merge method:** the main thread uses the existing
  `../fathomdb-worktrees/release-0.8.28` checkout for W0 and later slice
  merges (`git merge --no-ff <slice branch>`), only when no other writer is
  active there. It runs the checks the merge touches and pushes the explicit
  branch ref. Do not create a second checkout of that checked-out branch.
- Fork branches are never rewritten after a pin. Worktrees and clones are
  removed after their branch lands.

## 14. Risks

- **Two cudarc copies** (guard, AC31-07). **`ug-cuda` → cudarc 0.17.8**
  (max CUDA 13.0) is lock-only and must stay disabled.
- **Upstream PR #594** overlaps items 1–3 and 5 (D-9, W7).
- **`windchill3` safety:** a driver change would disable the host (Q4). Only
  toolkit-only, user-local installs; the guard stops the slice on any change.
- **CUDA 13.1+ runtime** cannot be measured on driver 580 (PTX rule);
  covered by the compile matrix and D-8; recorded UNMEASURED.
- **Tegra on CUDA 13** UNMEASURED (Q5); follow-up proposed (§ 16).
- **Cap on a driver older than the bindings** (D-4).
- **Licensing:** cudarc MIT/Apache-2.0; attribution kept.

## 15. Questions and rulings

- **Q1.** Version — ruled: base v0.19.10, first publication `0.19.10`.
- **Q2.** `candle-kernels-fathomdb` — ruled: no, not now (static CUDART is
  required for FathomDB-built artifacts per
  `ADR-0.8.25-driverless-cuda-runtime-linkage.md`, decision index row 52,
  REQ-068; those are built in the workspace with the git patch).
- **Q3.** Slice number — ruled: 31.
- **Q4.** R615 driver on `windchill3` — ruled: **never**; it would disable
  the host. CUDA 13.1+ runtime cells are UNMEASURED.
- **Q5.** Tegra on CUDA 13 — ruled: UNMEASURED in Slice 31; propose the work
  before the 0.8.28 release (§ 16).
- **Q6.** CUDA 13 on `windchill3` — ruled: build against CUDA 13.0 only,
  without changing the primary 12.6 (§ 2.2); 0.8.28 artifacts stay on 12.6.
  **Why 12.x (history search, 2026-10-11):** no ADR, design, ledger, commit
  or memory records a decision of 12.x over 13.x. CUDA 12.6 was the toolkit
  already installed on `windchill3` in June 2026
  (`dev/design/0.8.1-embedder-gpu-and-portability.md:24`; 0.8.7 status:
  "Driver 580.x (CUDA 13 capable, runs 12.6 builds)"), and the 0.8.23
  release contract froze it as one literal shared by the x86 and Jetson
  builds (`scripts/release/cuda-artifact-contract.sh:11-17`). The binding
  dependency is the **Jetson Orin**: JetPack 6 cannot run CUDA 13, and
  NVIDIA excludes Orin from CUDA 13's Arm toolkit unification
  (`dev/tegra-platform-reference.md:80-83`), so moving the x86 toolkit to
  13.x would split the shared pin. Incidental: the 0.8.18 ONNX-CUDA eval
  used `onnxruntime-gpu` 1.26.0 with CUDA 12 / cuDNN 9 wheels (not a shipped
  path).

## 16. Ledger and follow-ups

- Todo `TC-d71a9e36-4509-470e-a677-525ea91193e4`: remove the superseded
  `features/cudarc-fork/plan.md` copy after the merge-forward (W0 step 2).
- Todo `TC-b267d940-1add-4d08-a596-a7669ae4a500`: the Q5 Tegra/CUDA 13
  follow-up below.
- At W0: record the 2026-10-11 directions and rulings Q4–Q6 with
  `ledgerwrite`; set `FATHOMDB-PATCH.md` item 5 to "on fork, not posted"
  until W6 deletes the file.
- **Proposed follow-up (Q5):** before the 0.8.28 release, qualify Tegra on
  CUDA 13 — Orin reflashed to JetPack 7.2.x (CUDA 13.2, then 13.4 via
  `cuda-compat-orin-13-4`), re-measure whether the default-pool failure
  persists, run the T-DEV/T-ORIN cells, then restore JetPack 6 for the
  shipping target. Needs the owner's reflash decision. Todo
  `TC-b267d940-1add-4d08-a596-a7669ae4a500` (opened 2026-10-11).
