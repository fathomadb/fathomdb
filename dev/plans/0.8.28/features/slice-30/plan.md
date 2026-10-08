---
title: FathomDB 0.8.28 Slice 30 — Tegra private CUDA memory pool (D28-08) plan
status: APPROVED FOR EXECUTION (revision 3.2, 2026-10-08; aligned with design revision 3)
target_release: 0.8.28
observed_on: 2026-10-08
---

# 0.8.28 Slice 30: Tegra private CUDA memory pool

This plan turns the 0.8.28 Tegra CUDA memory-pool study into product work.
It is the working plan for D28-08 in `dev/plans/0.8.28-draft-scope.md`
until the 0.8.28 plan, release-state file and board exist. Those must copy
§ 9 (host clean-up) into the 0.8.28 plan when they are created. It does not
replace the owner's rulings; it cites them.

Files of this slice, all in `dev/plans/0.8.28/features/slice-30/`:

- this plan;
- `design.md`: the design, completed in S30-T1;
- `design-review.md`: the design and code reviews;
- `status.md`: the slice status record.

Revision 3.2 (S30-T9) only corrects statements the implementation made
false: the decision is reported by `doctor cuda-allocator`, not
`doctor gpu` (C-10, SD-8, G1, S30-T5, § 8), and the registry check uses a
scratch downstream crate (AC30-07). Design revision 3 § 8 has the detail.

Work runs in the worktree `.claude/worktrees/slice-30-tegra-pool` on
branch `slice/0.8.28-30-tegra-pool`, cut from `release/0.8.28` at
`cb9594f44`. It merges back to `release/0.8.28`, and the worktree and
branch are then removed (S30-T12).

## 0. What changed since revision 2 (2026-10-07), and what it changes

Each change was checked against the branch at `cb9594f44`.

