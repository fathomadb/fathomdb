---
title: 0.8.31 Prework Slice 9 — Triage and HITL Gate 1
status: APPROVED
target_release: 0.8.31
slice: 9
date: 2026-11-22
---

# 0.8.31 Prework Slice 9 — Triage & HITL Gate 1

Consolidated, scored triage of all prework proposals from **Slices 0–8** (`P1-01`..`P7-10`, **84 total proposals** across `prework/slice-{1..7}.md`) per [`RELEASE-AND-SLICE-PLANNING-TEMPLATE.md`](../../prompts/RELEASE-AND-SLICE-PLANNING-TEMPLATE.md).

## 1. Key architectural & planning decisions requiring HITL Gate 1 sign-off

Before executing **Slice 10 (prework prep)** and moving to Phase B/C feature-slice planning, four concrete decisions need owner confirmation:

1. **CRITICAL (`P4-09`) — `as_of_epoch_ms` on Schema 38:** Direct inspection of `canonical_nodes`, `canonical_edges`, and `_fathomdb_artifact_revisions` revealed **no commit epoch timestamp column** exists today (`superseded_at` stores a `write_cursor` integer).
   - **Recommended Option A:** Add a lightweight `_fathomdb_commit_timestamps(write_cursor INTEGER PRIMARY KEY, committed_at_ms INTEGER NOT NULL)` table in Schema 38 (populated atomically inside `commit_batch` / `actuate` with monotonic timestamps, zero overhead on reads unless `as_of_epoch_ms` is requested; upgraded schema-37 rows retain cursor-only history for pre-upgrade writes), so both `as_of_write_cursor` AND `as_of_epoch_ms` work as promised in `ADR-0.8.31-historical-graph-and-fused-seeding.md`.
   - **Alternative Option B:** Scope `0.8.31` `known_as_of` strictly to `as_of_write_cursor` and amend `ADR-0.8.31-historical-graph-and-fused-seeding.md` to defer `as_of_epoch_ms`.
2. **Slice renumbering to match canonical template (`Phase B`):** Adopt `Slice 10 = Prework Prep` (approved hygiene, security, CI/release-friction, and ADR/index prep from this triage) and shift the feature-slice ladder by +10 to start at **Slice 20** (`20` Property Schema 38+ & W12, `30` Strict Integrity + Predicates + Fused Seeding, `40` Historical Graph `as_of`, `50` Memex SDK Unblockers + `fathomdb migrate` + `fathomdb-cli` Wheel, `60` Inspector Upgrade + Scoped CI/Timeouts, `70` Full Qualification + Paired 10k/50k/100k Latency Gate, `80` Authorized Publication — held for separate HITL approval).
3. **Prework triage bundle (`Slice 10` scope vs. feature slices vs. `0.8.33` deferrals):** Confirm the recommended `include` / `postpone` / `reject` dispositions below (especially whether to include `P1-01`/`P1-02`/`P1-03` dependency/branch cleanups and `P7-03`/`P7-07`–`P7-10` CI/release-friction fixes in Slice 10).
4. **Governed surface pin pre-authorization (`P3-03` / `P5-08`):** Confirm that Slice 20 (`read.operational_collections`, `read.operational_collection`) and Slice 50 (`resolve_evidence_many`) may update `src/conformance/governed-{operation-parity,surface-allowlist}.json` and `scripts/governed-surface-pin.json` while preserving the 5 core verbs and 5 `recovery_denylist` names (`AC-041`).

---

## 2. Scored Proposal Triage Table (`P1-01` – `P7-10`)

Legend: **Understood / Risk** = `H` (High) / `M` (Medium) / `L` (Low); **Effort** = `S` (Small) / `M` (Medium) / `L` (Large); **Recommendation** = `include` (with target slice: `Slice 10` prework prep or feature `Slice 20`–`80`) / `postpone` (later release) / `reject` (keep invariant, no change).

