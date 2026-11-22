---
title: FathomDB 0.8.34 release plan
status: PROPOSED
target_release: 0.8.34
date: 2026-11-21
---

# FathomDB 0.8.34 release plan

> **Status: PROPOSED — awaiting completion of `0.8.31`–`0.8.33` and owner
> go-ahead.**
> Execution requires an approved `release-state-0.8.34.json`, live board
> `dev/plans/runs/STATUS-0.8.34.md`, and separate publication authority.

## Goal

Ship FathomDB's **full, crash-resilient multi-version schema and data migration
engine (`fathomdb migrate`)** across `0.8.30` (schema 37), `0.8.31`, and
`0.8.32` databases, and deliver turnkey integration adapters, MCP profiles, and
verified local-agent workflows for **Hermes, OpenClaw**, and similar
local personal agents.

## Quality roadmap and standing tripwire check

- **Quality testing roadmap**
  ([fathomdb-quality-testing-roadmap.md](../../notes/fathomdb-quality-testing-roadmap.md)):
  - **Applies in `0.8.34`**: stateful write/supersede/erase/migrate/reopen
    sequences; anomaly/fault injection mid-migration (interrupted transaction,
    disk-full, lock contention); exact post-migration semantic state oracles
    (verifying exact revisions, spans, graph paths, properties, scopes, and
    erased-row absence); resource-leak checks across long-lived local-agent
    open/close/release cycles.
  - **Deferred**: broad ANN/2.x backend qualification.
- **Standing tripwires**
  ([overflow-chunking-reevaluation.md](../../design/overflow-chunking-reevaluation.md)):
  Re-check `T1`–`T3` against real personal-agent record-length
  distributions collected across `0.8.30`–`0.8.34`; do not enable chunking
  without meeting § 4 latency/CS64 cost analysis.

## Requirements and acceptance signals

| ID | Requirement | Falsifiable acceptance signal |
| --- | --- | --- |
| R34-01 | Ratified migration contract ADR and lint-policy update | Accepted ADR supersedes `ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md`; `scripts/agent-lint-migrations.sh` and `AGENTS.md` enforce tested versioned upgrades while blocking unverified shims. |
| R34-02 | Verified in-place migration from `0.8.30` (schema 37), `0.8.31`, and `0.8.32` via lockstep `fathomdb-cli` PyPI wheel | Real closed golden `0.8.30`, `0.8.31`, and `0.8.32` databases migrate via `fathomdb migrate` (tested through the PyPI-installed `fathomdb-cli` wheel with no Rust toolchain) with 100% preservation of canonical nodes/edges, properties, scopes, revisions, provenance, dependencies, operational state, and queued erasure; `fathomdb doctor full-integrity` passes; previously erased records remain absent. |
| R34-03 | Crash-resilient and idempotent migration | Injected failure/interruption mid-migration rolls back atomically or recovers cleanly from verified backup; re-running `fathomdb migrate` on an already-current database is a safe no-op; unsupported pre-37 formats fail with a clear typed error without modifying the file. |
| R34-04 | Personal agent integration kits for Hermes, OpenClaw, and generic local agents | Documented, CI-tested integration adapters and read+write MCP profiles demonstrate full personal-agent lifecycle (plain-text `body` + outside-`body` properties, hybrid + graph search, frozen evidence, supersession, checkpoints, scoped user-vs-inference separation, erasure, backup/restore) for Hermes, OpenClaw, and generic local-agent patterns. |
| R34-05 | Real-world `0.8.30` and `0.8.31` store upgrade proof | Real closed `0.8.30` and `0.8.31` databases upgrade cleanly to `0.8.34` via the Python-installed `fathomdb-cli` wheel, pass integrity and Inspector checks, and serve identical citations, memories, and graph links after upgrade. |
| R34-06 | Personal-agent daemon ergonomics verified | Idle memory (`release_models()`), offline model-cache bootstrap, and concurrent SDK-writer + MCP-reader/writer + Inspector coordination workflows are tested and documented with zero lock leaks or runaway RSS. |

## Slice ladder (proposed)

| Slice | Title | Requirements | Depends on | Parallel / conflict notes |
| --- | --- | --- | --- | --- |
| 0 | Migration ADR, lint-policy update, and golden `.30`/`.31`/`.32` fixture verification | R34-01 | `0.8.33` closed + golden fixtures verified | Ratifies standing upgrade contract and backup/rollback invariants before coding. |
| 10 | General multi-version migration engine (`37 -> 38+ -> current`) and `fathomdb migrate` CLI | R34-02, R34-03 | 0 | Generalizes `0.8.31`'s minimal 37→38 helper into full transactional step-up, backup verification, projection rebuild, and integrity verification in `fathomdb-schema` / `fathomdb-engine` / `fathomdb-cli`. |
| 20 | Migration fault-injection, erasure-preservation, and Inspector compatibility tests | R34-02, R34-03 | 10 | Tests crash/disk-full rollback, queued-erasure preservation, and Inspector inspection of migrated stores. |
| 30 | Personal agent integration adapters and MCP profiles (Hermes, OpenClaw, generic local agents) | R34-04, R34-06 | 10 | Can run parallel to 20; builds reference connectors, MCP configs, persona scoping templates, and daemon lifecycle docs. |
| 40 | End-to-end `.30`/`.31 -> .34` upgrade validation and personal-agent E2E suite | R34-05, R34-06 | 20, 30 | Executes end-to-end upgrades on real archived DBs plus Hermes/OpenClaw/generic integration scenarios. |
| 50 | Full-workspace qualification and authorized `0.8.34` publication | All | 40 + HITL publish approval | Full `./scripts/agent-verify.sh`, 5-target + Orin + `fathomdb-cli` + `fathomdb-mcp` + Inspector smokes, updated `docs/operations/` and `docs/guides/`. |

## Open HITL decisions required before Slice 0 starts

1. **Explicit CLI vs. opt-in SDK auto-migrate**: Keep migration CLI-only
   (`fathomdb migrate`) or also expose an explicit `Engine.open(..., migrate=True)`
   opt-in for embedded personal agents like Hermes/OpenClaw and similar apps?
2. **Adapter repo placement**: Host Hermes/OpenClaw/generic reference connectors
   inside `docs/recipes/` + `examples/` in this repo, or publish lightweight
   companion integration packages?
3. **Backward window guarantee**: Confirm whether `0.8.34` establishes a
   standing "migrate from `0.8.30+`" support policy heading toward 1.0.
