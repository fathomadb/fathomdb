---
title: FathomDB 0.8.27 Slice 90 — code change plan
status: PLANNED
target_release: 0.8.27
---

# Slice 90 code change plan

This document translates the accepted Slice 90 design and
`ADR-0.8.27-engine-owned-runtime-topology.md` into an implementation sequence.
It is a planning artifact only: no production change is implemented here.

The draft was derived at `b4e90afe7292fccbbe41fb98e6ba95c3fbcf00bd`.
Current release source at `e689000d4` includes later Slice 85 recovery code
through `294af94b5`; its exact owner and scanner inventory is the operational
Slice 90 entry. The immutable D27 performance reference remains historical
`7a2f9bf90783f545603516502bac0016d4b93a14`. Recovery qualification is
still open and cannot be inferred from source ancestry. The reconciled
[Slice 90 plan](plan.md) enumerates changes since this draft.

## Outcome and boundaries

Slice 90 must deliver two ordered outcomes:

1. The five advertised engine settings become validated, effective controls
   across Rust, installed Python, and installed Node/TypeScript. Engine-owned
   projection and embedding capacities, universal embedding deadlines, and the
   accepted shutdown behavior are implemented and qualified.
2. The remaining oversized engine root is decomposed into the owners fixed by
   the design without changing behavior beyond the separately reviewed runtime
   correction.

The runtime checkpoint is a hard boundary between these outcomes. No root
extraction starts until the corrected runtime candidate, its installed
bindings, performance evidence, and independent reviews are recorded as a
structured PASS.

This slice does not:

- add Tokio to `fathomdb-engine`;
- move commits to the primary connection or add a dedicated writer thread;
- change the synchronous Rust or Python API model;
- decompose the Python or Node bindings beyond the configuration forwarding
  required here;
- add a public saturation-metrics API or change `CounterSnapshot`;
- route `Engine::write_vector_for_test` through the new dispatcher. That method
  remains the sole temporary direct-provider exception and cannot be runtime,
  timeout, or performance evidence;
- perform unrelated dependency cleanup, whole-crate import normalization, or
  public-surface redesign.

## Current code map

The implementation should begin from these observed seams rather than from a
new abstraction hierarchy.

| Concern | Current owner and behavior | Required change |
| --- | --- | --- |
| Per-engine configuration | Constants and atomics in `fathomdb-engine/src/lib.rs`; no unified open input | Add public `EngineConfig`, typed `EngineConfigurationError`, private resolved configuration, and a configured open path. Keep the existing process-global `RuntimeConfigurationError` distinct. |
| Projection scheduling | `projection_runtime.rs` starts one dispatcher and the fixed `PROJECTION_WORKERS` worker set; `projection_worker.rs` uses fixed inflight and scan limits | Resolve the worker count from `scheduler_runtime_threads`, derive admission as `threads * PROJECTION_COMMIT_BATCH`, and propagate the count through startup, inventory, pause/ack, stop, and failure cleanup. |
| Provider execution | Projection calls use `embed_serialize` plus detached watchdog threads; search, direct embed, and vector-equivalence paths call the provider directly | Add one bounded, engine-owned synchronous embed dispatcher and route every production provider invocation through it. |
| Projection fallback | `embed_projection_batch` can call its per-job fallback while holding `embed_serialize` | Add a bounded regression first; ensure every fallback releases its guard or permit before per-job dispatch. |
| Open | `Engine::open*` converge in root; configuration is not resolved before path, lock, admission, or connection work; the vector-equivalence probe calls the provider before runtime construction | Resolve configuration first, construct embed dispatch before the probe, and unwind partially started runtime state without hiding the original open error. |
| Close | Root `Engine::close` stops projection, reader workers, writer connection, profiles, and lock, then returns success | Split database quiescence from the one-time 30-second embed-runtime drain and report retained provider workers truthfully. |
| Python | Frozen Python `EngineConfig` is accepted by the wrapper but only stored; PyO3 open receives path and default-embedder choice | Forward all five fields through one native configured-open seam while preserving `config=` versus keyword exclusivity. |
| Node/TypeScript | TypeScript snapshots `engineConfig`; NAPI declares five optional `u32` fields but ignores them | Validate JavaScript numbers, use boundary types capable of the accepted safe-integer ranges, convert to Rust configuration, and retain an immutable requested-value snapshot. |
| Structural ownership | The engine root still owns open, runtime configuration, connections, lifecycle, WAL, operator, projectors, and many domain carriers | Move bodies to the exact owners in the Slice 90 design; retain root paths through re-exports where public contracts require them. |

