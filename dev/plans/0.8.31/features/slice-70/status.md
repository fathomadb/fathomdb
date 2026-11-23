---
title: Slice 70 Status — Full Qualification, Paired Latency Gate, Feature-Complete GPU, Inspector, CLI Proof & Golden Archive
status: ACTIVE
target_release: 0.8.31
slice: 70
date: 2026-11-23
---

# Slice 70 Status — `0.8.31` Qualification (`QUALIFICATION`)

- **Branch / Worktree:** `release/0.8.31` in `/mnt/data/worktrees/release-0.8.31`
- **Plan:** [`dev/plans/0.8.31/features/slice-70/plan.md`](./plan.md)
- **Plan Review:** Independent adversarial Plan Reviewer (`d5dbd158`) returned **`APPROVE`** (Round 2).
- **Qualification Candidate Commits (`b33b959a7..4e23f212f`):**
  - `584bec282` — Layer 1: `plan.md` `ACTIVE`, WP-70F pre-publication docs (`docs/release-notes/0.8.31.md`, `mkdocs.yml`, `CHANGELOG.md`, `scripts/generate-public-docs.py`, `docs/generated/*`, `dev/{needs,requirements}.md` `P3-01`, `dev/interfaces/*.md` `P6-08`), and Phase 5.2 harnesses (`run-paired-latency-gate.py`, `build-golden-0.8.31.py`, `test_slice70_harness.py`).
  - `4f5ec7ce7`, `cfe63ca95` — WP-70C fixes: reconciled 28 feature-gated test targets for Schema 38 fields, fixed `slice50_migrate_37_to_38.rs` `RuntimeConfiguration(TooLate)` race under `--test-threads=1`, reconciled `scripts/test-feature-matrix.toml` (`fathomdb-inspector` `fixture-control`), and allowlisted pre-existing `#[ignore]` CUDA/stress tests in `scripts/test-skip-allowlist.toml`.
  - `e58ecf394` — `AC-050a` (`V05_VERBS`) fix: renamed internal engine/py/napi symbol `read_operational_collection` to `read_operational_collection_info` (`readOperationalCollectionInfo`) while preserving all public SDK APIs unchanged.
  - `a2e8e413a`, `dc8e9929b`, `ab63b8a52` — WP-70A fixes: resolved `migrate_cli` runtime-init race, `slice55_composed_integrity` ADR `#75` `[t_valid, t_invalid)` edge window, `slice30_measure.rs` & `g_runtime_tests.py` Schema 38 updates, `operator/data_plane.rs` single-owner gating + `module-boundary-policy.txt` classification, `slice65_managed_connection_owners.py` 3 audited `operator/migrate.rs` opens, `phase3-root-reconciliation.md` `lib_rs_blob`, `test_slice10_preparation.py` + `dev/DOC-INDEX.md`, `test_check_sdk_surface_parity.py` (`93`/`56`), `slice135_python_capabilities.py`, `test_cuda_release_contract.sh`, `test_bootstrap_heavy.sh`, and `release-state-0.8.31.json` views.
  - `030faecde`, `9531fe892`, `b146dc78d`, `f13244689` — WP-70B performance optimizations: 6 partial Schema 38 indexes (`WHERE ... IS NOT NULL`), `canonical_property_values` `WITHOUT ROWID` + redundant cursor index removal, O(1)-RAM streaming SHA-256 open-time projection parity (`projection_registry.rs`), covering CPV index `INNER JOIN` pushdown without redundant `value_type` predicates (`graph_expand/execution.rs`, `filter.rs`), 64-way batched strict edge endpoint validation + hoisted `erasure_queue::guard_plain` (`write_commit.rs`, `erasure_queue.rs`), cached batch statements + writer statement cache capacity `64`.
  - `56ff722f2` — Reconciled `experiments/sdk_registry.py` (12 live governed operations), `experiments/sdk_drivers/python_driver.py` (`open_report().dense_disabled`), `experiments/sdk_drivers/typescript_driver.mjs` (Node `>=25 <27`), `run-paired-latency-gate.py` (`repetitions: 4` for `p99_ms` `n>=100` floor), and committed `golden-0.8.31-manifest.json`.
  - `4e23f212f` — Committed unified all-green `dev/plans/0.8.31/features/slice-70/paired-latency-report.json`.
