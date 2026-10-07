---
title: FathomDB 0.8.28 Slice 30 — Tegra private CUDA memory pool (D28-08) plan
status: PROPOSED (revision 2, 2026-10-07)
target_release: 0.8.28
observed_on: 2026-10-07
---

# 0.8.28 Slice 30: Tegra private CUDA memory pool

This plan turns the 0.8.28 Tegra CUDA memory-pool study into product work.
It is the working plan for D28-08 in `dev/plans/0.8.28-draft-scope.md`
until the 0.8.28 plan, release-state file and board exist. Those must copy
§ 8 (host clean-up) into the 0.8.28 plan when they are created. It does
not replace the owner's rulings; it cites them.

The design input is `dev/plans/0.8.28/slice-30-design-draft.md`. It
records every decision already taken, and the design agent completes it in
S30-T1.

## 1. Sources

All study sources are as of `b045489d2` on `llm/0.8.28-tegra-pool-study`.
They are all on `release/0.8.28` except the vendored Candle copy, which
SD-1 makes unnecessary.

| Source | Role |
| --- | --- |
| `dev/plans/0.8.28/prework/tegra-cuda-memory-pool-study.md` | Study plan, owner rulings 1–38, design notes, gate tables |
| `dev/plans/0.8.28/prework/tegra-cuda-memory-pool-experiment-protocol.md` | Executable method; governs where it is more specific |
| `dev/plans/0.8.28/prework/cudarc-upstream-patch-notes.md` | Upstream cudarc shape (Phase 5 input) |
| `dev/plans/runs/0.8.28-pool-study/results.md` | Results of record, Phases 0–4 and the revision-6 spot check |
| `dev/plans/runs/0.8.28-pool-study/{harness,matrices,summaries,samples,patches,seeds-and-layouts}/` | Evidence and the harness that qualification adapts (§ 7) |

Study code to port (reference only; Slice 30 rewrites it test-first):

| Commit / file | What |
| --- | --- |
| `3ce5c5c46` | cudarc: explicit pool primitive and three-state allocator decision |
| `642347d3a` | embedder: `tegra-pool-experiment` policy and harness |
| `dbd08e313` | P-first-use private pool, fail-closed policy |
| `74a6b4d62` | trim arm (dropped), `cuda_pool_exhausted` kind, Python hook, constants |
| `7b9b18146` | runtime gating and sizing; Python and reranker exhaustion |
| `patches/candle-from-context.patch` | `CudaDevice::from_context` for Candle |

Prerequisite already in `release/0.8.28`: the embedder-close fix
(`c816b8653`, `2ff744b06`, `8247d91a4`; study ruling 26).

## 2. Decisions

Each decision is restated in the design draft and the ADR.

- **SD-1. Candle `from_context` lands on the Candle fork, not as a vendored
  copy.** The complete work is S30-T2 (§ 6):
  - **Fork.** In `coreyt/candle-fathomdb`, cut a branch from `1aefdd008`
    (the head of `fix/tegra-static-cudart-release`, the reviewed revision
    FathomDB pins today).
    - Commit the study's `from_context` change with a unit test.
    - Bump `candle-core-fathomdb`, `candle-nn-fathomdb` and
      `candle-transformers-fathomdb` to 0.10.3 together. `candle-kernels`
      stays at its current version.
  - **Pin set in FathomDB.** Move every reference to the old revision
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
  - **Contracts re-verified on the new revision:**
    - `check-cuda-release-contract.py`: static CUDA runtime, and CPU
      artifacts load without `libcudart`;
    - the pinned-override gate;
    - output equivalence (E2).
  - **HITL.**
    - Pushing the fork branch needs the owner's go-ahead.
    - Publishing the three 0.10.3 crates to crates.io needs it separately,
      before any FathomDB 0.8.28 publish.
    - The 0.8.28 publish gate gains a check that `fathomdb-embedder`, with
      `embed-cuda`, resolves the 0.10.3 crates from crates.io.
  - **Upstream Candle:** optional Phase 5 note (§ 6.1).
- **SD-2. The 8 GB Orin is off in every mode.** `on` lifts only the
  Tegra-identity and unmeasured-class gates. `TooSmall`, `Discrete` and
  `NoPools` hold in every mode; `on` logs the reason and stays off.
  - Basis: ruling 31 says off; results § 12.4 says "revisit only with
    on-device data"; ruling 25 says no host-memory hogging. A 2 GiB pool is
    27 % of an 8 GB board's RAM, unmeasured.
  - This narrows the settings design note's "`on` lifts `TooSmall`".
