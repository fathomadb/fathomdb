---
title: FathomDB 0.8.27 Slice 90 — test change plan
status: PLANNED
target_release: 0.8.27
---

# Slice 90 test change plan

This document is the test companion to `slice-90-code-change-plan.md`. It
translates the accepted Slice 90 design and runtime ADR into new tests, planned
oracle changes, fixtures, qualification runs, and structural regression gates.
It does not implement or edit tests.

The draft was prepared at `b4e90afe7292fccbbe41fb98e6ba95c3fbcf00bd`.
The operational source entry is current `release/0.8.27` at `e689000d4`,
including Slice 85 recovery code. Historical D27 performance evidence remains
bound to exact engine candidate `7a2f9bf90783f545603516502bac0016d4b93a14`.
The reconciled [Slice 90 plan](plan.md) distinguishes these baselines.

## Test strategy

Slice 90 has two test epochs separated by the runtime checkpoint:

1. **Semantic epoch.** Write genuine RED tests for configuration, bounded
   dispatch, projection scheduling, every production embedding path, shutdown,
   bindings, and performance. Establish the corrected runtime candidate and
   installed-artifact parity.
2. **Structural epoch.** Hold the corrected behavior fixed while moving code.
   Characterization, surface, feature, boundary, and source-scanner tests must
   prove that extraction changed ownership only.

Tests are read-only while a corresponding production fix is in progress. A
test changes only when the accepted contract itself changes, such as the PR-9
watchdog/circuit-breaker expectations superseded by fixed embed workers. Those
oracle changes land as explicit reviewed changes before the production change;
they are not adjusted afterward to make a failure pass.

All engine behavior tests use a real temporary SQLite database. Provider
behavior may use controlled test embedders, but the database, projection,
reader, writer, WAL, admission, and lifecycle paths are not mocked.

## Test organization

The following prospective files keep new Slice 90 concerns cohesive. Exact
names may change before the first RED commit, but the separation of concerns
must remain.

| Prospective test owner | Scope |
| --- | --- |
| `fathomdb-engine/tests/slice90_engine_config.rs` | Rust defaults, ranges, pre-side-effect rejection, derived capacities, consuming effects, and independent engines. |
| `fathomdb-engine/tests/slice90_embed_dispatch.rs` | Bounded queue, deadlines, concurrency, panic/result transport, late discard, no-provider behavior, and per-engine isolation. |
| `fathomdb-engine/tests/slice90_projection_dispatch.rs` | Batch fallback deadlock, projection outcome/retry accounting, dynamic scheduler capacity, saturation recovery, and durable pending work. |
| `fathomdb-engine/tests/slice90_foreground_dispatch.rs` | Ordinary/frozen search, direct embed, same-snapshot fallback, overload, timeout, and close cancellation. |
| `fathomdb-engine/tests/slice90_runtime_shutdown.rs` | Database quiescence, embed drain, concurrent/repeated close, retained-worker accounting, Drop, and startup unwind. |
| Existing `vector_equivalence_probe.rs` | Probe dispatch, degraded open, deadline behavior, and baseline/cache mutation preservation. |
| `src/python/tests/test_slice90_engine_config.py` | Python validation, either/or precedence, actual effects, immutable snapshot, and installed-wheel parity. |
| `src/ts/tests/slice90-engine-config.test.ts` | JavaScript number validation, actual effects, immutable snapshot, and installed-package parity. |
| D27 runner tests under `scripts/tests/` | Protocol enforcement, invalidation, hashes, metric completeness, comparison rule, and checkpoint binding. |

Private executor mechanics should be tested inside `embed_dispatch.rs` when
unit-level visibility is sufficient. Engine-level effects belong in integration
tests. Do not create a public API solely to expose private runtime state. Any
test-only inventory or rendezvous seam must remain cfg/feature-gated, appear in
hidden-surface evidence, and have an explicit removal or retention disposition.

## Shared deterministic fixtures

Build a small reusable set of controlled `Embedder` implementations rather
than duplicating subtly different timing mocks across suites:

- **Rendezvous embedder:** signals entry, waits on an explicit release, records
  active and peak calls, then returns a deterministic vector.
- **Selective blocking embedder:** blocks only tagged inputs so foreground and
  projection contention can be directed independently.
- **Error embedder:** returns a typed provider error on a selected invocation.
- **Invalid-vector embedder:** returns a wrong dimension or invalid vector on a
  selected invocation.
- **Panic embedder:** panics before or after the caller deadline under explicit
  test control.
- **Batch-fallback embedder:** fails `embed_batch` while succeeding or
  independently blocking per-item `embed` calls.
- **Probe embedder:** counts and controls vector-equivalence population and
  verification calls while retaining deterministic reference vectors.

Synchronization rules:

- use barriers, channels, condvars, and bounded subprocesses for causality;
- use elapsed-time bounds only as a final liveness assertion, not to decide when
  a race is ready;
- release every paused worker/provider before asserting or unwinding a fixture;
- give every child process a hard timeout and include captured stdout/stderr in
  failure output;
- use one absolute deadline in the fixture when the production contract uses
  one, so a test cannot accidentally reset the budget between stages.

The fixture API should expose observations, not production decisions: call
count, active count, peak count, entry/release points, and returned outcome.
Assertions about retry, terminal state, fallback, or shutdown remain in the
owning engine test.

## Batch 1: entry and harness tests

Before semantic runtime edits:

- Capture the exact current test inventory, feature requirements, qualified
  test names, public/hidden surfaces, and source-scanner inputs.
- Add self-tests for the D27 runner proving that it rejects the wrong candidate,
  modified protocol, changed corpus/seed, missing metric, invalid environment,
  insufficient repetition count, mismatched hash, and a comparison result that
  violates the frozen median/MAD formula.
- Add a positive fixture with a complete synthetic receipt so the verifier does
  not pass only by rejecting malformed inputs.
- Run the measurement-only harness against `7a2f9bf9` and bind protocol, runner,
  corpus, binary, environment, and raw-output hashes. Historical dispatch-only
  queue and fixed-worker metrics are unavailable, never fabricated as zero;
  candidate dispatch/resource metrics remain mandatory correctness evidence.
- Preserve raw measurements. A summarized PASS without reproducible or retained
  raw output is not an entry oracle.

The D27 self-tests must be runnable without the expensive workload. The actual
entry run is a separate candidate-bound qualification step.

## Batch 2: batch fallback deadlock

Add the first RED test before any executor work:

1. Start a child test process with `FATHOMDB_PROJECTION_BATCH` enabled.
2. Write enough vector rows to enter `embed_projection_batch`.
3. Make `embed_batch` return an error or controlled timeout.
4. Allow per-item embed to succeed.
5. Assert bounded drain and successful projection.

On the current code the child must hit the under-guard fallback deadlock and
exceed the bound. GREEN requires the fallback to execute after releasing the
guard. The test must also prove it reached the batch failure path; a fast skip
to per-job processing is a false green.

Add two companion checks:

- breaker/session-latch fast failure is not accepted as the deadlock witness;
- a mutation script that restores fallback-under-guard fails the bounded test
  and restores the source reliably on pass, failure, signal, or timeout.

Retain this test after `embed_serialize` disappears. Its invariant becomes
“per-job fallback never reacquires an executor permit still owned by the batch
path.”

## Batch 3: configuration validation and consuming effects

### Rust configuration tests

Add table-driven cases for all five fields:

| Field | Valid cases | Invalid cases | Required effect oracle |
| --- | --- | --- | --- |
| `scheduler_runtime_threads` | omitted/default, 1, 2, 4, 64 | 0, 65, native overflow | Exact projection worker/connection count and `N * PROJECTION_COMMIT_BATCH` admission. |
| `embedder_pool_size` | omitted/default 5, explicit 1, 2, 4, 64 | 0, 65, native overflow | Exact provider concurrency and queue capacity `4 * N`; no provider means no workers/queue. |
| `embedder_call_timeout_ms` | default, 1, representative, `u32::MAX` | 0, `u32::MAX + 1`, overflow | One queue-plus-service deadline used by every production provider path. |
| `provenance_row_cap` | default, 0, 1, representative, `2^53-1` | above `2^53-1`, overflow | Real write and actuation commits retain/prune rows according to the configured cap; zero keeps existing disable semantics. |
| `slow_threshold_ms` | default, 0, representative, `2^53-1` | above `2^53-1`, overflow | Operation and SQLite-statement slow events use the open-time value; setter changes effective threshold only. |

