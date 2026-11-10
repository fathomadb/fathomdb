---
title: Slice 110 — CUDA context-reset lifetime and safe teardown
status: PROPOSED
target_release: 0.8.29
slice: 110
---

# Slice 110 — CUDA context-reset lifetime and safe teardown

## Goal, maturity and inspected evidence

Address `TC-281155a1`: survive the approved context-loss fault model through
operation refusal, engine close, native object drop and normal process exit.
This draft follows the [brief](../../planning-brief.md) and
[reconciliation](../../reconciliation-plan.md); it authorizes no fault injection.
The [.28 design](../../../0.8.28/features/slice-30/design.md) and
[status](../../../0.8.28/features/slice-30/status.md) establish detection, not survival:
the Orin child returns typed loss twice, emits one snapshot, then SIGSEGVs in
`cublasDestroy_v2`. The earlier study also exposed slice/stream teardown hazards.
The [current characterization](../../../../../src/rust/crates/fathomdb-embedder/src/cuda_pool_policy_tests.rs)
intentionally asserts that crash; no reset experiment was run for this proposal.

The [accepted pool ADR](../../../../adr/ADR-0.8.28-tegra-private-cuda-pool.md),
[Rust](../../../../interfaces/rust.md), [Python](../../../../interfaces/python.md)
and [TypeScript](../../../../interfaces/typescript.md) still describe reset as
unsupported and closing as unsafe. Preserve those claims until replacement proof.
[Current policy](../../../../../src/rust/crates/fathomdb-embedder/src/cuda_pool_policy.rs)
holds a process-lifetime decision/first context; the last-model guard trims only
a live context, but runs after model/device fields have already dropped.
[Embedder](../../../../../src/rust/crates/fathomdb-embedder/src/candle_bge.rs) and
[reranker](../../../../../src/rust/crates/fathomdb-embedder/src/candle_reranker.rs)
field order therefore cannot by itself protect lower-level destructors.
Read local cudarc/Candle destructor sources as investigative leads; authenticate
exact consumed sources before implementation, using [.28 provenance](../../../0.8.28/features/slice-95/status.md).
Memory consulted: `windchill3-never-change-driver-or-primary-cuda.md`,
`forks-coreyt-cudarc-and-candle-fathomdb.md`, `physical-erasure-done-defects-only.md`.
Physical erasure remains closed except concrete defects; this slice adds no SLA.

## Slice preparation and local acceptance draft

Produce `requirements.md`, `design.md` and `test-map.md` beside this plan during
execution; these IDs are release-local drafts, not new canonical AC numbers.

| Need / requirement | Falsifiable local acceptance |
| --- | --- |
| S110-N1/R1: safely refuse lost-context use | S110-A1: supported loss returns existing typed payloads on embed, batch, rerank, open and standalone paths; repeated/concurrent detections emit one process snapshot and never invent successful vectors or neutral scores. |
| S110-N2/R2: preserve ownership after loss | S110-A2: cloned model/tensor/device owners and outstanding allocations cannot trigger stale waits, frees, library-handle destruction, pool trimming/destruction or context release after the approved loss transition; healthy destruction still releases resources. |
| S110-N2/R3: close/drop/exit survival | S110-A3: reset then immediate close without another forward, loss then repeated close, partial-load unwind, last-owner drop and normal exit finish within a preregistered deadline with expected status, no signal, panic or hang. |
| S110-N3/R4: truthful recovery boundary | S110-A4: lost generations remain unusable across new engine/standalone requests; approved refusal persists without CPU fallback or allocator switching; a fresh process can initialize and compute again. |
| S110-N3/R5: bounded cleanup policy | S110-A5: any quarantined ownership has an explicit count/byte bound and reclamation boundary; ordinary close and OOM controls retain their existing memory-release/retry behavior. |
| S110-N4/R6: prove actual consumers | S110-A6: fork unit proofs and installed Rust/Python/Node child receipts distinguish allocator/platform cells; private-pool behavior is not inferred from discrete-GPU success. |

## Design draft, challenges and planned research

First inventory the entire ownership graph: tensor/storage → slice/events/stream
→ context/pool, plus cuBLAS and other enabled handles, temporary allocations,
load-failure cleanup, model clones, queue work, singletons and process statics.
For each node record creator, final owner, in-flight users and every FFI action
on success, error, drop and exit. The observed cuBLAS frame is not the whole fix.
Draft a shared generation/lifetime state with monotonic loss and admission stop;
show how all reachable wrappers observe it before cleanup, including no-forward
close, and how outstanding work finishes or becomes safely abandoned.
A catch_unwind around Drop cannot catch SIGSEGV. A top-level lost flag or
last-model trim skip cannot protect destructors that have already executed.
Compare fork-level invalidation with whole-owner quarantine or worker isolation.
If quarantine is selected, prove the complete graph cannot later run unsafe
nested destructors; reject per-request unbounded leaks and mixed-generation frees.

