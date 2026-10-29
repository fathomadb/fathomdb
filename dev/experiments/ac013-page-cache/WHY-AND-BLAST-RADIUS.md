# AC-013: why the current design, and the blast radius of changing it

This note reads code and ADRs; it reports no measurement. Two experiments
run against it:

- `exp/0.8.29-ac013-vector-reads`, which stops the vector-stage re-reads
  (option 5);
- `exp/0.8.29-ac013-seed-connections`, which looks at the seed time on the
  projection connections.

It was written on 2026-10-29 against `dev/0.8.29` `9db251dbf`. Todo seq 350
(TC-1b1f69e6).

## 1. Vector search re-reads (option 5)

### How it works today

`vector_default` is a sqlite-vec 0.1.9 `vec0` table. It holds:

- `embedding float[dim]` and `embedding_bin bit[dim]`;
- a `source_type` partition key;
- the metadata columns `kind`, `created_at`, `status` and `attr_<hex>`.

`vector_partition_create_sql` in `vector_storage.rs` sets no `chunk_size`,
so vec0's default of 1,024 rows per chunk applies.

- **Chunk sizes.** One f32 chunk is a single blob of 1,024 × 3,072 B, about
  3 MB at 768-d. That blob is a chain of about 750 overflow pages. A bit
  chunk is 96 KB.
- **The f32 copy.** It exists only inside vec0. `_fathomdb_vector_rows` holds
  just `rowid`, `kind` and `write_cursor`.
- **Phase 1.** `build_vector_phase1_sql` in `filter.rs` runs a bit-KNN over
  `TOP_K_BIT_CANDIDATES` = 192 candidates.
- **Rerank.** The 192 candidates are then reranked through
  `JOIN vector_default v ON v.rowid = c.rowid` with
  `vec_distance_l2(v.embedding, …)`.
- **The read path per candidate.** vec0 resolves each candidate with
  `vec0_get_chunk_position`, then calls `vec0_get_vector_data` in
  `sqlite-vec.c`. That opens a fresh `sqlite3_blob_open` on the chunk row
  and calls `sqlite3_blob_read` at `chunk_offset × 3072`.
- **Why that is costly.** A blob read at an offset follows the overflow
  chain from the start. A new blob handle has no overflow-page cache, so
  each candidate walks on average half a chunk's chain, about 375 pages.
  For 192 candidates that is about 72,000 page requests per search.
  - This estimate matches the measured 72,300 misses at a 2 MB cache.
  - The 141k total suggests a second read per candidate; the experiment's
    trace will say which.
- **Why the cache does not help.** A chain longer than the cache is the
  classic sequential-scan case for an LRU cache: the walk evicts the pages
  it will need next. So a 2 MB cache (about 500 pages) misses nearly every
  chain page. With the shared cache under `SQLITE_ENABLE_MEMORY_MANAGEMENT`
  the cache was large enough to hold the chains, so the re-reads were cheap
  hits and stayed hidden.

### Why it was built this way

ADR-0.7.0-vector-binary-quant (locked) made these choices:

- **Bit column next to f32.** It added the bit column alongside the f32
  column on the existing vec0 table. That kept the write to one extra column
  in the same writer transaction, with "no new commit; no new lock; no new
  SQLite handle".
- **A sibling table was the fallback.** It was to be used only if vec0's
  16-column metadata limit was hit.
- **f32 kept inside vec0.** It stays there for two jobs: the rerank, and the
  recall ground truth (`ac_013b`, eu7).
- **K.** The rerank K started at 64. EU-5a2 raised it to 192, the recall knee
  for the default embedder.

The ADR's numbers were measured with the shared cache in place. None of its
documents considers the overflow-chain cost of reading one row out of a
multi-MB chunk blob. The cost was there all along, hidden by cache capacity.

### Blast radius of a change

