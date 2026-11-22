---
title: Slice 10 Status — 0.8.31 Prework Prep
status: CLOSED
target_release: 0.8.31
slice: 10
date: 2026-11-22
---

# Slice 10 Status — `0.8.31` Prework Prep (`PREWORK-PREP`)

- **Branch / Worktree:** `release/0.8.31` in `/home/coreyt/projects/fathomdb-worktrees/release-0.8.31`
- **Plan:** [`dev/plans/0.8.31/features/slice-10/plan.md`](./plan.md)
- **Verdict:** **CLOSED / LANDED** — all 4 work packages (`WP-10A`–`WP-10D`) verified green and approved by independent adversarial `pro` code review (`APPROVE` after `FIX-1`, zero open P1/P2 findings).

## 1. Completed Work Packages

1. **WP-10A — ADR Amendments & Project-Boundary Scrub (`P4-09`, `P4-01`, `P4-04`, `P4-06`, `P4-10`, `P5-01`, `P5-11`, W12 Amendment, HITL Gate 1 #5):**
   - Amended `dev/adr/ADR-0.8.31-property-graph-inline-and-declared-indexes.md` (`#73`), `dev/adr/ADR-0.8.31-historical-graph-and-fused-seeding.md` (`#75`), and `dev/adr/ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md` (`#76`) in lockstep:
     - Schema 38 `_fathomdb_commit_timestamps(write_cursor INTEGER PRIMARY KEY, committed_at_ms INTEGER NOT NULL)` + `_fathomdb_commit_timestamps_ms_idx(committed_at_ms, write_cursor)`, keyed by batch high-water `WriteReceipt.cursor`, populated in `commit_batch` and `actuate_new` strictly when `next_cursor > base_cursor`, monotonically clamped via reverse PK lookup, classified `DurableAuthority`.
     - Unified Schema 38 historical indexes: `canonical_edges_{from,to}_sup_kind_idx(from_id|to_id, superseded_at, kind)` and `canonical_nodes_logical_sup_idx(logical_id, superseded_at, write_cursor)`.
     - Scoped `as_of` historical graph reconstruction honestly to immutable write revisions alongside current-state `Engine::transition` semantics and migrated-store pre-upgrade `as_of_epoch_ms` -> cursor `0`/empty semantics.
     - Pinned `vec0` 8-slot node `Eq` metadata budget (excluding `entity_class = 'edge'`, reserving 1 slot for `0.8.32` `scope`), `visibility_coalescing_trigger_sql!` for `("cpv", "canonical_property_values")`, explicit opt-in `R1` vector kinds (`_fathomdb_vector_kinds`, rejecting `"edge_fact"`), and Step 6 `fathomdb migrate` DDL lockstep.
   - Amended `dev/adr/ADR-0.8.28-resumable-physical-cleanup.md` with the Autonomous two-truncate ordering (`w12-fence-review.md`).
   - Updated `dev/adr/ADR-0.6.0-decision-index.md` (`#72`–`#76`) and scrubbed all external consumer-project specifics from `dev/adr/ADR-0.8.31-*.md`, `dev/plans/0.8.{31..34}/*`, `README.md`, `src/ts/README.md`, and touched public docs (`0` leaks verified).

2. **WP-10B — Security & Lockfile Hygiene (`P1-01`, `P1-02`, opt. `P1-03`):**
   - `P1-01`: Updated root `package.json` / `package-lock.json` to `markdownlint-cli2 ^0.24.2` (`markdownlint@0.42.2`, `re2js@2.9.1`, `js-yaml@5.5.0`), kept `"overrides"` empty, verified `npm ci` + `scripts/check-pinned-override-rot.sh` + `scripts/tests/test_dev_environment_tools.sh`.
   - `P1-02`: Updated `Cargo.lock` narrowly (`rustix 1.1.4 -> 1.1.6`, closing `RUSTSEC-2026-0486` with zero product/fork pin drift); verified `cargo tree --locked` + `cargo check --workspace --locked --offline`.
   - `P1-03` (optional/non-blocking): Remote `origin/dependabot/*` branch deletion deferred per plan non-blocking rule (no remote destructive deletions performed).

3. **WP-10C — Public-Doc Truth Gate Green + Quick Doc & Contributor Hygiene (`P3-18`, `P3-19` pre-publish, `P2-02`/`P6-09`, `P2-04`/`P6-10`, `P6-01`, `P6-07`, `P6-11`):**
   - Flipped all 16 `PUBLIC_DOCS` to pre-publish `0.8.31` target (keeping `0.8.30` published truth and `SCHEMA_VERSION = 37` until Slice 20), removed all stale `"CUDA 13 ... planned for 0.8.31"` promises, updated `README.md` to `13 Rust workspace members`.
   - Fixed broken `engine.write([])` verify snippets (`P6-01`) across Python/TS install/quickstart/reference docs and `src/ts/tests/fixtures/consumer-package-root.mts`; executed both updated Python and TypeScript verify snippets in isolated `/tmp` tempdirs (`ok`).
   - Updated error subclass counts (`44 -> 46`, `P6-07`), `AGENTS.md` (`182 <= 300` lines, 13 workspace crates, removed `prettier`, added `rust-sdk.md`/`mcp.md`), `docs/contributing/index.md` (`P6-11`), `dev/DOC-INDEX.md`, `dev/ROADMAP.md`, `dev/design/fathomdb-data-plane-architecture-v2.md`, and `dev/plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md`.

4. **WP-10D — CI, Security & Release-Friction Hardening (`P7-03`, `P7-07`–`P7-10`):**
   - `P7-03`: Wrapped `strace`/`unshare` calls in `check-no-listen.sh`, `check-netns-deny-egress.sh`, and `check-netns-deny-egress-catch.sh` with `timeout --kill-after=5s`, capturing exit codes before `|| true` and mapping `124`/`137` to `VIOLATION` (`exit 1`) without matching ptrace-blocker patterns. Added `scripts/tests/test_security_timeouts.sh` (RED -> GREEN).
   - `P7-07`: Extended `scripts/release/wait-for-registry-version.py` with PEP 691 `/simple/<pkg>/` 5-platform wheel verification, PEP 440 pre-release normalization, and body-read transport-error handling (`URLError`/`TimeoutError`/`HTTPException`/`OSError`); added bounded transient CDN-skew retry + `>= 1` attempts guard in `smoke-pypi-wheel.{sh,ps1}` preserving `FIXTURE_INSTALL_EXIT=42` non-retry. Updated `test_wait_for_registry_visibility.py`, `test_pypi_smoke_shell.sh`, and `test_smoke_scripts.sh` in lockstep (RED -> GREEN).
   - `P7-08`: Added `scripts/release/extract-changelog-section.sh` and wired `actions/checkout@` (`persist-credentials: false`), changelog extraction, and `body_path: ${{ runner.temp }}/release-body.md` into `github-release` in both `.github/workflows/release.yml` and `.github/workflows/promote-npm-release.yml`. Added `scripts/tests/test_extract_changelog_section.sh` (RED -> GREEN).
   - `P7-09`: Added Bash 3 `smoke_complete=1` EXIT-trap completion sentinel to `smoke-npm-package.sh`, `smoke-crates-cli.sh`, and `smoke-crates-sdk.sh`, and added `smoke_slice55_installed.py` parity to `smoke-pypi-wheel.ps1`.
   - `P7-10`: Derived allowed bootstrap tag dynamically from `Cargo.toml [workspace.package].version` in `scripts/release/check-tag-cuda-route.sh` (+ `scripts/tests/test_check_tag_cuda_route.sh` and updated `scripts/tests/test_tag_cuda_route.sh`). Audited all 71 unclassified `HEAD`-reachable historical Gitleaks tuples (`--redact=100`: all SHA-256 content hashes or upstream SQLite C macros, zero live credentials), updated `gitleaks-history-manifest.json` (`171` records / `108` unique), `check-gitleaks-history.py`, and `test_gitleaks_history_classifier.py`, and verified clean CI-shaped single-branch `gitleaks-history.sh` `EXIT 0`.
   - Registered `test-security-timeouts`, `test-extract-changelog-section`, and `test-check-tag-cuda-route` in `scripts/agent-test.sh` (`fast` tier) and regenerated `docs/generated/contributor-map.md`.

## 2. Verification Summary

- `python3 scripts/check-public-doc-truth.py` — **PASS** (`documentation target 0.8.31; newest published receipt 0.8.30`)
- `./scripts/agent-lint-md.sh` & `./scripts/agent-lint-docs.sh` — **PASS** (`0` issues across 2,197 Markdown files and all public docs)
- `actionlint .github/workflows/release.yml .github/workflows/promote-npm-release.yml` + `test_actionlint_fixture.sh` — **PASS**
- `shellcheck` & `bash -n` on all touched shell scripts, `ruff check` on all touched Python scripts — **PASS**
- `scripts/check-pinned-override-rot.sh`, `scripts/check-release-state-views.sh`, `scripts/check-design-refs.sh` (`233/233`), `scripts/check-release-contract-truth.py` — **PASS**
- `cargo check --workspace --locked --offline` & `cargo tree --locked` — **PASS**
- Python & TypeScript `P6-01` verify snippets executed in isolated `/tmp` tempdirs (`tsc --noEmit` + `npm run build:debug`) — **PASS (`ok`)**
- WP-10D test suites: `test_security_timeouts.sh`, `test_ac036_ptrace_blocker.sh`, `test_extract_changelog_section.sh`, `test_check_tag_cuda_route.sh`, `test_tag_cuda_route.sh`, `test_wait_for_registry_visibility.py`, `test_pypi_smoke_shell.sh`, `test_smoke_scripts.sh`, `test_release_workflow_scope.sh`, `test_runtime_release_alignment.sh`, `test_public_smoke_recovery.py`, `test_npm_promotion.py`, `test_gitleaks_history_classifier.py`, `test_gitleaks_guards.sh` (31/31), `test_gitleaks_history_environment.sh`, E2E `gitleaks-history.sh` on single-branch clone (`EXIT 0`), `test_agent_test_tiers.sh` (246 labels), `test_agent_test_collect_all.sh`, `test_generated_public_docs.py` (8/8), `test_dev_environment_tools.sh`, `test_lint_md_hard_fail_on_missing_linter.sh`, `test_md_generators.sh` — **ALL PASS**
- Independent `pro` adversarial code review (`slice-reviewer`): **`APPROVE`** after `FIX-1` (all 6 findings closed, zero open P1/P2 findings).