- **SD-3. Where falling back ends and refusing begins.**
  - At the process's first GPU-use decision, every failure falls back to
    the 0.8.27 synchronous path, with a recorded reason (ruling 37). The
    failures: no early `cuInit`, `cuInit` opted out or failed, a gate is
    off, or pool creation failed.
  - After a private-pool context exists, a later context-build failure is a
    typed error, never a silent switch of allocator (the study's C1
    fail-closed rule).
- **SD-4. The Python import gate counts early `cuInit`.**
  - The Python gate is S + measured `cuInit` cost + 5 ms, with the cost
    reported on its own line.
  - Node keeps S + 5 ms: it already has early `cuInit` in 0.8.27.
- **SD-5. OD-1 is resolved as a non-default `tegra-pool` Cargo feature.**
  - The pool primitive exists only in the vendored cudarc, and the
    workspace `[patch]` does not reach crates.io.
  - The napi and Python Tegra builds enable `tegra-pool`.
  - Without the feature, the crate compiles against stock cudarc 0.19.7,
    pool mode resolves to `off` with reason `not_built`, and behaviour is
    exactly 0.8.27's.
  - The design must prove that the feature is clean against every other
    configuration and execution path (§ 5.2).
- **SD-6. Performance is re-measured against the study build, not the
  default-pool reference from scratch.**
  - The study already measured P-first-use against the default pool and
    the synchronous path, with references accepted in ruling 29.
  - The open question for Slice 30 is whether productisation changed
    anything. So Node compares product P with the study's P build
    (`b045489d2`, rebuilt) and with S, interleaved. The study's
    default-pool ratios carry over by transitivity.
  - Python S is the default pool on this host (results § 12.7.2), so the
    Python series compares product P with Python S directly.
  - This replaces a fresh default-pool reference, which needs many S
    processes because S lands on the default pool only part of the time.

## 3. Gates

### 3.1 The questions

Slice 30 changes how the study's P-first-use path is reached, configured,
reported and failed. It does not change the allocator mechanism itself. So
qualification answers six questions. Everything the study settled and
Slice 30 does not touch is carried, not re-run (§ 3.4).

| Q | Question | Gates |
| --- | --- | --- |
| Q1 | Does each process take the private pool exactly where the rules say, and the 0.8.27 synchronous path everywhere else, with the reason reported? | G1, G2 |
| Q2 | Is the memory given back, and does it stay bounded? | G3 |
| Q3 | Is pool exhaustion typed, recoverable and kept on CUDA, on every path in all three SDKs? | G4 |
| Q4 | Did productisation change behaviour: outputs, heap robustness, concurrency, endurance? | G5, G6, G7, G8 |
| Q5 | Did productisation cost speed? | G9 |
| Q6 | Do coexistence, multi-device isolation and the C7 instrumentation behave as designed? | G10 (tests only) |

### 3.2 Gate rows

**Disposition:**

- **TEST**: an automated test in the suite, written red first.
- **QUAL**: run once on the AGX Orin 64 GB, under the GPU lock, with the
  Node and Python artifacts installed from packed tarballs/wheels.

S is production 0.8.27. Every QUAL series passes only with zero allocator
errors, crashes and OOM kills, and only if every process satisfies
CB1 (`reserved_high` ≤ `maxSize`, read from the pool's own counter, ruling
34). The sample sizes confirm that the product behaves like the study
build. The study's own N carries the statistical power, cited in the last
column.

