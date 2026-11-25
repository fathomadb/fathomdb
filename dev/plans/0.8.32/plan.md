---
title: FathomDB 0.8.32 release plan
status: ACTIVE
target_release: 0.8.32
date: 2026-11-24
---

# FathomDB 0.8.32 release plan

> **Status: ACTIVE on `dev/0.8.32` (HITL Gate 1 approved 2026-11-24; `0.8.31` published 2026-11-23).**
> Live machine state lives in [release-state-0.8.32.json](../release-state-0.8.32.json)
> and [STATUS-0.8.32.md](../runs/STATUS-0.8.32.md).
> Phase A prework audits (`Slices 0–9`) and HITL Gate 1 rulings live in
> [prework/slice-9-triage.md](./prework/slice-9-triage.md) and
> [HANDOFF-FROM-0.8.31.md](./HANDOFF-FROM-0.8.31.md).
> Publication (`Slice 70`) remains separately gated on explicit HITL authorization.

## Immediate next slice

<!-- BEGIN GENERATED release-state:0.8.32:plan-immediate-next -->
**IMMEDIATE NEXT: Slice 40** (`MCP-RW`) — Atomic governed write + latest_state checkpoint seam + EP-07/EP-01 (Pin C), production read+write fathomdb-mcp (allow_writes: false default, per-store opt-in + bound scope, AC32-MCP-SAFETY-01..09), and 3-channel packaging

**Remaining ladder:** 40 → 50 → 60 → 70.<!-- END GENERATED release-state:0.8.32:plan-immediate-next -->

## Goal and scope

Deliver FathomDB **0.8.32**: engine-enforced **caller scope and write
authority**, production **read+write `fathomdb-mcp`** (scoped frozen evidence +
governed idempotent writes and checkpoints + 3-channel release packaging),
**opt-in bulk / non-strict write mode**, Inspector active-`scope` UI, and
hardened durability (**backup/restore/rebuild, power-loss recovery, and
coverage-guided fuzzing**) with zero latency regression on single-scope
workloads. Full scope boundaries live in [scope.md](./scope.md).

## Footprint and runtime posture

- Scope enforcement, bulk-import validation, frozen reads, and recovery remain
  **in-library, deterministic, and CPU-only**.
- `fathomdb-mcp` ships as a standalone Rust stdio binary over `fathomdb-sdk`
  with explicit scope configuration, `allow_writes: false` by default, and
  bounded admission/token budgets. Shipped 5-target GitHub Release binaries and
  PyPI `fathomdb-mcp` wheels build with `--features default-embedder`, while
  `fathomdb-mcp/Cargo.toml` keeps `default = []` for thin `crates.io` builds.
- Power-loss fault injection and coverage-guided fuzzing run in dedicated
  disposable test environments (user-mode QEMU/KVM + `dm-flakey` guest and
  isolated `fuzz/` workspace) and never alter runtime shipping dependencies.

## Quality roadmap, CUDA version review, and standing tripwire check

- **Quality testing roadmap**
  ([fathomdb-quality-testing-roadmap.md](../../notes/fathomdb-quality-testing-roadmap.md)):
  - **Applies in `0.8.32`**: directly executes
    [0.8.31/quality-backlog.md](../0.8.31/quality-backlog.md) (broad
    coverage-guided fuzzing + power-loss/torn-write recovery proof under a
    declared durability model), plus compound multi-caller authorization,
    idempotent write-retry, and resource-leak checks across MCP and SDK paths.
  - **Deferred**: global 100% MC/DC, general SQLite SQL/B-tree fuzzing, and
    ANN/2.x backend qualification.