Add a property test over every scheduler/embedder pair in `1..=64`. Assert
checked projection and embed queue capacities, their independent values, and
the exact accepted resource formula. This is a derivation test, not a thread
spawning test for all 4,096 pairs.

Validation-order tests must pass an invalid config with a path whose creation,
canonicalization, lock, admission, provider warm-up, or database open would be
observable. Assert the typed configuration error wins and no file, sidecar,
lock, connection, worker, or provider call exists afterward.

Run fresh-process cases alongside the existing process-global
`runtime_configuration.rs` suite to prove `EngineConfigurationError` and the
SQLite-mode `RuntimeConfigurationError` remain separate and retain their own
precedence.

### Binding validation tests

Python cases:

- frozen `EngineConfig` and per-knob keyword forms resolve identically;
- `config=` plus any per-knob keyword rejects before native open;
- booleans, non-integers, negatives, zero on positive controls, and each upper
  bound reject with the documented Python type;
- arbitrary-precision integers are checked before conversion;
- `engine.config` is the requested open snapshot and is unchanged by the slow
  threshold setter;
- two live engines may have different effective settings.

TypeScript/NAPI cases:

- reject strings, booleans, `NaN`, infinities, fractions, negatives, unsafe
  integers, zero on positive controls, and each upper bound;
- accept zero for provenance cap and slow threshold;
- verify both values at `Number.MAX_SAFE_INTEGER` and reject the next value;
- mutate the caller's object after open and attempt to mutate `engine.config`;
  neither may change the snapshot;
- prove NAPI's host Tokio pool does not change when either engine-owned capacity
  changes.

Wrapper surface tests may assert the snapshot, but the Rust engine owner must
prove all five real consuming effects. Installed Python and Node must prove
native forwarding for all five and each directly observable effect (including
scheduler inventory and provenance retention). Their receipts map provider,
timeout, and slow-event effects to the owner-level tests and state the existing
public binding observation limits.

## Batch 4: embed dispatcher core

Add private unit tests for the state machine and engine integration tests for
observable behavior.

### Capacity and admission

- With pool size 1, hold one running call and fill exactly four queued slots;
  the next request returns overload without blocking.
- With pool sizes 2 and 4, hold all workers and prove the exact `4 * N` waiting
  capacity independently of projection admission.
- With no provider, prove zero embed workers, zero queue allocation, and the
  existing not-configured outcome.
- Start two engines with different pool sizes and prove no shared queue,
  semaphore, worker, or accounting.

### Deadline and cancellation

- Assign the deadline before enqueue, consume part of it in the queue, and
  prove service receives only the remaining budget.
- Expire a queued request and prove the provider call count does not increase.
- Time out a started request, release it later, and prove its result is
  discarded and cannot be published or committed.
- Close while requests are queued and while callers await started work; all
  waiters receive the operation-appropriate closing outcome.
- Prove repeated polling or caller retries cannot renew one request deadline.

### Errors and panics

- A timely provider error is returned distinctly from overload and timeout.
- A timely panic reaches the established caller panic boundary.
- A late panic is discarded after the caller has timed out or closed.
- After a timely error or panic, the fixed worker continues accepting work.
- After a hung call, no replacement/reaper worker appears; concurrency remains
  reduced until that exact worker returns.

Peak-concurrency assertions must come from the controlled provider's active
counter. Thread-name counts alone are supporting inventory, not proof that
provider calls actually overlapped.

## Batch 5: projection integration