| Gate | Study rows | Disposition | Method and sample | Pass | Study evidence |
| --- | --- | --- | --- | --- | --- |
| G1 Decision | C1, C6 (pure), sizing | TEST + QUAL | Pure tests for every row of § 3.3 and every `GateReason`, what `on` lifts, `off` winning, SD-3, and the two-device case. Vendored cudarc tests for the three allocator states. QUAL: 20 Node + 20 Python processes in `auto`. | Every FathomDB context is private-pool; every free uses the allocating API; `doctor gpu` reports `private` and the size. | 751/751 private (§ 12.8) |
| G2 Fallback | E1 (ruling 37), R4 | TEST + QUAL | A test per early-`cuInit` state (ran, opted out, failed) and for injected pool-creation failure, in each binding. QUAL: 5 Node + 5 Python processes per state, plus late import without early `cuInit` at a 4M-object heap, 5 processes. | Never refuses. Synchronous path, with the reason in `doctor gpu` and the allocation-mode diagnostic. | R4 (§ 12.6) |
| G3 Release and lifetime | C2, CB1, CB2 | QUAL | 10 Node processes × 100 open/close cycles, no GC; 10 Python processes × 50 cycles; then close and 10 s idle. | All cycles on the private path. Median VmRSS growth ≤ 0.36 MiB per cycle, max ≤ 0.44. `reserved_cur` = 0 after the last close when no module-level model is loaded; otherwise spare < 2 chunks. | 20/20, 0.36 MiB (§ 13.2) |
| G4 Exhaustion | C3, CB3, CB4 | TEST + QUAL | Tests for `cuda_pool_exhausted` in Rust, pytest and vitest, on embed, batch, engine rerank, module-level `embed` and module-level `rerank()`, through a deterministic test allocator. QUAL: oversized batch (128 long passages) at 3 GiB, 5 Node + 5 Python processes. | Typed kind on every path (ruling 33), including the Python `CudaPoolExhaustedError` subclass. The next embed succeeds on device `cuda` with the pre-error embedding hash. | § 11.6, § 12.1 |
| G5 Equivalence | C8, E2 | QUAL | The G1 processes. | Embedding hash and rerank scores identical to S in both bindings; same decision and error kinds. | § 12.7.3 |
| G6 Heap robustness | R2, R3, R5 | QUAL | Node 25, import first: heap grown to 4M objects before first use, and to the R5 boundary cell; 20 processes each, with further heap growth and a second embed and rerank in the same process. | Private path in every process; no path change after the first decision. A failure stops the run for a ruling, never a retry. | 630 R5 runs; R2/R3 on Node 24–26 (§ 10.6, § 12.6) |
| G7 Concurrency | R7 | QUAL | 2, 4 and 8 processes, threshold 0, 5 trials per count; abort below 8 GiB `MemAvailable`. | No failure or OOM kill. | 280/280 (§ 12.6) |
| G8 Soak | R6 | QUAL | One Node and one Python process, concurrently, 20 minutes, no-swap condition. | No allocator error. Reserved memory and RSS within 10 % of the first-5-minute high. Steady drift < 10 %. | P n = 1 (§ 12.6) |
| G9 Performance | P1–P4, P7 | QUAL | Node 25: product P, study P and S, 20 processes each in interleaved randomised blocks. Python: product P and Python S, 20 each. Per process: import time, 5 warm-up + 50 timed steady embeds, rerank, batches 1/8/32/128. Bootstrap 95 % CI over processes (SD-6). | Node: product/study-P ratio ≤ 1.05 for steady embed, rerank and each batch, with CIs reported (if a CI upper bound exceeds 1.10, double N once); CI lower bound of the speed-up over S's synchronous processes > 1.5; import ≤ S + 5 ms and RSS ≤ S + 32 MiB. Python: product P / S ≤ 1.15 on steady embed and rerank; import per SD-4. | Phase 4 (§ 12.7.2), spot check (§ 13.3) |
| G10 Coexistence, devices, C7 | C5, C6, C7, C9 | TEST | Vendored device test that the current-pool handle is never the private pool; zero-length tensor tests on the private path; two-device pure tests; the reset probe as an expected-crash characterization test; context-id detection through a fault seam. | Coexistence and C5 hold. On the first detected loss, `cuda_context_lost` carries both context ids, the driver error and the operation, and one diagnostic snapshot is written. Survival is not required (ruling 36). | C9 Phase 2; § 12.3, § 13.4 |

GPU budget: about 2.5 h in one locked window after S30-T8. The study's
Phases 3 and 4 used about 5.5 h.

### 3.3 Sizing table (ruling 31, SD-2)

