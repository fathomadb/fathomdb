---
title: Slice 10 Status — 0.8.32 Prework Prep
status: CLOSED
target_release: 0.8.32
slice: 10
date: 2026-11-24
---

# Slice 10 Status — `0.8.32` Prework Prep (`PREWORK-PREP`)

- **Branch / Worktree:** `dev/0.8.32` in `/mnt/data/worktrees/release-0.8.32`
- **Plan:** [`dev/plans/0.8.32/features/slice-10/plan.md`](./plan.md)
- **Verdict:** **CLOSED / LANDED** — all 5 work packages (`WP-10A`–`WP-10E`) and `FIX-1` (`f961e4537`) landed, verified green by the full unconfined `./scripts/agent-verify.sh` gate (`246/249` suites passed, `3` platform-skipped, `0` failed, `0` security violations), `mkdocs build --strict`, and approved by independent `pro` adversarial code review (`VERDICT: APPROVE`, zero open P1/P2 findings).

## 1. Completed Work Packages

1. **WP-10A (`bab93843a`) — Worktree Bootstrap, Release State, `dev/<ver>` Validators & Hygiene:**
   - Provisioned checkout-owned `.venv` (`fathomdb 0.8.31`, `ruff 0.15.17`, `pyright 1.1.410`, `pytest 9.2.0`, `hypothesis`, `maturin`), root `node_modules`, and `src/ts/node_modules` (`test_dev_environment_tools.sh` exit `0`).
   - Updated `scripts/check-release-state-views.sh`, `scripts/preflight-release-state.py`, and their test suites (TDD RED → GREEN) to accept `refs/heads/dev/<ver>` and empty `closed` slices cleanly.
   - Commissioned `dev/plans/release-state-0.8.32.json` (`active_ref: "refs/heads/dev/0.8.32"`, ladder `10..70`, all 3 Arm R6 generated views), `dev/plans/runs/STATUS-0.8.32.md`, closed banner on `STATUS-0.8.31.md`, updated `dev/plans/0.8.32/{plan,scope}.md` (CUDA review, `T1`–`T3` NOT FIRED, Gate 1 rulings, fixed `scope.md:128` `module-boundary-policy.txt`), and retargeted ledger todo `TC-2a9f89fa-5302-44b7-a14e-97b9d9cbde90` (`0.8.29 → 0.8.32`, `seq: 387`).
   - **GitHub hygiene completed (main thread):** Dismissed all `106/106` non-shipping Dependabot alerts (`python/uv.lock` [20], `dev/experiments/0.8.29-slice60-projected-metadata/driver/Cargo.lock` [5], `dev/plans/0.8.27/features/slice-135/results/*/Cargo.lock` [81]; `0` failed; `44` default-branch alerts remain on `dev/tools/onnx/export-requirements.txt` and `package-lock.json`) and deleted the `2` remaining stale remote Dependabot branches (`dependabot/npm_and_yarn/multi-def6ea3883`, `dependabot/npm_and_yarn/multi-68e90142e5`) plus pruned `origin`.
   - **External publisher preflight (`fathomdb-mcp`):** Confirmed GitHub `pypi` environment and `CARGO_REGISTRY_TOKEN` secret exist. Live probes returned HTTP `404` for both `fathomdb-mcp` on PyPI and `fathomdb-mcp` on crates.io — **2 interactive owner web-UI actions are required before Slice 40/70:**
     1. Register a PyPI Trusted Publisher (Pending Publisher) for package `fathomdb-mcp` matching repository `fathomadb/fathomdb`, workflow `release.yml`, environment `pypi`.
     2. Ensure `CARGO_REGISTRY_TOKEN` has `publish-new` scope on crates.io (or perform the one-time initial crates.io publish/token check) for the new crate `fathomdb-mcp`.

2. **WP-10B (`9ffdfe0cf`) — Baseline `main` CI Regressions & `R32-07` Gate Diagnostics:**
   - Tracked `dev/fixtures/golden-0.8.30-schema37.sqlite` (`sha256: 59d56eabb9a6e817168c5c540b4bb2ec6eb7f741fb24a72c7617df4aa5004548`) and replaced hardcoded `/mnt/data/worktrees/...` lookups in `migrate_cli.rs` and `slice50_migrate_37_to_38.rs`.
   - Fixed hardcoded `0.8.30+tegra` version in `test_tegra_publication_operator_path.sh` (derived dynamically from `tegra-pages-retained.json`), hoisted Python `>= 3.11` lookup in `agent-lint.sh`, wired `generate-public-docs.py --check` into `agent-lint.sh`, and regenerated `docs/generated/contributor-map.md`.
   - Moved Pyright preflight ahead of Rust checks in `agent-typecheck.sh` and fixed `test_agent_typecheck_pyright_version.sh`.
   - Tightened `R32-07` error messages in `check-sdk-surface-parity.py`, `check-governed-surface-pin.sh`, and `dev/tools/module-boundary-gate/src/{main,config}.rs`, covered by TDD contract suite `scripts/tests/test_gate_diagnostics_0832.sh` registered in `scripts/agent-test.sh` (`fast` tier).

