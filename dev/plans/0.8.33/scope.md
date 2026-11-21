---
title: FathomDB 0.8.33 release scope — data migration from 0.8.30/0.8.31 and personal agent enablement
status: PROPOSED
target_release: 0.8.33
date: 2026-11-21
---

# FathomDB 0.8.33 release scope

## Theme

Deliver **safe, verified in-place data migration** from `0.8.30` (schema 37)
and `0.8.31`/`0.8.32` (schema 38+) database versions, and enable local personal
agents — **Memex, Hermes, OpenClaw**, and similar local-first agents — to adopt
FathomDB cleanly as their primary database.

## In scope

### 1. Controlled data migration from `0.8.30` and `0.8.31`

- **Policy ADR and guard update**:
  - Author an ADR superseding `ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md`
    to define FathomDB's controlled upgrade contract (explicit upgrade verb,
    mandatory backup/verification, atomic transaction + `PRAGMA user_version`
    step-up, deterministic projection rebuild, and post-migration integrity
    verification).
  - Update `AGENTS.md` § 10 and `scripts/agent-lint-migrations.sh` to distinguish
    forbidden ad-hoc rewrite-era shims from tested, versioned forward
    migrations.
- **Migration execution path (`37 -> 38+ -> current` and `38+ -> current`)**:
  - Provide an explicit CLI verb (`fathomdb migrate <db>`) — and evaluate an
    opt-in SDK/open upgrade path — that:
    1. Refuses active locks or corrupt inputs cleanly before touching the store.
    2. Creates a verified pre-migration backup (or requires explicit caller
       confirmation of an external backup).
    3. Applies deterministic schema/data steps atomically inside
       `BEGIN IMMEDIATE`, preserving all canonical nodes, edges, revisions,
       provenance links, source dependencies, closures, operational collections,
       and queued erasure state.
    4. Rebuilds affected derived projections cleanly and runs full post-upgrade
       integrity verification (`fathomdb doctor full-integrity`).
- **Real historical producer qualification**:
  - Qualify migrations against **closed golden `0.8.30` (schema 37)** and
    **`0.8.31` (schema 38+)** databases created by the actual published
    artifacts — including active/superseded rows, properties, provenance,
    dependencies, operational state, and queued erasure.
  - Test crash/interruption mid-migration (proving clean rollback and recovery
    from pre-migration backup with zero silent corruption).
  - Document the forward compatibility and upgrade policy clearly in
    `docs/operations/` and `docs/compatibility/`.

### 2. Local personal agent enablement (Memex, Hermes, OpenClaw, etc.)

- **Turnkey personal-agent integration kits / reference adapters**:
  - Provide tested integration adapters, MCP server profiles, and starter
    templates targeting **Memex, Hermes, and OpenClaw** (and a generic
    local-agent blueprint) mapping:
    - conversation turns, long-term preferences, and factual claims;
    - property-rich entity and relationship graphs with temporal supersession;
    - workflow checkpoints (`operational_state` / `latest_state`);
    - caller/persona scoping (separating user statements, agent inferences, and
      external tool/policy sources); and
    - user-initiated correction and erasure (`purge` / `erase_source`).
- **Personal-agent operational ergonomics (incorporating `0.8.30` Memex lessons)**:
  - Smooth zero-friction local install and offline embedder cache management.
  - Predictable idle memory footprint (`release_models()` and clean lifecycle
    guidance for long-running local daemons).
  - Documented and tested single-writer / multi-reader coordination patterns
    (agent SDK writer + MCP reader + Inspector inspection).
  - Portable local backup/export and live upgrade path verified on a real
    `0.8.30` Memex database upgraded directly to `0.8.33`.

## Out of scope

- Migration from pre-`0.8.30` prototype formats (`SCHEMA_VERSION < 37`).
- Multi-tenant cloud/networked server mode (FathomDB remains local-first
  embedded + local stdio MCP).
- Publishable competitive benchmark campaigns (allocated to
  [0.8.34](../0.8.34/scope.md)).
