---
title: D27 runtime topology Option B — successor ADR scaffold
status: REVIEW-SOURCE-SUPERSEDED-BY-ACCEPTED-ADR
target_release: 0.8.27
initial_review_base: 9b00a980ca222d05f3af063168ed45c2b0a4a526
corrected_review_base: 418240673f8a22027a67330c2c21cd19b773bb47
---

# D27 runtime topology Option B — successor ADR scaffold

This is the design-review source for the accepted
[`ADR-0.8.27-engine-owned-runtime-topology`](../../../../adr/ADR-0.8.27-engine-owned-runtime-topology.md),
not a second implementation authority or evidence receipt. HITL decision
`seq-293` selected Option B's direction; `seq-295` accepted its reviewed
specifics and clause-level supersession map. Historical proposal/approval
wording below records the review chronology. The ADR and Slice 90 design own
the current normative contract and implementation acceptance.
The latest candidate-bound review and its limitations are recorded in
[`independent findings resolution`](independent-findings-resolution.md).

## Proposed decision

Retain the synchronous engine API and the current projection/commit ownership
model. Replace the historical Tokio-task/dedicated-writer topology with two
separate, bounded, engine-owned synchronous executors:

1. a projection-orchestration executor whose worker count is controlled by
   `scheduler_runtime_threads`; and
2. an embed-dispatch executor whose worker count and maximum simultaneous
   provider calls are controlled by `embedder_pool_size`.

All production engine-owned embedding paths—open-time vector-equivalence probes,
projection, ordinary and frozen search/query, and direct `Engine::embed_text`—
submit to the embed-dispatch executor and observe one dispatch/deadline
contract. Model construction, warmup, and `Embedder::identity` remain separate
open-time operations; this decision does not claim they can be cancelled. The
universal deadline covers engine calls to `Embedder::{embed,embed_batch}`, not
the separate cross-encoder reranker or standalone SDK embedding utilities.
Provider-internal calls within the default `embed_batch` implementation share
the enclosing batch deadline; they do not create nested dispatch requests. The
caller-facing Rust/Python API remains synchronous. Bindings may move a complete
synchronous engine call off their event-loop thread, but neither a binding
pool nor a projection worker may invoke the embedder directly.

The default-compiled, `#[doc(hidden)]` `Engine::write_vector_for_test` method is
the sole temporary exception through Slice 90. Slice 140 removes it from
default public builds. It may not be used as deadline, dispatch, queue-bound,
lock-order, or performance evidence, and the exception does not extend to any
production path or another test seam.

Projection workers retain their own SQLite connections and serialize their
write commits through `commit_gate`. The primary caller writer remains the
mutex-protected `Engine.connection`. This successor does not introduce Tokio
into `fathomdb-engine`, a dedicated writer OS thread, or a scheduler-to-writer
commit channel.

### Authority changed and retained

This successor would replace the conflicting decision and consequence clauses
in:

- `dev/adr/ADR-0.6.0-scheduler-shape.md` (Tokio orchestration tasks, historical
  pool defaults, task/admission units, dedicated writer channel, shutdown and
  the consequences that require Tokio/task abandonment);
- `dev/adr/ADR-0.6.0-single-writer-thread.md` (dedicated writer OS thread,
  channel ownership, prohibition on projection-worker writers, and total
  submission-order claims; SQLite single-write serialization remains);
- `dev/adr/ADR-0.6.0-embedder-protocol.md` Invariant 4's CPU-count default and
  exact executor description, plus Invariant 5's deadline scope to include
  queue wait and fixed batches, while retaining the provider trait and
  finish-and-discard rule;
- `dev/adr/ADR-0.6.0-projection-model.md` Granularity's scheduler-entry batch
  unit, Backpressure's unbounded-internal-queue/single-writer/adapter-shedding
  prescriptions, and Restart durability's cursor-only discovery recipe:
  replace these with bounded admitted rows, batching at worker dequeue,
  generation/terminal-aware durable rediscovery, and the operation-specific
  admission outcomes below. Retain push dispatch, fixed batch deadlines,
  provider retry schedule, and terminal-state cursor authority; and
- `dev/adr/ADR-0.6.0-async-surface.md` Decision/Consequences and ASYNC-1's exact
  ThreadsafeFunction, binding-pool sizing/configuration, and internal Arc/async
  prescriptions. Retain Promise/off-event-loop/libuv isolation and A–D's
  product outcomes without requiring the historical scheduler mechanism.

Codification must name these clauses in the successor and add reciprocal
supersession pointers plus the decision-index entry. The current
`dev/design/engine.md` is code-grounded evidence, not a formal supersession of
the writer ADR. Its Close path's total-bounded-close assertion must be amended
to the two-phase contract below. Keep the freshness SLI and current
generation/mean/commit authority unchanged; a different default pool size is
not a waiver of their qualification gates.

It retains these load-bearing contracts:

- post-commit projection dispatch and no embed work under the originating
  write transaction (`ADR-0.6.0-async-surface.md:47-70`);
