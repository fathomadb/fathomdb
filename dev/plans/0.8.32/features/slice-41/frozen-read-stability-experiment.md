# Slice 41 Empirical Experiment: `FrozenReadContextV1` Stability Across Unrelated Additive Writes, WAL Pinning, BM25 Sensitivity, and Visibility Watermarks

**Decision target:** `0.8.32` `#267` Part 3 ([ADR-0.8.32-frozen-read-additive-stability-and-evidence-eligibility.md](../../../../adr/ADR-0.8.32-frozen-read-additive-stability-and-evidence-eligibility.md), `#103`) — determining how stateless `FrozenReadContextV1` should remain stable across unrelated additive mutations (`kind`, `operational_mutations` append-only log, isolated `scope`, and unrelated background `projection_worker` commits) while preserving security, erasure non-disclosure, query plans, and existing test compatibility.

**Reproducibility:** All runs were executed against the live `0.8.32` worktree wheel (`Schema 39`, `use_default_embedder=False` for structural concurrency/BM25/SQL plans and `use_default_embedder=True` for background projection worker and `vec0` tests) via an external script in `/tmp/slice41_experiments.py` producing `/tmp/slice41_results.json` (`0` scratch files retained in the worktree).

## 1. Environment & Experimental Design

| Parameter | Measured Value |
| --- | --- |
| CPU | `AMD Ryzen Threadripper PRO 5945WX 12-Cores` (`24` hardware threads) |
| OS / Runtime | `Linux-7.0.0-43-generic-x86_64`, Python `3.12.3` |
| SQLite / `sqlite-vec` | SQLite `3.45.1` (Python driver), Engine-bundled `sqlite-vec v0.1.9`, Schema `39` (`PRAGMA user_version = 39`) |
| Store pragmas | `PRAGMA journal_mode=WAL`, `PRAGMA synchronous=NORMAL` (Engine default), `PRAGMA wal_autocheckpoint=1000` (`4 KiB` pages) |
| Workload (Exp 1) | `8` concurrent reader threads × `30` iterations (`240` full workflows/case) running `freeze_read_context(ReadContextV1(eligibility=SearchFilter(kind="Doc"))) -> search_with_evidence -> resolve_evidence_many`, at `0 ms` tight loop and `15 ms` inter-step agent think window (`3.75 ms` freeze-to-search, `11.25 ms` search-to-resolve), on `60`-record and `5,000`-record stores; `25` waves for async projection worker (`C4`); `40` cycles for cross-scope tenant interleaving (`C5`) |
| Workload (Exp 2) | **2A:** `200` writer batches (`25` rows/batch = `5,000` rows, `512 B` payload) under a pinned `BEGIN DEFERRED` snapshot vs. stateless `COMMIT`-before-write with `PRAGMA wal_checkpoint(TRUNCATE)` every `25` batches. **2B:** `5,000`-row heterogeneous-length `Doc` corpus (`10`–`60` random words + rare tags at `~2.5%` DF) evaluated across `20` common-DF and `20` rare-DF 2-term FTS5 queries after appending `+100`, `+500`, and `+2,000` `AppTurn` rows (`total = 5,100`, `5,500`, `7,000`), testing both disjoint and overlapping vocabularies |
| Workload (Exp 3) | **3A:** `1,000` single-row commits and `100 × 50`-row batches across 4 trigger/tracking variants. **3B:** `5,000`- and `50,000`-row Schema `39` stores (`50%` `Doc`, `50%` `AppTurn`, `1,000` interleaved `canonical_edges`) comparing `EXPLAIN QUERY PLAN` and latency (`200`–`400` reps after `10` warmups). **3C:** Live `Engine` `vec0` (`TOP_K_BIT_CANDIDATES = 192`) saturation probe |

## 2. Experiment 1: Concurrent Reader/Writer Drift Rate on Live `0.8.32`

Every reader iteration executes the canonical three-step grounded retrieval workflow over `kind="Doc"` nodes:

1. `freeze_read_context(ReadContextV1(view=ReadView(), eligibility=SearchFilter(kind="Doc")))`
2. `search_with_evidence(EvidenceSearchRequestV1(query=q, context=frozen, limit=5))`
3. `resolve_evidence_many(EvidenceResolveManyRequestV1(evidence_refs=[...], context=frozen))`

