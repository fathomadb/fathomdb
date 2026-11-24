---
title: 0.8.32 Prework Slice 5 — Testing & Verification Adequacy Audit
status: COMPLETE
target_release: 0.8.32
slice: 5
date: 2026-11-24
---

# 0.8.32 Prework Slice 5 — Testing & Verification Adequacy Audit

## 1. Executive Summary & Verification Posture

Read-only audit of `/mnt/data/worktrees/release-0.8.32` (`dev/0.8.32` @ `38a1594bf`) against [`dev/plans/0.8.32/scope.md`](../scope.md), [`dev/plans/0.8.32/plan.md`](../plan.md), [`HANDOFF-FROM-0.8.31.md`](../HANDOFF-FROM-0.8.31.md), [`dev/notes/fathomdb-quality-testing-roadmap.md`](../../../notes/fathomdb-quality-testing-roadmap.md), [`dev/plans/0.8.31/quality-backlog.md`](../../0.8.31/quality-backlog.md), and Owner Rulings 1–3 (+ addenda) recorded in [`prework/slice-0.md`](./slice-0.md).

### Key Findings

1. **Strong existing reliability base, zero current coverage of `0.8.32`'s flagship invariants:**
   - `fathomdb-engine/tests/slice85_scope.rs` tests **physical erasure selector scope** (`SourceAllAtCut`), **not** Schema-38 caller/namespace `scope`.
   - `fathomdb-mcp` is hard-locked read-only and unscoped (`config.rs`, `edge.rs`, `protocol.rs`, `readonly.rs`), has **zero** startup conflict checks (`--check-config` in `main.rs:61-65` exits before `Adapter::new`), and `bridge::call` + `registry::shutdown` currently allow uninterruptible background blocking closures to outlive client cancellation or shutdown (`bridge.rs:46-86`, `registry.rs:990-996`).
2. **Critical WAL durability model mismatch (`open.rs:1262`):** `fathomdb-engine` opens the writer connection with `PRAGMA journal_mode = WAL` and **`PRAGMA synchronous = NORMAL`** (`open.rs:1262`). Under SQLite WAL semantics, `NORMAL` guarantees crash consistency without corruption, **not** zero-transaction-loss across real power failure (`dm-flakey`) for un-checkpointed WAL commits sitting in volatile buffers. `0.8.32` **must** ratify and declare its exact synchronous/checkpoint durability contract before running the `dm-flakey` acknowledged-write oracle.
3. **Host-path golden fixture defects & CI blindspots:**
   - `fathomdb-cli/tests/migrate_cli.rs:14` and `fathomdb-engine/tests/slice50_migrate_37_to_38.rs:18` hardcode `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.30-schema37.sqlite` (breaking `windows-wal-checkpoint-diagnosis`).
   - `ci.yml` executes `fathomdb-inspector` Rust tests only; Inspector Playwright browser tests (`src/inspector/tests/browser/*.mjs`) and JS unit tests (`src/inspector/tests/unit/*.mjs`) run only under manual `src/inspector/tools/local-ci.sh`.
4. **Property, leak, backup/rebuild, and storage-headroom gaps:**
   - `property_template.rs` and `test_property_template.py` only test 12 trivial identity-reopen cases; `safe_export.rs` only tests SHA-256 tamper detection, not full corpus backup/restore/rebuild or `fathomdb.logical-export/v1`.
   - `database_bytes` at 100k nodes hit `200.4 MB` (`+13.09%` over `0.8.30`, near `+15%` ceiling); prefer partial indexes `WHERE scope != ''` so single-scope DB growth stays ~`0%`.
   - Neither `fuzz/` (`exclude = ["fuzz"]`), `cargo-fuzz`, nor a pinned Ubuntu 24.04 QEMU/KVM `dm_flakey.ko` guest image exist yet.

### Alignment with `fathomdb-quality-testing-roadmap.md`

