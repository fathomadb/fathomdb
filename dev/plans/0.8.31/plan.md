---
title: FathomDB 0.8.31 release plan
status: COMPLETE
target_release: 0.8.31
date: 2026-11-23
---

# FathomDB 0.8.31 release plan

> **Status: COMPLETE — `v0.8.31` published 2026-11-23.**
> Final state lives in [release-state-0.8.31.json](../release-state-0.8.31.json),
> [STATUS-0.8.31.md](../runs/STATUS-0.8.31.md), and
> [features/slice-80/status.md](features/slice-80/status.md).

## Goal and scope

Deliver FathomDB **0.8.31**: outside-`body` inline typed properties on **both
nodes and edges** with declared typed indexes (Option 1), app-declared
vector-committable kinds, strict-only write endpoint integrity, bitemporal
historical graph reconstruction (`as_of_write_cursor` and `as_of_epoch_ms`),
opt-in fused-hit graph seeding with seed provenance, SDK/CLI adoption
enhancements (`resolve_evidence_many`, `search(hints=)`, `OpenReport`
capability booleans, op-collection readback, W12 erasure ergonomics fixes,
lockstep `fathomdb-cli` PyPI wheel, minimal schema-37→38 `fathomdb migrate`
helper), quantified zero latency regression against `0.8.30`, and **Inspector**
integrated into `main` and the release cycle. Full scope boundaries live in
[scope.md](scope.md).

## Settled HITL decisions (2026-11-21 & 2026-11-22)

| Decision | Ruling | Consequence |
| --- | --- | --- |
| Release split | Split into `0.8.31` (Property Graph + Strict Writes + History + SDK/CLI Upgrade + Inspector) and [0.8.32](../0.8.32/plan.md) (Caller Scope + Read/Write MCP + Bulk Opt-In + Hardened Durability); shift Docs → `0.8.33`, Migration + Agent Connectors → `0.8.34`, Benchmarks → `0.8.35`. | Keeps `0.8.31` tight and unblocks property-graph + Inspector adoption first; reserves `scope` column in schema 38 so `0.8.32` avoids a breaking schema bump. |
| Property indexing (A4) | **Option 1**: inline canonical typed JSON `properties` (outside `body`, ad-hoc predicates on `graph.expand`/`use_graph_arm`) + declared typed B-tree/`vec0` indexes for corpus-wide `SearchFilter`. | O(1) hydration, clean plain-text `body` (eliminating projected-text hybrid indexing complexity), zero B-tree bloat on hashes/UUIDs, competitive write/read latency. |
| Write integrity (A3) | **Strict-only** endpoint integrity in `0.8.31`; opt-in bulk/non-strict import lands in `0.8.32`. | Slice 0 ADR formalizes strict-only contract and updates legacy dangling-edge tests. |
| Schema 37→38 upgrade bridge (A5) | Include a **minimal schema-37→38 `fathomdb migrate` helper** in `0.8.31`. | Unblocks live `0.8.30` store upgrades ahead of full multi-version migration in `0.8.34`. |
| CUDA 13 (A1) | **Defer** [cuda-delivery-plan.md](cuda-delivery-plan.md) out of `0.8.31`. | Retains qualified `0.8.30` CUDA 12.6 + Orin stack; CUDA 13 re-slotted post-`0.8.35` or dedicated release. |
| Opt-in fused-hit seeding & provenance | **Include** opt-in fused-hit graph seeding (`graph_seed`) + per-hit seed provenance in `0.8.31`. | Dense/reranked hits seed body-less property-graph edges with auditable seed/depth provenance. |
| Projected-text hybrid & `ingest_with_extractor` | **Skip** projected-text hybrid indexing (superseded by Option 1 outside-`body` properties); **defer** `ingest_with_extractor` rework post-`0.8.34` (documented as legacy vs. `actuate` in `0.8.33`). | Reduces engine churn and keeps focus on governed `actuate` + property graph. |
| Inspector `0.2.1` handoff | Integrate completed `origin/inspector/dev/0.2.1` (`d6501cc5ac9b4fd1747be28f1896f7f7670ff39a`, already verified on `0.8.30`) plus 4 retrospective CI/bootstrap/harness improvements. | Eliminates `.28 -> .30` port risk; speeds up Inspector CI (~5 min scoped default), fixes Ubuntu 24.04 worktree Python/TS bootstrap, adds harness timeouts, and locks in Docker-free native qualification (`D4 = B`). |
| HITL Gate 1 (`2026-11-22`) — Schema 38 commit timestamps (`P4-09`), ladder, triage & project boundary | Add `_fathomdb_commit_timestamps(write_cursor INTEGER PRIMARY KEY, committed_at_ms INTEGER NOT NULL)` + `(committed_at_ms, write_cursor)` index in Schema 38; renumber ladder to `Slice 10 = Prework Prep`, feature slices `20–80`; approve `slice-9-triage.md`; pre-authorize `governed-surface-pin.json` updates for `read.operational_collections`/`read.operational_collection` (Slice 20) & `resolve_evidence_many` (Slice 50); enforce strict project-boundary rule (never write external consumer-project specifics into tracked FathomDB repo/public files). | Enables both `as_of_write_cursor` and `as_of_epoch_ms` with negligible per-batch write cost (~1–5 µs) and zero overhead on default reads; aligns ladder with `RELEASE-AND-SLICE-PLANNING-TEMPLATE.md`; keeps tracked FathomDB files strictly product-generic. |

