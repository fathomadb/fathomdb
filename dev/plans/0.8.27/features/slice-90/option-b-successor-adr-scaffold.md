---
title: D27 runtime topology Option B — successor ADR scaffold
status: DRAFT-REVISED-UNACCEPTED
target_release: 0.8.27
reviewed_candidate: 9b00a980ca222d05f3af063168ed45c2b0a4a526
---

# D27 runtime topology Option B — successor ADR scaffold

This is a design-review scaffold, not an accepted ADR, implementation
authority, or evidence receipt. `D27-runtime-topology` remains unruled and
AC27-90B remains blocked. If accepted after review and HITL ruling, this
material should become a successor ADR and the approved portions should be
folded into the Slice 90 design before commissioning.

## Proposed decision

Retain the synchronous engine API and the current projection/commit ownership
model. Replace the historical Tokio-task/dedicated-writer topology with two
separate, bounded, engine-owned synchronous executors:

1. a projection-orchestration executor whose worker count is controlled by
   `scheduler_runtime_threads`; and
2. an embed-dispatch executor whose worker count and maximum simultaneous
   provider calls are controlled by `embedder_pool_size`.

All engine-owned inference paths—open-time vector-equivalence probes,
projection, ordinary and frozen search/query, and direct `Engine::embed_text`—
submit to the embed-dispatch executor and observe one dispatch/deadline
contract. Model construction, warmup, and `Embedder::identity` remain separate
open-time operations; this decision does not claim they can be cancelled. The
caller-facing Rust/Python API remains synchronous. Bindings may move a complete
synchronous engine call off their event-loop thread, but neither a binding
pool nor a projection worker may invoke the embedder directly.

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
  the exact executor description, while retaining engine ownership and the
  provider protocol;
- `dev/adr/ADR-0.6.0-projection-model.md` only if the final batch-deadline,
  admission-unit, shedding, or metric clauses differ; and
- `dev/adr/ADR-0.6.0-async-surface.md` where it prescribes a specific NAPI
  ThreadsafeFunction mechanism rather than the retained isolation outcome.

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

SQLite and rusqlite require disciplined connection ownership, post-commit
background work, and write serialization. They do not require Tokio, a
dedicated writer OS thread, or an MPSC commit channel. Replacing the working
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
invoking the provider. A running request that reaches the deadline becomes a
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

The default effective embed concurrency remains one, preserving the PR-9
safety contract proven by
`tests/pr9_embed_serialization.rs:1-16,87-128`. The prior implementation
serialized arbitrary binding-supplied embedders because their practical
thread safety may be weaker than the Rust `Send + Sync` surface promises;
concurrent Candle calls were measured safe but throughput-neutral.

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

The fixed upper bound of 64 below is proposed, not accepted. It bounds thread,
SQLite-connection, WAL-inventory, and queue multiplication while remaining
well above intended embedded deployments. Review should replace it if there
is stronger repository evidence for another bound; leaving the limit
unbounded is not proposed.

| Knob | Proposed canonical contract | Sole consuming effect | Evidence / amendment |
| --- | --- | --- | --- |
| `scheduler_runtime_threads` | Integer `1..=64`; default `2`; immutable after open. The ceiling is a new policy requiring acceptance. | Exact number of projection-orchestration workers and worker SQLite connections, excluding the dispatcher. Projection admission is checked `active_rows + queued_rows <= N * PROJECTION_COMMIT_BATCH`. Does not affect readers, NAPI, or embed threads. | Default matches `ADR-0.6.0-scheduler-shape.md:26`; current hard-coded two workers are at `lib.rs:576` and `projection_runtime.rs:418-419`. Meaning changes from Tokio runtime threads to synchronous projection-orchestration workers. |
| `embedder_pool_size` | Integer `1..=64`; default `1`; immutable after open. The ceiling is a new policy requiring acceptance; `N>1` explicitly opts into concurrent provider calls. | With an embedder, exact number of long-lived engine embed-dispatch workers, maximum simultaneous provider calls, and queue capacity `4*N`. With no embedder, no idle embed workers are allocated. Independent per engine. | Historical default was CPU count (`scheduler-shape.md:27-31`; `embedder-protocol.md:83-89`). Default 1 preserves healthy PR-9 projection behavior (`pr9_embed_serialization.rs:1-16`) while newly sharing capacity with foreground calls. This is a deliberate successor amendment. |
| `embedder_call_timeout_ms` | Integer `1..=u32::MAX` milliseconds; default `30_000`; immutable after open. Zero is invalid. | One absolute queue-plus-service deadline for every engine provider invocation, including one fixed deadline per batch; late result discarded. It is not the close deadline. | Default and finish/discard semantics come from `embedder-protocol.md:97-117`. Current projection-only atomic is `projection_runtime.rs:43-48`; direct bypass is `embedding.rs:119-131`; open-time bypass is `vector_equivalence.rs:37-42`. |
| `provenance_row_cap` | Integer `0..=2^53-1`; default `1_000_000`; immutable after open; zero retains the current disable-retention meaning. The Number-safe ceiling is a new cross-binding policy requiring acceptance. | Existing `write_commit::enforce_provenance_retention`; unrelated to either executor. | Preserve hysteresis and erasure-accountability exemptions. Internal representation remains `u64`; NAPI checks instead of coercing to its current `u32`. |
| `slow_threshold_ms` | Integer `0..=2^53-1` milliseconds; default `100`; initialized before profile callbacks; the existing setter remains. The Number-safe ceiling is a new cross-binding policy requiring acceptance. | Existing operation/SQLite-profile slow-event threshold; unrelated to either executor. | Preserve strict `elapsed > threshold`; internal representation remains `u64`; NAPI checks instead of coercing to its current `u32`. |

