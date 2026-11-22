---
title: 0.8.31 Prework Slice 6 — Stale Documentation Forensics
status: COMPLETE
target_release: 0.8.31
slice: 6
date: 2026-11-22
---

# 0.8.31 Prework Slice 6 — Stale Documentation Forensics

Strictly read-only audit of `/home/coreyt/projects/fathomdb-worktrees/release-0.8.31` (`release/0.8.31` @ `6ee16b9e8`) across:

1. End-user docs (`docs/`, `README.md`),
2. Developer/consumer SDK & CLI docs (`docs/reference/`, `docs/install/`, `dev/interfaces/`), and
3. Contributor docs (`AGENTS.md`, `dev/README.md`, `dev/ROADMAP.md`, `docs/contributing/`, `ROADMAP.md`).

All findings below are verified against source, CI workflows, and accepted `0.8.31` Slice 0 ADRs. Proposals `P6-01` through `P6-13` separate **quick `0.8.31` fixes** from the **`0.8.33` full OSS docs overhaul**.

---

## Executive Summary

- **All five Memex `0.6.2` doc bugs are confirmed in tracked docs:**
  1. `engine.write([])` verify snippets and "valid empty batch" claims contradict `write.rs:431` (`EngineError::WriteValidation`).
  2. `use_graph_arm` / `useGraphArm` reference docs still claim top-10 fused-hit seeding, contradicting `search.rs:1550-1585` (`SEED_FTS_N = 10` edge/entity FTS seeding) and `0.8.30` release notes.
  3. PyPI/npm CUDA and cross-encoder (`default-reranker` / `rerank-cuda`) wheel coverage is undocumented or conflated (`docs/compatibility/index.md:116` mislabels Python x86_64 glibc as `2.39`; `docs/install/python.md` omits Linux x86_64 CUDA/CE; four docs still promise `0.8.31` CUDA 13 delivery despite Slice 0 deferral).
  4. Mixed-`source_id` `Engine.write(batch)` single-transaction atomicity and zero-row rollback on `WriteValidationError` are omitted across `docs/reference/` and `dev/interfaces/`.
  5. Hardcoded `schema 37` / "no upgrade/migration" claims appear in 35+ lines across 17 public/interface files ahead of `0.8.31`'s schema `38` bump and `fathomdb migrate` helper.
- **Additional verified drift:** stale error-class count (`44` vs. `48` actual), operation-count contradiction (`44` vs. `53`), upcoming `0.8.31` dangling-edge doc contradiction, stale `dev/ROADMAP.md` and `dev/plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md`, and `AGENTS.md` workspace crate count (`12` vs. `13` after Inspector merge) / `prettier` devDep claim.

---

## Findings & Numbered Proposals (`P6-01` – `P6-13`)

### Layer 1 & 2 — End-User, SDK, CLI, and Interface Docs

#### `P6-01` (`0.8.31` Slice 40 — Quick Fix) — Fix `engine.write([])` verify snippets & empty-batch claims

- **Evidence:** `src/rust/crates/fathomdb-engine/src/write.rs:431-433` rejects empty batches with `Err(EngineError::WriteValidation)` (asserted in `tests/lifecycle_observability.rs:495`).
- **Stale locations:**
  - `docs/install/python.md:103` (`engine.write([])` in Verify snippet — fails at runtime).
  - `docs/install/typescript.md:205` (`await engine.write([]);` in Verify snippet — fails at runtime).
  - `docs/getting-started/quickstart.md:86-90` (falsely claims `write([])` is valid and used by `smoke-pypi-wheel.sh`; `smoke-pypi-wheel.sh` delegates to `registry-python.py`, which writes real rows).
  - `docs/reference/python-api.md:97-98` and `docs/reference/typescript-api.md:106-107` (claim default `[]` is a "valid, item-less batch").
- **Proposal:** Replace verify snippets with a minimal valid node write (`kind`, `body`, `source_id`/`sourceId`), fix quickstart/reference prose to state that empty batches raise `WriteValidationError`, and update `src/ts/tests/fixtures/consumer-package-root.mts:20` if appropriate.