- **Pre-Publication Boundary Preserved:** Axis-W remains at `0.8.30`, `"published": null`, `publish_gate.publication_authorized: false`, `dev/acceptance.md` untouched (`git diff 6685b0ccf -- dev/acceptance.md` empty), zero `v0.8.31` git tags locally or on `origin`, and zero external consumer-project references added to tracked public docs/ADRs/interfaces.

## 1. Qualification Matrix Summary

| Work Package | Scope | Candidate SHA | Result | Key Receipt |
| --- | --- | --- | --- | --- |
| **WP-70A** | Unconfined `agent-verify.sh`, long/perf gates, supply-chain & security | `4e23f212f` (verify), `ab63b8a52` (supply-chain/gitleaks), `dc8e9929b` (long/perf) | **PASS** | `agent-verify.sh` exit 0 (`243/246` suites, `0` failed, `3` skipped, `0` excluded); `agent-security` `0` violations / `0` blockers / `0` downgrades; all 7 canonical `AGENT_LONG=1` & `--release` perf gates pass; `gitleaks-current.sh` & single-branch `gitleaks-history.sh` `0` leaks |
| **WP-70B** | Paired `v0.8.30` vs. `0.8.31` latency/resource gate (`10k`/`50k`/`100k`), `50k` micro-gates & pinned `10k` SDK suite | `f13244689` / `4e23f212f` | **PASS** | [`paired-latency-report.json`](./paired-latency-report.json): `passed: true` (`10000: true`, `50000: true`, `100000: true`, `micro_gates_50k: true`, `0` parity mismatches); `--run-sdk-benchmark-10k` Python & TypeScript both `"complete"` |
| **WP-70C** | Feature-complete & live GPU matrix on RTX 3090 GPUs `0,1` | `cfe63ca95` (full matrix) + `9531fe892` (`cached-cpu` & migrate spot re-verify) | **PASS** | `check-test-target-coverage.py`: `431` targets (`46` feature-complete only); `test-feature-complete.sh`: `32` runs, `250` planned tests: **`223` passed, `0` failed, `27` allowlisted ignored** |
| **WP-70D** | Inspector `0.3.0` `--full-workspace` & 4-arg `qualify-installed-network.sh` | `4e23f212f` | **PASS** | `local-ci PASS mode=full-workspace`: `25/25` tooling, `11/11` unit, **`86/86` Playwright browser tests pass** across all 8 specs incl. `attributes.mjs`; `ELF maximum GLIBC 2.39`; `0` worker Internet-family attempts, `0` supervisor outbound attempts, `0` cache mutations |
| **WP-70E** | Clean-venv `fathomdb` + `fathomdb-cli` wheels & stripped-`PATH` `fathomdb migrate` golden Schema-37 proof | `9531fe892` | **PASS** | Wheels built & validated (`sdk`: 26 members, `cli`: 6 members), GLIBC `<= 2.39`, zero `cargo`/`rustc` on `PATH`, `AC-041` intact, `Engine.open` refuses Schema 37 untouched (`59d56eabb...`), `fathomdb migrate` upgrades `37 -> 38` with backup SHA-256 parity, `doctor check-integrity --full` clean before & after post-upgrade typed-property write/read/`as_of`/erase |
| **WP-70F** | Pre-publication docs, `CHANGELOG.md`, `docs/generated/*`, `dev/{needs,requirements}.md`, `dev/interfaces/*.md` | `584bec282` / `ab63b8a52` | **PASS** | `check-public-doc-truth.py`, `generate-public-docs.py --check`, `check-removal-changelog.sh` (default & `--base v0.8.30 --head HEAD`), `verify-release-gates.sh`, `agent-lint-docs.sh`, `agent-lint-md.sh`, `mkdocs build --strict` all exit `0` |
| **WP-70G** | Closed golden Schema-38 archive for `0.8.34` | `9531fe892` / `56ff722f2` | **PASS** | `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.31-schema38.sqlite` (`0444`, `1,007,616` bytes, SHA-256 `e0a078bde37ec3e973bba8b03c9bfb7c42505b16da1b038ca946fb08332c8036`), manifest committed at [`golden-0.8.31-manifest.json`](./golden-0.8.31-manifest.json) |

