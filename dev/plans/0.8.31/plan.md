---
title: FathomDB 0.8.31 release plan
status: ACTIVE
target_release: 0.8.31
date: 2026-11-21
---

# FathomDB 0.8.31 release plan

> **Status: ACTIVE on `release/0.8.31` (owner go-ahead 2026-11-22).**
> Live state lives in [release-state-0.8.31.json](../release-state-0.8.31.json)
> and [STATUS-0.8.31.md](../runs/STATUS-0.8.31.md). Slice 70 publication
> remains separately HITL-gated.

## Goal and scope

Deliver FathomDB **0.8.31**: outside-`body` inline typed properties on **both
nodes and edges** with declared typed indexes (Option 1), app-declared
vector-committable kinds, strict-only write endpoint integrity, bitemporal
historical graph reconstruction, opt-in fused-hit graph seeding with seed
provenance, Memex SDK/CLI unblockers (`resolve_evidence_many`, `search(hints=)`,
`OpenReport` booleans, op-collection readback, W12 erasure fixes, lockstep
`fathomdb-cli` PyPI wheel, minimal schema-37→38 upgrade helper), zero latency
regression against `0.8.30`, and **Inspector** integrated into `main` and the
release cycle. Full scope boundaries live in [scope.md](scope.md).

## Settled HITL decisions (2026-11-21)

| Decision | Ruling | Consequence |
| --- | --- | --- |
| Release split | Split into `0.8.31` (Property Graph + Strict Writes + History + Memex + Inspector) and [0.8.32](../0.8.32/plan.md) (Caller Scope + Read/Write MCP + Bulk Opt-In + Hardened Durability); shift Docs → `0.8.33`, Migration + Agents → `0.8.34`, Benchmarks → `0.8.35`. | Keeps `0.8.31` tight and unblocks Memex + Inspector first; reserves `scope` column in schema 38 so `0.8.32` avoids a breaking schema bump. |
| Property indexing (A4) | **Option 1**: inline canonical typed JSON `properties` (outside `body`, ad-hoc predicates on `graph.expand`/`use_graph_arm`) + declared typed B-tree/`vec0` indexes for corpus-wide `SearchFilter`. | O(1) hydration, clean plain-text `body` (eliminating Memex R2), zero B-tree bloat on hashes/UUIDs, competitive write/read latency. |
| Write integrity (A3) | **Strict-only** endpoint integrity in `0.8.31`; opt-in bulk/non-strict import lands in `0.8.32`. | Slice 0 ADR formalizes strict-only contract and updates legacy dangling-edge tests. |
| Memex upgrade bridge (A5) | Include a **minimal schema-37→38 export/upgrade helper** in `0.8.31`. | Unblocks live `0.8.30` Memex upgrades ahead of full `0.8.34` `fathomdb migrate`. |
| CUDA 13 (A1) | **Defer** [cuda-delivery-plan.md](cuda-delivery-plan.md) out of `0.8.31`. | Retains qualified `0.8.30` CUDA 12.6 + Orin stack; CUDA 13 re-slotted post-`0.8.35` or dedicated release. |
| Memex Graph Asks 2 & 3 | **Include** opt-in fused-hit graph seeding + per-hit seed provenance in `0.8.31`. | Dense/reranked hits seed body-less property-graph edges with auditable seed/depth provenance. |
| Memex R2 & E3 | **Skip R2** (superseded by Option 1); **defer E3** (`ingest_with_extractor` post-`0.8.34`, documented legacy in `0.8.33`). | Reduces engine churn and keeps focus on governed `actuate` + property graph. |
| Inspector `0.2.1` handoff | Integrate completed `origin/inspector/dev/0.2.1` (`d6501cc5ac9b4fd1747be28f1896f7f7670ff39a`, already verified on `0.8.30`) plus 4 retrospective CI/bootstrap/harness improvements. | Eliminates `.28 -> .30` port risk; speeds up Inspector CI (~5 min scoped default), fixes Ubuntu 24.04 worktree Python/TS bootstrap, adds harness timeouts, and locks in Docker-free native qualification (`D4 = B`). |

## Footprint and runtime posture

- Core storage, inline property storage, declared typed indexing, predicate
  evaluation, historical graph traversal, strict write validation, frozen reads,
  and recovery remain **in-library, deterministic, and CPU-only**.
