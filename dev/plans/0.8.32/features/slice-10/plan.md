---
title: FathomDB 0.8.32 Slice 10 Implementation Plan — Prework Prep, Baseline CI Fixes, Orientation Aids, ADRs, Venue Staging, and Schema-38 vs. Schema-39 Scope Experiment
status: APPROVED
target_release: 0.8.32
slice: 10
date: 2026-11-24
---

# FathomDB `0.8.32` Slice 10 Implementation Plan (`PREWORK-PREP`)

## 1. Goal & Exit Contract

Execute all HITL Gate 1 approved prework prep items from [`prework/slice-9-triage.md`](../../prework/slice-9-triage.md) cleanly on `dev/0.8.32` (`/mnt/data/worktrees/release-0.8.32`), restore 100% green baseline CI (`./scripts/agent-verify.sh`), stage Slice 50 durability/fuzzing venues, draft the `0.8.32` ADRs, and complete the **Decision 1 Schema-38 vs. Schema-39 empirical performance experiment** so the owner can lock Decision 1 and finalize feature slice scopes (`Slices 20–70`).

### Slice-Local Acceptance Oracles (`AC32-S10-*`, `P3-02`)

- `AC32-S10-01`: Worktree `.venv` (`ruff 0.15.17`, `pyright 1.1.410`, `pytest`, `hypothesis`, `maturin`), root `node_modules`, and `src/ts/node_modules` pass `bash scripts/tests/test_dev_environment_tools.sh`; `release-state-0.8.32.json` & `STATUS-0.8.32.md` pass `scripts/check-release-state-views.sh` and `scripts/check-board-currency.sh`.
- `AC32-S10-02`: All 5 baseline `main` CI failures are fixed RED→GREEN: `migrate_cli` passes without `/mnt/data/worktrees/...`, `test_tegra_publication_operator_path.sh` adapts dynamically when `tegra-pages-retained.json` changes, `scripts/agent-typecheck.sh` fails fast on stale/bad Pyright *before* invoking `cargo check` (`test_agent_typecheck_pyright_version.sh`), and `scripts/agent-lint.sh` runs `generate-public-docs.py --check` via a `>= 3.11` interpreter (`test-pages-site` & `test-tegra-pages-retention` green).
- `AC32-S10-03`: `scripts/tests/test_gate_diagnostics_0832.sh` proves tightened `R32-07` error messages name exact owner config/registry/policy files while preserving all existing `scripts/tests/test_module_boundary_gate.sh` assertions.
- `AC32-S10-04`: `dev/design/cross-cutting-change-checklist.md` (`status: ACTIVE`), expanded `dev/design/README.md` (preserving `fathomdb-data-plane-architecture-v2.md` link), updated `document-lifecycle.json` + `current-owner-authority.json` + `lint-design-status.sh` (`LEGACY_BUDGET` if `orchestration.md` frontmatter changes), split `ADR-0.6.0-decision-index.md`, and `0.8.30` / 6-op / consumer-boundary doc fixes pass `check-design-lifecycle.py`, `lint-design-status.sh`, `check-architecture-authority.py`, `check-agents-md-paths.py`, `check-public-doc-truth.py`, `check-glibc-floor-doc-truth.sh`, `agent-lint-md.sh`, `agent-lint-docs.sh`, and `mkdocs build --strict`.
- `AC32-S10-05`: `WP-10D` empirical experiment in `fathomdb-tc5-benchmark` delivers reproducible single-scope and multi-scope 100k measurements (`B0`, `S38-PARTIAL`, `S39-COMPOSITE`, `NORMAL` vs. `FULL`), `recall@10`, `EXPLAIN QUERY PLAN` (zero `USE TEMP B-TREE FOR ORDER BY` on FTS5+`scope` joins), `database_bytes`, and subprocess `process_open`/`peak_rss_bytes`.
- `AC32-S10-06`: All 4 `ADR-0.8.32-*.md` ADRs (including `AC32-MCP-SAFETY-01..09`, `P2-07` `locomo_*` detachment, `AC32-DMFLAKEY-*`, and `AC32-FUZZ-*` oracles) are authored and indexed; `cargo-fuzz` `/tmp/` `ASan` link smoke and QEMU `/dev/kvm` `dm-flakey` venue readiness are verified; full `./scripts/agent-verify.sh` passes clean.

### Non-Goals (Out of Slice 10 Scope)