3. **WP-10C (`59b8e566a`, merged at `15c094afb`) — `R32-07` Orientation Docs, ADR Index Split & Factual Doc Fixes:**
   - Added `dev/design/cross-cutting-change-checklist.md` (`status: ACTIVE`), expanded `dev/design/README.md` (27 maintained docs), synced `document-lifecycle.json` + `current-owner-authority.json` (demoted `orchestration.md` to `historical`; preserved `LEGACY_BUDGET=43`), split `dev/adr/ADR-0.6.0-decision-index.md` into Active/Superseded (fixing `#66`/`#67`, indexing all 20 missing ADRs), and linked the checklist from `AGENTS.md` (`187 <= 300` lines) and `docs/contributing/index.md`.
   - Fixed `PropertyPredicate` 6-operator docs across 5 files, added `$.action_kind` to `docs/reference/errors.md`, purged residual `0.8.30` install/link references across `docs/` and `src/**/README.md` (including `0.8.31+tegra`), added CUDA+CE wheel matrices and glibc split, added `NEWEST_PUBLISHED_RE` negative banner check in `check-public-doc-truth.py` + `test_public_doc_truth.sh`, and neutralized consumer-project framing in `fathomdb-data-plane-architecture-v2.md`, `nested-source-projections.md`, and `src/python/fathomdb/engine.py:574`.

4. **WP-10D (`4fb1748b9`, merged at `f5574d884`, reconciled in `f961e4537`) — Decision 1 Schema 38 vs. Schema 39 & `SyncMode::Normal` vs. `Full` Empirical Experiment:**
   - Added `src/rust/crates/fathomdb-tc5-benchmark/src/bin/scope_schema_experiment.rs` and ran the `--release` 100k-node / 150k-edge / 10k-`operational_state` / 10k-`operational_mutations` (`seed = 20261124`) controlled benchmark across `single_scope`, `multi_scope_uniform`, and `multi_scope_skewed` comparing `B0`, `S38-PARTIAL-A1`, `S38-PARTIAL-A2`, and `S39-COMPOSITE` under `SyncMode::Normal` and `SyncMode::Full`.
   - Committed [`scope-schema-results.json`](./scope-schema-results.json) (`sha256: db8e75c0d35c4a03af02c071c281353fdfe78bf62eb2acab0f82a65920a2ecd2`) and [`scope-schema-experiment.md`](./scope-schema-experiment.md).
   - **Key empirical conclusions:**
     - **8-column `vec0` (`scope TEXT DEFAULT ''`) is mandatory:** `S38-PARTIAL-A1` (7-col CTE pre-selection) collapsed to `543.11 ms` p50 (`805.57 ms` p95) on `heavy_90pct` and `67.71 ms` p50 on `uniform_10pct`, whereas 8-col `vec0` (`S38-PARTIAL-A2` and `S39-COMPOSITE`) held `27.71–27.91 ms` p50 (`-94.9%`), `14.98–15.09 ms` p50 (`-77.9%`), and `9.00–9.06 ms` p50 with `0.9970` `recall@10` (`10.0/10` hits, `0` empty) on `light_1pct`.
     - **`S39-COMPOSITE` (Schema 39, recommended technical winner) vs. `S38-PARTIAL-A2` (Schema 38 fallback):** Both preserve single-scope read parity (`-4.4%` to `+3.1%` vs. `B0`) and zero RSS/`process_open` regression; `S39` is `5.83–6.65 MiB` smaller on multi-scope DBs, `-8.2%` to `-13.0%` faster on `node_batch_200_fresh` (`NORMAL`) vs. `S38-A2` (`-8.0%` vs. `B0`), `-26.0%` to `-42.8%` faster on all 4 multi-scope single-row checkpoints (`NORMAL`) vs. `S38-A2` (`-1.2%` [CI spans zero] to `-57.5%` vs. `S38-A1`), avoids silent partial-index scan traps (`WHERE scope != ''`), and provides native per-scope `(scope, logical_id)` / `(scope, collection_name, record_key)` isolation (noting a `191.01 ms` WAL autocheckpoint outlier on `multi_scope_skewed` `node_batch_200_upsert` `NORMAL`, dropping to `137.95 ms` under `FULL`). Trade-off: bumps `SCHEMA_VERSION` `38 → 39` (`fathomdb migrate 38 → 39`).
     - **`SyncMode::Normal` vs. `Full`:** `FULL` added `+431.7%` to `+2007.8%` (`5.3×–21.1×`) p50 latency on single-row checkpoints, validating Decision 2 Option 1.

