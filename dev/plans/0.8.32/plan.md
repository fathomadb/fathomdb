---
title: FathomDB 0.8.32 release plan
status: PROPOSED
target_release: 0.8.32
date: 2026-11-21
---

# FathomDB 0.8.32 release plan

> **Status: PROPOSED — awaiting completion of `0.8.31` and owner go-ahead.**
> Execution requires an approved `release-state-0.8.32.json`, live board
> `dev/plans/runs/STATUS-0.8.32.md`, and separate publication authority.

## Goal

Deliver FathomDB **0.8.32**: engine-enforced **caller scope and write
authority** (using the `scope` column reserved in `0.8.31` schema 38),
production **read+write `fathomdb-mcp`** (scoped frozen evidence + governed
idempotent writes and checkpoints + release binaries), **opt-in bulk /
non-strict write mode**, and hardened durability (**backup/restore/rebuild,
power-loss recovery, and coverage-guided fuzzing**) with zero latency
regression on single-scope workloads.

## Footprint and runtime posture

- Scope enforcement, bulk-import validation, frozen reads, and recovery remain
  **in-library, deterministic, and CPU-only**.
- `fathomdb-mcp` ships as a standalone Rust stdio binary over `fathomdb-sdk`
  with explicit scope configuration and bounded admission/token budgets.
- Power-loss fault injection and coverage-guided fuzzing run in dedicated
  disposable test environments and never alter runtime shipping dependencies.

## Quality roadmap and standing tripwire check

- **Quality testing roadmap**
  ([fathomdb-quality-testing-roadmap.md](../../notes/fathomdb-quality-testing-roadmap.md)):
  - **Applies in `0.8.32`**: directly executes
    [0.8.31/quality-backlog.md](../0.8.31/quality-backlog.md) (broad
    coverage-guided fuzzing + power-loss/torn-write recovery proof under a
    declared durability model), plus compound multi-caller authorization,
    idempotent write-retry, and resource-leak checks across MCP and SDK paths.
  - **Deferred**: global 100% MC/DC, general SQLite SQL/B-tree fuzzing, and
    ANN/2.x backend qualification.
- **Standing tripwires**
  ([overflow-chunking-reevaluation.md](../../design/overflow-chunking-reevaluation.md)):
  No tripwire (`T1`–`T3`) touched; re-check before Slice 0 against any new
  `0.8.31` field-adoption evidence.

## Requirements and acceptance signals

| ID | Requirement | Falsifiable acceptance signal |
| --- | --- | --- |
| R32-01 | Engine-enforced caller scope and write authority (no breaking schema bump over `0.8.31`) | Two-caller fixtures with overlapping records and distinct grants prove zero cross-scope leakage across `search` (including `use_graph_arm`), `fetch`, `expand`, frozen contexts, continuations, and stale refs after grant revocation; unauthorized/stale writes leave state unchanged and retries remain idempotent; existing `0.8.31` stores open cleanly without schema break. |
| R32-02 | Production read+write `fathomdb-mcp` with scoped frozen evidence, governed writes/checkpoints, and release packaging | Identical tasks run through SDK and MCP prove exact bytes, revision identity, typed drift/erasure/truncation outcomes, caller scope enforcement, and idempotent governed writes/`latest_state` checkpoints (`operation_id` replay + `expected_write_boundary`); `fathomdb-mcp` release binaries pass installed consumer smoke tests. |
| R32-03 | Opt-in bulk / non-strict write mode (default remains strict) | Explicit bulk-write opt-in admits out-of-order edges with accurate `dangling_edge_endpoints` counts and post-import `fathomdb doctor` verification; default `engine.write` remains strict and atomically refuses missing/inactive endpoints. |
| R32-04 | Backup, restore, and full rebuild proof (External Reviewer P0) | On pinned installed artifacts, backup/restore and full rebuild from authoritative corpus match revisions, evidence spans, properties, scopes, and graph results identically. |
| R32-05 | Power-loss recovery proof and coverage-guided fuzzing (`quality-backlog.md`) | Declared durability/fault model executes real dropped/torn/reordered-write interruptions across commit, WAL/checkpoint, and recovery with exact semantic + SQLite integrity verification; instrumented coverage-guided fuzzing completes declared budgets across query/predicate/scope/cursor/MCP/recovery targets with minimized regressions committed. |
| R32-06 | Erasure, metadata audit, Inspector scope view, and zero latency regression | `sqlite_master` completeness audit, scope-aware `purge`/`erase_source`, and Inspector scope UI tests pass; paired `v0.8.31` vs. `0.8.32` benchmark confirms zero statistically significant p50/p95 regression on single-scope workloads. |

## Slice ladder (proposed)

| Slice | Title | Requirements | Depends on | Parallel / conflict notes |
| --- | --- | --- | --- | --- |
| 0 | Preflight, ADRs (caller scope, MCP write/evidence contract, bulk opt-in, durability fault model), and `0.8.31` baseline capture | All | `0.8.31` published | Freezes scope token/policy grammar, MCP tool schemas, bulk opt-in API, and disposable power-loss/fuzzing venue contracts. |
| 10 | Engine-enforced caller scope and write authority (SDK parity across Rust/Python/TS) | R32-01, R32-06 | 0 | Keystone activating schema-38 reserved `scope` column across read, search, graph, frozen, write, actuate, and erasure paths with single-scope fast path. |
| 20 | Opt-in bulk / non-strict write mode and Inspector scope UI update | R32-03, R32-06 | 10 | Touches `write_validation.rs`, `write_commit.rs`, SDK bindings, and `src/inspector/`. |
| 30 | Production read+write `fathomdb-mcp` (scoped frozen evidence, governed writes/checkpoints, release packaging) | R32-02, R32-06 | 10 | Can run parallel to 20; touches `fathomdb-mcp`, `dev/interfaces/mcp.md`, and release binary workflows. |
| 40 | Backup/restore/rebuild proof, power-loss recovery campaign, and coverage-guided fuzzing | R32-04, R32-05 | 20, 30 | Executes `quality-backlog.md` and External Reviewer P0 durability proofs on real stores and disposable fault environment. |
| 50 | Full-workspace qualification and paired `0.8.31` vs. `0.8.32` latency gate | R32-06, All | 40 | Full `./scripts/agent-verify.sh`, paired latency/RSS gate, 5-target + Orin + `fathomdb-cli` + `fathomdb-mcp` + Inspector candidate qualification; archives golden `0.8.32` fixture DB for `0.8.34`. |
| 60 | Authorized `0.8.32` publication | All | 50 + explicit HITL publish approval | Separate publish authorization; post-publish installed consumer smokes across all targets. |

## Open HITL decisions required before Slice 0 starts

1. **MCP write capability gating**: Confirm whether MCP writes/checkpoints are
   disabled by default in `fathomdb-mcp` config (requiring explicit opt-in
   `allow_writes: true` per store + caller scope policy).
2. **Power-loss fault venue**: Confirm the disposable local VM / block-device
   fault seam (`dm-flakey` / `dm-dust` or equivalent) for the Slice 40
   power-loss campaign.