| # | Change | Effect on the plan |
| --- | --- | --- |
| C-1 | `release/0.8.27` has not moved since `b65283317`. There is nothing to merge forward. | S30-T0 records it. Merge forward again before review. |
| C-2 | The napi (0.8.27 Slice 110), TypeScript (120), Python (130) and Rust SDK (132) refactors all landed after the study's merge-base `9a6f01b82`. The files the study changed are untouched by them, though. TS and Python moved engine wrappers into new files the study never edited. | The study's binding and SDK hunks apply in spirit almost unchanged. The port is smaller than revision 2 assumed. |
| C-3 | The engine files of the close fix, already on the branch, are byte-identical to the study tip: `embed_dispatch/core.rs`, `projection_runtime.rs`, `projection_worker.rs`, `runtime_lifecycle.rs`, `vector_storage.rs`, `slice90_close_tests.rs`, `tests/projection_runtime.rs`. | Nothing to port there. |
| C-4 | **`fathomdb-sdk`** (Slice 132) is a fourth error surface the study never touched. Its `engine_kind` has a `_ => ErrorKind::Engine` catch-all (`fathomdb-sdk/src/error.rs:89`) that would silently swallow a new kind. Its `RerankerDevicePolicy(_)` arm (:58) would hide a pool variant. | New requirement R30-04c. Add the kinds to the SDK's `error_kinds!` table and to the `Embedder` family, with tests. The SDK gains `tegra-pool` forwarding. |
| C-5 | Error-class counts are hand-maintained. They appear in `fathomdb-sdk/tests/surface.rs:216` (42), `dev/design/errors.md` § "Binding-facing class matrix" (41 rows), `dev/interfaces/python.md` ("41 … 1:1 with TypeScript"), `rust-sdk.md` ("42 variants"), and the TS and Python hierarchy tests. | Every count moves together (AC30-04). |
| C-6 | Module-level `rerank()` flattens reranker device-policy errors to strings: `fathomdb-engine/src/rerank.rs:130,153`. napi and Python surface them as `WriteValidation` (napi `embedding.rs:111`, py `embedding.rs:91`), and so does the SDK (`standalone.rs:52`). The study did not fix this. | Ruling 33 requires a typed kind on this path, so R30-04b adds it. Only pool exhaustion and the existing device-policy kinds change; other validation stays as it is. |
| C-7 | Module-level CLS `embed_batch_cls` maps forward errors to the `Embedder` class with a Debug string (napi `embedding.rs:138`, py `:199`, SDK `standalone.rs:105`). | Pool exhaustion there must surface as `cuda_pool_exhausted` (R30-04b). |
| C-8 | Early `cuInit` already has a process-wide record, `fathomdb-embedder/src/cuda_driver_init.rs` (`RECORD`, slots `ModuleLoad` / `EmbedderProbe` / `RerankerProbe`). napi calls it at registration, with the opt-out `FATHOMDB_CUDA_EARLY_INIT=off` (`fathomdb-napi/src/cuda_early_init.rs:207`). Python has no hook. | Ruling 37 builds on this record; there is no new state store. Python gains the same hook and opt-out. |
| C-9 | `fathomdb doctor gpu` already creates a CUDA context, through `probe_cuda` (`candle_bge.rs:260`). It reports no allocator facts. Its v1 record is frozen (`cli.md:80`), and new facts go into sibling records (precedent: `reranker-gpu.v1`, `platform.v1`). | Revision 2's "doctor must not create a context" is withdrawn. A new verb, `doctor cuda-allocator`, runs early `cuInit`, decides as an SDK process would, and reports the decision (R30-06). |
| C-10 | The workspace has no `tracing` dependency. | Revision 2's "decision and exhaustion events as `tracing` events" is replaced. The decision goes into the existing `OpenReport.embedder_device_resolution` and into `doctor cuda-allocator` (C-9). Exhaustion is the typed error. No new dependency. |
| C-11 | The Tegra Python wheel is built without `rerank-cuda` (`scripts/release/cuda-artifact-contract.sh:78`, shared with x86). No Tegra Node addon is built in CI or release; it is manual, and 0.8.27 Slice 117 is still planned. | `tegra-pool` goes into a Tegra-only Python feature set (R30-07). For the Node addon, Slice 30 adds the feature to the manual build recipe and records it for Slice 117. A contract check forbids `tegra-pool` in x86 sets. |
| C-12 | The study's `Cargo.toml` comments claim `cuda-artifact-contract.sh` forbids the experiment feature. No such guard exists. | The guard is real work (AC30-07). |
| C-13 | The study's local Candle commit `bd68a7ca` ("`CudaDevice::from_context` and `Device::new_cuda_from_context`", on `1aefdd00`) exists only in the earlier session's `/tmp` scratchpad. | S30-T2 moves it to a durable clone before the scratchpad is cleaned (§ 9). |

**Allocated draft items.** D28-08 is the only item allocated to Slice 30.
D28-01 to D28-07 stay candidates.

**Evaluation.** Revision 2's scope stands, with these adjustments:

- **Added:** R30-04b (module-level paths) and R30-04c (`fathomdb-sdk`),
  because of C-4 to C-7.
- **Removed or replaced:** `tracing` events (C-10); "doctor creates no
  context" (C-9).
- **Simplified:**
  - settings come from environment variables only; no module-level
    configure call is added;
  - S in qualification is this build with pool mode `off` (and, for the
    Python import gate, with early `cuInit` opted out). By design that is
    the 0.8.27 path, so no separate 0.8.27 build is needed.

Nothing else is added.

## 1. Sources

All study sources are as of `b045489d2` on `llm/0.8.28-tegra-pool-study`.

| Source | Role |
| --- | --- |
| `dev/plans/0.8.28/prework/tegra-cuda-memory-pool-study.md` | Study plan, owner rulings 1–38, design notes, gate tables |
| `dev/plans/0.8.28/prework/tegra-cuda-memory-pool-experiment-protocol.md` | Executable method; governs where it is more specific |
| `dev/plans/0.8.28/prework/cudarc-upstream-patch-notes.md` | Upstream cudarc shape (Phase 5 input) |
| `dev/plans/runs/0.8.28-pool-study/results.md` | Results of record, Phases 0–4 and the revision-6 spot check |
| `dev/plans/runs/0.8.28-pool-study/` (other directories) | Evidence and the harness that qualification adapts (§ 8) |

