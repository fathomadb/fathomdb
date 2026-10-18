---
title: Queues and locks on one MCP tool call, from stdin to SQLite
date: 2026-10-18
status: UNREVIEWED
desc: Code trace of every queue, channel, semaphore, mutex, condvar, thread pool and busy loop that one fathomdb-mcp tool call crosses, with the bundled SQLite build flags, the hazards they create (abandoned work, head-of-line blocking, shared embed queue, writer-mutex convoy, global page-cache mutex, checkpoint starvation, retry waves) and the simplifications that remove each class.
inputs:
  - dev/design/agent-access/queue-admission.md
  - src/rust/crates/fathomdb-mcp/src/{edge,registry,bridge,tools,config,session,main,lib}.rs
  - src/rust/crates/fathomdb-sdk/src/{engine,read,graph}.rs
  - src/rust/crates/fathomdb-engine/src/{reader_pool,search_api,read_api,graph_api,reader_transaction,open,lib,connection_runtime,runtime_configuration,embedding,telemetry,write,write_commit,wal_runtime,runtime_lifecycle}.rs
  - src/rust/crates/fathomdb-engine/src/embed_dispatch/core.rs
  - src/rust/crates/fathomdb-engine/src/{projection_runtime,projection_worker,projection_commit,projection_registry,projection_generation}.rs
  - rmcp 3.5.1, tokio 1.52.3, rusqlite 0.40.1, libsqlite3-sys 0.38.1 (bundled SQLite 3.53.2) from the cargo registry
---

# Queues and locks on one MCP tool call, from stdin to SQLite

Companion to [queue-admission.md](queue-admission.md) § 2. That table lists
the hops where a call can wait or be refused. This document goes one level
down: the concrete primitive at each hop, its capacity and fairness, what it
is held across, and the ways the stack of queues can lock up, starve, convoy
or oscillate. It is a read-only trace. Nothing here changes code.

Scope and provenance:

- FathomDB code at `exp/queue-design` `40a1c192e`. Line numbers refer to
  that commit.
- Dependency code read from the cargo registry at the versions in
  `Cargo.lock`: rmcp 3.5.1, tokio 1.52.3, rusqlite 0.40.1, libsqlite3-sys
  0.38.1 (bundled SQLite 3.53.2), candle-core-fathomdb 0.10.2,
  rayon-core 1.13.0. `parking_lot` 0.12.5 is in the lock file but no
  FathomDB crate on this path uses it. `crossbeam-channel` is absent.
- Toolchain `rustc 1.95.0`. `rust-src` is not installed, so statements about
  std internals come from the documented implementation, not from source:
  `std::sync::Mutex` is futex-based on Linux since Rust 1.62 and gives no
  fairness guarantee (unlock does not hand off, a running thread can
  barge); `std::sync::mpsc` uses the crossbeam-derived implementation since
  Rust 1.67, and `sync_channel(n)` blocks the sender when full.
- The trace was done by a read-only agent and spot-checked: the round-robin
  dispatch (`reader_pool.rs:1224-1225`), the silent embed fallback and the
  mean-vector read under the writer mutex (`search_api.rs:543-556`), the
  projection retry wait (`projection_worker.rs:579-593`) and the SQLite
  page-cache mode (`sqlite3.c:58370-58376`) were re-read by hand. Rows marked
  "not verified" were not confirmed.

## 1. The path

The nominal path `fathomdb-mcp → rmcp → fathomdb-sdk → fathomdb-engine →
rusqlite → libsqlite3-sys → SQLite` is right in order. It misses the tokio
I/O and blocking-pool hops, the embed dispatcher's rayon pool, and the
background actors that share the same locks.