## 2. WP-70A — Full Workspace Verification, Long/Perf Gates & Supply-Chain Receipts

- **Unconfined Clean-Tree `./scripts/agent-verify.sh` on `4e23f212f` (via `local-ci.sh --full-workspace` Stage `source`):**
  - `agent-lint.sh`: PASS (module boundaries, clippy, rustfmt, migration policy, ruff, actionlint, markdown, lychee).
  - `agent-typecheck.sh`: PASS (`cargo check`, `pyright`, `tsc --noEmit`).
  - `agent-security`: **0 violation(s), 0 blocker(s), 0 downgrade(s)** — `AC-036` no-listen-syscall PASS, `AC-037` netns-deny-egress PASS, `AC-037` catch (offline + live netns) PASS, `AC-037` policy self-test PASS, `AC-038` FTS5-injection-safe (2/2) PASS, `AC-050a` AST scan (Rust / Python / TS) PASS, `AC-050c` removal-detect (47 documented removals) PASS.
  - `agent-test.sh`: **`243/246` suites passed, `0` failed, `3` skipped, `0` excluded**.
- **Canonical `AGENT_LONG=1` & `--release` Performance Gates:**
  - `ac_021` (60s): `59` projection add/drop rebuilds, `sqlite_schema=0` — **PASS**.
  - `projection_cursor_bounds_observed_row_count`: `1000` searches, `341` concurrent writes, `0` violations — **PASS**.
  - `ac_011a` (1KB release, 60s): `81,288` commits, `1,354.6 commits/s` (floor `1,000`) — **PASS**.
  - `ac_011b` (100KB release, 60s): `18,610` commits, `310.2 commits/s` (floor `100`) — **PASS**.
  - `ac_012` (FTS5 `N=10000` release): `p50=1 ms`, `p99=3 ms` — **PASS**.
  - `ac_013` (Vector `N=10000 dim=768` release): `p50=61 ms`, `p99=68 ms` (budget `<= 80 ms`) — **PASS**.
  - `ac_081` (`AGENT_LONG=1` release, 8 threads): `sequential=350.33 ms` (`< 500 ms`), `concurrent=46.35 ms` (`< 100 ms`), speedup `7.56x` — **PASS**.
- **Supply-Chain, License & Secret Scans:**
  - `scripts/security/gitleaks-current.sh`: `252.99 MB` scanned, **0 leaks found**.
  - `scripts/security/gitleaks-history.sh` (clean single-branch clone of `release/0.8.31` `HEAD`): `252.99 MB` scanned, **0 leaks found**.
  - `scripts/check-license-consistency.sh --with-wheel`: **OK (MIT)** across all 8 Rust crates, all 6 npm packages, and both Python wheels (`fathomdb`, `fathomdb_cli`).
  - `scripts/check-governed-surface-pin.sh`: **OK** (`93` allowlist / `5` core / `5` recovery_denylist / `2` runtime_controls).
  - `scripts/check-sdk-surface-parity.py` + `--binding rust --rust-crate src/rust/crates/fathomdb-sdk`: **OK** (`93` signed members / `56/56` live canonical operations).
  - `scripts/check-pinned-override-rot.sh`, `scripts/check-platform-capabilities.sh` (`10` loader triples, `5` matrix, `5` published), `scripts/set-version.sh --check-files` (`Axis W = 0.8.30; Axis E = 0.8.0`): **ALL PASS**.

## 3. WP-70B — Paired `v0.8.30` vs. `0.8.31` Latency, Resource & Micro-Gates

Executed on the same Linux x86_64 host comparing `v0.8.30` (`db271323f`) vs. `0.8.31` (`f13244689`) with interleaved pair execution, `10,000`-resample paired bootstrap 95% CIs (`--query-samples 200 --warmup-rounds 3`), and strict zero-mismatch `logical_id` parity. Full machine-readable report: [`paired-latency-report.json`](./paired-latency-report.json) (**`passed: true`**).