- **Applies directly in `0.8.32`:** `0.8.31/quality-backlog.md` (fuzzing + `dm-flakey` power-loss proof), anomaly testing, resource-leak checks (MCP + SDK FDs/WAL/workers), disabled-optimization equivalence oracles (single-scope fast path vs scoped reference path; indexed vs inline property filters), and boundary-value / targeted mutation checks.
- **Explicitly deferred:** 100% workspace MC/DC, general SQLite SQL/B-tree fuzzing, all-allocation OOM sweeps, and `2.x` ANN backend qualification.

---

## 2. Requirements -> AC -> Test Traceability Matrix

| Requirement / Ruling | Release-Local AC | Existing Coverage Today | Adequacy Gap / Missing Verification | Required Target Test / Gate (`0.8.32` RED->GREEN) | Phase B Slice |
| --- | --- | --- | --- | --- | --- |
| **`R32-07`** | `AC32-ORIENT-01..02`, `AC32-GATE-01..02`, `AC32-REFACTOR-01` | Parity/pin/boundary/lint scripts; 4 `verify-fast` & `migrate_cli` failing on `38a1594bf` | No diagnostic-string contract tests; 10 `*_tests.rs` in `fathomdb-engine/src/`; monolithic `search.rs`/`evidence.rs`; host-hardcoded golden path | `test_gate_diagnostics_0832.sh`; relocate 10 `*_tests.rs` & split `search`/`evidence` with full test parity; fix `verify-fast` + move golden Schema-37 fixture to `dev/fixtures/` | **10**, **20** |
| **`R32-01`** | `AC32-SCOPE-01..05` | `slice85_scope.rs` (erasure only), `slice35_frozen_read.rs`, `slice38_query_plans.rs` | Zero caller `scope` isolation, frozen/evidence `scope` binding, anti-spoof, or scoped `EXPLAIN QUERY PLAN` tests; workstation-only `golden-0.8.31` path | `scope_isolation.rs` (2-caller overlapping corpus across all routes + revocation + anti-spoof); `scope_query_plans.rs`; Python/TS parity; CI-portable Schema-38 compat test + golden verification | **20**, **60** |
| **`R32-02` + Owner Ruling 1(a)** | `AC32-MCP-STARTUP-01..08` | `config.rs` rejects `"write": true` and non-null `"scope"` | `Config` struct omits `allow_writes`/`scope`; `--check-config` exits before `Adapter::new`; zero startup conflict matrix | `fathomdb-mcp/tests/startup_conflicts.rs` (`SC-01`..`SC-10` via `--check-config` exit `2` + `Adapter::new` / `register_engine`) | **40** |
| **`R32-02` + Owner Ruling 1(b)** | `AC32-MCP-SAFETY-01..09` | `admission.rs`, `readonly.rs`, `protocol.rs`, `lifecycle.rs` | Zero write/checkpoint/frozen-evidence tools; `bridge.rs`/`registry.rs` abandoned blocking work on cancel/shutdown; no adversarial secret-scrub tests | `mcp_write_safety.rs` (`AC32-MCP-SAFETY-01..09`) + `mcp_frozen_evidence.rs` (SDK-vs-MCP exact-byte/revision/error equivalence) | **40** |
| **Owner Ruling 3** | `AC32-MCP-PKG-01..03`, `AC-041` | `manifest.rs`, `test_slice130_package_boundary.py`, `check_python_wheel_contents.py` | `publish = false`, missing `LICENSE`/`README.md`, `src/python-mcp/` absent, 16 contract tests + smokes missing MCP | `cargo package --locked` dry-run, `check_python_wheel_contents.py --kind mcp`, `AC-041` empty base scripts check, `smoke-crates-mcp.sh` + PyPI/5-target GH Release stdio smokes | **40**, **60–70** |
| **`R32-03`** | `AC32-BULK-01..03` | `slice38_strict_endpoints.rs`, `pr_g8_dangling_edges.rs`, `operator_cli.rs` | No opt-in bulk write mode or `doctor` dangling-edge flow | `bulk_write_opt_in.rs` + `doctor_dangling_edges.rs` + Python/TS parity (strict default + strict `actuate` preserved) | **30** |
| **`R32-06`** | `AC32-MANIFEST-01`, `AC32-ERASE-01`, `AC32-INS-01..03`, `AC32-PERF-01..02` | `slice38_erasure_and_audit.rs`, `disclosure.rs`, `run-paired-latency-gate.py` | Duplicated manual table lists; no scope-aware erasure checks; CI skips Inspector Playwright/JS unit tests; `200.4 MB` DB-size headroom tight | `table_manifest_completeness.rs`; scope erasure tests; Inspector Rust + Playwright scope/D-01 wired into CI/qual; paired `0.8.31` vs `0.8.32` 10k/50k/100k + scoped micro-gate | **20**, **30**, **60** |
| **`R32-04`** | `AC32-BACKUP-01..03` | `safe_export.rs`, `rebuild_projections.rs`, `rebuild_vec0.rs` | No full corpus backup/restore/rebuild parity or `fathomdb.logical-export/v1` round-trip oracle | `logical_export_roundtrip.rs` + `qualify-backup-restore-rebuild.sh` on pinned installed artifacts | **50**, **60** |
| **`R32-05` + Owner Ruling 2** | `AC32-DMFLAKEY-01..04`, `AC32-FUZZ-01..03`, `AC32-LEAK-01..02` | `durability_soak.rs` (`SIGKILL`-only), `AC-022b`, `property_template.rs` (12 cases) | `open.rs:1262` `synchronous = NORMAL` mismatch; no QEMU/KVM `dm_flakey.ko` guest; no `fuzz/` workspace or `cargo-fuzz`; no MCP/SDK cancel/shutdown leak test | QEMU/KVM `dm-flakey` fault campaign + external ack oracle; isolated `fuzz/` (7 targets + ASan/UBSan + committed regressions); MCP/SDK leak + equivalence + boundary/mutation tests | **10**, **50** |

