---
title: Slice 60 Status — 0.8.32 Full Qualification, Paired Latency Gate, GPU/Inspector/Packaging Smokes, Pre-Publish Docs & Golden Archive
status: LANDED
target_release: 0.8.32
slice: 60
date: 2026-11-27
---

# Slice 60 Status — `QUALIFICATION`

## Outcome

**LANDED locally on `dev/0.8.32`** (`/mnt/data/worktrees/release-0.8.32`, frozen qualified product SHA **`8c0021f051ba5a6a6d0432ac658b531deac9cd49`**). Satisfies `AC32-60-01` through `AC32-60-09` and **STOPS strictly before Slice 70 (`PUBLICATION`)** with Axis-W held at `0.8.31`, `"published": null`, `publish_gate.publication_authorized == false`, `dev/acceptance.md` untouched, zero `v0.8.32` tags, and zero `git push`.

1. **Step 1 — Harnesses (`TDD RED -> GREEN`) & `WP-60F` Pre-Publication Docs (`5625cd54e`):**
   - Landed [plan.md](./plan.md) (`APPROVED` by independent `pro` adversarial review), [run-paired-latency-gate.py](./run-paired-latency-gate.py), [test_slice60_harness.py](./test_slice60_harness.py) (`11/11` passing), and [build-golden-0.8.32.py](./build-golden-0.8.32.py).
   - Dedicated 4-tier documentation subagent updated `docs/release-notes/0.8.32.md` (pre-publication stamped), `mkdocs.yml`, `CHANGELOG.md` (`## 0.8.32`), `scripts/generate-public-docs.py` (`SOURCE_URL` -> `release/0.8.32/`) + `docs/generated/*`, and `dev/{needs,requirements}.md`, passing all `17/17` scoped doc/truth/lifecycle/MkDocs validators (`dev/acceptance.md` untouched).
2. **Step 2 — `WP-60A` Source, Security, Supply-Chain, `AGENT_LONG=1` / `--release` Perf Gates & Standing Tripwires (`AC32-60-01`):**
   - **Full unconfined `./scripts/agent-verify.sh` on clean `HEAD` `8c0021f05`** (executed as `stage=source` of `local-ci.sh --full-workspace`): **EXIT `0`** — `agent-security: 0 violation(s), 0 blocker(s), 0 downgrade(s)` (live `AC-036` netns/strace, `AC-037`, `AC-038`, `AC-050a`, `AC-050c`), `agent-test.sh: 246/249 suites passed (skipped=3 excluded=0)`.
   - **All 7 canonical `AGENT_LONG=1` / `--release` gates PASSED on quiet host:**
     - `ac_021_zero_sqlite_schema_warnings_under_concurrent_reads_and_ddl`: `duration=60s, cycles=59, rebuilds=59, sqlite_schema=0`
     - `projection_cursor_bounds_observed_row_count`: `searches=1000, successful_writes=3224, violations=0, hit_cursor_violations=0`
     - `ac_011a_write_throughput_1kb`: **`1659.248 commits/s`** (`99,556` commits / `60s`; min `1000.0`)
     - `ac_011b_write_throughput_100kb`: **`240.178 commits/s`** (`14,414` commits / `60s`; min `100.0`)
     - `ac_012_text_query_latency_on_fts5_path` (`n=10000`): **`p50=0 ms, p99=1 ms`** (`seed_ms=1388`)
     - `ac_013_vector_retrieval_latency` (`n=10000, dim=768`): **`p50=64 ms, p99=71 ms`** (`seed_ms=3103`)
     - `ac_081_absolute_read_performance`: **`sequential=382.53 ms, concurrent=47.53 ms, ratio=8.048x`** (`sequential_failure=false, concurrent_failure=false`)
   - **Non-debug `check.sh` legs & supply-chain/governance gates (all EXIT `0`):** `cargo fmt --check`, `cargo check --workspace --locked`, `.venv/bin/python -m compileall -q src/python/fathomdb src/python/tests`, `cd src/ts && npx tsc --noEmit`, `mkdocs build --strict`, `gitleaks-current.sh` (`256.72 MB`, `0` leaks), single-branch `dev/0.8.32` `gitleaks-history.sh` (`256.72 MB`, `0` leaks), `check-license-consistency.sh --with-wheel` (`MIT` across 9 crates, 6 npm packages, 3 wheels), `check-governed-surface-pin.sh` (`58` ops / `97` allowlist, `core=5`, `recovery_denylist=5`, `runtime_controls=2`), `check-sdk-surface-parity.py` (Python/TS `97`/`58` + Rust `58/58`), `check-pinned-override-rot.sh`, `check-platform-capabilities.sh` (`10/5/5`), `set-version.sh --check-files` (`Axis W = 0.8.31; Axis E = 0.8.0`), `verify-release-gates.sh` (`ok`), standing tripwires `T1`–`T3` **NOT FIRED** (`RRF_K=30.0`, `RRF_WEIGHT_TEXT=3.0`, `RRF_WEIGHT_VECTOR=1.0`, `DEFAULT_EMBEDDER_NAME="fathomdb-bge-small-en-v1.5"`, `MAX_SEQUENCE_TOKENS=512`, `0` exact-scan symbols), and `TC-2a9f89fa-5302-44b7-a14e-97b9d9cbde90` re-verified.
