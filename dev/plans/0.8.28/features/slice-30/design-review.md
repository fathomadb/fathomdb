---
title: FathomDB 0.8.28 Slice 30 — design and code reviews
status: DESIGN REVIEW RESOLVED; CODE REVIEW FINDINGS BEING FIXED
target_release: 0.8.28
observed_on: 2026-10-08
---

# Slice 30 reviews

## Design review (independent Opus subagent, high effort, 2026-10-08)

**Input:** design revision 1 at `b11e4523d`.

**Verdict:** APPROVE-WITH-FIXES. One blocker, eight majors, five minors.
The reviewer confirmed the overall shape: P-first-use, fall back before
the first private context and fail closed after it, no settings in
cudarc, and only the pool primitive ported. Every finding is resolved in
design revision 2.

| F | Sev. | Finding | Resolution (design rev. 2) |
| --- | --- | --- | --- |
| 1 | blocker | Under `auto`, reranker pool exhaustion and context loss become neutral `0.0` scores (`rerank.rs:315,332-337`). Under forced CUDA every forward error is `CudaProbeFailed`. | § 2.4: `score_batch` and `score` classify through `forward_error` first. Pool and context errors propagate under every policy. An AC30-04 test runs under `auto`. |
| 2 | major | Load-time failures are memoized (reranker `OnceLock` → `Unavailable`) or mis-typed (`ModelDeserialize`, `EMBEDDER_NOT_CONFIGURED`, open `Failed`). R30-03's error is unnamed. | § 2.4: typed load variants; open, CLS and `rerank()` load paths typed; the singleton becomes a `Mutex<Option<…>>` that caches only `Loaded`, `DevicePolicy` and `Unavailable`. R30-03's error is `cuda_private_build_refused`. |
| 3 | major | Reranker variants hit the generic `RerankerDevicePolicyError` arm in py, napi and the SDK. SDK `Error::sdk` cannot carry the payload. | § 2.4: explicit arms ahead of the generic ones in all four surfaces, with a mapping test per variant. SDK `Error::cuda_details()`. |
| 4 | major | The zero-length null synthesis contradicts results § 10.2. `CudaSlice::drop` frees null anyway. It diverges from upstream. | § 3.1: no special case; the driver call is kept; the C5 test asserts a null pointer that is freed. |
| 5 | major | `not_built` is untestable when the report is `None`. "Behaviour equals 0.8.27" is false. The Python hook's gating was unstated. | § 1.4 and § 2.6: `not_built` is reported on aarch64 Linux CUDA builds. The wording is now "the allocator decision equals 0.8.27's". The Python hook is gated on `tegra-pool`. |
| 6 | major | A `doctor gpu` v2 bump breaks the frozen v1 contract and the sibling-record precedent. Calling the helper first breaks the `cpu` early return. | § 2.6: a new sibling verb, `doctor cuda-allocator` (`fathomdb.doctor.cuda-allocator.v1`), whose helper runs after the `cpu` and SBSA early returns. `doctor gpu` is unchanged. |
| 7 | major | The `Once` + `MODE` mutex is fragile: poisoning on panic, a lock held across device builds. Multi-device is unspecified. Probe, probe failure and first-build failure are missing. | § 1.2, § 1.3 and § 3.3: a single `OnceLock<Decision>` with `catch_unwind`; the first private build happens inside the decision; other ordinals take the 0.8.27 path; the probe is described. |
| 8 | major | The loss check retains, and so creates, a primary context after a reset. The snapshot reaches across layers. The characterization test cannot fail. | § 3.5: `cuDevicePrimaryCtxGetState` first; `strong_count` replaces the engine count; the test asserts results § 13.4's deterministic outcome. |
| 9 | major | `tegra-pool` from crates.io fails opaquely. `--config` cannot strip `[patch]`. | § 3.1 and § 4.2: the vendored `fathomdb-private-pool` marker feature gives a clear Cargo error. The T8 check strips `[patch]` in a scratch workspace copy. |
| 10 | minor | `#[non_exhaustive]` forces `_` arms in sibling crates. Further public breaks are unrecorded. | § 2.4 and § 2.7: explicit arms ahead of `_`, with tests; the breaks are listed for the ADR; `CudaDeviceInfo` is now `#[non_exhaustive]`. |
| 11 | minor | The helper's `cfg!` changes meaning when it moves. New `CudaDriverInit` states break `cu_result()`. Unmapped states. The Rust contract was unstated. | § 3.4: the flags are passed in; there is a separate `ModuleLoadInit`; every state has a reason; the Rust contract is stated. |
| 12 | minor | The threshold loses precision in JS; `MAXSIZE` units and bounds are missing; compute capability facts are missing; the Tegra check is one source; the CLI exit code is unnumbered. | § 2.1, § 2.2 and § 2.4: `"0"`/`"max"`; GiB/MiB with [32 MiB, total]; the compute capability attributes; the two-source check; exit 70. |
| 13 | minor | Fork while the decision is in progress copies a held lock into the child. | § 2.5: documented with H-1. Not detected; an atfork guard would be over-built. |
| 14 | minor | Traceability: § 5.2 versus § 4, the header SHA, the `tegra_fragmented_va_cuda.rs` direct calls. | Plan references fixed; citations on `b11e4523d`; § 3.3 excludes that test from `tegra-pool` runs. |