Meanwhile, a concurrent writer appends unrelated mutations roughly every `5 ms` (or `projection_worker` drains async embeddings in `C4`, or an interleaved `ScopePolicy.single("tenant-b")` opener commits in `C5`). Note that on `0.8.32`, both `search_with_evidence` and `resolve_evidence_many` map internal `FrozenReadErrorReason::StateDrifted` to the non-disclosing `EvidenceError("evidence_unavailable at /evidenceRef")` contract (`authorize_source_access` / `validate_snapshot` path), whereas `search_frozen` raises `FrozenReadError("state_drifted at /token")`.

### 2.1 Measured Drift Failure Rates (`60`-Record and `5,000`-Record Stores)

| Case | Mutation Workload | Store Rows | Think Window | Writer Commits | Attempts | Failed (`EvidenceUnavailable`) | Search Fails / Resolve Fails | Baseline Failure Rate | Additive-Unrelated Stable Failure Rate |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `C0` | Control (no writer) | 60 | 15 ms | 0 | 240 | 0 | 0 / 0 | **0.00%** | **0.00%** |
| `C1a` | `AppTurn` (`Engine.write`) | 60 | 0 ms | 28 | 240 | 47 | 34 / 13 | **19.58%** | **0.00%** |
| `C1b` | `AppTurn` (`Engine.write`) | 60 | 15 ms | 67 | 240 | 240 | 105 / 135 | **100.00%** | **0.00%** |
| `C1c` | `AppTurn` (`Engine.actuate`) | 60 | 15 ms | 44 | 240 | 240 | 140 / 100 | **100.00%** | **0.00%** |
| `C2a` | `operational_mutations` (`append_only_log`) | 60 | 0 ms | 28 | 240 | 35 | 28 / 7 | **14.58%** | **0.00%** |
| `C2b` | `operational_mutations` (`append_only_log`) | 60 | 15 ms | 59 | 240 | 240 | 138 / 102 | **100.00%** | **0.00%** |
| `C3` | Mixed (`AppTurn` write/actuate + `op_mutations`) | 60 | 15 ms | 86 | 240 | 240 | 92 / 148 | **100.00%** | **0.00%** |
| `C6` | Control: same-`kind` `Doc` additive | 60 | 15 ms | 68 | 240 | 240 | 108 / 132 | **100.00%** | `100.00%` (same `(scope, kind)`) |
| `C7` | Control: destructive `Doc` supersede | 60 | 15 ms | 78 | 240 | 240 | 56 / 184 | **100.00%** | `100.00%` (destructive) |
| `C0` | Control (no writer) | 5,000 | 15 ms | 0 | 240 | 0 | 0 / 0 | **0.00%** | **0.00%** |
| `C1a` | `AppTurn` (`Engine.write`) | 5,000 | 0 ms | 65 | 240 | 240 | 11 / 229 | **100.00%** | **0.00%** |
| `C1b` | `AppTurn` (`Engine.write`) | 5,000 | 15 ms | 84 | 240 | 240 | 126 / 114 | **100.00%** | **0.00%** |
| `C1c` | `AppTurn` (`Engine.actuate`) | 5,000 | 15 ms | 76 | 240 | 240 | 101 / 139 | **100.00%** | **0.00%** |
| `C2a` | `operational_mutations` (`append_only_log`) | 5,000 | 0 ms | 71 | 240 | 240 | 20 / 220 | **100.00%** | **0.00%** |
| `C2b` | `operational_mutations` (`append_only_log`) | 5,000 | 15 ms | 103 | 240 | 240 | 108 / 132 | **100.00%** | **0.00%** |
| `C3` | Mixed (`AppTurn` write/actuate + `op_mutations`) | 5,000 | 15 ms | 73 | 240 | 240 | 139 / 101 | **100.00%** | **0.00%** |
| `C6` | Control: same-`kind` `Doc` additive | 5,000 | 15 ms | 79 | 240 | 240 | 138 / 102 | **100.00%** | `100.00%` (same `(scope, kind)`) |
| `C7` | Control: destructive `Doc` supersede | 5,000 | 15 ms | 81 | 240 | 240 | 123 / 117 | **100.00%** | `100.00%` (destructive) |
| `C4` | Background `projection_worker` only (`AppTurn` async embed, 0 concurrent writer threads) | 60 | drain wait | 0 during read | 25 | 25 | 0 / 25 | **100.00%** | **0.00%** |
| `C5` | Cross-scope write (`ScopePolicy.single("tenant-b")` while `tenant-a` context open) | 60 | reopen | 40 | 40 | 40 | 0 / 40 | **100.00%** | **0.00%** |