Study code (reference only; Slice 30 rewrites it test-first):

| Commit / file | What |
| --- | --- |
| `3ce5c5c46` | cudarc: explicit pool primitive and three-state allocator decision |
| `dbd08e313` | P-first-use private pool, fail-closed policy |
| `74a6b4d62` | `cuda_pool_exhausted` kind, Python hook, constants (and the dropped trim arm) |
| `7b9b18146` | runtime gating and sizing; Python and reranker exhaustion |
| `fathomdb-embedder/src/cuda_pool_policy.rs` at `b045489d2` | The policy module the product version is cut down from |
| `patches/candle-from-context.patch` and local commit `bd68a7ca` | `CudaDevice::from_context` for Candle |

## 2. Decisions

Each one is restated in `design.md` and the ADR.

- **SD-1. Candle `from_context` lands on the Candle fork, not as a vendored
  copy.** The complete work is S30-T2:
  - **Fork.** In `coreyt/candle-fathomdb`, cut a branch from `1aefdd008`
    and apply `bd68a7ca` with a unit test. Bump `candle-core-fathomdb`,
    `candle-nn-fathomdb` and `candle-transformers-fathomdb` to 0.10.3
    together. `candle-kernels` keeps its version.
  - **FathomDB pin set.** Move every reference to the old revision
    together:
    - the four rev pins and their comment in the root `Cargo.toml`;
    - the three `=0.10.2` pins in `fathomdb-embedder/Cargo.toml`;
    - `Cargo.lock`;
    - `CANDLE_GIT_REV` in `scripts/check-cuda-release-contract.py`;
    - `CANDLE_REV` in `scripts/check-pinned-override-rot.py`, and
      `scripts/pinned-override-rot.json`;
    - the fixture revs in `scripts/tests/test_check_pinned_override_rot.sh`
      and `scripts/tests/test_cuda_release_contract.sh`. This is a
      mechanical rev bump, the TDD exception;
    - the version and line references in
      `dev/design/delivery-requirements-map-20260821.md`.
  - **Re-verified on the new revision:** the CUDA release contract and the
    pinned-override gate.
  - **HITL.** Pushing the fork branch needs the owner's go-ahead in this
    slice. Publishing the three 0.10.3 crates to crates.io is a 0.8.28
    release task. It needs its own go-ahead and is not part of this slice.
- **SD-2. The 8 GB Orin is off in every mode.** `on` lifts only the
  Tegra-identity and unmeasured-class gates. `TooSmall`, `Discrete`,
  `NoPools` and `not_built` hold in every mode.
- **SD-3. Where falling back ends and refusing begins.**
  - Every failure at the process's first GPU-use decision falls back to the
    0.8.27 path, with a recorded reason (ruling 37).
  - After a private-pool context exists, a later failure to build one is a
    typed error, never a silent switch of allocator.
- **SD-4. The Python import gate counts early `cuInit`:** S + measured
  `cuInit` cost + 5 ms. Node keeps S + 5 ms.
- **SD-5. The non-default `tegra-pool` Cargo feature.**
  - The napi and Python Tegra builds enable it.
  - Without it, the code compiles against stock cudarc 0.19.7, the reason is
    `not_built`, and behaviour is 0.8.27's.
  - It forwards through `fathomdb-embedder`, `fathomdb-engine`,
    `fathomdb-napi`, `fathomdb-py`, `fathomdb-sdk`, `fathomdb` and
    `fathomdb-cli`.
- **SD-6. Performance is compared with the study's P build, not with a
  fresh default-pool reference.**
  - Node: product P against the retained study P artifact (rebuilt from
    `b045489d2` if its checksum does not match `artifacts.sha256`) and
    against S.
  - Python: product P against Python S, which is the default pool on this
    host.
