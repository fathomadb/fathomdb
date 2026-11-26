---
title: FathomDB 0.8.32 release scope — caller scope, production read/write MCP, bulk opt-in and hardened durability
status: ACTIVE
target_release: 0.8.32
date: 2026-11-24
---

# FathomDB 0.8.32 release scope

## Theme

Complete FathomDB's **production-grade security, agent-interface, bulk-ingest,
and durability guarantees**: engine-enforced **caller scope and write
authority**, a production **read+write `fathomdb-mcp`** (scoped frozen-evidence
resolution + governed idempotent writes and checkpoints), **opt-in bulk /
non-strict write mode**, and deep durability qualification (**backup/restore/
rebuild, power-loss recovery, and coverage-guided fuzzing**).

*(Created by owner 2026-11-21 split of the original `0.8.31` scope so property
graph + SDK/CLI unblockers + Inspector ship cleanly in `0.8.31` while security,
read+write MCP, bulk opt-in, and deep durability receive dedicated focus here.)*

## In scope

### 1. Engine-enforced caller scope and write authority (External Reviewer P1)

- Activate and enforce the first-class caller/namespace **`scope`** boundary
  via **Schema 39 (`S39-COMPOSITE`, composite `(scope, ...)` PKs/indexes +
  8-column `vec0` + verified offline `fathomdb migrate 38 → 39`)**, ratified
  by owner Decision 1 Option 1 (`2026-11-24`) from Slice 10 `WP-10D` 100k
  measurements (`ADR-0.8.32-caller-scope-isolation.md`):
  - **Default operational posture:** By default, a FathomDB store is
    **single-scope / single-agent** (`scope = ""`, one agent per database);
    Schema 39 adds `scope` as the opt-in isolation layer for multi-agent /
    shared-store deployments. User documentation (`docs/`) **MUST** state that
    single-scope is the default (`0.8.33` tracks a follow-up to revisit/relax
    this direction).
  - Enforced across:
    - `search` (FTS, vector, hybrid, filtered, and `use_graph_arm`),
    - `fetch` / `read.*`,
    - `graph.expand` (current and historical `as_of`) and `graph.search_expand`
      (`ReadView` parity),
    - `freeze_read_context`, continuations, and `resolve_evidence` /
      `resolve_evidence_many` / `resolve_graph_evidence`, and
    - `write`, `actuate`, `transition`, `purge`, and `erase_source`.
- Prove with overlapping multi-caller fixtures and distinct grants that:
  - restricted records stay inaccessible through every route, including old
    references and cursors after access changes;
  - unauthorized or stale write requests leave database state unchanged;
  - lost responses retry idempotently without duplicating mutations; and
  - single/default-scope stores (e.g., personal agents) short-circuit with
    **zero latency penalty**.

### 2. Production read+write `fathomdb-mcp` (External Reviewer P1 + owner A2 ruling 2026-11-21)

- Graduate `fathomdb-mcp` from evaluation build to a first-class supported
  surface:
  - **Bounded exact frozen-evidence search-and-resolve workflow**: return exact
    bytes, revision identity, and typed drift, erasure, and truncation outcomes
    (matching the SDK workflow on identical tasks).
  - **Governed, idempotent writes and `latest_state` checkpoints over MCP**:
    expose bounded caller-scoped write / actuation and checkpoint tools backed
    by `operation_id` digest replay and `expected_write_boundary` preconditions.
  - **Caller scope enforcement** across every MCP tool, session, and
    continuation.
  - **Release packaging**: build, smoke-test, and distribute `fathomdb-mcp`
    release binaries (and evaluate PyPI/npm wrapper distribution alongside
    `fathomdb-cli`).
- Add **Inspector scope/readiness UI view** to reflect caller scopes cleanly.

### 3. Opt-in bulk / non-strict write mode (owner A3 follow-up 2026-11-21)

- Keep `0.8.31`'s **strict endpoint integrity as the default** on `engine.write`.
- Add an explicit **opt-in bulk / non-strict import mode** (per batch request
  or explicit bulk session) for controlled out-of-order ingestion that reports
  `dangling_edge_endpoints` cleanly and supports explicit post-import integrity
  verification (`fathomdb doctor`).

### 4. Hardened durability and quality backlog (`0.8.31` `quality-backlog.md` re-slotted)

- **Backup, restore, and full rebuild proof (External Reviewer P0)**:
  - Verify backup, restore, and rebuild from an authoritative corpus on pinned
    installed artifacts, asserting identical revisions, evidence spans,
    properties, scopes, and graph results (preserving the CLI operator boundary
    in [recovery-surface.md](../../../docs/positions/recovery-surface.md)).
  - Exercise the internal `fathomdb.logical-export/v1` durable-authority
    round-trip as part of the rebuild proof (validating `0.8.31`'s
    `ExportClass` manifest against real stores with properties, history, and
    scopes, and verifying uncentered f32 vector + `_fathomdb_vector_spans` +
    `EmbedderIdentity` export without `embedding_bin` or `mean_vec`).