- synchronous Rust/Python/CLI engine surfaces
  (`ADR-0.6.0-async-surface.md:41-45`);
- engine-owned embed execution, no engine re-entry, eager warmup, universal
  deadlines, and finish-and-discard late completion
  (`ADR-0.6.0-embedder-protocol.md:63-117`);
- one SQLite write transaction at a time, explicit write serialization, and
  no `SQLITE_BUSY` retry topology masquerading as concurrency
  (`ADR-0.6.0-single-writer-thread.md:28-52,111-133`); and
- the current authoritative primary-writer/projection-worker/reader split
  (`dev/design/engine.md:52-69`) and post-commit publication rule
  (`dev/design/engine.md:98-111`).

The successor retires the exact TypeScript ThreadsafeFunction prescription but
retains its purpose: Promise-returning APIs, no synchronous engine work on the
event-loop thread, and isolation from libuv's general blocking pool. NAPI's
current `tokio::task::spawn_blocking` handoff
(`src/rust/crates/fathomdb-napi/src/lib.rs:591-613`) must be qualified against
those outcome requirements in Slice 110; it is not evidence of an engine-owned
executor. Slice 110 may move that already-correct handoff but may not implement
the missing engine executors or redefine engine knobs as binding-pool knobs.

## Why this is the narrow successor

The current engine already has the SQLite-safe ownership model that the
release must preserve:

- a mutex-serialized primary caller writer;
- two projection workers with worker-owned SQLite connections;
- expensive embedding outside the SQLite commit critical section;
- projection commits serialized by `commit_gate`; and
- a separate eight-connection reader pool.

References: `dev/design/engine.md:52-69,125-140`,
`src/rust/crates/fathomdb-engine/src/projection_runtime.rs:121-127,402-432`,
and `src/rust/crates/fathomdb-engine/src/lib.rs:576-598`.

SQLite and rusqlite require disciplined connection ownership and write
serialization; post-commit background dispatch is a FathomDB contract. They
do not require Tokio, a dedicated writer OS thread, or an MPSC commit channel.
Replacing the working
commit topology solely to make a historical ADR literal would enlarge Slice
90, disturb WAL/thread inventories, and combine a behavior-changing writer
rewrite with the release's structural root closure.

## Runtime ownership

### Projection orchestration

`projection_runtime` owns:

- one dispatcher thread that discovers durable pending work;
- `scheduler_runtime_threads` projection-orchestration workers;
- one worker-owned SQLite connection per orchestration worker;
- the bounded in-memory job queue and idle/drain state; and
- the existing shared `commit_gate` used by projection commits and mean
  recomputation.

An orchestration worker may classify work, perform generation checks, submit
an embed request, wait for its result, prepare projection state, and commit
under `commit_gate`. It never invokes `Embedder::embed` or `embed_batch`
directly. The durable database backlog remains the authority when the bounded
in-memory queue is full.

The in-memory projection admission bound is
`active_rows + queued_rows <= scheduler_runtime_threads *
PROJECTION_COMMIT_BATCH`, preserving the current row-count relationship
(`src/rust/crates/fathomdb-engine/src/projection_worker.rs:156,217,253,384-385`)
while making the worker factor configurable. This is count-bounded, not
byte-bounded: bodies, vectors, and provider scratch space vary in size, so
admission occurs before unnecessary request-payload copies. Multiplication is
checked before any thread or connection is created. Queue saturation delays
generation/terminal-aware discovery; it does not reject a committed canonical
write, terminally fail durable pending work, or allocate one future per row.

### Embed dispatch

A new private `embed_dispatch` owner sits below open-time equivalence,
projection, search, and direct embedding. It is constructed after provider
warmup but before the first vector-equivalence probe or production inference.
It owns:

- `embedder_pool_size` long-lived worker threads;
- a bounded FIFO queue with capacity `4 * embedder_pool_size`;
- the sole production calls to `Embedder::embed` and `embed_batch`;
- request deadlines, start/result channels, late-result discard, panic
  capture, active/queued/timed-out-live accounting, and the circuit breaker;
  and
- shutdown of queued work and bounded waiting for running work.

The old `thread::spawn` per call in
`src/rust/crates/fathomdb-engine/src/embedding.rs:26-117` is retired. A running
provider call is never forcibly cancelled. If its receiver deadline expires,
the caller receives the existing mapped embedder failure, the result channel
is closed, and the worker discards any later result. The worker slot remains
occupied until the provider returns; it is not falsely advertised as free.

Admission is bounded and nonblocking. The caller creates one absolute monotonic
deadline before enqueue; a full queue returns internal overload immediately,
and an accepted request carries that deadline through queue wait and service.
A queued request whose deadline expires is cancelled or skipped without
invoking the provider. Worker start atomically checks request state and the
absolute deadline; it cannot start an expired or cancelled request. No queue
or request-state lock is held during provider code. A running request that reaches the deadline becomes a
timed-out-live call until it returns. Result arrival versus expiration is
linearized under the request state so exactly one outcome wins. This prevents
queue admission or wait from evading the universal timeout and prevents dead
queued entries from retaining capacity.