- **SD-7 (new). Settings come from environment variables only:**
  `FATHOMDB_POOL_MODE` (`auto` / `on` / `off`), `FATHOMDB_POOL_MAXSIZE`,
  and `FATHOMDB_POOL_RELEASE_THRESHOLD`. They are read once, before the
  first device decision.
  - A malformed value is reported as the reason `invalid_setting` and the
    pool stays off. It never aborts the process; the study's abort was for
    measurement hygiene.
- **SD-8 (new). Where the decision is reported:**
  - a `cuda_allocator` field on the CUDA facts of `DeviceResolution`
    (`OpenReport.embedder_device_resolution`) in all four SDK surfaces;
  - `fathomdb doctor cuda-allocator` (C-9, C-10).

## 3. Requirements and acceptance criteria

The need (D28-08; rulings 25–38): on a shared Jetson host, every allocation
that fits must succeed and its memory must be given back, without paying
the synchronous path's 2–5× latency. Allocation correctness and release
rank above latency (ruling 25).

| Req | Requirement | Acceptance (falsifiable) |
| --- | --- | --- |
| R30-01 | In a `tegra-pool` build on an aarch64 Linux integrated GPU of a measured class, the process creates a private pool at first GPU use, with release threshold 0, sized per § 4.3, and builds every FathomDB CUDA context on it. The device's current pool is untouched. | **AC30-01:** pure tests cover every § 4.3 row, every gate reason, what `on` lifts and `off` winning. A vendored cudarc test shows a private-pool context allocates from its pool and leaves the current pool alone. G1 QUAL. |
| R30-02 | The pool is created only if the module-load early `cuInit` ran. If `cuInit` was opted out, failed or never ran at load, a gate is off, a setting is invalid, or pool creation fails, the process uses the 0.8.27 path with a recorded reason. It never refuses. Python gains early `cuInit` at import, with the opt-out `FATHOMDB_CUDA_EARLY_INIT=off`. | **AC30-02:** a pure test maps each init state and each failure to its path and reason. A Python test shows the hook runs at import and honours the opt-out. G2 QUAL. |
| R30-03 | Once a private-pool context exists, a failure to build another is a typed error: `cuda_pool_exhausted`, `cuda_context_lost`, or `cuda_private_build_refused` (design § 1.3). Another ordinal takes the 0.8.27 path. | **AC30-03:** a pure test of SD-3, plus the other-ordinal rule. |
| R30-04 | `cuda_pool_exhausted` is raised when a private-pool process gets `CUDA_ERROR_OUT_OF_MEMORY`. It carries the ordinal, the pool's `maxSize` and the message, and is present on every path in every SDK, including load paths and the reranker under `auto`: (a) engine embed, batch embed and engine rerank; (b) module-level `embed_batch_cls` / `embedBatchCls` and `rerank()`; (c) `fathomdb-sdk`. A following request runs on CUDA. | **AC30-04:** for each path in (a)–(c), a Rust, pytest and vitest test drives exhaustion through a deterministic seam and asserts the kind, class and payload. The SDK `ErrorKind` table, the Python and TS classes and every count (C-5) move together. The SDK's catch-all cannot swallow the kind, and a test proves it. G4 QUAL. |
| R30-05 | C7 instrumentation (ruling 36). Record the primary-context id (`cuCtxGetId`) when the private context is created. On a CUDA error, compare it. On a mismatch, raise typed `cuda_context_lost` (both ids, the driver error, the operation) and write one diagnostic snapshot per process. Survival is not required. | **AC30-05:** tests through a fault seam show the typed error, its payload, and exactly one snapshot under two concurrent detections. A GPU characterization test (heavy tier) runs the study's reset probe and records the current outcome. |
| R30-06 | The decision is reported: `cuda_allocator` in `DeviceResolution`'s CUDA facts, and a new sibling verb `fathomdb doctor cuda-allocator` (`fathomdb.doctor.cuda-allocator.v1`, design § 2.6). `doctor gpu` v1 is unchanged. The new verb runs early `cuInit` after its `cpu` and SBSA early returns. | **AC30-06:** CLI JSON/text tests pin the record's keys and order and the early returns. Tests in each SDK show the report field on a CUDA resolution, `not_built` without the feature, and its absence on CPU. |
| R30-07 | Isolation: `tegra-pool` is non-default. Built without it, the allocator decision equals 0.8.27's. The Python early-`cuInit` hook is compiled only with it. The x86 and CPU release feature sets cannot carry it, and the Tegra Python wheel does. Built against registry cudarc, it fails with a clear message. | **AC30-07:** `cargo check` per crate with and without the feature. A `not_built` test. A contract test rejects `tegra-pool` in x86 or CPU sets. A scratch downstream crate, patched like the workspace minus cudarc so that cudarc comes from the registry, compiles without the feature and fails with the marker message with it (`scripts/check-tegra-pool-registry-build.sh`, a manual release check). The design § 4 tables are complete, each row cited and tested. |
| R30-08 | Candle `from_context` (SD-1). | **AC30-08:** the Candle unit test; the pin set moved together; the CUDA release contract and the pinned-override gate pass. |
| R30-09 | The cudarc pool primitive (`CudaMemPool`, `CudaContext::new_with_mem_pool`, `AllocMode::Private`, `CudaContext::mem_pool`) is written upstream-first and vendored as its backport. Each `FATHOMDB-PATCH.md` item has an upstream status and a removal path. The pinned-override gate checks the vendored tree against 0.19.7 plus exactly the listed items. | **AC30-09:** vendored unit tests (`test_vendored_cudarc.sh`): the three allocator states, zero-length (C5), drop order. The pinned-override gate passes with the new item set. |
| R30-10 | Study scaffolding removed (§ 5). | **AC30-10:** a test asserts that the removed environment variables and `fdb-pool-exp` are absent from the source tree outside `dev/`. |
| R30-11 | Contract documents: the ADR; `dev/interfaces/{rust,python,typescript,cli,rust-sdk}.md`; `dev/design/errors.md`; the changelog; and the module-level-model note (ruling 32). | **AC30-11:** the ADR states every `design.md` § 3 item. The ADR is indexed. The Markdown checks pass. |
| R30-12 | Qualification on the AGX Orin 64 GB (§ 4). | **AC30-12:** G1–G10 pass, recorded in `dev/plans/runs/0.8.28-slice-30/qualification.md`. |
| R30-13 | Phase 5 folded in (§ 7). | **AC30-13:** the closing analysis section of `results.md`; the upstream package prepared, not posted; `FATHOMDB-PATCH.md` statuses set. |
| R30-14 | Host clean-up, HITL (§ 9). | **AC30-14:** each deletion approved and recorded in `status.md`. |