| ID | Source Slice | Proposal | Understood (H/M/L) | Risk (H/M/L) | Effort (S/M/L) | Recommendation (`include` / `postpone` / `reject`) |
| --- | --- | --- | --- | --- | --- | --- |
| `P1-01` | Slice 1 | Adopt root `markdownlint-cli2 ^0.24.0 -> ^0.24.2` (`re2js 2.9.1`, `js-yaml 5.5.0`) + online `npm audit` check of remaining devDep findings, keeping `overrides` empty | H | L | S | `include` (Slice 10) |
| `P1-02` | Slice 1 | Narrow `Cargo.lock`-only bump `rustix 1.1.4 -> >=1.1.6` closing `RUSTSEC-2026-0486` (`tempfile`/`which`) with zero product-pin movement | H | L | S | `include` (Slice 10) |
| `P1-03` | Slice 1 | Prune 6 already-satisfied/superseded remote `origin/dependabot/*` branches (`rustls-0.23.46`, `dompurify-3.4.17`, `multi-010aad1c49`, `multi-68e90142e5`, `certifi-2026.10.18`, `setuptools-83.0.0`) | H | L | S | `include` (Slice 10 — optional / non-blocking) |
| `P1-04` | Slice 1 | Retain 0.8.30 Slice 60 `cudarc-fathomdb =0.21.0` + `candle-*-fathomdb =0.13.0` fork cohort and HTTPS mirrors | H | L | S | `reject` (no change; keep invariant) |
| `P1-05` | Slice 1 | Retain `rusqlite 0.41.1`, `libsqlite3-sys 0.39.1` (SQLite `3.54.1`), `sqlite-vec =0.1.9`, `dunce =1.0.5` | H | L | S | `reject` (no change; keep invariant) |
| `P1-06` | Slice 1 | Retain `pyo3 0.29.0`, `napi 2.16.17`, `rmcp =3.5.1`, `tokio 1.54.1`, `ureq`/`rustls`/`webpki-roots`, `httpmock`, `ort` | H | L | S | `reject` (no change; keep invariant) |
| `P1-07` | Slice 1 | Retain `ruff==0.15.17`, `pyright==1.1.410`, `maturin>=1.9,<2`, `mkdocs==1.6.1`, Rust `1.95.0`, Mermaid `12.2.0`, Dependabot pause | H | L | S | `reject` (no change; keep invariant) |
| `P1-08` | Slice 1 | Retire EOL Node `25.9.0` (and Tegra `npm 11.12.1`) from release/Tegra workflows & reconcile `packageManager` vs npm `12.x` | H | M | M | `postpone` (dedicated release-tooling slice post-0.8.31) |
| `P1-09` | Slice 1 | Refresh stale `src/python/uv.lock` (`0.8.20`) and deduplicate Rust `tokenizers`/`sha2`/`thiserror` multi-versions | H | M | M | `postpone` (0.8.32/0.8.33; `tokenizers` needs parity proof) |
| `P1-10` | Slice 1 | Hold ONNX exporter bumps (`filelock 4.2.1`, `setuptools 84.1.0`, `transformers 5.10.1`) until coordinated exporter refresh | H | M | M | `postpone` (coordinated exporter refresh) |
| `P2-01` | Slice 2 | Archive repo-root stray working files (`0.8.23-release-todo.md`, `CORPUS-01-*`, `plan-slice-85-*`) | H | L | S | `postpone` (0.8.33 Slice 10 `R33-01`) |
| `P2-02` | Slice 2 | Update `AGENTS.md` workspace crate count (`12 -> 13` with `fathomdb-inspector`) and prepend `DEPRECATED` headers to retired `.claude/agents/{orchestrator,sealed-orchestrator,steward}.md` (defer archive of legacy seats/orient to 0.8.33) | H | L | S | `include` (Slice 10) |
| `P2-03` | Slice 2 | Update public docs stale `0.8.30` / schema-37 / `use_graph_arm` / CUDA-CE / Inspector text (`check-public-doc-truth.py`) | H | L | M | `include` (Slices 50, 60, 70) |
| `P2-04` | Slice 2 | Lightweight factual `0.8.31` sync of `dev/DOC-INDEX.md`, `dev/design/fathomdb-data-plane-architecture-v2.md`, and `ADR-0.6.0-decision-index.md` #72 (defer slimming/`hygeine-roadmap` rename to 0.8.33) | H | L | S | `include` (Slice 10 + Slice 70) |
| `P2-05` | Slice 2 | Update legacy dangling-edge write tests & write-commit comments for strict refusal (`ADR-0.8.31 #74`) | H | M | M | `include` (Slice 30) |
| `P2-06` | Slice 2 | Archive historical `dev/plans/` bulk tree (`0.8.27/` 6,915 files, `runs/` 1,508 files, pre-0.8.30 plans) via HITL prune manifest | H | M | M | `postpone` (0.8.33 Slice 0/10) |
| `P2-07` | Slice 2 | Archive frozen subtrees (`dev/progress/`, `dev/turso-watch/`, `dev/prototypes/l2-router/`, `dev/roadmap/`, etc.) | H | L | S | `postpone` (0.8.33 Slice 10) |
| `P2-08` | Slice 2 | Archive `dev/inspector/plans/{0.1,0.2.0}/` & retire `dev/design/db-inspector/*` redirect stubs | H | L | S | `postpone` (0.8.33 Slice 10) |
| `P2-09` | Slice 2 | Archive `scripts/repo-prune/` and remove coupled `test_md_generators.sh` / ratchet hooks | H | M | S | `postpone` (0.8.33 Slice 10) |
| `P2-10` | Slice 2 | Audit/archive `scripts/slice135_*` (171 files) and `scripts/test-slice30-*` actively wired in `agent-test.sh` & `ci.yml` | H | M | M | `postpone` (0.8.33 / 0.8.35) |
| `P2-11` | Slice 2 | Review `scripts/sbom-survey/` (keep `perf-experiments/` and `gcp-lab/`) | H | L | S | `postpone` (0.8.33) |
| `P2-12` | Slice 2 | Delete empty `test/` placeholder tree and archive unwired `tests/experiments/` (75 files) / `dev/experiments/` | H | L | M | `postpone` (0.8.33 / 0.8.35) |
| `P2-13` | Slice 2 | Decouple `src/inspector/tools/local-ci.sh` & `local-release.py` from full-monorepo default / standalone branch flow (`R31-08`) | H | L | M | `include` (Slice 60; paired with `P7-01`/`P7-04`/`P7-06`) |
| `P2-14` | Slice 2 | Keep authoritative append-only ledgers, ADRs, interfaces, and `document-lifecycle.json` | H | L | S | `reject` (no change; keep invariant) |
| `P3-01` | Slice 3 | Update `dev/needs.md` & `dev/requirements.md` applicability pointers to schema 38, `0.8.31`, and narrow `fathomdb migrate` root (leave `dev/acceptance.md` LOCKED) | H | L | S | `include` (Slice 70) |
| `P3-02` | Slice 3 | Author slice-local `R31-*` refinements, `AC31-*` oracles, and named TDD bindings in feature slice plans (`20`–`60`) | H | L | M | `include` (Phase C / Slices 20–60) |
| `P3-03` | Slice 3 | Register `read.operational_collections`/`read.operational_collection` (Slice 20) & `resolve_evidence_many` (Slice 50) in `governed-{operation-parity,surface-allowlist}.json` & `governed-surface-pin.json` (with HITL sign-off) | H | M | S | `include` (Slices 20 & 50) |
| `P3-04` | Slice 3 | Update `dev/interfaces/{rust,rust-sdk,python,typescript,wire}.md` for Schema 38 inline `properties`, `canonical_property_values`, `scope`, and `attributes` vs `properties` disambiguation | H | L | M | `include` (Slice 20 & Slice 30) |
| `P3-05` | Slice 3 | Document app-declared vector kinds (`R1`) and remove stale "fixed 7-kind" exclusivity prose in interfaces & `docs/guides/extractor-provenance.md` | H | L | S | `include` (Slice 20 & Slice 30) |
| `P3-06` | Slice 3 | Document op-collection readback/refusal (`R12`), W12 Autonomous ordering, `completed_pending_erasure`, and `WalAttributionCollector` detail | H | L | S | `include` (Slice 20 & Slice 30) |
| `P3-07` | Slice 3 | Update property-graph public guides & compiling Rust `PreparedWrite` examples (`properties: None`) | H | L | M | `include` (Slice 30) |
| `P3-08` | Slice 3 | Update `Engine::write`/`actuate` interfaces & reference docs for strict endpoint integrity (`ADR-0.8.31 #74`); delete stale flag-and-count prose | H | L | S | `include` (Slice 30) |
| `P3-09` | Slice 3 | Specify `edge_predicates`/`target_predicates`, `graph_seed` (`graph_fts`/`fused_top_k`/`both`), `graph_seed_source`/`graph_hop_depth`, and `SEED_FTS_N` fix in interfaces & `graph-retrieval.md` | H | L | M | `include` (Slice 30) |
| `P3-10` | Slice 3 | Specify historical graph `as_of` interfaces (`GraphExpandRequestV1`/`GraphReadContextV1`, erasure subordination) in `dev/interfaces/*` | H | L | M | `include` (Slice 40) |
| `P3-11` | Slice 3 | Update `docs/guides/{graph-retrieval,frozen-evidence}.md` & references for `as_of` history; delete unconditional superseded-exclusion prose | H | L | S | `include` (Slice 40) |
| `P3-12` | Slice 3 | Specify `resolve_evidence_many`, `search(hints=)`, and `OpenReport` booleans in interfaces & reference docs | H | L | M | `include` (Slice 50) |
| `P3-13` | Slice 3 | Apply Memex Graph Asks 1, 4, 5 targeted doc fixes (`use_graph_arm`, PyPI CUDA/CE wheel coverage, mixed-`source_id` batch atomicity) | H | L | S | `include` (Slice 50; paired with `P6-02`–`P6-04`) |
| `P3-14` | Slice 3 | Document `fathomdb-cli` PyPI binary wheels + `fathomdb[cli]` extra while keeping `fathomdb` `[project.scripts]` empty (`AC-041`) | H | L | S | `include` (Slice 50) |
| `P3-15` | Slice 3 | Reframe CLI docs to Three-root CLI (`doctor`, `recover`, `migrate`), specify `fathomdb migrate` 37→38, and reserve `fathomdb.logical-export/v1` in `wire.md` | H | L | M | `include` (Slice 50) |
| `P3-16` | Slice 3 | Update `dev/inspector/{needs,requirements,ROADMAP}.md` for Schema 38+ properties, `as_of` history, scoped CI, and timeouts | H | L | S | `include` (Slice 60) |
| `P3-17` | Slice 3 | Update `docs/inspector/*`, `src/inspector/tools/README.md`, and Inspector release notes (`mkdocs.yml`) for `0.8.31` | H | L | M | `include` (Slice 60) |
| `P3-18` | Slice 3 | Remove stale `"CUDA 13 ... planned for 0.8.31"` promises from `README.md`, `docs/compatibility/index.md`, `docs/install/{python,typescript}.md` | H | L | S | `include` (Slice 10) |
| `P3-19` | Slice 3 | Release-wide `0.8.30 -> 0.8.31` pre-publish target flip & 13-crate currency (`check-public-doc-truth.py` green in Slice 10); schema `37 -> 38` in `docs/compatibility/index.md` (Slice 20) & remaining docs (Slice 50/70) | H | L | M | `include` (Slice 10 pre-publish flip; Slice 20 schema 38; Slice 70 sweep) |
| `P3-20` | Slice 3 | Create `docs/release-notes/0.8.31.md`, update `scripts/generate-public-docs.py` `SOURCE_URL`, and regenerate `docs/generated/*` | H | L | S | `include` (Slice 70) |
| `P3-21` | Slice 3 | Post-authorization publication truth flip (`RELEASE_SCHEMAS["0.8.31"] = 38`, published receipts, `CHANGELOG.md`) | H | L | S | `include` (Slice 80 — held for HITL publish approval) |
| `P4-01` | Slice 4 | Enforce `vec0` 16-column metadata budget (exclude `entity_class='edge'`, reserve 1 slot for `0.8.32` `scope`), fix stale "six built-in" comment, and route `Ne`/numeric ranges cleanly via `canonical_property_values` with post-KNN parity | H | H | M | `include` (Slice 20) |
| `P4-02` | Slice 4 | Thread `(body, properties)` through all attribute/vector projectors, backfills, relaxed-`ReadView` filter paths, and `NodeRecord`/`SearchHit`/`Graph*` hydration with `properties > body` precedence | H | M | M | `include` (Slice 20 & Slice 30) |
| `P4-03` | Slice 4 | Reject reserved `"edge_fact"` on custom node vector-kind enrollment; replace `&'static str` `resolve_source_type` and update all SQL 7-kind mirrors (`filter.rs`, `projection_generation.rs`, `search.rs`, `data_plane_integrity.rs`) | H | H | M | `include` (Slice 20) |
| `P4-04` | Slice 4 | Invoke `visibility_coalescing_trigger_sql!` (NOT `read_visibility_coalescing_upgrade_sql!`) for `cpv` in step 38; update `VISIBILITY_TRIGGER_TABLES` (`19 -> 20`), `canonical_batch_has_no_custom_triggers`, `bootstrap_is_fresh`, and `projection_registry_digest` | H | H | S | `include` (Slice 20) |
| `P4-05` | Slice 4 | Enforce `AdminSchema` operational-collection `kind` match in `write_validation.rs` and writer transaction before `ON CONFLICT DO UPDATE` | H | L | S | `include` (Slice 20) |
| `P4-06` | Slice 4 | Keep reserved `scope TEXT NOT NULL DEFAULT ''` strictly inert in Schema 38 and budget 1 `vec0` metadata slot for `0.8.32` | H | L | S | `include` (Slice 20) |
| `P4-07` | Slice 4 | Enforce strict endpoint validation in `commit_batch` (or mode-parameterized) without breaking `actuate`'s per-op `apply_batch_in_transaction` loop; check ALL batch edges including in-batch-superseded ones | H | H | M | `include` (Slice 30) |
| `P4-08` | Slice 4 | Audit `ingest_with_extractor` two-tx edge writes and rewrite inactive-intermediate / convenience-edge fixtures across Rust/Py/TS | H | M | M | `include` (Slice 30) |
| `P4-09` | Slice 4 | **CRITICAL SCHEMA-38 DECISION:** Resolve missing commit epoch timestamps for `as_of_epoch_ms` before Slice 20 freezes Schema 38 (Option A: add `_fathomdb_commit_timestamps(write_cursor, committed_at_ms)` in Schema 38; Option B: narrow `0.8.31` to `as_of_write_cursor` only) | H | H | M | `include` (Decision at HITL Gate 1 -> ADR update in Slice 10 + impl in Slice 20/40) |
| `P4-10` | Slice 4 | Fix `graph_expand` pre-filter `LIMIT`/`work_units`, rewrite `Both` `OR` as `UNION ALL`, select/check missing `t_valid`, index `(from_id\|to_id, superseded_at, kind)` for empty-`edge_kinds` historical range seeks, bound `load_node`/`resolve_seeds` by `known_as_of`, fix terminal early-skip, and update `encode_graph_evidence_request` | H | H | L | `include` (index in Slice 20; execution in Slices 30 & 40) |
| `P4-11` | Slice 4 | Thread `known_as_of` + `properties` through `evidence.rs` (keeping erasure barriers `Unavailable`); fix `search.rs` Seed Source A mixed-edge slot bug & Phase 2 `UNION ALL` + `edge_kinds`/`edge_predicates` | H | M | M | `include` (Slices 30 & 40) |
| `P4-12` | Slice 4 | Implement `sqlite_master` + `ExportClass` completeness audit (handling dynamic `vector_default_*` / FTS5 shadow tables via prefix/suffix rules), pin `_fathomdb_vector_kinds` as `DurableAuthority`, and extend `data_plane_integrity.rs` + `physical_erasure/admission.rs` for `properties` | H | M | M | `include` (Slice 20) |
| `P4-13` | Slice 4 | Keep `src/inspector` compiling green under `cargo clippy --workspace --all-targets` across Slices 20–40 (`Default`/struct-literal updates on `GraphExpandRequestV1`), and lift `INDEXED BY canonical_nodes_logical_active_idx` for historical inspection in Slice 60 | H | M | S | `include` (Slices 20, 30, 40, 60) |
| `P4-14` | Slice 4 | Scope `fathomdb-cli` Maturin `bindings = "bin"` strictly to `--bin fathomdb` (excluding `perf-regression-check` from PyPI `PATH`) and keep architecture/interface docs synced | H | M | S | `include` (Slice 50) |
| `P5-01` | Slice 5 | Pin explicit opt-in semantics for `R1` custom vector kinds before coding so `tc67_unsupported_vector_kind_report` and `slice22_projection_status` stay green | H | M | S | `include` (Slice 20) |
| `P5-02` | Slice 5 | Build Schema 38 inline-properties + `canonical_property_values` + `R1` + `R12` TDD & `proptest`/Hypothesis numeric/codec boundary suite across Rust/Py/TS | H | M | L | `include` (Slice 20) |
| `P5-03` | Slice 5 | Build `sqlite_master`/`ExportClass` negative audit, inline-`properties` physical erasure (nodes + nullable-`body` edges), and W12 Ask 3a/3b/3c regression tests (including cross-source audit retention & release-mode WAL attribution) | H | M | M | `include` (Slice 20) |
| `P5-04` | Slice 5 | Build strict-endpoint atomicity TDD suite (`Engine::write` + `actuate`, all inactive states, in-batch ordering, authorizer zero-read fast path) and update all ~25 convenience-edge tests + Inspector fixtures | H | M | M | `include` (Slice 30; Inspector fixtures in Slice 20/30/60) |
| `P5-05` | Slice 5 | Build typed traversal predicates, `proptest` disabled-optimization equivalence (indexed vs inline), adversarial `>=12`-edge `SEED_FTS_N` slot proof, and `FusedTopK`/`Both` seeding + provenance tests | H | M | L | `include` (Slice 30) |
| `P5-06` | Slice 5 | Build 4-quadrant bitemporal `as_of` matrix, `proptest` snapshot reference oracle, endpoint-version coherence, and erasure non-resurrection tests | H | M | L | `include` (Slice 40) |
| `P5-07` | Slice 5 | Unified `tests/slice38_query_plans.rs` `EXPLAIN QUERY PLAN` `SEARCH`-only regression gate across write-validation, `SearchFilter`, current `graph.expand`, historical `as_of`, and unpredicated fast-path invariance | H | L | M | `include` (Slices 20, 30, 40) |
| `P5-08` | Slice 5 | Build `resolve_evidence_many`, `search(hints=)`, and `OpenReport` cross-SDK tests + lockstep updates to `python_surface_additions.json`, `test_slice130_package_boundary.py`, `test_openreport.py`, `openreport.test.ts`, `sdk-surface-parity.test.ts` | H | M | M | `include` (Slice 50; Slice 20 for `EraseReport` stub delta) |
| `P5-09` | Slice 5 | Build `fathomdb migrate` 37→38 fault-injection/rollback, dangling-vs-superseded preflight, real golden schema-37 DB upgrade, and clean-venv `fathomdb-cli` PyPI wheel tests | H | M | M | `include` (Slice 50) |
| `P5-10` | Slice 5 | Build Inspector schema-38 admission, node+edge properties XSS/display, `as_of` historical graph + 8-flag erasure non-disclosure, dual-feature (`default` + `cached-cpu`), and `test_local_ci.py`/`test_local_release.py` tests | H | M | L | `include` (Slice 60) |
| `P5-11` | Slice 5 | Quantified same-host paired `v0.8.30` (`db271323f`) vs `0.8.31` benchmark at **10k / 50k / 100k** (plus 5k baseline check) with 10,000-resample paired bootstrap 95% CI gate `<= max(5.0%, 1.0 ms)` on p50/p95 | H | M | M | `include` (Slice 70; pin spec in Slice 10 plan update) |
| `P5-12` | Slice 5 | Full unconfined `./scripts/agent-verify.sh`, `AGENT_LONG=1 ./scripts/check.sh`, Inspector `--full-workspace` + `D4 = B`, `fathomdb-cli` clean-venv smoke, and golden schema-38 archive for `0.8.34` | H | L | M | `include` (Slice 70) |
| `P6-01` | Slice 6 | Fix broken `engine.write([])` verify snippets & empty-batch claims across `docs/install/{python,typescript}.md`, `quickstart.md`, `reference/{python,typescript}-api.md` | H | L | S | `include` (Slice 10 or Slice 50; recommend Slice 10 quick fix) |
| `P6-02` | Slice 6 | Fix `use_graph_arm` / `useGraphArm` stale top-10 fused-hit wording & document `0.8.31` `graph_seed` options | H | L | S | `include` (Slice 30 & Slice 50) |
| `P6-03` | Slice 6 | Add exact per-target PyPI/npm CUDA + CE wheel matrix, split Python (`2.28`) vs N-API (`2.39`) glibc in `docs/compatibility/index.md`, drop `0.8.31` CUDA 13 promises | H | L | S | `include` (Slice 50) |
| `P6-04` | Slice 6 | Document mixed-`source_id` single-txn `Engine.write(batch)` atomicity & zero-row rollback on `WriteValidationError` | H | L | S | `include` (Slice 30 & Slice 50) |
| `P6-05` | Slice 6 | Update 35+ hardcoded `schema 37` / "no migration" lines across public, Inspector, and interface docs to schema `38` + `fathomdb migrate` | H | L | M | `include` (Slices 20, 50, 60, 70) |
| `P6-06` | Slice 6 | Reconcile `dangling_edge_endpoints`, 7-kind vector limit, and `graph-retrieval.md` superseded/seeding prose with `0.8.31` behavior | H | L | S | `include` (Slices 20, 30, 40) |
| `P6-07` | Slice 6 | Fix stale error-class count (`44 -> 46` concrete subclasses) in Python/TS API docs and replace hardcoded `44-operation` count in `docs/positions/sdk-parity.md` | H | L | S | `include` (Slice 10 quick fix; re-verified in Slice 50) |
| `P6-08` | Slice 6 | Refresh `dev/interfaces/` headers (`0.8.28 -> 0.8.31`) & `generate-public-docs.py` `SOURCE_URL` | H | L | S | `include` (Slices 20–70) |
| `P6-09` | Slice 6 | Fix `AGENTS.md` crate count (`12 -> 13`), remove stale `prettier` devDep mention, and include `rust-sdk.md`/`mcp.md` in `dev/interfaces/` pointer | H | L | S | `include` (Slice 10; merged with `P2-02`) |
| `P6-10` | Slice 6 | Lightweight header/table sync of `dev/ROADMAP.md` & `dev/plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md` to `0.8.31`–`0.8.35` roadmap (defer full rewrite to `0.8.33`) | H | L | S | `include` (Slice 10) |
| `P6-11` | Slice 6 | Add Slice 0 Python `3.12`/TS preflight & Slice 60 scoped `local-ci.sh` notes to `docs/contributing/index.md` | H | L | S | `include` (Slice 10 + Slice 60) |
| `P6-12` | Slice 6 | Root clutter cleanup, `dev/` pruning, `hygeine-roadmap` rename, `dev/README.md`, `ROADMAP.md`, and OSS community files | H | M | L | `postpone` (0.8.33 Slice 10 `R33-01`) |
| `P6-13` | Slice 6 | Public docs auto-doc pipeline, MkDocs theme/diagrams, `docs/architecture/`, `docs/mcp/`, `docs/recipes/`, product-generic `ingest_with_extractor` vs. `actuate` framing (zero external-project/Memex info in tracked repo files) | H | M | L | `postpone` (0.8.33 Slices 20–50) |
| `P7-01` | Slice 7 | Decouple `src/inspector/tools/local-ci.sh` to `--scoped` default (~5 min when shared core crates unchanged) while retaining `--full-workspace` + fail-closed shared-diff check | H | L | M | `include` (Slice 60) |
| `P7-02` | Slice 7 | Add bounded `timeout --kill-after` + non-blocking `/proc` / `Xvfb` / CDP teardown across Inspector GUI, `strace`, and `launch.rs` harnesses | H | L | M | `include` (Slice 60) |
| `P7-03` | Slice 7 | Add `timeout --kill-after=5s` bounds with explicit exit-124 diagnostics to `scripts/security/{check-no-listen,check-netns-deny-egress,check-netns-deny-egress-catch}.sh` | H | L | S | `include` (Slice 10) |
| `P7-04` | Slice 7 | Retain and regression-lock `D4 = B` Docker-free native Ubuntu 24.04 qualification (static zero-`docker`/`sudo` check) and bump `local-release.py` to `compiled_schema = 38` | H | L | S | `include` (Slice 60) |
| `P7-05` | Slice 7 | Wire lockstep `fathomdb-cli` PyPI binary wheels (`src/python-cli/pyproject.toml` + `fathomdb[cli]` extra) across `set-version.sh`, `check-license-consistency.sh`, `release.yml`, visibility wait, and smoke scripts | H | M | M | `include` (Slice 50 & Slice 70) |
| `P7-06` | Slice 7 | Add `build-inspector` job (`ubuntu-24.04`, `D4 = B` ABI/seal + package browser check) and GitHub Release upload in `.github/workflows/release.yml` | H | M | M | `include` (Slice 60 & Slice 70) |
| `P7-07` | Slice 7 | Poll PyPI PEP 503/691 `/simple/<pkg>/` index (verifying all target wheel filenames) in `wait-for-registry-version.py` + bounded CDN-skew retry in `smoke-pypi-wheel.{sh,ps1}` | H | L | S | `include` (Slice 10) |
| `P7-08` | Slice 7 | Auto-populate GitHub Release body from `CHANGELOG.md` `## [v]<version>` via `body_path:` in `.github/workflows/release.yml` | H | L | S | `include` (Slice 10) |
| `P7-09` | Slice 7 | Add Bash 3 `smoke_complete=1` sentinel to `smoke-{npm-package,crates-cli,crates-sdk}.sh` and Windows `smoke_slice55_installed.py` parity in `smoke-pypi-wheel.ps1` | H | L | S | `include` (Slice 10) |
| `P7-10` | Slice 7 | Derive tag dynamically in `check-tag-cuda-route.sh` from `Cargo.toml` (`+ test_check_tag_cuda_route.sh`) and inspect `--redact=100` tuples before reconciling `gitleaks-history-manifest.json` + `check-gitleaks-history.py` + `test_gitleaks_history_classifier.py` (keep locked `0.8.20`/`0.8.27`/`0.8.30` workflow branches untouched) | H | L | S | `include` (Slice 10) |

