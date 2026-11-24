---
title: 0.8.32 Prework Slice 9 — Triage and HITL Gate 1
status: APPROVED
target_release: 0.8.32
slice: 9
date: 2026-11-24
---

# 0.8.32 Prework Slice 9 — Triage & HITL Gate 1

Consolidated, scored triage of all prework proposals from **Slices 0–8** (`P1-01`–`P8-05`, **102 total proposals** across `prework/slice-{1..8}.md`) per [`RELEASE-AND-SLICE-PLANNING-TEMPLATE.md`](../../prompts/RELEASE-AND-SLICE-PLANNING-TEMPLATE.md).

> **Status: STOP GATE — Awaiting Owner HITL Gate 1 Review.**
> Zero product code, dependency pins, or canonical repo configs have been mutated during Slices 0–9. Execution of Slice 10 (`PREWORK-PREP`) and Phase C draft plans begins only after owner rulings below.

---

## 1. Key Decisions Requiring HITL Gate 1 Sign-Off

Six concrete architectural and planning rulings surfaced from direct code and CI forensics in Slices 0–8:

### Decision 1 (`P4-01`, `P4-02` — CRITICAL): Non-breaking Schema-38 `scope` activation semantics (`vec0`, uniqueness, `operational_collections`, indexes)

Direct code inspection (`vector_storage.rs:41-99`, `fathomdb-schema/src/lib.rs:161-165,282-306,442-445`, `write_commit.rs:974-982`, and `golden-0.8.31-schema38.sqlite`) revealed four structural realities: (a) `0.8.31` `vector_default` (`vec0`) has 7 built-in columns and **no `scope` column** on disk (`ensure_vector_partition` currently fails closed on any DDL mismatch, and `reshape_vector_partition_nondestructive` omits `LEFT JOIN canonical_edges`); (b) `canonical_nodes_logical_active_idx`, `canonical_edges_logical_active_idx`, and `operational_state PRIMARY KEY(collection_name, record_key)` are **store-wide unique**; (c) `operational_collections` has no `scope` column; and (d) zero `scope` indexes exist in `STEP_38_SQL` while `0.8.31` 100k DB size (`200.4 MB`) is already `+13.09%` over `0.8.30`.

- **Option 1 (Recommended):** Keep **`SCHEMA_VERSION = 38`** (zero breaking migration for `0.8.31` stores):
  1. Update `ensure_vector_partition` to accept both 7-col (`0.8.31`) and 8-col (`scope TEXT DEFAULT ''`) layouts; activate `scope` via lazy/writable-open `reshape_vector_partition_nondestructive` (fixing the `canonical_edges` join), with a `canonical_nodes` preselection/post-filter fallback for read-only legacy 7-col stores.
  2. Enforce **store-wide `logical_id` & `(collection_name, record_key)` uniqueness with a strict same-scope ownership guard** (Scope B targeting an active ID/key owned by Scope A fails atomically with a typed scope/conflict error instead of overwriting).
  3. Keep `operational_collections` schemas store-global (row-level `scope` on `operational_state`/`operational_mutations`), and create additive **partial indexes `WHERE scope != ''`** idempotently on writable open so single-scope `database_bytes` overhead stays `~0%`.
- **Option 2:** Bump to breaking `SCHEMA_VERSION = 39` to rewrite `operational_state` PK and `canonical_*` unique indexes into `(scope, logical_id)` composites (requires offline `fathomdb migrate 38 -> 39` for all `0.8.31` stores, overturning `HANDOFF-04`).

### Decision 2 (`P4-07`, `P5-03` — CRITICAL): WAL `PRAGMA synchronous` durability contract vs. Slice 50 `dm-flakey` power-loss oracle

`open.rs:1262` sets `PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;`. Under real block-level power failure (`dm-flakey` `drop_writes` + QEMU power-off), `NORMAL` guarantees crash consistency without corruption, **not** zero loss of un-checkpointed WAL commits sitting in volatile storage buffers.

- **Option 1 (Recommended):** Keep **`SyncMode::Normal` as default** (protecting `0.8.31` batch write speed `32.8 ms`/`15.5 ms` and honestly declaring WAL checkpoint-bounded durability under `Normal`), **and** add opt-in `EngineConfig` **`SyncMode::{Normal, Full}`** (`PRAGMA synchronous = FULL`) for strict per-acknowledged-commit power-loss durability, qualifying **both** modes in the Slice 50 QEMU/KVM `dm-flakey` harness.
- **Option 2:** Switch the global default to `PRAGMA synchronous = FULL` for all stores (strict per-commit power-loss survival by default, accepting potential regression on the paired `0.8.31` vs `0.8.32` write-latency gate).

### Decision 3 (`Phase B`): Canonical slice ladder renumbering (`Slice 10 = PREWORK-PREP`, feature slices `20–70`)

- **Option 1 (Recommended):** Adopt canonical `RELEASE-AND-SLICE-PLANNING-TEMPLATE.md` numbering: `Slice 10` (`PREWORK-PREP`) -> `Slice 20` (`SCOPE-AND-REFACTOR`) -> `Slice 30` (`BULK-AND-INSPECTOR`) || `Slice 40` (`MCP-RW`) -> `Slice 50` (`DURABILITY-AND-FUZZ`) -> `Slice 60` (`QUALIFICATION`) -> `Slice 70` (`PUBLICATION`, held for separate HITL publish approval). Mirrors `0.8.31` cleanly and separates CI/ADR/prep hygiene from feature code.
- **Option 2:** Keep draft `plan.md` numbering (`0, 10..60`) by folding prework prep into Slice 0.

### Decision 4 (`P3-03`, `P4-05/08/10/11`): Pre-authorization for `scripts/governed-surface-pin.json` updates

