# SQLite memory-accounting contention and AC-020: research report

- **Date:** 2026-09-10 (revision 2, addressing the critic review of the first draft; no verified content dropped).
- **Repo state grounded against:** `fathomdb` HEAD `8b4bc1c6` (read-only); rusqlite 0.40.1 / libsqlite3-sys 0.38.1 (bundled SQLite **3.53.2**, `-DSQLITE_THREADSAFE=1`, no `SQLITE_ENABLE_SETLK_TIMEOUT`, no `SQLITE_DEFAULT_MEMSTATUS`, no `LIBSQLITE3_FLAGS` in repo); sqlite-vec 0.1.9 (compiled with `SQLITE_CORE`, registered via `sqlite3_auto_extension`). Amalgamation line numbers below are from `libsqlite3-sys-0.38.1/sqlite3/sqlite3.c`.
- **Labels:** **[Documented fact]** (primary docs/source), **[Repo-verified]** (read in this repo, its git history, or the vendored crate sources), **[Measured result]** (a reported number with provenance), **[Hypothesis]**, **[Recommendation]**.

## 0. Grounding corrections and the repo's own AC-020 record

1. **AC-020 is `AGENT_LONG`-gated** (`tests/perf_gates.rs:1164`, `long_run_enabled()` `:95`) — vacuously green otherwise. Nothing here changes the test.
2. **"One shared Engine" ≠ one shared connection.** Eight reader *worker threads* each own one read-only connection (`READER_POOL_SIZE = 8`, `lib.rs:664`). `Engine::search` dispatches round-robin (`next.fetch_add % n`, `:2268-2269`) into a per-worker bounded channel of capacity 4 (`READER_WORKER_CHANNEL_CAPACITY`, `:2038`) and waits on a per-request `sync_channel(1)`. The sequential arm also goes through the pool; only the concurrent arm exposes cross-worker contention.
3. **Connections open with `SQLITE_OPEN_NO_MUTEX`** (rusqlite `OpenFlags::default()`, `rusqlite-0.40.1/src/lib.rs:1258-1263`), so no per-connection mutex is on the hot path — consistent with `d448263` ("runtime CONFIG_MULTITHREAD applied-but-didn't-help", A.1 vs B.1: 1.58× vs 1.526×, N=5).
4. **MEMSTATUS-off is not recorded as "rejected"; it is an opt-in experiment with a measured history.** Hook: `init_perf_experiments_runtime()` (`lib.rs:18315-18348`, `FATHOMDB_PERF_EXPERIMENTS=1` + `FATHOMDB_PERF_SQLITE_MEMSTATUS_OFF=1`). The 0.7.0 campaign ran this exact test on the same 2×2-document fixture (`seed_ac020_fixture` is byte-identical between `57354a6c` and `2aaf4f12`; HEAD adds 0.8.x lifecycle fields to the write calls but still seeds four documents and drains) **[Repo-verified, git]**:

   | Commit (date) | Configuration | AC-020 speedup | N |
   |---|---|---|---|
   | `57354a6c` (2026-05-25, dev-box x86-64 24-core) | baseline | 2.689× | 1 |
   | | seven PRAGMA-tier configs | 2.5–2.98× | 1 each |
   | | `SQLITE_CONFIG_MEMSTATUS=0` alone | 4.033× | 1 |
   | | MEMSTATUS + reader PRAGMAs | 4.393× | 1 |
   | `df3a1204` (2026-05-25) | + `page_size=8K` | 4.808× | 1 |
   | | + `SQLITE_CONFIG_PAGECACHE` 256 MB | 5.042× | 1 |
   | | + `SQLITE_CONFIG_PAGECACHE` 1 GB | median 4.577× [5.391, 4.556, 4.917, 4.464, 4.577] — INCONCLUSIVE | 5 |
   | `2aaf4f12` (2026-05-26) | `SQLITE_CONFIG_PCACHE2` (`pcache2.rs`) alone | median 3.459× | 5 |
   | | PCACHE2 + MEMSTATUS + 8K + reader PRAGMAs | median **7.471×** [7.235, 7.471, 7.625, 7.812, 7.000]; "H1 (residual contention is the default pcache1 mutex) confirmed" | 5 |
   | `84914d36` (2026-05-26) | same stack, O(1) pcache2 rewrite | median 7.375× | 5 |

   `dev/experiments-ledger.md:34-39`: dev-box "×7.0–7.9" but **canonical 4-core CI only 3.07×** (W5.3), "no CI combo cleared 5.33×"; `:52-57` and `:659-661` record that no closure artifact exists and `dev/adr/ADR-0.7.0-ac020-architectural-lever.md` is still `status: draft, HITL-required`. The per-instance page cache is implemented (`src/rust/crates/fathomdb-engine/src/pcache2.rs`, 349 lines, gated by `FATHOMDB_PERF_SQLITE_PCACHE2=1`). Whitepaper `dev/notes/performance-whitepaper-notes.md` §7.4 (`:384-387`) already rated MEMSTATUS "cheap … will not be sufficient on its own"; §7.6 (`:395-402`) already proposes "collapse search to one prepared statement … fewer prepares → less malloc churn". The question's newer prototype (129 → 99 ms, passing) is a different HEAD and box from the 0.7.0 record (4.033× alone, failing); the two are not comparable, which is why Phase 0 re-baselines.