The first implementation batch must expand this table into a machine-readable
or otherwise mechanically checked symbol inventory at current release source. It
must cover every root item, `Engine` method and field, cfg-gated twin, static,
runtime constant, test carrier, caller, and approved destination. An item not
allocated by the accepted design stops the move and requires a design amendment;
it is not assigned opportunistically during extraction.

## Phase 1: freeze the entry evidence

### 1.1 Complete the source and scanner inventories

- Capture public, hidden, test, feature, and qualified-test surfaces of current
  release source before semantic edits, identifying missing Slice 85 recovery
  qualification without substituting the historical candidate's receipts.
- Record exact call sites for every production `Embedder::embed` and
  `embed_batch`. The current list includes projection batch/per-job work,
  ordinary and frozen search, `Engine::embed_text`, and vector-equivalence
  population/checking. Separately label standalone SDK utilities and
  `write_vector_for_test` so they cannot create a false universal-routing claim.
- Record every use of `PROJECTION_WORKERS`, `PROJECTION_INFLIGHT_LIMIT`,
  `PROJECTION_SCAN_FETCH`, embed watchdog state, circuit state, and connection
  inventory literals. This list becomes the dynamic-topology conversion
  checklist.
- Inventory scripts that parse named files or function bodies. At minimum this
  includes the Windows WAL attribution guard, the Slice 85 boundary gate and
  manifest, C1 conformance, public/hidden removal detection, Slice 35 mutation
  coverage, `slice60_fix1_wire`, and plan-anchor checks.

Each source scanner must receive a negative fixture or mutant before its input
is moved. A parser retarget is complete only when the old location fails, the
new location passes, and deletion or weakening of the asserted behavior still
fails.

### 1.2 Land and run the measurement-only D27 harness

- Implement the runner described by
  `d27-runtime-qualification-protocol.json` without changing runtime behavior.
- Verify that the runner enforces the frozen candidate, corpus, seeds,
  operation ratios, warm-up, duration, repetitions, environment invalidators,
  metrics, and median/MAD rule.
- Run it in an exact-source checkout of `7a2f9bf9` and retain hashes for the
  protocol, runner, binary, corpus, environment, and raw output. Historical
  dispatch-only metrics are explicitly unavailable; compare only shared
  operation metrics and require dispatch/resource correctness at the candidate.
- Treat inability to execute on the exact historical source as a blocker. The
  reviewed harness executed there, but its swap-invalid attempts provide no
  qualifying baseline. Under the 2026-10-01 current-host HITL direction,
  Phase 2 RED/GREEN work may proceed while the swap policy is settled; a valid
  historical entry and candidate comparison remain required before the runtime
  checkpoint or structural Phase 3. Never use a post-change measurement as
  the baseline.

## Phase 2: runtime correction

All new behavior follows test-first batches. Keep structural moves out of these
changes so the runtime checkpoint compares a coherent semantic candidate.

### 2.1 Fix the batch fallback self-deadlock

Production target:

- `fathomdb-engine/src/projection_worker.rs`, especially
  `embed_projection_batch` and its transition to `run_projection_job`.

RED coverage:

- Enable `FATHOMDB_PROJECTION_BATCH` in a bounded subprocess fixture.
- Make the batch provider return an error or time out, then prove the per-job
  fallback currently cannot reacquire the same non-reentrant guard.
- Add a separate breaker-open/fast-failure case so it is not mistaken for the
  deadlock witness.
- Add a restoration mutant that puts the fallback back under the guard and is
  killed by a bounded harness without hanging `Engine::drop` or the test binary.

GREEN shape:

- End the guard scope before invoking any per-job fallback.
- Preserve the same invariant when the later embed-dispatch permit replaces
  `embed_serialize`: no fallback may synchronously reacquire a permit it still
  owns.

### 2.2 Add the Rust configuration contract and configured-open seam

Primary targets:

- a new `runtime_configuration.rs` owner;
- `errors.rs` for open-error integration;
- root re-exports and the `Engine::open*` family.

Implementation shape:

- Add public `EngineConfig` with requested optional values and documented
  defaults.
- Add public `EngineConfigurationError` for per-engine validation. Do not reuse
  the existing process-global `RuntimeConfigurationError`, whose SQLite-mode
  meaning remains unchanged.
- Add private `ResolvedRuntimeConfiguration` containing fully resolved native
  widths and checked derived capacities.
