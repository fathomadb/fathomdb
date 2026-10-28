---
title: What DeepSeek-V4.1-Flash's approach says about scaling FathomDB
date: 2026-10-27
status: NOTE
desc: Reads the systems approach behind DeepSeek-V4.1-Flash and DeepInfer (compress derived state so it moves, split phases, bound every tail, spend speculative work by value, replicate hot units from counters) against FathomDB's position after the agent-access admission queue, and names what to adopt before HA, sharding or replication is requested.
inputs:
  - arXiv 2609.19969, DeepSeek-V4.1-Flash — Pushing the Limits of KV Cache Compression (summary supplied by the owner, 2026-10-23; the paper and the DeepInfer / deepseek-recipe repositories were not read directly)
  - feat/agent-access-block1:dev/design/agent-access/queue-admission.md
  - feat/agent-access-block1:dev/design/agent-access/queue-lock-trace.md
  - feat/agent-access-block1:dev/design/mcp-adapter/engine-proposals/EP-09-interruptible-reads.md
  - feat/agent-access-block1:dev/design/mcp-adapter/engine-proposals/EP-18-reader-dispatch-and-pool-size.md
  - feat/agent-access-block1:dev/interfaces/mcp.md
  - dev/design/engine.md
  - dev/adr/ADR-0.6.0-vector-identity-embedder-owned.md
  - dev/adr/ADR-0.6.0-single-writer-thread.md
  - dev/adr/ADR-0.8.27-offline-physical-erasure-completion.md
  - src/rust/crates/fathomdb-engine/src/{erasure,reader_pool,search_api}.rs
---

# What DeepSeek-V4.1-Flash's approach says about scaling FathomDB

## 1. Why this note exists

FathomDB now has a server interface (the read-only MCP adapter) and request
management (the per-store admission queue built in agent-access Block 1).
Once a database has those, the next requests are predictable: higher
availability, then some form of sharding and replication. Before those
requests arrive, this note asks what the FathomDB team can learn from how
DeepSeek solved its own scaling problems, and what must not be copied.

Provenance: the DeepSeek material is the owner's summary of arXiv 2609.19969
(submitted 2026-09-17) and the DeepInfer engineering dossier. The paper and
repositories were not read for this note; the numbers below are quoted from
the summary and are not verified here. The FathomDB facts are read from code
and design documents at the commits named in the inputs.

## 2. DeepSeek's approach, stripped of the Transformer

Every mechanism in the summary (Causal Encoder-Decoder, CSA2 with the
hierarchical sparse indexer, bounded SWA replay, DSpark, Mega-mHC, Engram,
Wide-EP with EPLB) is an instance of one of five moves:

| # | Move | DeepSeek instance | Why it mattered |
| --- | --- | --- | --- |
| M1 | **Make derived state tiny, then treat it as portable** | One FP4 global KV tensor shared by all 20 decoder layers: 890 B/token, 79× smaller than V3 | A 128K context moves between pools in ~2.3 ms and can sit on shared NVMe for 72 h. The compression is what made disaggregation and persistence economical |
| M2 | **Split phases with different resource profiles into different pools** | Prefill runs 8B active parameters on layers 1–20; decode runs 16B on all 40 | Each pool is sized for its own work; the two are joined by the cheap transfer from M1 |
| M3 | **Bound the worst case, not the average** | Cache-miss recovery replays exactly 140 tokens regardless of prefix length (O(W)); the sparse indexer caps the candidate pool at 16,384 so no layer pays O(N) | Tail latency is engineered, not observed |
| M4 | **Spend speculative and auxiliary work by expected value** | DSpark truncates the draft when cumulative acceptance confidence drops, cutting verified tokens ~30 % with accepted tokens unchanged | Protects the shared All-to-All fabric at high concurrency |
| M5 | **Replicate hot units from rolling counters** | EPLB re-solves expert placement every ~10,000 steps, using 63 spare slots for the hottest experts | A collective never waits on one hot rank |

Plus one organisational move, M6: an engineering dossier (the Inference
Deployment Q&A) ships with the engine and answers the questions the paper
does not.

The mechanisms themselves (attention, MoE routing, speculative decoding) solve
problems FathomDB does not have. The moves transfer.

## 3. Where FathomDB stands against each move

FathomDB's constraints are different in kind: SQLite allows one write
transaction at a time (`dev/adr/ADR-0.6.0-single-writer-thread.md`), the
product is local-first agent memory, and erasure must be complete at rest.
Against that background:

### M1 — portable derived state: the primitives exist, the contract is unwritten

- Vector rows carry no identity strings; identity belongs to the pinned
  embedder (`dev/adr/ADR-0.6.0-vector-identity-embedder-owned.md`). So the
  vector rows, FTS index and graph tables are a function of the canonical
  rows plus one embedder identity. A replica can receive or rebuild
  projections without re-deriving identity.
- `WriteReceipt.cursor`, the batch high-water `write_cursor`
  (`dev/interfaces/rust.md` § write receipts), is a monotonic position. It is
  the natural replication position and the natural "read at least cursor X"
  token.
- `ReadView`, the frozen-read binding and the currency fields are the
  vocabulary for stating how stale a read is allowed to be.

None of this is written down as a replication contract. It should be, before
any replication design starts, because the pieces were built for other reasons
and nobody has checked that they compose.

### M2 — phase separation: the lock trace already found the seam

Ingest and projection are GPU-heavy write work. Query is latency-sensitive
read work. Today they share one process, one embed dispatcher with no
priority (one FIFO of 20 shared with projection batches of up to 64 texts;
projection retries saturation every 25 ms), one writer mutex, and one GPU.
The queue-sim found that writer-mutex and projection contention hold hybrid
search p95 at 0.5–1 s whenever a writer or projection is active, against
20 ms otherwise, independent of every admission setting
(`feat/agent-access-block1:dev/experiments/queue-sim/REPORT.md` § 10).