```text
agent stdin line
 ├─ tokio Stdin = Blocking<std::io::Stdin>: each read is a spawn_blocking job      (tokio io/stdin.rs:30, io/blocking.rs:76)
 ├─ rmcp serve loop: one select! over receive / sink / peer / JoinSets              (rmcp service.rs:1400-1470)
 ├─ rmcp: tokio::spawn per request, unbounded                                       (service.rs:1603)
 ├─ fathomdb-mcp edge: tokio::spawn again, for panic isolation                      (edge.rs:254-257)
 ├─ Registry::acquire: std Mutex<Inner> + AtomicUsize in_flight; busy at once at cap (registry.rs:179, :71, :343-346)
 ├─ bridge::call: spawn_blocking raced with timeout_at(deadline)                    (bridge.rs:39-44)
 │    → tokio blocking pool: 512 threads, unbounded VecDeque, std Mutex + Condvar   (tokio runtime/builder.rs:300, blocking/pool.rs:406-412)
 ├─ fathomdb-sdk: pass-through (argument guards, then self.core.*)                  (sdk engine.rs:185-222, read.rs:30-37, graph.rs:14-16)
 ├─ fathomdb-engine, on the blocking thread:
 │    [hybrid] EmbedDispatcher::submit: std Mutex<QueueState> + VecDeque (cap 20) + Condvar;
 │             reply via Mutex + Condvar wait_timeout(30 s)                          (embed_dispatch/core.rs:309-310, :356-358, :98-99, :226-252)
 │             → one of 5 fathomdb-embed-N threads → candle forward → rayon global pool (core.rs:380-382, :604-610; candle cpu_backend/mod.rs:1401)
 │    [hybrid, bge identity] writer connection std Mutex for read_pinned_mean_vec   (search_api.rs:551-556, lib.rs:470)
 │    [orient] writer Mutex taken three times; no reader pool                       (projection_registry.rs:1595, projection_generation.rs:1793, :1839)
 │    ReaderWorkerPool::dispatch: fetch_add % 8 → sync_channel(4) blocking send     (reader_pool.rs:1221-1225, :955, :499)
 │    caller blocks on sync_channel(1) recv(), no timeout                           (search_api.rs:589, :624; :1041, :1052; read_api.rs:96-101; graph_api.rs:551-563)
 │    fathomdb-reader-N: rx.recv() FIFO; BEGIN DEFERRED on its own connection        (reader_pool.rs:528, reader_transaction.rs:13)
 ├─ rusqlite: open flags READ_WRITE|CREATE|URI|NO_MUTEX; busy_timeout 5000 ms       (rusqlite lib.rs:1256-1264, inner_connection.rs:118)
 ├─ SQLite 3.53.2, THREADSAFE=1, SQLITE_ENABLE_MEMORY_MANAGEMENT                     (libsqlite3-sys build.rs:139, :135)
 │    no per-connection mutex (NOMUTEX); mem0 mutex off (engine sets MEMSTATUS=0)    (sqlite3.c:190546-190547; runtime_configuration.rs:103-109)
 │    ONE global page-cache group mutex on every page fetch, all connections         (sqlite3.c:58370-58381)
 │    WAL read marks: 5 slots shared by all connections; walTryBeginRead retry loop  (sqlite3.c:67741, :70399, :70487-70493)
 │    busy handler sleep-poll {1,2,5,10,15,20,25,25,25,50,50,100} ms up to 5 s       (sqlite3.c:188923-188926)
 │    auto-checkpoint PASSIVE at 1000 pages, run by the committing connection        (sqlite3.c:14857)
 └─ reply: respond.send (dropped if the receiver is gone) → bridge returns → tool task
      → rmcp sink mpsc(64) → serve loop → JoinSet task per response
      → tokio::sync::Mutex<FramedWrite<Stdout>> (fair FIFO) → Blocking<Stdout>: spawn_blocking per write
                                                                                     (reader_pool.rs:574-576; rmcp service.rs:1360-1362, :1510-1520; async_rw.rs:54, :111-123; tokio io/stdout.rs:63)
```

Background actors that contend for the same resources:

- Projection: one dispatcher thread and 2 workers
  (`scheduler_runtime_threads` default 2, `runtime_configuration.rs:201`),
  each with its own SQLite connection (`projection_worker.rs:356`,
  `connection_runtime.rs:190-207`). They use a `Mutex<VecDeque<ProjectionJob>>`
  with a Condvar, a `commit_gate: Mutex<()>` that serializes their
  `BEGIN IMMEDIATE` commits (`projection_runtime.rs:163-166`, `:187`;
  `projection_commit.rs:134`, `:149`), and **the same embed dispatcher as
  queries**, with batches of up to 64 texts (`projection_runtime.rs:126`,
  `projection_worker.rs:420-430`, `:617`).
- Embedded host mode: SDK writers hold the writer mutex for a whole
  `BEGIN IMMEDIATE … COMMIT`, including any auto-checkpoint
  (`write.rs:206-212`, `write_commit.rs:508`). Host SDK reads use the same
  eight readers without passing admission.
- The registry idle clock ticks every second on the tokio runtime
  (`lib.rs:73-81`).