- **Power-loss / torn-write recovery proof** (from
  [0.8.31/quality-backlog.md](../0.8.31/quality-backlog.md)):
  - Declare the supported filesystem/storage/cache configuration, SQLite
    durability settings, acknowledged-write oracle, and fault model; exercise
    dropped, torn, or reordered writes at commit, WAL/checkpoint, and recovery
    boundaries in a disposable environment (beyond process `SIGKILL`).
- **Broad coverage-guided fuzzing** (from
  [0.8.31/quality-backlog.md](../0.8.31/quality-backlog.md)):
  - Run instrumented coverage-guided fuzzing across query/filter, typed
    property predicates, scope enforcement, authenticated cursors, codec, MCP
    JSON-RPC protocol, and recovery boundaries with declared budgets; retain
    minimized reproducers as ordinary regressions.

### 5. Non-negotiable invariants: erasure, metadata completeness, and zero latency regression

- Scope revocation (immediate visibility hide) remains cleanly separated from
  physical erasure (`purge` / `erase_source`), and erasure respects scope
  authority while removing any scope-owned metadata cleanly.
- Promote `0.8.31`'s `sqlite_master` completeness audit into a single-source
  declarative `TABLE_MANIFEST` in `fathomdb-schema` deriving
  `CANONICAL_TABLES`, `ROW_OWNED_PROJECTIONS`, visibility triggers, erasure
  coverage, `doctor` checks, Inspector visibility, and `ExportClass` from one
  definition so distributed implicit invariants cannot drift.
- `sqlite_master` completeness audit, read-visibility triggers, and paired
  `v0.8.31` vs. `0.8.32` latency/RSS regression gate pass (verifying single-
  scope fast path preserves `0.8.30`/`0.8.31` latency).
- **Cached projection-generation status across pooled readers (`Gate1-D10`)**:
  `search(..., explain=true)`, `search_frozen(..., explain=true)`,
  `search_with_evidence`, and query-seeded `graph.expand` share one unified
  snapshot boundary helper and reuse the scope-partitioned
  projection-generation status cache with monotonic cache-epoch guarding and
  per-connection `PRAGMA data_version` (`+ total_changes` on `Engine.connection`)
  invalidation so declaring a vector projection never triggers an `O(N)`
  full-table `status_in_snapshot` completion scan per steady-state query
  (including when opened with `use_default_embedder = false`).
- **Raw pre-RRF vector cosine similarity (`PerHitExplain.vector_cosine`,
  `Gate1-D10`)**: extend `dev/design/0.8.8-explain-and-telemetry-adr.md`
  §A.4 Q1 specifically for additive `vector_cosine: Option<f64>` (true cosine
  similarity `1.0 - vec_distance_cosine` on uncentered f32 vectors,
  populated iff scored by `execute_vector_arm` / `vector_rank.is_some()`,
  including vector-scored `TextEdge` `edge_fact` hits; strictly `None` on
  zero-norm/`NULL`/`NaN` or vector-absent hits; finite clamped to
  `[-1.0, 1.0]`; zero `vec_distance_cosine` cost when `explain = false`;
  unchanged L2 ordering) on `#[non_exhaustive]` `PerHitExplain` across Rust,
  SDK, Python, and TypeScript while keeping `SearchHit`,
  `EvidenceContributionV1`, `fathomdb-mcp` `PerHitOut`, and `EP-13`'s
  `vector_distance`/`text_bm25` unchanged.