`embed_batch` receives one fixed `embedder_call_timeout_ms` deadline per
provider invocation, matching the accepted projection-model granularity. The
current `timeout * item_count` behavior in `projection_worker.rs:549-570` is
retired rather than promoted into the successor. Projection overload or
cancellation defers durable work for bounded rediscovery/retry; it does not
record a permanent projection failure merely because the shared dispatch queue
was transiently full.

### Provider concurrency and PR-9

The default healthy projection embed concurrency remains one, preserving the PR-9
safety contract proven by
`tests/pr9_embed_serialization.rs:1-16,87-128`. The prior implementation
serialized caller-supplied providers defensively, partly motivated by future
binding-bridge safety concerns. Current custom providers use the Rust
`Send + Sync` surface; the bindings have no such bridge today. Concurrent
Candle calls were measured safe but throughput-neutral.

Proposed successor rule:

- omitted `embedder_pool_size` resolves to `1`;
- an explicit value `N > 1` is an explicit request to permit at most `N`
  simultaneous calls on that engine's shared embedder;
- the engine still owns and bounds all calls; and
- the current Python and TypeScript bindings continue to support only the
  built-in/default-or-none choices. Their deferred custom-embedder bridges are
  not added by this work. A future bridge must define GIL/JS thread affinity,
  teardown, concurrency, no-reentry, and callback rules before exposing custom
  providers.

This changes the historical CPU-count default in
`ADR-0.6.0-scheduler-shape.md:26-31` and
`ADR-0.6.0-embedder-protocol.md:83-89`, but preserves the behavior users have
actually received on the projection path since PR-9. It deliberately changes
the combined-engine contention model because query/direct calls currently
bypass that mutex; mixed foreground/background load and frozen-reader WAL
retention therefore require qualification. Rust's existing `Embedder: Send +
Sync` contract is sufficient for the current explicit opt-in; no hypothetical
binding capability is added. An internally synchronized provider may achieve
less than N-way parallelism, but the engine still enforces an observable N-slot
upper bound. An unobservable N-worker pool behind an engine-wide unconditional
mutex is not acceptable.

## Configuration contract

Configuration is resolved once, before connection/profile/runtime startup.
No pool resizes after open. `runtime_configuration` validates language input
and produces one engine-owned `ResolvedRuntimeConfiguration`; runtime owners
receive only their typed fields. Defaults and rejected values are the same in
Rust, Python, and TypeScript after language-level conversion.

The accepted fixed upper bound of 64 bounds thread, SQLite-connection,
WAL-inventory and queue multiplication while remaining well above intended
embedded deployments. Changing it now requires a successor decision; leaving
the limit unbounded is not permitted.

With scheduler count S and attached-provider count E, the engine-owned steady
inventory is `1 + S + 8 + E` threads (dispatcher, projection, readers, embed)
and `1 + 1 + S + 8` SQLite connections (primary, dispatcher, projection,
readers). At S=E=64 this is 137 threads, 74 connections, 4,096 admitted
projection rows and 256 queued embed requests. With no provider E contributes
zero threads/requests. Caller/binding threads and provider-internal resources
are outside these counts; the ceiling is a guardrail, not a memory or
throughput guarantee. Exact inventory/fault-cleanup tests must match this map.

| Knob | Proposed canonical contract | Sole consuming effect | Evidence / amendment |
| --- | --- | --- | --- |
| `scheduler_runtime_threads` | Integer `1..=64`; default `2`; immutable after open. The ceiling is a new policy requiring acceptance. | Exact number of projection-orchestration workers and worker SQLite connections, excluding the dispatcher. Projection admission is checked `active_rows + queued_rows <= N * PROJECTION_COMMIT_BATCH`. Does not affect readers, NAPI, or embed threads. | Default matches `ADR-0.6.0-scheduler-shape.md:26`; current hard-coded two workers are at `lib.rs:576` and `projection_runtime.rs:418-419`. Meaning changes from Tokio runtime threads to synchronous projection-orchestration workers. |
| `embedder_pool_size` | Integer `1..=64`; default `1`; immutable after open. The ceiling is a new policy requiring acceptance; `N>1` explicitly opts into concurrent provider calls. | With an embedder, exact number of long-lived engine embed-dispatch workers, maximum simultaneous provider calls, and queue capacity `4*N`. With no embedder, no idle embed workers are allocated. Independent per engine. | Historical default was CPU count (`scheduler-shape.md:27-31`; `embedder-protocol.md:83-89`). Default 1 preserves healthy PR-9 projection behavior (`pr9_embed_serialization.rs:1-16`) while newly sharing capacity with foreground calls. This is a deliberate successor amendment. |
| `embedder_call_timeout_ms` | Integer `1..=u32::MAX` milliseconds; default `30_000`; immutable after open. Zero is invalid. | One absolute queue-plus-service deadline for every engine provider invocation, including one fixed deadline per batch; late result discarded. It is not the close deadline. | Default and finish/discard semantics come from `embedder-protocol.md:97-117`. Current projection-only atomic is `projection_runtime.rs:43-48`; direct bypass is `embedding.rs:119-131`; open-time bypass is `vector_equivalence.rs:37-42`. |
| `provenance_row_cap` | Integer `0..=2^53-1`; default `1_000_000`; immutable after open; zero retains the current disable-retention meaning. The Number-safe ceiling is a new cross-binding policy requiring acceptance. | Existing `write_commit::enforce_provenance_retention`; unrelated to either executor. | Preserve hysteresis and erasure-accountability exemptions. Internal representation remains `u64`; NAPI checks instead of coercing to its current `u32`. |
| `slow_threshold_ms` | Integer `0..=2^53-1` milliseconds; default `100`; initialized before profile callbacks; the existing setter remains. The Number-safe ceiling is a new cross-binding policy requiring acceptance. | Existing operation/SQLite-profile slow-event threshold; unrelated to either executor. | Preserve strict `elapsed > threshold`; internal representation remains `u64`; NAPI checks instead of coercing to its current `u32`. |

Python rejects booleans, negative values, zero for the three strictly positive
runtime knobs, and values outside the canonical range before native open.
TypeScript rejects non-finite, fractional, unsafe, negative, and out-of-range
numbers; it rejects zero only for those same three strictly positive runtime
knobs and accepts zero for `provenance_row_cap` and `slow_threshold_ms`. NAPI
performs checked conversion rather than relying on coercion. Rust uses the same
validation constructor. Installed-artifact tests must accept both zero-valued
existing controls and prove their disabling/zero-threshold behavior. Validation
finishes before filesystem mutation, database admission, model warmup,
connection creation, or thread creation.

The public configuration value is a requested-open snapshot. It is immutable
after open even though the existing slow-threshold setter changes the effective
atomic; the setter does not rewrite the snapshot. Python retains a frozen value
object. TypeScript must clone and freeze the caller input, expose readonly
fields, and test mutation of both the original object and returned snapshot.

### Configuration seam

The minimal proposed native authority is one engine-owned configuration value
covering all five existing advertised knobs, plus one configured member of the
existing `EmbedderChoice` open family. Python/PyO3 and TypeScript/NAPI forward
path, choice, and configuration to that authority; they do not each
reimplement defaults. The current `Engine::open(path)` and existing
choice-based open delegate with defaults. A configured open that cannot select
the embedder would force bindings into a second hidden construction seam and
is rejected.

Because the engine crate currently has no public configuration input, the
exact Rust shape is an approved-surface question. Preferred scaffold:

```text
EngineConfig                    // public, documented value contract
Engine::open_with_choice_and_config(...)
                                // public, documented authority
