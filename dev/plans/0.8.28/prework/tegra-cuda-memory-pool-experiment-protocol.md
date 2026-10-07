---
title: FathomDB 0.8.28 prework — Tegra CUDA memory-pool experiment protocol
status: PROPOSED (revision 6, 2026-10-07; owner rulings 12-35 applied)
target_release: 0.8.28
observed_on: 2026-10-06
---

# Tegra CUDA memory-pool experiment protocol (0.8.28)

This document executes the study plan
`dev/plans/0.8.28/prework/tegra-cuda-memory-pool-study.md`. The plan says
what must be shown; this protocol says exactly what to build, run, record and
compute, so that an engineer can run the evaluation without further design
decisions. Where the two differ, this protocol's method governs and the plan
is corrected in place. Nothing here authorizes a product change. Nobody posts
anything upstream or to NVIDIA on the strength of this document; § 10 only
prepares material for the owner.

Every number quoted below comes from the Slice 110 evidence on this branch
under `dev/plans/runs/0.8.27-slice-110-tegra/` or from this study's Phases 0–1
results (`dev/plans/runs/0.8.28-pool-study/results.md`), and was measured on
one Jetson AGX Orin 64 GB (L4T R36.5.2, driver 540.5.0, CUDA 12.6.68). Statements marked
*inference* or *hypothesis* are not measured and are tested by this protocol.

## Status (2026-10-07)

**This branch's copies of the protocol and the plan are the record.** The study branch is
`llm/0.8.28-tegra-pool-study`. An older copy of the study plan, dated
2026-10-05, is being merged onto `release/0.8.27` with a status header
pointing here. Where they differ, this branch governs.

- **Done.**
  - Phases 0–4 are complete.
  - A revision-6 spot check ran on the corrected close fix.
  - Results are in `dev/plans/runs/0.8.28-pool-study/results.md`: §§ 2–11
    for Phases 0–2, § 12 for Phases 3 and 4, § 13 for the spot check.
  - Owner rulings 12–35 are recorded below and in the plan, revision 6.
- **Not started.** Phase 5: analysis and the upstream package. The
  upstream shape is prepared in
  `dev/plans/0.8.28/prework/cudarc-upstream-patch-notes.md`; nothing is
  posted without the owner.
- **Finding on this host** (Jetson AGX Orin 64 GB). The private pool
  (P-first-use) passes:
  - **Allocation correctness:** 751/751 private-pool processes in Phases
    3–4, 50/50 in the spot check.
  - **Release after close:** the pool returns to 0 MiB after the last close
    when no module singleton is in use.
  - **Caps.**
  - **The performance gate:** Phase 4 pooled embed 0.986 [0.907, 1.114] of
    the default pool, and 1.9–4.9× faster than the synchronous path. Rerank
    passes as re-based by ruling 28.
  - **C2 on the corrected fix:** 20/20, 0.36 MiB per cycle.
- **What still blocks adoption:**
  1. The owner's C7 ruling. After a co-resident context reset, every
     variant, the shipped one included, crashes in `Engine::close` in
     cudarc's `CudaSlice` drop (ruling 27).
  2. Early `cuInit` at module load, accepted for this round, is still to be
     discussed with the owner (ruling 35).
  3. Typed `cuda_pool_exhausted` in all three SDKs on every path,
     including module-level `rerank` and `embed`, and the Python subclass
     (ruling 33).
  4. Sizing checks off the 64 GB Orin (ruling 31):
     - on-device C3 and R5 for 32 and 16 GB;
     - Thor opt-in;
     - a Tegra-identity condition for GB10-class devices;
     - GH200's integrated attribute.
  5. At adoption:
     - the pool-mode setting;
     - removal of the dead trim arm, the comparison arms and the study
       diagnostics (design note "Settings and constants");
     - the corrected close fix shipping with 0.8.27 (ruling 26).

## Owner rulings (2026-10-05)

The owner ruled on the § 11 questions on 2026-10-05; the plan records the
same rulings. They amend this protocol as follows, and the affected sections
are updated to match.

1. **Python import hook: yes.** The Tegra wheel gains an import-time `cuInit`
   and pool hook behind `tegra-pool-experiment`, so Python runs every variant
   of § 2.1 (A-load-hold and A-load-release included). It is designed and
   built in Phase 2; Phases 0 and 1 do not depend on it.
2. **Private pool (P): deferred,** built only if it is the shape the cudarc
   maintainer would prefer. § 10 gains an assessment, from cudarc issues and
   maintainer comments (#536, #594, #544, #106, #174, #194), of whether a
   private pool or the set-current-pool shape is more upstream-compatible.
3. **Goal:** a working pool that recovers stream-ordered speed becomes the
   default on aarch64 Linux, with the synchronous fallback as the safety net.
   The performance gate is read as recovering default-pool (fast-path) speed,
   with S's synchronous path as the floor to beat; the decision table
   (§ 8.1 item 12) names which arm, if any, can be the default.
4. **Soak:** 15–30 minute R6 runs only, until the arms are shown correct,
   robust and fast; the 8 h and 24 h soaks are withdrawn and re-planned
   later.
5. **8 GiB for main testing.** C3 stays at `maxSize` ≤ 8 GiB; the 16 GiB
   exhaustion cell is not run unless the owner rules again.
6. **Host-memory bound (question, not yet ruled).** Whether the product needs
   a "circuit breaker" so a pool cannot hog the shared host memory (`maxSize`
   cap, release threshold, trimming). `pool_capacity.c` (§ 4.5) therefore also
   samples `MemAvailable` before the pool exists, at the first error, after
   freeing and synchronizing, and after `cuMemPoolTrimTo(0)`, beside the
   pool's reserved memory at the same points; the Phase 1 results report how
   `maxSize` and the threshold bound real host-memory use.

## Owner rulings (2026-10-06) and revision 2

The owner ruled on the Phase 1 decisions on 2026-10-06. These rulings
supersede the 2026-10-05 rulings where they differ (ruling 2 above is
replaced by ruling 7; the load-time half of ruling 1 is replaced by ruling
10). This revision rewrites § 1.1, § 1.3, § 2, § 5, § 7 and § 10 item 6 to
match. Phases 0–1 results: `dev/plans/runs/0.8.28-pool-study/results.md`.

- **Ruling 7.** **Primary design: a private pool (variant P-first-use).** An explicit pool
  created with `cuMemPoolCreate` and **never installed**. FathomDB allocates
  from it with `cuMemAllocFromPoolAsync` and frees with `cuMemFreeAsync`;
  the device's current pool and every other CUDA user in the process are
  untouched. It is created lazily at the first CUDA use. Installing a
  device-level pool (B and A-first-use) is kept only as a comparison arm.
  A coexistence test is added (C9, § 7): `cuDeviceGetMemPool` is unchanged
  across engine open/close, and another CUDA user in the same process still
  gets the default pool. The cudarc change takes the upstream-compatible
  shape: a per-context field fixed at construction, opt-in, with no global
  table, environment variable or cfg inside cudarc (§ 2.2). The pool-shape
  assessment of § 10 item 6 is written into this revision.
- **Ruling 8.** **Swap is monitored and recorded, not a general stop.** `SwapFree` and
  the zram counters are recorded per process and per series (§ 1.1). Only
  series that need a no-swap condition stop on swap growth; § 1.1 names
  them and gives the reason.
- **Ruling 9.** **Sizes.** The working `maxSize` is **3 GiB** (about 1 GiB usable, about
  1 GiB of contiguous address space). **2 GiB** is the minimum size that
  passes the capacity criterion. **8 GiB** is the host-memory cap for
  exhaustion and capacity testing, not a product pool size.
- **Ruling 10.** **Load-time forms dropped.** A-load-hold and A-load-release are removed
  from the study: P1 measured +92 ms / +109 MiB and +126 ms / +19 MiB at
  import, against an import-cost gate of S + 5 ms / + 32 MiB (the plan's
  "Design note: load-time pool creation" gives the rationale and the
  revisit trigger). The Python import-time hook of ruling 1 therefore
  carries early `cuInit` only, no pool.
- **Ruling 11.** **Tracking id (§ 11 question 6) resolved.** `TC-1c70e523-38a0-4bb7-b238-0307d2b0489f`
  exists on branch `llm/slice117-jetson-node-cuda-plan` (commit
  `e978ee516`) and reaches `release/0.8.27` when that branch merges. No
  ledger action.

### Revision 3 (2026-10-06): independent-review findings

Revision 2 was reviewed independently on 2026-10-06 (PASS-WITH-FIXES).
Revision 3 applies every finding:

- **Host-memory bound (circuit breaker, ruling 6).** Four falsifiable rows,
  CB1–CB4 (§ 7), each a separate clause of the decision table (§ 8.1 item 12)
  and of the decision rule (§ 8.2): the cap, trimming after close, the typed
  error at the cap, and no CPU move after it.
- **Zero-length requests (§ 2.2).** The P path never synthesizes a null
  pointer upstream; any special case stays in FathomDB's residual patch.
- **R5 reach (§ 7, § 6.2).** A heap pilot first finds the heap size at which
  the largest unmapped hole falls below 1 GiB; R5 adds boundary cells there,
  states its detection power, runs the comparison arms on Node 25 only, and
  interleaves variant blocks.
- **Swap (§ 1.1).** `VmSwap` is recorded at every measurement point, memory
  is reported as `VmRSS` + `VmSwap`, and R6's latency drift under swap is
  flagged, not stopped. The harness takes the swap baseline at series start
  and stops only rows flagged no-swap.
- **Fail closed (§ 2.2).** Once a private-pool context exists, a later
  failure to build one is a typed error, never a fallback.
- **C9 observables, the context-free `cuMemPoolCreate` hypothesis, and the
  low-severity corrections** in § 2.2, § 2.3, § 4.2, § 6.2, § 7 and § 10.

## Owner rulings (2026-10-06, after Phase 1b)

The owner ruled on the six decisions in the Phase 1b results
(`dev/plans/runs/0.8.28-pool-study/results.md` § 10.11). Protocol and plan
are amended to match.

- **Ruling 12: early `cuInit` is part of P.** P-first-use is a candidate
  only together with early `cuInit` at module load (Node now; Python once
  its import hook exists). The pool lands inside the early-`cuInit` driver
  reservation (results § 10.7), and a late `cuInit` fails at large heaps
  before any pool is attempted.
- **Ruling 13: C9 pass rule.** The current pool never equals the private
  pool, and it equals the default pool whenever both reads succeed. An
  `OOM`-to-success transition of the current-pool read is the default
  pool's own lazy behaviour and is recorded, not failed. New cell C9b: a
  co-resident library's default-pool success (its 20960 MiB contiguous
  need) with P-first-use against S, at heaps 0, 1M and 4M, about 10
  processes each.
- **Ruling 14: trim is conditional.** Trimming the private pool on close or
  on idle is an **experimental arm** (`FATHOMDB_POOL_TRIM`), never the
  default, until experiments show it is safe. Its tests: trim while the
  embedder and reranker singletons still hold slices; an embed running
  concurrently with trim (from the worker thread and from Node workers);
  reopen after trim; repeated open/close/trim cycles; the latency cost of
  trim plus re-grow; no crash, no use-after-free and an identical embed
  hash. Adoption is a later ruling.