Update or extend `projection_runtime.rs`, `pr9_embed_watchdog.rs`, and
`pr9_embed_serialization.rs` deliberately.

### Dynamic scheduler tests

- For scheduler sizes 1, 2, 4, and 64, assert exact startup roles, worker-owned
  SQLite connections, projection admission, inventory replies, pause/ack
  behavior, and shutdown joins.
- Inject startup failure at the dispatcher and at early/middle/final projection
  workers. Every already-started connection and thread must unwind.
- Run two engines with different scheduler sizes and prove their capacities and
  inventories remain independent.
- Preserve worker-zero pause/ack coverage by selecting an actual worker index
  from the resolved topology rather than assuming exactly two workers.

### Projection outcome tests

- Admission full and queued expiry leave the row durably pending and do not
  increment provider-failure attempts or terminal residue.
- Hold saturation longer than the entire configured retry schedule, release
  capacity, and prove projection succeeds without manual repair.
- Started provider error and started timeout consume the existing retry budget
  and reach the current terminal policy.
- Invalid vector/dimension retains its current typed outcome and retry policy.
- Close cancellation leaves accepted writes durable and repairable on reopen.
- Batch and per-item paths produce equivalent successful vectors and failure
  accounting.

### Intentional PR-9 oracle changes

Replace the obsolete expectations that detached watchdog calls are abandoned
and a session circuit breaker lets later work continue. Under an explicit
pool size one:

- a permanently hung provider occupies the only slot;
- later dense work remains pending;
- `drain` reports `EngineError::Scheduler` while the slot is occupied;
- canonical writes remain live and durable;
- no late vector is committed after its waiter has timed out or closed;
- work recovers after the original provider call returns;
- the engine creates no unbounded detached threads.

Retain a pool-size-two test showing one hung slot does not prevent the other
slot from making bounded progress. The selected default five has the same
fixed-slot rule: one hung call retains one slot, and five hung calls exhaust
capacity. Adapt the serialization test into exact
configured concurrency: peak one at size one and peak two at size two, never
more than the configured pool.

## Batch 6: foreground and open-time paths

### Direct embed

For `Engine::embed_text`, test success, overload, queued expiry, started
provider error, started timeout, panic, close cancellation, and recovery. Assert
the exact public mappings: queue full and queued expiry produce
`EngineError::Overloaded`/binding `OverloadedError`; started provider error and
timeout produce `EngineError::Embedder`/binding `EmbedderError`; close while
pending produces `EngineError::Closing`/binding `ClosingError`. A result
linearized before close remains that result, and configured-embedder/refusal
validation retains its earlier precedence.

### Ordinary and frozen hybrid search

- Saturate the embed dispatcher and prove sparse fallback still serves under
  the existing hybrid policy.
- Pause after the frozen reader transaction is established, cause dispatch
  failure, mutate the database from another allowed path, then release. The
  fallback must reflect the original authenticated snapshot, not a reacquired
  transaction.
- Preserve refusal precedence for vector-equivalence degradation, malformed
  frozen context, and closing.
- Prove the embedding timeout bounds only provider-related snapshot retention;
  it is not treated as a total query deadline.
- Cover query/provider panic without converting it into silent sparse fallback
  when the existing panic boundary requires propagation.

Extend existing projection and frozen-read suites where their fixtures already
establish the authoritative transaction. Do not replace their current tamper,
database-mismatch, context-drift, eligibility, deduplication, or graph-cap
assertions.

### Vector-equivalence open probe

Extend `vector_equivalence_probe.rs` with:

- proof that population and verification calls enter embed dispatch;
- queue-inclusive timeout yielding a live degraded engine;
- an occupied provider slot retained by that live engine after timeout;
- no baseline, cache marker, or prospective-arm mutation on failed/timed-out
  population;
- unchanged degraded query refusal and FTS-only service;
- late result/panic discard;
- post-probe startup failure that preserves its original error while cleaning
  every database owner;
- reopen after provider recovery.

Keep all existing malformed baseline, identity, mean-centering, P1/P2,
partial-row, and degraded-report tests. New tests supplement those oracles.