#### `P6-02` (`0.8.31` Slice 20/40 — Quick Fix) — Fix `use_graph_arm` / `useGraphArm` seeding wording & document `0.8.31` seeding options

- **Evidence:** `src/rust/crates/fathomdb-engine/src/search.rs:1550-1585` seeds `bfs_graph_arm_candidates` from query-matched edge-fact FTS (`search_index_edges`) and entity FTS (`search_index`) up to `SEED_FTS_N = 10` per source ("NOT doc-node hits"), matching `docs/release-notes/0.8.30.md:46-48` and `dev/adr/ADR-0.8.31-historical-graph-and-fused-seeding.md:24-29`.
- **Stale locations:**
  - `docs/reference/python-api.md:180-185` ("seeds a BFS over temporal fact-edges from the top-10 fused hits").
  - `docs/reference/typescript-api.md:187-192` ("seeds a BFS over temporal fact-edges from the top-10 fused hits").
- **Proposal:** Correct `use_graph_arm` / `useGraphArm` descriptions to document default FTS-based edge/entity seeding (`graph_seed="graph_fts"`), the Slice 20 ineligible-endpoint slot fix, and the new opt-in `graph_seed` (`"graph_fts" | "fused_top_k" | "both"`), `graph_seed_k`, `graph_edge_kinds`, `graph_edge_predicates`, and seed-provenance fields.

#### `P6-03` (`0.8.31` Slice 40 — Quick Fix) — Document exact PyPI/npm CUDA & CE wheel matrix; remove stale `0.8.31` CUDA 13 promises

- **Evidence:**
  - `.github/workflows/release.yml:501` (`build-python` generic Linux AArch64, macOS x64/arm64, Windows x64) builds with `--features pyo3/extension-module,default-embedder` — **CPU-only, no CUDA, no `default-reranker`**. Generic `build-napi` (`src/ts/package.json:25`) is likewise CPU-only without CE.
  - `.github/workflows/release.yml:884-945` (`build-cuda-linux-x64-gnu`) runs `scripts/release/cuda-preflight.sh --rerank-cuda` (`scripts/release/cuda-artifact-contract.sh:83-84`), so **Linux `x86_64` PyPI** (`manylinux_2_28`, glibc `2.28`) and **Linux `x64` npm** (`linux-x64-gnu`, glibc `2.39`) ship **both `embed-cuda` and `rerank-cuda`** (implying `default-embedder` + `default-reranker`).
  - Tegra Pages artifacts (`+tegra` wheel and `fathomdb-linux-arm64-gnu-tegra`, glibc `2.35`) ship `embed-cuda,rerank-cuda,tegra-pool` (`cuda-artifact-contract.sh:80,145`).
  - Slice 0 deferred CUDA 13 out of `0.8.31` (`dev/plans/0.8.31/plan.md:35`).
- **Stale locations:**
  - `docs/install/python.md`: omits Linux `x86_64` PyPI CUDA (`embed-cuda` + `rerank-cuda`) and per-platform CE coverage; line 59 promises CUDA 13 in `0.8.31`.
  - `docs/install/typescript.md:47-48`: promises CUDA 13 in `0.8.31`; lacks an explicit per-target CUDA + CE support table.
  - `README.md:96`: promises CUDA 13 in `0.8.31` (and line 108 states no prebuilt CLI binary exists, which changes in `0.8.31` Slice 40).
  - `docs/compatibility/index.md:72-73`: promises CUDA 13 in `0.8.31`; line 116 table cell `yes (CUDA host glibc 2.39)` conflates the Python `manylinux_2_28` wheel (glibc `2.28` per line 40) with the N-API `linux-x64-gnu` host package (glibc `2.39`) and omits CE (`rerank-cuda`/`default-reranker`) availability.
  - `docs/reference/python-api.md:177-179` and `docs/reference/typescript-api.md:184-186`: imply the default wheel has no `default-reranker` without distinguishing Linux `x86_64` / Tegra (`rerank-cuda` compiled in) from CPU-only targets (note `0.8.31` Slice 40 adds `OpenReport` booleans `reranker_compiled`, `cuda_compiled`, `rerank_available`).