- No product feature implementation for `scope` enforcement (`Slice 20`), `search.rs`/`evidence.rs` hotspot splits (`Slice 20` entry), `TABLE_MANIFEST` refactor (`Slice 20`), bulk write / Inspector UI (`Slice 30`), `fathomdb-mcp` read+write code/packaging (`Slice 40`), or full `dm-flakey`/fuzzing campaigns (`Slice 50`).
- No edits to immutable `0.8.27` `slice-135` provenance lockfiles (Dependabot alerts handled via API dismissal `P1-01`).
- No edits to locked `dev/acceptance.md`, no NVIDIA driver / `/usr/local/cuda` mutations, no `v0.8.32` tagging or registry pushes, and no premature `"0.8.32"` entry in `RELEASE_SCHEMAS` until Decision 1 locks `SCHEMA_VERSION`.

---

## 2. Work Packages (`WP-10A` – `WP-10E`)

### `WP-10A` — Worktree Environment Bootstrap, Release State, Ledger & GitHub Hygiene

*Covers: `slice-0.md`, `P1-01`, `P1-02`, `P1-03`, `P1-05` (publisher preflight), `P2-08`, `P6-04`, `P8-01`, `P8-03`.*

1. **Checkout-owned dev environment (`/mnt/data/worktrees/release-0.8.32`):**
   - Run `bash scripts/setup-python-owned.sh --provision` (owned noneditable `.venv` with pinned `ruff 0.15.17`, `pyright 1.1.410`, `pytest`, `hypothesis`, `maturin`).
   - Run `npm install` at worktree root (`markdownlint-cli2`, `js-yaml`) and `npm install` in `src/ts/`.
   - Verify with `bash scripts/tests/test_dev_environment_tools.sh`.
2. **GitHub Dependabot & stale branch cleanup (`P1-01`, `P1-02`, `P1-03`):**
   - Dismiss the exact `106` non-shipping Dependabot alerts via `gh api -X PATCH repos/fathomadb/fathomdb/dependabot/alerts/<N>` (`state=dismissed`, `dismissed_reason=not_used`, comment citing `0.8.32` `prework/slice-1.md`): `20` deleted pre-0.6.0 `python/uv.lock`, `81` immutable `0.8.27` `slice-135/results/*/Cargo.lock`, `5` historical `0.8.29-slice60-projected-metadata/driver/Cargo.lock`.
   - Delete the `7` already-satisfied/superseded remote `origin/dependabot/*` branches (`multi-def6ea3883`, `rustls-0.23.46`, `dompurify-3.4.17`, `multi-010aad1c49`, `multi-68e90142e5`, `certifi-2026.10.18`, `setuptools-83.0.0`) — leaving the 3 intentionally held ONNX exporter branches untouched.
   - Keep root `package.json` `overrides` empty (`P1-03`, `P1-12`).