- Optional embedding and cross-encoder reranking retain the `0.8.30` CPU / CUDA
  12.6 / Orin policy.
- `fathomdb-cli` ships as a lockstep Rust binary and PyPI binary wheel.
- Inspector remains a standalone local tool embedding `fathomdb-engine` with
  model-free default builds and opt-in `cached-cpu`.

## Quality roadmap and standing tripwire check

- **Quality testing roadmap**
  ([fathomdb-quality-testing-roadmap.md](../../notes/fathomdb-quality-testing-roadmap.md)):
  - **Applies in `0.8.31`**: anomaly/fault injection at commit, property-index,
    upgrade-helper, and erasure boundaries; resource-leak checks on close/cancel;
    boundary-value property tests on typed predicates and bitemporal edges;
    disabled-optimization equivalence tests (comparing declared-index fast paths
    against reference scans).
  - **Re-slotted to `0.8.32`**: [quality-backlog.md](quality-backlog.md)
    (power-loss recovery proof and broad coverage-guided fuzzing) moves cleanly
    to [0.8.32](../0.8.32/plan.md) alongside caller scope and read+write MCP.
  - **Deferred**: global 100% MC/DC, general SQLite SQL/B-tree fuzzing, and
    ANN/2.x backend qualification.
- **Standing tripwires**
  ([overflow-chunking-reevaluation.md](../../design/overflow-chunking-reevaluation.md)):
  No tripwire (`T1`–`T3`) has fired; overflow chunking remains parked unless
  `0.8.30` Memex adoption evidence triggers a separate owner review.

## Requirements and acceptance signals

| ID | Requirement | Falsifiable acceptance signal |
| --- | --- | --- |
| R31-01 | Outside-`body` inline typed node/edge properties + declared typed B-tree/`vec0` indexes (`SCHEMA_VERSION` 38+, reserved `scope` column), app-declared vector kinds (R1), and op-collection readback (R12) with Rust/Python/TS parity | Real-DB TDD + `proptest`/Hypothesis round-trip tests across all 3 SDKs for string/int/float/bool properties outside `body`; declared properties pre-filter cleanly in `SearchFilter` (undeclared rejected cleanly); custom app-declared kinds vector-index; op-collection kind readback/mismatch passes; property updates create immutable revisions (`_fathomdb_artifact_revisions`). |
| R31-02 | Ad-hoc + declared typed predicates on `graph.expand` and `use_graph_arm`, opt-in fused-hit seeding + seed provenance (Memex Graph Asks 2, 3, 5), and mixed-edge seed-slot fix | Traversal tests prove `=`, `!=`, `<`, `<=`, `>`, `>=` on both undeclared (inline) and declared edge/target properties; ineligible edges never enter paths, displace eligible results, or consume `SEED_FTS_N` slots; opt-in `graph_seed="fused_top_k"\|"graph_fts"\|"both"` seeds body-less edges from fused/reranked hits with per-hit seed provenance + hop depth; `EXPLAIN QUERY PLAN` verifies indexed `SEARCH` (no full `SCAN`). |
| R31-03 | Historical graph reconstruction (`as_of` / revision-aware traversal) | Real-DB tests reconstruct exact paths before and after edge supersession, including divergent validity (`t_valid`/`t_invalid`) and revision timestamps; purged/erased rows are never replayed. |
| R31-04 | Strict-only write endpoint integrity | Real-DB tests prove every `engine.write` / `actuate` call referencing a missing or inactive endpoint causes atomic refusal with zero partial state after reopen; zero dangling edges exist in `0.8.31` stores. |
| R31-05 | Memex SDK/CLI unblockers, minimal 37→38 upgrade helper, lockstep `fathomdb-cli` PyPI wheel, and doc fixes | `resolve_evidence_many`, `search(hints=)`, and `OpenReport` booleans (`reranker_compiled`, `cuda_compiled`, `rerank_available`) pass across all 3 SDKs; `fathomdb-cli` PyPI wheel installs and runs `fathomdb doctor` + minimal 37→38 upgrade on a real closed `0.8.30` DB with no Rust toolchain; `docs/reference/python-api.md` and `docs/install/python.md` fixes verified. |
| R31-06 | Erasure, W12 ergonomics fixes (Memex Ask 3), and metadata completeness invariants | `sqlite_master` audit test passes (every table classified in canonical/projection, read-visibility triggers, erasure, `doctor`, Inspector); `purge` and `erase_source` (Autonomous + ExplicitMaintenance) leave zero property rows, zero WAL/file remnants, and valid closure proofs; completing-retry `EraseReport` is distinguishable from already-erased and BUSY checkpoint diagnostics are accurate. |
| R31-07 | Zero latency regression vs. `0.8.30` | Paired `v0.8.30` vs. `0.8.31` benchmark (10k / 50k / 100k) via `experiments/sdk_benchmark.py` shows no statistically significant p50/p95 regression on unpredicated open, write, streamed FTS, optimized hybrid, and current-graph expansion; `EXPLAIN QUERY PLAN` checks pass. |
| R31-08 | Inspector `0.2.1` integrated onto `main`, upgraded to `0.8.31`, wired into release cycle, and CI/bootstrap/harness hardened | `origin/inspector/dev/0.2.1` (`d6501cc5a`) is integrated; `src/inspector/` compiles against `0.8.31` Engine (model-free + `cached-cpu`) and passes property/history/erasure qualification under the retained Docker-free native Ubuntu 24.04 pipeline (`D4 = B`); `local-ci.sh` defaults to an Inspector-scoped gate (~5 min) when shared core paths are unchanged (retaining `--full-workspace`); `bootstrap.sh`/`setup-python-owned.sh` prefer system `/usr/bin/python3.12` on Ubuntu 24.04 and provision `src/ts/node_modules`; process/GUI/`strace` harnesses enforce explicit per-command timeouts; Inspector release artifacts build and smoke-pass in release CI. |