One engine holds 12 SQLite connections: 1 writer, 8 readers, 1 projection
dispatcher, 2 projection workers (`wal_runtime.rs:641`).

### Corrections to queue-admission.md § 2

- **H13 is wrong.** The bundled build defines
  `SQLITE_ENABLE_MEMORY_MANAGEMENT` (libsqlite3-sys `build.rs:135`), which
  forces `pcache1.separateCache = 0` (`sqlite3.c:58370-58371`). All page
  caches share one group (`pcache1.grp`) with one global mutex
  (`SQLITE_MUTEX_STATIC_LRU`, `sqlite3.c:58380`), taken on every page fetch
  and unpin by every connection. Eviction is from the group LRU, so the
  claim that one heavy reader cannot take other readers' pages needs
  re-checking under this build. The page-cache ADR lives on
  `release/0.8.28` and was not re-read here (not verified).
- **H5**: the 512-thread blocking pool is also used for every stdin read
  and stdout write, not only for bridge calls.
- **H7**: the embed provider call runs on candle's rayon global pool, shared
  by all five embed workers and by projection batches.

## 2. Primitives

"std Mutex" means `std::sync::Mutex`: futex-based, unfair, no timeout.
"Across" says what the holder does while holding it.

| # | Where | Type | Capacity | Order | When full | Timeout / cancel | Held by, across |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P1 | tokio `io/stdin.rs:30`, `io/blocking.rs:76` | `Blocking<Stdin>`, spawn_blocking per read | 1 read in flight | — | — | none | one blocking-pool thread per read |
| P2 | rmcp `service.rs:1400-1470` | one serve loop, unbiased `select!` | 1 | random among ready arms | — | serve-loop token | nothing long; `receive` is cancellation-safe (`async_rw.rs:130-137`) |
| P3 | rmcp `service.rs:1603` | `tokio::spawn` per request | unbounded | arrival | never | `notifications/cancelled` cancels a child token (`service.rs` ~1647-1652); the adapter never reads it | until the handler returns |
| P4 | rmcp `service.rs:813`, `:818` | tokio mpsc `peer_rx` | 1024 | FIFO | await | none | outbound progress notifications |
| P5 | rmcp `service.rs:1360-1362`, `:1617` | tokio mpsc `sink_proxy` | **64** | FIFO | handler task awaits | none | responses wait here when the serve loop is slow |
| P6 | rmcp `service.rs:1510-1520` | `JoinSet` of response sends | unbounded | — | never | none | one task per response until written |
| P7 | rmcp `transport/async_rw.rs:54`, `:111-123` | `tokio::sync::Mutex<Option<FramedWrite<Stdout>>>` | 1 | **fair FIFO** | await | none | held across the stdout write, which is itself a spawn_blocking job |
| P8 | tokio `io/stdout.rs:63`, `io/blocking.rs:124` | `Blocking<Stdout>` | 1 write in flight | — | — | none | one blocking-pool thread per write |
| P9 | `edge.rs:254-257` | second `tokio::spawn` per tool call | unbounded | — | never | none; a panic becomes a JSON-RPC error | until the outcome |
| P10 | `edge.rs:100-110` | `tokio::spawn` per progress notification | unbounded | — | never | none | fire and forget |
| P11 | `registry.rs:179`, `:198-199`, `:335-385` | std Mutex<Inner> (registry) | 1 | unfair | block | none | microseconds; never across an await (scope ends at `:385`, before the await at `:396`) |
| P12 | `registry.rs:71`, `:343-346`, `:158-166` | `AtomicUsize in_flight` admission gate | `admission_cap` 4, allowed 1..=7 (`config.rs:109`, `:243-246`) | none | **refuse at once**, `busy/admission` | none | held by `StoreGuard` for the whole engine call, **including after the adapter abandons it** (`bridge.rs:40-42`) |
| P13 | `registry.rs:72`, `:163-165`, `:640-650` | `tokio::sync::Notify` idle | — | — | — | — | drain waits for `in_flight == 0` |
| P14 | `registry.rs:75`, `:91`, `:373`, `:396` | `tokio::sync::watch<u64>` slot phase | unbounded waiters | all wake | — | call deadline, 1 s steps | store-open wait only |
| P15 | `registry.rs:114-130` | std Mutex<Instant> last-used | 1 | unfair | block | none | nanoseconds, on every admit and drop |
| P16 | `session.rs:84-85` | std Mutex for default store and tables | 1 | unfair | block | none | per session, short, before the engine call |
| P17 | `bridge.rs:39-44`; tokio `builder.rs:300`, `blocking/pool.rs:78-81`, `:106`, `:406-412` | blocking pool: std Mutex + Condvar, unbounded `VecDeque<Task>`, keep-alive 10 s | **512** threads | FIFO | queue grows, no error | `timeout_at` on the JoinHandle only; the closure keeps running | one thread per engine call, through the embed wait (up to 30 s) and the reader wait (unbounded) |
| P18 | sdk `engine.rs:185-222` | none | — | — | — | — | pass-through |
| P19 | engine `lib.rs:467` | `AtomicBool closed` | — | — | — | — | — |
| P20 | `embed_dispatch/core.rs:296-310`, `:422`, `:441-446` | std Mutex<QueueState{waiting VecDeque, active Vec}> + Condvar | waiting = pool size × 4 = **20** (`runtime_configuration.rs:205`, `:233-235`) | **FIFO**, work-conserving (`core.rs:641`) | `Saturated` at once | 30 s per request (`embed_dispatch.rs:14`, `core.rs:420-421`); expired entries dropped lazily (`core.rs:441`, `:189-195`) | microseconds, but an O(queue) `retain` under the lock on every submit |
| P21 | `core.rs:98-99`, `:226-252` | reply: std Mutex<ReplyValue> + Condvar `wait_timeout` | 1 | — | — | 30 s; `Drop` cancels (`:286-290`), but the engine never drops early | **the bridge thread waits here holding P12 and P17** |
| P22 | `core.rs:305`, `:592-599` | std Mutex<Option<Arc<dyn Embedder>>> | 1 | unfair | block | none | only to clone the Arc; the model call (`:604-610`) runs outside every dispatcher lock |
| P23 | `core.rs:380-382` | 5 OS threads `fathomdb-embed-N` (1..=64) | 5 | FIFO from P20 | — | provider call not interruptible | one `embed` (1 text) or `embed_batch` (≤ 64 texts from projection) |
| P24 | candle `cpu_backend/mod.rs:1401`, `utils.rs:4-12`; `candle_bge.rs:431` | rayon global pool (num_cpus threads, unbounded injector) | num_cpus | work-stealing | — | none | shared by all embed workers and projection batches; CUDA builds add `Arc<Mutex<CudaRng>>` and `RwLock<ModuleStore>` (candle `cuda_backend/device.rs:37-41`), not on the embed path (not verified further) |
| P25 | `lib.rs:470`; `search_api.rs:551-556`; `projection_registry.rs:1595`; `projection_generation.rs:1793`, `:1839` | **writer connection std Mutex<Option<rusqlite::Connection>>** | 1 | unfair | block, no timeout | none | hybrid: one `query_row` (mean vector); orient: three acquisitions with several SELECTs each; host writers: a whole write transaction plus any auto-checkpoint |
| P26 | `reader_pool.rs:1221-1225`, `:17` | `AtomicUsize next`, round-robin | — | **ignores load** | — | — | — |
| P27 | `reader_pool.rs:955`, `:499`, `:1225` | `std::sync::mpsc::sync_channel::<ReaderRequest>(4)` per worker, 8 workers | 4 per worker | FIFO per worker | **blocking `send`** on the bridge thread; `try_send` only for shutdown (`:1238`) | none | the request waits until that worker finishes earlier ones |
| P28 | `reader_pool.rs:528`, `:962-964` | 8 OS threads `fathomdb-reader-N`, one connection each | 1 running each | FIFO | — | shutdown checked only between requests (`:529-531`) | the whole read; no `sqlite3_interrupt` anywhere on the read path |
| P29 | `search_api.rs:589`, `:624`, `:1041`, `:1052`; `read_api.rs:96`, `:100`; `graph_api.rs:551`, `:563` | `sync_channel(1)` reply, `recv()` | 1 | — | — | **no timeout**; `Err` only if the worker drops the sender (→ `EngineError::Storage`) | the bridge thread blocks holding P12 and P17 |
| P30 | `reader_transaction.rs:13` | `BEGIN DEFERRED` on a `query_only` reader | — | — | — | — | one read transaction per request; pins a WAL read mark |
| P31 | rusqlite `inner_connection.rs:118` | SQLite busy handler, 5000 ms, every connection; no FathomDB override on the open path | — | sleep-poll, unfair (`sqlite3.c:188923-188926`) | — | 5 s, then `SQLITE_BUSY` | readers rarely hit it in WAL; writer and projection `BEGIN IMMEDIATE` do |
| P32 | `sqlite3.c:67741`, `:70399`, `:70487-70493` | WAL read-mark locks: **5 slots** shared by 12 connections; `walTryBeginRead` retry with growing sleeps, 100 retries → `SQLITE_PROTOCOL` | 5 | spin / sleep | retry | ~10 s worst | each read transaction pins one mark for its duration, blocking checkpoint past that frame |
| P33 | `sqlite3.c:58370-58381`, `pcache1Fetch` → `pcache1FetchWithMutex` | **global page-cache group mutex** `SQLITE_MUTEX_STATIC_LRU` (pthread) | 1 | unfair | block | none | every page fetch and unpin, every connection |
| P34 | `runtime_configuration.rs:103-109`; `sqlite3.c:31767`, `:31862` | `mem0` allocator mutex | — | — | — | — | skipped while MEMSTATUS is 0 (engine default `Performance`); on only in `Diagnostics` |
| P35 | `sqlite3.c:190546-190547` | per-connection `db->mutex` | — | — | — | — | absent (NOMUTEX) |
| P36 | `sqlite3.c:14857`; no `wal_autocheckpoint` pragma in `open.rs` | auto-checkpoint, PASSIVE, at 1000 WAL pages | — | — | — | never waits for readers | inside a writer commit under P25, or a projection commit under `commit_gate` |
| P37 | `lib.rs:526`, `:532`; `telemetry.rs:176-181`, `:192` | std Mutex<Option<TelemetrySink>> gated by `AtomicBool` | 1 | unfair | block | none | when `telemetry_dir` is set (enabled per open, `registry.rs:702-713`): a JSONL file append on the bridge thread after every search; hybrid forces `explain` on (`tools.rs:643`) |
| P38 | `projection_runtime.rs:163-166`, `:444`; `projection_worker.rs:420-464` | projection state and job queue: std Mutex + Condvar each | 128 jobs (threads × 64) | FIFO | dispatcher stops fetching | none | background |
| P39 | `projection_runtime.rs:187`; `projection_commit.rs:134`, `:149` | `commit_gate: std Mutex<()>`, then `BEGIN IMMEDIATE` on the worker's connection | 1 | unfair | block | SQLite busy 5 s | the whole projection commit |
| P40 | `projection_worker.rs:579-593`, `:617-627` | projection embed retry: on `Saturated` or `QueuedExpired`, wait 25 ms (Condvar `wait_timeout`) and resubmit the same batch | — | poll | — | until stopping | **a 25 ms retry wave against P20 while queries saturate it** |
| P41 | `runtime_configuration.rs:63` | static std Mutex<RuntimeState> | 1 | — | — | — | open time only |
| P42 | `main.rs:115`; `registry.rs:186` | `shutdown_timeout(5 s)`; `SHUTDOWN_MARGIN` 5 s | — | — | — | — | assumes abandoned work ends within deadline + 5 s |

