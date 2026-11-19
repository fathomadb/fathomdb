---
title: 0.8.30 Slice 50 concurrency, lifecycle and resources
status: PROPOSED
slice: 50
target_release: 0.8.30
---

# Slice 50 — Concurrency, lifecycle and resource bug hunt and remediation

## Entry and prerequisites

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
| R30-ROB conditional fault/release | For an authorized checkpoint interruption, show actual delivery and expected recovered canonical, projection and queue state. For selected B30-01 after safe-drop proof, show explicit Rust/Python/TypeScript release, concurrent-use safety, repeat release, healthy reload and error parity. For separately authorized real CUDA loss, require typed refusal, safe teardown and fresh-process recovery on each selected venue. |

## Selected test-coverage additions under seq-406

The owner requests broader explicit robustness coverage using the
[SQLite-method roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md).
The [coverage audit](../../coverage-audit.md) records inherited assertions,
actual .30 proof and remaining gaps separately. These additions are part of
this draft slice plan when commissioned; they do not activate it or grant live
hardware-fault authority. Preserve all original concurrency, ownership and
resource cases above. Erasure remains lower-priority, defects-only regression.

| Local requirement | Bounded implementation and acceptance |
| --- | --- |
| R30-TEST / object cohesion and temporal relationships | Reuse one small real-store fixture with two sources, Unicode and equal-body distinct identities, authored vectors, declared attributes and named relationships. Cover write/commit, truthful projection lag, drain, supersession/update, rollback and fresh-process reopen. Assert canonical revision/source ownership, FTS/BM25 and vector eligibility, graph reachability and exact evidence against independently specified expected facts at each boundary. Modalities may rank differently; async indexes need truthful readiness and correct converged state. Cover node half-open validity and expiry without a generation change, inactive/history flags, filtered bridges and graph direction/kinds/caps. Preserve the actual edge-recency contract: future `t_valid` does not currently block traversal. Add a small seeded real-store operation model with minimized failures; no new ontology or relatedness algorithm. |
| R30-TEST / process crash and WAL composition | Reuse the existing before/after-acknowledged-write child-kill and projection-commit/reopen fixtures. Add only a missing composed seam, at most three selected CPU fault points with healthy controls. Require actual delivered termination, exact committed/uncommitted canonical and dependency/projection work state, unrelated survivors, healthy retry and fresh-process reopen. A held reader performs positive work while writes grow WAL; assert the declared busy checkpoint outcome, reader release/join, successful checkpoint and exact recovered records. Observe owned handles/workers/queues and subsequent positive progress. Watchdogs detect hangs, not a product SLA. This is owned temporary-store process-failure testing; power-loss/torn-write and exhaustive VFS faults remain separate. |
| R30-TEST / real malformed-input fuzz | Select at most three actual query/filter, reference/cursor/frozen-wire or MCP/native argument boundaries. Implement and invoke bounded product-input fuzzing with a valid seed corpus, size/depth limits, nonzero execution counts, reproducible seeds and minimized regressions. Select prospective time/iteration budgets, initially no more than 60 seconds per target and five minutes total per smoke run. Separate property/random testing from coverage-guided fuzzing in results; evaluate and document the coverage-guided target/toolchain route rather than treating queue simulation as product fuzz. Invalid inputs must have declared typed outcomes and no unintended persisted effects; valid inputs use their actual contract. Retain panic/hang and state-integrity oracles. Reuse the existing Proptest/Hypothesis infrastructure and persist discriminating cases; no generic SQLite SQL/B-tree fuzzer or large framework. |
| R30-TEST / doctor outcome and recovery oracles | Inventory all 17 current `DoctorCommand` variants and their real execution paths with70. Extend existing CLI tests only for uncovered outcomes/effects. Prioritize an actual full-integrity findings fixture: an early open refusal or `Clean` result cannot satisfy a findings assertion. Keep open-refusal coverage separate. Add one composed WAL-only export/recover case. Before invoking either CLI route, prove the selected committed record is visible in an isolated source copy with its WAL and absent from a main-file-only copy prepared at the same quiescent boundary; both copies must open successfully and retain an independently chosen control record. Then verify the exported/recovered record and independently inspect canonical/profile/lexical/vector facts through the SDK. Reconcile the seven ignored CLI placeholders with newer engine assertions; replace or retire only demonstrated overlap. Do not invent a CLI manifest-verifier command. Classify read diagnostics, immutable offline inspection, cache/network work, export artifacts and transactional `recompute-mean` separately. |
| R30-TEST / behavioral boundary and oracle sensitivity | Reuse the cohesion fixture for thin Rust/Python/TypeScript and both MCP-format expectations: identity, Unicode, temporal/filter scope, provenance, typed refusals, budgets, epoch/continuation rules and persisted reopen. Keep deep semantics in one engine owner and exercise actual boundary differences. SDK membership counts do not prove equivalent behavior. For selected high-risk predicates use a few plausible wrong-order, swallowed-error or missing-cleanup variants to show the independent oracle detects them; preserve useful regressions rather than adding a broad mutation campaign. |

Record a concise test/feature/oracle map for70, including existing tests reused,
new gaps closed, exact selectors, actual case counts and remaining omissions.
Prefer existing maintained test targets and shared fixtures; keep fast
regressions separate from explicitly selected fuzz/crash qualification runs.
Do not add a new binary or harness per doctor or per failure variant.
The fuzz method and any testing-only toolchain setup require the normal local
design review; no supported product toolchain change is implied. Generic native
memory-safety fuzzing and compound VFS/power-loss campaigns are not claimed by
a parser smoke. A required unavailable proof stays open for owner disposition.

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
- Where do process fault and release claims end? Match checkpoint delivery to
  owned WAL/SHM state and post-reopen proof. Check whether Rust ownership keeps
  model references live through queued work and native execution until actual
  last use. Revise the design and identify unresolved venue or API choices.
  [SQLite WAL](https://sqlite.org/wal.html), [Rust Arc](https://doc.rust-lang.org/std/sync/struct.Arc.html).

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
3. Select one changed checkpoint boundary only with explicit live-fault
   authority. Reuse [Slice 100](../../../../experiments/0.8.29-slice100-checkpoint/README.md)
   healthy and delivered-fault controls, then preserve pre-reopen companions
   and assert exact durable outcomes. Process interruption covers process-crash
   recovery. Reuse qualified healthy, OOM and software-loss evidence for CUDA.
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
concurrency/resource/recovery gaps, their discriminating oracles and dispositions
to 70. Use scoped tests during development and the required full gate once for
the coherent source phase. Update affected contracts, SDKs, changelog and
authorized release state once. Integrate into the owning release branch and
clean only owned inactive worktrees and scratch after proof is retained. Use
direct agents throughout.
