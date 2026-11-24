---
title: Slice 70 Plan — 0.8.31 Full Qualification, Paired Latency Gate, Pre-Publish Docs & Golden Archive
status: CLOSED
target_release: 0.8.31
slice: 70
date: 2026-11-23
---

# Slice 70 Plan — `0.8.31` Qualification (`QUALIFICATION`)

- **Branch / Worktree:** `release/0.8.31` in `/mnt/data/worktrees/release-0.8.31`
- **Depends on:** Slices `10` (`PREWORK-PREP`), `20` (`PROPERTY-SCHEMA`), `30` (`STRICT-GRAPH`), `40` (`HISTORY-GRAPH`), `50` (`SDK-CLI-UPGRADE`), and `60` (`INSPECTOR`) landed cleanly on `release/0.8.31` (`release-state-0.8.31.json` updated to `slice 60 = LANDED`, `next_slice: 70`)
- **Axis-W Version Policy (Pre-Slice-80):** Root `Cargo.toml`, `src/python/pyproject.toml`, `src/python-cli/pyproject.toml`, and `src/ts/package.json` stay at live Axis-W **`0.8.30`** throughout Slice 70 (deferring `scripts/set-version.sh --workspace 0.8.31` to Slice 80 upon HITL publication authorization). All local wheel installs and `verify-release-gates.sh` invocations use live workspace version (`0.8.30`) dynamically read from `Cargo.toml`, paired with explicit `CHANGELOG.md` `## 0.8.31` verification.
- **Authorities:** [`dev/plans/0.8.31/plan.md`](../../plan.md), [`dev/plans/0.8.31/scope.md`](../../scope.md), [`prework/slice-3.md`](../../prework/slice-3.md) (`P3-01`, `P3-20`), [`prework/slice-5.md`](../../prework/slice-5.md) (`P5-11`, `P5-12`), [`prework/slice-6.md`](../../prework/slice-6.md) (`P6-08`), [`prework/slice-9-triage.md`](../../prework/slice-9-triage.md), [`RELEASE-AND-SLICE-PLANNING-TEMPLATE.md`](../../../prompts/RELEASE-AND-SLICE-PLANNING-TEMPLATE.md), [`dev/plans/0.8.34/plan.md`](../../../0.8.34/plan.md)
- **Publication Boundary:** **Stops strictly BEFORE Slice 80 (`PUBLICATION`).** Zero `v0.8.31` git tags, zero registry uploads, and zero `published` truth flips occur in this slice.

---

## 1. Slice Prep — Goals & Non-Goals

### Goals

Qualify the integrated `0.8.31` release candidate end-to-end across source, security, long-running reliability, paired latency/resource bounds, GPU/CUDA feature completeness, Inspector native Ubuntu 24.04 qualification, clean-venv CLI wheel & golden upgrade proof, pre-publication documentation convergence, and closed schema-38 golden archival for `0.8.34`:

1. **WP-70A — Full Unconfined Source, Long-Window & Supply-Chain Qualification (`P5-12`, All `R31-01`..`R31-08`):**
   - Run full `./scripts/agent-verify.sh` inside an **unconfined ptrace-capable executor** (`STRICT=1 AC037_LIVE_OPTIONAL=1` locally so `AC-036` executes natively without sandbox `PTRACE_TRACEME` denial; record `AC-037` live userns status explicitly and verify live netns on a userns-capable Linux runner if required by CI policy).
   - Run `AGENT_LONG=1 ./scripts/check.sh` and verify canonical long-running selectors (`ac_021_zero_sqlite_schema_warnings_under_concurrent_reads_and_ddl`, `projection_cursor_bounds_observed_row_count`, `ac_011a_write_throughput_1kb` and `ac_011b_write_throughput_100kb` explicitly with **`AGENT_LONG=1 FATHOMDB_TIER1_WRITE_PERF=1`** asserting `AC011_NUMBERS` in output, `ac_012_text_query_latency_on_fts5_path`, `ac_013_vector_retrieval_latency`, `ac_081_absolute_read_performance`).
   - Run supply-chain, security, and surface-pin gates: `scripts/security/gitleaks-current.sh`, `scripts/security/gitleaks-history.sh` (deferred from Slices 20–60 per user directive), `scripts/check-license-consistency.sh --with-wheel`, `scripts/check-governed-surface-pin.sh` (live post-Slice-50 pin: `56` live ops / `93` allowlist members pinned at `9780e9181`), `scripts/check-sdk-surface-parity.py` AND `scripts/check-sdk-surface-parity.py --binding rust --rust-crate src/rust/crates/fathomdb-sdk`, `scripts/check-pinned-override-rot.sh`, `scripts/check-platform-capabilities.sh`, and `scripts/set-version.sh --check-files`.

2. **WP-70B — Same-Host Interleaved Paired `v0.8.30` (`db271323f`) vs. `0.8.31` Latency, Resource & Query-Plan Gate (`R31-07`, `P5-11`):**
   - Execute a **5k baseline parity check** against `dev/plans/0.8.31/features/slice-0/baseline-0.8.30-latency.json` using the `dev/plans/0.8.31/features/slice-0/capture-baseline-and-golden.py` workload (`5,000` nodes, `7,500` edges, `100` queries) to verify host repeatability since Slice 0.
   - Execute a **same-host interleaved (`AB`/`BA`) paired benchmark** comparing isolated noneditable builds of `v0.8.30` (`db271323fdd38672fafd51898ca81c5763329af2`, schema `37`) vs. the `0.8.31` candidate (schema `38`) at **10k, 50k, and 100k nodes (`1.5x` edges)** with **`>= 5` warmup rounds** and **`n >= 30` measured samples per workload**, asserting a **10,000-resample paired bootstrap 95% CI upper bound `<= max(5.0%, 1.0 ms)`** on both `p50` and `p95` for:
     - Unpredicated `open` (`process_open` and `warm_reopen`),
     - Batch write (`write_node_batch_200` and `write_edge_batch_200`),
     - Streamed FTS (`search_text_only`),
     - Optimized hybrid (`search` via `hybrid_deferred_identity`, CPU without embedder), and
     - Current-time 2-hop `graph.expand` (`graph_expand_2hop`),
     plus `database_bytes <= +15.0%` and `peak_rss_bytes <= +10.0%`.
   - Execute `experiments/sdk_benchmark.py` (`sdk.experiment.v1`) on pinned cross-SDK `10k` synthetic parity/latency verification to retain formal `experiments.record.v1` receipts without mutating `sdk.experiment.v1` invariants.
   - Execute dedicated `0.8.31` **50k micro-gates**:
     - High-fanout declared property predicate (`canonical_property_values` B-tree) `p50` and `p95 <= 0.25x` unindexed inline baseline at `50k` nodes;
     - Historical `as_of` composite-seek (`canonical_edges_{from,to}_sup_kind_idx`) `< 5.0 ms` `p95` across `as_of_write_cursor` and `as_of_epoch_ms` (both empty and non-empty `edge_kinds`);
     - `cargo test -p fathomdb-engine --test slice38_query_plans` confirming `EXPLAIN QUERY PLAN` `SEARCH`-only plans and zero extra joins on unpredicated fast paths.

3. **WP-70C — GPU & Feature-Complete Qualification (`P5-12` + GPU Backoff Protocol):**
   - Run `python3 scripts/check-test-target-coverage.py` and `FATHOMDB_FEATURE_COMPLETE=1 ./scripts/test-feature-complete.sh` on the two RTX 3090 GPUs (`CUDA_DEVICE_ORDER=PCI_BUS_ID`, `CUDA_VISIBLE_DEVICES=0,1`; never touching GPU 2 Quadro K620), covering `embed-cuda`, `rerank-cuda`, `onnx-embedder`, `fathomdb-embedder`, and `slice72_concurrent_gpu`.
   - Apply the mandatory **GPU Availability & Backoff Protocol** (using native `schedule` one-shot timers if GPUs are busy or lack free VRAM — never exiting/failing immediately and never calling shell `sleep`).

4. **WP-70D — Inspector `--full-workspace` + `D4 = B` Browser Suite (`P5-12`, `R31-08`):**
   - Run `bash src/inspector/tools/local-ci.sh --full-workspace <candidate_dir> <fixtures_dir> <build_target>` from a clean checkout, exercising full source verification, dual-feature `fathomdb-inspector` (`default` model-free and `cached-cpu`), `local-release.py seal` / `verify`, ELF `GLIBC <= 2.39`, `D4 = B` native Ubuntu 24.04 (`Xvfb -displayfd`, `/proc` PID discovery, CDP `Browser.close`, parent-wrapped `strace -ff`) **81/81+ installed browser suite** (including Slice 60 schema-38 inline property & `as_of` history specs), and `src/inspector/tools/qualify-installed-network.sh`.

