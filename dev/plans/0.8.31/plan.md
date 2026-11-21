---
title: FathomDB 0.8.31 release plan
status: PROPOSED
target_release: 0.8.31
date: 2026-11-21
---

# FathomDB 0.8.31 release plan

> **Status: PROPOSED — paused pending owner go-ahead and open-decision rulings.**
> Execution requires an approved `release-state-0.8.31.json`, live board
> `dev/plans/runs/STATUS-0.8.31.md`, and separate publication authority.

## Goal

Deliver FathomDB **0.8.31** as a production-grade embedded property graph and
evidence database with typed node/edge properties, bitemporal historical graph
reconstruction, strict endpoint integrity, engine-enforced caller scoping, a
production-ready frozen-evidence MCP surface, verified operational durability,
zero latency regression against `0.8.30`, and **Inspector** integrated into the
main repository and release cycle.

## Footprint and runtime posture

- Core storage, property indexing, typed predicate evaluation, historical graph
  traversal, scoping, frozen reads, and recovery remain **in-library,
  deterministic, and CPU-only**.
- Optional embedding and cross-encoder reranking retain the `0.8.30` CPU/CUDA
  12.6 / Orin policy.
- `fathomdb-mcp` remains a standalone Rust stdio binary over `fathomdb-sdk`.
- Inspector remains a standalone desktop/local tool embedding `fathomdb-engine`
  with model-free default builds and opt-in `cached-cpu`.

## Quality roadmap and standing tripwire check

- **Quality testing roadmap**
  ([fathomdb-quality-testing-roadmap.md](../../notes/fathomdb-quality-testing-roadmap.md)):
  - **Applies in `0.8.31`**: anomaly/fault injection at commit, queue, provider,
    and erasure boundaries; resource-leak checks on close/cancel; boundary-value
    property tests on typed predicates, bitemporal edges, and scopes;
    disabled-optimization equivalence tests (comparing indexed property/scope
    fast paths against reference scans); plus [quality-backlog.md](quality-backlog.md)
    (coverage-guided fuzzing and power-loss recovery proof).
  - **Deferred**: global 100% MC/DC, general SQLite SQL/B-tree fuzzing, and
    ANN/2.x backend qualification.
- **Standing tripwires**
  ([overflow-chunking-reevaluation.md](../../design/overflow-chunking-reevaluation.md)):
  No tripwire (`T1`–`T3`) has fired; overflow chunking remains parked unless
  `0.8.30` Memex adoption evidence triggers a separate owner review.

## Requirements and acceptance signals

| ID | Requirement | Falsifiable acceptance signal |
| --- | --- | --- |
| R31-01 | Typed properties on nodes and edges (`SCHEMA_VERSION` 38+) with Rust/Python/TS SDK parity | Real-DB TDD + `proptest`/Hypothesis round-trip tests across all 3 SDKs for string/int/float/bool properties on both nodes and edges; updates create immutable revisions (`_fathomdb_artifact_revisions`) and never mutate in place. |
| R31-02 | Closed typed predicates on edges and target nodes in `graph.expand`, returning edge properties | Traversal tests prove `=`, `!=`, `<`, `<=`, `>`, `>=` on edge and target properties; ineligible edges never enter paths or displace eligible results; `EXPLAIN QUERY PLAN` verifies indexed `SEARCH` (no full `SCAN`). |
| R31-03 | Historical graph reconstruction (`as_of` / revision-aware traversal) | Real-DB tests reconstruct exact paths before and after edge supersession, including divergent validity (`t_valid`/`t_invalid`) and revision timestamps; erased rows are never replayed. |
| R31-04 | Opt-in strict write endpoint integrity | Real-DB tests prove missing or inactive endpoints cause atomic refusal on strict `engine.write` with zero partial state after reopen; default non-strict mode retains explicit `dangling_edge_endpoints` reporting. |
| R31-05 | Engine-enforced caller scope and write authority | Multi-caller fixtures with overlapping data and distinct grants prove zero cross-scope leakage across `search`, `fetch`, `expand`, frozen contexts, continuations, and stale handles after grant revocation; unauthorized/stale writes leave state unchanged and retries remain idempotent. |
| R31-06 | Production `fathomdb-mcp` with scoped frozen-evidence resolution and release packaging | Same evidence tasks run via SDK and MCP verify exact bytes, revision identity, typed drift/erasure/truncation outcomes, and caller scope enforcement; `fathomdb-mcp` builds and passes installed consumer smoke tests in release CI. |
| R31-07 | Erasure and metadata completeness invariants | `sqlite_master` audit test passes (every table classified in canonical/projection, read-visibility triggers, erasure, `doctor`, Inspector); `purge` and `erase_source` (Autonomous + ExplicitMaintenance) leave zero property/scope rows, zero WAL/file remnants, and valid closure proofs. |
| R31-08 | Zero latency regression vs. `0.8.30` | Paired `v0.8.30` vs. `0.8.31` benchmark (10k / 50k / 100k) via `experiments/sdk_benchmark.py` shows no statistically significant p50/p95 regression on unpredicated open, write, streamed FTS, optimized hybrid, and current-graph expansion; `EXPLAIN QUERY PLAN` checks pass. |
| R31-09 | Operational durability: backup/restore/rebuild, power-loss proof, and coverage-guided fuzzing | Round-trip backup/restore and rebuild match revisions, evidence spans, and graph outputs; power-loss fault campaign passes under declared durability model; instrumented coverage-guided fuzzing runs within declared budgets with minimized regressions committed. |
| R31-10 | Inspector merged into `main`, upgraded to `0.8.31`, and wired into release cycle | `src/inspector/` compiles against `0.8.31` Engine (model-free + `cached-cpu`), passes retrieval/ownership/property/history/scope/erasure qualification on real `.30` and `.31` fixtures, and produces qualified release artifacts in the release workflow. |