## Batch 7: shutdown and startup failure

Use real database operations and explicit release points for the two-phase
close contract.

### Database quiescence

- Fill the reader request queue, begin close, and prove the stop/cancel signal
  is not trapped behind ordinary work.
- Hold an active reader query and separately hold an active primary/writer
  operation. Close must wait for the safe boundary and must not drop connection
  or profile state early.
- Hold projection on embed capacity and prove cancellation wakes its capacity
  and retry waits before projection-worker join.
- After quiescence, assert no SQLite owner, WAL pin, admission record, runtime
  probe, or profile callback context remains.

### Embed-runtime drain

- Use a test-only shortened drain budget while preserving the production
  one-deadline logic.
- Hold multiple provider workers across the deadline and prove they share one
  absolute budget rather than receiving it sequentially.
- Assert incomplete close returns `EngineError::Scheduler`, retained workers
  own provider/accounting state only, and no replacement or reaper exists.
- Call close concurrently and repeatedly; there is one teardown and no deadline
  renewal.
- Release the providers and prove a later close succeeds and live accounting
  reaches zero.
- Drop after incomplete explicit close and prove Drop neither blocks for a new
  budget nor reports false completion.

### Startup unwind

Inject failures after configuration, lock/admission, writer connection, reader
pool, projection workers, vector-equivalence timeout, and profile installation.
Assert the original failure is returned and cleanup does not replace it with a
secondary scheduler result.

## Batch 8: binding and installed-artifact parity

Source-tree wrapper tests are insufficient because a stale native extension can
make them pass against the wrong ABI. Build candidate artifacts and install them
into clean environments.

For both Python and Node, run:

- omitted/default configuration;
- every valid minimum and maximum;
- every invalid/overflow/wrong-type case;
- installed positive controls for scheduler inventory and provenance retention,
  plus exact native configured-open forwarding of all five settings;
- independent simultaneous engines;
- open/close/reopen and failed-open cleanup;
- snapshot immutability and slow-threshold setter behavior;
- generated stub/declaration and runtime-export comparisons.

Prove provider concurrency, timeout, and slow-operation/SQLite-statement event
effects with deterministic caller providers and a lifecycle subscriber at the
Rust engine boundary. Existing Python/Node public opens expose neither a
caller provider nor delivered subscriber events. Do not add a test-only binding
adapter solely for duplicate effect witnesses. The installed receipts must map
each setting to its native forwarding check, owner-level consuming-effect test,
and any installed binding-level observable effect, stating the observation
limits explicitly.

The installed tests must record candidate SHA, artifact SHA-256, platform,
architecture, language/runtime version, feature set, and exact command. A local
source import or workspace-linked native library is a failure, not evidence.

## Batch 9: performance and resource qualification

Pin candidate-only instrumentation with test-first fixtures before the costly
run: the default-only workload must fail the candidate observation check;
missing or duplicate engine request IDs and unowned records must fail; each
measured foreground sequence and committed projection cursor must have at
least one valid engine-owned request. Retries and per-job fallback may produce
multiple requests per owner. Review the typed hook-to-JSON handoff so a
fabricated `source: "engine"` label cannot stand in for engine evidence.
Candidate raw without an engine-observed scheduler count of two, or with an
embed count outside the approved `2..=64` sweep, must fail the default
comparison check. The workload opens the engine's defaults; candidate-bound
tests and the receipt pin the selected default `2/5`.
Preserve raw admission/start/terminal instants,
resolved worker counts, projection admission high-water and exact live
thread/SQLite inventories. The retained historical bundle is revalidated from
its own hashed artifacts under the common protocol and corpus. The candidate
runner bundle may differ because it adds observations, without changing the
historical workload or comparison formula. A self-test confirms candidate-only
fields do not change historical recomputation or median/MAD evaluation.

