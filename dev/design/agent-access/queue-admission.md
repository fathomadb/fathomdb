---
title: Admission queue between an agent and SQLite
date: 2026-10-18
status: PROPOSED
desc: Maps every queue and limit from an MCP agent to SQLite, records the immediate-busy admission defect and the reader-pool head-of-line risk, and proposes a bounded FIFO admission queue with deadlines, abandoned-work accounting and an honest retry_after_ms, plus engine proposal EP-18 and a concurrency sweep with its decision rule written before the run.
inputs:
  - dev/design/agent-access/README.md
  - dev/design/mcp-adapter/architecture.md
  - dev/design/mcp-adapter/requirements.md
  - dev/interfaces/mcp.md
  - dev/design/mcp-adapter/engine-proposals/EP-09-interruptible-reads.md
  - src/rust/crates/fathomdb-mcp/src/{registry,bridge,config,edge,error,tools,lib,main}.rs
  - src/rust/crates/fathomdb-sdk/src/engine.rs
  - src/rust/crates/fathomdb-engine/src/{reader_pool,search_api,embedding,runtime_configuration,projection_generation,runtime_lifecycle,open}.rs
  - src/rust/crates/fathomdb-engine/src/embed_dispatch/core.rs
  - release/0.8.28:dev/adr/ADR-0.8.28-sqlite-per-connection-page-cache.md
---

# Admission queue between an agent and SQLite

Work item 1 and Block 2 row B2-7 of [README.md](README.md). This is a design
only: no code or test changes until the Fable-medium review and HITL review
have passed.

## 1. Summary

- **Defect.** The adapter refuses a call as `busy/admission` the moment a
  store is at its cap (`fathomdb-mcp/src/registry.rs:342-345`). The
  architecture says excess calls wait up to the deadline. Six concurrent
  Claude subagents got 29 refusals in about 125 calls (README § Why).
- **Recommendation.** Replace the counter check with a bounded FIFO wait
  queue in the adapter: a fair `tokio::sync::Semaphore` per store, a queue
  depth limit, the call's own deadline as the wait bound, and the permit
  moved into the blocking closure so abandoned work keeps counting. A call
  is admitted only while enough of its deadline remains to finish, so late
  admissions do not turn into abandoned work. Busy carries an honest
  `retry_after_ms` computed from the capacity that is actually free (B2-7).
- **Owner decision, 2026-10-18.** The contract in `dev/interfaces/mcp.md`
  changes from "refuse at once" to "wait FIFO up to the deadline". One
  admission cap is shared by text and hybrid, provided the mixed-load check
  in § 7.4 holds. A call that cannot get a slot in time fails with `busy`
  and `retry_after_ms`; "run anyway with a partial result" is rejected. The
  early-shed threshold and the deadline are parameters whose values (fixed
  or adaptive) are set by a separate queue simulation, still pending.
- **The queue alone is not enough.** The engine's reader pool dispatches
  round-robin to per-worker queues with no regard for load
  (`reader_pool.rs:1216-1226`). One slow read blocks the reads queued behind
  it on that worker while other workers are idle, and every blocked read
  holds an admission permit. The adapter queue hides this as latency. It
  does not fix it. EP-18 (new) proposes a work-conserving shared queue in
  the engine and a configurable reader pool.
- **Defaults.** Do not move `admission_cap` from 4 until the sweep in § 7
  has run. The decision rule is fixed below, before any measurement.

## 2. The path, hop by hop

Each row is one place a call can wait or be refused. Line numbers are on
`exp/queue-design` at `9ea33ff19`, except the page-cache ADR, which is on
`release/0.8.28`.