### SQLite build and connection settings

- libsqlite3-sys `build.rs:125-143`: `SQLITE_THREADSAFE=1`,
  `SQLITE_ENABLE_MEMORY_MANAGEMENT`, FTS3, FTS5, JSON1, RTREE, STAT4,
  DBSTAT, COLUMN_METADATA, API_ARMOR, LOAD_EXTENSION, `SQLITE_USE_URI`,
  `HAVE_USLEEP`.
- Left at SQLite defaults: `SQLITE_DEFAULT_MEMSTATUS=1` (turned off at run
  time by the engine), `SQLITE_DEFAULT_WAL_AUTOCHECKPOINT=1000`,
  `SQLITE_DEFAULT_CACHE_SIZE=-2000` (KiB), `SQLITE_DEFAULT_MMAP_SIZE=0`.
- Writer pragmas: `secure_delete=ON`, `journal_mode=WAL`,
  `synchronous=NORMAL` (`open.rs:1104-1112`). Readers: `journal_mode=WAL`,
  `secure_delete=ON`, `query_only=ON`, lookaside via `sqlite3_db_config`,
  a 10-statement prepared cache (`open.rs:1323-1361`,
  `reader_pool.rs:511`). Runtime connections: WAL, `secure_delete`
  (`connection_runtime.rs:201-207`).