- **Ruling 15: a dedicated error kind for pool exhaustion,** shaped like the
  existing error taxonomy (Rust error types, the napi typed envelope and
  its kind strings, Python exceptions, `dev/interfaces/`), implemented
  experiment-gated. The interface-doc wording is drafted in the study
  evidence only; a public change needs an interface-doc and ADR update at
  adoption.
- **Ruling 16: the witness is off in pass/fail rows.** Witness results get
  their own row.
- **Ruling 17: no-swap series tolerate a swap rise of up to 1 MiB.** Every
  numeric literal in the policy, the vendored patch, the napi hook, the
  tests and the harness is audited and classified as a measured platform
  fact (never product logic), a product default (one named, documented,
  overridable place) or a test tolerance (one harness config,
  overridable); the inventory is a Phase 2 deliverable.

## Owner rulings (2026-10-06, after Phase 2) and revision 5

The owner ruled on the Phase 2 decisions in the results
(`dev/plans/runs/0.8.28-pool-study/results.md` § 11.14). Protocol and plan
are amended to match.

- **Ruling 18: C2 close retention is real.** `Engine::close()` stopped the
  workers, but the live `Engine` and the projection runtime each kept a
  reference to the model. The owner's fix (`c816b8653`, "release embedder
  after close drains workers") is cherry-picked onto the study branch for
  the study only. P cannot become the default until that fix ships. C2 is
  rerun on a build that includes it (P, A, B and S; 50 cycles; no garbage
  collection). GC-based workarounds stay out of pass rows.
- **Ruling 19: C7 is not yet ruled.** First, investigate the cause of the
  exit crashes after a co-resident primary-context reset: native
  backtraces of S and P, and the first error after the reset. If the
  frames point at the study's own pool teardown, a study-gated guard is
  allowed. No other product change is made for C7.
- **Ruling 20: CB2 is amended** to "no wholly free chunk stays reserved
  after close and idle". Threshold 0 meets this. The trim arm is dropped
  from the candidate design; its evidence stays as a negative result.
- **Ruling 21: CB1.**
  - The primary check is the pool's own `reserved_high` ≤ its `maxSize`.
  - A system-level measure is a sanity check only, and only when the
    device reports `CU_DEVICE_ATTRIBUTE_INTEGRATED` = 1 at runtime.
    Otherwise it is skipped with a reason and never fails.
  - It must tolerate page-cache and other-process noise.
  - Its thresholds are named constants (`harness/cb1check.py`), and its
    skip and classification logic has pure tests.
- **Ruling 22: rerun the timing comparisons** affected by the fixed-order
  interleave defect in Phases 3–4, with the fixed harness: the
  equivalence control, P1 and the others.
- **Ruling 23: error kind.**
  - Python gets a dedicated `CudaPoolExhaustedError`, a subclass of
    `EmbedderError`, as in TypeScript.
  - The cross-encoder's forward is classified too: under forced CUDA,
    `RerankerDevicePolicyError::CudaPoolExhausted`, kind
    `cuda_pool_exhausted`.
  - Experiment-gated.
- **Ruling 24: platform detection at runtime.**
  - The policy reads `CU_DEVICE_ATTRIBUTE_INTEGRATED`, pool support and
    total device memory. The `cfg` gate only limits compilation.
  - The pool is sized from device memory with a floor and a ceiling;
    3 GiB on the 64 GB Orin is accepted.
  - The sizing for other devices is an analysis, not a decision
    (results § 12.4).
  - Only the runtime detection and the sizing function are implemented,
    behind the experiment feature, with pure tests for each case.
- **Ruling 25 (2026-10-06, during Phase 4): allocation correctness and
  release rank above latency.** The Phase 4 report leads with them:
  - every requested allocation that fits succeeds;
  - the pool's reserved memory returns to its baseline after release or
    close;
  - caps are respected;
  - exhaustion is typed;
  - no host-memory hogging.

  Latency against the S-sync floor and the default-pool target follows.
  Every P-to-default-pool ratio states the Node reference's n, and is
  labelled underpowered when n < 15. Python S processes that take the
  default pool form a second default-pool reference. It is reported
  separately, never pooled with Node, because the binding overheads
  differ.

Phase 4 timing runs on the build that includes the close fix, so the
comparisons describe the code that would ship.

## Owner rulings (2026-10-07, after Phase 4) and revision 6

The owner ruled on the Phase 4 decisions in the results
(`dev/plans/runs/0.8.28-pool-study/results.md` § 12.11). The owner listed
ten items against the numbers 26–34. They are recorded here as rulings
26–35, one per item, so that each keeps its own number.

- **Ruling 26: adopt the corrected close fix.**
  - On `release/0.8.27` (merge `96796fe04`) the fix is two commits:
    - `2ff744b06`, the test, which is red on `c816b8653`;
    - `8247d91a4`, which drops the released embedder after the close lock.
  - Both are cherry-picked onto the study branch after `f0b6b4c7e`. Pushed
    history is not rewritten; the two commits complete the fix.
  - `c816b8653` alone deadlocks when a caller-supplied embedder's `Drop`
    re-enters `Engine::close`.
  - C2 and a Phase 4 spot check are rerun on the corrected build.
- **Ruling 27: C7 stays unruled.** On the corrected build, a small reset
  probe records where the crash now lands: 3 S + 3 P under gdb, 5 + 5
  without. No guard and no cudarc change.
- **Ruling 28: the rerank floor is re-based on the default pool's own
  speed-up.**
  - P's rerank speed-up over S-sync must be within the same 1.15× of the
    default pool's speed-up over S-sync. Equivalently, P / default-pool
    rerank time ≤ 1.15, with its interval stated.
  - Reason: the default pool itself reaches only 1.49× over S-sync on
    rerank, so the old 1.5× floor tested the reference, not P.
  - The results report both the old 1.5× reading and the re-based one.
- **Ruling 29: references accepted.** The pooled Node default-pool
  reference plus the Python reference are accepted. There is no Node 26
  top-up.
- **Ruling 30: release threshold 0 is the default.** The owner approved
  this outright (2026-10-07 follow-up), not only accepted it.
  - Threshold `max` gave no latency gain: embed 1.044 [0.938, 1.134], and
    no difference in R8.
  - It held more memory: 288 MiB against about 48 MiB of spare after
    close, and 96 MiB more after idle.
  - On integrated memory, pool-held memory is host RAM that other processes
    cannot use, and ruling 25 ranks release above latency.
  - Caveat: threshold 0 could cost re-mapping on workloads that churn large
    allocations across synchronization points. None was seen up to batch
    128 and 8 concurrent processes.
- **Ruling 31: the sizing table is accepted** as proposed in results
  § 12.4:

  | Device class | Pool |
  | --- | --- |
  | 64 GB Orin | 3 GiB |
  | 32 and 16 GB Orin | 2 GiB, after an on-device check |
  | 8 GB Orin | off |
  | Thor | opt-in until measured |
  | discrete GPU | off |
  | GH200 | verify the integrated attribute |
  | GB10 | add a Tegra-identity condition |

  Which parameters are named constants and which are runtime settings is
  determined in the plan's design note "Settings and constants (ruling
  31)". The owner accepted that split for FathomDB (2026-10-07
  follow-up). The settings stay in FathomDB, never in cudarc: the cudarc
  maintainer accepts no environment variables there. How to shape the
  upstream cudarc patch is a standing note,
  `dev/plans/0.8.28/prework/cudarc-upstream-patch-notes.md`.
- **Ruling 32: the module-level singletons are documented shipped
  behaviour.**
  - The corrected fix releases only the engine-owned embedder.
  - Two singletons live until the process exits:
    - the static `OnceLock` `CandleBgeEmbedder` in the napi and Python
      `embedding.rs`;
    - the reranker singleton.
  - A release API is a later item.
- **Ruling 33: typed exhaustion everywhere at adoption.**
  - The module-level `rerank()` string path gets typed.
  - The Python `CudaPoolExhaustedError` subclass ships beyond the
    experiment.
  - Requirement: the `cuda_pool_exhausted` kind is present and tested in
    all three SDKs (Rust, Python, TypeScript) on every path, including
    module-level `rerank` and `embed`.
- **Ruling 34: CB1's primary check is the pool's own reserved counter.**
  The system-level check is a sanity check on integrated GPUs only.
  Reasoning:
  - `MemAvailable` has a 442 MiB bimodal noise band from S's own behaviour
    in `perf` mode.
  - `VmRSS` does not see pinned pool memory.
  - cgroup `memory.max` does not bound CUDA allocations on Tegra.

  So only the pool counter is exact.
- **Ruling 35: early `cuInit` at module load is accepted** for this round
  of work. It is marked for further discussion with the owner.

## 0. Conventions

- `<worktree>`: the study checkout. `<scratch>`: the session scratch
  directory. `<evidence>`: `dev/plans/runs/0.8.28-pool-study/` in the
  worktree (§ 9). Committed files never contain a real home path or user
  name; redact to `<worktree>` and `<scratch>` before `git add` (§ 9.3).
- **GPU lock.** Every GPU series runs under
  `flock <scratch>/gpu.lock <command>`, the same discipline as Slice 110
  (`fix-verification/run-series.sh`, `early-cuinit-verification/run-series.sh`
  both say so in their headers). The lock is held for a whole series, not per
  process. R7 (concurrent processes) holds the lock in its one launcher
  process. Nothing else uses the GPU while a series runs.
- **Fresh process per run.** One Node, Python or C process per data point.
  The study never reuses a process across runs, because every effect under
  study (address-space layout, pool creation, the once-per-device decision)
  is a process-lifetime effect.
- **Host and artifact record.** Each series directory holds
  `series-header.txt` with: UTC start, L4T release
  (`/etc/nv_tegra_release`), kernel, `cuDriverGetVersion`, Node version,
  Python version, `nvpmodel -q` output, `jetson_clocks --show` output,
  the sha256 of the `.node`, wheel and C binaries used, the variant, the
  pool parameters and the full environment filtered to `FATHOMDB_*`,
  `CUDA_*`, `NODE_OPTIONS`.
- **No sudo.** `nvpmodel` and `jetson_clocks` are read, never changed. If
  the power mode changes between the start and end of a series, the series is
  discarded and rerun.

## 1. Preconditions, safety and stop conditions

### 1.1 Host-quiet check (before every series, recorded per run)

A run starts only when all of the following hold; the runner records the
values and refuses otherwise:

1. No process named `cargo`, `rustc`, `nvcc`, `pytest`, `maturin`,
   `Runner.Worker`, or another `node`/`python` running a FathomDB consumer, is
   alive (`ps -eo args`). The Slice 110 runners counted these as `busy_procs`
   (`explicit-pool-experiment/run-series.sh.txt`); here they are a refusal.