| Device class | `auto` | `on` |
| --- | --- | --- |
| AGX Orin 64 GB | 3 GiB (1/20 clamped to [2, 3] GiB) | same |
| Orin 32 / 16 GB | off until an on-device C3 + R5 run | 2 GiB |
| Orin 8 GB (Nano/NX) | off (`TooSmall`) | off (`TooSmall`) |
| Thor | off (unmeasured) | 3 GiB |
| GB10-class (not Tegra) | off (Tegra-identity gate) | sized by rule |
| GH200 | off if discrete (verify `INTEGRATED`) | off if discrete |
| Discrete GPU | off (`Discrete`) | off (`Discrete`) |
| Built without `tegra-pool` | off (`not_built`) | off (`not_built`) |

`off` always wins.

### 3.4 Carried, not re-run

- **R1** synthetic layouts: the C harness exercises the driver, not
  FathomDB code. 300+ runs.
- **R8** release threshold: ruling 30 set 0.
- **C3** capacity model (1/3 of `maxSize`, within 5 %): a driver property.
- **C4** cuBLAS workspaces: bounded in Phase 2. G9's rerank covers the
  chosen size.
- **R2/R3 on Node 24 and 26**: G6 runs Node 25 only.
- **P5** ingest throughput and **P6** memory overhead: reported in Phase 4.
  Neither touches code Slice 30 changes.
- **Unmeasured (declared, kept safe by § 3.3):** real multi-GPU (C6); R9
  boards (8, 16 and 32 GB Orin; Thor; GB10; GH200).

## 4. Product changes

These come from D28-08 and rulings 26–38.

- **Removed:**
  - the trim arm: `TrimArm`, `parse_trim`, the `TRIM_*` constants,
    `trim_tick_ms`, `should_trim`, the trim thread and its counters, and
    `FATHOMDB_POOL_TRIM*`;
  - the comparison arms: `A-first-use`, `B`, `PoolAction::CreateInstalled*`,
    `CudaMemPool::install` and `AllocMode::Explicit`;
  - the study diagnostics: `FATHOMDB_POOL_STATS_EVERY_S`,
    `FATHOMDB_POOL_COEXIST_CHECK`, `FATHOMDB_POOL_MAPS_DIR` and the
    `fdb-pool-exp` stderr lines;
  - `parse_max_size(None)`.
- **Kept:**
  - pool mode `auto`/`on`/`off`, set per process (it replaces
    `FATHOMDB_POOL_VARIANT`);
  - `FATHOMDB_POOL_MAXSIZE`;
  - `FATHOMDB_POOL_RELEASE_THRESHOLD`, default 0;
  - the named constants, each with a pure test;
  - `DEFAULT_MAX_SIZE`, as `POOL_SIZE_CEILING`;
  - the decision and exhaustion events, as `tracing` events;
  - the `tegra-pool` feature (SD-5).

## 5. Design (S30-T1)

### 5.1 What the design must state

The design draft, `dev/plans/0.8.28/slice-30-design-draft.md`, already
holds every decided item. The design agent completes it into the ADR and
the `dev/interfaces/` updates. Neither is ready for review until each item
in the draft's checklist is stated as a sentence in the ADR. A pointer to
the study does not count.

### 5.2 Instruction to the design agent: trace the paths

Before writing the ADR, trace the real code. Do not rely on the study
write-up. Record the result in the design draft's § 4 tables. It must show
that the `tegra-pool` feature, on or off, and the pool mode do not
interfere with any other configuration, and do not introduce races on any
execution path.

1. **Configuration matrix.** For each combination, record the build
   outcome, the allocator decision and the reported reason, and cite the
   code at file:line:
   - targets: aarch64 Linux Tegra, aarch64 Linux non-Tegra, x86_64 Linux
     CUDA, macOS Metal, CPU-only;
   - Cargo features: `tegra-pool` on and off, crossed with `embed-cuda`,
     `rerank-cuda`, `embed-metal` and none;
   - pool mode: `auto`, `on`, `off`, invalid;
   - early `cuInit` state.

   Expected: built without `tegra-pool`, or on any non-Tegra target,
   behaviour is byte-for-byte 0.8.27, with no warnings and no dead-code
   lints.
2. **Feature unification.** `cargo build --workspace` and `cargo test
   --workspace` unify features.
   - Trace which crates enable `tegra-pool`: `fathomdb-napi`,
     `fathomdb-py`, and whether `fathomdb`, `fathomdb-sdk` or
     `fathomdb-cli` inherit it.
   - Show that workspace test runs exercise both the on and off builds,
     and that crates.io packaging of each crate compiles without the
     vendored cudarc. Add the CI or verify step that proves it.
