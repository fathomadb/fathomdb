---
title: FathomDB 0.8.28 Slice 30 — Tegra private CUDA memory pool, design draft
status: DRAFT (input to S30-T1; decided items filled, traces open)
target_release: 0.8.28
observed_on: 2026-10-07
---

# Slice 30 design draft: Tegra private CUDA memory pool

This is the design agent's starting point for S30-T1 of
`dev/plans/0.8.28/slice-30-tegra-pool-plan.md`. § 1 and § 2 hold what is
already decided; the agent does not reopen them without the owner. § 4 is
open: the agent fills it by tracing the code, as the plan's § 5.2
instructs, before writing the ADR. Every item in § 3 must appear in the ADR
as a stated sentence.

## 1. Decided rule

- **Adoption.** On an aarch64 Linux integrated GPU of a measured class,
  FathomDB creates a private CUDA memory pool (P-first-use) at the
  process's first GPU use, and builds its CUDA context on that pool
  (rulings 7, 12, 38).
  - Release threshold: 0.
  - The device's current pool is never read or changed.
- **Only after early `cuInit`.** The pool is created only if FathomDB's
  module-load `cuInit` ran (ruling 37).
- **Never refuses at the first decision (SD-3).**
  - If `cuInit` was opted out or failed, a gate is off, or pool creation
    fails, the process uses the 0.8.27 synchronous path and records the
    reason.
  - Once a private-pool context exists, a later context-build failure is a
    typed error, never a silent switch of allocator.
- **Built only with `tegra-pool` (SD-5).** The feature is non-default; the
  napi and Python Tegra builds enable it. Without it, behaviour is exactly
  0.8.27's, and the reason is `not_built`.

## 2. Decided parameters

### 2.1 Sizing (ruling 31; SD-2)

Size = total device memory / 20, clamped to [2 GiB, 3 GiB]. The pool is off
(`TooSmall`) when 2 GiB > ¼ of device memory.

| Device class | `auto` | `on` |
| --- | --- | --- |
| AGX Orin 64 GB | 3 GiB | same |
| Orin 32 / 16 GB | off until an on-device C3 + R5 run | 2 GiB |
| Orin 8 GB | off (`TooSmall`) | off (`TooSmall`) |
| Thor | off (unmeasured) | 3 GiB |
| GB10-class, not Tegra | off (Tegra-identity gate) | sized by rule |
| GH200 | off if discrete (verify `INTEGRATED`) | off if discrete |
| Discrete GPU | off (`Discrete`) | off (`Discrete`) |
| No pool support | off (`NoPools`) | off (`NoPools`) |
| Built without `tegra-pool` | off (`not_built`) | off (`not_built`) |

`on` lifts only the Tegra-identity and unmeasured-class gates. `off` always
wins. The measured basis of the floor and ceiling is evidence, not logic:
a pool holds ceil32(`maxSize`/3), measured in 32 MiB chunks on one AGX Orin.

### 2.2 Settings (study design note "Settings and constants")

| Setting | Default | Values | Carrier |
| --- | --- | --- | --- |
| Pool mode | `auto` | `auto`, `on`, `off` | process-level; replaces `FATHOMDB_POOL_VARIANT`; the name is fixed in the ADR |
| `FATHOMDB_POOL_MAXSIZE` | derived (§ 2.1) | bytes, `<n>G`, `<n>M` | process-level |
| `FATHOMDB_POOL_RELEASE_THRESHOLD` | `0` (ruling 30) | `0`, `max` | process-level |

Settings live in FathomDB, never in cudarc. They are read once, before the
first device decision. The precedence of an environment variable over a
module-level configure call, if one exists, is TO DETERMINE (§ 4).

### 2.3 Named constants (not settable; each has a pure test)

`POOL_SIZE_DEVICE_DIVISOR` = 20 · `POOL_SIZE_FLOOR` = 2 GiB ·
`POOL_SIZE_CEILING` = 3 GiB (replaces `DEFAULT_MAX_SIZE`) ·
`POOL_FLOOR_MAX_SHARE_DIVISOR` = 4 · `PROBE_BYTES` = 4 ·
`POOL_EXHAUSTED_KIND` = `"cuda_pool_exhausted"`.

### 2.4 Error kinds

- **`cuda_pool_exhausted`** (ruling 33).
  - Raised on every path: embed, batch embed, engine rerank, module-level
    `embed` and module-level `rerank()`.
  - Present in Rust, Python (`CudaPoolExhaustedError` subclass) and
    TypeScript.
  - The next request is served on CUDA. Exhaustion never moves work to the
    CPU.
- **`cuda_context_lost`** (ruling 36).
  - Raised when a CUDA error coincides with a primary-context id
    (`cuCtxGetId`) different from the one recorded at first context
    creation.
  - Carries both context ids, the driver error and the failed operation.
  - The first detected loss writes one structured diagnostic snapshot:
    `cuDevicePrimaryCtxGetState`, the CUDA libraries loaded per
    `/proc/self/maps`, the pool counters, and the live FathomDB CUDA
    objects.
  - Survival is unsupported in 0.8.28. The fix is 0.8.29 (TC-281155a1),
    and the reset probe is an expected-crash characterization test that
    the fix flips.

### 2.5 Dependencies

- **Candle (SD-1).**
  - `CudaDevice::from_context` is on a `coreyt/candle-fathomdb` branch cut
    from `1aefdd008`.
  - `candle-core-fathomdb`, `candle-nn-fathomdb` and
    `candle-transformers-fathomdb` move to 0.10.3, pinned by rev in the
    workspace and by `=0.10.3` in `fathomdb-embedder`.
  - Push and crates.io publish are HITL.