- Validate before path canonicalization, lock acquisition, database admission,
  provider construction/warm-up, SQLite connections, or worker creation.
- Add `Engine::open_with_choice_and_config`; make existing open functions
  delegate with defaults so old callers retain behavior.
- Store the requested immutable configuration snapshot separately from mutable
  effective state. `set_slow_threshold_ms` changes only the effective atomic.

Validation contract:

- `scheduler_runtime_threads`: default 2, range `1..=64`.
- `embedder_pool_size`: default 1, range `1..=64`.
- `embedder_call_timeout_ms`: default 30,000, range `1..=u32::MAX`.
- `provenance_row_cap`: default 1,000,000, range `0..=2^53-1`.
- `slow_threshold_ms`: default 100, range `0..=2^53-1`.
- Derive `4 * embedder_pool_size` and
  `scheduler_runtime_threads * PROJECTION_COMMIT_BATCH` with checked arithmetic,
  even though the accepted maxima fit current platforms.

Tests must cover omission, defaults, minima, maxima, zero policy, one-past-max,
overflow, and failure precedence before filesystem mutation. A property test
should cover every scheduler/embedder pair in `1..=64` and assert both derived
capacities remain distinct and exact.

### 2.3 Implement the bounded embed dispatcher

Add a private `embed_dispatch.rs` with no dependency on `Engine`, SQLite,
projection state, search state, or binding runtimes.

The minimal internal model should contain:

- one fixed worker set of exactly `embedder_pool_size` OS threads when a
  provider exists, and no embed threads or queue when it does not;
- one bounded waiting queue of `4 * embedder_pool_size` requests;
- request variants for one text and one provider batch, so a batch receives one
  deadline rather than multiplying the budget by row count;
- an absolute deadline assigned before nonblocking admission and shared by
  queue wait plus provider service;
- reply/cancellation state that lets a caller stop waiting while a started
  provider call continues to completion;
- worker-owned live/retained accounting that survives an engine dropping its
  front-end handles;
- typed internal outcomes for admission full, queued expiry, started timeout,
  provider error, invalid output, timely panic, late result/panic, closing, and
  success.

Admission must never block. A full queue returns saturation immediately.
Workers must discard expired or cancelled queued work before calling the
provider. A started provider call is never forcibly cancelled, never receives a
replacement worker, and keeps its slot occupied until it returns. Timely panics
are transported to the caller's existing panic boundary; late results and late
panics are discarded.

Executor-only tests use rendezvous providers and bounded waits to prove queue
capacity, exact concurrency, queue-inclusive deadlines, no provider call for
expired queued work, late-result discard, worker reuse after timely error or
panic, hung-slot accounting, independent engines, and no-provider allocation.
Do not use sleeps as the synchronization oracle.

### 2.4 Parameterize projection orchestration

Primary targets:

- `projection_runtime.rs`;
- `projection_worker.rs`;
- root inventory and WAL observation helpers that assume two workers.

Changes:

- Replace the fixed worker count with the resolved
  `scheduler_runtime_threads` value.
- Allocate exactly that many projection workers and SQLite connections while
  retaining the existing dispatcher, worker-owned connection, primary writer,
  and `commit_gate` model.
- Replace fixed inflight/scan constants with the checked per-engine admission
  capacity. Carry the capacity through scans, waits, retries, startup reports,
  pause/ack controls, connection inventories, and stop joins.
- Preserve the four shared search-control fields on
  `ProjectionRuntimeShared`, including their defaults and atomics.
- Ensure partial startup failure joins every successfully started SQLite owner
  and releases database admission in the existing safe order.

Tests should exercise 1, 2, 4, and 64 projection workers, exact row admission,
startup failure at each role, shutdown joins, WAL inventory counts, and
independent engines with different settings.

### 2.5 Route projection embedding through dispatch

Replace `embed_with_watchdog`, `embed_batch_with_watchdog`, `embed_serialize`,
live detached-thread accounting, and the session circuit latch on the
production projection path.

Outcome translation must remain stage-aware:

- queue full and queued expiry retain durable pending work, do not consume the
  provider-failure retry budget, and do not leave terminal failure residue;
- started provider failure or timeout uses the existing retry and terminal
  policy;
- invalid dimension/vector behavior keeps its current error and retry meaning;
- close cancellation wakes capacity/retry waits and leaves durable work for a
  later session;
- provider panics remain distinct from retryable embed errors.