## Cross-cutting DoD (every feature slice)

- **X1 (SDK parity)**: Rust (`fathomdb-sdk`), Python, TypeScript, and
  `src/conformance/governed-operation-parity.json` stay in lockstep.
- **X2 (Erasure & metadata audit)**: `sqlite_master` table-classification test,
  read-visibility triggers, `purge`/`erase_source`, and `doctor` checks pass.
- **X3 (Performance guard)**: `EXPLAIN QUERY PLAN` checks and scoped latency
  spot-checks pass before slice close; full paired gate runs in Slice 60.
- **X4 (Docs & interfaces)**: `dev/interfaces/{rust,rust-sdk,python,typescript,cli,wire}.md`
  and `docs/` updated in the same slice as any contract change.

## Slice ladder (proposed)

| Slice | Title | Requirements | Depends on | Parallel / conflict notes |
| --- | --- | --- | --- | --- |
| 0 | Preflight, integrate `origin/inspector/dev/0.2.1` (`d6501cc5a`), fix worktree Python/TS bootstrap, write ADRs, W12 fence review, and capture `0.8.30` baseline latency | R31-08 (bootstrap/merge), All | `0.8.30` closed + owner go-ahead | Merges/integrates `origin/inspector/dev/0.2.1`; updates `scripts/bootstrap.sh` / `scripts/setup-python-owned.sh` (Ubuntu 24.04 `/usr/bin/python3.12` preference + `src/ts/node_modules`); writes ADRs for Option-1 property schema (with reserved `scope` column), app-declared vector kinds, strict-only endpoint integrity, historical graph, minimal 37→38 upgrade helper, and `fathomdb-cli` PyPI wheel; evaluates W12 checkpoint-only fence safety; captures `v0.8.30` baseline numbers. |
| 10 | Schema 38+, outside-`body` inline properties + declared typed indexes, app-declared vector kinds, op-collection readback, W12 fixes, and `sqlite_master` audit | R31-01, R31-06 | 0 | Keystone for `fathomdb-schema`, `SearchFilter` property predicates, `ROW_OWNED_PROJECTIONS`, triggers, `physical_erasure`, W12 ergonomics fixes, and `sqlite_master` completeness audit. |
| 20 | Strict-only endpoint integrity, edge/target predicates, opt-in fused-hit seeding + seed provenance, and mixed-edge slot fix | R31-02, R31-04, R31-06 | 10 | Touches `write_validation.rs`, `write_commit.rs`, `graph_expand/`, `search.rs`, `filter.rs`, SDK bindings; updates legacy dangling-edge tests per Slice 0 ADR. |
| 30 | Historical graph reconstruction (`as_of` / revision-aware traversal) | R31-03, R31-06 | 10 | Can run parallel to 20 after shared `graph_expand/types.rs` contract is frozen; serialize merge on `graph_expand/execution.rs`. |
| 40 | SDK unblockers (`resolve_evidence_many`, `search(hints=)`, `OpenReport` booleans), minimal 37→38 upgrade helper, `fathomdb-cli` PyPI wheel, and doc fixes | R31-05, R31-06 | 20, 30 | Touches SDK bindings, `fathomdb-cli` upgrade helper + PyPI Maturin bin packaging, and Memex doc fixes. |
| 50 | Inspector `0.8.31` Engine + property/history UI upgrade, scoped `local-ci.sh`, harness timeouts, and release-cycle wiring | R31-08 | 20, 30 | Can overlap with 40; builds on Slice 0's integrated `inspector/dev/0.2.1` baseline, adds property and historical graph UI views, decouples `local-ci.sh` (keeping `--full-workspace`), adds per-command harness timeouts, retains Docker-free native qualification (`D4 = B`), and integrates Inspector into release CI. |
| 60 | Full-workspace qualification and paired `0.8.30` vs. `0.8.31` latency gate | R31-07, All | 40, 50 | Full `./scripts/agent-verify.sh`, paired latency/RSS gate, 5-target + Orin + `fathomdb-cli` PyPI wheel + Inspector candidate qualification; archives closed golden `0.8.31` fixture DB for `0.8.34`. |
| 70 | Authorized `0.8.31` publication (crates, PyPI `fathomdb` + `fathomdb-cli`, npm, Tegra, Inspector) | All | 60 + explicit HITL publish approval | Separate publish authorization; post-publish installed consumer smokes across all targets. |