- **CUDA version review (`2026-11-24`, [prework/slice-8.md](./prework/slice-8.md)):**
  Primary-source refresh conducted `2026-11-24` against
  [ADR-0.8.28-cuda-version-support-policy.md](../../adr/ADR-0.8.28-cuda-version-support-policy.md)
  and the `0.8.28`/`0.8.29`/`0.8.30` reviews (`ClickPy` SQL, `Pepy` API, and
  GitHub asset download-count splits remain `UNKNOWN` without shell network
  access). **Changes observed:** `vLLM` (`v0.33.0`/`v0.34.0`, Nov 2026) joined
  `PyTorch 2.15.0` (CUDA `13.2.2` default) in dropping standard CUDA 12 wheels
  for CUDA `13.0+`; `Ollama` (`v0.40.18+`) dropped JetPack 5 (`11.4`); NVIDIA
  published `CUDA 13.5` / `R620` LTSB (EOL Sept 2029) and `JetPack 7.2.2`
  (`L4T 39.2.2`, CUDA `13.2.2`, Nov 2026) while keeping `JetPack 6.2.3`
  (`CUDA 12.6.10`) active for Orin and `R580` LTSB active through June 2028;
  `llama.cpp v1.0.0`, `onnxruntime-gpu 1.32.0` (`cuda12` build), `JAX`, and
  `CuPy` retain CUDA 12 support. **Importance table & host posture:** All
  `ADR-0.8.28` importance ratings (`11.x` none; `12.6` tier 1 shipping floor;
  `other 12.x` covered; `13.0` tier 1 next; `13.2` low; `13.4+` compile-check
  only) remain **unchanged** for `0.8.32`, `windchill3` (`580.178.04` +
  `/usr/local/cuda-12.6`, RTX 3090 GPUs `0,1` and K620 GPU `2` unused) and Orin
  JetPack 6 (`12.6`) remain valid for what `0.8.32` ships, and CUDA 13 delivery
  ([0.8.31/cuda-delivery-plan.md](../0.8.31/cuda-delivery-plan.md), gated on
  compiler issue `6111901` kernel-exposure resolution) stays **deferred**
  post-`0.8.35` with no `ADR-0.8.28` amendment required unless the owner elects
  to pull it forward.
- **Standing tripwires (`T1`–`T3`) — NOT FIRED (`2026-11-24`, [prework/slice-8.md](./prework/slice-8.md)):**
  Verified against shipped `0.8.31` (`38a1594bf`) per
  [overflow-chunking-reevaluation.md](../../design/overflow-chunking-reevaluation.md):
  - **`T1` — NOT FIRED** (unmeasured; answer-level overflow probe slated for `0.8.35`).
  - **`T2` — NOT FIRED** (`RRF_K = 30.0`, `RRF_WEIGHT_TEXT = 3.0`,
    `RRF_WEIGHT_VECTOR = 1.0`, `DEFAULT_EMBEDDER_NAME = "fathomdb-bge-small-en-v1.5"`,
    `MAX_SEQUENCE_TOKENS = 512`, and pinned tripwire tests intact; must remain
    zero-behavior-change across Slice 20 `search.rs` split).
  - **`T3` — NOT FIRED** (no exact-scan kernel symbols in `src/`; two-phase
    `vec0` bit-KNN active).
  - Standing ledger todo `TC-2a9f89fa-5302-44b7-a14e-97b9d9cbde90` retargeted
    from `0.8.29` to `0.8.32` via `ledgerwrite`; re-verify at `0.8.32` close.

## Requirements and acceptance signals