Update the PR-9 tests deliberately. The old circuit-breaker oracle is no longer
the contract. With default embed concurrency one, a permanently hung provider
occupies the only slot, later dense work remains pending, and `drain` reports
`EngineError::Scheduler` until that provider returns. Preserve and adapt the
valuable existing assertions for write liveness, no writer corruption, no late
vector commit, finite threads, and recovery.

Add a saturation test that withholds capacity longer than the entire retry
schedule, releases it, and proves the row projects successfully without repair
or a consumed provider-failure attempt.

### 2.6 Route foreground and open-time provider calls

Production call sites:

- ordinary search in `search_api.rs`;
- reader/frozen hybrid search in `search.rs` and its frozen-read entry points;
- `Engine::embed_text` in `embedding.rs`;
- all population and verification calls in `vector_equivalence.rs`.

Required translations:

- ordinary and frozen hybrid search retain same-snapshot sparse fallback and
  existing refusal precedence on dispatch saturation, expiry, or provider
  failure;
- frozen search keeps its existing reader transaction while waiting and while
  producing sparse fallback; it must not reacquire a newer snapshot;
- direct `embed_text` maps queue full and queued expiry to
  `EngineError::Overloaded`, close cancellation to `EngineError::Closing`, and
  started provider failure or timeout to `EngineError::Embedder`; the bindings
  retain the corresponding `OverloadedError`, `ClosingError` and
  `EmbedderError` classes. A completion that linearizes before close retains
  its result; close wins only while pending;
- vector-equivalence failure or timeout retains degraded-open behavior and
  existing baseline/cache mutation rules.

Open must create embed dispatch before the vector-equivalence probe. A timed-out
probe may return a live degraded engine whose provider worker is still occupied.
A later startup failure must cancel waiters, unwind SQLite owners, profile
contexts, lock, and admission, attempt the bounded provider drain, and return
the original open error rather than a cleanup error.

Tests need controlled queue-full, queued-expiry, provider-error, timeout,
panic, late completion, and close-cancellation cases for each operation class.
Extend the existing vector-equivalence suite rather than replacing its detailed
baseline-mutation and degraded-query oracles.

### 2.7 Implement two-phase close

Move lifecycle coordination toward the future `runtime_lifecycle.rs` owner only
after its behavior is green; the final mechanical extraction occurs later.

Phase one, database quiescence:

- atomically stop public and embed admission;
- cancel queued dispatch requests and all result waiters;
- wake projection capacity, retry, reader, and drain waits;
- allow already active database operations to reach their safe boundary;
- stop and join projection workers, reader workers, and every other SQLite
  owner in the existing teardown order;
- uninstall/drop profile callback state only after no connection can call it;
- drop the primary connection, unregister admission, and release the lock.

Phase two, provider drain:

- start one absolute 30-second deadline only after database quiescence;
- share that deadline across all embed-worker joins;
- detach only unfinished provider-only workers after expiry;
- retain worker-owned accounting until they actually return;
- return `EngineError::Scheduler` while retained workers exist and success on a
  later close after they exit;
- never renew the deadline through concurrent close, repeated close, or Drop.

Tests should cover a full reader queue, an active reader query, an active
primary operation, projection waiting for embed capacity, concurrent close,
repeated close, Drop after an incomplete close, provider return after the
deadline, and failed startup. Every retained worker inventory must prove it
owns no SQLite connection, WAL pin, admission guard, or profile context.

### 2.8 Forward Python configuration

Targets:

- `src/python/fathomdb/config.py` and `engine.py`;
- the native stub/declarations;
- `fathomdb-py/src/lib.rs`.

Changes:

- Keep the frozen Python dataclass and current `config=` versus per-knob
  keyword exclusivity.
- Validate Python booleans separately from integers, then negative values and
  upper bounds, without lossy conversion.
- Pass one native configuration object or an equivalent explicit five-field
  native signature to PyO3; convert it to Rust `EngineConfig` and call
  `open_with_choice_and_config`.
- Return/store the requested open snapshot. Do not mutate it when the slow
  threshold setter is called.
- Update the EARP catalog: all five fields now have real consuming open paths;
  `slow_threshold_ms` is no longer supported only through the post-open setter.

Tests must prove actual effects, not just `engine.config`: worker inventory and
provenance pruning including zero through an installed candidate wheel; provider
capacity, timeout, and open-time slow operation and SQLite statement signals
through the configured Rust engine with a caller provider/subscriber. The wheel
must also prove all five values reach the candidate native configured-open path,
along with setter-after-open behavior, invalid types and bounds, mixed-input
rejection, and independent engines. Record the cross-layer evidence mapping and
the existing Python public observation limits; see the design's binding handoff.