## 4. Gates

### 4.1 The questions

| Q | Question | Gates |
| --- | --- | --- |
| Q1 | Does each process take the private pool exactly where the rules say, and the 0.8.27 path everywhere else, with the reason reported? | G1, G2 |
| Q2 | Is the memory given back, and does it stay bounded? | G3 |
| Q3 | Is pool exhaustion typed, recoverable and kept on CUDA, on every path in every SDK? | G4 |
| Q4 | Did productisation change behaviour: outputs, heap robustness, concurrency, endurance? | G5–G8 |
| Q5 | Did productisation cost speed? | G9 |
| Q6 | Do coexistence, multi-device isolation and the C7 instrumentation behave as designed? | G10 (tests only) |

### 4.2 Gate rows

**Disposition:**

- **TEST**: an automated test, written red first.
- **QUAL**: run once on the AGX Orin 64 GB under the GPU lock
  (`flock <scratch>/gpu.lock`), with a packed Node addon and Python wheel
  built with `tegra-pool`.

S is the same build with `FATHOMDB_POOL_MODE=off`. Every QUAL series
passes only with zero allocator errors, crashes and OOM kills, and with
`reserved_high` ≤ `maxSize` in every process (CB1, read from the pool's own
counter, ruling 34). The study's N carries the statistical power; these
samples confirm the product build behaves like it.