5. **WP-70E — Clean-Venv `fathomdb-cli` Wheel & Golden Schema-37 `fathomdb migrate` Proof (`P5-12`, `R31-05`):**
   - Build lockstep `fathomdb` (from `src/python`) and `fathomdb-cli` (from `src/python-cli`, `bindings = "bin"`, `--bin fathomdb` only) wheels via `maturin build --release` at live Axis-W version (`0.8.30`), and run `scripts/check_python_wheel_contents.py --kind sdk` and `--kind cli`.
   - Install `"fathomdb[cli]==0.8.30"` (resolved dynamically from `Cargo.toml` Axis-W) into a clean venv with `cargo`/`rustc` stripped from `PATH`; verify `fathomdb --version`, `fathomdb[cli]` packaging, `scripts/check-glibc-floor.sh --floor 2.39` on host-built `_fathomdb.abi3.so` and `bin/fathomdb` (distinct from CI `manylinux_2_28` `2.28`), `AC-041` SDK-vs-CLI separation (`fathomdb` `[project.scripts]` remains empty; `perf-regression-check` is absent from venv `bin/`), `src/python/tests/smoke_slice55_installed.py` against the local installed wheel, and `fathomdb migrate` (`37 -> 38`) on a disposable copy of `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.30-schema37.sqlite` followed by `fathomdb doctor check-integrity --full`, SHA-256 backup check, erased-row non-resurrection, and post-upgrade `0.8.31` property/`as_of`/erasure operations.

6. **WP-70F — Pre-Publication Release Notes, Regenerated Docs, `dev/{needs,requirements}.md` Sync & Final Currency (`P3-01`, `P3-20`, `P3-19` sweep, `P6-08`):**
   - Create `docs/release-notes/0.8.31.md` marked strictly **pre-publication** (publication pending Slice 80 HITL authorization) and wire it into `mkdocs.yml` as `- 0.8.31 (pre-publication): release-notes/0.8.31.md`.
   - Prepare `CHANGELOG.md` `## 0.8.31` pre-publication heading (explicitly stating publication is pending and listing any removed public symbols under `### Removed` for `AC-050c`), verified with both default `scripts/security/check-removal-changelog.sh` and `--base v0.8.30 --head HEAD`, explicit `grep -qE '^##[[:space:]]+v?0\.8\.31([[:space:]].*)?$' CHANGELOG.md`, and `RELEASE_GATES_SKIP_GIT_REACH=1 RELEASE_GATES_TAG=v0.8.30 bash scripts/verify-release-gates.sh` (at live Axis-W `0.8.30`).
   - Update `SOURCE_URL` in `scripts/generate-public-docs.py` from `release/0.8.30` to `release/0.8.31`, update the matching branch-URL expectation in `scripts/tests/test_generated_public_docs.py`, regenerate `docs/generated/sdk-operation-map.md` and `docs/generated/contributor-map.md` (`--write`, verified with `--check` and `unittest scripts/tests/test_generated_public_docs.py`).
   - Apply `P3-01`: update `Current applicability` pointers in `dev/needs.md` and `dev/requirements.md` from `format37` and `plans/plan-0.8.29.md` to `schema 38` and `scripts/release-current.py` / `dev/plans/0.8.31/plan.md`, clarifying that `REQ-036`/`REQ-045` permit the narrow third CLI root `fathomdb migrate` (`37 -> 38`) while `Engine::open` stays schema-38-only and `REQ-037`/`AC-041` stay intact via separate `fathomdb-cli` wheels (`fathomdb` `[project.scripts]` empty). Leave `dev/acceptance.md` **LOCKED and untouched**.
   - Apply final `P6-08` / `P3-19` currency refresh across `dev/interfaces/*.md` headers (`target_release: 0.8.31`, keeping `dev/interfaces/mcp.md` scoped to future evaluation/`0.8.32`) and public `docs/` (keeping `"published": null` and zero publication claims so `scripts/check-public-doc-truth.py` stays green), verified with `scripts/agent-lint-docs.sh`, `scripts/agent-lint-md.sh`, `scripts/check-release-state-views.sh`, and `mkdocs build --strict`.

7. **WP-70G — Closed Golden Schema-38 `0.8.31` DB & Manifest Archive for `0.8.34` (`P5-12`):**
   - Generate `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.31-schema38.sqlite` (read-only `0444`, clean close, zero nonempty WAL, `fathomdb doctor check-integrity --full` clean) via `dev/plans/0.8.31/features/slice-70/build-golden-0.8.31.py` and commit its manifest `dev/plans/0.8.31/features/slice-70/golden-0.8.31-manifest.json` for `0.8.34` migration qualification (`R34-02`, `R34-05`).

### Non-Goals (Explicitly Excluded — Stops BEFORE Slice 80)

- **Zero tagging or registry publishing:** Do NOT create or push any `v0.8.31` git tag (pushing a `v*` tag triggers the real multi-registry publish). Do NOT upload to crates.io, PyPI (`fathomdb` or `fathomdb-cli`), npm, GitHub Pages (Tegra), or GitHub Releases.
- **Zero `P3-21` publication truth flip:** Do NOT add `"0.8.31": 38` to `RELEASE_SCHEMAS` in `scripts/check-public-doc-truth.py` and do NOT set `"published"` non-null in `dev/plans/release-state-0.8.31.json` (that stays `"published": null` with `publish_gate.status: "HELD"` until explicit Slice 80 HITL authorization).
- **Zero `dev/acceptance.md` edits:** Canonical `AC-NNN` assertions and thresholds remain locked.
- **Zero new engine/schema feature engineering:** Any product or test defect discovered during Slice 70 qualification is routed back to a bounded RED→GREEN fix commit and re-verified across the blast radius before re-freezing the candidate.
- **Zero external consumer-project references:** All docs, scripts, manifests, release notes, and golden fixtures remain strictly product-generic FathomDB terminology.

---

## 2. Slice Prep — Draft Needs, Requirements & Acceptance Criteria

### Requirement Bindings

- **Primary owner (`R31-07`):** Quantified zero latency regression vs. `0.8.30` across `10k`, `50k`, and `100k` nodes (`1.5x` edges) with paired bootstrap CI bounds, `database_bytes`/`peak_rss_bytes` bounds, 50k predicate/`as_of` micro-gates, and `EXPLAIN QUERY PLAN` `SEARCH`-only pins.
- **Final integrated candidate proof (`R31-01`..`R31-08`):** Unifies all Slice `20`–`60` behaviors under a single clean commit SHA prior to Slice 80 publication authorization.
- **Documentation applicability (`P3-01`):** Updates `dev/needs.md` and `dev/requirements.md` `Current applicability` pointers to `0.8.31` / schema `38` / narrow `fathomdb migrate` while leaving `dev/acceptance.md` **LOCKED**.

### Slice-Local Acceptance Criteria (`AC31-70-*`)