---

## 3. Deep-Dive Verification & Adequacy Analysis

### 3.1 Owner Ruling 1(a): Fail-Fast Runtime Startup Configuration Conflict Matrix (`fathomdb-mcp`)

Every case below must fail **before** opening any engine, acquiring `.lock`, modifying disk state, or advertising `tools/list` (CLI `--check-config`: exit `2`, empty `stdout`, structured error JSON on `stderr`; embedded `Adapter::new` / `register_engine`: typed error):

| Case ID | Conflicting Startup Inputs | Expected Outcome & Conflict Reason |
| --- | --- | --- |
| `SC-01` | Store `allow_writes: true` with `scope: null` / omitted / empty `""` | Reject: mutating store requires explicit non-empty bound caller `scope` policy |
| `SC-02` | Store `allow_writes: true` with read-only `scope` grant | Reject: store write enablement conflicts with non-writing caller `scope` grant |
| `SC-03` | Store `allow_writes: false` paired with write-only/write-grant keys or `create: true` | Reject: incoherent read-only store configuration |
| `SC-04` | Legacy evaluation key `"write"` combined with `"allow_writes"` (or `"write": true`) | Reject: ambiguous/legacy key conflict naming exact keys |
| `SC-05` | `roots` auto-discovery paired with write enablement | Reject: directory-discovered stores must not inherit `allow_writes: true` |
| `SC-06` | Two store names resolving via canonical path/symlink to the **same** DB with incompatible `allow_writes`/`scope`/`eager` | Reject at startup: prevents cross-scope policy bypass or self-lock deadlock |
| `SC-07` | Multiple `eager: true` stores when `max_open_stores == 1` | Reject: single-open-store invariant violation |
| `SC-08` | Contradictory `scope` policy grammar (empty/duplicate patterns, wildcard `"*"` with write, contradicting allow/deny, NUL/control chars) | Reject: fail-closed scope parser |
| `SC-09` | Non-addressable `default_store`, `hybrid` with `use_default_embedder: false`, `run_limit_ms < call_deadline_ms` | Reject: cross-field constraints preserved |
| `SC-10` | Embedded `Adapter::register_engine` conflicts with configured `stores[name]` or host `EngineConfig.scope_policy` | Reject before serving MCP tools |