### 2.9 Forward Node/TypeScript configuration

Targets:

- `src/ts/src/index.ts` and generated declarations;
- `fathomdb-napi/src/lib.rs`.

Changes:

- Keep one `engineConfig` object and the immutable requested-value snapshot.
- Add TypeScript runtime validation for wrong types, booleans, non-finite and
  fractional values, negatives, unsafe integers, zero on the three positive
  controls, and each field's upper bound.
- Replace NAPI's insufficient `u32` representation for
  `provenance_row_cap` and `slow_threshold_ms` with a boundary representation
  that can accept and validate JavaScript safe integers through `2^53-1`.
- Convert only after validation and call Rust
  `open_with_choice_and_config` inside the existing blocking handoff.
- Do not treat NAPI's Tokio host pool as either engine-owned configured
  executor.

Extend the surface tests with boundary, native-forwarding and directly
observable consuming-effect cases, caller-object mutation, returned-snapshot
mutation, omission, and independent engines. Repeat those cases against the
installed native package and compare generated declarations with the approved
surface delta. Map provider/timeout/slow-event effects to the deterministic
Rust engine-owner tests in the installed receipt; do not imply that the current
public Node open/subscriber API can observe them directly.

### 2.10 Documentation and runtime qualification

Update the ADR index, engine/scheduler/embedder/bindings designs, Rust/Python/
TypeScript interfaces, public configuration reference, and error guidance in
the same semantic correction. Each field must name spelling, units, default,
range, zero and omission behavior, mutability, precedence, consumer,
backpressure, and observable failure/fallback.

Run and retain the D27 matrix:

- default `2/1` with all performance gates;
- minimum `1/1`;
- concurrent `2/2`;
- representative override `4/4`;
- resource-only ceiling `64/64`;
- `2/no-provider`, proving zero embed workers and zero embed queue.

At default, run the exact AC-011a/b selectors plus AC-017, AC-018, AC-029,
AC-072, AC-073, AC-076, and AC-081a/b/c. The mixed workload must record both
contention directions, throughput, p50/p95/p99 latency, queue wait and
saturation, backlog high-water mark, provider concurrency, thread and
connection inventories, close latency, retained workers, and starvation.
Resolve a default miss within the accepted contract before proceeding.

Build candidate wheel and Node artifacts, install them into clean consumers,
and repeat configuration and fault coverage. Then populate the structured
runtime checkpoint with exact candidate and binding SHAs plus hashed PASS
receipts for performance, independent code review, and independent read-only
verification. `scripts/check-runtime-checkpoints.py` must pass, and the binding
commit must precede the first structural-move commit.

## Phase 3: structural extraction

These batches move already-green code. A discovered semantic defect gets its
own test-first correction and review; it is not hidden inside a move.

### 3.1 Runtime configuration and connection primitives

- Finish moving process-global SQLite mode and per-engine configuration into
  `runtime_configuration.rs`, retaining public root re-exports.
- Add `connection_runtime.rs` and move connection opening, extension setup,
  pragmas, lookaside, and profile callback installation/uninstallation.
- Preserve exact ABI types, pragma ordering, error mappings, and feature/cfg
  gates.
- Retarget connection and profile source scanners with their negative fixtures.

### 3.2 Open and admission

- Add `open.rs` and move all `Engine::open*` entry points, `OpenedEngine`,
  `OpenReport`, `EmbedderChoice`, path/lock/admission probes, migrations,
  embedder/reranker gates, GPU witnesses, startup wiring, and open-only cleanup.
- Keep one common open sequence and preserve the tested error precedence.
- Keep edge-vector pruning with open maintenance.
- Leave only `Engine` storage, module declarations, public re-exports, and the
  small path/`ensure_open` controls in root.

### 3.3 Lifecycle and WAL

- Add `runtime_lifecycle.rs` for close, drain, non-embedding drain, and Drop.
- Add `wal_runtime.rs` for checkpoint/truncate orchestration and its reports and
  observations.
- Keep worker-bound snapshot/inventory request arms in `reader_pool.rs`; do not
  move operations away from the connection/thread that must execute them.
- Retarget the Windows WAL attribution guard so it reads the real new owners.
  Its fixtures must still catch missing ordering, missing inventory, wrong
  qualified tests, and cfg-test leakage.
- Re-run shutdown, pause/ack, active transaction, erasure, checkpoint, and
  non-Linux compile coverage after each move.

### 3.4 Index projectors and open maintenance