For open-time configuration effects, RED tests must fail if either the
configured provenance cap or slow threshold is ignored at open. Prove cap
zero/one/nondefault through real writes and post-sweep retention, and slow operation plus
SQLite-statement signals at configured zero/nondefault thresholds, followed by
the setter changing only effective state. A crate-local shortened drain-budget
fixture must show prompt database quiescence, zero SQLite owners and a truthful
`Scheduler` result while the provider remains held; then release it and prove
successful final close/reopen. Keep the production 30-second budget unchanged.

Before rerunning D27, independently review the HITL `seq-297` amendment.
RED/GREEN tests pin 128-page combined swap movement as valid, 129 pages as
invalid, missing or decreasing counters at any sample as invalid, and exact
raw-to-receipt deltas. Complete entry and candidate receipts with in-bound
nonzero per-child movement must validate, replacing the old six-repetition
global zero-swap check. The same rule applies to entry and candidate. Other
host invalidators remain strict. Old invalid attempts remain tied to their old
protocol hashes and cannot qualify retroactively.

Run the frozen D27 matrix:

| Cell | Purpose |
| --- | --- |
| `2/5` | Selected default, entry comparison, and all release performance gates. |
| `2/1` | Explicit prior default, one-slot overload and cleanup behavior. |
| `1/1` | Minimum-bound progress, backpressure, timeout, reopen, and cleanup. |
| `2/2` | Real provider overlap, mixed foreground/projection contention, and isolation. |
| `4/4` | Representative larger override and bounded resource scaling. |
| `64/64` | Exact maximum resource inventory and cleanup; no throughput claim. |
| `2/no-provider` | No embed workers, queue, or idle provider resources. |

At default, run the unchanged selectors for AC-011a/b, AC-017, AC-018, AC-029,
AC-072, AC-073, AC-076, and AC-081a/b/c. Do not lower thresholds, shorten the
frozen measurement, change datasets, or substitute a development build.

The D27 receipt must contain commit throughput; foreground query and projection
p50/p95/p99; queue wait/saturation; durable backlog high-water mark; provider
concurrency; thread and SQLite-connection inventories; close latency and
retained workers; and progress/starvation in both contention directions.

Run provenance-cap and slow-threshold effect tests in this matrix without
counting them as executor-capacity evidence. `write_vector_for_test` is excluded
from every timeout, concurrency, lock-order, and performance oracle.

## Batch 10: runtime checkpoint tests

Extend the structured checkpoint verifier and its self-tests to reject:

- a missing receipt or non-PASS status;
- a receipt bound to a different candidate;
- a wrong SHA-256;
- a binding artifact built from a different SHA;
- a runtime checkpoint commit that does not precede the first structural move;
- missing independent code review or independent read-only verification;
- a performance receipt that omits a required matrix cell;
- a path outside the declared Slice 90 evidence set.

Include one complete positive fixture and mutation cases for every required
field. The runtime checkpoint must pass before any structural-move test baseline
is captured.

## Structural extraction regression plan

Structural batches add few new behavioral tests. Their job is to rerun focused
owners and strengthen structural guards before changing their inputs.

### Runtime configuration and connection moves

- Run process-global runtime configuration, per-engine config, admission,
  profile callback, lookaside, pragma, open-error precedence, and fresh-process
  tests.
- Add source-scanner mutants proving the new module locations are read and the
  old root location no longer supplies a false pass.
- Compare public Rust paths before and after re-exporting.

### Open move

- Run lock contention, path, header/schema/WAL probe, migration, embedder and
  reranker policy, vector-equivalence, GPU witness, and every startup-failure
  injection.
- Verify identical side effects and error precedence against the post-runtime
  checkpoint candidate.
- Run removal detection so a re-export cannot hide a missing function body or
  method.

### Lifecycle and WAL moves

- Run the complete Slice 90 shutdown suite plus existing drain, projection,
  erasure, checkpoint, truncate-WAL, reader snapshot, and WAL attribution tests.
- Retarget the Windows WAL guard with negative fixtures for wrong source,
  missing ordering, missing qualified test, and cfg-test leakage.
- Preserve reader-pool request arms on their worker-owned connection and test
  pin/ack/release ordering.