| Gate | Study rows | Disposition | Method and sample | Pass | Study evidence |
| --- | --- | --- | --- | --- | --- |
| G1 Decision | C1, C6 (pure), sizing | TEST + QUAL | AC30-01 tests. QUAL: 20 Node + 20 Python processes in `auto`. | Every FathomDB context is private-pool; every free uses the allocating API; the report and `doctor cuda-allocator` show `private` and the size. | 751/751 (§ 12.8) |
| G2 Fallback | ruling 37, R4 | TEST + QUAL | AC30-02 tests. QUAL: 5 Node + 5 Python processes per state (opted out, `off`, invalid setting), plus 5 late-import Node processes at a 4M-object heap. | Never refuses; 0.8.27 path; the reason is reported. | R4 (§ 12.6) |
| G3 Release and lifetime | C2, CB1, CB2 | QUAL | 10 Node processes × 100 open/close cycles, no GC; 10 Python × 50; then close and 10 s idle. | All cycles on the private path. Median VmRSS growth ≤ 0.36 MiB per cycle, max ≤ 0.44. `reserved_cur` = 0 after the last close when no module-level model is loaded; otherwise spare < 2 chunks. | 20/20, 0.36 MiB (§ 13.2) |
| G4 Exhaustion | C3, CB3, CB4 | TEST + QUAL | AC30-04 tests. QUAL: oversized batch (128 long passages) at 3 GiB, 5 Node + 5 Python processes. | Typed kind; the next embed succeeds on `cuda` with the pre-error hash. | § 11.6, § 12.1 |
| G5 Equivalence | C8 | QUAL | The G1 processes against S. | Identical embedding hash and rerank scores; same decision and error kinds per binding. | § 12.7.3 |
| G6 Heap robustness | R2, R3, R5 | QUAL | Node 25, import first; heap grown to 4M objects before first use, and to the R5 boundary cell; 20 processes each; more growth, then a second embed and rerank. | Private path in every process; no path change. A failure stops the run for a ruling. | 630 R5 runs (§ 10.6, § 12.6) |
| G7 Concurrency | R7 | QUAL | 2, 4 and 8 processes, 5 trials each; abort below 8 GiB `MemAvailable`. | No failure or OOM kill. | 280/280 (§ 12.6) |
| G8 Soak | R6 | QUAL | One Node and one Python process, concurrently, 20 minutes, no-swap condition. | No allocator error. Reserved memory and RSS within 10 % of the first-5-minute high. Drift < 10 %. | n = 1 (§ 12.6) |
| G9 Performance | P1–P4, P7 | QUAL | Node 25: product P, study P and S, 20 processes each, interleaved randomised blocks. Python: product P and S, 20 each. Per process: import time, 5 warm-up + 50 timed steady embeds, rerank, batches 1/8/32/128. Bootstrap 95 % CI. | Node: product/study-P ≤ 1.05 per measure, CIs reported (CI upper > 1.10 → double N once); speed-up over S > 1.5 (CI lower bound); import ≤ S + 5 ms; RSS ≤ S + 32 MiB. Python: P / S ≤ 1.15; import per SD-4. | Phase 4 (§ 12.7.2), § 13.3 |
| G10 Coexistence, devices, C7 | C5, C6, C7, C9 | TEST | AC30-05 and AC30-09 tests; the current-pool handle is never the private pool; two-device pure tests. | As AC30-05 and AC30-09. | C9 Phase 2; § 12.3, § 13.4 |

The GPU budget is about 2.5 h in one locked window.

### 4.3 Sizing (ruling 31, SD-2)

| Device class | `auto` | `on` |
| --- | --- | --- |
| AGX Orin 64 GB | 3 GiB (1/20 clamped to [2, 3] GiB) | same |
| Orin 32 / 16 GB | off until on-device C3 + R5 | 2 GiB |
| Orin 8 GB | off (`TooSmall`) | off (`TooSmall`) |
| Thor | off (unmeasured) | 3 GiB |
| GB10-class, not Tegra | off (Tegra-identity gate) | sized by rule |
| GH200 / any discrete | off (`Discrete`) | off (`Discrete`) |
| No pool support | off (`NoPools`) | off |
| Without `tegra-pool` | off (`not_built`) | off |