## Footprint and runtime posture

- Core storage, inline property storage, declared typed indexing, predicate
  evaluation, historical graph traversal, strict write validation, frozen reads,
  and recovery remain **in-library, deterministic, and CPU-only**.
- Optional embedding and cross-encoder reranking retain the `0.8.30` CPU / CUDA
  12.6 / Orin policy.
- `fathomdb-cli` ships as a lockstep Rust binary and PyPI binary wheel (`--bin fathomdb`).
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
  post-`0.8.30` field adoption evidence triggers a separate owner review.

## Requirements and acceptance signals

| ID | Requirement | Falsifiable acceptance signal |
| --- | --- | --- |
| R31-01 | Outside-`body` inline typed node/edge properties + declared typed B-tree/`vec0` indexes (`SCHEMA_VERSION` 38, reserved `scope` column, `_fathomdb_commit_timestamps`), explicit opt-in app-declared vector kinds (rejecting reserved `"edge_fact"`), and op-collection readback/refusal with Rust/Python/TS parity | Real-DB TDD + `proptest`/Hypothesis round-trip tests across all 3 SDKs for `String`/`Int` (including `i64::MIN`/`MAX` & `abs(x) > 2^53` exact `NUMERIC` precision)/`Float`/`Bool` properties outside `body`; declared properties pre-filter cleanly in `SearchFilter` (undeclared rejected cleanly; `vec0` `attr_<hex>` handles node `Eq` within 16-col budget reserving 1 slot for `0.8.32` `scope`, routing `Ne`/ranges via `canonical_property_values`); explicitly enrolled custom kinds vector-index while unenrolled kinds stay in `vector_unsupported_kinds` (`tc67` green); op-collection kind readback/mismatch passes; property updates create immutable revisions (`_fathomdb_artifact_revisions`). |
| R31-02 | Ad-hoc + declared typed predicates on `graph.expand` and `use_graph_arm`, opt-in fused-hit seeding + seed provenance, and mixed-edge seed-slot fix | Traversal tests prove `=`, `!=`, `<`, `<=`, `>`, `>=` on both undeclared (inline) and declared edge/target properties (pinning missing-key semantics, cross-type scalar comparisons, and `proptest` disabled-optimization equivalence: indexed B-tree == inline scan); ineligible edges never enter paths, displace `result_limit`, or consume `SEED_FTS_N` slots (adversarial `>= 12`-edge proof); opt-in `graph_seed="fused_top_k"\|"graph_fts"\|"both"` seeds body-less edges from fused/reranked hits with per-hit `graph_seed_source` + `graph_hop_depth` while `use_graph_arm=False` stays zero-diff; `EXPLAIN QUERY PLAN` verifies indexed `SEARCH` (no full `SCAN`). |
| R31-03 | Historical graph reconstruction (`as_of_write_cursor` and `as_of_epoch_ms` revision-aware traversal) | Real-DB four-quadrant bitemporal tests + `proptest` snapshot oracle reconstruct exact immutable write revisions before/after edge & node supersession with explicit boundary operators (`write_cursor <= as_of_cursor`, `superseded_at IS NULL OR superseded_at > as_of_cursor`, half-open `[t_valid, t_invalid)`) and node `body`/`properties` coherence as of `as_of` (documenting current-state `Engine::transition` semantics since `transition` mutates `state` in place); `as_of_epoch_ms` resolves via `_fathomdb_commit_timestamps` (returning cursor `0`/empty before the first Schema-38 commit on migrated stores); `EXPLAIN QUERY PLAN` verifies `canonical_edges_{from,to}_active_idx` (current) and `canonical_edges_{from,to}_sup_kind_idx` (`as_of`); purged/erased rows are never replayed. |
| R31-04 | Strict-only write endpoint integrity | Real-DB tests prove every `engine.write` and `actuate` call referencing a missing, superseded-only, or inactive (`Pending`/`Deleted`) endpoint causes atomic refusal (`WriteValidationError` / `ActuationOutcomeV1::Refused`) with zero partial state after reopen (including in-batch edge-before-node valid ordering and node-only zero-SQL authorizer fast path); zero dangling edges exist in `0.8.31` stores. |
| R31-05 | SDK unblockers, minimal 37→38 `fathomdb migrate` helper, lockstep `fathomdb-cli` PyPI wheel, and targeted contract doc fixes | `resolve_evidence_many`, `search(hints=)`, and `OpenReport` booleans (`reranker_compiled`, `cuda_compiled`, `rerank_available`) pass across all 3 SDKs; `Engine::open` continues rejecting schema 37 directly; `fathomdb migrate` refuses incomplete physical erasures & genuinely dangling `0.8.30` edges while admitting valid superseded edges, rolls back byte-for-byte on mid-DDL fault, and upgrades a real closed `0.8.30` golden DB cleanly (`doctor check-integrity --full` green); `fathomdb-cli` PyPI wheel (`--bin fathomdb` only) installs into a clean venv with no Rust toolchain and runs `fathomdb doctor` + `migrate`; `docs/reference/python-api.md` and `docs/install/python.md` fixes verified. |
| R31-06 | Erasure, W12 ergonomics fixes, and metadata completeness invariants | Automated `sqlite_master` + `ExportClass` audit test passes (every table including dynamic `vector_default_*` / FTS5 shadows classified across canonical/projection, read-visibility triggers, erasure, `doctor`, Inspector, and `ExportClass`); `purge` and `erase_source` (Autonomous + ExplicitMaintenance) leave zero `canonical_property_values` rows, zero WAL/file remnants of inline node/edge `properties`, and valid closure proofs; W12 Autonomous two-truncate keeps unrelated plain writes open on BUSY, `completed_pending_erasure` distinguishes completing retries while retaining cross-source audit rows, and release-mode BUSY diagnostics use lightweight atomic counters truthfully. |
| R31-07 | Quantified zero latency regression vs. `0.8.30` | Same-host interleaved paired `v0.8.30` (`db271323f`) vs. `0.8.31` benchmark at **10k, 50k, and 100k** nodes (`1.5x` edges, plus 5k baseline parity check) via `experiments/sdk_benchmark.py` with `>= 5` warmup rounds and `n >= 30` measured samples per workload: 10,000-resample paired bootstrap 95% CI upper bound for `(0.8.31 - 0.8.30)` `<= max(5.0%, 1.0 ms)` on p50/p95 across unpredicated open, batch write, streamed FTS, optimized hybrid (`hybrid_deferred_identity`), and current 2-hop `graph.expand`; `database_bytes` overhead `<= +15%` and `peak_rss_bytes` overhead `<= +10%`; dedicated high-fanout predicate (B-tree <= 0.25x unindexed at 50k) and `as_of` composite-seek (< 5 ms p95) sanity checks plus `tests/slice38_query_plans.rs` `SEARCH`-only pins pass. |
| R31-08 | Inspector `0.2.1` integrated onto `main`, upgraded to `0.8.31`, wired into release cycle, and CI/bootstrap/harness hardened | `origin/inspector/dev/0.2.1` (`d6501cc5a`) is integrated; `src/inspector/` compiles against `0.8.31` Engine (model-free + `cached-cpu`) and passes property/history/erasure qualification under the retained Docker-free native Ubuntu 24.04 pipeline (`D4 = B`); `local-ci.sh` defaults to an Inspector-scoped gate (~5 min) when shared core paths are unchanged (retaining `--full-workspace`); `bootstrap.sh`/`setup-python-owned.sh` prefer system `/usr/bin/python3.12` on Ubuntu 24.04 and provision `src/ts/node_modules`; process/GUI/`strace` harnesses enforce explicit per-command timeouts; Inspector release artifacts build and smoke-pass in release CI. |

