---
title: Slice 60 fork maintenance and qualification
status: COMPLETE
target_release: 0.8.30
slice: 60
---

# Slice 60 fork maintenance and qualification

Refresh the maintained Candle dependency, preserve the qualified cudarc
allocator and lifetime behavior, and qualify selected native and PTX kernel
targets on the exact bytes FathomDB will use. Owner seq-367 places F30-REBASE,
F30-MARKER and F30-MULTIARCH in .30. Slice 9 selects scope and Slice 10 owns
approved preparation. Implementation, native sessions, live faults and
publication require their applicable authority. [Fork allocation](../../fork-maintenance-plan.md) · [Master execution contract](../../plan.md#c-slice-planning-and-execution-contract)

Owner seq-409 activates this plan after reviewed55 completion at
`3595192b0a3d4587556aa00b24768a06469af3ba`, using the full execution template. Prioritize the exact-pin
safe-drop prerequisite needed to finish50. No repeat sequence confirmation is
needed; concrete venue/session/live-fault authority remains separate.

## Concrete execution selection

The [execution design](design.md) reconciles all selected requirements against
exact consumed source and published upstream Candle0.11.1 at
`59f8e903ca80a32ea7f8514b39748fc979e69e82`. Its source/delta map, bounded
sm75/sm86/sm87+PTX candidate, prospective cost rules, owning TDD controls,
installed-consumer joins and concrete live-protocol preparation govern this
execution. Direct Astra reviews plan and design before behavioral changes.
The whole scope remains selected; an unavailable mandatory cell stays OPEN.

## Entry and boundaries

- Reconcile current `origin/main`, both owner forks, and changes since `f7fb1da9a`.
  Review the dependency candidates from prework 1, assigned requirement,
  architecture and test drafts from 3–5, Slice 9 decisions and Slice 10
  environment proof. Identify candidate APIs, affected functions and tests,
  and disposition each allocated draft against actual source and requirements.
- The draft baseline is cudarc-fathomdb 0.20.0 at `343b04fa`, the Candle
  0.12.0 trio at `2ee39305`, and separate kernel source K9 at `182ceab0`.
  The workspace carries exact logical Git IDs with same-commit HTTPS mirrors.
  Recheck the package versions, lock identities and any subsequent authorized
  registry delivery. [Workspace manifest](../../../../../Cargo.toml) · [Source mirrors](../../../../../.cargo/config.toml) · [0.8.29 closure record](../../../../../dev/plans/release-state-0.8.29.json)
- x86 compute-75 PTX in the .29 build excludes >=80-only BF16/F8 kernels,
  and the MoE archive has one native target without PTX. Treat these as build
  leads and inspect actual baseline artifacts. Retain product F32 precision
  and the existing support and driver/toolkit policy. CUDA 13 delivery belongs
  to .31. Generic ARM64 CPU delivery remains distinct from Tegra CUDA and
  provides no SBSA GPU evidence. [CUDA policy](../../../../../dev/adr/ADR-0.8.28-cuda-version-support-policy.md) · [.30 policy review](../../research/cuda-version-adoption.md)

## Local requirements and acceptance

Consumers need a maintainable dependency and an accurate support contract.
Kernel selection must preserve numerical behavior, typed refusals and resource
ownership. The following local acceptance criteria are proposals for the
selected fork and product configuration.

| Unit | Proposed behavior | Acceptance evidence |
| --- | --- | --- |
| F30-REBASE / TC-509dc8a6 | Compare candidate upstream releases and exact revisions against the current fork. Adapt callers directly to the selected API. Track each inherited cudarc delta and each Candle K1–K9 change, including K7t, and preserve downstream behavior of .29 repairs. | A reviewed delta map links every retained, replaced or removed change to source and a current requirement. It covers K1 packaging, K2 ARM CPU F16 refusal, K3 dynamic driver loading, K4 static CUDART, K5 context construction, K6 rebuild inputs, K7 cudarc and the private-pool test, K8 registry naming/versioning and K9 the published cohort and kernel pin. A removal requires the corresponding contract to pass on the selected replacement. |
| Pin coherence and portability | Resolve one qualified cudarc for the selected CUDA graphs, including direct and reexported callers. Treat the `ug-cuda` path to older upstream cudarc as a lock inventory item and determine its reachability by selected features and target. | Exact locked normal and feature graphs for x86_64 and AArch64 embed, rerank, combined and CPU cases match their declared inputs, including the standalone SDK driver. Reject a second active implementation, wrong revision/checksum, unintended CUDA version feature, mirror drift and accidental kernel source substitution. A clean checkout resolves without local source access. |
| F30-MARKER / TC-93d11583 | Evaluate the cudarc `fathomdb-private-pool` marker independently of `CudaMemPool`, `new_with_mem_pool` and the product `tegra-pool` switch. Update fork `FATHOMDB.md` removal conditions before a selected removal. | Compare caller feature resolution and run the actual fork context/pool test. Private allocations affect only the selected private pool, and healthy last-owner release still works. The marker change keeps allocator selection, accounting and real customer feature behavior intact. |
| Lifetime and Slice 50 prerequisite | Carry forward the .29 shared lifetime admission, cached terminal loss, typed propagation and safe nested destruction through cudarc, Candle raw F32 GEMM, and product construction/use/drop. | Healthy calls and ordinary OOM remain recoverable. Cached terminal loss refuses later native work, leaves no stale free, trim or nested destroy, and preserves cause and close behavior. Hand Slice 50 exact pin-bound evidence. When separately authorized, a real Orin private-pool child must prove typed refusal, immediate close, last-clone drop, normal exit and fresh-process recovery before B30-01 can exercise or ship that release path. A selected 3090 case qualifies its own venue. |
| F30-MULTIARCH / TC-323f15b0 | Select a bounded native SASS/cubin target set and PTX fallback where supported. Make kernel loading explicit by target and kernel family, including a declared MoE disposition and BF16/F8 availability. | Inspect linked/packaged cubin targets, PTX targets/ISA and family symbols. Independently run known-output F32 cases on a native route with JIT excluded and a PTX route with native images excluded. Confirm actual dispatch and reference agreement using human-set tolerances. Below-floor, wrong-driver/library and missing family cases fail with the applicable typed error or documented `auto` fallback. Table rows without selected proof remain unqualified. |
| Measured adoption | Set comparable startup/JIT, build and size acceptance before trying the candidate. Select only artifacts that meet the reviewed correctness and cost limits. | Record baseline and treatment with matching drivers, toolkit, host, model/inputs and build features. Separate cold process and empty isolated JIT cache from warm behavior. Report startup through first valid output, JIT contribution, steady latency, clean build time and compressed/unpacked bytes by artifact. Retain neutral and adverse results and the supported baseline if the selected treatment misses its criterion. |
| Installed FathomDB consumers | Consume qualified commits in cudarc → Candle/core and kernels → FathomDB order. Bind the product graph, build inputs and artifact digests to the same source evidence. | Selected fresh installed Python/Node consumers exercise F32 embedding and reranking, positive device attribution, persisted operations and clean close/exit. Actual CLI archives exercise existing device diagnostics and read-only integrity/row-count verbs over those SDK-created DBs; the accepted CLI scope ADR forbids new application writes/queries, and diagnostics do not count as inference. Driverless x86 SDK consumers exercise CPU and `auto` on CUDA-capable bytes, and forced CUDA yields the existing typed refusal. Tegra Node follows its accepted detection-gated contract: maintained driverless addon/linkage proof and expected thin-loader generic-selection refusal are separate from actual Tegra-host SDK GPU lifecycle; current-version generic ARM SDK driverless lifecycle belongs to75 with its real matching generic artifact. Preserve ARM CPU F32/F16 behavior and the Tegra private allocator on actual selected venues. |

## Design seams and research

- Own fork plans, requirements, design, tests and qualification in each fork's
  `fathomdb/` tree. Review both FATHOMDB.md inventories, Candle
  `candle-kernels/build.rs` and `candle-core/src/cuda_backend/`, and cudarc
  `src/driver/safe/`. Compare selected upstream source/tags and release notes
  for existing-context construction, dynamic loading and ARM CPU dispatch.
  [Candle inventory](https://github.com/coreyt/candle-fathomdb/blob/2ee39305a78ed1463ff913f71dcd3d45ecbfd84f/FATHOMDB.md) · [cudarc inventory](https://github.com/coreyt/cudarc/blob/343b04fa73f85ee78754d6ee2baa07b959cc9ccc/FATHOMDB.md)
- Use primary NVIDIA documentation and selected upstream build/loader source
  to resolve cubin compatibility, PTX ISA/driver floors, fatbinary and JIT
  selection, and MoE archive coverage on the approved toolkit. Trace each
  candidate through the shared-artifact link and inspect actual bytes.
  [Driverless linkage ADR](../../../../../dev/adr/ADR-0.8.25-driverless-cuda-runtime-linkage.md) · [Artifact contract](../../../../../scripts/check-cuda-release-contract.py)
- Check primary Cargo documentation on source replacement and downstream
  packaging. A root patch applies only to its workspace. If a later registry
  route is selected, prove the normalized crate graph with checksums and
  downstream artifacts, including kernel linkage, before claiming equivalence.
  [Pin policy](../../../../../dev/design/pinned-override-rot-guard.md) · [SDK driver manifest](../../../../../experiments/sdk_drivers/rust/Cargo.toml)
- Compare the final design with the accepted private-pool, library/lifetime
  and host policies and Slice 50's ownership contract. Use known pool counters
  and owned-process cleanup for release proof. Live reset review is scoped to
  completed construction and quiescent owned children. Partial-load real
  reset, host reset and same-process recovery are outside this work.
  [Private-pool ADR](../../../../../dev/adr/ADR-0.8.28-tegra-private-cuda-pool.md) · [Reset allocation](../../release-followups.md) · [B30-01](../../../../../dev/ROADMAP.md#0830-backlog)

## Execution and verification

1. Update the fork-local designs and any affected product contract with concrete
   input pins, target rows, cost criteria, test selections and prerequisite
   owners. Direct Astra-medium reviews design additions. Direct Astra-medium
   reviews the updated plan with at most one FIX-1 and re-review. Commit that
   plan only after PASS and hold the transition if the limit is reached.
2. With authority and preparation in place, qualify affected cudarc behavior
   first, then the chosen Candle/core/kernel cohort, then the exact product
   adoption. Review marker removal separately while preserving evidence on the
   final combination. Capture genuine behavioral RED per change, then GREEN
   and refactor. Keep existing human oracles and real DBs.
3. Reuse cudarc's `lost_slice_disarms_all_native_free_and_nested_event_cleanup`
   and healthy paired control, Candle `cuda_private_pool` and
   `aarch64_linux_cpu`, and the accepted raw GEMM seam where affected.
   New native and PTX execution controls are proposed tests. In FathomDB use
   `scripts/check-single-cudarc.sh`, `scripts/check-pinned-override-rot.sh`,
   `scripts/tests/test_qualified_fork_pins.py`, the affected embedder ARM and
   CUDA library tests, and the existing [installed artifact smoke](../../../../../scripts/release/cuda-package-rehearsal-smoke.sh).
   [0.8.29 lifetime test map](../../../../../dev/plans/0.8.29/features/slice-110/test-map.md)
4. Direct Sol 6.2 high reviews code and direct Terra independently verifies
   affected tests and artifact identity. Use scoped checks while working and
   one required full gate per stable source phase. Retain .29 credit within
   exact source/features/inputs and claims, and rerun only affected, failed or
   unrun scope after narrow changes. Apply direct review per the master.
5. Record concise actual outcomes, supported targets, limitations and pending
   authority in the owning repositories. Hand the safe-drop determination to
   Slice50, final source/pin evidence to70 and installed consumer/native
   applicability evidence to75 underseq-407. Integrate reviewed
   pins into the owning release branch, update affected SDK/build docs, and
   clean owned inactive scratch and worktrees after evidence retention.

## Open choices and prerequisites

- Slice9 selected placement; this execution selects published upstream Candle0.11.1, exact qualified Git delivery and the reviewed fresh0.13/0.21 normalized-package proposal. Slice10 tooling is retained. Owner seq413 subsequently authorized the exact five-package publication and Q2 successor; actual public delivery is recorded in [status](status.md). Old0.12/0.20 versions remain immutable.
- The design prospectively selects target rows and cost limits. Recheck actual exclusive venue availability before native execution. Keep R580/primary12.6 and exclude K620.
- Live reset qualification needs separate venue, resource and fault authority. Slice 50 can proceed with independent safe work while that prerequisite is held.