5. **No prepared-statement reuse on the search path.** `read_search_in_tx` (`:14219-15019`) has 12 statement sites (`tx.prepare(...)`/`query_row`), none cached; `prepare_cached` is used only on projection/registry paths; `set_prepared_statement_cache_capacity` is never called (rusqlite default 16, `lib.rs:168`). SQL text is `format!`-assembled from placeholders, constants and an optional `LIMIT`, so it is stable per query shape (distinct-string count to be confirmed, §7).
6. **Query vectors are bound as JSON text** (`Value::Text(query_vector.to_string())`, `:14326-14330`) and wrapped in `vec_quantize_binary(vec_f32(?1))` / `vec_distance_l2(v.embedding, vec_f32(?2))` (`build_vector_phase1_sql`, `:13853`); sqlite-vec parses JSON into a growable array before copying into `sqlite3_malloc` memory (`sqlite-vec.c:699-760`).
7. **Lookaside 1200 B × 500 slots is already default per reader** (`b0aceca6`; `lib.rs:23474-23494`). Recorded: G.0 baseline 3.339× → 3.530× (N=5, seq 563 / conc 161 ms), INCONCLUSIVE, landed anyway (`dev/notes/performance-whitepaper-notes.md:175-190` and §12.3 `:1475-1500`). Only `LOOKASIDE_USED` high-water (57/500) was ever read; `HIT/MISS_SIZE/MISS_FULL` never. The `b0aceca6` message also records the G.0 attribution: `allocator_lookaside` = 26.67% of concurrent cycles, hot stacks `sqlite3DbMallocRaw → __GI___libc_malloc` with pthread contention on the **glibc arena**, from `sqlite3Fts5ExprNew` and `vec0Filter_knn` — i.e. system-allocator contention was seen alongside SQLite's own mutexes.
8. **FathomDB uses no SQLite heap limit or global memory statistic** (`rg heap_limit|sqlite3_memory_used|sqlite3_status(` → 0 hits). The loss in §4.4 falls on hosts, not on FathomDB.

## 1. Executive recommendation

**[Recommendation] Lead path, statistics enabled: cut general-allocator and page-cache allocation calls per search.** With `MEMSTATUS` on, every `sqlite3Malloc`/`sqlite3_free` that misses connection lookaside enters the process-global `mem0.mutex` (§2.1); every page-cache page allocated outside a `SQLITE_CONFIG_PAGECACHE` pool additionally enters `pcache1.mutex` twice (§2.1). Contention scales with **(allocator calls per search) × concurrency**, and the search path is allocation-heavy for reasons fixable inside FathomDB:

1. **Reuse prepared statements on the search path** (`prepare_cached` + capacity sized to the distinct statement count). Each uncached prepare parses, plans and builds a VDBE, then frees it all at finalize. In 3.53.2 a re-executed statement also **reuses its ephemeral b-trees** instead of re-creating them (`OP_OpenEphemeral`: `if( pCx && !pCx->noReuse … ) sqlite3BtreeClearTable(...)` else `sqlite3BtreeOpen(...)`) — so if the search SQL materializes any subquery (§2.1, hypothesis), caching also removes per-execution pager/pcache create, 20-page bulk allocation and destroy. That is the most plausible statistics-on route to the effect the PCACHE2 prototype produced. Never tried on this path; whitepaper §7.6 already points here.
2. **Bind query vectors as BLOBs and quantize in Rust**, removing the JSON parse and one function-call allocation chain per search (all outside lookaside).
3. **Check dispatch for application-level queuing** (round-robin into capacity-4 channels) — not a memstatus effect, but a possible ceiling that would cap any allocator fix; cheap to measure.
4. Only then **re-size lookaside from measured `MISS_SIZE`/`MISS_FULL`**, which the earlier sizing never read.

**Do first, once:** a single attribution run (§5, Phase 1) that counts allocator calls per search and classifies concurrent-arm futex stacks into four buckets — `mem0.mutex`/allocator, `pcache1` page allocation, WAL shared-memory/VFS mutexes (§2.2, new), and channel/park. It decides how much residual is FTS5/vec0-internal allocation (not removable by 1–2) versus prepare/ephemeral/bind allocation (removable).

**Isolation is a fallback, triggered by evidence.** Recommend it only if, after candidates 1–3 plus one combination run, median speedup is still < 5.33× **and** attribution shows the residual dominated by FTS5/vec0 internals or by page-cache traffic that statement reuse did not remove. Even then the answer is tiered: the Python wheel and Node addon already carry a runtime-private SQLite copy (bundled static, ELF-local symbols reported for Linux x86-64; §4.1 for other platforms), so a **compile-time** `SQLITE_DEFAULT_MEMSTATUS=0` for *those copies only* is not "a shared runtime" in the sense of constraint 2 — but it must be compile-time, never a runtime `sqlite3_shutdown` sequence (§4.3), and it creates a documented cross-SDK semantic difference (§4.4). The Rust crate is the genuinely shared case: AC-020 is a Rust-crate test, so the Rust tier passes either by the statistics-enabled path or by a costly private runtime (§4.2) — there is no cheap third route.

## 2. Mechanism, grounded in source

### 2.1 Allocation paths in the bundled SQLite 3.53.2

