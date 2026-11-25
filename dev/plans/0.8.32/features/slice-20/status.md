---
title: Slice 20 Status — 0.8.32 Scope Isolation and Refactor
status: CLOSED
target_release: 0.8.32
slice: 20
date: 2026-11-25
---

# Slice 20 Status — `0.8.32` Scope Isolation and Refactor (`SCOPE-AND-REFACTOR`)

- **Branch / Worktree:** `dev/0.8.32` in `/mnt/data/worktrees/release-0.8.32`
- **Plan:** [`dev/plans/0.8.32/features/slice-20/plan.md`](./plan.md)
- **Verdict:** **CLOSED / LANDED** — all 5 work packages (`WP-20A`–`WP-20E`) plus review/gate remediations (`FIX-1` `862829ae2`, `FIX-2` `97dd23550`, `4f77aa5bd`, `FIX-3` `7bd48de48`) landed, verified green by the full unconfined `./scripts/agent-verify.sh` gate (`246/249` suites passed, `3` platform-skipped, `0` failed, `0` security violations), `mkdocs build --strict` (`0.56s`, `0` warnings), and approved by independent `pro` adversarial code review (`VERDICT: APPROVE`, zero open P1 blockers).

## 1. Completed Work Packages

1. **WP-20A (`2408bdab5`) — Zero-Behavior-Change Engine Test Relocation & Hotspot Splits (`R32-07`):**
   - Relocated 10 top-level `*_tests.rs` files from `fathomdb-engine/src/` into `fathomdb-engine/src/unit_tests/` while keeping `lib.rs` `mod tests {` untouched (`test_slice90_root_reconciliation.py` green).
   - Split `search.rs` (`3,847 → 429` lines + 6 submodules: `bm25f`, `fts_fast_path`, `fusion_stage`, `graph_arm`, `projected_text`, `vector_arm`) and `evidence.rs` (`3,628 → 620` lines + 4 submodules: `codec`, `graph`, `ranked`, `tests`) with zero behavior change (`T2` tripwire constants intact).
   - Updated `dev/tools/module-boundary-policy.txt` and Slice 65/90/WAL owner gates.

2. **WP-20B (`bf4e3b598`) — Declarative `TABLE_MANIFEST`, Schema 39 (`S39-COMPOSITE`), 8-Col `vec0`, and `fathomdb migrate 38 → 39`:**
   - Promoted the 7 fragmented table inventories into a single-source declarative `TABLE_MANIFEST` (`35` Schema-39 tables + `TableExportClass`) in `fathomdb-schema/src/lib.rs`.
   - Bumped `SCHEMA_VERSION` `38 → 39` (`STEP_39_SQL` `TC-11`-safe non-recreating index swaps on `canonical_nodes`/`canonical_edges`/`operational_mutations`, composite `(scope, ...)` PKs/`UNIQUE` constraints + restored triggers across `operational_state`, `_fathomdb_actuation_receipts`, `_fathomdb_actuation_receipt_source_refs`, `_fathomdb_artifact_revisions`, `_fathomdb_source_versions`, `_fathomdb_source_links`, `_fathomdb_source_dependencies`, `_fathomdb_dependency_closures`, and `_fathomdb_erasure_requests`).
   - Upgraded `vector_default` `vec0` to 8 metadata columns adding `scope TEXT` (`7 → 8` upgrade joining both `canonical_nodes` and `canonical_edges`; `8 → 8` preserving `s.scope`; `mean.rs` requantize preserving `scope`).
   - Committed tracked golden fixture `dev/fixtures/golden-0.8.31-schema38.sqlite` (`sha256: e0a078bde37ec3e973bba8b03c9bfb7c42505b16da1b038ca946fb08332c8036`) and added `migrate_38_to_39` alongside `migrate_37_to_38` with CLI `38 → 39` and chained `37 → 38 → 39` support.

3. **WP-20C (`4000b7bf7`) — Engine `ScopePolicy` & Route-by-Route Isolation (`R32-01`):**
   - Added `src/scope.rs` (`ScopePolicy`, `ScopePredicate`, `MAX_READ_SCOPES = 32`, `MAX_SCOPE_TOKEN_BYTES = 128`), `EngineError::ScopeViolation`, and `graph::search_expand` `ReadView` parity while keeping `struct Engine` at `49` fields and preserving unmutated `requested_config`.
   - Threaded `ScopePredicate` across every read, FTS/vector/hybrid/BM25F/projected/historical/`use_graph_arm` search, `(scope, logical_id)` graph BFS + `AsOf` tombstones, frozen/cursor/evidence nondisclosure, writes, actuation, provenance, dependency, lifecycle, erasure, and queue path — with single-scope (`scope = ""`) binding the zero-overhead fast path.
   - Landed comprehensive two-caller / multi-read-scope / revocation / anti-spoof / idempotency tests (`slice20_caller_scope_isolation.rs`).