Proposed recovery scope is fail-closed until process restart, not same-process
CUDA reconstruction. Existing load retries must not resurrect a lost generation.
Distinguish reset between calls, reset with live allocations, queued work and
reset during active kernels/cleanup. A context-state check has a race with an
uncoordinated external reset; do not promise arbitrary asynchronous survival.
Resolve that race by an explicit supported quiescence/cooperation contract or
reviewed process isolation; inability to prove safety is a design stop, not a
reason to quietly narrow the documented claim after implementation.
Planned primary research: exact qualified cudarc/Candle source, NVIDIA primary
context reset/release, asynchronous free/event/pool ownership and cuBLAS handle
lifetime specifications. Record versions and distinguish guarantees from tests.
Apply the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
through bounded lifecycle/state-machine cases and minimized fault replays;
revise the design after fit review, deferring broad GPU fuzzing and unrelated tuning.

## Dependencies, resources and newly identified prerequisites

- Hard: Slice 9/10 execution authorization, approved fault/recovery/cleanup design,
  independent design review, checkout-owned tooling and explicitly permitted GPU venue.
- Conditional: a low-level repair belongs in the owning forks under Slice 140,
  with independent design approval and fork-local TDD/review/qualification first;
  qualified cudarc → Candle → FathomDB consumption then enables 110 integration.
  This gate does not require 110 completion; there is no 110 → 140 → 110 cycle.
- Settle included 140 pins before final 110/120 integration; Slice 130 reruns
  affected final platform/SDK/pool-release/soak cells on chosen pins. Its broader
  qualification is distinct from fork unit proof and this slice's fault proof.
- Serialize shared engine/embedder/native/interface landings with 40/50/70/100;
  use one writer per isolated checkout. Coordinate pin/lockfile edits with 120/140.
- Newly identified: enumerate non-private/default/synchronous paths, authenticate
  exact destructor sources, approve quiescence versus isolation, bound quarantine,
  and secure a private-pool venue before claiming the original defect closed.
- Later faults run only in dedicated child processes on an explicitly permitted
  RTX 3090, with exclusive reservation, bounded timeout and GPU UUID selection;
  never K620, host-wide GPU reset, driver change or primary CUDA symlink change.
  windchill3 remains R580 and primary CUDA12.6. Orin access remains conditional.

## Work, validation, review and exit

1. Complete ownership/fault/surface matrices and falsifiable requirement-to-test
   mapping; independent adversarial review resolves all unsafe lifetime gaps first.
2. Stage/commit a failing test per approved behavior before code; preserve .28
   crash evidence and replace its expected crash only under the reviewed new spec.
   Pure lifecycle/fault seams cover shared-owner drops, concurrent loss, outstanding
   allocations, query failures, close without detection and OOM/healthy controls.
3. Implement only reviewed scope; fork changes retain their own `fathomdb/`
   design/TDD/review receipts. FathomDB consumes qualified pins, not fork test reruns.
4. After venue permission, run bounded subprocess faults and prove post-loss typed
   errors plus close/drop/normal exit; retain actual statuses and timeout failures.
   A parent killing a child or bypassing destructors is not teardown survival proof.
5. Verify installed checkout-owned native Python/Node, Rust SDK and standalone
   paths; retain artifact/pin/GPU/driver identities and affected cell limitations.
6. Run scoped checks while iterating, required full agent verification on the final
   code phase, independent code review and affected integration reruns; hand final
   pin/artifact evidence to 130. Missing environments/venues remain explicit gaps.
7. Update ADR/interface claims only with approved scope and evidence in the same
   implementation PR; exit requires A1–A6, resolved reviews and stated refusals.

Open decisions: supported reset timing, cleanup/quarantine bound, fork versus
worker architecture, affected platform cells and venue permission. No fault,
review approval, product pass, recovery guarantee or release closure is claimed.

## Fork scope boundary (owner seq-367)

Only required repairs from .29 Slice140 may supply affected pins here. The
[broader .30 maintenance](../../../0.8.30/fork-maintenance-plan.md) is not a
prerequisite. A need for that broader work returns to the owner for explicit
scope review; it is not pulled into .29 automatically.