- Add `index_projector.rs` and move the canonical node/edge projectors,
  `IndexTargetSet`, row-kind mapping, tokenizer reprojection, canonical-row
  loading, and registered derived-projection restoration.
- Add `write_types.rs` for `RowKind` and its implementation.
- Call existing projection, vector, schema, temporal, and write owners rather
  than copying helpers back into a new catch-all module.
- Verify startup repair, projection rebuild, tokenizer upgrade, and vector
  registration behavior against the post-runtime checkpoint candidate.

### 3.5 Operator families

- Add `operator.rs` for embedder verification, integrity checks, safe export,
  schema/row/profile dumps, orphan provenance, report/option carriers, and pure
  formatting helpers.
- Keep data-plane integrity under its feature-gated domain owner and invoke it
  through the operator facade.
- Preserve nonmutating diagnostics, URI/admission rules, exact feature
  availability, and public root paths.

### 3.6 Remaining named carriers and facade wiring

Move only according to the accepted ownership table:

- trace-source carriers to `provenance`;
- dependency measurement to `dependency_trace`;
- embedder-event draining and dense-runtime capability to `embedding`;
- transition helpers to `record_lifecycle`;
- rebuild carriers to `projection_rebuild`;
- projection declaration types to `projection_registry` and runtime status
  types to `projection_runtime`;
- cursor high-water helpers to `write_commit`;
- batch safety to `projection_commit`;
- identity/hash helpers to `identity`;
- open/migration mappings to `errors`;
- event delivery and SQLite diagnostic naming to `lifecycle`;
- counters, profiling controls, threshold state, and RSS helpers to `telemetry`;
- each constant to the domain named by the design.

After each cluster, compare public and hidden surfaces, run the module-boundary
gate, and inspect root for reverse dependencies or wrapper duplicates. Preserve
qualified test names unless an approved test relocation explicitly updates the
surface receipt.

## Phase 4: root closure and final verification

Generate the final symbol inventory from source and compare it with the entry
inventory and approved owner map. The result must have:

- no unclassified production item;
- no wildcard “remaining helper” disposition;
- no duplicate wrapper retained only to make a move appear complete;
- no new governed return path or forbidden module cycle;
- exactly one stored value for each retained shared search control;
- no new production deferral to Slice 100 or 140.

Run the repository-required verification plus separate default, operator,
test-hooks, slice72-test-hooks, benchmark, debug/release, accelerator, and
non-Linux routes. Run installed Python and Node tests and the public/hidden
surface comparisons. Fresh processes must cover runtime-mode initialization,
defaults and overrides, invalid configuration, lock contention, open/close/
reopen, probe side effects, error precedence, and startup/shutdown faults.

Repeat the default performance gates, D27 mixed workload, and exact resource
inventory on the final candidate. If any structural batch changed runtime
semantics rather than moving code, repeat the entire phase-2 matrix. Obtain
independent code review and independent read-only verification at the exact
final candidate before marking Slice 90 complete.

## Suggested commit and review boundaries

Keep every boundary buildable and reviewable. A practical sequence is:

1. entry inventory and measurement harness;
2. batch-fallback deadlock RED/GREEN;
3. Rust configuration and configured-open seam;
4. embed-dispatch core;
5. dynamic projection orchestration;
6. projection dispatch integration;
7. search/direct integration;
8. vector-equivalence/open integration;
9. close protocol;
10. Python/PyO3 forwarding;
11. Node/TypeScript/NAPI forwarding;
12. documentation and runtime qualification;
13. runtime checkpoint;
14. runtime configuration and connection move;
15. open move;
16. lifecycle move;
17. WAL move;
18. index-projector move;
19. operator move;
20. remaining named owner moves;
21. root reconciliation and final receipts.

Semantic batches should normally stay near the design's 300–600
non-mechanical-line target and split above 800 unless cohesion is recorded.
Mechanical moves should normally contain one owner cluster and 300–1,200 moved
lines. These sizes guide review; they do not permit partial completion or an
artificial abstraction.

## Definition of implementation-ready

Implementation may start when the entry symbol/scanner inventories are
complete, the D27 runner has been reviewed and executed against `7a2f9bf9`, and
the first RED test for the batch fallback is reproducible under a bounded
harness. Structural extraction is ready only after the runtime checkpoint is a
validated PASS. Slice 90 is complete only when the final source inventory,
surface comparisons, platform/feature matrix, installed artifacts, performance
evidence, and independent reviews all bind to the same final candidate.
