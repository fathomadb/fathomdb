---
title: ADR-0.8.27-typescript-subscriber-delivery
date: 2026-10-04
target_release: 0.8.27
desc: Bound TypeScript subscriber delivery across the N-API boundary
blast_radius: NAPI subscriber adapter; TypeScript native binding and SDK; TypeScript interface and public docs; binding and lifecycle design
status: accepted
---

# ADR-0.8.27 — Bounded TypeScript subscriber delivery

Accepted for Slice 110 implementation after the independent high-effort
design reviews recorded in
[`design-review.md`](../plans/0.8.27/features/slice-110/design-review.md).

## Context

The TypeScript SDK promises `Engine.attachSubscriber(callback,
{ heartbeatIntervalMs? })`, but the native method discards both arguments.
The engine has lifecycle events, SQLite profiles, slow statements and stress
failures, but no operation-scoped heartbeat producer or operation identity.
Some events arise inside SQLite's C profile callback while database resources
are held. JavaScript execution or a blocking handoff there can deadlock an
operation; an unbounded asynchronous queue can exhaust memory.

The accepted runtime-topology ADR permits Tokio `spawn_blocking` for ordinary
Promise-returning engine calls. Host callback delivery is a separate boundary.
The Python subscriber ADR does not decide TypeScript's contract.

## Decision

Keep `attachSubscriber(callback)` as a synchronous replacement of the one
active subscriber on an Engine. Remove `heartbeatIntervalMs` and the options
parameter from the public/native signatures. A binding timer cannot assert
database or provider progress. This narrows the general heartbeat design for
the current TypeScript adapter only; an actual future engine Heartbeat event
still passes through unchanged. Do not synthesize an operation ID.

Keep an owned, bounded 4096-record queue in the binding. Producers use a
nonblocking lock attempt to append owned records and coalesce a data-free raw
N-API ThreadsafeFunction wakeup. The pinned napi-rs 2.16.17 helper boxes a payload
before calling N-API and does not reclaim it on `QueueFull`/`Closing`, so
putting records directly into its bounded queue would violate ownership.
The raw shim enqueues a null data pointer, releases its handle on detachment,
and frees its context in the N-API finalizer even when the environment is
closing. The TSFN handle sits in a short-lived mutex slot. Enqueue callers
try-lock it, call N-API while holding it, and release the lock immediately;
detachment takes the slot under the same mutex and calls N-API release as its
last handle use. A contended try-lock is a drop, not a wait on a SQLite
callback thread. JS-thread continuation uses the same slot protocol.

The owned queue lock also owns a `wake_pending` bit. Enqueue appends, then
sets the bit and schedules a wake only on a false-to-true transition. While a
wakeup drains, the bit stays true. After at most 64 deliveries, the JS thread
atomically clears the bit under that same queue lock and, if records remain,
sets it true before releasing the lock and scheduling the next wake. An
enqueue before that transition is seen by the continuation; an enqueue after
it sees false and schedules its own wake. A failed wake (`Closing` or any
unexpected status) disables and clears the attachment under the queue lock.
Use an unlimited N-API wake queue with at most one outstanding null
notification by this bit invariant, so `QueueFull` is unreachable; any
`QueueFull` result is still treated as a failed wake. The JS thread drains
at most 64 records per turn and schedules another wakeup when needed. The engine/SQLite
thread never invokes JavaScript or waits for the listener. A full or contended
binding queue drops the new record. Every delivered record includes a
cumulative `droppedRecordsTotal` decimal string so the listener can detect
overload after delivery resumes. This is best-effort host delivery; it does
not weaken the engine's own event emission contract.

The callback receives one tagged `SubscriberEvent` object. `kind` is `event`,
`profile`, `slowStatement`, or `stressFailure`. Event `phase` is `started`,
`slow`, `heartbeat`, `finished` or `failed`; `source` is `engine` or
`sqlite_internal`; `category` is `writer`, `search`, `admin`, `error`,
`corruption`, `recovery` or `io`. `code` is absent when the engine event has
none, otherwise a string. Profile records contain `wallClockMs`, `stepCount`,
`cacheDelta`; slow records contain `statement`, `wallClockMs`; stress records
contain `threadGroupId`, `opKind`, `lastErrorChain`, `projectionState`.
Numeric diagnostic fields are canonical decimal strings, including a leading
minus sign for negative `cacheDelta`. All records include
`droppedRecordsTotal`, a per-attachment saturating `u64` decimal string
sampled immediately before listener invocation so already queued records
expose later overload. Callback-fault accounting stays private and does not
change `CounterSnapshot`.

The listener runs on the JavaScript thread through a safe callback trampoline;
stock napi-rs TSFN forwarding would turn its exception into a fatal exception.
The raw N-API callback first checks for null `env` or null `js_callback` during
environment teardown. In that case it performs no N-API or JavaScript work;
the N-API finalizer clears the owned queue and drops its context. With live
handles, the callback checks and clears pending synchronous exceptions;
the entire C callback frame also catches Rust panics. The trampoline catches
and counts synchronous failures without changing engine operation
outcomes or aborting the process. Reentry is allowed: the adapter holds no
registry/Engine/SQLite lock during delivery, and reentered engine work uses
the ordinary nonblocking handoff. Sync accessors return normally; asynchronous
work returns a Promise and is never awaited by the drain loop. The 64-record
per-turn cap prevents one wakeup from monopolizing the loop, but an
unbounded application callback → query → profile cycle remains the caller's
responsibility. A returned Promise is not awaited; its later rejection is
caller-owned, and applications should catch it inside an async listener.
The listener must not treat callback order or counts as a durability oracle.

An active/generation gate is checked by each producer and before each listener
call. Validate and construct a replacement before changing the active attachment.
Replacement atomically detaches and silences the previous generation, including
its queued records. At close entry, synchronously disable and detach delivery,
even if the close later reports a scheduler failure; attach after close entry
refuses with `ClosingError`. A callback already executing may finish. In-flight
engine work retains its Arc until completion but may still receive the engine's
documented `Closing` or `Scheduler` outcomes. The ThreadsafeFunction is
unreferenced so an idle subscriber cannot keep a Node process alive.
Environment shutdown may discard queued diagnostics without blocking exit;
N-API's finalizer releases the callback reference on that environment's
thread. The Rust queue is cleared on replacement, close and finalization.
To make close-entry ordering real, native `close()` is a synchronous Rust
function that first creates its JavaScript Promise, then marks the binding
closing and detaches the subscriber before scheduling the existing Tokio
`spawn_blocking` engine close. It returns the same `Promise<void>` JS shape;
changing an `async fn` body alone would not establish the ordering.

The queue bounds record count, not bytes: SQL text and diagnostic strings can
vary in size. Applications should consider this cost before enabling
per-statement profiling for unbounded SQL payloads.

## Verification

RED tests against the actual native artifact establish inert delivery before
implementation. GREEN tests prove delivery and JS-thread affinity for event,
profile and slow records; overload disclosure, callback exception containment,
same-callback reentry, replacement, close, and process exit with a pending
wakeup at shutdown. A focused native
test proves nonblocking enqueue from a blocked producer. Public declarations,
runtime exports and installed package behavior reflect only the named
subscriber signature/payload delta.
