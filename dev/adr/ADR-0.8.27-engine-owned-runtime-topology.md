---
title: ADR-0.8.27-engine-owned-runtime-topology
date: 2026-09-28
target_release: 0.8.27
desc: Preserve synchronous SQLite ownership while adding bounded engine-owned projection orchestration and embed dispatch.
blast_radius: engine open and close; projection runtime; all engine embed calls; Rust/Python/TypeScript configuration; runtime and performance qualification
status: accepted (HITL seq-295)
---

# ADR-0.8.27 — Engine-owned synchronous runtime topology

**Status:** accepted by owner ruling `seq-295`, following the architectural
direction selected at `seq-293`.

## Context

The accepted 0.6.0 ADRs described Tokio orchestration, a CPU-count embed pool,
a dedicated writer thread and binding mechanisms that the shipped engine never
implemented. The current engine instead has a synchronous public API, a
mutex-serialized primary writer, projection workers with worker-owned SQLite
connections, projection commits serialized by `commit_gate`, and an
eight-connection reader pool.

SQLite and rusqlite require disciplined connection ownership and one active
write transaction at a time. They do not require Tokio, a dedicated writer OS
thread or an MPSC commit channel. Replacing the working ownership topology only
to make those historical mechanisms literal would combine a writer rewrite
with the 0.8.27 decomposition and would not improve SQLite compatibility.

The reviewed source design is
`dev/plans/0.8.27/features/slice-90/option-b-successor-adr-scaffold.md`.
This ADR records the accepted contract; the scaffold remains design evidence,
not a second authority.

## Decision

Retain the synchronous Rust/Python engine API and the current primary-writer,
projection-worker and `commit_gate` ownership model. Add two separate, bounded,
engine-owned synchronous executors:

1. a projection-orchestration executor sized by
   `scheduler_runtime_threads`; and
2. an embed-dispatch executor sized by `embedder_pool_size`.

Do not add Tokio to `fathomdb-engine`, move projection commits to the primary
connection, create a dedicated writer OS thread, or make a binding executor an
engine configuration consumer.

All production engine-owned calls to `Embedder::embed` or `embed_batch`—open-time vector
equivalence, projection, ordinary and frozen search, and direct
`Engine::embed_text`—must run on the embed-dispatch executor. A projection
worker, binding worker or public caller never invokes the provider directly.
Provider construction, warmup, identity, reranking and standalone SDK embed
utilities are outside this dispatch deadline.

The default-compiled, `#[doc(hidden)]` `Engine::write_vector_for_test` seam is
an explicit temporary exception until Slice 140 removes it from default public
builds. It may continue to invoke the provider directly and therefore must not
be used as evidence for dispatch coverage, deadlines, queue bounds, lock order,
or performance. Slice 90 tests those properties through production paths and
private test controls instead. This exception records current public truth; it
does not permit another production or test-only bypass.

Projection workers retain worker-owned SQLite connections and commit under
`commit_gate`. Embed workers own no SQLite connection. Embedding happens before
the projection worker acquires `commit_gate`.

## Configuration contract

Configuration resolves and validates once, before filesystem mutation,
database admission, provider warmup, connection creation or thread creation.
The open-time snapshot is immutable after open except that the existing
`set_slow_threshold_ms` control changes the effective slow threshold. Rust owns one public `EngineConfig`, one
`Engine::open_with_choice_and_config` member of the existing `EmbedderChoice`
open family, and one private `ResolvedRuntimeConfiguration`. Existing open
methods delegate with defaults.

| Setting | Accepted contract | Consuming effect |
| --- | --- | --- |
| `scheduler_runtime_threads` | Integer `1..=64`, default `2`. | Exact projection-orchestration worker and worker-connection count. Projection admission is `active_rows + queued_rows <= N * PROJECTION_COMMIT_BATCH`. |
| `embedder_pool_size` | Integer `1..=64`, default `1`; `N > 1` explicitly permits at most N simultaneous calls to the shared provider. | Exact embed worker count and provider-call ceiling. With a provider the waiting queue is `4 * N`; without a provider no embed worker or request queue is allocated. |
| `embedder_call_timeout_ms` | Integer `1..=u32::MAX`, default `30_000` ms. | One absolute queue-plus-service deadline per provider invocation, including one fixed deadline for a batch. |
| `provenance_row_cap` | Integer `0..=2^53-1`, default `1_000_000`; zero retains the existing disable-retention meaning. | Existing provenance-retention consumer only. |
| `slow_threshold_ms` | Integer `0..=2^53-1`, default `100`; zero retains existing zero-threshold behavior; the existing setter may replace the effective value after open. | Existing operation/SQLite-profile slow-event consumer only. |

With scheduler count S and an attached provider with embed count E, steady
engine ownership is `1 + S + 8 + E` threads and `1 + 1 + S + 8` SQLite
connections. At `S=E=64`, that is 137 threads, 74 connections, 4,096 admitted
projection rows and 256 queued embed requests. This ceiling is a resource
guardrail, not a throughput promise.

Python retains either `config=` or per-knob keywords and rejects their
combination. TypeScript retains one `engineConfig` object. Omission remains
distinct from explicit zero. Python rejects booleans, negative values and zero
for the three strictly positive controls. TypeScript rejects non-finite,
fractional, unsafe, negative and out-of-range numbers, and rejects zero only
for those three controls. NAPI performs checked conversion. All bindings use
the engine defaults and validation rather than reproducing them.