ResolvedRuntimeConfiguration    // private, fully validated engine form
```

A `#[doc(hidden)] pub` binding-only constructor is not treated as private and
is not preferred. Whatever shape is accepted must be recorded as the narrow
Slice 30 allowed delta and tested from installed Python and Node artifacts.

Configuration input retains each binding's existing shape. Python accepts
either `config=` or per-knob keyword arguments and rejects any combination of
the two (`src/python/fathomdb/engine.py:1931-1946`). TypeScript accepts its
existing `engineConfig` object rather than adding parallel per-knob keywords.
Omission remains distinct from an explicit zero where zero is accepted.

## Failure, shutdown, and isolation

- Publishing an engine is all-or-nothing; provider termination is not. Before
  any probe starts, executor-start failure joins the started idle workers and
  unwinds database resources. After a probe starts, a later startup failure
  cancels dispatch and safely unwinds all SQLite/profile/admission resources,
  then uses the same bounded embed drain and provider-only retention rule as
  close. Preserve the original open error; cleanup must not mask it or publish
  an engine. A timed-out probe alone instead returns a live degraded engine
  with its executor and occupied slot intact, not a half-destroyed engine.
- A projection-queue capacity limit never rejects or rolls back the canonical
  write that created durable pending work.
- An embed-dispatch queue admission failure or deadline maps through the
  operation-specific table below; it never commits a late projection result.
- Timeout accounting is per engine. One engine's hung provider cannot consume
  another engine's slots or open its circuit.
- Admission becomes unavailable while all slots are occupied by
  timed-out-live calls. A returning call releases its slot and automatically
  reopens capacity. There are no replacement workers and no speculative
  half-open provider calls. Queue expiration never counts as a live call. This
  availability state replaces the current session-latched leak breaker; a
  permanent latch is not introduced.