- No `cache_size`, `mmap_size`, `busy_timeout`, `wal_autocheckpoint` or
  shared-cache setting on the open path, apart from the
  `FATHOMDB_PERF_*_PRAGMAS` experiment hooks
  (`connection_runtime.rs:313-385`).
- Checkpoints: only SQLite's auto-checkpoint in steady state. An explicit
  `wal_checkpoint(TRUNCATE)` exists only in operator and test seams
  (`wal_runtime.rs:763-800`).

## 3. Hazards

Severity is for the standalone MCP binary unless stated otherwise.

| ID | Hazard | Mechanism and evidence | Trigger | Severity |
| --- | --- | --- | --- | --- |
| A | **Abandoned work keeps every resource** | `timeout_at` abandons only the JoinHandle (`bridge.rs:44`). The closure owns the `StoreGuard` (`bridge.rs:40`), so the admission slot (P12) and the blocking thread (P17) stay taken. Nothing below checks a deadline: the embed wait runs to its own 30 s (`core.rs:245-250`), a queued reader request still runs in full (P27, P28), and the reply is dropped (`reader_pool.rs:574-576`) | Any read slower than `call_deadline_ms` (10 s), or an embed stall | High. With cap 4, four slow calls make the store refuse or queue everything for up to 30 s plus the read time, while readers may be idle. Also breaks the shutdown assumption (P42): a hybrid call can hold its slot 30 s, so shutdown reports busy stores and then kills threads mid-read |
| B | **Round-robin head-of-line blocking** | `idx = next.fetch_add(1) % 8; senders[idx].send()` with no load check (`reader_pool.rs:1224-1225`); `recv()` has no timeout (P29); the blocked bridge thread holds P12 and P17 | A slow read on one worker while others are idle | High under mixed load. With cap ≤ 7 the adapter alone cannot fill a channel, but host SDK reads bypass P12 and can, which makes `send` itself block |
| C | **Query and projection share one embed queue** | One FIFO, no priority (`core.rs:641`). Projection submits batches of up to 64 texts (`projection_worker.rs:420-430`, `:617`), each holding one of five embed threads for a long forward, and all forwards share one rayon pool (P24). At 20 waiting, queries get `Saturated`, and `search_api.rs:543-548` turns every dispatch error except a panic into "no vector": **silent text-only search**. `EngineError::Overloaded` comes only from `embed_text` (`embedding.rs:162-168`), which the adapter does not call. Projection answers `Saturated` with a 25 ms poll (P40) | Projection backlog (fresh store, bulk ingest) during query traffic | Medium |
| D | **Writer-mutex convoy and priority inversion** | P25 is an unfair std Mutex shared by the hybrid mean-vector read (after embedding, inside the admission window), orient (three acquisitions), and in embedded mode every SDK write transaction including its auto-checkpoint (P36). A hybrid search can wait, with no timeout, for a write and checkpoint it has nothing to do with, while holding P12 and P17 | Embedded host writing while agents search | Low standalone (nothing writes on that connection; only orient competes), medium embedded |
| E | **Global page-cache mutex** | `SQLITE_ENABLE_MEMORY_MANAGEMENT` (`build.rs:135`) → `separateCache = 0` (`sqlite3.c:58370-58371`) → one group mutex (`:58380`) on every page fetch for all 12 connections; eviction from one group LRU | 8 readers and projection commits touching pages concurrently | Medium: a pthread mutex per page touch, convoys on many cores; contradicts H13 |
| F | **Shared tokio blocking pool** | Bridge calls (including abandoned ones), store open, drain and close, and **every stdin read and stdout write** share one 512-thread pool with an unbounded queue (`blocking/pool.rs:406`) | ~500 concurrent blocking jobs | Low standalone; in an embedded host that also uses `spawn_blocking` heavily, stdout writes, and so every response, queue behind engine calls |
| G | **No inbound backpressure in rmcp** | P3 and P9 are unbounded; the only inbound limit is the immediate refusal at P12. Outbound is bounded (P5 = 64, P7 fair). Each progress notification spawns a task (P10) | Client flood | Low: memory, not latency. Each refused call still costs two task spawns and a registry lock |
| H | **Checkpoint starvation feeding read latency** | Only PASSIVE auto-checkpoints (P36), which cannot pass the oldest reader's frame. Readers that keep opening DEFERRED transactions keep marks pinned (P32), so the WAL grows while writes continue; a longer WAL means a larger wal-index and slower frame lookup on every read. Five read marks across 12 connections mean `WAL_RETRY` sleeps when snapshots churn (`sqlite3.c:70487-70493`). Nothing truncates the WAL in steady state | Write-heavy embedded mode with constant reads | Low standalone, medium embedded (partly confirmed) |
| I | **Write-lock sleep-poll between writer and projection** | Both use `BEGIN IMMEDIATE` (`write_commit.rs:508`, `projection_commit.rs:149`) and rely on the 5 s busy handler (P31), an unfair sleep-poll with sleeps up to 100 ms. `commit_gate` (P39) serializes projection workers but not against the writer | Concurrent host writes and projection | Low for MCP reads; either writer can starve up to 5 s, then `SQLITE_BUSY` → `Storage` |
| J | **Retry and burst waves** | Immediate refusal (P12), agents that re-issue on their next turn, and abandoned work that frees capacity in bursts (when a slow read finishes, several permits free together) give alternating all-busy and all-admit phases. The engine runs expired work (A), so capacity is lowest exactly when demand is highest. Projection's 25 ms poll (P40) is a second, internal wave generator against the embed queue | Agent fan-out bursts above the cap; any slow read under A | Confirmed mechanism. The 2026-10-18 incident was the burst half of this: each Claude subagent's 10-call fan-out turn got 4 ok and 6 busy |
| K | **Lock-order inversion** | Searched; none found on the read path. Hybrid order: P11 (released) → P20/P21 (released) → P25 (released, `search_api.rs:551-556`) → P27/P29. Projection commit nests P39 → SQLite write lock → `mean_accumulator` (`projection_commit.rs:184-185`) → `pending_events`, never P25. `close` nests `close_lock` → P25 → reader join (`runtime_lifecycle.rs:18-29`); readers never take P25, so the join cannot deadlock. The registry mutex is never held across an await | — | None found. Operator paths (`erasure.rs`, `mean.rs:429`, `consolidation.rs`) were not checked exhaustively (not verified) |
| L | **Telemetry mutex on the search tail** | With `telemetry_dir` set, every search takes P37 and appends a JSONL line on the bridge thread (`telemetry.rs:179`, `:192`); hybrid always runs with `explain` on (`tools.rs:643`) | `telemetry_dir` configured | Low: short, but a file write under an unfair mutex on the critical path |
| M | **Unbounded queues** | P3, P6, P9, P10 (tokio tasks), the P17 queue, P14 waiters, the rayon injector. All are bounded in practice only by P12 | — | Inventory |