**[Documented fact]** [malloc.html](https://sqlite.org/malloc.html): global memory statistics (default on; `SQLITE_CONFIG_MEMSTATUS` / compile-time `SQLITE_DEFAULT_MEMSTATUS`) require a mutex; lookaside needs none because a connection is single-threaded. `SQLITE_CONFIG_MEMSTATUS` is start-time only — before `sqlite3_initialize()` or after `sqlite3_shutdown()`, else `SQLITE_MISUSE` ([sqlite3_config](https://sqlite.org/c3ref/config.html)); only `SQLITE_CONFIG_LOG` and `SQLITE_CONFIG_PCACHE_HDRSZ` are "anytime" options. `sqlite3_shutdown()` is not threadsafe and "all open database connections must be closed and all other SQLite resources must be deallocated prior to invoking" it ([initialize.html](https://sqlite.org/c3ref/initialize.html)).

**[Repo-verified, amalgamation]**

- `sqlite3Malloc` (`:31763`): `if( bMemstat ){ mutex_enter(mem0.mutex); mallocWithAlarm(); mutex_leave } else xMalloc()`; `sqlite3_free` (`:31858`) same shape; `mem0.mutex` = `SQLITE_MUTEX_STATIC_MEM` (`:31631`), one per linked copy.
- `sqlite3DbMallocRawNN` (`:32110`): lookaside for `n ≤ 1200` (small pool ≤ 128 B), counting hits/size-misses/full-misses; else `sqlite3Malloc`.
- **Direct callers of the public allocator (never lookaside):** FTS5 (`sqlite3Fts5MallocZero → sqlite3_malloc64`, `:245433`); sqlite-vec (51 `sqlite3_malloc` sites, incl. `fvec_from_value` `:717` and seven scratch arrays in `vec0Filter_knn` `:6657-6711`).
- **Page cache (corrected from draft 1).** `pcache1.separateCache = pPage==0 || bCoreMutex>0` (`:58372-58374`, **OR**): with `SQLITE_THREADSAFE=1` every cache has its own `PGroup` whose `mutex` is NULL, so *fetches* of already-cached pages take no global mutex. But page *allocation* does: `pcache1Alloc` takes `pcache1.mutex` (`SQLITE_MUTEX_STATIC_PMEM`) for the `SQLITE_CONFIG_PAGECACHE` pool path, and on the non-pool path calls `sqlite3Malloc` (`mem0.mutex`) **and then** `pcache1.mutex` again for `PAGECACHE_OVERFLOW` stats unless `SQLITE_DISABLE_PAGECACHE_OVERFLOW_STATS`; `pcache1Free` mirrors this (source at `pcache1Alloc`/`pcache1Free`, `:57996ff`). New caches bulk-allocate `SQLITE_DEFAULT_PCACHE_INITSZ = 20` pages (`:15776`, `pcache1InitBulk` `:57952`). Draft 1 called page traffic "steady-state-free in a 4-record fixture"; the repo's own measurement contradicts that: PCACHE2 alone moved this fixture 2.689× → 3.459× (§0.4). **[Hypothesis, source-grounded]** the page traffic comes from *ephemeral b-trees created per statement execution*: `build_vector_phase1_sql` nests a `SELECT rowid … MATCH … ORDER BY distance LIMIT top_k` subquery inside an outer `ORDER BY l2 LIMIT` query, and the flattener does not flatten a subquery that uses `LIMIT` when the outer query is a join (`:156657`), which materializes it via `OP_OpenEphemeral` → temp pager → `pcache1Create` + `pcache1InitBulk` → destroy at finalize. Because uncached statements are finalized after every search, every execution repeats this. Verification: `EXPLAIN` on the phase-1/phase-2 statements (look for `OpenEphemeral`/`SorterOpen`) and `SQLITE_STATUS_PAGECACHE_OVERFLOW`/`MALLOC_COUNT` deltas per search in a diagnostic process.

**Consequence [Hypothesis]:** per search, every uncached statement's parse/codegen/VDBE/finalize cycle, any ephemeral pager it creates, every FTS5 expression/iteration allocation and every sqlite-vec JSON-parse/KNN scratch allocation round-trips through `mem0.mutex` (and page pages through `pcache1.mutex`), shared by all 8 workers. The reported MEMSTATUS-off signature (concurrent 129 → 99 ms, sequential within noise) is that of removing a cheap-when-uncontended global lock.

### 2.2 Shared runtime state that separate reader connections still contend on (new)

**[Repo-verified, amalgamation]** Every read transaction on a WAL database, with or without memstatus:

- `walTryBeginRead` (`:70456`) → `walLockShared(pWal, WAL_READ_LOCK(mxI))` (`:70648`) → `sqlite3OsShmLock` → `unixShmLock` (`:45471`). Without `SQLITE_ENABLE_SETLK_TIMEOUT` (not set by libsqlite3-sys's `build.rs`) this enters **`pShmNode->pShmMutex`** (`:45569`), one object per open WAL file per process, shared by all 8 reader connections; `fcntl(F_RDLCK)` is issued only when the in-process count for the slot goes 0 → 1 (`aLock[ofst]==0`), and `F_UNLCK` only when the last local holder releases (`:45575-45620`). `sqlite3WalEndReadTransaction` (`:70942`) takes the same mutex again to release. With `SQLITE_ENABLE_SETLK_TIMEOUT` SQLite uses one mutex per slot (`aMutex[SQLITE_SHM_NLOCK]`, `:44732-44756`) — a compile-time upstream improvement, but only reachable here via `LIBSQLITE3_FLAGS`, i.e. a host-global change (§3 rank 8).
- `walIndexTryHdr` (`:70026`) reads the wal-index header twice around `walShmBarrier` → `unixShmBarrier`, which executes `unixEnterMutex(); unixLeaveMutex();` on the **global unix VFS mutex** `unixBigLock` (`SQLITE_MUTEX_STATIC_VFS1`, `:48732`) — an empty critical section, so a cache-line bounce rather than a hold, but process-global and per read-transaction begin. (This is a different code path from the open/close contention in the refuted claim M6, which the reviewers found to be an NFS/many-files phenomenon; no throughput number is asserted here.)
- In WAL mode the database-file SHARED lock is *not* released at read end (`pager_unlock` WAL branch calls only `sqlite3WalEndReadTransaction`; `sqlite3PagerSharedLock` skips `pager_wait_on_lock` when `pagerUseWal`, `:64871`), so there is no per-transaction `fcntl` on the main file.

**[Hypothesis]** With 1,600 searches these add ≥ 3 global mutex enter/leave pairs per search whose critical sections are a few counter updates. Their contention share is unknown and must be measured (Phase 1 bucket 3). The `d448263` A.1 profile (aarch64) reported `___pthread_mutex_lock` plus `__aarch64_swp4_rel/cas4_acq` "likely WAL-pager spinlocks" — consistent with this class being visible. Neither MEMSTATUS-off nor PCACHE2 removes it; only the per-slot mutex build option or a smaller number of transactions per search would.

### 2.3 Contention taxonomy on the AC-020 path

| Source | On path? | Evidence |
|---|---|---|
| `mem0.mutex` (memstatus, global) | **Yes**, per non-lookaside alloc/free | §2.1; MEMSTATUS-off results [Measured] |
| `pcache1.mutex` (global) + page-alloc `sqlite3Malloc` | **Yes** if pages are allocated per search | PCACHE2-alone 2.689 → 3.459× on this fixture [Measured, `2aaf4f12`]; ephemeral hypothesis §2.1 |
| WAL shm lock `pShmMutex` + `unixBigLock` barrier | **Yes**, per read transaction | §2.2 [Repo-verified source]; magnitude unmeasured |
| Connection mutex | No | `SQLITE_OPEN_NO_MUTEX`; `d448263` |
| Database-file lock | No per-transaction cost in WAL mode | §2.2 |
| Shared-cache locks | No | shared cache not enabled [Repo-verified] |
| Page-cache group mutex on fetch | No (mode-1 separate groups, `mutex==NULL`) | `:58372-58374` |
| System allocator | Possible, secondary | G.0 saw glibc-arena contention under `sqlite3DbMallocRaw` (`b0aceca6`) |
| Application-level serialization | Possible | round-robin into capacity-4 channels; writer `Mutex` untouched with `RoutedEmbedder` (`:9558-9562`) |
| Rust-side allocation | Yes, not under any SQLite mutex | Rust global allocator |

### 2.4 Comparable cases

- **[Measured result, external]** SQLite Forum microbenchmark ([021b434e](https://sqlite.org/forum/info/021b434e7aae435749886ec78248b47b9d379c1af247af0e41a96a792ddcc49e)): default config 1 thread 0.320 s vs 64 threads 7.020 s; `SQLITE_CONFIG_MEMSTATUS=0` 0.210 s vs 1.640 s. Machine, version and workload unstated; directional only; the single-thread delta is accounting overhead, not contention.
- **[Documented fact]** Firefox ([bug 445525](https://bugzilla.mozilla.org/show_bug.cgi?id=445525), 2008–09): `sqlite3_config(SQLITE_CONFIG_MEMSTATUS, 0)` → explicit `sqlite3_initialize()` → matching `sqlite3_shutdown()` in the destructor; justified on mechanism, no benchmark in the bug; the Windows `.def` fix was specific to Firefox's XPCOM DLL exports. drh's contemporaneous "mainly for memory-scarce embedded apps" framing understates today's loss (§4.4).
- **[Documented fact]** rusqlite [#964](https://github.com/rusqlite/rusqlite/issues/964) (2021-06 → 2024-10): ~10% from MEMSTATUS-off + larger lookaside reported from prior C/C++ experience; maintainer: process-wide `sqlite3_config` cannot be assumed under rusqlite's control because other linked code may already have initialized SQLite — only exposable as `unsafe` under a "before anything else" contract; `SQLITE_DBCONFIG_LOOKASIDE` unexposed (argument shape; overflow safety). libsqlite3-sys does export raw `sqlite3_config`; what is missing is a safe wrapper (the "compile-time only" reading was refuted in verification). FathomDB already uses raw FFI for lookaside.
- **Reducing allocation frequency without disabling accounting:** SQLite's own lookaside (10–15% workstation gain, workload-dependent, [malloc.html](https://sqlite.org/malloc.html)) is the canonical precedent, and FathomDB applied that class first (INCONCLUSIVE, §0.7) before the larger sources (uncached prepares, ephemeral pagers, JSON vector parsing). No public report of a parallel-read gate closed purely by statement caching was found in this bounded scope; §3's ranking rests on mechanism plus the repo's PCACHE2 evidence.

### 2.5 What the MEMSTATUS-off result establishes and what it does not

**Establishes [Measured, reported + `57354a6c`]:** `mem0.mutex` is a material component of concurrent-arm overhead on both boxes measured (+50% speedup in 0.7.0; −23% concurrent time in the newer prototype), and on the newer box the gate is reachable once that component is removed.

**Does not establish:** (a) that memstatus is the only binding component — in the 0.7.0 record MEMSTATUS-off alone stayed RED and the pass needed PCACHE2 as well; page-cache and WAL-lock classes coexist; (b) a pass on the canonical 4-core CI (3.07×); (c) any margin — N=1, no variability; (d) that a private-runtime build reproduces the gain in shipped packages; (e) anything about production retrieval (§5.0).

## 3. Ranked comparison of statistics-enabled optimizations

Ranking = expected reduction of global-mutex-protected allocator/page calls per search × confidence ÷ semantic risk.

| Rank | Candidate | Mechanism | Scope | Semantic risk | Minimal experiment |
|---|---|---|---|---|---|
| 1 | **Statement reuse on the search path** (`prepare_cached`, `set_prepared_statement_cache_capacity` ≥ distinct statements × 2, min 32) | removes parse/plan/VDBE build + finalize per statement; reuses ephemeral b-trees across executions (3.53.2 `OP_OpenEphemeral` reuse) so per-execution pcache create/bulk-alloc/destroy disappears if present | per connection | low; auto-reprepare transparent (count `SQLITE_STMTSTATUS_REPREPARE`; note it does not count cache evictions — count `sqlite3_prepare*` calls separately) | distinct-SQL census; pre-registered plan identity (§5.2); allocator-call count before/after |
| 2 | **BLOB vector binding + Rust-side quantization** | removes JSON parse and the `vec_quantize_binary` output allocation; `fvec_from_value` still copies once | per query | low–moderate; must be byte-identical (f32 LE; sign-quantization must equal sqlite-vec's; `fvec_from_value` rejects zero-length/non-multiple-of-4 BLOBs) | equality test on existing fixtures; allocator-call count |
| 3 | **Dispatch policy** (shared queue or least-loaded) | removes head-of-line blocking when round-robin assigns two in-flight requests to one worker | engine | low; connections still thread-owned | count `try_send` `Full`; per-worker busy fraction; drop if ≈ 0 |
| 4 | **Lookaside re-sizing from counters** | larger slot if `MISS_SIZE` dominates; more slots if `MISS_FULL` | per connection | none (memory) | read `LOOKASIDE_HIT/MISS_SIZE/MISS_FULL` after the sequential arm; ≤ 2 sizes |
| 5 | **Fewer statements per search** (whitepaper §7.6; fold hydration lookups; skip unneeded fallbacks) | fewer prepares and fewer read-transaction-independent allocations | engine SQL | moderate (plans, ranking tiebreaks) | only after 1–3 if per-statement fixed cost still dominates |
| 6 | **Fewer bind/result copies** (rusqlite binds `&str` as `SQLITE_TRANSIENT`, `statement.rs:665`; `Statement::raw_bind_parameter` `:567` / `raw_query` `:609` avoid the params-slice path) | small; text copies ≤ 1200 B are lookaside-eligible, mostly off the mutex path | engine | low | measure via allocation counter only |
| 7 | `SQLITE_CONFIG_PAGECACHE` / `PCACHE2` / `SQLITE_CONFIG_HEAP` | move page/heap traffic off the default allocator | **process-global, start-time** | high in a shared runtime; [pcache_methods2](https://sqlite.org/c3ref/pcache_methods2.html): "an extreme measure" | **already built and measured** (`pcache2.rs`; 3.459× alone, 7.471× stacked, §0.4). Excluded as a shared-runtime config, but its result is the evidence that page allocation is on this path — which rank 1 targets with statistics on |
| 8 | Upstream / rusqlite | SQLite: per-slot shm mutexes under `SQLITE_ENABLE_SETLK_TIMEOUT` (§2.2) — compile-time, host-global via `LIBSQLITE3_FLAGS`, not a drop-in; lookaside two-pool split (3.31.0) already in effect; no change to the `mem0.mutex` path in the vendored 3.53.2 (release notes beyond the vendored source were not searched). rusqlite 0.40.1: `prepare_cached`, `set_prepared_statement_cache_capacity` (`cache.rs:48`), `flush_prepared_statement_cache` (`:54`), `raw_bind_parameter`/`raw_execute`/`raw_query` | — | — | none needed beyond using the rusqlite APIs in ranks 1 and 6 |

**Why 1 outranks 2 [Hypothesis]:** a prepare allocates dozens of objects (parse tree, `Expr`/`Select` nodes, VDBE op array, `aMem`, cursors, vtab cursor state for vec0/FTS5) and — if any subquery is materialized — a whole temp pager with 20 bulk pages; the JSON parse is one growable array plus one copy. Phase 1 counts decide.

**What 1–6 cannot remove:** FTS5's `sqlite3_malloc64` traffic during MATCH iteration, sqlite-vec's KNN scratch arrays, and the per-transaction WAL shm/VFS mutex pairs (§2.2). If attribution shows these dominate the residual, the statistics-enabled ceiling is below the gate — the isolation trigger (§5.3).

**Not drop-in fixes:** any `SQLITE_CONFIG_*` on a shared runtime; `SQLITE_THREADSAFE=0`/single-thread mode; `-DSQLITE_DEFAULT_MEMSTATUS=0` or `-DSQLITE_ENABLE_SETLK_TIMEOUT` via `LIBSQLITE3_FLAGS` (a build-time env var read by libsqlite3-sys `build.rs:294-305` that reconfigures the *host's* shared copy).

## 4. Isolation options — fork avoidance and maintenance burden

### 4.1 Where the runtime is already private, where it is shared, and how "private" stops being private

**[Repo-verified, config level]** `fathomdb-py` and `fathomdb-napi` are `cdylib`s statically containing SQLite 3.53.2 + sqlite-vec via `rusqlite/bundled`.

**Symbol export per platform [Documented fact, rustc `compiler/rustc_codegen_ssa/src/back/linker.rs`, master, fetched 2026-09-10]:** for dynamic libraries rustc restricts exports on every platform — GNU/ELF via a generated version script (`global:` listed symbols; `local: *;`), Apple/Mach-O via `-exported_symbols_list`, MSVC via a generated `lib.def` passed as `/DEF:`. Draft 1 and the grounding dossier said rustc also passes `--exclude-libs=ALL` by default; the fetched code shows no such flag — the version script alone makes the statically linked `sqlite3_*` symbols non-dynamic on ELF. The question's `nm -D` evidence (Linux x86-64) is consistent with this; macOS (`nm -gU`), Windows (`dumpbin /exports`) and Linux AArch64 artifacts must still be inspected because build-script or linker-flag drift would not be caught otherwise.

**Loader behaviour [Documented fact]:** Node's `process.dlopen(module, filename[, flags])` defaults to `os.constants.dlopen.RTLD_LAZY` and exposes `RTLD_NOW/GLOBAL/LOCAL/DEEPBIND` ([Node docs](https://nodejs.org/api/process.html#processdlopenmodule-filename-flags)); Python documents `sys.setdlopenflags(os.RTLD_GLOBAL)` "to share symbols across extension modules" ([sys docs](https://docs.python.org/3/library/sys.html#sys.setdlopenflags)). These are the realistic ways a host changes symbol scope — **but they only affect symbols in the dynamic symbol table**. A symbol made local by the version script cannot be shared or interposed under `RTLD_GLOBAL`; conversely, if an artifact ever exports `sqlite3_*` (export list bypassed), an `RTLD_GLOBAL` host or a later-loaded module would interpose. On Mach-O the default two-level namespace binds imports per image, so even exported symbols do not interpose unless flat namespace is forced; on PE imports bind per DLL by name (platform linker/loader behaviour stated from general knowledge, not fetched for this report). Jetson: the AArch64 wheel is built for `aarch64-unknown-linux-gnu` with `manylinux: "2_28"` (`.github/workflows/aarch64-release-preflight.yml:33-38`) and a separate Tegra CUDA evidence workflow validates a wheel on a self-hosted `aarch64, jetson` runner (`jetson-tegra-cuda-evidence.yml:32`); the ELF mechanism is identical but the artifact is different (CUDA-linked), so locality must be checked on that wheel, and AC-020 itself behaves differently on aarch64 (LSE outlined atomics in the `d448263` profile).

**Two copies in one process is already the status quo for the SDKs.** A Python host using stdlib `sqlite3` (on Linux distribution Pythons `_sqlite3` dynamically links the system `libsqlite3.so.0`; python.org macOS/Windows installers ship their own copy — stated from packaging knowledge, not verified against sqlite.org) or a Node host using `node:sqlite`/`better-sqlite3` has a second SQLite copy alongside the wheel's/addon's private one.

**Cargo [Documented fact]:** one package per `links` value per graph ([resolver](https://doc.rust-lang.org/cargo/reference/resolver.html); [rusqlite #1605](https://github.com/rusqlite/rusqlite/issues/1605)); `libsqlite3-sys` declares `links = "sqlite3"` (`Cargo.toml:18`) for every feature set. A Rust host depending on `fathomdb` and its own `rusqlite` gets one SQLite copy; bundling escapes *system* SQLite, not the host's rusqlite; `[patch]` is root-only.

### 4.2 Options

| Option | Actual runtime separation? | Coexists with host rusqlite? | `migrate(&rusqlite::Connection)` | Maintenance categories |
|---|---|---|---|---|
| **A. Status quo bundled cdylib** (Py/Node) | Yes per artifact *if* symbols are local (verify on all four platforms) | N/A | unaffected | 4-target symbol-locality validation; document the howtocorrupt hazard (§4.3) |
| **B. `LIBSQLITE3_FLAGS=-DSQLITE_DEFAULT_MEMSTATUS=0` on the shared runtime** | No | changes the host | unaffected | rejected (constraint 2) |
| **C. Private prefixed sys crate, engine off rusqlite** (`fathomdb-sqlite-sys`: amalgamation + a *generated* rename header for the public API, produced from `sqlite3.h` at build time; internals already `static`; sqlite-vec compiled against the renamed header with `SQLITE_CORE`; no `links` key or a distinct one; thin internal wrapper) | Yes on every platform (compiler-level rename) | Yes | must change: keep `fathomdb-schema` on public rusqlite (host's copy — hazard §4.3) or expose a FathomDB-owned connection type | amalgamation bump per SQLite security release (rename list regenerated, not hand-kept); sqlite-vec re-check per bump; bindgen or vendored FFI; publication order sys → engine → SDKs; licensing trivial (SQLite public domain, [copyright](https://sqlite.org/copyright.html); sqlite-vec `MIT/Apache-2.0`, crate `Cargo.toml:30`); MSVC/Apple/GNU link validation; **large one-time engine rewrite** |
| **D. C + rusqlite fork** | Yes | Yes | must change | everything in C **plus a long-lived rusqlite fork** — constraint 5 |
| **E. C-ABI dynamic boundary for Rust hosts** (prebuilt cdylib + FFI wrapper crate) | Yes (loader boundary) | Yes | must change | prebuilt binaries per target/toolchain, fetch logic, ABI stability, symbols; loses source builds |
| **F. `objcopy --redefine-syms`** | ELF only | Linux only | must change | excluded (no Mach-O/PE equivalent) |

**Maintained precedents examined [Repo/registry-verified and fetched 2026-09-10]:** (i) `libsqlite3-sys` — `links = "sqlite3"`; its `bundled-sqlcipher*` features are *variants of the same crate and links value*, so SQLCipher-via-rusqlite is not a coexisting second copy. (ii) `libsql-ffi` (tursodatabase/libsql, `main`) — **no `links` key**, compiles its bundled `sqlite3.c` as a library named `libsql`, and **does no symbol prefixing**. It is therefore the textbook "renamed Cargo dependency without runtime separation": Cargo will not stop it being linked next to `libsqlite3-sys`, both define `sqlite3_*`, and the outcome is a duplicate-definition link error or silent mixing depending on archive-member selection. A GitHub issue search for rusqlite/duplicate-symbol reports in that repo returned no results — absence of reports, not evidence of coexistence. (iii) `sqlite-vec` crate — no `links` key; a `SQLITE_CORE` static lib that calls whatever `sqlite3_*` the final link resolves. Not examined: `sqlite3mc` Rust bindings; Turso/Limbo (not the SQLite C library). No maintained Rust project with a prefixed-symbol SQLite was found; treat as "not found".

### 4.3 Hazards that apply now, not only to C–E

**howtocorrupt §2.3 [Documented fact]** ([How To Corrupt](https://www.sqlite.org/howtocorrupt.html)): each linked copy keeps its own mutex-protected global list of open files to work around POSIX advisory-lock quirks; with two copies, "a close() operation on one connection might unknowingly clear the locks on a different database connection, leading to database corruption" — the developers cite a real product. §2.2 states the mechanism is "on unix platforms" (POSIX advisory locks); Windows is not listed as affected. **This already applies to option A today:** a Python or Node host that opens the FathomDB database file through its own SQLite copy in the same process as the wheel/addon is in exactly this configuration. Constraint 6 requires documenting it now ("do not open the engine's database through another SQLite in the same process on unix"); a private Rust runtime (C–E) would extend the hazard to `fathomdb-schema::migrate(&rusqlite::Connection)` callers unless `migrate` moves to a FathomDB-owned connection type.

**Initialization order with `sqlite3_auto_extension` [Repo-verified]:** `sqlite3_auto_extension` calls `sqlite3_initialize()` before registering (`:143854-143864`), and `sqlite3_shutdown()` calls `sqlite3_reset_auto_extension()` (`:187570ff`), discarding every registered auto-extension. FathomDB registers sqlite-vec this way (`lib.rs:18393-18403`) *after* the perf hook (`:6783-6784`, both `Once`), which is why the repo's shutdown → config → initialize sequence works in a FathomDB-only process on first open. It is **not** valid if any connection exists on that copy (contract above) and it would silently drop sqlite-vec if ever reordered. For the Python/Node tier the only safe form is therefore **compile-time** `SQLITE_DEFAULT_MEMSTATUS=0` on the private copy; library code must never call `sqlite3_shutdown`, and any runtime `sqlite3_config` must precede the first `sqlite3_auto_extension` and the first open.

### 4.4 Memory-control consequences if a private copy disables statistics

**[Documented fact]** For that copy: `sqlite3_memory_used()`, `sqlite3_memory_highwater()`, `sqlite3_status64()`, and soft/hard heap limits become non-operational and unenforced ([config constants](https://www.sqlite.org/c3ref/c_config_covering_index_scan.html); [hard_heap_limit64](https://sqlite.org/c3ref/hard_heap_limit64.html)). Limits are also unenforced under a custom PCACHE2 or `SQLITE_CONFIG_PAGECACHE` pool; the soft limit is advisory, the hard limit makes allocations fail; both cover only the general-purpose allocator and only `sqlite3_malloc` traffic — not mmap'd pages, WAL-index shared memory, sqlite-vec buffers outside `sqlite3_malloc`, or Rust/Python/Node memory ([malloc.html](https://sqlite.org/malloc.html)); `PRAGMA hard_heap_limit` can only lower.

**What remains:** per-connection `sqlite3_db_status()`/`_64()` (`CACHE_USED`, `SCHEMA_USED`, `STMT_USED`, `LOOKASIDE_*`, `CACHE_HIT/MISS/WRITE/SPILL`) use no global state ([db_status](https://sqlite.org/c3ref/db_status.html), [options](https://sqlite.org/c3ref/c_dbstatus_options.html)); `PRAGMA cache_size`, `sqlite3_limit()`, `sqlite3_db_release_memory()` (attempts only; says nothing about RSS or extension memory) remain.

**Equivalent bounded memory?** Only approximately: `SQLITE_CONFIG_HEAP` + MEMSYS5 (needs `-DSQLITE_ENABLE_MEMSYS5`, not in the default bundled build; process-global per copy) gives a hard arena bound with a different failure mode; a `SQLITE_CONFIG_MALLOC` wrapper with per-thread counters can account without a global mutex but enforces nothing unless written to. Neither is per-connection.

**Selection granularity:** per **build** (compile flag) or per **runtime instance** (one `sqlite3_config` per linked copy before its `sqlite3_initialize`) — never per connection or request.

**Cross-SDK parity consequence (explicit):** if the Python wheel and Node addon ship `SQLITE_DEFAULT_MEMSTATUS=0` while the Rust crate keeps statistics, three SDKs of the same engine version have different SQLite semantics: in a Rust host, a host-set hard heap limit is enforced against FathomDB's allocations and `sqlite3_status64` sees them; in Python/Node those global facilities are dark for the private copy. FathomDB's own behaviour is unaffected (§0.8), and no host can reach the private copies' globals except through FathomDB APIs, which expose none — but the difference must be recorded in SDK docs and release evidence, and ideally surfaced as a build-time flag in a status/doctor API. Do not describe isolation as "preserving FathomDB's memory controls": it preserves the *host's* controls by not touching the host's copy; the private copy forgoes the global ones.

## 5. Bounded experiment plan with quantitative decision criteria

### 5.0 What AC-020 measures

Four documents, an 8-dimensional synthetic embedder, 1,600 searches dominated by fixed per-statement, per-function and per-transaction costs. It measures the serialization fraction of the read path across 8 workers at fixed cost — sensitive to allocator/mutex behaviour, page-cache allocation, WAL lock handshakes, dispatch, wake-up latency and core count. It says nothing about production retrieval (I/O, page-cache misses on real corpora, real KNN or FTS5 posting iteration) or about the canonical CI host. Passing it says "no serialization worth more than ~1/5.33 of the fixed-cost runtime"; keep the gate as registered.

### 5.1 Ground rules

- Registered test unchanged; verdict runs use it as-is (`AGENT_LONG=1`, read `AC020_NUMBERS`).
- **Verdict quality** = N = 7 back-to-back runs per configuration on an idle box; report median, min, max, IQR of `sequential_ms`, `concurrent_ms`, speedup. Profiling is never attached to verdict runs.
- **Sequential-arm guard:** disqualify any candidate whose median `sequential_ms` rises > 3% over baseline (constraint 7; the bound is proportional to sequential time).
- **Parameter discipline:** ≤ 2 values per tunable; no sweeps.
- **Budget cap:** Phase 0 (7) + Phase 1 (≤ 9 profiling runs total, one iteration) + ≤ 3 candidates × 7 + 1 combination × 7 + Phase 4 (≤ 4) ≤ 48 short runs. Exceeding the cap is itself a stop.

### 5.2 Phases

**Phase 0 — Baseline on HEAD (verdict quality).** N = 7. Record per-reader `LOOKASIDE_HIT/MISS_SIZE/MISS_FULL/USED` high-water after the sequential arm (extend the helper at `lib.rs:23495`). Success: IQR/median ≤ 10%; noise band defines "significant" (Δ > 2 × IQR).

**Phase 1 — Attribution (profiling quality; separate runs; ≤ 3 per instrument):**

1. *Allocator/page-cache counts per search.* First `nm <perf_gates binary> | grep -E 'sqlite3Malloc|mallocWithAlarm|pcache1Alloc'`: these are `SQLITE_PRIVATE` (static) and may be inlined or stripped; if absent, probe the public `sqlite3_malloc`, `sqlite3_malloc64`, `sqlite3_realloc*`, `sqlite3_free`, `sqlite3_prepare_v2/_v3`, `sqlite3_finalize` (`perf probe -x`), noting that core allocations via `sqlite3DbMallocRawNN → sqlite3Malloc` bypass the public symbols and are undercounted. The complete count comes from a diagnostic-only harness that installs a counting `SQLITE_CONFIG_MALLOC` wrapper before the first open (the harness owns its process; every non-lookaside allocation passes through `xMalloc/xFree`) and reads `SQLITE_STATUS_MALLOC_COUNT`/`PAGECACHE_OVERFLOW` deltas per search. Attribute with `perf record --call-graph dwarf`: prepare/finalize, `fts5*`, `vec0*`/`fvec_from_value`, `pcache1*`, `sqlite3VdbeExec`.
2. *Contention classification.* `perf stat -e syscalls:sys_enter_futex` and `perf record` on futex entry, sequential vs concurrent; bucket user stacks into (a) `sqlite3Malloc`/`sqlite3_free` → `pthread_mutex_lock` (`mem0.mutex`), (b) `pcache1Alloc/Free` (`pcache1.mutex`), (c) `unixShmLock`/`unixShmBarrier`/`walTryBeginRead` (WAL/VFS), (d) channel/`park` (dispatch), (e) glibc arena. (`perf lock contention` exists but its man page does not document user-space mutex attribution; `mutrace` is unsuitable on modern glibc.)
3. *Plan census and dispatch collisions.* `EXPLAIN` and `EXPLAIN QUERY PLAN` for every distinct search statement (record `OpenEphemeral`/`SorterOpen` presence and the distinct-SQL count); diagnostic build: `try_send` + fallback counting `Full`, per-worker busy fraction.

Decision: rank candidates 1–4 by the measured share they can remove; carry ≤ 3 forward. Pre-register: if prepare/finalize + ephemeral + vector-parse < 30% of allocator calls **and** FTS5/vec0 internals + WAL/VFS > 60%, note that the statistics-enabled ceiling is probably below the gate; still run Phase 2 (cheap) with the isolation trigger armed.

**Phase 2 — Candidates one at a time (N = 7 each).**

- **C1 statement reuse.** Pre-registered plan-identity check: capture `EXPLAIN` opcode listings for every distinct statement before and after; they must be identical modulo addresses, and `SQLITE_STMTSTATUS_REPREPARE` must be 0 after warm-up; capacity = distinct strings × 2 (min 32). Keep if Δ`concurrent_ms` < −2 × IQR and the sequential guard holds (the sequential arm improving is legitimate).
- **C2 BLOB binding.** Keep only with byte-identical results on existing recall/parity fixtures and the same Δ criterion.
- **C3 dispatch** (only if collisions > 0 in Phase 1.3). **C4 lookaside** (only if `MISS_SIZE` or `MISS_FULL` ≫ hits); ≤ 2 sizes.

**Phase 3 — Combination (N = 7).** Pass: registered assertion passes in ≥ 6 of 7 **and** median speedup ≥ 5.33 × 1.05.

**Phase 4 — Narrow representative checks (passing combination only).** One run each of the registered latency and write gates (AC-012 text/vector latency at the existing corpus size; the write-throughput gate) within their own noise bands; the plan-identity artefact from C1 is re-attached. Broader verification (`cargo clippy/check --workspace --all-targets`, cross-platform CI, Python/Node parity) is reserved for the selected final implementation.

### 5.3 Stop rules and the isolation trigger

- **Phase 0 already passes** (plausible: 0.7.0 dev-box stacks reached 4.6–7.5×, and HEAD differs substantially): record N = 7, run Phase 4 checks and one canonical-CI confirmation; do no candidate work; the remaining question becomes CI hosting, not engine change.
- **Baseline IQR/median > 10% after one environment fix** (CPU pinning, idle box): stop and escalate the measurement environment; do not proceed to candidates on a noisy baseline and do not touch the test.
- **Phase 1 is one iteration**, ≤ 3 profiling runs per instrument; no re-profiling after each candidate.
- Each candidate gets its 7 runs and no re-runs.
- **Isolation trigger:** after Phase 3, median < 5.33× **and** Phase 1 shows the residual ≥ 60% in buckets (b)-after-C1, FTS5/vec0 internals, or WAL/VFS. If the residual is instead prepare/bind/result-dominated, iterate once on C5 before deciding.
- Dev-box pass but canonical-CI fail → a CI-capacity question (the 0.7.0 ledger shows a 4-core host cannot exhibit 8-way scaling); do not tune to the CI host.
- Budget cap exceeded → stop and report.

## 6. Likely wrong paths and compatibility hazards

1. **Any `sqlite3_config` in a library that may not own initialization.** Host-first → `SQLITE_MISUSE`/no-op; FathomDB-first → silently reconfigures the host (the reported 8 MiB allocation under a 1 MiB host hard limit is the expected consequence). Applies to `MEMSTATUS`, `PAGECACHE`, `PCACHE2`, `HEAP`, `MALLOC`, `MULTITHREAD`.
2. **`sqlite3_shutdown` in library code.** Undefined with open connections; resets auto-extensions (drops sqlite-vec) — §4.3. The repo hook is safe only because it is opt-in, first, and `Once`.
3. **`LIBSQLITE3_FLAGS` as a "build-time fix"** — an env var that configures the host's shared copy.
4. **Single-thread / `THREADSAFE=0` builds** — removes all mutexes, unsafe for a multi-connection engine; no per-connection form exists.
5. **Lookaside as the main lever** — cannot cover FTS5/sqlite-vec allocations; sized without miss counters.
6. **Statement caching with unstable SQL text** — request-dependent literals thrash the cache silently; default capacity 16 is below the warm statement count for both shapes. Census first.
7. **Silent semantic drift from BLOB binding** — quantization/format must match sqlite-vec exactly.
8. **Assuming symbol locality from Linux x86-64** — verify each artifact; remember hosts can load with `RTLD_GLOBAL`, which matters only if symbols are exported.
9. **Two SQLite copies opening one file on unix** — already possible with Python/Node hosts (§4.3); worse with a private Rust runtime and public `migrate(&rusqlite::Connection)`.
10. **`objcopy` renaming** — ELF only.
11. **Reading the forum's 4×, Firefox, or the 0.7.0 dev-box 7.47× as predictions for HEAD** — different workloads, boxes, engine versions; use N = 7 on HEAD.
12. **Chasing the CI host** — a gate-hosting question, not an engine defect.

## 7. Remaining uncertainties and questions for the implementer

**Code questions requiring local verification:**

1. Distinct SQL strings per `semantic-*`/`hybrid-*` search, and whether any is request-dependent (`limit_clause` `:14526`; `candidate_limit` in `build_vector_phase1_sql`).
2. Does `EXPLAIN` of the phase-1/phase-2 statements show `OpenEphemeral` (materialized subquery) or only `SorterOpen`? This decides whether C1 also removes per-execution page-cache traffic (the statistics-on analogue of the PCACHE2 result) — the central untested link in §2.1.
3. Does SQLite hoist `vec_f32(?2)` out of the phase-2 row loop (`OP_Once` prologue) or parse JSON per candidate row?
4. Measured allocator and page-allocation calls per search and their split (Phase 1) — §3's ranking is contingent on it.
5. Contention share of `pShmMutex`/`unixBigLock` (Phase 1 bucket c) — source-verified as on the path, magnitude unknown.
6. Does the capacity-4 per-worker channel ever fill under the 8-caller test; per-worker busy fraction.
7. Symbol locality of `sqlite3_*` in shipped macOS wheels/addons, Windows `.pyd`/`.node`, Linux AArch64 and the Tegra CUDA wheel.
8. Current AC-020 numbers on HEAD, N = 7 (the last recorded series is 0.7.0-era; `dev/perf-history/` has no AC-020 JSON); which HEAD/box produced the question's 537/129/564/99 ms figures.
9. Whether `SQLITE_ENABLE_SETLK_TIMEOUT` per-slot mutexes would matter (only testable in a FathomDB-only build; not a shippable change).

**Open uncertainties in the evidence base:**

- No public report quantifies statement caching or BLOB binding against memstatus contention; the ranking is mechanism-based plus the repo's PCACHE2 evidence.
- The Python stdlib linkage statement (§4.1) is from packaging knowledge, not a verified source; the two-copies conclusion holds either way.
- libsql-ffi's coexistence failure mode with libsqlite3-sys is inferred from its manifest/build script (no `links`, no prefixing); no linking experiment was run.
- The forum microbenchmark's environment is unknown; the Firefox precedent has no numbers.
- Maintenance estimates for option C are categorical only, per the brief.

**Questions for the implementer/HITL:**

- Must the Rust-crate tier pass AC-020 in the host-shared configuration, or may the Rust SDK carry a documented private-runtime option while Python/Node ship a compile-time per-copy setting (accepting the §4.4 parity difference)?
- If `migrate(&rusqlite::Connection)` stays public, is a documented "no other SQLite copy opens this file in the same process on unix" rule acceptable — and should it be documented now for the Python/Node SDKs regardless of isolation?
- Which registered gates constitute the "previously recovered read and write performance" Phase 4 must preserve (AC-012 variants and the write-throughput gate assumed here)?