### 4.4 Carried, not re-run

- **R1** (driver-only harness), **R8** (ruling 30), **C3** capacity model,
  **C4**, **R2/R3 on Node 24 and 26**, **P5** and **P6**.
- **Unmeasured:** real multi-GPU; R9 boards (8, 16 and 32 GB Orin, Thor,
  GB10, GH200), kept safe by § 4.3.

## 5. Product changes

- **Removed from the study code:**
  - the trim arm and `FATHOMDB_POOL_TRIM*`;
  - the comparison arms (`A-first-use`, `B`, `PoolAction::CreateInstalled*`,
    `CudaMemPool::install`, `AllocMode::Explicit`);
  - `FATHOMDB_POOL_VARIANT`, `FATHOMDB_POOL_STATS_EVERY_S`,
    `FATHOMDB_POOL_COEXIST_CHECK` and `FATHOMDB_POOL_MAPS_DIR`;
  - the `fdb-pool-exp` lines and the exit hook;
  - the abort on a malformed setting (SD-7);
  - `parse_max_size(None)`.
- **Kept:**
  - P-first-use as the only pool action;
  - the settings of SD-7;
  - the named constants (divisor 20, floor 2 GiB, ceiling 3 GiB, quarter
    rule 4, probe 4 bytes, `cuda_pool_exhausted`), each with a pure test;
  - the fail-closed process mode;
  - the `tegra-pool` feature (SD-5).

## 6. Process and TDD ladder

### 6.1 Roles

- **Main thread:** plans, writes the design, implements step by step in
  this worktree, and keeps one writer at a time.
- **Design review:** an independent Opus subagent at high effort reviews
  `design.md` and the ADR before implementation (S30-T1). Findings are
  resolved or recorded in `design-review.md`.
- **Code review:** an independent Opus subagent at high effort reviews the
  diff after S30-T8, before qualification. Findings go into
  `design-review.md`. A finding is fixed with a focused check, not a full
  regression, unless the fix is broad.
- **Test and verify:** an independent Sonnet subagent runs the slice's
  tests and checks each acceptance criterion against the branch (S30-T10b).
  It also runs, or audits, the QUAL series.

Note: the brief asked for "opus-6.1-high". The model selector here offers
`opus`, which is Opus 5.5, so it is used at high effort.

### 6.2 Ladder (red → green → refactor)

Each implementation step commits its failing test first, or shows the red
in the step's log, then the code that makes it pass. Test files are
read-only during fix-to-spec.

| Step | Work | Red first | Accept |
| --- | --- | --- | --- |
| S30-T0 | Preflight; merge-forward check (C-1); write `status.md` (started). | — | — |
| S30-T1 | Complete `design.md` (traces in its § 4, SD-1 to SD-9) and the ADR draft; design review; resolve. Owner HITL: Candle push, DQ-2, SD-9. | — | AC30-11 (draft) |
| S30-T2 | Candle: a durable clone; apply `bd68a7ca` plus a unit test; versions to 0.10.3. **HITL: push the fork branch.** Then move the pin set. | Candle `from_context` test | AC30-08 |
| S30-T3 | cudarc primitive, upstream-first, vendored; `FATHOMDB-PATCH.md`; pinned-override set; `test_vendored_cudarc.sh`. | Vendored tests: three states, C5, drop order, current pool untouched | AC30-09 |
| S30-T4 | Policy module (product): feature, sizing, gates, modes, settings, constants, SD-2, SD-3, SD-7; device creation sites; report field (SD-8). | Pure tests; `not_built` test | AC30-01, AC30-03, AC30-07 (part) |
| S30-T5 | Early-`cuInit` contract: gate the pool on `ModuleLoad`; Python hook and opt-out; CLI `doctor cuda-allocator` runs it after its early returns. | State-mapping tests; Python import test | AC30-02 |
| S30-T6 | Typed exhaustion: engine, napi, py, Python SDK, TS, `fathomdb-sdk`, CLI exit code; module-level paths typed (C-6, C-7); counts (C-5). | AC30-04 tests per path and surface | AC30-04 |
| S30-T7 | C7 instrumentation and `cuda_context_lost` in every surface. | AC30-05 tests | AC30-05 |
| S30-T8 | `doctor cuda-allocator`; feature wiring in Cargo, the Tegra wheel script and the manual addon recipe; contract guard (C-12); the no-`[patch]` scratch check; removal test; interface docs, ADR final, changelog. | CLI tests; contract test; removal test | AC30-06, AC30-07, AC30-10, AC30-11 |
| S30-T9 | Phase 5 (§ 7). | — | AC30-13 |
| S30-T10a | Code review (§ 6.1); resolve. | — | — |
| S30-T10b | Sonnet verification of AC30-01 to 11; packed artifacts; QUAL G1–G10 with the adapted harness; `qualification.md`. | — | AC30-12 |
| S30-T11 | Host clean-up, HITL (§ 9). | — | AC30-14 |
| S30-T12 | `status.md` final; merge to `release/0.8.28`; remove the worktree and branch. | — | — |