## 4. Simplifications that remove a class of hazard

| # | Change | Removes | Cost |
| --- | --- | --- | --- |
| 1 | Deadline-aware, work-conserving reader queue (EP-18 part A): one shared `Mutex<VecDeque<ReaderRequest>>` + Condvar, the pattern the embed dispatcher already uses (`core.rs:634-649`), with the caller's deadline on each request and expired requests dropped at dequeue | B, and most of A's amplification | One mutex acquisition per dispatch, the hot-path cost the round-robin comment avoids (`reader_pool.rs:496-498`); microseconds against millisecond reads |
| 2 | Reply timeout plus `sqlite3_interrupt` (EP-09): `recv_timeout(remaining)` and an interrupt handle per reader connection, fired at the deadline | The rest of A: threads and permits are freed at the deadline | `SQLITE_INTERRUPT` must map to `timeout`, not `Storage`; a request inside `walTryBeginRead` is not interruptible |
| 3 | Take the mean-vector and orient reads off the writer mutex: read `mean_vec` inside the reader's DEFERRED transaction, and send orient's three reads to the reader pool as one request | D on every MCP path; also fixes orient reading three different snapshots | `read_pinned_mean_vec` moves into `search.rs`; orient becomes one reader round trip |
| 4 | Two embed lanes, or strict query priority: a second dispatcher over the same `Arc<dyn Embedder>` for projection (`core.rs:305`), or a two-deque priority inside P20 with projection capped at `pool_size − 1` active; and report `Saturated` / `QueuedExpired` as a typed degradation instead of silent text-only | C, and most of P40's poll waves | Projection throughput under query load; a new wire field |
| 5 | Build without `SQLITE_ENABLE_MEMORY_MANAGEMENT`: `LIBSQLITE3_FLAGS` entries starting with `-` are passed verbatim (libsqlite3-sys `build.rs:293-299`), so `-USQLITE_ENABLE_MEMORY_MANAGEMENT` after the defaults should restore per-connection caches (`sqlite3.c:58372-58374`) | E (P33 off the hot path) | Loses `sqlite3_release_memory`; a build-environment dependency. Whether the `-U` lands after the `-D` in the cc command line is not verified; needs a measurement |
| 6 | One admission point with FIFO wait and an honest `retry_after_ms` (queue-admission.md § 6.1, fair `tokio::sync::Semaphore`) | The immediate-refusal half of J, which is the whole of the 2026-10-18 incident | Latency replaces `busy`; on its own it hides B as waiting, so it needs 1 or 2 to stop head-of-line blocking turning into waits |
| 7 | Align deadlines: `embedder_call_timeout_ms` ≤ `call_deadline_ms` for adapter-opened stores | An abandoned hybrid call outliving the shutdown margin (P42) by 20 s | None functionally; projection batches inherit the shorter timeout unless the dispatcher gets per-request timeouts (it already stores a deadline per request, `core.rs:97`) |
| 8 | A periodic `wal_checkpoint(PASSIVE)` or `TRUNCATE` with busy timeout 0 at a quiet moment (for example the registry idle tick when `in_flight == 0`), as `wal_runtime.rs:763-790` already does for operators | H's unbounded WAL growth | One more writer-mutex user |