3. **Interference with existing behaviour.** Trace each against the pool
   decision:
   - the 0.8.27 synchronous-fallback decision table and `FATHOMDB-PATCH.md`
     items 1–3;
   - the Slice 110 early `cuInit` and its opt-out;
   - the reranker device policy;
   - forced-device and CPU-fallback options;
   - the embedder-close fix (`close_lock`, the drop order);
   - the module-level embedder and reranker singletons;
   - the existing `FATHOMDB_*` settings;
   - `doctor gpu` on hosts with no CUDA.

   Each must either be unaffected or have its change named in the
   behaviour-change list.
4. **Execution paths.** Trace each end to end:
   - Node module load → `cuInit` record → first GPU use → decision →
     context and pool → Candle device → allocation and free → close →
     environment teardown and process exit. First GPU use can be
     `Engine.open`, module-level `embedBatchCls`, or `rerank()`;
   - the same for Python: import, first use with the GIL released, and
     interpreter finalisation;
   - the same for direct Rust use;
   - `fathomdb doctor gpu`, which must report the decision without creating
     a context or a pool.
5. **Races and shared state.** Build a table with these columns: shared
   state, writers, readers, synchronisation primitive, and the test that
   pins it. Cover at least:
   - concurrent first use from several threads: Node worker threads and
     the libuv pool, Python threads, Rust threads. The decision is made
     exactly once, and every thread sees the same result;
   - settings read once before the decision; a later environment change is
     ignored, and that is documented;
   - the order in which the early-`cuInit` state is written and read;
   - a pool-creation failure while other threads wait;
   - close racing an in-flight embed or rerank;
   - drop order of slices, stream, context and pool at close and at
     process exit (static destructors, napi environment teardown, Python
     finalisation);
   - context-loss detection firing from two threads, so that exactly one
     snapshot is written;
   - exhaustion under concurrent requests;
   - `fork()` after `cuInit` (Python `multiprocessing` with the fork start
     method): detected or documented.
6. **Outputs.** The three tables in the design draft's § 4, each row with a
   file:line citation and a named test. Every hazard found gets a test in
   the TDD ladder (§ 6) or a documented limit in the ADR.

## 6. TDD ladder

Each step is red → green → refactor. The test is committed first and its
red is shown against the step's parent. Test files are read-only during
fix-to-spec. Merge `release/0.8.27` forward before S30-T0 and before each
review gate (D28-08).

| Step | Work | Red test first | Reference |
| --- | --- | --- | --- |
| S30-T0 | Preflight; merge 0.8.27 forward. Create the 0.8.28 plan, release-state file and board, copying § 8 into the plan, or record that this file stands in. | — | `scripts/preflight.sh` |
| S30-T1 | Design: complete the design draft (§ 5.2 traces), then the ADR and interface-doc drafts. Owner review. | — | design draft; study design notes |
| S30-T2 | Candle fork and pin set (SD-1). **HITL: push. HITL: crates.io publish.** | Candle unit test for `from_context` (default stream; allocator of the given context) | `patches/candle-from-context.patch` |
| S30-T3 | cudarc pool primitive, written upstream-first as small commits in upstream style; the vendored change is the backport. Update the `FATHOMDB-PATCH.md` items, the pinned-override expected set and `test_vendored_cudarc.sh`. | Vendored unit tests: the three allocator states, C5, drop order | `3ce5c5c46`; upstream notes § 1 |
| S30-T4 | Policy: the `tegra-pool` feature, sizing, gates, modes, settings, constants, SD-2, SD-3. | G1 pure tests; feature-off build test | `dbd08e313`, `7b9b18146` |
| S30-T5 | Early `cuInit` contract: state record in Node and Python, Python hook and opt-out, fallback. | G2 state tests in each binding | `74a6b4d62` |
| S30-T6 | Typed exhaustion on every path in all three SDKs. | G4 tests per path | `7b9b18146`; `src/ts/tests/typed-errors.test.ts` |
| S30-T7 | C7 instrumentation: context id, `cuda_context_lost`, snapshot, `doctor gpu` context state, characterization test. | G10 C7 tests | `harness/pool_reset.c` |
| S30-T8 | Race and path tests from § 5.2 item 5; `doctor gpu` and allocation-mode fields; `tracing` events; scaffolding removal (§ 4). | Race tests; CLI output tests; a test that the removed env vars and `fdb-pool-exp` are absent | design draft § 4 |
| S30-T9 | Phase 5 (§ 6.1). | — | study plan "Upstream cudarc compatibility" |
| S30-T10 | Bring the nine study harness shell scripts onto the branch lint-clean (§ 7). Adapt the harness to the product in a new qualification directory. Qualification (§ 3) on packed artifacts; results file; codex § 9 review; land. | Shell lint (`scripts/agent-lint-shell.sh`) passes on the tracked scripts | protocol; study harness (§ 7) |
| S30-T11 | Host clean-up (§ 8). **HITL.** | — | — |

