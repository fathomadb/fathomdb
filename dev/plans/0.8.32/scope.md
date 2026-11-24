---
title: FathomDB 0.8.32 release scope — caller scope, production read/write MCP, bulk opt-in and hardened durability
status: PROPOSED
target_release: 0.8.32
date: 2026-11-21
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
  (using the `scope` column reserved in `0.8.31` schema 38 — **no breaking
  schema bump for `0.8.31` stores**) across:
  - `search` (FTS, vector, hybrid, filtered, and `use_graph_arm`),
  - `fetch` / `read.*`,
  - `graph.expand` (current and historical `as_of`),
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

### 6. Contributor & agent ergonomics and targeted engine hotspot splits

- **Slice 0 orientation & gate diagnostics**:
  - Add `dev/design/cross-cutting-change-checklist.md` (linked from
    `AGENTS.md`) and a 1-page `dev/design/README.md` live subsystem owner map.
  - Split `dev/adr/ADR-0.6.0-decision-index.md` into **Active / Governing** and
    **Superseded / Historical** tables so readers orient on current rulings
    first.
  - Tighten failure diagnostics in `scripts/check-sdk-surface-parity.py`,
    `scripts/check-governed-surface-pin.sh`, and
    `scripts/check-module-boundaries.sh` to name the exact registry or config
    file to update.
- **Zero-behavior-change engine hotspot splits (Slice 10 entry, pre-scope)**:
  - Move top-level `slice*_tests.rs` and `*_tests.rs` files out of
    `fathomdb-engine/src/` so the source directory maps cleanly to engine
    subsystems.
  - Split `fathomdb-engine/src/search.rs` and
    `fathomdb-engine/src/evidence.rs` into cohesive submodules
    (`search/{mod,fts_fast_path,vector_arm,fusion_stage,hydration}.rs`,
    `evidence/{mod,ranked,graph}.rs`) and update
    `dev/tools/module-boundary-gate/src/config.rs` before threading `scope`
    through them, reducing diff blast radius and merge conflicts across Slices
    10, 20, and 30.

## Out of scope

- Public OSS repo/doc overhaul, `dev/` declutter, and auto-doc redesign —
  scheduled in [0.8.33](../0.8.33/scope.md).
- Full general multi-version `fathomdb migrate` and Hermes/OpenClaw personal
  agent kits — scheduled in [0.8.34](../0.8.34/scope.md).
- Publishable competitive benchmark campaigns — scheduled in
  [0.8.35](../0.8.35/scope.md).
- CUDA 13 delivery — deferred post-`0.8.35` or dedicated release.