`check-governed-surface-pin.sh` fails closed on any unapproved change to `governed-{operation-parity,surface-allowlist}.json` (`56` ops / `93` members today; `core` = `5` and `recovery_denylist` = `5` remain strictly invariant). Note also `CRITICAL-4`: `fathomdb-mcp` obeys `MR-070.2` (`fathomdb-sdk` only), and neither `ActuationOperationV1` (lacks `latest_state` op) nor `Engine::write` (lacks `operation_id` / `expected_write_boundary`) currently provides an atomic governed checkpoint seam.

- **Option 1 (Recommended):** Pre-authorize targeted governed-surface pin updates across Slices 20/30/40 for:
  - **Pin A (Slice 20):** Caller `ScopePolicy` on `OpenOptions` / `EngineConfig` (+ `scope` fields on records/hits, `ScopeViolation` error if needed, `ReadView` parity on `graph::search_expand`).
  - **Pin B (Slice 30):** Opt-in bulk write (`WriteOptions` / `Engine::write_with_options`, preserving strict default `Engine::write` and strict-only `Engine::actuate`).
  - **Pin C (Slice 40):** Atomic governed `PutLatestState` checkpoint seam (with scope-bound `_fathomdb_actuation_receipts`) + `EP-07` (`OpenOptions.create_if_missing`) + `EP-01` (`read::visibility_state` lock-free snapshot & `canonical_write_boundary`), deferring `EP-05`, `EP-10`'s `origin_class`, and `EP-16` post-`0.8.32`.
- **Option 2:** Pre-authorize Pin A, Pin B, and only `PutLatestState` + `EP-07` in Pin C (omit `EP-01` `read::visibility_state`, leaving MCP `state` unavailable in `orient`).

### Decision 5 (`P7-09`, `P1-04`): `fathomdb-mcp` shipped binary & PyPI wheel feature set

- **Option 1 (Recommended):** Build shipped 5-target GitHub Release binaries and lockstep PyPI `fathomdb-mcp` wheels with **`--features default-embedder`** (CPU built-in BGE embedder so `search_mode: "hybrid"` works out of the box, matching `fathomdb-cli`), while keeping `default = []` in `fathomdb-mcp/Cargo.toml` for thin `crates.io` builds (documenting `--features default-embedder` in `README.md`).
- **Option 2:** Ship thin `--no-default-features` binaries/wheels (`search_mode: "text"` only unless callers supply external vectors).

### Decision 6 (Consolidated Prework Triage Bundle — `P1-01`..`P8-05`)

- **Option 1 (Recommended):** Approve all `include` / `postpone` / `reject` dispositions in Section 2 below — specifically reconciling the Slice 1 vs. Slice 2 Dependabot approach in favor of **Slice 1 (`P1-01`)**: dismiss the `106` non-shipping/deleted/historical alerts via GitHub API as `not_used` without mutating immutable `0.8.27` `slice-135` provenance lockfiles (guarded by `test_slice135_cargo_receipt_snapshots.py`), prune `7` satisfied `origin/dependabot/*` branches, fix all 5 baseline `main` CI failures in Slice 10, and keep `0.8.33`/`0.8.34`/CUDA 13 deferrals intact.
- **Option 2:** Approve with explicit modifications to listed proposal IDs.

---

## 2. Scored Proposal Triage Table (`P1-01` – `P8-05`)

Legend: **Understood / Risk** = `H`/`M`/`L`; **Effort** = `S`/`M`/`L`; **Recommendation** = `include` (with Phase B target slice) / `postpone` / `reject`. Cross-slice duplicates are explicitly cross-referenced.