Control latency (`C0` end-to-end `freeze -> search_with_evidence -> resolve_evidence_many` including `15 ms` think window) measured `p50 = 20.694 ms` (`p95 = 24.249 ms`) on `60` rows and `p50 = 26.224 ms` (`p95 = 30.565 ms`) on `5,000` rows. On `5,000` rows even at **`0 ms` think time**, that ~14 ms reader window (`C1a` `p50 = 14.398 ms`, `C2a` `p50 = 13.653 ms`) exceeds the `5 ms` writer cadence, collapsing goodput completely (**`100.00%` failure rate**, `240/240`). With a realistic `15 ms` workflow gap between freeze, search, and evidence resolution, **every single unrelated condition (`C1b`, `C1c`, `C2b`, `C3`, `C4`, `C5`) failed `100.00%` of attempts**.

### 2.2 Exact Code Attribution of False-Positive Drift

Inspection of `binding_for_snapshot` in `src/rust/crates/fathomdb-engine/src/frozen_read.rs` explains why all five unrelated mutation classes drift:

| Failure Driver | Exact Mechanism in `0.8.32` | Affected Cases |
| --- | --- | --- |
| Global `canonical_write_boundary` | `load_next_cursor` in `src/rust/crates/fathomdb-engine/src/write_commit.rs` reads `MAX(write_cursor)` across `canonical_nodes`, `canonical_edges`, `operational_mutations`, `operational_state`, `_fathomdb_commit_timestamps`, and `_fathomdb_projection_terminal` globally across all scopes and kinds | `C1`, `C2`, `C3`, `C4`, `C5` |
| Global `read_visibility_generation` & `state_nonce` | `advance_read_visibility` / visibility triggers increment `_fathomdb_read_visibility_state` singleton row `1` and rotate `state_nonce` on every canonical batch (`AppTurn`), every `_fathomdb_projection_terminal` insert during `operational_mutations` (`commit_batch_transaction` `OpStore` path), and every `projection_worker` commit | `C1`, `C2`, `C3`, `C4`, `C5` |
| Non-canonical terminal pollution | `commit_batch_transaction` inserts `('up_to_date', 'non_canonical')` into `_fathomdb_projection_terminal` (firing `_fathomdb_read_visibility_pt_ai`) and invokes `advance_projection_cursor` even for `AppendOnlyLog` writes that touch no search or vector state | `C2`, `C3` |
| Global `projection_cursor` & `serving_digest` | `projection_serving_encoding` scans `SELECT kind,last_enqueued_cursor,updated_at FROM _fathomdb_projection_state ORDER BY kind` globally and hashes global `state_nonce`, plus `binding_for_snapshot` binds global `load_projection_cursor` | `C1`, `C2`, `C3`, `C4`, `C5` |
| Unpartitioned `dependency_generation` | Global `_fathomdb_dependency_generation` key in `_fathomdb_open_state` drifts all scopes even when only another isolated caller scope mutates | `C5` (cross-scope dependencies) |

Eliminating false-positive drift on unrelated additive mutations reduces **`100.00%` failure rates in `C1`–`C5` to `0.00%`**, while retaining `100.00%` fail-closed drift on destructive mutations (`C7`) and matching-partition canonical writes (`C6`).

## 3. Experiment 2: SQLite WAL Pinning vs. Stateless Token & BM25 Sensitivity

### 3.1 Experiment 2A: SQLite WAL Pinning vs. Stateless Token Across 200 Commit Batches

We compared holding an open `BEGIN DEFERRED` snapshot transaction (`SELECT COUNT(*) FROM canonical_nodes`) across `200` writer batches (`25` rows × `512 B` payload = `5,000` inserts) against closing the snapshot transaction immediately (`stateless_token`) while executing `PRAGMA wal_checkpoint(TRUNCATE)` every `25` batches (`wal_autocheckpoint = 1000`, `4 KiB` pages):