---

## 3. Proposed Slice 10 (Prework Prep) Summary vs. Feature Ladder (`Slices 20–80`)

### Slice 10 — Prework Prep (approved at HITL Gate 1; reconciled in FIX-1)

1. **Security & lockfile hygiene:** `P1-01` (root `markdownlint-cli2` `0.24.2` + `npm audit` verification), `P1-02` (`Cargo.lock` `rustix >= 1.1.6`), `P1-03` (optional/non-blocking prune of 6 stale remote `origin/dependabot/*` branches).
2. **ADR / planning alignment before Schema 38 freeze:** Amend ADRs `#73`, `#75`, `#76` (`P4-09` `_fathomdb_commit_timestamps` batch high-water cursor + `actuate_new`, `P4-01`/`P4-06` `vec0` budget, `P4-04` `visibility_coalescing_trigger_sql!`, `P4-10` unified `canonical_edges_{from,to}_sup_kind_idx`, `P5-01` explicit opt-in `R1`, `R31-03` immutable-revision scope vs. in-place `Engine::transition`), amend `ADR-0.8.28-resumable-physical-cleanup.md` for W12 Ask 3a two-truncate Autonomous sequence, scrub external consumer-project specifics from tracked `0.8.31`–`0.8.34` ADRs/plans, and pin `P5-11` execution parameters.
3. **Public-doc truth gate + quick factual doc & contributor fixes:** `P3-18` (remove stale `"CUDA 13 ... planned for 0.8.31"` promises) + `P3-19` pre-publish `0.8.31` target flip across 16 `PUBLIC_DOCS` (so `check-public-doc-truth.py` stays green across Slices 10–60), `P2-02` + `P6-09` (`AGENTS.md` 13 crates / drop `prettier` / `dev/interfaces/` pointer), `P2-04` + `P6-10` (factual `DOC-INDEX`/`architecture-v2`/`ROADMAP`/`PROGRAM-SEQUENCING` sync), `P6-01` (`engine.write([])` broken verify snippets + `consumer-package-root.mts`), `P6-07` (`44 -> 46` concrete error subclasses / dynamic op count), `P6-11` (`docs/contributing/index.md` bootstrap notes).
4. **CI / security / release-friction fixes:** `P7-03` (`strace`/`unshare` timeouts + `FATHOMDB_SECURITY_TIMEOUT_SECS`), `P7-07` (PyPI PEP 691 `/simple/<pkg>/` visibility polling + bounded smoke retry), `P7-08` (auto GitHub Release body from `CHANGELOG.md` with compliant checkout ref), `P7-09` (Bash 3 sentinel + Windows Slice 55 parity), `P7-10` (dynamic `check-tag-cuda-route.sh` + `--redact=100` verified `gitleaks-history` manifest/checker/test reconciliation; keep locked workflow branches untouched).