- **Proposal:** Add an explicit per-target matrix (Embedder, CE Reranker, CUDA, glibc floor) to `docs/install/python.md`, `docs/install/typescript.md`, and `docs/compatibility/index.md`; separate Python vs. N-API glibc columns in `compatibility/index.md:114-120` (preserving `check-glibc-floor-doc-truth.sh` markers); cross-link `OpenReport` booleans in API references; remove all `0.8.31` CUDA 13 promises.

#### `P6-04` (`0.8.31` Slice 20/40 — Quick Fix) — Document mixed-`source_id` `Engine.write(batch)` single-transaction atomicity

- **Evidence:** `src/rust/crates/fathomdb-engine/src/write.rs:453-481` and `write_commit.rs:554-591` validate all rows via `validate_batch` and execute `apply_batch_in_transaction` in a single SQLite `tx.commit()?`; `source_id` is per-item with no single-source batch restriction, and any `WriteValidationError` (including `0.8.31` strict endpoint failures) rolls back all rows atomically.
- **Gap:** `docs/reference/{python-api,typescript-api,rust-sdk,rust-api}.md` and `dev/interfaces/{python,typescript,rust,rust-sdk}.md` only mention whole-batch rejection under the validity-window sub-bullet (`python-api.md:137-138`) and never document mixed-`source_id` batch support or zero-row rollback on `WriteValidationError`.
- **Proposal:** Add an explicit batch-atomicity contract subsection under `engine.write` in all four reference docs and matching `dev/interfaces/` specs.

#### `P6-05` (`0.8.31` Slices 10, 40, 50 — Required `0.8.31` Update; `0.8.33` Automation) — Update hardcoded `schema 37` and "no migration" claims

- **Evidence:** `0.8.31` Slice 10 bumps `SCHEMA_VERSION` `37 -> 38` (`ADR-0.8.31-property-graph-inline-and-declared-indexes.md`), and Slice 40 adds `fathomdb migrate` (`37 -> 38`) + `fathomdb-cli` PyPI wheels (`ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md`).
- **Locations (35+ lines across 17 active files):**
  - *Public docs (Slices 10/40):* `README.md:13,108-112`, `docs/getting-started/index.md:56-57,90-91`, `docs/concepts/index.md:17-18`, `docs/install/rust.md:58-60`, `docs/compatibility/index.md:144-148,182`, `docs/operations/erasure.md:182,220-222`, `docs/reference/rust-api.md:76`, `docs/reference/cli.md:115-116`, `docs/reference/errors.md:71`.
  - *Inspector docs (Slice 50, upon `0.8.31` Engine upgrade):* `docs/inspector/index.md:6`, `docs/inspector/getting-started/index.md:10`, `docs/inspector/operations/index.md:4`, `docs/inspector/reference/index.md:41`, `docs/inspector/ROADMAP.md:6`.
  - *Interface specs (Slices 10/40):* `dev/interfaces/python.md:20-22,1582`, `dev/interfaces/typescript.md:20-22,1529`, `dev/interfaces/rust.md:22-24,2308,2318`, `dev/interfaces/wire.md:31,67,75,77`, `dev/interfaces/cli.md:365`.
  - *Leave untouched (historical):* `docs/release-notes/0.8.{28,29,30}.md`, `docs/inspector/releases/0.2.{0,1}.md`.
- **Proposal:** Update public/interface schema mentions to `38`, replace blanket "no migration/upgrade" text with the `0.8.31` `fathomdb migrate` (37→38) helper + `fathomdb-cli` PyPI wheel route while keeping `Engine::open` strictness accurate. In `0.8.33`, derive schema version / CLI `--help` docs from source via `scripts/generate-public-docs.py` (`R33-02`) to eliminate manual schema-bump sweeps.

