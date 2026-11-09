---
title: Attributable CUDA allocation witness v2
status: accepted technical design; Astra design review; implementation and qualification pending
date: 2026-11-09
target_release: 0.8.28 Slice 95
---

# Attributable CUDA allocation witness v2

This technical successor is authorized by Slice95's GPU qualification repair
scope. It declares no new HITL ruling. It updates the measurement/control/schema
of [the original witness](../design/0.8.23-aarch64-tegra.md),
while retaining device identity, an actual forward and the64MiB model floor.
The [private-pool allocation policy](ADR-0.8.28-tegra-private-cuda-pool.md), store
format37, ordinary opens, model pins and WAL/NORMAL policy stay unchanged.

## Evidence and decision

The earlier [pool study results](../plans/runs/0.8.28-pool-study/results.md)
(2026-10-06, section10.6) already record this failure class: a private pool and
loaded model with used_high127MiB, but a global witness delta of-126MB.
Concurrent page-cache reclaim was explicitly inferred from cache/MemAvailable
changes. The original failure remains59/60. Section10's warning and
[study ruling16](../plans/0.8.28/prework/tegra-cuda-memory-pool-study.md) separate
witness results from pool-study pass/fail rows. That ruling scopes study
reporting; Slice95 still requires current installed-artifact GPU proof.
This prior evidence should have informed diagnosis before the CI20 campaign.

CI20's checked completion repair did not stabilize the shared UMA counter:
Node01 passed140574720 bytes; Node02 failed9539584 against67108864. The campaign
stopped. The one preregistered CI22 process at395e1a99d, installed addon9083bbc1,
retains another v1 failure (-374050816 bytes, exit1). In that SAME process,
checked current private-pool USED increased0→132848640 during model load; the
forward produced384 dimensions. One touched67108864-byte same-pool control
increased USED132848640→199957504 and released exactly to132848640.
Independent verification preserved all75 files with equal hashes, unchanged
source/artifacts/models and the earlier620 files. This is counter feasibility,
not qualification or an established cause for earlier shared-counter failures.
Raw local evidence: `target/slice95-final-format37/approved-ci/ci22-hardware/`.

The follow-up dependency audit binds that exact sealed addon to published
cudarc-fathomdb0.19.10 atb777f8d94 and Candle core/nn/transformers0.11.0 atK9
182ceab054, plus the active candle-kernels Git patch atK9 with static CUDART.
All499 packaged files match lock-authenticated crate archives; all25 kernel
files match Git objects. Compiler dependency paths/fingerprints and the sealed
addon agree. Local fork runtime sources match those qualified pins; newer local
fork commits concern qualification/CI tooling and documentation. The remote
fork history reveals no missing runtime repair. Tracking refs were inspected
without fetching. This correction requires no library/pin update or fork rerun.
Sanitized dependency evidence is under
`target/slice95-final-format37/approved-ci/tegra-patched-dependencies-research/`.