### 3.2 Owner Ruling 1(b): `fathomdb-mcp` Write-Safety Test Suite (`AC32-MCP-SAFETY-01..09`)

- `AC32-MCP-SAFETY-01`: Read-only default (`allow_writes: false`, write tools absent from `tools/list`, direct calls refused with zero DB mutation) + per-store write serialization under concurrent MCP write tasks.
- `AC32-MCP-SAFETY-02`: Idempotent `(store, scope, operation_id)` replay across session/idle-close/restart + conflict refusal on changed payload or changed scope (zero mutation).
- `AC32-MCP-SAFETY-03`: Concurrent `expected_write_boundary` CAS race -> one winner, loser gets `stale_write_boundary` with zero mutation, clean retry succeeds.
- `AC32-MCP-SAFETY-04`: Two-caller scope isolation & anti-spoof across all read/write/checkpoint/frozen-evidence/continuation routes (nondisclosing `not_found`).
- `AC32-MCP-SAFETY-05`: Strict-only MCP writes (dangling/inactive/cross-scope endpoints, invalid properties, batch limits, bulk flags refused; atomic zero-row rollback).
- `AC32-MCP-SAFETY-06`: Deadline/timeout/`notifications/cancelled` & `Adapter::shutdown()` drain fence (remediating `bridge.rs:46-86` & `registry.rs:990-996` abandoned blocking closures).
- `AC32-MCP-SAFETY-07`: `SIGKILL` crash/reopen durability (`PRAGMA integrity_check == ok`, no dangling locks, `operation_id` replay intact).
- `AC32-MCP-SAFETY-08`: Adversarial secret-scrubbed error tests across all error classes and `stderr` logs (zero secret substrings, SQL text, or out-of-scope existence leaks).
- `AC32-MCP-SAFETY-09`: Forbidden operator escape hatches (`recover`/`restore`/`repair`/`fix`/`rebuild`/`purge`/`erase_source`/`migrate`/`safe_export`/bulk/raw SQL) never listed and always refused.
- Plus `mcp_frozen_evidence.rs`: SDK-vs-MCP identical-task equivalence oracle (exact bytes, `artifact_revision_id`, scope isolation, typed drift/erasure/truncation).

### 3.3 Caller `scope`, `EXPLAIN QUERY PLAN`, Bulk, Inspector, Durability & Fuzzing

- **Caller `scope`:** `scope_isolation.rs` + `scope_query_plans.rs` (`SEARCH`-only + single-scope fast-path equivalence oracle) + CI-portable Schema-38 backward-compat test + SHA-256 `golden-0.8.31-schema38.sqlite` qualification verification.
- **Durability (`open.rs:1262`):** Reconcile `PRAGMA synchronous = NORMAL` vs `FULL` before running the QEMU/KVM `dm-flakey` power-loss acknowledged-write oracle across commit, WAL, `wal_checkpoint(TRUNCATE)` (incl. erasure), and reopen/recovery boundaries.
- **Fuzzing:** Isolated `fuzz/` workspace (`exclude = ["fuzz"]`) with 7 targets (`fuzz_query_filter`, `fuzz_property_predicates`, `fuzz_scope_policy`, `fuzz_auth_tokens`, `fuzz_wire_codecs`, `fuzz_mcp_jsonrpc`, `fuzz_export_and_open_recovery`) + `ASan`/`UBSan` + committed deterministic regression tests.
- **Paired `0.8.31` vs `0.8.32` latency/RSS/DB gate:** Hold `0.8.31` 100k baselines (`process_open 238.9 ms`, `warm_reopen 231.9 ms`, `write_node_batch_200 32.8 ms`, `write_edge_batch_200 15.5 ms`, `search_text_only 75.5 ms`, `search_hybrid 126.9 ms`, `graph_expand_2hop 1.41 ms`, `database_bytes 200.4 MB`, `peak_rss 165.0 MB`), watching `database_bytes` (`+13.09%` already used in `0.8.31`) via partial indexes `WHERE scope != ''`, plus add multi-caller scoped 50k/100k checks.

### 3.4 Existing Tests Intentionally Impacted Across `0.8.32` Slices