- Close has two ordered phases. **Database quiescence** first marks the engine
  closing, stops new public/database admission and projection discovery,
  closes embed admission, cancels queued and running-result waiters, and wakes
  projection retry/capacity waits **before** joining any database worker.
  Cancellation competes with start/completion under the request-state lock;
  it never waits for or aborts a running provider. Disconnect/cancel pending
  reader work without placing a stop message behind a full work queue, then
  wait for active primary, reader, dispatcher and projection database work.
  Each worker removes its callbacks and releases its own connection before
  exiting; join every SQLite owner before releasing shared profile contexts
  or the sidecar admission lock. Preserve the existing safe teardown order.
  Cancelled projection work stays durable and creates no terminal failure.
  This phase preserves the existing safe ownership contract and is
  intentionally not covered by the embed-drain
  deadline; Option B does not invent cancellation of arbitrary SQLite work or
  claim that total `Engine::close` latency is bounded.
- **Embed-runtime drain** begins only after database quiescence has removed all
  SQLite ownership from outstanding inference work. It uses one absolute
  30-second budget independent of `embedder_call_timeout_ms` and batch size,
  shared by all joins rather than renewed per worker. Admission and waiter
  cancellation have already happened in phase one. Only a still-running embed
  worker may remain after this budget, and only with provider/request state and no
  engine-owned SQLite connection, transaction, profile context, callback
  registration, admission lock, or sidecar lock.
- On drain expiry, detach only the unfinished embed worker handles; no reaper
  thread or replacement worker is created. Those workers retain only provider,
  request and shared accounting state and decrement the live count on exit.
  Propose the existing `EngineError::Scheduler` for incomplete embed shutdown;
  no new public close-error variant is needed. Close is serialized/idempotent:
  concurrent callers observe one teardown, subsequent close does not restart
  its budget, and it reports `Scheduler` while retained workers remain, `Ok`
  once none remain. `Drop` never restarts the wait or panics. Retention is at
  most the configured worker count **per session**, not a process-wide bound:
  repeated opens with permanently hung providers can retain additional
  threads/models. Document that residual; do not claim guaranteed provider
  reclamation. These are successor proposals, not exact API rulings by seq-293.
- Reader shutdown, primary-profile teardown, connection release, and sidecar
  lock release retain the order in `dev/design/engine.md:151-168`. Real-database
  tests cover a full reader queue, a synchronized active long query, and a
  synchronized active primary operation: close stops new admission, remains
  pending without unsafe release, and completes after the operation is allowed
  to finish. Those tests do not apply the 30-second embed-drain budget to
  database quiescence.

Operation-specific mapping distinguishes failure stages:

| Caller | Admission full / queued expiration | Started-provider timeout / provider error | Close cancellation |
| --- | --- | --- | --- |
| Projection | Retain durable pending work and defer with bounded backoff or capacity notification. These outcomes do **not** consume the provider-failure retry budget and cannot create terminal projection residue. | Apply the existing provider-failure retry budget and existing terminal outcome only after that budget is exhausted. | Retain durable pending work for rediscovery; never consume provider-failure retries or create terminal residue. |
| Ordinary or frozen hybrid search | Preserve the existing sparse/text fallback produced by `.embed(...).ok()`; no new query exception or changed refusal precedence. | Preserve the same sparse/text fallback and refusal precedence. | A cancelled embed wait returns `EngineError::Closing` through the settled reader/API boundary and releases its transaction; add only a narrow private closing outcome where needed, not a new broad error carrier or ownership change. A result that already won may finish normally on its snapshot while close waits. Do not replace earlier validation/refusal errors. |
| Direct `Engine::embed_text` | Return the existing `EngineError::Overloaded` before provider invocation. | Preserve `EngineError::Embedder`. | A cancelled wait returns `EngineError::Closing`; preserve already-winning completion and earlier validation errors. |
| Open-time vector-equivalence probe | Preserve degraded open/dense-disabled behavior, baseline/cache mutation rules, and identity/error precedence; never persist a new accepted baseline. | Preserve the same degraded-open and mutation rules. | Unwind open without publishing a partially initialized engine or baseline. |

A real saturation test holds the dispatch queue unavailable longer than the
existing projection provider-failure retry schedule, proves that no failure
budget or terminal residue is consumed, then releases capacity and proves the
durable projection succeeds without manual residue repair.

Capacity unavailability while all slots are timed-out-live is an admission
outcome, not another provider failure. Keep already-spent provider retry counts
across capacity waits for the same admitted generation; a wait neither resets
nor increments them. Retain that work within the existing admitted-row bound,
with interruptible waits, not an unbounded retry-state side map. Generation
change/close releases admission and current durable state drives rediscovery.
Existing returned-invalid-vector handling (including dimension mismatch) still
consumes its current retry budget and retains its existing failure code.

Panics are not ordinary provider errors. Catch them inside the fixed worker,
restore accounting, and transport timely panic payloads to the existing
caller panic boundary: projection records `ProjectionPanic`, the open probe
degrades, and direct/query callers retain their existing panic handling.
Do not retry or silently sparse-fallback a panic as an `EmbedderError`. A late
panic loses to timeout/cancellation, is discarded, and cannot poison/kill the
dispatch worker or reduce pool capacity. This covers unwind builds; process
abort is not cancellable. Result validation remains with the existing caller,
not duplicated inside a generic executor.