4. **WP-20D (`919ed15e1`) — `Pin A` Rust / Python / TypeScript Parity:**
   - Wired `ScopePolicy`, `ScopeViolationError` (`48` shared error classes: base + `47` leaf), and `graph.search_expand` `ReadView` across `fathomdb`, `fathomdb-sdk`, `fathomdb-cli`, `fathomdb-mcp`, `fathomdb-py` / `src/python/`, and `fathomdb-napi` / `src/ts/`.
   - Preserved `scripts/governed-surface-pin.json` unchanged (`56` ops, `93` allowlist, `5` core, `5` recovery denylist) and rebuilt owned `.venv` wheel + NAPI addon.

5. **WP-20E (`a88e322b0`) — Interfaces, Release State, and Pre-Publish `0.8.32` Docs:**
   - Updated `dev/interfaces/{rust,rust-sdk,python,typescript,cli,wire}.md`, bumped `release-state-0.8.32.json` (`schema_version: 39`, `public_docs_target_release: "0.8.32"`), and updated all 16 `PUBLIC_DOCS` to pre-publish `0.8.32` stating explicitly that single-scope / single-agent (`scope = ""`) is the default operational posture (`newest published 0.8.31` preserved).
   - Fixed 8 `lychee` links broken by `WP-20A`'s `unit_tests/` relocation.

6. **Review & Full-Gate Remediations (`FIX-1` `862829ae2`, `FIX-2` `97dd23550`, `4f77aa5bd`, `FIX-3` `7bd48de48`):**
   - **`FIX-1` (`862829ae2`):** Scoped `historical_dense_incomplete`, partitioned `projection_generation::status_in_snapshot` + cache by scope, fixed multi-read-scope disambiguation in `read_mutation_projection_status`/`dependency_for_derived`/`dependency_trace`, aligned `dev/interfaces/python.md` `ValueError` + TS lone UTF-16 surrogate `TypeError` guard (`AC-068b`), refreshed CLI/migrate Schema-39 help docs, `_fathomdb.pyi`, `test_slice90_{read,write}_importance_owner.py`, and `dev/DOC-INDEX.md`.
   - **`FIX-2` (`97dd23550`, `4f77aa5bd`):** Updated minimal Schema-39 index/layout compatibility in `src/inspector/`, `fathomdb-tc5-benchmark/src/bin/scope_schema_experiment.rs`, `fathomdb-sdk/examples/slice30_measure.rs`, and `dev/experiments/0.8.29-slice30-measurements/g_runtime_tests.py`, and registered `EngineConfig.scope_policy` as `KnobClass.HELD_CONSTANT` in the EARP knob catalog (`src/python/eval/earp/knobs.py`, `src/python/tests/earp/test_config_resolver.py`).
   - **`FIX-3` (`7bd48de48`):** Derived `COMPILED_SCHEMA` dynamically from `fathomdb-schema/src/lib.rs` in `src/inspector/tools/local-release.py` (unblocking `build-inspector` in `release.yml`), synced `test_local_release.py` (refusing both Schema `37` and `38`), `src/inspector/tests/browser/{package,installed}.mjs`, and `scope_schema_experiment.rs` `ON CONFLICT(scope, collection_name, record_key)`.

## 2. Verification Summary

- Full unconfined `./scripts/agent-verify.sh` on `/mnt/data/worktrees/release-0.8.32` at `7bd48de48`: **EXIT `0`**
  - `agent-lint.sh`, `agent-typecheck.sh`, `agent-build.sh`: **PASS** (`skip lint-rust-tegra-pool` on x86_64 host)
  - `agent-security`: **PASS** (`AC-036`, `AC-037`, `AC-038`, `AC-050a`, `AC-050c` — `0 violation(s), 0 blocker(s), 0 downgrade(s)`)
  - `agent-test.sh`: **PASS** (`246/249` suites passed, `3` platform-skipped, `0` excluded, `0` failed; `test-python`: `1,815 passed, 36 skipped, 0 failed` + verified native receipt)
- `mkdocs build --strict`: **PASS** (`0.56s`, `0` warnings)
- Independent `pro` adversarial code review (`slice-reviewer`): **`VERDICT: APPROVE`** at `7bd48de48` (zero open P1 blockers; two non-blocking Inspector multi-scope SQL & Inspector README carry-forwards explicitly scheduled into Slice 30 `WP-30C`).