5. **WP-10E (`62c91830e`) & `FIX-1` (`f961e4537`) — Four `0.8.32` ADRs & Slice 50 Venue Prep:**
   - Authored and indexed 4 ADRs in `dev/adr/ADR-0.6.0-decision-index.md` (`#99`–`#102`) and `dev/DOC-INDEX.md`:
     - `#99` [`ADR-0.8.32-mcp-read-write-and-distribution.md`](../../../../adr/ADR-0.8.32-mcp-read-write-and-distribution.md) (`accepted`)
     - `#100` [`ADR-0.8.32-opt-in-bulk-write-mode.md`](../../../../adr/ADR-0.8.32-opt-in-bulk-write-mode.md) (`accepted`)
     - `#101` [`ADR-0.8.32-durability-sync-mode-and-power-loss-oracle.md`](../../../../adr/ADR-0.8.32-durability-sync-mode-and-power-loss-oracle.md) (`accepted`; amended `ADR-0.6.0-durability-fsync-policy.md`)
     - `#102` [`ADR-0.8.32-caller-scope-isolation.md`](../../../../adr/ADR-0.8.32-caller-scope-isolation.md) (`proposed` — awaiting Decision 1 owner confirmation from `WP-10D`)
   - **Fuzzing venue prep:** Installed `cargo-fuzz v0.13.2` and verified an ephemeral `/tmp/` `cargo +nightly-2026-08-01 fuzz run --sanitizer address` smoke (`1,475,822` runs, `g++-13` + `librustc-nightly_rt.asan.a`, cleaned `/tmp/` afterward; zero root `Cargo.toml`/`Cargo.lock` mutations). Note for Slice 50: install `rustup component add --toolchain nightly-2026-08-01 llvm-tools` when collecting coverage dumps.
   - **Power-loss VM venue prep (`/dev/kvm`):** Confirmed `/dev/kvm`, `qemu-system-x86_64 8.2.2`, `qemu-img`, and `xorriso` on host (`~550 GiB` free on `/mnt/data`). No third-party image download was executed in Slice 10 (`AGENTS.md` § 11 third-party egress policy); exact pinned Ubuntu 24.04 Noble official cloud-image URL (`cloud-images.ubuntu.com`) and NoCloud `cidata` staging recipe are documented in `ADR-0.8.32-durability-sync-mode-and-power-loss-oracle.md` pending owner confirmation ahead of Slice 50.

## 2. Verification Summary

- Full unconfined `./scripts/agent-verify.sh` on `/mnt/data/worktrees/release-0.8.32` at `f961e4537`: **EXIT `0`**
  - `agent-lint.sh`, `agent-typecheck.sh`, `agent-build.sh`: **PASS** (`skip lint-rust-tegra-pool` on x86_64 host)
  - `agent-security`: **PASS** (`AC-036`, `AC-037`, `AC-038`, `AC-050a`, `AC-050c` — `0 violation(s), 0 blocker(s), 0 downgrade(s)`)
  - `agent-test.sh`: **PASS** (`246/249` suites passed, `3` platform-skipped, `0` excluded, `0` failed)
- `mkdocs build --strict`: **PASS** (`1.72s`, `0` warnings)
- `bash scripts/check-board-currency.sh`, `bash scripts/check-release-state-views.sh`, `bash scripts/agent-lint-md.sh`, `python3 scripts/check-public-doc-truth.py`, `python3 scripts/check-design-lifecycle.py`, `bash scripts/lint-design-status.sh`, `python3 scripts/check-architecture-authority.py`, `python3 scripts/check-agents-md-paths.py`, `.venv/bin/python scripts/generate-public-docs.py --check`: **ALL PASS**
- Independent `pro` adversarial code review (`slice-reviewer`): **`VERDICT: APPROVE`** after `FIX-1` (`f961e4537`; all `2` P1 and `3` P2 items verified resolved, zero open findings).