### 3.1 Paired Multi-Scale Results (`10k` / `50k` / `100k` Nodes)

| Metric | 10k Base -> Cand (`p50` / `p95` ms) | 50k Base -> Cand (`p50` / `p95` ms) | 100k Base -> Cand (`p50` / `p95` ms) | 100k Observed Delta (`p50` / `p95`) | Status |
| --- | --- | --- | --- | --- | --- |
| `process_open` | `130.56/133.45` -> `53.74/57.55` | `581.58/639.41` -> `141.95/159.00` | `1121.14/1164.54` -> `246.17/257.97` | **`-874.02 ms` / `-906.58 ms`** (4.6x faster) | **PASS** |
| `warm_reopen` | `124.32/129.51` -> `48.27/50.15` | `553.91/564.36` -> `131.45/133.24` | `1113.19/1175.78` -> `250.13/265.27` | **`-862.05 ms` / `-910.51 ms`** (4.5x faster) | **PASS** |
| `write_node_batch_200` | `40.88/50.87` -> `26.68/36.79` | `44.71/58.66` -> `31.88/45.15` | `46.95/60.52` -> `34.74/48.58` | **`-12.22 ms` / `-11.94 ms`** (-26%) | **PASS** |
| `write_edge_batch_200` | `15.59/23.81` -> `12.71/19.23` | `18.18/28.28` -> `15.23/24.53` | `18.12/28.65` -> `16.13/26.08` | **`-1.99 ms` / `-2.57 ms`** (-11%) | **PASS** |
| `search_text_only` | `7.24/7.77` -> `7.41/7.90` | `35.58/36.41` -> `35.64/36.73` | `75.57/80.13` -> `75.56/80.08` | **`-0.01 ms` / `-0.05 ms`** | **PASS** |
| `search_hybrid_cpu_no_embedder` | `12.01/12.90` -> `12.35/13.11` | `60.93/62.64` -> `62.34/64.02` | `125.71/132.20` -> `128.51/134.20` | `+2.80 ms` (`ci95u +3.34 <= 6.29`) / `+2.00 ms` (`ci95u +4.28 <= 6.61`) | **PASS** |
| `graph_expand_2hop` | `0.72/0.89` -> `0.84/1.03` | `0.98/1.17` -> `1.05/1.23` | `1.38/1.76` -> `1.47/1.83` | `+0.09 ms` / `+0.07 ms` (`<= 1.00 ms`) | **PASS** |
| `database_bytes` (`<= +15.0%`) | `17.52 MB` -> `19.85 MB` (`+13.30%`) | `88.28 MB` -> `99.84 MB` (`+13.09%`) | `177.22 MB` -> `200.42 MB` (`+13.09%`) | `+13.09%` (`<= +15.0%`) | **PASS** |
| `peak_rss_bytes` (`<= +10.0%`) | `72.91 MB` -> `75.26 MB` (`+3.23%`) | `107.33 MB` -> `109.41 MB` (`+1.93%`) | `162.95 MB` -> `165.20 MB` (`+1.38%`) | `+1.38%` (`<= +10.0%`) | **PASS** |
| `logical_id_parity` | `0` mismatches | `0` mismatches | `0` mismatches | `0` mismatches | **PASS** |

### 3.2 `50k` Property, Hub & Historical `as_of` Micro-Gates + Pinned `10k` SDK Suite

- **`high_fanout_predicate` (`<= 0.25x` unindexed):** indexed `p50=57.59 ms` / `p95=62.05 ms` vs. unindexed `p50=312.00 ms` / `p95=351.59 ms` -> `p50_ratio=0.1846`, **`p95_ratio=0.1765`** — **PASS**.
- **`hub_5000_edge_predicates` (`p95 <= 5.0 ms`):** `p50=3.07 ms`, **`p95=4.37 ms`**, `max=4.65 ms` — **PASS**.
- **`as_of_composite_seek` (`1,275` `_fathomdb_commit_timestamps` rows; `p95 <= 5.0 ms`):**
  - `as_of_write_cursor_all_kinds`: `p50=2.22 ms`, `p95=3.25 ms` — **PASS**.
  - `as_of_write_cursor_verifies`: `p50=1.58 ms`, `p95=2.30 ms` — **PASS**.
  - `as_of_epoch_ms_all_kinds`: `p50=2.38 ms`, `p95=3.55 ms` — **PASS**.
  - `as_of_epoch_ms_verifies`: `p50=1.47 ms`, `p95=2.08 ms` — **PASS**.