| Checkpoint Batch | Pinned WAL Size (B) | Pinned Checkpointed / Busy | Pinned Post-Ckpt WAL (B) | Stateless Pre-Ckpt WAL (B) | Stateless Checkpointed / Busy | Stateless Post-Ckpt WAL (B) |
| ---: | ---: | --- | ---: | ---: | --- | ---: |
| 25 | 659,232 | `0` / `busy=1` (`log=160`) | 659,232 | 659,232 | `0` / `busy=0` | 0 |
| 50 | 1,322,552 | `0` / `busy=1` (`log=321`) | 1,322,552 | 663,352 | `0` / `busy=0` | 0 |
| 75 | 1,985,872 | `0` / `busy=1` (`log=482`) | 1,985,872 | 663,352 | `0` / `busy=0` | 0 |
| 100 | 2,645,072 | `0` / `busy=1` (`log=642`) | 2,645,072 | 659,232 | `0` / `busy=0` | 0 |
| 125 | 3,316,632 | `0` / `busy=1` (`log=805`) | 3,316,632 | 671,592 | `0` / `busy=0` | 0 |
| 150 | 3,979,952 | `0` / `busy=1` (`log=966`) | 3,979,952 | 663,352 | `0` / `busy=0` | 0 |
| 175 | 4,643,272 | `0` / `busy=1` (`log=1,127`) | 4,643,272 | 663,352 | `0` / `busy=0` | 0 |
| 200 | **5,310,712** | **`0` / `busy=1` (`log=1,289`)** | **5,310,712** | 667,472 | **`0` / `busy=0`** | **0** |

**Why WAL pinning is architecturally unacceptable:**