3. **Release-state, `dev/<ver>` branch support & board commissioning (`P2-08`, `P8-01`, `P8-03`):**
   - Update `scripts/check-release-state-views.sh:652-658` and `scripts/preflight-release-state.py:115-122` (including `baseline_ref = active_ref` at line `121`) so either `refs/heads/release/<version>` or `refs/heads/dev/<version>` is accepted as a valid active release ref (matching `RELEASE-AND-SLICE-PLANNING-TEMPLATE.md`'s `dev/<x.y.z>` convention without creating a duplicate `release/0.8.32` branch).
   - Update `scripts/tests/test_preflight_release_state.py:324-351` (`RealPreflightRegressionTest`) so when `closed` is empty (initial release state before any slice lands), it invokes `preflight.sh` without `--expect-closed` instead of crashing on `max([])`, and verify `test_check_board_currency.sh` / `test_check_release_state_views.sh` cover the initial-state case (`landed: []`, `next_slice: 10`, `ladder[0].status == "IN_PROGRESS"`).
   - Create `dev/plans/release-state-0.8.32.json` with:
     - `published: null`, `completion: null`, `active_ref: "refs/heads/dev/0.8.32"`, `plan: "dev/plans/0.8.32/plan.md"`, `board: "dev/plans/runs/STATUS-0.8.32.md"`, `schema_version: 38` *(provisional pending Decision 1)*, `public_docs_target_release` **omitted/unset** (so `check-public-doc-truth.py` continues validating `PUBLIC_DOCS` against published `0.8.31`),
     - Complete `ladder` objects (`slice`, `short`, `title`, `depends_on`, `status`, `sha`) for `[10, 20, 30, 40, 50, 60, 70]` (per Decision 3, since Slices 0–9 are prework), with `landed: []`, `next_slice: 10`, `ladder[0].status: "IN_PROGRESS"` (remaining `"NOT_STARTED"`, `sha: null`), `remaining_ladder: [10, 20, 30, 40, 50, 60, 70]`, recorded Gate 1 decisions, and all **three** `Arm R6`-required `generated_views`:
       1. `plan-immediate-next` fenced in `dev/plans/0.8.32/plan.md`
       2. `status-current-state` fenced in `dev/plans/runs/STATUS-0.8.32.md`
       3. `status-next-action` fenced in `dev/plans/runs/STATUS-0.8.32.md`
   - Create `dev/plans/runs/STATUS-0.8.32.md` with matching `status-current-state` and `status-next-action` generated-view markers, and add the `plan-immediate-next` marker to `dev/plans/0.8.32/plan.md`, verified by `scripts/check-release-state-views.sh`, `scripts/check-board-currency.sh`, `scripts/tests/test_check_release_state_views.sh`, and `scripts/tests/test_preflight_release_state.py`.
   - Add historical-closed notice to `dev/plans/runs/STATUS-0.8.31.md` (confirming `scripts/release-current.py` selects `0.8.32`).
   - Update `dev/plans/0.8.32/plan.md` & `scope.md`: replace the stale `"Open HITL decisions required before Slice 0 starts"` and `"Slice 0/10/..."` requirement references (`plan.md:50-81`) with Gate 1 rulings and the canonical `10..70` ladder; paste § 3 CUDA version review paragraph (`P8-01`); record standing tripwires `T1`–`T3` = NOT FIRED (`P8-03`); fix `scope.md:128` (`dev/tools/module-boundary-gate/src/config.rs` -> `dev/tools/module-boundary-policy.txt`).
   - Retarget ledger todo `TC-2a9f89fa-5302-44b7-a14e-97b9d9cbde90` from `0.8.29` to `0.8.32` via `dev/agent-tools/ledgerwrite` (reading full tail with `ledgerwatch` first).
   - Commit `dev/plans/0.8.32/prework/slice-{0..8,9-triage}.md` + `WP-10A` state/scaffold cleanly on `dev/0.8.32`.
4. **Early `fathomdb-mcp` external publisher preflight (`P1-05`, `P7-06`, `P7-07`):**
   - Probe crates.io (`fathomdb-mcp`), PyPI (`fathomdb-mcp`), and GitHub Actions `release.yml` environment/secret prerequisites (`CARGO_REGISTRY_TOKEN` / `pypi` env); record an exact operator checklist in `features/slice-10/status.md` for interactive PyPI Pending Publisher / crates.io web-UI registration prior to Slice 40/70.

---

### `WP-10B` — Baseline `main` CI Fixes & `R32-07` Gate Diagnostics (TDD RED → GREEN)

*Disjoint file ownership: `dev/fixtures/`, `fathomdb-cli/tests/migrate_cli.rs`, `fathomdb-engine/tests/slice50_migrate_37_to_38.rs`, `scripts/agent-{lint,typecheck,test}.sh`, `scripts/check-{sdk-surface-parity.py,governed-surface-pin.sh}`, `dev/tools/module-boundary-gate/src/{main,config}.rs`, `docs/generated/contributor-map.md`, and `scripts/tests/test_{tegra_publication_operator_path,agent_typecheck_pyright_version,gate_diagnostics_0832}.sh`. Does **NOT** touch `docs/reference/cli.md`.*
*Covers: `P1-08`, `P2-05`, `P5-01`, `P5-02`, `P6-02`, `P7-01`, `P7-02`, `P7-03`, `P7-04`, `P7-05`.*

1. **Windows & portable Schema-37 migrate fixture (`P7-01`, `P5-01`):**
   - Copy verified `golden-0.8.30-schema37.sqlite` (`sha256: 59d56eabb9a6e817168c5c540b4bb2ec6eb7f741fb24a72c7617df4aa5004548`, 634,880 bytes) into `dev/fixtures/golden-0.8.30-schema37.sqlite` (outside `src/rust/crates/*`).
   - Update `src/rust/crates/fathomdb-cli/tests/migrate_cli.rs` and `src/rust/crates/fathomdb-engine/tests/slice50_migrate_37_to_38.rs` to resolve via `FATHOMDB_GOLDEN_SCHEMA_37_PATH` env override falling back to `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../dev/fixtures/golden-0.8.30-schema37.sqlite")`, retaining the SHA-256 assertion.
   - Verify `cargo test --locked -p fathomdb-cli --test migrate_cli` passes without `/mnt/data/worktrees/...`.
2. **Fix all 4 `verify-fast` failures on `38a1594bf` (`P7-02`, `P7-03`, `P7-04`, `P1-08`):**
   - `scripts/tests/test_tegra_publication_operator_path.sh`: derive newest `+tegra` version dynamically from `scripts/release/tegra-pages-retained.json` instead of hardcoding `0.8.30+tegra` (verify RED→GREEN with a dynamic fixture test case).
   - `scripts/agent-lint.sh`: hoist/reuse the `>= 3.11` Python interpreter lookup (`$classification_python` from lines 103–115, honoring Ubuntu 22.04 / Jetson Orin Python 3.10 per `AGENTS.md` § 3) to run `"$docs_python" "$SCRIPT_DIR/generate-public-docs.py" --check` (keeping `SOURCE_URL` in `generate-public-docs.py:18` at `release/0.8.31` until Slice 60/70).
   - `scripts/agent-typecheck.sh` & `scripts/tests/test_agent_typecheck_pyright_version.sh`: move the Pyright version preflight *before* Rust `run_capped` steps (asserting fail-fast before `cargo check` runs!) AND copy/stub `check-release-smoke-sdk-compile.sh` inside `$FIX` (preserving `pyright==1.1.410`).
3. **Tighten `R32-07` parity, pin, and module-boundary failure diagnostics (`P2-05`, `P5-02`, `P6-02`, `P7-05`):**
   - Write failing contract test `scripts/tests/test_gate_diagnostics_0832.sh` first (RED), asserting that:
     - `scripts/check-sdk-surface-parity.py` names `src/conformance/governed-operation-parity.json`, `src/conformance/governed-surface-allowlist.json`, `scripts/governed-surface-pin.json`, and binding source paths on failure;
     - `scripts/check-governed-surface-pin.sh` cross-references `governed-operation-parity.json`, `check-sdk-surface-parity.py`, and `dev/design/cross-cutting-change-checklist.md`;
     - `dev/tools/module-boundary-gate/src/{main,config}.rs` appends exact file pointers (`dev/tools/module-boundary-policy.txt`, `dev/tools/module-boundary-gate/src/config.rs`, `src/rust/crates/fathomdb-engine/src/lib.rs`) while **preserving all existing failure substrings asserted by `scripts/tests/test_module_boundary_gate.sh`**.
   - Implement the diagnostic updates (GREEN), verify both `scripts/tests/test_gate_diagnostics_0832.sh` and `scripts/tests/test_module_boundary_gate.sh` pass, and wire `test_gate_diagnostics_0832.sh` into `scripts/agent-test.sh` using `run_tier_suite fast`.
4. **Regenerate `docs/generated/contributor-map.md` (`P7-03`):**
   - As the **final step of `WP-10B`** (after all edits to `scripts/agent-{lint,typecheck,test}.sh` are complete), run `.venv/bin/python scripts/generate-public-docs.py --write` (using a Python `>= 3.11` interpreter) so `docs/generated/contributor-map.md` matches the updated script hashes and `./scripts/agent-lint.sh`, `test-pages-site`, and `test-tegra-pages-retention` pass green inside `WP-10B` and on `dev/0.8.32` immediately upon merge.

---

### `WP-10C` — `R32-07` Orientation Docs & Immediate Factual / Consumer-Boundary Doc Fixes

*Disjoint file ownership: `dev/design/{cross-cutting-change-checklist.md,README.md,document-lifecycle.json,current-owner-authority.json,fathomdb-data-plane-architecture-v2.md,nested-source-projections.md}`, `scripts/lint-design-status.sh` (only if `LEGACY_BUDGET` changes), `dev/adr/ADR-0.6.0-decision-index.md`, `AGENTS.md`, `docs/**` (including `docs/reference/cli.md`), `src/**/README.md`, `src/python/fathomdb/engine.py`, `dev/{ROADMAP.md,DOC-INDEX.md,plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md}`, `ROADMAP.md`, `scripts/check-public-doc-truth.py`, and `scripts/tests/test_public_doc_truth.sh` (zero edits to `scripts/agent-test.sh`).*
*Covers: `P2-03`, `P2-04`, `P3-04`, `P3-06`, `P6-01`, `P6-03`, `P6-04`, `P6-05`.*

1. **Orientation aids (`P2-03`, `P2-04`, `P3-04`, `P6-01`):**
   - Create `dev/design/cross-cutting-change-checklist.md` with uppercase `status: ACTIVE` (covering schema/table changes via `TABLE_MANIFEST`, governed SDK ops/parity/pin, MCP config/tools, module-boundary splits, `registry-sdk.rs` smoke, and `generate-public-docs.py --write`).
   - Expand `dev/design/README.md` with a concise 1-page live subsystem owner map covering all `"maintained"` design docs while preserving the link to `fathomdb-data-plane-architecture-v2.md` (`check-architecture-authority.py:111-118`).
   - Update BOTH `dev/design/document-lifecycle.json` AND `dev/design/current-owner-authority.json` in lockstep: add `cross-cutting-change-checklist.md` (`"maintained"` with `semantic_authority` and valid `implementation_witness` paths under `src/`/`scripts/`/`.github/workflows/`) and demote `dev/design/orchestration.md` from `"maintained"` to `"historical"` (updating `LEGACY_BUDGET` in `scripts/lint-design-status.sh` if `orchestration.md`'s lowercase frontmatter status is touched, or keeping its historical frontmatter untouched so `LEGACY_BUDGET=43` remains exact).
   - Split `dev/adr/ADR-0.6.0-decision-index.md` into **Active / Governing** and **Superseded / Historical** tables, fix duplicate `#66` and unformatted `#67` (`ADR-0.8.28-vector-compatibility-open-policy.md`), update frontmatter `status`, and index the 20 previously unindexed existing `ADR-*.md` files.
   - Link `dev/design/cross-cutting-change-checklist.md` and `dev/design/README.md` in `AGENTS.md` (staying `<= 300` lines; `check-agents-md-paths.py` green) and `docs/contributing/index.md`.
2. **Immediate factual & product-boundary fixes (`P3-06`, `P6-03`, `P6-04`, `P6-05`):**
   - Fix incorrect `PropertyPredicate` operator list (closed 6 variants: `eq, ne, lt, lte, gt, gte`) across `docs/guides/hybrid-search-filtering.md` and `docs/reference/{rust-sdk,rust-api,python-api,typescript-api}.md`, and add `$.action_kind` to `docs/reference/errors.md`.
   - Purge residual `0.8.30` `"newest published release"` / install commands / `blob/release/0.8.30/` links across 13+ `docs/` pages (including `docs/reference/cli.md:182` `0.8.30+tegra -> 0.8.31+tegra`) and all 8 `src/**/README.md` files (updating to `0.8.31` published truth while keeping `0.8.32` `published: null` pre-publish posture intact); add explicit per-target CUDA + CE wheel matrices in `docs/install/{python,typescript}.md` and split Python `manylinux_2_28` (`glibc 2.28`) vs. N-API CUDA (`glibc 2.39`) in `docs/compatibility/index.md:116` (preserving `check-glibc-floor-doc-truth.sh` anchor lines).
   - Sync `dev/ROADMAP.md`, root `ROADMAP.md`, `dev/plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md`, and `dev/DOC-INDEX.md` (`0.8.31` published `2026-11-23`; `0.8.32` active on `dev/0.8.32`).
   - Update `scripts/check-public-doc-truth.py` with a narrowly scoped negative check rejecting stale `"newest published release: <ver>"` banners where `<ver> != published_version` across `PUBLIC_DOCS` (without blocking legitimate historical measurement citations or `0.8.26`/`0.8.27` test fixtures), and extend existing `scripts/tests/test_public_doc_truth.sh` (RED→GREEN). **Do not** add `"0.8.32"` to `RELEASE_SCHEMAS` until Decision 1 locks `SCHEMA_VERSION`.
   - Generalize consumer-project framing out of the 2 maintained `dev/design/` files (`fathomdb-data-plane-architecture-v2.md`, `nested-source-projections.md`) and replace `"agent:hermes"` in `src/python/fathomdb/engine.py:574` docstring with `"agent:assistant"`.

---

### `WP-10D` — Decision 1 Empirical Experiment: Schema 38 vs. Schema 39 `scope` Performance + `SyncMode` Write Latency

*Disjoint file ownership: `src/rust/crates/fathomdb-tc5-benchmark/{Cargo.toml,src/bin/scope_schema_experiment.rs}` (existing `eval-only` workspace crate in `Cargo.toml:15` and `scripts/sbom-survey/tiers.toml:68-71` — adding only already-locked workspace dependencies `rusqlite`, `fathomdb-schema`, `tempfile` to `[dependencies]` with zero new external crates or version bumps), root `Cargo.lock` (workspace dependency edge update only; zero new `Cargo.lock` files), and `dev/plans/0.8.32/features/slice-10/{scope-schema-experiment.md,scope-schema-results.json}`. Must pass `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` inside `WP-10D`.*
*Covers: Owner Decision 1 directive, `HANDOFF-02`, `P4-01`, `P4-02`, `P4-07`, `P5-17`.*

1. **Methodologically rigorous workspace benchmark (`src/rust/crates/fathomdb-tc5-benchmark/src/bin/scope_schema_experiment.rs`):**
   - **Real engine anchor (`B0`):** Construct the `100,000`-node / `150,000`-edge baseline database via real `fathomdb-engine` (`Engine::open` + `Engine::write` + `drain`) with `chunk_size=64` and `MAX_VEC0_NODE_PROPERTY_SLOTS = 8` (`vector_storage.rs:75-99`) so open-path pragmas, triggers, `_fathomdb_read_visibility_state`, FTS5, `canonical_property_values`, and `vec0` geometry match production `0.8.31`.
   - **Three compared layouts (on identical seeded corpus, deterministic PRNG seed, `warmup_rounds = 5`, `N = 200` read/graph/search trials, `N = 30` fresh-subprocess `process_open`/`warm_reopen`/`peak_rss_bytes` trials, `N = 100` write/upsert/checkpoint trials from identical restored DB snapshots + truncated WAL before each `NORMAL` vs. `FULL` arm):**
     - **Baseline (`B0`):** Exact `0.8.31` Schema 38 DDL & 7-col `vec0` (`scope == ""`).
     - **Candidate A (`S38-PARTIAL`):** Non-breaking Schema 38 — existing unique indexes/PKs untouched + additive **partial indexes `WHERE scope != ''`** (`canonical_nodes`, `canonical_edges`, `operational_state`, `operational_mutations`) + **both** (A1) lazy 7-col `vec0` using real `WITH candidates AS (SELECT cn.write_cursor AS rowid FROM canonical_nodes cn WHERE cn.scope = ? ...) ... vec_distance_l2(...)` pre-selection (`filter.rs:970-974`, reporting `recall@10` & returned count!) and (A2) reshaped 8-col `vec0` (`scope TEXT DEFAULT ''`), plus real **same-scope ownership lookup (`SELECT scope ...`)** on upsert/supersede/checkpoint writes!
     - **Candidate B (`S39-COMPOSITE`):** Breaking Schema 39 — composite `(scope, logical_id)` active unique indexes on `canonical_nodes`/`canonical_edges`, composite `PRIMARY KEY(scope, collection_name, record_key)` on `operational_state`, full `(scope, ...)` composite indexes, and native 8-col `vec0` (`scope TEXT DEFAULT ''`).
   - **Three dataset profiles (100k nodes / 150k edges each):**
     1. **Single-scope (`scope == ""`):** 100% unscoped rows (tests `HANDOFF-02`/`HANDOFF-04` zero-regression floor and `200.4 MB` DB-size headroom).
     2. **Multi-scope Uniform (10 callers × 10k nodes / 15k edges):** balanced multi-tenant workload.
     3. **Multi-scope Skewed (1 heavy caller 90% + 9 light callers 10% total, ~1.1% per light caller):** stress-tests selectivity, `recall@10`, and `vec0` pre-selection vs. 8-col `vec0` on small scopes inside a large store.
   - **Measured operations (p50 / p95 + 10,000-resample paired bootstrap 95% CI, plus captured `EXPLAIN QUERY PLAN` asserting zero `SCAN` and zero `USE TEMP B-TREE FOR ORDER BY` on FTS5 BM25 + `scope` joins):**
     - `process_open`, `warm_reopen`, `peak_rss_bytes` (measured via fresh child subprocesses for clean RSS/open isolation)
     - `write_node_batch_200` (fresh inserts AND existing-ID upserts with ownership check; under BOTH `SyncMode::Normal` and `SyncMode::Full`)
     - `write_edge_batch_200` (under BOTH `NORMAL` and `FULL`)
     - `single_row_checkpoint_write` (simulating MCP `write`/`checkpoint` on `operational_state`, fresh & upsert; under BOTH `NORMAL` and `FULL`)
     - `search_text_only` (FTS5 BM25 + lifecycle + `scope` filter)
     - `search_vector_k10` & `search_hybrid` (comparing A1 pre-selection vs. A2 8-col `vec0` vs. B 8-col `vec0`, recording `recall@10` against exact L2 ground truth!)
     - `graph_expand_2hop` (scoped edge + endpoint filter)
     - `operational_state_point_read` & `operational_state_page`
     - `database_bytes` (full `.sqlite` + WAL-truncated size)
2. **Decision report (`dev/plans/0.8.32/features/slice-10/scope-schema-experiment.md` + `scope-schema-results.json`, linted clean by `agent-lint-md.sh`):**
   - Present exact tables, bootstrap CIs, `EXPLAIN QUERY PLAN` diffs, `recall@10`, `database_bytes`, `NORMAL` vs. `FULL` write deltas, and a clear, evidence-backed **Schema 38 vs. Schema 39 recommendation** for the owner to finalize Decision 1 right after Slice 10.

---

### `WP-10E` — Slice 10 ADRs (`ADR-0.8.32-*`) & Slice 50 Venue / Fuzzing Prep

*Runs on `/mnt/data/worktrees/release-0.8.32` after `WP-10B`, `WP-10C`, `WP-10D` merge so `ADR-0.8.32-caller-scope-isolation.md` cites exact `WP-10D` measurements and `ADR-0.6.0-decision-index.md` indexes all 4 ADRs cleanly.*
*Covers: `P2-07`, `P3-02`, `P3-05`, `P4-07`, `P5-03`, `P7-09`, `P8-04`, `P8-05`.*

1. **Author four `0.8.32` ADRs (indexed in `dev/adr/ADR-0.6.0-decision-index.md`):**
   - `dev/adr/ADR-0.8.32-mcp-read-write-and-distribution.md` (`status: accepted` per Owner Rulings 1, 3 & Decisions 4, 5): `allow_writes: false` default, per-store `allow_writes: true` + bound `scope`, `SC-01`–`SC-10` fail-fast startup conflict matrix, `AC32-MCP-SAFETY-01..09`, atomic `PutLatestState` + `EP-07` + `EP-01` (Pin C), 3-channel packaging (`--features default-embedder` on GH Releases/PyPI, `default = []` on crates.io), detaching `locomo_load`/`locomo_eval` examples while retaining `slice30_m_helper` (`P2-07`), and blocking cancel/shutdown drain fence (`bridge.rs`/`registry.rs`).
   - `dev/adr/ADR-0.8.32-opt-in-bulk-write-mode.md` (`status: accepted` per Decision 4 Pin B): strict default `Engine::write` + strict-only `Engine::actuate`, opt-in `WriteOptions` / `write_with_options`, truthful `WriteReceipt.dangling_edge_endpoints`, `fathomdb doctor` dangling + cross-scope edge verification.
   - `dev/adr/ADR-0.8.32-durability-sync-mode-and-power-loss-oracle.md` (`status: accepted` per Decision 2 Option 1; amends `ADR-0.6.0-durability-fsync-policy.md`): honest `SyncMode::Normal` WAL checkpoint-bounded durability + opt-in `SyncMode::Full` (`PRAGMA synchronous = FULL`), explicit `AC32-DMFLAKEY-*` and `AC32-FUZZ-*` oracles (`P3-02`), disposable QEMU/KVM Ubuntu 24.04 + `dm-flakey` storage stack, external acknowledged-write oracle, and isolated `fuzz/` workspace policy (`exclude = ["fuzz"]`).
   - `dev/adr/ADR-0.8.32-caller-scope-isolation.md` (`status: proposed` — awaiting owner ruling on `WP-10D` Schema 38 vs. Schema 39 results): scope token/policy grammar, opener-bound anti-spoof, read/search/graph/frozen/evidence/erasure isolation, Pin A surface, and direct citation of `WP-10D` measurements.
2. **Pre-stage Slice 50 durability & fuzzing venues (`P8-04`, `P8-05`, `P5-03`):**
   - Install `cargo-fuzz --locked` (`cargo install --locked cargo-fuzz`) and run an ephemeral `/tmp/fathomdb-fuzz-smoke-*` `nightly-2026-08-01` `libfuzzer-sys` + `ASan` link & coverage-tool smoke outside the worktree (cleaning `/tmp/` afterward; zero mutations to root `Cargo.toml`/`Cargo.lock`).
   - **Escalated-network checkpoint (`AGENTS.md` § 11):** Before downloading any third-party Ubuntu 24.04 cloud image / `linux-generic` kernel outside GitHub/PyPI/npm/crates.io registries, either confirm the exact pinned official Ubuntu `cloud-images.ubuntu.com` URL + SHA-256 checksum with the owner (or record the exact staging command/URL for approval at Slice 10 close if network egress requires confirmation) and validate `/dev/kvm` + `xorriso` NoCloud `cidata` + `dm_flakey.ko` boot smoke cleanly under `/mnt/data/worktrees/fathomdb-0.8.32-dmflakey-vm/`.

---

## 3. Execution Topology, Post-Merge Regeneration & Verification Gate

- **One writer per checkout:**
  1. Execute **`WP-10A`** on `/mnt/data/worktrees/release-0.8.32` (bootstrap `.venv`/`node_modules`, Dependabot/branch hygiene, `dev/<ver>` release-state validator update, `release-state-0.8.32.json` + `STATUS-0.8.32.md`, commit prework `slice-0..9`).
  2. Cut **3 isolated parallel `slice-implementer` worktrees** off `dev/0.8.32` under `/mnt/data/worktrees/`:
     - `release-0.8.32-wp10b` -> **`WP-10B`** (CI fixes + `R32-07` gate diagnostics + tests; zero overlap with `WP-10C`/`WP-10D`)
     - `release-0.8.32-wp10c` -> **`WP-10C`** (Orientation docs + factual/consumer-boundary doc fixes + `ADR-0.6.0` split + `test_public_doc_truth.sh`; zero overlap with `WP-10B`/`WP-10D`)
     - `release-0.8.32-wp10d` -> **`WP-10D`** (`fathomdb-tc5-benchmark` Schema-38 vs. Schema-39 + `NORMAL`/`FULL` benchmark & experiment report)
  3. Merge `WP-10B`, `WP-10C`, `WP-10D` sequentially into `dev/0.8.32`, remove temporary worktrees & branches (`git worktree remove` + `git branch -d`), then run **`WP-10E`** (4 `ADR-0.8.32-*` + `ADR-0.6.0` index entries + `cargo-fuzz` `/tmp/` smoke + QEMU `/dev/kvm` staging).
  4. **Post-merge generated-doc verification:** Run `.venv/bin/python scripts/generate-public-docs.py --check` after merging `WP-10B`/`WP-10C`/`WP-10D` and `WP-10E` (re-running `--write` only if any post-merge integration altered a hashed script).
- **Mandatory Verification Gate (`AGENTS.md` § 3):**
  - Run targeted checks: `scripts/check-release-state-views.sh`, `scripts/check-board-currency.sh`, `scripts/check-design-lifecycle.py`, `scripts/lint-design-status.sh`, `scripts/check-architecture-authority.py`, `scripts/check-agents-md-paths.py`, `scripts/check-public-doc-truth.py`, `scripts/check-glibc-floor-doc-truth.sh`, `.venv/bin/python scripts/generate-public-docs.py --check`, `mkdocs build --strict`, `cargo test --locked -p fathomdb-cli --test migrate_cli`, `bash scripts/tests/test_module_boundary_gate.sh`, `bash scripts/tests/test_public_doc_truth.sh`, `bash scripts/tests/test_gate_diagnostics_0832.sh`, and all 4 previously failing `verify-fast` suites.
  - Run the **full `./scripts/agent-verify.sh`** (unconfined if `AC-036` requires `ptrace`) on the clean integrated `dev/0.8.32` tree.
- **Independent Adversarial Code Review:**
  - Spawn read-only `slice-reviewer` (`Model: "pro"`) on the full Slice 10 diff until `VERDICT: APPROVE`.
  - Write `dev/plans/0.8.32/features/slice-10/status.md`, update `release-state-0.8.32.json` / `STATUS-0.8.32.md`, and present `WP-10D`'s Schema-38 vs. Schema-39 empirical results to the owner to finalize Decision 1 and lock Slices 20–70.