#### `P6-06` (`0.8.31` Slices 10–40 — Required `0.8.31` Update) — Reconcile `0.8.31` breaking & additive behavior in existing SDK/guide docs

- **Stale locations once `0.8.31` slices land:**
  - `docs/reference/python-api.md:153,156,661,667-676` & `docs/reference/typescript-api.md:160,163` (`dangling_edge_endpoints`): state `engine.write` always commits dangling edges ("flag-and-count; the write never rejects"), contradicting `ADR-0.8.31-strict-endpoint-integrity.md` (atomic `WriteValidationError` refusal in `0.8.31`).
  - `docs/guides/extractor-provenance.md:41-48`: states dense node projection is fixed to the 7-kind allowlist (`email`..`doc`), contradicting `0.8.31` Slice 10 app-declared vector-committable kinds (Memex R1). *(Note: framing `ingest_with_extractor` as legacy vs. `actuate` per Memex E3 is scheduled for `0.8.33` `R33-04`.)*
  - `docs/guides/graph-retrieval.md:18-23,31-33,50-56`: states `graph.expand` unconditionally excludes superseded edges and has no fused-hit seed mode, contradicting `0.8.31` Slices 20/30 (`edge_predicates`/`target_predicates`, `as_of` history, `graph_seed`).
- **Proposal:** Update `dangling_edge_endpoints` in Slice 20, `extractor-provenance.md` 7-kind claim in Slice 10/40, and `graph-retrieval.md` in Slice 20/30, per X4 DoD.

#### `P6-07` (`0.8.31` Slice 40 — Quick Fix) — Fix stale error-class & governed-operation counts

- **Evidence:**
  - `src/python/fathomdb/_fathomdb.pyi:697-1171`, `src/ts/src/errors.ts:20-474`, and `docs/reference/errors.md:43-90` export **48** concrete error classes under `EngineError`/`FathomDbError`, not `44`.
  - `docs/generated/sdk-operation-map.md:14` lists **53** governed operations (`src/conformance/governed-operation-parity.json`), contradicting `docs/positions/sdk-parity.md:9`.
- **Stale locations:**
  - `docs/reference/python-api.md:843` and `docs/reference/typescript-api.md:826` ("plus **44** concrete classes").
  - `docs/positions/sdk-parity.md:9` ("**44-operation** membership").
- **Proposal:** Update error counts to `48` (plus any `0.8.31` additions) and replace hardcoded `44-operation` text in `sdk-parity.md` with a reference to `sdk-operation-map.md` (removing the brittle hardcoded number). In `0.8.33`, enforce counts via auto-doc generation.

#### `P6-08` (`0.8.31` Slice 60/70 + `0.8.33` Overhaul) — Refresh `dev/interfaces/` currency & `0.8.30` release pins

- **Evidence:**
  - `dev/interfaces/{cli,python,rust,rust-sdk,typescript,wire}.md` headers all carry `target_release: 0.8.28` (`2026-09-14` / `2026-11-04`), predating `0.8.29`/`0.8.30` additions (`release_models`, `StoredHeadCoverageV1`, `scored_bytes`) and `0.8.31`.
  - `scripts/generate-public-docs.py:18` hardcodes `SOURCE_URL = '.../blob/release/0.8.30/'`, and `docs/{index,reference/*,install/*,compatibility/index,embedder,operations/tegra-pages-publication}.md` pin `0.8.30`.
- **Proposal:** Update `dev/interfaces/` headers and `0.8.31` contracts in Slices 10–40; bump `0.8.30 -> 0.8.31` pins/URLs in `generate-public-docs.py` and public docs during Slice 60/70 release preparation; leave full `dev/interfaces/` vs. `docs/reference/` consolidation and `rustdoc`/`typedoc`/MCP schema auto-generation to `0.8.33` (`R33-02`, `R33-04`).

---