Order: T2 and T3, then T4, then T5, T6 and T7 in parallel worktrees, then
T8, then T10 and T11. T9 runs alongside T6–T8. The error kinds touch
`errors.rs` in the engine, py and napi crates, so those additions in T6 and
T7 are serialized.

### 6.1 Phase 5, folded in

The study's Phase 5 (analysis and the upstream package) was never run. It
is S30-T9:

- **Analysis:** a closing section of
  `dev/plans/runs/0.8.28-pool-study/results.md`. It runs the decision rule
  over Phases 0–4 and the spot check, lists every UNMEASURED item, and
  records what moves to the 0.8.29 C7 fix (TC-281155a1).
- **cudarc upstream package:**
  - re-read every cited thread;
  - prepare the issue and the commit series for `CudaMemPool`,
    `CudaContext::new_with_mem_pool`, `AllocMode::Private` and
    `CudaContext::mem_pool()`, which S30-T3 already wrote upstream-first;
  - record each item's upstream status in `FATHOMDB-PATCH.md`.
  - Nothing is posted without the owner's sign-off. 0.8.28 does not wait
    for upstream (ruling 38; TC-b8e5fe4d).
- **Candle (optional):** a matching upstream note for `from_context`, under
  the same sign-off rule.
- **NVIDIA report:** record the adoption outcome and refresh the `cuInit`
  finding if it has changed.

## 7. Study evidence

The whole committed evidence directory `dev/plans/runs/0.8.28-pool-study/`
is on `release/0.8.28` (owner choice, option B): 143 of its 152 files.
Two exceptions:

- **The harness's nine shell scripts (25 KiB) stay on the study branch for
  now.** They fail the repository's shell lint: 75 SC2312 findings, one
  SC2012, and three pipes into an early-exiting `head`. The lint's ratchet
  may only shrink, so they cannot be exempted. Fixing them changes the
  record without a GPU re-run. S30-T10 brings them over lint-clean, or the
  owner rules otherwise.
- **The 1.5 MiB vendored Candle copy** stays on the study branch (SD-1).

S30-T10 adapts `harness/` to the product's `tracing` events and pool-mode
setting, in a new qualification directory. The study's harness files stay
as they are, as the record.

## 8. Host clean-up (HITL)

The study's untracked raw data is on the Jetson AGX Orin (Ubuntu) host in
an earlier session's scratchpad,
`/tmp/claude-1000/-home-coreyt-projects-fathomdb/ea4f7b06-2eaf-4683-9916-58e4b19cfc97/scratchpad/pool-study/`.
As of 2026-10-07 it is 5.4 GiB:

- `logs/`: 292 MiB of raw per-run logs;
- `dbs/`: 4.5 GiB of databases;
- the rest is builds, staged artifacts and virtual environments.

Protocol § 9.2 keeps it until the 0.8.28 ruling is recorded. Slice 30
keeps it until S30-T10 closes, because G9 rebuilds the study's P build and
may need to compare against its artifacts.

Then, as S30-T11, with the owner's explicit permission for each deletion:

1. List and size the directory again.
2. Ask the owner whether to keep a compressed archive of `logs/`, and
   where.
3. Delete `dbs/`, the builds and the virtual environments.
4. Delete `logs/` once any archive is verified.

Nothing is deleted without a HITL go-ahead. The 0.8.28 plan, when created,
carries this as a release task.

## 9. Ledger and todos

TC-1c70e523 (study, done) · TC-a7c4a599 (early `cuInit`, decided) ·
TC-f4e12b96 (typed context loss) · TC-99845d75 (C7 instrumentation) ·
TC-b8e5fe4d (upstream and drift) · TC-281155a1 (0.8.29 C7 fix).