### Feature Slice Ladder (Phase B, starting at Slice 20)

| Slice | Codename | Scope & Integrated Prework Proposals |
| --- | --- | --- |
| **Slice 0** | `FREEZE-AND-MERGE` | **LANDED (`bf8e905e3`, `42595212c`)** — Inspector `0.2.1` merge, ADRs `#73`–`#76`, W12 review, golden schema-37 DB, `0.8.30` baseline, canonical template, Phase A audits & Slice 9 triage |
| **Slice 10** | `PREWORK-PREP` | Approved Slice 9 prework prep items above (`features/slice-10/plan.md`) |
| **Slice 20** | `PROPERTY-SCHEMA` | Schema 38+ inline `properties`, `canonical_property_values`, `_fathomdb_commit_timestamps`, `vec0` (`P4-01`–`P4-06`, `P4-09`, `P4-12`, `P4-13` Inspector fixtures), custom vector kinds (`R1`, `P5-01`), op-collection readback/refusal (`R12`), W12 Ask 3a/3b/3c, `sqlite_master`/`ExportClass` audit (`P5-02`, `P5-03`, `P5-07`), `docs/compatibility/index.md` schema-`38` sync |
| **Slice 30** | `STRICT-GRAPH` | Strict-only write endpoint integrity (`P4-07`, `P4-08`, `P2-05`, `P5-04`, `P4-13` 3 Inspector fixtures), typed `edge_predicates`/`target_predicates`, opt-in `graph_seed` + provenance, mixed-edge `SEED_FTS_N` slot fix (`P4-10`, `P4-11`, `P5-05`, `P3-04`–`P3-09`) |
| **Slice 40** | `HISTORY-GRAPH` | Historical graph `as_of` traversal + immutable-revision node coherence + evidence/erasure subordination (`P4-09`, `P4-10`, `P4-11`, `P4-13`, `P5-06`, `P5-07`, `P3-10`, `P3-11`) |
| **Slice 50** | `SDK-CLI-UPGRADE` | `resolve_evidence_many`, `search(hints=)`, `OpenReport` booleans (`P5-08`), minimal `fathomdb migrate` 37→38 (`P5-09`), `fathomdb-cli` PyPI wheel (`--bin fathomdb`, `P4-14`, `P7-05`), targeted contract doc fixes (`P3-12`–`P3-15`, `P6-03`, `P6-05`) |
| **Slice 60** | `INSPECTOR` | Serialized after Slice 50 (`release.yml` single-writer): Inspector schema-38+ property + `as_of` history UI (`P5-10`), scoped `local-ci.sh` (`P7-01`), harness timeouts (`P7-02`), `D4 = B` lock (`P7-04`), release CI job (`P7-06`), Inspector docs (`P3-16`, `P3-17`) |
| **Slice 70** | `QUALIFICATION` | Full unconfined `./scripts/agent-verify.sh`, `AGENT_LONG=1`, paired 10k/50k/100k bootstrap latency gate (`P5-11`, `P5-12`), release notes & generated docs (`P3-01`, `P3-20`), golden schema-38 archive |
| **Slice 80** | `PUBLICATION` | **HELD for separate HITL publication authorization** — tag `v0.8.31`, multi-registry publish (`fathomdb` + `fathomdb-cli` + crates + npm + Tegra + Inspector), post-publish smoke, `P3-21` |