1. **Unbounded WAL growth and blocked checkpoints:** `8 / 8` (`100%`) `wal_checkpoint(TRUNCATE)` calls were blocked (`busy=1`), growing `-wal` monotonically to `5,310,712 B` (`5.06 MiB`, `1,289` frames — past SQLite's `1,000`-frame autocheckpoint ceiling) versus bounded `671,592 B` peak (`0 B` after truncate) for the stateless token.
2. **Pool starvation and memory retention:** Holding one connection per open `FrozenReadContextV1` exhausts `ReaderWorkerPool` slots under long-lived LLM sessions and holds stale B-tree pages until explicit release.
3. **Breaks cross-process / restart token portability:** `FrozenReadContextV1` tokens (`fdbfr1`, measured at **`424` bytes** for both `SearchFilter(kind="Doc")` and unfiltered contexts against `TOKEN_MAX_BYTES = 1024`) are designed to survive `Engine.close()` / reopen and IPC serialization (`ADR-0.8.25-eligibility-and-frozen-reads.md`), whereas an in-memory SQLite transaction dies on process exit.
4. **Cannot survive erasure:** A pinned WAL snapshot violates physical erasure SLA because a stale reader holds deleted/erased page versions in SQLite's WAL until the reader closes.

### 3.2 Experiment 2B: FTS5 `bm25(search_index)` Corpus-Wide Sensitivity on 5,000-Row Store

We seeded a `5,000`-row `Doc` corpus with heterogeneous document lengths (`10`–`60` tokens) and appended `+100` (`5,100` total rows), `+500` (`5,500` total), and `+2,000` (`7,000` total) `AppTurn` rows, re-running `ORDER BY bm25(search_index), cn.write_cursor LIMIT 20` **even with an explicit `AND cn.kind = 'Doc' AND cn.write_cursor <= 5000` boundary filter**:

| `AppTurn` Vocab Mode | Added `AppTurn` Rows (`Total FTS5`) | Query Suite (`20` Queries Each) | `\|ΔBM25\|` Mean (`Max`) | Rel. `ΔBM25` Mean (`Max`) | Top-10 Exact Order | Top-20 Exact Order | `Jaccard@10` | `Kendall τ@10` |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Disjoint vocab | +100 (`5,100`) | Common-DF (`~20–55%` DF) | `0.0661` (`0.1275`) | `11.96%` (`82.80%`) | `12/20` | `9/20` | `0.9485` | `0.8911` |
| Disjoint vocab | +500 (`5,500`) | Common-DF (`~20–55%` DF) | `0.3472` (`0.5952`) | high (IDF zero-crossing) | `6/20` | `2/20` | `0.8299` | `0.8307` |
| Disjoint vocab | +2,000 (`7,000`) | Common-DF (`~20–55%` DF) | `1.3572` (`1.9367`) | high (IDF zero-crossing) | `3/20` | `0/20` | `0.7229` | `0.6566` |
| Disjoint vocab | +100 (`5,100`) | Rare-DF (`~2.5%` DF) | `0.0332` (`0.0826`) | `0.80%` (`2.00%`) | **`20/20`** | `19/20` | `0.6000` | **`1.0000`** |
| Disjoint vocab | +500 (`5,500`) | Rare-DF (`~2.5%` DF) | `0.1727` (`0.3877`) | `4.20%` (`9.38%`) | `17/20` | `16/20` | `0.5909` | `0.9852` |
| Disjoint vocab | +2,000 (`7,000`) | Rare-DF (`~2.5%` DF) | `0.7611` (`1.2829`) | `18.38%` (`32.07%`) | `11/20` | `10/20` | `0.5455` | `0.9426` |
| Overlapping vocab | +100 (`5,100`) | Common-DF (`~20–55%` DF) | `0.0087` (`0.0407`) | `1.17%` (`3.52%`) | `18/20` | `14/20` | `1.0000` | `0.9956` |
| Overlapping vocab | +500 (`5,500`) | Common-DF (`~20–55%` DF) | `0.0364` (`0.0985`) | `4.71%` (`8.41%`) | `13/20` | `5/20` | `0.9652` | `0.9637` |
| Overlapping vocab | +2,000 (`7,000`) | Common-DF (`~20–55%` DF) | `0.1050` (`0.2474`) | high (IDF zero-crossing) | `3/20` | `0/20` | `0.8481` | `0.8385` |
| Overlapping vocab | +100 (`5,100`) | Rare-DF (`~2.5%` DF) | `0.0194` (`0.0260`) | `0.47%` (`0.63%`) | **`20/20`** | **`20/20`** | `0.6000` | **`1.0000`** |
| Overlapping vocab | +500 (`5,500`) | Rare-DF (`~2.5%` DF) | `0.0944` (`0.1302`) | `2.29%` (`3.02%`) | **`20/20`** | **`20/20`** | `0.6000` | **`1.0000`** |
| Overlapping vocab | +2,000 (`7,000`) | Rare-DF (`~2.5%` DF) | `0.3355` (`0.4349`) | `8.13%` (`10.62%`) | **`20/20`** | **`20/20`** | `0.6000` | **`1.0000`** |

*(Note: Rare-DF `Jaccard@10` baseline `< 1.0` reflects `< 10` matching docs per rare term query across set union vs. 2nd-term fallback OR-matches in FTS5; `top10_exact_order` and `Kendall τ` capture actual ranking inversions among returned hits. On common-DF queries at `+500`/`+2,000`, relative percent spikes above `554,629%` occurred on scores initially near `0.0` where FTS5's IDF `ln((N - df + 0.5)/(df + 0.5))` crosses zero as `N` increases past `2 · df`; absolute deltas stayed `0.066`–`1.94`.)*

**Crucial Mathematical Insight:** Even when `AppTurn` rows share **zero vocabulary** with `Doc` queries (`disjoint_vocab`) AND queries explicitly filter `cn.kind = 'Doc' AND cn.write_cursor <= 5000`, SQLite FTS5 computes `bm25(search_index)` from global `search_index` metadata (`N`, `avgdl`, `df(t)`). Adding `AppTurn` rows increases `N` (raising `IDF(t)` uniformly for disjoint terms) and shifts `avgdl` (`30`-token `AppTurn` vs. heterogeneous `Doc` lengths), altering relative document-length normalization `b · dl / avgdl` across hits and causing real top-10 rank swaps (`12/20` at `+100`, `3/20` at `+2,000` common-DF; `11/20` at `+2,000` rare-DF).

**Design implication:** A stateless FathomDB frozen read cannot claim bit-frozen FTS5 `bm25` scores across concurrent `search_index` writes without per-`(scope, kind)` separate FTS5 virtual tables. Instead, the ADR must adopt an explicit **BM25 Honesty Contract**: membership, lifecycle, eligibility, provenance, pagination, operational state, and graph topology are exact-frozen, while lexical BM25 scores/tie-adjacent ranks reflect live `search_index` corpus statistics.

## 4. Experiment 3: Trigger/Schema Costs, `EXPLAIN QUERY PLAN`, and `vec0` Isolation

### 4.1 Experiment 3A: Write-Path Visibility Tracking Overhead

| Variant | Mechanism | `1,000` Single-Row `p50` (`ms`) | `1,000` Single-Row `p95` (`ms`) | Single Total (`ms`) | `100 × 50`-Row Batch `p50` (`ms`) | Batch Total (`ms`) | Delta vs. `A` (Batch `p50`) |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| `A_global_coalesced` | Baseline `0.8.32` coalesced 1-row global update | `0.0216` | `0.0380` | `33.16` | `0.1791` | `20.98` | baseline |
| `B_per_scope_kind_trigger` | Per-row SQL trigger upserting `(scope, kind)` `WITHOUT ROWID` watermark | `0.0212` | `0.0331` | `31.29` | `0.2050` | `24.49` | `+0.0259 ms` (`+14.5%`) |
| `C_per_scope_kind_coalesced_app` | Coalescing guard + 1 upsert per distinct `(scope, kind)` at commit end | `0.0260` | `0.0476` | `41.08` | `0.1940` | `23.10` | `+0.0149 ms` (`+8.3%` across 4 scopes/batch) |
| `D_destructive_epoch_only` | Zero insert tracking; `UPDATE`/`DELETE` triggers only | `0.0180` | `0.0268` | `24.33` | `0.1290` | `16.05` | `-0.0501 ms` (`-28.0%`) |

Per-`(scope, kind)` coalesced tracking (`C`) adds only `+4.4 µs` per single-row commit and `+14.9 µs` (`+8.3%`) across 4 distinct `(scope, kind)` buckets in a 50-row batch (far below FathomDB's `> 10% and > 0.25 ms` materiality threshold in `ADR-0.8.25-frozen-pagination-and-operational-state.md`).

### 4.2 Experiment 3B: Validation Overhead, Query Latency, and `EXPLAIN QUERY PLAN` (`5k` and `50k` Schema 39 Stores)

| Operation | Scale | Unbounded / Baseline `p50` (`ms`) | Bounded / Scoped `p50` (`ms`) | `EXPLAIN QUERY PLAN` Observation |
| --- | ---: | ---: | ---: | --- |
| `validate_snapshot` (`baseline_global` vs. `scoped_watermark`) | `5,000` | `0.0114` (`p95 0.0130`) | **`0.0073`** (`p95 0.0075`, **`-36.0%`**) | Replaces 6-table `UNION ALL MAX(write_cursor)` + `_fathomdb_projection_state` scan with 1 singleton + 1 `WITHOUT ROWID` PK lookup |
| `validate_snapshot` (`baseline_global` vs. `scoped_watermark`) | `50,000` | `0.0114` (`p95 0.0120`) | **`0.0073`** (`p95 0.0075`, **`-36.0%`**) | Same $O(1)$ point lookup scaling |
| FTS5 `search_index` + `canonical_nodes` (`write_cursor <= ?`) | `5,000` | `6.1204` | `5.7046` | Both: `SCAN search_index VIRTUAL TABLE INDEX 0:M3` -> `SEARCH cn USING INDEX canonical_nodes_scope_state_active_idx (scope=? AND write_cursor=?)` -> `USE TEMP B-TREE FOR ORDER BY` |
| FTS5 `search_index` + `canonical_nodes` (`write_cursor <= ?`) | `50,000` | `65.7884` | `61.6217` | Identical plan; boundary check filters after join on `write_cursor` |
| Canonical keyset pagination (`write_cursor > ? [AND <= ?]`) | `50,000` | `0.0302` | `0.0306` | Both: `SEARCH canonical_nodes USING INDEX canonical_nodes_scope_kind_cursor_page_idx (scope=? AND kind=? AND write_cursor>?[ AND write_cursor<?])` |
| 2-hop recursive `graph_expand` (`ce.write_cursor <= ?`) | `5,000` | `0.0137` (`p95 0.0212`) | **`0.1135`** (`p95 0.1387`, **`+728%` / `8.3x` slower**) | **PLAN REGRESSION:** `unbounded` uses `canonical_edges_scope_from_active_idx (scope=? AND from_id=?)`; adding `ce.write_cursor <= ?` causes SQLite to switch to `canonical_edges_scope_cursor_idx (scope=? AND write_cursor<?)`, scanning every scope edge <= boundary per hop (`4` vs `3` returned rows)! |
| 2-hop recursive `graph_expand` (`ce.write_cursor <= ?`) | `50,000` | `0.0142` (`p95 0.0150`) | **`0.1099`** (`p95 0.1179`, **`+674%` / `7.7x` slower**) | Same index hijack (`canonical_edges_scope_cursor_idx`) |
| `resolve_evidence_many` (`20`-item batch + boundary) | `50,000` | — | `0.0458` (`p95 0.0529`) | Pure index lookups (`canonical_nodes_scope_cursor_idx`, `sqlite_autoindex__fathomdb_artifact_revisions_2`, `sqlite_autoindex__fathomdb_source_links_1`) |

### 4.3 Experiment 3C: Live `vec0` (`TOP_K_BIT_CANDIDATES = 192`) Saturation Test

On a live `use_default_embedder=True` Schema 39 `Engine` configured with `vector_kinds=["Doc", "AppTurn"]`:

| Step | Database State | Query Filter | Vector-Ranked Original `Doc` Hits | Interpretation |
| --- | --- | --- | ---: | --- |
| 1 | `5` `Doc` rows + `250` identical `AppTurn` rows | `SearchFilter(kind="Doc")` | **`5 / 5`** (`100%`) | `vec0` pushes `kind='Doc'` and `scope=?` into binary-KNN metadata constraints **before** `LIMIT 192` (`TOP_K_BIT_CANDIDATES`); unrelated `AppTurn` writes cannot starve `Doc` |
| 2 | Same DB | `SearchFilter()` (`kind=None`) | **`0 / 10` `Doc` in top-10** | Without `kind`, `AppTurn` dominates top-K hits |
| 3 | `+250` exact-query same-kind `Doc` rows (`alpha=1.0`, `limit=20`) | `SearchFilter(kind="Doc")` | **`0 / 5`** (`0%`) | Because `vector_default` (`vec0`) has **no `write_cursor` metadata column** (`rowid` is not pre-filtered inside `vec0` KNN), `250` newer same-`(scope, kind)` vectors exhaust all `192` KNN candidate slots and silently drop pre-boundary `Doc` hits if not fenced |

## 5. Architectural Trade-Off Matrix

| Architecture Option | `C1`–`C5` Drift Rate | Existing Tests (`slice35` / `slice45` / `slice50`) | `vec0` `192`-Cap Safety | Graph Query Plan | WAL Growth / Portability | Validation Latency (`50k` `p50`) | Verdict |
| --- | ---: | --- | --- | --- | --- | --- | --- |
| **O0: Status Quo (`0.8.32` global visibility)** | `14.6%`–`100%` | Passes 100% | Safe | Safe (`0.014 ms`) | `0 B` WAL / Portable | `0.0114 ms` | Unacceptable false-positive drift under concurrency |
| **O1: WAL Snapshot Lease (`BEGIN DEFERRED`)** | `0%` (frozen) | Breaks stateless restart & drift tests | Safe | Safe | **`5.31 MiB` (`8/8` blocked)**; non-portable | `N/A` (pinned tx) | **Disqualified** (WAL bloat, pool/erasure violation) |
| **O2: Global Boundary-Clamp (`write_cursor <= B`) + Destructive Epoch (as default)** | `0%` (for all additive) | **Breaks ~8 normative drift tests** (`slice35`, `slice45`, `slice50`) | **Broken (`0/5` hits)** without `vec0` rebuild | **Regresses `7.7x`–`8.3x`** (`scope_cursor_idx` hijack) | `0 B` WAL / Portable | `~0.008 ms` | **Rejected as default** (`vec0` cap, graph hijack, test breaks) |
| **O3: Scope-and-Kind Partitioned Additive Stability (Schema 39 `_fathomdb_scope_visibility_watermarks`, Default)** | **`0.0%` (`C1`–`C5`)**; `100%` on `C6`/`C7` | **Passes 100%** (zero behavioral breaks) | **Safe (`5/5` hits)** via native `vec0(scope, kind)` | **Safe (`0.014 ms`, no plan change)** | **`0 B` WAL / `~492`–`556 B` token** | **`0.0073 ms` (`-36%`)** | **Recommended (`ADR #103`)** — clean, default, zero regressions |
| **O4: Future Opt-In `Snapshot` Mode (`write_cursor <= B` on same `(scope, kind)`)** | `0%` even on `C6` | Passes 100% (since opt-in) | Requires `vec0` `write_cursor` column + rebuild | Requires `INDEXED BY` / `(scope, from_id, write_cursor)` idx | `0 B` WAL / Portable | `~0.008 ms` | Defer to `0.8.33+` only if same-`(scope, kind)` concurrent additive readers are demanded |
