---
title: FathomDB 0.8.28 Slice 30 — Tegra private CUDA memory pool (D28-08) plan
status: PROPOSED (2026-10-07)
target_release: 0.8.28
observed_on: 2026-10-07
---

# 0.8.28 Slice 30: Tegra private CUDA memory pool

This plan turns the 0.8.28 Tegra CUDA memory-pool study into product work.
It is the working plan for D28-08 in `dev/plans/0.8.28-draft-scope.md`
until the 0.8.28 plan, release-state file and board exist. It does not
replace the owner's rulings; it cites them.

## 1. Sources

All study sources are as of `b045489d2` on `llm/0.8.28-tegra-pool-study`.

| Source | On `release/0.8.28` | Role |
| --- | --- | --- |
| `dev/plans/0.8.28/prework/tegra-cuda-memory-pool-study.md` | yes | Study plan, owner rulings 1–38, design notes, gate tables |
| `dev/plans/0.8.28/prework/tegra-cuda-memory-pool-experiment-protocol.md` | yes | Executable method; governs where it is more specific |
| `dev/plans/0.8.28/prework/cudarc-upstream-patch-notes.md` | yes | Upstream cudarc shape (Phase 5 input) |
| `dev/plans/runs/0.8.28-pool-study/results.md` | yes | Results of record, Phases 0–4 and the revision-6 spot check |
| `dev/plans/runs/0.8.28-pool-study/{harness,matrices,summaries,samples,patches,seeds-and-layouts}/` | no (study branch) | Evidence; see § 7 |

Study code to port (reference only; Slice 30 rewrites it test-first):

| Commit | What |
| --- | --- |
| `3ce5c5c46` | cudarc: explicit pool primitive and three-state allocator decision |
| `642347d3a` | embedder: `tegra-pool-experiment` policy and harness |
| `dbd08e313` | P-first-use private pool, fail-closed policy |
| `74a6b4d62` | trim arm (dropped), `cuda_pool_exhausted` kind, Python hook, constants |
| `7b9b18146` | runtime gating and sizing; Python and reranker exhaustion |
| patch `candle-from-context.patch` | `CudaDevice::from_context` for Candle |

Prerequisite already in `release/0.8.28`: the embedder-close fix
(`c816b8653`, `2ff744b06`, `8247d91a4`; study ruling 26).

## 2. Decisions taken in this plan (2026-10-07)

The owner delegated these when approving the plan. Each one must be
restated in the ADR (§ 5).

- **SD-1. Candle `from_context` goes on the Candle fork, not a vendored
  copy.**
  - Context: the private pool needs Candle to build a device on a context
    FathomDB created (`CudaDevice::from_context`). The study vendored
    `third_party/candle-core-fathomdb-0.10.2` (1.5 MiB, 104 files) and marked
    it "study branch only (never merged)".
  - FathomDB already pins `coreyt/candle-fathomdb` at `1aefdd008`, the head
    of the fork's `fix/tegra-static-cudart-release` branch. That branch
    already carries the Fathom Tegra patches: static CUDA runtime, dynamic
    driver loading, the AArch64 CPU F16 fallback and packaging.
  - Decision: cut a fork branch from `1aefdd008`, commit the study's
    `candle-from-context.patch` there with a test, and move all four rev pins
    in the root `Cargo.toml` together. No vendored Candle tree lands.
  - Published crates: `fathomdb-embedder` pins
    `candle-core-fathomdb = "=0.10.2"` from crates.io, and the root `[patch]`
    does not reach crates.io users. The new API therefore needs a
    `candle-core-fathomdb` release (0.10.3, with `nn` and `transformers` at
    the same version) and the `=` pins moved to it.
  - Pushing to the fork and publishing to crates.io are outward actions.
    Each needs the owner's go-ahead at the step that does it (S30-T2).
  - An upstream Candle PR for `from_context` is optional Phase 5 work (§ 6).