3. **Step 3 — `WP-60B` Paired `v0.8.31` vs. `0.8.32` Latency/RSS/DB-Size Gate & `0.8.32` Micro-Gates (`AC32-60-02`, `AC32-60-03`):**
   - Bounded performance optimizations `FIX-PERF-1` through `FIX-PERF-8` (`db3938132`..`d887e4465`) eliminated single-scope FTS scope reprepare, deferred reader/runtime init, batched edge writes, compacted Schema 39 visibility triggers, right-sized reader cache/lookaside, and switched `rederive_projections_on_boot` to zero-alloc `row.get_ref(...)` borrows and 4 KiB buffered `StreamDigest`.
   - **Primary paired report [paired-latency-report.json](./paired-latency-report.json) (`02cdafdc1`, `candidate_sha = d887e4465`, `baseline_sha = 8965af93d`, `query_samples = 200`, `10,000` bootstrap resamples, `passed: true`, `0` `logical_id` parity mismatches):**
     - **`10k` (`passed: true`):** `process_open 55.16 -> 45.38 ms`, `warm_reopen 49.58 -> 41.84 ms`, `write_node_batch_200 34.23 -> 24.18 ms` (`p95 50.66 -> 32.83 ms`), `write_edge_batch_200 16.57 -> 16.59 ms` (`p95 24.83 -> 22.84 ms`), `search_text_only 7.71 -> 7.53 ms`, `search_hybrid 12.88 -> 7.58 ms`, `graph_expand_2hop 0.90 -> 1.31 ms`, `database_bytes +6.17%` (`<= 15%`), `peak_rss_bytes +7.85%` (`<= 10%`).
     - **`50k` (`passed: true`):** `process_open 138.53 -> 124.98 ms`, `warm_reopen 131.03 -> 122.23 ms`, `write_node_batch_200 38.16 -> 27.60 ms` (`p95 57.10 -> 37.77 ms`), `write_edge_batch_200 20.01 -> 18.89 ms` (`p95 30.70 -> 27.58 ms`), `search_text_only 36.78 -> 37.56 ms`, `search_hybrid 63.91 -> 37.53 ms`, `graph_expand_2hop 1.16 -> 1.43 ms`, `database_bytes +5.89%`, `peak_rss_bytes -2.06%`.
     - **`100k` (`passed: true`):** `process_open 240.84 -> 224.28 ms`, `warm_reopen 235.23 -> 218.94 ms`, `write_node_batch_200 43.40 -> 29.41 ms` (`p95 64.30 -> 38.81 ms`), `write_edge_batch_200 20.37 -> 19.40 ms` (`p95 31.25 -> 29.73 ms`), `search_text_only 77.81 -> 79.02 ms`, `search_hybrid 130.49 -> 79.19 ms` (`-39.3%`), `graph_expand_2hop 1.51 -> 1.48 ms`, `database_bytes +6.25%`, `peak_rss_bytes -1.86%`.
     - **`Gate1-D10` `10k` `search_with_evidence` (`passed: true`, threshold `<= 0.15x`):** `include_explanation=False` **`119.57 -> 10.59 ms` (`p50 0.089x`, `p95 0.102x`)**; `include_explanation=True` **`120.07 -> 10.99 ms` (`p50 0.092x`, `p95 0.094x`)**; `0` parity mismatches.
   - **Micro-gates & cross-SDK `10k` check (`AC32-60-03`):** `slice40_status_cache` (`7/7`), `slice41_query_scale` (`11/11`), `slice41_frozen_read_stability` (`6/6`), and `slice38_query_plans` (`5/5`) all passed; `experiments/sdk_registry.py` reconciled for `58` live ops (`03eb3f197`) and `experiments/sdk_benchmark.py` `10k` completed for both `python` (`p50=26.07 ms, p95=32.24 ms, oracle=verified, db=8,921,088 B`) and `typescript` (`p50=25.81 ms, p95=33.37 ms, oracle=verified, db=8,921,088 B`) outside tracked `experiments/`.