Invalid per-engine configuration receives a narrow typed
`EngineConfigurationError` carried by `EngineOpenError`, distinct from the
process-global SQLite `RuntimeConfigurationError`. Queue saturation uses the
existing `EngineError::Overloaded`; timeout/provider failure retains the
operation-specific mappings above. No string-only catch-all is introduced.

## SQLite and rusqlite compatibility

SQLite/rusqlite mandate nonconcurrent connection use and serialize active
write transactions. Post-commit embedding, bounded queues, deadlines,
executor isolation, and the close protocol are FathomDB product contracts,
not SQLite requirements. The distinction is explicit so implementation
preferences are not presented as database compatibility constraints.

The executor split never shares one `rusqlite::Connection` concurrently.
Projection worker count determines the number of worker-owned connections;
embed workers own no SQLite connection. Embedding occurs before acquiring
`commit_gate`. Projection SQL commits remain one-at-a-time under that gate,
while canonical caller writes retain their primary mutex path.

This preserves the current design's honest distinction: SQLite permits one
write transaction at a time, but the engine does not pretend every write is
performed by one dedicated writer thread (`dev/design/engine.md:52-69`). It
also preserves the reason the async-surface ADR retained rusqlite instead of
moving to sqlx (`ADR-0.6.0-async-surface.md:74-91,110-161`).

Changing `scheduler_runtime_threads` changes SQLite connection and WAL
inventories. The existing exact expectations around `PROJECTION_WORKERS`
(`lib.rs:4567-4794,9253-9704`) must become configuration-aware; no test may be
weakened to an unbounded or at-least count.

Runtime observation distinguishes durable pending projection rows, admitted
projection rows, queued embed requests, active provider calls, and
timed-out-but-live calls. Counters are per engine and separately testable;
`embedder_pool_size` is an invocation upper bound, not a promise that an
internally synchronized provider achieves N-way throughput.

Retain existing public projection-status/readiness reporting for durable
backlog. Executor accounting is a private per-engine snapshot with test-hook
access; do not add keys to the locked `CounterSnapshot` public contract. The
successor explicitly replaces scheduler Observability and projection-model
Backpressure's hypothetical public saturation metrics/automatic SDK shedding
with this bounded admission and existing readiness surface. No new adapter
shed policy or public telemetry API is silently introduced.

## Required RED/GREEN evidence

Before implementation, each behavior-changing batch adds a failing test or
production mutant. At minimum:

1. **Scheduler effect:** default creates exactly two projection workers;
   configured 1 and 3 create exactly those worker/connection counts, preserve
   post-commit dispatch, and leave reader/NAPI/embed counts unchanged.
2. **Embed effect:** default peak healthy provider concurrency is one;
   explicit 2 yields peak exactly two with a concurrency-safe probe and never
   exceeds two; two engines have independent limits.
3. **Universal dispatch:** projection, search/query, and direct `embed_text`
   plus first-open and reopen vector-equivalence tests record that the provider
   runs on named engine embed workers rather than the caller, projection
   worker, NAPI Tokio worker, Python caller, or JS event-loop thread. A hung
   open-time probe degrades dense availability within its deadline without
   persisting a baseline, leaving a usable text-only engine and an accurately
   occupied embed slot. Separately inject a later startup failure and prove
   complete SQLite/profile/admission cleanup, preserved original open error,
   and bounded provider-only retention without publishing an engine.
4. **Deadline:** queue wait and running service both consume the deadline;
   each operation preserves the mapping table above; a late result is discarded
   and cannot commit; healthy later work succeeds when capacity returns.
5. **Hung-slot accounting:** a timed-out provider continues to occupy its
   slot; no replacement-thread leak occurs; exhausting N timed-out-live slots
   closes admission, and a returning call reopens capacity without a permanent
   session latch.
6. **Backpressure:** full in-memory projection capacity leaves work durable
   and does not block/rollback canonical commit; bounded queues do not grow
   with database backlog.
7. **Shutdown:** close under queued, retry-sleeping, running, timed-out,
   panicking, and completed-late embeds first quiesces and joins every SQLite
   owner in the existing safe order, then applies the independent absolute
   embed-drain deadline, reports incomplete embed shutdown truthfully, and
   bounds retained calls per session across repeated close/reopen. Set the
   inference timeout above the drain budget and prove cancellation wakes the
   SQLite-owning waiter before join; cover concurrent/repeated close and later
   provider return without a replacement/reaper thread. Real-database
   full-reader-queue, active-query and active-primary-operation tests prove that
   database quiescence neither releases ownership early nor inherits a false
   30-second total-close guarantee.
8. **Validation:** default, boundary, invalid and overflow cases pass through
   Rust and installed Python/Node artifacts; effects are measured, not echoed
   from stored config.
