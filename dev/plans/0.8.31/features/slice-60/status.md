---
title: Slice 60 Status — FathomDB Inspector 0.3.0 (Schema 38, Historical Graph, Scoped CI & Release Packaging)
status: CLOSED
target_release: 0.8.31
slice: 60
date: 2026-11-23
---

# Slice 60 Status — `0.8.31` Inspector (`INSPECTOR`)

- **Branch / Worktree:** `release/0.8.31` in `/mnt/data/worktrees/release-0.8.31`
- **Plan:** [`dev/plans/0.8.31/features/slice-60/plan.md`](./plan.md)
- **Commits:** `35543da54` (Layer 1 — Inspector `0.3.0` Rust backend), `e86100641` (Layer 2 — UI, Playwright tests, `P7-02` timeouts, scoped `local-ci.sh`, `D4 = B` `local-release.py`), `8689b781d` (Layer 3 — `build-inspector` workflow, release contract tests, Inspector `0.3.0` docs), `6685b0ccf` (`FIX-1` — `i64` `graph_serving` properties, remaining 4 browser teardown bounds, `TMPDIR` canonicalization, empty `as_of` seed expansion, custom vector kind test)
- **Verdict:** **CLOSED / LANDED** — all 3 implementation layers + `FIX-1` verified green across Rust (`default` + `cached-cpu`), Playwright browser suites, Python tool unittests, shellcheck, release workflow contract suites, and MkDocs/doc gates; independent adversarial code review (`07d16275`) returned **`APPROVE`** at HEAD `6685b0ccf`.

## 1. Delivered Scope

1. **Layer 1 (`35543da54`) — Inspector `0.3.0` Rust Backend (`admission.rs`, `storage.rs`, `graph.rs`, `lexical.rs`, `vector.rs`, `worker.rs`, `failure.rs`):**
   - Bumped `src/inspector/Cargo.toml` + `Cargo.lock` to `0.3.0`, enforced strict Schema `38` admission (`malformed_layout` on missing Schema 38 columns/tables, Schema `37` refused byte-for-byte untouched).
   - Kept `"properties"` out of `Class::fields()`, threaded `Policy` into `summary` so unpermitted rows withhold `properties`, `properties_bytes`, and `typeof(properties)` (`"properties_status": "refused"`), added `"properties"` to `graph_admission()` and `lexical_admission()` 64 MiB / 16 MiB caps, added `tree_field(handle, field)` (`unsupported_field` on unknown columns), bounded `catalog` `canonical_property_values` + `_fathomdb_commit_timestamps` counts under the 100k `remaining` budget + `no_show_inactive`, and extended `associations` for both `Node` and `Edge`.
   - Implemented `Option<String>` `as_of_write_cursor` / `as_of_epoch_ms` (`"invalid_as_of"` registered in both `Failure::read` and `Failure::decode::CODES`), O(log N) `_fathomdb_commit_timestamps` reverse seek (cursor `0` pre-first-commit), `P4-13` `INDEXED BY` lift using two-branch `UNION ALL` over `canonical_nodes_logical_{active,sup}_idx` and `canonical_edges_{from,to}_sup_kind_idx` (preserving non-partial `canonical_edges_{from_id,to_id}_idx` in `!no_show_inactive` current mode), historical seed resolution, and full 8-flag erasure subordination.
2. **Layer 2 (`e86100641`) & `FIX-1` (`6685b0ccf`) — Web UI, Playwright Browser Tests, `P7-02` Timeouts, Scoped `local-ci.sh`, & `D4 = B` `local-release.py`:**
   - Threaded `field` through `app.js` `tree()` / `drawTree()` / copy actions, added `.textContent`-only XSS-safe `Properties` & `Scope` regions (zero `innerHTML`/`insertAdjacentHTML`/`document.write`), added `as_of` snapshot controls with pre-API validation + `seedExpanded` preservation across empty `as_of` snapshots + comparison pin forwarding, exact `i64` string serialization in `graph_serving` `property_map_json`, and `presentation.mjs` `entity_class === "edge"` exclusion.
   - Bounded subprocess, `Xvfb`, `strace`, `browser.close()`, child teardown, and `/proc` descendant reaping across `run-installed.sh`, `qualify-installed-network.sh`, `installed-harness.mjs`, `helpers.mjs`, all 8 Playwright specs, all `qualify-*.mjs` scripts, and `tests/launch.rs` (`wait_with_sigkill_fallback`).
   - Made `local-ci.sh` default to `--scoped` with `--full-workspace`, `--installed-only`, `--base <ref>`, canonicalized optional auto-temp workspace, fail-closed diff guard, `package.sh` `grep -Fq` + dynamic `Cargo.toml` version, and `D4 = B` `local-release.py` (`COMPILED_SCHEMA = 38`, dynamic engine provenance, static zero-`docker`/`openbox`/`xdotool`/`sudo` guard).
3. **Layer 3 (`8689b781d`) — `.github/workflows/release.yml` `build-inspector` & Docs:**
   - Added `build-inspector` on `ubuntu-24.04` (`needs: [verify-release, verify-cuda-trusted-route]`, `permissions: { contents: read }`, SHA-pinned checkout + Rust `1.95.0`, `cargo fetch --locked`, `package.sh`, `check-glibc-floor.sh --floor 2.39`, `local-release.py seal` + `verify --source`, non-colliding `.tar.gz` + `.tar.gz.sha256`, `upload-artifact-retry` count `14 -> 15`), wired `all-builds-passed.{needs,if}` and `github-release` download, and updated all 4 release workflow suites + `dev/inspector/*`, `src/inspector/{tools,package}/README.md`, `docs/inspector/**`, `docs/inspector/releases/0.3.0.md`, `mkdocs.yml`, and `docs/contributing/index.md`.

## 2. Verification Summary

- **Rust:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo clippy -p fathomdb-inspector --no-default-features --features cached-cpu --all-targets -- -D warnings` — **PASS** (0 warnings); `fathomdb-inspector` default **92 passed / 0 failed**, `cached-cpu` **93 passed / 0 failed** (incl. `launch` 3/3 and `schema38_custom_enrolled_node_vector_kind_and_edge_fact_rejection`).
- **Tools, Shell & Browser:** `src/inspector/tests/tools` **25/25 passed**, `scripts/agent-lint-shell.sh` **PASS**, `node --test src/inspector/tests/unit/*` **11/11 passed**, Playwright `{attributes,graph,workspace,ux,lexical}.mjs` **31/31 passed** (`comparison.mjs` 19/20 in sandbox; full unconfined installed suite executes in Slice 70 `WP-70D`), `package.sh` + `check-glibc-floor.sh --floor 2.39` **PASS**.
- **Workflow & Docs:** `actionlint`, `test_release_contract_truth.sh` (37/37), `test_release_workflow_scope.sh` (16/16), `test_runtime_release_alignment.sh` (18/18), `test_linux_first_platform_scope.sh`, `agent-lint.sh`, `agent-typecheck.sh`, `agent-lint-docs.sh`, `check-public-doc-truth.py`, `check-design-refs.sh`, `mkdocs build --strict` — **ALL PASS**; `dev/acceptance.md` untouched; zero external consumer-project mentions.
- **Adversarial Code Review:** Independent `slice-reviewer` (`07d16275`) returned **`APPROVE`** at HEAD `6685b0ccf` (0 open P0/P1/P2 findings).
