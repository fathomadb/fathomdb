---
title: FathomDB 0.8.33 release plan
status: PROPOSED
target_release: 0.8.33
date: 2026-11-21
---

# FathomDB 0.8.33 release plan

> **Status: PROPOSED — awaiting completion of `0.8.31`/`0.8.32` and owner go-ahead.**
> Execution requires an approved `release-state-0.8.33.json`, live board
> `dev/plans/runs/STATUS-0.8.33.md`, and separate publication authority.

## Goal

Ship **safe, crash-resilient schema and data migration** from `0.8.30` (schema
37) and `0.8.31` (schema 38+) databases to `0.8.33`, and provide turnkey
integration adapters, MCP profiles, and verified local-agent workflows for
**Memex, Hermes, OpenClaw**, and similar local personal agents.

## Quality roadmap and standing tripwire check

- **Quality testing roadmap**
  ([fathomdb-quality-testing-roadmap.md](../../notes/fathomdb-quality-testing-roadmap.md)):
  - **Applies in `0.8.33`**: stateful write/supersede/erase/migrate/reopen
    sequences; anomaly/fault injection mid-migration (interrupted transaction,
    disk-full, lock contention); exact post-migration semantic state oracles
    (verifying not just `PRAGMA integrity_check` but exact revisions, spans,
    graph paths, properties, and erased-row absence); resource-leak checks across
    long-lived local-agent open/close/release cycles.
  - **Deferred**: broad ANN/2.x backend qualification.
- **Standing tripwires**
  ([overflow-chunking-reevaluation.md](../../design/overflow-chunking-reevaluation.md)):
  Re-check `T1`–`T3` against real Memex/Hermes/OpenClaw record-length
  distributions collected in `0.8.30`–`0.8.33`; do not enable chunking without
  meeting § 4 latency/CS64 cost analysis.

## Requirements and acceptance signals

| ID | Requirement | Falsifiable acceptance signal |
| --- | --- | --- |
| R33-01 | Ratified migration contract ADR and lint-policy update | Accepted ADR supersedes `ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md`; `scripts/agent-lint-migrations.sh` and `AGENTS.md` enforce tested versioned upgrades while blocking unverified shims. |
| R33-02 | Verified in-place migration from `0.8.30` (schema 37) and `0.8.31` (schema 38+) via lockstep `fathomdb-cli` PyPI wheel | Real closed golden `0.8.30` and `0.8.31` databases migrate via `fathomdb migrate` (tested through the PyPI-installed `fathomdb-cli` wheel with no Rust toolchain) with 100% preservation of canonical nodes/edges, properties, revisions, provenance, dependencies, operational state, and queued erasure; `fathomdb doctor full-integrity` passes; previously erased records remain absent. |
| R33-03 | Crash-resilient and idempotent migration | Injected failure/interruption mid-migration rolls back atomically or recovers cleanly from verified backup; re-running `fathomdb migrate` on an already-current database is a safe no-op; unsupported pre-37 formats fail with a clear typed error without modifying the file. |
| R33-04 | Personal agent integration kits for Memex, Hermes, and OpenClaw | Documented, CI-tested integration adapters/MCP profiles demonstrate full personal-agent lifecycle (ingest, hybrid + graph search, frozen evidence, supersession, checkpoints, scoped user-vs-inference separation, erasure, backup/restore) for Memex, Hermes, and OpenClaw patterns. |
| R33-05 | Live `0.8.30` Memex database upgrade proof | A real `0.8.30` Memex database upgrades cleanly to `0.8.33` via the Python-installed `fathomdb-cli` wheel, passes integrity and Inspector checks, serves identical citations, memories, and graph links after upgrade, and supports Memex's transition to plain-text `body` + outside-`body` typed node properties + caller scope. |
| R33-06 | Personal-agent daemon ergonomics verified | Idle memory (`release_models()`), offline model-cache bootstrap, and concurrent SDK-writer + MCP-reader / Inspector coordination workflows are tested and documented with zero lock leaks or runaway RSS. |

## Slice ladder (proposed)

| Slice | Title | Requirements | Depends on | Parallel / conflict notes |
| --- | --- | --- | --- | --- |
| 0 | Migration ADR, lint-policy update, and golden fixture verification | R33-01 | `0.8.32` closed + golden `.30`/`.31` fixtures verified | Ratifies upgrade contract and backup/rollback invariants before coding. |
| 10 | Schema & data migration engine (`37 -> 38+ -> current`) and `fathomdb migrate` CLI | R33-02, R33-03 | 0 | Implements transactional step-up, backup verification, projection rebuild, and integrity verification in `fathomdb-schema` / `fathomdb-engine` / `fathomdb-cli`. |
| 20 | Migration fault-injection, erasure-preservation, and Inspector compatibility tests | R33-02, R33-03 | 10 | Tests crash/disk-full rollback, queued-erasure preservation, and Inspector inspection of migrated stores. |
| 30 | Personal agent integration adapters and MCP profiles (Memex, Hermes, OpenClaw) | R33-04, R33-06 | 10 | Can run parallel to 20; builds reference connectors, MCP configs, persona scoping templates, and daemon lifecycle docs. |
| 40 | Live Memex `.30 -> .33` upgrade validation and personal-agent E2E suite | R33-05, R33-06 | 20, 30 | Executes end-to-end upgrade on real `.30` Memex DB plus Hermes/OpenClaw integration scenarios. |
| 50 | Full-workspace qualification and authorized `0.8.33` publication | All | 40 + HITL publish approval | Full `./scripts/agent-verify.sh`, 5-target + Orin + MCP + Inspector smokes, updated `docs/operations/` and `docs/guides/`. |

## Open HITL decisions required before Slice 0 starts

1. **Explicit CLI vs. opt-in SDK auto-migrate**: Keep migration CLI-only
   (`fathomdb migrate`) or also expose an explicit `Engine.open(..., migrate=True)`
   opt-in for embedded personal agents like Memex/Hermes/OpenClaw?
2. **Adapter repo placement**: Host Hermes/OpenClaw/Memex reference connectors
   inside `docs/recipes/` + `examples/` in this repo, or publish lightweight
   companion integration packages?
3. **Backward window guarantee**: Confirm whether `0.8.33` establishes a
   standing "migrate from `N-2` / `0.8.30+`" support policy heading toward 1.0.
