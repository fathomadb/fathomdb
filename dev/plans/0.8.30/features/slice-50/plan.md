---
title: 0.8.30 Slice 50 runtime concurrency and resources
status: ACTIVE
slice: 50
target_release: 0.8.30
---

# Slice 50 — Runtime concurrency and resource bug hunt and remediation

Owner seq-408 commissions the whole revised plan with `gpt-6.1-sol` high
after40 source completion and remediation of its review findings.

## Entry and prerequisites

Execution baseline is `cd64360a63bf06c7d1bc5f27865cca6d7400eff8` in
`/tmp/fathomdb-0830-slice50-impl`, branch `impl/0.8.30-slice50`.
The [execution design](design.md) reconciles the drafted scope, bounded
schedules, local acceptance and prerequisite findings. It is reviewed before
behavioral edits. Parent owns canonical release records and final integration.

Reconcile [master § C](../../plan.md#c-slice-planning-and-execution-contract)
on the commissioned checkout. List changes since `f7fb1da9a`, overlapping work,
the actual source and installed/native inputs, and Slice 3/4/5 drafts assigned
here. Give each a keep, revise, decline or defer disposition and owner.
Consume approved Slice 10 preparation and relevant Slice 30 fixes. Safe source
and CPU work can proceed while Slice 60 qualifies the consumed cudarc/Candle
pins. Slice 60 owns proof that final `CudaSlice` drop is safe after context loss
before Slice 50 exercises or ships B30-01 CUDA release. Check whether .29 proof
covers that path on actual consumed pins or whether proof remains outstanding.
The owner decides work activation, live-fault authority and venue.
[Receiving allocations](../../release-followups.md), [fork plan](../../fork-maintenance-plan.md).

## Goals, requirements and acceptance

Users need positive concurrent progress, truthful cancellation and timeout
outcomes, safe close and reopen, and predictable resource ownership. Select the
following bounded hypotheses and at most one discriminating follow-up per case
unless evidence supports broader scope. Preserve accepted deadlines, device
policy and the physical-erasure defect-only scope.

| Local requirement | Observable exit |
| --- | --- |
| R30-ROB MCP admission | Rendezvous proves an admitted MCP call and a second waiting for its permit. Queued cancellation never admits or starts the call. An admitted run-limit timeout keeps the MCP permit until the abandoned call actually returns. One registry retains `max_open_stores=1`: assert second-store busy/switch behavior; use independent registries for simultaneous store success. Subsequent admissible calls complete with no stranded waiter. |
| R30-ROB engine dispatch and isolation | Test two independent engines on distinct databases, using a canonical/path-alias hook only for its intended engine. Queue saturation/expiry yields `Overloaded`; a started embedding timeout yields `Embedder` at the direct dispatch boundary while its worker remains occupied until provider return. Ordinary search may use its documented sparse fallback. Assert each layer's actual error/fallback, capacity recovery and exact positive results rather than equating worker slots with MCP permits. |
| R30-ROB close and recovery | Exercise concurrent/repeated close and canonical/alias ownership using positive committed writes and reads. Record waiter completion, owner teardown, release of locks and exact read/write state after fresh-process reopen. Observe SQLite quiescence before the one shared provider-drain deadline, including expiry and later return. |
| R30-ROB default Node | Compare matched pool ON/OFF on the default Node runtime with positive calls, exact outputs, passive JS and native observations, resource bounds and normal close/exit. Classify growth against owned live allocations, allocator reservation and measurement overhead. Keep the .29 default FAIL/OFF_NOT_RUN visible and preserve its semispace4 result separately. |
| R30-ROB conditional model release | For selected B30-01 after safe-drop proof, show explicit Rust/Python/TypeScript release, concurrent-use safety, repeat release, healthy reload and error parity. For separately authorized real CUDA loss, require typed refusal, safe teardown and fresh-process recovery on each selected venue. |
| R30-ROB logging synchronization | Reproduce the retained asynchronous logging fixture failure (`6 != 3`) and establish its intended event/owner contract. Use rendezvous or observable completion, assert the actual intended events and successful outer work, and distinguish a fixture timing defect from duplicate product emission. A sleep, dropped assertion or arbitrary new count does not close the case. |

## Scope after the approved split

Owner seq-407 splits the prior50 plan. This slice retains runtime concurrency,
shutdown, asynchronous logging, default Node resource investigation and conditional
B30-01 model release. [Slice55](../slice-55/plan.md) owns the seq-406 data-integrity
fixture, temporal/relationship composition, product fuzzing, process-crash/WAL
and doctor gap fixes, and thin shared behavioral tests. No selected work is
removed. The seq-407 split itself granted no execution authority; seq-408 now
commissions50.

Reuse [the SQLite-method roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
for first/persistent runtime faults, positive progress and owned-resource cleanup.
Do not duplicate55's data model or its deep engine matrix in each SDK. Keep
thin actual binding checks for any runtime/API change made here and hand the
stable source and discriminating runtime regressions to70;75 owns final installed
platform proof. Erasure remains lower-priority, defects-only regression.

## Architecture and draft design

- Keep MCP `StoreRegistry` permit ownership and engine dispatcher worker
  ownership distinct. For selected MCP probes use its test rendezvous and
  public request, and for engine probes use the real provider dispatch seam.
  For MCP use second-store refusal/switch within one registry or separate
  registries for concurrent success. Separately exercise two independent engines
  on distinct databases, with a real path alias for the targeted engine hook;
  verify the unrelated engine completes positive work.
  Reuse the [.29 alias controls](../../../0.8.29/features/slice-20/test-map.md#actual-path-alias-regressions),
  [MCP admission](../../../../../src/rust/crates/fathomdb-mcp/tests/admission.rs)
  and [dispatcher](../../../../../src/rust/crates/fathomdb-engine/src/embed_dispatch/core.rs).
- `Engine::close` quiesces database owners before joining provider workers.
  Use rendezvous and recorded deadline events to detect early clock start or
  renewal. Assert provider-only retention on drain expiry and reuse after
  release. Fixture watchdogs bound tests. The production 30-second budget
  applies after quiescence. [Runtime ADR](../../../../adr/ADR-0.8.27-engine-owned-runtime-topology.md),
  [close observer tests](../../../../../src/rust/crates/fathomdb-engine/src/slice90_close_review_tests.rs).
- For B30-01, inventory actual consumers of the
  [PyO3 CLS](../../../../../src/rust/crates/fathomdb-py/src/embedding.rs),
  [NAPI CLS](../../../../../src/rust/crates/fathomdb-napi/src/embedding.rs) and
  [engine reranker](../../../../../src/rust/crates/fathomdb-engine/src/rerank.rs)
  cells, including existing load/refusal memoization and in-flight tasks.
  Draft clearable ownership and bounded typed refusal while a model is in use.
  Compare owned leases and a serialized release state so a new model cannot
  reload during teardown. Define empty/repeated/concurrent release and lock
  order before choosing API spelling. Keep engine-owned close separate and
  require a caller action for release. [B30-01](../../../../ROADMAP.md#0830-backlog).
- Which signals distinguish JS heap capacity, native live ownership and pool
  reservation? Check the pinned runtime’s passive counters and their cost,
  compare Node output/ownership and Linux RSS/anonymous/swap with model counts
  and private used/reserved bytes in the equivalent observer build. Identify
  unavailable counters explicitly. [Node process memory](https://nodejs.org/download/release/v25.9.0/docs/api/process.html#processmemoryusage),
  [V8 heap spaces](https://nodejs.org/download/release/v25.9.0/docs/api/v8.html#v8getheapspacestatistics),
  [CUDA allocator](https://docs.nvidia.com/cuda/cuda-driver-api/cuda_driver_api/group__CUDA__MALLOC__ASYNC.html).
- Check whether Rust ownership keeps model references live through queued work
  and native execution until actual last use. Revise the design and identify
  unresolved venue or API choices. Keep close ownership assertions here;55 owns
  composed durable crash/WAL assertions. [Rust Arc](https://doc.rust-lang.org/std/sync/struct.Arc.html).

## Probe, review and implement

1. Select request counts, barriers, resource limits and watchdogs before
   execution. Start with `fathomdb-engine` `--features test-hooks --test
   slice90_foreground_dispatch`, in-crate close observations,
   `cargo test -p fathomdb-engine --features operator,test-hooks --lib slice20_open_hook_tests`
   for engine alias controls, and
   `fathomdb-mcp` `--features test-seams --test admission` where that layer is
   affected. Check exact database state and typed outcomes for each schedule.
   Apply the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
   to selected gaps. Defer exhaustive scheduler and VFS sweeps.
2. Reuse the [soak tools](../../../../experiments/slice130-soak/README.md) and
   [prior status](../../../0.8.29/features/slice-130/status.md). Freeze exact
   addon/Node/model bytes, execution flags, fixture, sample window and stop
   rules. Keep shipping artifacts and equivalent instrumented arms distinct.
   Match default profiles without forced GC, allocator trims or heap-flag
   changes. An ON refusal or resource stop may leave OFF unrun. Attribute only
   what was observed and retain the original result. Use a separately selected
   bounded diagnostic pair if needed and authorized.
3. Slice55 owns checkpoint interruption and durable crash/WAL composition.
   Reuse qualified healthy, OOM and software-loss evidence for CUDA here.
   Apply the [held F9–F11 scope](../../../0.8.29/features/slice-110/test-map.md#faultcontrol-matrix-and-deadlines)
   only with confirmed venue/session/resource authority. Require actual Orin
   for the original Orin claim, preserve completed-construction scope and keep
   reset local to the selected child. Do not change the host driver/toolkit.
4. Direct Astra-medium reviews design and real interface/ADR deltas before code.
   Direct Astra-medium critically reviews the reconciled plan, with one FIX-1
   at most and re-review. Commit after PASS and implement when commissioned.
   Classify confirmed product defect, fixture/operator failure, expected
   behavior, bounded non-reproduction or unresolved consequence. For every
   selected behavior change record an actual failing test before GREEN and
   refactor. Passing characterization supplies baseline evidence. Direct
   Sol6.2-high reviews code and direct Terra verifies affected runtime tests
   and native artifacts, including thin installed checks for changed APIs.

## Closure

Record actual exits and limits, preserved original failures, fixed regressions,
and any consequence needing owner disposition. Carry selected R30-TEST
runtime concurrency/resource gaps, their discriminating oracles and dispositions
to70, with final installed-platform applicability in75. Use scoped tests during
development and the required full gate once for the coherent source phase. Update affected contracts, SDKs, changelog and
authorized release state once. Integrate into the owning release branch and
clean only owned inactive worktrees and scratch after proof is retained. Use
direct agents throughout.