9. **Existing invariants:** PR-9 watchdog/serialization/circuit tests,
   projection runtime close/drain tests, generation/terminal residue tests,
   write-race tests, exact WAL inventories, and public/hidden surface
   comparisons pass with only explicitly reviewed successor deltas. In
   particular, the old hung-provider PR-9 oracle expecting successful drain
   and terminal failure is replaced: with one permanently occupied slot,
   further attempts are unavailable, work remains pending, and bounded drain
   reports `Scheduler`. Releasing the provider restores progress. Retain its
   no-late-commit, finite-thread, write-liveness and recovery assertions; do
   not invent provider failures to preserve the historical oracle.
10. **Batch fallback self-deadlock:** first reproduce the current
    `embed_projection_batch` returned-error/timeout path that invokes `per_job()`
    while holding `embed_serialize` (`projection_worker.rs:553-575` at
    `ab8f43be`; the offending arm is `Err(_) => return per_job()` at line 575,
    reached only with `FATHOMDB_PROJECTION_BATCH` enabled). The RED test sets
    that variable, uses bounded synchronization and proves the per-job path
    attempts the same guard. GREEN requires dropping the guard/permit before every per-job
    fallback, and a production mutant that returns under the guard must fail.
    The executor replacement must preserve that lock-order proof rather than
    merely making the old mutex disappear.
    Exercise breaker-open separately: it fast-fails before reacquisition and
    is not the RED deadlock witness. Use a subprocess or cancellation-safe
    bounded harness so the mutant cannot hang the test process during Drop.
11. **Mixed workload and snapshot effects:** saturate the shared default-one
    embed capacity with projection and foreground calls in both directions;
    prove bounded foreground behavior, durable projection progress, no
    starvation loop, and correct frozen-read authority. The absolute inference
    deadline bounds only the **additional** frozen-snapshot retention caused by
    waiting for dispatch/provider completion. On timeout, the existing sparse
    fallback continues on the same authoritative reader transaction and releases
    that transaction/WAL pin at normal query completion or error; reacquiring a
    different snapshot is forbidden. A separate total-query deadline would be
    a new behavior decision and is not introduced here.
12. **Outcome preservation:** provider errors and invalid vectors exhaust the
    existing retry policy with unchanged codes; capacity waits preserve rather
    than reset that budget. Timely and late panics preserve the operation's
    panic boundary, release accounting and leave dispatch workers reusable.
13. **Configuration compatibility and performance:** bind a release-build
    qualification receipt to the exact stage-2 candidate. Every valid
    configuration must be behaviorally correct, resource-bounded and exhibit
    its advertised consuming effect; only the default configuration carries
    the release performance promises. The matrix is:
    - `scheduler_runtime_threads=2`, `embedder_pool_size=1`: canonical default
      performance gate and installed-binding parity;
    - `1/1`: minimum-bound correctness, progress and shutdown;
    - `2/2`: real provider concurrency and foreground/projection contention;
    - `4/4`: representative larger override, capacity effect and bounded
      resource growth; and
    - `64/64`: ceiling validation, exact thread/connection/queue inventory and
      cleanup only, not a throughput target.

    A pure resolved-configuration property test covers every scheduler/embed
    pair in `1..=64`, including checked row/queue derivation. A `2/no-provider`
    case proves that configured embed capacity allocates no idle embed workers
    or requests when no provider is attached.

    On the default, run AC-011a/b through
    `scripts/run-ac011-write-throughput.sh`, then AC-017
    projection-freshness, AC-018 projection-drain, AC-029 projection-stall
    write-tolerance, AC-072 vector-retrieval, AC-073 real-corpus retrieval-tail,
    AC-076 text/hybrid-query and AC-081a/b/c reader-progress gates. Add one
    D27-specific mixed workload that concurrently performs canonical writes,
    dense projection, foreground hybrid queries and direct embeds. Record
    commit throughput; projection and foreground-query p50/p95/p99; queue wait,
    saturation and durable-backlog high-water marks; observed provider
    concurrency; thread/SQLite-connection inventories; close latency and
    residual workers; and starvation in both foreground-to-projection and
    projection-to-foreground directions.

    The frozen workload and oracle are
    `d27-runtime-qualification-protocol.json`. Before semantic runtime work,
    land and independently review the measurement-only harness, then run it
    against exact engine candidate `7a2f9bf9`. Bind the protocol, binary,
    generated-corpus and raw-output hashes plus entry center/MAD values.

    Capture exact candidate, features, optimized build, hardware/software,
    dataset/workload, warm-up, repetitions, raw or reproducible output and an
    entry-candidate comparison. The protocol freezes the median/MAD formula
    before the entry run; later runs may substitute only the recorded entry
    values. The new mixed workload must show progress in both directions
    without exceeding contractual queues or resource counts. A default
    regression, starvation result or missed project gate blocks the runtime
    checkpoint: optimize within the accepted contract and repeat the identical
    matrix, or formally revise/succeed the ADR if the remedy changes a default
    or executor shape. It cannot be deferred to Slices 114, 115 or 135. A
    separate Slice 91 is justified only if evidence demonstrates a materially
    different executor/remediation boundary; normal qualification remains
    Slice 90 work. Preserve the candidate, commands, raw/reproducible outputs
    and hashes in
    `dev/plans/0.8.27/features/slice-90/runtime-performance-qualification.md`.
    After Slice 90's mechanical moves, repeat the default gates, mixed workload
    and exact resource/cleanup inventory at the final slice candidate. Any
    post-checkpoint semantic runtime/configuration change requires the full
    matrix again.