| ID | Requirement | Falsifiable acceptance signal |
| --- | --- | --- |
| R32-01 | Engine-enforced caller scope and write authority (Schema 39 `S39-COMPOSITE` + 8-col `vec0` + `fathomdb migrate 38 → 39`; single-scope/single-agent default) | Two-caller fixtures with overlapping records and distinct grants prove zero cross-scope leakage across `search` (including `use_graph_arm`), `fetch`, `expand`, frozen contexts, continuations, and stale refs after grant revocation; unauthorized/stale writes leave state unchanged and retries remain idempotent; single-scope (`scope = ''`) holds `0.8.31` parity and is documented as the default in `docs/`; verified offline `fathomdb migrate` upgrades golden `0.8.31` Schema 38 stores to Schema 39 cleanly (`ADR-0.8.32-caller-scope-isolation.md`). |
| R32-02 | Production read+write `fathomdb-mcp` with scoped frozen evidence, governed writes/checkpoints, and 3-channel release packaging | `allow_writes: false` default, per-store opt-in + bound `scope`, `SC-01`–`SC-10` fail-fast startup conflicts, and `AC32-MCP-SAFETY-01`–`09` pass; identical tasks run through SDK and MCP prove exact bytes, revision identity, typed drift/erasure/truncation outcomes, caller scope enforcement, and idempotent governed writes/`latest_state` checkpoints (`operation_id` replay + `expected_write_boundary`); `fathomdb-mcp` release binaries, PyPI wheel, and crates.io crate pass installed consumer smoke tests. |
| R32-03 | Opt-in bulk / non-strict write mode (default remains strict) | Explicit bulk-write opt-in (`WriteOptions` / `write_with_options`) admits out-of-order edges with accurate `dangling_edge_endpoints` counts and post-import `fathomdb doctor` verification; default `Engine::write` and `Engine::actuate` remain strict and atomically refuse missing/inactive/cross-scope endpoints. |
| R32-04 | Backup, restore, and full rebuild proof (External Reviewer P0) | On pinned installed artifacts, backup/restore and full rebuild from authoritative corpus match revisions, evidence spans, properties, scopes, and graph results identically; internal `fathomdb.logical-export/v1` round-trip validates `ExportClass` completeness (uncentered f32 + `_fathomdb_vector_spans` + `EmbedderIdentity`, excluding `embedding_bin`/`mean_vec`). |
| R32-05 | Power-loss recovery proof and coverage-guided fuzzing (`quality-backlog.md`) | Declared durability/fault model (`SyncMode::Normal` default + opt-in `SyncMode::Full`) executes real dropped/torn/reordered-write interruptions in disposable QEMU/KVM + `dm-flakey` across commit, WAL/checkpoint, and recovery with exact semantic + SQLite integrity verification; isolated `fuzz/` workspace completes declared budgets across 7 targets (`query`, `predicate`, `scope`, `cursor`, `codec`, `MCP`, `recovery`) with minimized regressions committed. |
| R32-06 | Erasure, single-source `TABLE_MANIFEST` audit, Inspector scope view, and zero latency regression | Declarative `TABLE_MANIFEST` in `fathomdb-schema` drives `CANONICAL_TABLES`, `ROW_OWNED_PROJECTIONS`, triggers, erasure, and `ExportClass`; `sqlite_master` completeness audit, scope-aware `purge`/`erase_source`, and Inspector scope UI + Playwright CI tests pass; paired `v0.8.31` vs. `0.8.32` benchmark confirms zero statistically significant p50/p95 regression on single-scope workloads. |
| R32-07 | Contributor/agent orientation aids and zero-behavior-change `search`/`evidence` hotspot splits | `dev/design/cross-cutting-change-checklist.md`, `dev/design/README.md`, Active/Superseded ADR index split, and actionable parity/pin/boundary gate diagnostics land in Slice 10; top-level `slice*_tests.rs`/`*_tests.rs` move out of `fathomdb-engine/src/` and `search.rs`/`evidence.rs` split into submodules in Slice 20 entry with `check-module-boundaries.sh` and full test parity green before scope edits. |

## Cross-cutting DoD (every feature slice)

- **X1 (SDK & surface parity)**: Rust (`fathomdb-sdk`), Python (`_fathomdb.pyi`,
  `python_surface_additions.json`, `test_slice130_package_boundary.py`),
  TypeScript (`sdk-surface-parity.test.ts`),
  `src/conformance/governed-{operation-parity,surface-allowlist}.json`, and
  `scripts/governed-surface-pin.json` (pre-authorized at HITL Gate 1 Decision 4)
  stay in lockstep.
- **X2 (Erasure, scope & metadata audit)**: declarative `TABLE_MANIFEST`,
  `sqlite_master` + `ExportClass` table-classification tests, read-visibility
  triggers, scope-aware `purge`/`erase_source`, and `doctor` checks pass.
- **X3 (Performance guard)**: `EXPLAIN QUERY PLAN` `SEARCH`-only assertions and
  single-scope fast-path checks pass before slice close; full paired
  `0.8.31` vs. `0.8.32` 10k/50k/100k bootstrap gate runs in Slice 60.
- **X4 (Docs & interfaces)**: `dev/interfaces/{rust,rust-sdk,python,typescript,cli,wire,mcp}.md`
  and `docs/` updated in the same slice as any contract change (strictly
  product-generic — zero external consumer-project specifics written to tracked
  repo files).

## Slice ladder (canonical, Decision 3)