- **Pinned `10k` Cross-SDK Benchmark (`--run-sdk-benchmark-10k`, `experiments/sdk_benchmark.py`):**
  - `fts-10k-latency-python`: `"status": "complete"` (`p50=29.80 ms`, `p95=40.92 ms`, `p99=45.46 ms`, `database_bytes=8,413,184`, `driver_peak_rss_bytes=102,514,688`).
  - `fts-10k-latency-typescript`: `"status": "complete"` (`p50=30.26 ms`, `p95=42.69 ms`, `p99=47.13 ms`, `database_bytes=8,413,184`, `driver_peak_rss_bytes=137,359,360`).

## 4. WP-70C, WP-70D, WP-70E, WP-70F & WP-70G Detailed Receipts

- **WP-70C (`test-feature-complete.sh` on RTX 3090 GPUs `0,1`):**
  - `check-test-target-coverage.py`: PASS (`431` targets; `46` feature-complete only).
  - Unconfined `CUDA_DEVICE_ORDER=PCI_BUS_ID CUDA_VISIBLE_DEVICES=0,1 FATHOMDB_FEATURE_COMPLETE=1 FATHOMDB_REQUIRE_LIVE=1 bash scripts/test-feature-complete.sh`: **EXIT 0** — `32` runs, `250` planned tests: **`223` passed, `0` failed, `27` allowlisted ignored** (covering `slice72_concurrent_gpu`, `slice50_migrate_37_to_38`, `embed-cuda`, `rerank-cuda`, `onnx-embedder`, `fathomdb-inspector[cached-cpu]`, `fathomdb-mcp[default-embedder]`, `fathomdb-sdk[embed-cuda]`). Display GPU `2` (Quadro K620) remained untouched.
- **WP-70D (Inspector `0.3.0` `--full-workspace` + 4-arg Network Silence on `4e23f212f`):**
  - Generated all 6 fresh Schema-38 fixtures (`vector`, `person`, `study-training`, `study-unseen`, `f06` + 100k `scale` `121M`, `lexical-deadline` `122M`).
  - `local-ci.sh --full-workspace`: **`local-ci PASS mode=full-workspace source=4e23f212fac4d71330021e9ebc4bd908142140b7; source/tooling/unit/native-build/seal/candidate/abi/installed`** — `tooling` `25/25`, `unit` `11/11`, `installed` **`86/86` Playwright browser tests passed** (`0` failed, `0` skipped) across `installed.mjs`, `attributes.mjs`, `comparison.mjs`, `lexical.mjs`, `vector.mjs`, `graph.mjs`, `workspace.mjs`, `ux.mjs`; `ELF maximum GLIBC 2.39`; archive SHA-256 `4047d641115bca3a9c989261142c06a1bbe48cf18fd4012547699db775ebc94f`; build ID `89ffe1a7ff11d3d059a9d8b6ac8f343d98c19286e739a1458a001582e38b978e`.
  - `qualify-installed-network.sh` (4-arg, Landlock ABI 8, glibc 2.39, kernel `7.0.0-43-generic`, Chromium `156.0.8078.4`): **PASS** (`traced_files=320`, `supervisor_traced_pids=49`, `worker_traced_pids=13`, **`worker_internet_family_attempts=0`, `supervisor_outbound_attempts=0`**, `source_asset_readonly_opens=3`, `source_cache_mutations=0`).