DeepSeek's answer, two pools joined by a cheap transfer, is the same answer
the lock trace reached from below (simplifications 3 and 4): move the
mean-vector and orient reads off the writer connection, and give query
embedding a lane or priority over projection. The single-process version
comes first and is measurable in the planned concurrency sweep; a separate
projection service is the later, cross-process version of the same split.

The MCP adapter being read-only is already the reader-pool shape. One
writer-owning leader plus N read replicas is a straight extension of the
engine's 1-writer, 8-reader split (`dev/design/engine.md` § Writer / reader
split).

### M3 — bounded worst case: FathomDB's weakest point

The admission queue bounds *waiting* at 10 s. Nothing bounds a *read*:

- A dispatched read blocks on its reply channel with no timeout
  (`src/rust/crates/fathomdb-engine/src/search_api.rs`, the `recv()` after
  reader dispatch). EP-09 (deadline and interruption for reader-pool reads)
  is a draft.
- Graph expand has a work-unit cap in the adapter (10,000) and none in the
  SDK.
- Erasure's at-rest completion runs `wal_checkpoint(TRUNCATE)` with busy
  timeout zero, five attempts 25 ms apart, about 100 ms total
  (`src/rust/crates/fathomdb-engine/src/erasure.rs`,
  `complete_erasure_at_rest`). Any reader pinning a WAL snapshot makes it
  report busy, and the verb fails with `ErasureIncomplete`, rows deleted,
  bytes still in the WAL. Under steady agent reads (cap 4, queue 32, each
  read a DEFERRED transaction) the window may never open. The remedy on
  failure is the offline operator path in
  `dev/adr/ADR-0.8.27-offline-physical-erasure-completion.md`, which has no
  actor inside an embedded host serving agents.

DeepSeek would not ship any of these. The rule to adopt: **every engine
entry point has either a bounded cost or a bounded, typed failure, and the
bound is a parameter the caller sees.** EP-09 is the first instance and is
also the precondition for availability work: a replica you cannot abandon is
a replica you cannot route around.

### M4 — expected-value scheduling: priority, not silent degradation

Hybrid search always runs every arm. The tempting DSpark analogue, dropping
the vector arm when the lexical arm looks confident, changes answer quality
silently, and the owner has already rejected partial results under load
(queue-admission.md § 10, "run anyway with a partial result" is rejected).
The honest analogue is priority and caller-visible knobs:

- query embedding ahead of projection batches in the embed dispatcher;
- snippet point-queries bounded per search (open todo TC-86dbc496);
- rerank depth and pool size as load-sensitive knobs the caller sets, never
  ones the engine guesses.

### M5 — replicate hot units: the store is already the shard

One store is one SQLite file, one tenant or one agent's memory, and the MCP
registry is the router. Splitting a store across nodes would turn hybrid
fusion and graph expand into distributed problems for no benefit. The EPLB
analogue is therefore: many stores, a registry that keeps per-store load
counters, and more read replicas for hot stores. Two things stand in the way
today:

- the adapter forces `max_open_stores = 1`
  (`feat/agent-access-block1:dev/interfaces/mcp.md` § 2);
- the per-store semaphore is the only admission unit; it would need to become
  per replica.

### M6 — the dossier: the public tree has no MCP page

The config keys, busy semantics, run limit and failure modes of the server
exist only in `feat/agent-access-block1:dev/interfaces/mcp.md`. The public
docs tree (`docs/operations/`) has no MCP server page. A deployment Q&A in the
DeepInfer style, written from the interface contract, is cheap and overdue
once Block 1 merges.

## 4. Where DeepSeek's approach must not be copied: erasure

DeepSeek keeps global KV on shared storage for 72 hours and evicts SWA
snapshots after 180 seconds. Persistence with a TTL is a feature for them.
For FathomDB, any WAL-shipping replication (the natural mechanism for a
single-writer SQLite engine) puts erased bytes into replicas, WAL segment
archives and object storage for the retention window. The erasure-at-rest
contract, that success means the bytes are gone, does not survive replication
without an explicit propagation and retention rule, and the offline
completion ADR already notes that WAL truncation "destroys the last
recoverable WAL copy of erased content", which is exactly the copy a replica
would have received. This is the one place FathomDB's requirement is stricter
than DeepSeek's, and it needs an ADR before any replication design, not
after.

## 5. What to do, in order

1. **Write the replication contract now.** One design note: `cursor` as the
   position, `ReadView` and currency as the staleness contract, store as the
   shard unit, erasure propagation as a hard requirement. Everything in it is
   derivable from accepted ADRs; nothing in it is stated.
2. **Treat EP-09 as the first availability feature.** Bounded reads with a
   typed `Interrupted` error, on the SDK surface, before any router or
   replica work.
3. **Separate projection from query as an engine lane first.** The
   writer-mutex and embed-priority fixes from the lock trace, measured in the
   planned live sweep, before any separate service.
4. **Make erasure admission-aware.** Either a quiesce that pauses admission
   and waits for in-flight reads before the TRUNCATE, or a durable deferred
   at-rest completion discharged at the next quiet moment, with the verb
   reporting the obligation. Decide which in the erasure ADR from § 4.
5. **Lift `max_open_stores`** and give the registry per-store load counters.
   This is the sharding work, and it is small.
6. **Publish the MCP deployment dossier** under `docs/operations/`.

Do not borrow CSA2, DSpark or EPLB as mechanisms. Borrow the discipline:
compress the derived state so it can move, split phases, bound every tail,
spend speculative work by value, replicate hot units from counters, and ship
the operational answers with the engine.
