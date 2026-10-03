---
title: ADR-0.8.27-python-subscriber-delivery
date: 2026-10-03
target_release: 0.8.27
desc: Bound Python logger delivery to protect SQLite operation availability
blast_radius: PyO3 subscriber adapter; Python native stub and SDK; lifecycle and binding design; subscriber fault containment
status: proposed
---

# ADR-0.8.27 — Bounded Python subscriber delivery

## Context

The accepted 0.6.0 binding design promises a Python logging adapter and a
heartbeat interval. The shipped `attach_logging_subscriber` method silently
discards both arguments. The engine has structured events, statement profiles,
slow signals, a subscriber registry and RAII detachment, but emits no
Heartbeat and has no operation identity. It sends some events while the writer
mutex or SQLite connection is held. SQLite profile callbacks enter the engine
subscriber boundary from C during statement execution.

Calling Python logging directly at that boundary risks deadlock, recursive
queries and unbounded database latency. A lossless asynchronous adapter with
no bound can instead exhaust process memory if a logger is slow, especially
when per-statement profiling is enabled. A queue cannot be simultaneously
bounded, lossless, nonblocking and tolerant of an arbitrarily slow logger.
Synthetic binding heartbeats would measure the logger worker's timer, not
SQLite or provider progress. Neither Memex's examined integration nor the
current Python SDK requires them for database correctness.

## Decision

Keep the existing engine `Subscriber` and the Python
`attach_logging_subscriber(logger)` helper. The helper delivers structured,
opt-in diagnostics to the supplied Python logger through a bounded,
nonblocking adapter. The logger remains caller-owned and may be collected;
the adapter holds only a weak reference. A logger must expose a callable `log`
method and support weak references. An idle worker checks the weak reference
at least once per second and exits when its logger is collected.

Each attachment has a 4096-record queue and one delivery worker. Engine and
SQLite callback threads enqueue owned records without running Python,
blocking on the logger or doing I/O. When full, the adapter drops the new
record and counts loss; when the worker catches up, it emits one warning
LogRecord with `fathomdb.dropped_records`. Exact Python LogRecord delivery and
pairing are guaranteed only while the queue does not overload and the logger
is live and functioning. Rust engine event emission retains its existing
phase guarantees. Logger exceptions are contained and do not alter database
operation outcomes. Interpreter shutdown, explicit close, replacement and
logger collection end delivery; an already running callback is not forcibly
cancelled. An engine permits at most one retiring worker alongside its active
worker; further replacement while the retiring callback is blocked raises
`OverloadedError` without changing the active attachment.

A logger callback cannot reenter FathomDB database operations through the
same binding call path. Reentry raises `InvalidArgumentError` before engine
dispatch. This avoids a cycle where a handler runs search, reader workers emit
profiles, and each profile causes another handler search. Read-only value
getters that do not enter SQLite may still run.

Remove `heartbeat_interval_ms` from the Python native, stub, SDK and interface
signature. The engine has no truthful operation-scoped heartbeat source today.
This decision narrows the general periodic-liveness requirement in
`dev/design/lifecycle.md` only for Python logger-adapter delivery: the
current engine has no operation-scoped Heartbeat emission, and the adapter
will not synthesize it. The design and `dev/interfaces/python.md` are amended
in the same change. Rust engine `Phase` semantics remain authoritative for
any Heartbeat that a producing subsystem actually emits. It does not remove the
`Phase::Heartbeat` enum member, change existing Rust event fields, forbid a
future separately designed engine heartbeat, or alter TypeScript's subscriber
contract. Do not synthesize public operation IDs in the binding.

## Consequences and verification

- Python applications can opt into engine/SQLite diagnostics without putting
  their logger on a database-owning thread. Under overload, they receive an
  explicit loss signal when delivery resumes. They must not use LogRecord
  counts as a durability or transaction oracle.
- Host callback latency is isolated from normal database work except for a
  short queue lock and bounded record copy. With no subscriber, no worker or
  queue exists.
- The binding owns replacement/close detachment. The engine contains Rust
  subscriber panics at the dispatch boundary, especially the SQLite C
  profile trampoline, so one subscriber cannot abort the process.
- RED/GREEN tests cover real native delivery, slow logger overflow, handler
  failure and reentry, replacement/close, profile and stress routes, and
  process exit. The installed wheel surface records the deliberate removal of
  the heartbeat argument separately from mechanical extraction.