- **Engine query scaling, purge-lineage hardening, artifact-scoped evidence
  `property_predicates`, and partitioned frozen-read additive stability
  (`Gate1-D11`, Slice 41, `ADR-0.8.32` `#103`)**:
  - **Purge lineage & shared-bucket receipt hardening (`#268`)**: verify
    `actuate -> transition("deleted") + purge() -> actuate` across sole-node
    and trailing-cursor purges (in-process and reopened) and narrow
    `purge_inner` actuation-receipt redaction so purging one `logical_id`
    redacts only that entity's receipt references without over-redacting
    surviving sibling entities sharing a `source_id` bucket (while preserving
    full source redaction on `erase_source`).
  - **Zero-schema-change FTS/evidence & `read.get` `O(N)` elimination
    (`#267` Parts 1 & 2, Option 2B)**: eliminate the verified `O(N)`
    hotspots in FTS-only / evidence search (decouple ranking-contribution
    capture from full `explain=true` when `include_explanation=false`; enable
    `search_index.kind = ?` prefiltering, FTS5 `ORDER BY rank` streaming /
    deferred body hydration, guarded unconfigured-vector `fts_only_limit`
    capping, and post-truncation importance/confidence hydration when
    `!importance_enabled`; skip the internal `canonical_nodes` scan in
    `empty_physical_fast_path` when no node vector arm is declared or
    enrolled; replace `WHERE scope != ''` full-index scan in
    `store_single_unscoped` with `EXISTS` range probes; `O(1)` short-circuit
    `detect_vector_soft_fallback`; replace `SELECT COUNT(*)` snapshot pins in
    `frozen_read::mint_inner` and `graph_expand/execution.rs`, short-circuit
    `historical_dense_incomplete` when `query_vector.is_none()`, and eliminate
    duplicate `source_body` SHA-256) and rewrite `read.get` / `read.get_many`
    with `include_superseded=true` using `UNION ALL` over existing
    `canonical_nodes_scope_logical_{active,sup}_idx` with Rust-side
    max-`write_cursor` tiebreaking and `has_source_dependencies`-gated
    eligibility SQL.
  - **Artifact-scoped `SearchFilter.property_predicates` on evidence
    `source_filter` (`#269` Option A, amending `ADR-0.8.25` `#48`)**: clear
    `source_filter.property_predicates` alongside `kind`/`source_type` across
    `authorize_source_access`, `query_graph_evidence_batch`, and
    `resolve_graph_evidence` while keeping `attributes`, lifecycle/validity,
    dependency closures, and Schema 39 `ScopePredicate` strictly
    source-enforced, clarifying `trace_dependency`, and documenting
    intentional `GraphArm` `SearchHit.source_id` vs. `ResolvedEvidenceV1.source_id`
    semantics.
  - **Partitioned Schema 39 frozen-read additive stability (`#267` Part 3,
    `ADR-0.8.32` `#103`)**: extend unreleased Schema 39 in place with
    `_fathomdb_scope_visibility_watermarks` (`WITHOUT ROWID`) and
    `(global_struct_gen, global_struct_nonce)` on
    `_fathomdb_read_visibility_state`, classify all 20 trigger tables,
    generalize `VisibilityCoalescingGuard` across non-`AdminSchema` batches
    without deadlocking `projection_terminal`, implement `ReadSurface`-aware
    `validate_snapshot` with compact `0x53 0x39` wire tags (`492`–`556 B` vs.
    `1,024 B` cap) and universal `serving_digest`, and remove `fathomdb-mcp`
    global `minted_state` pre-gates in favor of `MR-070.2`-compliant frozen
    SDK probes — leaving `scripts/governed-surface-pin.json` (`58` ops / `97`
    allowlist) strictly unchanged.

### 6. Contributor & agent ergonomics and targeted engine hotspot splits

- **Slice 10 orientation & gate diagnostics**:
  - Add `dev/design/cross-cutting-change-checklist.md` (linked from
    `AGENTS.md`) and a 1-page `dev/design/README.md` live subsystem owner map.
  - Split `dev/adr/ADR-0.6.0-decision-index.md` into **Active / Governing** and
    **Superseded / Historical** tables so readers orient on current rulings
    first.
  - Tighten failure diagnostics in `scripts/check-sdk-surface-parity.py`,
    `scripts/check-governed-surface-pin.sh`, and
    `scripts/check-module-boundaries.sh` to name the exact registry or config
    file to update.
- **Zero-behavior-change engine hotspot splits (Slice 20 entry, pre-scope)**:
  - Move top-level `slice*_tests.rs` and `*_tests.rs` files out of
    `fathomdb-engine/src/` so the source directory maps cleanly to engine
    subsystems.
  - Split `fathomdb-engine/src/search.rs` and
    `fathomdb-engine/src/evidence.rs` into cohesive submodules
    (`search/{mod,fts_fast_path,vector_arm,fusion_stage,hydration}.rs`,
    `evidence/{mod,ranked,graph}.rs`) and update
    `dev/tools/module-boundary-policy.txt` before threading `scope`
    through them, reducing diff blast radius and merge conflicts across Slices
    20, 30, and 40.

## Out of scope

- Public OSS repo/doc overhaul, `dev/` declutter, auto-doc redesign, Rust
  toolchain/MSRV upgrade (`1.95.0 → 1.100.0`), and revisiting the single-agent
  default doc direction — scheduled in [0.8.33](../0.8.33/scope.md).
- Full general multi-version `fathomdb migrate` and Hermes/OpenClaw personal
  agent kits — scheduled in [0.8.34](../0.8.34/scope.md).
- Publishable competitive benchmark campaigns — scheduled in
  [0.8.35](../0.8.35/scope.md).
- CUDA 13 delivery — deferred post-`0.8.35` or dedicated release.
