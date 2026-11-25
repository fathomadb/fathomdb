---
title: Slice 30 Status — 0.8.32 Opt-In Bulk Write Mode and Inspector Scope UI
status: CLOSED
target_release: 0.8.32
slice: 30
date: 2026-11-25
---

# Slice 30 Status — `0.8.32` Opt-In Bulk Write Mode and Inspector Scope UI (`BULK-AND-INSPECTOR`)

- **Branch / Worktree:** `dev/0.8.32` in `/mnt/data/worktrees/release-0.8.32`
- **Plan:** [`dev/plans/0.8.32/features/slice-30/plan.md`](./plan.md)
- **Verdict:** **CLOSED / LANDED** — all 3 work packages (`WP-30A`–`WP-30C`) plus review/gate remediations (`FIX-1` `e03dd3a62`, `FIX-2` `46e5cdccc`, `FIX-3` `56b897085`) landed, verified green by the full unconfined `./scripts/agent-verify.sh` gate (`246/249` suites passed, `3` platform-skipped, `0` failed, `0` security violations), live local Inspector Playwright browser smoke (`13/13` including `scope.mjs`), `mkdocs build --strict` (`0.56s`, `0` warnings), and approved by independent `pro` adversarial code review (`VERDICT: APPROVE`, zero open P1 blockers).

## 1. Completed Work Packages

1. **WP-30A (`9a0fd6b7d`) — Engine Opt-In Bulk Write (`WriteOptions` / `write_with_options`) & `fathomdb doctor` Edge Verification (`R32-03`):**
   - Added public `EndpointValidationMode::{Strict, BulkNonStrict}`, `#[non_exhaustive] WriteOptions`, and `Engine::write_with_options` in `fathomdb-engine/src/write.rs` (keeping `Engine::write` strictly validated, `Engine::actuate` strict-only, `struct Engine` at `49` fields, and `phase3-root-reconciliation.md` `lib_rs_blob` synchronized).
   - Implemented `account_bulk_non_strict_batch_edge_endpoints` in `write_commit.rs` with zero-SQL fast path on non-edge batches, surviving-active-edge accounting across same-batch supersession, chunked same-scope resolution, and O(log N) distinct-scope index-jump probing aborting cross-scope active endpoints atomically with `EngineError::ScopeViolation`.
   - Extended `fathomdb doctor`: added `DataPlaneIntegrityCheckV1::EdgeEndpointIntegrity` (5th canonical check, `--check edge_endpoint_integrity`, content-free `dangling_edge_endpoint` / `cross_scope_edge_endpoint` findings) and `operator::semantic_section` (`E_DANGLING_EDGE_ENDPOINT` / `E_CROSS_SCOPE_EDGE_ENDPOINT` anchored in `dev/design/recovery.md`, preserving `AC-043a` 3-key `IntegrityReport`).
   - Added TDD integration suites `bulk_write_opt_in.rs` (`8/8`) and `doctor_dangling_edges.rs` (`5/5`).

2. **WP-30B (`b8276b51a`, pin SHA `2e7219b1d`) — `Pin B` Multi-Language SDK Parity, Governed Surface Pin & Docs (`X1`, `X4`):**
   - Exposed `EndpointValidationMode`, `WriteOptions`, and `Engine.write_with_options` / `Engine.writeWithOptions` across Rust (`fathomdb`, `fathomdb-sdk` keeping 3 governed namespaces + `guard::prepared_write`), Python (`fathomdb-py`, `src/python/`, `_fathomdb.pyi`, `python_surface_additions.json`, `test_slice130_package_boundary.py`), and TypeScript (`fathomdb-napi`, `src/ts/`, keeping `write.ts` private).
   - Updated the Gate 1 Decision 4 Pin B governed surface (`56 → 57` operations, `93 → 95` allowlist members; `core = 5`, `recovery_denylist = 5`, `runtime_controls = 2` unchanged), regenerated `docs/generated/sdk-operation-map.md`, updated `dev/interfaces/{rust,rust-sdk,python,typescript,cli}.md` and `docs/reference/*`, and added Python (`test_bulk_write_opt_in.py`) and TypeScript (`bulk-write-opt-in.test.ts`) parity suites.