## 5. What this means for the admission queue

- The adapter queue (6) fixes the incident, which was pure fan-out above
  the cap. It does not fix A or B; under a slow read it converts refusals
  into waits that hold permits.
- A and B share one root: nothing below the adapter knows the call's
  deadline. Changes 1 and 2 put the deadline into the engine, and together
  they remove the hazards that make a deeper adapter queue risky.
- C and D are independent of admission. They decide whether a hybrid
  search silently degrades or waits on a writer, whatever the cap is.
- E is a build-flag question that needs a benchmark before any change.

## 6. Measured: how long a burst takes at the stdio boundary

A live probe (2026-10-18) wrote k `search` calls to the staged
`fathomdb-mcp` binary in one stdin write and timed each response line on
stdout. Host: RTX 3090 #0 (`CUDA_VISIBLE_DEVICES=0`), store `conv0`
(2.9 MB), `admission_cap` 4 or 7, machine otherwise idle. Timings are from
the write to each response, so they include rmcp, the adapter, the engine
and stdout serialization, and no LLM client. Median over repetitions:

| Burst | Cap | Busy responses back | Last ok response |
| --- | --- | --- | --- |
| 10 text, warm | 4 | 0.7 ms | 3.5 ms |
| 10 hybrid, warm | 4 | 0.7 ms | 45 ms |
| 10 hybrid, warm | 7 | 0.5 ms | 68 ms |
| 10 hybrid, first traffic on a fresh process | 4 | 355 ms | 490 ms |

