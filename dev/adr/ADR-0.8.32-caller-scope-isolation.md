---
title: ADR-0.8.32 — Engine-enforced caller scope isolation, 8-column vec0, TABLE_MANIFEST, and Schema 38 vs. 39 selection
date: 2026-11-24
target_release: 0.8.32
desc: Proposed specification for opener-bound ScopePolicy anti-spoof enforcement across read, search, graph, frozen, evidence, write, actuate, and erasure paths; mandatory 8-column vec0 (scope TEXT DEFAULT ''); single-source TABLE_MANIFEST; and WP-10D empirical comparison between S39-COMPOSITE (recommended technical winner) and S38-PARTIAL-A2 (non-breaking fallback) awaiting HITL Decision 1 ruling.
blast_radius: src/rust/crates/fathomdb-schema/src/lib.rs; src/rust/crates/fathomdb-engine/**; src/rust/crates/fathomdb-sdk/**; src/rust/crates/fathomdb-py/**; src/rust/crates/fathomdb-napi/**; src/python/**; src/ts/**; src/conformance/governed-*.json; scripts/governed-surface-pin.json; dev/interfaces/{rust,rust-sdk,python,typescript,wire}.md
status: proposed
---

# ADR-0.8.32: Engine-enforced caller `scope` isolation, 8-column `vec0`, `TABLE_MANIFEST`, and Schema 38 vs. 39 selection

> **Status: `proposed` — awaiting owner HITL Decision 1 ruling from Slice 10 `WP-10D`.**
> All runtime isolation, `ScopePolicy` anti-spoof, `TABLE_MANIFEST`, and mandatory 8-column `vec0` contracts below are settled architecturally; **Section 4 presents the measured `WP-10D` evidence for the final owner choice between `S39-COMPOSITE` (Schema 39, recommended technical winner) and `S38-PARTIAL-A2` (Schema 38 non-breaking fallback)** before ratifying this ADR to `accepted` and unlocking Slice 20.

## Context

`ADR-0.8.31-property-graph-inline-and-declared-indexes.md` (`#73`, `STEP_38_SQL` in `src/rust/crates/fathomdb-schema/src/lib.rs`) added `scope TEXT NOT NULL DEFAULT ''` to `canonical_nodes`, `canonical_edges`, `operational_mutations`, and `operational_state`, and documented a reserved metadata slot in `vec0`, while leaving `scope` inactive (`""`) in `0.8.31`.

Direct code audit (`dev/plans/0.8.32/prework/slice-4.md`) and the `WP-10D` empirical benchmark (`dev/plans/0.8.32/features/slice-10/scope-schema-experiment.md`) exposed four architectural constraints that `0.8.32` must resolve cleanly (`R32-01`, `R32-06`):

1. **`vector_default` (`vec0`) has 7 built-in columns and no `scope` column on disk:** `ensure_vector_partition` (`src/rust/crates/fathomdb-engine/src/vector_storage.rs`) fails closed with `InvalidQuery` on any DDL mismatch, and `reshape_vector_partition_nondestructive` currently joins `LEFT JOIN canonical_nodes` only (omitting `canonical_edges` for `"edge_fact"` vectors). Moreover, `WP-10D` proved 7-column CTE pre-selection collapses by **`19.5x`** on large scopes.
2. **Global unique indexes vs. tenant namespaces:** `0.8.31` Schema 38 has store-wide active unique indexes on `canonical_nodes(logical_id)` and `canonical_edges(logical_id)`, `PRIMARY KEY(collection_name, record_key)` on `operational_state`, zero `(scope, ...)` indexes, and no `scope` column on `_fathomdb_actuation_receipts`.
3. **Fragmented table inventories:** Seven separate table lists (`CANONICAL_TABLES`, visibility triggers, `ROW_OWNED_PROJECTIONS`, non-projection cursor tables, `export_manifest.rs`, `write_commit.rs` already missing 7+ tables, and `physical_erasure/admission.rs`) can drift whenever schema objects or scope rules change.
4. **Full read/write/frozen/graph threading without single-scope regression:** Every read, search, graph, frozen-context, continuation, write, actuation, lifecycle transition, and erasure route must enforce caller scope, while single-scope (`scope == ""`) workloads must hold `0.8.31` latency parity (`HANDOFF-02`).

## Proposed Specification

### 1. Caller `ScopePolicy` Grammar & Opener-Bound Anti-Spoof (`Pin A`)

- **Token and policy grammar:**
  - Empty `""` denotes the default single-scope (`unscoped`) namespace for single-tenant stores.
  - Non-empty caller scope tokens follow a strict canonical UTF-8 namespace grammar: non-empty, bounded length, zero NUL/control bytes, and zero internal delimiter `\x1e`. Wildcard `"*"` is **forbidden** on write grants (`ScopePolicy` parsing fails closed in `OpenOptions` / `EngineConfig` and `fathomdb-mcp` startup validation).
- **Opener-bound anti-spoof (`Pin A`):**
  - Bind caller `ScopePolicy` at `Engine::open` via `OpenOptions` / `EngineConfig` across Rust (`fathomdb-engine`, `fathomdb-sdk`), Python (`fathomdb-py`, `src/python/`), and TypeScript (`fathomdb-napi`, `src/ts/`) with pre-authorized `scripts/governed-surface-pin.json` updates (`Gate 1 Decision 4 Pin A`; `core = 5` and `recovery_denylist = 5` invariant).
  - Write, actuate, transition, purge, and source-erasure operations stamp and validate `scope` strictly from the opener's bound `ScopePolicy` — never from record bodies, `PreparedWrite` payloads, or tool arguments.
  - Cross-scope writes, lifecycle transitions, purge/erase targets, or cross-scope edge endpoints fail atomically (`ScopeViolation`), leaving database state untouched.
  - `_fathomdb_actuation_receipts` binds `operation_id` entries to caller `scope` so idempotent replay and conflict detection are strictly scoped (`ADR-0.8.32-mcp-read-write-and-distribution.md`).
  - `operational_collections` remains a store-global schema registry (`kind = 'latest_state'` / `'append_only_log'`), while `operational_state` and `operational_mutations` enforce row-level caller `scope`.

### 2. Full Route Isolation & Zero-Overhead Single-Scope Fast Path

Thread a single-authority `ScopeFilter` across every data-plane route:

- **Search & lexical/vector/hybrid paths:**
  - Thread `scope` through `search`, `search_text_only`, `search_projected_text`, `use_graph_arm`, and `graph_seed`.
  - Gate FTS5 fast paths (`rank_stream_eligible`, `defer_full_hydration`, `defer_text_identity`, `can_defer_text_identity`) when a non-empty scope filter is active, exclude ownerless `LEFT JOIN` rows (`ownerless_existence_sql`), and scope-filter `read_historical_projected_text` so BM25/IDF corpus statistics never leak cross-scope token frequencies.
  - Preserve `CROSS JOIN canonical_nodes` + `AND rank MATCH 'bm25()' ORDER BY rank` on FTS5 + `scope` joins so SQLite never invents a `USE TEMP B-TREE FOR ORDER BY` even at `1.1%` scope selectivity (`WP-10D` verified `clean = true` across all 78 captured plans).
- **Point reads, graph expansion, frozen contexts, continuations, and evidence:**
  - Enforce `scope` across `read.get`, `read.get_many`, `read.list`, `read.latest_state`, `read.mutations`, and `read.visibility_state`.
  - Enforce same-scope edges and endpoints across current and `as_of` historical `graph.expand`, and add `ReadView` parity on `graph::search_expand`.
  - Bind `scope` inside `FrozenReadContextV1`, `EvidenceRefV1`, `GraphEvidenceRefV1`, and `PageCursor`. Stale or revoked grants invalidate old tokens/cursors, and out-of-scope references return nondisclosing `not_found` (`D-01` nondisclosure).
- **Erasure separation:** Immediate scope-grant revocation hides out-of-scope records from visibility at once, independently of physical erasure (`purge`, `erase_source`, `fathomdb doctor orphan-provenance`, `fathomdb recover --excise-source`), which remains scope-aware and scrubs only authorized rows cleanly.
- **Single-scope (`scope == ""`) fast path:** Single-scope callers preserve `0.8.31` read/write performance (`-4.4%` to `+3.1%` in `WP-10D`, zero `process_open` or peak RSS regression).

### 3. Mandatory 8-Column `vec0` (`scope TEXT DEFAULT ''`) & `TABLE_MANIFEST`

- **8-column `vec0` (`S38-PARTIAL-A1` rejected):**
  - `WP-10D` proved 7-column `vec0` CTE pre-selection (`S38-PARTIAL-A1`) collapses to **`543.11 ms` p50 / `805.57 ms` p95** on `heavy_90pct` (`90,000` vectors in scope — **`19.5x` slower** than 8-col `vec0`) and `67.71 ms` p50 on `uniform_10pct` (`4.5x` slower), while evicting SQLite's 2 MiB cache (`graph_2hop` rises from `0.09 ms` to `1.14 ms`).
  - Therefore, `vector_default` **MUST** adopt the 8 built-in column layout adding `scope TEXT` (`vec_bit(?)` copy of quantized bits, keeping `chunk_size=64` and `MAX_VEC0_NODE_PROPERTY_SLOTS = 8`) via non-destructive reshape (`reshape_vector_partition_nondestructive` fixed to include `LEFT JOIN canonical_edges ce` alongside `canonical_nodes cn`, `COALESCE(cn.scope, ce.scope, '')`), while `ensure_vector_partition` accepts 7-column `0.8.31` partitions during upgrade/read-only open.
  - On `1.1%` light scopes (`light_1pct`) inside a 100k-node store, 8-column `vec0` delivers **`9.00–9.06 ms` p50**, **`0.9970` `recall@10`** (`min = 0.90`), and **`10.00 / 10` results (`empty = 0`)**.
- **Single-source declarative `TABLE_MANIFEST`:**
  - Unify all 7 fragmented table inventories (`CANONICAL_TABLES`, visibility triggers, `ROW_OWNED_PROJECTIONS`, non-projection cursor tables, `export_manifest.rs`, `write_commit.rs`, and `physical_erasure/admission.rs`) into one declarative `TABLE_MANIFEST` in `fathomdb-schema` before threading `scope`, enforced by `sqlite_master` completeness and `ExportClass` audit tests (`R32-06`).

### 4. Decision 1 Empirical Tradeoff: `S39-COMPOSITE` vs. `S38-PARTIAL-A2` (`WP-10D`)

Measured in `src/rust/crates/fathomdb-tc5-benchmark/src/bin/scope_schema_experiment.rs` (`dev/plans/0.8.32/features/slice-10/scope-schema-experiment.md`, `scope-schema-results.json` `sha256=db8e75c0d35c4a03af02c071c281353fdfe78bf62eb2acab0f82a65920a2ecd2`, SQLite `3.54.1` / `sqlite-vec v0.1.9`, `--release`, `seed = 20261124`, `100,000` nodes / `150,000` edges, `N = 200` reads, `N = 100` writes, `N = 30` child-subprocess opens):

| Dimension (`100k` nodes / `150k` edges) | `S39-COMPOSITE` (Recommended Winner, Schema 39) | `S38-PARTIAL-A2` (Fallback, Schema 38) | `B0` / `S38-A1` Context |
| --- | --- | --- | --- |
| Schema version & `0.8.31` store upgrade | **`SCHEMA_VERSION = 39`** (offline `fathomdb migrate 38 -> 39` w/ `MIGRATION-ACCRETION-EXEMPTION` + 8-col `vec0` reshape) | **`SCHEMA_VERSION = 38`** (zero migration; idempotent `WHERE scope != ''` partial indexes + lazy 8-col `vec0` reshape) | `B0` `user_version = 38`, 7-col `vec0` |
| Tenant keyspace isolation | **Native per-scope namespaces:** `UNIQUE (scope, logical_id) WHERE superseded_at IS NULL` & `PRIMARY KEY(scope, collection_name, record_key)` | **Store-wide global keyspace:** shared unique IDs guarded by pre-insert `SELECT scope` ownership check (`ScopeViolation` on cross-scope collision) | Global keyspace, no ownership guard |
| `node_batch_200_fresh` (`NORMAL` p50) | **`85.29–87.07 ms`** (`86.93` single, `85.29` uniform, `87.07` skewed — **`-8.0%` to `-9.1%` faster** than `S38-A1`/`A2`) | `94.81–98.06 ms` (`97.50` single, `98.06` uniform, `94.81` skewed) | `B0` `94.52 ms` single; `S38-A1` `93.78–100.50 ms` |
| `checkpoint_fresh` / `upsert` (`NORMAL` p50) | **`0.0396–0.0649 ms`** (**`-13%` to `-58%` faster** on multi-scope; no pre-insert `SELECT scope`) | `0.0677–0.0998 ms` | `B0` `0.0525` / `0.0636 ms`; `S38-A1` `0.0540–0.1057 ms` |
| `database_bytes` (`single_scope`) | `469.79 MiB` (`+1.43%` / `+6.60 MiB` vs `B0`) | **`465.30 MiB`** (`+0.46%` / `+2.11 MiB` vs `B0`) | `B0` `463.19 MiB` (`485,691,392 B`) |
| `database_bytes` (`multi_scope` uniform / skewed) | **`484.79` / `483.02 MiB`** (**`5.83–6.65 MiB` smaller** than `A2`; `+4.36%` / `+3.99%` vs `B0`) | `490.62` / `489.66 MiB` (maintains both legacy & partial B-trees; `+5.62%` / `+5.43%` vs `B0`) | `B0` `464.53` / `464.46 MiB` |
| `search_vector_k10` (`single` / `uniform` / `heavy` / `light` p50) | `32.29` / **`14.98`** / `27.91` / `9.06 ms` (`light` `recall@10 = 0.9970`, `10.0/10` hits) | **`30.24`** / `15.09` / **`27.71`** / **`9.00 ms`** (`light` `recall@10 = 0.9970`, `10.0/10` hits) | `S38-A1` collapses to `67.71 ms` (`uniform`) and **`543.11 ms`** (`heavy`) |
| `operational_state_page` (`uniform` / `heavy` / `light` p50) | **`0.0421` / `0.0458` / `0.0293 ms`** (**`-12.6%` to `-49.8%` faster** vs `S38-A1`) | `0.0517` / `0.0506` / `0.0347 ms` | `S38-A1` `0.0839` / `0.0524` / `0.0350 ms` |
| Subprocess `process_open` & Peak RSS (`N = 30`) | Zero regression (`10.09–12.50 ms` probe; `21.59–22.23 MiB` RSS) | Zero regression (`9.48–12.67 ms` probe; `21.57–22.25 MiB` RSS) | `B0` `10.43 ms` / `22.32 MiB`; full `Engine::open` `1264.94 ms` / `29.69 MiB` max |
| SQLite query-planner safety (`EXPLAIN QUERY PLAN`) | **No `WHERE scope != ''` trap;** requires binding `scope = ''` (never omitting `scope`) on single-scope paths | **Planner trap:** SQLite cannot prove `? != ''` at prepare time — every non-empty scoped query MUST emit `AND col.scope != ''` explicitly, and partial indexes cannot accelerate `scope = ''` | `clean = true` on all 78 captured plans once guards applied |

#### Pending Owner Decision 1 Ruling

1. **Recommended Technical Winner (`S39-COMPOSITE` — Schema 39):** Strictly beats `S38-PARTIAL-A2` on `200`-node write speed (`-8.0%` to `-9.1%`), single-row checkpoint speed (`-13%` to `-58%`), multi-scope disk footprint (`5.83–6.65 MiB` smaller), operational paging (`-12.6%` to `-49.8%`), native per-scope key isolation (`(scope, logical_id)` and `(scope, collection_name, record_key)`), and zero SQLite partial-index planner traps — at the cost of bumping `SCHEMA_VERSION` `38 -> 39` (requiring `fathomdb migrate 38 -> 39` for existing `0.8.31` stores).
2. **Non-Breaking Fallback (`S38-PARTIAL-A2` — Schema 38):** Retains `SCHEMA_VERSION = 38` so `0.8.31` stores open without migration and limits `single_scope` disk growth to `+0.46%`, while adopting store-global keyspace semantics (same-scope ownership rejection on collision) and mandatory `AND col.scope != ''` query-builder guards.

Once the owner locks Decision 1 (`S39-COMPOSITE` vs. `S38-PARTIAL-A2`), this ADR will be updated to record the ratified option, flip `status: accepted`, and synchronize `dev/plans/release-state-0.8.32.json` (`schema_version`) ahead of Slice 20.

## Consequences

- Slice 20 entry remains gated on owner ratification of Decision 1 (`S39-COMPOSITE` vs. `S38-PARTIAL-A2`).
- Zero-behavior-change test relocation (`10` `*_tests.rs` files) and `search.rs`/`evidence.rs` submodule splits (`P2-01`, `P2-02`, `R32-07`) plus declarative `TABLE_MANIFEST` (`R32-06`) land at Slice 20 entry ahead of `scope` threading.

## Citations

- `dev/plans/0.8.32/features/slice-10/scope-schema-experiment.md` and `scope-schema-results.json` (`WP-10D`)
- `dev/plans/0.8.32/prework/slice-4.md` (`CRITICAL-1`, `CRITICAL-2`, `P4-01`–`P4-06`) and `slice-9-triage.md` (Decision 1, Decision 4 Pin A)
- `ADR-0.8.31-property-graph-inline-and-declared-indexes.md` (`#73`)