4. **Step 4 — `WP-60C` GPU Matrix (`AC32-60-04`) & `WP-60E` 3-Wheel / Migration / `R32-04` / TS Qualification (`AC32-60-06`):**
   - **`WP-60C`:** `check-test-target-coverage.py` passed (`456` targets, `47` feature-complete-only, `0` drift). Unconfined RTX 3090 GPU (`CUDA_DEVICE_ORDER=PCI_BUS_ID CUDA_VISIBLE_DEVICES=0,1`, GPU `2` untouched) `test-feature-complete.sh` exited `0`: **`32` runs, `254` planned tests: `227` passed, `0` failed, `27` allowlisted ignored**.
   - **Real Schema `38 -> 39` migration defect fixed (`579165215`):** Migrating `golden-0.8.31-schema38.sqlite` caught `STEP_39_SQL` leaving non-tail Schema-38 `AdminSchema` batch cursors as `owner_kind='canonical'`; fixed via TDD RED->GREEN in `fathomdb-schema/src/lib.rs`, `slice20_migrate_38_to_39.rs` (upgraded to all 5 `data-plane-integrity` checks), and `slice40_reopen_cursor_and_projection_ns.rs`.
   - **`WP-60E`:** Built and verified all 3 lockstep wheels (`fathomdb`, `fathomdb-cli`, `fathomdb-mcp` with `--features default-embedder`, `check_python_wheel_contents.py --kind {sdk,cli,mcp}` OK), verified `cargo check --locked --offline -p fathomdb-mcp --no-default-features` thin crates.io build, confirmed `GLIBC <= 2.39` across `_fathomdb.abi3.so`, `bin/fathomdb`, and `bin/fathomdb-mcp`, and verified under a clean venv (`"fathomdb[cli,mcp]==0.8.31"`) with stripped `PATH` (zero `cargo`/`rustc`): `AC-041` (`0` console_scripts, `perf-regression-check` absent), `smoke_slice55_installed.py`, `fathomdb-mcp` (`--version`, `SC-01`/`SC-05` exit `2`, RO `7` tools + `writes_disabled`, RW `9` tools + bound `scope` + `sync_mode="full"` + live `actuate`/`search`), `fathomdb migrate 38 -> 39` on a disposable copy of `golden-0.8.31-schema38.sqlite` (source SHA-256 `e0a078bde...` untouched, backup parity, `uv=39`, `doctor check-integrity --full` + all 5 `data-plane-integrity` checks `0` findings, post-upgrade `ScopePolicy`/`BulkNonStrict` + `edge_endpoint_integrity` count=`2`/`SyncMode`/`ProjectionTarget`/`PerHitExplain.vector_cosine`/`ADR #103`), **`R32-04` installed-wheel backup/restore/rebuild proof** (`doctor safe-export` -> separate `--rebuild-projections` & `--rebuild-vec0` exit `64` -> dense reopen + `drain()` -> `--complete-erasures` exit `64` -> `0` findings + full semantic parity), and **`690/690` TypeScript tests** (`0` fail, `0` skipped) + release `npm run build`.