---

## 4. HITL Gate 1 Decisions (Approved 2026-11-22)

Recorded durably from owner approval on `2026-11-22`:

1. **`P4-09` (Schema 38 commit timestamps — Option A approved):** Add `_fathomdb_commit_timestamps(write_cursor INTEGER PRIMARY KEY, committed_at_ms INTEGER NOT NULL)` with an index on `(committed_at_ms, write_cursor)` in Schema 38 (populated once per committed batch inside `commit_batch` / `actuate` with monotonic clamping `max(now_ms, prev_committed_at_ms)`), so both `as_of_write_cursor` and `as_of_epoch_ms` work in `0.8.31` with ~1–5 µs/batch write cost and zero overhead on default reads.
2. **Phase B ladder renumbering (approved):** Adopt `Slice 10 = Prework Prep` and renumber feature slices to `Slices 20–80` (`20` Property Schema 38+ & W12, `30` Strict Integrity + Predicates + Fused Seeding, `40` Historical Graph `as_of`, `50` SDK Unblockers + `fathomdb migrate` + `fathomdb-cli` Wheel, `60` Inspector Upgrade + Scoped CI/Timeouts, `70` Full Qualification + Paired 10k/50k/100k Latency Gate, `80` Authorized Publication — held for separate HITL publication authorization).
3. **Slice 9 triage dispositions (approved):** All recommended `include` / `postpone` / `reject` decisions in Section 2 are approved, with Slice 10 executing the Prework Prep bundle listed in Section 3.
4. **Governed surface pin (`P3-03` / `P5-08` — pre-authorized):** Updates to `src/conformance/governed-{operation-parity,surface-allowlist}.json` and `scripts/governed-surface-pin.json` are pre-authorized for `read.operational_collections` / `read.operational_collection` (Slice 20) and `resolve_evidence_many` (Slice 50), preserving the 5 core verbs and 5 `recovery_denylist` names (`AC-041`).
5. **Project-boundary hygiene rule (`P6-13` & all tracked repo files — owner directive):** Although FathomDB coordinates closely with external consumer projects, **do not write Memex-specific information or internal consumer-project details into tracked FathomDB repository files** (especially public `docs/`, `README.md`, interfaces, recipes, and forward-looking scope docs). Frame every feature and guide (including `ingest_with_extractor` vs. `Engine.actuate`, `use_graph_arm` seeding, `fathomdb-cli`, and MCP/recipes) strictly in product-generic FathomDB terms. (`P6-13`, `prework/slice-6.md`, and `dev/plans/0.8.33/scope.md` cleaned accordingly.)