| # | Hop | Queue / limit type | Capacity | Ordering | When full | Timeout / cancellation | What the agent sees |
| --- | --- | --- | --- | --- | --- | --- | --- |
| H1 | MCP stdio edge, rmcp 3.5.1 | None. Each request is spawned as its own task (`rmcp-3.5.1/src/service.rs:1585-1617`); the adapter spawns again so a panic stays local (`fathomdb-mcp/src/edge.rs:248-265`) | Unbounded | Arrival order on stdin; no ordering after spawn | Never full | `notifications/cancelled` cancels the request's token (`service.rs:1647-1652`). The adapter never reads `context.ct`, so a client cancel has no effect today | Nothing at this hop |
| H2 | Store open wait | `watch` channel on slot phase (`registry.rs:375-397`) | Unbounded waiters | None (all wake on change) | n/a | Waits up to the call deadline (`tools.rs:235-243`, default 10 s, `config.rs:110`); progress notifications when a token is given (`registry.rs:391-394`) | `not_ready/<phase>` at the deadline (`registry.rs:388-390`) |
| H3 | Open-store limit | `max_open_stores` = 1 (`config.rs:108`, forced by `config.rs:240-242`) | 1 managed store | n/a | Drains an idle LRU store; refuses if none is drainable (`registry.rs:418-432`) | None | `busy/admission`, same code as H4 (`error.rs:172-178`) |
| H4 | **Admission** (`Registry::acquire`) | Per-store `AtomicUsize` `in_flight` (`registry.rs:71`), checked under the registry mutex | `admission_cap`, default 4 (`config.rs:109`), allowed 1..=7 (`config.rs:243-246`) | None | **Refuses at once** (`registry.rs:342-345`) | No wait at a ready store | `busy/admission`, hint "retry after in-flight calls finish" (`error.rs:172-178`) |
| H5 | Bridge | `spawn_blocking` raced against the deadline (`bridge.rs:39-45`); tokio's blocking pool, 512 threads by default (`tokio-1.52.3/src/runtime/builder.rs:300`) | Bounded in practice by H4 | n/a | n/a | At the deadline the call is **abandoned**, not cancelled: the closure owns the `StoreGuard`, so the slot stays taken until the thread returns (`bridge.rs:4-7`, `registry.rs:158-167`) | `timeout/deadline` (`error.rs:157-170`) |
| H6 | `fathomdb-sdk` | None: argument guards, then a direct engine call (`fathomdb-sdk/src/engine.rs:185-222`) | — | — | — | — | SDK errors map through `ToolError::from_sdk` (`error.rs:225-247`) |
| H7 | Query embedding (hybrid only), on the bridge thread before reader dispatch | One shared `VecDeque` drained by N provider workers (`embed_dispatch/core.rs:296`, `:641`) | `embedder_pool_size` default 5, waiting capacity ×4 = 20 (`runtime_configuration.rs:205`, `:233-235`; `core.rs:356-358`) | **FIFO**, work-conserving | `Saturated` at once (`core.rs:441-446`) | Per-request deadline `embedder_call_timeout_ms`, default 30 s (`embed_dispatch.rs:14`), covering queue and service; expired queued requests are dropped (`core.rs:186-192`, `:441`) | **Silent text-only search.** Search maps every dispatch error to "no query vector" (`search_api.rs:543-550`), and the engine sets `soft_fallback` only when a vector exists (`search.rs:1476`, `:1699-1716`). `Overloaded` (`embedding.rs:164`) is raised only by `embed_text`, which the adapter does not call, so `error.rs:242` is unreachable from search |
| H8 | Mean-vector read (hybrid, `bge-small` identity) | Writer connection `Mutex` (`lib.rs:470`) | 1 | Mutex (unfair) | Blocks | None | Latency only (`search_api.rs:551-556`, `mean.rs:345-347`) |
| H9 | Orient readiness read | Same writer `Mutex` (`projection_generation.rs:1837-1841`) | 1 | Mutex | Blocks | None | Latency only |
| H10 | **Reader pool dispatch** | 8 workers (`reader_pool.rs:4`, `open.rs:1323-1355`), each with its own bounded `sync_channel` | 4 per worker (`reader_pool.rs:499`, `:955`) | **Round-robin** on a global `fetch_add` (`reader_pool.rs:1224`); load is ignored | Blocking `send` (`reader_pool.rs:1225`). `try_send` is used only for the shutdown wake-up (`:1238`) | None. The caller blocks on `response_rx.recv()` with no timeout (`search_api.rs:624`, `:1052`) | Latency only; `Closing` when the pool is shut down before dispatch (`search_api.rs:621-623`) |
| H11 | Worker loop | FIFO per worker (`reader_pool.rs:528`) | 1 running | FIFO | — | On shutdown the worker exits before taking the next queued request (`reader_pool.rs:529-531`); queued callers see their response channel drop | `EngineError::Storage` (`search_api.rs:624`), which the adapter maps to `internal` (`error.rs:244`) |
| H12 | rusqlite → SQLite | Busy handler, 5 000 ms default set by rusqlite at open (`rusqlite-0.40.1/src/inner_connection.rs:118`); readers keep it (`connection_runtime.rs:97-106`) | — | — | — | WAL readers do not block on the writer; `SQLITE_BUSY` remains possible during recovery, exclusive locking or last-connection cleanup ([sqlite.org/wal.html § 9](https://www.sqlite.org/wal.html)). One read transaction per request (`search_api.rs:529-535`) | `Storage` → `internal`, rare |
| H13 | Page cache | One page cache per connection, default `cache_size`, no global page-cache mutex (`release/0.8.28:dev/adr/ADR-0.8.28-sqlite-per-connection-page-cache.md`, Decision) | 2 000 KiB per reader | — | — | — | Latency only |

Notes on the table:

- **The cap is per store.** `in_flight` lives on the slot
  (`registry.rs:71`, `:343`). Every connection made by `Adapter::connection`
  shares one registry (`lib.rs:111-114`), so in embedded mode several MCP
  sessions share one cap, and the host's own SDK reads draw on the same
  eight readers without passing H4.
- **H3 and H4 share one error code.** A client cannot tell "this store is
  full" from "another store is in use". That matters for `retry_after_ms`:
  the two have different wait times.
- **The page-cache ADR removes one cross-worker effect.** Before it, one
  heavy reader could take other readers' cached pages, and later reads ran
  about 2.5× slower (ADR § Context). That robbery was a coupling between
  workers. It is gone, so head-of-line blocking (§ 4) is now the remaining
  coupling.

## 3. The defect

`Registry::acquire` returns `busy` as soon as `in_flight >= cap` at a ready
store (`registry.rs:342-345`). The sources disagree:

| Source | Says |
| --- | --- |
| `architecture.md` § "Threads, cancellation and admission" (lines 237-242) | "Excess calls wait up to the deadline, then return busy (MR-033.2)." |
| `requirements.md` MR-033.2 (lines 352-355) | "excess calls wait or return a typed busy result". Either is allowed |
| `dev/interfaces/mcp.md:42` | "A call is refused as `busy/admission` when `in_flight >= admission_cap`." Immediate refusal is the contract |
| `tests/bridge.rs:45-63`, `:79-100` | Assert "busy is immediate" and the exact busy/timeout class sequence |

So the code matches the interface and its tests. It contradicts the
architecture. MR-033.2 permits both. The fix is therefore a contract change
to `dev/interfaces/mcp.md`, and the two bridge tests change with it (§ 9).
They are rewritten to the new contract, not edited to pass.

**Owner decision, 2026-10-18:** accepted. Excess calls wait FIFO up to the
deadline, replacing immediate refusal. The busy codes follow § 6.1.

Why it matters to agents: an agent cannot sleep, and the hint "retry after
in-flight calls finish" gives no time. Six agents sharing one stdio
connection retried straight into the cap again. A call that would have
been served 200 ms later instead failed, and the agent spent a turn on it.

## 4. Head-of-line risk in the reader pool

The dispatch rule is `idx = next.fetch_add(1) % 8` followed by a blocking
`send` to that worker (`reader_pool.rs:1224-1225`). The comment explains the
choice: no runtime mutex on the hot path, and "round-robin dispatch keeps
queue depth at ~0 on hot paths" (`reader_pool.rs:496-498`, `:1207-1209`).
That holds only when every read takes about the same time.

When one read is slow (a deep `expand`, a large-limit hybrid search, a cold
page cache):

1. Worker *k* runs the slow read.
2. Each eighth dispatch after it lands in worker *k*'s queue, whatever the
   other seven are doing. That read waits for the slow one to finish.
3. Its bridge thread blocks on `response_rx.recv()` with no timeout
   (`search_api.rs:624`). At the adapter deadline it is abandoned, but it
   keeps its admission slot (H5).
4. With a cap of 4, the slow read plus three reads queued behind it fill
   the cap. From then on every call is refused, or under the proposed
   queue waits, while seven workers are idle.

How the adapter queue interacts with this:

- **It hides it.** Calls wait instead of failing, so the agent sees latency
  instead of `busy`. Throughput is the same.
- **It compounds it in one way.** Admission counts a blocked read as in
  flight, so head-of-line blocking takes capacity away from every other
  call, not only from the ones on worker *k*. A larger cap delays the point
  at which the cap is full, but it also raises the number of reads that can
  pile up behind worker *k*.
- **It cannot fix it.** The adapter cannot see or choose a worker. Only the
  engine can make dispatch work-conserving. That is EP-18 part A (§ 6.2).

A spare reader does not give headroom. `architecture.md` keeps the cap
below 8 "so that abandoned work cannot exhaust the engine". Round-robin
never steers a request to the idle worker, so the eighth reader is not held
in reserve. It only lowers the odds of a collision.

EP-09 (interruptible reads) shortens the slow read once its deadline
passes. It does not stop requests queueing behind the slow read before
then. The two proposals are complementary.

## 5. Exemplars

Three were chosen for their structure. Claims were checked against source
or primary documentation.

### 5.1 tower `ConcurrencyLimit` and `LoadShed`

Source: tower 0.5.3, `src/limit/concurrency/service.rs`,
`src/limit/concurrency/future.rs`, `src/load_shed/mod.rs`
(<https://docs.rs/tower/0.5.3/tower/limit/concurrency/struct.ConcurrencyLimit.html>,
<https://docs.rs/tower/0.5.3/tower/load_shed/struct.LoadShed.html>).

- **Structure.** A `tokio::sync::Semaphore` of `max` permits
  (`service.rs:26-28`). `poll_ready` acquires a permit (`service.rs:64-68`),
  `call` moves it into the response future (`service.rs:81-92`), and the
  permit is released when that future completes or is dropped
  (`future.rs:21-27`). `LoadShed` turns "not ready" into an immediate
  `Overloaded` error instead of waiting (`load_shed/mod.rs:43-59`).
- **Ordering.** FIFO, inherited from tokio's fair semaphore.
- **Adopt.** The permit owns the slot: admission is a permit, not a
  counter, and the permit lives as long as the work. The split between
  waiting (`ConcurrencyLimit`) and shedding (`LoadShed`) maps onto "wait
  while there is time" and "queue full".
- **Reject.** Tower ties the permit to the response future, so dropping the
  future frees the slot. Our work runs on a blocking thread that outlives
  the future, so the permit moves into the blocking closure, as
  `StoreGuard` already does (`bridge.rs:39-43`). We do not adopt
  `Service`/`Layer` as a framework; one semaphore does not need it.

### 5.2 PgBouncer

Sources: <https://www.pgbouncer.org/config.html>;
<https://github.com/pgbouncer/pgbouncer/blob/master/src/objects.c>.

- **Structure.** A proxy in front of a database, with a fixed pool of
  server connections per user/database pair (`default_pool_size`, default
  20). A client that finds no free server connection is parked on the
  pool's `waiting_client_list`: `change_client_state` appends it at the tail
  and stamps `wait_start`, and `reuse_on_release` hands a released
  connection to the head of the list, "to give fair chance". That is FIFO
  per pool. Clients are bounded by `max_client_conn` (default 100); the
  documentation describes the gap between client and server limits as the
  queue. `query_wait_timeout` (default 120 s) bounds the wait: "If the query
  is not assigned to a server during that time, the client is
  disconnected." `query_wait_notify` (default 5 s) tells a client that it is
  queued. `reserve_pool_size` (default 0) extra connections are opened when
  the head waiter has waited `reserve_pool_timeout` (default 5 s).
- **Adopt.** It is the same problem: a bounded set of database connections
  shared by many callers through a proxy. We take a FIFO queue per pool, a
  bound on waiters, a bounded wait, and a notice that the call is queued
  (our progress notification).
- **Reject.** Disconnecting on timeout: we return a typed busy with
  `retry_after_ms`. The reserve pool: today it would be the eighth reader,
  but round-robin cannot route a waiter to it (§ 4). After EP-18 part A, a
  reserve permit for a waiter older than some threshold is a possible
  follow-up; it is not part of this design.

### 5.3 Envoy adaptive concurrency and Netflix `concurrency-limits`

Sources:
<https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/adaptive_concurrency_filter>,
<https://www.envoyproxy.io/docs/envoy/latest/api-v3/extensions/filters/http/adaptive_concurrency/v3/adaptive_concurrency.proto>,
<https://github.com/Netflix/concurrency-limits>,
`LifoBlockingLimiter.java` in that repository.

- **Structure.** Envoy's gradient controller samples request latency,
  periodically pins concurrency at `min_concurrency` (default 3) to measure
  `minRTT`, and sets `limit_new = gradient × limit_old + headroom`, where
  `gradient = (minRTT + B) / sampleRTT`, `B` defaults to 25 % of `minRTT`, and
  headroom is the square root of the limit. Requests over the limit are
  refused at once with `concurrency_limit_exceeded_status` (default 503).
  Netflix applies the same Little's-law reasoning (Vegas, Gradient2) and
  offers `LifoBlockingLimiter`: a bounded backlog (default 100) with a wait
  timeout (default 1 s), served last in, first out.
- **Adopt.** Latency is the signal that a queue is building, so the queue
  keeps service-time samples (§ 6.1) for `retry_after_ms`, the admission
  rule and the sweep. A partitioned limit (Netflix "percentage
  enforcement") is the shape to use if embedded hosts ever need a
  guaranteed share, or if § 7.4 calls for separate text and hybrid limits.
- **Reject, for now.** An adaptive limit needs steady traffic to sample.
  An MCP store serves bursts from a few agents, and the ceiling is a fixed
  pool of eight readers, not an unknown backend. A fixed, measured cap is
  simpler and testable. LIFO suits services that prefer fresh requests; an
  agent waiting on its own tool call is never stale before its deadline,
  and LIFO starves the oldest call, so FIFO fits better.

### 5.4 Considered and not chosen

- **HikariCP** (<https://github.com/brettwooldridge/HikariCP>). A fixed pool
  (`maximumPoolSize`, default 10) where `getConnection()` blocks up to
  `connectionTimeout` (default 30 000 ms) and then throws. It has no bound on
  waiters. Its `leakDetectionThreshold` only logs: "the amount of time that
  a connection can be out of the pool before a message is logged indicating
  a possible connection leak" (default 0, disabled). It does not reclaim
  the connection. That supports the `abandoned` counter here: when work
  cannot be reclaimed, the useful thing is to make it visible. PgBouncer
  covers the same ground with a bounded queue, so it was preferred.
- **SQLite's busy handler.** It sleeps and retries one connection on a lock,
  with no queue, no ordering and no fairness, and SQLite may skip it to
  avoid deadlock ([sqlite3_busy_handler](https://www.sqlite.org/c3ref/busy_handler.html),
  also recorded at `write_commit.rs:476-483`). It is not an admission
  model. It stays as H12.
- **deadpool and bb8.** The same structure as HikariCP.
- **The engine's own embed dispatcher** (H7) is the in-repo model for
  EP-18: one FIFO, N workers pulling, a deadline per request, expired
  entries purged on submit.

## 6. The queue

### 6.1 Adapter: bounded FIFO admission

Where it lives: `fathomdb-mcp`, in `Registry::acquire`, per store slot. The
SDK needs nothing: it is a pass-through (H6).

**Building block.** `Arc<tokio::sync::Semaphore>` with `admission_cap`
permits, acquired with `acquire_owned`. Why this one:

- It is fair: "permits are given out in the order they were requested"
  (`tokio-1.52.3/src/sync/semaphore.rs:19-24`,
  <https://docs.rs/tokio/1.52.3/tokio/sync/struct.Semaphore.html>). That
  gives FIFO without writing a queue.
- A dropped `Acquire` future removes itself from the wait list and returns
  any permits already assigned to it (`batch_semaphore.rs:686-707`). So a
  waiter that times out or is cancelled leaves the queue at no cost, and a
  permit granted just before the timeout fires is not leaked.
- `OwnedSemaphorePermit` is `'static`, so it can move into the
  `spawn_blocking` closure with the `StoreGuard`.

Rejected alternatives: `Mutex<VecDeque>` + `Notify` (re-implements the
semaphore, more to test); `async-channel` or `flume` as a ticket queue (a
new dependency for the same FIFO); keeping the counter and polling (no
ordering, wasted wakeups).

**State per store.** Three quantities, each with one owner:

| Quantity | Source | Changed by |
| --- | --- | --- |
| `in_flight` | Derived: `cap − semaphore.available_permits()`. The separate `AtomicUsize` (`registry.rs:71`) is removed | Permit acquire and drop |
| `queued` | `AtomicUsize` | A `QueueTicket` drop guard: increment on creation, decrement in `Drop`. Timeout, cancel, panic and admission all release it the same way |
| `abandoned` | `AtomicUsize` | Set by `bridge::call` when its deadline fires; the guard's `Drop` decrements it when the thread returns |

Deriving `in_flight` from the permits closes a window that the idle sweep
(`registry.rs:595`) and LRU drain (`registry.rs:422`) could otherwise see:
a waiter that holds a permit but has not yet built its guard already
counts. The idle `Notify` fires from the guard's `Drop` after the permit is
released, when `available_permits() == cap`.

**Service-time estimate.** An exponentially weighted mean, `α = 0.2`, kept
per store and per tool, with search split by mode (`search_text`,
`search_hybrid`, `fetch`, `expand`, `orient`). A sample is the time from
permit acquired to permit released, for completed calls only. Abandoned
calls never complete within their deadline, so the mean leaves them out
and reads low exactly when abandonment is happening. The formulas below
correct for that through effective capacity, not through the mean.

**Effective capacity.** `c_eff = cap − abandoned`. Abandoned calls hold
permits with no known end, so they are not capacity. `stuck` is true if
and only if `c_eff == 0`.

**Acquire sequence** (replaces the `Phase::Ready` arm at
`registry.rs:342-357`). `need` is the **early-shed threshold**: the least
deadline a call must have left to be started.

- **Parameter.** `need`, together with the call deadline itself
  (`call_deadline_ms`, default 10 s). **Value set by the queue simulation
  (pending).** The simulation decides whether each is a fixed configured
  number or adaptive, for example a percentile of the last *n* service
  times of that tool.
- **Placeholder until then.** `need = max(floor, ewma_tool)`, with
  `floor = 100 ms` (the minimum `call_deadline_ms`, `config.rs:248-250`).

The sequence:

1. Wait for `Ready` as today (H2).
2. If no permit is free and `queued >= queue_limit`, refuse at once:
   `busy/admission`, reason `queue_full`.
3. Otherwise take a `QueueTicket` and await `acquire_owned()` under
   `timeout_at(deadline − need)`, in a `select!` with the client's cancel
   token (`context.ct`, H1) and the slot's phase watch.
4. On a permit, re-check under the registry mutex that the slot is still
   `Ready` at the same epoch, and that `deadline − now ≥ need`. If both hold,
   build the `StoreGuard` holding the permit. If the time check fails,
   drop the permit and refuse: `busy/admission`, reason `deadline`.
5. If the wait in step 3 times out, refuse: `busy/admission`, reason
   `deadline`, at once. This sheds on time spent waiting, as CoDel does:
   a call that cannot finish before its deadline is not started.
6. On client cancel, drop out of the queue and return an error result. The
   MCP specification says the server should not respond to a cancelled
   request, but rmcp sends whatever the handler returns
   (`rmcp-3.5.1/src/service.rs:1603-1617`), so the client receives an error
   it ignores. There is no JSON-RPC "cancelled" result to send.
7. If the phase changed to `Draining` or `Closed` while waiting, fall back
   into the existing open wait with the remaining deadline.

Why step 4 is needed now, not after the sweep: `tools.rs:245-253` computes
one deadline and passes the same instant to `bridge::call`. A waiter
admitted at 9.9 s of a 10 s deadline has 100 ms to run. It times out, and
its permit stays held until its thread returns. Late admission turns queue
wait directly into abandoned work.

**Queue depth limit.** `queue_limit`, a new configuration key, proposed
default `2 × admission_cap`. A call that joins behind *q* waiters on *c*
permits waits about `ceil((q+1)/c)` service times; at 2 × cap that is three
service times, well inside a 10 s deadline for sub-second reads. A deeper
queue mostly converts quick refusals into refusals at `deadline − need`,
which cost the agent more time. The overload arm in § 7.2 tests this.

**`retry_after_ms` (B2-7).** An estimate the server can justify, never a
promise. Clamped to `[50, call_deadline_ms]`.

| Reason | What the caller needs | Formula |
| --- | --- | --- |
| `queue_full` | One free queue slot: one call must finish so that the head waiter moves to running | `ceil(ewma_tool / c_eff)` |
| `deadline` | A permit within its budget, given the *q* queued at the moment of refusal | `ceil((q + 1) / c_eff) × ewma_tool` |

- Omit `retry_after_ms` when fewer than 8 calls of that tool have completed
  since the store opened. A made-up number is worse than none.
- Omit it when `stuck` is true. Abandoned work has no known end.
- `store_switch` (H3) reports the `deadline` formula for the store that is
  in use.

**Busy codes.** One code where the agent's action is the same, a second
where it differs:

| `code` | `busy.reason` | When | Agent action |
| --- | --- | --- | --- |
| `admission` | `queue_full` | Depth limit reached on arrival | Wait `retry_after_ms`, retry |
| `admission` | `deadline` | No permit in time to finish | Wait `retry_after_ms`, retry |
| `admission` | `engine_overloaded` | SDK `Overloaded` (`error.rs:242`); today only `embed_text` raises it, which no tool calls | Retry later; no `retry_after_ms`, since the adapter cannot see the engine queue |
| `store_switch` | — | Another store is open with calls in flight (H3) | Use the open store, or wait `retry_after_ms` |

`queue_full` and `deadline` ask for the same action, so they share the
code `admission` and differ only by `reason`. The hints sidecar renders
them with the same hint. `store_switch` asks for a different action, so it
is its own code.

**Field placement.** The error object is `{class, code, message, remedy,
candidates?, detail?}` and `detail` is reserved for the SDK `ErrorKind`
name (`dev/interfaces/mcp.md` § 6). The busy fields go in one optional
sub-object, present only when `class` is `busy`:

```json
{"error": {"class": "busy", "code": "admission", "message": "…", "remedy": "…",
  "busy": {"reason": "deadline", "retry_after_ms": 600,
           "in_flight": 4, "queued": 8, "abandoned": 0, "stuck": false}}}
```

The hints design (item 2) reserves `retry_after_ms` for this document, and
its reviewer settles placement within the error object. This proposes the
`busy` sub-object and aligns with whatever that review decides.

**Fairness across sessions and agents.**

- stdio: one client, one session. If Claude Code subagents share the
  parent's MCP connection (likely, not verified here), the six subagents
  were one session. Either way the server cannot tell agents apart; there
  is no agent identity in the request. FIFO per call is the only fairness
  available, and it is enough: agents with similar call rates get similar
  shares.
- Embedded: several `connection()` sessions share one registry and cap
  (`lib.rs:111-114`). FIFO across all of them is the first step. If one
  session floods the queue, the next step is a per-session limit on queued
  entries (for example half of `queue_limit`). Weighted or partitioned
  shares wait for evidence of starvation.
- Host SDK reads in embedded mode bypass the adapter. Only the host can
  bound them. The host sets `admission_cap` (MR-033.2), and EP-18 part B
  lets it size the reader pool to match.

**Progress and visibility.** A queued call with a progress token gets a
notification each second, as the open wait does (`registry.rs:391-394`),
with the message `queued: q waiting, c running`. A position in the queue is
not reported: computing it needs a ticket sequence with arrival and served
counters, and cancelled waiters make served counts drift, so it would be an
estimate dressed as a fact. Tracing at stderr logs each refusal (with
reason), each abandoned call and each admission after a wait, with store,
`in_flight`, `queued` and `abandoned`.

**Edge cases.**

- A panic on the blocking thread unwinds and drops the guard and the
  permit, so the slot returns. The workspace sets no `panic = "abort"`
  profile; under abort the process ends anyway.
- A timeout cannot leak a permit: `Acquire::drop` returns any permits
  assigned before `timeout_at` fired (`batch_semaphore.rs:700-706`).
- `QueueTicket` is dropped on every exit path, including panics in the
  waiting task, so `queued` cannot drift.

**Drain, idle close and shutdown.**

- Idle expiry and LRU drain require `in_flight == 0` and `queued == 0`
  (`registry.rs:422`, `:595`). Otherwise a store could start draining while
  calls wait for it.
- When a drain starts, queued waiters see the phase change and return to
  the open wait (step 7), so a reopen within the deadline still serves
  them.
- Shutdown sets `shutting_down` (`registry.rs:612-613`). Waiters wake on the
  phase change and return `not_ready/draining`, as today. The semaphore is
  per slot and survives reopen; it is not closed, so nothing is rebuilt.

### 6.2 Engine: EP-18 (new engine proposal)

Title: work-conserving reader dispatch and a configurable reader pool.
Drafted here; it gets its own file under
`dev/design/mcp-adapter/engine-proposals/` after review. Priority order:
A, then B. Part C, honest embed degradation, moved to EP-03 (owner
decision 2026-10-18).

- **Part A, dispatch.** Replace round-robin per-worker channels with one
  shared bounded FIFO that idle workers pull from, as the embed dispatcher
  does (H7).
  - *The mutex cost.* The round-robin comment avoids a mutex on the hot
    path (`reader_pool.rs:1207-1209`). But the hybrid path already takes the
    embed dispatcher's `QueueState` mutex per request
    (`embed_dispatch/core.rs:309`, `:422`), and an uncontended mutex costs
    tens of nanoseconds against reads measured in milliseconds. The
    engine-direct harness in § 7 confirms or refutes this.
  - *Not a shared connection.* `dev/design/engine.md:52-66` requires reads
    not to queue behind one connection. A shared queue does not do that:
    each worker still owns its connection for life
    (`reader_pool.rs:6-13`). Only the hand-off of requests is shared.
  - *Lock-free alternative.* Give each worker a rendezvous channel
    (`sync_channel(0)`). Dispatch tries `try_send` on each worker in turn;
    on a rendezvous channel that succeeds only when the worker is waiting
    in `recv`, that is, idle
    (<https://doc.rust-lang.org/std/sync/mpsc/struct.SyncSender.html>). If
    no worker is idle, fall back to today's round-robin blocking `send`. This
    removes queueing behind a busy worker whenever any worker is idle, with
    no mutex, but it still queues behind a slow worker under full load.
- **Former part C, now in EP-03.** When query embedding fails with
  `Saturated` or `QueuedExpired`, the search silently runs text-only (H7).
  By owner decision (2026-10-18) this is reported by EP-03 as the common
  degradation reason `embedder_overloaded`, alongside its other hybrid
  degradation reasons
  ([EP-03](../mcp-adapter/engine-proposals/EP-03-hybrid-search-degradation.md)).
- **Part B, pool size.** Add `reader_pool_size: Option<u64>` to
  `EngineConfig` (`runtime_configuration.rs:146-157`), default 8, range set
  by the sweep. This goes last because it is the widest change: code
  assumes exactly `READER_POOL_SIZE` (for example `wal_runtime.rs:74`,
  `:389`, `:422`), and AC-021 exercises 8 concurrent readers
  (`reader_pool.rs:1-4`). The adapter would then check
  `admission_cap < reader_pool_size` from the open engine instead of the
  constant 8 (`config.rs:243-246`).
- **Relation to EP-09.** EP-09 frees a worker after the deadline. EP-18
  stops healthy requests from waiting behind a slow one before the
  deadline. Both are needed; neither replaces the adapter queue.

## 7. Defaults: the concurrency sweep

### 7.1 Why not just raise the cap

- 4 → 7 against 8 readers raises the number of reads that can pile up behind
  one slow worker (§ 4).
- Hybrid calls hold a permit while they embed (H7 runs on the bridge
  thread). With 5 provider workers on one GPU, more than 5 hybrid calls
  queue in the embedder, and the extra permits buy nothing.
- How much concurrency one RTX 3090 embedder serves is unknown. Guessing it
  would be guessing the answer.

### 7.2 Design

**Scope of the result.** All arms run on one host with an RTX 3090,
default embedder on `cuda:0` only (`FATHOMDB_EMBED_DEVICE=cuda:0`, feature
`embed-cuda` in `fathomdb-mcp/Cargo.toml`). The defaults chosen are
defaults for that host class. They are not CPU or Jetson defaults; those
platforms keep the current default until measured on their own.

**Stores.**

- *Small:* `conv0.sqlite`, the store used by
  `dev/experiments/mcp-agent-loop/` (2.9 MB, about the size of one reader's
  2 000 KiB page cache).
- *Large:* all ten LoCoMo conversations in one store, built with the
  `locomo_load` example (the store README item 5 also needs). The
  head-of-line probe runs only on this store, because on the small store a
  slow read may not exist.

**Common settings.** A fixed list of LoCoMo questions, each issued as
`search` with the default limit, in a seeded fixed order, the same for
every cell. Closed loop: each client issues its next call when the previous
one returns. One warm-up pass, then 5 repetitions of 200 calls per client
per cell; report the median across repetitions with the range. Two
harnesses: the MCP binary over stdio (what agents see) and SDK-direct at
the same concurrency (engine only); the difference is adapter overhead.

**Arms.**

| Arm | Clients | Cap | Store | Load | Purpose |
| --- | --- | --- | --- | --- | --- |
| Capacity | *N* ∈ {1, 2, 4, 5, 6, 7} | *N* | Small and large | text; hybrid | Cap choice (§ 7.3). Cap = *N*, so the queue stays empty here |
| Overload | 6 and 12 | 4, and the chosen cap | Large | text; hybrid; mixed 50/50 | Mirrors the incident: queue depth, refusals, `retry_after_ms` accuracy, mixed-load mutex check |
| HOL probe | 4 and 7 | *N* | Large | text, plus one slow read every 2 s | EP-18 part A |
| Writer | 4 and the chosen cap | *N* | Large | text and hybrid while an embedded host writes and checkpoints | Writer mutex and checkpoint interference; projection embedding contends for the same embed dispatcher (`projection_worker.rs:617-620`) and the same GPU |
| Cold cache | 4 | 4 | Large | text | First 50 calls after a fresh engine open, reported apart from the warm calls. Dropping the OS page cache needs root and is an escalated step; without it this arm measures a cold SQLite cache only, and says so |

**Measures in every arm.** Latency p50, p95, p99 per call and per tool;
throughput (calls per second); counts of busy by reason, timeout and soft
fallback; GPU utilisation.

**Overload arm, additional measures.** `queued` (sampled each 100 ms from
tracing), refusals by reason, and `retry_after_ms` accuracy: the client
waits the advised time and retries, and the harness records the advised
wait against the observed time from refusal to admission.

**Mixed-load mutex check (owner, 2026-10-18).** Text calls in the mixed
arm cross three shared points that hybrid calls also use: the writer
connection mutex that hybrid with `bge-small` takes for the mean vector
(H8, `search_api.rs:553`), the embed dispatcher's `QueueState` mutex
(`embed_dispatch/core.rs:309`, `:422`), and the reader channels (H10).
Compare text p95 in the mixed arm with text p95 in the text-only overload
arm at the same client count and cap. If a measurement build with timing
spans is approved, record wait time at each of the three points so a
breach can be attributed; without it the comparison still decides.

**HOL probe.** The slow read is an `expand` at `max_depth = 3` from a
high-degree node, or a hybrid search at limit 100, whichever is slower on
the large store. Validity check, fixed before the run: the probe read
takes at least 1 s standalone (median of 5). If no read on the large
store reaches 1 s, the probe is invalid and reports "not measured", not
"no head-of-line blocking".

### 7.3 Decision rules (fixed before the run)

- **Cap.** The bound: p95 at most 2 × the *N* = 1 p95 of the same mode and at
  most 2 500 ms (a quarter of the call deadline). Choose the largest *N* in
  {1, 2, 4, 5, 6, 7} for which throughput rises by at least 10 % over the
  next smaller *N* and p95 meets the bound. Ties and overlapping ranges go
  to the smaller *N*. The rule can select 5 or 6 because they are in the
  sweep.
- **Shared cap.** One cap for both modes: the smaller of the text and
  hybrid choices, unless § 7.4 says otherwise.
- **Queue limit.** In the overload arm at the chosen cap, keep
  `2 × admission_cap` unless more than 1 % of calls end in reason
  `deadline`. Then halve it and re-run that cell. If more than 5 % end in
  `queue_full` while fewer than 1 % end in `deadline`, double it and re-run.
- **`retry_after_ms`.** Ship it if the median of |observed − advised| /
  advised is at most 0.5 and fewer than 20 % of retries after the advised
  wait are refused again. Otherwise ship busy without `retry_after_ms` and
  revise the formula.
- **EP-18 part A (pending simulation).** The queue simulation decides
  part A. This probe supplies the measured input: proceed if the valid HOL
  probe raises p95 of the other calls by more than 2 × at either *N*;
  otherwise record part A as not needed at today's call mix, unless the
  simulation shows otherwise.
- **EP-18 part B.** Proceed only if the *N* = 7 text cell meets the cap rule
  and its p95 is still under 2 × the *N* = 1 p95: then the ceiling, not the
  workload, is the limit.
- **Writer and cold-cache arms.** These do not change the cap. If either
  breaks the bound at the chosen cap, the result is reported to the owner
  with the cap unchanged.

### 7.4 Shared or separate caps (owner, 2026-10-18)

The owner leans toward one shared pool for text and hybrid, conditional on
the path through the mutexes to SQLite holding up. Rule, fixed before the
run: keep one shared cap unless, in the mixed overload arm, text p95 breaks
the cap bound of § 7.3 (2 × text *N* = 1 p95, or 2 500 ms) while the text-only
overload arm at the same client count and cap meets it. In that case,
propose separate caps (a partitioned limit, § 5.3) to the owner, with the
attribution data if the timing build was approved.

## 8. Changes this design implies

| Artefact | Change |
| --- | --- |
| `dev/interfaces/mcp.md` | `admission_cap` row (line 42): calls wait FIFO while time remains (owner decision 2026-10-18); new `queue_limit` key; busy codes `admission` (with `busy.reason`) and `store_switch`; the `busy` sub-object in § 6 errors (line 249) |
| `dev/design/mcp-adapter/architecture.md` | § Threads: drop the "headroom" reasoning (§ 4); point at this design and EP-18 |
| `dev/design/mcp-adapter/requirements.md` | MR-033.2: choose "wait, then busy with `retry_after_ms`" |
| `fathomdb-mcp` | `registry.rs` acquire and derived `in_flight`, `bridge.rs` abandoned flag, `error.rs` codes and the `busy` object, `edge.rs`/`tools.rs` pass `context.ct`, `config.rs` `queue_limit` |
| Engine | EP-18 (A, then B), after its own review; `embedder_overloaded` in EP-03 |

## 9. Test plan for later (RED first)

Each line is one RED test, committed failing before its fix, using the
existing `test-seams` call blocking (`fathomdb-mcp/src/seams.rs`). The two
existing contract tests (`tests/bridge.rs:45-63`, `:79-100`) are replaced
by the first two, together with the interface change.

1. `an_excess_call_waits_for_a_permit_and_then_runs`: cap 1, first call
   blocked, second call released after 200 ms of waiting succeeds.
2. `an_excess_call_is_refused_when_too_little_deadline_remains`: cap 1,
   never released; the second call returns `busy/admission` reason
   `deadline` at `deadline − need` (± 100 ms), not at once and not at the
   deadline.
3. `a_late_permit_is_not_used`: the permit is released when less than
   `need` remains; the waiter refuses with reason `deadline` and the permit
   is free again.
4. `waiters_are_admitted_in_arrival_order`: cap 1, three waiters with
   staggered arrivals; release one at a time; completion order equals
   arrival order.
5. `a_full_queue_refuses_at_once_with_queue_full`: cap 1, `queue_limit` 1,
   third call is reason `queue_full` in under 50 ms.
6. `an_abandoned_call_keeps_its_permit_and_is_counted`: a blocked call times
   out; the busy object reports `abandoned: 1`; after release, `abandoned: 0`
   and the permit is free.
7. `in_flight_is_derived_from_permits`: no path changes `in_flight` other
   than permit acquire and drop.
8. `queued_returns_to_zero_on_timeout_cancel_and_panic`.
9. `retry_after_ms_is_omitted_before_enough_samples`: fresh store, forced
   busy, no `retry_after_ms`.
10. `retry_after_ms_uses_effective_capacity`: seam delays every call by
    100 ms; cap 2 with one abandoned call and 2 queued; reason `deadline`
    reports 250..=350 ms (`ceil(3/1) × 100`).
11. `queue_full_retry_after_is_one_service_time_over_effective_capacity`.
12. `stuck_is_reported_when_every_permit_is_abandoned`: `stuck: true`, no
    `retry_after_ms`.
13. `a_client_cancel_removes_a_queued_call`: `queued` drops by one; the next
    waiter is admitted on release.
14. `a_store_with_queued_calls_does_not_expire_idle`: `idle_timeout_secs` 1,
    a call queued for 2 s; the store stays ready.
15. `a_drain_during_the_wait_returns_waiters_to_the_open_wait`.
16. `shutdown_wakes_queued_calls_with_not_ready_draining`.
17. `another_store_in_use_is_busy_store_switch`.
18. `sdk_overloaded_maps_to_admission_engine_overloaded_without_retry_after`.
19. `queued_calls_get_progress_notifications` (with a progress token).
20. Engine, EP-18 part A: `a_slow_read_does_not_delay_reads_on_idle_workers`,
    using the engine's `test-hooks` to hold one worker; seven following
    reads complete while it is held.
21. Engine, EP-03: `a_saturated_embed_queue_is_reported_as_embedder_overloaded`.

## 10. Open questions for the owner

Owner decisions, 2026-10-18:

- **Contract change (§ 3): accepted.** Excess calls wait FIFO while time
  remains.
- **Shared cap: one shared cap is the lean,** subject to the mixed-load
  check in § 7.4.
- **Former Q2, refuse or start late: decided.** A call that cannot get a
  slot in time fails with `busy` and `retry_after_ms`. "Run anyway with a
  partial result" is rejected: the engine cannot return part of a search,
  so a late start would only become abandoned work. How the early-shed
  threshold `need` and the deadline are set (fixed or adaptive) is a
  parameter whose value comes from the queue simulation (pending); § 6.1
  keeps the current proposal as the placeholder.
- **Former Q5, where honest embed degradation lives: decided.** It folds
  into EP-03 as the common degradation reason `embedder_overloaded`,
  alongside the existing hybrid degradation reasons. EP-18 no longer has a
  part C.

Pending:

1. **EP-18 part A: one shared queue in the engine, at the cost of one
   mutex per read? Pending simulation.** The separate queue simulation
   settles this, alongside the HOL probe in § 7. The trade-off it weighs:
   - *For.* No read waits behind a slow one while a worker is idle. The
     cost is tens of nanoseconds per read when uncontended, and the hybrid
     path already pays a similar lock in the embed dispatcher. The
     reviewer considered the mutex concern answered by this argument.
   - *Against.* It changes a hot path chosen deliberately to avoid a lock,
     and under heavy contention the lock itself becomes a queue. The
     lock-free rendezvous variant avoids the lock but still queues behind a
     slow worker when every worker is busy.