- **SD-2. The 8 GB Orin is off in every mode.**
  - Ruling 31 says the 8 GB Orin is off. The results (§ 12.4) say "revisit
    only with on-device data". The code enforces it through the quarter rule
    (`TooSmall`: 2 GiB floor > ¼ of the ~7.4 GiB reported).
  - The accepted settings note lets `on` lift `TooSmall`. On an 8 GB board
    that would reserve 27 % of system RAM, unmeasured. That conflicts with
    ruling 31 and with ruling 25 (no host-memory hogging).
  - Decision: `on` lifts only the Tegra-identity gate and the
    unmeasured-class gate. `TooSmall`, `Discrete` and `NoPools` hold in every
    mode, and `on` logs the reason and stays off. Revisit with on-device 8 GB
    data under a new ruling.
- **SD-3. Where the boundary between fallback and refusal sits.**
  - Ruling 37 says FathomDB never refuses. The study's C1 rule is fail
    closed.
  - Both hold, at different times:
    - At the process's first GPU-use decision, every failure falls back to
      the 0.8.27 synchronous path, with a recorded reason. The failures are:
      no early `cuInit`, `cuInit` opted out or failed, a gate is off, or pool
      creation failed.
    - After a private-pool context exists, a later context-build failure is
      a typed error, never a silent switch of allocator.