| Concurrent calls (cap 7, no refusals) | 1 | 2 | 4 | 7 |
| --- | --- | --- | --- | --- |
| text, last response | 0.95 ms | 1.2 ms | 1.9 ms | 5.5 ms |
| hybrid, last response | 6.2 ms | 19 ms | 44 ms | 67 ms |

- **Query embeddings serialize on the GPU.** A lone hybrid call takes
  6.2 ms; concurrent ones add about 9.5 ms each, so concurrent forwards are
  slower than a solo one (5.2 ms) and the five embed workers (P23) give no
  parallelism on one GPU. Effective query-embedding throughput is about
  100 per second.
- **Text has a knee at five concurrent calls** (1.9 ms at 4, 4.1 ms at 5).
  The cause is not identified. Candidates from § 2: the five WAL read-mark
  slots (P32), the global page-cache mutex (P33), response serialization
  (P5–P8). Its size is milliseconds.
- **The incident's ~330 ms was client-side.** In the six subagent
  transcripts, busy results reached the agents 294–338 ms after the turn's
  last tool call was emitted, and ok results 536–709 ms after. The server
  returns busy in under 1 ms. Three agents' 10-call bursts reached the one
  shared MCP session 70 ms and 89 ms apart, and each got exactly 4 ok and
  6 busy, so the server finished each burst's four admitted calls before
  the next burst arrived. Server-side burst time was therefore under about
  70 ms, consistent with the 45 ms measured here; the rest is Claude
  Code's own dispatch and result handling.