| Slice | Codename | Title | Requirements | Depends on | Parallel / conflict notes |
| --- | --- | --- | --- | --- | --- |
| 10 | `PREWORK-PREP` | Worktree bootstrap, release state, baseline CI fixes, `R32-07` orientation/gate aids, immediate doc fixes, four `ADR-0.8.32-*` drafts, Slice 50 venue/fuzzing prep, and Decision 1 empirical experiment | R32-07, All | Prework `0–9` + Gate 1 | Restores green baseline CI, dismisses non-shipping Dependabot noise, delivers `WP-10D` 100k Schema-38 vs. Schema-39 + `NORMAL`/`FULL` measurements, and pre-stages `cargo-fuzz` + QEMU `/dev/kvm` venue. |
| 20 | `SCOPE-AND-REFACTOR` | Pre-scope `search`/`evidence` hotspot splits, single-source `TABLE_MANIFEST`, Schema 39 (`S39-COMPOSITE` + 8-col `vec0` + `fathomdb migrate 38 → 39`), and engine-enforced caller scope/write authority (Rust/Python/TS SDK parity, `Pin A`) | R32-01, R32-06, R32-07 | 10 (DONE) | Opens with zero-behavior-change test-file relocation and `search.rs`/`evidence.rs` submodule split (`module-boundary-policy.txt` updated), promotes `sqlite_master` audit to declarative `TABLE_MANIFEST`, lands `SCHEMA_VERSION = 39` (`STEP_39_SQL`, 8-col `vec0`, verified `fathomdb migrate 38 → 39` against golden `0.8.31` fixture), activates `scope` across read/search/graph/frozen/write/actuate/erasure paths binding `scope = ''` on the single-scope default fast path, and updates `docs/` to state single-scope (one agent per DB) is the default. |
| 30 | `BULK-AND-INSPECTOR` | Opt-in bulk / non-strict write mode (`Pin B`) and Inspector active-`scope` UI + Playwright CI wiring | R32-03, R32-06 | 20 | Parallelizable with Slice 40 after Slice 20; touches write validation/commit, SDK bindings, and `src/inspector/`. |
| 40 | `MCP-RW` | Atomic governed write + `latest_state` checkpoint seam + `EP-07`/`EP-01` (`Pin C`), production read+write `fathomdb-mcp`, and 3-channel packaging | R32-02, R32-06 | 20 | Parallelizable with Slice 30 after Slice 20; touches SDK actuation/read surface, `fathomdb-mcp`, `src/python-mcp/`, `dev/interfaces/mcp.md`, and release workflows/contract tests. |
| 50 | `DURABILITY-AND-FUZZ` | Backup/restore/rebuild proof (`logical-export/v1`), `SyncMode::{Normal, Full}` + QEMU/KVM `dm-flakey` power-loss campaign, and isolated `fuzz/` workspace | R32-04, R32-05 | 30, 40 | Executes `quality-backlog.md`, External Reviewer P0 durability proofs, and internal `fathomdb.logical-export/v1` round-trip validation on real stores and disposable fault environment. |
| 60 | `QUALIFICATION` | Full-workspace qualification and paired `0.8.31` vs. `0.8.32` latency/RSS/DB-size gate | R32-06, All | 50 | Full unconfined `./scripts/agent-verify.sh` + `AGENT_LONG=1 ./scripts/check.sh`, paired latency/RSS gate, 5-target + Orin + `fathomdb-cli` + `fathomdb-mcp` + Inspector candidate qualification, release docs, and golden `0.8.32` fixture DB archive. |
| 70 | `PUBLICATION` | Authorized `0.8.32` publication | All | 60 + explicit HITL publish approval | Held for separate publish authorization; multi-registry publish and post-publish installed consumer smokes across all targets. |

## Settled HITL Gate 1 decisions (2026-11-24) and open checkpoints

Recorded in [prework/slice-9-triage.md](./prework/slice-9-triage.md) § 4:

| Decision | Status | Ruling / Checkpoint |
| --- | --- | --- |
| **Decision 1** — Schema-38 non-breaking vs. Schema-39 breaking `scope` layout | **RULED (Option 1 — `S39-COMPOSITE`, Schema 39)** | Owner ratified `S39-COMPOSITE` (`SCHEMA_VERSION = 39`, composite `(scope, ...)` PKs/indexes, 8-column `vec0`, verified `fathomdb migrate 38 → 39`) from Slice 10 `WP-10D` measurements (`ADR-0.8.32-caller-scope-isolation.md` `#102` accepted). Single-scope (one agent per database, `scope = ''`) is the default posture and MUST be stated in user `docs/`; `0.8.33` records a follow-up to revisit/relax that direction. |
| **Decision 2** — WAL `PRAGMA synchronous` durability contract vs. `dm-flakey` | **RULED (Option 1)** | Retain `SyncMode::Normal` as library default (honest WAL checkpoint-bounded durability amending `ADR-0.6.0-durability-fsync-policy.md`) and add opt-in `EngineConfig` `SyncMode::{Normal, Full}`; qualify both in Slice 50 disposable QEMU/KVM + `dm-flakey`. |
| **Decision 3** — Canonical slice ladder renumbering | **RULED (Option 1)** | Adopt `Slice 10` (`PREWORK-PREP`) and feature slices `20–70` (`70 = PUBLICATION`, separately HITL-gated). |
| **Decision 4** — Governed-surface pin pre-authorization | **RULED (Option 1)** | Pre-authorize targeted `scripts/governed-surface-pin.json` updates (`core = 5`, `recovery_denylist = 5` fixed) for Pin A (Slice 20), Pin B (Slice 30), and Pin C (`PutLatestState` + `EP-07` + `EP-01`, Slice 40). |
| **Decision 5** — `fathomdb-mcp` shipped binary & PyPI wheel feature set | **RULED (Option 1)** | Build 5-target GitHub Release binaries and lockstep PyPI `fathomdb-mcp` wheels with `--features default-embedder`; keep `default = []` in `fathomdb-mcp/Cargo.toml` for `crates.io`. |
| **Decision 6** — Consolidated 102-proposal triage bundle (`P1-01`–`P8-05`) | **RULED (Option 1)** | Approve all recommended `include` / `postpone` / `reject` dispositions (including `P1-01` API dismissal of 106 non-shipping Dependabot alerts without editing immutable `slice-135` lockfiles, `allow_writes: false` MCP default with per-store opt-in + bound `scope`, and `0.8.33`/`0.8.34`/CUDA 13 deferrals). |
| **Decision 7** — Rust `1.95.0 → 1.100.0` toolchain upgrade timing | **RULED (Defer to `0.8.33`)** | Keep Rust `1.95.0` pinned through `0.8.32`; upgrade to `1.100.0` (with Cargo build-dir v2 smoke/`rust-cache` fixes, 19 `clippy::chunks_exact_to_as_chunks` sites, Docker/runner updates) in `0.8.33` (`dev/plans/0.8.33/scope.md` § 6). |
| **Decision 8** — Slice 50 Ubuntu 24.04 cloud-image egress (`AGENTS.md` § 11) | **RULED (Approved)** | Owner authorized downloading the pinned official Ubuntu `24.04` Noble cloud image (`cloud-images.ubuntu.com`, SHA-256 verified) into `/mnt/data/worktrees/fathomdb-0.8.32-dmflakey-vm/` for Slice 50's `/dev/kvm` `dm-flakey` VM. |
| **Publication Gate** — `v0.8.32` tag & multi-registry publish | **HELD** | Requires completed Slice 60 qualification and separate explicit HITL publication approval before Slice 70 begins. |

## Reserved-gap policy and deferred work

Slice numbers advance in steps of 10 (`10, 20, 30, 40, 50, 60, 70`), leaving
gaps reserved for necessary coherent follow-ups; scope overflow halts for owner
disposition. Explicitly deferred out of `0.8.32` per [scope.md](./scope.md) and
[prework/slice-9-triage.md](./prework/slice-9-triage.md): public OSS repo/doc
IA overhaul, bulk `dev/` pruning, auto-doc pipeline redesign, Rust `1.95.0 →
1.100.0` toolchain/MSRV upgrade, and revisiting the single-agent default doc
direction (`0.8.33`); general multi-version `fathomdb migrate`, public CLI
`export`/`import`, and personal-agent kits (`0.8.34`); publishable competitive
benchmark campaigns (`0.8.35`); and CUDA 13 delivery (`cuda-delivery-plan.md`,
post-`0.8.35` or dedicated release).