## Behavior-change register (expected)

- **Breaking (pre-1.0) — schema version bump (`37 -> 38+`)**: non-empty
  schema-37 stores are refused on direct open; use the `0.8.31` minimal 37→38
  upgrade helper (or full `0.8.34` `fathomdb migrate` later).
- **Breaking (pre-1.0) — strict-only write endpoint integrity**: edges with
  missing or inactive endpoints are atomically refused on `engine.write` (opt-in
  bulk/non-strict import arrives in `0.8.32`).
- **Additive SDK & CLI surface**: outside-`body` typed node/edge properties
  (inline JSON + declared typed indexes for `SearchFilter`), app-declared
  vector-committable kinds, ad-hoc & declared edge/target predicates, opt-in
  fused-hit graph seeding (`graph_seed`) with seed/depth provenance,
  `use_graph_arm` edge filters, historical graph `as_of` mode, batch
  `resolve_evidence_many`, `search(hints=)`, `OpenReport` capability booleans,
  op-collection kind readback, minimal 37→38 upgrade helper, and lockstep
  `fathomdb-cli` PyPI binary wheels.
- **Targeted defect/ergonomics fixes**: mixed-edge `SEED_FTS_N` seed-slot fix
  and W12 erasure ergonomics.
- **No silent behavior change** on unpredicated, current-time queries.

## Reserved-gap policy and deferred work

Slice numbers advance in steps of 10 (`0, 10, 20, 30, 40, 50, 60, 70`), leaving
gaps reserved for necessary coherent follow-ups; scope overflow halts for owner
disposition. Explicitly deferred out of `0.8.31`: caller scope, read+write MCP,
bulk write opt-in, and `quality-backlog.md` (moved to [0.8.32](../0.8.32/plan.md));
CUDA 13 (`cuda-delivery-plan.md`, deferred post-`0.8.35`); Memex E3
(`ingest_with_extractor`, deferred post-`0.8.34`).

## Immediate next slice

Active slice is owned by [release-state-0.8.31.json](../release-state-0.8.31.json)
and [STATUS-0.8.31.md](../runs/STATUS-0.8.31.md). Slice prompts live under
`dev/plans/0.8.31/features/slice-<N>/prompt.md`.

## Open HITL decisions

None remaining for scoping. Publication (Slice 70) requires separate HITL
authorization after Slice 60 qualification completes.

## Landed slice roll-up

<!-- BEGIN GENERATED release-state:0.8.31:plan-landed-roll-up -->
**LANDED on `origin/main`, in full:** no slices. SCHEMA is 37; remaining ladder = 0 → 10 → 20 → 30 → 40 → 50 → 60 → 70.<!-- END GENERATED release-state:0.8.31:plan-landed-roll-up -->