## Cross-cutting Definition of Done (every feature slice)

- **X1 (SDK parity)**: Rust (`fathomdb-sdk`), Python, TypeScript, and
  `src/conformance/governed-operation-parity.json` stay in lockstep.
- **X2 (Erasure & metadata audit)**: `sqlite_master` table-classification test,
  read-visibility triggers, `purge`/`erase_source`, and `doctor` checks pass.
- **X3 (Performance guard)**: `EXPLAIN QUERY PLAN` checks and scoped latency
  spot-checks pass before slice close; full paired gate runs in Slice 80.
- **X4 (Docs & interfaces)**: `dev/interfaces/{rust,rust-sdk,python,typescript,cli,mcp,wire}.md`
  and `docs/` updated in the same slice as any contract change.

## Slice ladder (proposed)

| Slice | Title | Requirements | Depends on | Parallel / conflict notes |
| --- | --- | --- | --- | --- |
| 0 | Preflight, ADRs, and baseline `0.8.30` latency capture | All | `0.8.30` closed + open decisions ruled | Writes ADRs for property schema, historical graph, caller scope, MCP graduation, and strict-write contract; captures `v0.8.30` baseline numbers. |
| 10 | Schema 38+, node/edge typed properties, and metadata/erasure audit | R31-01, R31-07 | 0 | Keystone for schema, `fathomdb-schema`, `ROW_OWNED_PROJECTIONS`, triggers, `physical_erasure`, and `sqlite_master` audit. |
| 20 | Typed property predicates in `graph.expand` and strict endpoint integrity | R31-02, R31-04, R31-07 | 10 | Touches `graph_expand/`, `filter.rs`, `write_validation.rs`, `write_commit.rs`, SDK bindings. |
| 30 | Historical graph reconstruction (`as_of` / revision-aware traversal) | R31-03, R31-07 | 10 | Can run parallel to 20 after shared `graph_expand/types.rs` contract is frozen; serialize merge on `graph_expand/execution.rs`. |
| 40 | Engine-enforced caller scope and write authority | R31-05, R31-07 | 10 | Touches `read.rs`, `search.rs`, `frozen_read.rs`, `graph_expand/`, `write.rs`, `actuation.rs`, SDKs. |
| 50 | Production `fathomdb-mcp` (scoped frozen evidence + release packaging) | R31-06, R31-07 | 20, 30, 40 | Touches `fathomdb-mcp` and release packaging workflow. |
| 60 | Inspector merge, `0.8.31` Engine/UI upgrade, and release-cycle wiring | R31-10 | 20, 30, 40 | Merges `inspector/dev/0.2.0` onto `main`; adds property, history, and scope UI views + release CI integration. |
| 70 | Operational durability: backup/restore/rebuild, power-loss proof, and coverage-guided fuzzing | R31-09 | 50 | Executes `quality-backlog.md` and External Reviewer P0 durability proofs on real stores. |
| 80 | Full-workspace qualification and paired `0.8.30` vs. `0.8.31` latency gate | R31-08, All | 60, 70 | Full `./scripts/agent-verify.sh`, paired latency/RSS gate, 5-target + Orin + Inspector candidate qualification; archives closed golden `0.8.31` fixture DB for `0.8.33`. |
| 90 | Authorized `0.8.31` publication (crates, PyPI, npm, Tegra, MCP, Inspector) | All | 80 + explicit HITL publish approval | Separate publish authorization; post-publish installed consumer smokes across all targets. |

## Behavior-change register (expected)

- **Schema version bump (`37 -> 38+`)**: non-empty schema-37 stores remain
  refused at open until `0.8.33` migration lands (documented clearly in release
  notes and compatibility docs).
- **Additive SDK & MCP surface**: node/edge properties, edge/target traversal
  predicates, historical graph view mode, opt-in strict endpoint writes, caller
  scope parameters, and MCP frozen-evidence tools.
- **No silent behavior change** on unpredicated, unscoped, current-time queries
  or default write receipts.

## Open HITL decisions required before Slice 0 starts

1. **CUDA 13 disposition**: Confirm deferring
   [cuda-delivery-plan.md](cuda-delivery-plan.md) out of `0.8.31`.
2. **MCP write surface**: Keep `0.8.31` MCP scoped + frozen-evidence read-only
   (writes/checkpoints via SDK), or expose governed/idempotent writes over MCP
   in `0.8.31`?
3. **Default write integrity**: Keep strict endpoint integrity opt-in per
   request/store (recommended for backward compatibility with bulk imports), or
   make strict the default?
4. **Property schema shape**: Confirm inline canonical JSON properties +
   declared typed index table (`text_val`/`num_val`/`bool_val`) vs. indexing all
   properties automatically (tradeoff: write amplification & DB size vs. ad-hoc
   predicate speed).
5. **Memex `.30 -> .31` bridge**: Confirm Memex stays on `0.8.30` (or rebuilds)
   until `0.8.33` migration ships.