| Existing Test / Gate | Current Assumption in `38a1594bf` | Required `0.8.32` Lockstep Update | Slice |
| --- | --- | --- | --- |
| `fathomdb-cli/tests/migrate_cli.rs:14` & `fathomdb-engine/tests/slice50_migrate_37_to_38.rs:18` | Hardcoded `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.30-schema37.sqlite` | Resolve tracked `dev/fixtures/golden-0.8.30-schema37.sqlite` via `CARGO_MANIFEST_DIR` (+ optional env override) | **10** |
| `scripts/tests/test_{tegra_publication_operator_path,pages_site,tegra_pages_retention,agent_typecheck_pyright_version}.sh` | Stale `0.8.30+tegra` hardcode, stale `contributor-map.md`, broken isolated fixture | Dynamic `tegra-pages-retained.json` lookup, regenerate docs + lint `--check`, preflight ordering + fixture fix | **10** |
| `fathomdb-mcp/tests/config.rs:99-103, 311-321` | Asserts `"write": true` and non-null `"scope"` fail unconditionally | Update for `allow_writes` + `scope` config grammar & startup conflict matrix | **40** |
| `fathomdb-mcp/tests/protocol.rs:12-13, 50` & `lifecycle.rs:52` | Asserts static `TOOL_NAMES` length `== 7` across all configs | Assert `7` tools when `allow_writes: false`, expanded write/checkpoint/frozen-evidence toolset when `allow_writes: true` + scope bound | **40** |
| `fathomdb-mcp/tests/readonly.rs` & `manifest.rs` | Tests global read-only only; `Cargo.toml` examples include `locomo_eval` | Keep `allow_writes: false` read-only digest test AND add direct write-refusal test; update manifest checks for `publish = true` + `LICENSE` | **40** |
| `fathomdb-sdk/tests/surface.rs`, `fathomdb/tests/governed_surface.rs`, `src/ts/tests/sdk-surface-parity.test.ts` (`56/56`), `src/conformance/governed-*.json`, `scripts/governed-surface-pin.json`, `_fathomdb.pyi` | Pinned at `56` canonical ops / `93` allowlist members | Update **only** for HITL Gate 1 approved additions (`ScopePolicy`, opt-in bulk API, or promoted `EP-*` unblockers); keep `core` (`5`) & `recovery_denylist` (`5`) fixed | **20 / 30 / 40** |
| `scripts/tests/test_release_workflow_scope.sh:296`, `test_runtime_release_alignment.sh:262`, `test_release_version_surfaces.sh:95`, `check-public-doc-truth.py:61` | Hardcoded `upload-artifact-retry` count (`15`), pre-`fathomdb-mcp` smoke/version lists, `"0.8.31": 38` | Update counts/lists for `fathomdb-mcp` (`build-python-mcp`, `publish-rust-t7-mcp`, `publish-pypi`, `promote-npm-release.yml`) & `0.8.32` | **40 / 60 / 70** |

---

## 4. Numbered Proposals (`P5-01`–`P5-18`)