- **cudarc (ruling 38).**
  - The pool primitive is written upstream-first; the vendored change is
    its backport. The primitive is `CudaMemPool`,
    `CudaContext::new_with_mem_pool`, `AllocMode::Private` and
    `CudaContext::mem_pool()`.
  - Each `FATHOMDB-PATCH.md` item carries an upstream status and a removal
    path.
  - The aarch64 synchronous fallback stays local.
  - The pinned-override gate checks 0.19.7 plus exactly the listed items.
- **Module-level models (ruling 32).** They stay loaded until the process
  exits and hold pool memory. The release call is 0.8.30 B30-01.

## 3. ADR checklist

The ADR is not ready for review until each item is a stated sentence.

1. The adoption rule (§ 1).
2. SD-3: the fallback/refusal boundary.
3. The early `cuInit` states (ran / opted out / failed), how each binding
   records them, Python's new hook and opt-out, and the Slice 117
   import-time `cuInit` item settled the same way.
4. The sizing rule and constants (§ 2.1, § 2.3), with their measured basis
   cited as evidence.
5. The sizing table, including SD-2 and exactly what `on` lifts.
6. The settings: names, defaults, values, carrier, precedence, read time
   (§ 2.2).
7. Release threshold 0, its re-mapping caveat, and why (ruling 30).
8. The error kinds, payloads, SDK classes and raising paths (§ 2.4).
9. The `doctor gpu` and allocation-mode diagnostic fields:
   - chosen path and reason;
   - pool size and threshold;
   - `cuInit` state;
   - context state.

   `doctor gpu` reports without creating a context or a pool.
10. C7: unsupported and documented, what is detected and raised, the
    snapshot, and the 0.8.29 plan.
11. Module-level models (§ 2.5).
12. SD-5: the `tegra-pool` feature, and what a crates.io build gets.
13. SD-1: the Candle fork rev and the crate versions.
14. The cudarc patch items with upstream status and removal path.
15. The gate table, by reference to the plan's § 3.
16. The behaviour changes (§ 5).
17. The § 4 findings: the configuration matrix, the paths and the races,
    with each hazard's test or documented limit.

## 4. Traces (TO DETERMINE by the design agent)

Fill these from the code, with file:line citations, per the plan's § 5.2.
Rows already listed are the minimum. Add rows for anything else found.

### 4.1 Configuration matrix

| Target | Features | Pool mode | `cuInit` | Expected | Actual (file:line) | Test |
| --- | --- | --- | --- | --- | --- | --- |
| aarch64 Linux Tegra, 64 GB | `tegra-pool` + `embed-cuda` | `auto` | ran | private, 3 GiB | | |
| aarch64 Linux Tegra, 64 GB | `tegra-pool` + `embed-cuda` | `auto` | opted out | synchronous, reason `cuinit_opted_out` | | |
| aarch64 Linux Tegra, 64 GB | `tegra-pool` + `embed-cuda` | `off` | ran | 0.8.27 path | | |
| aarch64 Linux Tegra | `embed-cuda` only | any | any | 0.8.27 path, `not_built` | | |
| aarch64 Linux non-Tegra | `tegra-pool` + `embed-cuda` | `auto` | ran | off (Tegra-identity gate) | | |
| x86_64 Linux CUDA | `tegra-pool` + `embed-cuda` | `on` | — | off (`Discrete`), 0.8.27 default pool | | |
| macOS | `embed-metal` | any | — | unaffected; pool settings ignored | | |
| any | no GPU feature | any | — | unaffected | | |
| any | any | invalid value | any | the error or fallback named in the ADR | | |

### 4.2 Execution paths

| Path | Entry | First GPU use | Decision site | Teardown | Notes (file:line) |
| --- | --- | --- | --- | --- | --- |
| Node | module load | `Engine.open` / `embedBatchCls` / `rerank()` | | env teardown, exit | |
| Python | import | `Engine.open` / `embed_batch_cls` / `rerank()`, GIL released | | interpreter finalisation | |
| Rust | — | `Engine::open` / embedder construction | | drop, exit | |
| CLI | `fathomdb doctor gpu` | none: must not create a context or a pool | | — | |

### 4.3 Shared state and races

| Shared state | Writers | Readers | Primitive | Hazard | Test |
| --- | --- | --- | --- | --- | --- |
| Process-wide allocator decision | first GPU user | every later user | | concurrent first use from several threads | |
| Pool settings | decision | decision | | env change after the decision | |
| Early-`cuInit` state | module load | decision, `doctor gpu` | | write/read ordering | |
| Pool-creation result | decision | waiting threads | | failure while others wait | |
| Engine embedder | open, close | embed, rerank | `close_lock` | close racing in-flight work | |
| Slices → stream → context → pool | Candle, cudarc | drop | | drop order at close and at exit | |
| Recorded context id and snapshot flag | first loss | any CUDA error | | two threads detect a loss together | |
| Pool reservation | allocations | exhaustion path | | concurrent exhaustion | |
| CUDA state across `fork()` | — | child process | | Python `multiprocessing` fork start method | |

## 5. Behaviour changes (changelog lines)

- The default allocator on AGX Orin 64 GB in the napi and Python Tegra
  builds is the private pool. Other classes are unchanged unless set to
  `on`.
- Python runs early `cuInit` at import, with an opt-out.
- New error kinds: `cuda_pool_exhausted` and `cuda_context_lost`.
- New settings: pool mode, `FATHOMDB_POOL_MAXSIZE` and
  `FATHOMDB_POOL_RELEASE_THRESHOLD`.
- New `doctor gpu` fields.
- Candle crates move to 0.10.3.