3. **WP-30C (`67874c511`) — Inspector `TABLE_MANIFEST` Admission, Active-`scope` UI & Playwright CI Wiring (`R32-06`):**
   - Replaced hardcoded 35-table validation in `src/inspector/src/admission.rs` with `fathomdb_schema::TABLE_MANIFEST` while keeping 8-col `vec0` (`scope TEXT`) guards intact.
   - Scope-qualified all remaining Inspector SQL joins (`storage.rs`, `graph.rs`, `lexical.rs`, `vector.rs`), added `Session::ensure_engine_scope` (`Option<Engine>`) for native serving/lexical/graph scope switching (`scope: None` uniformly `D-01`-oblivious `""`), added `D-01`-safe per-scope `catalog()` discovery, and added the active-`scope` UI switcher/filter in `assets/app.js` (uncluttered on single-scope stores).
   - Added multi-scope `"scoped"` fixture mode, Rust integration suite `src/inspector/tests/integration/scope.rs`, Playwright suite `src/inspector/tests/browser/scope.mjs` (registered in `local-ci.sh`), updated Inspector docs/READMEs for Schema `39`, and wired `inspector-browser-smoke` (pinned `playwright@1.62.1` in `$RUNNER_TEMP/inspector-pw`) into `.github/workflows/ci.yml` + `scripts/tests/test_ci_proportional_routing.py`.

4. **Review & Full-Gate Remediations (`FIX-1` `e03dd3a62`, `FIX-2` `46e5cdccc`, `FIX-3` `56b897085`):**
   - **`FIX-1` (`e03dd3a62`):** Split `use crate::write_commit::load_next_cursor;` in `data_plane_integrity.rs` (`test_slice90_write_cursor_owner.py`), bumped `setup-node` pin counts in `test_actionlint_fixture.sh` (`5 → 6` in `ci.yml`, `20 → 21` total), and resolved P2 doc/anchor refinements in `dev/interfaces/{cli,rust-sdk}.md`, `docs/reference/cli.md`, and `src/inspector/tools/README.md`.
   - **`FIX-2` (`46e5cdccc`) & `FIX-3` (`56b897085`):** Registered `engine.write_with_options` in `scripts/slice135_python_capabilities.py` (`288/288`), implemented the exact double-partial-index O(log N) superseded-only check in `collect_operator_edge_endpoint_findings` (admitting genuine superseded-only same-scope active nodes with no `superseded_at IS NULL` row to match `fathomdb migrate` while still flagging missing, `pending`, `deleted`, `purged`, and cross-scope endpoints; keeping `doctor data-plane-integrity --check edge_endpoint_integrity` strictly `superseded_at IS NULL AND state='active'`), aligned `dev/design/recovery.md` / `dev/interfaces/cli.md` / `docs/reference/cli.md`, and added regression coverage in `doctor_dangling_edges.rs`.

## 2. Verification Summary

- Full unconfined `./scripts/agent-verify.sh` on `/mnt/data/worktrees/release-0.8.32` at `56b897085`: **EXIT `0`**
  - `agent-lint.sh`, `agent-typecheck.sh`, `agent-build.sh`: **PASS** (`skip lint-rust-tegra-pool` on x86_64 host)
  - `agent-security`: **PASS** (`AC-036`, `AC-037`, `AC-038`, `AC-050a`, `AC-050c` — `0 violation(s), 0 blocker(s), 0 downgrade(s)`)
  - `agent-test.sh`: **PASS** (`246/249` suites passed, `3` platform-skipped, `0` excluded, `0` failed; `test-python` + native receipt PASS; `test-ts` PASS; `test-slice135-harness` `288/288` PASS)
- Live local Inspector verification: `src/inspector/tests/tools` (`25/25`), JS unit (`11/11`), and live Playwright browser smoke (`api.mjs`, `ux.mjs`, `scope.mjs` — `13/13` PASS against Chromium `1248`)
- `mkdocs build --strict`: **PASS** (`0.56s`, `0` warnings)
- Independent `pro` adversarial code review (`slice-reviewer`): **`VERDICT: APPROVE`** at `56b897085` (`0` P1 blockers).