Checks per step: the crate tests the step touches, `clippy -D warnings`
on those crates, and the Markdown checks for doc changes. The full
`agent-verify.sh` runs once, before the merge.

## 7. Phase 5, folded in (S30-T9)

- **Analysis:** a closing section of `results.md`. It runs the decision rule
  over Phases 0–4 and the spot check, lists every UNMEASURED item, and
  records what moves to 0.8.29 (TC-281155a1).
- **cudarc upstream package:**
  - re-read the cited threads;
  - prepare the issue text and the commit series written in S30-T3;
  - set each `FATHOMDB-PATCH.md` status.
  - Nothing is posted without the owner's sign-off (ruling 38;
    TC-b8e5fe4d).
- **Candle (optional):** an upstream note for `from_context`, under the same
  rule.
- **NVIDIA report:** record the adoption outcome.

## 8. Study evidence

`dev/plans/runs/0.8.28-pool-study/` is on the branch (option B), except:

- **The harness's nine shell scripts** (25 KiB, study branch only). They
  fail shell lint. S30-T10b brings them over lint-clean, inside the new
  qualification directory `dev/plans/runs/0.8.28-slice-30/harness/`. The
  harness is adapted there to `FATHOMDB_POOL_MODE`, the report field and
  `doctor cuda-allocator`. The study copies stay as the record.
- **The vendored Candle copy** stays on the study branch only (SD-1).

## 9. Host clean-up (HITL)

The study's untracked raw data is on the Jetson AGX Orin (Ubuntu) host in
an earlier session's scratchpad,
`/tmp/claude-1000/-home-coreyt-projects-fathomdb/ea4f7b06-2eaf-4683-9916-58e4b19cfc97/scratchpad/pool-study/`.
As of 2026-10-07 it is 5.4 GiB:

- `logs/`: 292 MiB of raw per-run logs;
- `dbs/`: 4.5 GiB of databases;
- the rest is builds, staged artifacts (including the study P build that G9
  uses), the Candle clone (C-13) and virtual environments.

Keep it until S30-T10b closes. Then, as S30-T11, with the owner's explicit
permission for each deletion:

1. List and size the directory again.
2. Ask the owner whether to keep a compressed archive of `logs/`, and
   where.
3. Delete `dbs/`, the builds and the virtual environments.
4. Delete `logs/` once any archive is verified.

Nothing is deleted without a HITL go-ahead. The 0.8.28 plan, when created,
carries this as a release task.

## 10. Ledger and todos

TC-1c70e523 (study, done) · TC-a7c4a599 (early `cuInit`, decided) ·
TC-f4e12b96 (typed context loss) · TC-99845d75 (C7 instrumentation) ·
TC-b8e5fe4d (upstream and drift) · TC-281155a1 (0.8.29 C7 fix).