- **WP-70E (Clean-Venv Wheels & Stripped-`PATH` `fathomdb migrate` Proof):**
  - Built release wheels `fathomdb-0.8.30-cp310-abi3-manylinux_2_39_x86_64.whl` (`26` members, `--kind sdk` ok) and `fathomdb_cli-0.8.30-py3-none-manylinux_2_34_x86_64.whl` (`6` members, `--kind cli` ok); installed `fathomdb[cli]==0.8.30` into isolated venv; `check-glibc-floor.sh --floor 2.39` passed on both `_fathomdb.abi3.so` and `bin/fathomdb`.
  - Under stripped `PATH` (verified `shutil.which("cargo") is None` and `shutil.which("rustc") is None`): `fathomdb --version` PASS, `AC-041` (`fathomdb` zero `console_scripts`, `perf-regression-check` absent) PASS, `smoke_slice55_installed.py` PASS, `Engine.open` refused disposable copy of `golden-0.8.30-schema37.sqlite` with `IncompatibleSchemaVersionError` without mutating SHA-256 `59d56eabb9a6e817168c5c540b4bb2ec6eb7f741fb24a72c7617df4aa5004548`, `fathomdb migrate --backup-dir ... --json` upgraded `37 -> 38` with matching backup SHA-256 and `PRAGMA user_version == 38`, `scope == ''`, `properties IS NULL` on pre-upgrade rows, `doctor check-integrity --full --json` `clean` across `logical`/`physical`/`semantic`, and post-upgrade Python SDK verified `req:1`/`ver:1`, `secret:1` absent, `agent_checkpoints`/`agent_audit`, `graph.expand` with `as_of_write_cursor="7"` & `as_of_epoch_ms`, typed `properties` write/readback, `erase_source`, and post-erasure `doctor check-integrity --full --json` `clean`.
- **WP-70F (Pre-Publication Docs & Truth Guard):**
  - `docs/release-notes/0.8.31.md` (stamped pre-publication), `mkdocs.yml`, `CHANGELOG.md`, `scripts/generate-public-docs.py` (`release/0.8.31/`), `docs/generated/{sdk-operation-map,contributor-map}.md`, `dev/{needs,requirements}.md` (`P3-01`), and `dev/interfaces/*.md` (`P6-08`) all updated and verified via `check-public-doc-truth.py`, `generate-public-docs.py --check`, `check-removal-changelog.sh`, `verify-release-gates.sh`, `agent-lint-docs.sh`, `agent-lint-md.sh`, and `mkdocs build --strict`.
- **WP-70G (Golden Schema-38 Archive for `0.8.34`):**
  - `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.31-schema38.sqlite` (`-r--r--r--`, `1,007,616` bytes, SHA-256 `e0a078bde37ec3e973bba8b03c9bfb7c42505b16da1b038ca946fb08332c8036`), Schema 38 (`WITHOUT ROWID` `canonical_property_values` + partial indexes), `doctor check-integrity --full --json` clean, zero WAL, zero erased canary bytes, manifest committed at [`golden-0.8.31-manifest.json`](./golden-0.8.31-manifest.json).

## 5. Slice 80 HITL Publication Handoff Checklist (STOP Gate)

Execution **STOPS strictly before Slice 80 (`PUBLICATION`)**. Separate explicit owner authorization is required for:

1. **PyPI Pending Publisher prerequisite (`fathomdb-cli`):** Configure a PyPI Trusted Publisher (Pending Publisher, `environment: pypi`) for the new `fathomdb-cli` project targeting `.github/workflows/release.yml` **before** tagging `v0.8.31`.
2. **Axis-W version cut & merge:** Run `bash scripts/set-version.sh --workspace 0.8.31` and merge `release/0.8.31 -> main` (`verify-release-gates.sh` Check 3).
3. **Tag, publish & post-publish smoke:** Create and push tag `v0.8.31`, verify all registry uploads (`crates.io`, PyPI `fathomdb` + `fathomdb-cli`, `npm`, Tegra Pages, GitHub Release Inspector `0.3.0`), and run clean-venv post-publish smoke installs.
4. **Post-publish truth flip (`P3-21`):** Add `"0.8.31": 38` to `RELEASE_SCHEMAS` in `scripts/check-public-doc-truth.py`, remove `(pre-publication)` labels from `docs/release-notes/0.8.31.md`, `mkdocs.yml`, and `CHANGELOG.md`, and record the `"published"` receipt in `dev/plans/release-state-0.8.31.json`.
