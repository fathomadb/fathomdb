---
title: FathomDB 0.8.34 release scope — full multi-version data migration and personal agent enablement
status: PROPOSED
target_release: 0.8.34
date: 2026-11-21
---

# FathomDB 0.8.34 release scope

## Theme

Deliver FathomDB's **full, crash-resilient multi-version schema and data
migration contract** across `0.8.30` (schema 37), `0.8.31`, and `0.8.32`
databases — generalizing the minimal Memex helper introduced in `0.8.31` — and
enable local personal agents (**Memex, Hermes, OpenClaw**, and similar
local-first agents) to adopt FathomDB cleanly as their primary database.

## In scope

### 1. Full controlled data migration from `0.8.30`, `0.8.31`, and `0.8.32`

- **Policy ADR and guard update**:
  - Author an ADR superseding `ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md`
    to formalize FathomDB's standing forward-migration contract (explicit
    `fathomdb migrate` verb, mandatory backup/verification, atomic transaction +
    `PRAGMA user_version` step-up, deterministic projection rebuild, and
    post-migration integrity verification).
  - Update `AGENTS.md` § 10 and `scripts/agent-lint-migrations.sh` to distinguish
    forbidden ad-hoc rewrite-era shims from tested, versioned forward
    migrations.
- **Migration execution path (`37 -> 38+ -> current` and `38+ -> current`)**:
  - Deliver `fathomdb migrate <db>` — distributed directly via the lockstep
    `fathomdb-cli` PyPI binary wheel established in `0.8.31` so Python-only
    agent installs (`pip install` / `uv tool install`) upgrade with zero Rust
    toolchain — and evaluate an opt-in SDK/open upgrade path that:
    1. Refuses active locks or corrupt inputs cleanly before touching the store.
    2. Creates a verified pre-migration backup (or requires explicit caller
       confirmation of an external backup).
    3. Applies deterministic schema/data steps atomically inside
       `BEGIN IMMEDIATE`, preserving all canonical nodes, edges, properties,
       scopes, revisions, provenance links, source dependencies, closures,
       operational collections, and queued erasure state.
    4. Rebuilds affected derived projections cleanly and runs full post-upgrade
       integrity verification (`fathomdb doctor full-integrity`).
- **Public logical export / import (`fathomdb export` / `fathomdb import`,
  `fathomdb.logical-export/v1`)**:
  - Ship streaming cursor-free NDJSON + `manifest.json` export/import via the
    lockstep `fathomdb-cli` PyPI wheel alongside `fathomdb migrate` (preserving
    `scope`, outside-`body` `properties`, app-declared vector kinds,
    provenance/dependencies, operational collections/state, and causal
    `full-history` order; fail-closed on unfinished erasure obligations).
  - Support opt-in `--include-vectors` (packed uncentered `vectors.f32.bin` +
    `_fathomdb_vector_spans` + `EmbedderIdentity`, never `embedding_bin` or
    `mean_vec`), defaulting FathomDB-to-FathomDB import to re-embedding with an
    opt-in same-`EmbedderIdentity` `--reuse-vectors` fast path (running
    `recompute-mean` and open-time equivalence verification).
- **Zero-behavior-change CLI / schema / engine-root hotspot splits (Slice 10)**:
  - Split `fathomdb-cli/src/lib.rs` into
    `cli/{args,doctor,recover,migrate,export,output}.rs`, split
    `fathomdb-schema/src/lib.rs` into `schema/{mod,migrations,manifest,triggers}.rs`,
    and thin `fathomdb-engine/src/lib.rs` into focused handle/admin modules
    (updating `dev/tools/module-boundary-gate/src/config.rs`) as Slice 10 builds
    the generalized migration and export engine.
- **Real historical producer qualification**:
  - Qualify migrations against **closed golden `0.8.30` (schema 37)**,
    **`0.8.31` (schema 38+)**, and **`0.8.32`** databases created by the actual
    published artifacts — including active/superseded rows, properties, scopes,
    provenance, dependencies, operational state, and queued erasure.
  - Test crash/interruption mid-migration (proving clean rollback and recovery
    from pre-migration backup with zero silent corruption).
  - Document the forward compatibility and upgrade policy clearly in
    `docs/operations/` and `docs/compatibility/`.

### 2. Local personal agent enablement (Memex, Hermes, OpenClaw, etc.)

- **Turnkey personal-agent integration kits / reference adapters**:
  - Provide tested integration adapters, read+write MCP server profiles, and
    starter templates targeting **Memex, Hermes, and OpenClaw** (and a generic
    local-agent blueprint) mapping:
    - conversation turns, long-term preferences, and factual claims (plain-text
      `body` + outside-`body` typed properties);
    - property-rich entity and relationship graphs with temporal supersession
      and historical replay;
    - workflow checkpoints (`operational_state` / `latest_state` via SDK or
      MCP);
    - engine-enforced caller/persona scoping (separating user statements, agent
      inferences, and external tool/policy sources); and
    - user-initiated correction and erasure (`purge` / `erase_source`).
- **Personal-agent operational ergonomics (incorporating `0.8.30`–`0.8.32` Memex
  lessons)**:
  - Smooth zero-friction local install (`fathomdb`, `fathomdb-cli` PyPI wheels,
    and `fathomdb-mcp`) and offline embedder cache management.
  - Predictable idle memory footprint (`release_models()` and clean lifecycle
    guidance for long-running local daemons).
  - Documented and tested single-writer / multi-reader coordination patterns
    (agent SDK writer + MCP reader/writer + Inspector inspection).
  - Portable local backup/export and live upgrade proof on real `0.8.30` and
    `0.8.31` Memex databases upgraded directly to `0.8.34`.
  - Tested **Postgres + `pgvector` exit recipe** and **multi-database combine
    workflow** (explicit conflict policies for `logical_id`/`source_id`/`scope`
    and erasure-resurrection blocking) in `docs/operations/`, eliminating
    pre-1.0 data lock-in concerns for personal-agent and application adopters.

## Out of scope

- Migration from pre-`0.8.30` prototype formats (`SCHEMA_VERSION < 37`).
- Multi-tenant cloud/networked server mode (FathomDB remains local-first
  embedded + local stdio MCP).
- Publishable competitive benchmark campaigns (allocated to
  [0.8.35](../0.8.35/scope.md)).