5. **Step 5 — `WP-60D` Inspector `--full-workspace`, Network Silence & Dual-Feature Cargo Qualification (`AC32-60-05`):**
   - Registered 2 `FIX-PERF-1` unit test modules (`ce267cb15`) and folded `project_canonical_{node,edge}_row_with_terminal_sink` wrappers into `project_canonical_{node,edge}_row` (`8c0021f05`, `-79` net LoC, zero behavior change, `slice35_virtual_mutation_manifest.rs` untouched and `2/2` green).
   - Unconfined `bash src/inspector/tools/local-ci.sh --full-workspace` on clean `HEAD` **`8c0021f05`**: **EXIT `0`** (`source/tooling/unit/native-build/seal/candidate/abi/installed`; `246/249` suites, `25/25` tooling, `11/11` unit, `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` `sha256: cd3bc93e24eec1839a7a3511899817dd135bc99e48d961620ab5254c76fb0f0d`, `build: cafb798b191f2ae661affc6c1254fe78948f6d367e8fccb6b37443200c2cf1cc`, `compiled_schema: 39`, `ELF GLIBC <= 2.39`, **`89/89` Playwright browser specs passed, `0` fail, `0` skipped**, including active-`scope` UI).
   - Unconfined 4-arg `qualify-installed-network.sh`: **EXIT `0`** (`traced_files: 327`, `supervisor_traced_pids: 51`, `worker_traced_pids: 13`, `worker_internet_family_attempts: 0`, `supervisor_outbound_attempts: 0`, `source_asset_readonly_opens: 3`, `source_cache_mutations: 0`, `native_window_closed: true`, `owned_descendants_no_longer_live: true`).
   - Dual-feature `cargo test -p fathomdb-inspector --locked --offline` (`default` and `--no-default-features --features cached-cpu`): **EXIT `0`** (`81` unit + `1` cli + `3` launch + `12` worker passed, `0` failed).
6. **Step 6 — `WP-60G` Closed Schema 39 Golden Archive (`AC32-60-08`):**
   - Generated `/mnt/data/worktrees/fathomdb-0.8.32-golden/golden-0.8.32-schema39.sqlite` on clean `HEAD` `8c0021f05` via [build-golden-0.8.32.py](./build-golden-0.8.32.py) and committed [golden-0.8.32-manifest.json](./golden-0.8.32-manifest.json): **`1,200,128` bytes**, `sha256: e232232e2c669a53400bdd78e655efad16c3d92b2e6305ba6d6431d41e3ffaa8`, `schema_version: 39`, scopes `["", "team-a", "team-b"]`, shared `domain_tier` per-class projection on both `node` and `edge`, 8-column `vec0` (`13` vector rows/spans across scopes), `38` `_fathomdb_scope_visibility_watermarks` rows, operational collections, and physical erasure; zero WAL, erased-canary byte absence, `doctor check-integrity --full --json`, and `doctor data-plane-integrity --json` (`0` findings) all verified strictly before `chmod 0444`.

## Provenance & Deduplication Note

