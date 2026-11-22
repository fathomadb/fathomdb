---
title: ADR-0.8.31 — Lockstep fathomdb-cli PyPI binary wheel and minimal schema 37-to-38 upgrade helper
date: 2026-11-22
target_release: 0.8.31
desc: Distribute lockstep fathomdb-cli binaries on PyPI via Maturin bin wheels preserving the SDK-vs-CLI operator boundary, and provide a minimal verified schema-37-to-38 upgrade helper for live 0.8.30 stores ahead of 0.8.34 full migration.
blast_radius: src/rust/crates/fathomdb-cli/**; src/rust/crates/fathomdb-schema/**; src/python/**; .github/workflows/release.yml
status: accepted
---

# ADR-0.8.31: Lockstep `fathomdb-cli` PyPI binary wheel and minimal schema 37→38 upgrade helper

## Context

1. FathomDB intentionally keeps operator and maintenance verbs (`doctor`,
   `recover`, `recompute-mean`, `safe-export`) out of the Python/TypeScript/Rust
   SDK surface (`docs/positions/recovery-surface.md`,
   `ADR-0.8.26-cli-derived-maintenance-boundary.md`). However, Python-only
   applications installed via `pip install` or `uv tool install` do not assume
   a Rust/`cargo` toolchain, leaving `fathomdb doctor` and `recompute-mean`
   unavailable on `PATH`.
2. Schema 38 (`ADR-0.8.31-property-graph-inline-and-declared-indexes.md`) bumps
   `SCHEMA_VERSION` from `37` (`0.8.30`) to `38`. Full general multi-version
   migration (`fathomdb migrate`) is scheduled for `0.8.34`, while live `0.8.30`
   deployments need a safe, verified upgrade bridge to `0.8.31`.

## Decision (Owner rulings 2026-11-21; amended HITL Gate 1, 2026-11-22)

1. **Lockstep `fathomdb-cli` PyPI binary wheel**:
   - Package `fathomdb-cli` as a companion PyPI package (`fathomdb-cli==0.8.31`)
     using Maturin `bindings = "bin"` scoped strictly to `--bin fathomdb`
     (excluding internal workspace helper binaries such as
     `perf-regression-check` from PyPI `PATH`), with an optional `[cli]` extra
     on `fathomdb` (`fathomdb[cli]` -> `fathomdb-cli==0.8.31`).
   - Preserves the SDK-vs-CLI operator boundary completely: zero recovery or
     doctor verbs are added to the Python SDK API (and `fathomdb` keeps
     `[project.scripts]` empty per `AC-041`), while Python users get the exact
     lockstep `fathomdb` executable on `PATH` with no Rust toolchain.
   - Built and smoke-tested across the five generic native targets in
     `release.yml` (verifying `fathomdb --version`, `fathomdb doctor`, and
     `fathomdb migrate` on a real store from a clean venv without `cargo`).
2. **Minimal schema-37→38 upgrade helper under top-level `fathomdb migrate`**:
   - Introduce the top-level `fathomdb migrate` CLI root (narrowly accepting
     schema `37 -> 38` in `0.8.31`, and generalizing across versions in
     `0.8.34`) rather than overloading `doctor` (read-only) or
     `recover --accept-data-loss` (lossy recovery). Reserve `fathomdb export` /
     `fathomdb import` as sibling roots for `0.8.32`–`0.8.34`.
   - `fathomdb migrate <db-path> --backup-dir <dir>` upgrades a **closed,
     integrity-clean `0.8.30` (schema 37)** store to schema 38:
     1. Acquires the sidecar lock and verifies no active writer/lock holder.
     2. Confirms `PRAGMA user_version == 37` (refuses any other version without
        modifying the file).
     3. **Fail-closed erasure precondition**: inspects
        `_fathomdb_dependency_closures` (`phase != 'complete'`) and
        `_fathomdb_erasure_requests` (active unfinished phases) using immutable
        quiescent read checks; refuses upgrade if any physical erasure is
        incomplete so un-scrubbed bytes are never carried forward.
     4. **Read-only dangling-endpoint preflight**: distinguishes valid
        historical edges (whose target existed and was later superseded) from
        genuinely dangling endpoints written under `0.8.30`; fails closed with
        a typed diagnostic report by default (never silent drop) so upgraded
        stores honor `0.8.31` strict integrity.
     5. Writes and SHA-256-verifies a full pre-upgrade backup (WAL checkpointed
        and copied) before mutating the target store.
     6. Applies schema-38 additive DDL inside a single `BEGIN IMMEDIATE`
        transaction (`properties TEXT` NULL on `canonical_nodes`/`canonical_edges`,
        `scope TEXT NOT NULL DEFAULT ''` across canonical & operational tables,
        `_fathomdb_projection_registry` `entity_class` + `vector_kinds` columns,
        `_fathomdb_vector_kinds` explicit opt-in vector-kind registry table,
        `canonical_property_values` + 4 indexes,
        `_fathomdb_commit_timestamps(write_cursor INTEGER PRIMARY KEY, committed_at_ms INTEGER NOT NULL)`
        and `_fathomdb_commit_timestamps_ms_idx(committed_at_ms, write_cursor)`,
        partial active edge incident indexes (`canonical_edges_from_active_idx`,
        `canonical_edges_to_active_idx`), unified historical edge composite
        indexes (`canonical_edges_from_sup_kind_idx(from_id, superseded_at, kind)`,
        `canonical_edges_to_sup_kind_idx(to_id, superseded_at, kind)`),
        historical node lookup index
        (`canonical_nodes_logical_sup_idx(logical_id, superseded_at, write_cursor)`),
        updated read-visibility triggers via `visibility_coalescing_trigger_sql!`
        for `("cpv", "canonical_property_values")`, and `PRAGMA user_version = 38`),
        preserving existing `vector_default` / `_fathomdb_vector_spans` /
        `_fathomdb_embedder_profiles` verbatim and rolling back atomically on
        any failure.
     7. Runs `doctor full-integrity` verification on the upgraded database and
        fails closed if any finding is reported.
   - **Pre-upgrade `as_of_epoch_ms` semantics on migrated stores**: pre-upgrade
     Schema-37 commits have no historical wall-clock timestamps in
     `_fathomdb_commit_timestamps`; `as_of_write_cursor` addresses all pre- and
     post-upgrade history, while `as_of_epoch_ms` earlier than the first
     post-upgrade Schema-38 commit resolves to cursor `0` (empty).
   - **Shared durable-authority interchange alignment
     (`fathomdb.logical-export/v1`)**: reserve `scope` (default `""`),
     outside-`body` `properties`, `_fathomdb_commit_timestamps`, and declared
     vector-committable kinds in the planned `fathomdb.logical-export/v1`
     manifest model (`0.8.32`–`0.8.34`). For future logical export/rebuild
     paths, never export `embedding_bin` or `mean_vec`; export only uncentered
     f32 vectors, `_fathomdb_vector_spans`, and `EmbedderIdentity`, defaulting
     to re-embedding on import with opt-in `--reuse-vectors` (same
     `EmbedderIdentity` only, recomputing target `mean_vec` + `embedding_bin`
     and verifying open-time equivalence probes).
   - Public `Engine::open` continues to refuse schema-37 stores directly in
     `0.8.31` (preserving `ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md`
     until `0.8.34` ratifies the standing multi-version migration ADR).
   - All existing `0.8.30` rows remain intact (`properties = NULL`,
     `scope = ''`); previously erased rows remain absent.

3. **Slice 50 SDK enhancements (`resolve_evidence_many`, `search(hints=)`, `OpenReport` booleans)**:
   - **`resolve_evidence_many`**: bounded batch evidence resolver under one
     authenticated frozen context returning per-reference resolved artifacts or
     typed per-reference refusals (`get_many` partial-result style), eliminating
     N round trips and N frozen-context validations across Rust (`fathomdb-sdk`),
     Python, and TypeScript (`governed-operation-parity.json`).
   - **`search(hints=)` SDK parity**: expose the pure engine hints subset
     (`no_results`, `degraded`, `more_results`, per `ADR-0.8.29-agent-hints-sidecar.md`)
     opt-in on `search`, `search_frozen`, and `EvidenceSearchRequestV1` across
     Rust, Python, and TypeScript.
   - **`OpenReport` capability booleans**: add `reranker_compiled`,
     `cuda_compiled`, and `rerank_available` booleans to `OpenReport` across
     all three SDKs so consumers inspect compiled/runtime capabilities without
     blind probing.

## Consequences

- `pip install "fathomdb[cli]==0.8.31"` (or `fathomdb-cli==0.8.31`) unlocks
  `fathomdb doctor`, `recompute-mean`, `recover`, and live `0.8.30 -> 0.8.31`
  store upgrades with zero Rust dependency.
- Establishes and qualifies the PyPI CLI wheel pipeline ahead of `0.8.34`'s
  general `fathomdb migrate`.