| ID | Source Slice | Proposal | Understood (H/M/L) | Risk (H/M/L) | Effort (S/M/L) | Recommendation (`include` / `postpone` / `reject`) |
| --- | --- | --- | :---: | :---: | :---: | --- |
| `P1-01` | Slice 1 | Dismiss `106` non-shipping GitHub Dependabot alerts (`20` deleted `python/uv.lock`, `81` immutable `0.8.27` `slice-135` locks, `5` `0.8.29` experiment lock) via API as `not_used` without editing provenance snapshots | H | L | S | **include** (Slice 10; supersedes `P2-06` rename) |
| `P1-02` | Slice 1 | Prune `7` satisfied/superseded `origin/dependabot/*` remote branches | H | L | S | **include** (Slice 10) |
| `P1-03` | Slice 1 | Keep root `package.json` `overrides` empty; accept `katex 0.16.47` (low, dev-only `#77`) until upstream widens | H | L | S | **include** (Slice 10) |
| `P1-04` | Slice 1 | Prepare `fathomdb-mcp` for `crates.io` (`publish = true`, `LICENSE`, `README.md`, `cargo package --list`/`--dry-run`, `rmcp =3.5.1`, optional `sha2 0.11`, release scripts) | H | M | M | **include** (Slice 40) |
| `P1-05` | Slice 1 | Create lockstep `src/python-mcp/` maturin wheel + `fathomdb[mcp]` extra (`AC-041` empty base scripts), 5-target GH Release binaries, `tiers.toml` update, and pre-configure PyPI Pending / crates.io Trusted Publishers | H | M | M | **include** (Slice 10 publishers / Slice 40 wiring) |
| `P1-06` | Slice 1 | Isolate fuzzing deps (`cargo-fuzz`, `libfuzzer-sys`, `arbitrary`) inside standalone `fuzz/` workspace (`exclude = ["fuzz"]` + `cargo tree --locked` CI gate) | H | L | S | **include** (Slice 50) |
| `P1-07` | Slice 1 | Retain all qualified Rust runtime, binding, and CUDA 12.6 fork pins unchanged (`0` shipping alerts) | H | L | S | **include** (All slices — invariant) |
| `P1-08` | Slice 1 | Retain reviewed dev tool pins (`ruff 0.15.17`, `pyright 1.1.410`, `maturin >=1.9,<2`, `mkdocs 1.6.1`, `markdownlint-cli2 ^0.24.2`, Mermaid `12.2.0`) and fix `test_agent_typecheck_pyright_version.sh` fixture | H | L | S | **include** (Slice 10; paired with `P7-04`) |
| `P1-09` | Slice 1 | Refresh offline `dev/tools/onnx/export-requirements.txt` (`43` alerts) with model SHA-256 `c92689ec` verification | M | M | M | **postpone** (dedicated exporter refresh) |
| `P1-10` | Slice 1 | Retire EOL Node `25.9.0` & Tegra `npm 11.12.1` in release/Tegra workflows | M | M | M | **postpone** (`0.8.33` / release-tooling slice) |
| `P1-11` | Slice 1 | Retire `[patch.crates-io]` fork patches, stale `src/python/uv.lock`, `tokenizers`/`thiserror` dedup, historical lockfile declutter | M | M | M | **postpone** (`0.8.33`) |
| `P1-12` | Slice 1 | Inject root npm `overrides` for `katex@0.18.2` or add fuzzing deps to root workspace | H | H | S | **reject** (policy guard) |
| `P2-01` | Slice 2 | Relocate 10 top-level `*_tests.rs` (`2,421` LOC) out of `fathomdb-engine/src/` (`lib.rs`, `module-boundary-policy.txt`), keeping feature-gated `slice30_observer.rs` | H | L | M | **include** (Slice 20 entry; paired with `P4-04`) |
| `P2-02` | Slice 2 | Split `search.rs` (`3,847` LOC) and `evidence.rs` (`3,628` LOC) into submodules + update `module-boundary-policy.txt` before `scope` edits | H | M | M | **include** (Slice 20 entry; paired with `P4-04`) |
| `P2-03` | Slice 2 | Split `dev/adr/ADR-0.6.0-decision-index.md` into Active/Governing vs Superseded/Historical, fix duplicate `#66` / `#67`, index 20 unindexed ADRs | H | L | S | **include** (Slice 10) |
| `P2-04` | Slice 2 | Create `dev/design/cross-cutting-change-checklist.md`, add 1-page subsystem owner map to `dev/design/README.md`, link in `AGENTS.md`, update `document-lifecycle.json` (`orchestration.md` -> `historical`) | H | L | S | **include** (Slice 10) |
| `P2-05` | Slice 2 | Tighten parity/pin/module-boundary gate failure messages to name exact owner files (`R32-07`) | H | L | S | **include** (Slice 10; paired with `P7-05`) |
| `P2-06` | Slice 2 | Rename 33 `slice-135/results/**/Cargo.lock` + 4 `dev/experiments/**/Cargo.lock` to `.snapshot` & delete `dev_requirements_md.txt` | H | L | S | **reject** rename of immutable `slice-135` locks (use `P1-01` API dismissal); **postpone** `dev/` cleanup to `0.8.33` |
| `P2-07` | Slice 2 | Scrub evaluation-era read-only/unscoped/LoCoMo remnants in `fathomdb-mcp` & detach `locomo_load`/`locomo_eval` examples (`keep slice30_m_helper`) | H | M | M | **include** (Slice 10 ADR / Slice 40) |
| `P2-08` | Slice 2 | Commission `release-state-0.8.32.json` + `STATUS-0.8.32.md`, mark `STATUS-0.8.31.md` historical, sync `0.8.32/{plan,scope}.md` with Gate 1 rulings, fix baseline CI | H | L | S | **include** (Slice 10) |
| `P2-09` | Slice 2 | Bulk `dev/` pruning (`0.8.27/`, `runs/`, `DOC-INDEX.md`, `hygeine-roadmap.md`), root stray files, `experiments/` archival | H | M | L | **postpone** (`0.8.33`) |
| `P2-10` | Slice 2 | Retire deprecated `.claude/agents/{orchestrator,sealed-orchestrator,steward}.md`, `steward-orient.sh`, `test_seat_path_guard.sh`, `scripts/repo-prune/` | H | M | M | **postpone** (`0.8.33`) |
| `P3-01` | Slice 3 | Sync `dev/needs.md` & `dev/requirements.md` applicability to `0.8.32`/Schema 38; keep `dev/acceptance.md` LOCKED | H | L | S | **include** (Slice 60) |
| `P3-02` | Slice 3 | Author slice-local `R32-*`/`AC32-*` oracles (incl. `AC32-MCP-SAFETY-01..09`, `AC32-DMFLAKEY-*`, `AC32-FUZZ-*`) & RED->GREEN tests | H | L | M | **include** (Slices 10–50) |
| `P3-03` | Slice 3 | **[HITL Gate 1 Pin Flag]** Update `governed-{operation-parity,surface-allowlist}.json` & `governed-surface-pin.json` for approved `scope`/bulk/`EP-*` additions (`core` 5 & `recovery_denylist` 5 fixed) | M | H | M | **include** (HITL Gate 1 Decision 4; Slices 20/30/40) |
| `P3-04` | Slice 3 | Orientation aids (`cross-cutting-change-checklist.md`, `dev/design/README.md`, `ADR-0.6.0` split, gate diagnostics) | H | L | S | **include** (Slice 10; merged with `P2-03`–`P2-05`) |
| `P3-05` | Slice 3 | Draft and ratify four `dev/adr/ADR-0.8.32-*.md` ADRs (Scope, Read+Write MCP, Opt-in Bulk, Declared Durability/`dm-flakey`) | H | M | M | **include** (Slice 10) |
| `P3-06` | Slice 3 | Immediate factual doc fixes: `PropertyPredicate` 6-op correction across 5 `docs/`, `$.action_kind` in `errors.md`, leftover `0.8.30` prose, consumer-neutral `architecture-v2.md` | H | L | S | **include** (Slice 10; paired with `P6-03`/`P6-05`) |
| `P3-07` | Slice 3 | Update `dev/tools/module-boundary-policy.txt` & gate for `search.rs`/`evidence.rs` splits & test relocations | H | M | M | **include** (Slice 20; merged with `P2-01`/`P2-02`) |
| `P3-08` | Slice 3 | Document single-source `TABLE_MANIFEST` across `wire.md`, `architecture-v2.md`, `fathomdb-schema`, `export_manifest.rs` | H | L | M | **include** (Slice 20) |
| `P3-09` | Slice 3 | Specify non-breaking Schema-38 `scope` activation across `dev/interfaces/*` & `docs/*`; delete `"inert/unused scope"` claims | H | M | L | **include** (Slice 20) |
| `P3-10` | Slice 3 | Specify opt-in bulk write mode across interfaces, SDK docstrings (`write.rs`, `types.py`, `write.ts`), and `docs/reference/*` | H | M | M | **include** (Slice 30) |
| `P3-11` | Slice 3 | Update `dev/inspector/*` & `docs/inspector/*` for active Schema-38+ `scope` UI subordinate to `D-01` nondisclosure | H | L | M | **include** (Slice 30) |
| `P3-12` | Slice 3 | Upgrade `dev/interfaces/mcp.md` & `dev/design/mcp-adapter/*` for `allow_writes: false` default, per-store opt-in + bound `scope`, startup conflicts, governed writes/checkpoints, scoped evidence, `AC32-MCP-SAFETY-01..09` | H | H | L | **include** (Slice 40) |
| `P3-13` | Slice 3 | Package `fathomdb-mcp` per Owner Ruling 3 (`LICENSE`, `README.md`, `src/python-mcp/`, `publish = true`, `fathomdb[mcp]`, workflows, `AGENTS.md:31`) | H | M | M | **include** (Slices 10/40; merged with `P1-04`/`P1-05`) |
| `P3-14` | Slice 3 | Create `docs/guides/mcp-server.md` & `docs/reference/mcp.md` and update `docs/{use-cases,install,reference,operations,positions}/*` | H | L | M | **include** (Slice 40) |
| `P3-15` | Slice 3 | Reconcile accepted `EP-*` engine proposals (`EP-01`, `EP-07`, `EP-10` scope) into interfaces & reference docs | M | M | M | **include** (Slice 40; per Decision 4) |
| `P3-16` | Slice 3 | Finalize internal `fathomdb.logical-export/v1` contract & rebuild docs keeping CLI `export`/`import` non-public until `0.8.34` | H | M | M | **include** (Slice 50) |
| `P3-17` | Slice 3 | Document declared WAL/`SyncMode` durability model, `dm-flakey` fault oracle, and fuzzing specs | H | M | M | **include** (Slice 50) |
| `P3-18` | Slice 3 | Pre-publish `docs/release-notes/0.8.32.md`, `generate-public-docs.py` (`SOURCE_URL` + `src/python-mcp`), regenerated `docs/generated/*`, `published: null` intact | H | L | M | **include** (Slice 60) |
| `P3-19` | Slice 3 | Post-HITL publication truth flip (`RELEASE_SCHEMAS["0.8.32"] = 38`, published notices, `CHANGELOG.md`, live consumer receipts) | H | L | S | **include** (Slice 70) |
| `P3-20` | Slice 3 | Bulk `dev/` archival, OSS docs IA overhaul, `hygeine-roadmap` rename, Steward removal | H | M | L | **postpone** (`0.8.33`) |
| `P3-21` | Slice 3 | Public `fathomdb export`/`import` CLI, personal-agent kits, public benchmarks, CUDA 13 delivery | H | L | L | **postpone** (`0.8.34`–`0.8.35+`) |
| `P4-01` | Slice 4 | **[CRITICAL]** Schema-38 `vector_default` compat: accept 7-col & 8-col in `ensure_vector_partition`, lazy/boot `reshape_vector_partition_nondestructive` (fixing `canonical_edges` join), read-only fallback | H | H | M | **include** (Decision 1; Slices 10 ADR / 20) |
| `P4-02` | Slice 4 | **[CRITICAL]** Store-wide `logical_id`/`(collection, key)` uniqueness w/ strict same-scope ownership guard under Schema 38, store-global `operational_collections`, idempotent partial `(scope, ...)` indexes `WHERE scope != ''` | H | H | M | **include** (Decision 1; Slices 10 ADR / 20) |
| `P4-03` | Slice 4 | Unify all 7 fragmented table inventories into declarative `TABLE_MANIFEST` in `fathomdb-schema` (fixing 7+ tables missing from `write_commit.rs:448-470`) | H | M | M | **include** (Slice 20) |
| `P4-04` | Slice 4 | Zero-behavior-change Slice 20 entry: 10 `*_tests.rs` relocation + `search.rs`/`evidence.rs` submodule splits honoring `REQUIRED_FORBIDDEN_*` & `owner` rules | H | M | M | **include** (Slice 20 entry) |
| `P4-05` | Slice 4 | Opener-bound anti-spoof `ScopePolicy` across `write`, `actuate`, `transition`, `purge`, `erase_source` + cross-scope endpoint refusal | H | H | M | **include** (Slice 20) |
| `P4-06` | Slice 4 | Full read/search/evidence/graph/continuation `scope` threading with zero-overhead `scope == ""` fast path (gating FTS fast paths, `ownerless_existence_sql`, `read_historical_projected_text` IDF, `vec0` pre-cap, `FrozenReadContextV1`/`PageCursor`, `graph_expand` + `search_expand` `ReadView`) | H | H | L | **include** (Slice 20) |
| `P4-07` | Slice 4 | **[CRITICAL]** Reconcile `open.rs:1262` `PRAGMA synchronous = NORMAL`: honest `NORMAL` durability contract + opt-in `SyncMode::{Normal, Full}`, qualifying both in `dm-flakey` | H | M | M | **include** (Decision 2; Slices 10 ADR / 50) |
| `P4-08` | Slice 4 | Opt-in `WriteOptions`/`write_with_options` bulk mode restoring real `dangling_edge_endpoints`, keeping `write`/`actuate` strict by default, plus `fathomdb doctor` dangling/cross-scope audit | H | M | M | **include** (Slice 30) |
| `P4-09` | Slice 4 | Extend `src/inspector/` (`Filter.scope`, `lexical`/`vector`/`graph`, per-scope `inventory`) over `TABLE_MANIFEST` subordinate to `D-01` nondisclosure | H | L | M | **include** (Slice 30) |
| `P4-10` | Slice 4 | **[CRITICAL]** Atomic governed write + `latest_state` checkpoint Engine/SDK seam (`PutLatestState` + scope-bound `_fathomdb_actuation_receipts`) preserving `MR-070.2` | H | H | M | **include** (Decision 4; Slices 20/40) |
| `P4-11` | Slice 4 | Promote `EP-07` (`create_if_missing: false`) & `EP-01` (`read::visibility_state`) into `fathomdb-sdk`; defer `EP-05`, `EP-10` `origin_class`, `EP-16` | M | M | M | **include** (Decision 4; Slices 10 ADR / 40) |
| `P4-12` | Slice 4 | Graduate `fathomdb-mcp` (`allow_writes: false` default, per-store opt-in + bound `scope`, startup conflicts, scoped frozen evidence, `(store, epoch, scope)` session isolation, `AC32-MCP-SAFETY-01..09`, 3-channel packaging) | H | H | L | **include** (Slices 10/40) |
| `P4-13` | Slice 4 | Build internal `fathomdb.logical-export/v1` archive codec & full rebuild-equivalence verifier (`DurableAuthority` + `OptInVector` f32, omitting `embedding_bin`/`mean_vec`) | H | M | M | **include** (Slice 50) |
| `P4-14` | Slice 4 | Execute `/dev/kvm` Ubuntu 24.04 + guest `dm-flakey` power-loss suite + isolated `fuzz/` workspace (`nightly-2026-08-01` + `cargo-fuzz` + `ASan`) | H | M | L | **include** (Slices 10 prep / 50) |
| `P4-15` | Slice 4 | Execute Slice 10 `Prework Prep` first, then Slice 60 `QUALIFICATION` and HITL-gated Slice 70 `PUBLICATION` | H | L | M | **include** (Slices 10, 60, 70) |
| `P5-01` | Slice 5 | Commit `dev/fixtures/golden-0.8.30-schema37.sqlite` + fix `migrate_cli.rs` & `slice50_migrate_37_to_38.rs` + fix 4 `verify-fast` suites | H | L | S | **include** (Slice 10; paired with `P7-01`–`P7-04`) |
| `P5-02` | Slice 5 | Add shell contract tests asserting tightened `R32-07` diagnostic file pointers | H | L | S | **include** (Slice 10) |
| `P5-03` | Slice 5 | Ratify `PRAGMA synchronous` durability posture + pre-stage Ubuntu 24.04 QEMU/KVM (`dm_flakey.ko`) guest image + `cargo-fuzz` smoke | H | H | M | **include** (Slice 10; paired with `P4-07`/`P8-04`/`P8-05`) |
| `P5-04` | Slice 5 | Zero-regression test parity across `search.rs`/`evidence.rs` splits & 10 `*_tests.rs` moves (`test-feature-matrix.toml`) | H | M | M | **include** (Slice 20) |
| `P5-05` | Slice 5 | Declarative `TABLE_MANIFEST` completeness & negative unknown-table tests in `fathomdb-schema` | H | L | M | **include** (Slice 20) |
| `P5-06` | Slice 5 | `scope_isolation.rs` + Python/TS parity tests (2-caller overlapping corpus across all routes, stale ref/cursor revocation, anti-spoof) | H | H | L | **include** (Slice 20) |
| `P5-07` | Slice 5 | `scope_query_plans.rs` (`EXPLAIN QUERY PLAN` `SEARCH`-only + single-scope fast-path & property equivalence oracles) + CI-portable Schema-38 compat tests | H | M | M | **include** (Slice 20) |
| `P5-08` | Slice 5 | `bulk_write_opt_in.rs` + `doctor_dangling_edges.rs` + Python/TS parity tests | H | M | M | **include** (Slice 30) |
| `P5-09` | Slice 5 | Inspector Rust + Playwright scope/`D-01` tests AND wire Inspector JS unit + Playwright smoke into CI/qualification | H | M | M | **include** (Slices 30, 60) |
| `P5-10` | Slice 5 | `fathomdb-mcp` fail-fast startup conflict matrix (`SC-01`..`SC-10`) across `--check-config` (`exit 2`, zero open) & `Adapter::new` / `register_engine` | H | H | M | **include** (Slice 40) |
| `P5-11` | Slice 5 | `mcp_write_safety.rs` (`AC32-MCP-SAFETY-01..09`) + `mcp_frozen_evidence.rs` (SDK-vs-MCP exact equivalence) + blocking cancel/shutdown fence (`bridge.rs`/`registry.rs`) + secret-scrub tests | H | H | L | **include** (Slice 40) |
| `P5-12` | Slice 5 | `fathomdb-mcp` packaging gates (`cargo package --locked` dry-run, `check_python_wheel_contents.py --kind mcp`, `AC-041` empty base scripts, 16 contract tests) | H | M | M | **include** (Slices 40, 60–70) |
| `P5-13` | Slice 5 | `logical_export_roundtrip.rs` + installed-artifact `qualify-backup-restore-rebuild.sh` (`R32-04`) | H | H | L | **include** (Slices 50, 60) |
| `P5-14` | Slice 5 | Disposable QEMU/KVM + `dm-flakey` power-loss fault campaign with external ack oracle, `doctor --full`, and subsequent writes (`R32-05`) | H | H | L | **include** (Slice 50) |
| `P5-15` | Slice 5 | Standalone `fuzz/` workspace (`7` targets + `ASan`/`UBSan` + committed deterministic regression tests) | H | M | L | **include** (Slice 50) |
| `P5-16` | Slice 5 | Resource-leak checks (`AC32-LEAK-01..02`: MCP + SDK FDs/WAL/tasks), domain state-machine `proptest`/Hypothesis replacing 12-case stubs, targeted `0.8.32` mutant kills | H | M | M | **include** (Slices 20, 40, 50) |
| `P5-17` | Slice 5 | Paired `0.8.31` vs `0.8.32` 10k/50k/100k latency/RSS/DB-size gate (`200.4 MB` headroom watch) + scoped 50k/100k check + SHA-256 `golden-0.8.31-schema38.sqlite` proof | H | M | M | **include** (Slice 60) |
| `P5-18` | Slice 5 | Post-publish `smoke-crates-mcp.sh`, clean-venv PyPI `fathomdb[mcp]` smoke, 5-target GH Release stdio smokes, and archive golden `0.8.32` Schema-38 DB | H | M | M | **include** (Slices 60–70) |
| `P6-01` | Slice 6 | Create `cross-cutting-change-checklist.md`, 1-page `dev/design/README.md`, Active/Superseded `ADR-0.6.0-decision-index.md`, link in `AGENTS.md` + `docs/contributing/index.md` | H | L | M | **include** (Slice 10; merged with `P2-03`/`P2-04`) |
| `P6-02` | Slice 6 | Tighten parity/pin/module-boundary error messages + fix `dev/plans/0.8.32/scope.md:128` (`config.rs` -> `module-boundary-policy.txt`) | H | L | S | **include** (Slice 10; merged with `P2-05`/`P7-05`) |
| `P6-03` | Slice 6 | Purge residual `0.8.30` install commands/`blob/release/0.8.30/` links across 13+ `docs/` & 8 `src/**/README.md`; add per-target CUDA+CE wheel matrices & split glibc cells (`docs/compatibility/index.md:116`) | H | L | M | **include** (Slice 10 & Slice 60/70) |
| `P6-04` | Slice 6 | Sync `dev/ROADMAP.md`, `PROGRAM-SEQUENCING.md`, `DOC-INDEX.md`; add `"0.8.32": 38` + stale-version negative check to `check-public-doc-truth.py` | H | L | S | **include** (Slice 10) |
| `P6-05` | Slice 6 | Generalize consumer-project framing out of 2 maintained `dev/design/` owner docs (`architecture-v2.md`, `nested-source-projections.md`) & replace `"agent:hermes"` in `engine.py:574` | H | L | S | **include** (Slice 10) |
| `P6-06` | Slice 6 | Update `dev/interfaces/*` & `docs/reference/*` for activated Schema-38 `scope` | H | M | M | **include** (Slice 20; merged with `P3-09`) |
| `P6-07` | Slice 6 | Update interfaces, SDK reference docs, & active `docs/inspector/*` for opt-in bulk & Inspector scope UI | H | M | M | **include** (Slice 30; merged with `P3-10`/`P3-11`) |
| `P6-08` | Slice 6 | Lockstep `fathomdb-mcp` docs (`README.md`, `Cargo.toml`, `dev/interfaces/mcp.md`, `docs/use-cases.md`, `AGENTS.md:31`, `release-and-packaging-traps.md`) | H | M | M | **include** (Slices 40, 60–70; merged with `P3-12`–`P3-14`) |
| `P6-09` | Slice 6 | Update `docs/operations/index.md`, `compatibility/index.md`, `cli.md` with declared `0.8.32` durability/fault/fuzzing contract | H | L | S | **include** (Slice 50; merged with `P3-17`) |
| `P6-10` | Slice 6 | Final `0.8.32` release-truth sweep (`SOURCE_URL`, `docs/generated/*`, `mkdocs build --strict`, `0.8.32.md`, `CHANGELOG.md`) | H | L | S | **include** (Slices 60–70; merged with `P3-18`/`P3-19`) |
| `P6-11` | Slice 6 | Defer repo/root declutter, `dev/` pruning, auto-doc pipeline redesign, `docs/architecture/`, `docs/mcp/` host guides, `docs/recipes/` | H | L | L | **postpone** (`0.8.33`) |
| `P7-01` | Slice 7 | Track `dev/fixtures/golden-0.8.30-schema37.sqlite` + `CARGO_MANIFEST_DIR` lookup in `migrate_cli.rs` & `slice50_migrate_37_to_38.rs` | H | L | S | **include** (Slice 10) |
| `P7-02` | Slice 7 | Derive latest `+tegra` version dynamically from `tegra-pages-retained.json` in `test_tegra_publication_operator_path.sh` + fix `docs/reference/cli.md:182` | H | L | S | **include** (Slice 10) |
| `P7-03` | Slice 7 | Regenerate `docs/generated/contributor-map.md` & wire `generate-public-docs.py --check` into `scripts/agent-lint.sh` | H | L | S | **include** (Slice 10) |
| `P7-04` | Slice 7 | Move Pyright preflight before Rust checks in `scripts/agent-typecheck.sh` & copy/stub `check-release-smoke-sdk-compile.sh` in `test_agent_typecheck_pyright_version.sh` | H | L | S | **include** (Slice 10) |
| `P7-05` | Slice 7 | Tighten failure diagnostics in `check-sdk-surface-parity.py`, `check-governed-surface-pin.sh`, and `module-boundary-gate/src/{main,config}.rs` | H | L | S | **include** (Slice 10) |
| `P7-06` | Slice 7 | Wire `fathomdb-mcp` `crates.io` publishing (`publish-rust-t7-mcp`, `cargo-publish-if-new.sh`, `verify-release-gates.sh`, `smoke-crates-mcp.sh`, first-publish bootstrap) | H | M | M | **include** (Slice 10 publisher / Slice 40 wiring) |
| `P7-07` | Slice 7 | Wire `src/python-mcp/` 5-target `build-python-mcp` (`manylinux_2_28` glibc-checked binary reused for GH Release archives + `.sha256`), `publish-pypi`, `github-release` in BOTH `release.yml` & `promote-npm-release.yml`, `set-version.sh`, `check-license-consistency.sh`, `check_python_wheel_contents.py`, `wait-for-registry-version.py` | H | M | L | **include** (Slice 10 Pending Publisher / Slice 40 wiring) |
| `P7-08` | Slice 7 | Update all 16 static workflow/contract tests alongside MCP pipeline edits (replacing hardcoded `upload-artifact-retry` count `15`) | H | M | M | **include** (Slice 40) |
| `P7-09` | Slice 7 | Freeze `fathomdb-mcp` release feature set (`--features default-embedder` for shipped wheels/binaries vs `default = []` crate) in Slice 10 ADR | H | L | S | **include** (Decision 5; Slice 10 ADR) |
| `P7-10` | Slice 7 | Add `fathomdb-mcp` + `src/python-mcp/pyproject.toml` assertions to `test_release_version_surfaces.sh` & `test_verify_release_gates.sh`, and `"0.8.32": 38` in `check-public-doc-truth.py` | H | L | S | **include** (Slices 40, 60–70) |
| `P8-01` | Slice 8 | Record `2026-11-24` CUDA version review summary in `0.8.32/plan.md`, keeping all `ADR-0.8.28` importance tiers & `windchill3` R580/CUDA 12.6 + Orin JetPack 6 unchanged | H | L | S | **include** (Slice 10) |
| `P8-02` | Slice 8 | Keep CUDA 13 delivery deferred post-`0.8.35` (logging `vLLM`/`PyTorch 2.15`/`JetPack 7.2.2`/`6111901` as inputs) | H | L | S | **postpone** (post-`0.8.35`) |
| `P8-03` | Slice 8 | Record standing tripwires `T1`–`T3` = NOT FIRED in `0.8.32/plan.md` & retarget ledger todo `TC-2a9f89fa` (`0.8.29` -> `0.8.32`) via `ledgerwrite` | H | L | S | **include** (Slice 10) |
| `P8-04` | Slice 8 | Pre-stage SHA-256-pinned Ubuntu 24.04 (`glibc >= 2.39`, `linux-generic` w/ `dm_flakey.ko`) guest image + `xorriso` NoCloud `cidata` under `/mnt/data/worktrees/` + pass 1 `/dev/kvm` boot smoke | H | M | M | **include** (Slice 10 prep / Slice 50) |
| `P8-05` | Slice 8 | Install `cargo-fuzz --locked` & verify minimal `nightly-2026-08-01` `libfuzzer-sys` + `ASan` link & coverage tooling in Slice 10 prep | H | M | S | **include** (Slice 10 prep) |