Python rejects booleans, negative values, zero for the three strictly positive
runtime knobs, and values outside the canonical range before native open.
TypeScript rejects non-finite, fractional, unsafe, negative, zero, and
out-of-range numbers. NAPI performs checked conversion rather than relying on
coercion. Rust uses the same validation constructor. Validation finishes
before filesystem mutation, database admission, model warmup, connection
creation, or thread creation.

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

- Opening any executor is all-or-nothing. A thread, channel, connection, or
  warmup failure unwinds already-created resources before returning an error.
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
- Close uses one absolute 30-second lifecycle budget independent of
  `embedder_call_timeout_ms` and batch size. It marks the engine closing, stops
  new projection discovery and embed admission, cancels queued/result waiters,
  interrupts retry waits, and preserves cancelled projection work for durable
  rediscovery rather than terminalizing it as provider failure.
- Every SQLite-owning dispatcher/projection/reader worker must join before its
  connection, profile context, or admission lock is released. Only a still-
  running embed worker may outlive database close, and only after it is reduced
  to provider/request state with no engine-owned SQLite connection,
  transaction, profile context, callback registration, or admission lock.
  This is a per-session count bound, not a claim of zero retained provider
  resources: a permanently hung arbitrary provider may retain its detached
  thread/model until it returns or the process exits. Explicit close returns
  the existing typed scheduler failure if the engine cannot honestly report
  complete embed shutdown; `Drop` remains best effort.
- Reader shutdown, primary-profile teardown, connection release, and sidecar
  lock release retain the order in `dev/design/engine.md:151-168`.

Operation-specific mapping is fixed as follows:

| Caller | Queue full / deadline / provider failure |
| --- | --- |
| Projection | Bounded defer/retry where work remains durable; after the existing retry policy, record the existing projection failure outcome. Cancellation for close never becomes a provider-failure terminal. |
| Ordinary or frozen hybrid search | Preserve the existing sparse/text fallback produced by `.embed(...).ok()`; no new query exception or changed refusal precedence. |
| Direct `Engine::embed_text` | Preserve `EngineError::Embedder`; queue saturation may use the existing `EngineError::Overloaded` before invocation. |
| Open-time vector-equivalence probe | Preserve degraded open/dense-disabled behavior, baseline/cache mutation rules, and identity/error precedence; a failed or timed-out probe never persists a new accepted baseline. |

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
   open-time probe degrades dense availability within its deadline and cleans
   every partially created executor/connection without persisting a baseline.
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
   panicking, and completed-late embeds respects the independent absolute
   lifecycle deadline, joins every SQLite owner, reports incomplete embed
   shutdown truthfully, and releases SQLite/WAL/admission resources in the
   existing order. Repeated close/reopen bounds retained calls per session.
8. **Validation:** default, boundary, invalid and overflow cases pass through
   Rust and installed Python/Node artifacts; effects are measured, not echoed
   from stored config.
9. **Existing invariants:** PR-9 watchdog/serialization/circuit tests,
   projection runtime close/drain tests, generation/terminal residue tests,
   write-race tests, exact WAL inventories, and public/hidden surface
   comparisons pass with only the reviewed configuration delta.
10. **Batch fallback self-deadlock:** first reproduce the current
    `embed_projection_batch` failure/breaker path that invokes `per_job()` while
    holding `embed_serialize` (`projection_worker.rs:553-575`). The RED test
    uses bounded synchronization and proves the per-job path attempts the same
    guard. GREEN requires dropping the guard/permit before every per-job
    fallback, and a production mutant that returns under the guard must fail.
    The executor replacement must preserve that lock-order proof rather than
    merely making the old mutex disappear.
11. **Mixed workload and snapshot effects:** saturate the shared default-one
    embed capacity with projection and foreground calls in both directions;
    prove bounded foreground behavior, durable projection progress, no
    starvation loop, and correct frozen-read authority. A frozen query waiting
    for dispatch may retain its WAL snapshot only within the request deadline;
    timeout/fallback must release the reader transaction and WAL pin.

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
successor ADR, `dev/design/engine.md`, `dev/design/embedder.md`,
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

## Independent-review dispositions and remaining approvals

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

The architectural direction, new numeric ceilings/ranges, changed embedder
default, typed configuration delta, exact close outcome, and ADR supersession
still require acceptance through `D27-runtime-topology`. This document does not
record that ruling.