- Product code froze at `8c0021f05` (zero-behavior-change wrapper fold over `d887e4465`). Full unconfined `./scripts/agent-verify.sh` (`246/249` suites), Inspector `local-ci.sh --full-workspace` (`89/89` Playwright specs), `qualify-installed-network.sh`, dual-feature `cargo test -p fathomdb-inspector`, and `WP-60G` golden archive all executed directly on `8c0021f05`.
- All post-`8c0021f05` Slice 60 edits are strictly non-code documentation, qualification receipts, Slice 70 plan, and release-state metadata, validated with scoped doc/truth/state/MkDocs validators per `AGENTS.md` § 3 rather than re-running duplicate heavy gates.

## Slice 70 Publication Handoff (Hard Stop)

- Draft deduplicated publication plan committed at [../slice-70/plan.md](../slice-70/plan.md) (`status: DRAFT`) inheriting Slice 60's frozen qualification receipts on `8c0021f05`.
- Publication remains **HELD** pending explicit owner HITL authorization covering:
  1. PyPI Pending Publisher for new package `fathomdb-mcp` (`fathomadb/fathomdb`, workflow `release.yml`, environment `pypi`),
  2. crates.io `CARGO_REGISTRY_TOKEN` (`publish-new` scope) for initial `fathomdb-mcp` crate creation,
  3. Creation/push of `refs/heads/release/0.8.32` + concurrent `release.yml` (`v0.8.32`) and `jetson-tegra-cuda-evidence.yml` (`publish_to_pages=true`) execution.

## Commits on `dev/0.8.32`

- `5625cd54e` — `chore(0.8.32): Slice 60 Step 1 approved plan, TDD qualification harnesses, and WP-60F pre-publication docs`
- `db3938132` — `perf(0.8.32): Slice 60 FIX-PERF-1 eliminate FTS scope reprepare and optimize single-scope search/write hot paths`
- `cbdfe7af8` — `perf(0.8.32): Slice 60 FIX-PERF-2 defer reader/runtime schema init and optimize batch writes`
- `9cb9d639b` — `perf(0.8.32): Slice 60 FIX-PERF-3 batch edge triples/terminals/revisions/attributes and cap reader cache RSS`
- `bb2356abf` — `perf(0.8.32): Slice 60 FIX-PERF-4 tune lazy reader cache_size to -1500 KiB for 50k/100k FTS headroom`
- `07b461eca` — `perf(0.8.32): Slice 60 FIX-PERF-5 balance lazy reader cache_size (-1750 KiB) and stmt cache (12)`
- `ca61fd180` — `perf(0.8.32): Slice 60 FIX-PERF-6 compact Schema 39 visibility triggers and restore default reader cache_size`
- `1b4a0770b` — `perf(0.8.32): Slice 60 FIX-PERF-7 restore reader stmt cache 24 and right-size lookaside 512x256`
- `03eb3f197` — `chore(slice-60): land WP-60B paired latency report and reconcile sdk_registry for 0.8.32`
- `579165215` — `fix(slice-60): backfill non-tail Schema-38 AdminSchema batch cursors as non_canonical in STEP_39_SQL`
- `d887e4465` — `perf(slice-60): FIX-PERF-8 zero-alloc row borrows and buffered StreamDigest in rederive_projections_on_boot`
- `02cdafdc1` — `chore(slice-60): land final d887e4465 WP-60B paired latency report (10k/50k/100k + Gate1-D10 all PASS)`
- `ce267cb15` — `chore(slice-60): register FIX-PERF-1 unit test modules in module-boundary-policy.txt`
- `8c0021f05` — `refactor(slice-60): fold terminal-sink helpers into project_canonical_{node,edge}_row to preserve slice35 manifest coupling`
- `7d1dd4f6b` — `docs(slice-60): land golden manifest, status, deduplicated Slice 70 plan, and 4-tier docs sync` (`17/17` scoped doc/truth/MkDocs validators EXIT `0`)
- `1be91d322` — `fix(slice-60): remediate Slice 70 draft plan review findings (pre-tag CUDA route, live manifest flags, published schema)` (independent `pro` adversarial review **`VERDICT: APPROVE`**, `0` P1 / `0` P2)