| ID | Proposal | Understood (H/M/L) | Risk (H/M/L) | Effort (S/M/L) | Recommendation | Target Slice / Rationale |
| --- | --- | :---: | :---: | :---: | --- | --- |
| `P5-01` | Commit Schema-37 golden DB (`dev/fixtures/golden-0.8.30-schema37.sqlite`) + fix `migrate_cli.rs` & `slice50_migrate_37_to_38.rs` relative lookup, and fix the 4 failing `verify-fast` test suites so baseline CI is 100% green before Slice 20. | H | L | S | **include** | **Slice 10 (`PREWORK-PREP`)** — Unblocks `windows-wal-checkpoint-diagnosis` and `verify-fast` immediately. |
| `P5-02` | Add shell contract tests for tightened `R32-07` error diagnostics (`check-sdk-surface-parity.py`, `check-governed-surface-pin.sh`, `module-boundary-gate`) asserting exact owner config file names appear on failure. | H | L | S | **include** | **Slice 10 (`PREWORK-PREP`)** — Prevents diagnostic regressions in `R32-07`. |
| `P5-03` | Ratify & document `PRAGMA synchronous` durability posture vs acknowledged-write guarantees (reconciling `open.rs:1262` `synchronous = NORMAL`) in the `ADR-0.8.32` durability fault spec, AND pre-stage Ubuntu 24.04 QEMU/KVM (`dm_flakey.ko`) guest image + `cargo-fuzz` toolchain check. | H | H | M | **include** | **Slice 10 (`PREWORK-PREP`)** — Eliminates silent oracle mismatch (`WAL + NORMAL` vs `dm-flakey`) and missing VM/fuzz prerequisites before Slice 50. |
| `P5-04` | Enforce zero-regression parity across the `fathomdb-engine` `search.rs`/`evidence.rs` submodule split and 10 `*_tests.rs` relocations (preserving full feature-matrix test target coverage in `scripts/test-feature-matrix.toml`). | H | M | M | **include** | **Slice 20 (`SCOPE-AND-REFACTOR`)** — Satisfies `HANDOFF-05` & `R32-07` cleanly. |
| `P5-05` | Implement declarative `TABLE_MANIFEST` completeness tests in `fathomdb-schema` verifying `CANONICAL_TABLES`, `ROW_OWNED_PROJECTIONS`, visibility triggers, erasure, Inspector parity, and `ExportClass` agreement (plus negative unknown-table failure). | H | L | M | **include** | **Slice 20 (`SCOPE-AND-REFACTOR`)** — Locks `R32-06` single-source invariant. |
| `P5-06` | Build `fathomdb-engine/tests/scope_isolation.rs` + Python/TS parity tests: two-caller overlapping fixtures across all read/search/graph/frozen/continuation/evidence/write/actuate/purge/erase routes, grant revocation on stale refs/cursors, and write anti-spoof. | H | H | L | **include** | **Slice 20 (`SCOPE-AND-REFACTOR`)** — Core proof for `R32-01` & External Reviewer P1. |
| `P5-07` | Add `scope_query_plans.rs` (`EXPLAIN QUERY PLAN` `SEARCH`-only gate + single-scope fast-path vs scoped equivalence oracle + indexed vs inline property predicate equivalence oracle) and CI-portable `0.8.31` Schema-38 backward-compat tests. | H | M | M | **include** | **Slice 20 (`SCOPE-AND-REFACTOR`)** — Guards single-scope zero-regression (`HANDOFF-04`) and avoids workstation path dependencies. |
| `P5-08` | Add `bulk_write_opt_in.rs` + `fathomdb-cli/tests/doctor_dangling_edges.rs` + Python/TS parity tests: verify strict `Engine::write` / `actuate` default preserved, opt-in bulk reports exact `dangling_edge_endpoints`, reconciles later endpoints, respects `scope`, and integrates with `doctor`. | H | M | M | **include** | **Slice 30 (`BULK-AND-INSPECTOR`)** — Proves `R32-03` across all SDKs and CLI. |
| `P5-09` | Add Inspector Rust + Playwright tests for active caller `scope` UI strictly subordinated to `D-01` nondisclosure, AND wire Inspector JS unit + Playwright smoke checks into CI/release-qualification (`ci.yml` / Slice 60) to close the manual-only `local-ci.sh` gap. | H | M | M | **include** | **Slice 30 (`BULK-AND-INSPECTOR`)** & Slice 60 — Closes Inspector browser CI blindspot (`R32-06`). |
| `P5-10` | Implement `fathomdb-mcp` fail-fast runtime startup conflict matrix (`SC-01`..`SC-10`) across both `--check-config` (`main.rs`, exit `2`, zero open) and `Adapter::new` / `register_engine` (`Config::validate` updated to hold `allow_writes`/`scope`). | H | H | M | **include** | **Slice 40 (`MCP-RW`)** — Directly fulfills Owner Ruling 1(a). |
| `P5-11` | Implement `AC32-MCP-SAFETY-01..09` (`mcp_write_safety.rs`) + `mcp_frozen_evidence.rs` (SDK-vs-MCP exact-byte/revision/error equivalence) + explicit blocking-write cancellation/shutdown fence tests addressing `bridge.rs`/`registry.rs` abandonment behavior and adversarial secret-scrubbing checks. | H | H | L | **include** | **Slice 40 (`MCP-RW`)** — Directly fulfills Owner Ruling 1(b) & `R32-02`. |
| `P5-12` | Verify `fathomdb-mcp` Owner Ruling 3 packaging gates: `cargo package --locked` dry-run (`publish = true`, `LICENSE`, detached `locomo_*` eval examples), `check_python_wheel_contents.py --kind mcp`, `AC-041` empty base `fathomdb` scripts test, updated 16 release-contract tests, `smoke-crates-mcp.sh`, and PyPI/5-target GH Release stdio smokes. | H | M | M | **include** | **Slice 40 (`MCP-RW`)** & Slices 60–70 — Directly fulfills Owner Ruling 3 & `AC-041`. |
| `P5-13` | Implement end-to-end `fathomdb.logical-export/v1` round-trip (`ExportClass` validation, uncentered `f32` + `_fathomdb_vector_spans` + `EmbedderIdentity` included, `embedding_bin`/`mean_vec`/erased bytes excluded) + pinned installed-artifact backup/restore/rebuild parity script (`R32-04`). | H | H | L | **include** | **Slice 50 (`DURABILITY-AND-FUZZ`)** & Slice 60 — Directly fulfills External Reviewer P0 (`R32-04`). |
| `P5-14` | Execute disposable QEMU/KVM + `dm-flakey` power-loss fault campaign across commit, WAL, `wal_checkpoint(TRUNCATE)` (incl. erasure), and reopen/recovery boundaries with external acknowledged-write oracle, `doctor check-integrity --full`, and post-recovery writes (`R32-05`). | H | H | L | **include** | **Slice 50 (`DURABILITY-AND-FUZZ`)** — Directly fulfills Owner Ruling 2 & `quality-backlog.md`. |
| `P5-15` | Stand up isolated `fuzz/` workspace (`exclude = ["fuzz"]` + `cargo tree` CI gate) with 7 coverage-guided targets + seed corpora + `ASan`/`UBSan` runs (`nightly-2026-08-01`) and commit all minimized reproducers into deterministic `cargo test` regression suites. | H | M | L | **include** | **Slice 50 (`DURABILITY-AND-FUZZ`)** — Directly fulfills `quality-backlog.md` & `R32-05`. |
| `P5-16` | Expand resource-leak checks (`AC32-LEAK-01..02`: FDs, `-wal`/`-shm`/`.lock`, Tokio/worker threads after MCP cancellation, deadline timeout, `Adapter::shutdown`, and SDK loops), replace 12-case `property_template` stubs with real domain state-machine/boundary checks, and run targeted `0.8.32` mutant kills. | H | M | M | **include** | **Slices 20 / 40 / 50** — Operationalizes `fathomdb-quality-testing-roadmap.md`. |
| `P5-17` | Run paired `0.8.31` vs `0.8.32` `10k`/`50k`/`100k` latency, RSS & DB-size gate (`run-paired-latency-gate.py`) watching `200.4 MB` `database_bytes` headroom closely (partial index `WHERE scope != ''` verified), plus add a multi-caller scoped 50k/100k benchmark + verify SHA-256 `golden-0.8.31-schema38.sqlite` open compat. | H | M | M | **include** | **Slice 60 (`QUALIFICATION`)** — Proves `HANDOFF-02` & `R32-06` before tagging. |
| `P5-18` | Wire `smoke-crates-mcp.sh`, clean-venv PyPI `fathomdb[mcp]` smoke, 5-target GH Release `fathomdb-mcp` stdio smokes, and archive the deterministic `0.8.32` Schema-38 golden DB + manifest (`0.8.34` readiness) at release qualification/publication. | H | M | M | **include** | **Slices 60–70 (`QUALIFICATION` / `PUBLICATION`)** — Enforces post-publish consumer verification (`AGENTS.md` § 10). |
