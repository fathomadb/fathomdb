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
  moved into the blocking closure so abandoned work keeps counting. Busy
  carries an honest `retry_after_ms` (B2-7).
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
- **Adopt.** The permit-owns-the-slot pattern: admission is a permit, not a
  counter, and the permit's lifetime is the work's lifetime. The split
  between waiting (`ConcurrencyLimit`) and shedding (`LoadShed`) maps onto
  "wait up to the deadline" and "queue full".
- **Reject.** Tower ties the permit to the response future, so dropping the
  future frees the slot. Our work runs on a blocking thread that outlives
  the future. The permit must move into the blocking closure instead, as
  `StoreGuard` already does (`bridge.rs:39-43`). We also do not adopt
  `Service`/`Layer` as a framework. One semaphore does not need it.

### 5.2 HikariCP connection pool

Source: <https://github.com/brettwooldridge/HikariCP> (README,
configuration section).

- **Structure.** A fixed-size pool (`maximumPoolSize`, default 10; the
  README advises leaving `minimumIdle` unset so the pool stays fixed). When
  no connection is idle, "calls to getConnection() will block for up to
  `connectionTimeout` milliseconds before timing out" (default 30 000 ms,
  minimum 250 ms), then an `SQLException` is thrown.
- **Adopt.** This is the closest analogue: a fixed pool of database
  connections, callers that wait with a bounded timeout, and a typed
  failure at the end. The wait bound is the caller's budget, not a
  separate queue timeout. Its pool-sizing argument (too many connections
  hurt throughput) is why the sweep in § 7 measures instead of raising the
  cap.
- **Reject.** HikariCP has no queue-depth limit, because the waiter count
  is bounded by application threads. An MCP server has no such bound
  (H1 is unbounded), so we add one. HikariCP can reclaim a leaked
  connection; we cannot reclaim a reader from an abandoned call (no
  interrupt until EP-09).

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
  timeout (default 1 s), served **last in, first out**.
- **Adopt.** Two ideas. First, latency is the signal of a queue building,
  so the queue records service-time samples (§ 6.1) that both
  `retry_after_ms` and the sweep use. Second, a partitioned limit (Netflix
  "percentage enforcement") is the shape to use if embedded hosts ever need
  a guaranteed share (§ 6.1, fairness).
- **Reject, for now.** An adaptive limit needs steady traffic to sample;
  an MCP store serves bursts from a few agents, and the ceiling is a hard
  pool of eight readers, not an unknown backend. A fixed, measured cap is
  simpler and testable. LIFO suits request-response services that prefer
  fresh requests. An agent waiting on its own tool call is never stale
  before its deadline, and LIFO starves the oldest call, so FIFO fits
  better.

### 5.4 Considered and not chosen

