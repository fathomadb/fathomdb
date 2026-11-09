---
title: Attributable CUDA allocation witness v2
status: accepted technical design; Astra design review; implementation and qualification pending
date: 2026-11-09
target_release: 0.8.28 Slice 95
---

# Attributable CUDA allocation witness v2

This technical successor is authorized by Slice 95's GPU qualification repair
scope. It declares no new HITL ruling. It updates the measurement/control/schema
of [the original witness](../design/0.8.23-aarch64-tegra.md),
while retaining device identity, an actual forward and the 64 MiB model floor.
The [private-pool allocation policy](ADR-0.8.28-tegra-private-cuda-pool.md), store
format37, ordinary opens, model pins and WAL/NORMAL policy stay unchanged.

## Evidence and decision

The earlier [pool study results](../plans/runs/0.8.28-pool-study/results.md)
(2026-10-06, section 10.6) already record this failure class: a private pool and
loaded model with used_high 127 MiB, but a global witness delta of -126 MB.
Concurrent page-cache reclaim was explicitly inferred from cache/MemAvailable
changes. The original failure remains 59/60. Section 10's warning and
[study ruling 16](../plans/0.8.28/prework/tegra-cuda-memory-pool-study.md) separate
witness results from pool-study pass/fail rows. That ruling scopes study
reporting; Slice 95 still requires current installed-artifact GPU proof.
This prior evidence should have informed diagnosis before the CI20 campaign.

CI20's checked completion repair did not stabilize the shared UMA counter:
Node01 passed 140574720 bytes; Node02 failed 9539584 against 67108864. The campaign
stopped. The one preregistered CI22 process at `395e1a99d`, installed addon `9083bbc1`,
retains another v1 failure (-374050816 bytes, exit1). In that SAME process,
checked current private-pool USED increased 0→132848640 during model load; the
forward produced 384 dimensions. One touched 67108864-byte same-pool control
increased USED 132848640→199957504 and released exactly to 132848640.
Independent verification preserved all 75 files with equal hashes, unchanged
source/artifacts/models and the earlier 620 files. This is counter feasibility,
not qualification or an established cause for earlier shared-counter failures.
Raw local evidence: `target/slice95-final-format37/approved-ci/ci22-hardware/`.

The follow-up dependency audit binds that exact sealed addon to published
cudarc-fathomdb 0.19.10 at `b777f8d94` and Candle core/nn/transformers 0.11.0 at K9
`182ceab054`, plus the active candle-kernels Git patch at K9 with static CUDART.
All 499 packaged files match lock-authenticated crate archives; all 25 kernel
files match Git objects. Compiler dependency paths/fingerprints and the sealed
addon agree. Local fork runtime sources match those qualified pins; newer local
fork commits concern qualification/CI tooling and documentation. The remote
fork history reveals no missing runtime repair. Tracking refs were inspected
without fetching. This correction requires no library/pin update or fork rerun.
Sanitized dependency evidence is under
`target/slice95-final-format37/approved-ci/tegra-patched-dependencies-research/`.

[NVIDIA CUDA 12.6.1](https://docs.nvidia.com/cuda/archive/12.6.1/cuda-driver-api/group__CUDA__MALLOC__ASYNC.html)
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
checked completion; sample USED afterwards; execute the real forward producing 384 dimensions.
Keep the model alive. Complete and sample a post-forward control baseline,
allocate exactly 64 MiB from that held stream/pool, touch nonzero, complete and
sample live usage. Release only that control, complete and sample again.
Model delta is signed `used_after-used_before` and must reach 67108864; control
rise must reach its exact 67108864 request and released USED must equal baseline.
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
reported process_id to match os.getpid/process.pid in the consumer namespace.
Container qualification binds it to the host GPU PID as specified below. Per-method preconditions are canonical text.

Rust uses a finite tagged measurement enum. Python and TypeScript expose matching
tagged shapes with their ordinary naming convention; native adapters must map
both variants without dropping identity or nested data. Context/pool identity
never passes through JavaScript floating-point conversion. Explicitly update
nested Node extraction; its old top-level-only snake-case conversion is
insufficient. Both strict allocation verifiers rederive arithmetic and method
compatibility; the Tegra route requires integrated/private proof. Cross-consumer
PID/UUID/artifact bindings remain mandatory.

## Container process binding

The existing x64 GPU probes keep Docker's default PID isolation. Their shell
entrypoints exec Python/Node as namespace PID1, while the trusted sealer's
Docker State.Pid and NVIDIA-SMI observation use the host PID. The witness's
process_id remains the unchanged in-process identifier; direct equality across
these namespaces would refuse a valid run.

Each existing hash-sealed gpu-*-cuda-witness.json gains required
`pid_namespace={source:"host_proc_status",nspid:[host_pid,...,1]}`. The host
collector reads exactly one NSpid line from `/proc/<State.Pid>/status` while
the report and consumer are live, then rechecks Docker Running and the same
State.Pid before sealing. Parsing/read/state/mapping errors refuse immediately;
no retry, environment-supplied PID claim or in-process record rewrite.

Require at least two positive integer PID entries, excluding booleans. The
first equals outer consumer.process_id and NVIDIA-SMI PID; the last equals
allocation_witness.process_id and1. Nested intermediate namespaces are permitted.
[Linux documents the outer-to-inner order](https://man7.org/linux/man-pages/man5/proc_pid_status.5.html).
Existing direct Python/Node checks still require their witness PID to match
os.getpid/process.pid. Both producers and verifiers enforce this binding.

This is one new required field in already sealed GPU observations, with no new
file/bundle allowlist or outer preflight v2/v3 change. Frozen historical files
stay untouched; current records have no optional proof fallback. Add actual
parser/state/PID mismatch REDs and preserve existing UUID/PID/artifact controls.

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