### Layer 3 — Contributor Docs (`AGENTS.md`, `dev/README.md`, `dev/ROADMAP.md`, `docs/contributing/`)

#### `P6-09` (`0.8.31` Slice 10/40 — Quick Fix) — Fix `AGENTS.md` workspace crate count, `prettier` claim, and interface list

- **Evidence:**
  - `Cargo.toml:2-16` now declares **13** workspace members (`src/inspector` / `fathomdb-inspector` merged in `086d5af0a`), contradicting `AGENTS.md:31` ("`src/rust/crates/` — 12 crates").
  - `package.json:10-12` removed `prettier` in `0.8.9.1`, contradicting `AGENTS.md:74` ("installs `markdownlint-cli2`, and `prettier` which is retained as a devDep for non-markdown use only").
  - `AGENTS.md:132` (`dev/interfaces/{rust,python,typescript,cli,wire}.md`) omits `rust-sdk.md` and `mcp.md`.
- **Proposal:** Update `AGENTS.md:31` to reflect the 13 workspace crates (12 under `src/rust/crates/` + `fathomdb-inspector` under `src/inspector/`), remove the stale `prettier` devDep clause on line 74, and point line 132 to `dev/interfaces/` (or include `rust-sdk.md`, `mcp.md`).

#### `P6-10` (`0.8.31` Slice 40 — Quick Fix, `0.8.33` Overhaul) — Repair `dev/ROADMAP.md` & `dev/plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md` current-position drift

- **Evidence:**
  - `dev/ROADMAP.md:24-40,58-67,69-211` still states `0.8.30 is the active release` (published `2026-11-21`), `0.8.27` is "Active implementation", `0.8.28` is "Proposed release scope", and `0.8.31`/`0.8.33` are "Review checkpoint" entries — contradicting the `2026-11-21` roadmap split (`0.8.31`–`0.8.35`).
  - `dev/plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md:28-32` similarly still lists `0.8.27`/`0.8.30` as active and `CUDA13 delivery belongs to 0.8.31`.
- **Proposal:** Perform a **lightweight `0.8.31` header/table sync** on `dev/ROADMAP.md` ("Current position" + "Release map" pointing to `scripts/release-current.py` and `0.8.31`–`0.8.35` plans) and `0.8.20-0.9.0-PROGRAM-SEQUENCING.md`, deferring full `dev/ROADMAP.md` + `dev/hygeine-roadmap.md` pruning/rewrite to `0.8.33` (`R33-01`).

#### `P6-11` (`0.8.31` Slice 40/50 — Quick Fix) — Sync `docs/contributing/index.md` with Slice 0 bootstrap fixes

