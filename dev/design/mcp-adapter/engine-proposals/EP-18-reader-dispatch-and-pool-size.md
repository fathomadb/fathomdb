---
title: EP-18 — Work-conserving reader dispatch and a configurable reader pool
date: 2026-10-18
status: ACTIVE
desc: Engine proposal. Part A, idle-first reader dispatch on rendezvous channels, is decided by the owner and built. Part B, a configurable reader pool size, waits for the live concurrency sweep.
inputs:
  - dev/design/agent-access/queue-admission.md
  - dev/experiments/queue-sim/REPORT.md
  - src/rust/crates/fathomdb-engine/src/reader_pool.rs
---

# EP-18 — Work-conserving reader dispatch and a configurable reader pool

Status: part A accepted (owner, 2026-10-18) and built. Part B is a draft that waits for the live sweep
(`queue-admission.md` § 7). Part C, honest embed degradation, moved to
[EP-03](EP-03-hybrid-search-degradation.md) as the degradation reason
`embedder_overloaded` (owner, 2026-10-18).

Paths are relative to `src/rust/crates/`.

## Problem

Before part A, dispatch was `idx = next.fetch_add(1) % 8` followed by a
blocking `send` to that worker's queue of 4. That keeps queues near empty only
when every read takes about the same time. When one read is slow, every
eighth dispatch after it waits behind it, whatever the other seven workers are
doing. The adapter's admission cap counts those blocked reads as in flight,
so head-of-line blocking takes capacity away from every call, not only the
ones on the slow worker. The adapter cannot see or choose a worker; only the
engine can make dispatch work-conserving (`queue-admission.md` § 4).

## Part A — idle-first dispatch (decided: P2, built)

- Each reader worker has a rendezvous channel, `sync_channel(0)`
  (`fathomdb-engine/src/reader_pool.rs`, `READER_WORKER_CHANNEL_CAPACITY`).
  A `try_send` on a rendezvous channel succeeds only when that worker is
  waiting in `recv`, that is, idle
  (<https://doc.rust-lang.org/std/sync/mpsc/struct.SyncSender.html>).
- `ReaderWorkerPool::dispatch` takes the round-robin cursor as a starting
  point and offers the request with `try_send` to each worker in turn. If no
  worker is idle, it falls back to a blocking `send` on the starting worker,
  which waits for that worker's next `recv`: the round-robin behaviour as
  before. A disconnected worker is skipped during the probe; the fallback
  `send` still returns `Err` for a disconnected starting worker.
- There is no mutex on the hot path. The cursor advances by one per dispatch,
  as before.
- Each worker still owns its connection for life, so reads do not queue behind
  one connection (`dev/design/engine.md` § Writer / reader split).
- Callers that wait now wait in `send` rather than behind queued work inside
  the worker. The latency is the same.
- Shutdown uses a blocking `send(Shutdown)` per worker. A non-blocking
  wake-up is lost on a rendezvous channel for a worker that is mid-read, which
  would leave `join` waiting forever. Work that a worker receives after the
  shutdown flag is set is dropped unrun, as before.

Why it is enough: in the simulation idle-first dispatch matched a shared FIFO
in all 2,500 configurations (REPORT § 7). The adapter cap is at most 7 against
8 readers, so an idle reader exists whenever only admitted calls read. In the
simulated head-of-line probe, one 3 s read every 2 s raised the other calls'
p95 from 35 ms to 1.9 s under round-robin; with idle-first dispatch it stayed at
35 ms (REPORT § 6, T5). The live sweep's head-of-line probe validates this on
the real server (`queue-admission.md` § 7.3).

Untested alternative, kept for reference: dispatch to the worker with the
fewest queued requests.

Tests (`fathomdb-engine/tests/reader_dispatch.rs`, feature `test-hooks`):

- `a_slow_read_does_not_delay_reads_while_a_reader_is_idle`: worker 0 held
  inside a read, seven following reads complete while it is held.
- `with_every_reader_busy_dispatch_falls_back_to_round_robin`: all eight held,
  the next read completes only after its round-robin worker is released.
- `close_while_a_reader_is_busy_returns`: close returns once the held read
  finishes.

## Part B — configurable pool size (proposed, waits for the sweep)

- Add `reader_pool_size: Option<u64>` to `EngineConfig`
  (`fathomdb-engine/src/runtime_configuration.rs`), default 8, range set by the
  sweep.
- This goes last because it is the widest change: code assumes exactly
  `READER_POOL_SIZE` (for example in `fathomdb-engine/src/wal_runtime.rs`),
  and AC-021 exercises 8 concurrent readers.
- The adapter would then check `admission_cap < reader_pool_size` from the
  open engine instead of the constant 8 (`fathomdb-mcp/src/config.rs`).
- Decision rule, fixed before the run (`queue-admission.md` § 7.3): proceed
  only if the *N* = 7 text cell meets the cap rule and its p95 is still under
  2 × the *N* = 1 p95.

## Relation to EP-09

[EP-09](EP-09-interruptible-reads.md) frees a worker after a read's deadline.
EP-18 part A stops healthy requests from waiting behind a slow one before the
deadline. Once single reads exceed the deadline, EP-09 is what helps. Both are
needed; neither replaces the adapter queue.