[NVIDIA CUDA12.6.1](https://docs.nvidia.com/cuda/archive/12.6.1/cuda-driver-api/group__CUDA__MALLOC__ASYNC.html)
distinguishes live application allocation accounting (USED) from reserved
backing memory. Neither reserved/high-water bytes nor shared UMA free memory
can substitute for the model's live private allocations. The private-pool ADR's
memory-release discussion does not waive release qualification.

Select the method BEFORE measurements, from the witnessed context's checked
INTEGRATED device attribute and actual allocator/pool. An integrated device with
an actual private pool uses `private_pool_used`. An integrated device without
one returns a named opt-in witness refusal; it never tries a global reading to
get a pass. A discrete device with a non-private allocator retains `global_free`,
its bounded raw control and existing external PID proof. Discrete+Private is an
unsupported witness combination and refuses before sampling; current allocation
policy never produces it. Driver/identity/attribute errors fail closed.
There is no result-dependent fallback or change to allocation/device policy.

## Private measurement and ordering

Hold the exact witnessed stream/context/private pool, with checked ordinal/UUID
binding. Require checked completion BEFORE the initial USED baseline; a failure
prevents baseline sampling and loading. Sample USED before load; load the pinned model on that device; require
checked completion; sample USED afterwards; execute the real forward384.
Keep the model alive. Complete and sample a post-forward control baseline,
allocate exactly64MiB from that held stream/pool, touch nonzero, complete and
sample live usage. Release only that control, complete and sample again.
Model delta is signed `used_after-used_before` and must reach67108864; control
rise must reach its exact67108864 request and released USED must equal baseline.
No trimming, sleeps, extra blocks, retries, counter reset or pool-setting changes.
All read/synchronization/load/forward/control failures are named refusals.

This method supersedes the old GiB depletion precondition on private pools:
raw cuMemAlloc is outside the pool and cannot calibrate its USED counter.
Discrete global measurement keeps its existing before-load control sequence.

The private window requires no competing allocations/frees in that same pool.
This is an explicit operator/harness precondition, not a new runtime lock or
claim that Node has one OS thread. Installed qualification awaits one Engine.open
without launching concurrent SDK/GPU work during its witness window. Driver
context ID and pool identity bind the observations to this process/device.
A null/default CUDA stream is valid. Checked context IDs are uint64 identifiers;
no undocumented nonzero requirement is imposed.

## Current record and public contract

Emit only `fathomdb.tegra-gpu-allocation-witness/v2` for current witnesses.
Current verifiers reject v1; frozen historical records remain unchanged and
failed v1 measurements remain failed v1 measurements. No compatibility shim.
The v2 canonical snake-case shape is:

- Common: schema; process_id; requested/actual device ordinals, UUID, name and
  compute capability; device_integrated boolean; actual global free_before_bytes,
  free_after_bytes, total_bytes and signed delta_bytes; delta_floor_bytes;
  embedded_vector_dim; and one tagged measurement object.
- `measurement.kind=global_free`: control_allocation_request_bytes,
  control_block_count, control_free_before_bytes, control_free_after_bytes,
  control_delta_bytes and sole_gpu_consumer_precondition. Model delta is the
  common signed global decrement. The existing bounded control/request and
  selected-device sole-GPU-consumer precondition remain.
- `measurement.kind=private_pool_used`: context_id (lossless decimal string),
  pool_identity (opaque string), used_before_bytes, used_after_bytes, signed
  delta_bytes, control_request_bytes, control_before_bytes, control_live_bytes,
  control_released_bytes and pool_exclusivity_precondition.

Private success can coexist with a NEGATIVE common global delta: those fields
remain truthful diagnostics. Global free samples may be zero; require
0<=free<=total, positive total and correct common signed arithmetic. Private acceptance uses only its own USED delta
and control accounting alongside common identity, process, floor and forward
checks. Exact variant key sets exclude irrelevant optional bags/fake values.
RESERVED/high-water observations remain optional developer diagnostics, not new
public acceptance fields. The canonical global precondition is "the witness process must be the sole GPU
consumer of the selected discrete device". The private precondition is "no
competing allocations or frees may occur in the witnessed private pool during
model and control observation". Installed Python/Node controls require the
reported process_id to match os.getpid/process.pid; the CUDA envelope also binds
it to its existing consumer PID. Per-method preconditions are canonical text.

Rust uses a finite tagged measurement enum. Python and TypeScript expose matching
tagged shapes with their ordinary naming convention; native adapters must map
both variants without dropping identity or nested data. Context/pool identity
never passes through JavaScript floating-point conversion. Explicitly update
nested Node extraction; its old top-level-only snake-case conversion is
insufficient. Both strict allocation verifiers rederive arithmetic and method
compatibility; the Tegra route requires integrated/private proof. Cross-consumer
PID/UUID/artifact bindings remain mandatory.

## Delivery and verification

Extend the existing witness primitives, SDK mappings, current fixtures and two
verifiers. Review contract-fixture amendments before freezing RED. Observe real
behavioral REDs for every selection/refusal combination and initial-completion
failure preventing sampling/load, private arithmetic including negative
global/positive USED, floor/control/release/identity failures and exact SDK
round trips; then GREEN with those assertions unchanged. Preserve existing
completion/error/order and global-method controls. Avoid broad refactors or a
new evidence framework/registration. Astra reviews design and exact code; an
independent verifier exercises the affected scopes.

The diagnostic remains reproducible at pinned395e1a99d with its committed runner
and source archive. Update current diagnostic terminology/documentation rather
than promising v1 semantics on a v2 implementation; keep the investigation tools
available and all original raw outcomes. Production acceptance must derive from
raw observations, never the diagnostic_valid boolean.

Qualify through the existing installed Node AND Python policy/model/heap controls,
a fresh exact-source Jetson workflow and genuine driverless ARM control, plus
existing local x64 CUDA preflight/rehearsal obligations. Stop on a genuine failure;
no added three-Node campaign or replacement run. Batch final full exact-source
manual CPU CI after reviewed repairs and affected hardware proof. Then perform
the existing trusted nonpublishing main-route release rehearsal. Publication,
tags, registries and Pages remain held. Reuse unaffected engine/performance proof
with explicit source/blast-radius comparisons; no new throughput/profile sweep.