- **Evidence:** Commit `db271323f` updated `scripts/setup-python-owned.sh` to prefer bare PATH Python `>=3.11` (e.g. `/usr/bin/python3.12`) over `uv` standalone Python `3.13` on Ubuntu 24.04, and `20269cee4` added upfront `src/ts/node_modules` preflight in `test_dev_environment_tools.sh`.
- **Gap:** `docs/contributing/index.md:24-38` describes `.venv` + `node_modules` setup without noting the Ubuntu 24.04 `/usr/bin/python3.12` preference or `src/ts/node_modules` preflight requirement (nor Inspector's scoped `local-ci.sh` once Slice 50 lands).
- **Proposal:** Add a brief note in `docs/contributing/index.md` in Slice 40/50; keep larger contributor-onboarding restructuring (`CONTRIBUTING.md`) in `0.8.33`.

---

### Explicitly Deferred to `0.8.33` Full OSS Docs Overhaul (`P6-12` – `P6-13`)

#### `P6-12` (`0.8.33` Slice 10 — OSS Repo Hygiene) — Root clutter, `dev/` pruning, `dev/README.md`, `ROADMAP.md`, and community files

- **Scope (`R33-01`):**
  - Remove stray root working files (`0.8.23-release-todo.md`, `CORPUS-01-supersession-questions.txt`, `plan-slice-85-recovery.md`, `plan-slice-85-review.md`).
  - Archive/prune historical pre-`0.8.30` slice runs/scratch under `dev/plans/`, rename `dev/hygeine-roadmap.md` -> `dev/hygiene-roadmap.md` and fix its stale `0.8.27`/`0.8.28` statuses, and rewrite `dev/README.md` + root `ROADMAP.md`.
  - Add `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, GitHub issue/PR templates, and a public-OSS root `README.md`.
- **Why not `0.8.31`:** Destructive/large-scale `dev/` pruning requires a dedicated `0.8.33` Slice 0 manifest and HITL sign-off; touching it in `0.8.31` risks churn against active `0.8.31`–`0.8.32` execution.

#### `P6-13` (`0.8.33` Slices 20–50 — Public Docs Architecture, Auto-Docs, MCP, Recipes, Memex E3)

- **Scope (`R33-02` – `R33-06`):**
  - Upgrade `scripts/generate-public-docs.py` (rich `rustdoc`, Python/TS API docs, CLI `--help`, `fathomdb-mcp` JSON schema docs, CI drift checks) and `mkdocs.yml` theme + sanitized Mermaid/SVG diagrams.
  - Author `docs/architecture/` internals (eliminating raw `dev/design/` links across `docs/`).
  - Author `docs/mcp/` for read + write/checkpoint tools (`0.8.32`), package-local agent docs, Memex E3 legacy framing (`ingest_with_extractor` vs. `actuate`), and 5 CI-tested runnable `docs/recipes/`.
- **Why not `0.8.31`:** Depends on `0.8.32` caller scope and read/write MCP contracts landing first.

---

## Quick `0.8.31` vs. `0.8.33` Placement Matrix

| Proposal | Target | Summary |
| --- | --- | --- |
| `P6-01` | `0.8.31` (Slice 40) | Fix `engine.write([])` verify snippets & empty-batch prose (5 files) |
| `P6-02` | `0.8.31` (Slice 20/40) | Fix `use_graph_arm` fused-hit wording + add `0.8.31` `graph_seed`/predicates |
| `P6-03` | `0.8.31` (Slice 40) | Document PyPI/npm CUDA + CE matrix, fix glibc table, drop `0.8.31` CUDA 13 promises |
| `P6-04` | `0.8.31` (Slice 20/40) | Document mixed-`source_id` single-txn `write(batch)` atomicity & zero-row rollback |
| `P6-05` | `0.8.31` (Slices 10/40/50) | Update hardcoded `schema 37` -> `38`, `fathomdb migrate`, `fathomdb-cli` PyPI wheel |
| `P6-06` | `0.8.31` (Slices 10/20/30) | Update `dangling_edge_endpoints`, 7-kind vector limit, and `graph-retrieval.md` |
| `P6-07` | `0.8.31` (Slice 40) | Fix error-class count (`44` -> `48`) & operation count (`44` -> `53`/dynamic) |
| `P6-08` | `0.8.31` (Slices 10–70) | Refresh `dev/interfaces/` metadata & `0.8.30 -> 0.8.31` release URLs/pins |
| `P6-09` | `0.8.31` (Slice 10/40) | Fix `AGENTS.md` crate count (`12` -> `13`), `prettier` claim, and `dev/interfaces/` list |
| `P6-10` | `0.8.31` (Slice 40) | Sync `dev/ROADMAP.md` & `PROGRAM-SEQUENCING.md` current-release tables |
| `P6-11` | `0.8.31` (Slice 40/50) | Add Slice 0 Python `3.12`/TS preflight & Slice 50 `local-ci.sh` notes to `docs/contributing/` |
| `P6-12` | `0.8.33` (Slice 10) | Root stray files, `dev/` pruning, `hygeine-roadmap` rename, OSS community files |
| `P6-13` | `0.8.33` (Slices 20–50) | Auto-doc pipeline, MkDocs theme/diagrams, `docs/architecture/`, `docs/mcp/`, `docs/recipes/`, E3 |