### Index projector and operator moves

- Run projection registry/generation/status/rebuild, tokenizer reprojection,
  vector registration, startup repair, nested-source projection, and terminal
  failure suites.
- Under `operator`, run integrity, export, schema/row/profile dump, provenance,
  dependency, erasure, truncate-WAL, and data-plane checks.
- Compare default versus operator surfaces so gated methods neither leak nor
  disappear.

### Remaining carrier moves and root closure

- Run each existing domain's focused tests immediately after its types/helpers
  move.
- Update the recovered Slice 85 boundary classification. Add negative fixtures
  for each distinct enforced boundary rule or new forbidden return path;
  do not rebuild a frozen edge census.
- Retarget C1, Slice 35 mutation coverage, `slice60_fix1_wire`, hidden/public
  surface, removal detection, plan anchors, and any root-specific scanner.
- Require a mutant proving each retargeted scanner still detects the behavior
  it protected before the move.
- Compare qualified test identities and feature requirements; unexplained test
  renames, cfg changes, or target disappearance fail the batch.

## Required execution tiers

Use the narrowest tier that can falsify a batch, then run broader gates at the
runtime checkpoint and final candidate.

| Stage | Required execution |
| --- | --- |
| Each RED/GREEN semantic batch | New exact test, affected existing suites, lint/typecheck for touched crates/bindings, relevant source scanners. |
| Runtime checkpoint | Full default verification; operator, test-hooks, binding builds; installed Python/Node suites; D27 matrix; named release performance gates; public/hidden surfaces; independent reviews. |
| Each structural move | Owning behavior suites, module-boundary gate, retargeted scanner mutants, surface/removal checks, affected feature builds. |
| Final candidate | Full repository gate plus distinct default, operator, test-hooks, slice72-test-hooks, benchmark, debug/release, accelerator, and non-Linux routes; installed artifacts; repeated default/D27/resource qualification. |

Do not substitute an all-features build for incompatible feature routes. Record
toolchain, target, platform, features, and candidate SHA for every receipt. An
unavailable required non-Linux or accelerator executor blocks closeout rather
than becoming an inferred pass.

## Requirement coverage

| Requirement | Primary test evidence |
| --- | --- |
| R27-90A | Entry/final source inventories, removal detection, root-residual comparison, qualified-test inventory. |
| R27-90B | Config validation/effects, dispatcher, projection, foreground, probe, binding, and installed-artifact suites. |
| R27-90C | Fresh-process open/admission/runtime-mode/error-order and startup-failure tests. |
| R27-90D | Database quiescence, embed drain, WAL, reader ownership, concurrent/repeated close, and retained-worker tests. |
| R27-90E | Projector, registry, rebuild, operator, integrity, export, and feature-surface suites. |
| R27-90F | Shared search-control default/atomic/lifetime characterization and hidden-surface inventory. |
| R27-90G | Rust/Python/Node surfaces, stubs/declarations, feature matrix, installed artifacts, and non-Linux compile. |
| R27-90H | Slice 85 boundary gate, graph classifications, source-scanner mutants, C1, Slice 35, and removal detection. |
| R27-90I | Runtime-checkpoint verifier/self-tests, independent reviews, candidate-bound receipts, and final matrix. |
| R27-90J | Bounded batch-fallback RED/GREEN plus restoration mutant. |
| R27-90K | D27 protocol self-tests, entry run, full matrix, named performance gates, resource inventory, and final rerun. |

## Completion criteria

The test plan is implementation-ready when the prospective test owners and
fixtures have been reviewed, the source/scanner inventory is complete, the D27
self-tests pass, the entry run is bound to `7a2f9bf9`, and the batch-fallback
RED test fails for the intended reason under a bounded harness.

The semantic epoch is complete only when the runtime checkpoint binds the same
qualified candidate used by Rust and installed Python/Node tests. The structural
epoch is complete only when every moved owner retains its behavior and scanner
oracles, the final source/surface inventories reconcile, and the final candidate
passes the repeated performance, resource, platform, and feature matrices.
