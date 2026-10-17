---
title: EP-09 — Deadlines and interruption for reader-pool reads
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. Let a caller give a read a deadline or a cancel token, which the engine enforces before dispatch, while queued, and during SQL through a reader-connection progress handler, so an abandoned call frees its reader worker.
---

# EP-09 — Deadlines and interruption for reader-pool reads

Status: Draft. Priority P1. Not an accepted decision.

## Problem

The engine is synchronous by decision. Cancellation was deferred to a later
phase (`ADR-0.6.0-async-surface.md`, "Cancellation ... Phase 2+ followups").
How the reader pool works, verified in code:

- There are 8 reader workers (`fathomdb-engine/src/reader_pool.rs:4`). Each owns
  one connection and has a bounded queue of 4
  (`reader_pool.rs:499`, `reader_pool.rs:955`).
- `dispatch` picks a worker round-robin from a global counter and calls a
  blocking `send` on that worker's `SyncSender` (`reader_pool.rs:1216-1226`).
  Nothing looks at worker load.
- No reader request carries a deadline. There is no `sqlite3_interrupt` or
  progress handler on reader connections. The only progress handler in the
  engine is the dependency-trace bound on the writer connection
  (`dependency_trace.rs:1645-1659`).

So a call the adapter abandons at its deadline keeps its worker busy until the
SQL finishes. Worse than the architecture assumed, round-robin means about 1 in
8 later requests lands in that worker's queue and waits behind it, even when the
other 7 are idle. The adapter's admission cap
(`src/rust/crates/fathomdb-mcp/src/registry.rs:331`, interface §2) limits how
many calls are in flight. It cannot stop a fresh call from queueing behind an
abandoned one.

Adapter workaround removed: abandoned calls keep their admission slot until the
blocking thread returns (`bridge.rs:4-7`, `bridge.rs:39-45`). The default
admission cap of 4 is below the pool size of 8 to leave headroom.

## Proposed surface

A cancel token, rather than raw `sqlite3_interrupt`. An interrupt handle is per
connection, so firing it late can abort the next request on that worker.

```rust
#[derive(Clone)]
pub struct CallControl { /* Arc<AtomicBool> + Option<Instant> */ }
impl CallControl {
    pub fn with_deadline(at: Instant) -> Self;
    pub fn cancel(&self);
}
```

- `SearchOptions`, `TextSearchOptions`, `get_many` (through a new
  `GetOptions`), `ListOptions` and `GraphExpandRequestV1` gain
  `control: Option<CallControl>`. The last is a Rust-only, non-serialized field.
  Its wire form is an open question.
- The engine checks it at three points. Before dispatch, an expired token
  fails at once. When a worker dequeues the request, an expired token is
  answered without running. During SQL, the worker installs
  `progress_handler(N, ...)` that returns `true` when the token is expired or
  cancelled, and removes it after the request.
- The query embedding step already has its own timeout
  (`embedder_call_timeout_ms`, `runtime_configuration.rs:153`). The token
  should bound it too, through `DispatchError::Cancelled`, which already exists
  (`embed_dispatch/core.rs:29`).
- Error: a new `EngineError::Interrupted { stage }` (`before_dispatch`,
  `queued`, `sql`, `embed`), with `ErrorKind::Interrupted`.
- Separately: dispatch to the least-loaded worker instead of round robin. That
  is a small change that removes head-of-line blocking even without tokens.

## Invariants and consistency

- An interrupted read returns no partial data. Reader transactions are
  read-only and deferred, so abandoning one has no durable effect.
- The progress handler belongs to the request. It is installed and removed
  inside the worker loop, so it cannot fire on another request.
- Writes are out of scope. The single writer and its commit paths are untouched.

## Interaction with ADRs

- `ADR-0.6.0-async-surface.md`: keeps the surface synchronous and supplies the
  deferred cancellation follow-up. An amending ADR records it.
- `ADR-0.6.0-error-taxonomy.md`: a new `#[non_exhaustive]` variant, plus binding
  tables.
- `ADR-0.6.0-database-lock-mechanism-reader-pool-revision.md` and the AC-020 and
  AC-021 reader-parallelism gates: changing dispatch policy must re-run them.
- TypeScript runs calls on a Rust-owned pool. Its Promise cancellation, "not
  supported; followup" in the async ADR, could map onto `cancel()`.

## Cost and performance

A progress handler every N virtual-machine steps costs one atomic load per
callback. N must be tuned against the retrieval latency gates
(`ADR-0.6.0-retrieval-latency-gates.md`). Least-loaded dispatch needs a
per-worker in-flight counter.

## Test plan sketch

- Real database large enough that one search takes over 500 ms. Run it with a
  100 ms deadline. It returns `Interrupted{sql}` near 100 ms, and the worker
  then serves the next request at once.
- Queue test: fill one worker with a blocked request (engine test hook), then
  dispatch 16 short reads. With least-loaded dispatch, none waits behind the
  blocked one.
- Token reuse: cancel after a request completes. The next request on that
  worker is unaffected.
- AC-020 and AC-021 stay green.

## Adapter change once it lands

The bridge passes `CallControl::with_deadline(call_deadline)`. A timed-out call
releases its admission slot when the engine returns `Interrupted`, normally
right after the deadline. The cap can rise to the pool size, and `timeout`
results map from `Interrupted`.

## Open questions for the owner

1. Least-loaded dispatch alone first, as a cheap step, before tokens?
2. Should `graph.expand` carry the token in its V1 wire request, or only through
   the Rust SDK?
3. What progress-handler step count N is acceptable on the latency gates?