## Cross-cutting DoD (every feature slice)

- **X1 (SDK & surface parity)**: Rust (`fathomdb-sdk`), Python (`_fathomdb.pyi`,
  `python_surface_additions.json`, `test_slice130_package_boundary.py`),
  TypeScript (`sdk-surface-parity.test.ts`), `src/conformance/governed-{operation-parity,surface-allowlist}.json`,
  and `scripts/governed-surface-pin.json` (pre-authorized at HITL Gate 1) stay
  in lockstep.
- **X2 (Erasure & metadata audit)**: `sqlite_master` + `ExportClass`
  table-classification test, read-visibility triggers, `purge`/`erase_source`,
  and `doctor` checks pass.
- **X3 (Performance guard)**: `tests/slice38_query_plans.rs` (`EXPLAIN QUERY PLAN`
  `SEARCH`-only assertions) and scoped latency spot-checks pass before slice
  close; full paired 10k/50k/100k bootstrap gate runs in Slice 70.
- **X4 (Docs & interfaces)**: `dev/interfaces/{rust,rust-sdk,python,typescript,cli,wire}.md`
  and `docs/` updated in the same slice as any contract change (strictly
  product-generic — zero external consumer-project specifics written to tracked
  repo files).

## Slice ladder (Phase B — renumbered per HITL Gate 1)