**Design questions.** DQ-1 is accepted as recommended. DQ-3 is accepted
with the F-10 conditions. DQ-2 goes to the owner before S30-T5, as the
reviewer recommended.

## Code review (independent Opus subagent, high effort, 2026-10-08)

**Input:** `git diff cb9594f44 deffb5fd6` (product code, scripts, tests,
vendored cudarc).

**Verdict:** APPROVE-WITH-FIXES. The reviewer confirmed these hold:

- every free uses the allocating API (`cuMemFreeAsync` back to the pool);
- the drop order (pool `Arc` → context → slices and streams);
- the retain/release guards;
- no re-entrancy into `DECISION` or the reranker cell, and a single lock
  order;
- `catch_unwind` works (no `panic=abort` profile);
- with the feature off, the path is identical to 0.8.27;
- the pinned-override gate has only stricter requirements;
- clippy is clean with the feature on.

| CR | Sev. | Finding | Resolution |
| --- | --- | --- | --- |
| 1 | major | Pool exhaustion or context loss while loading weights (`candle_bge.rs:603-614`, `candle_reranker.rs:454-472`) maps to `ModelDeserialize`, and the reranker memoizes it as `Unavailable`. | Fix: classify weight-load Candle errors through the pool-failure path first; test with a CUDA OOM under a private decision. |
| 2 | major | The resolution probe turns every private-build or pool failure into `CudaProbeError::ProbeFailed`. Forced `cuda:N` loses the kind, and `auto` silently runs on the CPU for the engine's life. | Fix: a typed probe failure. In a `Private` process the open raises the typed kind under any policy (no CPU move, CB4). Forced-policy and `auto` tests. |
| 3 | minor | Search query embedding (`search.rs:1420`, `search_api.rs:543`) falls back to sparse-only on a CUDA kind, and the projection worker records a generic `EmbedderError`. | Fix: search propagates the three CUDA kinds typed (ruling 33); the worker records the stable kind name. |
| 4 | minor | Forward classification uses the decision's ordinal, not the faulting device's. | Fix: pass the device ordinal; classify only when it is the pool's. |
| 5 | minor | `agent-verify` never compiles the `tegra-pool` driver part. | Fix: an aarch64-only clippy arm with `tegra-pool` in the lint verb. |
| 6 | minor | The release-contract guard is bypassable through double-quoted or composed values and the x86 Python build script. | Fix: forbid `tegra-pool` in every non-Tegra build script and in expanded values. |

The reviewer judged the other reported deviations acceptable. Separately,
the S30-T9 docs pass found a stale Cargo-message quote in
`third_party/cudarc-0.19.7/FATHOMDB-PATCH.md` item 6; it is fixed with
these findings.