2. 1-minute load average below 2.0 on this 12-core host.
3. `MemAvailable` at least 40 GiB (`/proc/meminfo`). Swap is **recorded,
   not required to be zero** (owner ruling 8): `SwapTotal − SwapFree` and,
   per zram device, `/sys/block/zram*/mm_stat` (original, compressed and
   used bytes) are recorded before and after every process and at the start
   and end of every series, and every summary reports the per-process and
   per-series deltas. Swap already in use at a series start is its baseline.
   Each process also records its own `VmSwap` beside `VmRSS` at every
   measurement point (§ 4.2), and process memory is reported as
   `VmRSS` + `VmSwap`, so memory swapped out of a process is not read as
   memory returned.
   **No-swap series** stop (and are discarded) if swap use rises above the
   series-start baseline during the series:
   - every timing comparison: the Phase 0 equivalence control, P1–P7 and
     the latency half of R8. Reason: a swap-out or swap-in inside a measured
     interval adds page-fault latency to that interval, so a variant that
     happens to trigger reclaim would be measured as slow. (The R5 run during
     which the 2026-10-06 swap event appeared took 3.4 s against about 2.1 s
     for its neighbours; that is one co-occurrence, a correlation, not a
     measured cause.) R6's latency-drift criterion is not stopped: a soak
     that sees swap growth records it, and its drift result is flagged
     *swap-affected* in table 9;
   - none other. Pass/fail, capacity and address-space series (the revisit
     check, C1–C9, C3, the gap sweep, R1–R5, R7) record swap and continue: a
     swap-out cannot turn a driver refusal into a success or the reverse, and
     R7 reports swap as part of its memory-sharing result.
4. GPU idle: `tegrastats` reports `GR3D_FREQ 0%` for 3 consecutive seconds.
5. Thermal zones (`/sys/devices/virtual/thermal/thermal_zone*/temp`) are
   recorded per run and reported; no threshold is enforced, but timing
   comparisons are made within interleaved blocks (§ 6.1) so that slow
   drift affects every variant alike.

### 1.2 Memory safety on the shared 64 GB unified memory

The host's desktop, this session and the GPU share one 64 GB DRAM
(`free -g`: 61 GiB total, about 51 GiB available at the time of writing).
Every probe that allocates real memory is bounded:

- **Pool exhaustion probes (C3)** allocate until the pool refuses, so they can
  reach `maxSize`. They run only for `maxSize` ≤ 8 GiB, only when
  `MemAvailable` ≥ 2 × `maxSize` + 16 GiB, one at a time, under
  `timeout 300`. The 16 GiB `maxSize` exhaustion cell of C3 is not run
  (owner ruling 5: 8 GiB for main testing); 16 GiB is characterised by pool
  creation only, and the cell is marked *not exhausted*.
- **Concurrent processes (R7)** are launched by one runner that samples
  `MemAvailable` every second and sends `SIGTERM` to every child if it falls
  below 8 GiB, recording the trial as *aborted-for-safety* (a result, not a
  failure of the variant).
- **Soak (R6)** runs with `ulimit -v` unset (CUDA needs the 61 GiB virtual
  reservation) but with a watchdog that stops the process if RSS exceeds
  4 GiB or `MemAvailable` falls below 16 GiB.
- Every Node and Python run is wrapped in `timeout 600`; every C probe in
  `timeout 120`.
- Release threshold `max` keeps freed memory in the pool. In R7 that is
  bounded by the workload's reserved high (160 MiB per process at 3 GiB
  `maxSize` in `explicit-pool-experiment/analysis.txt`), so eight processes
  hold about 1.3 GiB; this is a measured expectation, and R7 records the
  actual sum.

### 1.3 Stop conditions

Stop the study, write up what was measured and consult the owner when:

- any variant crashes a process (signal, not a typed error) at any point:
  that is a correctness failure and a probable wrong-API free; collect the
  core pattern and stop that variant;
- the production-equivalence control (§ 5, Phase 0) fails: the experiment
  build does not reproduce S, so nothing measured with it is about the
  product;
- the same unexplained failure repeats twice in a cell (the repository's
  retry budget, `AGENTS.md` § 5);
- swap use rises during a no-swap series (§ 1.1 item 3; that series only),
  the power mode changes, or `MemAvailable` falls below the floors above.

## 2. Variants

### 2.1 Definitions

All variants keep the production synchronous path as the last resort and keep
Node's early `cuInit` at module registration
(`src/rust/crates/fathomdb-napi/src/cuda_early_init.rs`) as shipped. "Pool"
means one explicit pool per device per process created with
`cuMemPoolCreate` (pinned device allocation, `maxSize` as configured, release
threshold as configured), probed with one 4-byte allocation, freed and
synchronized, and never destroyed. What differs is whether it is installed
as the device's current pool (`cuDeviceSetMemPool`).

| Variant | Role | Binding | `cuInit` | Pool created | Installed? | Default pool used? | FathomDB's allocator |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **S** | baseline | Node, Python | Node: at registration. Python: at first use (Phase 2 adds an import-time `cuInit`, ruling 1) | never | — | yes, if available | production rule: default pool available → stream-ordered, else synchronous (`third_party/cudarc-0.19.7/src/driver/safe/core.rs`, `async_alloc_decision`) |
| **P-first-use** | **primary** (ruling 7) | Node, Python | as S | immediately before the first Candle CUDA device of the process | **never** | not by FathomDB; the device's default pool stays the current pool for everyone else | `cuMemAllocFromPoolAsync` from the private pool, `cuMemFreeAsync`; § 2.2. If creation or the probe fails, the production rule (as S) |
| **A-first-use** | comparison | Node, Python | as S | as P-first-use | yes | no | the installed-pool rule of § 2.2 |
| **B** | comparison | Node, Python | as S | as P-first-use, **only if** `cuDeviceGetDefaultMemPool` fails with `CUDA_ERROR_OUT_OF_MEMORY` | yes | yes, if available | the installed-pool rule of § 2.2 |

Notes on the definitions:

- **Dropped:** A-load-hold and A-load-release (ruling 10). Their P1 results
  stay in the Phases 0–1 results; nothing later runs them. The plan's
  "Design note: load-time pool creation" records why and what would reopen
  them.
- P-first-use, A-first-use and B create the pool at the same moment with the
  same `cuMemPoolCreate` call, so the fragmentation exposure of lazy creation
  (R5) is the same for all three: creation needs `maxSize`/3 of contiguous
  free address space, inside the driver's `cuInit` reservation or as a new
  mapping (Phases 0–1 results, § 3.2). They differ in what the process looks
  like afterwards. P leaves the device's current pool alone. A-first-use and
  B change it for every CUDA user in the process. B also queries, and so may
  lazily map, the default pool first.
- P-first-use does not prefer the default pool when it is available: every
  FathomDB allocation comes from the private pool, so its speed, memory
  bound and behaviour are the same in every process. Whether a "default if
  available, else private" hybrid is worth adding is decided after Phase 4,
  and only if P-first-use's speed falls short of the default-pool reference.
- Python's import-time hook (ruling 1) carries early `cuInit` only (ruling
  10), so Python runs the same four variants as Node from Phase 2 on.

### 2.2 The allocator decision

**P-first-use (primary; the upstream-compatible shape, ruling 7).** The
choice is a per-context field fixed at construction and opt-in. cudarc gets
no global table, environment variable or cfg for it:

- `CudaContext` gains a field holding an optional `Arc<CudaMemPool>`. It is
  set only through a new opt-in constructor (working name
  `CudaContext::new_with_mem_pool(ordinal, pool)`) and is immutable
  afterwards. Upstream itself fixes the async/sync choice at construction
  "so we only have to run it once" (chelsea0x3b/cudarc#174).
- A context with a pool reports `has_async_alloc() == true` and
  `alloc_mode() == AllocMode::Private`. `CudaStream::alloc` (and every path
  that reaches it: `alloc_zeros`, `clone_htod`, `null`) calls
  `cuMemAllocFromPoolAsync(.., pool, stream)` instead of `cuMemAllocAsync`.
- `CudaSlice::drop` is unchanged: `cuMemFreeAsync` frees memory from any
  pool. There is no new `CudaSlice` variant (the maintainer's objection in
  chelsea0x3b/cudarc#594: "CudaSlice is almost turning into an enum at this
  point"). A slice keeps its stream, the stream keeps its context, and the
  context keeps the pool, so the pool outlives every slice allocated from it.
- Such a context never queries the default pool and never reads or writes
  the 0.8.27 process-wide decision table.
- Zero-length requests: what `cuMemAllocFromPoolAsync` does with 0 bytes is
  measured in C5 before the path is written. If it returns a null pointer,
  as `cuMemAllocAsync(0)` does, nothing is added. If it returns an error,
  the upstream-shaped P path **returns that error** and does not synthesize
  a null pointer: the maintainer declined exactly that in
  chelsea0x3b/cudarc#194 ("I'm hesitant to use a null pointer"). Any
  zero-length special case FathomDB then needs stays in FathomDB's residual
  vendored patch, marked as such, and is listed in § 10 item 6 as not
  proposed.
- Moving buffers: a pointer from a P context that is upgraded
  (`upgrade_device_ptr`) into a synchronous context of the same device would
  reach `cuMemFree`. Upstream's safety section already makes the matching
  allocator the caller's duty; the vendored doc of `upgrade_device_ptr` is
  rewritten to say that a stream-ordered pointer (from any pool, the private
  one included) may be upgraded only into a stream-ordered context.
  FathomDB builds only P contexts in a P process (C1 checks this).
- **Fail closed (C1).** The first FathomDB device of the process fixes the
  process's mode: *private* if a private-pool context was built, otherwise
  *production*. Before any P context exists, a failed pool creation, probe
  or first P context build falls back to the production rule (the process is
  then pure S). Once any P context exists, a later failure to build one
  returns a typed error and never falls back to `CudaContext::new` or a
  synchronous decision, because a process mixing P and S contexts is not
  allowed. This rule is a pure function in the policy with its own unit
  test.
- **cuBLAS and cuRAND.** Candle's `CudaBlas` and `CudaRng` allocate their
  workspaces through the CUDA runtime or library calls, not through
  cudarc's `CudaStream::alloc`, so they stay outside the private pool. C4
  bounds that residual.
- **Candle.** Candle builds its context inside `Device::new_cuda`. The
  pinned fork (`coreyt/candle-fathomdb` at the rev in the root
  `Cargo.toml`) has exactly two `CudaContext::new` sites, in
  `candle-core/src/cuda_backend/device.rs` (`CudaDevice::new_with_stream`
  and `BackendDevice::new`). It gains `CudaDevice::from_context(Arc<CudaContext>)`
  and `Device::new_cuda_from_context`, which share the rest of the
  constructor. For the study the change is applied as a local path override
  on the study branch only, and the exact diff is kept under
  `<evidence>/patches/` so a fork commit can follow. FathomDB's
  `new_cuda_device` builds the context with the pool and passes it in. The
  alternative, a setter on a live context, is rejected: it would make the
  allocator depend on when the setter ran relative to the first allocation,
  which is exactly the failure the per-device table exists to prevent.
- The policy retains the primary context, creates the pool with
  `CudaMemPool::create`, probes it with one 4-byte `cuMemAllocFromPoolAsync`
  / `cuMemFreeAsync` / `cuCtxSynchronize` on that context, keeps it in a
  `static OnceLock` and never installs it. *Hypothesis:* `cuMemPoolCreate`
  needs no current context; C5 tests it, and until it is shown the policy
  keeps the retain. If creation or the probe
  fails, every site builds its device as S does (production rule).

**A-first-use and B (comparison arms; experiment-only, not proposed
upstream).** The rule built and used in Phases 0–1 (vendored cudarc
`select_async_alloc`, `INSTALLED_POOLS`):

1. `CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED` ≤ 0 → synchronous (`sync`).
2. If a pool was installed on the device through `CudaMemPool::install` in
   this process, it is still the device's current pool, and a 4-byte
   `cuMemAllocAsync` / `cuMemFreeAsync` / `cuCtxSynchronize` on the null
   stream succeeds → stream-ordered (`explicit`). The default pool is not
   queried.
3. Otherwise the production rule: `cuDeviceGetDefaultMemPool` succeeds →
   `default`; `CUDA_ERROR_OUT_OF_MEMORY` or `CUDA_ERROR_NOT_SUPPORTED` →
   `sync`; any other error fails the constructor.

The decision is recorded once per device per process in `AllocModeByDevice`.
A process that never installs a pool makes exactly the 0.8.27 driver calls
(Phase 0 equivalence: PASS). The installed-pool table is a global table, so
it is study instrumentation, not part of the upstream proposal.

### 2.3 Where the code lives

**Vendored cudarc** (`third_party/cudarc-0.19.7`, experiment branch only):

- Built in Phase 1: `src/driver/safe/mem_pool.rs` (`CudaMemPool::{create,
  install, attribute, trim_to, raw}`, `MemPoolProps`). `Drop` destroys the
  pool, and destroying an installed pool reverts the device to its default
  pool. `max_size` is set only under `cuda-12020` and later. Also the
  comparison-arm rule of § 2.2 and `CudaContext::alloc_mode()`.
- To build (Phase 1b, test first): the per-context pool field, the opt-in
  constructor, `AllocMode::Private`, and the `cuMemAllocFromPoolAsync`
  branch of the allocation path. The upstream proposal (§ 10) is
  `CudaMemPool` without `install`'s table, plus these per-context pieces.
- No environment variables, no new `target_arch` cfg, no printing.

**Candle fork:** a `CudaDevice` / `Device` constructor from an existing
`Arc<CudaContext>`. The pin moves on the study branch only.

**FathomDB** (`src/rust/crates/fathomdb-embedder/src/cuda_pool_policy.rs`,
compiled only with `tegra-pool-experiment` on aarch64 Linux with
`embed-cuda` or `rerank-cuda`):

- `FATHOMDB_POOL_VARIANT` is `S` | `P-first-use` | `A-first-use` | `B`
  (unset means `S`; an unknown value, including the dropped A-load names,
  aborts the process). `FATHOMDB_POOL_MAXSIZE` defaults to `3G` (ruling 9).
  `FATHOMDB_POOL_RELEASE_THRESHOLD` is `0` | `max`.
- `new_cuda_device(site, ordinal)` is called at all five product sites
  (`candle_bge.rs`: `embedder-probe`, `embedder-witness`, `embedder-load`;
  `candle_reranker.rs`: `reranker-probe`, `reranker-load`). For P it creates
  and probes the private pool before the first device, then builds every
  device on a pool context. For A-first-use and B it installs the pool
  before the first device, as in Phases 0–1.
- The Phase 1 load-time hook (`install_pool_at_load`, called from the napi
  registration hook) is removed with the A-load variants.
- Reporting is unchanged: `fdb-pool-exp event=<install|decide|teardown|stats>`
  lines, now with `alloc_mode=<default|explicit|private|sync>` and, for P,
  `installed=0`. With `FATHOMDB_POOL_COEXIST_CHECK=1` (the C9 series only)
  the install and teardown lines for P also record the pair
  (`CUresult`, handle) of `cuDeviceGetMemPool`, and, when it succeeds, of
  `cuDeviceGetDefaultMemPool`. C9 passes (ruling 13) when the current-pool
  handle never equals the private pool's and equals the default-pool handle
  whenever both calls succeed; a change from an error to success between
  the reads is recorded as the default pool's lazy behaviour. *Hypothesis:* reading the current pool can lazily
  create the default pool (a new large mapping); the C5 probe diffs
  `/proc/self/maps` around the first `cuDeviceGetMemPool` to settle it. If no
  mapping appears, the environment gate is dropped and every P run records
  the pairs.

**Keeping it out of production.** The feature `tegra-pool-experiment` is not
in any feature set of `scripts/release/cuda-artifact-contract.sh`
(`CUDA_NAPI_FEATURES`, `CUDA_RERANK_NAPI_FEATURES`, `CUDA_PYTHON_FEATURES`,
`CUDA_RERANK_PYTHON_FEATURES`) and is added to the build commands of § 3 only.
The study branch is never merged to a release branch; a product
implementation, if ruled, is a separate 0.8.28 slice written to the ruling.
Every new build repeats the Phase 0 equivalence control: the experiment build
with `FATHOMDB_POOL_VARIANT=S` against the production build.

## 3. Build recipe (this host)

All builds run from `<worktree>`. Never `pip install -e` or
`maturin develop` from a worktree (`AGENTS.md` § 10). Record every artifact
hash in `<evidence>/artifacts.sha256`.

**Environment** for every CUDA build:

```sh
export PATH=/usr/local/cuda-12.6/bin:$PATH
export CUDA_PATH=/usr/local/cuda-12.6
export CUDA_COMPUTE_CAP=87
export LIBRARY_PATH=/usr/local/cuda-12.6/targets/aarch64-linux/lib
```

**Node CUDA addon** (Node 25.9.0 via nvm; `receipt.md` records the same
recipe and the `PATH` requirement for `nvcc`):

```sh
source ~/.nvm/nvm.sh && nvm use 25.9.0
cd <worktree>/src/ts && npm ci
npm exec -- napi build --platform --release \
  --cargo-cwd ../rust/crates/fathomdb-napi \
  --features embed-cuda,rerank-cuda,tegra-pool-experiment --js false
npm run build   # dist/ for the in-tree consumer
sha256sum fathomdb.linux-arm64-gnu.node
```

The **production build** for the Phase 0 equivalence control is the same
command without `tegra-pool-experiment`; keep both `.node` files, named by
hash. The **installed form** is staged with
`dev/plans/runs/0.8.27-slice-110-tegra/fix-verification/stage-installed.sh
<worktree> <scratch>/staging-<hash> <scratch>/consumer-<hash>`, which packs
the thin main and the `linux-arm64-gnu` platform package and installs them
offline with lifecycle scripts disabled.

**Tegra Python wheel:** `scripts/release/build-python-cuda-tegra.sh --out
<scratch>/wheels` builds with the contract's `CUDA_PYTHON_FEATURES`. For the
experiment wheel, the engineer appends `,tegra-pool-experiment` to that
exported value with an uncommitted local edit of the contract script, records
`git diff scripts/release/cuda-artifact-contract.sh` into
`<evidence>/harness/wheel-features.diff`, and reverts it before any commit.
Install into a scratch venv: `python3 -m venv <scratch>/venv-<hash> &&
<scratch>/venv-<hash>/bin/pip install --no-index <wheel>`. Slice 110 used the
same pattern (`receipt.md`, "Tegra Python wheel").

**C probes** (from `driver-isolation-evidence/results.md`, "Build"):

```sh
gcc -O2 -Wall -Wextra -std=gnu11 -I/usr/local/cuda-12.6/include pool_va_repro.c -o pool_va_repro -lcuda -lpthread
gcc -O2 -Wall -I/usr/local/cuda/include pool_lifecycle.c -o pool_lifecycle -lcuda
gcc -O2 -Wall -I/usr/local/cuda/include pool_teardown.c -o pool_teardown -lcuda
gcc -O2 -Wall -I/usr/local/cuda/include pool_capacity.c -o pool_capacity -lcuda
gcc -O2 -Wall -I/usr/local/cuda/include pool_gap.c -o pool_gap -lcuda
```

Build time is part of the Phase 0 budget (§ 12); the Slice 110 receipt does
not record it, so the engineer records it.

## 4. Harnesses

### 4.1 Reused as-is

| File (under `dev/plans/runs/0.8.27-slice-110-tegra/`) | Use |
| --- | --- |
| `driver-isolation-evidence/pool_va_repro.c` | Layout driver: `--at` (replayed blockers), `--random N --seed S`, `--pre-reserve`, `--pool-maxsize`, `--maps-dir`. R1 and the gap sweep. |
| `driver-isolation-evidence/minimal_repro.c`, `minimal_repro_runtime.c`, `cuinit_repro.c` | Reproducers for the upstream and NVIDIA packages (§ 10) and the revisit check of the plan's upstream section. |
| `pool-teardown-evidence/pool_teardown.c`, `run-variants.sh` | C7 baseline, extended with `--reset` (§ 4.6). |
| `explicit-pool-experiment/pool_lifecycle.c` | C2's destroy-while-current probe (its section D) and the churn benchmark (section E). |
| `explicit-pool-experiment/replay_layout.py`, `window_layout.py` | Turn a Node `/proc/self/maps` snapshot into `--at` blockers; classify a snapshot's window layout. |
| `driver-isolation-evidence/mapdiff.py`, `gaps.py` | Diff maps snapshots; largest reservation / largest hole per run. |
| `fix-verification/stage-installed.sh` | Installed-package staging (§ 3). |

### 4.2 `pool-consumer.mjs` (new; merges the Slice 110 consumers)

Derived from `explicit-pool-experiment/consumer.mjs` (timings, embed hash,
rerank scores, `HEAP_OBJECTS`, `MAPS_DIR`) and
`early-cuinit-verification/consumer.mjs` (`FATHOMDB_MODULE`, `IMPORT_ORDER`,
`CONSUMER_MODE`, RSS/VmSize from `/proc/self/status`). One process, one JSON
line on stdout; stderr is captured whole by the runner and the
`fdb-pool-exp` lines are parsed from it.

Inputs (environment): `FATHOMDB_MODULE` (in-tree `dist/index.js` path or
`fathomdb` when installed), `FATHOMDB_DB_SCRATCH`, `IMPORT_ORDER`
(`early`|`late`), `HEAP_OBJECTS` (live objects before open),
`HEAP_GROW_AFTER_OPEN` (objects to add between the first and second
measurement block, for R3), `CONSUMER_MODE` (`import` | `open` | `full` |
`perf` | `ingest` | `cycles`), `WARMUP_ITERS` (default 5), `TIMED_ITERS`
(default 50), `BATCH_SIZES` (default `1,8,32,128`), `INGEST_DOCS` (default
10000), `CYCLES` (open/close repetitions, default 50), `MAPS_DIR`, plus the
device policy variables, `FATHOMDB_GPU_ALLOCATION_WITNESS`, and the
`FATHOMDB_POOL_*` variables of § 2.3, which the runner sets.

Steps by mode: `import` imports and reports; `open` opens and reports; `full`
is the Slice 110 sequence (open, report, embed ×11, rerank ×2, close);
`perf` adds the warm-up and timed blocks for single embed, each batch size
(`embedBatchCls`), and rerank; `ingest` writes `INGEST_DOCS` documents in
batches of 100 through `engine.write` with the default embedder on; `cycles`
opens and closes the engine `CYCLES` times, embedding once per cycle. With
`IDLE_AFTER_CLOSE_S=<n>` (CB2), every mode that opens an engine closes it,
waits `n` seconds and exits, so the exit `teardown` line carries the pool's
reserved memory after close and idle. With `OVERSIZE_BATCH=<n>` (CB3, CB4),
`full` mode embeds one batch of `n` long passages after the first embed,
records the error (or success), checks that a following single embed still
reports `embedderDevice` `cuda`, and continues.

JSON fields recorded per process (all modes write every field, `null` when
not applicable):

```text
outcome, failedStep, error{name,code,kind,message}
node, variant, maxSize, releaseThreshold, order, preloaded, consumerMode
pid, heapObjects, heapUsedMiB, heapGrowAfterOpen
mem{startVszMiB, importRssDeltaMiB, importVszDeltaMiB, afterOpenRssMiB,
    afterOpenVszMiB, endRssMiB, endVszMiB,
    points{<point>: {rssMiB, swapMiB, vszMiB}}}  # every measurement point; VmSwap beside VmRSS
timingsMs{import, open, embedFirst, embedSteady[], embedBatch{1:[],8:[],32:[],128:[]},
          rerankFirst, rerankSteady[], ingestTotal, cyclesOpen[], cyclesClose[]}
embedderDevice, embedderReason, rerankerDevice
witness{deltaBytes, floorBytes, outcome}       # only when the witness is on
embedSha, rerankScores[]
ingest{docs, docsPerSecond}
allocMode                                      # parsed by the runner from stderr: default|explicit|private|sync|none
poolEvents[]                                   # parsed fdb-pool-exp lines, as objects
host{memAvailableKiB, swapFreeKiB, loadAvg1, thermalMilliC[], gpuIdleBefore}
```

The runner (§ 4.4) merges `allocMode`, `poolEvents` and `host` into the JSON
after the process exits, so the consumer stays a pure client.

### 4.3 `pool_smoke.py` (new; extends `fix-verification/python_smoke.py`)

Same `fragment` option (three blockers at 38, 68 and 98 GiB before import),
plus `--heap N` (allocate N Python objects before import, for a Python
analogue of R2), `--mode full|perf|cycles|reset`, and `--reset` which, after
the first embed, calls `cuDevicePrimaryCtxReset(0)` (or `cudaDeviceReset`
with `--reset runtime`) through `ctypes` on `libcuda.so` / `libcudart` and
then embeds and writes again (C7's co-resident reset). Output fields mirror
§ 4.2 in snake_case.

### 4.4 `run-series.sh` and `run-matrix.sh` (new)

`run-series.sh <variant> <binding:node|python> <runtime-version> <count>
<outdir> [consumer args]` runs `count` fresh processes one at a time, each
after the host-quiet check of § 1.1, writing `run-NNN.out/.err/.log`,
`run-NNN.host.json`, and a one-line summary. It is derived from
`early-cuinit-verification/run-series.sh` and adds the host record, the
`timeout`, and the `fdb-pool-exp` parsing.

`run-matrix.sh <matrix.tsv> <outdir>` executes a matrix of cells. A cell is
one line: criterion, variant, binding, runtime version, layout, `maxSize`,
threshold, consumer mode, count. For timing criteria (P1–P7) the matrix is
run in **randomised blocks**: each block runs one process of every cell in a
random order, blocks repeat `count` times, and the block order seed is
recorded (`shuf --random-source=<(yes <seed>)`). For pass/fail criteria the
cells run as series. Interleaving is what protects timing comparisons from
thermal drift, GPU rail-gating and page-cache state; the Slice 110 series
were run as blocks per variant and are therefore exposed to drift between
blocks.

### 4.5 `pool_capacity.c` (new; from `pool_lifecycle.c` section C)

Measures capacity directly instead of inferring it from where the product
fails.

Arguments: `--layout control|3pages|at LIST`, `--maxsize MiB`, `--threshold
0|max`, `--chunk MiB` (allocation granularity: 1, 16, 64, 256), `--single`
(bisect the largest single `cuMemAllocAsync` that succeeds from an empty
pool). Steps: blockers; `cuInit`; retain primary context; create pool;
install; (a) allocate `--chunk` MiB chunks until the first non-success,
touching each with `cuMemsetD8Async`; record count, bytes, `CUresult`, and
the pool's `RESERVED_MEM_CURRENT/HIGH`, `USED_MEM_CURRENT/HIGH`; (b) free
everything, synchronize, repeat (a) once to show recovery; (c) with
`--single`, bisect the largest single allocation that succeeds; (d) record
`cuMemGetInfo` before and after (sole consumer; informational). One
`RESULT` line: `layout maxsize_mib chunk_mib threshold allocated_mib
first_error reserved_high used_high single_max_mib recovered_mib
mem_free_before mem_free_after`.

### 4.6 `pool_gap.c` (new; from `pool_va_repro.c --pre-reserve` and the s4c method)

Measures the contiguous address-space need of an explicit pool as a function
of `maxSize`, instead of inferring `maxSize`/3 from one layout.

Method, from `driver-isolation-evidence/results.md` ("Threshold series"):
five blockers before `cuInit` force the driver's reservation into 15.34 GiB
quarters; then, after context creation, `cuMemAddressReserve` fences leave
exactly one unmapped hole of `G` bytes inside the window and fill every other
hole with 4 GiB-spaced blockers (the `pool_teardown.c` hole-filling step).
Arguments: `--gap MiB`, `--maxsize MiB`, `--in-unit` (place the hole inside a
driver unit with fences, as s4d/s4e did, instead of between units), `--maps-dir`.
Output: `RESULT gap_mib maxsize_mib in_unit create_rc set_rc probe_rc
new_mmap_bytes` where `new_mmap_bytes` is the size of any mapping added
between the pre-create and post-create maps snapshots.

`pool_teardown.c` gains `--reset primary|runtime`, which calls
`cuDevicePrimaryCtxReset` or `cudaDeviceReset` after the pool is created
and used, then retries `cuDeviceGetMemPool`, `cuMemAllocAsync` and allocation
from the explicit handle. (C7.)

### 4.7 `soak-consumer.mjs` and `soak-watch.sh` (new)

Loop until `SOAK_SECONDS`: embed (single and batch 32), rerank, write a batch
of 50 documents, every 60 s allocate and drop 200k JS objects and force a GC
(`--expose-gc`), every 60 s append a line to `soak.jsonl` with wall time, RSS,
VmSize, heapUsed, the last 60 s median embed and rerank latency, and the
`stats` line fields from `FATHOMDB_POOL_STATS_EVERY_S=60`. `soak-watch.sh`
samples `tegrastats` and `/proc/meminfo` every 10 s and enforces § 1.2.

### 4.8 `concurrent-runner.sh` (new)

`concurrent-runner.sh <variant> <k> <threshold> <outdir>` starts `k`
`pool-consumer.mjs` processes in `perf` mode within 100 ms of each other with
the witness **off** (`FATHOMDB_GPU_ALLOCATION_WITNESS` unset), records each
exit, and runs `tegrastats --interval 1000` for the trial's duration. The
witness is off because it reads a system-wide `cuMemGetInfo` counter and
requires a sole GPU consumer
(`src/rust/crates/fathomdb-embedder/src/gpu_witness.rs`,
`SOLE_GPU_CONSUMER_PRECONDITION`; `receipt.md` records two `insufficient_delta`
witness failures caused by a concurrent cargo test). A witness failure in R7
would be a harness artefact, not a variant failure.

### 4.9 `analyze.py` (new; from `explicit-pool-experiment/analyze_pool.py`)

Reads every `run-*.log` and `*.host.json` under a results tree and emits the
tables of § 8 as Markdown and TSV, with the statistics of § 6.

## 5. Phase order and the Phase 0 controls

Phases are ordered to front-load the measurements most likely to rule a
variant out. A variant that fails a Phase 1 gate is dropped from later phases
except where the plan needs its number for a comparison.

| Phase | Content | Rule-out it can produce |
| --- | --- | --- |
| 0 | Builds; harness smoke; production-equivalence control; `--random-seed` layout check; warm-cache check (§ 6.6) | the experiment build is not the product |
| 1 (done, except R5) | C3 capacity and the gap sweep (§ 4.5, § 4.6); P1 import cost of A-load-hold and A-load-release (both failed; ruling 10) | done: capacity = ceil32(`maxSize`/3), need = `maxSize`/3 |
| 1b (revisions 2–3) | (1) Harness: the swap baseline at series start and per-row no-swap flag (§ 1.1), `VmSwap` at every point, the CB consumer options (§ 4.2). (2) The C5 probe (`pool_c5.c`): `cuMemAllocFromPoolAsync(0)`, `cuMemPoolCreate` without a current context, the maps diff around the first `cuDeviceGetMemPool`, and the private-pool cap rows (typed OOM, `cuCtxSynchronize` afterwards, recovery, trim) that are the unit-testable halves of CB1–CB3. (3) Pure tests first: the cudarc decision for a pool context (`AllocMode::Private`, no default-pool or table query), the policy's fail-closed rule (M3), the variant parser without the A-load names. (4) The P-first-use cudarc and Candle changes and the policy; the C9 device test. (5) The Phase 0 equivalence control on the new build. (6) The heap pilot (§ 7). (7) R5 per the revised matrix (§ 7) | lazy private-pool creation fails in a heap cell; P breaks coexistence; a zero-length or cap error is not typed |
| 2 | C1, C2, C5, C6, C7, C8, C9; the Python import-time `cuInit` hook (ruling 1) | any correctness failure |
| 3 | R1, R2, R3, R4, R7, R8, then R6 soak on the surviving candidate(s) | robustness failure; memory-sharing failure |
| 4 | P2–P7 on the survivors against the default-pool reference (target) and S's synchronous path (floor) | performance gate failure |
| 5 | Analysis, results document, decision mapping, upstream package (§ 10) | — |

**Phase 0 controls:**

1. **Production equivalence** (repeated for every new experiment build).
   10 `full` runs each of the production `.node`
   and the experiment `.node` with `FATHOMDB_POOL_VARIANT=S`, Node 25, small
   consumer, unobstructed. Pass: identical embedding hash and rerank scores,
   the same set of steps passed, and steady-embed medians per path within
   10 %. The experiment build must also show `allocMode` consistent with the
   inferred path (bimodal latency) in every run; this is the one place the
   old inference is used, to validate the new direct reading.
2. **V8 layout reproducibility** (*hypothesis*: Node's `--random-seed` seeds
   the generator V8 uses for its mmap address hints, so two processes with
   the same seed and the same program produce the same in-window layout).
   20 import-only runs, seeds 1–10 twice, `MAPS_DIR` set; compare the
   `[8 GiB, 128 GiB)` mapping start addresses of the pairs with
   `replay_layout.py`. If ≥ 9 / 10 pairs are identical, the R2–R5 cells run
   with `--random-seed=<n>` for `n` = 1..count, the same seeds in every
   variant, giving **paired** layouts across variants. If not, R2–R5 run
   unpaired and the report says so. Either way the seeds used are recorded.
3. **Warm cache.** One throwaway `full` run before every series so that the
   model files are in the page cache; it is recorded and excluded. The open
   times in `explicit-pool-experiment/analysis.txt` cluster at about 1.1,
   1.4 and 2.0 s across otherwise identical runs; this protocol records the
   GPU idle state (`gpuIdleBefore`, from `tegrastats` in the 3 s before
   launch) and the page-cache state (`fincore`-style `vmtouch` is not
   installed; use `/proc/meminfo` `Cached` delta) to attribute that, and
   interleaves variants so it cannot bias a comparison.

## 6. Measurement methods

### 6.1 Timing

- Timing points are `performance.now()` around each awaited call in the
  consumer (Node) or `time.perf_counter()` (Python), as in Slice 110.
- Per process: `WARMUP_ITERS` discarded, `TIMED_ITERS` kept, for single
  embed, each batch size and rerank. The **unit of analysis is the process**:
  iterations within a process are correlated, so each process contributes
  one median per measure.
- Summary per cell: median and IQR of the per-process medians, and the count.
- Comparison: the ratio of cell medians, with a 95 % percentile bootstrap
  confidence interval from 10 000 resamples of processes within each cell.
  When Phase 0 establishes paired layouts, R2–R5 pass rates are compared
  paired (McNemar-style discordant counts) and reported alongside.
- Path-conditional reporting: every timing table is reported **by
  `allocMode`** (`default`, `explicit`, `private`, `sync`) as well as by variant,
  because S is a mixture (in Slice 110's unobstructed small-consumer runs the
  default pool was available in 1 / 10, 6 / 30 and 5 / 10 processes;
  `explicit-pool-experiment/analysis.txt`, "default-pool availability"). The
  plan's performance gate (owner ruling 3) is evaluated as recovery of the
  default-pool reference's speed, with S's synchronous-path runs (the path S
  takes in most Node processes) as the floor every arm must beat.

### 6.2 Proportions

- Pass rates, path fractions and pool-creation success rates are reported
  with Wilson 95 % intervals.
- Zero failures in N runs gives a 95 % upper bound of about 3/N on the
  failure rate (the rule of three): 30 runs → 10 %, 100 → 3 %, 300 → 1 %.
  Each table states the bound its N gives. A per-cell N of 30 bounds that
  cell at 10 %; the plan's 1 % bound applies only to the pooled row.
- **Detection power per cell.** With 30 runs in a cell, a true failure rate
  of 5 % produces at least one failure with probability 1 − 0.95³⁰ = 78 %,
  and a true rate of 2 % only 1 − 0.98³⁰ = 45 %. A clean 30-run cell
  therefore does not show that a rare failure is absent; R5 places extra
  P-first-use runs at the boundary cells (§ 7), where the rate is expected
  to be highest.
- **The discriminating endpoint for the pool variants is the stream-ordered
  fraction, not the pass rate.** S passes behind an early import (100 / 100
  in `receipt.md`) by falling back to the slow path; a pool variant is
  better only if its `allocMode` is `default`, `private` or `explicit` in
  more processes at equal pass rate. R2–R5 therefore report, per cell: pass rate, and the
  fraction of passing processes on each `allocMode`, both with intervals.

### 6.3 Memory

- Process memory: `VmRSS`, `VmSwap` and `VmSize` from `/proc/self/status`
  at the consumer's measurement points (§ 4.2), reported as
  `VmRSS` + `VmSwap`. Import deltas use the same method
  as `early-cuinit-verification/consumer.mjs`.
- Pool memory: `CU_MEMPOOL_ATTR_RESERVED_MEM_{CURRENT,HIGH}` and
  `USED_MEM_{CURRENT,HIGH}` from the `fdb-pool-exp` lines (install, each
  decide, teardown, and `stats`).
- System memory: `MemAvailable`, `Cached`, `CmaFree` from `/proc/meminfo`
  and `tegrastats` RAM, before and after each process.
- `cuMemGetInfo`: used only through the product's witness and only in
  single-process series. It is a shared, system-wide counter on this
  integrated GPU and the nvmap page pool absorbs allocations before they are
  charged (`dev/tegra-platform-reference.md` § 7.2, § 7.4;
  `gpu_witness.rs`). It is never used as a per-variant memory measure.

### 6.4 Which path a process took

Read from `allocMode` (§ 2.3), never inferred from latency after Phase 0.
Every passing process must have exactly one `decide` event per device; two
events with different `alloc_mode` in one process is a C1 failure.

### 6.5 Pool capacity (C3) and contiguous need (R5)

- Capacity: `pool_capacity.c` at `maxSize` ∈ {192 MiB, 384 MiB, 1 GiB,
  2 GiB, 3 GiB, 4 GiB, 8 GiB} (16 GiB per § 1.2), chunk ∈ {1, 16, 64, 256}
  MiB, layouts {control, 3pages}, threshold {0, max}, 5 runs per cell. The
  Slice 110 inference "usable capacity ≈ `maxSize`/3" rests on two product
  failures (192 MiB pool: 63.5 MiB used high; 384 MiB: 128.0 MiB;
  `explicit-pool-experiment/analysis.txt`). *Alternative hypothesis*: the
  product's failures were single large allocations (model weights at open,
  cross-encoder weights at the first rerank) that did not fit the remaining
  reservable space, and the apparent 1/3 is a granularity effect. The chunk
  sweep separates these: a cap independent of chunk size supports the 1/3
  rule; a cap that moves with chunk size or with `--single` does not. Fit
  `capacity = a·maxSize + b` on the measured points and report the residuals;
  the plan's C3 pass is "predicts within 5 % at every size".
- Contiguous need: `pool_gap.c` with gaps G ∈ {1, 2, 4, 8, 12, 16} GiB and
  `maxSize` swept in 1 GiB steps up to 48 GiB (3 runs per point, stop at the
  first failure plus two confirmations). The Slice 110 inference
  "need ≈ `maxSize`/3" comes from one layout (largest gap 14.66 GiB:
  creation succeeded to 44 GiB and failed from 46; control to 48 GiB;
  `driver-isolation-evidence/results.md`, "Explicit pools"). The control's
  own 48 GiB ceiling is unexplained by a gap rule (its reservation is
  61.36 GiB) and is reported as a separate finding (*hypothesis*: a driver
  cap related to device memory). The output is a table need(G) and the
  largest `maxSize` that is safe for the smallest gap the R5 heap cells
  produce.

### 6.6 Confounds and how each is handled

| Confound | Handling |
| --- | --- |
| Host load, other GPU users | § 1.1 refusal; GPU lock; one process at a time outside R7 |
| Thermal and clock drift, GPU rail-gating | Randomised blocks for timing (§ 4.4); `gpuIdleBefore` recorded and the first-after-idle effect reported separately (Slice 110: +110 ms on the first `cuInit` after idle, `receipt.md`) |
| Page cache / model files | Warm-cache run per series (§ 5) |
| Witness's system-wide counter | Witness on only in single-process series; off in R7 and R6 |
| V8 heap randomness | Paired seeds if Phase 0 confirms them; otherwise larger N and Wilson intervals; layouts replayed into C with `replay_layout.py` for deterministic R1 cells |
| Node version | 24.15.0, 25.9.0, 26.10.0 as separate cells; never pooled across versions in a gate |
| In-tree vs installed | Installed form for every Node series (Phases 0–1 did so because file copies were unavailable; it is kept so all phases are comparable) |
| Mixture in S | Path-conditional tables (§ 6.1) |
| Experiment build vs product | Phase 0 equivalence control |

## 7. Run matrix

Sizes (ruling 9): every product-path row runs at the working `maxSize`,
**3 GiB**; 2 GiB (the smallest passing size) is added only where a 3 GiB
cell fails, to show whether the smaller size passes; 8 GiB is the cap for
the C3 capacity and exhaustion rows only. Threshold {0, max} where a row
says so, otherwise 0. "Pool variants" means P-first-use (primary),
A-first-use and B (comparison), in both bindings from Phase 2 on.
Layouts: *unobstructed* (small
consumer), *3pages* (the three 4 KiB blockers; in Node via a tiny preload
that maps them before import is not possible from JS, so 3pages is a C and
Python layout only), *heap-N* (import first, then N live objects: 0, 100k,
400k, 1M, 4M), *late-N* (N objects, then import), *replayed* (C blockers at
a captured Node layout's mapping starts).

| Row | Variants | Binding / runtime | Layouts | `maxSize` × threshold | Runs | Phase |
| --- | --- | --- | --- | --- | --- | --- |
| Phase 0 equivalence | S (prod), S (exp) | Node 25 | unobstructed | — | 10 + 10 | 0 |
| Phase 0 seeds | S (exp), import-only | Node 25 | heap-400k | — | 20 | 0 |
| C3 capacity | C probe | — | control, 3pages | 7 sizes × 2 thr × 4 chunks | 5 per cell = 560 | 1 |
| Gap sweep | C probe | — | forced quarters + one gap | 6 gaps × ≤48 sizes | ≤ 3 per point, early stop | 1 |
| P1 import cost (done) | S, A-load-hold, A-load-release | Node 25, import-only, installed | unobstructed | 3 GiB × 0 | 20 each, interleaved | 1 |
| Phase 0 equivalence, new build | S (prod), S (exp, P build) | Node 25 | unobstructed | — | 10 + 10, interleaved | 1b |
| C5 / CB unit probe | `pool_c5.c` (private pool, never installed) | C | control, 3pages | 3 GiB × {0, max}; cap rows at 192 MiB and 3 GiB | 10 per cell | 1b |
| Heap pilot | P build, `import` mode, `MAPS_DIR` | Node 25, installed | heap-0/100k/400k/1M/4M, then 8M, 16M, 32M, … doubling until the median largest unmapped hole in [8, 128) GiB is below 1 GiB, V8's heap limit is reached, or the process RSS would pass 8 GiB | — | 10 per heap size | 1b |
| R5 lazy creation | **P-first-use**; A-first-use and B as comparison | P-first-use: Node 24/25/26; A-first-use and B: Node 25 only. Installed, `full` mode, witness on, unpaired. On Node 25 the three variants run in interleaved randomised blocks (one process per variant and heap cell per block, `interleave.sh`) | heap-0/100k/400k/1M/4M, plus the two **boundary cells** from the pilot: the first heap size whose median largest hole is below 1 GiB, and the size before it | 3 GiB × 0; 2 GiB × 0 only in a cell where 3 GiB fails | 30 per cell. P-first-use: 3 Node × 7 heaps × 30 = 630; A-first-use and B: 2 × 7 × 30 = 420; total 1050 (the runs saved by the Node-25-only comparison arms go to the P boundary cells) | 1b |
| C9 coexistence | P-first-use (A-first-use and B as contrast) | vendored cudarc device test; Node `cycles` mode with `FATHOMDB_POOL_COEXIST_CHECK=1`; Python with a `ctypes` co-resident user | unobstructed; 3pages (C, Python) | 3 GiB × 0 | test suite; Node 10 processes × 50 cycles; Python 20 processes | 1b (test), 2 |
| C1 provenance | all | Node 25; vendored unit tests on every target; the policy's fail-closed pure test | unobstructed, heap-400k | 3 GiB × 0 | 50 per variant | 2 |
| CB1 cap | P-first-use (A-first-use and B recorded) | every P2–P5 `perf` and `ingest` process, and every R5 `full` process | as the row | 3 GiB × 0 | as the row | 1b (R5), 4 |
| CB2 trim after close | P-first-use | Node 25, `full` mode with `IDLE_AFTER_CLOSE_S=10` | unobstructed, heap-400k | 3 GiB × {0, max} | 20 per threshold × layout | 2 |
| CB3 cap error, CB4 no CPU move | P-first-use (A-first-use and B as contrast) | Node 25, `full` mode with forced CUDA and `OVERSIZE_BATCH=128` long passages | unobstructed | 3 GiB × 0 | 20 per variant | 2 |
| C2 lifetime | all pool variants | C probe (`pool_lifecycle.c` D) + Node `cycles` mode | 3pages (C); unobstructed (Node) | 3 GiB × 0 | C: 20; Node: 10 processes × 50 cycles per variant | 2 |
| C5 zero-length | all | vendored tests + embedder zero-element tests, both bindings | — | 3 GiB | test suites | 2 |
| C6 multi-device | all | pure unit tests | — | — | tests | 2 |
| C7 resets | S, P-first-use, A-first-use, B | C (`pool_teardown.c --reset`) 20 per case; Python `--reset` 20 per variant | 3pages, free-gap | 3 GiB × 0 | 20 × cases | 2 |
| C8 parity | S, P-first-use, A-first-use, B | Python wheel and installed Node 25 | unobstructed, fragment/3pages (Python) | 3 GiB × 0 | 20 per binding × variant | 2 |
| R1 synthetic | pool creation in C (the same call for P, A-first-use and B) | C probe | 3pages; random-18 seeds 1–100; n-sweep n = 3, 5, 6, 7; free-gap | {2, 3} GiB | ≥ 300 per size | 3 |
| R2 real heaps | survivors + S | Node 24/25/26 | heap-0/100k/400k/1M/4M | chosen size × chosen thr | 30 per cell; ≤ 3 variants × 450 | 3 |
| R3 growth in use | survivors + S | Node 24/25/26 | heap-0 then `HEAP_GROW_AFTER_OPEN`=1M | chosen | 30 × 3 per variant | 3 |
| R4 late import | survivors + S | Node 25; with and without `node --import` | late-400k, late-1M | chosen | 30 per cell | 3 |
| R7 concurrency | survivors | Node 25, `perf` mode, witness off | unobstructed | chosen size × {0, max} | 10 trials × k ∈ {2, 4, 8} × 2 thr | 3 |
| R8 threshold | survivors | Node 25, `perf` then 60 s idle, then `teardown` stats | unobstructed | chosen × {0, max} | 20 per thr | 3 |
| R6 soak | surviving candidates, 15–30 min each (owner ruling 4) | Node 25, 2 processes sequentially or k = 2 concurrently with witness off | unobstructed | chosen | 15–30 min | 3 |
| P1–P4, P6 | survivors, S-sync (floor), default-pool reference (target: S runs with `allocMode=default`, plus B's runs with `allocMode=default`, which use the same pool) | Node 24/25/26, `perf`, randomised blocks | unobstructed | chosen | 20 per variant × version. Expected default-path n: Slice 110's availability (1 / 10 to 5 / 10) gives about 4–10 of each 20 S runs, so 12–30 over three versions before B's runs are added. If the pooled reference has fewer than 15 processes, one more 20-run S block per version is added; the reference's n is stated beside every ratio | 4 |
| P5 ingest | survivors, S | Node 25, `ingest` 10 000 docs | unobstructed | chosen | 5 per variant | 4 |
| P7 Python | P-first-use, A-first-use, B, S | wheel, `perf` and ingest | unobstructed | chosen | 20 per variant | 4 |
| R9 | — | other boards | — | — | not available on this host; declared unmeasured | — |

"Chosen size" is **3 GiB** (ruling 9): capacity 1024 MiB, 3.8 times the
Phase 1 workload high-water mark (272 MiB used at batch 128 with rerank),
and a contiguous need of 1 GiB. 2 GiB (capacity 704 MiB) is the smallest
size whose capacity is at least twice that mark. If R5 shows a heap cell
where 3 GiB cannot be created, the 2 GiB cell is run there and the finding
goes to the owner; the size does not change without a ruling. "Chosen
threshold" is fixed after R8.

The 35 A-first-use R5 processes measured in Phase 1 (Node 24, heap 0, 3 and
16 GiB, all created) used the pre-revision build. They are reported
separately and not pooled with revision-2 R5 cells.

**Circuit-breaker rows (ruling 6; CB1–CB4).** Each is a separate clause of
the decision table and the decision rule:

- **CB1 cap (revision 5, ruling 21).**
  - **Primary check:** in every process of the row, the pool's
    `reserved_high` ≤ its `maxSize`. The C3 capacity, 1024 MiB at 3 GiB, is
    reported beside it on the measured device class.
  - **Sanity check:** only when the `install` event reports
    `integrated=1`. It compares the process's `MemAvailable` drop
    (before open → after rerank) with S's synchronous-path processes in the
    same series, using the named thresholds and noise band of
    `harness/cb1check.py`.
  - The sanity check reports consistent, inconsistent, inconclusive or
    skipped (with a reason). It never fails the row.
  - Phase 2 showed that per-process `VmRSS` does not see the pool's pinned
    memory, so it is recorded but is not the check.
- **CB2 trim (revision 5, ruling 20).** After `engine.close()` and 10 s
  idle, no wholly free chunk stays reserved at threshold 0. The exit
  `teardown` line shows `reserved_cur` within the 32 MiB chunks that still
  hold live slices (the module singletons'). Threshold `max` is recorded.
  The trim arm is not part of the candidate.
- **CB3 cap error.** An oversized batch under forced CUDA at 3 GiB returns a
  typed FathomDB error (its `kind` recorded), the next embed in the same
  process succeeds (so the context was not left with a sticky error;
  `pool_c5.c` checks `cuCtxSynchronize` directly), and `embedderDevice`
  stays `cuda`.
- **CB4 no CPU move.** In the same run, the embed after the error reports
  `embedderDevice` `cuda` and the same embedding hash as before the error.

## 8. Analysis and reporting

### 8.1 Tables

1. Environment and artifacts (one table, from the series headers).
2. Phase 0 equivalence: per build, pass count, hash identity, steady medians
   by path; the inferred-vs-read path agreement count.
3. Capacity: per `maxSize` × chunk × threshold × layout, allocated MiB at
   first error, `used_high`, `reserved_high`, single-max; the fitted model
   and residuals; the verdict on the 1/3 rule.
4. Contiguous need: need(G) and the safe `maxSize` per gap; the control
   ceiling.
5. R5: the heap pilot (median and range of the largest unmapped hole per
   heap size, and the boundary found); per variant × Node × heap × size,
   pool-creation success (Wilson), path fractions, the maps-derived largest
   gap per run, and the detection power per cell (§ 6.2); the predictive
   rule and its misses.
6. P1: import ms and RSS/VmSize deltas per variant with intervals; the
   import-cost gate verdict per A-load sub-variant (done in Phase 1; both
   failed).
7. Correctness: one row per C-row (C1–C9) with sample sizes, failures, and
   the typed error kinds seen; for C9, the device's current pool before and
   after each open/close and the co-resident user's pool.
8. Robustness R1–R4, R7, R8: pass rate, path fractions, intervals, and the
   3/N bound stated; R7 memory sums; R8 held memory versus latency.
9. R6: per-minute RSS, pool reserved, median latencies; drift against the
   high over the first 5 minutes (soaks are 15–30 min, ruling 4).
10. Performance P2–P7: by variant and by `allocMode`, medians, IQR, ratio
    against S-sync and against the default-pool reference with bootstrap
    intervals; the gate verdicts.
11. Output identity: embedding hash and rerank score sets by variant.
12. **Decision table:** one row per plan decision-rule clause, the criteria it
    depends on, and PASS / FAIL / UNMEASURED with the table that supports it.
    CB1, CB2, CB3 and CB4 are four separate rows.
13. Circuit breaker: CB1 per process (`reserved_high`, the
    `VmRSS` + `VmSwap` difference against its baseline), CB2 per process
    (`reserved_cur` after close and idle), CB3/CB4 per process (error kind,
    following embed, device, hash).

### 8.2 Mapping onto the decision rule

The owner's goal (ruling 3) is that a working pool recovering stream-ordered
speed becomes the default on aarch64 Linux; the decision table names the arm
that can be that default, or none. The plan's rule (revision 3) reads: make
**P-first-use** the default if C1–C9 and R1–R8 pass on this host, the
performance gates pass, R5 shows lazy private-pool creation at 3 GiB in
every heap cell including the boundary cells, **with early `cuInit` at
module load** (ruling 12) **and the owner's close fix shipped** (ruling 18), and each circuit-breaker
clause passes on its own: CB1 (cap), CB2 (trim after close), CB3 (typed cap
error) and CB4 (no CPU move). The comparison arms (A-first-use, B) do not ship; their
results say what the installed shape would have cost or gained and inform
the upstream comment. If P-first-use fails only R5 at 3 GiB, the 2 GiB cells
and the owner decide. Otherwise keep the 0.8.27 synchronous fallback. The decision
table (§ 8.1 item 12) is the only input to that rule; narrative in the
results document may explain a row but never override it. A row whose N gives
a bound weaker than the claim it supports is marked PASS (bounded at x %),
never plain PASS.

### 8.3 Results document

`<evidence>/results.md`, with the frontmatter of this file's kind, states
the host, the artifacts, each table, the decision table, what was not
measured, and the open questions. It is the input to a 0.8.28 release-state
ruling; it does not itself rule.

## 9. Evidence directory layout and retention

### 9.1 Committed (curated)

```text
dev/plans/runs/0.8.28-pool-study/
  results.md                      # § 8.3
  env.txt                         # host, driver, toolchain, Node, Python
  artifacts.sha256                # every .node, wheel, C binary used
  harness/                        # pool-consumer.mjs, pool_smoke.py, run-series.sh,
                                  # run-matrix.sh, concurrent-runner.sh, soak-*.{mjs,sh},
                                  # pool_capacity.c, pool_gap.c, analyze.py,
                                  # pool_teardown.c (with --reset), wheel-features.diff
  patches/                        # cudarc mem_pool + decision rule diff; FathomDB policy diff
  matrices/*.tsv                  # the exact cells run, with seeds
  summaries/*.txt                 # analyze.py output per phase
  samples/<series>/               # at most 2 full run logs per series (one pass, one fail if any)
  seeds-and-layouts/              # Phase 0 seed check summary; one maps snapshot per R5 cell
```

Slice 110's retention practice applies (`receipt.md`, "Evidence retained and
omitted"): reproducer sources, analysis scripts, patches, summaries and
samples are committed; raw per-run logs, maps archives and `strace` output
are not.

### 9.2 Untracked (left on the host)

`<scratch>/pool-study/logs/<phase>/<series>/run-*.{out,err,log,host.json}`,
`maps/`, `strace/`, build logs, databases. Keep until the 0.8.28 ruling is
recorded; `results.md` names the host path pattern with `<scratch>`.

### 9.3 Redaction gate

Before `git add`, grep `<evidence>` recursively for the host's home-directory
prefix and for `$USER`; both must print nothing. Scripts refer to `<worktree>` and `<scratch>` as placeholders
and are archived as run records, as Slice 110 did with `*.sh.txt`.

## 10. Upstream track (preparation only)

Nothing in this section is posted by anyone but the owner. The study
assembles the following under `<evidence>/upstream/` so that a comment on
cudarc issue #536 (open, no maintainer reply; plan § "Upstream state") can be
made from facts the maintainer can run:

1. **Minimal reproducer:** `minimal_repro.c` with its 20 / 20 and 0 / 20
   outputs (`driver-isolation-evidence/results.md`), one paragraph on the
   host, no FathomDB internals. The maintainer requires representative
   failing tests or benchmarks (plan § "Upstream style", from issues #555,
   #556, #595).
2. **A test in cudarc's `mod tests` style** in the `safe` layer: create a
   `CudaMemPool` with `maxSize` 1 GiB, create a context with it through the
   opt-in constructor, allocate and free through the ordinary
   `CudaStream::alloc`, read `used_high`, assert the slice round-trips, and
   assert that `cuDeviceGetMemPool` is unchanged (the private shape, ruling 7). It must compile across the `cuda-*`
   features (the `max_size` field under `cuda-12020`+), on Windows and under
   the `no-std` clippy check (plan § "Upstream state"); it must skip cleanly
   when no device is present, as the existing GPU tests do.
3. **A benchmark:** a small program in the style of the vendored crate's
   `examples/` (`third_party/cudarc-0.19.7/examples/01-allocate.rs` and
   siblings) that times alloc/free churn with the default pool, a private pool, an
   installed explicit pool and synchronous allocation, derived from
   `pool_lifecycle.c` section E; run on this Orin and reported with medians
   and intervals.
4. **A draft comment** for #536, written in the maintainer's terms:
   observed failure, the reproducer, the proposed minimal primitive
   (`CudaMemPool` plus a context that allocates from it, chosen at
   construction; no new `CudaSlice` variant, per the maintainer's
   reservation on #594), what FathomDB keeps on its side, and a question on
   preferred shape. No claims about merge timing.
5. **The revisit check** the plan's upstream section asks for: rerun
   `minimal_repro.c` and `pool_teardown.c` against the current L4T/driver
   at the start of the study (Phase 0) and record whether the failure still
   reproduces; if it does not, the study stops and reports that instead.

6. **Pool-shape assessment (owner rulings 2 and 7; written 2026-10-06).**
   Which shape is more upstream-compatible: a private pool (allocate with
   `cuMemAllocFromPoolAsync`; the device's current pool untouched) or the
   set-current-pool shape (`cuDeviceSetMemPool`; existing allocation paths
   unchanged)? Sources were read on 2026-10-06. Quotes are verbatim; readings
   are marked *inferred*.

   | Source | What it says | Bearing |
   | --- | --- | --- |
   | NVIDIA, "Using the NVIDIA CUDA Stream-Ordered Memory Allocator, Part 2" (developer.nvidia.com blog) | "In general, libraries should not change a device's pool, as doing so affects the entire top-level application." A library that needs other properties may "create its own pool and then allocate from that pool using `cudaMallocFromPoolAsync`." Setting the current pool is described as an *application's* choice. On the release threshold, for unknown co-resident processes it advises setting it to zero unless profiling shows allocation is a bottleneck. | Directly favours the private shape for a library such as cudarc or FathomDB. Supports threshold 0 as the default (R8 decides). |
   | chelsea0x3b/cudarc#536 (open, 2026-02-28) | Asks for `cuMemPoolCreate` / `cuMemAllocFromPoolAsync` wrappers and proposes "a safe-level `CudaMemPool` type with Drop" and `CudaStream::alloc_from_pool()`. Its open questions include whether pool-allocated `CudaSlice`s should track their pool. No maintainer reply. | The request is for the private shape. The maintainer has not yet chosen between shapes. |
   | chelsea0x3b/cudarc#544 (merged 2026-03-23) | Result-level `mem_pool::{create, destroy, trim_to, get_attribute, set_attribute, alloc_async}` and `device::{get_default_mem_pool, get_mem_pool, set_mem_pool}`; the maintainer: "Nice pr, ty for opening". Safe wrappers were left for a later PR. | Both shapes' driver calls are already accepted upstream, including `alloc_async` (`cuMemAllocFromPoolAsync`). Neither shape needs new sys or result code. |
   | chelsea0x3b/cudarc#594 (open, "needs redesign", maintainer review 2026-09-22) | A pool-owned `CudaSlice` whose `Drop` does not free. Maintainer: "I'm not sure if this design is the correct way forward. CudaSlice is almost turning into an enum at this point … We should probaly discuss this a bit more." The same comment continues: "I definitely want to support memory pools". The thread also carries a counter-proposal: an `Arc<dyn Any>` owner field on `CudaSlice` (as reported by the 2026-10-06 independent review; not re-read here). | The maintainer resists new `CudaSlice` variants and ownership modes but wants pools. A private pool whose slices are freed with the ordinary `cuMemFreeAsync` adds none. An `alloc_from_pool` returning a pool-tagged slice would. Unknown: whether the owner-field counter-proposal, if adopted upstream, would be preferred to a per-context pool for keeping the pool alive. |
   | chelsea0x3b/cudarc#174 (merged; the runtime async/sync choice) | Maintainer: "I like the runtime detection better than adding an additional compiler flag TBH." The capability is cached in the device/context structure, so detection runs once. | Favours a choice made once and stored on the context, at runtime, without features: a per-context pool fixed at construction fits. |
   | chelsea0x3b/cudarc#106 (closed in favour of #174) | Maintainer (2023-03-29): "Would prefer opt-in to the non-standard behavior, rather than opting out of it." | Pool allocation must be opt-in: a new constructor, not a changed default. |
   | chelsea0x3b/cudarc#194 (zero-length allocation; closed) | Reporter: `cuMemAlloc(0)` returns `CUDA_ERROR_INVALID_VALUE` on a GTX 970 under Candle. Maintainer: "I'm hesitant to use a null pointer" and "We don't have any device specific code in cudarc" (quotes verified by the 2026-10-06 independent review). | No device-specific (`target_arch`) code, and no synthesized null pointer in anything proposed: if C5 shows `cuMemAllocFromPoolAsync(0)` is an error, the upstream shape returns it. |

   **Assessment.** The private shape is the more upstream-compatible one,
   *provided* it is expressed as a per-context opt-in rather than a new slice
   kind. NVIDIA's guidance is explicit that a library should not install a
   device pool. #536 asks for the private shape. #544 already supplies its
   driver calls. #174 and #106 favour a choice made once, at runtime, opt-in.
   The one constraint is #594: no new `CudaSlice` variant and no pool-owned
   slices. The design in § 2.2 meets it, because `cuMemFreeAsync` frees memory
   from any pool. The set-current-pool shape needs the least new code in
   cudarc (`CudaMemPool::install` and nothing else), but it is the shape
   NVIDIA tells libraries to avoid. It also changes behaviour for every other
   CUDA user in the process, the thing ruling 7's coexistence test forbids.
   *Inferred:* a proposal of "`CudaMemPool` plus a context constructor that
   allocates from it" is the shape most likely to be accepted. The
   installed-pool rule built for the comparison arms is not proposed. The
   private-pool variant is therefore built (ruling 7 makes it primary).
   Unknowns: the maintainer has not commented on #536, and whether upstream
   would accept a context constructor or prefer a stream-level setting is
   open. The draft comment (item 4) asks.

   **Not proposed (FathomDB residual patch).** The installed-pool decision
   table of the comparison arms; the aarch64 cfg and the 0.8.27 fallback's
   process-wide table (question 1 of the plan); and any zero-length special
   case of the P path that C5 makes necessary (§ 2.2).

The question whether the 0.8.27 default-pool fallback itself is expressible
with upstream's current API is answered by inspection in Phase 5: list the
calls the fallback needs (`cuDeviceGetDefaultMemPool`, a per-device decision
before the first allocation, the zero-length rule) against `has_async_alloc()`
and the result-level pool functions, and record what residual patch remains.

## 11. Open questions for the owner

Questions 1–5 were ruled on 2026-10-05 and question 6 on 2026-10-06 (see
"Owner rulings" at the top); they are kept here as asked. None is open. What still blocks adoption after Phase 4
and the revision-6 spot check is listed under "Status (2026-10-07)" at the
top.

1. Should the Tegra Python wheel gain an import-time `cuInit` (and, for the
   A-load variants, pool creation) so that Python can run the same variant
   set as Node? Without it, Python's best case is A-first-use or B.
2. Is the deferred private-pool variant (P) wanted at all, given the
   upstream shape? If C7/R7 force it, it adds a cudarc allocation-path change.
3. The performance gate compares against "S": confirm that the S reference
   is the synchronous path (what most Node processes get) and that the
   default-pool reference is a secondary comparison.
4. The 24 h soak costs a GPU-exclusive day; is an 8 h soak per surviving
   candidate plus 24 h on the final candidate acceptable?
5. Whether to run the 16 GiB exhaustion cell at all on this shared host
   (§ 1.2).
6. The todos-ledger id the plan cites for tracking
   (`TC-1c70e523-38a0-4bb7-b238-0307d2b0489f`) is not present in this
   worktree's `dev/todos-and-considerations-ledger.jsonl`; the revisit entry
   `TC-9fef1b7c-4442-4c77-b925-992f338c9aac` is. Confirm the tracking id.

## 12. Time and resource estimate

Per-process wall time, from Slice 110: a `full` Node run is about 2–3 s
(`sync-repair-experiment/analysis.txt`, `wall` column: 2.0–2.7 s); a `perf`
run with 5 + 50 timed iterations at four batch sizes and rerank is estimated
at 15–25 s; `ingest` of 10 000 documents at 10–25 ms per embed is 2–5 min; a C
probe is 1–5 s; an exhaustion probe up to 8 GiB is under 60 s. Builds: the
Node CUDA addon and the wheel each compile Candle's CUDA kernels; budget
30–90 min each on this host (unrecorded in Slice 110; Phase 0 records it).

| Phase | GPU-exclusive time | Notes |
| --- | --- | --- |
| 0 | 3–5 h | two addon builds, one wheel, harness smoke, 40 runs, revisit check |
| 1 | done: about 1 h 55 min (results § 7) | C3 560 probes; gap sweep; P1 80 runs; R5 stopped at 35 processes |
| 1b | build plus about 2.5–3 h | P-first-use build and tests; C5 probe (~0.2 h); equivalence 20 runs; heap pilot (≥ 80 import runs, ~0.3 h); R5 1050 runs × about 5–9 s with the quiet check (measured 4.7 s per process at heap 0; larger heaps take longer) |
| 2 | 4–6 h | C1 500 runs (~1 h); C2 C probes + 40 cycle processes (~1 h); C7 and C8 (~1.5 h); test suites |
| 3 | 10–14 h + soak | R1 600 C runs (~1 h); R2 ≤ 1350 runs (~3.5 h); R3 270 (~1 h); R4 360 (~1 h); R7 60 trials (~1 h); R8 (~0.5 h); R6 15–30 min per candidate (owner ruling 4) |
| 4 | 3–4 h | P1–P4/P6 ≤ 240 `perf` runs (~1.5 h); P5 ≤ 15 ingests (~1 h); P7 60 Python runs |
| 5 | engineer time | analysis, results document, upstream package |

Total: about 30–40 h of GPU-exclusive time plus about an hour of soak, so roughly
one to two calendar weeks with the host shared, if no phase rules a variant
out early. A Phase 1 rule-out (for example, lazy creation failing in a heap
cell for both A-first-use and B) removes most of Phases 3–4 for those
variants and shortens the study to about half.