- **SD-4. The Python import gate counts early `cuInit`.**
  - Python gains early `cuInit` (ruling 37), so its import time rises by the
    `cuInit` cost (about 12 ms, owner's figure).
  - The P1 import gate for Python is S + measured `cuInit` cost + 5 ms. The
    cost is reported on its own line.
  - Node already has early `cuInit` in 0.8.27, so its gate stays S + 5 ms.

### Open design decision (resolve in the ADR, before S30-T3)

- **OD-1. crates.io consumers of `fathomdb` with `embed-cuda`.**
  - The pool primitive (`CudaMemPool`, `CudaContext::new_with_mem_pool`)
    exists only in the vendored cudarc, through a workspace `[patch]` that
    does not reach crates.io. If product code calls it unconditionally, the
    published Rust crates stop compiling with `embed-cuda` against stock
    cudarc 0.19.7.
  - The study avoided this with a non-default feature.
  - Recommendation: a non-default Cargo feature, `tegra-pool`, which the
    napi and Python Tegra builds turn on. Without it the code compiles
    against stock cudarc, and pool mode resolves to `off` with reason
    `not_built`.
  - Alternatives: publish a `cudarc-fathomdb` fork crate, or wait for
    upstream. Either way, the ADR must state what a crates.io build gets.

## 3. Gates

Each study gate is one row. **Disposition** says how Slice 30 meets it:

- **TEST**: an automated test in the suite, written red first.
- **QUAL**: re-run on the product build, on the AGX Orin 64 GB, under the
  GPU lock, with the Node and Python artifacts installed from packed
  tarballs/wheels.
- **STUDY**: carried from the study evidence, unchanged. Allowed only where
  product code does not alter the thing measured.
- **UNMEASURED**: declared, with the gate that keeps it safe.

S means the production 0.8.27 build (synchronous fallback). "Default pool"
means S's default-pool processes, the reference accepted in ruling 29. The
study's pass bars are kept. QUAL sample sizes confirm that the product
build behaves like the study build; the statistical power comes from the
study's own N, which each row cites. A QUAL row passes only with zero
allocator errors, crashes or OOM kills, unless it says otherwise.

Allocation correctness and release are gated first (ruling 25). Latency
rows are gated after them.

### 3.1 Allocation correctness and release

| # | Gate | Disposition | Qualification method and sample | Pass | Study evidence |
| --- | --- | --- | --- | --- | --- |
| C1 | Allocator provenance | TEST + QUAL | Vendored cudarc unit tests for the three shipped states (default, private, synchronous); pure test of SD-3; constructor and cross-wrapper free tests as the study ran them. Device: 20 Node + 20 Python processes in `auto`. | Every FathomDB context is a private-pool context; every free uses the allocating API. | 751/751 private (results § 12.8) |
| C2 | Pool lifetime: open/close cycles | QUAL | 20 Node processes × 100 cycles, no GC. | All cycles allocate on the private path. Median VmRSS growth ≤ 0.36 MiB per cycle, max ≤ 0.44. `reserved_cur` = 0 after the last close when no module-level model is loaded. | 20/20, 0.36 MiB (§ 13.2) |
| C3 | Exhaustion behaviour | TEST + STUDY | Capacity model carried. Deterministic exhaustion via a test allocator. | Policy (i), typed refusal. Capacity model within 5 % (carried). No silent CPU move. | § 3.1, § 10.2 |
| C4 | cuBLAS/cuRAND workspaces | STUDY + QUAL | Carried; rerank at 3 GiB is exercised in P3 and R5. | Rerank passes at the chosen size. | Phase 2, § 11 |
| C5 | Zero-length buffers | TEST | Zero-element tensor and `CudaStream::null()` tests on the private path. | No null pointer reaches a free. | § 10.2 |
| C6 | Multi-device | TEST + UNMEASURED | Pure two-device decision tests. | Decisions and pools never cross devices. Real multi-GPU declared unmeasured. | — |
| C7 | Co-resident context reset | TEST (characterization) | The study's reset probe as an expected-crash test; context-id check on CUDA errors. | The probe documents the current crash. The first detected loss raises typed `cuda_context_lost` with both context ids, the driver error and the operation, and writes one diagnostic snapshot. Survival is **not** required (ruling 36; fix in 0.8.29). | § 12.3, § 13.4 |
| C8 | Python and Node parity | QUAL | 20 processes per binding, packed artifacts. | Same decision; embedding hash and rerank scores identical to S; same error kinds. | § 12.7.3 |
| C9 | Coexistence with the device's current pool | TEST + QUAL | Vendored device test; Node 10 processes × 50 cycles; Python co-resident `ctypes` user, 10 processes. A test-support hook replaces the removed `FATHOMDB_POOL_COEXIST_CHECK`. | The current-pool handle never equals the private pool's. The co-resident user's allocations land in the default pool. | Phase 2 |
| CB1 | Cap | QUAL (all QUAL processes) | The pool's own `reserved_high` counter (ruling 34). | `reserved_high` ≤ `maxSize` in every process. `MemAvailable` is a sanity check on integrated devices, never a failure. | 290/290 (§ 12.8) |
| CB2 | Release after close | QUAL | C2 processes, plus 20 `full` processes: close, 10 s idle. | Threshold 0: `reserved_cur` = 0 after the last close with no module model loaded. Otherwise spare < 2 chunks. | § 12.7.1 |
| CB3 | Typed error at the cap | TEST + QUAL | Rust test. QUAL: oversized batch (128 long passages) at 3 GiB, 20 Node + 20 Python processes. TS and Python tests on embed, batch, engine rerank, module-level `embed` and module-level `rerank`. | `cuda_pool_exhausted` on every path in all three SDKs (ruling 33). The next embed succeeds. | § 11.6, § 12.1 |
| CB4 | No CPU move | QUAL | Same runs as CB3. | The embed after the error reports device `cuda` and the pre-error embedding hash. | § 11.6 |
| E1 | Early `cuInit` check and fallback (ruling 37) | TEST + QUAL | Tests for each state: ran, opted out, failed, pool creation failed (injected). QUAL: 10 processes per state, Node and Python. | Never refuses. Synchronous path with the reason in `doctor gpu` and the allocation-mode diagnostic. | — (new) |
| E2 | Output equivalence | QUAL | The C8 processes. | Embedding hash and rerank scores identical to S. | § 12.7.3 |

### 3.2 Robustness

| # | Gate | Disposition | Qualification method and sample | Pass | Study evidence |
| --- | --- | --- | --- | --- | --- |
| R1 | Synthetic fragmented layouts | STUDY | Not re-run: the C harness exercises the driver, not FathomDB code. | — | 300+ runs, ≥ 98.9 % bound (§ 12.6) |
| R2 | Real Node heaps | QUAL | Heaps 0, 400k and 4M objects; Node 24, 25, 26; 20 processes per cell (180). | Zero failures; every process private. | § 12.6 |
| R3 | Heap growth during use | QUAL | 20 processes × Node 24, 25, 26. | Zero failures; no path change after the first decision. | § 12.6 |
| R4 | Late import | TEST + QUAL | With and without early `cuInit`; 20 processes per heap size (0, 4M). | With early `cuInit`: private. Without: synchronous with the reason (E1). | § 12.6 |
| R5 | Lazy creation after heap growth | QUAL | 3 GiB at the study's boundary cells and 4M, Node 25, 30 processes per cell. | P passes in every cell. A failure is a stop-and-rule event, not a retry. | 630 P runs (§ 10.6, § 12.6) |
| R6 | Soak | QUAL | Two processes, 20 min each, Node and Python, no-swap condition. | No allocator error. Reserved memory and RSS within 10 % of the first-5-minute high. Steady drift < 10 %. | P n = 1 (§ 12.6); the protocol asks for 2 |
| R7 | Concurrent processes | QUAL | 2, 4 and 8 processes, threshold 0, 10 trials per count; abort below 8 GiB `MemAvailable`. | No process fails or is OOM-killed. | 280/280 (§ 12.6) |
| R8 | Release threshold | STUDY | Ruling 30: threshold 0. | — | § 12.7.2 |
| R9 | Unmeasured boards | UNMEASURED | Sizing table (§ 3.4). | Unmeasured classes stay off or opt-in. | — |

### 3.3 Performance (gated after § 3.1)

20 fresh processes per cell, 5 warm-up and 50 timed iterations, the process
as the unit, interleaved randomised blocks, a percentile-bootstrap 95 % CI,
no-swap condition (protocol § 1.1). Reference: "Default pool" is
Node + Python pooled per ruling 29.

| # | Measure | Pass |
| --- | --- | --- |
| P1 | Import time, open time, RSS | Node: import ≤ S + 5 ms; RSS ≤ S + 32 MiB. Python: SD-4. Open time reported. |
| P2 | Steady embed | P / default-pool ratio ≤ 1.15. CI lower bound of the speed-up over S's synchronous path > 1.5. If the ratio's CI upper bound exceeds 1.25, double N once before ruling. |
| P3 | Steady rerank (ruling 28) | P / default-pool rerank time ≤ 1.15, with the CI stated. Report the old 1.5× reading as well. |
| P4 | Embed batch 1, 8, 32, 128 | Each batch size: P / default-pool ratio ≤ 1.15. |
| P5 | Bulk ingest (≥ 10,000 docs) | CI lower bound of the P / S throughput ratio ≥ 0.95. |
| P6 | Memory overhead | Reported; gated by CB1/CB2. |
| P7 | Python steady embed and ingest | P / Python default-pool ratio ≤ 1.15. |
| P8 | First embed | No worse than S: CI upper bound of the P / S time ratio ≤ 1.05. |

GPU budget: the study's Phases 3 and 4 used about 5.5 h of lock time. This
qualification is about 4 h, in one locked window after S30-T8.

### 3.4 Sizing table (ruling 31, SD-2)

| Device class | `auto` | `on` |
| --- | --- | --- |
| AGX Orin 64 GB | 3 GiB (1/20 clamped to [2, 3] GiB) | same |
| Orin 32 / 16 GB | off until an on-device C3 + R5 run | 2 GiB |
| Orin 8 GB (Nano/NX) | off (`TooSmall`) | off (`TooSmall`, SD-2) |
| Thor | off (unmeasured) | 3 GiB |
| GB10-class (not Tegra) | off (Tegra-identity gate) | sized by rule |
| GH200 | off if `INTEGRATED` = 0 (verify) | off if discrete |
| Discrete GPU | off | off (`Discrete`) |

`off` always wins.

## 4. Product changes

These are from D28-08 and rulings 26–38. The study code is the reference
only. Everything else is removed:

- **Removed:** the trim arm (`TrimArm`, `parse_trim`, the `TRIM_*`
  constants, `trim_tick_ms`, `should_trim`, the trim thread and its counters,
  `FATHOMDB_POOL_TRIM*`); the comparison arms (`A-first-use`, `B`,
  `PoolAction::CreateInstalled*`, `CudaMemPool::install`,
  `AllocMode::Explicit`); the study diagnostics (`FATHOMDB_POOL_STATS_EVERY_S`,
  `FATHOMDB_POOL_COEXIST_CHECK`, `FATHOMDB_POOL_MAPS_DIR`, the
  `fdb-pool-exp` stderr lines); and `parse_max_size(None)`.
- **Kept:**
  - pool mode `auto`/`on`/`off`, set per process (it replaces
    `FATHOMDB_POOL_VARIANT`);
  - `FATHOMDB_POOL_MAXSIZE`;
  - `FATHOMDB_POOL_RELEASE_THRESHOLD`, default 0;
  - the named constants, each with a pure test;
  - `DEFAULT_MAX_SIZE` as `POOL_SIZE_CEILING`;
  - the decision and exhaustion events, which become `tracing` events.

## 5. What the design (ADR + interface docs) must state

S30-T1 writes the ADR. It is not ready for review until it states each item
below explicitly. Each item gets a sentence, not a pointer to the study.

1. The adoption rule: P-first-use, created at first GPU use, only after
   early `cuInit` ran (ruling 37).
2. SD-3: the boundary between falling back and refusing.
3. The early `cuInit` states (ran / opted out / failed). How each binding
   records them. Python's opt-out. The Slice 117 import-time `cuInit` item
   settled the same way.
4. The sizing rule and the constants: divisor 20, floor 2 GiB, ceiling
   3 GiB, the quarter rule. The measured basis of the floor and ceiling
   (1/3 capacity, 32 MiB chunks) is cited as evidence, not as logic.
5. The sizing table in § 3.4, including SD-2 and exactly what `on` lifts.
6. The settings: names, defaults, accepted values, the process-level
   carrier, precedence, and when they are read (before the first device).
   They live in FathomDB, never in cudarc.
7. The release threshold of 0, its caveat (ruling 30) and why.
8. The error kinds `cuda_pool_exhausted` and `cuda_context_lost`: payload
   fields, the class in each SDK (including the Python
   `CudaPoolExhaustedError` subclass), and every path that raises them.
9. The `fathomdb doctor gpu` fields and the allocation-mode diagnostic:
   chosen path, reason, pool size, threshold, the `cuInit` state, context
   state.
10. C7: unsupported and documented. What is detected (`cuCtxGetId`), what
    is raised, and the snapshot contents. The 0.8.29 fix and its
    characterization test.
11. Module-level models stay loaded until the process exits and hold pool
    memory (ruling 32). The release call is 0.8.30 B30-01.
12. OD-1: what a crates.io build gets.
13. SD-1: the Candle fork rev and the `candle-core-fathomdb` version.
14. The cudarc patch items: each with its upstream status (local-only,
    proposed, merged, released through Candle) and removal path. The
    aarch64 synchronous fallback stays local. The pinned-override gate's
    expected set.
15. The gate table in § 3 by reference, with each disposition.
16. The behaviour changes for the changelog: the default allocator on AGX
    Orin 64 GB, Python early `cuInit`, the new error kinds, the new
    settings.

The `dev/interfaces/` updates (rust, python, typescript, cli) land in the
same change as the code that they describe (D28-08).

## 6. TDD ladder

Each step is red → green → refactor. The test is committed first and its
red is shown against the step's parent. Test files are read-only during
fix-to-spec. Merge `release/0.8.27` forward before S30-T0 and before each
review gate (D28-08).

| Step | Work | Red test first | Reference |
| --- | --- | --- | --- |
| S30-T0 | Preflight. Merge 0.8.27 forward. Create the 0.8.28 plan, release-state file and board, or record that this file stands in until they exist. | — | `scripts/preflight.sh` |
| S30-T1 | ADR + interface-doc drafts covering § 5; resolve OD-1. Owner review. | — | study plan design notes |
| S30-T2 | Candle fork: `from_context` with a test; rev pins; `candle-core-fathomdb` 0.10.3. **Owner OK to push and publish.** | Candle unit test for `from_context` (default stream, allocator of the given context) | `candle-from-context.patch` |
| S30-T3 | cudarc pool primitive, upstream-first: small commits in upstream style; the vendored change is the backport. Update the `FATHOMDB-PATCH.md` items, the pinned-override expected set and `test_vendored_cudarc.sh`. | Vendored unit tests: C1 (three states), C5, C6, drop order (C2 lifetime) | `3ce5c5c46`; upstream notes § 1 |
| S30-T4 | Policy: sizing, gates, modes, settings, constants, SD-2, SD-3. | Pure tests: sizing table rows, each `GateReason`, what `on` lifts, `off` wins, the constants, SD-3 | `dbd08e313`, `7b9b18146` |
| S30-T5 | Early `cuInit` contract: state record (Node + Python), Python hook and opt-out, fallback. | E1 state tests in each binding | `74a6b4d62` (Python hook) |
| S30-T6 | Typed exhaustion on every path in all three SDKs, including module-level `rerank()`. | CB3 tests in Rust, pytest and vitest per path | `7b9b18146`, `src/ts/tests/typed-errors.test.ts` |
| S30-T7 | C7 instrumentation: context id, `cuda_context_lost`, snapshot, `doctor gpu` context state, characterization test. | Characterization test (expected crash); typed-error tests through a fault seam | study reset probe (`harness/pool_reset.c`) |
| S30-T8 | `doctor gpu` / allocation-mode diagnostic fields; `tracing` events; scaffolding removal (§ 4). | CLI output tests; a grep test that the removed env vars and `fdb-pool-exp` are absent | design note "Settings and constants" |
| S30-T9 | Phase 5: analysis and the upstream package (§ 6.1). | — | study plan "Upstream cudarc compatibility" |
| S30-T10 | Qualification (§ 3) on packed artifacts; results file; codex § 9 review; land. | — | protocol; adapted harness (§ 7) |

S30-T3 → T4 → T5/T6/T7 (parallel worktrees; shared files `errors.rs` in the
engine, py and napi crates — serialize the error-kind additions) → T8 → T10.
T2 must land before T4 builds a device. T9 runs alongside T6–T8.

### 6.1 Phase 5, folded in

The study's Phase 5 (analysis and the upstream package) was never run. It
is done as S30-T9:

- **Analysis:** a closing section of
  `dev/plans/runs/0.8.28-pool-study/results.md`. It runs the decision rule
  over Phases 0–4 and the revision-6 spot check, lists every UNMEASURED
  item, and records what moves to the 0.8.29 C7 fix (TC-281155a1).
- **cudarc upstream package:**
  - re-read every cited thread (upstream notes, "Sources");
  - prepare the issue and the small commit series for `CudaMemPool`,
    `CudaContext::new_with_mem_pool`, `AllocMode::Private` and
    `CudaContext::mem_pool()`, which S30-T3 already wrote upstream-first;
  - record each item's upstream status in `FATHOMDB-PATCH.md`.
  - Nothing is posted without the owner's sign-off. 0.8.28 does not wait
    for upstream (ruling 38; TC-b8e5fe4d).
- **Candle (optional):** a matching upstream note for `from_context`, under
  the same sign-off rule.
- **NVIDIA report:** the study's decision rule sends unshippable partial
  results there. Record that adoption passed, and refresh the cuInit
  finding if it has changed.

## 7. Study evidence

Sizes as of 2026-10-07:

| Set | Where | Size |
| --- | --- | --- |
| Docs (plan, protocol, upstream notes) | now on `release/0.8.28` | 169 KiB |
| `results.md` | now on `release/0.8.28` | 125 KiB |
| Committed evidence: harness 167 KiB, summaries 189, matrices 103, samples 131, patches 62, seeds 45, top-level 128 (`results.md`, `env.txt`, `artifacts.sha256`) | study branch only, except `results.md` | 152 files, 826 KiB |
| Vendored Candle copy | study branch only | 104 files, 1.5 MiB; not needed after SD-1 |
| Raw logs, builds, databases | host: an earlier session's scratchpad under `/tmp/claude-1000/.../scratchpad/pool-study/` | 5.4 GiB: logs 292 MiB, databases 4.5 GiB, builds and venvs the rest |

**Owner decision pending.** The options are listed in the hand-off. S30-T10
needs the harness, adapted to the product's `tracing` events, whichever
option is chosen.

## 8. Ledger and todos

TC-1c70e523 (study, done) · TC-a7c4a599 (early `cuInit`, decided) ·
TC-f4e12b96 (typed context loss) · TC-99845d75 (C7 instrumentation) ·
TC-b8e5fe4d (upstream and drift) · TC-281155a1 (0.8.29 C7 fix).
