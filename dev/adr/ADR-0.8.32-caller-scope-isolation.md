---
title: ADR-0.8.32 — Engine-enforced caller scope isolation, Schema 39 composite layout, 8-column vec0, and TABLE_MANIFEST
date: 2026-11-24
target_release: 0.8.32
desc: Accepted specification for opener-bound ScopePolicy anti-spoof enforcement across read, search, graph, frozen, evidence, write, actuate, and erasure paths; S39-COMPOSITE (Schema 39 + fathomdb migrate 38 -> 39) with single-scope/single-agent default posture; mandatory 8-column vec0 (scope TEXT DEFAULT ''); and single-source TABLE_MANIFEST.
blast_radius: src/rust/crates/fathomdb-schema/src/lib.rs; src/rust/crates/fathomdb-engine/**; src/rust/crates/fathomdb-cli/**; src/rust/crates/fathomdb-sdk/**; src/rust/crates/fathomdb-py/**; src/rust/crates/fathomdb-napi/**; src/python/**; src/ts/**; src/conformance/governed-*.json; scripts/governed-surface-pin.json; dev/interfaces/{rust,rust-sdk,python,typescript,cli,wire}.md; docs/**
status: accepted
---

# ADR-0.8.32: Engine-enforced caller `scope` isolation, Schema 39 composite layout, 8-column `vec0`, and `TABLE_MANIFEST`

> **Status: `accepted` — ratified by owner HITL Decision 1 Option 1 (`2026-11-24`) from Slice 10 `WP-10D` measurements.**
> **Ratified layout:** `S39-COMPOSITE` (`SCHEMA_VERSION = 39`, composite `(scope, ...)` indexes/PKs, 8-column `vec0`, and verified offline `fathomdb migrate 38 -> 39` upgrade).
> **Default operational posture (owner addendum `2026-11-24`):** By default, a FathomDB database is **single-scope / single-agent** (`scope = ""`); Schema 39 adds `scope` as the opt-in isolation layer for multi-agent / shared-store deployments. Public user documentation (`docs/`) **MUST** state that single-scope (one agent per database) is the default. A follow-up note is recorded in `0.8.33` (`dev/plans/0.8.33/{scope,plan}.md`) to revisit/relax this single-agent default direction as multi-agent usage matures.

## Context

`ADR-0.8.31-property-graph-inline-and-declared-indexes.md` (`#73`, `STEP_38_SQL` in `src/rust/crates/fathomdb-schema/src/lib.rs`) added `scope TEXT NOT NULL DEFAULT ''` to `canonical_nodes`, `canonical_edges`, `operational_mutations`, and `operational_state`, and documented a reserved metadata slot in `vec0`, while leaving `scope` inactive (`""`) in `0.8.31`.

Direct code audit (`dev/plans/0.8.32/prework/slice-4.md`) and the `WP-10D` empirical benchmark (`dev/plans/0.8.32/features/slice-10/scope-schema-experiment.md`) exposed four architectural constraints that `0.8.32` must resolve cleanly (`R32-01`, `R32-06`):

1. **`vector_default` (`vec0`) has 7 built-in columns and no `scope` column on disk:** `ensure_vector_partition` (`src/rust/crates/fathomdb-engine/src/vector_storage.rs`) fails closed with `InvalidQuery` on any DDL mismatch, and `reshape_vector_partition_nondestructive` currently joins `LEFT JOIN canonical_nodes` only (omitting `canonical_edges` for `"edge_fact"` vectors). Moreover, `WP-10D` proved 7-column CTE pre-selection collapses by **`19.5x`** on large scopes.
2. **Global unique indexes vs. tenant namespaces:** `0.8.31` Schema 38 has store-wide active unique indexes on `canonical_nodes(logical_id)` and `canonical_edges(logical_id)`, `PRIMARY KEY(collection_name, record_key)` on `operational_state`, zero `(scope, ...)` indexes, and no `scope` column on `_fathomdb_actuation_receipts`.
3. **Fragmented table inventories:** Seven separate table lists (`CANONICAL_TABLES`, visibility triggers, `ROW_OWNED_PROJECTIONS`, non-projection cursor tables, `export_manifest.rs`, `write_commit.rs` already missing 7+ tables, and `physical_erasure/admission.rs`) can drift whenever schema objects or scope rules change.
4. **Full read/write/frozen/graph threading without single-scope regression:** Every read, search, graph, frozen-context, continuation, write, actuation, lifecycle transition, and erasure route must enforce caller scope, while single-scope (`scope == ""`) workloads must hold `0.8.31` latency parity (`HANDOFF-02`).

## Decision

### 1. Caller `ScopePolicy` Grammar, Default Single-Agent Posture & Opener-Bound Anti-Spoof (`Pin A`)

- **Default posture and user-documentation requirement (owner ruling `2026-11-24`):**
  - By default, a FathomDB store operates in **single-scope / single-agent** mode (`scope = ""`, default `ScopePolicy`). Schema 39 adds `scope` specifically to isolate multi-agent / shared-store deployments without degrading the default single-scope path.
  - Public user documentation (`docs/`) **MUST** explicitly state that single-scope (one agent per database) is the default posture and document `scope` as the opt-in multi-agent isolation boundary. `0.8.33` tracks a follow-up to revisit/relax this single-agent default guidance once multi-agent deployments mature.

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
| `node_batch_200_fresh` (`NORMAL` p50) | **`85.29–87.07 ms`** (`86.93` single, `85.29` uniform, `87.07` skewed — **`-8.2%` to `-13.0%` vs. `S38-A2`**, `-8.2%` to `-13.5%` vs. `S38-A1`/`A2`, `-8.0%` vs. `B0`; note `multi_scope_skewed` `upsert` `NORMAL` autocheckpoint outlier `191.01 ms`, `137.95 ms` under `FULL`) | `94.81–98.06 ms` (`97.50` single, `98.06` uniform, `94.81` skewed; `skewed` `upsert` `152.98 ms`) | `B0` `94.52 ms` single; `S38-A1` `93.78–100.50 ms` |
| `checkpoint_fresh` / `upsert` (`NORMAL` p50) | **`0.0396–0.0649 ms`** (**`-26.0%` to `-42.8%` faster** vs. `S38-A2` across all 4 multi-scope cells; `-1.2%` [CI spans zero] to `-57.5%` vs. `S38-A1`; no pre-insert `SELECT scope`) | `0.0677–0.0998 ms` (`0.0692–0.0998 ms` multi-scope) | `B0` `0.0525` / `0.0636 ms`; `S38-A1` `0.0540–0.1057 ms` |
| `database_bytes` (`single_scope`) | `469.79 MiB` (`+1.43%` / `+6.60 MiB` vs `B0`) | **`465.30 MiB`** (`+0.46%` / `+2.11 MiB` vs `B0`) | `B0` `463.19 MiB` (`485,691,392 B`) |
| `database_bytes` (`multi_scope` uniform / skewed) | **`484.79` / `483.02 MiB`** (**`5.83–6.65 MiB` smaller** than `A2`; `+4.36%` / `+3.99%` vs `B0`) | `490.62` / `489.66 MiB` (maintains both legacy & partial B-trees; `+5.62%` / `+5.43%` vs `B0`) | `B0` `464.53` / `464.46 MiB` |
| `search_vector_k10` (`single` / `uniform` / `heavy` / `light` p50) | `32.29` / **`14.98`** / `27.91` / `9.06 ms` (`light` `recall@10 = 0.9970`, `10.0/10` hits) | **`30.24`** / `15.09` / **`27.71`** / **`9.00 ms`** (`light` `recall@10 = 0.9970`, `10.0/10` hits) | `S38-A1` collapses to `67.71 ms` (`uniform`) and **`543.11 ms`** (`heavy`) |
| `operational_state_page` (`uniform` / `heavy` / `light` p50) | **`0.0421` / `0.0458` / `0.0293 ms`** (**`-12.6%` to `-49.8%` faster** vs `S38-A1`) | `0.0517` / `0.0506` / `0.0347 ms` | `S38-A1` `0.0839` / `0.0524` / `0.0350 ms` |
| Subprocess `process_open` & Peak RSS (`N = 30`) | Zero regression (`10.09–12.50 ms` probe; `21.59–22.23 MiB` RSS) | Zero regression (`9.48–12.67 ms` probe; `21.57–22.25 MiB` RSS) | `B0` `10.43 ms` / `22.32 MiB`; full `Engine::open` `1264.94 ms` / `29.69 MiB` max |
| SQLite query-planner safety (`EXPLAIN QUERY PLAN`) | **No `WHERE scope != ''` trap;** requires binding `scope = ''` (never omitting `scope`) on single-scope paths | **Planner trap:** SQLite cannot prove `? != ''` at prepare time — every non-empty scoped query MUST emit `AND col.scope != ''` explicitly, and partial indexes cannot accelerate `scope = ''` | `clean = true` on all 78 captured plans once guards applied |

#### Ratified Owner Decision 1 Ruling (`2026-11-24`)

- **Accepted (`S39-COMPOSITE` — Schema 39, Option 1):** Owner ratified `S39-COMPOSITE` (`SCHEMA_VERSION = 39`, composite `(scope, ...)` PKs/indexes, 8-column `vec0`, and verified offline `fathomdb migrate 38 -> 39` upgrade alongside the existing `37 -> 38` path under `MIGRATION-ACCRETION-EXEMPTION`). Single-scope queries MUST always bind `scope = ''` so SQLite uses the composite leading-column B-trees with zero planner traps (`EXPLAIN QUERY PLAN` `SEARCH`-only).
- **Rejected (`S38-PARTIAL-A2` — Schema 38 fallback, and `S38-PARTIAL-A1`):** Rejected because `S38-PARTIAL-A1` collapses `19.5x` on heavy vector scopes and `S38-PARTIAL-A2` incurs duplicate multi-scope B-tree disk/write overhead, store-wide `logical_id`/`record_key` collisions across callers, and silent `WHERE scope != ''` partial-index scan traps.

## Consequences

- `SCHEMA_VERSION` advances `38 -> 39` in Slice 20 (`STEP_39_SQL` in `fathomdb-schema/src/lib.rs`), paired with a verified offline `fathomdb migrate` (`38 -> 39`) path tested against a committed golden `0.8.31` Schema 38 fixture.
- Single-scope (one agent per database, `scope = ""`) remains the documented default in `docs/`; `dev/plans/0.8.33/{scope,plan}.md` records the follow-up note to revisit/relax that single-agent default guidance.
- Zero-behavior-change test relocation (`10` `*_tests.rs` files) and `search.rs`/`evidence.rs` submodule splits (`P2-01`, `P2-02`, `R32-07`) plus declarative `TABLE_MANIFEST` (`R32-06`) land at Slice 20 entry ahead of `scope` threading.

## Citations

- `dev/plans/0.8.32/features/slice-10/scope-schema-experiment.md` and `scope-schema-results.json` (`WP-10D`)
- `dev/plans/0.8.32/prework/slice-4.md` (`CRITICAL-1`, `CRITICAL-2`, `P4-01`–`P4-06`) and `slice-9-triage.md` (Decision 1, Decision 4 Pin A)
- `ADR-0.8.31-property-graph-inline-and-declared-indexes.md` (`#73`)