| Change | What it touches | Existing stores |
| --- | --- | --- |
| A. f32 in a rowid-keyed table (3 KB row, about 3 pages per lookup) | Schema (a new table, `SCHEMA_VERSION` 34 → 35); `projection_commit.rs` and `vector_storage.rs` insert paths; `mean.rs`, which re-centres and rewrites vectors; `erasure.rs` purge and `excise_source`, which must delete the new rows under `secure_delete` (GDPR axis); `data_plane_integrity.rs` checks; the vector-equivalence self-check; `rebuild_vec0` and `rebuild_projections`; `dependency_trace.rs`; `consolidation.rs`; recall ground truth; disk (+3 KB per row if f32 stays in vec0 as well) | Filling it from vec0 is an `INSERT…SELECT`, forbidden by `AGENTS.md` § 10 in managed-vec releases. The routes are a rebuild or re-projection, a maintenance verb (ADR-0.8.26 derived-maintenance boundary), or a fallback to the vec0 read for rows not yet copied |
| B. Smaller vec0 `chunk_size` | `vector_partition_create_sql` (a one-line option); the shape sentinel in `ensure_vector_partition`, which today keys on `status`; the Slice 15e attribute-reshape recreate path; KNN scan cost (more chunks, so more blob opens per scan, most visible at 100k–1M) | vec0 cannot change `chunk_size` in place, so existing tables must be recreated and refilled. That is the same class as the Pack-1 → Pack-2 stage-and-recreate, and it hits the same § 10 rule. New stores get the option for free |
| C. Read the rerank vectors once per chunk, or reuse the blob handle (`sqlite3_blob_reopen`) | A patch to the vendored sqlite-vec, or engine code that reads chunk blobs directly (coupling to vec0's shadow-table layout); the pinned-override-rot guard; sqlite-vec upgrades | No change to stored data |

Under every option the public API, the bindings and the MCP surface stay
unchanged. Retrieval results must stay identical. A and B change storage
and need an ADR, as a successor note to ADR-0.7.0-vector-binary-quant.

## 2. Seed time on the projection connections

### How it works today

- **Topology.** ADR-0.8.27-engine-owned-runtime-topology (accepted,
  seq-295) sets out:
  - one writer;
  - one projection dispatcher;
  - `scheduler_runtime_threads` projection workers, default 2;
  - eight readers;
  - short-lived probe connections.

  Each projection worker owns its SQLite connection and commits embedded
  vectors under `commit_gate`. Embed workers own no connection.
- **Where vectors are written.** On the projection worker connections
  (`projection_commit.rs` `commit_projection_outcomes`), not on the writer.
  That explains why the writer `cache_size` lever had no effect.
- **How those connections are set up.** `open_runtime_connection` in
  `connection_runtime.rs` sets only `journal_mode=WAL` and
  `secure_delete=ON`, which was added in 0.8.19 to close a GDPR-erasure
  leak.
  - It was introduced with the Phase 9 vector runtime (`1980bf65d`,
    2026-05-03).
  - The `FATHOMDB_PERF_*_PRAGMAS` experiment hooks were later added only to
    the writer and the readers, so the runtime connections were never
    tunable.
  - Nothing chose their `cache_size`; they took the default.
- **Likely mechanism, for the experiment to confirm.** A vec0 insert writes
  into the current chunk's 3 MB f32 blob at an offset. That walks the same
  overflow chain as a read.
  - With 2 MB per connection the walk misses on every insert.
  - In WAL mode a connection also drops its whole page cache when it starts
    a transaction after another connection has committed, and the two
    workers alternate commits. So capacity can help only inside one batch.

### Blast radius of a fixed `cache_size` on the runtime connections

- **Code.** One PRAGMA in `open_runtime_connection`, which also covers the
  dispatcher and the probe connections, or one applied only to the worker
  connections.
- **Memory.** (workers + 1) × the new size per open store, live only while
  the projection runtime has work. With 2 workers and 32 MB that is about
  96 MB.
- **ADRs.**
  - It amends ADR-0.8.28's "each connection keeps SQLite's default
    `cache_size` … no page-cache knob".
  - It touches the ADR-0.8.27 topology qualification, its connection and
    memory accounting.
  - ADR-0.8.25-sqlite-runtime-configuration is unaffected, because this is a
    PRAGMA, not `sqlite3_config`.
- **Tests and gates.** The Slice 40 tests are reader-only, so AC40-96's
  2,000 KiB assertion does not cover these connections. Seed-time numbers in
  the perf gates and in tc5 would move. No public surface changes.
- **The structural alternative.** This is the same fix as option 5 B or C:
  shorter chains make both reads and inserts cheap, whatever the cache size.