| Slice | Title | Requirements | Depends on | Parallel / conflict notes |
| --- | --- | --- | --- | --- |
| 0 | Preflight, integrate `origin/inspector/dev/0.2.1` (`d6501cc5a`), fix worktree Python/TS bootstrap, write ADRs, W12 fence review, capture `0.8.30` baseline, and run Phase A prework audits (`Slices 1–9`) | R31-08 (bootstrap/merge), All | `0.8.30` closed + owner go-ahead | **LANDED (`bf8e905e3`, `42595212c`)** — Inspector merged, bootstrap fixed, ADRs `#73`–`#76` authored, golden schema-37 DB & baseline captured, Phase A audits & Slice 9 triage approved at HITL Gate 1. |
| 10 | Prework prep (`PREWORK-PREP`): ADR/plan amendments (`P4-09` commit timestamps, `vec0`/index/R1/`P5-11`, W12 ADR #0.8.28 amendment, boundary scrub), security/lockfile hygiene (`P1-01`–`P1-03`), public-doc truth flip (`P3-18`, `P3-19` pre-publish) + quick doc fixes (`P2-02`/`P6-01`/`P6-07`/`P6-09`–`P6-11`), and CI/release-friction hardening (`P7-03`, `P7-07`–`P7-10`) | Prework prep (`slice-9-triage.md`) | 0 + HITL Gate 2 | Low-risk hygiene, ADR pre-freeze clarifications, public-doc truth gate green, and CI/release-friction fixes before Schema 38 coding starts. |
| 20 | Schema 38+, outside-`body` inline properties + declared typed indexes, commit timestamps (`_fathomdb_commit_timestamps`), app-declared vector kinds, op-collection readback, W12 fixes, and `sqlite_master` + `ExportClass` audit | R31-01, R31-06 | 10 | Keystone for `fathomdb-schema`, `SearchFilter` property predicates, `vec0` 16-col budget (`P4-01`/`P4-06`), `ROW_OWNED_PROJECTIONS`, `visibility_coalescing_trigger_sql!` (`P4-04`), `physical_erasure`, W12 fixes, `sqlite_master`/`ExportClass` audit (`P4-12`), `docs/compatibility/index.md` schema-`38` sync, and `P4-13` Inspector compile/fixture helper updates (`src/inspector/tests/{fixtures/mod.rs,integration/graph.rs}`). |
| 30 | Strict-only endpoint integrity, edge/target predicates, opt-in fused-hit seeding + seed provenance, and mixed-edge slot fix | R31-02, R31-04, R31-06 | 20 | Touches `write_validation.rs`, `write_commit.rs` (`P4-07`), `graph_expand/` (`P4-10`, `#[non_exhaustive]` `GraphExpandRequestV1::new`), `search.rs` (`P4-11`), `filter.rs`, SDK bindings; updates legacy dangling-edge, convenience-edge, and 3 Inspector dangling-edge fixtures (`P2-05`, `P4-13`, `P5-04`). |
| 40 | Historical graph reconstruction (`as_of_write_cursor` / `as_of_epoch_ms` revision-aware traversal) | R31-03, R31-06 | 20, 30 | Depends on Slice 20 schema/commit timestamps and Slice 30 `graph_expand/execution.rs` SQL pushdown (`P4-10`, `P4-11`, `P5-06`); keeps `src/inspector/src/{graph,lexical}.rs` compiling cleanly (`P4-13`). |
| 50 | SDK unblockers (`resolve_evidence_many`, `search(hints=)`, `OpenReport` booleans), minimal 37→38 `fathomdb migrate` helper, `fathomdb-cli` PyPI wheel (`--bin fathomdb`), and targeted contract doc fixes | R31-05, R31-06 | 30, 40 | Touches SDK bindings, `fathomdb-cli` upgrade helper + PyPI Maturin bin packaging (`P4-14`, `P7-05` in `.github/workflows/release.yml`), and targeted public/interface doc fixes (`P3-12`–`P3-15`, `P6-03`, `P6-05`). |
| 60 | Inspector `0.8.31` Engine + property/history UI upgrade, scoped `local-ci.sh`, harness timeouts, `D4 = B` lock, and release-cycle wiring | R31-08 | 30, 40, 50 | Serialized after Slice 50 so `.github/workflows/release.yml`, `check-release-contract-truth.py`, and `test_release_workflow_scope.sh` have a single writer (`P7-06`); upgrades Inspector to schema 38+ properties & `as_of` history (`P5-10`), decouples `local-ci.sh` (`P7-01`), adds per-command harness timeouts (`P7-02`), and locks `D4 = B` (`P7-04`). |
| 70 | Full-workspace qualification, paired `0.8.30` vs. `0.8.31` 10k/50k/100k bootstrap latency gate, release notes & generated docs, and golden schema-38 archive | R31-07, All | 50, 60 | Unconfined `./scripts/agent-verify.sh`, `AGENT_LONG=1`, paired 10k/50k/100k bootstrap latency/RSS gate (`P5-11`, `P5-12`), `0.8.31` release notes & generated docs (`P3-01`, `P3-20`), golden schema-38 archive for `0.8.34`. |
| 80 | Authorized `0.8.31` publication (crates, PyPI `fathomdb` + `fathomdb-cli`, npm, Tegra, Inspector) | All | 70 + explicit HITL publish approval | Separate publish authorization; post-publish installed consumer smokes across all targets + `P3-21` publication truth flip (`RELEASE_SCHEMAS["0.8.31"] = 38`). |

## Behavior-change register (expected)

- **Breaking (pre-1.0) — schema version bump (`37 -> 38`)**: non-empty
  schema-37 stores are refused on direct `Engine::open`; use the `0.8.31`
  minimal `fathomdb migrate` (37→38) helper (or full `0.8.34` multi-version
  `fathomdb migrate` later).
- **Breaking (pre-1.0) — strict-only write endpoint integrity**: edges with
  missing or inactive endpoints are atomically refused on `engine.write` (opt-in
  bulk/non-strict import arrives in `0.8.32`).
- **Additive SDK & CLI surface**: outside-`body` typed node/edge properties
  (inline JSON + declared typed indexes for `SearchFilter`), `_fathomdb_commit_timestamps`,
  explicit opt-in app-declared vector-committable kinds, ad-hoc & declared
  edge/target predicates, opt-in fused-hit graph seeding (`graph_seed`) with
  seed/depth provenance, `use_graph_arm` edge filters, historical graph `as_of`
  mode (`as_of_write_cursor` & `as_of_epoch_ms`), batch `resolve_evidence_many`,
  `search(hints=)`, `OpenReport` capability booleans, op-collection kind
  readback, minimal 37→38 `fathomdb migrate` helper, and lockstep `fathomdb-cli`
  PyPI binary wheels.
- **Targeted defect/ergonomics fixes**: mixed-edge `SEED_FTS_N` seed-slot fix
  and W12 erasure ergonomics.
- **No silent behavior change** on unpredicated, current-time queries.

## Reserved-gap policy and deferred work

Slice numbers advance in steps of 10 (`0, 10, 20, 30, 40, 50, 60, 70, 80`), leaving
gaps reserved for necessary coherent follow-ups; scope overflow halts for owner
disposition. Explicitly deferred out of `0.8.31`: caller scope, read+write MCP,
bulk write opt-in, and `quality-backlog.md` (moved to [0.8.32](../0.8.32/plan.md));
CUDA 13 (`cuda-delivery-plan.md`, deferred post-`0.8.35`); `ingest_with_extractor`
rework (deferred post-`0.8.34`, documented as legacy in `0.8.33`).

## Immediate next slice

**None — `0.8.31` ladder is complete (`v0.8.31` published 2026-11-23).** See
[release-state-0.8.31.json](../release-state-0.8.31.json),
[STATUS-0.8.31.md](../runs/STATUS-0.8.31.md), and
[features/slice-80/status.md](features/slice-80/status.md).

## Open HITL decisions

All `0.8.31` HITL gates (Gate 1, Gate 2, and Slice 80 publication authorization)
are resolved and closed.

## Landed slice roll-up

<!-- BEGIN GENERATED release-state:0.8.31:plan-landed-roll-up -->
**LANDED on `origin/main`, in full:** Slices 0 (`42595212c`) · 10 (`3c4b84439`) · 20 (`975637f3c`) · 30 (`8e44a47ad`) · 40 (`fe5e7d8a1`) · 50 (`28f5def55`) · 60 (`6685b0ccf`) · 70 (`dda61cfae`) · 80 (`64e21c3c5`). SCHEMA is 38; remaining ladder = none.<!-- END GENERATED release-state:0.8.31:plan-landed-roll-up -->