| ID | Requirement Ref | Falsifiable Assertion & Oracle |
| --- | --- | --- |
| `AC31-70-01` | All (`R31-01`..`R31-08`), `P5-12` | Unconfined `./scripts/agent-verify.sh` (`STRICT=1`, `AC-036` ptrace executed, zero excluded suites) and `AGENT_LONG=1 ./scripts/check.sh` (plus canonical `ac_021`, `projection_cursor_bounds_observed_row_count`, `ac_011a`/`ac_011b` with `AGENT_LONG=1 FATHOMDB_TIER1_WRITE_PERF=1` printing `AC011_NUMBERS`, `ac_012` at `N=10,000`, `ac_013` at `N=10,000, dim=768`, `ac_081`), plus `gitleaks-history.sh`, `check-governed-surface-pin.sh` (`56` ops / `93` allowlist), and `check-sdk-surface-parity.py --binding rust` exit `0` on the frozen candidate commit with retained log receipts. |
| `AC31-70-02` | `R31-07`, `P5-11` | `5k` parity check matches `baseline-0.8.30-latency.json` host profile; same-host interleaved paired `v0.8.30` (`db271323f`) vs. `0.8.31` benchmark at **10k, 50k, and 100k** nodes (`1.5x` edges, `>= 5` warmups, `n >= 30` paired samples per workload) satisfies 10,000-resample paired bootstrap 95% CI upper bound `<= max(5.0%, 1.0 ms)` on `p50` and `p95` across `process_open`, `warm_reopen`, `write_node_batch_200`, `write_edge_batch_200`, `search_text_only`, `search` (`hybrid_deferred_identity`), and `graph_expand_2hop`, with `database_bytes <= +15.0%` and `peak_rss_bytes <= +10.0%`, plus pinned `experiments/sdk_benchmark.py` receipt (written outside tracked `experiments/`). |
| `AC31-70-03` | `R31-07`, `R31-02`, `R31-03` | Dedicated `0.8.31` 50k micro-gates pass (`n >= 30` after `>= 5` warmups): `SearchFilter` declared `canonical_property_values` B-tree predicate `p50` and `p95` are `<= 0.25x` unindexed inline evaluation; `as_of` composite seek (`canonical_edges_{from,to}_sup_kind_idx`) achieves `p95 < 5.0 ms` for both empty and non-empty `edge_kinds`; `cargo test -p fathomdb-engine --test slice38_query_plans` passes with zero unexpected `SCAN` and zero unpredicated fast-path joins. |
| `AC31-70-04` | `R31-01`, `R31-05`, `R31-07` | `python3 scripts/check-test-target-coverage.py` exits `0` and `FATHOMDB_FEATURE_COMPLETE=1 ./scripts/test-feature-complete.sh --scratch /tmp/fathomdb-0831-qual/feature-complete` exits `0` on RTX 3090 GPUs `0,1` (`CUDA_DEVICE_ORDER=PCI_BUS_ID`, `FATHOMDB_REQUIRE_LIVE=1`) with `0` failures and `0` unallowlisted skips, polling every 7 minutes via native `schedule` (up to 24 attempts / 6h; never GPU `2`) if GPUs are busy. |
| `AC31-70-05` | `R31-08`, `P5-12` | `bash src/inspector/tools/local-ci.sh --full-workspace` + 4-arg `qualify-installed-network.sh` + dual-feature `cargo test -p fathomdb-inspector` (`default` and `cached-cpu`) exit `0` under `D4 = B` native Ubuntu 24.04 (`ELF GLIBC <= 2.39`, all 8 installed browser specs incl. `attributes.mjs`, `0` Docker/`openbox`/`xdotool`/`sudo`, bounded per-command timeouts, zero leftover `.lock`/worker processes). |
| `AC31-70-06` | `R31-05`, `P5-12` | Clean-venv install (`cargo`/`rustc` absent from `PATH`) of built `fathomdb` and `fathomdb-cli` wheels (`fathomdb[cli]==0.8.30` at live Axis-W, checked with `check_python_wheel_contents.py --kind sdk` & `--kind cli`) passes `fathomdb --version`, `fathomdb doctor check-integrity`, `scripts/check-glibc-floor.sh --floor 2.39`, `AC-041` boundary checks (`fathomdb` `[project.scripts]` empty; `perf-regression-check` absent), `smoke_slice55_installed.py`, and `fathomdb migrate` (`37 -> 38`) on a disposable copy of `golden-0.8.30-schema37.sqlite` with `doctor check-integrity --full` clean and working post-upgrade `0.8.31` writes/`as_of`/erasure. |
| `AC31-70-07` | `P3-01`, `P3-20`, `P6-08` | `docs/release-notes/0.8.31.md` (marked pre-publication), `mkdocs.yml`, `CHANGELOG.md` `## 0.8.31` (`AC-050c`), `scripts/generate-public-docs.py` (`SOURCE_URL` = `release/0.8.31`) + `scripts/tests/test_generated_public_docs.py` + regenerated `docs/generated/*`, `dev/{needs,requirements}.md` applicability sync (`dev/acceptance.md` byte-identical to Slice 60 close), and `dev/interfaces/*.md` currency refresh pass `check-public-doc-truth.py` (`published: null`), `generate-public-docs.py --check`, `check-license-consistency.sh --with-wheel`, `verify-release-gates.sh` (`RELEASE_GATES_SKIP_GIT_REACH=1 RELEASE_GATES_TAG=v0.8.30` + `CHANGELOG.md` `0.8.31` header), `check-removal-changelog.sh` (default + `--base v0.8.30`), `check-release-state-views.sh`, `agent-lint-docs.sh`, `agent-lint-md.sh`, and `mkdocs build --strict` with zero external consumer-project references. |
| `AC31-70-08` | `R31-01`..`R31-06`, `R34-02` | Closed schema-38 archive `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.31-schema38.sqlite` (`0444`, zero nonempty WAL) + `dev/plans/0.8.31/features/slice-70/golden-0.8.31-manifest.json` pass `fathomdb doctor check-integrity --full --json` and prove exact SHA-256, schema `38`, inline typed `properties`, `canonical_property_values`, `_fathomdb_commit_timestamps`, historical superseded edges, enrolled custom vector kinds, op collections, and physically erased rows. |
| `AC31-70-09` | `Slice 80` Handoff | Clean `git status`, `dev/plans/release-state-0.8.31.json` keeps `"published": null` and `publish_gate.publication_authorized == false`, zero `v0.8.31` tags exist locally or on `origin`, `dev/plans/0.8.31/features/slice-70/status.md` records all receipts, and Slice 80 HITL publication authorization packet is ready. |

---

## 3. Slice Prep — Architecture & Draft Design

### 3.1 Paired Benchmark Harness Architecture (`R31-07`, `P5-11`)

#### Architectural Fit with `experiments/sdk_benchmark.py` and `capture-baseline-and-golden.py`

Inspection of `experiments/sdk_benchmark.py` and `experiments/sdk_registry.py` shows:

1. `sdk_benchmark.py` (`sdk.experiment.v1`) enforces distinct SDK names per `suite.json` (`python`, `typescript`, `rust`) — it rejects two same-SDK arms (`0.8.30` Python vs. `0.8.31` Python) inside one suite.
2. `_MEASURE` in `experiments/sdk_registry.py` measures `engine.search_text_only`, `engine.search_projected_text`, `engine.search`, `read.get`, and `graph.neighbors`; it does **not** expose `engine.open`, 200-item `engine.write` batches, `graph.expand`, declared `SearchFilter` property predicates, or historical `as_of` graph traversal, and `_aggregate` computes raw `_nearest` quantiles with no built-in paired bootstrap CI.
3. Conversely, `dev/plans/0.8.31/features/slice-0/capture-baseline-and-golden.py` already defines the canonical `0.8.30` workload generators (`cold_open_ms`, `warm_open`, `write_node_batch_200`, `write_edge_batch_200`, `search_text_only`, `search_hybrid_cpu_no_embedder`, `graph_expand_2hop`, `db_bytes`, `peak_rss_bytes`), while `experiments/comp03_retrieval.py` (`bootstrap_ci`) defines the repository's 10,000-resample paired percentile CI convention.

To satisfy `R31-07` and `P5-11` cleanly without distorting `sdk.experiment.v1` or `experiments/measurement_classification.py`:

- **Primary paired cross-version gate (`dev/plans/0.8.31/features/slice-70/run-paired-latency-gate.py`):** Self-contained Python orchestrator (mirroring Slice 0's `capture-baseline-and-golden.py` placement) that drives two isolated noneditable Python wheel environments on the same host in counterbalanced `AB` / `BA` interleaved blocks and computes 10,000-resample paired bootstrap 95% CIs.
- **Secondary pinned `experiments/sdk_benchmark.py` check:** Uses unmodified `experiments/sdk_benchmark.py` on a `10k` synthetic `sdk.fixture.v1` suite (`engine.search_text_only` + `engine.search`) generated into `/tmp/fathomdb-0831-qual/sdk-suite-10k.json` with pinned `0.8.31` Python/TS/Rust arm manifests, invoked with cleared `FATHOMDB_*` env vars (so `_check_treatment_environment` passes) and `receipts_root=Path("/tmp/fathomdb-0831-qual/sdk-receipts")` (never dirtying tracked `experiments/` before `local-ci.sh`).

#### Artifact Isolation & Environment Pinning

- **Arm A (`v0.8.30`, schema `37`):** Built from a clean detached worktree at `db271323fdd38672fafd51898ca81c5763329af2` (the exact Slice 0 commit on `release/0.8.31` with Ubuntu 24.04 Python 3.12 bootstrap and pre-schema-38 Engine) from `cwd=<wt-0830>/src/python` with isolated `CARGO_TARGET_DIR=/tmp/fathomdb-0831-qual/target-0830` using `maturin build --release` into `/tmp/fathomdb-0831-qual/venv-0830`, removing the detached worktree via `git worktree remove --force` inside a `finally` block so `git worktree list` stays spotless.
- **Arm B (`0.8.31`, schema `38`):** Built from the frozen Slice 70 candidate commit (`release/0.8.31` `HEAD`) from `cwd=src/python` using `maturin build --release` into an isolated scratch venv `/tmp/fathomdb-0831-qual/venv-0831`.
- **Subprocess Hygiene:** Each timed arm runs in a fresh isolated worker invoked via `env -i PATH=... HOME=... FATHOMDB_EMBED_DEVICE=cpu FATHOMDB_RERANK_DEVICE=cpu <venv>/bin/python -I ...` (zero `PYTHONPATH` leakage). Startup verifies and records `git_sha`, `open_report().schema_version_after` (`37` vs. `38`), Python version, CPU model/core count, and `_fathomdb.abi3.so` SHA-256.

#### Scale Tiers, Corpus Topology & Paired Sampling (`>= 5` Warmups, `n >= 30`)

| Scale Tier | Nodes | Edges (`1.5x`: `relates_to` + `verifies`) | Queries / Open Trials | Pairing & Block Schedule |
| --- | --- | --- | --- | --- |
| `5k` parity | `5,000` | `7,500` | `100` queries, `5` warmups | Cross-check `db271323f` arm against committed `baseline-0.8.30-latency.json` plus `0.8.31` delta summary |
| `10k` gate | `10,000` | `15,000` | `n = 100` paired queries (`10` warmup), `n = 30` paired opens (`5` warmup), `45` node / `70` edge batches (`5` warmup discarded) | `2` counterbalanced store builds (`AB` then `BA`) + `5` interleaved `AB`/`BA` query blocks (`20` queries/block) + `30` paired interleaved `process_open`/`warm_reopen` cycles |
| `50k` gate | `50,000` | `75,000` | `n = 100` paired queries (`10` warmup), `n = 30` paired opens (`5` warmup), `245` node / `370` edge batches (`5` warmup discarded) | `2` counterbalanced store builds (`AB` then `BA`) + `5` interleaved `AB`/`BA` query blocks + `30` paired interleaved `process_open`/`warm_reopen` cycles |
| `100k` gate | `100,000` | `150,000` | `n = 100` paired queries (`10` warmup), `n = 30` paired opens (`5` warmup), `495` node / `745` edge batches (`5` warmup discarded) | `2` counterbalanced store builds (`AB` then `BA`) + `5` interleaved `AB`/`BA` query blocks + `30` paired interleaved `process_open`/`warm_reopen` cycles |

Every measured workload strictly enforces `n >= 30` paired samples after discarding `>= 5` warmup iterations/batches/queries:

- **`process_open` (`n = 30`):** Fresh subprocess per sample opening the closed populated store, timing `Engine.open(...)` to return, capturing `resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024`, and cleanly calling `eng.close()`. Interleaved `AB` on odd `i`, `BA` on even `i`.
- **`warm_reopen` (`n = 30`):** Within a single open-reuse worker per block (`5` warmup opens discarded, then `30` measured `Engine.open` + `eng.close` cycles), interleaved across paired blocks.
- **`write_node_batch_200` / `write_edge_batch_200` (`n >= 45`..`745` per build, paired across `2` counterbalanced builds):** Batches of `200` items matching `capture-baseline-and-golden.py`'s exact payload shape (nodes: `"kind": "note"`, JSON `body` containing `"content"`, `"tier"`, `"topic"` so both `0.8.30` and `0.8.31` exercise identical unpredicated payload bytes; edges: `1.0x` `relates_to` + `0.5x` `verifies`). First `5` batches per build are discarded as warmup; remaining batches are paired by `(build_pair_idx, batch_idx)`.
- **`search_text_only`, `search_hybrid_cpu_no_embedder` (`hybrid_deferred_identity`), `graph_expand_2hop` (`n = 100` each):** `10` warmup queries per path discarded, then `100` measured paired queries executed across `5` balanced `AB`/`BA` turn blocks (`20` queries per turn), asserting identical returned logical IDs between `0.8.30` and `0.8.31` for deterministic queries.
- **Storage & Memory Ceilings:** Measure `database_bytes` as `sum(size(db, db-wal, db-shm))` after clean `eng.close()` (asserting `(bytes_0831 - bytes_0830) / bytes_0830 <= 0.15`) and `peak_rss_bytes` as `max(ru_maxrss * 1024)` across build and query workers (asserting `(rss_0831 - rss_0830) / rss_0830 <= 0.10`).

#### 10,000-Resample Paired Bootstrap Pass/Fail Rule

For each `(scale_tier in {10k, 50k, 100k}, workload, metric in {p50_ms, p95_ms})` with `n >= 30` paired observations `{(x_i, y_i)}_{i=1}^n` (`x_i = 0.8.30`, `y_i = 0.8.31`):

1. Draw `B = 10,000` deterministic bootstrap resamples (`seed = 20261122`) of `n` index pairs with replacement.
2. In each resample `b`, compute `delta_b = percentile(y*, pct) - percentile(x*, pct)`.
3. Sort `{delta_b}` and compute the 95% percentile CI `[sorted[249], sorted[9749]]` (matching `bootstrap_ci` in `experiments/comp03_retrieval.py`).
4. **Pass condition:** `ci_95_upper <= max(0.05 * baseline_stat_ms, 1.0)` ms! Any violation fails the gate hard (`exit 1`).

#### Dedicated `0.8.31` 50k Micro-Gates & `EXPLAIN QUERY PLAN` Gate

Executed on the `0.8.31` candidate (`50,000` nodes, `75,000` edges, `>= 5` warmups, `n = 50 >= 30` measured samples per gate):

1. **High-fanout predicate gate (`<= 0.25x` unindexed at `50k`):**
   - Seed `50,000` nodes with outside-`body` typed `properties` (`{"bucket": i % 500, "score": float(i % 100), "flag": (i % 10 == 0)}`), where `"bucket"` is declared `Filterable` (`canonical_property_values`) and an identically distributed sibling key `"bucket_unindexed"` is undeclared (plus a `5,000`-edge fanout hub where `20` edges match an edge property predicate).
   - Compare `SearchFilter` over `canonical_property_values` (indexed B-tree) against an unindexed corpus scan evaluator on identical matching subsets (`bucket = 42`, selectivity `100 / 50,000 = 0.2%`): assert `p50_indexed <= 0.25 * p50_unindexed` AND `p95_indexed <= 0.25 * p95_unindexed` (expecting `<< 0.1x`), and confirm the `5,000`-edge hub `graph.expand(edge_predicates=...)` stays bounded and sub-5 ms.
2. **`as_of` composite-seek gate (`< 5.0 ms` `p95` at `50k`):**
   - Across the `50,000`-node store, build `>= 1,000` revision batches (`_fathomdb_commit_timestamps`) with historical superseded edges on queried hub nodes (`canonical_edges_{from,to}_sup_kind_idx`).
   - Run `10` warmups + `50` measured `graph.expand` calls at historical `as_of_write_cursor` and `as_of_epoch_ms` boundaries (covering both empty `edge_kinds=()` and non-empty `edge_kinds=("verifies",)`), asserting **`p95 < 5.0 ms`**.
3. **Query-plan gate:** Run `cargo test -p fathomdb-engine --test slice38_query_plans -- --nocapture` verifying `SEARCH` (zero full `SCAN`) across endpoint validation, `SearchFilter`, current `graph.expand`, `as_of` `graph.expand`, and zero extra joins on unpredicated fast paths.

---

### 3.2 GPU Qualification & Backoff Protocol (`P5-12`)

1. **Hardware Policy:** Use RTX 3090 GPUs `0` and `1` only (`CUDA_DEVICE_ORDER=PCI_BUS_ID`, `CUDA_VISIBLE_DEVICES=0,1`, `CUDA_ROOT=/usr/local/cuda`). **Never** target GPU 2 (Quadro K620, host display). CUDA 13 remains deferred out of `0.8.31` (retaining CUDA 12.6 policy).
2. **Pre-Flight Check:** Query `nvidia-smi --query-gpu=index,pci.bus_id,name,memory.used,memory.free,memory.total --format=csv,noheader` and `nvidia-smi --query-compute-apps=gpu_uuid,pid,used_memory --format=csv,noheader`. Require RTX 3090 identity on indices `0` and `1` and sufficient free VRAM (`>= 4,096 MiB` free on GPU `0` for `slice72_concurrent_gpu` / feature-complete CUDA suites).
3. **GPU Availability & Polling Protocol (Antigravity `schedule` Polling):**
   If GPUs `0`/`1` are occupied or lack sufficient VRAM, **do not abort or fail** and **do not use shell `sleep`**. Enter a polling loop using native `schedule` one-shot timers (`DurationSeconds: 420` — every **7 minutes** per user directive):
   - **Interval:** Poll every **7 minutes** (`420` seconds) via native `schedule` one-shot timers.
   - **Cap:** Abort and escalate to the user only if GPUs remain unavailable after **24 attempts (6 hours)**.
   - **Immediate Claim:** Immediately upon waking up and confirming free VRAM via `nvidia-smi`, launch `FATHOMDB_FEATURE_COMPLETE=1 ./scripts/test-feature-complete.sh --scratch /tmp/fathomdb-0831-qual/feature-complete` right away so another agent does not claim the memory window.
4. **Telemetry & Receipts:** Retain `/tmp/fathomdb-0831-qual/feature-complete/summary.json`, per-crate logs, and `slice72-receipts/` proving `0` failures, `0` unallowlisted skip markers, and verified GPU UUID / PID telemetry.

---

### 3.3 Inspector `--full-workspace` & `D4 = B` Qualification (`R31-08`, `P5-12`)

- Consume Slice 60's decoupled `src/inspector/tools/local-ci.sh` via its explicit full-workspace gate:
  - Ensure `git status --porcelain` is strictly empty (zero stray untracked harness files such as `claude-*-cwd` — `local-ci.sh` and `local-release.py seal` refuse dirty trees).
  - Pre-build `target/debug/fathomdb-inspector` (`cargo build --locked --offline -p fathomdb-inspector`), create `/tmp/fathomdb-0831-qual/inspector-target`, ensure `/tmp/fathomdb-0831-qual/inspector-candidate` does **not** exist, and generate all 6 fresh schema-38 fixtures (`person`, `vector`, `f06` with `fixture scale` append, `study-training`, `study-unseen`, `lexical-deadline`) inside `/tmp/fathomdb-0831-qual/inspector-fixtures` using the built noneditable Python wheel.
  - Execute `bash src/inspector/tools/local-ci.sh --full-workspace /tmp/fathomdb-0831-qual/inspector-candidate /tmp/fathomdb-0831-qual/inspector-fixtures /tmp/fathomdb-0831-qual/inspector-target` on clean `HEAD`, exercising:
    1. Source gate (`scripts/agent-verify.sh`),
    2. Tooling & JS unit suites (`src/inspector/tests/tools/test_*.py`, `src/inspector/tests/unit/{comparison,ux}.mjs`),
    3. Dual-feature Rust verification (`cargo test -p fathomdb-inspector --locked --offline` bare and `--no-default-features --features cached-cpu`),
    4. Native Ubuntu 24.04 package build (`src/inspector/tools/package.sh`), `local-release.py seal`, `local-release.py verify --source` (`compiled_schema == 38`), and ELF `readelf` `GLIBC <= 2.39` floor check,
    5. `D4 = B` native browser execution (`src/inspector/tools/run-installed.sh` using native `Xvfb -displayfd`, `/proc` PID discovery, CDP `Browser.close`, parent-wrapped `strace -ff`, and bounded per-command timeouts; **zero Docker, `openbox`, `xdotool`, or `sudo`**) passing all **8 installed browser specs** (`installed.mjs`, `attributes.mjs`, `comparison.mjs`, `lexical.mjs`, `vector.mjs`, `graph.mjs`, `workspace.mjs`, `ux.mjs`),
    6. 4-argument `bash src/inspector/tools/qualify-installed-network.sh <extracted-bundle> <fixtures> <target> <new-trace-dir>` verifying zero outbound sockets across `--session-worker` and read-only `O_RDONLY` cache access.

---

### 3.4 Clean-Venv `fathomdb-cli` Wheel & Golden Schema-37 Upgrade Proof (`R31-05`, `P5-12`)

1. **Build Lockstep Candidate Wheels:** Build `fathomdb` (from `cwd=src/python`) and `fathomdb-cli` (from `cwd=src/python-cli`, `bindings = "bin"` scoped strictly to `targets = [{ name = "fathomdb", kind = "bin" }]`) with `maturin build --release` into `/tmp/fathomdb-0831-qual/wheels`, and verify both wheels with `scripts/check_python_wheel_contents.py --kind sdk` and `--kind cli`.
2. **Clean-Venv Verification (No Rust Toolchain on `PATH`):**
   - Create a fresh venv `/tmp/fathomdb-0831-qual/cli-smoke-venv` and run `pip install --no-index --find-links /tmp/fathomdb-0831-qual/wheels "fathomdb[cli]==${WS_VER}"` (where `WS_VER` is live Axis-W `0.8.30` from `Cargo.toml`).
   - Strip `~/.cargo/bin` (and any `cargo`/`rustc` path) from `PATH` when invoking the smoke harness:
     - Verify `command -v cargo || true` is empty and `which fathomdb` resolves inside `cli-smoke-venv/bin/fathomdb`.
     - Verify `perf-regression-check` does **not** exist in `cli-smoke-venv/bin/`.
     - Verify `fathomdb` package metadata declares zero `[project.scripts]` (`AC-041` / `REQ-037`).
     - Verify `bash scripts/check-glibc-floor.sh --floor 2.39` against the host-built `_fathomdb.abi3.so` and `cli-smoke-venv/bin/fathomdb` (noting host Ubuntu 24.04 native `2.39` floor vs. CI `manylinux_2_28` `2.28`).
     - Execute `src/python/tests/smoke_slice55_installed.py` against `cli-smoke-venv/bin/python` (testing separate-process open/write/reopen/close/exit and typed refusals).
3. **Golden Schema-37 `fathomdb migrate` Proof:**
   - Copy `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.30-schema37.sqlite` (and verify source SHA-256 `59d56eabb9a6e817168c5c540b4bb2ec6eb7f741fb24a72c7617df4aa5004548` untouched) into a disposable scratch dir `/tmp/fathomdb-0831-qual/migrate-proof/`.
   - Confirm direct `Engine.open` in `0.8.31` refuses the schema-37 copy with `IncompatibleSchemaVersion` (`seen: 37, supported: 38`) without modifying its SHA-256.
   - Run `fathomdb migrate <disposable-copy> --backup-dir <backup-dir> --json`.
   - Assert:
     - Exit `0`, valid pre-upgrade backup file matching source SHA-256 `59d56eabb...`,
     - `PRAGMA user_version == 38`, `scope == ''` on migrated canonical/operational rows, `properties IS NULL` on pre-upgrade rows,
     - `fathomdb doctor check-integrity --full --json <disposable-copy>` returns `0` findings,
     - Active records (`req:1`, `ver:1`, `edge:verifies:1`), operational collections (`agent_checkpoints`, `agent_audit`), and `vec0` state are preserved; erased `secret:1` (`src-erase-1`) remains physically absent,
     - Subsequent `0.8.31` Python SDK writes with outside-`body` typed `properties`, `_fathomdb_commit_timestamps` batch timestamps, `as_of_write_cursor` / `as_of_epoch_ms` queries (verifying pre-upgrade `as_of_write_cursor` works and `as_of_epoch_ms` before first Schema-38 commit resolves empty), and `erase_source` all succeed cleanly.

---

### 3.5 Golden Schema-38 `0.8.31` Archive Design (for `0.8.34`)

Author deterministic builder `dev/plans/0.8.31/features/slice-70/build-golden-0.8.31.py` producing `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.31-schema38.sqlite` **outside the repository (never committed as binary bloat)** and committing only its JSON manifest `dev/plans/0.8.31/features/slice-70/golden-0.8.31-manifest.json` so `0.8.34` (`R34-02`, `R34-05`) has an authentic closed `0.8.31` producer artifact covering every Schema 38 construct. Open the builder with `use_default_embedder=True` against the pre-warmed local BGE cache (zero network) and call `eng.drain()` so real `vec0` (`attr_<hex>`) and enrolled custom-kind (`"domain_fact"`) rows materialize in `vector_default` / `_fathomdb_vector_rows`. After clean close, assert zero nonempty `-wal`, assert raw-file byte absence of erased `"secret:1"` body and inline `properties` canaries, run `fathomdb doctor check-integrity --full --json` **before** `chmod 0444`, and then apply `chmod 0444`:

| Artifact Class | Concrete Golden Fixture Content | `0.8.34` Verification Purpose |
| --- | --- | --- |
| Inline typed `properties` (nodes + edges, outside `body`) | Active nodes (`req:1`, `ver:1`, `fact:1`) & edges (`edge:verifies:1`, `edge:supports:1`) carrying `String`, `Int` (including `9007199254740993` `> 2^53`), `Float`, and `Bool` properties outside plain-text `body`, plus one legacy-shape `properties = NULL` node | Proves O(1) property preservation & exact `i64` precision across `0.8.31 -> 0.8.34` migration |
| Declared `Filterable` `canonical_property_values` & `vec0` slots | `configure_projections` registering node `Filterable` (`domain_tier`, `priority_rank`, `verified`) and edge `Filterable` (`weight_bp`) plus `vec0` pre-KNN node `attr_<hex>` coverage | Proves `canonical_property_values` (`text_val`, `num_val NUMERIC`, `bool_val`) and `vec0` attributes migrate/rebuild cleanly |
| `_fathomdb_commit_timestamps` | Multiple distinct committed batches recording monotonic `(write_cursor, committed_at_ms)` pairs, captured verbatim in manifest | Proves both `as_of_write_cursor` and `as_of_epoch_ms` replay across migrated stores |
| Historical superseded nodes & edges + bitemporal windows | `req:1` superseded `v1 -> v2`, `edge:verifies:1` superseded `v1 -> v2` with distinct `[t_valid, t_invalid)` intervals | Proves pre- and post-supersession historical graph reconstruction survives `0.8.34` migration |
| Explicitly enrolled custom vector kinds (`R1`) | Enroll custom kind `"domain_fact"` into `_fathomdb_vector_kinds`, write + `drain()` vector rows (`vector_default` / `_fathomdb_vector_rows`), and write one unenrolled kind `"raw_stub"` verified present in `vector_unsupported_kinds` | Proves `_fathomdb_vector_kinds` (`DurableAuthority`) and custom-kind vectors survive export/migration |
| Operational collections (`R12`) | Register & populate `agent_checkpoints` (`latest_state`) and `agent_audit` (`append_only_log`) | Proves `operational_collections`, `operational_state`, and `operational_mutations` integrity |
| Physically erased rows | Write `"secret:1"` (`source_id: "src-erase-1"`) with inline `properties`, an edge, and a custom-kind vector, then run `erase_source("src-erase-1")` and `purge("temp:1")` | Proves erased rows remain physically absent (`0` canonical/property/vector rows and raw byte absence) after upgrade |
| Reserved `scope` column & integrity metadata | Verify `scope == ''` on all surviving canonical & operational rows; attach `doctor check-integrity --full --json` summary, exact file size, `git_sha`, and SHA-256 in `golden-0.8.31-manifest.json` | Proves schema-38 `scope` readiness for `0.8.32` and clean input provenance for `0.8.34` |

---

### 3.6 Pre-Publication Release Notes, Generated Docs, `dev/{needs,requirements}.md` Sync & Truth Guard (`P3-01`, `P3-20`, `P6-08`)

1. **`docs/release-notes/0.8.31.md` & `mkdocs.yml` (`P3-20`):**
   - Author `docs/release-notes/0.8.31.md` following `docs/release-notes/0.8.30.md`'s structure, explicitly stamped at the top: **Pre-publication qualification notes — `0.8.31` is not yet published; registry publication awaits Slice 80 authorization.** Summarize all landed `0.8.31` features (Schema 38 inline properties & declared indexes, `_fathomdb_commit_timestamps`, explicit opt-in custom vector kinds, op-collection readback/refusal, strict-only write endpoints, typed traversal predicates, opt-in `graph_seed` + provenance, `as_of` historical graph, `resolve_evidence_many`, `search(hints=)`, `OpenReport` booleans, `fathomdb migrate` `37 -> 38`, `fathomdb-cli` PyPI wheel, W12 erasure fixes, Inspector `0.8.31` integration), platform/CUDA 12.6 limits (CUDA 13 deferred), and qualification summary.
   - Add `- 0.8.31 (pre-publication): release-notes/0.8.31.md` to `mkdocs.yml` under `Release Notes:` (Slice 80 `P3-21` removes `(pre-publication)` upon publication).
2. **`CHANGELOG.md` (`AC-050c` + `verify-release-gates.sh` Check 4):**
   - Add `## 0.8.31` (marked `Pre-publication qualification` per `CHANGELOG.md` preamble) summarizing all user-visible changes, breaking changes (Schema `37 -> 38` bump, strict write endpoint integrity), and any removed public API symbols detected by `scripts/security/check-removal-changelog.sh` (both default and `--base v0.8.30 --head HEAD`) under `### Removed`.
3. **`scripts/generate-public-docs.py`, `scripts/tests/test_generated_public_docs.py` & `docs/generated/*` (`P3-20`):**
   - Update `SOURCE_URL` in `scripts/generate-public-docs.py` to `'https://github.com/fathomadb/fathomdb/blob/release/0.8.31/'` AND update the matching URL assertion in `scripts/tests/test_generated_public_docs.py` to `release/0.8.31/`.
   - Run `.venv/bin/python scripts/generate-public-docs.py --write` to update `docs/generated/sdk-operation-map.md` (56 live governed operations) and `docs/generated/contributor-map.md` (13 workspace crates including `fathomdb-inspector`), then verify `--check` and `.venv/bin/python -m unittest scripts/tests/test_generated_public_docs.py`.
4. **`dev/needs.md` & `dev/requirements.md` (`P3-01`):**
   - Update `## Current applicability` in both `dev/needs.md` and `dev/requirements.md` from `format37` and `plans/plan-0.8.29.md` to `schema 38` and `scripts/release-current.py` / `dev/plans/0.8.31/plan.md`.
   - Clarify in applicability prose that `REQ-036`/`REQ-045` permit the narrow third CLI root `fathomdb migrate` (`37 -> 38`, `ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md`) while `Engine::open` remains schema-38-only (no public automatic migration), and that `REQ-037`/`AC-041` remain intact because the `fathomdb` Python package keeps `[project.scripts]` empty while the companion `fathomdb-cli` PyPI wheel distributes the CLI executable.
   - Leave `dev/acceptance.md` **completely untouched** (`status: locked`).
5. **`dev/interfaces/*.md` & Final Public-Doc Truth Currency (`P6-08`, `P3-19`):**
   - Refresh `dev/interfaces/{cli,python,rust,rust-sdk,typescript,wire}.md` frontmatter (`target_release: 0.8.31`, `date: 2026-11-23`), leaving `dev/interfaces/mcp.md` scoped cleanly to future evaluation/`0.8.32`.
   - Audit all remaining `0.8.30` references across public `docs/` and package `README.md` files (retaining historical `0.8.28`–`0.8.30` release notes and `0.8.30` retained Tegra manifest lineage where historical).
   - **Pre-publish guard:** Confirm `dev/plans/release-state-0.8.31.json` has `"published": null` and `scripts/check-public-doc-truth.py` keeps `RELEASE_SCHEMAS` without `"0.8.31"` so `check-public-doc-truth.py` fails immediately if any document claims `0.8.31` is published.

---

## 4. Slice Prep — Prerequisites Discovered

1. **Unconfined `ptrace`-Capable Executor for `AC-036` & `strace` Gates:**
   - `scripts/agent-verify.sh` invokes `STRICT=1 AC037_LIVE_OPTIONAL=1 bash scripts/agent-security.sh`, where `scripts/security/check-no-listen.sh` (`AC-036`) uses `strace`. Sandboxed executors denying `PTRACE_TRACEME` fail with `rc=2` (`BLOCKER` promoted to `exit 1` under `STRICT=1`). Likewise, `src/inspector/tools/qualify-installed-network.sh` requires parent-wrapped `strace -ff`. Both MUST execute in an unconfined ptrace-capable host session (`AGENTS.md` § 3).
   - Note that Ubuntu 24.04 AppArmor blocks unprivileged user namespaces locally (`AC037_LIVE_OPTIONAL=1` downgrades only the live netns layer while offline catch + policy tests run strict; full `AC037_LIVE_OPTIONAL=0` proof belongs on a userns-capable Linux CI runner).
2. **GPU Availability, VRAM Preflight & Native `schedule` Backoff:**
   - Host carries two RTX 3090 GPUs (indices `0`, `1`, 24 GiB each) plus a Quadro K620 display GPU (index `2`). Check `nvidia-smi` before running `FATHOMDB_FEATURE_COMPLETE=1 ./scripts/test-feature-complete.sh` or CUDA tests; if GPUs `0`/`1` are occupied or have `< 4,096 MiB` free VRAM, follow the `schedule` backoff protocol (Section 3.2) and never touch GPU `2`.
   - Ensure pinned model assets (BGE / reranker via `doctor warm-cache`, `nomic-v1.5`, pinned ONNX Runtime + exported `bge-small` ONNX model) are cached/verified so `FATHOMDB_REQUIRE_LIVE=1` does not fail on absent assets or offline restrictions.
3. **`scripts/check-public-doc-truth.py` Pre-Publish Guardrail (`P3-21` Boundary):**
   - `check-public-doc-truth.py` fails closed if:
     - Any of the 16 `PUBLIC_DOCS` contains phrases matching `asserts_published_version(..., "0.8.31")` while `"published": null`;
     - `docs/compatibility/index.md` omits `SCHEMA_VERSION` `**38**` or `"no public automatic migration"` (referring to `Engine::open`);
     - `docs/install/typescript.md` omits `npm install --save-exact fathomdb@0.8.31`;
     - `docs/generated/contributor-map.md` or `README.md` diverges from `Cargo.toml`'s 13 workspace members;
     - `RELEASE_SCHEMAS` in `scripts/check-public-doc-truth.py` is mutated prematurely before Slice 80 `published` receipt creation.
4. **Slice `10`–`60` Landed Entry & Clean-Tree Prerequisite:**
   - All Slices `10`–`60` must be merged on `release/0.8.31` with a clean `git status` (`local-ci.sh` rejects dirty working trees), checkout-owned `.venv` (CPython 3.12 on Ubuntu 24.04), `node_modules`, `src/ts/node_modules`, Chromium/Playwright caches, and intact Slice 0 golden DB (`/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.30-schema37.sqlite`, SHA-256 `59d56eabb9a6e817168c5c540b4bb2ec6eb7f741fb24a72c7617df4aa5004548`).
5. **`experiments/sdk_benchmark.py` Schema Isolation:**
   - Do not modify `experiments/sdk_benchmark.py`'s `PLAN_SCHEMA` (`sdk.experiment.v1`) or `experiments/measurement_classification.py` rules just to force two same-SDK versions into one suite; keep cross-version paired bootstrap orchestration in `dev/plans/0.8.31/features/slice-70/run-paired-latency-gate.py` alongside standard `experiments/sdk_benchmark.py` cross-SDK verification.
6. **Plan/Markdown Hygiene Before `ACTIVE` Promotion:**
   - When promoting this plan from `status: READY` to `status: ACTIVE` at Slice 70 execution start, ensure zero bare `<name>:<3+digits>` line anchors, zero orphan release-state markers, zero unverified symbol citations, and zero consumer-project specifics.

---

## 5. Slice Prep — TDD Implementation & Verification Plan

### Phase 5.1 — Entry Reconciliation & Candidate Freeze Prep

1. Verify Slices `10`–`60` landed cleanly (`git log`, `dev/plans/release-state-0.8.31.json` showing `slice 60 = LANDED` and `next_slice: 70`, `dev/plans/0.8.31/features/slice-60/status.md` committed, `scripts/check-release-state-views.sh`), inspect any deltas, promote this plan to `status: ACTIVE` upon plan reviewer `APPROVE`, and commit.
2. Apply **WP-70F** doc updates first so the candidate commit SHA verified by all runtime gates matches the final documented tree:
   - **RED → GREEN checks for docs/tooling:**
     - Update `SOURCE_URL` in `scripts/generate-public-docs.py` to `release/0.8.31/` and update `scripts/tests/test_generated_public_docs.py` to expect `release/0.8.31/`, run `--write`, and verify `--check` + `unittest scripts/tests/test_generated_public_docs.py` (`GREEN`).
     - Update `dev/needs.md` and `dev/requirements.md` (`P3-01`), leaving `dev/acceptance.md` untouched (`git diff dev/acceptance.md` empty).
     - Author `docs/release-notes/0.8.31.md`, update `mkdocs.yml`, update `CHANGELOG.md` (`## 0.8.31` pre-publication + `### Removed`), refresh `dev/interfaces/*.md` headers (`P6-08`), and verify zero consumer-project mentions.
     - Run doc/release preflight checks:

```bash
.venv/bin/python scripts/check-public-doc-truth.py
.venv/bin/python scripts/generate-public-docs.py --check
.venv/bin/python -m unittest scripts/tests/test_generated_public_docs.py
bash scripts/security/check-removal-changelog.sh
bash scripts/security/check-removal-changelog.sh --base v0.8.30 --head HEAD
grep -qE '^##[[:space:]]+v?0\.8\.31([[:space:]].*)?$' CHANGELOG.md
RELEASE_GATES_SKIP_GIT_REACH=1 RELEASE_GATES_TAG=v0.8.30 bash scripts/verify-release-gates.sh
bash scripts/agent-lint-docs.sh
bash scripts/agent-lint-md.sh
bash scripts/check-release-state-views.sh
.venv/bin/mkdocs build --strict
```

### Phase 5.2 — TDD Harness Authoring (`run-paired-latency-gate.py` & `build-golden-0.8.31.py`)

1. **RED first:** Write unit self-tests (e.g., `--self-test` mode or pytest test under `dev/plans/0.8.31/features/slice-70/test_slice70_harness.py`) asserting:
   - Paired 10,000-resample bootstrap CI rejects synthetic `+8%` p95 degradation (`> max(5%, 1ms)`) and accepts identical/noise-bounded pairs (`<= 1ms`);
   - `database_bytes` (`> +15%`) and `peak_rss_bytes` (`> +10%`) threshold guards fail closed on breach;
   - High-fanout predicate (`0.30x > 0.25x`) and `as_of` composite-seek (`5.5ms >= 5.0ms`) threshold evaluators fail closed on breach;
   - `build-golden-0.8.31.py` fails if erased rows reappear or `doctor check-integrity --full` reports findings.
2. **GREEN:** Implement `dev/plans/0.8.31/features/slice-70/run-paired-latency-gate.py` and `dev/plans/0.8.31/features/slice-70/build-golden-0.8.31.py`; run self-tests and `ruff check` / `pyright` cleanly on both scripts (or `.venv/bin/python -m py_compile` + `ruff check`).

### Phase 5.3 — Execution Sequence & Proportional Verification Matrix

Run the following qualification gates in latency/dependency order against the frozen clean candidate SHA (`git status --porcelain` empty):

1. **Full Unconfined Verification & Long-Window Suite (`WP-70A`):**

   ```bash
   # Unconfined ptrace-capable executor required for AC-036
   ./scripts/agent-verify.sh

   # Broad workspace check + AGENT_LONG=1 long variants
   AGENT_LONG=1 ./scripts/check.sh

   # Explicit canonical long & perf selectors
   AGENT_LONG=1 cargo test --locked -p fathomdb-engine --features operator --test lifecycle_reliability ac_021_zero_sqlite_schema_warnings_under_concurrent_reads_and_ddl -- --exact --nocapture
   AGENT_LONG=1 cargo test --locked -p fathomdb-engine --test cursor_read_after_write projection_cursor_bounds_observed_row_count -- --exact --nocapture
   AGENT_LONG=1 FATHOMDB_TIER1_WRITE_PERF=1 cargo test --locked --release -p fathomdb-engine --features test-hooks --test perf_gates ac_011a_write_throughput_1kb -- --exact --nocapture
   AGENT_LONG=1 FATHOMDB_TIER1_WRITE_PERF=1 cargo test --locked --release -p fathomdb-engine --features test-hooks --test perf_gates ac_011b_write_throughput_100kb -- --exact --nocapture
   AGENT_LONG=1 AC012_CORPUS_N=10000 cargo test --locked --release -p fathomdb-engine --features test-hooks --test perf_gates ac_012_text_query_latency_on_fts5_path -- --exact --nocapture
   AGENT_LONG=1 AC013_CORPUS_N=10000 AC013_VECTOR_DIM=768 cargo test --locked --release -p fathomdb-engine --features test-hooks --test perf_gates ac_013_vector_retrieval_latency -- --exact --nocapture
   cargo test --locked --release -p fathomdb-engine --features test-hooks --test perf_gates ac_081_absolute_read_performance -- --exact --nocapture

   # Supply-chain, security & surface gates
   bash scripts/security/gitleaks-current.sh
   bash scripts/security/gitleaks-history.sh
   bash scripts/check-license-consistency.sh --with-wheel
   bash scripts/check-governed-surface-pin.sh
   .venv/bin/python scripts/check-sdk-surface-parity.py
   .venv/bin/python scripts/check-sdk-surface-parity.py --binding rust --rust-crate src/rust/crates/fathomdb-sdk
   bash scripts/check-pinned-override-rot.sh
   bash scripts/check-platform-capabilities.sh
   bash scripts/set-version.sh --check-files
   ```

2. **Paired `v0.8.30` (`db271323f`) vs. `0.8.31` Latency & Query-Plan Gate (`WP-70B`):**

   ```bash
   # Query-plan SEARCH-only regression pins
   cargo test --locked -p fathomdb-engine --test slice38_query_plans -- --nocapture

   # Build isolated v0.8.30 (db271323f, CARGO_TARGET_DIR=/tmp/fathomdb-0831-qual/target-0830) & 0.8.31 noneditable release wheels and run 5k parity + 10k/50k/100k paired bootstrap + 50k micro-gates
   .venv/bin/python dev/plans/0.8.31/features/slice-70/run-paired-latency-gate.py \
     --baseline-sha db271323fdd38672fafd51898ca81c5763329af2 \
     --baseline-5k-json dev/plans/0.8.31/features/slice-0/baseline-0.8.30-latency.json \
     --scales 10000,50000,100000 \
     --warmup-rounds 5 \
     --min-samples 30 \
     --query-samples 100 \
     --bootstrap-resamples 10000 \
     --seed 20261122 \
     --work-dir /tmp/fathomdb-0831-qual/paired-bench \
     --out dev/plans/0.8.31/features/slice-70/paired-latency-report.json

   # Pinned cross-SDK synthetic capacity/parity check via experiments/sdk_benchmark.py (cleared FATHOMDB_* env; external receipts_root so tracked experiments/ stays clean)
   env -u FATHOMDB_SKIP_NETWORK_TESTS -u FATHOMDB_EMBED_DEVICE -u FATHOMDB_RERANK_DEVICE \
     .venv/bin/python dev/plans/0.8.31/features/slice-70/run-paired-latency-gate.py \
     --run-sdk-benchmark-10k \
     --sdk-suite-out /tmp/fathomdb-0831-qual/sdk-suite-10k.json \
     --sdk-out /tmp/fathomdb-0831-qual/sdk-benchmark-out \
     --sdk-receipts-root /tmp/fathomdb-0831-qual/sdk-receipts
   ```

3. **GPU & Feature-Complete Gate (`WP-70C`, with 7-Minute `schedule` Polling if Busy):**

   ```bash
   # Preflight GPU state (must see RTX 3090 on indices 0 and 1; never touch GPU 2 K620)
   nvidia-smi --query-gpu=index,pci.bus_id,name,memory.used,memory.free,memory.total --format=csv,noheader
   nvidia-smi --query-compute-apps=gpu_uuid,pid,used_memory --format=csv,noheader

   # If GPUs 0/1 are occupied or have < 4096 MiB free VRAM, poll every 7 minutes via native `schedule` (up to 24 attempts / 6h)
   .venv/bin/python scripts/check-test-target-coverage.py
   CUDA_DEVICE_ORDER=PCI_BUS_ID CUDA_VISIBLE_DEVICES=0,1 FATHOMDB_FEATURE_COMPLETE=1 FATHOMDB_REQUIRE_LIVE=1 bash scripts/test-feature-complete.sh --scratch /tmp/fathomdb-0831-qual/feature-complete
   ```

4. **Inspector `--full-workspace` & `D4 = B` Native Qualification (`WP-70D`):**

   ```bash
   # Clean tree + pre-built fixtures prerequisite
   test -z "$(git status --porcelain)"
   cargo build --locked --offline -p fathomdb-inspector
   rm -rf /tmp/fathomdb-0831-qual/inspector-candidate /tmp/fathomdb-0831-qual/inspector-network-trace
   mkdir -p /tmp/fathomdb-0831-qual/inspector-target /tmp/fathomdb-0831-qual/inspector-fixtures
   # Generate person, vector, f06 (+ scale append), study-training, study-unseen, lexical-deadline fixtures into inspector-fixtures
   bash src/inspector/tools/local-ci.sh --full-workspace /tmp/fathomdb-0831-qual/inspector-candidate /tmp/fathomdb-0831-qual/inspector-fixtures /tmp/fathomdb-0831-qual/inspector-target
   bash src/inspector/tools/qualify-installed-network.sh \
     /tmp/fathomdb-0831-qual/inspector-candidate/fathomdb-inspector-0.3.0-linux-x86_64 \
     /tmp/fathomdb-0831-qual/inspector-fixtures \
     /tmp/fathomdb-0831-qual/inspector-target \
     /tmp/fathomdb-0831-qual/inspector-network-trace
   ```

5. **Clean-Venv `fathomdb-cli` Wheel & Golden Schema-37 `fathomdb migrate` Proof (`WP-70E`):**

   ```bash
   # Build lockstep candidate wheels from package roots (Axis-W 0.8.30) and validate wheel contents
   REPO_ROOT="$(git rev-parse --show-toplevel)"
   WS_VER="$(.venv/bin/python -c 'import tomllib; print(tomllib.load(open("Cargo.toml","rb"))["workspace"]["package"]["version"])')"
   rm -rf /tmp/fathomdb-0831-qual/wheels /tmp/fathomdb-0831-qual/cli-smoke-venv
   (cd "$REPO_ROOT/src/python" && "$REPO_ROOT/.venv/bin/maturin" build --release --out /tmp/fathomdb-0831-qual/wheels)
   (cd "$REPO_ROOT/src/python-cli" && "$REPO_ROOT/.venv/bin/maturin" build --release --out /tmp/fathomdb-0831-qual/wheels)
   .venv/bin/python scripts/check_python_wheel_contents.py --kind sdk /tmp/fathomdb-0831-qual/wheels/fathomdb-*.whl
   .venv/bin/python scripts/check_python_wheel_contents.py --kind cli /tmp/fathomdb-0831-qual/wheels/fathomdb_cli-*.whl
   /usr/bin/python3.12 -m venv /tmp/fathomdb-0831-qual/cli-smoke-venv
   /tmp/fathomdb-0831-qual/cli-smoke-venv/bin/pip install --no-index --find-links /tmp/fathomdb-0831-qual/wheels "fathomdb[cli]==${WS_VER}"
   bash scripts/check-glibc-floor.sh --floor 2.39 /tmp/fathomdb-0831-qual/cli-smoke-venv/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so /tmp/fathomdb-0831-qual/cli-smoke-venv/bin/fathomdb
   # Verify with cargo/rustc absent from PATH: fathomdb --version, doctor check-integrity, migrate 37->38 on disposable copy of golden-0.8.30-schema37.sqlite, and smoke_slice55_installed.py
   ```

6. **Closed Golden Schema-38 `0.8.31` Archive for `0.8.34` (`WP-70G`):**

   ```bash
   # Builder opens with use_default_embedder=True against warm local BGE cache, verifies zero -wal & erased-byte absence,
   # runs `fathomdb doctor check-integrity --full --json` BEFORE `chmod 0444`, and stores the .sqlite outside the repo.
   .venv/bin/python dev/plans/0.8.31/features/slice-70/build-golden-0.8.31.py \
     --out /mnt/data/worktrees/fathomdb-0831-golden/golden-0.8.31-schema38.sqlite \
     --manifest dev/plans/0.8.31/features/slice-70/golden-0.8.31-manifest.json
   chmod 0444 /mnt/data/worktrees/fathomdb-0831-golden/golden-0.8.31-schema38.sqlite
   ```

### Phase 5.4 — Slice 70 Closure & Slice 80 HITL Publication Handoff

1. Run independent adversarial review (`ROLE_REVIEWER`, `TIER_DEEP`) over Slice 70 receipts, harnesses, and pre-publication docs.
2. Record full qualification receipts, SHAs, paired bootstrap summary tables, GPU/Inspector/CLI smoke results, and golden archive SHA-256 in `dev/plans/0.8.31/features/slice-70/status.md`.
3. Advance `dev/plans/release-state-0.8.31.json` (`slice 70 = LANDED`, `next_slice: 80`, keeping `"published": null` and `publish_gate.publication_authorized = false`) and regenerate views (`./scripts/check-release-state-views.sh --write`).
4. **STOP BEFORE SLICE 80 — Slice 80 HITL Publication Handoff Checklist:** Present the complete `0.8.31` qualification packet to the owner and request explicit separate **HITL publication authorization** covering:
   1. **PyPI Pending Publisher prerequisite (`fathomdb-cli`):** Owner configures a PyPI Trusted Publisher (Pending Publisher, `environment: pypi`) for the brand-new `fathomdb-cli` project targeting `.github/workflows/release.yml` **before** tagging `v0.8.31` (preventing mid-publish failure after crates.io uploads).
   2. **Axis-W version cut & merge:** Run `bash scripts/set-version.sh --workspace 0.8.31` and merge `release/0.8.31 -> main` (`verify-release-gates.sh` Check 3).
   3. **Tag, publish & post-publish smoke:** Create and push `v0.8.31`, verify all registry uploads (`crates.io`, PyPI `fathomdb` + `fathomdb-cli`, `npm`, Tegra Pages, GitHub Release Inspector `0.3.0`), and run post-publish smoke installs.
   4. **Post-publish truth flip (`P3-21`):** Add `"0.8.31": 38` to `RELEASE_SCHEMAS` in `scripts/check-public-doc-truth.py`, remove `(pre-publication)` labels from `docs/release-notes/0.8.31.md` / `mkdocs.yml` / `CHANGELOG.md`, and record `"published"` receipt in `dev/plans/release-state-0.8.31.json`.