---

## 3. Proposed Phase B Slice Ladder (Pending HITL Gate 1 Approval)

| Slice | Codename | Scope & Integrated Prework Proposals |
| --- | --- | --- |
| **Slice 0–9** | `PREWORK` | **COMPLETE (awaiting HITL Gate 1):** Branch `dev/0.8.32` & worktree `/mnt/data/worktrees/release-0.8.32`, host/CI/Dependabot/CUDA/tripwire/venue audits (`slice-0..8.md`), scored triage (`slice-9-triage.md`). |
| **Slice 10** | `PREWORK-PREP` | Provision `.venv` & `node_modules`; commission `release-state-0.8.32.json` & `STATUS-0.8.32.md`; fix all 5 baseline CI failures (`P7-01`–`P7-04`); dismiss `106` Dependabot alerts & prune `7` branches (`P1-01`–`P1-03`); land `R32-07` orientation & gate diagnostics (`P2-03`–`P2-05`, `P5-02`, `P7-05`); fix stale `0.8.30`/6-op/consumer docs (`P3-06`, `P6-03`–`P6-05`); record CUDA review & tripwires + retarget `TC-2a9f89fa` (`P8-01`, `P8-03`); ratify 4 `ADR-0.8.32-*` ADRs (`P3-05`, `P4-01/02/07/11`, `P7-09`); stage Ubuntu 24.04 `dm_flakey.ko` QEMU guest + `cargo-fuzz` smoke (`P8-04`, `P8-05`); bootstrap `fathomdb-mcp` PyPI Pending / crates.io Trusted Publishers (`P1-05`). |
| **Slice 20** | `SCOPE-AND-REFACTOR` | Zero-behavior-change relocation of 10 `*_tests.rs` & `search.rs`/`evidence.rs` splits (`P2-01`, `P2-02`, `P4-04`, `P5-04`); single-source `TABLE_MANIFEST` (`P4-03`, `P5-05`); non-breaking Schema-38 `scope` activation across `vec0` (`P4-01`), ownership-guarded uniqueness + partial indexes (`P4-02`), write/actuate/erasure (`P4-05`), all read/search/graph/frozen/evidence routes w/ zero-overhead `""` fast path (`P4-06`), Rust/Py/TS SDK parity (`Pin A`), `scope_isolation.rs` + `scope_query_plans.rs` (`P5-06`, `P5-07`). |
| **Slice 30** | `BULK-AND-INSPECTOR` | *(Parallelizable with Slice 40 after Slice 20)* Opt-in bulk/non-strict write mode (`Pin B`, strict default & strict `actuate` preserved, `dangling_edge_endpoints`, `fathomdb doctor`, `P4-08`, `P5-08`); Inspector active-`scope` UI + `TABLE_MANIFEST` + `D-01` nondisclosure + Playwright CI wiring (`P4-09`, `P5-09`). |
| **Slice 40** | `MCP-RW` | *(Parallelizable with Slice 30 after Slice 20)* Atomic governed write + `latest_state` checkpoint Engine/SDK seam + `EP-07`/`EP-01` (`Pin C`, `P4-10`, `P4-11`); production read+write `fathomdb-mcp` (`allow_writes: false` default, per-store opt-in + bound `scope`, `SC-01..SC-10` startup conflict matrix, `AC32-MCP-SAFETY-01..09`, blocking cancel/shutdown fence, scoped frozen evidence, SDK-vs-MCP exact equivalence, `P4-12`, `P5-10`, `P5-11`); 3-channel packaging + `src/python-mcp/` + 16 contract tests (`P1-04`, `P1-05`, `P5-12`, `P7-06`–`P7-08`, `P7-10`). |
| **Slice 50** | `DURABILITY-AND-FUZZ` | Internal `fathomdb.logical-export/v1` codec + backup/restore/rebuild equivalence proof (`P4-13`, `P5-13`); `SyncMode::{Normal, Full}` + `/dev/kvm` Ubuntu 24.04 `dm-flakey` power-loss fault campaign (`P4-07`, `P4-14`, `P5-14`); isolated `fuzz/` (`7` targets + `ASan`/`UBSan` + committed regressions, `P1-06`, `P5-15`); MCP/SDK resource-leak + state-machine/mutant checks (`P5-16`). |
| **Slice 60** | `QUALIFICATION` | Full unconfined `./scripts/agent-verify.sh` + `AGENT_LONG=1 ./scripts/check.sh`, paired `0.8.31` vs `0.8.32` 10k/50k/100k latency/RSS/DB-size gate + scoped micro-gate (`P5-17`), installed-artifact backup/rebuild proof, Inspector & `fathomdb-mcp` candidate smokes, release docs (`P3-01`, `P3-18`), golden `0.8.32` Schema-38 archive (`P5-18`). |
| **Slice 70** | `PUBLICATION` | **HELD for separate HITL publish approval** — tag `v0.8.32`, multi-registry publish (`crates.io` incl. `fathomdb-mcp`, PyPI `fathomdb`/`fathomdb-cli`/`fathomdb-mcp`, npm, 5-target GH Releases incl. Inspector & `fathomdb-mcp`, Tegra Pages), post-publish consumer smokes, final truth flip (`P3-19`). |