- **SQLite's busy handler.** It sleeps and retries one connection on a lock,
  with no queue, no ordering and no fairness, and SQLite may skip it to
  avoid deadlock ([sqlite3_busy_handler](https://www.sqlite.org/c3ref/busy_handler.html),
  also recorded at `write_commit.rs:476-483`). It is not an admission
  model. It stays as H12.
- **deadpool and bb8.** Same structure as HikariCP (wait with timeout on a
  fixed pool); HikariCP has the clearer primary documentation.
- **The engine's own embed dispatcher** (H7) is a good in-repo model for
  EP-18: one FIFO, N workers pulling, deadline per request, expired entries
  purged on submit.

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
- A dropped `Acquire` future removes itself from the wait list
  (`batch_semaphore.rs:686-698`). So a waiter that times out, or whose
  client cancels, leaves the queue at no cost.
- `OwnedSemaphorePermit` is `'static`, so it can move into the
  `spawn_blocking` closure with the `StoreGuard`.
- It does not report the number of waiters, so the queue keeps its own
  `AtomicUsize queued` counter for depth limits, `retry_after_ms` and idle
  accounting.

Rejected alternatives: `Mutex<VecDeque>` + `Notify` (re-implements the
semaphore, more to test); `async-channel` or `flume` as a ticket queue (new
dependency for the same FIFO); keeping the counter and polling (no
ordering, wasted wakeups).

**Acquire sequence** (replaces the `Phase::Ready` arm at
`registry.rs:342-357`):

1. Wait for `Ready` as today (H2).
2. If `queued >= queue_limit` and no permit is free, refuse at once:
   `busy/queue_full` with `retry_after_ms`.
3. Otherwise increment `queued` and await `acquire_owned()` under
   `timeout_at(deadline)`, in a `select!` with the client's cancel token
   (`context.ct`, H1) and the slot's phase watch.
4. On a permit, decrement `queued`, re-check under the registry mutex that
   the slot is still `Ready` at the same epoch, then increment `in_flight`
   and build the `StoreGuard` holding the permit.
5. At the deadline, refuse: `busy/admission_timeout` with
   `retry_after_ms`. A call that waited the whole deadline has no time left
   to run, so it fails instead of starting late.
6. On client cancel, return the JSON-RPC cancelled result; the dropped
   future has left the queue.
7. If the phase changed to `Draining` or `Closed` while waiting, fall back
   into the existing open-wait loop with the remaining deadline.

**Deadline split.** The call deadline (10 s) covers queue wait and
execution together. The queue does not get its own timeout, as in
HikariCP: the agent's budget is the only budget. A call admitted with
little time left is still admitted; a call admitted with less than the
measured p50 service time left is not worth starting. That early rejection
is an option, not part of this design, until the sweep shows it helps.

**Queue depth limit.** `queue_limit`, a new configuration key. Proposed
default: `2 × admission_cap`. Reason: a call that joins behind *q* waiters
on *c* permits waits about `ceil((q+1)/c)` service times. At 2 × cap, that
is three service times, well inside a 10 s deadline for sub-second reads.
More depth than that converts busy into timeouts, which cost the agent the
full deadline instead of a short wait. The sweep validates this (§ 7).

**Abandoned-work accounting.** Keep today's rule: the permit and guard
live in the blocking closure, so an abandoned call holds its permit until
its thread returns (`bridge.rs:4-7`). Add a visible count: when
`bridge::call` times out, it sets an `abandoned` flag shared with the
guard and increments a per-store `abandoned` counter; the guard's `Drop`
decrements it. This counter appears in busy details and in tracing, so an
operator can tell "the store is busy" from "the store is stuck behind dead
work".

**Fairness across sessions and agents.**

- stdio: one client, one session. If Claude Code subagents share the
  parent's MCP connection (likely, not verified here), the six subagents
  were one session. Either way the server cannot tell agents apart; there
  is no agent identity in the request. FIFO per call
  is the only fairness available, and it is enough: agents with similar
  call rates get similar shares.
- Embedded: several `connection()` sessions share one registry and cap
  (`lib.rs:111-114`). FIFO across all of them is the first step. If a
  single session floods the queue, the next step is a per-session limit on
  queued entries (for example half of `queue_limit`). Weighted or
  partitioned shares (Netflix partitioned limiter) wait for evidence of
  starvation.
- Host SDK reads in embedded mode bypass the adapter entirely. Only the
  host can bound them. The host sets `admission_cap` (MR-033.2), and EP-18
  part B lets it size the reader pool to match.

**`retry_after_ms` (B2-7).** It must be an estimate the server can justify,
never a promise.

- Keep an exponentially weighted mean of service time (permit acquired to
  permit released) per store and mode, from completed, non-abandoned calls.
- On busy, with *q* calls queued and *c* permits:
  `retry_after_ms = clamp(ceil((q + 1) / c) × ewma_service_ms, 50, call_deadline_ms)`.
- When fewer than 8 calls have completed since the store opened, omit the
  field. A made-up number is worse than none.
- When `abandoned > 0` and every permit is held by abandoned calls, omit
  the field and add `stuck: true`. Abandoned work has no known end, so any
  number would be a guess.
- For H3 (another store in use) report the same estimate for the other
  store, under a distinct code `busy/store_switch`, so the two causes are
  separate.

**Busy error shape** (a change to `dev/interfaces/mcp.md` § errors):

| Code | When | Fields |
| --- | --- | --- |
| `queue_full` | Depth limit reached on arrival | `retry_after_ms?`, `in_flight`, `queued`, `abandoned` |
| `admission_timeout` | Waited until the deadline | same |
| `store_switch` | Another store is open with calls in flight (H3) | `retry_after_ms?` |

The hints sidecar (item 2) renders these as plain-language hints. This
document defines only the fields.

**Progress and visibility.** A queued call with a progress token gets a
notification each second, as the open wait does (`registry.rs:391-394`),
with the message `queued: position k of q`. Tracing at stderr logs each
shed, each timeout in the queue and each abandoned call, with store,
`in_flight`, `queued` and `abandoned`.

**Drain, idle close and shutdown.**

- Idle expiry and LRU drain test `in_flight == 0` (`registry.rs:422`,
  `:595`). A queued call holds no guard yet, so they must also require
  `queued == 0`. Otherwise a store could start draining while calls wait
  for it.
- When a drain starts, queued waiters see the phase change and return to
  the open wait (step 7), so a reopen within the deadline still serves
  them.
- Shutdown sets `shutting_down` (`registry.rs:612-613`). Waiters wake on the
  phase change and return `not_ready/draining`, as today. The semaphore is
  per slot and survives reopen; it is not closed, so no rebuild is needed.

### 6.2 Engine: EP-18 (new engine proposal)

Title: work-conserving reader dispatch and a configurable reader pool.
Drafted here; it gets its own file under
`dev/design/mcp-adapter/engine-proposals/` after review.

- **Part A, dispatch.** Replace round-robin per-worker channels with one
  shared bounded FIFO that idle workers pull from, as the embed dispatcher
  does (H7). Each worker keeps its own connection, so connection thread
  affinity (`reader_pool.rs:6-13`) is unchanged. The cost is one mutex
  acquire per request, which the current comment avoids
  (`reader_pool.rs:1207-1209`). That cost must be measured against the
  head-of-line win; the HOL probe in § 7 is the evidence. A lighter
  alternative is to pick the least-loaded worker from per-worker depth
  counters, which keeps per-worker channels but still cannot move a request
  already queued behind a slow one.
- **Part B, pool size.** Add `reader_pool_size: Option<u64>` to
  `EngineConfig` (`runtime_configuration.rs:146-157`), default 8, range to
  be set by the sweep, validated with the other fields. Many places assume
  exactly `READER_POOL_SIZE` (for example `wal_runtime.rs:74`, `:389`,
  `:422`), so this is a wider change than the field. The adapter would then
  validate `admission_cap < reader_pool_size` from the open engine instead
  of the constant 8 (`config.rs:243-246`).
- **Part C, honest degradation.** When query embedding fails with
  `Saturated` or `QueuedExpired`, report it (for example a
  `soft_fallback` with an `embed_overloaded` reason) instead of silently
  running text-only (H7). This may fold into EP-03 (hybrid search
  degradation) instead.
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

- **Store.** The LoCoMo store used by `dev/experiments/mcp-agent-loop/`, loaded
  once and reused read-only.
- **Queries.** A fixed list of LoCoMo conversation-0 questions, each issued as
  `search` with the default limit. Order fixed by seed; the same list for
  every cell.
- **Load.** Closed loop: *N* clients, each issuing its next call when the
  previous one returns, *N* ∈ {1, 2, 4, 7}. `admission_cap = N`, so the
  cap is not the bottleneck being measured. Run with the queue build; the
  current build gives the same numbers for N ≤ cap.
- **Modes.** `text` and `hybrid`.
- **Embedder.** Default embedder on CUDA, `FATHOMDB_EMBED_DEVICE=cuda:0`
  (feature `embed-cuda`, `fathomdb-mcp/Cargo.toml`), RTX 3090 cuda:0 only.
  No CPU arm.
- **Two harnesses.** Through the MCP binary over stdio (what agents see),
  and SDK-direct with the same concurrency (engine-only). The difference is
  adapter overhead.
- **Repetition.** One warm-up pass, then 5 repetitions of 200 calls per
  client per cell. Report the median across repetitions; the range is
  shown.
- **Measures.** Latency p50, p95, p99 per call; throughput (calls per
  second); counts of busy, timeout and soft fallback; GPU utilisation per
  cell.
- **HOL probe.** At *N* = 4 and 7, text mode, inject one slow read every 2 s
  (an `expand` at `max_depth = 3` from a high-degree node, or a hybrid
  search at limit 100) and measure p95 of the other calls against the same
  cell without the probe.

### 7.3 Decision rules (fixed before the run)

- **Cap, per mode.** Choose the largest *N* in {1, 2, 4, 7} for which both
  hold against the next smaller *N*: throughput rises by at least 10 %, and
  p95 latency is at most 2 × the *N* = 1 p95 and at most 2 500 ms (a
  quarter of the call deadline). The default `admission_cap` is the
  smaller of the text and hybrid choices, unless the owner accepts a
  per-mode cap.
- **Queue limit.** Keep `2 × admission_cap` unless, at the chosen cap, more
  than 1 % of calls in any cell end in `admission_timeout`. Then halve it
  and re-run that cell.
- **EP-18 part A.** Proceed if the HOL probe raises p95 of the other calls
  by more than 2 × at either *N*. Otherwise record EP-18 part A as not needed
  at today's call mix.
- **EP-18 part B.** Proceed only if the *N* = 7 text cell meets the cap rule
  and its p95 is still under 2 × the *N* = 1 p95. That would mean the
  ceiling, not the workload, is limiting.
- **Inconclusive.** If repetition ranges overlap between adjacent *N*, the
  smaller *N* wins.

## 8. Changes this design implies

| Artefact | Change |
| --- | --- |
| `dev/interfaces/mcp.md` | `admission_cap` row (line 42): calls wait FIFO up to the deadline; new `queue_limit` key; busy codes `queue_full`, `admission_timeout`, `store_switch` with their fields (line 249) |
| `dev/design/mcp-adapter/architecture.md` | § Threads: drop the "headroom" reasoning (§ 4); point at this design and EP-18 |
| `dev/design/mcp-adapter/requirements.md` | MR-033.2: choose "wait, then busy with `retry_after_ms`" |
| `fathomdb-mcp` | `registry.rs` acquire, `bridge.rs` abandoned flag, `error.rs` codes, `edge.rs`/`tools.rs` pass `context.ct`, `config.rs` `queue_limit` |
| Engine | EP-18, after its own review |

## 9. Test plan for later (RED first)

Each line is one RED test, committed failing before its fix, using the
existing `test-seams` call blocking (`fathomdb-mcp/src/seams.rs`). The two
existing contract tests (`tests/bridge.rs:45-63`, `:79-100`) are replaced
by the first two, together with the interface change.

1. `an_excess_call_waits_for_a_permit_and_then_runs`: cap 1, first call
   blocked, second call released after 200 ms of waiting succeeds.
2. `an_excess_call_waits_until_its_deadline_then_returns_admission_timeout`:
   cap 1, never released; the second call returns `busy/admission_timeout`
   at the deadline (± 100 ms), not at once.
3. `waiters_are_admitted_in_arrival_order`: cap 1, three waiters with
   staggered arrivals; release one at a time; completion order equals
   arrival order.
4. `a_full_queue_refuses_at_once_with_queue_full`: cap 1, `queue_limit` 1,
   third call is `busy/queue_full` in under 50 ms.
5. `an_abandoned_call_keeps_its_permit_and_is_counted`: blocked call times
   out; the busy detail reports `abandoned: 1`; after release, `abandoned: 0`
   and the permit is free.
6. `retry_after_ms_is_omitted_before_enough_samples`: fresh store, forced
   busy, no `retry_after_ms`.
7. `retry_after_ms_tracks_measured_service_time`: seam delays every call by
   100 ms; after 10 calls, a busy with 2 queued on cap 1 reports a value in
   250..=350 ms.
8. `retry_after_ms_is_omitted_when_only_abandoned_calls_hold_permits`:
   reports `stuck: true` instead.
9. `a_client_cancel_removes_a_queued_call`: cancel a waiting call; `queued`
   drops by one; the next waiter is admitted on release.
10. `a_store_with_queued_calls_does_not_expire_idle`: `idle_timeout_secs` 1,
    a call queued for 2 s; the store stays ready.
11. `a_drain_during_the_wait_returns_waiters_to_the_open_wait`: start a
    drain while calls are queued; they are served after the reopen within
    the deadline, or return `not_ready` at it.
12. `shutdown_wakes_queued_calls_with_not_ready_draining`.
13. `another_store_in_use_is_busy_store_switch_not_admission`.
14. `queued_calls_get_progress_with_their_position` (with a progress
    token).
15. Engine, EP-18 part A: `a_slow_read_does_not_delay_reads_on_idle_workers`,
    using the engine's `test-hooks` to hold one worker; seven following
    reads complete while it is held.
16. Engine, EP-18 part C: `a_saturated_embed_queue_is_reported_as_degradation`.

## 10. Open questions for the owner

1. Is the contract change acceptable: `dev/interfaces/mcp.md` moves from
   "refuse at once" to "wait FIFO up to the deadline", with three busy
   codes instead of one?
2. Should a call that waited until its deadline fail (proposed), or start
   anyway and return a partial result labelled per MR-032?
3. Is one `admission_cap` for both modes acceptable, or should text and
   hybrid get separate caps if the sweep says they differ?
4. EP-18 part A trades a mutex per read for no head-of-line blocking. Is
   that trade open, given the round-robin was chosen to avoid the mutex?
5. Should honest embed degradation (EP-18 part C) fold into EP-03?