The existing relevant oracles include:

- `tests/pr9_embed_watchdog.rs:185-365`;
- `tests/pr9_embed_serialization.rs:87-128`;
- `tests/pr9_concurrent_embed.rs:71-177`;
- `tests/projection_runtime.rs:123-424`;
- `tests/tc57_worker_commit_pressure.rs:254-352`; and
- `tests/tc91_duplicate_embeds_and_silent_commit_loss.rs:400-421,741-802`.

Existing Python/TypeScript surface tests only prove value retention
(`src/python/tests/test_surface.py:294-307` and
`src/ts/tests/surface.test.ts:285-297`); they are not consuming-effect proof.

Before Slice 90 closes, document every accepted setting, default, unit, range,
zero/omission meaning, mutability, precedence, consuming component,
backpressure behavior, error/fallback outcome, and binding spelling in the
successor ADR, `dev/design/engine.md`, `dev/design/scheduler.md`,
`dev/design/embedder.md`,
`dev/design/bindings.md`, all three `dev/interfaces/` language contracts, and
`docs/reference/config.md`. Correct `docs/reference/errors.md` wherever its
operator guidance assumes the historical pools. Documentation is part of the
configuration batch and must match installed-artifact effect tests; Slice 140
may converge prose but may not supply missing Slice 90 configuration truth.

## Scope guardrails

- Do not add Tokio to `fathomdb-engine` for this successor.
- Do not move projection commits onto the primary caller connection.
- Do not give embed workers SQLite connections.
- Do not make the reader pool, NAPI pool, Python executor, or JS worker pool an
  engine scheduler/embedder configuration consumer.
- Do not add Python or TypeScript custom-embedder bridges or change the
  independently versioned `Embedder` trait for hypothetical bridge capacity.
- Do not create a generic executor framework or reusable trait hierarchy for
  two concrete queues.
- Do not widen fields or introduce facade wrappers merely to satisfy the
  dependency gate.
- Do not redesign Slice 85 ownership boundaries while implementing runtime
  behavior; consume its final owners.
- Do not defer any accepted runtime behavior to Slice 100 or 110. Binding
  decomposition may move already-correct forwarding and handoff code, but may
  not create the engine executors later.

## Independent-review dispositions and acceptance

The independent design review's eight questions are resolved in this revision:

1. Explicit `embedder_pool_size > 1` is the current Rust/provider opt-in;
   nonexistent Python/TypeScript custom bridges and a new trait capability are
   out of scope.
2. Default `1` is recommended because it preserves healthy PR-9 projection
   behavior; the new shared foreground/background contention is qualified
   rather than mislabeled unchanged behavior.
3. `1..=64` is a proposed new operational ceiling, not historical authority;
   acceptance requires the documented worst-case thread/connection/queue
   accounting.
4. One fixed timeout applies per provider invocation, including one batch.
5. Only embed workers may outlive database close; explicit close reports an
   incomplete runtime rather than claiming arbitrary provider teardown.
6. Invalid configuration gets a narrow open-time typed error, queue saturation
   uses `Overloaded`, and other outcomes retain operation-specific mappings.
7. The exact ThreadsafeFunction mechanism is retired while its isolation
   outcomes remain for Slice 110 qualification.
8. The public configured open joins the existing `EmbedderChoice` family; no
   binding-only public seam is introduced.

`D27-runtime-topology` decision `seq-293` selects only Option B's architectural
direction: preserve synchronous projection/commit ownership and define real
engine-owned orchestration/embed-dispatch capacities, universal inference
deadlines, operation-specific outcomes, and bounded embed-runtime shutdown.
The numeric ceilings/ranges, default-one embed concurrency, typed configuration
delta, exact configured-open API, distinct queue/admission bounds and exact
incomplete-close outcome were accepted by `seq-295` and are now authoritative
through the successor ADR. This scaffold remains the detailed review source,
not a substitute for that ADR.

The acceptance explicitly includes the reliability posture carried by
default-one embed concurrency: one hung provider call occupies the only embed
slot, so all later
dense projection and foreground embeds in that session are
admission-unavailable until it returns, durable projection work remains
pending, and bounded close reports `Scheduler`. Under PR-9 today the projection
watchdog retries and can terminalize the affected row while later work
continues until the session live-thread breaker prevents additional projection
embedding; it does not provide a general foreground-safe pool. Accepting the
default accepts the new fixed-slot stall trade; rejecting it requires a
different default or separate foreground/projection capacity, either of which
is a change to this scaffold.