---

## 4. HITL Gate 1 Decisions (Approved 2026-11-24)

- **Decision 1 (Schema-38 non-breaking vs. Schema-39 breaking `scope` layout) — EXPERIMENT APPROVED:** Run a controlled 100k-node / 150k-edge benchmark experiment in Slice 10 (`WP-10D`, comparing Baseline `0.8.31` Schema 38 vs. Candidate A Non-breaking Schema 38 + partial indexes `WHERE scope != ''` vs. Candidate B Breaking Schema 39 composite PKs/indexes across both single-scope and multi-scope workloads, plus `SyncMode::Normal` vs. `Full` write latency) before freezing `ADR-0.8.32-caller-scope-isolation.md` and defining Slices 20–70.
- **Decision 2 (WAL `PRAGMA synchronous` durability contract vs. `dm-flakey`) — APPROVED (Option 1):** Keep `SyncMode::Normal` as the library default (amending `ADR-0.6.0-durability-fsync-policy.md` truthfully for WAL checkpoint-bounded power-loss durability and preserving `0.8.31` write speed) AND add opt-in `EngineConfig` `SyncMode::{Normal, Full}` (`PRAGMA synchronous = FULL`) for strict per-acknowledged-commit power-loss durability; qualify **both** profiles in Slice 50's disposable QEMU/KVM `dm-flakey` campaign.
- **Decision 3 (Phase B canonical ladder renumbering) — APPROVED (Option 1):** Adopt `Slice 10` (`PREWORK-PREP`) and feature slices `20–70` (`70 = PUBLICATION`, separately gated).
- **Decision 4 (Governed-surface pin pre-authorization) — APPROVED (Option 1):** Pre-authorize targeted `scripts/governed-surface-pin.json` updates (`core = 5`, `recovery_denylist = 5` fixed) for Pin A (`ScopePolicy` / `scope` / `search_expand` `ReadView`), Pin B (`WriteOptions` / `write_with_options`, default strict & `actuate` strict-only), and Pin C (atomic `PutLatestState` + `EP-07` `create_if_missing: false` + `EP-01` `read::visibility_state`).
- **Decision 5 (`fathomdb-mcp` shipped binary/wheel feature set) — APPROVED (Option 1):** Build 5-target GH Release binaries & PyPI wheels with `--features default-embedder`, keeping `default = []` in `fathomdb-mcp/Cargo.toml`.
- **Decision 6 (Consolidated 102-proposal triage dispositions) — APPROVED (Option 1):** All recommended `include` / `postpone` / `reject` dispositions in § 2 approved.