Python and TypeScript expose the requested open-time snapshot, not a live
effective-value view. The slow-threshold setter does not rewrite that snapshot.
Python's frozen value object and TypeScript's cloned, frozen, readonly object
must reject caller alias mutation; effective behavior after the setter is
proved through slow-event output.

## Admission, deadlines and outcomes

Embed admission is bounded and nonblocking. A request receives one absolute
monotonic deadline before enqueue. A full queue is overload; queue wait and
provider service consume the same deadline. Expired or cancelled queued work
never invokes the provider. A running provider call is not forcibly cancelled;
its late result or late panic is discarded, and its worker slot remains
occupied until it returns. No replacement or speculative worker is created.

Default embed concurrency is one. Consequently, one permanently hung provider
call makes later dense projection and foreground embedding unavailable in that
engine until it returns. Durable projection work remains pending and bounded
close reports `EngineError::Scheduler`. This is an accepted reliability trade,
not an assertion that current PR-9 behavior was identical. Under PR-9, the
projection watchdog retries and can terminalize the affected row while later
work continues until the session live-thread breaker prevents further
projection embedding.

Outcomes remain operation-specific:

- projection admission failure or queued expiration retains durable pending
  work and consumes no provider-failure retry; a started provider failure or
  timeout uses the existing retry and terminal policy;
- ordinary and frozen hybrid search preserve existing same-snapshot sparse
  fallback and refusal precedence;
- direct `embed_text` maps admission saturation to `EngineError::Overloaded`
  and a started provider failure to `EngineError::Embedder`;
- an open-time vector-equivalence failure preserves degraded-open and baseline
  mutation rules; and
- close cancellation returns `EngineError::Closing` where the settled public
  boundary permits it, without replacing earlier validation errors.

Provider panics remain distinct from provider errors. Timely panics reach the
existing operation-specific panic boundary; late panics are discarded without
killing a fixed dispatch worker.

## Shutdown and failed open

Close has two ordered phases.

1. **Database quiescence:** stop new public/database and embed admission,
   cancel queued/result waiters, wake projection capacity/retry waits, and join
   every primary, reader, dispatcher and projection SQLite owner in the
   existing safe teardown order. This phase has no newly invented 30-second
   total-close promise.
2. **Embed-runtime drain:** after all outstanding inference owns no SQLite,
   profile, callback or sidecar/admission state, use one absolute 30-second
   budget shared across embed-worker joins. Do not renew it for repeated close
   or Drop.

After drain expiry, detach only unfinished embed-worker handles. They retain
provider/request/accounting state only, create no reaper or replacement thread,
and decrement shared live accounting when they return. Explicit close reports
the existing `EngineError::Scheduler` while retained workers remain and `Ok`
after they exit. The bound is per engine session; repeated sessions with
permanently hung providers can retain additional provider threads.

Executor startup failure unwinds started workers and database state. A failure
after an open-time probe starts preserves the original open error while applying
the same database cleanup and bounded provider-only retention rule. A timed-out
probe alone returns a live degraded engine with no accepted baseline mutation.

## Supersession map

This ADR partially supersedes only the named clauses below. Unnamed invariants
remain authoritative.

- `ADR-0.6.0-scheduler-shape.md`: replaces Tokio orchestration tasks, historical
  pool defaults, per-job task/admission units, dedicated writer channel,
  shutdown mechanism and required Tokio/task consequences.
- `ADR-0.6.0-single-writer-thread.md`: replaces the dedicated writer OS thread,
  channel ownership, prohibition on projection-worker writers and total
  submission-order claims. One-at-a-time SQLite write serialization remains.
- `ADR-0.6.0-embedder-protocol.md`: replaces Invariant 4's CPU-count default and
  exact pool mechanism; expands Invariant 5 to one absolute queue-plus-service
  deadline, including fixed batches. The provider trait, no re-entry and
  finish-and-discard rules remain.
- `ADR-0.6.0-projection-model.md`: replaces scheduler-entry batch granularity,
  unbounded internal queue, single-writer/adapter-shedding prescriptions and
  cursor-only rediscovery with bounded admitted rows, dequeue batching and
  generation/terminal-aware durable rediscovery. Push dispatch, provider retry
  and terminal-state cursor authority remain.
- `ADR-0.6.0-async-surface.md`: replaces the exact ThreadsafeFunction,
  binding-pool sizing/configuration and internal Arc/async mechanisms. The
  synchronous Rust/Python surface, TypeScript Promise/off-event-loop behavior,
  post-commit dispatch and product outcomes A–D remain.

The hypothetical public scheduler saturation metrics and automatic SDK
shedding are retired. Existing public projection readiness remains; executor
accounting is private and testable. No keys are added to `CounterSnapshot`.

## Verification and consequences

Slice 90 must implement this decision with RED/GREEN tests and the requirements
in its accepted design. The stage-2 checkpoint requires configuration effect,
binding parity, deadline/fault, shutdown/resource and default-performance
qualification at the exact candidate before mechanical runtime moves. The
final Slice 90 candidate repeats the default performance, mixed-load and
resource/cleanup evidence; a semantic runtime/configuration change after the
checkpoint repeats the full matrix.

The default configuration carries the project's performance gates. Other valid
settings must remain correct, bounded and observably effective; `64/64` is a
resource/cleanup qualification, not a throughput target. Missing correctness or
default-performance evidence cannot be deferred to Slices 114, 115 or 135.

This ADR does not authorize release publication, a custom Python/TypeScript
embedder bridge, a generic executor framework, or a redesign of Slice 85's
ownership boundaries.
